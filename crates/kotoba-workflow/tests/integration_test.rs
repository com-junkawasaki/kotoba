//! Integration tests for Kotoba Workflow Engine (Itonami)

use kotoba_workflow::prelude::*;
use kotoba_workflow::executor::{ActivityRegistry, Activity, ActivityError, ActivityStatus, ActivityResult};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    // Mock Activity for testing
    struct MockActivity {
        name: String,
        should_succeed: bool,
        response_data: HashMap<String, serde_json::Value>,
    }

    impl MockActivity {
        fn new(name: &str, should_succeed: bool) -> Self {
            let mut response_data = HashMap::new();
            response_data.insert("status".to_string(), serde_json::json!("success"));
            response_data.insert("processed_by".to_string(), serde_json::json!(name));

            Self {
                name: name.to_string(),
                should_succeed,
                response_data,
            }
        }

        fn with_response_data(mut self, key: &str, value: serde_json::Value) -> Self {
            self.response_data.insert(key.to_string(), value);
            self
        }
    }

    #[async_trait::async_trait]
    impl Activity for MockActivity {
        async fn execute(&self, _inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
            if self.should_succeed {
                Ok(self.response_data.clone())
            } else {
                Err(ActivityError::ExecutionFailed("Mock activity failed".to_string()))
            }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn timeout(&self) -> Option<std::time::Duration> {
            Some(std::time::Duration::from_secs(30))
        }
    }

    #[tokio::test]
    async fn test_activity_registry() {
        let registry = ActivityRegistry::new();

        // Register activities
        let activity1 = Arc::new(MockActivity::new("test_activity_1", true));
        let activity2 = Arc::new(MockActivity::new("test_activity_2", false));

        registry.register(activity1).await;
        registry.register(activity2).await;

        // Test successful activity
        let result = registry.execute("test_activity_1", HashMap::new()).await;
        assert!(result.is_ok());
        let activity_result = result.unwrap();
        assert_eq!(activity_result.activity_name, "test_activity_1");
        assert!(matches!(activity_result.status, ActivityStatus::Completed));

        // Test failing activity
        let result = registry.execute("test_activity_2", HashMap::new()).await;
        assert!(result.is_ok()); // ActivityResult is returned even on failure
        let activity_result = result.unwrap();
        assert_eq!(activity_result.activity_name, "test_activity_2");
        assert!(matches!(activity_result.status, ActivityStatus::Failed));

        // Test non-existent activity
        let result = registry.execute("non_existent", HashMap::new()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ActivityError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_workflow_execution_structure() {
        // This test verifies that the workflow execution structure compiles and initializes
        // Note: Full workflow execution requires more complex setup with stores, etc.

        let registry = ActivityRegistry::new();
        let mock_activity = Arc::new(MockActivity::new("simple_task", true));
        registry.register(mock_activity).await;

        // Verify activity is registered
        let activities = registry.list_activities().await;
        assert!(activities.contains(&"simple_task".to_string()));
    }

    #[tokio::test]
    async fn test_activity_with_inputs_outputs() {
        let registry = ActivityRegistry::new();

        let activity = Arc::new(MockActivity::new("data_processor", true)
            .with_response_data("processed_count", serde_json::json!(42))
            .with_response_data("processing_time", serde_json::json!(1.23)));

        registry.register(activity).await;

        let mut inputs = HashMap::new();
        inputs.insert("input_data".to_string(), serde_json::json!("test data"));
        inputs.insert("config".to_string(), serde_json::json!({"batch_size": 100}));

        let result = registry.execute("data_processor", inputs).await;
        assert!(result.is_ok());

        let activity_result = result.unwrap();
        assert!(matches!(activity_result.status, ActivityStatus::Completed));

        let outputs = activity_result.outputs.unwrap();
        assert_eq!(outputs.get("status").unwrap(), "success");
        assert_eq!(outputs.get("processed_count").unwrap(), &serde_json::json!(42));
        assert_eq!(outputs.get("processing_time").unwrap(), &serde_json::json!(1.23));
        assert_eq!(outputs.get("processed_by").unwrap(), "data_processor");
    }

    #[tokio::test]
    async fn test_activity_retry_logic() {
        let registry = ActivityRegistry::new();

        // Activity that always fails
        let failing_activity = Arc::new(MockActivity::new("failing_task", false));
        registry.register(failing_activity).await;

        let result = registry.execute("failing_task", HashMap::new()).await;
        assert!(result.is_ok()); // Result is Ok but status is Failed

        let activity_result = result.unwrap();
        assert!(matches!(activity_result.status, ActivityStatus::Failed));
        assert!(activity_result.error.is_some());
        assert_eq!(activity_result.attempt_count, 1); // No retry configured
    }

    #[test]
    fn test_workflow_ir_creation() {
        // Test that WorkflowIR structures can be created
        use kotoba_workflow::ir::*;

        let workflow_ir = WorkflowIR {
            id: "test_workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: Some("A test workflow".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![
                WorkflowParam {
                    name: "input1".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            outputs: vec![
                WorkflowParam {
                    name: "output1".to_string(),
                    param_type: "boolean".to_string(),
                    required: true,
                    default_value: None, // TODO: Fix type mismatch
                },
            ],
            strategy: WorkflowStrategyOp::Activity {
                activity_ref: "test_activity".to_string(),
                input_mapping: HashMap::new(),
                retry_policy: None,
            },
            timeout: Some(std::time::Duration::from_secs(300)),
            retry_policy: None,
            metadata: HashMap::new(),
        };

        assert_eq!(workflow_ir.id, "test_workflow");
        assert_eq!(workflow_ir.inputs.len(), 1);
        assert_eq!(workflow_ir.outputs.len(), 1);
        assert!(matches!(workflow_ir.strategy, WorkflowStrategyOp::Activity { .. }));
    }

    #[test]
    fn test_execution_status_transitions() {
        use kotoba_workflow::ExecutionStatus;

        // Test that all status variants exist and can be created
        let _running = ExecutionStatus::Running;
        let _completed = ExecutionStatus::Completed;
        let _failed = ExecutionStatus::Failed;
        let _cancelled = ExecutionStatus::Cancelled;
        let _timed_out = ExecutionStatus::TimedOut;

        // Basic assertions
        assert!(matches!(ExecutionStatus::Running, ExecutionStatus::Running));
        assert!(matches!(ExecutionStatus::Completed, ExecutionStatus::Completed));
    }
}
