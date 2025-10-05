use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json;
use blake3::hash;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::core::{Node, Edge, Incidence, ProgramInteractionHypergraph, SubgraphInfo, SubgraphMembers, CidMetadata};

/// Represents available hash algorithms for CID computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Blake3,
    Sha256,
    Sha3_256,
}

/// Represents available multibase encodings for CID representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultibaseEncoding {
    Base64Url,
    Base58Btc,
    Base32,
    Base16,
}

/// System for computing Content IDs (CIDs) for PIH objects.
#[derive(Debug, Clone)]
pub struct CidSystem {
    hash_algorithm: HashAlgorithm,
    multibase_encoding: MultibaseEncoding,
}

impl CidSystem {
    /// Create a new CID system with default settings (Blake3 + Base64URL).
    pub fn new() -> Self {
        Self {
            hash_algorithm: HashAlgorithm::Blake3,
            multibase_encoding: MultibaseEncoding::Base64Url,
        }
    }

    /// Create a new CID system with custom settings.
    pub fn with_settings(hash_algorithm: HashAlgorithm, multibase_encoding: MultibaseEncoding) -> Self {
        Self {
            hash_algorithm,
            multibase_encoding,
        }
    }

    /// Compute CID for a node.
    pub fn compute_node_cid(&self, node: &Node) -> Result<String, Box<dyn std::error::Error>> {
        let canonical_node = CanonicalNode {
            kind: node.kind.clone(),
            node_type: node.node_type.clone(),
            attributes: node.attributes.clone(),
        };
        let payload = serde_json::to_string(&canonical_node)?;
        self.compute_cid_from_string(&payload)
    }

    /// Compute CID for an edge.
    pub fn compute_edge_cid(&self, edge: &Edge) -> Result<String, Box<dyn std::error::Error>> {
        let canonical_edge = CanonicalEdge {
            kind: edge.kind.clone(),
            label: edge.label.clone(),
            attributes: edge.attributes.clone(),
        };
        let payload = serde_json::to_string(&canonical_edge)?;
        self.compute_cid_from_string(&payload)
    }

    /// Compute CID for an incidence.
    pub fn compute_incidence_cid(&self, incidence: &Incidence) -> Result<String, Box<dyn std::error::Error>> {
        let canonical_incidence = CanonicalIncidence {
            edge_id: incidence.edge.clone(),
            node_id: incidence.node.clone(),
            role: incidence.role.clone(),
            idx: incidence.idx,
            attrs: incidence.attrs.clone(),
        };
        let payload = serde_json::to_string(&canonical_incidence)?;
        self.compute_cid_from_string(&payload)
    }

    /// Compute graph CID (Merkle root over all CIDs).
    pub fn compute_graph_cid(&self, pih: &ProgramInteractionHypergraph) -> Result<String, Box<dyn std::error::Error>> {
        // Collect all node, edge, and incidence CIDs
        let mut all_cids: Vec<String> = Vec::new();

        // Add edge CIDs
        for edge in &pih.edges {
            if let Some(cid) = &edge.cid {
                all_cids.push(cid.clone());
            }
        }

        // Add node CIDs
        for node in &pih.nodes {
            if let Some(cid) = &node.cid {
                all_cids.push(cid.clone());
            }
        }

        // Add incidence CIDs
        for incidence in &pih.incidences {
            if let Some(cid) = &incidence.cid {
                all_cids.push(cid.clone());
            }
        }

        // Sort for deterministic Merkle root
        all_cids.sort();

        // Compute Merkle root over sorted CIDs
        let mut combined_payload = String::new();
        for cid in &all_cids {
            combined_payload.push_str(cid);
        }

        self.compute_merkle_root(&combined_payload)
    }

    /// Compute Merkle root over sorted member CIDs.
    pub fn compute_merkle_root(&self, payload: &str) -> Result<String, Box<dyn std::error::Error>> {
        let hash_bytes = match self.hash_algorithm {
            HashAlgorithm::Blake3 => hash(payload.as_bytes()).as_bytes().to_vec(),
            HashAlgorithm::Sha256 => {
                use sha2::Sha256;
                use sha2::Digest;
                let mut hasher = Sha256::new();
                hasher.update(payload.as_bytes());
                hasher.finalize().to_vec()
            }
            HashAlgorithm::Sha3_256 => {
                use sha3::Sha3_256;
                use sha3::Digest;
                let mut hasher = Sha3_256::new();
                hasher.update(payload.as_bytes());
                hasher.finalize().to_vec()
            }
        };

        self.encode_multibase(&hash_bytes)
    }

    /// Compute CID from a string payload.
    pub fn compute_cid_from_string(&self, payload: &str) -> Result<String, Box<dyn std::error::Error>> {
        let hash_bytes = match self.hash_algorithm {
            HashAlgorithm::Blake3 => hash(payload.as_bytes()).as_bytes().to_vec(),
            HashAlgorithm::Sha256 => {
                use sha2::Sha256;
                use sha2::Digest;
                let mut hasher = Sha256::new();
                hasher.update(payload.as_bytes());
                hasher.finalize().to_vec()
            }
            HashAlgorithm::Sha3_256 => {
                use sha3::Sha3_256;
                use sha3::Digest;
                let mut hasher = Sha3_256::new();
                hasher.update(payload.as_bytes());
                hasher.finalize().to_vec()
            }
        };

        self.encode_multibase(&hash_bytes)
    }

    /// Encode hash bytes with multibase encoding.
    fn encode_multibase(&self, data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        match self.multibase_encoding {
            MultibaseEncoding::Base64Url => {
                let encoded = URL_SAFE_NO_PAD.encode(data);
                Ok(format!("u{}", encoded))
            }
            MultibaseEncoding::Base58Btc => {
                use bs58::encode;
                let encoded = encode(data).into_string();
                Ok(format!("z{}", encoded))
            }
            MultibaseEncoding::Base32 => {
                use base32::encode;
                let encoded = encode(base32::Alphabet::RFC4648 { padding: false }, data);
                Ok(format!("b{}", encoded.to_lowercase()))
            }
            MultibaseEncoding::Base16 => {
                use hex::encode;
                let encoded = encode(data);
                Ok(format!("f{}", encoded.to_lowercase()))
            }
        }
    }

    /// Canonicalize a JSON value for deterministic hashing.
    pub fn canonicalize_value(&mut self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                // Sort keys in map
                let mut sorted_keys: Vec<&String> = map.keys().collect();
                sorted_keys.sort();

                let mut new_map = serde_json::Map::new();
                for key in sorted_keys {
                    if let Some(value) = map.get(key) {
                        let mut value_clone = value.clone();
                        self.canonicalize_value(&mut value_clone);
                        new_map.insert(key.clone(), value_clone);
                    }
                }
                *map = new_map;
            }
            serde_json::Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.canonicalize_value(item);
                }
                if arr.iter().all(|v| matches!(v, serde_json::Value::String(_))) {
                    let mut sorted_arr: Vec<_> = arr.drain(..).collect();
                    sorted_arr.sort_by(|a, b| {
                        match (a, b) {
                            (serde_json::Value::String(s1), serde_json::Value::String(s2)) => s1.cmp(s2),
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    *arr = sorted_arr;
                }
            }
            _ => {} // Other types don't need canonicalization
        }
    }
}

impl Default for CidSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Canonical representation of a node for CID computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CanonicalNode {
    pub kind: crate::core::NodeKind,
    pub node_type: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Canonical representation of an edge for CID computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CanonicalEdge {
    pub kind: crate::core::EdgeKind,
    pub label: Option<String>,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Canonical representation of an incidence for CID computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CanonicalIncidence {
    pub edge_id: String,
    pub node_id: String,
    pub role: crate::core::RoleKind,
    pub idx: Option<u32>,
    pub attrs: HashMap<String, serde_json::Value>,
}

impl ProgramInteractionHypergraph {
    /// Compute and assign CIDs to all objects in the graph.
    pub fn compute_all_cids(&mut self, cid_system: &mut CidSystem) -> Result<(), Box<dyn std::error::Error>> {
        // Compute CIDs for edges (collect CIDs first to avoid borrow conflicts)
        let mut edge_cids = Vec::new();
        for edge in &self.edges {
            let cid = cid_system.compute_edge_cid(edge)?;
            edge_cids.push((edge.id.clone(), cid));
        }
        // Assign CIDs to edges
        for (id, cid) in edge_cids {
            if let Some(edge_mut) = self.edges.iter_mut().find(|e| e.id == id) {
                edge_mut.cid = Some(cid);
            }
        }

        // Compute CIDs for nodes
        let mut node_cids = Vec::new();
        for node in &self.nodes {
            let cid = cid_system.compute_node_cid(node)?;
            node_cids.push((node.id.clone(), cid));
        }
        // Assign CIDs to nodes
        for (id, cid) in node_cids {
            if let Some(node_mut) = self.nodes.iter_mut().find(|n| n.id == id) {
                node_mut.cid = Some(cid);
            }
        }

        // Compute CIDs for incidences
        let mut incidence_cids = Vec::new();
        for incidence in &self.incidences {
            let cid = cid_system.compute_incidence_cid(incidence)?;
            incidence_cids.push((incidence.edge.clone(), incidence.node.clone(), cid));
        }
        // Assign CIDs to incidences
        for (edge_id, node_id, cid) in incidence_cids {
            if let Some(inc_mut) = self.incidences.iter_mut().find(|i| i.edge == edge_id && i.node == node_id) {
                inc_mut.cid = Some(cid);
            }
        }

        // Compute graph CID
        self.graph_cid = Some(cid_system.compute_graph_cid(self)?);

        Ok(())
    }
}
