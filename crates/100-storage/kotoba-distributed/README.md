# Kotoba Distributed

分散実行システム for Kotoba graph processing system.

## 概要

Kotoba Distributed は、Kotoba のグラフ処理を分散環境で実行するためのクレートです。CIDベースのキャッシュとタスク分散により、高いパフォーマンスを実現します。

## 主な機能

- **分散実行エンジン**: グラフ処理タスクの分散実行
- **CIDキャッシュ**: コンテンツIDベースのキャッシュシステム
- **クラスタ管理**: 分散ノードの管理と負荷分散
- **タスクスケジューリング**: 優先度ベースのタスク分散

## 使用例

```rust
use kotoba_distributed::{DistributedEngine, NodeId};

// 分散実行エンジンの作成
let node_id = NodeId("node_1".to_string());
let engine = DistributedEngine::new(node_id);

// ルール適用の分散実行
let result = engine.apply_rule_distributed(&rule, &graph, &mut cid_manager).await?;
```

## アーキテクチャ

- **DistributedEngine**: メインの分散実行エンジン
- **CidCache**: CIDベースのキャッシュマネージャー
- **ClusterManager**: クラスタ内のノード管理
- **LoadBalancer**: タスクの負荷分散

## 依存関係

- `kotoba-core`: 基本型定義
- `kotoba-graph`: グラフデータ構造
- `kotoba-execution`: クエリ実行
- `tokio`: 非同期ランタイム
