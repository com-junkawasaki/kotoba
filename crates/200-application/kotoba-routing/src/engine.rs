//! The core of the file-based routing system.
use crate::schema::{ApiRoute, PageModule, LayoutModule, HandlerWorkflow};
use anyhow::{Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use glob::glob;
use kotoba_storage::KeyValueStore;
use tokio::sync::Mutex;
use tracing::warn;

// TODO: These will be replaced with KeyValueStore-based implementations
// use kotoba_workflow::prelude::{WorkflowEngine};
// use kotoba_ssg::renderer::ComponentRenderer;
// use kotoba_cid::Cid;


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
#[derive(Debug, Clone, Default)]
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
#[derive(Clone)]
struct RouteMatch {
    layouts: Vec<String>, // Store layout names instead of references
    api: Option<String>,  // Store route path instead of reference
    page: Option<String>, // Store page path instead of reference
    params: HashMap<String, String>,
}

/// The main engine for processing file-based routes with KeyValueStore backend.
pub struct HttpRoutingEngine<T: KeyValueStore + 'static> {
    storage: Arc<T>,
    root_node: Arc<RouteNode>,
    // TODO: Replace with KeyValueStore-based workflow engine
    // workflow_engine: Arc<WorkflowEngine>,
    // ui_renderer: ComponentRenderer,
    // The cache key is the CID of the HttpRequest.
    route_cache: Mutex<HashMap<String, Arc<RouteMatch>>>,
}

impl<T: KeyValueStore + 'static> HttpRoutingEngine<T> {
    /// Creates a new `HttpRoutingEngine`.
    pub async fn new(storage: Arc<T>, app_dir: &Path) -> anyhow::Result<Self> {
        let root_node = Self::discover_routes(app_dir).await?;
        Ok(Self {
            storage,
            root_node: Arc::new(root_node), // Remove .leak() - not available in stable Rust
            // TODO: Initialize with KeyValueStore-based implementations
            // workflow_engine,
            // ui_renderer: ComponentRenderer::new(),
            route_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Recursively scans the directory to build a routing tree.
    async fn discover_routes(app_dir: &Path) -> anyhow::Result<RouteNode> {
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
    fn find_match(&self, path_segments: &[&str]) -> Option<RouteMatch> {
        let mut current_node = &*self.root_node;
        let mut layouts = vec![];
        let mut params = HashMap::new();

        if current_node.layout.is_some() {
            layouts.push("layout".to_string()); // Store layout name
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
            
            if current_node.layout.is_some() {
                layouts.push("layout".to_string()); // Store layout name
            }
        }

        Some(RouteMatch {
            layouts,
            api: if current_node.api.is_some() { Some("api".to_string()) } else { None },
            page: if current_node.page.is_some() { Some("page".to_string()) } else { None },
            params
        })
    }

    /// Helper to execute a workflow and return its final result.
    async fn execute_workflow(
        &self,
        workflow: &HandlerWorkflow,
        context: &WorkflowContext,
    ) -> anyhow::Result<serde_json::Value> {
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

    /// Processes an HTTP request, utilizing a KeyValueStore-based cache.
    pub async fn handle_request(
        &self,
        request: HttpRequest,
    ) -> anyhow::Result<HttpResponse> {
        // 1. Calculate a cache key for the incoming request.
        // Using a simple hash for now - TODO: replace with proper CID
        let cache_key = format!("request:{}:{}", request.method, request.path);
        
        // 2. Check the cache first.
        let mut cache = self.route_cache.lock().await;
        if let Some(cached_match) = cache.get(&cache_key) {
            println!("[Cache] HIT for key: {}", cache_key);
            // NOTE: We drop the lock before executing the rest of the logic
            // to avoid holding it for too long.
            let route_match = Arc::clone(cached_match);
            drop(cache);
            return self.execute_match(request, &route_match).await;
        }
        drop(cache); // Explicitly drop lock

        println!("[Cache] MISS for key: {}", cache_key);

        // 3. If cache miss, perform the expensive route matching.
        let path_segments: Vec<&str> = request.path.split('/').filter(|s| !s.is_empty()).collect();
        if let Some(route_match) = self.find_match(&path_segments) {
            let boxed_match = Arc::new(route_match);

            // 4. Store the result in the cache.
            let mut cache = self.route_cache.lock().await;
            cache.insert(cache_key, Arc::clone(&boxed_match));
            drop(cache);

            return self.execute_match(request, &boxed_match).await;
        }

        // Nothing matched
        Ok(HttpResponse {
            status_code: 404,
            headers: Default::default(),
            body: serde_json::json!({ "error": "Not Found" }),
        })
    }

    /// A new helper function to execute the logic after a match is found (either from cache or fresh).
    async fn execute_match(&self, request: HttpRequest, route_match: &Arc<RouteMatch>) -> anyhow::Result<HttpResponse> {
        if route_match.api.is_some() {
            // TODO: Implement API handler execution with KeyValueStore
            println!("Executing API handler for {} {}", request.method, request.path);

            // For now, return a simple response
            return Ok(HttpResponse {
                status_code: 200,
                headers: Default::default(),
                body: serde_json::json!({ "message": "API handler executed" }),
            });
        }

        // Priority 2: Page Route
        if route_match.page.is_some() {
            println!("Rendering Page for {}", request.path);

            // TODO: Implement page workflow execution with KeyValueStore
            warn!("Page workflow execution not implemented yet");

            return Ok(HttpResponse {
                status_code: 200,
                headers: { let mut map = HashMap::new(); map.insert("Content-Type".to_string(), "text/html".to_string()); map },
                body: serde_json::Value::String("<html><body>Page rendering TODO</body></html>".to_string()),
            });
        }

        // If no API or Page route matched
        anyhow::bail!("No route matched for the given request.");
    }
}
