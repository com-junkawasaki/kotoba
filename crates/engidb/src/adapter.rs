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

/// Optional: FCDB adapter placeholder behind feature flag
#[cfg(feature = "fcdb")]
pub mod fcdb_adapter {
    use super::GraphAdapter;
    use crate::Result;
    use kotoba_types::{Graph, Node};

    /// Placeholder for FCDB-backed adapter. Implementations should wire to fcdb-* crates.
    #[derive(Clone)]
    pub struct FcdbAdapter;

    impl FcdbAdapter {
        pub fn new() -> Self { Self }
    }

    impl GraphAdapter for FcdbAdapter {
        fn add_vertex(&self, _node: &Node) -> Result<u64> {
            unimplemented!("fcdb feature enabled but adapter not implemented yet")
        }

        fn add_edge(&self, _source_id: u64, _edge_type: &str, _target_id: u64) -> Result<()> {
            unimplemented!("fcdb feature enabled but adapter not implemented yet")
        }

        fn get_edges_from(&self, _source_id: u64, _edge_type: &str) -> Result<Vec<u64>> {
            unimplemented!("fcdb feature enabled but adapter not implemented yet")
        }

        fn import_graph(&self, _graph: &Graph) -> Result<()> {
            unimplemented!("fcdb feature enabled but adapter not implemented yet")
        }
    }
}

