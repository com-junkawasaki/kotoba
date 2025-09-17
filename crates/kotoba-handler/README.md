# Kotoba Handler - Unified Integration

`kotoba-handler` はKotobaエコシステム全体の統合的なhandlerを提供するクレートです。

## 統合アーキテクチャ

### 既存クレートとの統合

#### 1. kotoba-jsonnet との統合
```rust
// Jsonnet評価機能の統合
#[cfg(feature = "jsonnet-integration")]
use kotoba_handler::integration::jsonnet_integration::JsonnetEvaluationHandler;

let handler = JsonnetEvaluationHandler::new();
let result = handler.evaluate(jsonnet_content, &context)?;
```

#### 2. kotoba-kotobas との統合
```rust
// HTTP設定パーサーの統合
use kotoba_handler::integration::kotobas_integration::KotobasHttpHandler;

let handler = KotobasHttpHandler::new(kotobas_config)?;
let route = handler.find_route("GET", "/api/users")?;
```

#### 3. kotoba-server との統合
```rust
// HTTPサーバー機能の統合
use kotoba_handler::server;

server::run_server("127.0.0.1:3000").await?;
```

## 使用方法

### 基本的な使用例

```rust
use kotoba_handler::{UnifiedHandler, HandlerContext, execute_simple_handler};

// シンプルな実行
let context = HandlerContext {
    method: "GET".to_string(),
    path: "/api/test".to_string(),
    headers: HashMap::new(),
    query_params: HashMap::new(),
    body: None,
    environment: HashMap::new(),
};

let result = execute_simple_handler(kotoba_content, context).await?;
```

### 統合handlerの使用

```rust
use kotoba_handler::integration::IntegratedHandler;

// JsonnetとKotobasの両方を統合
let mut handler = IntegratedHandler::new(kotobas_config)?;
let result = handler.process_request(context, Some(jsonnet_content)).await?;
```

### Executorを使用した実行

```rust
use kotoba_handler::{HandlerExecutor, ExecutionMode};

let executor = HandlerExecutor::new(handler_arc)
    .with_mode(ExecutionMode::Async);

let result = executor.execute_batch(requests).await?;
```

## 機能

### コア機能
- **Unified Handler**: すべてのKotoba操作の統一インターフェース
- **Multiple Runtimes**: Native, WASM, Node.js, Deno, Browser
- **Execution Modes**: Sync, Async, Streaming
- **Caching**: 自動結果キャッシュ
- **Configuration**: 柔軟な設定管理

### 統合機能
- **Jsonnet Integration**: オプションのJsonnet評価（後方互換）
- **Kotobas Integration**: HTTP設定パーサー
- **Server Integration**: HTTPサーバー統合
- **CLI Integration**: コマンドライン統合
- **WASM Integration**: ブラウザ実行

## 設定

### Cargo.toml
```toml
[dependencies]
kotoba-handler = { git = "https://github.com/jun784/kotoba", features = ["full"] }
```

### 機能フラグ
- `default`: CLIとサーバー機能
- `cli`: CLI統合
- `server`: HTTPサーバー統合
- `wasm`: WASM実行環境
- `websocket`: WebSocketサポート
- `jsonnet-integration`: Jsonnet統合（オプション）
- `full`: すべての機能

## 統合フロー

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   densha        │────│ kotoba-handler   │────│ kotoba-*        │
│                 │    │                  │    │ crates          │
│ - CLI Interface │    │ - Unified API    │    │                 │
│ - HTTP Server   │    │ - Runtime Mgmt   │    │ - jsonnet       │
│ - File System   │    │ - Caching        │    │ - kotobas       │
└─────────────────┘    │ - Integration    │    │ - server        │
                       └──────────────────┘    │ - 2tsx          │
                                               └─────────────────┘
```

## 後方互換性

- Jsonnet統合はオプション機能として提供
- 既存のkotoba-jsonnet, kotoba-kotobasのAPIは維持
- 段階的な移行が可能

## 利点

1. **統合インターフェース**: すべてのKotoba機能を統一的に扱える
2. **柔軟な実行環境**: 複数のランタイムをサポート
3. **パフォーマンス最適化**: キャッシュとストリーミング
4. **開発効率向上**: シンプルなAPIで複雑な処理が可能
5. **将来拡張性**: 新しい統合機能の容易な追加
