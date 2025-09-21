//! # Kotoba Types
//!
//! This crate provides the foundational data types used throughout the Kotoba ecosystem.
//! By isolating these types into a separate, stable crate, we can ensure a clear
//! dependency hierarchy and avoid circular dependencies.
//!
//! ## Core Types
//!
//! - `Value`: Represents various data types (e.g., string, number, boolean).
//! - `VertexId`, `EdgeId`: Unique identifiers for graph elements.
//! - `TxId`: Unique identifier for transactions.
//! - `ContentHash`: Hash representing the content of a piece of data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;
use sha2::{Digest, Sha256};
use schemars::JsonSchema;

/// Content Identifier (Merkle-Link)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Cid(pub [u8; 32]);

impl Cid {
    pub fn new(data: &[u8]) -> Self {
        let hash = Sha256::digest(data);
        Cid(hash.into())
    }

    pub fn as_str(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for Cid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// 頂点ID
pub type VertexId = Uuid;

/// エッジID
pub type EdgeId = Uuid;

/// ラベル（型）
pub type Label = String;

/// プロパティキー
pub type PropertyKey = String;

/// プロパティ値
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Integer(i64), // 互換性のため
    // Float(f64), // Hashを実装できないので除外
    String(String),
    Array(Vec<String>), // セキュリティ統合のために追加
    // List(Vec<Value>), // 再帰的なHashが複雑なので除外
    // Map(HashMap<String, Value>), // 再帰的なHashが複雑なので除外
}

/// プロパティ
pub type Properties = HashMap<PropertyKey, Value>;

/// グラフ参照（Merkleハッシュ）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphRef_(pub String);

/// トランザクションID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TxId(pub String);

/// コンテンツハッシュ
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub String);

// --- Auth Types ---

/// Represents a user identifier.
pub type User = String;

/// Represents a role identifier.
pub type Role = String;

/// Represents a permission identifier (e.g., "read", "write").
pub type Permission = String;

/// Represents a resource identifier, which can be a path, a CID, etc.
pub type ResourceId = Cid;


// Note: The `Cid` and `ContentHash` logic seems more appropriate for `kotoba-cid`
// and will be moved there or refactored. For now, only the basic types are in this crate.
// The `Result` type alias and error conversions will be handled in crates that declare errors.
