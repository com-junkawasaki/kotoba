//! Serverless Workflow JSON parser
//!
//! Parses Serverless Workflow specification JSON documents into WorkflowDocument structures.
//! Based on https://serverlessworkflow.io/specification

use serde_json::{Value as JsonValue, Map};
use std::collections::HashMap;

use crate::{
    WorkflowDocument,
    WorkflowState,
    CallHttpState,
    CallGrpcState,
    CallOpenApiState,
    CallAsyncApiState,
    EmitState,
    ListenState,
    RunScriptState,
    RunContainerState,
    RunWorkflowState,
    SwitchState,
    SwitchCase,
    ForState,
    ForkState,
    CompletionCondition,
    TryState,
    CatchDefinition,
    WaitState,
    WaitDefinition,
    RaiseState,
    ErrorDefinition,
    SetState,
    EventDefinition,
    Authentication,
    OutputMapping,
    Schema,
    Timeout,
    GrpcService,
    OpenApiDocument,
    AsyncApiDocument,
    AsyncApiMessage,
    EventFilter,
    ListenDefinition,
    ScriptDefinition,
    ContainerDefinition,
    WorkflowReference,
    WorkflowError,
};

/// Serverless Workflow parser
#[derive(Debug)]
pub struct WorkflowParser;

impl WorkflowParser {
    /// Parse a Serverless Workflow JSON document
    pub fn parse(json: &str) -> Result<WorkflowDocument, WorkflowError> {
        let value: JsonValue = serde_json::from_str(json)
            .map_err(WorkflowError::Serialization)?;

        Self::parse_value(&value)
    }

    /// Parse a JSON value into a WorkflowDocument
    pub fn parse_value(value: &JsonValue) -> Result<WorkflowDocument, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::Validation("Root must be an object".to_string()))?;

        // Extract required fields
        let dsl = Self::extract_string(obj, "dsl")?;
        let namespace = Self::extract_string(obj, "namespace")?;
        let name = Self::extract_string(obj, "name")?;
        let version = Self::extract_string(obj, "version")?;

        // Extract optional fields
        let title = Self::extract_optional_string(obj, "title");
        let summary = Self::extract_optional_string(obj, "summary");
        let description = Self::extract_optional_string(obj, "description");

        // Extract schema definitions
        let input = Self::extract_optional_schema(obj, "input")?;
        let output = Self::extract_optional_schema(obj, "output")?;
        let timeout = Self::extract_optional_timeout(obj, "timeout")?;

        // Extract workflow states - this is the core "do" array
        let do_states = Self::extract_workflow_states(obj, "do")?;

        Ok(WorkflowDocument {
            dsl,
            namespace,
            name,
            version,
            title,
            summary,
            description,
            input,
            output,
            timeout,
            r#do: do_states,
        })
    }

    /// Extract workflow states from the "do" array
    fn extract_workflow_states(obj: &Map<String, JsonValue>, key: &str) -> Result<Vec<WorkflowState>, WorkflowError> {
        let do_array = obj.get(key)
            .and_then(|v| v.as_array())
            .ok_or_else(|| WorkflowError::Validation(format!("'{}' must be an array of workflow states", key)))?;

        let mut states = Vec::new();
        for (index, state_value) in do_array.iter().enumerate() {
            let state = Self::parse_workflow_state(state_value, index)?;
            states.push(state);
        }

        Ok(states)
    }

    /// Parse a single workflow state
    fn parse_workflow_state(value: &JsonValue, index: usize) -> Result<WorkflowState, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::Validation(format!("State {} must be an object", index)))?;

        let state_type = obj.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkflowError::Validation(format!("State {} missing 'type' field", index)))?;

        match state_type {
            "callHttp" => Ok(WorkflowState::CallHttp(Self::parse_call_http_state(obj)?)),
            "callGrpc" => Ok(WorkflowState::CallGrpc(Self::parse_call_grpc_state(obj)?)),
            "callOpenApi" => Ok(WorkflowState::CallOpenApi(Self::parse_call_openapi_state(obj)?)),
            "callAsyncApi" => Ok(WorkflowState::CallAsyncApi(Self::parse_call_asyncapi_state(obj)?)),
            "emit" => Ok(WorkflowState::Emit(Self::parse_emit_state(obj)?)),
            "listen" => Ok(WorkflowState::Listen(Self::parse_listen_state(obj)?)),
            "runScript" => Ok(WorkflowState::RunScript(Self::parse_run_script_state(obj)?)),
            "runContainer" => Ok(WorkflowState::RunContainer(Self::parse_run_container_state(obj)?)),
            "runWorkflow" => Ok(WorkflowState::RunWorkflow(Self::parse_run_workflow_state(obj)?)),
            "switch" => Ok(WorkflowState::Switch(Self::parse_switch_state(obj)?)),
            "for" => Ok(WorkflowState::For(Self::parse_for_state(obj)?)),
            "fork" => Ok(WorkflowState::Fork(Self::parse_fork_state(obj)?)),
            "try" => Ok(WorkflowState::Try(Self::parse_try_state(obj)?)),
            "wait" => Ok(WorkflowState::Wait(Self::parse_wait_state(obj)?)),
            "raise" => Ok(WorkflowState::Raise(Self::parse_raise_state(obj)?)),
            "set" => Ok(WorkflowState::Set(Self::parse_set_state(obj)?)),
            _ => Err(WorkflowError::Validation(format!("Unknown state type: {}", state_type))),
        }
    }

    /// Parse HTTP call state
    fn parse_call_http_state(obj: &Map<String, JsonValue>) -> Result<CallHttpState, WorkflowError> {
        Ok(CallHttpState {
            name: Self::extract_optional_string(obj, "name"),
            method: Self::extract_string(obj, "method")?,
            endpoint: Self::extract_string(obj, "endpoint")?,
            headers: Self::extract_optional_string_map(obj, "headers")?,
            body: obj.get("body").cloned(),
            query: Self::extract_optional_string_map(obj, "query")?,
            auth: Self::extract_optional_auth(obj, "auth")?,
            output: Self::extract_optional_output_mapping(obj, "output")?,
            transition: Self::extract_optional_string(obj, "transition"),
        })
    }

    /// Parse gRPC call state
    fn parse_call_grpc_state(obj: &Map<String, JsonValue>) -> Result<CallGrpcState, WorkflowError> {
        let service = Self::extract_grpc_service(obj, "service")?;
        let method = Self::extract_string(obj, "method")?;

        Ok(CallGrpcState {
            name: Self::extract_optional_string(obj, "name"),
            service,
            method,
            arguments: Self::extract_optional_value_map(obj, "arguments")?,
            output: Self::extract_optional_output_mapping(obj, "output")?,
            transition: Self::extract_optional_string(obj, "transition"),
        })
    }

    /// Parse set variables state (simplified)
    fn parse_set_state(obj: &Map<String, JsonValue>) -> Result<SetState, WorkflowError> {
        let variables = obj.get("variables")
            .and_then(|v| v.as_object())
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        Ok(SetState {
            name: Self::extract_optional_string(obj, "name"),
            variables,
            transition: Self::extract_optional_string(obj, "transition"),
        })
    }

    /// Parse wait state
    fn parse_wait_state(obj: &Map<String, JsonValue>) -> Result<WaitState, WorkflowError> {
        let wait = Self::extract_wait_definition(obj, "wait")?;

        Ok(WaitState {
            name: Self::extract_optional_string(obj, "name"),
            wait,
            transition: Self::extract_optional_string(obj, "transition"),
        })
    }

    /// Parse raise error state
    fn parse_raise_state(obj: &Map<String, JsonValue>) -> Result<RaiseState, WorkflowError> {
        let raise = Self::extract_error_definition(obj, "raise")?;

        Ok(RaiseState {
            name: Self::extract_optional_string(obj, "name"),
            raise,
        })
    }

    // Placeholder implementations for other state types
    fn parse_call_openapi_state(_obj: &Map<String, JsonValue>) -> Result<CallOpenApiState, WorkflowError> {
        Err(WorkflowError::Validation("OpenAPI calls not yet implemented".to_string()))
    }

    fn parse_call_asyncapi_state(_obj: &Map<String, JsonValue>) -> Result<CallAsyncApiState, WorkflowError> {
        Err(WorkflowError::Validation("AsyncAPI calls not yet implemented".to_string()))
    }

    fn parse_emit_state(_obj: &Map<String, JsonValue>) -> Result<EmitState, WorkflowError> {
        Err(WorkflowError::Validation("Event emission not yet implemented".to_string()))
    }

    fn parse_listen_state(_obj: &Map<String, JsonValue>) -> Result<ListenState, WorkflowError> {
        Err(WorkflowError::Validation("Event listening not yet implemented".to_string()))
    }

    fn parse_run_script_state(_obj: &Map<String, JsonValue>) -> Result<RunScriptState, WorkflowError> {
        Err(WorkflowError::Validation("Script execution not yet implemented".to_string()))
    }

    fn parse_run_container_state(_obj: &Map<String, JsonValue>) -> Result<RunContainerState, WorkflowError> {
        Err(WorkflowError::Validation("Container execution not yet implemented".to_string()))
    }

    fn parse_run_workflow_state(_obj: &Map<String, JsonValue>) -> Result<RunWorkflowState, WorkflowError> {
        Err(WorkflowError::Validation("Sub-workflow execution not yet implemented".to_string()))
    }

    fn parse_switch_state(_obj: &Map<String, JsonValue>) -> Result<SwitchState, WorkflowError> {
        Err(WorkflowError::Validation("Switch statements not yet implemented".to_string()))
    }

    fn parse_for_state(_obj: &Map<String, JsonValue>) -> Result<ForState, WorkflowError> {
        Err(WorkflowError::Validation("For loops not yet implemented".to_string()))
    }

    fn parse_fork_state(_obj: &Map<String, JsonValue>) -> Result<ForkState, WorkflowError> {
        Err(WorkflowError::Validation("Parallel execution not yet implemented".to_string()))
    }

    fn parse_try_state(_obj: &Map<String, JsonValue>) -> Result<TryState, WorkflowError> {
        Err(WorkflowError::Validation("Try-catch not yet implemented".to_string()))
    }

    // Helper methods for extracting values
    fn extract_string(obj: &Map<String, JsonValue>, key: &str) -> Result<String, WorkflowError> {
        obj.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| WorkflowError::Validation(format!("Missing or invalid '{}' field", key)))
    }

    fn extract_optional_string(obj: &Map<String, JsonValue>, key: &str) -> Option<String> {
        obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    fn extract_optional_string_map(obj: &Map<String, JsonValue>, key: &str) -> Result<Option<HashMap<String, String>>, WorkflowError> {
        if let Some(value) = obj.get(key) {
            let map = value.as_object()
                .ok_or_else(|| WorkflowError::Validation(format!("'{}' must be an object", key)))?
                .iter()
                .map(|(k, v)| {
                    let str_val = v.as_str()
                        .ok_or_else(|| WorkflowError::Validation(format!("'{}' values must be strings", key)))?;
                    Ok((k.clone(), str_val.to_string()))
                })
                .collect::<Result<HashMap<String, String>, WorkflowError>>()?;
            Ok(Some(map))
        } else {
            Ok(None)
        }
    }

    fn extract_optional_value_map(obj: &Map<String, JsonValue>, key: &str) -> Result<Option<HashMap<String, JsonValue>>, WorkflowError> {
        if let Some(value) = obj.get(key) {
            let map = value.as_object()
                .ok_or_else(|| WorkflowError::Validation(format!("'{}' must be an object", key)))?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Ok(Some(map))
        } else {
            Ok(None)
        }
    }

    fn extract_optional_schema(obj: &Map<String, JsonValue>, key: &str) -> Result<Option<Schema>, WorkflowError> {
        if let Some(value) = obj.get(key) {
            // For now, just wrap the JSON value
            Ok(Some(Schema::Inline(value.clone())))
        } else {
            Ok(None)
        }
    }

    fn extract_optional_timeout(obj: &Map<String, JsonValue>, key: &str) -> Result<Option<Timeout>, WorkflowError> {
        if let Some(value) = obj.get(key) {
            if let Some(seconds) = value.as_u64() {
                Ok(Some(Timeout::Seconds(seconds)))
            } else if let Some(duration_str) = value.as_str() {
                Ok(Some(Timeout::Duration(duration_str.to_string())))
            } else {
                Err(WorkflowError::Validation(format!("Invalid timeout format for '{}'", key)))
            }
        } else {
            Ok(None)
        }
    }

    fn extract_optional_auth(obj: &Map<String, JsonValue>, _key: &str) -> Result<Option<Authentication>, WorkflowError> {
        // Authentication parsing not implemented yet
        Ok(None)
    }

    fn extract_optional_output_mapping(obj: &Map<String, JsonValue>, _key: &str) -> Result<Option<OutputMapping>, WorkflowError> {
        // Output mapping parsing not implemented yet
        Ok(None)
    }

    fn extract_grpc_service(obj: &Map<String, JsonValue>, key: &str) -> Result<GrpcService, WorkflowError> {
        let service_obj = obj.get(key)
            .and_then(|v| v.as_object())
            .ok_or_else(|| WorkflowError::Validation(format!("'{}' must be an object", key)))?;

        Ok(GrpcService {
            name: Self::extract_string(service_obj, "name")?,
            host: Self::extract_string(service_obj, "host")?,
            port: service_obj.get("port").and_then(|v| v.as_u64()).map(|p| p as u16),
            tls: service_obj.get("tls").and_then(|v| v.as_bool()),
        })
    }

    fn extract_wait_definition(obj: &Map<String, JsonValue>, key: &str) -> Result<WaitDefinition, WorkflowError> {
        let wait_obj = obj.get(key)
            .and_then(|v| v.as_object())
            .ok_or_else(|| WorkflowError::Validation(format!("'{}' must be an object", key)))?;

        if let Some(seconds) = wait_obj.get("seconds").and_then(|v| v.as_u64()) {
            Ok(WaitDefinition::Seconds { seconds })
        } else if let Some(duration) = wait_obj.get("duration").and_then(|v| v.as_str()) {
            Ok(WaitDefinition::Duration { duration: duration.to_string() })
        } else {
            Err(WorkflowError::Validation(format!("Invalid wait definition in '{}'", key)))
        }
    }

    fn extract_error_definition(obj: &Map<String, JsonValue>, key: &str) -> Result<ErrorDefinition, WorkflowError> {
        let error_obj = obj.get(key)
            .and_then(|v| v.as_object())
            .ok_or_else(|| WorkflowError::Validation(format!("'{}' must be an object", key)))?;

        Ok(ErrorDefinition {
            r#type: Self::extract_string(error_obj, "type")?,
            status: error_obj.get("status").and_then(|v| v.as_u64()).map(|s| s as u16),
            title: Self::extract_optional_string(error_obj, "title"),
            detail: Self::extract_optional_string(error_obj, "detail"),
        })
    }
}

/// Convenience function to parse Serverless Workflow JSON
pub fn parse_workflow(json: &str) -> Result<WorkflowDocument, WorkflowError> {
    WorkflowParser::parse(json)
}

/// Convenience function to parse Serverless Workflow from JSON value
pub fn parse_workflow_value(value: &JsonValue) -> Result<WorkflowDocument, WorkflowError> {
    WorkflowParser::parse_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_workflow() {
        let json = r#"
        {
            "dsl": "1.0.0",
            "namespace": "examples",
            "name": "simple-workflow",
            "version": "1.0.0",
            "do": [
                {
                    "type": "set",
                    "variables": {
                        "message": "Hello, World!"
                    }
                },
                {
                    "type": "wait",
                    "wait": {
                        "seconds": 5
                    }
                }
            ]
        }
        "#;

        let result = parse_workflow(json);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "simple-workflow");
        assert_eq!(workflow.r#do.len(), 2);
    }

    #[test]
    fn test_parse_http_call_workflow() {
        let json = r#"
        {
            "dsl": "1.0.0",
            "namespace": "examples",
            "name": "http-workflow",
            "version": "1.0.0",
            "do": [
                {
                    "type": "callHttp",
                    "method": "GET",
                    "endpoint": "https://api.example.com/data"
                }
            ]
        }
        "#;

        let result = parse_workflow(json);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "http-workflow");
        assert_eq!(workflow.r#do.len(), 1);

        match &workflow.r#do[0] {
            WorkflowState::CallHttp(http_state) => {
                assert_eq!(http_state.method, "GET");
                assert_eq!(http_state.endpoint, "https://api.example.com/data");
            }
            _ => panic!("Expected CallHttp state"),
        }
    }

    #[test]
    fn test_parse_invalid_workflow() {
        let json = r#"
        {
            "dsl": "1.0.0",
            "namespace": "examples",
            "name": "invalid-workflow",
            "version": "1.0.0",
            "do": "not-an-array"
        }
        "#;

        let result = parse_workflow(json);
        assert!(result.is_err());
    }
}
