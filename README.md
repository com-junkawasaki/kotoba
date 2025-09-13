# Kotoba (言葉)

**GP2系グラフ書換え言語** - ISO GQL準拠クエリ、MVCC+Merkle永続、分散実行まで一貫させたグラフ処理システム

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/workflow/status/jun784/kotoba/CI)](https://github.com/jun784/kotoba/actions)

## 📖 概要

Kotobaは、グラフ理論に基づく強力なグラフ処理システムです。GP2 (Graph Programs 2) の書換えシステムを核に、ISO GQL準拠のクエリ言語、MVCC+Merkleツリーによる永続化、分散実行までを一貫して実装しています。

### 🎯 主な特徴

- **DPO (Double Pushout) 型付き属性グラフ書換え**: 理論的基盤のあるグラフ変換
- **ISO GQL準拠クエリ**: 標準化されたグラフクエリ言語
- **MVCC + Merkle DAG**: 一貫性のある分散永続化
- **列指向ストレージ**: LSMツリーによる効率的なデータアクセス
- **プロセスネットワークグラフモデル**: dag.jsonnetによる一元管理
- **Rustネイティブ**: メモリ安全で高性能

## 🚀 クイックスタート

### 必要条件

- Rust 1.70.0 以上
- Cargo パッケージマネージャー

### インストール

```bash
# リポジトリをクローン
git clone https://github.com/jun784/kotoba.git
cd kotoba

# 依存関係をインストール
cargo build

# テストを実行
cargo test

# CLIツールをビルド
cargo build --release
```

### 基本的な使用例

```rust
use kotoba::*;

fn main() -> Result<()> {
    // グラフを作成
    let mut graph = Graph::empty();

    // 頂点を追加
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

    // エッジを追加
    graph.add_edge(EdgeData {
        id: uuid::Uuid::new_v4(),
        src: v1,
        dst: v2,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    // GQLクエリを実行
    let gql = "MATCH (p:Person) RETURN p.name";
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();
    let results = executor.execute_gql(gql, &GraphRef::new(graph), &catalog)?;

    println!("Query results: {:?}", results);
    Ok(())
}
```

## 🏗️ アーキテクチャ

### プロセスネットワークグラフモデル

Kotobaは**プロセスネットワークグラフモデル**に基づいており、すべてのコンポーネントが`dag.jsonnet`で一元管理されています。

#### 主要コンポーネント

```
┌─────────────────────────────────────────────────────────────┐
│                          lib.rs                             │
│                    (メインライブラリ)                       │
├─────────────────────────────────────────────────────────────┤
│          execution/          │          rewrite/            │
│       (クエリ実行器)         │       (DPO書換え器)          │
├─────────────────────────────────────────────────────────────┤
│          planner/            │          storage/            │
│       (クエリプランナー)      │       (MVCC+Merkle)         │
├─────────────────────────────────────────────────────────────┤
│           graph/             │            ir/               │
│       (データ構造)           │       (中核IR)               │
├─────────────────────────────────────────────────────────────┤
│                          types.rs                           │
│                    (共通型定義)                            │
└─────────────────────────────────────────────────────────────┘
```

### ビルド順序 (トポロジカルソート)

```jsonnet
// dag.jsonnetからビルド順序を取得
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

## 📋 使用方法

### 1. 基本的なGQLクエリ

```rust
use kotoba::{QueryExecutor, Catalog, GraphRef};

// クエリ実行器を作成
let executor = QueryExecutor::new();
let catalog = Catalog::empty();

// GQLクエリを実行
let gql = r#"
    MATCH (p:Person)-[:FOLLOWS]->(f:Person)
    WHERE p.age > 20
    RETURN p.name, f.name
"#;

let results = executor.execute_gql(gql, &graph_ref, &catalog)?;
```

### 2. グラフ書換え

```rust
use kotoba::{RewriteEngine, RuleIR, StrategyIR};

// 書換えエンジンを作成
let engine = RewriteEngine::new();

// ルールを定義
let rule = RuleIR { /* ルール定義 */ };
let strategy = StrategyIR { /* 戦略定義 */ };

// 書換えを実行
let patch = engine.rewrite(&graph_ref, &rule, &strategy)?;
```

### 3. 手動によるグラフ操作

```rust
use kotoba::{Graph, VertexBuilder, EdgeBuilder};

// グラフを作成
let mut graph = Graph::empty();

// 頂点を追加
let v1 = graph.add_vertex(VertexBuilder::new()
    .label("Person")
    .prop("name", Value::String("Alice"))
    .build());

// エッジを追加
let e1 = graph.add_edge(EdgeBuilder::new()
    .src(v1)
    .dst(v2)
    .label("FOLLOWS")
    .build());
```

## 🛠️ 開発方法

### dag.jsonnetの利用

#### 1. 依存関係分析

```bash
# 特定のコンポーネントの依存関係を確認
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependencies('execution_engine')"

# 依存されているコンポーネントを確認
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependents('types')"
```

#### 2. ビルド順序の確認

```bash
# 全体のビルド順序を取得
jsonnet eval dag.jsonnet | jq .topological_order[]

# 特定のノードのビルド順序を確認
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_build_order('graph_core')"
```

#### 3. 問題解決時の因果特定

```bash
# 問題発生時の調査順序を取得
jsonnet eval dag.jsonnet | jq .reverse_topological_order[]
```

### lib.jsonnetの利用

#### 1. ビルド設定の確認

```bash
# 特定のターゲットの設定を取得
jsonnet eval -e "local lib = import 'lib.jsonnet'; lib.get_target_config('x86_64-apple-darwin')"

# コンポーネントの依存関係を解決
jsonnet eval -e "local lib = import 'lib.jsonnet'; lib.resolve_dependencies('kotoba-core', ['full'])"
```

#### 2. パッケージング設定

```bash
# Dockerイメージ設定を取得
jsonnet eval lib.jsonnet | jq .packaging.docker

# Debianパッケージ設定を取得
jsonnet eval lib.jsonnet | jq .packaging.debian
```

### 開発ワークフロー

```bash
# 1. コード変更
vim src/some_component.rs

# 2. 依存関係を確認
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.get_dependencies('some_component')"

# 3. テストを実行
cargo test --package some_component

# 4. 全体の整合性をチェック
cargo check

# 5. DAGの検証
jsonnet eval -e "local dag = import 'dag.jsonnet'; dag.validate_dag()"

# 6. コミット
git add .
git commit -m "Update some_component"
```

## 🧪 テスト

### ユニットテスト

```bash
# 全テストを実行
cargo test

# 特定のテストを実行
cargo test test_graph_operations

# ドキュメントテストを実行
cargo test --doc
```

### 統合テスト

```bash
# 統合テストを実行
cargo test --test integration

# ベンチマークを実行
cargo bench
```

### LDBC-SNBベンチマーク

```bash
# LDBC-SNBデータセットでベンチマーク
cargo run --bin kotoba-bench -- --dataset ldbc-snb
```

## 📦 パッケージング

### Dockerイメージ

```bash
# Dockerイメージをビルド
docker build -t kotoba:latest .

# イメージを実行
docker run -p 8080:8080 kotoba:latest
```

### Debianパッケージ

```bash
# Debianパッケージを作成
cargo deb

# パッケージをインストール
sudo dpkg -i target/debian/kotoba_0.1.0_amd64.deb
```

### Homebrew

```bash
# Homebrew Formulaをインストール
brew install kotoba
```

## 🔧 CLIツール

### kotoba-cli

```bash
# ヘルプを表示
./target/release/kotoba-cli --help

# GQLクエリを実行
./target/release/kotoba-cli query "MATCH (p:Person) RETURN p.name"

# グラフファイルをロード
./target/release/kotoba-cli load --file graph.json

# 統計情報を表示
./target/release/kotoba-cli stats
```

## 📚 APIドキュメント

```bash
# ドキュメントを生成
cargo doc --open

# プライベートアイテムを含むドキュメントを生成
cargo doc --document-private-items --open
```

## 🤝 貢献

### 貢献ガイドライン

1. **Issueを作成**: バグ報告や機能リクエスト
2. **ブランチを作成**: `feature/your-feature-name`
3. **変更を実装**:
   - テストを追加
   - ドキュメントを更新
   - dag.jsonnetの整合性を確認
4. **Pull Requestを作成**

### 開発環境のセットアップ

```bash
# 開発用依存関係をインストール
cargo install cargo-edit cargo-watch cargo-deb

# pre-commit hooksを設定
cp pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### コーディング規約

- **Rust**: `rustfmt` と `clippy` を使用
- **コミットメッセージ**: [Conventional Commits](https://conventionalcommits.org/)
- **テスト**: すべての変更にテストを追加
- **ドキュメント**: すべての公開APIにドキュメントを追加

## 📄 ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 🙏 謝辞

- GP2チーム: グラフ書換えシステムの理論的基盤
- ISO/IEC: GQL標準仕様
- Rustコミュニティ: 優れたプログラミング言語

## 📞 サポート

- **ドキュメント**: [https://kotoba.jun784.dev](https://kotoba.jun784.dev)
- **Issues**: [GitHub Issues](https://github.com/jun784/kotoba/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jun784/kotoba/discussions)

---

**Kotoba** - 言葉を通じてグラフの世界を探索する
