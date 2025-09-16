# Kotoba CLI

Command Line Interface for Kotoba graph processing system.

## 概要

Kotoba CLI は、Kotoba のグラフ処理システムをコマンドラインから操作するためのインターフェースです。Deno CLI を参考にした使いやすいコマンド体系を提供します。

## インストール

```bash
cargo install kotoba-cli
```

## 主なコマンド

### ファイル実行
```bash
kotoba run src/main.kotoba
kotoba run --watch src/main.kotoba
```

### サーバー起動
```bash
kotoba serve
kotoba serve --port 8080 --host 0.0.0.0
```

### プロジェクト管理
```bash
kotoba init my-project
kotoba init --template advanced my-project
```

### コンパイル
```bash
kotoba compile src/main.kotoba
kotoba compile --output dist/main --optimize 2 src/main.kotoba
```

### 開発支援
```bash
kotoba test
kotoba fmt
kotoba lint
kotoba doc
```

### 情報表示
```bash
kotoba info
kotoba info --verbose
```

## 設定

CLIの設定は `~/.config/kotoba/cli.toml` で管理されます：

```toml
[default]
log_level = "info"
default_port = 3000

[cache]
enabled = true
directory = "~/.cache/kotoba"
max_size_mb = 100
ttl_hours = 24

[server]
host = "127.0.0.1"
port = 3000
timeout_seconds = 30
max_connections = 100
cors_enabled = true

[compiler]
optimization_level = 0
include_debug_info = true
generate_source_maps = true
target_arch = "x86_64"
```

## アーキテクチャ

- **CLI Parser**: clap を使用したコマンドライン引数解析
- **Configuration**: TOML ベースの設定管理
- **Logging**: tracing を使用した構造化ログ
- **Async Runtime**: tokio を使用した非同期処理
- **File Operations**: tokio-util を使用したファイル操作

## 拡張性

CLI は以下の機能をサポートしています：

- **プラグインシステム**: カスタムコマンドの追加
- **設定プロファイル**: 環境別の設定切り替え
- **シェル補完**: Bash, Zsh, Fish の補完スクリプト生成
- **スクリプト実行**: プロジェクト固有のスクリプト実行

## 依存関係

- `clap`: コマンドライン引数解析
- `tokio`: 非同期ランタイム
- `tracing`: 構造化ログ
- `serde`: シリアライズ/デシリアライズ
- `anyhow`: エラーハンドリング
