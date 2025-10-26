//! AI Agent Parser for .manimani files

use crate::{KotobaNetError, Result};
use serde::{Deserialize, Serialize};

/// Parsed AI agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAgentConfig {
    pub name: String,
    pub description: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub tools: Vec<AiToolConfig>,
}

/// AI tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolConfig {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// AI Agent Parser
pub struct AiParser;

impl AiParser {
    /// Parse AI agent configuration from JSON
    pub fn parse_agent_config(json: &str) -> Result<AiAgentConfig> {
        serde_json::from_str(json)
            .map_err(|e| KotobaNetError::Parse(format!("Failed to parse AI agent config: {}", e)))
    }

    /// Parse AI tool configuration from JSON
    pub fn parse_tool_config(json: &str) -> Result<AiToolConfig> {
        serde_json::from_str(json)
            .map_err(|e| KotobaNetError::Parse(format!("Failed to parse AI tool config: {}", e)))
    }
}
