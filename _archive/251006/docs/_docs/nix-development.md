---
layout: default
title: Nix Development
---

# Nix Development Environment

このドキュメントでは、KotobaプロジェクトでNixを使用した開発環境の設定と使用方法について説明します。

## 🎯 概要

Nixは再現可能なビルドと宣言的なシステム設定を提供するパッケージマネージャーです。KotobaプロジェクトではNix flakesを使用して、以下を実現しています：

- **再現可能なビルド**: すべての開発者が同じ環境を使用
- **依存関係の分離**: システムに影響を与えずに開発
- **クロスプラットフォーム対応**: Linux/macOS/Windows (WSL)
- **自動CI/CD**: GitHub Actionsでの自動ビルド

## 📦 必要なツール

### Nixのインストール

```bash
# 公式インストーラー (推奨)
curl -L https://nixos.org/nix/install | sh

# または、macOSでHomebrewを使用
brew install nix

# または、Linuxディストリビューションのパッケージマネージャー
# Ubuntu/Debian
sudo apt install nix-bin
# Fedora
sudo dnf install nix
# Arch Linux
sudo pacman -S nix
```

### Flakesの有効化

Nix flakesを有効にするため、`~/.config/nix/nix.conf`に以下の設定を追加：

```bash
experimental-features = nix-command flakes
```

### オプション: direnv

自動環境有効化のために[direnv](https://direnv.net/)をインストール：

```bash
# macOS
brew install direnv

# シェル設定に追加 (.zshrc or .bashrc)
eval "$(direnv hook zsh)"  # or bash
```

## 🚀 使用方法

### 基本的な使用

```bash
# リポジトリをクローン
git clone https://github.com/com-junkawasaki/kotoba.git
cd kotoba

# セットアップスクリプト実行 (推奨)
./scripts/setup-nix.sh

# 開発環境に入る
nix develop

# またはdirenvを使用する場合
direnv allow
```

### 利用可能なコマンド

開発環境内で以下のコマンドが利用可能です：

```bash
# ビルド
cargo build
cargo build --release

# テスト
cargo test
cargo test --workspace

# リントとフォーマット
cargo clippy
cargo fmt

# ベンチマーク
cargo bench

# ドキュメント生成
cargo doc --open
```

### Docker関連 (Linuxのみ)

```bash
# Dockerイメージビルド
docker build -t kotoba .

# Kindでのローカルデプロイ
./k8s/kind/deploy-local.sh
```

## 🏗️ Nix Flakeの構造

### flake.nix

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }: {
    # 開発シェル
    devShells.default = pkgs.mkShell {
      # Rust 1.82.0 + 必要な拡張機能
      # システム依存関係
      # 開発ツール
    };

    # パッケージ定義
    packages.default = rustPlatform.buildRustPackage {
      # Kotobaのビルド定義
    };

    # Dockerイメージ
    packages.dockerImage = dockerTools.buildImage {
      # NixベースのDockerイメージ
    };
  };
}
```

### 主要コンポーネント

- **Rust Toolchain**: バージョン1.82.0のRust + rust-analyzer, clippy, rustfmt
- **システム依存関係**: pkg-config, openssl, libclang
- **開発ツール**: git, jq, yq, curl, docker, kind, kubectl, helm (Linuxのみ)
- **追加ツール**: cargo拡張機能、ベンチマークツール、ドキュメント生成ツール

## 🔧 トラブルシューティング

### Flakesが有効になっていない

```
error: experimental Nix feature 'flakes' is disabled
```

**解決方法**:
```bash
echo 'experimental-features = nix-command flakes' >> ~/.config/nix/nix.conf
```

### キャッシュの問題

```bash
# Nixストアのクリーンアップ
nix store gc

# 特定のビルドの再試行
nix build .#default --rebuild
```

### プラットフォーム固有の問題

macOSではDocker/Kind関連ツールが利用できないため、ビルドとテストのみ可能です：

```bash
# macOSでのビルド
nix develop --command 'cargo build --release'

# macOSでのテスト (統合テストをスキップ)
nix develop --command 'cargo test --lib'
```

## 📊 CI/CD

GitHub Actionsで自動ビルドとテストを実行：

```yaml
# .github/workflows/nix-build.yml
- Nix環境でのビルド
- テスト実行
- Dockerイメージ生成
- クロスプラットフォーム対応 (Linux/macOS)
```

### Caching

[Cachix](https://cachix.org/)を使用してビルドキャッシュを共有：

```bash
# キャッシュの設定 (オプション)
cachix use kotoba
```

## 🎨 カスタマイズ

### ローカル設定

`.envrc.local`ファイルでローカル設定を上書き可能：

```bash
# .envrc.local
export RUST_LOG=debug
export CARGO_INCREMENTAL=0
```

### Flakeのカスタマイズ

`flake.nix`を編集して環境をカスタマイズ：

```nix
# 追加のRustターゲット
targets = [
  "wasm32-unknown-unknown"  # WebAssembly
  "aarch64-linux-android"   # Android
];
```

## 📚 参考資料

- [Nix Manual](https://nixos.org/manual/nix/stable/)
- [Nix Flakes](https://nixos.wiki/wiki/Flakes)
- [Rust Overlay](https://github.com/oxalica/rust-overlay)
- [Direnv](https://direnv.net/)

## 🤝 貢献

Nix環境を使用する場合は：

1. `flake.nix`の変更をテスト
2. `nix flake check`で検証
3. クロスプラットフォーム互換性を確認
4. ドキュメントを更新

---

このNix環境により、Kotobaプロジェクトは安定した再現可能な開発体験を提供します。
