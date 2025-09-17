//! AI Tools for external command execution and function calling

use crate::{KotobaNetError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// AI tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTool {
    pub name: String,
    pub description: String,
    pub command: String,
    pub parameters: Vec<String>,
}

/// AI Tools manager
pub struct AiTools {
    tools: Vec<AiTool>,
}

impl AiTools {
    /// Create new AI tools manager
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
        }
    }

    /// Add tool
    pub fn add_tool(&mut self, tool: AiTool) {
        self.tools.push(tool);
    }

    /// Execute tool by name
    pub async fn execute_tool(&self, name: &str, args: &[String]) -> Result<String> {
        if let Some(tool) = self.tools.iter().find(|t| t.name == name) {
            let mut command = Command::new(&tool.command);
            for arg in &tool.parameters {
                command.arg(arg);
            }
            for arg in args {
                command.arg(arg);
            }

            let output = command.output()
                .map_err(|e| KotobaNetError::Io(e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(KotobaNetError::Execution(format!("Tool execution failed: {}", stderr)))
            }
        } else {
            Err(KotobaNetError::NotFound(format!("Tool '{}' not found", name)))
        }
    }
}
