//! Kotoba Workflow - Pure Kernel & Effects Shell Architecture
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate provides workflow orchestration with clear separation:
//!
//! - **Pure Kernel**: Workflow definitions, execution planning, dependency resolution
//! - **Effects Shell**: Task execution, state persistence, external integrations

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Pure workflow definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow ID
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow tasks
    pub tasks: Vec<Task>,
    /// Workflow triggers
    pub triggers: Vec<Trigger>,
}

impl Workflow {
    /// Create a new workflow
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            tasks: vec![],
            triggers: vec![],
        }
    }

    /// Add a task to the workflow
    pub fn with_task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    /// Add a trigger to the workflow
    pub fn with_trigger(mut self, trigger: Trigger) -> Self {
        self.triggers.push(trigger);
        self
    }
}

/// Pure task definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Task ID (unique within workflow)
    pub id: String,
    /// Task name
    pub name: String,
    /// Task type
    pub task_type: TaskType,
    /// Task configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Task dependencies
    pub dependencies: Vec<String>,
    /// Task timeout (seconds)
    pub timeout: Option<u64>,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
}

impl Task {
    /// Create a new task
    pub fn new(id: impl Into<String>, name: impl Into<String>, task_type: TaskType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            task_type,
            config: HashMap::new(),
            dependencies: vec![],
            timeout: None,
            retry_policy: None,
        }
    }

    /// Add dependency
    pub fn depends_on(mut self, task_id: impl Into<String>) -> Self {
        self.dependencies.push(task_id.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }
}

/// Task types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    /// HTTP request
    HttpRequest,
    /// Database query
    DatabaseQuery,
    /// Custom function
    CustomFunction,
    /// Wait/delay
    Wait,
    /// Conditional branch
    Conditional,
    /// Parallel execution
    Parallel,
}

/// Retry policy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_multiplier: f64,
    pub initial_delay: u64,
}

/// Workflow triggers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trigger {
    /// Schedule-based trigger
    Schedule(String), // cron expression
    /// Event-based trigger
    Event(String), // event name
    /// Manual trigger
    Manual,
}

/// Pure execution plan
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionPlan {
    /// Workflow to execute
    pub workflow: Workflow,
    /// Execution context
    pub context: ExecutionContext,
    /// Ordered task execution sequence
    pub task_sequence: Vec<String>,
    /// Task dependencies graph
    pub dependencies: HashMap<String, Vec<String>>,
}

impl ExecutionPlan {
    /// Create execution plan from workflow (pure function)
    pub fn from_workflow(workflow: Workflow, input_data: HashMap<String, serde_json::Value>) -> Result<Self, WorkflowError> {
        let planner = PureWorkflowPlanner::new();
        planner.create_plan(workflow, input_data)
    }

    /// Validate execution plan (pure function)
    pub fn validate(&self) -> Result<(), WorkflowError> {
        // Check that all dependencies are satisfied
        for task_id in &self.task_sequence {
            if let Some(deps) = self.dependencies.get(task_id) {
                for dep in deps {
                    if !self.task_sequence.contains(dep) {
                        return Err(WorkflowError::MissingDependency(dep.clone()));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Execution context
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionContext {
    /// Execution ID
    pub execution_id: String,
    /// Input data
    pub input_data: HashMap<String, serde_json::Value>,
    /// Execution start time
    pub start_time: u64,
}

/// Pure workflow planner - no side effects
pub struct PureWorkflowPlanner;

impl PureWorkflowPlanner {
    /// Create a new planner
    pub fn new() -> Self {
        Self
    }

    /// Create execution plan from workflow (pure function)
    pub fn create_plan(&self, workflow: Workflow, input_data: HashMap<String, serde_json::Value>) -> Result<ExecutionPlan, WorkflowError> {
        // Validate workflow
        self.validate_workflow(&workflow)?;

        // Create task dependency graph
        let dependencies = self.build_dependency_graph(&workflow.tasks)?;

        // Perform topological sort to get execution order
        let task_sequence = self.topological_sort(&dependencies)?;

        let context = ExecutionContext {
            execution_id: format!("exec_{}", workflow.id),
            input_data,
            start_time: Self::current_timestamp(),
        };

        let plan = ExecutionPlan {
            workflow,
            context,
            task_sequence,
            dependencies,
        };

        plan.validate()?;
        Ok(plan)
    }

    /// Validate workflow definition (pure function)
    fn validate_workflow(&self, workflow: &Workflow) -> Result<(), WorkflowError> {
        if workflow.id.is_empty() {
            return Err(WorkflowError::InvalidWorkflow("Empty workflow ID".to_string()));
        }

        let task_ids: HashSet<_> = workflow.tasks.iter().map(|t| &t.id).collect();

        for task in &workflow.tasks {
            if task.id.is_empty() {
                return Err(WorkflowError::InvalidTask(format!("Task '{}' has empty ID", task.name)));
            }

            // Check dependencies exist
            for dep in &task.dependencies {
                if !task_ids.contains(dep) {
                    return Err(WorkflowError::InvalidTask(format!("Task '{}' depends on non-existent task '{}'", task.id, dep)));
                }
            }
        }

        Ok(())
    }

    /// Build dependency graph (pure function)
    fn build_dependency_graph(&self, tasks: &[Task]) -> Result<HashMap<String, Vec<String>>, WorkflowError> {
        let mut graph = HashMap::new();

        for task in tasks {
            graph.insert(task.id.clone(), task.dependencies.clone());
        }

        Ok(graph)
    }

    /// Perform topological sort (pure function)
    fn topological_sort(&self, dependencies: &HashMap<String, Vec<String>>) -> Result<Vec<String>, WorkflowError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for task_id in dependencies.keys() {
            if !visited.contains(task_id) {
                self.visit(task_id, dependencies, &mut visited, &mut visiting, &mut result)?;
            }
        }

        result.reverse(); // Reverse to get correct execution order
        Ok(result)
    }

    /// Visit node in topological sort (pure function)
    fn visit(
        &self,
        task_id: &str,
        dependencies: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), WorkflowError> {
        if visiting.contains(task_id) {
            return Err(WorkflowError::CircularDependency(task_id.to_string()));
        }

        if visited.contains(task_id) {
            return Ok(());
        }

        visiting.insert(task_id.to_string());

        if let Some(deps) = dependencies.get(task_id) {
            for dep in deps {
                self.visit(dep, dependencies, visited, visiting, result)?;
            }
        }

        visiting.remove(task_id);
        visited.insert(task_id.to_string());
        result.push(task_id.to_string());

        Ok(())
    }

    /// Get current timestamp
    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// Effects Shell workflow executor
pub struct WorkflowExecutor;

impl WorkflowExecutor {
    /// Create a new executor
    pub fn new() -> Self {
        Self
    }

    /// Execute workflow plan (effects: task execution, I/O)
    pub async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<ExecutionResult, WorkflowError> {
        let mut task_results = HashMap::new();

        for task_id in &plan.task_sequence {
            // Check if dependencies are satisfied
            if let Some(deps) = plan.dependencies.get(task_id) {
                for dep in deps {
                    if !task_results.contains_key(dep) {
                        return Err(WorkflowError::DependencyNotSatisfied(dep.clone()));
                    }
                }
            }

            // Execute task (effects)
            let result = self.execute_task(task_id, &plan.workflow, &plan.context).await?;
            task_results.insert(task_id.clone(), result);
        }

        Ok(ExecutionResult {
            execution_id: plan.context.execution_id.clone(),
            task_results,
            success: true,
            execution_time_ms: Self::current_timestamp() - plan.context.start_time,
        })
    }

    /// Execute individual task (effects: external operations)
    async fn execute_task(&self, task_id: &str, workflow: &Workflow, context: &ExecutionContext) -> Result<TaskResult, WorkflowError> {
        let task = workflow.tasks.iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| WorkflowError::TaskNotFound(task_id.to_string()))?;

        // In real implementation, this would execute the actual task
        // For now, simulate execution based on task type
        match task.task_type {
            TaskType::HttpRequest => {
                // Simulate HTTP request
                Ok(TaskResult::Success(serde_json::json!({"status": "ok"})))
            }
            TaskType::DatabaseQuery => {
                // Simulate DB query
                Ok(TaskResult::Success(serde_json::json!({"rows": []})))
            }
            TaskType::Wait => {
                // Simulate wait
                Ok(TaskResult::Success(serde_json::json!({"waited": true})))
            }
            _ => Ok(TaskResult::Success(serde_json::json!({"executed": true}))),
        }
    }

    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub task_results: HashMap<String, TaskResult>,
    pub success: bool,
    pub execution_time_ms: u64,
}

/// Task execution result
#[derive(Debug, Clone)]
pub enum TaskResult {
    Success(serde_json::Value),
    Failure(String),
    Skipped,
}

/// Workflow errors
#[derive(Debug, Clone)]
pub enum WorkflowError {
    InvalidWorkflow(String),
    InvalidTask(String),
    MissingDependency(String),
    CircularDependency(String),
    TaskNotFound(String),
    DependencyNotSatisfied(String),
    ExecutionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_workflow_planning() {
        let workflow = Workflow::new("test_workflow", "Test Workflow")
            .with_task(
                Task::new("task_a", "Task A", TaskType::CustomFunction)
            )
            .with_task(
                Task::new("task_b", "Task B", TaskType::CustomFunction)
                    .depends_on("task_a")
            );

        let input_data = HashMap::new();
        let plan = ExecutionPlan::from_workflow(workflow, input_data).unwrap();

        // Task A should come before Task B
        assert_eq!(plan.task_sequence, vec!["task_a", "task_b"]);

        // Check dependencies
        assert_eq!(plan.dependencies.get("task_b"), Some(&vec!["task_a".to_string()]));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let workflow = Workflow::new("circular", "Circular Workflow")
            .with_task(
                Task::new("task_a", "Task A", TaskType::CustomFunction)
                    .depends_on("task_b")
            )
            .with_task(
                Task::new("task_b", "Task B", TaskType::CustomFunction)
                    .depends_on("task_a")
            );

        let input_data = HashMap::new();
        let result = ExecutionPlan::from_workflow(workflow, input_data);
        assert!(matches!(result, Err(WorkflowError::CircularDependency(_))));
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let workflow = Workflow::new("simple", "Simple Workflow")
            .with_task(Task::new("hello", "Hello Task", TaskType::CustomFunction));

        let input_data = HashMap::new();
        let plan = ExecutionPlan::from_workflow(workflow, input_data).unwrap();

        let executor = WorkflowExecutor::new();
        let result = executor.execute_plan(&plan).await.unwrap();

        assert!(result.success);
        assert_eq!(result.execution_id, "exec_simple");
        assert!(result.task_results.contains_key("hello"));
    }
}
