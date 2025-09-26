use std::collections::HashMap;
use serde_json::{json, Value};

use crate::core::{ProgramInteractionHypergraph, Node, Edge, Incidence, NodeKind, EdgeKind, RoleKind};
use crate::gnn::{GnnFeatures, OptimizationLabels, TrainingSample};
use crate::hardware::HardwareFeatures;
use crate::training::ProductionTrainingSystem;

/// Generate synthetic datasets for training and testing.
pub struct SyntheticDataGenerator;

impl SyntheticDataGenerator {
    /// Generate a synthetic dataset of the specified size.
    pub fn generate_synthetic_dataset(size: usize) -> Vec<TrainingSample> {
        let mut samples = Vec::new();

        for i in 0..size {
            let pih = Self::create_synthetic_pih(i);
            let features = crate::gnn::FeatureExtractor::extract_features(&pih);
            let labels = Self::generate_synthetic_labels(i);
            let hardware_features = Self::generate_synthetic_hardware_features();

            samples.push(TrainingSample {
                sample_id: format!("synthetic_{}", i),
                features,
                labels,
                hardware_features,
                sample_weight: 1.0,
            });
        }

        samples
    }

    /// Generate synthetic optimization labels.
    fn generate_synthetic_labels(index: usize) -> OptimizationLabels {
        let mut applied_rules = Vec::new();

        // Vary optimization rules based on index
        if index % 4 == 0 {
            applied_rules.push("LoopTiling".to_string());
            applied_rules.push("LoopUnrolling".to_string());
        } else if index % 4 == 1 {
            applied_rules.push("SIMDVectorization".to_string());
            applied_rules.push("CgraSpatialMapping".to_string());
        } else if index % 4 == 2 {
            applied_rules.push("Parallelization".to_string());
            applied_rules.push("PowerOptimization".to_string());
        } else {
            applied_rules.push("ConstantFolding".to_string());
            applied_rules.push("DeadCodeElimination".to_string());
        }

        // Vary performance characteristics
        let performance_base = 0.3 + (index as f32 % 0.5);
        let power_base = 20.0 + (index as f32 % 10.0);

        OptimizationLabels {
            applied_rules,
            performance_improvement: performance_base,
            power_consumption: power_base,
            hardware_constraints: vec!["latency".to_string(), "throughput".to_string()],
            optimization_impact: vec![0.1, 0.2, 0.15],
        }
    }

    /// Generate synthetic hardware features.
    fn generate_synthetic_hardware_features() -> HardwareFeatures {
        // Create synthetic CGRA features
        let cgra_features = crate::hardware::CgraFeatures {
            spatial_compute_units: 16,
            reconfiguration_time: 0.5,
            memory_bandwidth: 12.8,
            compute_density: 0.8,
            power_efficiency: 0.9,
        };

        // Create synthetic FPGA features
        let fpga_features = crate::hardware::FpgaFeatures {
            lut_count: 50000,
            dsp_blocks: 100,
            bram_blocks: 200,
            clock_frequency: 250.0,
            power_consumption: 15.0,
        };

        // Create synthetic hardware constraints
        let hardware_constraints = crate::hardware::HardwareConstraints {
            max_power_consumption: 50.0,
            max_latency: 100.0,
            min_throughput: 1000.0,
            memory_bandwidth: 25.6,
            resource_limits: [
                ("lut".to_string(), 0.8),
                ("dsp".to_string(), 0.6),
                ("bram".to_string(), 0.7),
            ].iter().cloned().collect(),
        };

        // Create synthetic spatial patterns
        let spatial_patterns = vec![
            crate::hardware::SpatialPattern {
                pattern_type: "pipeline".to_string(),
                compute_units: 8,
                memory_accesses: 4,
                communication_volume: 0.3,
                parallelism_degree: 4,
            },
            crate::hardware::SpatialPattern {
                pattern_type: "parallel".to_string(),
                compute_units: 16,
                memory_accesses: 2,
                communication_volume: 0.1,
                parallelism_degree: 8,
            },
        ];

        // Create synthetic RTL patterns
        let rtl_patterns = vec![
            crate::hardware::RtlPattern {
                pattern_type: crate::hardware::RtlPatternType::PipelinedMultiplier,
                pipeline_stages: 3,
                resource_usage: crate::hardware::ResourceEstimate {
                    lut_count: 500,
                    dsp_count: 2,
                    ff_count: 300,
                    bram_count: 0,
                    power_estimate: 0.5,
                },
                timing_estimate: 2.5,
            },
        ];

        // Create synthetic resource utilization
        let resource_utilization = crate::hardware::ResourceUtilization {
            dsp_usage: 0.6,
            lut_usage: 0.4,
            ff_usage: 0.3,
            bram_usage: 0.5,
            power_usage: 0.7,
        };

        // Create synthetic timing constraints
        let timing_constraints = crate::hardware::TimingConstraints {
            clock_frequency: 200.0,
            setup_time: 1.0,
            hold_time: 0.5,
            critical_path_delay: 5.0,
            slack: 2.0,
        };

        HardwareFeatures {
            cgra_features,
            fpga_features,
            hardware_constraints,
            spatial_patterns,
            rtl_patterns,
            resource_utilization,
            timing_constraints,
        }
    }

    /// Create a synthetic PIH for testing and training.
    fn create_synthetic_pih(index: usize) -> ProgramInteractionHypergraph {
        let mut pih = ProgramInteractionHypergraph::new();

        // Create nodes with varying complexity
        if index % 4 == 0 {
            // Simple linear pattern
            let a = Node {
                id: "a".to_string(),
                kind: NodeKind::Val,
                node_type: "i32".to_string(),
                entity_type: Some("i32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            let b = Node {
                id: "b".to_string(),
                kind: NodeKind::Val,
                node_type: "i32".to_string(),
                entity_type: Some("i32".to_string()),
                attributes: [
                    ("is_const".to_string(), serde_json::json!(true)),
                    ("value".to_string(), serde_json::json!(5)),
                ].iter().cloned().collect(),
                cid: None,
            };

            let result = Node {
                id: "result".to_string(),
                kind: NodeKind::Val,
                node_type: "i32".to_string(),
                entity_type: Some("i32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            pih.nodes.push(a);
            pih.nodes.push(b);
            pih.nodes.push(result);

            // Simple add operation
            let add_edge = Edge {
                id: "add".to_string(),
                kind: EdgeKind::Event,
                label: Some("add".to_string()),
                opcode: Some("add".to_string()),
                dtype: Some("i32".to_string()),
                can_throw: false,
                attributes: [
                    ("opcode".to_string(), serde_json::json!("add")),
                    ("commutative".to_string(), serde_json::json!(true)),
                ].iter().cloned().collect(),
                cid: None,
            };

            pih.edges.push(add_edge);

            pih.incidences.push(Incidence {
                edge: "add".to_string(),
                node: "a".to_string(),
                role: RoleKind::DataIn,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "add".to_string(),
                node: "b".to_string(),
                role: RoleKind::DataIn,
                idx: Some(1),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "add".to_string(),
                node: "result".to_string(),
                role: RoleKind::DataOut,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
        } else if index % 4 == 1 {
            // Loop pattern
            let array = Node {
                id: "array".to_string(),
                kind: NodeKind::Obj,
                node_type: "i32*".to_string(),
                entity_type: Some("i32*".to_string()),
                attributes: [
                    ("size".to_string(), serde_json::json!(1000)),
                ].iter().cloned().collect(),
                cid: None,
            };

            let i = Node {
                id: "i".to_string(),
                kind: NodeKind::Val,
                node_type: "i32".to_string(),
                entity_type: Some("i32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            let sum = Node {
                id: "sum".to_string(),
                kind: NodeKind::Val,
                node_type: "i32".to_string(),
                entity_type: Some("i32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            pih.nodes.push(array);
            pih.nodes.push(i);
            pih.nodes.push(sum);

            // Loop edge
            let loop_edge = Edge {
                id: "loop".to_string(),
                kind: EdgeKind::Event,
                label: Some("for".to_string()),
                opcode: Some("for".to_string()),
                dtype: Some("i32".to_string()),
                can_throw: false,
                attributes: [
                    ("start".to_string(), serde_json::json!(0)),
                    ("end".to_string(), serde_json::json!(100)),
                    ("step".to_string(), serde_json::json!(1)),
                    ("loop_type".to_string(), serde_json::json!("reduction")),
                ].iter().cloned().collect(),
                cid: None,
            };

            pih.edges.push(loop_edge);

            pih.incidences.push(Incidence {
                edge: "loop".to_string(),
                node: "array".to_string(),
                role: RoleKind::Obj,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "loop".to_string(),
                node: "i".to_string(),
                role: RoleKind::CtrlIn,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "loop".to_string(),
                node: "sum".to_string(),
                role: RoleKind::StateOut,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
        } else if index % 4 == 2 {
            // Complex computation pattern
            let x = Node {
                id: "x".to_string(),
                kind: NodeKind::Val,
                node_type: "f32".to_string(),
                entity_type: Some("f32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            let y = Node {
                id: "y".to_string(),
                kind: NodeKind::Val,
                node_type: "f32".to_string(),
                entity_type: Some("f32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            let result = Node {
                id: "result".to_string(),
                kind: NodeKind::Val,
                node_type: "f32".to_string(),
                entity_type: Some("f32".to_string()),
                attributes: HashMap::new(),
                cid: None,
            };

            pih.nodes.push(x);
            pih.nodes.push(y);
            pih.nodes.push(result);

            // Multiple operations
            let mul_edge = Edge {
                id: "mul".to_string(),
                kind: EdgeKind::Event,
                label: Some("mul".to_string()),
                opcode: Some("mul".to_string()),
                dtype: Some("f32".to_string()),
                can_throw: false,
                attributes: [
                    ("opcode".to_string(), serde_json::json!("mul")),
                ].iter().cloned().collect(),
                cid: None,
            };

            let add_edge = Edge {
                id: "add".to_string(),
                kind: EdgeKind::Event,
                label: Some("add".to_string()),
                opcode: Some("add".to_string()),
                dtype: Some("f32".to_string()),
                can_throw: false,
                attributes: [
                    ("opcode".to_string(), serde_json::json!("add")),
                ].iter().cloned().collect(),
                cid: None,
            };

            pih.edges.push(mul_edge);
            pih.edges.push(add_edge);

            // Connect operations
            pih.incidences.push(Incidence {
                edge: "mul".to_string(),
                node: "x".to_string(),
                role: RoleKind::DataIn,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "mul".to_string(),
                node: "y".to_string(),
                role: RoleKind::DataIn,
                idx: Some(1),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "add".to_string(),
                node: "mul".to_string(),
                role: RoleKind::DataIn,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "add".to_string(),
                node: "y".to_string(),
                role: RoleKind::DataIn,
                idx: Some(1),
                attrs: HashMap::new(),
                cid: None,
            });
            pih.incidences.push(Incidence {
                edge: "add".to_string(),
                node: "result".to_string(),
                role: RoleKind::DataOut,
                idx: Some(0),
                attrs: HashMap::new(),
                cid: None,
            });
        } else {
            // Advanced compiler transformation patterns - nested loops for tiling
            let outer_loop_id = "outer_loop".to_string();
            let inner_loop_id = "inner_loop".to_string();
            let innermost_loop_id = "tiling_candidate".to_string();

            let outer_loop = Edge {
                id: outer_loop_id.clone(),
                kind: EdgeKind::Event,
                label: Some("for".to_string()),
                opcode: Some("for".to_string()),
                dtype: Some("i32".to_string()),
                can_throw: false,
                attributes: [
                    ("start".to_string(), serde_json::json!(0)),
                    ("end".to_string(), serde_json::json!("N")),
                    ("step".to_string(), serde_json::json!(1)),
                    ("nested_levels".to_string(), serde_json::json!(3)),
                ].iter().cloned().collect(),
                cid: None,
            };

            let inner_loop = Edge {
                id: inner_loop_id.clone(),
                kind: EdgeKind::Event,
                label: Some("for".to_string()),
                opcode: Some("for".to_string()),
                dtype: Some("i32".to_string()),
                can_throw: false,
                attributes: [
                    ("start".to_string(), serde_json::json!(0)),
                    ("end".to_string(), serde_json::json!(100)),
                    ("step".to_string(), serde_json::json!(1)),
                    ("parent_loop".to_string(), serde_json::json!("outer_loop")),
                ].iter().cloned().collect(),
                cid: None,
            };

            let innermost_loop = Edge {
                id: innermost_loop_id.clone(),
                kind: EdgeKind::Event,
                label: Some("for".to_string()),
                opcode: Some("for".to_string()),
                dtype: Some("i32".to_string()),
                can_throw: false,
                attributes: [
                    ("start".to_string(), serde_json::json!(0)),
                    ("end".to_string(), serde_json::json!(4)),
                    ("step".to_string(), serde_json::json!(1)),
                    ("parent_loop".to_string(), serde_json::json!("inner_loop")),
                    ("tiling_candidate".to_string(), serde_json::json!(true)),
                ].iter().cloned().collect(),
                cid: None,
            };

            pih.edges.push(outer_loop);
            pih.edges.push(inner_loop);
            pih.edges.push(innermost_loop);

            // Add array entities for tiling analysis
            let array_node = Node {
                id: "nested_array".to_string(),
                kind: NodeKind::Obj,
                node_type: "i32**".to_string(),
                entity_type: Some("i32**".to_string()),
                attributes: [
                    ("dimensions".to_string(), serde_json::json!(3)),
                    ("size".to_string(), serde_json::json!(1000)),
                ].iter().cloned().collect(),
                cid: None,
            };

            let benchmark_node = Node {
                id: "benchmark_data".to_string(),
                kind: NodeKind::Obj,
                node_type: "benchmark".to_string(),
                entity_type: Some("benchmark".to_string()),
                attributes: [
                    ("dataset_size".to_string(), serde_json::json!(10000)),
                    ("pattern".to_string(), serde_json::json!("nested_loops")),
                ].iter().cloned().collect(),
                cid: None,
            };

            let hardware_node = Node {
                id: "hardware_profile".to_string(),
                kind: NodeKind::Obj,
                node_type: "hardware".to_string(),
                entity_type: Some("hardware".to_string()),
                attributes: [
                    ("target".to_string(), serde_json::json!("cgra")),
                    ("compute_units".to_string(), serde_json::json!(16)),
                ].iter().cloned().collect(),
                cid: None,
            };

            pih.nodes.push(array_node);
            pih.nodes.push(benchmark_node);
            pih.nodes.push(hardware_node);
        }

        pih
    }
}
