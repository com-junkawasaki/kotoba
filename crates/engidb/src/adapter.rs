use crate::{EngiDB, Result};
use kotoba_types::{Graph, Node};

/// Merkle-DAG note: This trait represents the storage/process node for graph I/O
/// in the overall process network. Adapters should keep dependencies minimal.
pub trait GraphAdapter {
    fn add_vertex(&self, node: &Node) -> Result<u64>;
    fn add_edge(&self, source_id: u64, edge_type: &str, target_id: u64) -> Result<()>;
    fn get_edges_from(&self, source_id: u64, edge_type: &str) -> Result<Vec<u64>>;
    fn import_graph(&self, graph: &Graph) -> Result<()>;
}

/// Default adapter backed by sled-based EngiDB
#[derive(Clone)]
pub struct SledAdapter {
    inner: EngiDB,
}

impl SledAdapter {
    pub fn new(inner: EngiDB) -> Self {
        Self { inner }
    }

    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(Self { inner: EngiDB::open(path)? })
    }

    pub fn inner(&self) -> &EngiDB {
        &self.inner
    }
}

impl GraphAdapter for SledAdapter {
    fn add_vertex(&self, node: &Node) -> Result<u64> {
        self.inner.add_vertex(node)
    }

    fn add_edge(&self, source_id: u64, edge_type: &str, target_id: u64) -> Result<()> {
        self.inner.add_edge(source_id, edge_type, target_id)
    }

    fn get_edges_from(&self, source_id: u64, edge_type: &str) -> Result<Vec<u64>> {
        self.inner.get_edges_from(source_id, edge_type)
    }

    fn import_graph(&self, graph: &Graph) -> Result<()> {
        self.inner.import_graph(graph)
    }
}

/// FCDB adapter implementation - disk-based persistent storage
/// This provides persistent storage capabilities for benchmarking
/// TODO: Replace with actual FCDB integration once API is fully understood
#[cfg(feature = "fcdb")]
pub mod fcdb_adapter {
    use super::GraphAdapter;
    use crate::Result;
    use kotoba_types::{Graph, Node};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::collections::HashMap;
    use sled::{Db, Tree};

    /// Disk-based FCDB adapter using sled for persistence
    #[derive(Clone)]
    pub struct FcdbAdapter {
        db: Arc<Db>,
        // Trees for different data types
        nodes_tree: Arc<Tree>,
        edges_tree: Arc<Tree>,
        id_counter_tree: Arc<Tree>,
        // In-memory caches for performance
        nodes_cache: Arc<std::sync::Mutex<HashMap<String, u64>>>,
        reverse_cache: Arc<std::sync::Mutex<HashMap<u64, String>>>,
        id_counter: Arc<std::sync::Mutex<u64>>,
    }

    impl FcdbAdapter {
        const NODES_TREE: &str = "nodes";
        const EDGES_TREE: &str = "edges";
        const ID_COUNTER_TREE: &str = "id_counter";

        pub fn new_sync(data_dir: PathBuf) -> Result<Self> {
            let db_path = data_dir.join("fcdb_data");
            let db = sled::open(db_path)
                .map_err(|e| crate::Error::Sled(e))?;

            let nodes_tree = db.open_tree(Self::NODES_TREE)
                .map_err(|e| crate::Error::Sled(e))?;
            let edges_tree = db.open_tree(Self::EDGES_TREE)
                .map_err(|e| crate::Error::Sled(e))?;
            let id_counter_tree = db.open_tree(Self::ID_COUNTER_TREE)
                .map_err(|e| crate::Error::Sled(e))?;

            // Initialize or load ID counter
            let mut initial_counter = 1u64;
            if let Some(counter_bytes) = id_counter_tree.get(b"counter")
                .map_err(|e| crate::Error::Sled(e))? {
                initial_counter = u64::from_be_bytes(counter_bytes.as_ref().try_into()
                    .map_err(|_| crate::Error::Serialization("Invalid counter bytes".to_string()))?);
            } else {
                id_counter_tree.insert(b"counter", &initial_counter.to_be_bytes())
                    .map_err(|e| crate::Error::Sled(e))?;
            }

            // Load existing nodes into cache
            let mut nodes_cache = HashMap::new();
            let mut reverse_cache = HashMap::new();
            for result in nodes_tree.iter() {
                let (key_bytes, value_bytes) = result
                    .map_err(|e| crate::Error::Sled(e))?;
                let node_id = String::from_utf8(key_bytes.to_vec())
                    .map_err(|e| crate::Error::Utf8(e.utf8_error()))?;
                let vertex_id = u64::from_be_bytes(value_bytes.as_ref().try_into()
                    .map_err(|_| crate::Error::Serialization("Invalid vertex ID bytes".to_string()))?);
                nodes_cache.insert(node_id.clone(), vertex_id);
                reverse_cache.insert(vertex_id, node_id);
            }

            Ok(Self {
                db: Arc::new(db),
                nodes_tree: Arc::new(nodes_tree),
                edges_tree: Arc::new(edges_tree),
                id_counter_tree: Arc::new(id_counter_tree),
                nodes_cache: Arc::new(std::sync::Mutex::new(nodes_cache)),
                reverse_cache: Arc::new(std::sync::Mutex::new(reverse_cache)),
                id_counter: Arc::new(std::sync::Mutex::new(initial_counter)),
            })
        }
    }

    impl GraphAdapter for FcdbAdapter {
        fn add_vertex(&self, node: &Node) -> Result<u64> {
            let mut nodes_cache = self.nodes_cache.lock().unwrap();
            let mut id_counter = self.id_counter.lock().unwrap();

            // Check if node already exists
            if let Some(existing_id) = nodes_cache.get(&node.id) {
                return Ok(*existing_id);
            }

            // Create new vertex
            let vertex_id = *id_counter;
            *id_counter += 1;

            // Store in database
            self.nodes_tree.insert(node.id.as_bytes(), &vertex_id.to_be_bytes())
                .map_err(|e| crate::Error::Sled(e))?;

            // Update ID counter in database
            self.id_counter_tree.insert(b"counter", &id_counter.to_be_bytes())
                .map_err(|e| crate::Error::Sled(e))?;

            // Update caches
            nodes_cache.insert(node.id.clone(), vertex_id);
            self.reverse_cache.lock().unwrap().insert(vertex_id, node.id.clone());

            Ok(vertex_id)
        }

        fn add_edge(&self, source_id: u64, edge_type: &str, target_id: u64) -> Result<()> {
            // Create edge key and store target IDs as a list
            let edge_key = format!("{}:{}", source_id, edge_type);

            // Get existing targets for this edge type
            let mut targets: Vec<u64> = if let Some(existing_data) = self.edges_tree.get(edge_key.as_bytes())
                .map_err(|e| crate::Error::Sled(e))? {
                bincode::deserialize(&existing_data)
                    .map_err(|e| crate::Error::Serialization(e.to_string()))?
            } else {
                Vec::new()
            };

            // Add new target if not already present
            if !targets.contains(&target_id) {
                targets.push(target_id);

                // Store updated targets
                let targets_data = bincode::serialize(&targets)
                    .map_err(|e| crate::Error::Serialization(e.to_string()))?;
                self.edges_tree.insert(edge_key.as_bytes(), targets_data)
                    .map_err(|e| crate::Error::Sled(e))?;
            }

            Ok(())
        }

        fn get_edges_from(&self, source_id: u64, edge_type: &str) -> Result<Vec<u64>> {
            let edge_key = format!("{}:{}", source_id, edge_type);

            if let Some(targets_data) = self.edges_tree.get(edge_key.as_bytes())
                .map_err(|e| crate::Error::Sled(e))? {
                let targets: Vec<u64> = bincode::deserialize(&targets_data)
                    .map_err(|e| crate::Error::Serialization(e.to_string()))?;
                Ok(targets)
            } else {
                Ok(Vec::new())
            }
        }

        fn import_graph(&self, graph: &Graph) -> Result<()> {
            // Import vertices first
            let mut node_id_map = std::collections::HashMap::new();
            for node in &graph.node {
                let vertex_id = self.add_vertex(node)?;
                node_id_map.insert(node.id.clone(), vertex_id);
            }

            // Import edges
            let mut edge_sources: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
            let mut edge_targets: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();

            for i in &graph.incidence {
                if i.role == "source" {
                    edge_sources.insert(&i.edge, &i.node);
                } else if i.role == "target" {
                    edge_targets.insert(&i.edge, &i.node);
                }
            }

            for edge in &graph.edge {
                if let (Some(source_node_id), Some(target_node_id)) = (
                    edge_sources.get(edge.id.as_str()),
                    edge_targets.get(edge.id.as_str())
                ) {
                    if let (Some(source_vertex_id), Some(target_vertex_id)) = (
                        node_id_map.get(*source_node_id),
                        node_id_map.get(*target_node_id)
                    ) {
                        self.add_edge(*source_vertex_id, &edge.kind, *target_vertex_id)?;
                    }
                }
            }

            Ok(())
        }
    }
}

