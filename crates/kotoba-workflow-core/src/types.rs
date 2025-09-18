//! Additional type definitions for workflow core

use serde::{Deserialize, Serialize};

/// Activity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityConfig {
    pub timeout_seconds: Option<u64>,
    pub retry_count: Option<u32>,
    pub retry_delay_seconds: Option<u64>,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(300), // 5 minutes
            retry_count: Some(3),
            retry_delay_seconds: Some(1),
        }
    }
}

/// Workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl WorkflowMetadata {
    pub fn new(id: String, name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }
}

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub execution_id: String,
    pub workflow_id: String,
    pub activity_id: Option<String>,
    pub input: serde_json::Value,
    pub variables: serde_json::Value,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl ExecutionContext {
    pub fn new(execution_id: String, workflow_id: String, input: serde_json::Value) -> Self {
        Self {
            execution_id,
            workflow_id,
            activity_id: None,
            input,
            variables: serde_json::Value::Object(serde_json::Map::new()),
            start_time: chrono::Utc::now(),
        }
    }
}

/// Activity result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityResult {
    pub activity_id: String,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub execution_id: String,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub activities: Vec<ActivityResult>,
}
