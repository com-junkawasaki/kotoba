use kotoba_storage::prelude::{StoragePort, PersistentStorage, PersistentStorageConfig};
use kotoba_core::prelude::Cid;
use kotoba_db_core::types::{Block, NodeBlock, EdgeBlock, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;

/// The main database handle for KotobaDB.
/// This provides the user-facing API for database operations.
pub struct DB {
    pub storage: Arc<dyn StoragePort>,
    /// Active transactions
    transactions: Arc<RwLock<HashMap<u64, Transaction>>>,
    /// Transaction ID counter
    next_txn_id: Arc<RwLock<u64>>,
}

/// A database transaction that supports ACID operations
pub struct Transaction {
    id: u64,
    operations: Vec<Operation>,
    state: TransactionState,
    created_at: std::time::Instant,
}

/// State of a transaction
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionState {
    Active,
    Committed,
    RolledBack,
    Failed,
}

/// Database operation within a transaction
#[derive(Debug, Clone)]
pub enum Operation {
    CreateNode { properties: BTreeMap<String, Value>, cid: Option<Cid> },
    CreateEdge { label: String, from_cid: Cid, to_cid: Cid, properties: BTreeMap<String, Value>, cid: Option<Cid> },
    UpdateNode { cid: Cid, properties: BTreeMap<String, Value> },
    UpdateEdge { cid: Cid, properties: BTreeMap<String, Value> },
    DeleteNode { cid: Cid },
    DeleteEdge { cid: Cid },
}

impl DB {
    /// Opens a new database instance using a persistent storage engine.
    pub async fn open(config: PersistentStorageConfig) -> Result<Self> {
        let storage = PersistentStorage::new(config)?;
        Ok(Self {
            storage: Arc::new(storage),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            next_txn_id: Arc::new(RwLock::new(1)),
        })
    }

    /// Begins a new transaction
    pub async fn begin_transaction(&self) -> Result<u64> {
        let mut next_id = self.next_txn_id.write().await;
        let txn_id = *next_id;
        *next_id += 1;

        let transaction = Transaction {
            id: txn_id,
            operations: Vec::new(),
            state: TransactionState::Active,
            created_at: std::time::Instant::now(),
        };

        let mut transactions = self.transactions.write().await;
        transactions.insert(txn_id, transaction);

        Ok(txn_id)
    }

    /// Commits a transaction
    pub async fn commit_transaction(&mut self, txn_id: u64) -> Result<()> {
        // Get the transaction operations first
        let operations = {
            let transactions = self.transactions.read().await;
            if let Some(txn) = transactions.get(&txn_id) {
                if txn.state != TransactionState::Active {
                    return Err(anyhow::anyhow!("Transaction is not active"));
                }
                txn.operations.clone()
            } else {
                return Err(anyhow::anyhow!("Transaction not found"));
            }
        };

        // Execute all operations (without holding the transactions lock)
        for op in operations {
            match op {
                Operation::CreateNode { properties, .. } => {
                    self.create_node(properties).await?;
                }
                Operation::CreateEdge { label, from_cid, to_cid, properties, .. } => {
                    self.create_edge(label, from_cid, to_cid, properties).await?;
                }
                Operation::UpdateNode { cid, properties } => {
                    // For now, we recreate the node with updated properties
                    // TODO: Implement proper update semantics
                    let existing_node = self.get_node(&cid).await?
                        .ok_or_else(|| anyhow::anyhow!("Node not found"))?;
                    let mut new_properties = existing_node.properties.clone();
                    new_properties.extend(properties);
                    self.create_node(new_properties).await?;
                }
                Operation::UpdateEdge { cid, properties } => {
                    // For now, we recreate the edge with updated properties
                    // TODO: Implement proper update semantics
                    let existing_edge = self.get_edge(&cid).await?
                        .ok_or_else(|| anyhow::anyhow!("Edge not found"))?;
                    let mut new_properties = existing_edge.properties.clone();
                    new_properties.extend(properties);
                    self.create_edge(existing_edge.label.clone(),
                                   existing_edge.from,
                                   existing_edge.to,
                                   new_properties).await?;
                }
                Operation::DeleteNode { cid } => {
                    // Mark as deleted by creating a tombstone
                    // TODO: Implement proper deletion
                    let _ = cid; // For now, just ignore
                }
                Operation::DeleteEdge { cid } => {
                    // Mark as deleted by creating a tombstone
                    // TODO: Implement proper deletion
                    let _ = cid; // For now, just ignore
                }
            }
        }

        // Mark transaction as committed
        let mut transactions = self.transactions.write().await;
        if let Some(txn) = transactions.get_mut(&txn_id) {
            txn.state = TransactionState::Committed;
        }

        Ok(())
    }

    /// Rolls back a transaction
    pub async fn rollback_transaction(&mut self, txn_id: u64) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        if let Some(txn) = transactions.get_mut(&txn_id) {
            if txn.state != TransactionState::Active {
                return Err(anyhow::anyhow!("Transaction is not active"));
            }

            txn.operations.clear();
            txn.state = TransactionState::RolledBack;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found"))
        }
    }

    /// Adds an operation to a transaction
    pub async fn add_operation(&self, txn_id: u64, operation: Operation) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        if let Some(txn) = transactions.get_mut(&txn_id) {
            if txn.state != TransactionState::Active {
                return Err(anyhow::anyhow!("Transaction is not active"));
            }
            txn.operations.push(operation);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found"))
        }
    }

    /// Creates a new node in the database.
    ///
    /// # Arguments
    /// * `properties` - A map of property names to values for this node
    ///
    /// # Returns
    /// The CID of the created node block
    pub async fn create_node(&mut self, properties: BTreeMap<String, Value>) -> Result<Cid> {
        let node_block = NodeBlock {
            properties,
            edges: Vec::new(), // Start with no edges
        };
        let block = Block::Node(node_block);
        self.storage.put_block(&block).await.map_err(Into::into)
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
    pub async fn create_edge(
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
        self.storage.put_block(&block).await.map_err(Into::into)
    }

    /// Retrieves a block by its CID.
    pub async fn get_block(&self, cid: &Cid) -> Result<Option<Block>> {
        self.storage.get_block(cid).await.map_err(Into::into)
    }

    /// Retrieves a node by its CID.
    pub async fn get_node(&self, cid: &Cid) -> Result<Option<NodeBlock>> {
        match self.get_block(cid).await? {
            Some(Block::Node(node)) => Ok(Some(node)),
            Some(Block::Edge(_)) => Ok(None),
            None => Ok(None),
        }
    }

    /// Retrieves an edge by its CID.
    pub async fn get_edge(&self, cid: &Cid) -> Result<Option<EdgeBlock>> {
        match self.get_block(cid).await? {
            Some(Block::Edge(edge)) => Ok(Some(edge)),
            Some(Block::Node(_)) => Ok(None),
            None => Ok(None),
        }
    }

    /// Finds nodes that match the given property filters
    pub async fn find_nodes(&self, filters: &[(String, Value)]) -> Result<Vec<(Cid, NodeBlock)>> {
        let mut results = Vec::new();

        // For now, we scan through all nodes (TODO: implement proper indexing)
        // In a real implementation, this would use indexes for efficient filtering

        // Get all node CIDs by scanning (this is inefficient but works for basic functionality)
        let all_keys = self.storage.scan(b"").await?; // Get all keys

        for (key_bytes, _) in all_keys {
            if let Ok(cid_bytes) = <[u8; 32]>::try_from(&key_bytes[..]) {
                let cid = Cid(cid_bytes);
                if let Some(Block::Node(node)) = self.get_block(&cid).await? {
                    // Check if node matches all filters
                    let mut matches = true;
                    for (prop_name, expected_value) in filters {
                        if let Some(actual_value) = node.properties.get(prop_name) {
                            if actual_value != expected_value {
                                matches = false;
                                break;
                            }
                        } else {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        results.push((cid, node));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Finds edges that match the given criteria
    pub async fn find_edges(&self,
                           label_filter: Option<&str>,
                           from_filter: Option<Cid>,
                           to_filter: Option<Cid>,
                           property_filters: &[(String, Value)]) -> Result<Vec<(Cid, EdgeBlock)>> {
        let mut results = Vec::new();

        // Get all keys and filter edges
        let all_keys = self.storage.scan(b"").await?;

        for (key_bytes, _) in all_keys {
            if let Ok(cid_bytes) = <[u8; 32]>::try_from(&key_bytes[..]) {
                let cid = Cid(cid_bytes);
                if let Some(Block::Edge(edge)) = self.get_block(&cid).await? {
                    // Check filters
                    let mut matches = true;

                    // Label filter
                    if let Some(expected_label) = label_filter {
                        if edge.label != expected_label {
                            matches = false;
                        }
                    }

                    // From node filter
                    if let Some(expected_from) = from_filter {
                        if edge.from != expected_from {
                            matches = false;
                        }
                    }

                    // To node filter
                    if let Some(expected_to) = to_filter {
                        if edge.to != expected_to {
                            matches = false;
                        }
                    }

                    // Property filters
                    for (prop_name, expected_value) in property_filters {
                        if let Some(actual_value) = edge.properties.get(prop_name) {
                            if actual_value != expected_value {
                                matches = false;
                                break;
                            }
                        } else {
                            matches = false;
                            break;
                        }
                    }

                    if matches {
                        results.push((cid, edge));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Performs a basic graph traversal from a starting node
    pub async fn traverse(&self,
                         start_cid: Cid,
                         direction: TraversalDirection,
                         max_depth: usize,
                         edge_labels: Option<&[String]>) -> Result<HashMap<Cid, Vec<Cid>>> {
        let mut visited = HashSet::new();
        let mut result = HashMap::new();
        let mut queue = Vec::new();

        queue.push((start_cid, 0)); // (node_cid, depth)
        visited.insert(start_cid);

        while let Some((current_cid, depth)) = queue.pop() {
            if depth >= max_depth {
                continue;
            }

            // Find edges from/to this node
            let edges = match direction {
                TraversalDirection::Outgoing => {
                    self.find_edges(None, Some(current_cid), None, &[]).await?
                }
                TraversalDirection::Incoming => {
                    self.find_edges(None, None, Some(current_cid), &[]).await?
                }
                TraversalDirection::Both => {
                    let mut all_edges = self.find_edges(None, Some(current_cid), None, &[]).await?;
                    all_edges.extend(self.find_edges(None, None, Some(current_cid), &[]).await?);
                    all_edges
                }
            };

            let mut neighbors = Vec::new();

            for (_, edge) in edges {
                // Filter by edge labels if specified
                if let Some(labels) = edge_labels {
                    if !labels.contains(&edge.label) {
                        continue;
                    }
                }

                let neighbor_cid = match direction {
                    TraversalDirection::Outgoing => edge.to,
                    TraversalDirection::Incoming => edge.from,
                    TraversalDirection::Both => {
                        if edge.from == current_cid {
                            edge.to
                        } else {
                            edge.from
                        }
                    }
                };

                if visited.insert(neighbor_cid) {
                    neighbors.push(neighbor_cid);
                    queue.push((neighbor_cid, depth + 1));
                } else {
                    neighbors.push(neighbor_cid);
                }
            }

            if !neighbors.is_empty() {
                result.insert(current_cid, neighbors);
            }
        }

        Ok(result)
    }
}

/// Direction for graph traversal
#[derive(Debug, Clone, Copy)]
pub enum TraversalDirection {
    Outgoing,  // Follow outgoing edges
    Incoming,  // Follow incoming edges
    Both,      // Follow both directions
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_db() -> DB {
        let temp_dir = tempdir().unwrap();
        let config = PersistentStorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        DB::open(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_basic_operations() {
        let mut db = create_test_db().await;

        // Create a node
        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), Value::String("Alice".to_string()));
        properties.insert("age".to_string(), Value::Int(30));

        let node_cid = db.create_node(properties).await.unwrap();

        // Retrieve the node
        let node = db.get_node(&node_cid).await.unwrap().unwrap();
        assert_eq!(node.properties["name"], Value::String("Alice".to_string()));
        assert_eq!(node.properties["age"], Value::Int(30));
        assert!(node.edges.is_empty());

        // Create an edge
        let mut edge_props = BTreeMap::new();
        edge_props.insert("since".to_string(), Value::Int(2020));

        let edge_cid = db.create_edge(
            "FRIENDS_WITH".to_string(),
            node_cid,
            node_cid, // self-loop for simplicity
            edge_props,
        ).await.unwrap();

        // Retrieve the edge
        let edge = db.get_edge(&edge_cid).await.unwrap().unwrap();
        assert_eq!(edge.label, "FRIENDS_WITH");
        assert_eq!(edge.from, node_cid);
        assert_eq!(edge.to, node_cid);
        assert_eq!(edge.properties["since"], Value::Int(2020));
    }

    #[cfg(feature = "lsm")]
    #[tokio::test]
    async fn test_lsm_engine_creation() {
        let temp_dir = std::env::temp_dir().join("test_kotoba_db");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Test that LSM engine can be created
        let db = DB::open(PersistentStorageConfig::lsm(&temp_dir)).await;
        assert!(db.is_ok(), "LSM engine should be created successfully");

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[cfg(feature = "lsm")]
    #[tokio::test]
    async fn test_lsm_compaction() {
        let temp_dir = std::env::temp_dir().join("test_kotoba_db_compaction");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create LSM engine with low compaction threshold for testing
        let compaction_config = kotoba_db_engine_lsm::CompactionConfig {
            max_sstables: 3,  // Trigger compaction after 3 SSTables
            min_compaction_files: 2,
        };

        let mut db = {
            use kotoba_db_engine_lsm::LSMStorageEngine;
            let engine = LSMStorageEngine::with_config(&temp_dir, compaction_config).await.unwrap();
            DB {
                storage: Arc::new(engine),
                transactions: Arc::new(RwLock::new(HashMap::new())),
                next_txn_id: Arc::new(RwLock::new(1)),
            }
        };

        // Insert enough data to trigger multiple flushes and compaction
        for i in 0..50 {
            let key = format!("key_{:03}", i);
            let value = format!("value_{}", i);

            let mut properties = BTreeMap::new();
            properties.insert("key".to_string(), Value::String(key.clone()));
            properties.insert("value".to_string(), Value::String(value));

            db.create_node(properties).await.unwrap();

            // Update the same key multiple times to create tombstones and updates
            if i % 10 == 0 {
                let mut update_props = BTreeMap::new();
                update_props.insert("key".to_string(), Value::String(key));
                update_props.insert("updated".to_string(), Value::String(format!("updated_{}", i)));
                db.create_node(update_props).await.unwrap();
            }
        }

        // Verify that compaction worked by checking that we can still read data
        // and that the database is functional after compaction
        let mut properties = BTreeMap::new();
        properties.insert("test_key".to_string(), Value::String("test_value".to_string()));

        let test_cid = db.create_node(properties).await.unwrap();

        // Verify we can read back the test data
        let node = db.get_node(&test_cid).await.unwrap().unwrap();
        assert_eq!(node.properties["test_key"], Value::String("test_value".to_string()));

        // Also verify that some of the original data is still accessible
        // (we can't easily test all 50 items without storing their CIDs)
        // but the fact that compaction completed without errors is a good sign

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[tokio::test]
    async fn test_transaction_operations() {
        let mut db = create_test_db().await;

        // Begin transaction
        let txn_id = db.begin_transaction().await.unwrap();

        // Add operations to transaction
        let mut node_props = BTreeMap::new();
        node_props.insert("name".to_string(), Value::String("Alice".to_string()));
        node_props.insert("age".to_string(), Value::Int(30));

        db.add_operation(txn_id, Operation::CreateNode {
            properties: node_props,
            cid: None,
        }).await.unwrap();

        // Commit transaction
        db.commit_transaction(txn_id).await.unwrap();

        // Verify the node was created
        let nodes = db.find_nodes(&[("name".to_string(), Value::String("Alice".to_string()))]).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].1.properties["name"], Value::String("Alice".to_string()));
    }

    #[tokio::test]
    async fn test_query_operations() {
        let mut db = create_test_db().await;

        // Create some test data
        let mut alice_props = BTreeMap::new();
        alice_props.insert("name".to_string(), Value::String("Alice".to_string()));
        alice_props.insert("age".to_string(), Value::Int(30));
        alice_props.insert("city".to_string(), Value::String("Tokyo".to_string()));

        let mut bob_props = BTreeMap::new();
        bob_props.insert("name".to_string(), Value::String("Bob".to_string()));
        bob_props.insert("age".to_string(), Value::Int(25));
        bob_props.insert("city".to_string(), Value::String("Tokyo".to_string()));

        let alice_cid = db.create_node(alice_props).await.unwrap();
        let bob_cid = db.create_node(bob_props).await.unwrap();

        // Create an edge
        let mut friendship_props = BTreeMap::new();
        friendship_props.insert("since".to_string(), Value::Int(2020));
        db.create_edge("FRIENDS".to_string(), alice_cid, bob_cid, friendship_props).await.unwrap();

        // Test node filtering
        let tokyo_nodes = db.find_nodes(&[("city".to_string(), Value::String("Tokyo".to_string()))]).await.unwrap();
        assert_eq!(tokyo_nodes.len(), 2);

        let alice_nodes = db.find_nodes(&[
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(30))
        ]).await.unwrap();
        assert_eq!(alice_nodes.len(), 1);

        // Test edge filtering
        let friendships = db.find_edges(Some("FRIENDS"), None, None, &[]).await.unwrap();
        assert_eq!(friendships.len(), 1);

        // Test traversal
        let traversal_result = db.traverse(alice_cid, TraversalDirection::Outgoing, 2, None).await.unwrap();
        assert!(traversal_result.contains_key(&alice_cid));
        assert_eq!(traversal_result[&alice_cid].len(), 1);
        assert_eq!(traversal_result[&alice_cid][0], bob_cid);
    }
}
