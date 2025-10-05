# Kotoba Frontend App Example

このディレクトリは、`.kotobanet` ファイルをプログラミング言語として使用して、**Rust コードを一切書かずに**フロントエンドアプリケーションを実装する例を示しています。

## ファイル構成

- `app.kotobanet` - **メインのプログラミング言語ファイル**。Jsonnet ベースのフロントエンドアプリケーション定義
- `generated_app.tsx` - `kotoba-runtime` によって自動生成された React TSX コード
- `test_app.html` - 生成されたコードがブラウザで動作することを確認するための HTML ファイル

## 動作確認

```bash
cd examples/frontend_app
open test_app.html
```

ブラウザで `.kotobanet` ファイルから生成された完全な React アプリケーションが動作することを確認できます。

## 特徴

- ✅ **Rust コード不要**: `.kotobanet` ファイルのみでフロントエンドアプリを実装
- ✅ **宣言的プログラミング**: Jsonnet ベースの設定言語でコンポーネント定義
- ✅ **自動コード生成**: TSX、型定義、props インターフェースの自動生成
- ✅ **完全な React アプリ**: 生成されたコードは標準的な React アプリケーションとして動作

## ワークフロー

1. **定義**: `app.kotobanet` にコンポーネントとページを宣言的に記述
2. **生成**: `kotoba-runtime` が自動的に TSX コードを生成
3. **実行**: 生成されたコードをブラウザで実行

```bash
# 将来の実装: .kotobanet ファイルを直接実行
kotoba run app.kotobanet
```

## 注意

この例では、従来の Rust ベースの実装（`main.rs`）は**意図的に排除**しています。目標は `.kotobanet` ファイルを唯一の信頼できる情報源（Single Source of Truth）とし、Rust コードを完全に不要にすることです。
