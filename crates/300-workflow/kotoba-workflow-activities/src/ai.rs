//! AI activities - stub implementation
// TODO: Implement AI activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct AiActivity;
impl Default for AiActivity {
    fn default() -> Self { Self }
}

impl Activity for AiActivity {
    fn name(&self) -> &str {
        "ai"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("AiActivity not implemented")
    }
}

