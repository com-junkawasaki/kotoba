//! Timer activities - stub implementation
// TODO: Implement timer activities

use async_trait::async_trait;
use kotoba_workflow::Activity;
use kotoba_workflow::executor::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct TimerWaitActivity;
impl Default for TimerWaitActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for TimerWaitActivity {
    fn name(&self) -> &str {
        "timer_wait"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("TimerWaitActivity not implemented")
    }
}


pub struct TimerScheduleActivity;
impl Default for TimerScheduleActivity {
    fn default() -> Self { Self }
}

#[async_trait]
impl Activity for TimerScheduleActivity {
    fn name(&self) -> &str {
        "timer_schedule"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("TimerScheduleActivity not implemented")
    }
}

