//! AI activities - stub implementation
// TODO: Implement AI activities

use async_trait::async_trait;
use kotoba_workflow::Activity;
use kotoba_workflow::executor::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct AiActivity;
impl Default for AiActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for AiActivity {
    fn name(&self) -> &str {
        "ai"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("AiActivity not implemented")
    }
}

