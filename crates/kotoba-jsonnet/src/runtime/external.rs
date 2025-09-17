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
pub struct AiModelHandler;

impl AiModelHandler {
    pub fn new() -> Self {
        AiModelHandler
    }
}

impl super::super::evaluator::ExternalHandler for AiModelHandler {
    fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match name {
            "ai.callModel" => self.call_model(args),
            _ => Err(JsonnetError::runtime_error(format!("Unknown AI function: {}", name))),
        }
    }
}

impl AiModelHandler {
    fn call_model(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.len() < 2 {
            return Err(JsonnetError::runtime_error("callModel requires at least two arguments (model, messages)"));
        }

        let model = args[0].as_string()?;
        let messages = args[1].as_array()?;

        // Mock AI model response
        let response_text = format!("This is a mock response from {} model. Received {} messages.", model, messages.len());

        let result = serde_json::json!({
            "model": model,
            "response": response_text,
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            },
            "note": "This is a mock response. Real AI calls require async runtime."
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

impl super::super::evaluator::ExternalHandler for ToolHandler {
    fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match name {
            "tool.execute" => self.execute(args),
            _ => Err(JsonnetError::runtime_error(format!("Unknown tool function: {}", name))),
        }
    }
}

impl ToolHandler {
    fn execute(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if args.is_empty() {
            return Err(JsonnetError::runtime_error("execute requires at least one argument (command)"));
        }

        let command = args[0].as_string()?;
        let cmd_args = if args.len() > 1 {
            args[1].as_array()?.iter().filter_map(|v| v.as_string().ok()).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Mock command execution (for security and simplicity)
        let result = serde_json::json!({
            "command": command,
            "args": cmd_args,
            "stdout": format!("Mock execution of: {} {:?}", command, cmd_args),
            "stderr": "",
            "success": true,
            "exit_code": 0,
            "note": "This is a mock response. Real command execution requires security review."
        });

        Ok(JsonnetValue::from_json_value(&result))
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

impl super::super::evaluator::ExternalHandler for MemoryHandler {
    fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match name {
            "memory.get" => self.get(args),
            "memory.set" => self.set(args),
            _ => Err(JsonnetError::runtime_error(format!("Unknown memory function: {}", name))),
        }
    }
}

impl MemoryHandler {
    fn get(&self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
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

    fn set(&mut self, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
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
