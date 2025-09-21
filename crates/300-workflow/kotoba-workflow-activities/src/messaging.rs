//! Messaging activities - stub implementation
// TODO: Implement messaging activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct RabbitMqPublishActivity;
impl Default for RabbitMqPublishActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for RabbitMqPublishActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("RabbitMqPublishActivity not implemented")
    }
}

pub struct RabbitMqConsumeActivity;
impl Default for RabbitMqConsumeActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for RabbitMqConsumeActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("RabbitMqConsumeActivity not implemented")
    }
}
