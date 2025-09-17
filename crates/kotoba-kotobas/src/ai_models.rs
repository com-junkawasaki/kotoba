//! AI Models integration for OpenAI, Anthropic, Google AI

use crate::{KotobaNetError, Result};
use serde::{Deserialize, Serialize};

/// Supported AI model providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    Google,
}

/// AI model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelConfig {
    pub provider: AiProvider,
    pub model_name: String,
    pub api_key: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

/// AI Models manager
pub struct AiModels {
    configs: Vec<AiModelConfig>,
}

impl AiModels {
    /// Create new AI models manager
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
        }
    }

    /// Add model configuration
    pub fn add_model(&mut self, config: AiModelConfig) {
        self.configs.push(config);
    }

    /// Get model configuration by name
    pub fn get_model(&self, name: &str) -> Option<&AiModelConfig> {
        self.configs.iter().find(|c| c.model_name == name)
    }
}
