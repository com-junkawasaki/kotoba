use kotoba_db_engine_memory::MemoryStorageEngine;
use kotoba_db_core::engine::StorageEngine;
use kotoba_db_core::types::{Block, Cid, NodeBlock, EdgeBlock, Value};
use std::collections::BTreeMap;
use anyhow::Result;

/// The main database handle for KotobaDB.
/// This provides the user-facing API for database operations.
pub struct DB {
    engine: Box<dyn StorageEngine>,
}

impl DB {
    /// Opens a new database instance using an in-memory storage engine.
    /// This is useful for testing, prototyping, or temporary data.
    pub fn open_memory() -> Self {
        Self {
            engine: Box::new(MemoryStorageEngine::new()),
        }
    }

    /// Creates a new node in the database.
    ///
    /// # Arguments
    /// * `properties` - A map of property names to values for this node
    ///
    /// # Returns
    /// The CID of the created node block
    pub fn create_node(&mut self, properties: BTreeMap<String, Value>) -> Result<Cid> {
        let node_block = NodeBlock {
            properties,
            edges: Vec::new(), // Start with no edges
        };
        let block = Block::Node(node_block);
        self.engine.put_block(&block)
    }

    /// Creates a new edge in the database.
    ///
    /// # Arguments
    /// * `label` - The label/type of the edge (e.g., "FRIENDS_WITH", "WORKS_AT")
    /// * `from_cid` - CID of the source node
    /// * `to_cid` - CID of the target node
    /// * `properties` - A map of property names to values for this edge
    ///
    /// # Returns
    /// The CID of the created edge block
    pub fn create_edge(
        &mut self,
        label: String,
        from_cid: Cid,
        to_cid: Cid,
        properties: BTreeMap<String, Value>,
    ) -> Result<Cid> {
        let edge_block = EdgeBlock {
            label,
            from: from_cid,
            to: to_cid,
            properties,
        };
        let block = Block::Edge(edge_block);
        self.engine.put_block(&block)
    }

    /// Retrieves a block by its CID.
    pub fn get_block(&self, cid: &Cid) -> Result<Option<Block>> {
        self.engine.get_block(cid)
    }

    /// Retrieves a node by its CID.
    pub fn get_node(&self, cid: &Cid) -> Result<Option<NodeBlock>> {
        match self.get_block(cid)? {
            Some(Block::Node(node)) => Ok(Some(node)),
            Some(Block::Edge(_)) => Ok(None),
            None => Ok(None),
        }
    }

    /// Retrieves an edge by its CID.
    pub fn get_edge(&self, cid: &Cid) -> Result<Option<EdgeBlock>> {
        match self.get_block(cid)? {
            Some(Block::Edge(edge)) => Ok(Some(edge)),
            Some(Block::Node(_)) => Ok(None),
            None => Ok(None),
        }
    }
}
