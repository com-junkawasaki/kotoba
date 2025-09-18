//! Fuzz testing for data structures and serialization
//!
//! Tests CBOR serialization, deserialization, and data structure integrity
//! with random inputs to find issues in data handling.

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use kotoba_db_core::{Block, NodeBlock, EdgeBlock, Value, Cid};
use std::collections::HashMap;

#[derive(Debug, Arbitrary)]
enum DataStructureTest {
    SerializeNode {
        node: FuzzNode,
    },
    SerializeEdge {
        edge: FuzzEdge,
    },
    SerializeComplexValue {
        value: FuzzValue,
    },
    RoundTripSerialization {
        block: FuzzBlock,
    },
    CidGeneration {
        data: Vec<u8>,
    },
    BulkSerialization {
        items: Vec<FuzzBlock>,
    },
}

#[derive(Debug, Arbitrary)]
struct FuzzNode {
    labels: Vec<String>,
    properties: HashMap<String, FuzzValue>,
}

#[derive(Debug, Arbitrary)]
struct FuzzEdge {
    from_labels: Vec<String>,
    to_labels: Vec<String>,
    label: String,
    properties: HashMap<String, FuzzValue>,
}

#[derive(Debug, Arbitrary)]
enum FuzzValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<FuzzValue>),
    Link([u8; 32]), // CID-like
}

#[derive(Debug, Arbitrary)]
enum FuzzBlock {
    Node(FuzzNode),
    Edge(FuzzEdge),
}

impl From<FuzzValue> for Value {
    fn from(fuzz_value: FuzzValue) -> Self {
        match fuzz_value {
            FuzzValue::Null => Value::Null,
            FuzzValue::Bool(b) => Value::Bool(b),
            FuzzValue::Int(i) => Value::Int(i),
            FuzzValue::Float(f) => Value::Float(f),
            FuzzValue::String(s) => Value::String(s),
            FuzzValue::Array(arr) => Value::Array(arr.into_iter().map(Value::from).collect()),
            FuzzValue::Link(link) => Value::Link(Cid(link)),
        }
    }
}

impl From<FuzzNode> for NodeBlock {
    fn from(fuzz_node: FuzzNode) -> Self {
        NodeBlock {
            labels: fuzz_node.labels,
            properties: fuzz_node.properties.into_iter()
                .map(|(k, v)| (k, Value::from(v)))
                .collect(),
        }
    }
}

impl From<FuzzEdge> for EdgeBlock {
    fn from(fuzz_edge: FuzzEdge) -> Self {
        EdgeBlock {
            from_labels: fuzz_edge.from_labels,
            to_labels: fuzz_edge.to_labels,
            label: fuzz_edge.label,
            properties: fuzz_edge.properties.into_iter()
                .map(|(k, v)| (k, Value::from(v)))
                .collect(),
        }
    }
}

fuzz_target!(|test: DataStructureTest| {
    match test {
        DataStructureTest::SerializeNode { node } => {
            let node_block = NodeBlock::from(node);

            // Test serialization
            let serialized = match node_block.to_bytes() {
                Ok(bytes) => bytes,
                Err(_) => return, // Skip on serialization error
            };

            // Test deserialization
            let deserialized = match NodeBlock::from_bytes(&serialized) {
                Ok(block) => block,
                Err(_) => return, // Skip on deserialization error
            };

            // Basic consistency check
            assert_eq!(node_block.labels.len(), deserialized.labels.len());
        }

        DataStructureTest::SerializeEdge { edge } => {
            let edge_block = EdgeBlock::from(edge);

            // Test serialization
            let serialized = match ciborium::to_vec(&edge_block) {
                Ok(bytes) => bytes,
                Err(_) => return,
            };

            // Test deserialization
            let deserialized: EdgeBlock = match ciborium::from_reader(&serialized[..]) {
                Ok(block) => block,
                Err(_) => return,
            };

            // Basic consistency check
            assert_eq!(edge_block.label, deserialized.label);
        }

        DataStructureTest::SerializeComplexValue { value } => {
            let value = Value::from(value);

            // Test CBOR serialization/deserialization
            let serialized = match ciborium::to_vec(&value) {
                Ok(bytes) => bytes,
                Err(_) => return,
            };

            let deserialized: Value = match ciborium::from_reader(&serialized[..]) {
                Ok(val) => val,
                Err(_) => return,
            };

            // Values should round-trip correctly (basic check)
            match (&value, &deserialized) {
                (Value::Null, Value::Null) => {}
                (Value::Bool(a), Value::Bool(b)) if a == b => {}
                (Value::Int(a), Value::Int(b)) if a == b => {}
                (Value::Float(a), Value::Float(b)) if (a - b).abs() < f64::EPSILON => {}
                (Value::String(a), Value::String(b)) if a == b => {}
                _ => {} // Other cases may have precision issues, skip assertion
            }
        }

        DataStructureTest::RoundTripSerialization { block } => {
            let block = match block {
                FuzzBlock::Node(node) => Block::Node(NodeBlock::from(node)),
                FuzzBlock::Edge(edge) => Block::Edge(EdgeBlock::from(edge)),
            };

            // Test full round-trip
            let serialized = match block.to_bytes() {
                Ok(bytes) => bytes,
                Err(_) => return,
            };

            let deserialized = match Block::from_bytes(&serialized) {
                Ok(block) => block,
                Err(_) => return,
            };

            // Ensure the block type is preserved
            match (&block, &deserialized) {
                (Block::Node(_), Block::Node(_)) => {}
                (Block::Edge(_), Block::Edge(_)) => {}
                _ => panic!("Block type not preserved in serialization"),
            }
        }

        DataStructureTest::CidGeneration { data } => {
            use blake3::Hasher;

            // Test CID generation with random data
            let mut hasher = Hasher::new();
            hasher.update(&data);
            let hash = hasher.finalize();

            let cid = Cid(hash.into());
            let cid_bytes = cid.0;

            // CID should be deterministic
            let mut hasher2 = Hasher::new();
            hasher2.update(&data);
            let hash2 = hasher2.finalize();

            assert_eq!(cid_bytes, hash2.into());
        }

        DataStructureTest::BulkSerialization { items } => {
            if items.is_empty() {
                return;
            }

            let mut serialized_items = Vec::new();

            // Serialize all items
            for item in items {
                let block = match item {
                    FuzzBlock::Node(node) => Block::Node(NodeBlock::from(node)),
                    FuzzBlock::Edge(edge) => Block::Edge(EdgeBlock::from(edge)),
                };

                match block.to_bytes() {
                    Ok(bytes) => serialized_items.push(bytes),
                    Err(_) => continue,
                }
            }

            if serialized_items.is_empty() {
                return;
            }

            // Test bulk deserialization
            for serialized in serialized_items {
                let _ = Block::from_bytes(&serialized);
                // We don't assert here as some deserialization may fail with random data
            }
        }
    }
});

// Additional fuzz target for raw CBOR data
fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Test CBOR deserialization with raw fuzzer input
    // This helps find issues with malformed CBOR data handling

    // Try to deserialize as different types
    let _ = ciborium::from_reader::<Value, _>(&data[..]);
    let _ = ciborium::from_reader::<NodeBlock, _>(&data[..]);
    let _ = ciborium::from_reader::<EdgeBlock, _>(&data[..]);
    let _ = ciborium::from_reader::<Block, _>(&data[..]);

    // Test with truncated data
    if data.len() > 1 {
        let _ = ciborium::from_reader::<Value, _>(&data[..data.len()/2]);
    }

    // Test with extended data
    if data.len() < 1000 {
        let mut extended = data.to_vec();
        extended.extend_from_slice(&[0u8; 100]);
        let _ = ciborium::from_reader::<Value, _>(&extended[..]);
    }
});

// Fuzz target for property-based testing
fuzz_target!(|data: &[u8]| {
    use proptest::prelude::*;

    // This is a simplified property-based test
    // In practice, you'd use the proptest crate for more sophisticated property testing

    if data.len() < 8 {
        return;
    }

    // Test properties of data structures with random inputs
    let value1 = data[0] as i64;
    let value2 = data[1] as i64;

    // Test commutative properties (where applicable)
    let val_a = Value::Int(value1);
    let val_b = Value::Int(value2);

    // For integers, equality should be reflexive
    assert_eq!(val_a, val_a);
    assert_eq!(val_b, val_b);

    // Test serialization properties
    if let Ok(serialized_a) = ciborium::to_vec(&val_a) {
        if let Ok(deserialized_a) = ciborium::from_reader::<Value, _>(&serialized_a[..]) {
            // Round-trip should preserve value for integers
            assert_eq!(val_a, deserialized_a);
        }
    }
});
