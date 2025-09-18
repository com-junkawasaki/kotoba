//! `StoragePort` defines the core interface for all storage backends.
use crate::domain::models::StorageStats;
use crate::domain::merkle::MerkleDAG;
use crate::domain::mvcc::MVCCManager;
use kotoba_core::prelude::{Result, Cid};
use kotoba_graph::prelude::Graph;
use std::sync::Arc;
use async_trait::async_trait;

#[async_trait]
pub trait StoragePort: Send + Sync {
    /// Stores a graph and returns its root CID.
    async fn store_graph(&self, graph: &Graph) -> Result<Cid>;

    /// Loads a graph by its root CID.
    async fn load_graph(&self, cid: &Cid) -> Result<Graph>;

    /// Retrieves storage statistics.
    fn get_stats(&self) -> StorageStats;

    /// Returns a reference to the Merkle DAG.
    fn merkle_dag(&self) -> Arc<MerkleDAG>;

    /// Returns a reference to the MVCC manager.
    fn mvcc_manager(&self) -> Arc<MVCCManager>;
}
