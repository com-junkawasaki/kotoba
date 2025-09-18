//! The core of the file-based routing system.
use crate::schema::{ApiRoute, PageModule, LayoutModule, HandlerWorkflow};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use glob::glob;
use kotoba_workflow::prelude::{WorkflowEngine, WorkflowExecution, WorkflowExecutionId, WorkflowStatus};
use kotoba_ssg::renderer::ComponentRenderer; // Assuming this is the main renderer


// The context available to workflow steps.
#[derive(Clone, Debug, Default)]
struct WorkflowContext {
    request: HttpRequest,
    // Results of previous steps are stored here.
    steps: HashMap<String, serde_json::Value>,
}

/// Simplified workflow executor for routing
#[derive(Debug, Clone)]
pub struct WorkflowExecutor;

/// A placeholder for an HTTP Request struct
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: serde_json::Value,
}

/// A placeholder for an HTTP Response struct
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: serde_json::Value,
}

/// Represents a node in the routing tree, corresponding to a directory.
#[derive(Default, Debug)]
pub struct RouteNode {
    api: Option<ApiRoute>,
    page: Option<PageModule>,
    layout: Option<LayoutModule>,
    children: HashMap<String, Box<RouteNode>>,
    dynamic_child: Option<String>, // To store the name of the dynamic segment, e.g., "id" from "[id]"
}

/// The result of a route matching operation.
struct RouteMatch<'a> {
    layouts: Vec<&'a LayoutModule>,
    api: Option<&'a ApiRoute>,
    page: Option<&'a PageModule>,
    params: HashMap<String, String>,
}

/// The main engine for processing file-based routes.
pub struct HttpRoutingEngine {
    root_node: Arc<RouteNode>,
    workflow_engine: Arc<WorkflowEngine>,
    ui_renderer: ComponentRenderer,
}

impl HttpRoutingEngine {
    /// Creates a new `HttpRoutingEngine`.
    pub async fn new(app_dir: &Path, workflow_engine: Arc<WorkflowEngine>) -> Result<Self> {
        let root_node = Self::discover_routes(app_dir).await?;
        Ok(Self {
            root_node: Arc::new(root_node),
            workflow_engine,
            ui_renderer: ComponentRenderer::new(), // Initialize the UI renderer
        })
    }

    /// Recursively scans the directory to build a routing tree.
    async fn discover_routes(app_dir: &Path) -> Result<RouteNode> {
        let mut root = RouteNode::default();
        let pattern = format!("{}/**", app_dir.to_str().unwrap());

        for entry in glob(&pattern)? {
            let path = entry?;
            if path.is_dir() {
                // We'll process files within directories, not the dirs themselves.
                continue;
            }
            
            let relative_path = path.strip_prefix(app_dir)?.parent().unwrap();
            let mut current_node = &mut root;
            for component in relative_path.components() {
                let key = component.as_os_str().to_string_lossy().to_string();
                if key.starts_with("[") && key.ends_with("]") {
                    let param_name = key[1..key.len()-1].to_string();
                    current_node.dynamic_child = Some(param_name);
                    current_node = current_node.children.entry(key).or_default();
                } else {
                    current_node = current_node.children.entry(key).or_default();
                }
            }

            match path.file_name().and_then(|s| s.to_str()) {
                Some("route.rs") | Some("route.kotoba") => {
                    // In a real implementation, we'd parse the file.
                    // For now, we assume it's an ApiRoute.
                    println!("Discovered API route at: {:?}", relative_path);
                    current_node.api = Some(ApiRoute::default());
                }
                Some("page.kotoba") => {
                    println!("Discovered Page at: {:?}", relative_path);
                    current_node.page = Some(PageModule::default());
                }
                Some("layout.kotoba") => {
                    println!("Discovered Layout at: {:?}", relative_path);
                    current_node.layout = Some(LayoutModule::default());
                }
                _ => {}
            }
        }
        Ok(root)
    }
    
    /// Finds a matching route by traversing the node tree.
    fn find_match<'a>(&'a self, path_segments: &[&str]) -> Option<RouteMatch<'a>> {
        let mut current_node = &*self.root_node;
        let mut layouts = vec![];
        let mut params = HashMap::new();

        if let Some(layout) = &current_node.layout {
            layouts.push(layout);
        }

        for (i, segment) in path_segments.iter().enumerate() {
            if let Some(child) = current_node.children.get(*segment) {
                current_node = child;
            } else if let Some(param_name) = &current_node.dynamic_child {
                // Found a dynamic segment (e.g., [id])
                params.insert(param_name.clone(), segment.to_string());
                let dynamic_key = format!("[{}]", param_name);
                if let Some(child) = current_node.children.get(&dynamic_key) {
                     current_node = child;
                } else {
                    return None; // No matching route
                }
            } else {
                return None; // No matching route
            }
            
            if let Some(layout) = &current_node.layout {
                layouts.push(layout);
            }
        }

        Some(RouteMatch { layouts, api: current_node.api.as_ref(), page: current_node.page.as_ref(), params })
    }

    /// Helper to execute a workflow and return its final result.
    async fn execute_workflow(
        &self,
        workflow: &HandlerWorkflow,
        context: &WorkflowContext,
    ) -> Result<serde_json::Value> {
        // This is still a simplified loop. A real implementation would involve
        // passing a mutable context and handling step dependencies.
        for step in &workflow.steps {
            println!("  - Executing step: {}", step.id);
            if step.step_type == crate::schema::WorkflowStepType::Return {
                // TODO: Interpolate context variables (e.g., from request params) into the body.
                return Ok(step.body.clone());
            }
        }
        // If no return step, return Null.
        Ok(serde_json::Value::Null)
    }

    /// Processes an HTTP request.
    pub async fn handle_request(
        &self,
        request: HttpRequest,
    ) -> Result<HttpResponse> {
        let path_segments: Vec<&str> = request.path.split('/').filter(|s| !s.is_empty()).collect();
        let route_match = self.find_match(&path_segments);
        
        if let Some(m) = route_match {
            if let Some(api) = m.api {
                if let Some(handler_workflow) = api.handlers.get(&request.method) {
                    println!("Executing API handler for {} {}", request.method, request.path);
                    
                    // --- Execute the actual workflow ---
                    let mut context = WorkflowContext { request: request.clone(), ..Default::default() };
                    
                    for step in &handler_workflow.steps {
                        // This is a simplified execution loop. A real implementation
                        // would use the `workflow_engine` to execute GQL, rewrites etc.
                        println!("  - Executing step: {}", step.id);
                        
                        // Example: if step.step_type == DbQuery {
                        //   let result = self.workflow_engine.execute_query(&step.query, &step.params).await?;
                        //   context.steps.insert(step.id.clone(), result);
                        // }
                        
                        // For now, we mock the final return step
                        if step.step_type == crate::schema::WorkflowStepType::Return {
                            // TODO: Interpolate context variables into the response body
                            return Ok(HttpResponse {
                                status_code: step.status_code,
                                headers: Default::default(),
                                body: step.body.clone(),
                            });
                        }
                    }
                    
                    // If workflow doesn't end with a return step
                    anyhow::bail!("Workflow did not produce a response.");
                }
            }

            // Priority 2: Page Route
            if let Some(page) = m.page {
                println!("Rendering Page for {}", request.path);
                
                let mut props = serde_json::Map::new();
                props.insert("params".to_string(), serde_json::to_value(&m.params)?);

                // --- Execute data loading workflows ---
                let initial_context = WorkflowContext { request: request.clone(), ..Default::default() };

                // 1. Execute layout workflows
                for layout in &m.layouts {
                    if let Some(workflow) = &layout.load_workflow {
                        println!("  - Loading data for layout...");
                        let result = self.execute_workflow(workflow, &initial_context).await?;
                        // We assume the result is an object to merge into props.
                        if let serde_json::Value::Object(map) = result {
                            props.extend(map);
                        }
                    }
                }
                
                // 2. Execute page workflow
                if let Some(workflow) = &page.load_workflow {
                    println!("  - Loading data for page...");
                    let result = self.execute_workflow(workflow, &initial_context).await?;
                    if let serde_json::Value::Object(map) = result {
                        props.extend(map);
                    }
                }

                // --- Render the UI component tree to HTML ---
                let html_body = self.ui_renderer.render_page(&m.layouts, page, props.into())?;
                
                return Ok(HttpResponse {
                    status_code: 200,
                    headers: { let mut map = HashMap::new(); map.insert("Content-Type".to_string(), "text/html".to_string()); map },
                    body: serde_json::Value::String(html_body),
                });
            }
        }

        // Nothing matched
        Ok(HttpResponse {
            status_code: 404,
            headers: Default::default(),
            body: serde_json::json!({ "error": "Not Found" }),
        })
    }
}
