//! External function handlers for Jsonnet evaluation

use crate::error::{JsonnetError, Result};
use crate::value::JsonnetValue;
use crate::eval::Context;
use std::collections::HashMap;
use std::process::Command;

/// HTTP client handler for ai.httpGet, ai.httpPost functions
pub struct HttpHandler {
    // For synchronous implementation, we'll use a simple approach
    // In a real implementation, this would need async runtime integration
}

impl HttpHandler {
    pub fn new() -> Self {
        HttpHandler {}
    }
}

impl super::super::evaluator::ExternalHandler for HttpHandler {
    fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match name {
            "ai.httpGet" => self.http_get(args),
            "ai.httpPost" => self.http_post(args),
            _ => Err(JsonnetError::runtime_error(format!("Unknown HTTP function: {}", name))),
        }
    }
}

impl HttpHandler {
    fn http_get(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.is_empty() {
            return Err(JsonnetError::runtime_error("httpGet requires at least one argument (URL)"));
        }

        let url = args[0].as_string()?;

        // For now, return a mock response
        // In a real implementation, this would make actual HTTP requests
        let result = serde_json::json!({
            "url": url,
            "method": "GET",
            "status": 200,
            "body": "Mock HTTP response",
            "success": true,
            "note": "This is a mock response. Real HTTP calls require async runtime."
        });

        Ok(JsonnetValue::from_json_value(&result))
    }

    fn http_post(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.len() < 2 {
            return Err(JsonnetError::runtime_error("httpPost requires at least two arguments (URL, body)"));
        }

        let url = args[0].as_string()?;
        let body = &args[1];

        // For now, return a mock response
        let result = serde_json::json!({
            "url": url,
            "method": "POST",
            "body": body,
            "status": 200,
            "response": "Mock HTTP response",
            "success": true,
            "note": "This is a mock response. Real HTTP calls require async runtime."
        });

        Ok(JsonnetValue::from_json_value(&result))
    }
}

/// AI model handler for ai.callModel function
pub struct AiModelHandler {
    http_handler: HttpHandler,
}

impl AiModelHandler {
    pub fn new() -> Self {
        AiModelHandler {
            http_handler: HttpHandler::new(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalHandler for AiModelHandler {
    fn namespace(&self) -> &str {
        "ai"
    }

    async fn call(&mut self, function: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match function {
            "callModel" => self.call_model(args).await,
            _ => Err(JsonnetError::runtime_error(format!("Unknown AI function: {}", function))),
        }
    }
}

impl AiModelHandler {
    async fn call_model(&mut self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.len() < 2 {
            return Err(JsonnetError::runtime_error("callModel requires at least two arguments (model, messages)"));
        }

        let model = args[0].as_string()?;
        let messages = args[1].as_array()?;
        let options = if args.len() > 2 {
            args[2].as_object()?.clone()
        } else {
            HashMap::new()
        };

        // For now, simulate different AI models
        // In real implementation, this would call actual AI APIs
        match model.as_str() {
            "gpt-3.5-turbo" | "gpt-4" => self.call_openai_like_model(&model, messages, &options).await,
            "claude-2" | "claude-3" => self.call_anthropic_like_model(&model, messages, &options).await,
            _ => {
                let result = serde_json::json!({
                    "model": model,
                    "response": format!("Mock response from {}", model),
                    "usage": { "tokens": 42 }
                });
                Ok(JsonnetValue::from_json_value(&result))
            }
        }
    }

    async fn call_openai_like_model(&mut self, model: &str, messages: &Vec<JsonnetValue>, options: &HashMap<String, JsonnetValue>) -> Result<JsonnetValue> {
        // Mock OpenAI API call
        let response_text = "This is a mock response from an OpenAI-like model.".to_string();

        let result = serde_json::json!({
            "model": model,
            "response": response_text,
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });

        Ok(JsonnetValue::from_json_value(&result))
    }

    async fn call_anthropic_like_model(&mut self, model: &str, messages: &Vec<JsonnetValue>, options: &HashMap<String, JsonnetValue>) -> Result<JsonnetValue> {
        // Mock Anthropic API call
        let response_text = "This is a mock response from an Anthropic-like model.".to_string();

        let result = serde_json::json!({
            "model": model,
            "response": response_text,
            "usage": {
                "input_tokens": 15,
                "output_tokens": 25
            }
        });

        Ok(JsonnetValue::from_json_value(&result))
    }
}

/// Tool execution handler for tool.execute function
pub struct ToolHandler;

impl ToolHandler {
    pub fn new() -> Self {
        ToolHandler
    }
}

#[async_trait::async_trait]
impl ExternalHandler for ToolHandler {
    fn namespace(&self) -> &str {
        "tool"
    }

    async fn call(&mut self, function: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match function {
            "execute" => self.execute(args).await,
            _ => Err(JsonnetError::runtime_error(format!("Unknown tool function: {}", function))),
        }
    }
}

impl ToolHandler {
    async fn execute(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.is_empty() {
            return Err(JsonnetError::runtime_error("execute requires at least one argument (command)"));
        }

        let command = args[0].as_string()?;
        let cmd_args = if args.len() > 1 {
            args[1].as_array()?.iter().filter_map(|v| v.as_string().ok()).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Execute command
        match Command::new(&command).args(&cmd_args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success = output.status.success();

                let result = serde_json::json!({
                    "command": command,
                    "args": cmd_args,
                    "stdout": stdout,
                    "stderr": stderr,
                    "success": success,
                    "exit_code": output.status.code()
                });

                Ok(JsonnetValue::from_json_value(&result))
            }
            Err(e) => {
                let result = serde_json::json!({
                    "command": command,
                    "args": cmd_args,
                    "stdout": "",
                    "stderr": e.to_string(),
                    "success": false,
                    "exit_code": null
                });

                Ok(JsonnetValue::from_json_value(&result))
            }
        }
    }
}

/// Memory handler for memory.get, memory.set functions
pub struct MemoryHandler {
    storage: HashMap<String, JsonnetValue>,
}

impl MemoryHandler {
    pub fn new() -> Self {
        MemoryHandler {
            storage: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalHandler for MemoryHandler {
    fn namespace(&self) -> &str {
        "memory"
    }

    async fn call(&mut self, function: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match function {
            "get" => self.get(args).await,
            "set" => self.set(args).await,
            _ => Err(JsonnetError::runtime_error(format!("Unknown memory function: {}", function))),
        }
    }
}

impl MemoryHandler {
    async fn get(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.is_empty() {
            return Err(JsonnetError::runtime_error("get requires one argument (key)"));
        }

        let key = args[0].as_string()?;
        let value = self.storage.get(&key).cloned();

        let result = serde_json::json!({
            "key": key,
            "value": value,
            "found": value.is_some()
        });

        Ok(JsonnetValue::from_json_value(&result))
    }

    async fn set(&mut self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.len() < 2 {
            return Err(JsonnetError::runtime_error("set requires two arguments (key, value)"));
        }

        let key = args[0].as_string()?;
        let value = args[1].clone();

        self.storage.insert(key.clone(), value.clone());

        let result = serde_json::json!({
            "key": key,
            "value": value,
            "success": true
        });

        Ok(JsonnetValue::from_json_value(&result))
    }
}
