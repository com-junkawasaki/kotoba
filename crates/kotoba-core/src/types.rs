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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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

// kotoba-workflow From implementation removed

pub type Result<T> = std::result::Result<T, kotoba_errors::KotobaError>;

#[cfg(feature = "security")]
impl From<kotoba_security::SecurityError> for kotoba_errors::KotobaError {
    fn from(error: kotoba_security::SecurityError) -> Self {
        match error {
            kotoba_security::SecurityError::Configuration(msg) => kotoba_errors::KotobaError::SecurityConfiguration(msg),
            kotoba_security::SecurityError::Authentication(msg) => kotoba_errors::KotobaError::SecurityAuthentication(msg),
            kotoba_security::SecurityError::Authorization(msg) => kotoba_errors::KotobaError::SecurityAuthorization(msg),
            kotoba_security::SecurityError::Jwt(e) => kotoba_errors::KotobaError::SecurityJwt(format!("{}", e)),
            kotoba_security::SecurityError::OAuth2(msg) => kotoba_errors::KotobaError::SecurityOAuth2(msg),
            kotoba_security::SecurityError::Mfa(msg) => kotoba_errors::KotobaError::SecurityMfa(msg),
            kotoba_security::SecurityError::Password(msg) => kotoba_errors::KotobaError::SecurityPassword(msg),
            kotoba_security::SecurityError::Session(msg) => kotoba_errors::KotobaError::SecuritySession(msg),
            _ => kotoba_errors::KotobaError::Security(format!("Security error: {:?}", error)),
        }
    }
}

// Duplicate From<SystemTimeError> implementation removed (already handled by #[from])
