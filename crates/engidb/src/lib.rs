//! EngiDB - The Unified Language Graph Database for Kotoba.
//! Pure Rust implementation using sled (no native dependencies).

use kotoba_types::{Node, Graph};
use cid::Cid;
use multihash::Multihash;
use sled::{Db, Tree};
use std::path::Path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sled database error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

// Tree names for different data layers
const IPLD_BLOCKS: &str = "ipld_blocks";
const VERTICES: &str = "vertices";
const CID_TO_VERTEX: &str = "cid_to_vertex";
const EDGES: &str = "edges";
const COMMITS: &str = "commits";
const TRANSACTIONS: &str = "transactions";
const BRANCHES: &str = "branches";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub timestamp: u64,
    // For now, we'll keep it simple. We can add more details later.
    // pub added_vertices: Vec<u64>,
    // pub added_edges: Vec<(u64, String, u64)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub transaction_cid: Cid,
    pub parents: Vec<Cid>,
    pub author: String,
    pub message: String,
}


/// EngiDB main database structure.
#[derive(Clone)]
pub struct EngiDB {
    db: sled::Db,
}

impl EngiDB {
    /// Opens a database at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(EngiDB { db })
    }

    /// Puts an IPLD block into the store.
    pub fn put_block(&self, cid: &Cid, data: &[u8]) -> Result<()> {
        let tree = self.db.open_tree(IPLD_BLOCKS)?;
        tree.insert(cid.to_bytes(), data)?;
        Ok(())
    }

    /// Gets an IPLD block from the store.
    pub fn get_block(&self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        let tree = self.db.open_tree(IPLD_BLOCKS)?;
        let result = tree.get(cid.to_bytes())?
            .map(|v| v.to_vec());
        Ok(result)
    }

    /// Adds an edge between two vertices.
    pub fn add_edge(&self, source_id: u64, edge_type: &str, target_id: u64) -> Result<()> {
        let tree = self.db.open_tree(EDGES)?;
        let key = format!("{}:{}:{}", source_id, edge_type, target_id);

        // Check if edge already exists
        if tree.contains_key(key.as_bytes())? {
            return Ok(());
        }

        // Add the edge (empty value)
        tree.insert(key.as_bytes(), &[])?;
        Ok(())
    }

    /// Gets all target vertex IDs for a given source vertex and edge type.
    pub fn get_edges_from(&self, source_id: u64, edge_type: &str) -> Result<Vec<u64>> {
        let tree = self.db.open_tree(EDGES)?;
        let prefix = format!("{}:{}:", source_id, edge_type);
        let mut targets = Vec::new();

        for result in tree.scan_prefix(prefix.as_bytes()) {
            let (key, _) = result?;
            let key_str = std::str::from_utf8(&key)?;
            if let Some(target_part) = key_str.split(':').nth(2) {
                if let Ok(target_id) = target_part.parse::<u64>() {
                    targets.push(target_id);
                }
            }
        }

        Ok(targets)
    }

    /// Imports a `kotoba` Graph into the database.
    /// This method is transactional.
    pub fn import_graph(&self, graph: &Graph) -> Result<()> {
        let mut node_id_map = HashMap::new();
        for node in &graph.node {
            let vertex_id = self.add_vertex(node)?;
            node_id_map.insert(node.id.clone(), vertex_id);
        }

        let mut edge_sources: HashMap<&str, &str> = HashMap::new();
        let mut edge_targets: HashMap<&str, &str> = HashMap::new();

        for i in &graph.incidence {
            if i.role == "source" {
                edge_sources.insert(&i.edge, &i.node);
            } else if i.role == "target" {
                edge_targets.insert(&i.edge, &i.node);
            }
        }
        
        for edge in &graph.edge {
            if let (Some(source_node_id), Some(target_node_id)) = (edge_sources.get(edge.id.as_str()), edge_targets.get(edge.id.as_str())) {
                if let (Some(source_vertex_id), Some(target_vertex_id)) = (node_id_map.get(*source_node_id), node_id_map.get(*target_node_id)) {
                    self.add_edge(*source_vertex_id, &edge.kind, *target_vertex_id)?;
                }
            }
        }

        Ok(())
    }

    /// Creates a new commit for the current state of the database.
    pub fn commit(&self, branch: &str, author: String, message: String) -> Result<Cid> {
        let branches_tree = self.db.open_tree(BRANCHES)?;
        let parent_cid_bytes = branches_tree.get(branch.as_bytes())?;
        let parents = if let Some(bytes) = parent_cid_bytes {
            vec![Cid::try_from(bytes.to_vec()).map_err(|e| Error::Serialization(e.to_string()))?]
        } else {
            vec![]
        };

        // 1. Create and store the transaction object
        let transaction = Transaction {
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };
        let tx_data = serde_ipld_dagcbor::to_vec(&transaction).map_err(|e| Error::Serialization(e.to_string()))?;
        let tx_cid = self.calculate_cid(&tx_data)?;
        self.put_block(&tx_cid, &tx_data)?;

        // 2. Create and store the commit object
        let commit = Commit {
            transaction_cid: tx_cid,
            parents,
            author,
            message,
        };
        let commit_data = serde_ipld_dagcbor::to_vec(&commit).map_err(|e| Error::Serialization(e.to_string()))?;
        let commit_cid = self.calculate_cid(&commit_data)?;
        self.put_block(&commit_cid, &commit_data)?;

        // 3. Update the branch to point to the new commit
        branches_tree.insert(branch.as_bytes(), commit_cid.to_bytes())?;

        Ok(commit_cid)
    }

    // Helper function to calculate CID for any serializable data
    fn calculate_cid(&self, data: &[u8]) -> Result<Cid> {
        const SHA2_256_CODE: u64 = 0x12; // SHA-256 multihash code
        let hash = Sha256::digest(data);
        let multihash = Multihash::<64>::wrap(SHA2_256_CODE, &hash).unwrap();
        Ok(Cid::new_v1(0x71, multihash))
    }

    /// Adds a vertex to the graph from a `kotoba` Node.
    pub fn add_vertex(&self, node: &Node) -> Result<u64> {
        // 1. Serialize node and calculate CID
        let data = serde_ipld_dagcbor::to_vec(node).map_err(|e| Error::Serialization(e.to_string()))?;
        let cid = self.calculate_cid(&data)?;

        // 2. Check if vertex already exists
        let cid_to_vertex_tree = self.db.open_tree(CID_TO_VERTEX)?;
        if let Some(existing_id_bytes) = cid_to_vertex_tree.get(cid.to_bytes())? {
            let existing_id = u64::from_be_bytes(existing_id_bytes.as_ref().try_into().unwrap());
            return Ok(existing_id);
        }

        // 3. If not, create it
        let vertices_tree = self.db.open_tree(VERTICES)?;

        // Get next vertex ID (simple counter)
        let next_id = (vertices_tree.len() + 1) as u64;

        // Store the data
        self.put_block(&cid, &data)?;
        vertices_tree.insert(&next_id.to_be_bytes(), cid.to_bytes())?;
        cid_to_vertex_tree.insert(cid.to_bytes(), &next_id.to_be_bytes())?;

        Ok(next_id)
    }

    /// Scan all TodoItem nodes from the database
    pub fn scan_todo_items(&self) -> Result<Vec<kotoba_types::Node>> {
        let vertices_tree = self.db.open_tree(VERTICES)?;
        let mut todos = Vec::new();

        for result in vertices_tree.iter() {
            let (_id_bytes, cid_bytes) = result?;
            let cid = cid::Cid::try_from(cid_bytes.to_vec())
                .map_err(|e| Error::Serialization(e.to_string()))?;
            if let Some(block) = self.get_block(&cid)? {
                let node: kotoba_types::Node =
                    serde_ipld_dagcbor::from_slice(&block).map_err(|e| Error::Serialization(e.to_string()))?;
                if node.kind == "TodoItem" {
                    todos.push(node);
                }
            }
        }
        Ok(todos)
    }

    /// Store a TodoItem node
    pub fn store_todo_item(&self, node: &kotoba_types::Node) -> Result<u64> {
        self.add_vertex(node)
    }
}
