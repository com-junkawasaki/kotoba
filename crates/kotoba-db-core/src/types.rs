//! Defines the core data structures for KotobaDB.
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use anyhow::Result;
use blake3;
use ciborium;

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

impl Block {
    /// Computes the CID (Content ID) for this block.
    /// The CID is the BLAKE3 hash of the CBOR-serialized block.
    pub fn cid(&self) -> Result<Cid> {
        let mut hasher = blake3::Hasher::new();
        ciborium::into_writer(self, &mut hasher)?;
        let hash = hasher.finalize();
        Ok(hash.into())
    }

    /// Serializes this block to CBOR bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        ciborium::into_writer(self, &mut bytes)?;
        Ok(bytes)
    }

    /// Deserializes a block from CBOR bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let block: Block = ciborium::from_reader(bytes)?;
        Ok(block)
    }
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
