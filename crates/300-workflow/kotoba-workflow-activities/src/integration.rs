//! Integration activities - stub implementation
// TODO: Implement integration activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct IntegrationActivity;
impl Default for IntegrationActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for IntegrationActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("IntegrationActivity not implemented")
    }
}
