//! Email activities - stub implementation
// TODO: Implement email activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct SmtpSendActivity;
impl Default for SmtpSendActivity {
    fn default() -> Self { Self }
}

impl Activity for SmtpSendActivity {
    fn name(&self) -> &str {
        "smtp_send"
    }
}

#[async_trait::async_trait]
impl kotoba_workflow::Activity for SmtpSendActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, kotoba_workflow::ActivityError> {
        todo!("SmtpSendActivity not implemented")
    }
}
