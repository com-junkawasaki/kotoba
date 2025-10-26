use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Represents the kind of a node in the hypergraph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// Value node: SSA value, constant, argument, or return value.
    Val,
    /// Object node: Object, array, or composite data structure.
    Obj,
    /// State node: Memory state or versioned data.
    State,
    /// Control node: Control point, branch, or join point.
    Ctrl,
    /// UI node: User interface element or interaction point.
    UI,
    /// Other node: Custom or specialized node types.
    Other,
}

/// Represents the kind of an edge in the hypergraph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Event edge: Computation operations (add, mul, for, etc.)
    Event,
    /// Flow edge: Data or state flow relationships (effects, dependencies)
    Flow,
    /// Meta edge: Metadata relationships (alias, reference, etc.)
    Meta,
}

/// Represents the role of a node in an incidence relationship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RoleKind {
    // Event roles
    DataIn,
    DataOut,
    CtrlIn,
    CtrlOut,
    StateIn,
    StateOut,
    Obj,
    ExcOut,
    // Flow roles
    Src,
    Dst,
    // Meta roles
    Left,
    Right,
    // Custom roles (for extensibility)
    Custom(String),
}

/// Represents a node in the Program Interaction Hypergraph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    #[serde(rename = "type")]
    pub node_type: String,
    // Legacy field for backward compatibility (deprecated, use node_type instead)
    #[serde(rename = "entity_type", default)]
    pub entity_type: Option<String>,
    // Additional attributes based on kind
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
    /// Content ID for canonical representation (for content-addressable storage)
    #[serde(default = "default_cid")]
    pub cid: Option<String>,
}

/// Represents an edge in the Program Interaction Hypergraph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    pub id: String,
    pub kind: EdgeKind,
    // Common attributes
    #[serde(default)]
    pub label: Option<String>,
    // Event-specific attributes (only used when kind is Event)
    #[serde(default)]
    pub opcode: Option<String>,
    #[serde(default)]
    pub dtype: Option<String>,
    #[serde(default = "default_can_throw")]
    pub can_throw: bool,
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
    /// Content ID for canonical representation (for content-addressable storage)
    #[serde(default = "default_cid")]
    pub cid: Option<String>,
}

/// Represents an incidence connecting an edge to a node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Incidence {
    pub edge: String,
    pub node: String,
    pub role: RoleKind,
    /// Index for ordering multiple incidences with same edge and role
    #[serde(default)]
    pub idx: Option<u32>,
    /// Additional attributes for this incidence
    #[serde(default)]
    pub attrs: HashMap<String, serde_json::Value>,
    /// Content ID for canonical representation (for content-addressable storage)
    #[serde(default = "default_cid")]
    pub cid: Option<String>,
}

/// The main Program Interaction Hypergraph structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInteractionHypergraph {
    /// Metadata about the graph
    #[serde(default)]
    pub meta: HashMap<String, serde_json::Value>,
    /// All nodes in the hypergraph (formerly entities)
    pub nodes: Vec<Node>,
    /// All edges in the hypergraph (formerly events)
    pub edges: Vec<Edge>,
    /// All incidences connecting nodes and edges
    pub incidences: Vec<Incidence>,
    /// Node embeddings computed by GNN for learning-based optimization
    #[serde(default)]
    pub node_embeddings: HashMap<String, Vec<f32>>,
    /// Content ID for the entire hypergraph
    #[serde(default = "default_cid")]
    pub graph_cid: Option<String>,
    /// Subgraphs with their Merkle DAG CIDs
    #[serde(default)]
    pub subgraphs: HashMap<String, SubgraphInfo>,
    /// Embedding cache: CID -> embedding vector
    #[serde(default)]
    pub embedding_cache: HashMap<String, Vec<f32>>,
    /// Metadata for CID computation
    #[serde(default)]
    pub cid_metadata: Option<CidMetadata>,
}

/// Information about a subgraph in the Merkle DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphInfo {
    pub members: SubgraphMembers,
    pub gcid: String,
}

/// Members of a subgraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphMembers {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub incidences: Vec<String>,
}

/// Metadata for CID computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CidMetadata {
    pub hash_algorithm: String,
    pub multibase_encoding: String,
    pub canonicalization_rules: Vec<String>,
}

/// Default function for CID field (returns None)
fn default_cid() -> Option<String> {
    None
}

/// Default function for can_throw field (returns false)
fn default_can_throw() -> bool {
    false
}

impl ProgramInteractionHypergraph {
    /// Create a new empty Program Interaction Hypergraph.
    pub fn new() -> Self {
        Self {
            meta: HashMap::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            incidences: Vec::new(),
            node_embeddings: HashMap::new(),
            graph_cid: None,
            subgraphs: HashMap::new(),
            embedding_cache: HashMap::new(),
            cid_metadata: None,
        }
    }

    /// Create a subgraph from the current graph.
    pub fn create_subgraph(&self, node_ids: Vec<String>, edge_ids: Vec<String>) -> Option<ProgramInteractionHypergraph> {
        let mut subgraph = ProgramInteractionHypergraph::new();

        // Find nodes by iterating over self.nodes
        let subgraph_nodes: Vec<Node> = self.nodes.iter()
            .filter(|node| node_ids.contains(&node.id))
            .cloned()
            .collect();

        // Find edges by iterating over self.edges
        let subgraph_edges: Vec<Edge> = self.edges.iter()
            .filter(|edge| edge_ids.contains(&edge.id))
            .cloned()
            .collect();

        // Find incidences connecting the selected nodes and edges
        let subgraph_incidences: Vec<Incidence> = self.incidences.iter()
            .filter(|inc| edge_ids.contains(&inc.edge) && node_ids.contains(&inc.node))
            .cloned()
            .collect();

        if subgraph_nodes.is_empty() || subgraph_edges.is_empty() {
            return None;
        }

        subgraph.nodes = subgraph_nodes;
        subgraph.edges = subgraph_edges;
        subgraph.incidences = subgraph_incidences;

        Some(subgraph)
    }
}

impl PartialEq for ProgramInteractionHypergraph {
    fn eq(&self, other: &Self) -> bool {
        self.edges == other.edges &&
        self.nodes == other.nodes &&
        self.incidences == other.incidences &&
        self.meta == other.meta
        // Note: node_embeddings may not be compared for equality in rule matching
    }
}
