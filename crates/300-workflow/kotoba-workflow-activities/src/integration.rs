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

impl Activity for IntegrationActivity {
    fn name(&self) -> &str {
        "integration"
    }
}

