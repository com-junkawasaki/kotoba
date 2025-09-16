# Kotoba Network

ネットワーク通信プロトコル実装 for Kotoba distributed system.

## 概要

Kotoba Network は、Kotoba の分散システムにおけるノード間通信を担当するクレートです。TCP/IP ベースの通信プロトコルを実装し、分散タスクの実行、キャッシュ同期、クラスタ管理などの機能をサポートします。

## 主な機能

- **ネットワークプロトコル**: 構造化されたメッセージ交換
- **接続管理**: TCP 接続の確立と管理
- **メッセージハンドリング**: 非同期メッセージ処理
- **クラスタ通信**: ノード間でのタスク分散と結果収集
- **キャッシュ同期**: 分散キャッシュの一貫性維持

## 主な構造体

- **NetworkMessage**: ネットワークメッセージの列挙型
- **NetworkManager**: ネットワーク通信の管理
- **NetworkServer**: TCP サーバーの実装
- **MessageHandler**: メッセージ処理ハンドラー
- **TcpConnectionManager**: TCP 接続管理

## 使用例

```rust
use kotoba_network::{ServerBuilder, NodeId};
use kotoba_distributed::DistributedEngine;

// 分散実行エンジンを作成
let node_id = NodeId("node_1".to_string());
let engine = std::sync::Arc::new(DistributedEngine::new(node_id.clone()));

// ネットワークサーバーを作成
let server = ServerBuilder::new()
    .listen_addr("127.0.0.1:8080".to_string())
    .node_id(node_id)
    .distributed_engine(engine)
    .build()
    .await?;

// サーバーを起動
server.run().await?;
```

## メッセージタイプ

- **TaskRequest**: 分散タスクの実行リクエスト
- **TaskResponse**: タスク実行結果のレスポンス
- **Heartbeat**: ノードの生存確認
- **JoinRequest**: クラスタ参加リクエスト
- **CacheSync**: キャッシュ同期メッセージ
- **GraphTransfer**: グラフデータの転送

## 依存関係

- `kotoba-core`: 基本型定義
- `kotoba-distributed`: 分散実行エンジン
- `tokio`: 非同期ランタイム
- `serde`: シリアライズ/デシリアライズ
