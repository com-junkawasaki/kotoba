//! Validation activities - stub implementation
// TODO: Implement validation activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct JsonValidateActivity;
impl Default for JsonValidateActivity {
    fn default() -> Self { Self }
}

impl Activity for JsonValidateActivity {
    fn name(&self) -> &str {
        "json_validate"
    }
}


pub struct RegexMatchActivity;
impl Default for RegexMatchActivity {
    fn default() -> Self { Self }
}

impl Activity for RegexMatchActivity {
    fn name(&self) -> &str {
        "regex_match"
    }
}

pub struct SchemaValidateActivity;
impl Default for SchemaValidateActivity {
    fn default() -> Self { Self }
}

impl Activity for SchemaValidateActivity {
    fn name(&self) -> &str {
        "schema_validate"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("SchemaValidateActivity not implemented")
    }
}
