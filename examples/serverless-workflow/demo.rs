//! Serverless Workflow Demo
//!
//! このファイルは、Serverless Workflow を使用してワークフローを実行する方法を示すデモです。

use kotoba_workflow_core::{WorkflowEngine, parse_workflow};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Kotoba Serverless Workflow Demo");
    println!("==================================");

    // ワークフローエンジンを作成
    let engine = WorkflowEngine::new();
    println!("✅ Workflow engine created");

    // シンプルワークフローを実行
    println!("\n📋 Running simple workflow...");
    let simple_workflow_json = include_str!("simple-workflow.json");
    let simple_workflow = parse_workflow(simple_workflow_json)?;

    println!("   Workflow: {}", simple_workflow.name);
    println!("   Description: {}", simple_workflow.description.as_deref().unwrap_or("No description"));

    let execution_id = engine.start_workflow(&simple_workflow, serde_json::json!({})).await?;
    println!("   Execution ID: {}", execution_id);

    // 実行状態を確認（少し待機してから）
    tokio::time::sleep(Duration::from_secs(1)).await;

    if let Some(status) = engine.get_execution_status(&execution_id).await? {
        println!("   Status: {:?}", status);
    }

    if let Some(result) = engine.get_execution_result(&execution_id).await? {
        println!("   Duration: {}ms", result.duration_ms);
        println!("   Final status: {:?}", result.status);

        if let Some(output) = &result.output {
            println!("   Output: {}", serde_json::to_string_pretty(output)?);
        }
    }

    // HTTPワークフローを実行
    println!("\n🌐 Running HTTP workflow...");
    let http_workflow_json = include_str!("http-workflow.json");
    let http_workflow = parse_workflow(http_workflow_json)?;

    println!("   Workflow: {}", http_workflow.name);
    let http_execution_id = engine.start_workflow(&http_workflow, serde_json::json!({})).await?;
    println!("   Execution ID: {}", http_execution_id);

    // 少し待機してから状態を確認
    tokio::time::sleep(Duration::from_secs(2)).await;

    if let Some(status) = engine.get_execution_status(&http_execution_id).await? {
        println!("   Status: {:?}", status);
    }

    // 実行中のワークフロー一覧を表示
    let executions = engine.list_executions().await?;
    println!("\n📊 Total executions: {}", executions.len());

    println!("\n✨ Demo completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_workflow_core::parse_workflow;

    #[test]
    fn test_parse_simple_workflow() {
        let json = include_str!("simple-workflow.json");
        let result = parse_workflow(json);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "simple-workflow");
        assert_eq!(workflow.r#do.len(), 2);
    }

    #[test]
    fn test_parse_http_workflow() {
        let json = include_str!("http-workflow.json");
        let result = parse_workflow(json);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "http-workflow");
        assert_eq!(workflow.r#do.len(), 4);
    }

    #[test]
    fn test_parse_error_handling_workflow() {
        let json = include_str!("error-handling-workflow.json");
        let result = parse_workflow(json);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "error-handling-workflow");
        assert_eq!(workflow.r#do.len(), 3);
    }
}
