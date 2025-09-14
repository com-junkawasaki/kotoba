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

ユーザーは `.kotoba` ファイル（Jsonnet形式）を作成し、`kotoba run` コマンドで実行します：

**app.kotoba**
```jsonnet
{
  // アプリケーション設定
  config: {
    type: "config",
    name: "MyGraphApp",
    version: "1.0.0",
  },

  // グラフデータ
  graph: {
    vertices: [
      { id: "alice", labels: ["Person"], properties: { name: "Alice", age: 30 } },
      { id: "bob", labels: ["Person"], properties: { name: "Bob", age: 25 } },
    ],
    edges: [
      { id: "follows_1", src: "alice", dst: "bob", label: "FOLLOWS" },
    ],
  },

  // GQLクエリ
  queries: [
    {
      name: "find_people",
      gql: "MATCH (p:Person) RETURN p.name, p.age",
    },
  ],

  // 実行ロジック
  handlers: [
    {
      name: "main",
      function: "execute_queries",
      metadata: { description: "Execute all defined queries" },
    },
  ],
}
```

**実行方法**
```bash
# .kotobaファイルを実行
kotoba run app.kotoba

# またはサーバーモードで起動
kotoba server --config app.kotoba
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

```bash
# .kotobaファイルで全て定義
kotoba run myapp.kotoba

# 開発時はウォッチモード
kotoba run myapp.kotoba --watch
```

**Rust API（内部使用）**
```rust
// Rust APIは主に内部実装で使用
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

**queries.kotoba**
```jsonnet
{
  config: {
    type: "config",
    name: "QueryExample",
  },

  // グラフデータ
  graph: {
    vertices: [
      { id: "alice", labels: ["Person"], properties: { name: "Alice", age: 30 } },
      { id: "bob", labels: ["Person"], properties: { name: "Bob", age: 25 } },
      { id: "charlie", labels: ["Person"], properties: { name: "Charlie", age: 35 } },
    ],
    edges: [
      { id: "f1", src: "alice", dst: "bob", label: "FOLLOWS" },
      { id: "f2", src: "bob", dst: "charlie", label: "FOLLOWS" },
    ],
  },

  // GQLクエリ定義
  queries: [
    {
      name: "follow_network",
      gql: "MATCH (p:Person)-[:FOLLOWS]->(f:Person) WHERE p.age > 25 RETURN p.name, f.name",
      description: "25歳以上の人がフォローしている人を取得",
    },
  ],

  handlers: [
    {
      name: "execute_query",
      function: "run_gql_query",
      parameters: { query_name: "follow_network" },
    },
  ],
}
```

```bash
kotoba run queries.kotoba
```

### 2. Graph Rewriting

**rewrite.kotoba**
```jsonnet
{
  config: {
    type: "config",
    name: "RewriteExample",
  },

  // グラフ書換えルール
  rules: [
    {
      name: "triangle_collapse",
      description: "三角形を折りたたむ",
      lhs: {
        nodes: [
          { id: "u", type: "Person" },
          { id: "v", type: "Person" },
          { id: "w", type: "Person" },
        ],
        edges: [
          { id: "e1", src: "u", dst: "v", type: "FOLLOWS" },
          { id: "e2", src: "v", dst: "w", type: "FOLLOWS" },
        ],
      },
      rhs: {
        nodes: [
          { id: "u", type: "Person" },
          { id: "w", type: "Person" },
        ],
        edges: [
          { id: "e3", src: "u", dst: "w", type: "FOLLOWS" },
        ],
      },
    },
  ],

  // 実行戦略
  strategies: [
    {
      name: "exhaust_triangle_collapse",
      rule: "triangle_collapse",
      strategy: "exhaust",
      order: "topdown",
    },
  ],

  handlers: [
    {
      name: "apply_rewrite",
      function: "execute_rewrite",
      parameters: { strategy_name: "exhaust_triangle_collapse" },
    },
  ],
}
```

### 3. HTTP Server with Graph Operations

**server.kotoba**
```jsonnet
{
  config: {
    type: "config",
    name: "GraphServer",
    server: { host: "127.0.0.1", port: 3000 },
  },

  // ルート定義
  routes: [
    {
      method: "GET",
      pattern: "/api/users",
      handler: "list_users",
      description: "ユーザー一覧を取得",
    },
    {
      method: "POST",
      pattern: "/api/users",
      handler: "create_user",
      description: "ユーザーを作成",
    },
  ],

  // グラフスキーマ
  schema: {
    node_types: ["User", "Post"],
    edge_types: ["FOLLOWS", "LIKES"],
  },

  handlers: [
    {
      name: "list_users",
      function: "execute_gql",
      parameters: {
        query: "MATCH (u:User) RETURN u.name, u.email",
        format: "json",
      },
    },
    {
      name: "create_user",
      function: "create_graph_node",
      parameters: {
        type: "User",
        properties: ["name", "email", "age"],
      },
    },
  ],
}
```

## 📄 .kotoba File Format

Kotobaプロジェクトでは、設定ファイルやUI定義などに`.kotoba`ファイル形式を使用します。これはJsonnet形式をベースとした構造化された設定フォーマットです。

### 概要

`.kotoba`ファイルは以下の特徴を持ちます：

- **Jsonnet形式**: JSONのスーパーセットで、変数、関数、条件分岐などの機能を活用
- **構造化設定**: オブジェクトと配列による階層的な設定構造
- **ユーティリティ関数**: 設定生成のための再利用可能な関数定義
- **計算プロパティ**: 動的な設定生成とバリデーション
- **型安全**: Jsonnetの型システムによる設定の整合性確保

### ファイル形式仕様

#### 基本構造

```jsonnet
// 設定ファイルの基本構造
{
  // 設定セクション
  config: {
    type: "config",
    name: "MyApp",
    version: "1.0.0",
    metadata: {
      description: "アプリケーション設定",
    },
  },

  // コンポーネント定義
  components: [
    // コンポーネントオブジェクト
  ],

  // ユーティリティ関数
  makeComponent: function(name, type, props={}) {
    // コンポーネント生成関数
  },
}
```

#### 主要なプロパティ

- **`type`** (必須): オブジェクトの種類を指定
- **`name`** (推奨): オブジェクトの一意な識別子
- **`metadata`** (オプション): 追加情報（説明、バージョンなど）
- **`local`変数**: Jsonnetのローカル変数による設定の共通化
- **`関数`**: 設定生成のための再利用可能な関数
- **`::`演算子**: 計算プロパティによる動的設定生成

### 主要なセクション

#### 1. `config` - 設定オブジェクト

アプリケーション全体の設定を定義します。

```jsonnet
local appVersion = "1.0.0";

config: {
  type: "config",
  name: "MyApp",
  version: appVersion,
  host: "127.0.0.1",
  port: 8080,
  theme: "light",
  metadata: {
    description: "アプリケーション設定",
    environment: "development",
  },
}
```

#### 2. `routes` / `middlewares` - HTTP設定

HTTPサーバーのルートとミドルウェアを構造化して定義します。

```jsonnet
// ユーティリティ関数
local makeRoute = function(method, pattern, handler, desc) {
  type: "route",
  method: method,
  pattern: pattern,
  handler: handler,
  metadata: { description: desc },
};

routes: [
  makeRoute("GET", "/api/" + appVersion + "/users", "list_users", "List users"),
  makeRoute("POST", "/api/" + appVersion + "/users", "create_user", "Create user"),
],

middlewares: [
  {
    type: "middleware",
    name: "cors",
    order: 10,
    function: "cors_middleware",
    metadata: {
      description: "CORS handling middleware",
      allowed_origins: ["*"],
    },
  },
],
```

#### 3. `components` - UIコンポーネント定義

Reactコンポーネントを構造化して定義します。

```jsonnet
local styles = {
  button: { primary: "button primary", secondary: "button secondary" },
  layout: { header: "header", sidebar: "sidebar" },
};

local makeButton = function(name, text, style, onClick) {
  type: "component",
  name: name,
  component_type: "button",
  props: {
    text: text,
    className: style,
    onClick: onClick,
  },
  metadata: { description: name + " button" },
};

components: [
  makeButton("SaveButton", "Save", styles.button.primary, "handleSave"),
  makeButton("CancelButton", "Cancel", styles.button.secondary, "handleCancel"),
],
```

#### 4. `handlers` / `states` - イベントと状態管理

イベントハンドラーと状態を定義します。

```jsonnet
handlers: [
  {
    type: "handler",
    name: "handleSave",
    function: "handleSave",
    metadata: { description: "Save form data" },
  },
],

states: [
  {
    type: "state",
    name: "user",
    initial: null,
    metadata: { description: "Current user state" },
  },
  {
    type: "state",
    name: "loading",
    initial: false,
    metadata: { description: "Loading state" },
  },
],
```

#### 5. 計算プロパティとバリデーション

Jsonnetの機能を活用した動的設定とバリデーション。

```jsonnet
// 計算プロパティ
allRoutes:: [r.pattern for r in self.routes],
routeCount:: std.length(self.routes),

// バリデーション関数
validateRoutes:: function() {
  local duplicates = [
    pattern
    for pattern in std.set([r.pattern for r in self.routes])
    if std.count([r.pattern for r in self.routes], pattern) > 1
  ];
  if std.length(duplicates) > 0 then
    error "Duplicate route patterns: " + std.join(", ", duplicates)
  else
    "Routes validation passed";
},
```

### 使用例

#### HTTPサーバー設定例

```jsonnet
// config.kotoba - HTTPサーバー設定
local apiVersion = "v1";
local defaultTimeout = 30000;

{
  // サーバー設定
  config: {
    type: "config",
    host: "127.0.0.1",
    port: 8080,
    max_connections: 1000,
    timeout_ms: defaultTimeout,
    metadata: {
      description: "HTTP server configuration",
      environment: "development",
    },
  },

  // ユーティリティ関数
  makeRoute: function(method, pattern, handler, desc) {
    type: "route",
    method: method,
    pattern: pattern,
    handler: handler,
    metadata: { description: desc },
  },

  makeMiddleware: function(name, order, func, desc) {
    type: "middleware",
    name: name,
    order: order,
    function: func,
    metadata: { description: desc },
  },

  // ルート定義
  routes: [
    $.makeRoute("GET", "/ping", "ping_handler", "Simple ping endpoint"),
    $.makeRoute("GET", "/health", "health_check", "Health check endpoint"),
    $.makeRoute("GET", "/api/" + apiVersion + "/users", "list_users", "List users"),
    $.makeRoute("POST", "/api/" + apiVersion + "/users", "create_user", "Create user"),
  ],

  // ミドルウェア定義
  middlewares: [
    $.makeMiddleware("cors", 10, "cors_middleware", "CORS handling"),
    $.makeMiddleware("auth", 20, "auth_middleware", "Authentication"),
    $.makeMiddleware("logger", 100, "request_logger", "Request logging"),
  ],

  // 計算プロパティ
  serverInfo:: {
    host: $.config.host,
    port: $.config.port,
    routes_count: std.length($.routes),
    middlewares_count: std.length($.middlewares),
  },
}
```

#### React UI設定例

```jsonnet
// app.kotoba - React UI設定
local appName = "MyApp";
local appVersion = "1.0.0";

{
  // アプリケーション設定
  config: {
    type: "config",
    name: appName,
    version: appVersion,
    theme: "light",
    title: "My App",
    metadata: {
      framework: "React",
      description: "Sample React application",
    },
  },

  // スタイル定数
  styles: {
    button: {
      primary: "button primary",
      secondary: "button secondary",
    },
    layout: {
      header: "header",
      main: "main-content",
    },
  },

  // ユーティリティ関数
  makeComponent: function(name, componentType, props={}, children=[], desc="") {
    type: "component",
    name: name,
    component_type: componentType,
    props: props,
    children: children,
    metadata: { description: desc },
  },

  makeButton: function(name, text, style, onClick, desc) {
    $.makeComponent(name, "button", {
      text: text,
      className: style,
      onClick: onClick,
    }, [], desc),
  },

  // コンポーネント定義
  components: [
    $.makeComponent("App", "div", {}, ["Header", "Main"], "Root application component"),
    $.makeComponent("Header", "header", {
      title: $.config.title,
      className: $.styles.layout.header,
    }, ["Nav"], "Application header"),
    $.makeButton("SaveBtn", "Save", $.styles.button.primary, "handleSave", "Save button"),
    $.makeButton("CancelBtn", "Cancel", $.styles.button.secondary, "handleCancel", "Cancel button"),
  ],

  // イベントハンドラー
  handlers: [
    {
      type: "handler",
      name: "handleSave",
      function: "handleSave",
      metadata: { description: "Handle save action" },
    },
    {
      type: "handler",
      name: "handleCancel",
      function: "handleCancel",
      metadata: { description: "Handle cancel action" },
    },
  ],

  // 状態管理
  states: [
    {
      type: "state",
      name: "user",
      initial: null,
      metadata: { description: "Current user state" },
    },
    {
      type: "state",
      name: "theme",
      initial: $.config.theme,
      metadata: { description: "Current theme state" },
    },
  ],
}
```

### パースと使用方法

Jsonnetファイルは`jsonnet`コマンドまたはプログラムによる評価が必要です：

```bash
# Jsonnetファイルを評価してJSONに変換
jsonnet eval config.kotoba

# またはプログラムで直接使用
jsonnet eval config.kotoba | jq .routes
```

```rust
// Rustでの使用例
use std::process::Command;

// Jsonnetファイルを評価
let output = Command::new("jsonnet")
    .arg("eval")
    .arg("config.kotoba")
    .output()?;

let config_json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

// 設定を使用
if let Some(routes) = config_json.get("routes") {
    println!("Found {} routes", routes.as_array().unwrap().len());
}
```

### Jsonnet固有の機能活用

#### 1. 変数と定数の使用

```jsonnet
local appVersion = "v1";
local defaultPort = 8080;

{
  config: {
    version: appVersion,
    port: defaultPort,
  },
  routes: [
    { pattern: "/api/" + appVersion + "/users" },
  ],
}
```

#### 2. 関数による設定生成

```jsonnet
local makeApiRoute = function(method, resource, action) {
  type: "route",
  method: method,
  pattern: "/api/v1/" + resource + "/" + action,
  handler: resource + "_" + action,
};

routes: [
  makeApiRoute("GET", "users", "list"),
  makeApiRoute("POST", "users", "create"),
],
```

#### 3. 計算プロパティによる動的設定

```jsonnet
{
  components: [/* ... */],
  // コンポーネント数の計算
  componentCount:: std.length(self.components),

  // コンポーネントタイプ別の集計
  componentTypes:: std.set([c.component_type for c in self.components]),
}
```

#### 4. 条件分岐とバリデーション

```jsonnet
local environment = "production";

{
  config: {
    debug: if environment == "development" then true else false,
    port: if environment == "production" then 80 else 3000,
  },

  // バリデーション
  validate:: function() {
    if std.length(self.config.name) == 0 then
      error "Application name is required"
    else
      "Configuration is valid";
  },
}
```

### ベストプラクティス

1. **変数の活用**: 共通の値を`local`変数で定義してDRY原則を守る
2. **関数による抽象化**: 設定生成パターンを関数化して再利用性を高める
3. **計算プロパティの使用**: `::`演算子で動的な設定値を生成
4. **構造化**: 設定を論理的なセクション（config, routes, components等）に分ける
5. **バリデーション**: 設定の妥当性を検証する関数を定義
6. **コメント**: Jsonnetの`//`コメントを活用して設定の意図を明確に
7. **再利用**: 共通の関数やスタイルを別ファイルに分離してimport

### 拡張性

`.kotoba`形式（Jsonnet）は非常に拡張性が高く、Jsonnetの全機能を活用できます：

#### カスタム関数ライブラリ

```jsonnet
// utils.libsonnet
{
  // 汎用ユーティリティ関数
  makeCrudRoutes(resource):: [
    {
      type: "route",
      method: "GET",
      pattern: "/api/v1/" + resource,
      handler: resource + "_list",
    },
    {
      type: "route",
      method: "POST",
      pattern: "/api/v1/" + resource,
      handler: resource + "_create",
    },
  ],

  // スタイル定数
  themes: {
    light: { bg: "#ffffff", fg: "#000000" },
    dark: { bg: "#000000", fg: "#ffffff" },
  },
}
```

#### 設定の合成

```jsonnet
// 複数の設定ファイルを合成
local base = import "base.libsonnet";
local api = import "api.libsonnet";

base + api + {
  // 追加設定
  customRoutes: [
    { pattern: "/health", handler: "health_check" },
  ],
}
```

#### 環境別設定

```jsonnet
// 環境に応じた設定切り替え
local environment = std.extVar("ENVIRONMENT");

{
  config: {
    debug: environment != "production",
    port: if environment == "production" then 80 else 3000,
    database: {
      host: if environment == "production"
            then "prod-db.example.com"
            else "localhost",
    },
  },
}
```

### 開発ワークフロー

```bash
# 設定ファイルの検証
jsonnet eval config.kotoba

# 特定のセクションのみ取得
jsonnet eval -e "(import 'config.kotoba').routes"

# バリデーション実行
jsonnet eval -e "(import 'config.kotoba').validate()"

# 設定をJSONとして保存
jsonnet eval config.kotoba > config.json
```

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

### Kotoba CLI

Kotoba CLIはDeno CLIを参考にした使いやすいコマンドラインインターフェースを提供します。グラフ処理、クエリ実行、ファイル操作などをサポートしています。

#### インストール

```bash
# ビルドしてインストール
cargo build --release --features binary
cp target/release/kotoba ~/.local/bin/  # またはPATHの通った場所に
```

#### 基本的な使用方法

```bash
# ヘルプ表示
kotoba --help

# プロジェクト情報表示
kotoba info
kotoba info --detailed --json

# GQLクエリ実行
kotoba query "MATCH (n) RETURN n" --format json

# ファイル実行
kotoba run myfile.kotoba

# ファイル検証
kotoba check src/
kotoba check --all

# ファイルフォーマット
kotoba fmt src/
kotoba fmt --all --check

# サーバー起動
kotoba server --port 3000 --host 127.0.0.1

# 新規プロジェクト初期化
kotoba init my-project --template web

# ドキュメント生成
kotoba doc --output ./docs --format html

# バージョン表示
kotoba version
```

#### 主なコマンド

| コマンド | 説明 |
|---------|------|
| `run <file.kotoba>` | .kotobaファイルを実行 |
| `server --config <file.kotoba>` | HTTPサーバーを起動 |
| `query "MATCH..." --graph <file>` | GQLクエリを直接実行 |
| `check <file.kotoba>` | .kotobaファイルを検証 |
| `fmt <file.kotoba>` | .kotobaファイルをフォーマット |
| `info` | プロジェクト情報を表示 |
| `repl` | インタラクティブGQL REPL |
| `init <project>` | 新規.kotobaプロジェクトを初期化 |
| `version` | バージョン情報を表示 |

#### グローバルオプション

| オプション | 説明 |
|-----------|------|
| `-c, --config <CONFIG>` | 設定ファイルパス |
| `-l, --log-level <LEVEL>` | ログレベル (info, debug, warn, error) |
| `-C, --cwd <DIR>` | 作業ディレクトリ |
| `-h, --help` | ヘルプ表示 |
| `-V, --version` | バージョン表示 |

#### 使用例

```bash
# .kotobaファイルを実行
kotoba run app.kotoba

# ウォッチモードで開発（ファイル変更時に自動再実行）
kotoba run app.kotoba --watch

# サーバーモードで起動
kotoba server --config server.kotoba --port 3000

# ファイルを検証
kotoba check app.kotoba

# ファイルをフォーマット
kotoba fmt app.kotoba

# インタラクティブREPLでクエリを実行
kotoba repl

# 新規プロジェクトを作成
kotoba init my-project --template web

# プロジェクト情報を表示
kotoba info --detailed
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
