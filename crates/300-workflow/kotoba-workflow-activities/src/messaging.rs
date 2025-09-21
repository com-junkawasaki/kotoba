//! Messaging activities - stub implementation
// TODO: Implement messaging activities

use async_trait::async_trait;
use kotoba_workflow::Activity;
use kotoba_workflow::executor::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct RabbitMqPublishActivity;
impl Default for RabbitMqPublishActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for RabbitMqPublishActivity {
    fn name(&self) -> &str {
        "rabbitmq_publish"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("RabbitMqPublishActivity not implemented")
    }
}


pub struct RabbitMqConsumeActivity;
impl Default for RabbitMqConsumeActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for RabbitMqConsumeActivity {
    fn name(&self) -> &str {
        "rabbitmq_consume"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("RabbitMqConsumeActivity not implemented")
    }
}

