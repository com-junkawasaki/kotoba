//! Timer activities - stub implementation
// TODO: Implement timer activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct TimerWaitActivity;
impl Default for TimerWaitActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for TimerWaitActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("TimerWaitActivity not implemented")
    }
}

pub struct TimerScheduleActivity;
impl Default for TimerScheduleActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for TimerScheduleActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("TimerScheduleActivity not implemented")
    }
}
