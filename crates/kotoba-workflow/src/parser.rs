//! KotobaファイルからのWorkflowIRパーサー
//!
//! Jsonnetパーサーを利用して.kotobaファイルからWorkflowIRを構築します。

use serde_json::{Value as JsonValue, Map};
use std::collections::HashMap;
use std::time::Duration;
use kotoba_jsonnet::{Evaluator, JsonnetValue};
use kotoba_core::types::Value;
use kotoba_core::prelude::*;

use crate::ir::*;
use crate::WorkflowError;

/// Kotobaワークフローパーサー
pub struct WorkflowParser {
    evaluator: Evaluator,
}

impl WorkflowParser {
    /// 新しいパーサーを作成
    pub fn new() -> Self {
        Self {
            evaluator: Evaluator::new(),
        }
    }

    /// .kotobaファイルからWorkflowIRをパース
    pub fn parse_file(&mut self, file_path: &str) -> Result<WorkflowIR, WorkflowError> {
        // Jsonnetファイルを評価
        let result = self.evaluator.evaluate_file(file_path, file_path)
            .map_err(|e| WorkflowError::InvalidDefinition(format!("Jsonnet evaluation failed: {}", e)))?;

        // JsonnetValueをserde_json::Valueに変換
        let json_value = jsonnet_to_json(result)?;

        // ワークフロー定義を抽出
        self.parse_workflow_definition(&json_value)
    }

    /// JSON文字列からWorkflowIRをパース
    pub fn parse_string(&mut self, content: &str) -> Result<WorkflowIR, WorkflowError> {
        // Jsonnet文字列を評価
        let result = self.evaluator.evaluate_file(content, "<string>")
            .map_err(|e| WorkflowError::InvalidDefinition(format!("Jsonnet evaluation failed: {}", e)))?;

        // JsonnetValueをserde_json::Valueに変換
        let json_value = jsonnet_to_json(result)?;

        // ワークフロー定義を抽出
        self.parse_workflow_definition(&json_value)
    }

    /// JSON ValueからWorkflowIRを構築
    fn parse_workflow_definition(&self, value: &JsonValue) -> Result<WorkflowIR, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Root must be an object".to_string()))?;

        let workflow = obj.get("workflow")
            .ok_or_else(|| WorkflowError::InvalidDefinition("Missing 'workflow' field".to_string()))?
            .as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("'workflow' must be an object".to_string()))?;

        let id = self.extract_string(workflow, "id")?;
        let name = self.extract_string(workflow, "name")?;
        let description = workflow.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
        let version = self.extract_string(workflow, "version")?;

        let inputs = self.parse_workflow_params(workflow, "inputs")?;
        let outputs = self.parse_workflow_params(workflow, "outputs")?;

        let strategy = self.parse_workflow_strategy(workflow)?;

        let timeout = workflow.get("timeout")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_duration(s).ok());

        let retry_policy = workflow.get("retry_policy")
            .and_then(|v| self.parse_retry_policy(v).ok())
            .flatten();

        let metadata = workflow.get("metadata")
            .and_then(|v| v.as_object())
            .map(|obj| self.parse_metadata(obj))
            .unwrap_or_default();

        Ok(WorkflowIR {
            id,
            name,
            description,
            version,
            inputs,
            outputs,
            strategy,
            timeout,
            retry_policy,
            metadata,
        })
    }

    /// ワークフローパラメータをパース
    fn parse_workflow_params(&self, obj: &Map<String, JsonValue>, key: &str) -> Result<Vec<WorkflowParam>, WorkflowError> {
        let params = obj.get(key)
            .and_then(|v| v.as_array())
            .ok_or_else(|| WorkflowError::InvalidDefinition(format!("'{}' must be an array", key)))?;

        params.iter()
            .map(|param| self.parse_workflow_param(param))
            .collect::<Result<Vec<_>, _>>()
    }

    /// 単一のワークフローパラメータをパース
    fn parse_workflow_param(&self, value: &JsonValue) -> Result<WorkflowParam, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Parameter must be an object".to_string()))?;

        let name = self.extract_string(obj, "name")?;
        let param_type = self.extract_string(obj, "type")?;
        let required = obj.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
        let default_value = obj.get("default_value").and_then(|v| self.parse_value(v).ok());

        Ok(WorkflowParam {
            name,
            param_type,
            required,
            default_value,
        })
    }

    /// ワークフロー戦略をパース
    fn parse_workflow_strategy(&self, obj: &Map<String, JsonValue>) -> Result<WorkflowStrategyOp, WorkflowError> {
        let strategy = obj.get("strategy")
            .ok_or_else(|| WorkflowError::InvalidDefinition("Missing 'strategy' field".to_string()))?;

        self.parse_strategy_op(strategy)
    }

    /// 戦略演算子をパース
    fn parse_strategy_op(&self, value: &JsonValue) -> Result<WorkflowStrategyOp, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Strategy must be an object".to_string()))?;

        let op = obj.get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkflowError::InvalidDefinition("Strategy must have 'op' field".to_string()))?;

        match op {
            "once" => {
                let rule = obj.get("rule")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'once' strategy must have 'rule' field".to_string()))?;

                Ok(WorkflowStrategyOp::Basic {
                    strategy: StrategyOp::Once { rule: rule.to_string() },
                })
            }

            "exhaust" => {
                let rule = obj.get("rule")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'exhaust' strategy must have 'rule' field".to_string()))?;

                let order = obj.get("order")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "topdown" => Some(Order::TopDown),
                        "bottomup" => Some(Order::BottomUp),
                        "fair" => Some(Order::Fair),
                        _ => None,
                    })
                    .unwrap_or(Order::TopDown);

                let measure = obj.get("measure").and_then(|v| v.as_str()).map(|s| s.to_string());

                Ok(WorkflowStrategyOp::Basic {
                    strategy: StrategyOp::Exhaust {
                        rule: rule.to_string(),
                        order,
                        measure,
                    },
                })
            }

            "while" => {
                let rule = obj.get("rule")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'while' strategy must have 'rule' field".to_string()))?;

                let pred = obj.get("pred")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'while' strategy must have 'pred' field".to_string()))?;

                let order = obj.get("order")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "topdown" => Some(Order::TopDown),
                        "bottomup" => Some(Order::BottomUp),
                        "fair" => Some(Order::Fair),
                        _ => None,
                    })
                    .unwrap_or(Order::TopDown);

                Ok(WorkflowStrategyOp::Basic {
                    strategy: StrategyOp::While {
                        rule: rule.to_string(),
                        pred: pred.to_string(),
                        order,
                    },
                })
            }

            "choice" => {
                let strategies = obj.get("strategies")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'choice' strategy must have 'strategies' array".to_string()))?;

                let strategies = strategies.iter()
                    .map(|s| self.parse_basic_strategy(s))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(WorkflowStrategyOp::Basic {
                    strategy: StrategyOp::Choice {
                        strategies: strategies.into_iter().map(Box::new).collect(),
                    },
                })
            }

            "priority" => {
                let strategies = obj.get("strategies")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'priority' strategy must have 'strategies' array".to_string()))?;

                let strategies = strategies.iter()
                    .map(|s| self.parse_prioritized_strategy(s))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(WorkflowStrategyOp::Basic {
                    strategy: StrategyOp::Priority { strategies },
                })
            }

            "seq" => {
                let strategies = obj.get("strategies")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'seq' strategy must have 'strategies' array".to_string()))?;

                let strategies = strategies.iter()
                    .map(|s| self.parse_strategy_op(s))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(WorkflowStrategyOp::Seq { strategies: strategies.into_iter().map(Box::new).collect() })
            }

            "parallel" => {
                let branches = obj.get("branches")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'parallel' strategy must have 'branches' array".to_string()))?;

                let branches = branches.iter()
                    .map(|b| self.parse_strategy_op(b))
                    .collect::<Result<Vec<_>, _>>()?;

                let completion_condition = obj.get("completion_condition")
                    .and_then(|v| self.parse_completion_condition(v))
                    .unwrap_or(CompletionCondition::All);

                Ok(WorkflowStrategyOp::Parallel {
                    branches: branches.into_iter().map(Box::new).collect(),
                    completion_condition,
                })
            }

            "decision" => {
                let conditions = obj.get("conditions")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'decision' strategy must have 'conditions' array".to_string()))?;

                let conditions = conditions.iter()
                    .map(|c| self.parse_decision_branch(c))
                    .collect::<Result<Vec<_>, _>>()?;

                let default_branch = obj.get("default_branch")
                    .and_then(|v| self.parse_strategy_op(v).ok())
                    .map(Box::new);

                Ok(WorkflowStrategyOp::Decision {
                    conditions,
                    default_branch,
                })
            }

            "wait" => {
                let condition = obj.get("condition")
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'wait' strategy must have 'condition' field".to_string()))?;

                let condition = self.parse_wait_condition(condition)?;

                let timeout = obj.get("timeout")
                    .and_then(|v| v.as_str())
                    .and_then(|s| parse_duration(s).ok());

                Ok(WorkflowStrategyOp::Wait { condition, timeout })
            }

            "saga" => {
                let main_flow = obj.get("main_flow")
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'saga' strategy must have 'main_flow' field".to_string()))?;

                let main_flow = self.parse_strategy_op(main_flow)?;

                let compensation = obj.get("compensation")
                    .ok_or_else(|| WorkflowError::InvalidDefinition("'saga' strategy must have 'compensation' field".to_string()))?;

                let compensation = self.parse_strategy_op(compensation)?;

                Ok(WorkflowStrategyOp::Saga {
                    main_flow: Box::new(main_flow),
                    compensation: Box::new(compensation),
                })
            }

            "activity" => {
                let activity_ref = self.extract_string(obj, "activity_ref")?;

                let input_mapping = obj.get("input_mapping")
                    .and_then(|v| v.as_object())
                    .map(|obj| self.parse_input_mapping(obj))
                    .unwrap_or_default();

                let retry_policy = obj.get("retry_policy")
                    .and_then(|v| self.parse_retry_policy(v).ok())
                    .flatten();

                Ok(WorkflowStrategyOp::Activity {
                    activity_ref,
                    input_mapping,
                    retry_policy,
                })
            }

            "subworkflow" => {
                let workflow_ref = self.extract_string(obj, "workflow_ref")?;

                let input_mapping = obj.get("input_mapping")
                    .and_then(|v| v.as_object())
                    .map(|obj| self.parse_input_mapping(obj))
                    .unwrap_or_default();

                Ok(WorkflowStrategyOp::SubWorkflow {
                    workflow_ref,
                    input_mapping,
                })
            }

            _ => Err(WorkflowError::InvalidDefinition(format!("Unknown strategy operation: {}", op))),
        }
    }

    /// 完了条件をパース
    fn parse_completion_condition(&self, value: &JsonValue) -> Option<CompletionCondition> {
        value.as_str().and_then(|s| match s {
            "all" => Some(CompletionCondition::All),
            "any" => Some(CompletionCondition::Any),
            "at_least" => Some(CompletionCondition::AtLeast(1)), // デフォルト値
            _ => None,
        })
    }

    /// 決定分岐をパース
    fn parse_decision_branch(&self, value: &JsonValue) -> Result<DecisionBranch, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Decision branch must be an object".to_string()))?;

        let condition = self.extract_string(obj, "condition")?;
        let branch = obj.get("branch")
            .ok_or_else(|| WorkflowError::InvalidDefinition("Decision branch must have 'branch' field".to_string()))?;

        let branch = self.parse_strategy_op(branch)?;

        Ok(DecisionBranch {
            condition,
            branch: Box::new(branch),
        })
    }

    /// 待機条件をパース
    fn parse_wait_condition(&self, value: &JsonValue) -> Result<WaitCondition, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Wait condition must be an object".to_string()))?;

        let condition_type = obj.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkflowError::InvalidDefinition("Wait condition must have 'type' field".to_string()))?;

        match condition_type {
            "timer" => {
                let duration_str = self.extract_string(obj, "duration")?;
                let duration = parse_duration(&duration_str)
                    .map_err(|_| WorkflowError::InvalidDefinition(format!("Invalid duration: {}", duration_str)))?;

                Ok(WaitCondition::Timer { duration })
            }

            "event" => {
                let event_type = self.extract_string(obj, "event_type")?;
                let filter = obj.get("filter").and_then(|v| v.as_object());

                Ok(WaitCondition::Event {
                    event_type,
                    filter: filter.map(|obj| self.parse_metadata(obj)),
                })
            }

            "signal" => {
                let signal_name = self.extract_string(obj, "signal_name")?;
                Ok(WaitCondition::Signal { signal_name })
            }

            _ => Err(WorkflowError::InvalidDefinition(format!("Unknown wait condition type: {}", condition_type))),
        }
    }

    /// リトライポリシーをパース
    fn parse_retry_policy(&self, value: &JsonValue) -> Result<Option<RetryPolicy>, WorkflowError> {
        if value.is_null() {
            return Ok(None);
        }

        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Retry policy must be an object".to_string()))?;

        let initial_interval_str = self.extract_string(obj, "initial_interval")?;
        let initial_interval = parse_duration(&initial_interval_str)
            .map_err(|_| WorkflowError::InvalidDefinition(format!("Invalid initial_interval: {}", initial_interval_str)))?;

        let backoff_coefficient = obj.get("backoff_coefficient")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let maximum_interval = obj.get("maximum_interval")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_duration(s).ok());

        let maximum_attempts = obj.get("maximum_attempts")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;

        let non_retryable_errors = obj.get("non_retryable_errors")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Ok(Some(RetryPolicy {
            initial_interval,
            backoff_coefficient,
            maximum_interval,
            maximum_attempts,
            non_retryable_errors,
        }))
    }

    /// 入力マッピングをパース
    fn parse_input_mapping(&self, obj: &Map<String, JsonValue>) -> HashMap<String, String> {
        obj.iter()
            .filter_map(|(k, v)| {
                v.as_str().map(|s| (k.clone(), s.to_string()))
            })
            .collect()
    }

    /// メタデータをパース
    fn parse_metadata(&self, obj: &Map<String, JsonValue>) -> HashMap<String, Value> {
        obj.iter()
            .filter_map(|(k, v)| {
                self.parse_value(v).ok().map(|value| (k.clone(), value))
            })
            .collect()
    }

    /// 値をパース
    fn parse_value(&self, value: &JsonValue) -> Result<Value, WorkflowError> {
        match value {
            JsonValue::Null => Ok(Value::Null),
            JsonValue::Bool(b) => Ok(Value::Bool(*b)),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Int(i))
                } else {
                    Err(WorkflowError::InvalidDefinition("Unsupported number type".to_string()))
                }
            }
            JsonValue::String(s) => Ok(Value::String(s.clone())),
            _ => Err(WorkflowError::InvalidDefinition("Unsupported value type".to_string())),
        }
    }

    /// オブジェクトから文字列を抽出
    fn extract_string(&self, obj: &Map<String, JsonValue>, key: &str) -> Result<String, WorkflowError> {
        obj.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| WorkflowError::InvalidDefinition(format!("Missing or invalid '{}' field", key)))
    }
}

/// JsonnetValueをserde_json::Valueに変換
fn jsonnet_to_json(value: JsonnetValue) -> Result<JsonValue, WorkflowError> {
    match value {
        JsonnetValue::Null => Ok(JsonValue::Null),
        JsonnetValue::Boolean(b) => Ok(JsonValue::Bool(b)),
        JsonnetValue::String(s) => Ok(JsonValue::String(s)),
        JsonnetValue::Number(n) => {
            // 整数として扱う
            if n.fract() == 0.0 {
                let i = n as i64;
                Ok(JsonValue::Number(serde_json::Number::from(i)))
            } else {
                Err(WorkflowError::InvalidDefinition(format!("Invalid number: {}", n)))
            }
        }
        JsonnetValue::Array(arr) => {
            let values = arr.into_iter()
                .map(jsonnet_to_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(JsonValue::Array(values))
        }
        JsonnetValue::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k, jsonnet_to_json(v)?);
            }
            Ok(JsonValue::Object(map))
        }
        _ => Err(WorkflowError::InvalidDefinition("Unsupported Jsonnet value type".to_string())),
    }
}

    /// 基本戦略をパース
    fn parse_basic_strategy(&self, value: &JsonValue) -> Result<StrategyOp, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Basic strategy must be an object".to_string()))?;

        let op = obj.get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkflowError::InvalidDefinition("Basic strategy must have 'op' field".to_string()))?;

        match op {
            "once" => {
                let rule = self.extract_string(obj, "rule")?;
                Ok(StrategyOp::Once { rule })
            }
            "exhaust" => {
                let rule = self.extract_string(obj, "rule")?;
                let order = obj.get("order")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "topdown" => Some(Order::TopDown),
                        "bottomup" => Some(Order::BottomUp),
                        "fair" => Some(Order::Fair),
                        _ => None,
                    })
                    .unwrap_or(Order::TopDown);
                let measure = obj.get("measure").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(StrategyOp::Exhaust { rule, order, measure })
            }
            "while" => {
                let rule = self.extract_string(obj, "rule")?;
                let pred = self.extract_string(obj, "pred")?;
                let order = obj.get("order")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "topdown" => Some(Order::TopDown),
                        "bottomup" => Some(Order::BottomUp),
                        "fair" => Some(Order::Fair),
                        _ => None,
                    })
                    .unwrap_or(Order::TopDown);
                Ok(StrategyOp::While { rule, pred, order })
            }
            _ => Err(WorkflowError::InvalidDefinition(format!("Unknown basic strategy operation: {}", op))),
        }
    }

    /// 優先順位付き戦略をパース
    fn parse_prioritized_strategy(&self, value: &JsonValue) -> Result<PrioritizedStrategy, WorkflowError> {
        let obj = value.as_object()
            .ok_or_else(|| WorkflowError::InvalidDefinition("Prioritized strategy must be an object".to_string()))?;

        let strategy = obj.get("strategy")
            .ok_or_else(|| WorkflowError::InvalidDefinition("Prioritized strategy must have 'strategy' field".to_string()))?;

        let strategy = self.parse_basic_strategy(strategy)?;
        let priority = obj.get("priority")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        Ok(PrioritizedStrategy {
            strategy: Box::new(strategy),
            priority,
        })
    }

/// ISO 8601 duration文字列をDurationに変換
fn parse_duration(s: &str) -> Result<Duration, WorkflowError> {
    // 簡易的なISO 8601 durationパーサー
    // PT5M, PT1H30M などの形式をサポート

    if !s.starts_with("PT") {
        return Err(WorkflowError::InvalidDefinition(format!("Invalid duration format: {}", s)));
    }

    let mut total_seconds = 0u64;
    let mut remaining = &s[2..]; // PTを除去

    // 時間部分（H）
    if let Some(h_pos) = remaining.find('H') {
        let hours: u64 = remaining[..h_pos].parse()
            .map_err(|_| WorkflowError::InvalidDefinition(format!("Invalid hours in duration: {}", s)))?;
        total_seconds += hours * 3600;
        remaining = &remaining[h_pos + 1..];
    }

    // 分部分（M）
    if let Some(m_pos) = remaining.find('M') {
        let minutes: u64 = remaining[..m_pos].parse()
            .map_err(|_| WorkflowError::InvalidDefinition(format!("Invalid minutes in duration: {}", s)))?;
        total_seconds += minutes * 60;
        remaining = &remaining[m_pos + 1..];
    }

    // 秒部分（S）
    if let Some(s_pos) = remaining.find('S') {
        let seconds: u64 = remaining[..s_pos].parse()
            .map_err(|_| WorkflowError::InvalidDefinition(format!("Invalid seconds in duration: {}", s)))?;
        total_seconds += seconds;
    }

    Ok(Duration::from_secs(total_seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("PT5M").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("PT1H30M").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse_duration("PT30S").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_simple_workflow_parsing() {
        let content = r#"
        {
          workflow: {
            id: "test_workflow",
            name: "Test Workflow",
            version: "1.0.0",
            inputs: [
              { name: "input1", type: "string", required: true }
            ],
            outputs: [
              { name: "output1", type: "string" }
            ],
            strategy: {
              op: "activity",
              activity_ref: "test_activity",
              input_mapping: {
                input: "$.inputs.input1"
              }
            }
          }
        }
        "#;

        let mut parser = WorkflowParser::new();
        let result = parser.parse_string(content);

        assert!(result.is_ok());
        let workflow_ir = result.unwrap();
        assert_eq!(workflow_ir.id, "test_workflow");
        assert_eq!(workflow_ir.name, "Test Workflow");
    }
}
