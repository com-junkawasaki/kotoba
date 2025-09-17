# Kotoba Build Tool

Kotoba Build Toolは、Denoのビルドシステムに似た使い勝手で、Kotobaプロジェクトのビルド、依存関係解決、タスク実行を統合的に管理するツールです。

## 特徴

- 🚀 **高速なビルド**: 非同期処理による高速なビルド実行
- 📦 **依存関係管理**: プロジェクトの依存関係を自動的に解決
- 🎯 **タスク実行**: カスタムタスクの定義と実行
- 👀 **ファイル監視**: ファイル変更時の自動再ビルド
- 🎨 **美しい出力**: カラフルで分かりやすいCLI出力
- 🔧 **拡張性**: プラグインシステムによる拡張が可能

## インストール

```bash
# リポジトリからクローン
git clone https://github.com/jun784/kotoba.git
cd kotoba

# ビルドツールをビルド
cargo build --release --package kotoba-build

# 実行ファイルを使用
./target/release/kotoba-build --help
```

## 使い方

### 基本的な使用方法

```bash
# ヘルプ表示
kotoba-build --help

# 利用可能なタスク一覧を表示
kotoba-build --list

# 特定のタスクを実行
kotoba-build dev
kotoba-build build
kotoba-build test

# デフォルトビルドを実行
kotoba-build

# クリーン実行
kotoba-build --clean
```

### 設定ファイル

プロジェクトルートに `kotoba-build.toml` ファイルを作成して、タスクを定義します。

```toml
# Kotoba Build Configuration
name = "my-project"
version = "0.1.0"
description = "My awesome project"

[tasks.dev]
command = "cargo"
args = ["run"]
description = "Start development server"
depends_on = []
cwd = "."
env = {}

[tasks.build]
command = "cargo"
args = ["build", "--release"]
description = "Build project in release mode"
depends_on = []
cwd = "."
env = {}

[tasks.test]
command = "cargo"
args = ["test"]
description = "Run tests"
depends_on = []
cwd = "."
env = {}

[tasks.clean]
command = "cargo"
args = ["clean"]
description = "Clean build artifacts"
depends_on = []
cwd = "."
env = {}

[dependencies]
tokio = "1.0"
serde = "1.0"
```

### タスクの定義

各タスクは以下のフィールドをサポートします：

- `command`: 実行するコマンド
- `args`: コマンドの引数（配列）
- `description`: タスクの説明
- `depends_on`: 依存するタスク（配列）
- `cwd`: 作業ディレクトリ
- `env`: 環境変数（ハッシュマップ）

### 高度な機能

#### ウォッチモード（開発中）

```bash
# ファイル変更を監視して自動再ビルド
kotoba-build --watch
```

#### 詳細出力

```bash
# 詳細なログを表示
kotoba-build --verbose dev
```

#### 設定ファイルの指定

```bash
# 特定の設定ファイルを使用
kotoba-build --config custom.toml dev
```

## サポートされている設定ファイル形式

- `kotoba-build.toml` (推奨)
- `kotoba-build.json`
- `kotoba-build.yaml`

## 自動検出

ビルドツールは以下の設定ファイルを自動的に検出します：

- `package.json` (Node.jsプロジェクト)
- `Cargo.toml` (Rustプロジェクト)
- `requirements.txt` (Pythonプロジェクト)
- `go.mod` (Goプロジェクト)

## 例

### Rustプロジェクト

```toml
[tasks.dev]
command = "cargo"
args = ["run"]
description = "Start development server"

[tasks.build]
command = "cargo"
args = ["build", "--release"]
description = "Build project in release mode"

[tasks.test]
command = "cargo"
args = ["test"]
description = "Run tests"
```

### Node.jsプロジェクト

```toml
[tasks.dev]
command = "npm"
args = ["run", "dev"]
description = "Start development server"

[tasks.build]
command = "npm"
args = ["run", "build"]
description = "Build project"

[tasks.test]
command = "npm"
args = ["test"]
description = "Run tests"
```

## アーキテクチャ

Kotoba Build Toolは以下のコンポーネントで構成されています：

- **設定管理**: TOML/JSON/YAML設定ファイルの読み込み
- **タスク実行**: 非同期タスク実行エンジン
- **ファイル監視**: ファイル変更検知と自動再ビルド
- **依存関係解決**: タスク間の依存関係の解決
- **CLIインターフェース**: 使いやすいコマンドラインインターフェース

## 開発

### ビルド

```bash
cargo build --release
```

### テスト

```bash
cargo test
```

### ドキュメント

```bash
cargo doc --open
```

## 貢献

Kotobaプロジェクトへの貢献を歓迎します！以下の方法で貢献できます：

1. Issueの作成
2. Pull Requestの送信
3. ドキュメントの改善

## ライセンス

このプロジェクトはMITライセンスの下で公開されています。
