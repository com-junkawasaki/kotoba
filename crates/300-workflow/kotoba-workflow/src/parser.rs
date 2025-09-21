//! Serverless Workflow Parser
//!
//! Parses Serverless Workflow DSL (https://serverlessworkflow.io/) from JSON/YAML
//! and converts to WorkflowIR for execution.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::result::Result as StdResult;

use crate::spec::{RunDefinition, ServerlessWorkflow};
use crate::ir::{ActivityIR, WorkflowIR, ActivityImplementation};
use kotoba_errors::WorkflowError;

/// Serverless Workflow Parser
pub struct ServerlessWorkflowParser;

impl ServerlessWorkflowParser {
    /// 新しいパーサーを作成
    pub fn new() -> Self {
        Self
    }

    /// JSONからServerlessWorkflowをパース
    pub fn parse_json(&self, json: JsonValue) -> StdResult<ServerlessWorkflow, WorkflowError> {
        serde_json::from_value(json)
            .map_err(|e| WorkflowError::InvalidDefinition(format!("JSON parsing failed: {}", e)))
    }

    /// YAML文字列からServerlessWorkflowをパース
    pub fn parse_yaml(&self, yaml_content: &str) -> StdResult<ServerlessWorkflow, WorkflowError> {
        serde_yaml::from_str(yaml_content)
            .map_err(|e| WorkflowError::InvalidDefinition(format!("YAML parsing failed: {}", e)))
    }

    /// ServerlessWorkflowをWorkflowIRに変換
    pub fn convert_to_workflow_ir(&self, sw: ServerlessWorkflow) -> StdResult<WorkflowIR, WorkflowError> {
        let mut workflow_ir = WorkflowIR {
            id: format!("{}.{}.{}", sw.document.namespace, sw.document.name, sw.document.version),
            name: sw.document.name,
            version: sw.document.version,
            description: sw.document.description,
            inputs: Vec::new(),
            outputs: Vec::new(),
            strategy: crate::ir::WorkflowStrategyOp::Seq { strategies: Vec::new() },
            activities: Vec::new(),
            timeout: None,
            retry_policy: None,
            metadata: HashMap::new(),
        };

        // Convert workflow steps to activities
        for (index, step) in sw.r#do.iter().enumerate() {
            let activity = self.convert_step_to_activity(step, index)?;
            workflow_ir.activities.push(activity);
        }

        // Add metadata
        if let Some(title) = sw.document.title {
            workflow_ir.metadata.insert("title".to_string(), kotoba_core::types::Value::String(title));
        }
        if let Some(summary) = sw.document.summary {
            workflow_ir.metadata.insert("summary".to_string(), kotoba_core::types::Value::String(summary));
        }
        workflow_ir.metadata.insert("dsl".to_string(), kotoba_core::types::Value::String(sw.document.dsl));
        workflow_ir.metadata.insert("namespace".to_string(), kotoba_core::types::Value::String(sw.document.namespace));

        Ok(workflow_ir)
    }

    /// Convert workflow step to activity
    fn convert_step_to_activity(&self, step: &crate::spec::WorkflowStep, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        match step {
            crate::spec::WorkflowStep::Call { call, .. } => {
                self.convert_call_step(call, index)
            }
            crate::spec::WorkflowStep::Emit { emit, .. } => {
                self.convert_emit_step(emit, index)
            }
            crate::spec::WorkflowStep::Listen { listen, .. } => {
                self.convert_listen_step(listen, index)
            }
            crate::spec::WorkflowStep::Wait { wait, .. } => {
                self.convert_wait_step(wait, index)
            }
            crate::spec::WorkflowStep::Run { run, .. } => {
                self.convert_run_step(run, index)
            }
            crate::spec::WorkflowStep::Switch { switch, .. } => {
                self.convert_switch_step(switch, index)
            }
            crate::spec::WorkflowStep::For { r#for, .. } => {
                self.convert_for_step(r#for, index)
            }
            crate::spec::WorkflowStep::Fork { fork, .. } => {
                self.convert_fork_step(fork, index)
            }
            crate::spec::WorkflowStep::Try { r#try, .. } => {
                self.convert_try_step(r#try, index)
            }
            crate::spec::WorkflowStep::Raise { raise, .. } => {
                self.convert_raise_step(raise, index)
            }
            crate::spec::WorkflowStep::Set { set, .. } => {
                self.convert_set_step(set, index)
            }
        }
    }

    fn convert_call_step(&self, call: &crate::spec::CallDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        let activity_type = match call {
            crate::spec::CallDefinition::Http { .. } => "http_call",
            crate::spec::CallDefinition::Grpc { .. } => "grpc_call",
            crate::spec::CallDefinition::OpenApi { .. } => "openapi_call",
            crate::spec::CallDefinition::AsyncApi { .. } => "asyncapi_call",
        };

        let _config = serde_json::to_value(call)
            .map_err(|e| WorkflowError::InvalidDefinition(format!("Call config serialization failed: {}", e)))?;

        Ok(ActivityIR {
            name: format!("Call Step {}", index),
            description: Some(format!("Serverless Workflow {} step", activity_type)),
            inputs: Vec::new(), // TODO: 実際の入力パラメータを解析
            outputs: Vec::new(), // TODO: 実際の出力パラメータを解析
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Http {
                url: "http://localhost".to_string(), // TODO: 実際のURLを解析
                method: "GET".to_string(), // TODO: 実際のメソッドを解析
                headers: HashMap::new(),
            },
        })
    }

    fn convert_emit_step(&self, _emit: &crate::spec::EmitDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Emit Step {}", index),
            description: Some("Serverless Workflow emit step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "emit_event".to_string(),
            },
        })
    }

    fn convert_listen_step(&self, _listen: &crate::spec::ListenDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Listen Step {}", index),
            description: Some("Serverless Workflow listen step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "listen_event".to_string(),
            },
        })
    }

    fn convert_wait_step(&self, _wait: &crate::spec::WaitDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Wait Step {}", index),
            description: Some("Serverless Workflow wait step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "wait".to_string(),
            },
        })
    }

    fn convert_run_step(&self, run: &RunDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        let function_name = match run {
            crate::spec::RunDefinition::Container { .. } => "run_container",
            crate::spec::RunDefinition::Script { .. } => "run_script",
            crate::spec::RunDefinition::Workflow { .. } => "run_workflow",
        };

        Ok(ActivityIR {
            name: format!("Run Step {}", index),
            description: Some("Serverless Workflow run step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: function_name.to_string(),
            },
        })
    }

    fn convert_switch_step(&self, _switch: &[crate::spec::SwitchCase], index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Switch Step {}", index),
            description: Some("Serverless Workflow switch step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "switch".to_string(),
            },
        })
    }

    fn convert_for_step(&self, _for_def: &crate::spec::ForDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("For Step {}", index),
            description: Some("Serverless Workflow for loop step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "for_loop".to_string(),
            },
        })
    }

    fn convert_fork_step(&self, _fork: &crate::spec::ForkDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Fork Step {}", index),
            description: Some("Serverless Workflow fork step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "fork".to_string(),
            },
        })
    }

    fn convert_try_step(&self, _try_def: &crate::spec::TryDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Try Step {}", index),
            description: Some("Serverless Workflow try-catch step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "try_catch".to_string(),
            },
        })
    }

    fn convert_raise_step(&self, _raise: &crate::spec::RaiseDefinition, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Raise Step {}", index),
            description: Some("Serverless Workflow raise error step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "raise_error".to_string(),
            },
        })
    }

    fn convert_set_step(&self, _set: &HashMap<String, serde_json::Value>, index: usize) -> StdResult<ActivityIR, WorkflowError> {
        Ok(ActivityIR {
            name: format!("Set Step {}", index),
            description: Some("Serverless Workflow set variable step".to_string()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            timeout: None,
            retry_policy: None,
            implementation: ActivityImplementation::Function {
                function_name: "set_variable".to_string(),
            },
        })
    }
}
