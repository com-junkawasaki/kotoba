//! AI Chains for multi-step workflow orchestration

use crate::{KotobaNetError, Result, AiModels, AiTools, AiMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Step type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// LLM call step
    LlmCall,
    /// Tool call step
    ToolCall,
    /// Data transformation step
    Transform,
    /// Conditional execution step
    Conditional,
}

/// Chain step configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    pub name: String,
    pub step_type: StepType,
    pub tool: Option<String>,
    pub parameters: serde_json::Value,
    pub config: HashMap<String, serde_json::Value>,
    pub condition: Option<String>, // Jsonnet expression for conditional execution
}

/// AI Chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChain {
    pub name: String,
    pub description: String,
    pub steps: Vec<ChainStep>,
    pub max_iterations: u32,
    pub timeout_seconds: Option<u64>,
}

/// Chain execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub steps_executed: usize,
    pub total_steps: usize,
    pub error: Option<String>,
}

/// AI Chains orchestrator with advanced execution capabilities
pub struct AiChains {
    chains: Vec<AiChain>,
    models: AiModels,
    tools: AiTools,
}

impl AiChains {
    /// Create new AI chains orchestrator
    pub fn new() -> Self {
        Self {
            chains: Vec::new(),
            models: AiModels::new(),
            tools: AiTools::new(),
        }
    }

    /// Create with existing models and tools
    pub fn with_components(models: AiModels, tools: AiTools) -> Self {
        Self {
            chains: Vec::new(),
            models,
            tools,
        }
    }

    /// Add chain configuration
    pub fn add_chain(&mut self, chain: AiChain) {
        self.chains.push(chain);
    }

    /// Execute chain by name
    pub async fn execute_chain(&self, name: &str, initial_context: serde_json::Value) -> Result<ChainResult> {
        if let Some(chain) = self.chains.iter().find(|c| c.name == name) {
            self.execute_chain_internal(chain, initial_context).await
        } else {
            Err(KotobaNetError::NotFound(format!("Chain '{}' not found", name)))
        }
    }

    /// Execute chain with full implementation
    async fn execute_chain_internal(&self, chain: &AiChain, initial_context: serde_json::Value) -> Result<ChainResult> {
        let mut context = initial_context;
        let mut steps_executed = 0;

        for (i, step) in chain.steps.iter().enumerate() {
            // Check iteration limit
            if steps_executed >= chain.max_iterations as usize {
                return Ok(ChainResult {
                    success: false,
                    output: context,
                    steps_executed,
                    total_steps: chain.steps.len(),
                    error: Some("Maximum iterations exceeded".to_string()),
                });
            }

            // Evaluate condition if present
            if let Some(condition) = &step.condition {
                if !self.evaluate_condition(condition, &context).await? {
                    continue; // Skip this step
                }
            }

            // Execute step based on type
            match step.step_type {
                StepType::LlmCall => {
                    let result = self.execute_llm_step(step, &context).await?;
                    if let Some(key) = context.as_object_mut() {
                        key.insert(step.name.clone(), result);
                    }
                }
                StepType::ToolCall => {
                    let result = self.execute_tool_step(step, &context).await?;
                    if let Some(key) = context.as_object_mut() {
                        key.insert(step.name.clone(), result);
                    }
                }
                StepType::Transform => {
                    let result = self.execute_transform_step(step, &context).await?;
                    if let Some(key) = context.as_object_mut() {
                        key.insert(step.name.clone(), result);
                    }
                }
                StepType::Conditional => {
                    let result = self.execute_conditional_step(step, &context).await?;
                    if let Some(key) = context.as_object_mut() {
                        key.insert(step.name.clone(), result);
                    }
                }
            }

            steps_executed += 1;
        }

        Ok(ChainResult {
            success: true,
            output: context,
            steps_executed,
            total_steps: chain.steps.len(),
            error: None,
        })
    }

    /// Execute LLM step
    async fn execute_llm_step(&self, step: &ChainStep, context: &serde_json::Value) -> Result<serde_json::Value> {
        // Extract model configuration
        let model_name = step.config.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("gpt-3.5-turbo");

        let temperature = step.config.get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);

        let max_tokens = step.config.get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(1000);

        // Build prompt from template
        let prompt_template = step.config.get("prompt_template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KotobaNetError::Parse("Missing prompt_template for LLM step".to_string()))?;

        let prompt = self.interpolate_template(prompt_template, context)?;

        // Create messages for LLM
        let messages = vec![
            AiMessage {
                role: "user".to_string(),
                content: prompt,
                name: None,
            }
        ];

        // Call model
        let response = self.models.call_model(model_name, &messages, None).await
            .map_err(|e| KotobaNetError::Execution(format!("LLM call failed: {}", e)))?;

        Ok(serde_json::json!({
            "response": response.content,
            "model": model_name,
            "usage": response.usage
        }))
    }

    /// Execute tool step
    async fn execute_tool_step(&self, step: &ChainStep, context: &serde_json::Value) -> Result<serde_json::Value> {
        let tool_name = step.tool.as_ref()
            .ok_or_else(|| KotobaNetError::Parse("Missing tool name for tool step".to_string()))?;

        // Execute tool with parameters
        let result = self.tools.execute_tool(tool_name, step.parameters.clone()).await
            .map_err(|e| KotobaNetError::Execution(format!("Tool execution failed: {}", e)))?;

        Ok(serde_json::json!({
            "tool": tool_name,
            "result": result.content,
            "success": result.success
        }))
    }

    /// Execute transform step
    async fn execute_transform_step(&self, step: &ChainStep, context: &serde_json::Value) -> Result<serde_json::Value> {
        // Simple template interpolation for now
        // In a real implementation, this could support more complex transformations
        let template = step.config.get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KotobaNetError::Parse("Missing template for transform step".to_string()))?;

        let result = self.interpolate_template(template, context)?;

        Ok(serde_json::json!({
            "transformed": result
        }))
    }

    /// Execute conditional step
    async fn execute_conditional_step(&self, step: &ChainStep, context: &serde_json::Value) -> Result<serde_json::Value> {
        // Evaluate condition and return result
        let condition = step.condition.as_ref()
            .ok_or_else(|| KotobaNetError::Parse("Missing condition for conditional step".to_string()))?;

        let result = self.evaluate_condition(condition, context).await?;

        Ok(serde_json::json!({
            "condition": condition,
            "result": result
        }))
    }

    /// Evaluate condition (simplified implementation)
    async fn evaluate_condition(&self, condition: &str, context: &serde_json::Value) -> Result<bool> {
        // This is a simplified condition evaluator
        // In a real implementation, you might use Jsonnet or a more sophisticated expression evaluator

        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim();

                // Simple equality check
                if let Some(value) = self.get_context_value(context, left) {
                    if let Some(str_val) = value.as_str() {
                        return Ok(str_val == right.trim_matches('"'));
                    } else if let Some(bool_val) = value.as_bool() {
                        return Ok(bool_val == (right == "true"));
                    }
                }
            }
        }

        // Default to true if condition cannot be evaluated
        Ok(true)
    }

    /// Get value from context by path
    fn get_context_value<'a>(&self, context: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
        if let Some(obj) = context.as_object() {
            obj.get(path)
        } else {
            None
        }
    }

    /// Interpolate template with context values
    fn interpolate_template(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        let mut result = template.to_string();

        // Simple template interpolation: replace {{key}} with context values
        if let Some(obj) = context.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        Ok(result)
    }

    /// Get chain by name
    pub fn get_chain(&self, name: &str) -> Option<&AiChain> {
        self.chains.iter().find(|c| c.name == name)
    }

    /// List all available chains
    pub fn list_chains(&self) -> Vec<String> {
        self.chains.iter().map(|c| c.name.clone()).collect()
    }

    /// Create sequential chain builder
    pub fn create_sequential_chain(name: &str, description: &str) -> SequentialChainBuilder {
        SequentialChainBuilder::new(name, description)
    }
}

/// Builder for creating sequential chains
pub struct SequentialChainBuilder {
    chain: AiChain,
}

impl SequentialChainBuilder {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            chain: AiChain {
                name: name.to_string(),
                description: description.to_string(),
                steps: Vec::new(),
                max_iterations: 10,
                timeout_seconds: Some(300),
            }
        }
    }

    /// Add LLM call step
    pub fn add_llm_step(mut self, name: &str, model: &str, prompt_template: &str) -> Self {
        let step = ChainStep {
            name: name.to_string(),
            step_type: StepType::LlmCall,
            tool: None,
            parameters: serde_json::json!({}),
            config: {
                let mut config = HashMap::new();
                config.insert("model".to_string(), serde_json::json!(model));
                config.insert("prompt_template".to_string(), serde_json::json!(prompt_template));
                config
            },
            condition: None,
        };
        self.chain.steps.push(step);
        self
    }

    /// Add tool call step
    pub fn add_tool_step(mut self, name: &str, tool: &str, parameters: serde_json::Value) -> Self {
        let step = ChainStep {
            name: name.to_string(),
            step_type: StepType::ToolCall,
            tool: Some(tool.to_string()),
            parameters,
            config: HashMap::new(),
            condition: None,
        };
        self.chain.steps.push(step);
        self
    }

    /// Add transform step
    pub fn add_transform_step(mut self, name: &str, template: &str) -> Self {
        let step = ChainStep {
            name: name.to_string(),
            step_type: StepType::Transform,
            tool: None,
            parameters: serde_json::json!({}),
            config: {
                let mut config = HashMap::new();
                config.insert("template".to_string(), serde_json::json!(template));
                config
            },
            condition: None,
        };
        self.chain.steps.push(step);
        self
    }

    /// Set maximum iterations
    pub fn max_iterations(mut self, max_iter: u32) -> Self {
        self.chain.max_iterations = max_iter;
        self
    }

    /// Build the chain
    pub fn build(self) -> AiChain {
        self.chain
    }
}
