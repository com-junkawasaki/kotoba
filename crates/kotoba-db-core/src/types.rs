//! Defines the core data structures for KotobaDB.
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// A Content ID (CID), which is the BLAKE3 hash of a serialized Block.
pub type Cid = [u8; 32];

/// A generic, serializable value that can be stored as a property.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    /// A link to another content-addressed block in the database.
    Link(Cid),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

/// The fundamental, content-addressed unit of storage.
/// A block's CID is the hash of its CBOR-serialized representation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Block {
    Node(NodeBlock),
    Edge(EdgeBlock),
}

/// Represents a node (or vertex) in the graph.
/// It contains properties and a list of CIDs pointing to its incident edges.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NodeBlock {
    pub properties: BTreeMap<String, Value>,
    pub edges: Vec<Cid>,
}

/// Represents an edge in the graph, connecting two nodes.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EdgeBlock {
    pub label: String,
    pub from: Cid,
    pub to: Cid,
    pub properties: BTreeMap<String, Value>,
}
