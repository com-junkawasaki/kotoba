# Kotoba CID

Content ID (CID) システム for Kotoba graph processing system.

## 概要

Kotoba CID は、Kotoba のグラフ処理におけるコンテンツアドレッシングのためのクレートです。Merkle DAG における一意の識別子生成と検証、JSON の正規化、Merkle ツリー構築などの機能をサポートします。

## 主な機能

- **CID計算**: データのハッシュ値に基づく一意の識別子生成
- **JSON正規化**: JCS (RFC 8785) 準拠のJSON正規化
- **Merkleツリー**: コンテンツのMerkleツリー構築と検証
- **CIDマネージャー**: CIDのキャッシュと管理
- **ハッシュアルゴリズム**: SHA-256, BLAKE3 のサポート

## 使用例

```rust
use kotoba_cid::{CidCalculator, HashAlgorithm, CanonicalJsonMode};

// CID計算器を作成
let calculator = CidCalculator::new(
    HashAlgorithm::Sha2256,
    CanonicalJsonMode::JCS
);

// データをCIDに変換
let data = serde_json::json!({"name": "example", "value": 42});
let cid = calculator.compute_cid(&data)?;
println!("CID: {}", cid.as_str());

// CIDマネージャーを作成
let mut manager = CidManager::new();

// グラフのCIDを計算
let graph_cid = manager.compute_graph_cid(&graph_core)?;
```

## アーキテクチャ

- **CidCalculator**: CID計算のコア機能
- **CidManager**: CIDのキャッシュと管理
- **MerkleTreeBuilder**: Merkleツリーの構築
- **JsonCanonicalizer**: JSONの正規化

## ハッシュアルゴリズム

- **SHA-256**: 標準的な暗号学的ハッシュ関数
- **BLAKE3**: 高速で安全性の高いハッシュ関数

## JSON正規化

- **JCS (RFC 8785)**: JSON Canonicalization Scheme
- **キーのソート**: オブジェクトのキーを辞書順にソート
- **空白の除去**: 余分な空白文字を除去

## 依存関係

- `kotoba-core`: 基本型定義
- `sha2`: SHA-256 ハッシュ
- `blake3`: BLAKE3 ハッシュ
- `serde_json`: JSON 処理
