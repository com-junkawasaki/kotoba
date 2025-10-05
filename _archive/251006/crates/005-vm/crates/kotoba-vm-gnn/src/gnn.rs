use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::core::{ProgramInteractionHypergraph, Edge, Node, NodeKind, EdgeKind, RoleKind, Incidence};

/// Features extracted from the Program Interaction Hypergraph for GNN processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnnFeatures {
    pub node_features: HashMap<String, Vec<f32>>,
    pub edge_features: Vec<Vec<f32>>,
    pub global_features: Vec<f32>,
    pub bipartite_features: Vec<f32>,
    pub hypergraph_features: Vec<f32>,
    pub hardware_features: Vec<f32>,
    pub optimization_labels: OptimizationLabels,
}

/// Labels for optimization decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationLabels {
    pub applied_rules: Vec<String>,
    pub performance_improvement: f32,
    pub power_consumption: f32,
    pub hardware_constraints: Vec<String>,
    pub optimization_impact: Vec<f32>,
}

/// Feature extractor for converting PIH to GNN input format.
pub struct FeatureExtractor;

impl FeatureExtractor {
    /// Extract all features from a Program Interaction Hypergraph.
    pub fn extract_features(pih: &ProgramInteractionHypergraph) -> GnnFeatures {
        let mut node_features = HashMap::new();
        let mut edge_features = Vec::new();
        let mut node_hyperedge_membership = HashMap::new();

        // Count event and entity nodes
        let edge_node_count = pih.edges.len();
        let entity_node_count = pih.nodes.len();

        // Extract node features (edges and nodes)
        for edge in &pih.edges {
            let features = Self::extract_edge_features(edge);
            node_features.insert(format!("edge_{}", edge.id), features);
            edge_features.push(Self::extract_edge_features(edge));
        }

        for node in &pih.nodes {
            let features = Self::extract_node_features(node);
            node_features.insert(format!("node_{}", node.id), features);
        }

        // Build hyperedge membership matrix
        for incidence in &pih.incidences {
            node_hyperedge_membership.insert(incidence.node.clone(), incidence.edge.clone());
        }

        // Extract global features (PIH-level statistics)
        let global_features = Self::extract_global_features(pih);

        // Extract bipartite features
        let bipartite_features = Self::extract_bipartite_features(pih, edge_node_count, entity_node_count);

        // Extract hypergraph features
        let hypergraph_features = Self::extract_hypergraph_features(pih, &node_hyperedge_membership);

        // Extract hardware-specific features
        let hardware_features = Self::extract_hardware_features(pih);

        // Generate optimization labels
        let optimization_labels = Self::generate_optimization_labels(pih);

        GnnFeatures {
            node_features,
            edge_features,
            global_features,
            bipartite_features,
            hypergraph_features,
            hardware_features,
            optimization_labels,
        }
    }

    /// Extract features for an edge.
    fn extract_edge_features(edge: &Edge) -> Vec<f32> {
        let mut features = Vec::new();

        // Opcode-based features (one-hot encoding)
        let opcode = edge.opcode.as_ref().unwrap_or(&"unknown".to_string()).clone();
        match opcode.as_str() {
            "add" => features.extend(&[1.0, 0.0, 0.0, 0.0, 0.0]),
            "mul" => features.extend(&[0.0, 1.0, 0.0, 0.0, 0.0]),
            "sub" => features.extend(&[0.0, 0.0, 1.0, 0.0, 0.0]),
            "div" => features.extend(&[0.0, 0.0, 0.0, 1.0, 0.0]),
            "for" => features.extend(&[0.0, 0.0, 0.0, 0.0, 1.0]),
            _ => features.extend(&[0.0, 0.0, 0.0, 0.0, 0.0]),
        }

        // Data type features
        let dtype = edge.dtype.as_ref().unwrap_or(&"unknown".to_string()).clone();
        match dtype.as_str() {
            "i32" => features.extend(&[1.0, 0.0, 0.0, 0.0]),
            "f32" => features.extend(&[0.0, 1.0, 0.0, 0.0]),
            "i64" => features.extend(&[0.0, 0.0, 1.0, 0.0]),
            "f64" => features.extend(&[0.0, 0.0, 0.0, 1.0]),
            _ => features.extend(&[0.0, 0.0, 0.0, 0.0]),
        }

        // Exception handling feature
        features.push(if edge.can_throw { 1.0 } else { 0.0 });

        // Edge attributes count
        features.push(edge.attributes.len() as f32);

        // Label presence
        features.push(if edge.label.is_some() { 1.0 } else { 0.0 });

        features
    }

    /// Extract features for a node.
    fn extract_node_features(node: &Node) -> Vec<f32> {
        let mut features = Vec::new();

        // Node kind features (one-hot encoding)
        match node.kind {
            NodeKind::Val => features.extend(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            NodeKind::Obj => features.extend(&[0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            NodeKind::State => features.extend(&[0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
            NodeKind::Ctrl => features.extend(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            NodeKind::UI => features.extend(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            NodeKind::Other => features.extend(&[0.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
        }

        // Node type features (simplified)
        let node_type = &node.node_type;
        if node_type.contains("i32") {
            features.extend(&[1.0, 0.0, 0.0, 0.0]);
        } else if node_type.contains("f32") {
            features.extend(&[0.0, 1.0, 0.0, 0.0]);
        } else if node_type.contains("i64") {
            features.extend(&[0.0, 0.0, 1.0, 0.0]);
        } else if node_type.contains("*") {
            features.extend(&[0.0, 0.0, 0.0, 1.0]);
        } else {
            features.extend(&[0.0, 0.0, 0.0, 0.0]);
        }

        // Attributes count
        features.push(node.attributes.len() as f32);

        // CID presence
        features.push(if node.cid.is_some() { 1.0 } else { 0.0 });

        features
    }

    /// Extract global features from the PIH.
    fn extract_global_features(pih: &ProgramInteractionHypergraph) -> Vec<f32> {
        let mut features = Vec::new();

        // Basic statistics
        features.push(pih.nodes.len() as f32);
        features.push(pih.edges.len() as f32);
        features.push(pih.incidences.len() as f32);

        // Node type distribution
        let val_count = pih.nodes.iter().filter(|n| n.kind == NodeKind::Val).count();
        let obj_count = pih.nodes.iter().filter(|n| n.kind == NodeKind::Obj).count();
        let state_count = pih.nodes.iter().filter(|n| n.kind == NodeKind::State).count();
        let ctrl_count = pih.nodes.iter().filter(|n| n.kind == NodeKind::Ctrl).count();

        features.push(val_count as f32);
        features.push(obj_count as f32);
        features.push(state_count as f32);
        features.push(ctrl_count as f32);

        // Edge type distribution
        let event_count = pih.edges.iter().filter(|e| e.kind == EdgeKind::Event).count();
        let flow_count = pih.edges.iter().filter(|e| e.kind == EdgeKind::Flow).count();
        let meta_count = pih.edges.iter().filter(|e| e.kind == EdgeKind::Meta).count();

        features.push(event_count as f32);
        features.push(flow_count as f32);
        features.push(meta_count as f32);

        // Average degree
        let avg_degree = if !pih.nodes.is_empty() {
            pih.incidences.len() as f32 / pih.nodes.len() as f32
        } else {
            0.0
        };
        features.push(avg_degree);

        features
    }

    /// Extract bipartite features for bipartite GNN.
    fn extract_bipartite_features(pih: &ProgramInteractionHypergraph, edge_count: usize, node_count: usize) -> Vec<f32> {
        let mut features = Vec::new();

        // Edge-to-node ratio
        let ratio = if node_count > 0 {
            edge_count as f32 / node_count as f32
        } else {
            0.0
        };
        features.push(ratio);

        // Connection density
        let max_possible_connections = edge_count * node_count;
        let density = if max_possible_connections > 0 {
            pih.incidences.len() as f32 / max_possible_connections as f32
        } else {
            0.0
        };
        features.push(density);

        // Average connections per edge
        let avg_connections = if edge_count > 0 {
            pih.incidences.len() as f32 / edge_count as f32
        } else {
            0.0
        };
        features.push(avg_connections);

        // Average connections per node
        let avg_connections_per_node = if node_count > 0 {
            pih.incidences.len() as f32 / node_count as f32
        } else {
            0.0
        };
        features.push(avg_connections_per_node);

        features
    }

    /// Extract hypergraph-specific features.
    fn extract_hypergraph_features(pih: &ProgramInteractionHypergraph, node_hyperedge_membership: &HashMap<String, String>) -> Vec<f32> {
        let mut features = Vec::new();

        // Calculate hyperedge sizes (number of entities per event)
        let mut hyperedge_sizes = Vec::new();
        let mut hyperedge_degree_distribution = HashMap::new();

        for edge in &pih.edges {
            let hyperedge_size = pih.incidences.iter()
                .filter(|inc| inc.edge == edge.id)
                .count();
            hyperedge_sizes.push(hyperedge_size);

            *hyperedge_degree_distribution.entry(hyperedge_size).or_insert(0) += 1;
        }

        // Average hyperedge size
        let avg_hyperedge_size = if !hyperedge_sizes.is_empty() {
            hyperedge_sizes.iter().sum::<usize>() as f32 / hyperedge_sizes.len() as f32
        } else {
            0.0
        };
        features.push(avg_hyperedge_size);

        // Maximum hyperedge size
        let max_hyperedge_size = hyperedge_sizes.iter().max().unwrap_or(&0);
        features.push(*max_hyperedge_size as f32);

        // Hypergraph clustering coefficient (simplified)
        let hypergraph_clustering_coeff = if pih.edges.is_empty() {
            0.0
        } else {
            avg_hyperedge_size / pih.nodes.len() as f32
        };
        features.push(hypergraph_clustering_coeff);

        // Degree distribution entropy (simplified)
        let degree_dist: HashMap<usize, f32> = hyperedge_degree_distribution
            .iter()
            .map(|(&k, &v)| (k, v as f32 / hyperedge_sizes.len() as f32))
            .collect();

        features.push(degree_dist.len() as f32);

        features
    }

    /// Extract hardware-specific features.
    fn extract_hardware_features(pih: &ProgramInteractionHypergraph) -> Vec<f32> {
        let mut features = Vec::new();

        // Operation count
        let operation_count = pih.edges.len();
        features.push(operation_count as f32);

        // Pipeline depth estimation
        let pipeline_depth = if operation_count > 10 {
            4.0
        } else if operation_count > 5 {
            3.0
        } else {
            2.0
        };
        features.push(pipeline_depth);

        // Complexity measure
        let complexity = pih.edges.len() as f32;
        features.push(complexity);

        // Loop count
        let loop_count = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"for".to_string())).count();
        features.push(loop_count as f32);

        // Compute vs memory operations
        let compute_ops = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"mul".to_string()) || e.opcode.as_ref() == Some(&"add".to_string())).count();
        let memory_ops = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"load".to_string()) || e.opcode.as_ref() == Some(&"store".to_string())).count();

        features.push(compute_ops as f32);
        features.push(memory_ops as f32);

        // CGRA operations
        let cgra_compute_events = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"cgra_compute".to_string())).count();
        features.push(cgra_compute_events as f32);

        // Power-aware operations
        let power_aware_events = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"power_aware_compute".to_string())).count();
        features.push(power_aware_events as f32);

        features
    }

    /// Generate optimization labels.
    fn generate_optimization_labels(pih: &ProgramInteractionHypergraph) -> OptimizationLabels {
        let features = Self::extract_features(pih);

        let loop_count = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"for".to_string())).count();
        let compute_ops = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"mul".to_string()) || e.opcode.as_ref() == Some(&"add".to_string())).count();

        // Hardware-specific optimization predictions
        let mut applied_rules = Vec::new();

        // Advanced Loop Transformations
        if loop_count > 2 {
            applied_rules.push("LoopTiling".to_string());
            applied_rules.push("LoopUnrolling".to_string());
        }

        // SIMD Vectorization
        if compute_ops > 5 {
            applied_rules.push("SIMDVectorization".to_string());
        }

        // CGRA Mapping
        let cgra_compute_events = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"cgra_compute".to_string())).count();
        if cgra_compute_events > 0 {
            applied_rules.push("CgraSpatialMapping".to_string());
            applied_rules.push("CgraPipelining".to_string());
        }

        // Power Optimization
        if features.hardware_features.iter().sum::<f32>() > 50.0 {
            applied_rules.push("PowerOptimization".to_string());
            applied_rules.push("ThermalOptimization".to_string());
        }

        // Check for power-aware compute events
        let power_aware_events = pih.edges.iter().filter(|e| e.opcode.as_ref() == Some(&"power_aware_compute".to_string())).count();
        if power_aware_events > 0 {
            applied_rules.push("PowerOptimization".to_string());
            applied_rules.push("ThermalOptimization".to_string());
        }

        OptimizationLabels {
            applied_rules,
            performance_improvement: 0.3,
            power_consumption: 25.0,
            hardware_constraints: vec!["latency".to_string(), "throughput".to_string()],
            optimization_impact: vec![0.1, 0.2, 0.15],
        }
    }
}

/// A simple bipartite GNN implementation for PIH optimization.
pub struct BipartiteGnn {
    node_embedding_dim: usize,
    edge_embedding_dim: usize,
    hidden_dim: usize,
    num_layers: usize,
    dropout: f32,
}

impl BipartiteGnn {
    /// Create a new bipartite GNN.
    pub fn new(node_embedding_dim: usize, edge_embedding_dim: usize, hidden_dim: usize, num_layers: usize, dropout: f32) -> Self {
        Self {
            node_embedding_dim,
            edge_embedding_dim,
            hidden_dim,
            num_layers,
            dropout,
        }
    }

    /// Forward pass through the bipartite GNN.
    pub fn forward(&self, features: &GnnFeatures) -> (HashMap<String, Vec<f32>>, Vec<Vec<f32>>) {
        // Simplified forward pass
        let node_embeddings = features.node_features.clone();
        let edge_embeddings = features.edge_features.clone();

        (node_embeddings, edge_embeddings)
    }

    /// Predict optimization labels.
    pub fn predict_optimizations(&self, features: &GnnFeatures) -> OptimizationLabels {
        // Simplified prediction
        features.optimization_labels.clone()
    }
}

impl Default for BipartiteGnn {
    fn default() -> Self {
        Self::new(64, 32, 128, 3, 0.1)
    }
}

/// Training sample for the GNN model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub sample_id: String,
    pub features: GnnFeatures,
    pub labels: OptimizationLabels,
    pub hardware_features: crate::hardware::HardwareFeatures,
    pub sample_weight: f32,
}
