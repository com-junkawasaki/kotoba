//! Notification activities - stub implementation
// TODO: Implement notification activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct WebhookNotifyActivity;
impl Default for WebhookNotifyActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for WebhookNotifyActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("WebhookNotifyActivity not implemented")
    }
}

pub struct SlackNotifyActivity;
impl Default for SlackNotifyActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for SlackNotifyActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("SlackNotifyActivity not implemented")
    }
}
