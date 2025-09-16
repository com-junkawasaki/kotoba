//! 共通型定義

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 頂点ID
pub type VertexId = Uuid;

/// エッジID
pub type EdgeId = Uuid;

/// ラベル（型）
pub type Label = String;

/// プロパティキー
pub type PropertyKey = String;

/// プロパティ値
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxId(pub String);

/// コンテンツハッシュ
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn sha256(data: [u8; 32]) -> Self {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self(hex::encode(result))
    }
}

/// エラー型
#[derive(Debug, thiserror::Error)]
pub enum KotobaError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Rewrite error: {0}")]
    Rewrite(String),
    #[error("Security error: {0}")]
    Security(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Network error: {0}")]
    Network(String),
}

pub type Result<T> = std::result::Result<T, KotobaError>;

#[cfg(feature = "security")]
impl From<kotoba_security::SecurityError> for KotobaError {
    fn from(error: kotoba_security::SecurityError) -> Self {
        match error {
            kotoba_security::SecurityError::Configuration(msg) => KotobaError::Configuration(msg),
            kotoba_security::SecurityError::Authentication(msg) => KotobaError::Security(format!("Authentication error: {}", msg)),
            kotoba_security::SecurityError::Authorization(msg) => KotobaError::Security(format!("Authorization error: {}", msg)),
            kotoba_security::SecurityError::Jwt(e) => KotobaError::Security(format!("JWT error: {}", e)),
            kotoba_security::SecurityError::OAuth2(msg) => KotobaError::Security(format!("OAuth2 error: {}", msg)),
            kotoba_security::SecurityError::Mfa(msg) => KotobaError::Security(format!("MFA error: {}", msg)),
            kotoba_security::SecurityError::Password(msg) => KotobaError::Security(format!("Password error: {}", msg)),
            _ => KotobaError::Security(format!("Security error: {:?}", error)),
        }
    }
}
