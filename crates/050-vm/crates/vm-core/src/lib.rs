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
use std::future::Future;
use std::pin::Pin;

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
}

impl Vm {
    /// Creates a new VM instance with all components initialized.
    pub fn new() -> Self {
        let hardware_tiles = Self::initialize_hardware_tiles();

        Vm {
            memory: MemorySystemImpl::new(1024), // 1KB memory
            cpu: VonNeumannCoreImpl::new(),
            scheduler: DataflowRuntimeImpl::new(),
            hardware_tiles,
        }
    }

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

    /// Creates a test DAG for demonstration purposes with various task types.
    ///
    /// This creates tasks with different computational characteristics to test
    /// hardware dispatch to appropriate tiles:
    /// - Task 0: General purpose (should go to CPU)
    /// - Task 1: Highly parallel (should go to GPU)
    /// - Task 2: Memory bound (should go to PIM)
    /// - Task 3: Reconfigurable (should go to CGRA/FPGA)
    fn create_test_dag(&self) -> Dag {
        let task0 = Task {
            id: 0,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 100,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.5,
            },
        };

        let task1 = Task {
            id: 1,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 200,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::HighlyParallel,
                data_size: 65536, // Large dataset
                parallelism_factor: 64, // High parallelism
                memory_intensity: 0.3,
            },
        };

        let task2 = Task {
            id: 2,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 50,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::MemoryBound,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.9, // Very memory intensive
            },
        };

        let task3 = Task {
            id: 3,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 150,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::Reconfigurable,
                data_size: 4096,
                parallelism_factor: 8,
                memory_intensity: 0.4,
            },
        };

        Dag { tasks: vec![task0, task1, task2, task3] }
    }
}

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
}
