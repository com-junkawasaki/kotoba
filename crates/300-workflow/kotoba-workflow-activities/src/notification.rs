//! Notification activities - stub implementation
// TODO: Implement notification activities

use async_trait::async_trait;
use kotoba_workflow::Activity;
use kotoba_workflow::executor::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct WebhookNotifyActivity;
impl Default for WebhookNotifyActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for WebhookNotifyActivity {
    fn name(&self) -> &str {
        "webhook_notify"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("WebhookNotifyActivity not implemented")
    }
}


pub struct SlackNotifyActivity;
impl Default for SlackNotifyActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for SlackNotifyActivity {
    fn name(&self) -> &str {
        "slack_notify"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("SlackNotifyActivity not implemented")
    }
}

