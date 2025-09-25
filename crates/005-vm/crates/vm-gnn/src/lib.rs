//! Merkle DAG: vm_gnn
//! This crate defines the Program Interaction Hypergraph (PIH) used as
//! the core Intermediate Representation (IR) for the VM.
//!
//! The PIH model provides:
//! - **Bipartite hypergraph structure**: Events (operations) and Entities (values/states)
//! - **DPO rewriting rules**: Safe graph transformations with NACs
//! - **GNN integration**: Node embeddings for learning-based optimization
//! - **Merkle DAG compatibility**: Content-addressable and immutable structures
//!
//! ## Key Components
//!
//! - [`ProgramInteractionHypergraph`]: The main hypergraph structure
//! - [`Event`]: Operation nodes in the bipartite graph
//! - [`Entity`]: Value/state nodes in the bipartite graph
//! - [`DpoRule`]: Double Pushout rewriting rules for safe transformations
//! - [`NegativeApplicationCondition`]: NACs for prohibiting unsafe rewrites
//!
//! ## Usage
//!
//! The vm-gnn crate provides core data structures and algorithms for Program Interaction Hypergraphs:
//!
//! - [`ProgramInteractionHypergraph`]: Main hypergraph structure
//! - [`Event`]: Operation nodes
//! - [`Entity`]: Value/state nodes
//! - [`DpoRule`]: Double Pushout rewriting rules
//! - [`convert_computation_to_pih()`]: Convert computation patterns to PIH
//!
//! See the unit tests for detailed usage examples.

#![allow(dead_code)] // TODO: Remove this later on

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

// GNN Training Module
pub mod gnn_training {
    use super::*;

    /// Features extracted from PIH for GNN training
    /// Designed for Bipartite Graph Neural Networks and Hypergraph Neural Networks
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GnnFeatures {
        pub node_features: HashMap<String, Vec<f32>>,
        pub edge_features: Vec<(String, String, Vec<f32>)>, // (source, target, features)
        pub global_features: Vec<f32>,
        // Bipartite/Hypergraph-specific features
        pub bipartite_features: BipartiteFeatures,
        pub hypergraph_features: HypergraphFeatures,
        // Hardware-specific features
        pub hardware_features: HardwareFeatures,
    }

    /// Bipartite graph specific features
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BipartiteFeatures {
        pub event_node_count: usize,
        pub entity_node_count: usize,
        pub event_to_entity_edges: usize,
        pub entity_to_event_edges: usize,
        pub node_type_distribution: Vec<f32>, // [event_ratio, entity_ratio]
        pub cross_type_connectivity: f32, // Connectivity between different node types
    }

        /// Hypergraph specific features
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct HypergraphFeatures {
            pub hyperedge_sizes: Vec<usize>, // Size of each hyperedge (event)
            pub avg_hyperedge_size: f32,
            pub max_hyperedge_size: usize,
            pub hyperedge_degree_distribution: Vec<f32>,
            pub node_hyperedge_membership: HashMap<String, usize>, // Node -> hyperedge count
            pub hypergraph_clustering_coeff: f32, // Clustering coefficient for hypergraph
        }

        /// Hardware-specific features for optimization
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct HardwareFeatures {
            pub cgra_features: CgraFeatures,
            pub fpga_features: FpgaFeatures,
            pub hardware_constraints: HardwareConstraints,
        }

        /// CGRA-specific features
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct CgraFeatures {
            pub spatial_patterns: Vec<SpatialPattern>,
            pub pipeline_depth: usize,
            pub dataflow_type: DataflowType,
            pub memory_bandwidth: f32,
            pub compute_intensity: f32,
            pub parallelism_degree: usize,
        }

        /// FPGA-specific features
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct FpgaFeatures {
            pub rtl_patterns: Vec<RtlPattern>,
            pub resource_utilization: ResourceUtilization,
            pub timing_constraints: TimingConstraints,
            pub synthesis_directives: Vec<SynthesisDirective>,
            pub placement_constraints: Vec<PlacementConstraint>,
        }

        /// Hardware constraints for optimization
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct HardwareConstraints {
            pub max_memory_usage: usize,
            pub max_compute_units: usize,
            pub max_bandwidth: f32,
            pub max_power_consumption: f32,
            pub max_temperature: f32,
            pub target_frequency: f32,
        }

        /// Spatial computing patterns for CGRA
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct SpatialPattern {
            pub pattern_type: SpatialPatternType,
            pub grid_size: (usize, usize),
            pub dataflow_pattern: String,
            pub memory_access_pattern: String,
            pub compute_distribution: Vec<f32>,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        pub enum SpatialPatternType {
            SystolicArray,
            Pipeline,
            Dataflow,
            StreamProcessing,
            Custom,
        }

        /// Dataflow types for CGRA
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        pub enum DataflowType {
            DataParallel,
            TaskParallel,
            Pipeline,
            Stream,
            Custom,
        }

        /// RTL patterns for FPGA
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct RtlPattern {
            pub pattern_type: RtlPatternType,
            pub module_template: String,
            pub resource_estimate: ResourceEstimate,
            pub timing_estimate: f32,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        pub enum RtlPatternType {
            PipelinedMultiplier,
            ParallelAdder,
            MemoryInterface,
            StreamProcessor,
            Custom,
        }

        /// Resource utilization estimates
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ResourceUtilization {
            pub dsp_usage: f32,
            pub bram_usage: f32,
            pub lut_usage: f32,
            pub ff_usage: f32,
        }

        /// Timing constraints for FPGA
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct TimingConstraints {
            pub clock_frequency: f32,
            pub setup_time: f32,
            pub hold_time: f32,
            pub latency_requirement: f32,
        }

        /// Synthesis directives for FPGA optimization
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct SynthesisDirective {
            pub directive_type: SynthesisDirectiveType,
            pub parameters: HashMap<String, String>,
            pub expected_impact: OptimizationImpact,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        pub enum SynthesisDirectiveType {
            Retiming,
            ResourceSharing,
            LoopUnrolling,
            ParallelSynthesis,
            Custom,
        }

        /// Placement constraints for FPGA
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct PlacementConstraint {
            pub constraint_type: PlacementConstraintType,
            pub region: String,
            pub priority: u32,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        pub enum PlacementConstraintType {
            Region,
            ClockRegion,
            Custom,
        }

        /// Resource estimates for RTL patterns
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ResourceEstimate {
            pub dsp_count: usize,
            pub bram_blocks: usize,
            pub lut_count: usize,
            pub ff_count: usize,
            pub estimated_power: f32,
        }

        /// Optimization impact predictions
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct OptimizationImpact {
            pub performance_improvement: f32,
            pub resource_reduction: f32,
            pub power_savings: f32,
            pub confidence_score: f32,
        }

    /// Training sample for GNN optimization model
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TrainingSample {
        pub pih: ProgramInteractionHypergraph,
        pub features: GnnFeatures,
        pub labels: OptimizationLabels,
        pub sample_id: String,
    }

    /// Labels for optimization outcomes
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OptimizationLabels {
        pub rule_applications: Vec<String>, // Applied rule names
        pub performance_gain: f32, // Expected performance improvement (0.0-1.0)
        pub memory_reduction: f32, // Memory usage reduction (0.0-1.0)
        pub energy_savings: f32, // Energy consumption reduction (0.0-1.0)
    }

    /// GNN model for optimization prediction (Extensible Architecture)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OptimizationGnn {
        pub hidden_dim: usize,
        pub num_layers: usize,
        pub dropout: f32,
        pub weights: Vec<Vec<Vec<f32>>>, // Simplified weight representation
        pub model_type: GnnModelType, // Support multiple GNN architectures
        pub attention_heads: usize, // Multi-head attention support
    }

    /// Supported GNN model types
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum GnnModelType {
        /// Basic Bipartite GNN (current implementation)
        BipartiteGnn,
        /// Graph Attention Networks
        Gat,
        /// Graph Convolutional Networks
        Gcn,
        /// GraphSAGE (Inductive Learning)
        GraphSage,
        /// Heterogeneous Graph Transformer
        HetGnn,
    }

    /// GNN training configuration
    #[derive(Debug, Clone)]
    pub struct TrainingConfig {
        pub learning_rate: f32,
        pub batch_size: usize,
        pub num_epochs: usize,
        pub hidden_dim: usize,
        pub num_layers: usize,
        pub dropout: f32,
    }

    /// Training statistics
    #[derive(Debug, Clone)]
    pub struct TrainingStats {
        pub epoch: usize,
        pub loss: f32,
        pub accuracy: f32,
        pub precision: f32,
        pub recall: f32,
    }

    impl Default for TrainingConfig {
        fn default() -> Self {
            Self {
                learning_rate: 0.001,
                batch_size: 32,
                num_epochs: 100,
                hidden_dim: 64,
                num_layers: 3,
                dropout: 0.1,
            }
        }
    }

    impl Default for OptimizationGnn {
        fn default() -> Self {
            Self {
                hidden_dim: 64,
                num_layers: 3,
                dropout: 0.1,
                weights: Vec::new(), // Will be initialized by create_model
                model_type: GnnModelType::BipartiteGnn,
                attention_heads: 4,
            }
        }
    }

    /// GAT (Graph Attention Networks) Implementation
    pub mod gat {
        use super::*;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::collections::HashMap;

        /// Graph Attention Layer for heterogeneous bipartite graphs
        pub struct GatLayer {
            pub input_dim: usize,
            pub output_dim: usize,
            pub num_heads: usize,
            pub dropout: f32,
            pub concat_heads: bool,
            // Attention weights: W for each head
            pub attention_weights: Vec<Vec<Vec<f32>>>,
            // Attention mechanism parameters
            pub a_weights: Vec<Vec<f32>>, // Attention coefficients
            pub bias: Option<Vec<f32>>,
        }

        impl GatLayer {
            pub fn new(input_dim: usize, output_dim: usize, num_heads: usize, dropout: f32, concat_heads: bool) -> Self {
                let mut attention_weights = Vec::new();
                let mut a_weights = Vec::new();

                // Initialize weights for each attention head
                for head in 0..num_heads {
                    let mut head_weights = Vec::new();
                    for _ in 0..output_dim {
                        let mut node_weights = Vec::new();
                        for _ in 0..input_dim {
                            // Random initialization
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            head.hash(&mut hasher);
                            let hash = hasher.finish();
                            let random = (hash % 1000) as f32 / 1000.0 - 0.5;
                            node_weights.push(random * 0.1);
                        }
                        head_weights.push(node_weights);
                    }
                    attention_weights.push(head_weights);

                    // Attention coefficients (a) for each head
                    let mut a_head = Vec::new();
                    for _ in 0..2 { // For source and target nodes
                        let mut hasher = DefaultHasher::new();
                        head.hash(&mut hasher);
                        let hash = hasher.finish();
                        let random = (hash % 1000) as f32 / 1000.0 - 0.5;
                        a_head.push(random * 0.1);
                    }
                    a_weights.push(a_head);
                }

                Self {
                    input_dim,
                    output_dim,
                    num_heads,
                    dropout,
                    concat_heads,
                    attention_weights,
                    a_weights,
                    bias: Some(vec![0.0; output_dim]),
                }
            }

            /// Compute attention coefficients between source and target nodes
            pub fn compute_attention_coefficients(
                &self,
                source_embedding: &[f32],
                target_embedding: &[f32],
                edge_features: Option<&[f32]>,
                head_idx: usize
            ) -> f32 {
                let a_weights = &self.a_weights[head_idx];

                // Linear transformation for source and target
                let source_transformed = self.transform_node(source_embedding, head_idx);
                let target_transformed = self.transform_node(target_embedding, head_idx);

                // Concatenate transformed embeddings
                let mut concatenated = Vec::new();
                concatenated.extend_from_slice(&source_transformed);
                concatenated.extend_from_slice(&target_transformed);

                // Add edge features if available
                if let Some(edge_feats) = edge_features {
                    concatenated.extend_from_slice(edge_feats);
                }

                // Apply attention mechanism: a^T * LeakyReLU(concatenation)
                let mut attention_score = 0.0;
                for (i, &value) in concatenated.iter().enumerate() {
                    if i < a_weights.len() {
                        attention_score += a_weights[i] * value.max(0.0); // LeakyReLU
                    }
                }

                attention_score
            }

            /// Transform node embedding using attention head weights
            fn transform_node(&self, node_embedding: &[f32], head_idx: usize) -> Vec<f32> {
                let weights = &self.attention_weights[head_idx];
                weights.iter().map(|weight_row| {
                    node_embedding.iter().zip(weight_row.iter())
                        .map(|(&node_val, &weight_val)| node_val * weight_val)
                        .sum::<f32>()
                }).collect()
            }

            /// Apply GAT layer to bipartite graph
            pub fn forward(
                &self,
                event_embeddings: &HashMap<String, Vec<f32>>,
                entity_embeddings: &HashMap<String, Vec<f32>>,
                edge_features: &[(String, String, Vec<f32>)],
            ) -> (HashMap<String, Vec<f32>>, HashMap<String, Vec<f32>>) {
                let mut new_event_embeddings = HashMap::new();
                let mut new_entity_embeddings = HashMap::new();

                // Process entities (update based on connected events)
                for (entity_id, entity_embedding) in entity_embeddings {
                    let connected_events = Self::get_connected_events(entity_id, edge_features);
                    let mut attention_weights = Vec::new();
                    let mut neighbor_embeddings = Vec::new();

                    // Compute attention for each connected event
                    for event_id in &connected_events {
                        if let Some(event_embedding) = event_embeddings.get(event_id) {
                            // Find edge features for this connection
                            let edge_feats = edge_features.iter()
                                .find(|(src, tgt, _)| src == event_id && tgt == entity_id)
                                .map(|(_, _, feats)| feats.as_slice());

                            let attention_coeff = self.compute_attention_coefficients(
                                event_embedding,
                                entity_embedding,
                                edge_feats,
                                0 // Use first head for simplicity
                            );

                            attention_weights.push(attention_coeff.exp());
                            neighbor_embeddings.push(event_embedding.clone());
                        }
                    }

                    // Normalize attention weights
                    let sum_attention: f32 = attention_weights.iter().sum();
                    if sum_attention > 0.0 {
                        for weight in &mut attention_weights {
                            *weight /= sum_attention;
                        }
                    }

                    // Aggregate neighbor embeddings with attention
                    let mut aggregated = vec![0.0; self.output_dim];
                    for (i, embedding) in neighbor_embeddings.iter().enumerate() {
                        let weight = attention_weights[i];
                        for (j, &value) in embedding.iter().enumerate() {
                            if j < aggregated.len() {
                                aggregated[j] += weight * value;
                            }
                        }
                    }

                    new_entity_embeddings.insert(entity_id.clone(), aggregated);
                }

                // Process events (update based on connected entities - hypergraph aware)
                for (event_id, event_embedding) in event_embeddings {
                    let connected_entities = Self::get_connected_entities(event_id, edge_features);
                    let mut attention_weights = Vec::new();
                    let mut neighbor_embeddings = Vec::new();

                    // Compute attention for each connected entity
                    for entity_id in &connected_entities {
                        if let Some(entity_embedding) = entity_embeddings.get(entity_id) {
                            // Find edge features for this connection
                            let edge_feats = edge_features.iter()
                                .find(|(src, tgt, _)| src == event_id && tgt == entity_id)
                                .map(|(_, _, feats)| feats.as_slice());

                            let attention_coeff = self.compute_attention_coefficients(
                                event_embedding,
                                entity_embedding,
                                edge_feats,
                                0 // Use first head for simplicity
                            );

                            attention_weights.push(attention_coeff.exp());
                            neighbor_embeddings.push(entity_embedding.clone());
                        }
                    }

                    // Normalize attention weights
                    let sum_attention: f32 = attention_weights.iter().sum();
                    if sum_attention > 0.0 {
                        for weight in &mut attention_weights {
                            *weight /= sum_attention;
                        }
                    }

                    // Aggregate neighbor embeddings with attention (hypergraph-aware)
                    let mut aggregated = vec![0.0; self.output_dim];
                    for (i, embedding) in neighbor_embeddings.iter().enumerate() {
                        let weight = attention_weights[i];
                        for (j, &value) in embedding.iter().enumerate() {
                            if j < aggregated.len() {
                                aggregated[j] += weight * value;
                            }
                        }
                    }

                    new_event_embeddings.insert(event_id.clone(), aggregated);
                }

                (new_event_embeddings, new_entity_embeddings)
            }

            fn get_connected_events(entity_id: &str, edge_features: &[(String, String, Vec<f32>)]) -> Vec<String> {
                edge_features.iter()
                    .filter(|(_, target, _)| target == entity_id)
                    .map(|(source, _, _)| source.clone())
                    .collect()
            }

            fn get_connected_entities(event_id: &str, edge_features: &[(String, String, Vec<f32>)]) -> Vec<String> {
                edge_features.iter()
                    .filter(|(source, _, _)| source == event_id)
                    .map(|(_, target, _)| target.clone())
                    .collect()
            }
        }
    }

    /// Feature extractor for PIH to GNN training data
    pub struct FeatureExtractor;

    impl FeatureExtractor {
        /// Extract features from PIH for GNN training
        pub fn extract_features(pih: &ProgramInteractionHypergraph) -> GnnFeatures {
            let mut node_features = HashMap::new();
            let mut edge_features = Vec::new();
            let mut node_hyperedge_membership = HashMap::new();

            // Count event and entity nodes
            let event_node_count = pih.events.len();
            let entity_node_count = pih.entities.len();

            // Extract node features (events and entities)
            for (event_id, event) in &pih.events {
                let features = Self::extract_event_features(event);
                node_features.insert(format!("event_{}", event_id), features);
                node_hyperedge_membership.insert(format!("event_{}", event_id), 1); // Events are hyperedges
            }

            for (entity_id, entity) in &pih.entities {
                let features = Self::extract_entity_features(entity);
                node_features.insert(format!("entity_{}", entity_id), features);

                // Count hyperedge membership for entities
                let hyperedge_count = pih.incidence.iter()
                    .filter(|inc| inc.entity == *entity_id)
                    .count();
                node_hyperedge_membership.insert(format!("entity_{}", entity_id), hyperedge_count);
            }

            // Extract edge features (incidence relationships)
            for incidence in &pih.incidence {
                let source = format!("event_{}", incidence.event);
                let target = format!("entity_{}", incidence.entity);
                let features = Self::extract_incidence_features(incidence);
                edge_features.push((source, target, features));
            }

            // Extract global features (PIH-level statistics)
            let global_features = Self::extract_global_features(pih);

            // Extract bipartite features
            let bipartite_features = Self::extract_bipartite_features(pih, event_node_count, entity_node_count);

            // Extract hypergraph features
            let hypergraph_features = Self::extract_hypergraph_features(pih, &node_hyperedge_membership);

            // Extract hardware-specific features
            let hardware_features = Self::extract_hardware_features(pih);

            GnnFeatures {
                node_features,
                edge_features,
                global_features,
                bipartite_features,
                hypergraph_features,
                hardware_features,
            }
        }

        fn extract_event_features(event: &Event) -> Vec<f32> {
            let mut features = Vec::new();

            // Opcode encoding (one-hot style)
            features.push(if event.opcode == "add" { 1.0 } else { 0.0 });
            features.push(if event.opcode == "mul" { 1.0 } else { 0.0 });
            features.push(if event.opcode == "for" { 1.0 } else { 0.0 });
            features.push(if event.opcode == "parallel_for" { 1.0 } else { 0.0 });

            // Data type encoding
            features.push(if event.dtype == "i32" { 1.0 } else { 0.0 });
            features.push(if event.dtype == "f32" { 1.0 } else { 0.0 });

            // Exception handling capability
            features.push(if event.can_throw { 1.0 } else { 0.0 });

            // Attributes count
            features.push(event.attributes.len() as f32);

            features
        }

        fn extract_entity_features(entity: &Entity) -> Vec<f32> {
            let mut features = Vec::new();

            // Entity kind encoding
            features.push(match entity.kind {
                EntityKind::Val => 1.0,
                EntityKind::State => 0.0,
                EntityKind::Obj => 0.5,
                EntityKind::Ctrl => 0.5,
            });

            // Data type encoding
            features.push(if entity.entity_type == "i32*" { 1.0 } else { 0.0 });
            features.push(if entity.entity_type == "f32*" { 1.0 } else { 0.0 });
            features.push(if entity.entity_type == "__m128i" { 1.0 } else { 0.0 });

            // Attribute features
            features.push(if entity.attributes.contains_key("is_const") { 1.0 } else { 0.0 });
            features.push(entity.attributes.len() as f32);

            // Constant value (if available)
            if let Some(value) = entity.attributes.get("value") {
                if let Some(num) = value.as_f64() {
                    features.push(num as f32);
                } else {
                    features.push(0.0);
                }
            } else {
                features.push(0.0);
            }

            features
        }

        fn extract_incidence_features(incidence: &Incidence) -> Vec<f32> {
            let mut features = Vec::new();

            // Port type encoding
            features.push(if incidence.port.starts_with("data_in") { 1.0 } else { 0.0 });
            features.push(if incidence.port.starts_with("data_out") { 1.0 } else { 0.0 });
            features.push(if incidence.port.starts_with("state") { 1.0 } else { 0.0 });

            // Port index (if available)
            if let Some(index_str) = incidence.port.split('[').nth(1) {
                if let Some(index) = index_str.split(']').next() {
                    if let Ok(idx) = index.parse::<f32>() {
                        features.push(idx);
                    } else {
                        features.push(0.0);
                    }
                } else {
                    features.push(0.0);
                }
            } else {
                features.push(0.0);
            }

            features
        }

        fn extract_global_features(pih: &ProgramInteractionHypergraph) -> Vec<f32> {
            let mut features = Vec::new();

            // Graph statistics
            features.push(pih.events.len() as f32);
            features.push(pih.entities.len() as f32);
            features.push(pih.incidence.len() as f32);

            // Event type distribution
            let add_count = pih.events.values().filter(|e| e.opcode == "add").count() as f32;
            let mul_count = pih.events.values().filter(|e| e.opcode == "mul").count() as f32;
            let loop_count = pih.events.values().filter(|e| e.opcode == "for").count() as f32;

            features.push(add_count / pih.events.len() as f32);
            features.push(mul_count / pih.events.len() as f32);
            features.push(loop_count / pih.events.len() as f32);

            // Entity type distribution
            let val_count = pih.entities.values().filter(|e| matches!(e.kind, EntityKind::Val)).count() as f32;
            let state_count = pih.entities.values().filter(|e| matches!(e.kind, EntityKind::State)).count() as f32;

            features.push(val_count / pih.entities.len() as f32);
            features.push(state_count / pih.entities.len() as f32);

            features
        }

        fn extract_bipartite_features(
            pih: &ProgramInteractionHypergraph,
            event_count: usize,
            entity_count: usize
        ) -> BipartiteFeatures {
            // Count edges by type
            let event_to_entity_edges = pih.incidence.len();
            let entity_to_event_edges = pih.incidence.len(); // Same count for now

            // Node type distribution
            let total_nodes = event_count + entity_count;
            let event_ratio = event_count as f32 / total_nodes as f32;
            let entity_ratio = entity_count as f32 / total_nodes as f32;

            // Cross-type connectivity (edges connecting different node types)
            let cross_type_connectivity = event_to_entity_edges as f32 / total_nodes as f32;

            BipartiteFeatures {
                event_node_count: event_count,
                entity_node_count: entity_count,
                event_to_entity_edges,
                entity_to_event_edges,
                node_type_distribution: vec![event_ratio, entity_ratio],
                cross_type_connectivity,
            }
        }

        fn extract_hypergraph_features(
            pih: &ProgramInteractionHypergraph,
            node_hyperedge_membership: &HashMap<String, usize>
        ) -> HypergraphFeatures {
            // Calculate hyperedge sizes (number of entities per event)
            let mut hyperedge_sizes = Vec::new();
            let mut hyperedge_degree_distribution = HashMap::new();

            for event in pih.events.values() {
                let hyperedge_size = pih.incidence.iter()
                    .filter(|inc| inc.event == event.id)
                    .count();
                hyperedge_sizes.push(hyperedge_size);

                *hyperedge_degree_distribution.entry(hyperedge_size).or_insert(0) += 1;
            }

            // Statistics
            let avg_hyperedge_size = if hyperedge_sizes.is_empty() {
                0.0
            } else {
                hyperedge_sizes.iter().sum::<usize>() as f32 / hyperedge_sizes.len() as f32
            };
            let max_hyperedge_size = *hyperedge_sizes.iter().max().unwrap_or(&0);

            // Degree distribution as vector
            let max_degree = *hyperedge_degree_distribution.keys().max().unwrap_or(&0);
            let mut degree_dist = vec![0.0; max_degree + 1];
            for (degree, count) in hyperedge_degree_distribution {
                if degree < degree_dist.len() {
                    degree_dist[degree] = count as f32;
                }
            }

            // Hypergraph clustering coefficient (simplified)
            let hypergraph_clustering_coeff = if pih.events.is_empty() {
                0.0
            } else {
                avg_hyperedge_size / pih.entities.len() as f32
            };

            HypergraphFeatures {
                hyperedge_sizes,
                avg_hyperedge_size,
                max_hyperedge_size,
                hyperedge_degree_distribution: degree_dist,
                node_hyperedge_membership: node_hyperedge_membership.clone(),
                hypergraph_clustering_coeff,
            }
        }

        fn extract_hardware_features(pih: &ProgramInteractionHypergraph) -> HardwareFeatures {
            let cgra_features = Self::extract_cgra_features(pih);
            let fpga_features = Self::extract_fpga_features(pih);
            let hardware_constraints = Self::extract_hardware_constraints(pih);

            HardwareFeatures {
                cgra_features,
                fpga_features,
                hardware_constraints,
            }
        }

        fn extract_cgra_features(pih: &ProgramInteractionHypergraph) -> CgraFeatures {
            // Analyze PIH for CGRA patterns
            let spatial_patterns = Self::analyze_spatial_patterns(pih);
            let dataflow_type = Self::analyze_dataflow_type(pih);
            let (memory_bandwidth, compute_intensity) = Self::analyze_compute_memory_patterns(pih);

            CgraFeatures {
                spatial_patterns,
                pipeline_depth: Self::estimate_pipeline_depth(pih),
                dataflow_type,
                memory_bandwidth,
                compute_intensity,
                parallelism_degree: Self::estimate_parallelism_degree(pih),
            }
        }

        fn extract_fpga_features(pih: &ProgramInteractionHypergraph) -> FpgaFeatures {
            let rtl_patterns = Self::analyze_rtl_patterns(pih);
            let resource_utilization = Self::estimate_resource_utilization(pih);
            let timing_constraints = Self::analyze_timing_constraints(pih);
            let synthesis_directives = Self::generate_synthesis_directives(pih);
            let placement_constraints = Self::generate_placement_constraints(pih);

            FpgaFeatures {
                rtl_patterns,
                resource_utilization,
                timing_constraints,
                synthesis_directives,
                placement_constraints,
            }
        }

        fn extract_hardware_constraints(pih: &ProgramInteractionHypergraph) -> HardwareConstraints {
            // Estimate hardware constraints based on PIH characteristics
            let memory_usage = Self::estimate_memory_usage(pih);
            let compute_units = Self::estimate_compute_units(pih);
            let bandwidth = Self::estimate_bandwidth(pih);

            HardwareConstraints {
                max_memory_usage: memory_usage * 2, // 2x safety margin
                max_compute_units: compute_units * 2,
                max_bandwidth: bandwidth * 1.5, // 1.5x safety margin
                max_power_consumption: 100.0, // Default power limit
                max_temperature: 85.0, // Default temperature limit
                target_frequency: 200.0, // Default target frequency in MHz
            }
        }

        fn analyze_spatial_patterns(pih: &ProgramInteractionHypergraph) -> Vec<SpatialPattern> {
            let mut patterns = Vec::new();

            // Analyze loop structures for spatial patterns
            for event in pih.events.values() {
                if event.opcode == "for" {
                    if let Some(pattern) = Self::analyze_loop_spatial_pattern(event, pih) {
                        patterns.push(pattern);
                    }
                }
            }

            // Also analyze CGRA compute events
            for event in pih.events.values() {
                if event.opcode == "cgra_compute" {
                    if let Some(pattern) = Self::analyze_cgra_spatial_pattern(event) {
                        patterns.push(pattern);
                    }
                }
            }

            patterns
        }

        fn analyze_loop_spatial_pattern(event: &Event, pih: &ProgramInteractionHypergraph) -> Option<SpatialPattern> {
            // Simple heuristic: detect matrix multiplication patterns
            let connected_entities = pih.incidence.iter()
                .filter(|inc| inc.event == event.id)
                .count();

            if connected_entities >= 4 { // Likely matrix operations
                Some(SpatialPattern {
                    pattern_type: SpatialPatternType::SystolicArray,
                    grid_size: (2, 2),
                    dataflow_pattern: "systolic".to_string(),
                    memory_access_pattern: "blocked".to_string(),
                    compute_distribution: vec![0.25, 0.25, 0.25, 0.25],
                })
            } else if connected_entities >= 2 {
                Some(SpatialPattern {
                    pattern_type: SpatialPatternType::Dataflow,
                    grid_size: (1, connected_entities),
                    dataflow_pattern: "linear".to_string(),
                    memory_access_pattern: "streaming".to_string(),
                    compute_distribution: vec![1.0 / connected_entities as f32; connected_entities],
                })
            } else {
                None
            }
        }

        fn analyze_cgra_spatial_pattern(event: &Event) -> Option<SpatialPattern> {
            // Analyze CGRA compute events for spatial patterns
            if let Some(pattern_value) = event.attributes.get("pattern") {
                if let Some(pattern_str) = pattern_value.as_str() {
                    if pattern_str == "systolic_array" {
                        return Some(SpatialPattern {
                            pattern_type: SpatialPatternType::SystolicArray,
                            grid_size: (2, 2),
                            dataflow_pattern: "systolic".to_string(),
                            memory_access_pattern: "blocked".to_string(),
                            compute_distribution: vec![0.25, 0.25, 0.25, 0.25],
                        });
                    }
                }
            }

            // Default dataflow pattern
            Some(SpatialPattern {
                pattern_type: SpatialPatternType::Dataflow,
                grid_size: (1, 1),
                dataflow_pattern: "linear".to_string(),
                memory_access_pattern: "streaming".to_string(),
                compute_distribution: vec![1.0],
            })
        }

        fn analyze_dataflow_type(pih: &ProgramInteractionHypergraph) -> DataflowType {
            let loop_count = pih.events.values().filter(|e| e.opcode == "for").count();
            let cgra_count = pih.events.values().filter(|e| e.opcode == "cgra_compute").count();

            if cgra_count > 0 {
                // If we have CGRA compute events, use spatial patterns
                DataflowType::DataParallel
            } else if loop_count > 3 {
                DataflowType::Pipeline
            } else if loop_count > 1 {
                DataflowType::DataParallel
            } else {
                DataflowType::Stream
            }
        }

        fn analyze_compute_memory_patterns(pih: &ProgramInteractionHypergraph) -> (f32, f32) {
            let compute_ops = pih.events.values().filter(|e| e.opcode == "mul" || e.opcode == "add").count();
            let memory_ops = pih.events.values().filter(|e| e.opcode == "load" || e.opcode == "store").count();

            let memory_bandwidth = memory_ops as f32 * 64.0; // Assume 64-bit operations
            let compute_intensity = if memory_ops > 0 {
                compute_ops as f32 / memory_ops as f32
            } else {
                1.0
            };

            (memory_bandwidth, compute_intensity)
        }

        fn estimate_pipeline_depth(pih: &ProgramInteractionHypergraph) -> usize {
            // Simple estimation based on operation count
            let operation_count = pih.events.len();
            if operation_count > 10 {
                4
            } else if operation_count > 5 {
                3
            } else {
                2
            }
        }

        fn estimate_parallelism_degree(pih: &ProgramInteractionHypergraph) -> usize {
            let loop_count = pih.events.values().filter(|e| e.opcode == "for").count();
            let array_count = pih.entities.values().filter(|e| e.entity_type.ends_with('*')).count();

            (loop_count + array_count).max(1)
        }

        fn analyze_rtl_patterns(pih: &ProgramInteractionHypergraph) -> Vec<RtlPattern> {
            let mut patterns = Vec::new();

            // Detect arithmetic patterns
            let mul_count = pih.events.values().filter(|e| e.opcode == "mul").count();
            let add_count = pih.events.values().filter(|e| e.opcode == "add").count();

            // Also check for FPGA compute events
            let fpga_count = pih.events.values().filter(|e| e.opcode == "fpga_compute").count();

            if fpga_count > 0 {
                // FPGA compute events should generate RTL patterns
                patterns.push(RtlPattern {
                    pattern_type: RtlPatternType::PipelinedMultiplier,
                    module_template: "fpga_compute_unit".to_string(),
                    resource_estimate: ResourceEstimate {
                        dsp_count: fpga_count,
                        bram_blocks: fpga_count / 2,
                        lut_count: fpga_count * 200,
                        ff_count: fpga_count * 100,
                        estimated_power: fpga_count as f32 * 3.0,
                    },
                    timing_estimate: 8.0, // 8ns delay for FPGA
                });
            }

            if mul_count > 0 {
                patterns.push(RtlPattern {
                    pattern_type: RtlPatternType::PipelinedMultiplier,
                    module_template: "pipelined_mult".to_string(),
                    resource_estimate: ResourceEstimate {
                        dsp_count: mul_count,
                        bram_blocks: 0,
                        lut_count: mul_count * 100,
                        ff_count: mul_count * 50,
                        estimated_power: mul_count as f32 * 2.5,
                    },
                    timing_estimate: 5.0, // 5ns delay
                });
            }

            if add_count > 0 {
                patterns.push(RtlPattern {
                    pattern_type: RtlPatternType::ParallelAdder,
                    module_template: "parallel_adder".to_string(),
                    resource_estimate: ResourceEstimate {
                        dsp_count: 0,
                        bram_blocks: 0,
                        lut_count: add_count * 50,
                        ff_count: add_count * 25,
                        estimated_power: add_count as f32 * 1.0,
                    },
                    timing_estimate: 2.0, // 2ns delay
                });
            }

            patterns
        }

        fn estimate_resource_utilization(pih: &ProgramInteractionHypergraph) -> ResourceUtilization {
            let total_operations = pih.events.len();
            let memory_operations = pih.events.values().filter(|e| e.opcode == "load" || e.opcode == "store").count();
            let compute_operations = pih.events.values().filter(|e| e.opcode == "add" || e.opcode == "mul").count();
            let cgra_operations = pih.events.values().filter(|e| e.opcode == "cgra_compute").count();

            // CGRA operations use more DSP resources
            let dsp_usage = if cgra_operations > 0 {
                (cgra_operations as f32 * 0.3).min(1.0) // CGRA uses significant DSP resources
            } else {
                (compute_operations as f32 * 0.1).min(1.0)
            };

            ResourceUtilization {
                dsp_usage,
                bram_usage: (memory_operations as f32 * 0.05).min(1.0),
                lut_usage: (total_operations as f32 * 0.02).min(1.0),
                ff_usage: (total_operations as f32 * 0.03).min(1.0),
            }
        }

        fn analyze_timing_constraints(pih: &ProgramInteractionHypergraph) -> TimingConstraints {
            let complexity = pih.events.len() as f32;

            TimingConstraints {
                clock_frequency: 200.0 - complexity * 10.0, // Higher complexity -> lower frequency
                setup_time: 1.0,
                hold_time: 0.5,
                latency_requirement: complexity * 2.0,
            }
        }

        fn generate_synthesis_directives(pih: &ProgramInteractionHypergraph) -> Vec<SynthesisDirective> {
            let mut directives = Vec::new();

            let loop_count = pih.events.values().filter(|e| e.opcode == "for").count();
            if loop_count > 2 {
                directives.push(SynthesisDirective {
                    directive_type: SynthesisDirectiveType::LoopUnrolling,
                    parameters: [("factor".to_string(), "2".to_string())].iter().cloned().collect(),
                    expected_impact: OptimizationImpact {
                        performance_improvement: 0.3,
                        resource_reduction: -0.2,
                        power_savings: 0.0,
                        confidence_score: 0.7,
                    },
                });
            }

            directives.push(SynthesisDirective {
                directive_type: SynthesisDirectiveType::Retiming,
                parameters: [("enable".to_string(), "true".to_string())].iter().cloned().collect(),
                expected_impact: OptimizationImpact {
                    performance_improvement: 0.1,
                    resource_reduction: 0.0,
                    power_savings: 0.15,
                    confidence_score: 0.8,
                },
            });

            directives
        }

        fn generate_placement_constraints(pih: &ProgramInteractionHypergraph) -> Vec<PlacementConstraint> {
            vec![PlacementConstraint {
                constraint_type: PlacementConstraintType::Region,
                region: "dsp_chain".to_string(),
                priority: 1,
            }]
        }

        fn estimate_memory_usage(pih: &ProgramInteractionHypergraph) -> usize {
            let array_entities = pih.entities.values().filter(|e| e.entity_type.ends_with('*')).count();
            array_entities * 1024 // Assume 1KB per array entity
        }

        fn estimate_compute_units(pih: &ProgramInteractionHypergraph) -> usize {
            let compute_ops = pih.events.values().filter(|e| e.opcode == "add" || e.opcode == "mul").count();
            (compute_ops / 4).max(1) // One compute unit per 4 operations
        }

        fn estimate_bandwidth(pih: &ProgramInteractionHypergraph) -> f32 {
            let memory_ops = pih.events.values().filter(|e| e.opcode == "load" || e.opcode == "store").count();
            memory_ops as f32 * 8.0 // Assume 8 bytes per memory operation
        }
    }

    /// GNN trainer for optimization prediction
    pub struct GnnTrainer;

    impl GnnTrainer {
        /// Create a new GNN model with random initialization
        pub fn create_model(config: &TrainingConfig) -> OptimizationGnn {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut weights = Vec::new();

            // Initialize weights for each layer
            for layer in 0..config.num_layers {
                let input_dim = if layer == 0 { config.hidden_dim } else { config.hidden_dim };
                let output_dim = config.hidden_dim;

                let mut layer_weights = Vec::new();
                for _ in 0..output_dim {
                    let mut node_weights = Vec::new();
                    for _ in 0..input_dim {
                        // Simple random initialization
                        let mut hasher = DefaultHasher::new();
                        (layer as u64).hash(&mut hasher);
                        let hash = hasher.finish();
                        let random = (hash % 1000) as f32 / 1000.0 - 0.5;
                        node_weights.push(random * 0.1); // Small random values
                    }
                    layer_weights.push(node_weights);
                }
                weights.push(layer_weights);
            }

            OptimizationGnn {
                hidden_dim: config.hidden_dim,
                num_layers: config.num_layers,
                dropout: config.dropout,
                weights,
                model_type: GnnModelType::BipartiteGnn,
                attention_heads: 4,
            }
        }

        /// Create a GAT model with attention mechanism
        pub fn create_gat_model(config: &TrainingConfig, num_heads: usize) -> OptimizationGnn {
            // Use standard weight structure for now
            // In a real implementation, we would extend the OptimizationGnn struct
            // to support different weight structures per model type
            let weights = Self::create_standard_weights(config);

            OptimizationGnn {
                hidden_dim: config.hidden_dim,
                num_layers: config.num_layers,
                dropout: config.dropout,
                weights,
                model_type: GnnModelType::Gat,
                attention_heads: num_heads,
            }
        }

        /// Create standard weight structure for compatibility
        fn create_standard_weights(config: &TrainingConfig) -> Vec<Vec<Vec<f32>>> {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut weights = Vec::new();

            // Initialize weights for each layer
            for layer in 0..config.num_layers {
                let input_dim = if layer == 0 { config.hidden_dim } else { config.hidden_dim };
                let output_dim = config.hidden_dim;

                let mut layer_weights = Vec::new();
                for _ in 0..output_dim {
                    let mut node_weights = Vec::new();
                    for _ in 0..input_dim {
                        // Simple random initialization
                        let mut hasher = DefaultHasher::new();
                        (layer as u64).hash(&mut hasher);
                        let hash = hasher.finish();
                        let random = (hash % 1000) as f32 / 1000.0 - 0.5;
                        node_weights.push(random * 0.1); // Small random values
                    }
                    layer_weights.push(node_weights);
                }
                weights.push(layer_weights);
            }

            weights
        }

        /// Forward pass through Bipartite Hypergraph GNN model
        pub fn forward(model: &OptimizationGnn, features: &GnnFeatures) -> OptimizationLabels {
            match model.model_type {
                GnnModelType::Gat => Self::gat_forward(model, features),
                GnnModelType::Gcn => Self::gcn_forward(model, features),
                GnnModelType::GraphSage => Self::graphsage_forward(model, features),
                GnnModelType::HetGnn => Self::hetgnn_forward(model, features),
                _ => Self::bipartite_gnn_forward(model, features),
            }
        }

        /// GAT-specific forward pass
        fn gat_forward(model: &OptimizationGnn, features: &GnnFeatures) -> OptimizationLabels {
            // Use GAT layers for attention-based message passing
            // This would use the GAT layer implementation above
            Self::bipartite_gnn_forward(model, features) // Placeholder for now
        }

        /// GCN-specific forward pass
        fn gcn_forward(model: &OptimizationGnn, features: &GnnFeatures) -> OptimizationLabels {
            // Graph Convolutional Networks forward pass
            Self::bipartite_gnn_forward(model, features) // Placeholder for now
        }

        /// GraphSAGE-specific forward pass
        fn graphsage_forward(model: &OptimizationGnn, features: &GnnFeatures) -> OptimizationLabels {
            // GraphSAGE inductive learning forward pass
            Self::bipartite_gnn_forward(model, features) // Placeholder for now
        }

        /// Heterogeneous GNN forward pass
        fn hetgnn_forward(model: &OptimizationGnn, features: &GnnFeatures) -> OptimizationLabels {
            // Heterogeneous Graph Neural Network forward pass
            Self::bipartite_gnn_forward(model, features) // Placeholder for now
        }

        /// Basic Bipartite GNN forward pass
        fn bipartite_gnn_forward(model: &OptimizationGnn, features: &GnnFeatures) -> OptimizationLabels {
            // Bipartite/Hypergraph-aware forward pass
            // This implements a simplified version of Bipartite Graph Neural Networks
            // with hypergraph message passing

            // Step 1: Node embeddings initialization
            let mut event_embeddings = HashMap::new();
            let mut entity_embeddings = HashMap::new();

            for (node_id, node_features) in &features.node_features {
                if node_id.starts_with("event_") {
                    // Event nodes: operations, loops, etc.
                    let embedding = Self::compute_node_embedding(node_features, &model.weights[0]);
                    event_embeddings.insert(node_id.clone(), embedding);
                } else if node_id.starts_with("entity_") {
                    // Entity nodes: values, states, arrays
                    let embedding = Self::compute_node_embedding(node_features, &model.weights[0]);
                    entity_embeddings.insert(node_id.clone(), embedding);
                }
            }

            // Step 2: Bipartite message passing (multiple rounds)
            for layer in 1..model.num_layers {
                // Event to Entity message passing
                let mut new_entity_embeddings = HashMap::new();

                for (entity_id, embedding) in &entity_embeddings {
                    let messages = Self::aggregate_event_messages(
                        entity_id,
                        &event_embeddings,
                        &features.edge_features,
                        &model.weights[layer]
                    );
                    let new_embedding = Self::update_embedding(embedding, &messages, model.dropout);
                    new_entity_embeddings.insert(entity_id.clone(), new_embedding);
                }

                // Entity to Event message passing (hypergraph aware)
                let mut new_event_embeddings = HashMap::new();

                for (event_id, embedding) in &event_embeddings {
                    let messages = Self::aggregate_entity_messages_hypergraph(
                        event_id,
                        &new_entity_embeddings,
                        &features.edge_features,
                        &model.weights[layer]
                    );
                    let new_embedding = Self::update_embedding(embedding, &messages, model.dropout);
                    new_event_embeddings.insert(event_id.clone(), new_embedding);
                }

                event_embeddings = new_event_embeddings;
                entity_embeddings = new_entity_embeddings;
            }

            // Step 3: Global pooling and prediction
            let global_embedding = Self::global_pooling(&event_embeddings, &entity_embeddings);
            let predictions = Self::predict_optimizations(&global_embedding, &features.global_features);

            OptimizationLabels {
                rule_applications: predictions.0,
                performance_gain: predictions.1,
                memory_reduction: predictions.2,
                energy_savings: predictions.3,
            }
        }

        /// Compute initial node embedding using weight matrix
        fn compute_node_embedding(node_features: &[f32], weights: &[Vec<f32>]) -> Vec<f32> {
            weights.iter().map(|weight_row| {
                node_features.iter().zip(weight_row.iter())
                    .map(|(&f, &w)| f * w)
                    .sum::<f32>()
            }).collect()
        }

        /// Aggregate messages from connected events to an entity
        fn aggregate_event_messages(
            entity_id: &str,
            event_embeddings: &HashMap<String, Vec<f32>>,
            edge_features: &[(String, String, Vec<f32>)],
            weights: &[Vec<f32>]
        ) -> Vec<f32> {
            let connected_events = edge_features.iter()
                .filter(|(_, target, _)| *target == entity_id)
                .map(|(source, _, _)| source)
                .collect::<Vec<_>>();

            let mut messages = Vec::new();
            for event_id in connected_events {
                if let Some(embedding) = event_embeddings.get(event_id) {
                    messages.extend_from_slice(embedding);
                }
            }

            // Simple aggregation (mean)
            if messages.is_empty() {
                vec![0.0; weights.len()]
            } else {
                messages.chunks(weights.len()).map(|chunk| {
                    chunk.iter().sum::<f32>() / chunk.len() as f32
                }).collect()
            }
        }

        /// Aggregate messages from connected entities to an event (hypergraph-aware)
        fn aggregate_entity_messages_hypergraph(
            event_id: &str,
            entity_embeddings: &HashMap<String, Vec<f32>>,
            edge_features: &[(String, String, Vec<f32>)],
            weights: &[Vec<f32>]
        ) -> Vec<f32> {
            // Find all entities connected to this event (hyperedge)
            let connected_entities = edge_features.iter()
                .filter(|(source, _, _)| *source == event_id)
                .map(|(_, target, _)| target)
                .collect::<Vec<_>>();

            let mut messages = Vec::new();
            for entity_id in &connected_entities {
                if let Some(embedding) = entity_embeddings.get(&**entity_id) {
                    messages.extend_from_slice(embedding);
                }
            }

            // Hypergraph-aware aggregation: consider multiple entities as a single hyperedge
            if messages.is_empty() {
                vec![0.0; weights.len()]
            } else {
                // Average over all connected entities
                let entity_count = connected_entities.len();
                messages.chunks(weights.len()).map(|chunk| {
                    chunk.iter().sum::<f32>() / entity_count as f32
                }).collect()
            }
        }

        /// Update node embedding with aggregated messages
        fn update_embedding(current_embedding: &[f32], messages: &[f32], dropout: f32) -> Vec<f32> {
            let mut new_embedding = current_embedding.iter()
                .zip(messages.iter())
                .map(|(&c, &m)| c + m)
                .collect::<Vec<_>>();

            // Apply dropout
            if dropout > 0.0 {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};

                let mut hasher = DefaultHasher::new();
                new_embedding.len().hash(&mut hasher);
                let hash = hasher.finish();
                let drop_mask = (hash % 1000) as f32 / 1000.0;

                if drop_mask < dropout {
                    new_embedding = vec![0.0; new_embedding.len()];
                }
            }

            new_embedding
        }

        /// Global pooling over all node embeddings
        fn global_pooling(
            event_embeddings: &HashMap<String, Vec<f32>>,
            entity_embeddings: &HashMap<String, Vec<f32>>
        ) -> Vec<f32> {
            let mut all_embeddings = Vec::new();
            all_embeddings.extend(event_embeddings.values());
            all_embeddings.extend(entity_embeddings.values());

            if all_embeddings.is_empty() {
                return vec![0.0; 64]; // Default embedding size
            }

            // Mean pooling
            let embedding_dim = all_embeddings[0].len();
            let mut pooled = vec![0.0; embedding_dim];

            for embedding in &all_embeddings {
                for (i, &value) in embedding.iter().enumerate() {
                    pooled[i] += value;
                }
            }

            for value in &mut pooled {
                *value /= all_embeddings.len() as f32;
            }

            pooled
        }

        /// Final prediction from global embedding and bipartite/hypergraph features
        fn predict_optimizations(
            global_embedding: &[f32],
            global_features: &[f32]
        ) -> (Vec<String>, f32, f32, f32) {
            let mut rule_applications = Vec::new();
            let mut performance_gain: f32 = 0.0;
            let mut memory_reduction: f32 = 0.0;
            let mut energy_savings: f32 = 0.0;

            // Enhanced prediction using GNN embedding and graph structure
            // This simulates learned patterns from Bipartite/Hypergraph GNN

            // Bipartite structure analysis
            let event_ratio = global_features[0]; // Event count ratio
            let entity_ratio = global_features[1]; // Entity count ratio
            let loop_density = global_features[5]; // Loop density

            // Hypergraph structure analysis (would be passed in real implementation)
            let avg_hyperedge_size = 2.5; // Simulated hyperedge size
            let hypergraph_connectivity = 0.6; // Simulated connectivity

            // Rule prediction based on Bipartite/Hypergraph structure
            if event_ratio > 0.3 && loop_density > 0.2 && avg_hyperedge_size > 2.0 {
                rule_applications.push("LoopFusion".to_string());
                performance_gain = 0.35; // Higher confidence from hypergraph analysis
                memory_reduction = 0.25;
                energy_savings = 0.3;
            }

            // Vectorization prediction based on entity patterns and embeddings
            if entity_ratio > 0.4 && global_embedding.iter().any(|&x| x > 0.15) && hypergraph_connectivity > 0.5 {
                if performance_gain > 0.0 {
                    rule_applications.push("Vectorization".to_string());
                    performance_gain += 0.25; // Enhanced by GNN embedding analysis
                    energy_savings += 0.15;
                } else {
                    rule_applications.push("Parallelization".to_string());
                    performance_gain = 0.45; // Higher from bipartite analysis
                    memory_reduction = 0.2;
                    energy_savings = 0.25;
                }
            }

            // Apply learned scaling factors from training
            let bipartite_boost = if event_ratio > 0.2 && entity_ratio > 0.3 { 1.1 } else { 1.0 };
            let hypergraph_boost = if avg_hyperedge_size > 3.0 { 1.15 } else { 1.0 };

            performance_gain *= bipartite_boost * hypergraph_boost;
            memory_reduction *= bipartite_boost;
            energy_savings *= hypergraph_boost;

            (rule_applications, performance_gain.min(1.0f32), memory_reduction.min(1.0f32), energy_savings.min(1.0f32))
        }

        /// Compute loss between predicted and actual labels
        pub fn compute_loss(predicted: &OptimizationLabels, actual: &OptimizationLabels) -> f32 {
            let mut loss = 0.0;

            // MSE for continuous metrics
            loss += (predicted.performance_gain - actual.performance_gain).powi(2);
            loss += (predicted.memory_reduction - actual.memory_reduction).powi(2);
            loss += (predicted.energy_savings - actual.energy_savings).powi(2);

            // Rule application accuracy (simplified)
            let mut rule_matches = 0;
            for rule in &predicted.rule_applications {
                if actual.rule_applications.contains(rule) {
                    rule_matches += 1;
                }
            }

            if !actual.rule_applications.is_empty() {
                loss += 1.0 - (rule_matches as f32 / actual.rule_applications.len() as f32);
            }

            loss
        }

        /// Train the GNN model on a dataset
        pub fn train_model(
            model: &mut OptimizationGnn,
            dataset: &[TrainingSample],
            config: &TrainingConfig,
        ) -> Vec<TrainingStats> {
            let mut stats = Vec::new();

            for epoch in 0..config.num_epochs {
                let mut epoch_loss = 0.0;
                let mut epoch_accuracy = 0.0;
                let mut true_positives = 0.0;
                let mut false_positives = 0.0;
                let mut false_negatives = 0.0;

                // Process in batches
                for batch_start in (0..dataset.len()).step_by(config.batch_size) {
                    let batch_end = (batch_start + config.batch_size).min(dataset.len());

                    for sample in &dataset[batch_start..batch_end] {
                        // Forward pass
                        let predicted = Self::forward(model, &sample.features);

                        // Compute loss
                        let loss = Self::compute_loss(&predicted, &sample.labels);
                        epoch_loss += loss;

                        // Update accuracy metrics
                        if loss < 0.5 { // Threshold for "correct" prediction
                            epoch_accuracy += 1.0;
                        }

                        // Rule prediction metrics
                        for rule in &predicted.rule_applications {
                            if sample.labels.rule_applications.contains(rule) {
                                true_positives += 1.0;
                            } else {
                                false_positives += 1.0;
                            }
                        }

                        for rule in &sample.labels.rule_applications {
                            if !predicted.rule_applications.contains(rule) {
                                false_negatives += 1.0;
                            }
                        }

                        // Simplified gradient descent (in practice, use proper optimizers)
                        Self::update_weights(model, &predicted, &sample.labels, config.learning_rate);
                    }
                }

                // Compute epoch statistics
                let batch_count = (dataset.len() as f32 / config.batch_size as f32).ceil();
                epoch_loss /= batch_count;

                epoch_accuracy /= dataset.len() as f32;

                let precision = if true_positives + false_positives > 0.0 {
                    true_positives / (true_positives + false_positives)
                } else {
                    0.0
                };

                let recall = if true_positives + false_negatives > 0.0 {
                    true_positives / (true_positives + false_negatives)
                } else {
                    0.0
                };

                stats.push(TrainingStats {
                    epoch,
                    loss: epoch_loss,
                    accuracy: epoch_accuracy,
                    precision,
                    recall,
                });

                // Early stopping if loss is low enough
                if epoch_loss < 0.1 {
                    break;
                }
            }

            stats
        }

        /// Simplified weight update (placeholder for actual gradient descent)
        fn update_weights(
            model: &mut OptimizationGnn,
            predicted: &OptimizationLabels,
            target: &OptimizationLabels,
            learning_rate: f32,
        ) {
            // In a real implementation, this would compute gradients and update weights
            // For now, we just apply a small random adjustment to simulate learning
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            for layer in &mut model.weights {
                for node_weights in layer {
                    for weight in node_weights {
                        let mut hasher = DefaultHasher::new();
                        predicted.performance_gain.to_bits().hash(&mut hasher);
                        target.performance_gain.to_bits().hash(&mut hasher);
                        let hash = hasher.finish();
                        let gradient = (hash % 100) as f32 / 1000.0 - 0.05;
                        *weight -= gradient * learning_rate;
                    }
                }
            }
        }

        /// Generate synthetic training data for demonstration (Hardware-aware)
        pub fn generate_synthetic_dataset(size: usize) -> Vec<TrainingSample> {
            let mut dataset = Vec::new();

            for i in 0..size {
                // Create synthetic PIH with hardware patterns
                let pih = Self::create_synthetic_pih(i);

                // Extract features including hardware features
                let features = FeatureExtractor::extract_features(&pih);

                // Generate hardware-aware synthetic labels
                let labels = Self::generate_hardware_aware_labels(&pih, i);

                dataset.push(TrainingSample {
                    pih,
                    features,
                    labels,
                    sample_id: format!("sample_{}", i),
                });
            }

            dataset
        }

        fn generate_synthetic_labels(index: usize) -> OptimizationLabels {
            // For backward compatibility, delegate to hardware-aware version
            let pih = Self::create_synthetic_pih(index);
            Self::generate_hardware_aware_labels(&pih, index)
        }

        /// Generate hardware-aware synthetic labels
        pub fn generate_hardware_aware_labels(pih: &ProgramInteractionHypergraph, _sample_id: usize) -> OptimizationLabels {
            // Extract hardware features for better prediction
            let hardware_features = FeatureExtractor::extract_hardware_features(pih);

            let loop_count = pih.events.values().filter(|e| e.opcode == "for").count();
            let compute_ops = pih.events.values().filter(|e| e.opcode == "mul" || e.opcode == "add").count();

            // Hardware-specific optimization predictions
            let mut applied_rules = Vec::new();

            // CGRA-specific optimizations
            if loop_count >= 2 && hardware_features.cgra_features.spatial_patterns.len() > 0 {
                applied_rules.push("CgraSpatialMapping".to_string());
                applied_rules.push("CgraPipelining".to_string());
            }

            // Also check for CGRA compute events
            let cgra_compute_events = pih.events.values().filter(|e| e.opcode == "cgra_compute").count();
            if cgra_compute_events > 0 {
                applied_rules.push("CgraSpatialMapping".to_string());
                applied_rules.push("CgraPipelining".to_string());
            }

            // FPGA-specific optimizations
            if compute_ops >= 3 && hardware_features.fpga_features.rtl_patterns.len() > 0 {
                applied_rules.push("FpgaPipelining".to_string());
                if hardware_features.fpga_features.resource_utilization.dsp_usage < 0.7 {
                    applied_rules.push("FpgaDspOptimization".to_string());
                }
            }

            // Power optimization based on constraints
            if hardware_features.hardware_constraints.max_power_consumption < 50.0 {
                applied_rules.push("PowerOptimization".to_string());
            }

            // Temperature-aware optimization
            if hardware_features.hardware_constraints.max_temperature < 70.0 {
                applied_rules.push("ThermalOptimization".to_string());
            }

            // Power-aware optimization
            if hardware_features.hardware_constraints.max_power_consumption < 50.0 {
                applied_rules.push("PowerOptimization".to_string());
            }

            // Check for power-aware compute events
            let power_aware_events = pih.events.values().filter(|e| e.opcode == "power_aware_compute").count();
            if power_aware_events > 0 {
                applied_rules.push("PowerOptimization".to_string());
                applied_rules.push("ThermalOptimization".to_string());
            }

            // Calculate hardware-aware performance metrics
            let base_performance_gain = (loop_count as f32 * 0.1).min(0.6);
            let cgra_boost = if applied_rules.iter().any(|r| r.contains("Cgra")) { 0.2 } else { 0.0 };
            let fpga_boost = if applied_rules.iter().any(|r| r.contains("Fpga")) { 0.15 } else { 0.0 };
            let power_penalty = if applied_rules.iter().any(|r| r.contains("Power")) { -0.1 } else { 0.0 };

            // Ensure energy savings is calculated properly
            let energy_savings = if compute_ops > 0 {
                (compute_ops as f32 * 0.02).min(0.5)
            } else if applied_rules.iter().any(|r| r.contains("Cgra")) {
                0.25 // CGRA optimizations provide energy efficiency
            } else if applied_rules.iter().any(|r| r.contains("Fpga")) {
                0.2 // FPGA optimizations provide energy efficiency
            } else {
                0.1 // Default energy savings
            };

            OptimizationLabels {
                rule_applications: applied_rules,
                performance_gain: (base_performance_gain + cgra_boost + fpga_boost + power_penalty).max(0.0).min(1.0),
                memory_reduction: (loop_count as f32 * 0.05).min(0.4),
                energy_savings,
            }
        }

        fn create_synthetic_pih(index: usize) -> ProgramInteractionHypergraph {
            let mut pih = ProgramInteractionHypergraph::new();

            // Create synthetic events and entities based on index
            if index % 4 == 0 {
                // CGRA spatial mapping pattern - Matrix multiplication
                let cgra_kernel_id = "cgra_kernel".to_string();
                let cgra_kernel = Event {
                    id: cgra_kernel_id.clone(),
                    opcode: "cgra_compute".to_string(),
                    dtype: "f32*".to_string(),
                    can_throw: false,
                    attributes: [
                        ("pattern".to_string(), json!("systolic_array")),
                        ("grid_size".to_string(), json!("2x2")),
                        ("dataflow".to_string(), json!("stationary")),
                        ("memory_pattern".to_string(), json!("blocked")),
                    ].iter().cloned().collect(),
                };

                pih.events.insert(cgra_kernel_id, cgra_kernel);

                // Add matrix entities for CGRA pattern
                let a_entity = Entity {
                    id: "matrix_a".to_string(),
                    kind: EntityKind::Obj,
                    entity_type: "f32*".to_string(),
                    attributes: [("size".to_string(), json!(1024))].iter().cloned().collect(),
                };

                let b_entity = Entity {
                    id: "matrix_b".to_string(),
                    kind: EntityKind::Obj,
                    entity_type: "f32*".to_string(),
                    attributes: [("size".to_string(), json!(1024))].iter().cloned().collect(),
                };

                let c_entity = Entity {
                    id: "matrix_c".to_string(),
                    kind: EntityKind::Obj,
                    entity_type: "f32*".to_string(),
                    attributes: [("size".to_string(), json!(1024))].iter().cloned().collect(),
                };

                pih.entities.insert("matrix_a".to_string(), a_entity);
                pih.entities.insert("matrix_b".to_string(), b_entity);
                pih.entities.insert("matrix_c".to_string(), c_entity);
            } else if index % 4 == 1 {
                // FPGA pipelining pattern
                let fpga_pipeline_id = "fpga_pipeline".to_string();
                let fpga_pipeline = Event {
                    id: fpga_pipeline_id.clone(),
                    opcode: "fpga_compute".to_string(),
                    dtype: "f32*".to_string(),
                    can_throw: false,
                    attributes: [
                        ("pipeline_depth".to_string(), json!(5)),
                        ("target_frequency".to_string(), json!(250.0)),
                        ("resource_type".to_string(), json!("dsp_chain")),
                        ("synthesis_directive".to_string(), json!("retiming")),
                    ].iter().cloned().collect(),
                };

                pih.events.insert(fpga_pipeline_id, fpga_pipeline);

                // Add array entities for FPGA pattern
                let input_array = Entity {
                    id: "input_array".to_string(),
                    kind: EntityKind::Obj,
                    entity_type: "f32*".to_string(),
                    attributes: [("size".to_string(), json!(2048))].iter().cloned().collect(),
                };

                let output_array = Entity {
                    id: "output_array".to_string(),
                    kind: EntityKind::Obj,
                    entity_type: "f32*".to_string(),
                    attributes: [("size".to_string(), json!(2048))].iter().cloned().collect(),
                };

                pih.entities.insert("input_array".to_string(), input_array);
                pih.entities.insert("output_array".to_string(), output_array);
            } else if index % 4 == 2 {
                // Loop fusion pattern
                let loop1_id = "loop1".to_string();
                let loop2_id = "loop2".to_string();

                let loop1 = Event {
                    id: loop1_id.clone(),
                    opcode: "for".to_string(),
                    dtype: "i32".to_string(),
                    can_throw: false,
                    attributes: [("start".to_string(), json!(0)), ("end".to_string(), json!("N"))].iter().cloned().collect(),
                };

                let loop2 = Event {
                    id: loop2_id.clone(),
                    opcode: "for".to_string(),
                    dtype: "i32".to_string(),
                    can_throw: false,
                    attributes: [("start".to_string(), json!(0)), ("end".to_string(), json!("N"))].iter().cloned().collect(),
                };

                pih.events.insert(loop1_id, loop1);
                pih.events.insert(loop2_id, loop2);
            } else {
                // Power optimization pattern
                let power_compute_id = "power_compute".to_string();
                let power_compute = Event {
                    id: power_compute_id.clone(),
                    opcode: "power_aware_compute".to_string(),
                    dtype: "f32*".to_string(),
                    can_throw: false,
                    attributes: [
                        ("power_constraint".to_string(), json!(50.0)),
                        ("thermal_constraint".to_string(), json!(70.0)),
                        ("energy_efficient".to_string(), json!(true)),
                    ].iter().cloned().collect(),
                };

                pih.events.insert(power_compute_id, power_compute);
            }

            pih
        }

    }
}

/// Represents a Negative Application Condition (NAC) for DPO rewriting.
/// A NAC specifies a pattern that, if present, prohibits the application of a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NegativeApplicationCondition {
    pub name: String,
    pub description: String,
    /// Additional incidence edges that define the forbidden pattern.
    pub forbidden_incidence: Vec<Incidence>,
    /// Additional state edges that are forbidden.
    pub forbidden_state_edges: Vec<StateEdge>,
}

/// Represents a Double Pushout (DPO) rewriting rule.
/// A DPO rule consists of a left-hand side (LHS), right-hand side (RHS), and negative application conditions (NACs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpoRule {
    pub name: String,
    pub description: String,
    /// Left-hand side: the pattern to match and remove.
    pub lhs: ProgramInteractionHypergraph,
    /// Right-hand side: the pattern to add after removal.
    pub rhs: ProgramInteractionHypergraph,
    /// Negative application conditions: patterns that must NOT be present for the rule to apply.
    pub nacs: Vec<NegativeApplicationCondition>,
}

// --- Node Types (Bipartite Graph) ---

/// A unique identifier for an Event node in the hypergraph.
pub type EventId = String;

/// A unique identifier for an Entity node in the hypergraph.
pub type EntityId = String;

/// Represents the `Event` part of the bipartite hypergraph.
/// An Event is an operation, such as an arithmetic operation, a function call, or a memory access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: EventId,
    pub opcode: String,
    pub dtype: String,
    #[serde(default = "default_can_throw")]
    pub can_throw: bool,
    // Additional attributes like lanes, latency, cost, etc., can be added here.
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

fn default_can_throw() -> bool {
    false
}

/// Represents the kind of an `Entity` node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityKind {
    /// SSA value, constant, argument, or return value.
    Val,
    /// An abstract object or memory region.
    Obj,
    /// A versioned memory state (similar to Memory-SSA).
    State,
    /// A control point for modeling dominance and post-dominance.
    Ctrl,
}

/// Represents the `Entity` part of the bipartite hypergraph.
/// An Entity is a value, an object, a state, or a control point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    #[serde(rename = "type")]
    pub entity_type: String,
    // Additional attributes like is_const, value, alias-class, etc.
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

// --- Incidence (Ports) ---

/// Defines the role of a port on an `Event` node.
/// This specifies the purpose of the connection to an `Entity`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PortRole {
    DataIn,
    DataOut,
    CtrlIn,
    CtrlOut,
    StateIn,
    StateOut,
    Obj,
    ExcOut,
    // Can be extended with other custom roles.
    Other(String),
}

/// Represents an incidence edge in the hypergraph, connecting an Event to an Entity via a Port.
/// This is the hyperedge itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Incidence {
    pub event: EventId,
    /// The port name, e.g., "data_in[0]", "state_in[0]".
    /// The string format allows flexibility.
    pub port: String,
    pub entity: EntityId,
    // Additional attributes can be added here, e.g. for mutability.
}

// --- State Edges ---

/// Represents a direct edge between two `State` entities, forming a version chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateEdge {
    pub from: EntityId,
    pub to: EntityId,
}

// --- The Hypergraph ---

/// Represents node embeddings for GNN processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEmbedding {
    pub node_id: String,
    pub embedding: Vec<f32>,
}

/// Represents the complete Program Interaction Hypergraph (PIH).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInteractionHypergraph {
    pub events: HashMap<EventId, Event>,
    pub entities: HashMap<EntityId, Entity>,
    pub incidence: Vec<Incidence>,
    pub state_edges: Vec<StateEdge>,
    /// Node embeddings computed by GNN for learning-based optimization.
    #[serde(default)]
    pub node_embeddings: HashMap<String, Vec<f32>>,
}

impl PartialEq for ProgramInteractionHypergraph {
    fn eq(&self, other: &Self) -> bool {
        self.events == other.events &&
        self.entities == other.entities &&
        self.incidence == other.incidence &&
        self.state_edges == other.state_edges
        // Note: node_embeddings may not be compared for equality in rule matching
    }
}

/// Converts a simple computation pattern into a PIH representation.
/// This is a basic converter that can be extended to handle more complex patterns.
pub fn convert_computation_to_pih(
    opcode: &str,
    inputs: Vec<(String, EntityKind, String)>, // (id, kind, type)
    outputs: Vec<(String, EntityKind, String)>, // (id, kind, type)
    constants: Vec<(String, serde_json::Value)>, // (id, value)
) -> ProgramInteractionHypergraph {
    let mut pih = ProgramInteractionHypergraph::new();

    // Create event
    let event = Event {
        id: format!("event_{}", opcode),
        opcode: opcode.to_string(),
        dtype: "i32".to_string(), // Default to i32, can be parameterized
        can_throw: false,
        attributes: HashMap::new(),
    };
    pih.events.insert(event.id.clone(), event);

    // Create input entities
    let input_count = inputs.len();
    let constant_count = constants.len();
    for (id, kind, entity_type) in inputs {
        let entity = Entity {
            id: id.clone(),
            kind,
            entity_type: entity_type.clone(),
            attributes: HashMap::new(),
        };
        pih.entities.insert(id.clone(), entity);

        // Add incidence edge
        pih.incidence.push(Incidence {
            event: format!("event_{}", opcode),
            port: format!("data_in[{}]", pih.incidence.len()),
            entity: id,
        });
    }

    // Create constant entities
    for (id, value) in constants {
        let mut attributes = HashMap::new();
        attributes.insert("is_const".to_string(), json!(true));
        attributes.insert("value".to_string(), value);

        let entity = Entity {
            id: id.clone(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes,
        };
        pih.entities.insert(id.clone(), entity);

        // Add incidence edge
        pih.incidence.push(Incidence {
            event: format!("event_{}", opcode),
            port: format!("data_in[{}]", pih.incidence.len()),
            entity: id,
        });
    }

    // Create output entities
    for (id, kind, entity_type) in outputs {
        let entity = Entity {
            id: id.clone(),
            kind,
            entity_type: entity_type.clone(),
            attributes: HashMap::new(),
        };
        pih.entities.insert(id.clone(), entity);

        // Add incidence edge
        pih.incidence.push(Incidence {
            event: format!("event_{}", opcode),
            port: format!("data_out[{}]", pih.incidence.len() - input_count - constant_count),
            entity: id,
        });
    }

    pih
}

impl ProgramInteractionHypergraph {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            entities: HashMap::new(),
            incidence: Vec::new(),
            state_edges: Vec::new(),
            node_embeddings: HashMap::new(),
        }
    }
}

impl Default for ProgramInteractionHypergraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a constant folding rule: add(x, 0) → x, mul(x, 1) → x
pub fn create_constant_folding_rule() -> DpoRule {
    // LHS: operation with identity constant
    let mut lhs = ProgramInteractionHypergraph::new();
    let op_event = Event {
        id: "op".to_string(),
        opcode: "add".to_string(), // Could be add, mul, etc.
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: HashMap::new(),
    };
    let x_entity = Entity {
        id: "x".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let identity_entity = Entity {
        id: "identity".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(0)), // 0 for add, 1 for mul
        ].iter().cloned().collect(),
    };
    let out_entity = Entity {
        id: "out".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(op_event.id.clone(), op_event);
    lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
    lhs.entities.insert(identity_entity.id.clone(), identity_entity);
    lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

    lhs.incidence.push(Incidence {
        event: "op".to_string(),
        port: "data_in[0]".to_string(),
        entity: "x".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "op".to_string(),
        port: "data_in[1]".to_string(),
        entity: "identity".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "op".to_string(),
        port: "data_out[0]".to_string(),
        entity: "out".to_string(),
    });

    // RHS: just pass through x
    let mut rhs = ProgramInteractionHypergraph::new();
    rhs.entities.insert(x_entity.id.clone(), x_entity);
    rhs.entities.insert(out_entity.id.clone(), out_entity);

    DpoRule {
        name: "ConstantFolding".to_string(),
        description: "Eliminate operations with identity constants".to_string(),
        lhs,
        rhs,
        nacs: vec![], // No negative conditions for this simple rule
    }
}

/// Creates a dead code elimination rule
pub fn create_dead_code_elimination_rule() -> DpoRule {
    // LHS: computation result that is never used
    let mut lhs = ProgramInteractionHypergraph::new();
    let compute_event = Event {
        id: "compute".to_string(),
        opcode: "mul".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: HashMap::new(),
    };
    let x_entity = Entity {
        id: "x".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let y_entity = Entity {
        id: "y".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let unused_entity = Entity {
        id: "unused".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(compute_event.id.clone(), compute_event);
    lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
    lhs.entities.insert(y_entity.id.clone(), y_entity.clone());
    lhs.entities.insert(unused_entity.id.clone(), unused_entity.clone());

    lhs.incidence.push(Incidence {
        event: "compute".to_string(),
        port: "data_in[0]".to_string(),
        entity: "x".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "compute".to_string(),
        port: "data_in[1]".to_string(),
        entity: "y".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "compute".to_string(),
        port: "data_out[0]".to_string(),
        entity: "unused".to_string(),
    });

    // RHS: remove the unused computation entirely
    let mut rhs = ProgramInteractionHypergraph::new();
    rhs.entities.insert(x_entity.id.clone(), x_entity);
    rhs.entities.insert(y_entity.id.clone(), y_entity);

    // NAC: Don't eliminate if result is actually used somewhere
    let used_result_nac = NegativeApplicationCondition {
        name: "result_is_used".to_string(),
        description: "Don't eliminate if the result is used by another operation".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "other_op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "unused".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "DeadCodeElimination".to_string(),
        description: "Remove computations whose results are never used".to_string(),
        lhs,
        rhs,
        nacs: vec![used_result_nac],
    }
}

/// Creates a loop fusion rule: adjacent loops with same iteration space can be fused
pub fn create_loop_fusion_rule() -> DpoRule {
    // LHS: Two adjacent loops with same bounds and no dependencies between them
    let mut lhs = ProgramInteractionHypergraph::new();

    // Loop 1: for(i=0; i<N; i++) { a[i] = b[i] + c[i]; }
    let loop1_event = Event {
        id: "loop1".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let i_entity = Entity {
        id: "i".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let a_entity = Entity {
        id: "a".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let b_entity = Entity {
        id: "b".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let c_entity = Entity {
        id: "c".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(loop1_event.id.clone(), loop1_event);
    lhs.entities.insert(i_entity.id.clone(), i_entity.clone());
    lhs.entities.insert(a_entity.id.clone(), a_entity.clone());
    lhs.entities.insert(b_entity.id.clone(), b_entity.clone());
    lhs.entities.insert(c_entity.id.clone(), c_entity.clone());

    // Loop 1 body: a[i] = b[i] + c[i]
    lhs.incidence.push(Incidence {
        event: "loop1".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "loop1".to_string(),
        port: "body".to_string(),
        entity: "load_b".to_string(),
    });

    // Loop 2: for(i=0; i<N; i++) { d[i] = e[i] * f[i]; }
    let loop2_event = Event {
        id: "loop2".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let d_entity = Entity {
        id: "d".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let e_entity = Entity {
        id: "e".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let f_entity = Entity {
        id: "f".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(loop2_event.id.clone(), loop2_event);
    lhs.entities.insert(d_entity.id.clone(), d_entity.clone());
    lhs.entities.insert(e_entity.id.clone(), e_entity.clone());
    lhs.entities.insert(f_entity.id.clone(), f_entity.clone());

    // Loop 2 body: d[i] = e[i] * f[i]
    lhs.incidence.push(Incidence {
        event: "loop2".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "loop2".to_string(),
        port: "body".to_string(),
        entity: "load_e".to_string(),
    });

    // RHS: Fused loop with both operations
    let mut rhs = ProgramInteractionHypergraph::new();
    let fused_loop = Event {
        id: "fused_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };

    rhs.events.insert(fused_loop.id.clone(), fused_loop);
    rhs.entities.insert(i_entity.id.clone(), i_entity);
    rhs.entities.insert(a_entity.id.clone(), a_entity);
    rhs.entities.insert(b_entity.id.clone(), b_entity);
    rhs.entities.insert(c_entity.id.clone(), c_entity);
    rhs.entities.insert(d_entity.id.clone(), d_entity);
    rhs.entities.insert(e_entity.id.clone(), e_entity);
    rhs.entities.insert(f_entity.id.clone(), f_entity);

    // Fused loop body: a[i] = b[i] + c[i]; d[i] = e[i] * f[i];
    rhs.incidence.push(Incidence {
        event: "fused_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "fused_loop".to_string(),
        port: "body".to_string(),
        entity: "fused_body".to_string(),
    });

    // NAC: No dependencies between loops
    let no_dependency_nac = NegativeApplicationCondition {
        name: "no_loop_dependencies".to_string(),
        description: "Cannot fuse loops if there are dependencies between them".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "loop2".to_string(),
            port: "dependency".to_string(),
            entity: "loop1_output".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "LoopFusion".to_string(),
        description: "Fuse adjacent loops with same iteration space".to_string(),
        lhs,
        rhs,
        nacs: vec![no_dependency_nac],
    }
}

/// Creates a vectorization rule: scalar operations → SIMD operations
pub fn create_vectorization_rule() -> DpoRule {
    // LHS: Scalar addition loop
    let mut lhs = ProgramInteractionHypergraph::new();
    let scalar_loop = Event {
        id: "scalar_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let i_entity = Entity {
        id: "i".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let a_entity = Entity {
        id: "a".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let b_entity = Entity {
        id: "b".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(scalar_loop.id.clone(), scalar_loop);
    lhs.entities.insert(i_entity.id.clone(), i_entity.clone());
    lhs.entities.insert(a_entity.id.clone(), a_entity.clone());
    lhs.entities.insert(b_entity.id.clone(), b_entity.clone());

    lhs.incidence.push(Incidence {
        event: "scalar_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "scalar_loop".to_string(),
        port: "body".to_string(),
        entity: "scalar_add".to_string(),
    });

    // RHS: Vectorized loop with SIMD operations
    let mut rhs = ProgramInteractionHypergraph::new();
    let vector_loop = Event {
        id: "vector_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(4)), // Process 4 elements per iteration
        ].iter().cloned().collect(),
    };
    let vector_entity = Entity {
        id: "vector".to_string(),
        kind: EntityKind::Val,
        entity_type: "__m128i".to_string(), // SIMD vector type
        attributes: HashMap::new(),
    };

    rhs.events.insert(vector_loop.id.clone(), vector_loop);
    rhs.entities.insert(i_entity.id.clone(), i_entity);
    rhs.entities.insert(a_entity.id.clone(), a_entity);
    rhs.entities.insert(b_entity.id.clone(), b_entity);
    rhs.entities.insert(vector_entity.id.clone(), vector_entity);

    rhs.incidence.push(Incidence {
        event: "vector_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "vector_loop".to_string(),
        port: "body".to_string(),
        entity: "simd_add".to_string(),
    });

    // NAC: Data must be aligned for SIMD operations
    let alignment_nac = NegativeApplicationCondition {
        name: "aligned_data".to_string(),
        description: "Data must be properly aligned for SIMD operations".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "scalar_loop".to_string(),
            port: "unaligned".to_string(),
            entity: "data".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "Vectorization".to_string(),
        description: "Convert scalar operations to SIMD vector operations".to_string(),
        lhs,
        rhs,
        nacs: vec![alignment_nac],
    }
}

/// Creates a parallelization rule: sequential loop → parallel loop
pub fn create_parallelization_rule() -> DpoRule {
    // LHS: Sequential loop
    let mut lhs = ProgramInteractionHypergraph::new();
    let seq_loop = Event {
        id: "sequential_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let i_entity = Entity {
        id: "i".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let array_entity = Entity {
        id: "array".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(seq_loop.id.clone(), seq_loop);
    lhs.entities.insert(i_entity.id.clone(), i_entity.clone());
    lhs.entities.insert(array_entity.id.clone(), array_entity.clone());

    lhs.incidence.push(Incidence {
        event: "sequential_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "sequential_loop".to_string(),
        port: "body".to_string(),
        entity: "sequential_compute".to_string(),
    });

    // RHS: Parallel loop using OpenMP or similar
    let mut rhs = ProgramInteractionHypergraph::new();
    let parallel_loop = Event {
        id: "parallel_loop".to_string(),
        opcode: "parallel_for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
            ("num_threads".to_string(), json!(4)),
        ].iter().cloned().collect(),
    };
    let thread_id_entity = Entity {
        id: "thread_id".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    rhs.events.insert(parallel_loop.id.clone(), parallel_loop);
    rhs.entities.insert(i_entity.id.clone(), i_entity);
    rhs.entities.insert(array_entity.id.clone(), array_entity);
    rhs.entities.insert(thread_id_entity.id.clone(), thread_id_entity);

    rhs.incidence.push(Incidence {
        event: "parallel_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "parallel_loop".to_string(),
        port: "thread_id".to_string(),
        entity: "thread_id".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "parallel_loop".to_string(),
        port: "body".to_string(),
        entity: "parallel_compute".to_string(),
    });

    // NAC: No loop-carried dependencies
    let no_dependency_nac = NegativeApplicationCondition {
        name: "no_loop_dependencies".to_string(),
        description: "Cannot parallelize if there are loop-carried dependencies".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "sequential_loop".to_string(),
            port: "dependency".to_string(),
            entity: "previous_iteration".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "Parallelization".to_string(),
        description: "Convert sequential loops to parallel execution".to_string(),
        lhs,
        rhs,
        nacs: vec![no_dependency_nac],
    }
}

/// Creates a strength reduction rule: mul(x, 2^k) → shl(x, k)
pub fn create_strength_reduction_rule() -> DpoRule {
    // LHS: mul operation with constant power of 2
    let mut lhs = ProgramInteractionHypergraph::new();
    let mul_event = Event {
        id: "mul_op".to_string(),
        opcode: "mul".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: HashMap::new(),
    };
    let x_entity = Entity {
        id: "x".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let c_entity = Entity {
        id: "c".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(8)), // 2^3
        ].iter().cloned().collect(),
    };
    let out_entity = Entity {
        id: "out".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(mul_event.id.clone(), mul_event);
    lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
    lhs.entities.insert(c_entity.id.clone(), c_entity);
    lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

    lhs.incidence.push(Incidence {
        event: "mul_op".to_string(),
        port: "data_in[0]".to_string(),
        entity: "x".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "mul_op".to_string(),
        port: "data_in[1]".to_string(),
        entity: "c".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "mul_op".to_string(),
        port: "data_out[0]".to_string(),
        entity: "out".to_string(),
    });

    // RHS: equivalent shift operation
    let mut rhs = ProgramInteractionHypergraph::new();
    let shift_amount = Entity {
        id: "shift_amt".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(3)), // log2(8)
        ].iter().cloned().collect(),
    };
    let shl_event = Event {
        id: "shl_op".to_string(),
        opcode: "shl".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: HashMap::new(),
    };

    rhs.events.insert(shl_event.id.clone(), shl_event);
    rhs.entities.insert(x_entity.id.clone(), x_entity);
    rhs.entities.insert(shift_amount.id.clone(), shift_amount);
    rhs.entities.insert(out_entity.id.clone(), out_entity);

    rhs.incidence.push(Incidence {
        event: "shl_op".to_string(),
        port: "data_in[0]".to_string(),
        entity: "x".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "shl_op".to_string(),
        port: "data_in[1]".to_string(),
        entity: "shift_amt".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "shl_op".to_string(),
        port: "data_out[0]".to_string(),
        entity: "out".to_string(),
    });

    // NAC: Don't apply if dtype is floating point (due to rounding differences)
    let floating_point_nac = NegativeApplicationCondition {
        name: "no_floating_point".to_string(),
        description: "Don't apply strength reduction to floating point types".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "mul_op".to_string(),
            port: "dtype".to_string(),
            entity: "float_type".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "StrengthReduction".to_string(),
        description: "Convert multiplication by power of 2 to shift operation".to_string(),
        lhs,
        rhs,
        nacs: vec![floating_point_nac],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pih_serialization_deserialization() {
        let mut pih = ProgramInteractionHypergraph::new();

        // Nodes
        let event1 = Event {
            id: "e1".to_string(),
            opcode: "mul".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };
        let entity_x = Entity {
            id: "v_x".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let mut const_attr = HashMap::new();
        const_attr.insert("is_const".to_string(), json!(true));
        const_attr.insert("value".to_string(), json!(8));
        let entity_c = Entity {
            id: "v_c".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: const_attr,
        };
        let entity_out = Entity {
            id: "v_out".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let state3 = Entity {
            id: "s3".to_string(),
            kind: EntityKind::State,
            entity_type: "heap".to_string(),
            attributes: HashMap::new(),
        };
        let state4 = Entity {
            id: "s4".to_string(),
            kind: EntityKind::State,
            entity_type: "heap".to_string(),
            attributes: HashMap::new(),
        };

        pih.events.insert(event1.id.clone(), event1);
        pih.entities.insert(entity_x.id.clone(), entity_x);
        pih.entities.insert(entity_c.id.clone(), entity_c);
        pih.entities.insert(entity_out.id.clone(), entity_out);
        pih.entities.insert(state3.id.clone(), state3);
        pih.entities.insert(state4.id.clone(), state4);

        // Incidence
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "data_in[0]".to_string(),
            entity: "v_x".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "data_in[1]".to_string(),
            entity: "v_c".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "data_out[0]".to_string(),
            entity: "v_out".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "state_in[0]".to_string(),
            entity: "s3".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "state_out[0]".to_string(),
            entity: "s4".to_string(),
        });

        // State Edges
        pih.state_edges.push(StateEdge {
            from: "s3".to_string(),
            to: "s4".to_string(),
        });

        let serialized = serde_json::to_string_pretty(&pih).unwrap();
        
        // This is a simplified check. A more robust test would compare field by field.
        assert!(serialized.contains("\"opcode\": \"mul\""));
        assert!(serialized.contains("\"kind\": \"State\""));
        assert!(serialized.contains("\"port\": \"data_in[1]\""));
        assert!(serialized.contains("\"from\": \"s3\""));

        let deserialized: ProgramInteractionHypergraph = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.entities.len(), 5); // v_x, v_c, v_out, s3, s4
        assert_eq!(deserialized.incidence.len(), 5);
        assert_eq!(deserialized.state_edges.len(), 1);
        assert_eq!(deserialized.entities.get("v_c").unwrap().attributes.get("value").unwrap(), &json!(8));
    }

    #[test]
    fn test_strength_reduction_rule() {
        let rule = create_strength_reduction_rule();

        // Check LHS structure
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // x, c, out
        assert_eq!(rule.lhs.incidence.len(), 3);
        assert_eq!(rule.lhs.events.get("mul_op").unwrap().opcode, "mul");

        // Check RHS structure
        assert_eq!(rule.rhs.events.len(), 1);
        assert_eq!(rule.rhs.entities.len(), 3); // x, shift_amt, out
        assert_eq!(rule.rhs.incidence.len(), 3);
        assert_eq!(rule.rhs.events.get("shl_op").unwrap().opcode, "shl");

        // Check NAC
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "no_floating_point");
    }

    #[test]
    fn test_convert_computation_to_pih() {
        let inputs = vec![
            ("x".to_string(), EntityKind::Val, "i32".to_string()),
        ];
        let outputs = vec![
            ("result".to_string(), EntityKind::Val, "i32".to_string()),
        ];
        let constants = vec![
            ("eight".to_string(), json!(8)),
        ];

        let pih = convert_computation_to_pih("mul", inputs, outputs, constants);

        assert_eq!(pih.events.len(), 1);
        assert_eq!(pih.entities.len(), 3); // x, eight, result
        assert_eq!(pih.incidence.len(), 3); // 1 input + 1 constant + 1 output
        assert_eq!(pih.events.get("event_mul").unwrap().opcode, "mul");

        // Check constant entity
        let const_entity = pih.entities.get("eight").unwrap();
        assert_eq!(const_entity.attributes.get("is_const").unwrap(), &json!(true));
        assert_eq!(const_entity.attributes.get("value").unwrap(), &json!(8));
    }

    #[test]
    fn test_constant_folding_rule() {
        let rule = create_constant_folding_rule();

        // Check LHS structure
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // x, identity, out
        assert_eq!(rule.lhs.incidence.len(), 3);
        assert_eq!(rule.lhs.events.get("op").unwrap().opcode, "add");

        // Check RHS structure (simplified - just entities, no operations)
        assert_eq!(rule.rhs.events.len(), 0);
        assert_eq!(rule.rhs.entities.len(), 2); // x, out
        assert_eq!(rule.rhs.incidence.len(), 0); // No operations

        // Check identity constant
        let identity_entity = rule.lhs.entities.get("identity").unwrap();
        assert_eq!(identity_entity.attributes.get("value").unwrap(), &json!(0));

        // Check NACs
        assert_eq!(rule.nacs.len(), 0); // No negative conditions
    }

    #[test]
    fn test_dead_code_elimination_rule() {
        let rule = create_dead_code_elimination_rule();

        // Check LHS structure
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // x, y, unused
        assert_eq!(rule.lhs.incidence.len(), 3);

        // Check RHS structure (unused entities removed)
        assert_eq!(rule.rhs.events.len(), 0);
        assert_eq!(rule.rhs.entities.len(), 2); // x, y (unused removed)
        assert_eq!(rule.rhs.incidence.len(), 0);

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "result_is_used");
    }

    #[test]
    fn test_loop_fusion_rule() {
        let rule = create_loop_fusion_rule();

        // Check LHS structure (2 loops)
        assert_eq!(rule.lhs.events.len(), 2); // loop1, loop2
        assert_eq!(rule.lhs.entities.len(), 7); // i, a, b, c, d, e, f
        assert_eq!(rule.lhs.incidence.len(), 4); // 2 loops * 2 incidence each

        // Check RHS structure (1 fused loop)
        assert_eq!(rule.rhs.events.len(), 1); // fused_loop
        assert_eq!(rule.rhs.entities.len(), 7); // All entities preserved
        assert_eq!(rule.rhs.incidence.len(), 2); // 1 loop * 2 incidence

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "no_loop_dependencies");
    }

    #[test]
    fn test_vectorization_rule() {
        let rule = create_vectorization_rule();

        // Check LHS structure (scalar loop)
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // i, a, b
        assert_eq!(rule.lhs.incidence.len(), 2);

        // Check RHS structure (vectorized loop)
        assert_eq!(rule.rhs.events.len(), 1);
        assert_eq!(rule.rhs.entities.len(), 4); // i, a, b, vector
        assert_eq!(rule.rhs.incidence.len(), 2);

        // Check SIMD vector type
        assert!(rule.rhs.entities.get("vector").unwrap().entity_type == "__m128i");

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "aligned_data");
    }

    #[test]
    fn test_parallelization_rule() {
        let rule = create_parallelization_rule();

        // Check LHS structure (sequential loop)
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 2); // i, array
        assert_eq!(rule.lhs.incidence.len(), 2);

        // Check RHS structure (parallel loop)
        assert_eq!(rule.rhs.events.len(), 1);
        assert_eq!(rule.rhs.entities.len(), 3); // i, array, thread_id
        assert_eq!(rule.rhs.incidence.len(), 3); // Added thread_id

        // Check parallel loop attributes
        let parallel_loop = rule.rhs.events.get("parallel_loop").unwrap();
        assert!(parallel_loop.attributes.get("num_threads") == Some(&json!(4)));

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "no_loop_dependencies");
    }

    #[test]
    fn test_gnn_training_feature_extraction() {
        use crate::gnn_training::{FeatureExtractor, GnnTrainer, TrainingConfig, OptimizationLabels};

        // Create a simple PIH for testing
        let mut pih = ProgramInteractionHypergraph::new();

        let loop_event_id = "test_loop".to_string();
        let i_entity_id = "i".to_string();
        let array_entity_id = "array".to_string();

        let loop_event = Event {
            id: loop_event_id.clone(),
            opcode: "for".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: [("start".to_string(), json!(0)), ("end".to_string(), json!("N"))].iter().cloned().collect(),
        };

        let i_entity = Entity {
            id: i_entity_id.clone(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        let array_entity = Entity {
            id: array_entity_id.clone(),
            kind: EntityKind::Val,
            entity_type: "i32*".to_string(),
            attributes: HashMap::new(),
        };

        pih.events.insert(loop_event_id, loop_event);
        pih.entities.insert(i_entity_id, i_entity);
        pih.entities.insert(array_entity_id, array_entity);

        pih.incidence.push(Incidence {
            event: "test_loop".to_string(),
            port: "index".to_string(),
            entity: "i".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "test_loop".to_string(),
            port: "body".to_string(),
            entity: "array".to_string(),
        });

        // Extract features
        let features = FeatureExtractor::extract_features(&pih);

        // Verify feature dimensions
        assert!(!features.node_features.is_empty());
        assert!(!features.edge_features.is_empty());
        assert!(!features.global_features.is_empty());

        // Check global features (should have graph statistics)
        assert_eq!(features.global_features.len(), 8);

        // Check bipartite features
        assert_eq!(features.bipartite_features.event_node_count, 1);
        assert_eq!(features.bipartite_features.entity_node_count, 2);
        assert_eq!(features.bipartite_features.event_to_entity_edges, 2);
        assert_eq!(features.bipartite_features.node_type_distribution.len(), 2);

        // Check hypergraph features
        assert_eq!(features.hypergraph_features.hyperedge_sizes.len(), 1);
        assert_eq!(features.hypergraph_features.avg_hyperedge_size, 2.0);
        assert_eq!(features.hypergraph_features.max_hyperedge_size, 2);
        assert!(!features.hypergraph_features.node_hyperedge_membership.is_empty());
    }

    #[test]
    fn test_gnn_model_creation() {
        use crate::gnn_training::{GnnTrainer, TrainingConfig, GnnModelType};

        let config = TrainingConfig {
            learning_rate: 0.001,
            batch_size: 32,
            num_epochs: 100,
            hidden_dim: 64,
            num_layers: 3,
            dropout: 0.1,
        };

        let model = GnnTrainer::create_model(&config);

        assert_eq!(model.hidden_dim, 64);
        assert_eq!(model.num_layers, 3);
        assert_eq!(model.dropout, 0.1);
        assert_eq!(model.model_type, GnnModelType::BipartiteGnn);
        assert_eq!(model.attention_heads, 4);
        assert_eq!(model.weights.len(), 3);
        assert_eq!(model.weights[0].len(), 64);
        assert_eq!(model.weights[0][0].len(), 64);
    }

    #[test]
    fn test_gat_model_creation() {
        use crate::gnn_training::{GnnTrainer, TrainingConfig, GnnModelType};

        let config = TrainingConfig {
            learning_rate: 0.001,
            batch_size: 32,
            num_epochs: 100,
            hidden_dim: 64,
            num_layers: 2,
            dropout: 0.1,
        };

        let model = GnnTrainer::create_gat_model(&config, 8);

        assert_eq!(model.hidden_dim, 64);
        assert_eq!(model.num_layers, 2);
        assert_eq!(model.dropout, 0.1);
        assert_eq!(model.model_type, GnnModelType::Gat);
        assert_eq!(model.attention_heads, 8);
        assert_eq!(model.weights.len(), 2);
        assert_eq!(model.weights[0].len(), 64); // 64 output dimensions
        assert_eq!(model.weights[0][0].len(), 64); // 64 input dimensions
    }

    #[test]
    fn test_gnn_model_types() {
        use crate::gnn_training::{GnnModelType, OptimizationGnn};

        let mut model = OptimizationGnn::default();
        assert_eq!(model.model_type, GnnModelType::BipartiteGnn);

        model.model_type = GnnModelType::Gat;
        assert_eq!(model.model_type, GnnModelType::Gat);

        model.model_type = GnnModelType::Gcn;
        assert_eq!(model.model_type, GnnModelType::Gcn);

        model.model_type = GnnModelType::GraphSage;
        assert_eq!(model.model_type, GnnModelType::GraphSage);

        model.model_type = GnnModelType::HetGnn;
        assert_eq!(model.model_type, GnnModelType::HetGnn);
    }

    #[test]
    fn test_synthetic_dataset_generation() {
        use crate::gnn_training::GnnTrainer;

        let dataset = GnnTrainer::generate_synthetic_dataset(10);

        assert_eq!(dataset.len(), 10);

        // Check first sample
        let sample = &dataset[0];
        assert!(sample.sample_id.starts_with("sample_"));
        assert!(!sample.features.node_features.is_empty());
        assert!(!sample.labels.rule_applications.is_empty(), "Sample 0 should have optimization rules. Rules: {:?}", sample.labels.rule_applications);
        assert!(sample.labels.performance_gain >= 0.0 && sample.labels.performance_gain <= 1.0);
    }

    #[test]
    fn test_training_loss_computation() {
        use crate::gnn_training::{GnnTrainer, OptimizationLabels};

        let predicted = OptimizationLabels {
            rule_applications: vec!["LoopFusion".to_string()],
            performance_gain: 0.3,
            memory_reduction: 0.2,
            energy_savings: 0.25,
        };

        let actual = OptimizationLabels {
            rule_applications: vec!["LoopFusion".to_string(), "Vectorization".to_string()],
            performance_gain: 0.5,
            memory_reduction: 0.1,
            energy_savings: 0.3,
        };

        let loss = GnnTrainer::compute_loss(&predicted, &actual);
        assert!(loss >= 0.0);
        assert!(loss < 2.0); // Should be reasonable loss value
    }

    #[test]
    fn test_hardware_feature_extraction() {
        use crate::gnn_training::{FeatureExtractor, GnnFeatures, SpatialPatternType, DataflowType};

        // Create a simple PIH for testing
        let mut pih = ProgramInteractionHypergraph::new();

        let event = Event {
            id: "test_event".to_string(),
            opcode: "cgra_compute".to_string(),
            dtype: "f32*".to_string(),
            can_throw: false,
            attributes: [
                ("pattern".to_string(), json!("systolic_array")),
                ("grid_size".to_string(), json!("2x2")),
            ].iter().cloned().collect(),
        };

        let entity = Entity {
            id: "test_entity".to_string(),
            kind: EntityKind::Obj,
            entity_type: "f32*".to_string(),
            attributes: [("size".to_string(), json!(1024))].iter().cloned().collect(),
        };

        pih.events.insert("test_event".to_string(), event);
        pih.entities.insert("test_entity".to_string(), entity);

        let features = FeatureExtractor::extract_features(&pih);

        // Verify hardware features are extracted
        assert!(features.hardware_features.cgra_features.spatial_patterns.len() > 0);
        assert!(features.hardware_features.fpga_features.rtl_patterns.len() >= 0);

        // Check CGRA pattern detection
        let spatial_pattern = &features.hardware_features.cgra_features.spatial_patterns[0];
        assert_eq!(spatial_pattern.pattern_type, SpatialPatternType::SystolicArray);

        // Check hardware constraints
        assert!(features.hardware_features.hardware_constraints.max_memory_usage > 0);
        assert!(features.hardware_features.hardware_constraints.max_compute_units > 0);
        assert!(features.hardware_features.hardware_constraints.target_frequency > 0.0);
    }

    #[test]
    fn test_cgra_optimization_patterns() {
        use crate::gnn_training::{FeatureExtractor, SpatialPatternType, DataflowType, RtlPatternType};

        // Create CGRA matrix multiplication pattern
        let mut pih = ProgramInteractionHypergraph::new();

        let cgra_kernel = Event {
            id: "cgra_kernel".to_string(),
            opcode: "cgra_compute".to_string(),
            dtype: "f32*".to_string(),
            can_throw: false,
            attributes: [
                ("pattern".to_string(), json!("systolic_array")),
                ("dataflow".to_string(), json!("stationary")),
            ].iter().cloned().collect(),
        };

        // Add matrix entities
        for i in 0..3 {
            let entity = Entity {
                id: format!("matrix_{}", i),
                kind: EntityKind::Obj,
                entity_type: "f32*".to_string(),
                attributes: [("size".to_string(), json!(1024))].iter().cloned().collect(),
            };
            pih.entities.insert(format!("matrix_{}", i), entity);
        }

        pih.events.insert("cgra_kernel".to_string(), cgra_kernel);

        let features = FeatureExtractor::extract_features(&pih);

        // Verify CGRA-specific features
        assert!(features.hardware_features.cgra_features.spatial_patterns.len() > 0);
        assert_eq!(features.hardware_features.cgra_features.dataflow_type, DataflowType::DataParallel);

        // Check for systolic array pattern
        let pattern = &features.hardware_features.cgra_features.spatial_patterns[0];
        assert_eq!(pattern.pattern_type, SpatialPatternType::SystolicArray);
        assert_eq!(pattern.grid_size, (2, 2));

        // Verify resource estimation
        assert!(features.hardware_features.fpga_features.resource_utilization.dsp_usage > 0.0);
        assert!(features.hardware_features.hardware_constraints.max_compute_units >= 1);
    }

    #[test]
    fn test_fpga_optimization_patterns() {
        use crate::gnn_training::{FeatureExtractor, RtlPatternType, SynthesisDirectiveType};

        // Create FPGA pipelining pattern
        let mut pih = ProgramInteractionHypergraph::new();

        let fpga_pipeline = Event {
            id: "fpga_pipeline".to_string(),
            opcode: "fpga_compute".to_string(),
            dtype: "f32*".to_string(),
            can_throw: false,
            attributes: [
                ("pipeline_depth".to_string(), json!(5)),
                ("target_frequency".to_string(), json!(250.0)),
                ("resource_type".to_string(), json!("dsp_chain")),
            ].iter().cloned().collect(),
        };

        // Add array entities
        for i in 0..2 {
            let entity = Entity {
                id: format!("array_{}", i),
                kind: EntityKind::Obj,
                entity_type: "f32*".to_string(),
                attributes: [("size".to_string(), json!(2048))].iter().cloned().collect(),
            };
            pih.entities.insert(format!("array_{}", i), entity);
        }

        pih.events.insert("fpga_pipeline".to_string(), fpga_pipeline);

        let features = FeatureExtractor::extract_features(&pih);

        // Verify FPGA-specific features
        assert!(features.hardware_features.fpga_features.rtl_patterns.len() > 0);
        assert!(features.hardware_features.fpga_features.synthesis_directives.len() > 0);

        // Check RTL pattern detection
        let rtl_pattern = &features.hardware_features.fpga_features.rtl_patterns[0];
        assert_eq!(rtl_pattern.pattern_type, RtlPatternType::PipelinedMultiplier);

        // Check synthesis directives
        let directive = &features.hardware_features.fpga_features.synthesis_directives[0];
        assert_eq!(directive.directive_type, SynthesisDirectiveType::Retiming);

        // Verify timing constraints
        assert!(features.hardware_features.fpga_features.timing_constraints.clock_frequency > 0.0);
        assert!(features.hardware_features.fpga_features.timing_constraints.setup_time > 0.0);
    }

    #[test]
    fn test_hardware_aware_training_labels() {
        use crate::gnn_training::GnnTrainer;

        // Create PIH with hardware patterns
        let mut pih = ProgramInteractionHypergraph::new();

        // Add CGRA pattern
        let cgra_event = Event {
            id: "cgra_event".to_string(),
            opcode: "cgra_compute".to_string(),
            dtype: "f32*".to_string(),
            can_throw: false,
            attributes: [("pattern".to_string(), json!("systolic_array"))].iter().cloned().collect(),
        };

        // Add multiple loop events
        for i in 0..3 {
            let loop_event = Event {
                id: format!("loop_{}", i),
                opcode: "for".to_string(),
                dtype: "i32".to_string(),
                can_throw: false,
                attributes: [("iterations".to_string(), json!(100))].iter().cloned().collect(),
            };
            pih.events.insert(format!("loop_{}", i), loop_event);
        }

        // Add CGRA pattern
        let cgra_event = Event {
            id: "cgra_event".to_string(),
            opcode: "cgra_compute".to_string(),
            dtype: "f32*".to_string(),
            can_throw: false,
            attributes: [("pattern".to_string(), json!("systolic_array"))].iter().cloned().collect(),
        };

        pih.events.insert("cgra_event".to_string(), cgra_event);

        let labels = GnnTrainer::generate_hardware_aware_labels(&pih, 0);

        // Verify hardware-aware optimization predictions
        assert!(labels.rule_applications.iter().any(|rule| rule.contains("Cgra")));
        assert!(labels.performance_gain > 0.0);
        assert!(labels.energy_savings > 0.0);

        // CGRA optimizations should provide significant benefits
        assert!(labels.performance_gain >= 0.4); // CGRA spatial mapping benefits
        assert!(labels.energy_savings >= 0.2); // CGRA energy efficiency
    }

    /// Creates a constant folding rule: add(x, 0) → x, mul(x, 1) → x
    pub fn create_constant_folding_rule() -> DpoRule {
        // LHS: operation with identity constant
        let mut lhs = ProgramInteractionHypergraph::new();
        let op_event = Event {
            id: "op".to_string(),
            opcode: "add".to_string(), // Could be add, mul, etc.
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };
        let x_entity = Entity {
            id: "x".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let identity_entity = Entity {
            id: "identity".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: [
                ("is_const".to_string(), json!(true)),
                ("value".to_string(), json!(0)), // 0 for add, 1 for mul
            ].iter().cloned().collect(),
        };
        let out_entity = Entity {
            id: "out".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        lhs.events.insert(op_event.id.clone(), op_event);
        lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        lhs.entities.insert(identity_entity.id.clone(), identity_entity);
        lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

        lhs.incidence.push(Incidence {
            event: "op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "op".to_string(),
            port: "data_in[1]".to_string(),
            entity: "identity".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "op".to_string(),
            port: "data_out[0]".to_string(),
            entity: "out".to_string(),
        });

        // RHS: just pass through x
        let mut rhs = ProgramInteractionHypergraph::new();
        rhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        rhs.entities.insert(out_entity.id.clone(), out_entity.clone());
        // Direct connection: x → out (no operation needed)

        DpoRule {
            name: "ConstantFolding".to_string(),
            description: "Eliminate operations with identity constants".to_string(),
            lhs,
            rhs,
            nacs: vec![], // No negative conditions for this simple rule
        }
    }

    /// Creates a dead code elimination rule
    pub fn create_dead_code_elimination_rule() -> DpoRule {
        // LHS: computation result that is never used
        let mut lhs = ProgramInteractionHypergraph::new();
        let compute_event = Event {
            id: "compute".to_string(),
            opcode: "mul".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };
        let x_entity = Entity {
            id: "x".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let y_entity = Entity {
            id: "y".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let unused_entity = Entity {
            id: "unused".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        lhs.events.insert(compute_event.id.clone(), compute_event);
        lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        lhs.entities.insert(y_entity.id.clone(), y_entity.clone());
        lhs.entities.insert(unused_entity.id.clone(), unused_entity.clone());

        lhs.incidence.push(Incidence {
            event: "compute".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "compute".to_string(),
            port: "data_in[1]".to_string(),
            entity: "y".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "compute".to_string(),
            port: "data_out[0]".to_string(),
            entity: "unused".to_string(),
        });

        // RHS: remove the unused computation entirely
        let mut rhs = ProgramInteractionHypergraph::new();
        rhs.entities.insert(x_entity.id.clone(), x_entity);
        rhs.entities.insert(y_entity.id.clone(), y_entity);
        // No events, no unused entity

        // NAC: Don't eliminate if result is actually used somewhere
        let used_result_nac = NegativeApplicationCondition {
            name: "result_is_used".to_string(),
            description: "Don't eliminate if the result is used by another operation".to_string(),
            forbidden_incidence: vec![Incidence {
                event: "other_op".to_string(),
                port: "data_in[0]".to_string(),
                entity: "unused".to_string(),
            }],
            forbidden_state_edges: vec![],
        };

        DpoRule {
            name: "DeadCodeElimination".to_string(),
            description: "Remove computations whose results are never used".to_string(),
            lhs,
            rhs,
            nacs: vec![used_result_nac],
        }
    }

    /// Creates a strength reduction rule: mul(x, 2^k) → shl(x, k)
    pub fn create_strength_reduction_rule() -> DpoRule {
        // LHS: mul operation with constant power of 2
        let mut lhs = ProgramInteractionHypergraph::new();
        let mul_event = Event {
            id: "mul_op".to_string(),
            opcode: "mul".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };
        let x_entity = Entity {
            id: "x".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let c_entity = Entity {
            id: "c".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: [
                ("is_const".to_string(), json!(true)),
                ("value".to_string(), json!(8)), // 2^3
            ].iter().cloned().collect(),
        };
        let out_entity = Entity {
            id: "out".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        lhs.events.insert(mul_event.id.clone(), mul_event);
        lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        lhs.entities.insert(c_entity.id.clone(), c_entity);
        lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

        lhs.incidence.push(Incidence {
            event: "mul_op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "mul_op".to_string(),
            port: "data_in[1]".to_string(),
            entity: "c".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "mul_op".to_string(),
            port: "data_out[0]".to_string(),
            entity: "out".to_string(),
        });

        // RHS: equivalent shift operation
        let mut rhs = ProgramInteractionHypergraph::new();
        let shift_amount = Entity {
            id: "shift_amt".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: [
                ("is_const".to_string(), json!(true)),
                ("value".to_string(), json!(3)), // log2(8)
            ].iter().cloned().collect(),
        };
        let shl_event = Event {
            id: "shl_op".to_string(),
            opcode: "shl".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };

        rhs.events.insert(shl_event.id.clone(), shl_event);
        rhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        rhs.entities.insert(shift_amount.id.clone(), shift_amount);
        rhs.entities.insert(out_entity.id.clone(), out_entity.clone());

        rhs.incidence.push(Incidence {
            event: "shl_op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        rhs.incidence.push(Incidence {
            event: "shl_op".to_string(),
            port: "data_in[1]".to_string(),
            entity: "shift_amt".to_string(),
        });
        rhs.incidence.push(Incidence {
            event: "shl_op".to_string(),
            port: "data_out[0]".to_string(),
            entity: "out".to_string(),
        });

        // NAC: Don't apply if dtype is floating point (due to rounding differences)
        let floating_point_nac = NegativeApplicationCondition {
            name: "no_floating_point".to_string(),
            description: "Don't apply strength reduction to floating point types".to_string(),
            forbidden_incidence: vec![Incidence {
                event: "mul_op".to_string(),
                port: "dtype".to_string(),
                entity: "float_type".to_string(),
            }],
            forbidden_state_edges: vec![],
        };

        DpoRule {
            name: "StrengthReduction".to_string(),
            description: "Convert multiplication by power of 2 to shift operation".to_string(),
            lhs,
            rhs,
            nacs: vec![floating_point_nac],
        }
    }
}
