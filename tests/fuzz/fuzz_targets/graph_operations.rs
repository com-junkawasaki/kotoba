//! Fuzz testing for graph operations
//!
//! Tests random sequences of graph operations to find crashes, panics,
//! or assertion failures in the graph manipulation logic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use std::collections::HashMap;
use kotoba_db::{DB, Transaction};
use kotoba_db_core::{Block, NodeBlock, EdgeBlock, Value};
use tempfile::NamedTempFile;

#[derive(Debug, Arbitrary)]
enum GraphOperation {
    CreateNode {
        label: String,
        properties: HashMap<String, Value>,
    },
    UpdateNode {
        node_index: u8, // Use index to reference existing nodes
        properties: HashMap<String, Value>,
    },
    DeleteNode {
        node_index: u8,
    },
    CreateEdge {
        from_index: u8,
        to_index: u8,
        label: String,
        properties: HashMap<String, Value>,
    },
    UpdateEdge {
        edge_index: u8,
        properties: HashMap<String, Value>,
    },
    DeleteEdge {
        edge_index: u8,
    },
    QueryNodes {
        label_filter: Option<String>,
        property_filters: HashMap<String, Value>,
    },
    QueryEdges {
        label_filter: Option<String>,
    },
}

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    operations: Vec<GraphOperation>,
}

fuzz_target!(|input: FuzzInput| {
    // Setup temporary database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Run fuzz test in a tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        fuzz_graph_operations(db_path, input).await;
    });
});

async fn fuzz_graph_operations(db_path: &std::path::Path, input: FuzzInput) {
    let db = match DB::open_lsm(db_path).await {
        Ok(db) => db,
        Err(_) => return, // Skip if database creation fails
    };

    let mut created_nodes = Vec::new();
    let mut created_edges = Vec::new();

    for operation in input.operations {
        match operation {
            GraphOperation::CreateNode { label, properties } => {
                let node = NodeBlock {
                    labels: vec![label],
                    properties,
                };

                match db.put_block(&Block::Node(node)).await {
                    Ok(cid) => {
                        created_nodes.push(cid);
                    }
                    Err(_) => continue, // Skip operation on error
                }
            }

            GraphOperation::UpdateNode { node_index, properties } => {
                if created_nodes.is_empty() {
                    continue;
                }

                let node_idx = (node_index as usize) % created_nodes.len();
                let node_cid = created_nodes[node_idx];

                // Simplified update - in practice would need proper update logic
                let updated_node = NodeBlock {
                    labels: vec!["Updated".to_string()],
                    properties,
                };

                let _ = db.put_block(&Block::Node(updated_node)).await;
            }

            GraphOperation::DeleteNode { node_index } => {
                if created_nodes.is_empty() {
                    continue;
                }

                let node_idx = (node_index as usize) % created_nodes.len();
                created_nodes.remove(node_idx);
                // Note: Actual deletion would depend on implementation
            }

            GraphOperation::CreateEdge { from_index, to_index, label, properties } => {
                if created_nodes.len() < 2 {
                    continue;
                }

                let from_idx = (from_index as usize) % created_nodes.len();
                let to_idx = (to_index as usize) % created_nodes.len();

                if from_idx == to_idx {
                    continue; // Skip self-loops for simplicity
                }

                let from_cid = created_nodes[from_idx];
                let to_cid = created_nodes[to_idx];

                let edge = EdgeBlock {
                    from_labels: vec!["Node".to_string()],
                    to_labels: vec!["Node".to_string()],
                    label,
                    properties,
                };

                match db.put_block(&Block::Edge(edge)).await {
                    Ok(cid) => {
                        created_edges.push(cid);
                    }
                    Err(_) => continue,
                }
            }

            GraphOperation::UpdateEdge { edge_index, properties } => {
                if created_edges.is_empty() {
                    continue;
                }

                let edge_idx = (edge_index as usize) % created_edges.len();
                let edge_cid = created_edges[edge_idx];

                // Simplified update
                let updated_edge = EdgeBlock {
                    from_labels: vec!["Node".to_string()],
                    to_labels: vec!["Node".to_string()],
                    label: "Updated".to_string(),
                    properties,
                };

                let _ = db.put_block(&Block::Edge(updated_edge)).await;
            }

            GraphOperation::DeleteEdge { edge_index } => {
                if created_edges.is_empty() {
                    continue;
                }

                let edge_idx = (edge_index as usize) % created_edges.len();
                created_edges.remove(edge_idx);
            }

            GraphOperation::QueryNodes { label_filter, property_filters } => {
                // Test different query patterns
                if let Some(label) = label_filter {
                    let _ = db.find_nodes_by_label(&label).await;
                }

                // Test property-based queries (simplified)
                for (prop_name, prop_value) in property_filters {
                    let _ = db.find_nodes_by_property(&prop_name, &prop_value).await;
                }
            }

            GraphOperation::QueryEdges { label_filter } => {
                if let Some(label) = label_filter {
                    let _ = db.find_edges_by_label(&label).await;
                }
            }
        }
    }

    // Final consistency check - ensure basic operations still work
    let _ = db.find_nodes_by_label("Test").await;
}

// Additional fuzz target for specific edge cases
fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Test with raw bytes as potential graph data
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let db = match DB::open_lsm(db_path).await {
            Ok(db) => db,
            Err(_) => return,
        };

        // Try to interpret random bytes as various data structures
        // This helps find issues with data parsing and validation

        // Test 1: Random bytes as node properties
        if data.len() > 10 {
            let properties = HashMap::from([
                ("random_data".to_string(), Value::String(String::from_utf8_lossy(data).to_string())),
            ]);

            let node = NodeBlock {
                labels: vec!["FuzzTest".to_string()],
                properties,
            };

            let _ = db.put_block(&Block::Node(node)).await;
        }

        // Test 2: Random bytes split into multiple operations
        for chunk in data.chunks(32) {
            if chunk.len() >= 8 {
                let properties = HashMap::from([
                    ("chunk_data".to_string(), Value::String(String::from_utf8_lossy(chunk).to_string())),
                ]);

                let node = NodeBlock {
                    labels: vec!["ChunkTest".to_string()],
                    properties,
                };

                let _ = db.put_block(&Block::Node(node)).await;
            }
        }
    });
});
