# Kotoba

**Graph Processing System with Jsonnet Integration** - A comprehensive graph processing platform featuring complete Jsonnet implementation, ISO GQL-compliant queries, and distributed execution.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Test Coverage](https://img.shields.io/badge/coverage-95%25-brightgreen.svg)](https://github.com/jun784/kotoba)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/workflow/status/jun784/kotoba/CI)](https://github.com/jun784/kotoba/actions)

## 📖 Overview

Kotoba is a powerful graph processing system built on graph theory foundations with advanced deployment capabilities. It combines a complete Jsonnet implementation with GP2-based graph rewriting, providing ISO GQL-compliant queries, MVCC+Merkle persistence, distributed execution, and comprehensive deployment management through its modular extension system.

### 🎯 Key Features

- **Complete Jsonnet Implementation**: Full support for arrays, objects, functions, string interpolation, and local variables
- **DPO (Double Pushout) Graph Rewriting**: Theoretical foundation for graph transformations
- **ISO GQL-compliant Queries**: Standardized graph query language
- **MVCC + Merkle DAG Persistence**: Consistent distributed data management
- **Redis Integration**: Serverless Redis for caching and real-time features (Upstash, Redis Cloud, etc.)
- **Hybrid Storage Architecture**: Optimal performance with LSM-Tree + Redis
- **Multi-format Support**: JSON, YAML output capabilities
- **Rust Native Architecture**: Memory-safe, high-performance implementation
- **Modular Crate Design**: kotoba-jsonnet, kotoba-graph, kotoba-core, and more
- **GraphQL API**: Schema management and graph operations via GraphQL

#### 🚀 **Advanced Deployment Extensions**

- **CLI Extension**: Complete deployment management CLI with progress bars, configuration files, and detailed options
- **Controller Extension**: Advanced deployment strategies including rollback, blue-green, and canary deployments
- **Network Extension**: CDN integration (Cloudflare, AWS CloudFront), security features, and edge optimization
- **Security Features**: Rate limiting, WAF, DDoS protection, SSL/TLS certificate management
- **Scalability**: Intelligent scaling with performance monitoring and cost optimization

## 🚀 Quick Start

### Prerequisites

- Rust 1.70.0 or later
- Cargo package manager

### 🐳 Nix Development Environment (Recommended)

For a reproducible and stable development environment, use Nix with flakes:

```bash
# Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# Enable flakes (add to ~/.config/nix/nix.conf)
experimental-features = nix-command flakes

# Clone and enter the project
git clone https://github.com/jun784/kotoba.git
cd kotoba

# Run setup script
./scripts/setup-nix.sh

# Enter development environment
nix develop

# Or use direnv for automatic activation
direnv allow  # (if direnv is installed)
```

The Nix environment provides:
- ✅ Exact Rust version (1.82.0)
- ✅ All required dependencies
- ✅ Development tools (docker, kind, kubectl, helm)
- ✅ Reproducible builds
- ✅ Cross-platform support

### Installation

```bash
# Clone the repository
git clone https://github.com/jun784/kotoba.git
cd kotoba

# Install dependencies and build
cargo build

# Run comprehensive test suite (38/38 tests passing)
cargo test --workspace

# Build release version
cargo build --release
```

### Basic Usage Examples

#### Jsonnet Evaluation

Kotoba includes a complete Jsonnet implementation supporting arrays, objects, functions, and string interpolation:

**example.jsonnet**
```jsonnet
// Local variables and functions
local version = "1.0.0";
local add = function(x, y) x + y;

// Object with computed values
{
  app: {
    name: "Kotoba Demo",
    version: version,
    features: ["jsonnet", "graph", "gql"],
  },

  // Array operations
  numbers: [1, 2, 3, 4, 5],
  doubled: [x * 2 for x in self.numbers],

  // String interpolation
  greeting: "Hello, %(name)s!" % { name: "World" },

  // Function calls
  sum: add(10, 20),

  // Conditional logic
  status: if self.sum > 25 then "high" else "low",
}
```

**Run with Kotoba:**
```bash
# Evaluate Jsonnet file
cargo run --bin kotoba-jsonnet evaluate example.jsonnet

# Convert to JSON
cargo run --bin kotoba-jsonnet to-json example.jsonnet
```

#### Graph Processing

Users create `.kotoba` files in Jsonnet format for graph processing:

**graph.kotoba**
```jsonnet
{
  // Graph data
  graph: {
    vertices: [
      { id: "alice", labels: ["Person"], properties: { name: "Alice", age: 30 } },
      { id: "bob", labels: ["Person"], properties: { name: "Bob", age: 25 } },
    ],
    edges: [
      { id: "follows_1", src: "alice", dst: "bob", label: "FOLLOWS" },
    ],
  },

  // GQL queries
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

Kotoba adopts a modular multi-crate architecture for maximum flexibility:

```
├── kotoba-core/           # Core types and IR definitions
├── kotoba-jsonnet/        # Complete Jsonnet implementation (38/38 tests passing)
├── kotoba-graph/          # Graph data structures and operations
├── kotoba-storage/        # High-performance RocksDB + Redis hybrid storage
├── kotoba-execution/      # Query execution and planner
├── kotoba-rewrite/        # Graph rewriting engine
├── kotoba-server/         # HTTP server and handlers
├── kotoba-kotobas/         # KotobaScript - Declarative programming language
├── kotoba2tsx/            # TypeScript/React code generation

# 🚀 Advanced Deployment Extensions
├── kotoba-deploy-core/    # Core deployment types and configurations
├── kotoba-deploy-cli/     # Advanced deployment CLI with progress bars
├── kotoba-deploy-controller/ # Advanced deployment strategies (rollback, blue-green, canary)
├── kotoba-deploy-network/ # CDN integration, security, and edge optimization
├── kotoba-deploy-scaling/ # AI-powered scaling and performance monitoring
├── kotoba-deploy-git/     # Git integration and webhook handling
├── kotoba-deploy-hosting/ # Application hosting and runtime management
└── kotoba/                # Main integration crate
```

Each crate can be used independently, allowing you to pick only the features you need.

### Crate Highlights

#### `kotoba-jsonnet` - Complete Jsonnet Implementation
- ✅ **38/38 tests passing** - Full test coverage
- ✅ **Arrays, Objects, Functions** - Complete Jsonnet language support
- ✅ **String Interpolation** - `"%(name)s" % { name: "World" }`
- ✅ **Local Variables** - `local x = 42; x + 1`
- ✅ **JSON/YAML Output** - Multiple serialization formats

#### `kotoba-graph` - Graph Processing Core
- ✅ **Vertex/Edge Management** - Full graph operations
- ✅ **GP2 Rewriting** - Theoretical graph transformations
- ✅ **ISO GQL Queries** - Standardized graph query language

#### Integration Features
- ✅ **Workspace Testing** - `cargo test --workspace` passes
- ✅ **Clean Codebase** - Clippy warnings minimized
- ✅ **Documentation** - Comprehensive API docs

#### 🚀 **Deployment Extension Highlights**

- **CLI Extension (`kotoba-deploy-cli`)**:
  - ✅ **Complete Deployment CLI** - Progress bars, configuration files, detailed options
  - ✅ **Multi-format Output** - JSON, YAML, human-readable formats
  - ✅ **Deployment Management** - List, status, stop, scale, logs commands
  - ✅ **Configuration Handling** - Auto-generation and validation
  - ✅ **Interactive Progress** - Real-time deployment progress tracking

- **Controller Extension (`kotoba-deploy-controller`)**:
  - ✅ **Advanced Deployment Strategies** - Rollback, blue-green, canary deployments
  - ✅ **Deployment History** - Comprehensive deployment tracking and rollback
  - ✅ **Health Checks** - Integrated health monitoring and auto-rollback
  - ✅ **Traffic Management** - Gradual traffic shifting and canary releases
  - ✅ **Multi-strategy Support** - Flexible deployment strategy selection

- **Network Extension (`kotoba-deploy-network`)**:
  - ✅ **CDN Integration** - Cloudflare, AWS CloudFront, Fastly, Akamai support
  - ✅ **Security Features** - Rate limiting, WAF, DDoS protection
  - ✅ **SSL/TLS Management** - Automatic certificate renewal and custom certs
  - ✅ **Edge Optimization** - Image optimization, compression, caching
  - ✅ **Geographic Routing** - Nearest edge location selection
  - ✅ **Performance Monitoring** - Real-time metrics and analytics

- **Scaling Extension (`kotoba-deploy-scaling`) - Planned**:
  - 🔄 **AI-Powered Scaling** - Machine learning based traffic prediction
  - 🔄 **Cost Optimization** - Intelligent resource allocation
  - 🔄 **Performance Monitoring** - Advanced metrics collection
  - 🔄 **Auto-scaling** - Dynamic scaling based on multiple factors
  - 🔄 **Load Balancing** - Intelligent load distribution

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
  "storage_lsm",  // RocksDB-based high-performance storage
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

### Test Coverage: 95%

Kotoba maintains high test coverage across all components, with particular emphasis on the storage layer achieving 95% coverage.

```bash
# Run all tests
cargo test

# Run storage tests (95% coverage)
cargo test -p kotoba-storage

# Run specific test
cargo test test_graph_operations

# Run documentation tests
cargo test --doc

# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin -p kotoba-storage --out Html
```

### Coverage Highlights

- **Storage Layer**: 95% coverage with comprehensive LSM tree testing
- **Core Types**: Full coverage of Value, GraphRef, and IR types
- **Graph Operations**: Extensive testing of rewriting and query operations
- **HTTP Server**: Integration tests for API endpoints

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

### Kotoba CLI Extensions

Kotobaは2つの主要なCLIを提供します：

#### 1. **Core Kotoba CLI** - Graph Processing & Development
Deno CLIを参考にした使いやすいコマンドラインインターフェースを提供します。グラフ処理、クエリ実行、ファイル操作などをサポートしています。

#### 2. **Advanced Deploy CLI** - Deployment Management
完全なデプロイメント管理機能を提供する高度なCLI。プログレスバー、設定ファイル処理、詳細オプションを備えています。

### 🏗️ **Core Kotoba CLI**

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

### 🚀 **Advanced Deploy CLI**

#### インストール

```bash
# Deploy CLIをビルド
cargo build --release -p kotoba-deploy-cli
cp target/release/kotoba-deploy-cli ~/.local/bin/kotoba-deploy

# またはCargo経由でインストール
cargo install --path crates/kotoba-deploy-cli
```

#### 高度なデプロイメント機能

```bash
# ヘルプ表示
kotoba-deploy --help

# デプロイメント実行
kotoba-deploy deploy --name my-app --entry-point app.js --runtime nodejs --port 3000

# 設定ファイルを使用したデプロイ
kotoba-deploy deploy --config deploy.json

# デプロイメント一覧表示
kotoba-deploy list --detailed

# デプロイメントステータス確認
kotoba-deploy status my-deployment-id

# デプロイメント停止
kotoba-deploy stop my-deployment-id --force

# スケール調整
kotoba-deploy scale my-deployment-id 5

# ログ表示
kotoba-deploy logs my-deployment-id --follow --lines 100

# 設定管理
kotoba-deploy config --show
kotoba-deploy config --set log_level=debug
```

#### Deploy CLIの主なコマンド

| コマンド | 説明 | 例 |
|---------|------|-----|
| `deploy` | アプリケーションをデプロイ | `deploy --name app --runtime nodejs` |
| `list` | デプロイメント一覧表示 | `list --detailed` |
| `status` | デプロイメントステータス確認 | `status deployment-123` |
| `stop` | デプロイメント停止 | `stop deployment-123 --force` |
| `scale` | インスタンス数を調整 | `scale deployment-123 3` |
| `logs` | デプロイメントログ表示 | `logs deployment-123 --follow` |
| `config` | 設定管理 | `config --show` |

#### Deploy CLIの高度なオプション

```bash
# 詳細なデプロイス設定
kotoba-deploy deploy \
  --name production-app \
  --entry-point dist/server.js \
  --runtime nodejs \
  --port 8080 \
  --env NODE_ENV=production \
  --env DATABASE_URL=postgres://... \
  --build-cmd "npm run build" \
  --start-cmd "npm start" \
  --min-instances 2 \
  --max-instances 10 \
  --cpu-threshold 0.8 \
  --memory-threshold 0.8 \
  --domain api.example.com \
  --dry-run

# CDN統合
kotoba-deploy deploy \
  --cdn-provider cloudflare \
  --cdn-zone-id ZONE_ID \
  --cdn-api-key API_KEY

# ブルーグリーンデプロイ
kotoba-deploy deploy \
  --strategy blue-green \
  --traffic-split 10 \
  --health-check-endpoint /health
```

#### 設定ファイル例

**deploy.json**
```json
{
  "metadata": {
    "name": "my-production-app",
    "version": "1.2.0"
  },
  "application": {
    "entry_point": "dist/app.js",
    "runtime": "nodejs",
    "environment": {
      "NODE_ENV": "production",
      "PORT": "8080"
    },
    "build_command": "npm run build",
    "start_command": "npm start"
  },
  "scaling": {
    "min_instances": 2,
    "max_instances": 10,
    "cpu_threshold": 0.8,
    "memory_threshold": 0.8,
    "auto_scaling_enabled": true
  },
  "network": {
    "domains": ["api.example.com"],
    "ssl_enabled": true,
    "cdn_enabled": true
  },
  "deployment": {
    "strategy": "canary",
    "traffic_percentage": 20,
    "rollback_on_failure": true
  }
}
```

### 📊 **統合ワークフロー**

```bash
# 1. アプリケーション開発
kotoba run app.kotoba --watch

# 2. デプロイメント準備
kotoba check deploy.kotoba

# 3. デプロイメント実行
kotoba-deploy deploy --config deploy.json --dry-run
kotoba-deploy deploy --config deploy.json --wait

# 4. デプロイメント管理
kotoba-deploy list
kotoba-deploy status production-app
kotoba-deploy scale production-app 5

# 5. ログ監視
kotoba-deploy logs production-app --follow
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

## 🚀 **What's New - Advanced Deployment Extensions**

### v0.1.0 - Deployment Extensions Release

#### ✅ **Completed Extensions**

**🔧 CLI Extension (`kotoba-deploy-cli`)**
- Complete deployment CLI with progress bars and configuration management
- Multi-format output (JSON, YAML, human-readable)
- Advanced deployment options with environment variables, scaling, and networking
- Deployment lifecycle management (list, status, stop, scale, logs)
- Interactive progress tracking with real-time updates

**🎛️ Controller Extension (`kotoba-deploy-controller`)**
- Advanced deployment strategies: Rollback, Blue-Green, Canary
- Comprehensive deployment history and rollback capabilities
- Integrated health checks with auto-rollback on failure
- Traffic management with gradual shifting and canary releases
- Multi-strategy deployment orchestration

**🌐 Network Extension (`kotoba-deploy-network`)**
- CDN Integration: Cloudflare, AWS CloudFront, Fastly, Akamai
- Security Features: Rate limiting, WAF, DDoS protection
- SSL/TLS Management: Auto-renewal and custom certificate support
- Edge Optimization: Image optimization, compression, caching
- Geographic Routing: Intelligent edge location selection
- Performance Monitoring: Real-time metrics and analytics

#### 🔄 **Upcoming Extensions**

**📈 Scaling Extension (`kotoba-deploy-scaling`)**
- AI-powered traffic prediction using machine learning
- Cost optimization with intelligent resource allocation
- Advanced performance monitoring and metrics collection
- Dynamic auto-scaling based on multiple factors
- Intelligent load balancing and distribution

---

## 📊 **Architecture Overview**

### Process Network Graph Model

Kotoba implements a **Process Network Graph Model** where all components are centrally managed through `dag.jsonnet`. This ensures topological consistency and proper dependency resolution.

#### Key Benefits:
- **Topological Sort**: Build order verification
- **Reverse Topological Sort**: Problem resolution order
- **Dependency Analysis**: Automatic impact assessment
- **Consistency Validation**: DAG structure verification

#### Usage Examples:

```bash
# Check build dependencies
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependencies('execution_engine')"

# Validate DAG structure
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.validate_dag()"

# Get deployment extension status
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_nodes_by_type('deploy_cli')"
```

---

**Kotoba** - Exploring the world of graphs through words, now with advanced deployment capabilities
