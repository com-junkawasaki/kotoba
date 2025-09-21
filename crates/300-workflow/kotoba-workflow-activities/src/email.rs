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

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("SmtpSendActivity not implemented")
    }
}

