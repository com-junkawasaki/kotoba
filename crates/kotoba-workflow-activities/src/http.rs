//! HTTP Activity Implementations
//!
//! Pre-built activities for HTTP operations including GET, POST, PUT, DELETE, PATCH.

use async_trait::async_trait;
use kotoba_workflow::{Activity, ActivityError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::ActivityConfig;

/// HTTP GET Activity
#[derive(Debug, Clone)]
pub struct HttpGetActivity {
    config: ActivityConfig,
    client: reqwest::Client,
}

impl Default for HttpGetActivity {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpGetActivity {
    pub fn with_config(config: ActivityConfig) -> Self {
        let timeout = config.config.get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Activity for HttpGetActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        let url = inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'url' parameter".to_string()))?;

        let headers = inputs.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let query_params = inputs.get("query")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build request
        let mut request = self.client.get(url);

        // Add headers
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(&key, value_str);
            }
        }

        // Add query parameters
        let mut url_with_query = url.to_string();
        if !query_params.is_empty() {
            let query_string = serde_urlencoded::to_string(&query_params)
                .map_err(|e| ActivityError::InvalidInput(format!("Invalid query parameters: {}", e)))?;
            url_with_query = format!("{}?{}", url, query_string);
            request = self.client.get(&url_with_query);
        }

        // Execute request
        let response = request.send().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Try to parse JSON response
        let response_text = response.text().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let response_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json
        } else {
            serde_json::Value::String(response_text)
        };

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(status_code));
        outputs.insert("headers".to_string(), serde_json::json!(headers));
        outputs.insert("body".to_string(), response_body);

        Ok(outputs)
    }

    fn name(&self) -> &str {
        "http_get"
    }
}

/// HTTP POST Activity
#[derive(Debug, Clone)]
pub struct HttpPostActivity {
    config: ActivityConfig,
    client: reqwest::Client,
}

impl Default for HttpPostActivity {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpPostActivity {
    pub fn with_config(config: ActivityConfig) -> Self {
        let timeout = config.config.get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Activity for HttpPostActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        let url = inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'url' parameter".to_string()))?;

        let body = inputs.get("body")
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'body' parameter".to_string()))?;

        let headers = inputs.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build request
        let mut request = self.client.post(url);

        // Add headers
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(&key, value_str);
            }
        }

        // Set body
        request = request.json(body);

        // Execute request
        let response = request.send().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Try to parse JSON response
        let response_text = response.text().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let response_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json
        } else {
            serde_json::Value::String(response_text)
        };

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(status_code));
        outputs.insert("headers".to_string(), serde_json::json!(headers));
        outputs.insert("body".to_string(), response_body);

        Ok(outputs)
    }

    fn name(&self) -> &str {
        "http_post"
    }
}

/// HTTP PUT Activity
#[derive(Debug, Clone)]
pub struct HttpPutActivity {
    config: ActivityConfig,
    client: reqwest::Client,
}

impl Default for HttpPutActivity {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpPutActivity {
    pub fn with_config(config: ActivityConfig) -> Self {
        let timeout = config.config.get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Activity for HttpPutActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        let url = inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'url' parameter".to_string()))?;

        let body = inputs.get("body")
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'body' parameter".to_string()))?;

        let headers = inputs.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build request
        let mut request = self.client.put(url);

        // Add headers
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(&key, value_str);
            }
        }

        // Set body
        request = request.json(body);

        // Execute request
        let response = request.send().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Try to parse JSON response
        let response_text = response.text().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let response_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json
        } else {
            serde_json::Value::String(response_text)
        };

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(status_code));
        outputs.insert("headers".to_string(), serde_json::json!(headers));
        outputs.insert("body".to_string(), response_body);

        Ok(outputs)
    }

    fn name(&self) -> &str {
        "http_put"
    }
}

/// HTTP DELETE Activity
#[derive(Debug, Clone)]
pub struct HttpDeleteActivity {
    config: ActivityConfig,
    client: reqwest::Client,
}

impl Default for HttpDeleteActivity {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpDeleteActivity {
    pub fn with_config(config: ActivityConfig) -> Self {
        let timeout = config.config.get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Activity for HttpDeleteActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        let url = inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'url' parameter".to_string()))?;

        let headers = inputs.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build request
        let mut request = self.client.delete(url);

        // Add headers
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(&key, value_str);
            }
        }

        // Execute request
        let response = request.send().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Try to parse JSON response
        let response_text = response.text().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let response_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json
        } else {
            serde_json::Value::String(response_text)
        };

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(status_code));
        outputs.insert("headers".to_string(), serde_json::json!(headers));
        outputs.insert("body".to_string(), response_body);

        Ok(outputs)
    }

    fn name(&self) -> &str {
        "http_delete"
    }
}

/// HTTP PATCH Activity
#[derive(Debug, Clone)]
pub struct HttpPatchActivity {
    config: ActivityConfig,
    client: reqwest::Client,
}

impl Default for HttpPatchActivity {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpPatchActivity {
    pub fn with_config(config: ActivityConfig) -> Self {
        let timeout = config.config.get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Activity for HttpPatchActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        let url = inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'url' parameter".to_string()))?;

        let body = inputs.get("body")
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'body' parameter".to_string()))?;

        let headers = inputs.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build request
        let mut request = self.client.patch(url);

        // Add headers
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(&key, value_str);
            }
        }

        // Set body
        request = request.json(body);

        // Execute request
        let response = request.send().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Try to parse JSON response
        let response_text = response.text().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let response_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json
        } else {
            serde_json::Value::String(response_text)
        };

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(status_code));
        outputs.insert("headers".to_string(), serde_json::json!(headers));
        outputs.insert("body".to_string(), response_body);

        Ok(outputs)
    }

    fn name(&self) -> &str {
        "http_patch"
    }
}

/// HTTP Request Activity (Generic)
#[derive(Debug, Clone)]
pub struct HttpRequestActivity {
    config: ActivityConfig,
    client: reqwest::Client,
}

impl Default for HttpRequestActivity {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpRequestActivity {
    pub fn with_config(config: ActivityConfig) -> Self {
        let timeout = config.config.get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Activity for HttpRequestActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        let url = inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActivityError::InvalidInput("Missing 'url' parameter".to_string()))?;

        let method = inputs.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");

        let headers = inputs.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let query_params = inputs.get("query")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let body = inputs.get("body");

        // Build request based on method
        let mut request_builder = match method.to_uppercase().as_str() {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            "PATCH" => self.client.patch(url),
            "HEAD" => self.client.head(url),
            "OPTIONS" => self.client.request(reqwest::Method::OPTIONS, url),
            _ => return Err(ActivityError::InvalidInput(format!("Unsupported HTTP method: {}", method))),
        };

        // Add headers
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request_builder = request_builder.header(&key, value_str);
            }
        }

        // Add query parameters
        if !query_params.is_empty() {
            let query_string = serde_urlencoded::to_string(&query_params)
                .map_err(|e| ActivityError::InvalidInput(format!("Invalid query parameters: {}", e)))?;
            let url_with_query = format!("{}?{}", url, query_string);
            request_builder = match method.to_uppercase().as_str() {
                "GET" => self.client.get(&url_with_query),
                "POST" => self.client.post(&url_with_query),
                "PUT" => self.client.put(&url_with_query),
                "DELETE" => self.client.delete(&url_with_query),
                "PATCH" => self.client.patch(&url_with_query),
                "HEAD" => self.client.head(&url_with_query),
                "OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url_with_query),
                _ => unreachable!(),
            };
        }

        // Set body for methods that support it
        if let Some(body) = body {
            if matches!(method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
                request_builder = request_builder.json(body);
            }
        }

        // Execute request
        let response = request_builder.send().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Try to parse JSON response
        let response_text = response.text().await
            .map_err(|e| ActivityError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let response_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json
        } else {
            serde_json::Value::String(response_text)
        };

        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(status_code));
        outputs.insert("headers".to_string(), serde_json::json!(headers));
        outputs.insert("body".to_string(), response_body);

        Ok(outputs)
    }

    fn name(&self) -> &str {
        "http_request"
    }
}
