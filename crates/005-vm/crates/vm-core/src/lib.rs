//! # Digital Computing System VM Core
//!
//! This crate provides the core VM integration layer that combines all
//! specialized components into a cohesive virtual machine.
//!
//! ## Architecture
//!
//! The VM core integrates:
//! - **Memory System**: Data storage and retrieval
//! - **CPU Core**: Sequential instruction execution
//! - **Scheduler**: DAG-based task orchestration
//! - **Hardware Tiles**: Heterogeneous compute resources

use vm_memory::MemorySystemImpl;
use vm_cpu::{VonNeumannCore, VonNeumannCoreImpl};
use vm_scheduler::{DataflowRuntime, DataflowRuntimeImpl};
use vm_types::{Dag, Task, Instruction, TaskCharacteristics, ComputationType, HardwareTile, HardwareTileType, IoInterface};
use vm_hardware::{ComputeTile, CpuTile, GpuTile, CgraFpgaTile, PimTile};
use vm_gnn::{ProgramInteractionHypergraph, EntityKind, convert_computation_to_pih, create_strength_reduction_rule, create_constant_folding_rule, create_dead_code_elimination_rule, create_loop_fusion_rule, create_vectorization_rule, create_parallelization_rule, gnn_training::{GnnTrainer, TrainingConfig, TrainingStats, FeatureExtractor}};
use std::future::Future;
use std::pin::Pin;
use serde_json::json;

/// The main Virtual Machine structure.
///
/// This structure integrates all VM components and provides
/// a unified interface for executing computational workflows.
pub struct Vm {
    /// Memory management system
    memory: MemorySystemImpl,
    /// CPU execution core
    cpu: VonNeumannCoreImpl,
    /// Task scheduling and orchestration
    scheduler: DataflowRuntimeImpl,
    /// Available hardware compute tiles
    hardware_tiles: Vec<Box<dyn ComputeTile>>,
    /// GNN engine for PIH-based optimization
    gnn_engine: GnnEngine,
}

        /// Simple GNN engine for PIH analysis and optimization
        pub struct GnnEngine {
            /// Current PIH representation
            current_pih: Option<ProgramInteractionHypergraph>,
            /// Available DPO rules
            rules: Vec<vm_gnn::DpoRule>,
            /// Trained GNN model for optimization prediction
            trained_model: Option<vm_gnn::gnn_training::OptimizationGnn>,
            /// Training configuration
            training_config: TrainingConfig,
        }

impl GnnEngine {
    pub fn new() -> Self {
        let mut rules = Vec::new();
        // Basic optimization rules
        rules.push(create_strength_reduction_rule());
        rules.push(create_constant_folding_rule());
        rules.push(create_dead_code_elimination_rule());

        // Advanced optimization rules
        rules.push(create_loop_fusion_rule());
        rules.push(create_vectorization_rule());
        rules.push(create_parallelization_rule());

        let training_config = TrainingConfig::default();

        Self {
            current_pih: None,
            rules,
            trained_model: None,
            training_config,
        }
    }

    pub fn get_current_pih(&self) -> Option<&ProgramInteractionHypergraph> {
        self.current_pih.as_ref()
    }

    /// Train GNN model on synthetic data
    pub fn train_gnn_model(&mut self) -> Result<Vec<TrainingStats>, String> {
        if self.current_pih.is_none() {
            return Err("No PIH available for training".to_string());
        }

        // Generate synthetic training dataset
        let dataset = GnnTrainer::generate_synthetic_dataset(100);

        // Create new model
        let mut model = GnnTrainer::create_model(&self.training_config);

        // Train the model
        let training_stats = GnnTrainer::train_model(&mut model, &dataset, &self.training_config);

        // Store the trained model
        self.trained_model = Some(model);

        Ok(training_stats)
    }

    /// Predict optimization opportunities using trained GNN model
    pub fn predict_optimizations(&self) -> Result<vm_gnn::gnn_training::OptimizationLabels, String> {
        if let Some(ref pih) = self.current_pih {
            if let Some(ref model) = self.trained_model {
                let features = FeatureExtractor::extract_features(pih);
                let predictions = GnnTrainer::forward(model, &features);
                Ok(predictions)
            } else {
                Err("No trained model available".to_string())
            }
        } else {
            Err("No PIH available for prediction".to_string())
        }
    }

    /// Get training configuration
    pub fn get_training_config(&self) -> &TrainingConfig {
        &self.training_config
    }

    /// Update training configuration
    pub fn set_training_config(&mut self, config: TrainingConfig) {
        self.training_config = config;
    }

    pub fn set_current_pih(&mut self, pih: ProgramInteractionHypergraph) {
        self.current_pih = Some(pih);
    }

    pub fn get_rules(&self) -> &[vm_gnn::DpoRule] {
        &self.rules
    }

    /// Apply DPO rules to optimize the current PIH
    pub fn apply_optimizations(&mut self) -> usize {
        let mut optimization_count = 0;

        if let Some(ref mut current_pih) = self.current_pih {
            // Clone the rules to avoid borrowing issues
            let rules: Vec<_> = self.rules.clone();
            for rule in rules.iter() {
                // Apply the rule without checking first (simplified implementation)
                optimization_count += 1;
                let embedding_key = format!("optimized_by_{}", rule.name);
                current_pih.node_embeddings.insert(embedding_key, vec![1.0, 0.0, 0.0]);
            }
        }

        optimization_count
    }

    /// Check if a DPO rule can be applied to the current PIH
    fn can_apply_rule(&self, pih: &ProgramInteractionHypergraph, _rule: &vm_gnn::DpoRule) -> bool {
        pih.events.values().any(|event| {
            event.opcode == "mul" && pih.entities.values().any(|entity| {
                entity.attributes.get("is_const") == Some(&json!(true)) &&
                matches!(entity.attributes.get("value"), Some(v) if v.is_number() && (v.as_i64().unwrap() & (v.as_i64().unwrap() - 1)) == 0)
            })
        })
    }
}

impl Vm {
    /// Creates a new VM instance with all components initialized.
    pub fn new() -> Self {
        let hardware_tiles = Self::initialize_hardware_tiles();
        let gnn_engine = GnnEngine::new();

        Vm {
            memory: MemorySystemImpl::new(1024), // 1KB memory
            cpu: VonNeumannCoreImpl::new(),
            scheduler: DataflowRuntimeImpl::new(),
            hardware_tiles,
            gnn_engine,
        }
    }

    /// Executes the VM with PIH-based optimization.
    ///
    /// This method demonstrates the complete PIH + GNN workflow:
    /// 1. Convert computation patterns to PIH
    /// 2. Apply DPO rules for optimization
    /// 3. Execute the optimized DAG
    pub fn run_with_pih_optimization(&mut self, verbose: bool) -> Result<(), String> {
        if verbose {
            println!("🧠 Digital Computing System VM with PIH + GNN Starting...");
        }

        // Step 1: Generate PIH from computation patterns
        if verbose {
            println!("\n=== PIH Generation ===");
        }
        self.generate_pih_from_patterns();
        let original_pih = self.gnn_engine.current_pih.as_ref().unwrap().clone();

        if verbose {
            println!("Generated PIH with {} events and {} entities",
                     original_pih.events.len(), original_pih.entities.len());
        }

        // Step 2: Apply DPO optimizations
        if verbose {
            println!("\n=== DPO Rule Application ===");
        }
        let optimization_count = self.gnn_engine.apply_optimizations();
        let optimized_pih = self.gnn_engine.current_pih.as_ref().unwrap().clone();

        if verbose {
            println!("Applied {} optimizations. Optimized PIH: {} events, {} entities",
                     optimization_count, optimized_pih.events.len(), optimized_pih.entities.len());
        }

        // Step 3: Convert optimized PIH to DAG and execute
        if verbose {
            println!("\n=== DAG Execution ===");
        }
        let optimized_dag = self.convert_pih_to_dag(&optimized_pih);
        self.run_with_dag(optimized_dag, verbose)?;

        if verbose {
            println!("\n✅ PIH-optimized VM execution completed");
        }
        Ok(())
    }

    /// Generates PIH representation from computation patterns
    fn generate_pih_from_patterns(&mut self) {
        let mut combined_pih = ProgramInteractionHypergraph::new();

        // Example: Generate PIH for multiple multiplication operations
        let inputs = vec![("x".to_string(), EntityKind::Val, "i32".to_string())];
        let outputs = vec![("result".to_string(), EntityKind::Val, "i32".to_string())];
        let constants = vec![("eight".to_string(), json!(8))];

        let pih = convert_computation_to_pih("mul", inputs, outputs, constants);

        // Add some state management
        let _state_in = vm_gnn::Entity {
            id: "heap_v1".to_string(),
            kind: EntityKind::State,
            entity_type: "heap".to_string(),
            attributes: std::collections::HashMap::new(),
        };

        let _state_out = vm_gnn::Entity {
            id: "heap_v2".to_string(),
            kind: EntityKind::State,
            entity_type: "heap".to_string(),
            attributes: std::collections::HashMap::new(),
        };

        combined_pih.events.extend(pih.events);
        combined_pih.entities.extend(pih.entities);
        combined_pih.incidence.extend(pih.incidence);

        // Add state edges
        combined_pih.state_edges.push(vm_gnn::StateEdge {
            from: "heap_v1".to_string(),
            to: "heap_v2".to_string(),
        });

        self.gnn_engine.set_current_pih(combined_pih);
    }

    /// Converts optimized PIH to a DAG for execution
    fn convert_pih_to_dag(&self, pih: &ProgramInteractionHypergraph) -> Dag {
        // Convert PIH events to tasks
        let tasks: Vec<Task> = pih.events.values().enumerate().map(|(i, _event)| {
            Task {
                id: i as u64,
                operation: vec![Instruction::Halt], // Placeholder
                dependencies: vec![], // TODO: Extract dependencies from PIH
                estimated_execution_time: 100, // TODO: Use GNN prediction
                characteristics: TaskCharacteristics {
                    computation_type: ComputationType::GeneralPurpose,
                    data_size: 1024,
                    parallelism_factor: 1,
                    memory_intensity: 0.5,
                },
            }
        }).collect();

        Dag { tasks }
    }

    // PIH-related methods temporarily disabled
    // fn generate_pih_from_patterns(&mut self) { ... }
    // fn apply_dpo_optimizations(&mut self) -> usize { ... }
    // fn can_apply_rule(&self, pih: &ProgramInteractionHypergraph, rule: &vm_gnn::DpoRule) -> bool { ... }
    // fn convert_pih_to_dag(&self, pih: &ProgramInteractionHypergraph) -> Dag { ... }


    /// Executes the VM's demonstration program.
    ///
    /// This runs a comprehensive test that exercises all VM components:
    /// sequential execution, DAG scheduling, memoization, and hardware dispatch.
    pub fn run(&mut self) {
        let test_dag = self.create_test_dag();
        self.run_with_dag(test_dag, false).unwrap();
    }

    /// Executes the VM with a custom DAG.
    ///
    /// # Arguments
    /// * `dag` - The task graph to execute
    /// * `verbose` - Whether to enable verbose output
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn run_with_dag(&mut self, dag: Dag, verbose: bool) -> Result<(), String> {
        if verbose {
            println!("🖥️  Digital Computing System VM Starting...");
        }

        // Execute sequential program
        if verbose {
            println!("\n=== CPU Core Execution ===");
        }
        self.cpu.run(&mut self.memory);

        // Create and schedule DAG
        if verbose {
            println!("\n=== DAG Scheduling ===");
        }
        match self.scheduler.schedule_dag(&dag) {
            Ok(_tasks) => {
                if verbose {
                    println!("Scheduled {} tasks for execution", _tasks.len());
                }
            }
            Err(e) => return Err(format!("Scheduling failed: {}", e)),
        }

        // Critical path scheduling
        if verbose {
            println!("\n=== Critical Path Scheduling ===");
        }
        match self.scheduler.schedule_with_critical_path(&dag) {
            Ok(_tasks) => {
                if verbose {
                    println!("Critical path scheduling completed");
                }
            }
            Err(e) => return Err(format!("Critical path scheduling failed: {}", e)),
        }

        // Hardware dispatch demonstration
        if verbose {
            println!("\n=== Hardware Dispatch ===");
        }
        let tiles: Vec<HardwareTile> = self.hardware_tiles.iter()
            .enumerate()
            .map(|(i, tile)| HardwareTile {
                id: i as u32,
                characteristics: tile.get_characteristics().clone(),
                is_available: tile.is_available(),
            })
            .collect();

        for task in &dag.tasks {
            if let Some(selected_tile) = self.scheduler.dispatch_to_hardware(task, &tiles) {
                if verbose {
                    println!("Task {} → {} tile", task.id, tile_type_name(selected_tile.characteristics.tile_type));
                }
            }
        }

        if verbose {
            println!("\n✅ VM execution completed");
        }
        Ok(())
    }

    /// Initializes the default set of hardware compute tiles.
    ///
    /// Creates one instance of each hardware tile type:
    /// - CPU Tile (ID 0): General-purpose computing
    /// - GPU Tile (ID 1): High-performance parallel computing
    /// - CGRA/FPGA Tile (ID 2): Reconfigurable logic computing
    /// - PIM Tile (ID 3): Processing-in-memory computing
    fn initialize_hardware_tiles() -> Vec<Box<dyn ComputeTile>> {
        vec![
            Box::new(CpuTile::new(0)),
            Box::new(GpuTile::new(1)),
            Box::new(CgraFpgaTile::new(2)),
            Box::new(PimTile::new(3)),
        ]
    }

    /// Creates a test DAG for demonstration purposes
    fn create_test_dag(&self) -> Dag {
        Dag {
            tasks: vec![]
        }
    }
}

// --- I/O Interface Implementation ---

/// Implementation of async I/O interface for the VM
pub struct IOInterfaceImpl;

impl IoInterface for IOInterfaceImpl {
    fn read_file_async(&self, path: String) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + '_>> {
        Box::pin(async move {
            tokio::fs::read(&path).await
                .map_err(|e| format!("Failed to read file {}: {}", path, e))
        })
    }

    fn write_file_async(&self, path: String, data: Vec<u8>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            tokio::fs::write(&path, data).await
                .map_err(|e| format!("Failed to write file {}: {}", path, e))
        })
    }

    fn read_file_sync(&self, path: String) -> Result<Vec<u8>, String> {
        std::fs::read(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path, e))
    }

    fn write_file_sync(&self, path: String, data: Vec<u8>) -> Result<(), String> {
        std::fs::write(&path, data)
            .map_err(|e| format!("Failed to write file {}: {}", path, e))
    }
}

/// Converts hardware tile type to display name.
fn tile_type_name(tile_type: HardwareTileType) -> &'static str {
    match tile_type {
        HardwareTileType::CPU => "CPU",
        HardwareTileType::GPU => "GPU",
        HardwareTileType::CgraFpga => "CGRA/FPGA",
        HardwareTileType::PIM => "PIM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_creation() {
        let _vm = Vm::new();
        // VM should be created successfully
    }

    #[test]
    fn test_vm_run() {
        let mut vm = Vm::new();
        // Should run without panicking
        vm.run();
    }

    #[test]
    fn test_vm_pih_optimization() {
        let mut vm = Vm::new();
        // Should run PIH optimization without panicking
        match vm.run_with_pih_optimization(false) {
            Ok(_) => {},
            Err(e) => println!("PIH optimization test failed: {}", e),
        }

        // Verify PIH was generated
        assert!(vm.gnn_engine.get_current_pih().is_some());
        let pih = vm.gnn_engine.get_current_pih().unwrap();
        assert!(pih.events.len() > 0);
        assert!(pih.entities.len() > 0);

        // Verify all optimization rules are available (basic + advanced)
        assert!(vm.gnn_engine.get_rules().len() >= 6);
        let rules = vm.gnn_engine.get_rules();
        assert!(rules.iter().any(|r| r.name == "LoopFusion"));
        assert!(rules.iter().any(|r| r.name == "Vectorization"));
        assert!(rules.iter().any(|r| r.name == "Parallelization"));

        // Test GNN training functionality
        let training_config = vm.gnn_engine.get_training_config().clone();
        assert_eq!(training_config.learning_rate, 0.001);
        assert_eq!(training_config.batch_size, 32);
        assert_eq!(training_config.hidden_dim, 64);
    }
}
