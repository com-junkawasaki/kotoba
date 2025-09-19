//! `StoragePort` defines the core interface for all storage backends.
use crate::domain::models::StorageStats;
use crate::domain::merkle::MerkleDAG;
use crate::domain::mvcc::MVCCManager;
use anyhow::Result;
use kotoba_db_core::Cid;
use kotoba_graph::prelude::Graph;
use kotoba_db_core::types::Block;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

#[async_trait]
pub trait StoragePort: Send + Sync {
    /// Stores a graph and returns its root CID.
    async fn store_graph(&self, graph: &Graph) -> Result<Cid>;

    /// Loads a graph by its root CID.
    async fn load_graph(&self, cid: &Cid) -> Result<Graph>;

    /// Retrieves storage statistics.
    async fn get_stats(&self) -> Result<StorageStats>;

    /// Returns a reference to the Merkle DAG.
    fn merkle_dag(&self) -> Arc<RwLock<MerkleDAG>>;

    /// Returns a reference to the MVCC manager.
    fn mvcc_manager(&self) -> Arc<RwLock<MVCCManager>>;

    async fn put_block(&self, block: &Block) -> Result<Cid>;
    async fn get_block(&self, cid: &Cid) -> Result<Option<Block>>;
    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Stores arbitrary key-value data
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;
    /// Retrieves arbitrary key-value data
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    /// Gets all keys with a given prefix
    async fn get_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>>;
}
