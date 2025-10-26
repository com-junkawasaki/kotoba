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

/// Message for AI model conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// AI model response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub usage: Option<AiUsage>,
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// AI Models manager with actual API integration
pub struct AiModels {
    configs: Vec<AiModelConfig>,
    http_client: reqwest::Client,
}

impl AiModels {
    /// Create new AI models manager
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            http_client: reqwest::Client::new(),
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

    /// Call AI model with messages
    pub async fn call_model(&self, model_name: &str, messages: &[AiMessage], config_override: Option<&AiModelConfig>) -> Result<AiResponse> {
        let config = config_override.or_else(|| self.get_model(model_name))
            .ok_or_else(|| KotobaNetError::Config(format!("Model '{}' not found", model_name)))?;

        match config.provider {
            AiProvider::OpenAI => self.call_openai(config, messages).await,
            AiProvider::Anthropic => self.call_anthropic(config, messages).await,
            AiProvider::Google => self.call_google(config, messages).await,
        }
    }

    /// Call OpenAI API
    async fn call_openai(&self, config: &AiModelConfig, messages: &[AiMessage]) -> Result<AiResponse> {
        let url = "https://api.openai.com/v1/chat/completions";

        let request_body = serde_json::json!({
            "model": config.model_name,
            "messages": messages,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
        });

        let response = self.http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| KotobaNetError::Network(format!("OpenAI API request failed: {}", e)))?;

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| KotobaNetError::Parse(format!("Failed to parse OpenAI response: {}", e)))?;

        if let Some(error) = response_json.get("error") {
            return Err(KotobaNetError::Api(format!("OpenAI API error: {}", error)));
        }

        let choice = response_json["choices"][0].clone();
        let content = choice["message"]["content"].as_str()
            .unwrap_or("No response content")
            .to_string();

        let usage = if let Some(usage_obj) = response_json.get("usage") {
            Some(AiUsage {
                prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
        } else {
            None
        };

        Ok(AiResponse {
            content,
            usage,
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }

    /// Call Anthropic API (placeholder)
    async fn call_anthropic(&self, config: &AiModelConfig, messages: &[AiMessage]) -> Result<AiResponse> {
        // TODO: Implement Anthropic API integration
        Err(KotobaNetError::Api("Anthropic API integration not yet implemented".to_string()))
    }

    /// Call Google AI API (placeholder)
    async fn call_google(&self, config: &AiModelConfig, messages: &[AiMessage]) -> Result<AiResponse> {
        // TODO: Implement Google AI API integration
        Err(KotobaNetError::Api("Google AI API integration not yet implemented".to_string()))
    }
}
