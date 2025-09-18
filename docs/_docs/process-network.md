---
layout: default
title: Process Network Model
---

# Process Network Graph Model

Kotobaプロジェクトの基盤となる**Process Network Graph Model**について説明します。このモデルは、プロジェクト全体の依存関係とビルド順序を管理し、計算可能性を保証します。

## 🎯 概要

Process Network Graph Modelは、以下の特徴を持つプロセスネットワークグラフです：

- **階層構造**: ノードは再帰的なサブグラフとして定義
- **Merkle DAG**: 各ノードはMerkle DAGで表現
- **トポロジカルソート**: ビルド順序の決定
- **逆トポロジカルソート**: 問題解決順序の決定

## 📊 ネットワーク構造

### ノード定義

各ノードは以下のプロパティを持ちます：

```jsonnet
{
  nodes: {
    'component_name': {
      name: 'component_name',
      path: 'crates/kotoba-component/src/lib.rs',
      type: 'component_type',
      description: 'コンポーネントの説明',
      dependencies: ['dependency1', 'dependency2'],
      provides: ['feature1', 'feature2'],
      status: 'planned|in_progress|completed',
      build_order: 1,
    },
  },
}
```

### エッジ定義

ノード間の依存関係を定義：

```jsonnet
{
  edges: [
    { from: 'component_a', to: 'component_b' },
    { from: 'component_b', to: 'component_c' },
  ],
}
```

## 🏗️ ビルド順序 (Topological Sort)

### 現在のビルド順序

```jsonnet
topological_order: [
  'types',
  'jsonnet_error',
  'ir_catalog',
  'ir_rule',
  'ir_query',
  'ir_patch',
  'graph_vertex',
  'graph_edge',
  'jsonnet_value',
  'cid_system',
  'schema_validator',
  'ir_strategy',
  'frontend_component_ir',
  'docs_parser',
  'docs_config',
  'jsonnet_ast',
  'jsonnet_lexer',
  'graph_core',
  'storage_main',
  'db_core',
  'db_engine_memory',
  'db_engine_lsm',
  'db_cluster',
  // ... 続く
]
```

### ビルド順序の決定

1. **依存関係の解析**: 各ノードのdependenciesを収集
2. **サイクルの検出**: 循環依存がないことを確認
3. **トポロジカルソート**: Kahn's algorithmを使用
4. **順序の検証**: 各ノードのbuild_orderプロパティを設定

## 🔍 問題解決順序 (Reverse Topological Sort)

### 逆トポロジカルソート

```jsonnet
reverse_topological_order: [
  'db',
  'db_engine_memory',
  'db_core',
  // ... 逆順
  'types',
  'cid_system',
]
```

### 問題解決の流れ

1. **エラーの特定**: 問題が発生したノードを特定
2. **逆順追跡**: 依存関係を逆順に追跡
3. **因果関係の特定**: 根本原因を特定
4. **修正と再構築**: 修正後に順序通りに再構築

## 📋 ノードタイプ

### 基盤層 (Foundation)

- **types**: 共通型定義
- **jsonnet_error**: Jsonnet評価エラー定義

### IR層 (Intermediate Representation)

- **ir_catalog**: スキーマ/索引/不変量定義
- **ir_rule**: DPO型付き属性グラフ書換えルール
- **ir_query**: GQL論理プラン代数
- **ir_patch**: 差分表現
- **ir_strategy**: 戦略表現

### グラフ層 (Graph Layer)

- **graph_vertex**: 頂点関連構造体
- **graph_edge**: エッジ関連構造体
- **graph_core**: 列指向グラフ表現

### ストレージ層 (Storage Layer)

- **storage_mvcc**: MVCCマネージャー
- **storage_merkle**: Merkle DAG永続化
- **storage_lsm**: LSM-Treeベース高性能ストレージ

### プランナー層 (Planner Layer)

- **planner_logical**: 論理プランナー
- **planner_physical**: 物理プランナー
- **planner_optimizer**: クエリ最適化器

### 実行層 (Execution Layer)

- **execution_parser**: GQLパーサー
- **execution_engine**: クエリ実行器

### 書換え層 (Rewrite Layer)

- **rewrite_matcher**: ルールマッチング
- **rewrite_applier**: ルール適用
- **rewrite_engine**: DPO書換えエンジン

### セキュリティ層 (Security Layer)

- **security_jwt**: JWTトークン管理
- **security_oauth2**: OAuth2統合
- **security_core**: セキュリティ統合サービス

### Jsonnet層 (Jsonnet Layer)

- **jsonnet_error**: エラー定義
- **jsonnet_value**: 値型定義
- **jsonnet_ast**: 抽象構文木
- **jsonnet_lexer**: 字句解析器
- **jsonnet_parser**: 構文解析器
- **jsonnet_evaluator**: 評価器
- **jsonnet_stdlib**: 標準ライブラリ
- **jsonnet_core**: コアAPI

### Kotoba拡張層 (Kotoba Extensions)

- **kotobanet_error**: Kotobaエラー定義
- **kotobanet_http_parser**: HTTP設定パーサー
- **kotobanet_frontend**: フロントエンドパーサー
- **kotobanet_deploy**: デプロイ設定パーサー
- **kotobanet_core**: Kotobaコア統合

### ドキュメント層 (Documentation Layer)

- **docs_parser**: 多言語ソースコードパーサー
- **docs_config**: ドキュメント設定管理
- **docs_generator**: ドキュメント生成エンジン
- **docs_core**: ドキュメントコアAPI

### データベース層 (Database Layer)

- **db_core**: コアDBトレイト
- **db_engine_memory**: メモリストレージエンジン
- **db_engine_lsm**: LSM-Treeストレージエンジン
- **db**: ユーザーAPI

## 🔗 依存関係グラフ

### 主要な依存関係

```jsonnet
// types -> すべて
{ from: 'types', to: 'ir_catalog' },
{ from: 'types', to: 'schema_validator' },
{ from: 'types', to: 'graph_vertex' },
{ from: 'types', to: 'graph_edge' },

// IR相互依存
{ from: 'ir_catalog', to: 'schema_validator' },
{ from: 'ir_rule', to: 'rewrite_matcher' },
{ from: 'ir_patch', to: 'rewrite_applier' },

// グラフ層依存
{ from: 'graph_vertex', to: 'graph_core' },
{ from: 'graph_edge', to: 'graph_core' },

// ストレージ依存
{ from: 'graph_core', to: 'storage_mvcc' },
{ from: 'graph_core', to: 'storage_merkle' },

// Jsonnet依存
{ from: 'jsonnet_error', to: 'jsonnet_value' },
{ from: 'jsonnet_value', to: 'jsonnet_ast' },
{ from: 'jsonnet_ast', to: 'jsonnet_parser' },
{ from: 'jsonnet_parser', to: 'jsonnet_core' },
```

## 🛠️ ユーティリティ関数

### 依存関係取得

```jsonnet
// 指定されたノードの依存関係を取得
get_dependencies(node_name):: [
  edge.from for edge in self.edges if edge.to == node_name
]

// 指定されたノードが依存しているノードを取得
get_dependents(node_name):: [
  edge.to for edge in self.edges if edge.from == node_name
]
```

### ノード情報取得

```jsonnet
// 指定されたノードの情報を取得
get_node(node_name):: self.nodes[node_name]

// 指定されたタイプのノードを取得
get_nodes_by_type(node_type):: [
  node for node in std.objectValues(self.nodes) if node.type == node_type
]
```

### ソート関数

```jsonnet
// ビルド順序でソートされたノードを取得
get_nodes_in_build_order():: [
  self.nodes[name] for name in self.topological_order
]

// 問題解決順序でソートされたノードを取得
get_nodes_in_problem_resolution_order():: [
  self.nodes[name] for name in self.reverse_topological_order
]
```

## 🔍 DAG検証

### 循環依存チェック

```jsonnet
validate_dag():: {
  local node_names = std.objectFields(self.nodes);
  local edge_count = std.length(self.edges);
  local expected_edges = std.length(node_names) - 1;
  if edge_count > expected_edges then
    error '循環依存の可能性があります'
  else
    'DAGは有効です'
}
```

## 📊 ステータスサマリー

```jsonnet
get_status_summary():: {
  completed: std.length([n for n in std.objectValues(self.nodes) if n.status == 'completed']),
  total: std.length(std.objectValues(self.nodes)),
  completion_rate: completed / total * 100,
}
```

## 🎨 使用例

### 依存関係の確認

```bash
# 特定のコンポーネントの依存関係を確認
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependencies('execution_engine')"

# 特定のコンポーネントに依存するコンポーネントを確認
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependents('types')"
```

### ビルド順序の確認

```bash
# 全体のビルド順序を取得
jsonnet eval dag.jsonnet | jq .topological_order[]

# 特定のノードのビルド順序を取得
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_build_order('graph_core')"
```

### 問題解決順序

```bash
# 問題が発生した場合の調査順序
jsonnet eval dag.jsonnet | jq .reverse_topological_order[]
```

## 🔧 メンテナンス

### ノードの追加

1. `dag.jsonnet`に新しいノードを追加
2. 適切な依存関係を`edges`に追加
3. `build_order`を更新
4. トポロジカルソートを再計算

### 依存関係の変更

1. 関連する`edges`を更新
2. 循環依存がないことを確認
3. 影響を受けるノードの`build_order`を更新

### ステータスの更新

```jsonnet
// ノードのステータスを更新
nodes: {
  'my_component': {
    // ... 他のプロパティ
    status: 'completed',  // planned -> in_progress -> completed
  },
}
```

## 🎯 利点

### 計算可能性の保証

- **トポロジカルソート**: 正しいビルド順序を保証
- **依存関係追跡**: 変更の影響を正確に把握
- **並行処理**: 独立したノードを並行して処理

### 問題解決の効率化

- **逆トポロジカルソート**: 問題の根本原因を迅速に特定
- **影響範囲の特定**: 変更による影響を予測
- **復旧戦略**: 効率的な問題解決手順

### 保守性の向上

- **構造化された依存関係**: 明確なコンポーネント関係
- **自動検証**: DAGの整合性を自動チェック
- **ドキュメント化**: プロジェクト構造の自動文書化

## 📈 拡張性

### 新しいノードタイプの追加

```jsonnet
nodes: {
  'new_feature': {
    name: 'new_feature',
    path: 'crates/kotoba-new-feature/src/lib.rs',
    type: 'new_type',
    description: '新しい機能の説明',
    dependencies: ['types', 'graph_core'],
    provides: ['new_feature_api'],
    status: 'planned',
    build_order: 99,
  },
}
```

### 動的依存関係

```jsonnet
// 条件付き依存関係
get_conditional_dependencies(env):: {
  dependencies: if env == 'production' then
    ['types', 'storage_lsm']
  else
    ['types', 'storage_memory'],
}
```

## 🔍 高度なクエリ

### パス検索

```jsonnet
// 2つのノード間のパスを検索
find_path(from, to):: {
  // BFSまたはDFSによるパス検索の実装
}
```

### 影響分析

```jsonnet
// ノード変更時の影響を受けるノードを特定
get_impact_zone(node_name):: {
  // 再帰的に依存関係を追跡
}
```

---

Process Network Graph Modelは、Kotobaプロジェクトの複雑な依存関係を管理し、安定したビルドと効率的な問題解決を実現する基盤となります。
