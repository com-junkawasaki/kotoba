//! EngiDB - The Unified Language Graph Database for Kotoba.

use kotoba_types::{Node, Graph};
use cid::Cid;
use multihash::Multihash;
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::path::Path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Redb database error: {0}")]
    Redb(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<redb::Error> for Error {
    fn from(err: redb::Error) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::DatabaseError> for Error {
    fn from(err: redb::DatabaseError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::TransactionError> for Error {
    fn from(err: redb::TransactionError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::TableError> for Error {
    fn from(err: redb::TableError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::StorageError> for Error {
    fn from(err: redb::StorageError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::CommitError> for Error {
    fn from(err: redb::CommitError) -> Self { Error::Redb(err.to_string()) }
}

// Layer 1: IPLD Block Store
const IPLD_BLOCKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("ipld_blocks");

// Layer 2: Graph Logic Layer
const VERTICES: TableDefinition<u64, &[u8]> = TableDefinition::new("vertices");
const CID_TO_VERTEX: TableDefinition<&[u8], u64> = TableDefinition::new("cid_to_vertex");
// redb keys can be tuples of owned types or `&str`, `&[u8]`. `u64` is owned (Copy).
const EDGES: TableDefinition<(u64, &str), &[u8]> = TableDefinition::new("edges");

// Layer 3: Time/Versioning Layer
const COMMITS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("commits"); // Key: Commit CID, Value: Parent Commit CID
const TRANSACTIONS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("transactions"); // Key: Tx CID, Value: Commit CID
const BRANCHES: TableDefinition<&str, &[u8]> = TableDefinition::new("branches"); // Key: branch name, Value: head commit CID

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
    db: Database,
}

impl EngiDB {
    /// Opens a database at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Database::create(path)?;
        Ok(EngiDB { db })
    }

    /// Puts an IPLD block into the store.
    pub fn put_block(&self, cid: &Cid, data: &[u8]) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(IPLD_BLOCKS)?;
            table.insert(cid.to_bytes().as_slice(), data)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Gets an IPLD block from the store.
    pub fn get_block(&self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(IPLD_BLOCKS)?;
        let result = table
            .get(cid.to_bytes().as_slice())?
            .map(|v| v.value().to_vec());
        Ok(result)
    }

    /// Adds an edge between two vertices.
    pub fn add_edge(&self, source_id: u64, edge_type: &str, target_id: u64) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(EDGES)?;
            let key = (source_id, edge_type);
            
            let mut targets = table.get(&key)?
                .map(|v| bincode::deserialize(v.value()).unwrap_or_else(|_| vec![]))
                .unwrap_or_else(Vec::new);

            if !targets.contains(&target_id) {
                targets.push(target_id);
                let serialized_targets = bincode::serialize(&targets).map_err(|e| Error::Serialization(e.to_string()))?;
                table.insert(key, serialized_targets.as_slice())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Gets all target vertex IDs for a given source vertex and edge type.
    pub fn get_edges_from(&self, source_id: u64, edge_type: &str) -> Result<Vec<u64>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EDGES)?;
        let key = (source_id, edge_type);

        let targets = table.get(&key)?
            .map(|v| bincode::deserialize(v.value()).unwrap_or_else(|_| vec![]))
            .unwrap_or_else(Vec::new);

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
        let read_txn = self.db.begin_read()?;
        let branches_table = read_txn.open_table(BRANCHES)?;
        let parent_cid_bytes = branches_table.get(branch)?.map(|c| c.value().to_vec());
        let parents = if let Some(bytes) = parent_cid_bytes {
            vec![Cid::try_from(bytes).unwrap()]
        } else {
            vec![]
        };
        drop(read_txn); // Explicitly drop read transaction before starting write transaction

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
        let write_txn = self.db.begin_write()?;
        {
            let mut branches_table = write_txn.open_table(BRANCHES)?;
            branches_table.insert(branch, commit_cid.to_bytes().as_slice())?;
        }
        write_txn.commit()?;

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

        // 2. Check if vertex already exists in a read transaction.
        let read_txn = self.db.begin_read()?;
        let cid_to_vertex_table = read_txn.open_table(CID_TO_VERTEX)?;
        if let Some(existing_id) = cid_to_vertex_table.get(cid.to_bytes().as_slice())? {
            return Ok(existing_id.value());
        }
        // Drop the read transaction by letting it go out of scope.

        // 3. If not, create it in a new write transaction.
        let write_txn = self.db.begin_write()?;
        let new_id;
        {
            let mut vertices_table = write_txn.open_table(VERTICES)?;
            new_id = vertices_table.len()? + 1;

            let mut blocks_table = write_txn.open_table(IPLD_BLOCKS)?;
            let mut cid_to_vertex_table_mut = write_txn.open_table(CID_TO_VERTEX)?;

            blocks_table.insert(cid.to_bytes().as_slice(), data.as_slice())?;
            vertices_table.insert(new_id, cid.to_bytes().as_slice())?;
            cid_to_vertex_table_mut.insert(cid.to_bytes().as_slice(), new_id)?;
        }
        write_txn.commit()?;
        Ok(new_id)
    }

    /// Scan all TodoItem nodes from the database
    pub fn scan_todo_items(&self) -> Result<Vec<kotoba_types::Node>> {
        use redb::ReadableTable;
        let read_txn = self.db.begin_read()?;
        let vertices = read_txn.open_table(VERTICES)?;
        let mut todos = Vec::new();

        for entry in vertices.iter()? {
            let (_id, cid_bytes) = entry?;
            let cid = cid::Cid::try_from(cid_bytes.value().to_vec())
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
