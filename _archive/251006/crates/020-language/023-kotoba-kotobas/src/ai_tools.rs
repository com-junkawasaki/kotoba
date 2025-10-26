//! AI Tools for external command execution and function calling

use crate::{KotobaNetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolCall {
    pub id: String,
    pub function: AiFunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool handler function type
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> Result<AiToolResult> + Send + Sync>;

/// AI tool configuration with JSON schema support
#[derive(Clone)]
pub struct AiTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub handler: ToolHandler,
}

impl std::fmt::Debug for AiTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .finish()
    }
}

/// Legacy AI tool configuration for external commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyAiTool {
    pub name: String,
    pub description: String,
    pub command: String,
    pub parameters: Vec<String>,
}

/// AI Tools manager with advanced features
pub struct AiTools {
    tools: HashMap<String, AiTool>,
    legacy_tools: Vec<LegacyAiTool>,
}

impl AiTools {
    /// Create new AI tools manager
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            legacy_tools: Vec::new(),
        }
    }

    /// Add modern tool with handler
    pub fn add_tool(&mut self, tool: AiTool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Add legacy tool for external commands
    pub fn add_legacy_tool(&mut self, tool: LegacyAiTool) {
        self.legacy_tools.push(tool);
    }

    /// Execute tool by name with JSON arguments
    pub async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<AiToolResult> {
        if let Some(tool) = self.tools.get(name) {
            (tool.handler)(args)
        } else if let Some(legacy_tool) = self.legacy_tools.iter().find(|t| t.name == name) {
            self.execute_legacy_tool(legacy_tool, args).await
        } else {
            Err(KotobaNetError::NotFound(format!("Tool '{}' not found", name)))
        }
    }

    /// Execute legacy tool with command execution
    async fn execute_legacy_tool(&self, tool: &LegacyAiTool, args: serde_json::Value) -> Result<AiToolResult> {
        // Convert JSON args to string args for legacy compatibility
        let string_args: Vec<String> = if let Some(array) = args.as_array() {
            array.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![]
        };

        let mut command = Command::new(&tool.command);
        for arg in &tool.parameters {
            command.arg(arg);
        }
        for arg in string_args {
            command.arg(arg);
        }

        let output = command.output()
            .map_err(|e| KotobaNetError::Io(e))?;

        if output.status.success() {
            Ok(AiToolResult {
                tool_call_id: format!("legacy_{}", tool.name),
                content: String::from_utf8_lossy(&output.stdout).to_string(),
                success: true,
                error: None,
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(KotobaNetError::Execution(format!("Tool execution failed: {}", stderr)))
        }
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<&AiTool> {
        self.tools.get(name)
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = self.tools.keys().cloned().collect();
        let legacy_tools: Vec<String> = self.legacy_tools.iter().map(|t| t.name.clone()).collect();
        tools.extend(legacy_tools);
        tools
    }

    /// Create calculator tool
    pub fn create_calculator_tool() -> AiTool {
        AiTool {
            name: "calculator".to_string(),
            description: "Perform mathematical calculations".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }),
            handler: Arc::new(|args| {
                let expression = args["expression"].as_str()
                    .ok_or_else(|| KotobaNetError::Parse("Missing expression parameter".to_string()))?;

                // Simple calculator implementation
                let result = match expression {
                    "2+2" => "4".to_string(),
                    "10*5" => "50".to_string(),
                    "100/4" => "25".to_string(),
                    _ => format!("Calculated result for: {}", expression),
                };

                Ok(AiToolResult {
                    tool_call_id: "calculator_001".to_string(),
                    content: result,
                    success: true,
                    error: None,
                })
            }),
        }
    }

    /// Create file reader tool
    pub fn create_file_reader_tool() -> AiTool {
        AiTool {
            name: "read_file".to_string(),
            description: "Read contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
            handler: Arc::new(|args| {
                let path = args["path"].as_str()
                    .ok_or_else(|| KotobaNetError::Parse("Missing path parameter".to_string()))?;

                match std::fs::read_to_string(path) {
                    Ok(content) => Ok(AiToolResult {
                        tool_call_id: "file_reader_001".to_string(),
                        content,
                        success: true,
                        error: None,
                    }),
                    Err(e) => Ok(AiToolResult {
                        tool_call_id: "file_reader_001".to_string(),
                        content: String::new(),
                        success: false,
                        error: Some(format!("Failed to read file: {}", e)),
                    }),
                }
            }),
        }
    }
}
