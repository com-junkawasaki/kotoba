//! EAF-IPG Runtime Execution Engine
//!
//! Executes EAF-IPG graphs by:
//! 1. Lowering multi-layer graphs to execution DAGs
//! 2. Scheduling operations with Kahn algorithm + priority queue
//! 3. Executing operations with capability checks

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::*;
use crate::Error;

/// Lower multi-layer EAF-IPG graph to execution DAG
pub fn lower_to_exec_dag(graph: &Graph) -> Result<ExecDag, Error> {
    let mut exec_nodes = Vec::new();
    let mut exec_edges = Vec::new();

    // 1. Map nodes to execution operations
    let mut node_to_op = HashMap::new();
    for node in &graph.node {
        let op = map_node_to_op(node)?;
        node_to_op.insert(&node.id, exec_nodes.len());
        exec_nodes.push(ExecNode {
            id: node.id.clone(),
            op,
            properties: node.properties.clone(),
        });
    }

    // 2. Build dependency edges from all layers
    build_data_dependencies(graph, &node_to_op, &mut exec_edges)?;
    build_control_dependencies(graph, &node_to_op, &mut exec_edges)?;
    build_memory_dependencies(graph, &node_to_op, &mut exec_edges)?;
    build_time_dependencies(graph, &node_to_op, &mut exec_edges)?;

    // 3. Inject capability checks before memory operations
    inject_capability_checks(graph, &node_to_op, &mut exec_nodes, &mut exec_edges)?;

    Ok(ExecDag {
        nodes: exec_nodes,
        edges: exec_edges,
    })
}

/// Map EAF-IPG node to execution operation
fn map_node_to_op(node: &Node) -> Result<OpKind, Error> {
    match node.kind.as_str() {
        "Phi" => {
            // Count arity from properties or assume from context
            Ok(OpKind::Phi { arity: 2 }) // Simplified
        }
        "Load" => Ok(OpKind::CapLoad),
        "Store" => Ok(OpKind::CapStore),
        "Call" => Ok(OpKind::Call),
        "Branch" => Ok(OpKind::Branch),
        "Capability" => {
            // Capability nodes are handled separately during injection
            Ok(OpKind::Effect { effect_type: "capability_check".to_string() })
        }
        "Mmio" => {
            // Determine read/write from properties
            let is_read = node.properties.get("operation")
                .and_then(|v| v.as_str())
                .map(|s| s == "read")
                .unwrap_or(true);
            if is_read {
                Ok(OpKind::MmioRead)
            } else {
                Ok(OpKind::MmioWrite)
            }
        }
        _ => Ok(OpKind::Effect { effect_type: node.kind.clone() }),
    }
}

/// Build data flow dependencies
fn build_data_dependencies(
    graph: &Graph,
    node_to_op: &HashMap<&String, usize>,
    exec_edges: &mut Vec<ExecEdge>,
) -> Result<(), Error> {
    for edge in &graph.edge {
        if edge.layer != Layer::Data {
            continue;
        }

        let sources = get_edge_sources(graph, &edge.id);
        let targets = get_edge_targets(graph, &edge.id);

        for &source_idx in &sources {
            for &target_idx in &targets {
                exec_edges.push(ExecEdge {
                    from: graph.node[source_idx].id.clone(),
                    to: graph.node[target_idx].id.clone(),
                    kind: ExecEdgeKind::Data,
                });
            }
        }
    }
    Ok(())
}

/// Build control flow dependencies
fn build_control_dependencies(
    graph: &Graph,
    node_to_op: &HashMap<&String, usize>,
    exec_edges: &mut Vec<ExecEdge>,
) -> Result<(), Error> {
    for edge in &graph.edge {
        if edge.layer != Layer::Control {
            continue;
        }

        let sources = get_edge_sources(graph, &edge.id);
        let targets = get_edge_targets(graph, &edge.id);

        for &source_idx in &sources {
            for &target_idx in &targets {
                exec_edges.push(ExecEdge {
                    from: graph.node[source_idx].id.clone(),
                    to: graph.node[target_idx].id.clone(),
                    kind: ExecEdgeKind::Control,
                });
            }
        }
    }
    Ok(())
}

/// Build memory dependencies (MemorySSA)
fn build_memory_dependencies(
    graph: &Graph,
    node_to_op: &HashMap<&String, usize>,
    exec_edges: &mut Vec<ExecEdge>,
) -> Result<(), Error> {
    for edge in &graph.edge {
        if edge.layer != Layer::Memory {
            continue;
        }

        let sources = get_edge_sources(graph, &edge.id);
        let targets = get_edge_targets(graph, &edge.id);

        for &source_idx in &sources {
            for &target_idx in &targets {
                exec_edges.push(ExecEdge {
                    from: graph.node[source_idx].id.clone(),
                    to: graph.node[target_idx].id.clone(),
                    kind: ExecEdgeKind::Memory,
                });
            }
        }
    }
    Ok(())
}

/// Build time dependencies (happens-before)
fn build_time_dependencies(
    graph: &Graph,
    node_to_op: &HashMap<&String, usize>,
    exec_edges: &mut Vec<ExecEdge>,
) -> Result<(), Error> {
    for edge in &graph.edge {
        if edge.layer != Layer::Time {
            continue;
        }

        let sources = get_edge_sources(graph, &edge.id);
        let targets = get_edge_targets(graph, &edge.id);

        for &source_idx in &sources {
            for &target_idx in &targets {
                exec_edges.push(ExecEdge {
                    from: graph.node[source_idx].id.clone(),
                    to: graph.node[target_idx].id.clone(),
                    kind: ExecEdgeKind::Time,
                });
            }
        }
    }
    Ok(())
}

/// Inject capability checks before memory operations
fn inject_capability_checks(
    graph: &Graph,
    node_to_op: &HashMap<&String, usize>,
    exec_nodes: &mut Vec<ExecNode>,
    exec_edges: &mut Vec<ExecEdge>,
) -> Result<(), Error> {
    for node in &graph.node {
        if !matches!(node.kind.as_str(), "Load" | "Store" | "Call") {
            continue;
        }

        // Find associated capability
        let capability_id = find_node_capability(graph, &node.id)?;
        let cap_check_id = format!("{}_cap_check", node.id);

        // Insert capability check node
        let cap_check_node = ExecNode {
            id: cap_check_id.clone(),
            op: OpKind::Effect { effect_type: "capability_check".to_string() },
            properties: IndexMap::new(),
        };
        exec_nodes.push(cap_check_node);

        // Add dependency: capability check -> memory operation
        exec_edges.push(ExecEdge {
            from: cap_check_id,
            to: node.id.clone(),
            kind: ExecEdgeKind::Enable,
        });
    }
    Ok(())
}

/// Find capability associated with a memory operation
fn find_node_capability(graph: &Graph, node_id: &str) -> Result<String, Error> {
    for inc in &graph.incidence {
        if inc.node == node_id && inc.role == "cap_out" {
            if let Some(edge) = graph.get_edge(&inc.edge) {
                if edge.layer == Layer::Capability {
                    // Find the capability node
                    for cap_inc in &graph.incidence {
                        if cap_inc.edge == inc.edge && cap_inc.role == "cap_in" {
                            return Ok(cap_inc.node.clone());
                        }
                    }
                }
            }
        }
    }
    Err(Error::Validation(format!("No capability found for node {}", node_id)))
}

/// Get source node indices for an edge
fn get_edge_sources(graph: &Graph, edge_id: &str) -> Vec<usize> {
    graph.incidence.iter()
        .filter(|inc| inc.edge == edge_id && inc.role == "source")
        .filter_map(|inc| graph.node.iter().position(|n| n.id == inc.node))
        .collect()
}

/// Get target node indices for an edge
fn get_edge_targets(graph: &Graph, edge_id: &str) -> Vec<usize> {
    graph.incidence.iter()
        .filter(|inc| inc.edge == edge_id && inc.role == "target")
        .filter_map(|inc| graph.node.iter().position(|n| n.id == inc.node))
        .collect()
}

/// Schedule and execute operations
pub async fn schedule_and_run(
    runtime: &mut Runtime,
    exec_dag: &ExecDag,
) -> Result<(), Error> {
    // Build dependency tracking
    let mut indegrees = HashMap::new();
    let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

    // Initialize indegrees
    for node in &exec_dag.nodes {
        indegrees.insert(node.id.clone(), 0);
        adj_list.insert(node.id.clone(), Vec::new());
    }

    // Build graph and calculate indegrees
    for edge in &exec_dag.edges {
        *indegrees.entry(edge.to.clone()).or_insert(0) += 1;
        adj_list.entry(edge.from.clone()).or_insert(Vec::new()).push(edge.to.clone());
    }

    // Kahn's algorithm with priority queue
    let mut queue = VecDeque::new();

    // Start with nodes that have no dependencies
    for (id, &degree) in &indegrees {
        if degree == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut executed = HashSet::new();

    while let Some(node_id) = queue.pop_front() {
        if executed.contains(&node_id) {
            continue;
        }

        // Execute the operation
        execute_operation(runtime, exec_dag, &node_id).await?;
        executed.insert(node_id.clone());

        // Update indegrees and add new ready nodes
        if let Some(neighbors) = adj_list.get(&node_id) {
            for neighbor in neighbors {
                if let Some(degree) = indegrees.get_mut(neighbor) {
                    *degree -= 1;
                    if *degree == 0 && !executed.contains(neighbor) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
    }

    // Check if all nodes were executed
    if executed.len() != exec_dag.nodes.len() {
        return Err(Error::Runtime("Cycle detected or unreachable nodes in execution DAG".to_string()));
    }

    Ok(())
}

/// Execute a single operation
async fn execute_operation(
    runtime: &mut Runtime,
    exec_dag: &ExecDag,
    node_id: &str,
) -> Result<(), Error> {
    let node = exec_dag.nodes.iter()
        .find(|n| n.id == node_id)
        .ok_or_else(|| Error::Runtime(format!("Node {} not found", node_id)))?;

    match &node.op {
        OpKind::Phi { arity } => {
            // Phi selection based on control flow - simplified
            runtime.values.insert(node_id.to_string(), Value::Int(42)); // Placeholder
        }

        OpKind::CapLoad => {
            // Capability-checked load
            let address = runtime.values.get("address")
                .and_then(|v| match v {
                    Value::Address(addr) => Some(*addr),
                    _ => None,
                })
                .unwrap_or(0);

            // Check capability bounds/permissions (simplified)
            let value = runtime.memory.get(&address).copied().unwrap_or(0);
            runtime.values.insert(node_id.to_string(), Value::Int(value as i64));
        }

        OpKind::CapStore => {
            // Capability-checked store - simplified
            let address = 100; // Placeholder
            let value = 123;   // Placeholder
            runtime.memory.insert(address, value as u8);
        }

        OpKind::MmioRead => {
            // MMIO read with happens-before ordering - simplified
            let value = 0xFF; // Placeholder for MMIO read
            runtime.values.insert(node_id.to_string(), Value::Int(value));
        }

        OpKind::MmioWrite => {
            // MMIO write - simplified
            // In real implementation, this would write to hardware registers
        }

        OpKind::Effect { effect_type } => {
            // Handle effects
            match effect_type.as_str() {
                "capability_check" => {
                    // Perform capability validation - simplified
                    // In practice, this would check bounds, permissions, tags
                }
                _ => {
                    // Other effects
                }
            }
        }

        _ => {
            // Default handling for other operations
            runtime.values.insert(node_id.to_string(), Value::Int(0));
        }
    }

    Ok(())
}
