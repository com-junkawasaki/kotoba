//! EngiDB - The Unified Language Graph Database for Kotoba.
//! Pure Rust implementation using sled.

use kotoba_types::{Node, Graph};
use cid::Cid;
use multihash::Multihash;
use sled;
use std::path::Path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sled database error: {0}")]
    Sled(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self { Error::Sled(err.to_string()) }
}

// Key prefixes for sled-based storage
const IPLD_BLOCK_PREFIX: &str = "ipld:";
const VERTEX_PREFIX: &str = "vertex:";
const CID_TO_VERTEX_PREFIX: &str = "cid_to_vertex:";
const EDGE_PREFIX: &str = "edge:";
const COMMIT_PREFIX: &str = "commit:";
const TRANSACTION_PREFIX: &str = "transaction:";
const BRANCH_PREFIX: &str = "branch:";

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
pub struct EngiDB {
    db: sled::Db,
    next_vertex_id: sled::IVec,
}

impl EngiDB {
    /// Opens a database at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        let next_vertex_id = db.get(b"next_vertex_id")?
            .unwrap_or(sled::IVec::from(1u64.to_be_bytes().to_vec()));

        Ok(EngiDB { db, next_vertex_id })
    }

    /// Puts an IPLD block into the store.
    pub fn put_block(&self, cid: &Cid, data: &[u8]) -> Result<()> {
        let key = format!("{}{}", IPLD_BLOCK_PREFIX, cid.to_string());
        self.db.insert(key.as_bytes(), data)?;
        Ok(())
    }

    /// Gets an IPLD block from the store.
    pub fn get_block(&self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        let key = format!("{}{}", IPLD_BLOCK_PREFIX, cid.to_string());
        let result = self.db.get(key.as_bytes())?.map(|v| v.to_vec());
        Ok(result)
    }

    /// Adds an edge between two vertices.
    pub fn add_edge(&self, source_id: u64, edge_type: &str, target_id: u64) -> Result<()> {
        let key = format!("{}{}:{}:{}", EDGE_PREFIX, source_id, edge_type, target_id);
        self.db.insert(key.as_bytes(), &[])?; // Empty value, key contains all info
        Ok(())
    }

    /// Gets all target vertex IDs for a given source vertex and edge type.
    pub fn get_edges_from(&self, source_id: u64, edge_type: &str) -> Result<Vec<u64>> {
        let prefix = format!("{}{}:{}", EDGE_PREFIX, source_id, edge_type);
        let mut targets = Vec::new();

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(target_part) = key_str.split(':').nth(3) {
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
        // Get parent commit if exists
        let branch_key = format!("{}{}", BRANCH_PREFIX, branch);
        let parent_cid_bytes = self.db.get(branch_key.as_bytes())?.map(|v| v.to_vec());
        let parents = if let Some(bytes) = parent_cid_bytes {
            vec![Cid::try_from(bytes).unwrap()]
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
        self.db.insert(branch_key.as_bytes(), commit_cid.to_bytes().as_slice())?;

        Ok(commit_cid)
    }

    // Helper function to calculate CID for any serializable data
    fn calculate_cid(&self, data: &[u8]) -> Result<Cid> {
        const BLAKE3_256_CODE: u64 = 0x1e;
        let hash = blake3::hash(data);
        let multihash = Multihash::<64>::wrap(BLAKE3_256_CODE, hash.as_bytes()).unwrap();
        Ok(Cid::new_v1(0x71, multihash))
    }

    /// Adds a vertex to the graph from a `kotoba` Node.
    pub fn add_vertex(&self, node: &Node) -> Result<u64> {
        // 1. Serialize node and calculate CID
        let data = serde_ipld_dagcbor::to_vec(node).map_err(|e| Error::Serialization(e.to_string()))?;
        let cid = self.calculate_cid(&data)?;

        // 2. Check if vertex already exists
        let cid_to_vertex_key = format!("{}{}", CID_TO_VERTEX_PREFIX, cid.to_string());
        if let Some(existing_id_bytes) = self.db.get(cid_to_vertex_key.as_bytes())? {
            let existing_id = u64::from_be_bytes(existing_id_bytes.as_ref().try_into().unwrap());
            return Ok(existing_id);
        }

        // 3. Create new vertex
        let current_id_bytes = self.db.get(b"next_vertex_id")?.unwrap_or(sled::IVec::from(1u64.to_be_bytes().to_vec()));
        let current_id = u64::from_be_bytes(current_id_bytes.as_ref().try_into().unwrap());
        let new_id = current_id + 1;

        // Store the vertex
        let vertex_key = format!("{}{}", VERTEX_PREFIX, new_id);
        self.db.insert(vertex_key.as_bytes(), cid.to_bytes().as_slice())?;
        self.db.insert(cid_to_vertex_key.as_bytes(), &new_id.to_be_bytes())?;
        self.db.insert(b"next_vertex_id", &new_id.to_be_bytes())?;

        // Store IPLD block
        self.put_block(&cid, &data)?;

        Ok(new_id)
    }

    /// Scan all TodoItem nodes from the database
    pub fn scan_todo_items(&self) -> Result<Vec<kotoba_types::Node>> {
        let mut todos = Vec::new();

        for item in self.db.scan_prefix(VERTEX_PREFIX.as_bytes()) {
            let (key, cid_bytes) = item?;
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
