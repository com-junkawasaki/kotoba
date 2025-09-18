# Serverless Workflow Examples

このディレクトリには、[Serverless Workflow](https://serverlessworkflow.io/) 仕様に基づいた Kotoba のワークフロー例が含まれています。

## 概要

Serverless Workflow は、クラウドネイティブなワークフロー定義のための標準仕様です。Kotoba はこの仕様をサポートしており、JSON 形式でワークフローを定義できます。

## サンプルワークフロー

### 1. シンプルなワークフロー (`simple-workflow.json`)

変数を設定し、待機する基本的なワークフロー。

### 2. HTTP 呼び出しワークフロー (`http-workflow.json`)

外部 API を呼び出すワークフロー。

### 3. エラーハンドリングワークフロー (`error-handling-workflow.json`)

エラーハンドリングを含むワークフロー。

## ワークフローの実行

### Rust コードからの実行

```rust
use kotoba_workflow_core::{WorkflowEngine, parse_workflow};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ワークフローエンジンを作成
    let engine = WorkflowEngine::new();

    // JSONからワークフローをパース
    let workflow_json = include_str!("examples/serverless-workflow/simple-workflow.json");
    let workflow = parse_workflow(workflow_json)?;

    // ワークフローを実行
    let execution_id = engine.start_workflow(&workflow, serde_json::json!({})).await?;

    println!("Started workflow execution: {}", execution_id);

    // 実行状態を確認
    if let Some(status) = engine.get_execution_status(&execution_id).await? {
        println!("Execution status: {:?}", status);
    }

    Ok(())
}
```

### サポートされているステートタイプ

現在実装されているステートタイプ：

- `callHttp`: HTTP API 呼び出し
- `set`: 変数設定
- `wait`: 待機
- `raise`: エラー発生

## 詳細

詳細については [Serverless Workflow 仕様](https://serverlessworkflow.io/specification) を参照してください。