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

impl Activity for RabbitMqPublishActivity {
    fn name(&self) -> &str {
        "rabbitmq_publish"
    }
}

#[async_trait::async_trait]
impl kotoba_workflow::Activity for RabbitMqPublishActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, kotoba_workflow::ActivityError> {
        todo!("RabbitMqPublishActivity not implemented")
    }
}

pub struct RabbitMqConsumeActivity;
impl Default for RabbitMqConsumeActivity {
    fn default() -> Self { Self }
}

impl Activity for RabbitMqConsumeActivity {
    fn name(&self) -> &str {
        "rabbitmq_consume"
    }
}

#[async_trait::async_trait]
impl kotoba_workflow::Activity for RabbitMqConsumeActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, kotoba_workflow::ActivityError> {
        todo!("RabbitMqConsumeActivity not implemented")
    }
}
