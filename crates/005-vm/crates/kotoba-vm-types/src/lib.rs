//! Merkle DAG: vm.types
//! This crate defines the core data structures used throughout the VM.

/// Represents a single instruction for the VonNeumannCore.
/// For simplicity, we'll define a simple RISC-like instruction set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// Load value from memory address into register.
    Load { dest_reg: u8, addr: u64 },
    /// Store value from register to memory address.
    Store { src_reg: u8, addr: u64 },
    /// Add two source registers into a destination register.
    Add { dest_reg: u8, src1_reg: u8, src2_reg: u8 },
    /// Subtract second source register from first into a destination register.
    Sub { dest_reg: u8, src1_reg: u8, src2_reg: u8 },
    /// Jump to a new instruction pointer if register is zero.
    Jz { reg: u8, new_ip: u64 },
    /// Halt execution.
    Halt,
}

/// A unique identifier for a task in the dataflow graph.
pub type TaskId = u64;

/// Represents a single task in a Directed Acyclic Graph (DAG).
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    /// The actual operation to be performed. This could be a sequence of instructions
    /// or a more abstract operation.
    pub operation: Vec<Instruction>,
    /// A list of `TaskId`s that must be completed before this task can start.
    pub dependencies: Vec<TaskId>,
    /// Estimated execution time in arbitrary time units.
    /// This is used for critical path analysis and scheduling.
    pub estimated_execution_time: u64,
    /// Characteristics of this task for hardware selection.
    pub characteristics: TaskCharacteristics,
}

/// Represents a full Directed Acyclic Graph of tasks.
#[derive(Debug, Clone)]
pub struct Dag {
    pub tasks: Vec<Task>,
}

impl Dag {
    /// Get all task IDs in the DAG
    pub fn get_all_task_ids(&self) -> Vec<TaskId> {
        self.tasks.iter().map(|task| task.id).collect()
    }

    /// Get a task by its ID
    pub fn get_task(&self, task_id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == task_id)
    }

    /// Get a mutable reference to a task by its ID
    pub fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == task_id)
    }
}

/// Represents a data packet for the virtual network.
#[derive(Debug, Clone)]
pub struct Packet {
    pub source_tile_id: u32,
    pub dest_tile_id: u32,
    pub payload: Vec<u8>,
}

/// Represents an I/O request from the VM to the host.
#[derive(Debug, Clone)]
pub enum IoRequest {
    Read { path: String },
    Write { path: String, data: Vec<u8> },
}

/// A hash key for memoization cache, computed from task content.
pub type TaskHash = u64;

/// Represents cached computation results for memoization.
#[derive(Debug, Clone, PartialEq)]
pub struct CachedResult {
    pub task_hash: TaskHash,
    pub result_data: Vec<u8>,
    pub timestamp: u64,
}

/// Async I/O interface for non-blocking operations
pub trait IoInterface {
    fn read_file_async(&self, path: String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send + '_>>;
    fn write_file_async(&self, path: String, data: Vec<u8>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>;
    fn read_file_sync(&self, path: String) -> Result<Vec<u8>, String>;
    fn write_file_sync(&self, path: String, data: Vec<u8>) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_debug() {
        let load = Instruction::Load { dest_reg: 1, addr: 100 };
        assert!(format!("{:?}", load).contains("Load"));
    }

    #[test]
    fn test_task_creation() {
        let task = Task {
            id: 42,
            operation: vec![Instruction::Halt],
            dependencies: vec![1, 2, 3],
            estimated_execution_time: 100,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.5,
            },
        };

        assert_eq!(task.id, 42);
        assert_eq!(task.operation.len(), 1);
        assert_eq!(task.dependencies, vec![1, 2, 3]);
        assert_eq!(task.estimated_execution_time, 100);
        assert_eq!(task.characteristics.computation_type, ComputationType::GeneralPurpose);
    }

    #[test]
    fn test_dag_creation() {
        let task = Task {
            id: 1,
            operation: vec![Instruction::Halt],
            dependencies: vec![],
            estimated_execution_time: 50,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 512,
                parallelism_factor: 1,
                memory_intensity: 0.3,
            },
        };

        let dag = Dag { tasks: vec![task] };
        assert_eq!(dag.tasks.len(), 1);
        assert_eq!(dag.tasks[0].id, 1);
    }
}

/// Types of computation that tasks can perform.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComputationType {
    GeneralPurpose,     // General CPU workloads
    HighlyParallel,     // GPU-like parallel processing
    Reconfigurable,     // FPGA/CGRA adaptable logic
    MemoryBound,        // PIM memory-intensive operations
}

/// Characteristics of a computational task for hardware selection.
#[derive(Debug, Clone)]
pub struct TaskCharacteristics {
    pub computation_type: ComputationType,
    pub data_size: usize,           // Size of data to process in bytes
    pub parallelism_factor: u32,    // Degree of parallelism (1 = sequential, >1 = parallel)
    pub memory_intensity: f32,      // 0.0 = compute-bound, 1.0 = memory-bound
}

/// Types of hardware tiles available in the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HardwareTileType {
    CPU,
    GPU,
    CgraFpga,
    PIM,
}

/// Characteristics and capabilities of a hardware tile.
#[derive(Debug, Clone)]
pub struct HardwareCharacteristics {
    pub tile_type: HardwareTileType,
    pub compute_units: u32,         // Number of compute units
    pub memory_bandwidth: u64,      // Memory bandwidth in bytes/sec
    pub power_efficiency: f32,      // Power efficiency rating
    pub current_load: f32,          // Current load (0.0 = idle, 1.0 = fully loaded)
}

/// Represents a hardware tile with its characteristics and state.
#[derive(Debug, Clone)]
pub struct HardwareTile {
    pub id: u32,
    pub characteristics: HardwareCharacteristics,
    pub is_available: bool,
}
