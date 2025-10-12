//! EngiDB - The Unified Language Graph Database for Kotoba.

use crate::{types::Node, Error, Result};
use cid::Cid;
use multihash::Multihash;
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::path::Path;

// Layer 1: IPLD Block Store
const IPLD_BLOCKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("ipld_blocks");

// Layer 2: Graph Logic Layer
const VERTICES: TableDefinition<u64, &[u8]> = TableDefinition::new("vertices");
const CID_TO_VERTEX: TableDefinition<&[u8], u64> = TableDefinition::new("cid_to_vertex");
// redb keys can be tuples of owned types or `&str`, `&[u8]`. `u64` is owned (Copy).
const EDGES: TableDefinition<(u64, &str), &[u8]> = TableDefinition::new("edges");


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
                let serialized_targets = bincode::serialize(&targets).map_err(|e| Error::Storage(e.to_string()))?;
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

    /// Adds a vertex to the graph from a `kotoba` Node.
    pub fn add_vertex(&self, node: &Node) -> Result<u64> {
        // 1. Serialize node and calculate CID
        const BLAKE3_256_CODE: u64 = 0x1e;
        let data = serde_ipld_dagcbor::to_vec(node).map_err(|e| Error::Storage(e.to_string()))?;
        let hash = blake3::hash(&data);
        let multihash = Multihash::<64>::wrap(BLAKE3_256_CODE, hash.as_bytes()).unwrap();
        let cid = Cid::new_v1(0x71, multihash);

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
}
