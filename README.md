# Kotoba

**GP2-based Graph Rewriting Language** - A comprehensive graph processing system with ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/workflow/status/jun784/kotoba/CI)](https://github.com/jun784/kotoba/actions)

## 📖 Overview

Kotoba is a powerful graph processing system based on graph theory. Built around GP2 (Graph Programs 2) rewriting system, it provides comprehensive implementation of ISO GQL-compliant query language, MVCC+Merkle tree persistence, and distributed execution.

### 🎯 Key Features

- **DPO (Double Pushout) Typed Attribute Graph Rewriting**: Graph transformation with theoretical foundation
- **ISO GQL-compliant Queries**: Standardized graph query language
- **MVCC + Merkle DAG**: Consistent distributed persistence
- **Column-oriented Storage**: Efficient data access with LSM trees
- **Process Network Graph Model**: Centralized management via dag.jsonnet
- **Rust Native**: Memory-safe and high-performance

## 🚀 Quick Start

### Prerequisites

- Rust 1.70.0 or later
- Cargo package manager

### Installation

```bash
# Clone the repository
git clone https://github.com/jun784/kotoba.git
cd kotoba

# Install dependencies
cargo build

# Run tests
cargo test

# Build CLI tool
cargo build --release
```

### Basic Usage Example

```rust
use kotoba::*;

fn main() -> Result<()> {
    // Create a graph
    let mut graph = Graph::empty();

    // Add vertices
    let v1 = graph.add_vertex(VertexData {
        id: uuid::Uuid::new_v4(),
        labels: vec!["Person".to_string()],
        props: [("name".to_string(), Value::String("Alice".to_string()))].into(),
    });

    let v2 = graph.add_vertex(VertexData {
        id: uuid::Uuid::new_v4(),
        labels: vec!["Person".to_string()],
        props: [("name".to_string(), Value::String("Bob".to_string()))].into(),
    });

    // Add edge
    graph.add_edge(EdgeData {
        id: uuid::Uuid::new_v4(),
        src: v1,
        dst: v2,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    // Execute GQL query
    let gql = "MATCH (p:Person) RETURN p.name";
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();
    let results = executor.execute_gql(gql, &GraphRef::new(graph), &catalog)?;

    println!("Query results: {:?}", results);
    Ok(())
}
```

## 🏗️ Architecture

### Multi-Crate Architecture

Kotobaは以下のmulti crateアーキテクチャを採用しています：

```
├── kotoba-core/           # 基本型とIR定義
├── kotoba-graph/          # グラフデータ構造
├── kotoba-storage/        # 永続化層 (MVCC + Merkle)
├── kotoba-execution/      # クエリ実行とプランナー
├── kotoba-rewrite/        # グラフ書き換えエンジン
├── kotoba-web/            # WebフレームワークとHTTP
└── kotoba/                # 統合crate (全機能利用)
```

各crateは独立して使用可能で、必要な機能のみを選択して利用できます。

#### 使用例

```rust
// 統合crateを使用する場合
use kotoba::prelude::*;

// 個別crateを使用する場合
use kotoba_core::types::*;
use kotoba_graph::prelude::*;
```

#### WASM対応

各crateは条件付きコンパイルによりWASMターゲットにも対応しています：

```bash
# WASMビルド
cargo build --target wasm32-unknown-unknown --features wasm
```

### Process Network Graph Model

Kotoba is based on a **Process Network Graph Model**, where all components are centrally managed through `dag.jsonnet`.

#### Main Components

```
┌─────────────────────────────────────────────────────────────┐
│                          lib.rs                             │
│                    (Main Library)                           │
├─────────────────────────────────────────────────────────────┤
│          execution/          │          rewrite/            │
│       (Query Executor)       │       (DPO Rewriter)         │
├─────────────────────────────────────────────────────────────┤
│          planner/            │          storage/            │
│       (Query Planner)        │       (MVCC+Merkle)          │
├─────────────────────────────────────────────────────────────┤
│           graph/             │            ir/               │
│       (Data Structures)      │       (Core IR)              │
├─────────────────────────────────────────────────────────────┤
│                          types.rs                           │
│                    (Common Types)                           │
└─────────────────────────────────────────────────────────────┘
```

### Build Order (Topological Sort)

```jsonnet
// Get build order from dag.jsonnet
$ jsonnet eval dag.jsonnet | jq .topological_order
[
  "types",
  "ir_catalog",
  "ir_rule",
  "ir_query",
  "ir_patch",
  "graph_vertex",
  "graph_edge",
  "ir_strategy",
  "graph_core",
  "storage_mvcc",
  "storage_merkle",
  "storage_lsm",
  "planner_logical",
  "planner_physical",
  "execution_parser",
  "rewrite_matcher",
  "rewrite_applier",
  "planner_optimizer",
  "rewrite_engine",
  "execution_engine",
  "lib"
]
```

## 📋 Usage

### 1. Basic GQL Queries

```rust
use kotoba::{QueryExecutor, Catalog, GraphRef};

// Create query executor
let executor = QueryExecutor::new();
let catalog = Catalog::empty();

// Execute GQL query
let gql = r#"
    MATCH (p:Person)-[:FOLLOWS]->(f:Person)
    WHERE p.age > 20
    RETURN p.name, f.name
"#;

let results = executor.execute_gql(gql, &graph_ref, &catalog)?;
```

### 2. Graph Rewriting

```rust
use kotoba::{RewriteEngine, RuleIR, StrategyIR};

// Create rewrite engine
let engine = RewriteEngine::new();

// Define rules
let rule = RuleIR { /* rule definition */ };
let strategy = StrategyIR { /* strategy definition */ };

// Execute rewrite
let patch = engine.rewrite(&graph_ref, &rule, &strategy)?;
```

### 3. Manual Graph Operations

```rust
use kotoba::{Graph, VertexBuilder, EdgeBuilder};

// Create graph
let mut graph = Graph::empty();

// Add vertices
let v1 = graph.add_vertex(VertexBuilder::new()
    .label("Person")
    .prop("name", Value::String("Alice"))
    .build());

// Add edge
let e1 = graph.add_edge(EdgeBuilder::new()
    .src(v1)
    .dst(v2)
    .label("FOLLOWS")
    .build());
```

## 📄 .kotoba File Format

Kotobaプロジェクトでは、設定ファイルやUI定義などに`.kotoba`ファイル形式を使用します。これはJSON Lines (JSONL)形式をベースとした構造化された設定フォーマットです。

### 概要

`.kotoba`ファイルは以下の特徴を持ちます：

- **JSON Lines形式**: 各行が独立したJSONオブジェクト
- **コメント対応**: `#`で始まる行はコメントとして無視
- **型付け**: `type`フィールドによりオブジェクトの種類を明示
- **階層構造**: コンポーネント間の親子関係を表現可能
- **メタデータ**: 各オブジェクトに説明や追加情報を付与

### ファイル形式仕様

#### 基本構造

```json
{
  "type": "object_type",
  "name": "object_name",
  "property1": "value1",
  "property2": "value2",
  "metadata": {
    "description": "オブジェクトの説明",
    "version": "1.0"
  }
}
```

#### 主要なプロパティ

- **`type`** (必須): オブジェクトの種類を指定
- **`name`** (推奨): オブジェクトの一意な識別子
- **`metadata`** (オプション): 追加情報（説明、バージョンなど）

### オブジェクトタイプ

#### 1. `config` - 設定オブジェクト

アプリケーション全体の設定を定義します。

```json
{
  "type": "config",
  "name": "MyApp",
  "version": "1.0.0",
  "host": "127.0.0.1",
  "port": 8080,
  "theme": "light",
  "metadata": {
    "description": "アプリケーション設定",
    "environment": "development"
  }
}
```

#### 2. `route` - HTTPルート定義

HTTPサーバーのエンドポイントを定義します。

```json
{
  "type": "route",
  "method": "GET|POST|PUT|DELETE",
  "pattern": "/api/v1/users/{id}",
  "handler": "user_handler",
  "metadata": {
    "description": "ユーザー取得エンドポイント",
    "version": "v1",
    "auth_required": true
  }
}
```

#### 3. `middleware` - ミドルウェア定義

HTTPリクエスト処理の中間層を定義します。

```json
{
  "type": "middleware",
  "name": "cors",
  "order": 10,
  "function": "cors_middleware",
  "metadata": {
    "description": "CORS処理ミドルウェア",
    "allowed_origins": ["*"]
  }
}
```

#### 4. `component` - UIコンポーネント定義

ユーザーインターフェースのコンポーネントを定義します。

```json
{
  "type": "component",
  "name": "Header",
  "component_type": "header",
  "props": {
    "title": "My App",
    "className": "app-header"
  },
  "children": ["Logo", "Navigation"],
  "metadata": {
    "description": "アプリケーションのヘッダーコンポーネント"
  }
}
```

#### 5. `handler` - イベントハンドラー定義

UIコンポーネントのイベント処理を定義します。

```json
{
  "type": "handler",
  "name": "onButtonClick",
  "function": "handleButtonClick",
  "metadata": {
    "description": "ボタンクリックイベントハンドラー"
  }
}
```

#### 6. `state` - 状態管理定義

アプリケーションの状態を定義します。

```json
{
  "type": "state",
  "name": "user",
  "initial": null,
  "metadata": {
    "description": "現在のユーザー状態"
  }
}
```

### 使用例

#### HTTPサーバー設定例

```json
# config.kotoba
{"type": "config", "host": "127.0.0.1", "port": 8080, "max_connections": 1000}
{"type": "route", "method": "GET", "pattern": "/health", "handler": "health_check"}
{"type": "route", "method": "GET", "pattern": "/api/users", "handler": "list_users"}
{"type": "middleware", "name": "cors", "order": 10, "function": "cors_middleware"}
{"type": "middleware", "name": "auth", "order": 20, "function": "auth_middleware"}
```

#### React UI設定例

```json
# app.kotoba
{"type": "config", "name": "MyApp", "version": "1.0.0", "theme": "light"}
{"type": "component", "name": "App", "component_type": "div", "children": ["Header", "Main"]}
{"type": "component", "name": "Header", "component_type": "header", "props": {"title": "My App"}}
{"type": "handler", "name": "toggleTheme", "function": "handleThemeToggle"}
{"type": "state", "name": "theme", "initial": "light"}
```

### パースと使用方法

```rust
use kotoba::http::parser::KotobaParser;

// .kotobaファイルをパース
let parser = KotobaParser::new();
let config = parser.parse_file("config.kotoba")?;

// HTTPサーバーを設定
let server = HttpServer::from_config(config);
```

### ベストプラクティス

1. **コメントの活用**: 各オブジェクトの目的をコメントで明確に記述
2. **一貫した命名**: コンポーネント名やハンドラー名を一貫して命名
3. **メタデータの活用**: バージョン管理やドキュメントのためにmetadataを活用
4. **型の一貫性**: 同じtypeのオブジェクトは同じプロパティ構造を使用
5. **階層構造の整理**: 複雑なUIの場合は論理的な階層構造を使用

### 拡張性

`.kotoba`形式は拡張可能です。新しい`type`を定義することで、プロジェクト固有の設定や定義を追加できます。拡張する場合のガイドライン：

- 新しいtypeは既存の命名規則に従う
- 必須プロパティは明確に定義する
- metadataフィールドは常にオプションとする
- ドキュメントに新しいtypeの仕様を記載する

## 🛠️ Development

### Using dag.jsonnet

#### 1. Dependency Analysis

```bash
# Check dependencies of specific component
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependencies('execution_engine')"

# Check components that depend on this component
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependents('types')"
```

#### 2. Build Order Verification

```bash
# Get overall build order
jsonnet eval dag.jsonnet | jq .topological_order[]

# Check build order for specific node
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_build_order('graph_core')"
```

#### 3. Causality Identification During Troubleshooting

```bash
# Get investigation order when problems occur
jsonnet eval dag.jsonnet | jq .reverse_topological_order[]
```

### Using lib.jsonnet

#### 1. Build Configuration Verification

```bash
# Get configuration for specific target
jsonnet eval -e "local lib = import 'lib.jsonnet'; lib.get_target_config('x86_64-apple-darwin')"

# Resolve component dependencies
jsonnet eval -e "local lib = import 'lib.jsonnet'; lib.resolve_dependencies('kotoba-core', ['full'])"
```

#### 2. Packaging Configuration

```bash
# Get Docker image configuration
jsonnet eval lib.jsonnet | jq .packaging.docker

# Get Debian package configuration
jsonnet eval lib.jsonnet | jq .packaging.debian
```

### Development Workflow

```bash
# 1. Make code changes
vim src/some_component.rs

# 2. Check dependencies
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependencies('some_component')"

# 3. Run tests
cargo test --package some_component

# 4. Check overall consistency
cargo check

# 5. Validate DAG
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.validate_dag()"

# 6. Commit
git add .
git commit -m "Update some_component"
```

## 🧪 Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_graph_operations

# Run documentation tests
cargo test --doc
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

### LDBC-SNB Benchmark

```bash
# Run benchmark with LDBC-SNB dataset
cargo run --bin kotoba-bench -- --dataset ldbc-snb
```

## 📦 Packaging

### Docker Image

```bash
# Build Docker image
docker build -t kotoba:latest .

# Run the image
docker run -p 8080:8080 kotoba:latest
```

### Debian Package

```bash
# Create Debian package
cargo deb

# Install the package
sudo dpkg -i target/debian/kotoba_0.1.0_amd64.deb
```

### Homebrew

```bash
# Install Homebrew Formula
brew install kotoba
```

## 🔧 CLI Tools

### kotoba-cli

```bash
# Show help
./target/release/kotoba-cli --help

# Execute GQL query
./target/release/kotoba-cli query "MATCH (p:Person) RETURN p.name"

# Load graph file
./target/release/kotoba-cli load --file graph.json

# Display statistics
./target/release/kotoba-cli stats
```

## 📚 API Documentation

```bash
# Generate documentation
cargo doc --open

# Generate documentation including private items
cargo doc --document-private-items --open
```

## 🤝 Contributing

### Contribution Guidelines

1. **Create Issue**: Bug reports or feature requests
2. **Create Branch**: `feature/your-feature-name`
3. **Implement Changes**:
   - Add tests
   - Update documentation
   - Verify dag.jsonnet consistency
4. **Create Pull Request**

### Development Environment Setup

```bash
# Install development dependencies
cargo install cargo-edit cargo-watch cargo-deb

# Set up pre-commit hooks
cp pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### Coding Standards

- **Rust**: Use `rustfmt` and `clippy`
- **Commit Messages**: [Conventional Commits](https://conventionalcommits.org/)
- **Testing**: Add tests for all changes
- **Documentation**: Add documentation for all public APIs

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- GP2 Team: Theoretical foundation for graph rewriting systems
- ISO/IEC: GQL standard specification
- Rust Community: Excellent programming language

## 📞 Support

- **Documentation**: [https://kotoba.jun784.dev](https://kotoba.jun784.dev)
- **Issues**: [GitHub Issues](https://github.com/jun784/kotoba/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jun784/kotoba/discussions)

---

**Kotoba** - Exploring the world of graphs through words
