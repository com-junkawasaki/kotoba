//! `kotoba-storage`
//!
//! This crate defines the core traits (ports) for storage operations
//! in the Kotoba ecosystem. It provides abstractions for various storage
//! backends like Key-Value stores, Event stores, and Graph stores.

use anyhow::Result;
use async_trait::async_trait;
use std::fmt;
use kotoba_core::{auth::{Policy, RelationTuple, Principal}, crypto::EncryptionInfo};
use kotoba_cid::Cid;

/// Storage backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Memory,
    RocksDB,
    Redis,
    Custom(&'static str),
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageBackend::Memory => write!(f, "Memory"),
            StorageBackend::RocksDB => write!(f, "RocksDB"),
            StorageBackend::Redis => write!(f, "Redis"),
            StorageBackend::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// A generic key-value store trait.
/// Keys and values are treated as opaque byte arrays.
/// The key can be considered a path in a Merkle DAG, and the value is the content of a node.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Puts a key-value pair into the store.
    /// This operation can be seen as adding or updating a node in the Merkle DAG.
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Gets a value for a given key.
    /// Retrieves the content of a node from the Merkle DAG using its key (path).
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Deletes a key-value pair from the store.
    /// Removes a node from the Merkle DAG.
    async fn delete(&self, key: &[u8]) -> Result<()>;

    /// Scans for key-value pairs with a given prefix.
    /// Can be used to traverse a subtree of the Merkle DAG.
    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Returns the storage backend type
    fn backend_type(&self) -> StorageBackend {
        StorageBackend::Custom("Unknown")
    }

    /// Returns storage statistics
    async fn stats(&self) -> Result<StorageStats> {
        Ok(StorageStats::default())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_keys: u64,
    pub total_size_bytes: u64,
    pub operations_count: u64,
    pub hit_ratio: f64,
    pub backend_info: String,
}

/// 認証・認可データの永続化に関するトレイト
#[async_trait]
pub trait AuthStorage: Send + Sync {
    // --- ReBAC (関係性ベースアクセス制御) ---

    /// 関係性タプルを保存する
    async fn save_relation(&self, tuple: &RelationTuple) -> Result<()>;

    /// 指定されたオブジェクトに対する関係性タプルを検索する
    async fn find_relations_for_object(&self, object_id: &str) -> Result<Vec<RelationTuple>>;

    /// 指定された主体に対する関係性タプルを検索する
    async fn find_relations_for_subject(&self, subject_id: &str) -> Result<Vec<RelationTuple>>;

    /// 関係性の存在をチェックする
    async fn check_relation_exists(&self, tuple: &RelationTuple) -> Result<bool>;

    // --- ABAC/PBAC (属性/ポリシーベースアクセス制御) ---

    /// ポリシーを保存する
    async fn save_policy(&self, policy: &Policy) -> Result<Cid>;

    /// ポリシーを取得する
    async fn get_policy(&self, policy_cid: &Cid) -> Result<Option<Policy>>;

    /// 指定されたリソースに対するポリシーを検索する
    async fn find_policies_for_resource(&self, resource_id: &str) -> Result<Vec<Policy>>;

    // --- Principal（主体）管理 ---

    /// 主体を保存する
    async fn save_principal(&self, principal: &Principal) -> Result<()>;

    /// 主体を取得する
    async fn get_principal(&self, principal_id: &str) -> Result<Option<Principal>>;

    /// 主体の属性を更新する
    async fn update_principal_attributes(&self, principal_id: &str, attributes: std::collections::HashMap<String, String>) -> Result<()>;

    // --- 暗号化キー管理 ---

    /// 暗号化情報を保存する
    async fn save_encryption_info(&self, info: &EncryptionInfo) -> Result<Cid>;

    /// 暗号化情報を取得する
    async fn get_encryption_info(&self, info_cid: &Cid) -> Result<Option<EncryptionInfo>>;

    /// 指定された主体に対する暗号化情報を検索する
    async fn find_encryption_info_for_principal(&self, principal_id: &str) -> Result<Vec<EncryptionInfo>>;
}

// TODO: Define EventStore and GraphStore traits later

/// Re-export common storage implementations
#[cfg(feature = "memory")]
pub use kotoba_memory::MemoryKeyValueStore;

#[cfg(feature = "rocksdb")]
pub use kotoba_storage_rocksdb::RocksDbStore;

#[cfg(feature = "redis")]
pub use kotoba_storage_redis::RedisStore;