use kotoba_vm_types::{Task, HardwareCharacteristics, HardwareTileType, ComputationType};

// Merkle DAG: vm.VirtualHardware
// A common trait for any computational tile.
pub trait ComputeTile {
    /// Execute a computational task on this tile.
    fn execute_task(&mut self, task: &Task) -> Result<Vec<u8>, String>;

    /// Get the current hardware characteristics of this tile.
    fn get_characteristics(&self) -> &HardwareCharacteristics;

    /// Get the tile type.
    fn get_tile_type(&self) -> HardwareTileType;

    /// Check if the tile is available for task execution.
    fn is_available(&self) -> bool;

    /// Get current load (0.0 = idle, 1.0 = fully loaded).
    fn get_current_load(&self) -> f32;

    /// Update the tile's load after task execution.
    fn update_load(&mut self, new_load: f32);
}

// Merkle DAG: vm.VirtualHardware.CPU_Tile
/// CPU Tile - General purpose computing tile optimized for balanced performance across workloads.
pub struct CpuTile {
    pub id: u32,
    pub characteristics: HardwareCharacteristics,
    pub current_load: f32,
}

impl CpuTile {
    /// Create a new CPU tile with the given ID.
    pub fn new(id: u32) -> Self {
        CpuTile {
            id,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::CPU,
                compute_units: 8,        // 8 cores
                memory_bandwidth: 50_000, // 50 GB/s
                power_efficiency: 0.8,   // Good efficiency
                current_load: 0.0,
            },
            current_load: 0.0,
        }
    }

    /// Calculate execution time based on task characteristics for CPU tile.
    fn calculate_execution_time(&self, task: &Task) -> u64 {
        let base_time = task.estimated_execution_time;

        // CPU is balanced but not specialized for any particular type
        let type_multiplier = match task.characteristics.computation_type {
            ComputationType::GeneralPurpose => 1.0,      // Optimal for CPU
            ComputationType::HighlyParallel => 1.5,     // CPU can handle but not optimal
            ComputationType::Reconfigurable => 2.0,     // CPU not great for this
            ComputationType::MemoryBound => 1.2,        // Decent memory performance
        };

        // Factor in parallelism and data size
        let parallelism_factor = (task.characteristics.parallelism_factor as f64).sqrt();
        let data_factor = (task.characteristics.data_size as f64 / 1_000_000.0).max(1.0);

        ((base_time as f64) * type_multiplier / parallelism_factor * data_factor.sqrt()) as u64
    }

    /// Generate pseudo-random result data based on task ID.
    fn generate_result(&self, task: &Task) -> Vec<u8> {
        let result_size = (task.characteristics.data_size / 10).max(4);
        (0..result_size)
            .map(|i| ((task.id * 10 + i as u64) % 256) as u8)
            .collect()
    }

    /// Calculate load impact based on execution time.
    fn calculate_load_impact(&self, execution_time: u64) -> f32 {
        // CPU load increases moderately with execution time
        let impact = (execution_time as f32 / 1000.0).min(0.3);
        (self.current_load + impact).min(1.0)
    }
}

impl ComputeTile for CpuTile {
    fn execute_task(&mut self, task: &Task) -> Result<Vec<u8>, String> {
        if !self.is_available() {
            return Err(format!("CPU Tile {} is not available (load: {:.2})", self.id, self.current_load));
        }

        let execution_time = self.calculate_execution_time(task);
        let result = self.generate_result(task);
        let new_load = self.calculate_load_impact(execution_time);

        self.update_load(new_load);

        println!("✅ CPU Tile {} executed task {} in {} cycles, new load: {:.2}",
                self.id, task.id, execution_time, new_load);

        Ok(result)
    }

    fn get_characteristics(&self) -> &HardwareCharacteristics {
        &self.characteristics
    }

    fn get_tile_type(&self) -> HardwareTileType {
        HardwareTileType::CPU
    }

    fn is_available(&self) -> bool {
        self.current_load < 0.9 // Available if load < 90%
    }

    fn get_current_load(&self) -> f32 {
        self.current_load
    }

    fn update_load(&mut self, new_load: f32) {
        self.current_load = new_load.max(0.0).min(1.0);
        self.characteristics.current_load = self.current_load;
    }
}

impl CgraFpgaTile {
    /// Create a new CGRA/FPGA tile with the given ID.
    pub fn new(id: u32) -> Self {
        CgraFpgaTile {
            id,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::CgraFpga,
                compute_units: 64,       // Specialized processing elements
                memory_bandwidth: 100_000, // 100 GB/s
                power_efficiency: 0.9,   // Very efficient
                current_load: 0.0,
            },
            current_load: 0.0,
            reconfiguration_overhead: 1000, // Significant reconfiguration time
        }
    }

    /// Calculate execution time including reconfiguration overhead for CGRA/FPGA tile.
    fn calculate_execution_time(&self, task: &Task) -> u64 {
        let base_time = task.estimated_execution_time;

        // CGRA/FPGA excels at reconfigurable tasks but has reconfiguration overhead
        let (type_multiplier, reconfiguration_needed) = match task.characteristics.computation_type {
            ComputationType::GeneralPurpose => (1.8, true),    // Can handle but not optimal
            ComputationType::HighlyParallel => (1.2, false),  // Decent at parallel tasks
            ComputationType::Reconfigurable => (0.2, true),   // Excellent for reconfigurable logic
            ComputationType::MemoryBound => (1.5, true),      // Can be optimized for memory patterns
        };

        // Add reconfiguration overhead if needed
        let reconfiguration_cost = if reconfiguration_needed {
            self.reconfiguration_overhead
        } else {
            0
        };

        // Factor in task complexity
        let complexity_factor = (task.characteristics.parallelism_factor as f64).sqrt();
        let data_factor = (task.characteristics.data_size as f64 / 500_000.0).max(1.0);

        reconfiguration_cost + (base_time as f64 * type_multiplier / complexity_factor * data_factor) as u64
    }

    /// Generate result data for CGRA/FPGA tile (optimized for specific algorithms).
    fn generate_result(&self, task: &Task) -> Vec<u8> {
        let result_size = (task.characteristics.data_size / 12).max(4);
        (0..result_size)
            .map(|i| ((task.id * 50 + i as u64) % 256) as u8)
            .collect()
    }

    /// Calculate load impact - reconfiguration creates temporary high load.
    fn calculate_load_impact(&self, execution_time: u64, reconfiguration_needed: bool) -> f32 {
        let base_impact = execution_time as f32 / 2000.0; // Efficient execution

        // Reconfiguration creates burst load
        let reconfiguration_impact = if reconfiguration_needed {
            0.2 // Temporary load spike during reconfiguration
        } else {
            0.0
        };

        (self.current_load + base_impact + reconfiguration_impact).min(1.0)
    }
}

impl ComputeTile for CgraFpgaTile {
    fn execute_task(&mut self, task: &Task) -> Result<Vec<u8>, String> {
        if !self.is_available() {
            return Err(format!("CGRA/FPGA Tile {} is not available (load: {:.2})", self.id, self.current_load));
        }

        let reconfiguration_needed = matches!(task.characteristics.computation_type,
            ComputationType::Reconfigurable | ComputationType::MemoryBound);

        let execution_time = self.calculate_execution_time(task);
        let result = self.generate_result(task);
        let new_load = self.calculate_load_impact(execution_time, reconfiguration_needed);

        self.update_load(new_load);

        let reconfig_msg = if reconfiguration_needed { " (with reconfiguration)" } else { "" };
        println!("🔧 CGRA/FPGA Tile {} executed task {} in {} cycles{}, new load: {:.2}",
                self.id, task.id, execution_time, reconfig_msg, new_load);

        Ok(result)
    }

    fn get_characteristics(&self) -> &HardwareCharacteristics {
        &self.characteristics
    }

    fn get_tile_type(&self) -> HardwareTileType {
        HardwareTileType::CgraFpga
    }

    fn is_available(&self) -> bool {
        self.current_load < 0.85 // CGRA/FPGA prefers lower utilization for stability
    }

    fn get_current_load(&self) -> f32 {
        self.current_load
    }

    fn update_load(&mut self, new_load: f32) {
        self.current_load = new_load.max(0.0).min(1.0);
        self.characteristics.current_load = self.current_load;
    }
}

// Merkle DAG: vm.VirtualHardware.PIM_Tile
/// PIM Tile - Processing-In-Memory tile with integrated memory for ultra-low latency data processing.
pub struct PimTile {
    pub id: u32,
    pub characteristics: HardwareCharacteristics,
    pub current_load: f32,
    pub integrated_memory: Vec<u8>, // Integrated memory for PIM operations
}

impl PimTile {
    /// Create a new PIM tile with the given ID.
    pub fn new(id: u32) -> Self {
        PimTile {
            id,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::PIM,
                compute_units: 16,       // Specialized PIM processing units
                memory_bandwidth: 10_000_000, // Effectively unlimited (integrated)
                power_efficiency: 0.95,  // Extremely efficient
                current_load: 0.0,
            },
            current_load: 0.0,
            integrated_memory: vec![0; 1_000_000], // 1MB integrated memory
        }
    }

    /// Calculate execution time for PIM tile - extremely fast for memory-bound operations.
    fn calculate_execution_time(&self, task: &Task) -> u64 {
        let base_time = task.estimated_execution_time;

        // PIM excels at memory-bound tasks due to integrated processing
        let type_multiplier = match task.characteristics.computation_type {
            ComputationType::GeneralPurpose => 1.5,      // Can handle but memory advantage limited
            ComputationType::HighlyParallel => 2.0,     // PIM not optimized for parallelism
            ComputationType::Reconfigurable => 1.8,     // Limited reconfiguration capability
            ComputationType::MemoryBound => 0.05,      // Near-zero latency for memory operations
        };

        // Memory intensity greatly affects PIM performance
        let memory_intensity_factor = task.characteristics.memory_intensity as f64;
        let memory_speedup = if memory_intensity_factor > 0.7 {
            1.0 / memory_intensity_factor // Massive speedup for memory-intensive tasks
        } else {
            1.0
        };

        // Data size consideration (PIM has limited integrated memory)
        let data_factor = (task.characteristics.data_size as f64 / self.integrated_memory.len() as f64).max(0.1);

        ((base_time as f64) * type_multiplier * memory_speedup * data_factor) as u64
    }

    /// Generate result data for PIM tile with integrated memory operations.
    fn generate_result(&self, task: &Task) -> Vec<u8> {
        let result_size = (task.characteristics.data_size / 20).max(4); // PIM can be very efficient
        (0..result_size)
            .map(|i| ((task.id * 25 + i as u64) % 256) as u8)
            .collect()
    }

    /// Calculate load impact - PIM has minimal load increase due to efficiency.
    fn calculate_load_impact(&self, execution_time: u64, task: &Task) -> f32 {
        // PIM is very efficient, minimal load impact
        let base_impact = execution_time as f32 / 5000.0;

        // Memory-intensive tasks have even lower impact due to integrated processing
        let memory_factor = if task.characteristics.memory_intensity > 0.8 {
            0.5 // Reduced impact for memory-bound tasks
        } else {
            1.0
        };

        (self.current_load + base_impact * memory_factor).min(1.0)
    }

    /// Simulate PIM memory operation on integrated memory.
    fn perform_memory_operation(&mut self, task: &Task) {
        // Simulate memory-intensive operation on integrated memory
        let memory_size = self.integrated_memory.len();
        let operation_range = (task.characteristics.data_size as usize).min(memory_size);

        // Perform some memory operations (just simulation)
        for i in 0..operation_range.min(1000) {
            let addr = i % memory_size;
            self.integrated_memory[addr] = (self.integrated_memory[addr] + task.id as u8) & 0xFF;
        }
    }
}

impl ComputeTile for PimTile {
    fn execute_task(&mut self, task: &Task) -> Result<Vec<u8>, String> {
        if !self.is_available() {
            return Err(format!("PIM Tile {} is not available (load: {:.2})", self.id, self.current_load));
        }

        // Check if task fits in integrated memory
        if task.characteristics.data_size > self.integrated_memory.len() as usize {
            return Err(format!("Task data size ({}) exceeds PIM integrated memory capacity ({})",
                    task.characteristics.data_size, self.integrated_memory.len()));
        }

        let execution_time = self.calculate_execution_time(task);

        // Perform memory operation
        self.perform_memory_operation(task);

        let result = self.generate_result(task);
        let new_load = self.calculate_load_impact(execution_time, task);

        self.update_load(new_load);

        println!("🧠 PIM Tile {} executed task {} in {} cycles (memory-bound), new load: {:.2}",
                self.id, task.id, execution_time, new_load);

        Ok(result)
    }

    fn get_characteristics(&self) -> &HardwareCharacteristics {
        &self.characteristics
    }

    fn get_tile_type(&self) -> HardwareTileType {
        HardwareTileType::PIM
    }

    fn is_available(&self) -> bool {
        self.current_load < 0.8 // PIM can handle moderate loads efficiently
    }

    fn get_current_load(&self) -> f32 {
        self.current_load
    }

    fn update_load(&mut self, new_load: f32) {
        self.current_load = new_load.max(0.0).min(1.0);
        self.characteristics.current_load = self.current_load;
    }
}

// Merkle DAG: vm.VirtualHardware.GPU_Tile
/// GPU Tile - High-performance parallel computing tile optimized for massively parallel workloads.
pub struct GpuTile {
    pub id: u32,
    pub characteristics: HardwareCharacteristics,
    pub current_load: f32,
}

// Merkle DAG: vm.VirtualHardware.CGRA_FPGA_Tile
/// CGRA/FPGA Tile - Reconfigurable computing tile that can be optimized for specific algorithms.
pub struct CgraFpgaTile {
    pub id: u32,
    pub characteristics: HardwareCharacteristics,
    pub current_load: f32,
    pub reconfiguration_overhead: u64, // Cycles needed to reconfigure
}

impl GpuTile {
    /// Create a new GPU tile with the given ID.
    pub fn new(id: u32) -> Self {
        GpuTile {
            id,
            characteristics: HardwareCharacteristics {
                tile_type: HardwareTileType::GPU,
                compute_units: 1024,     // Many cores
                memory_bandwidth: 1_000_000, // 1 TB/s HBM
                power_efficiency: 0.6,   // Less efficient than CPU
                current_load: 0.0,
            },
            current_load: 0.0,
        }
    }

    /// Calculate execution time based on task characteristics for GPU tile.
    fn calculate_execution_time(&self, task: &Task) -> u64 {
        let base_time = task.estimated_execution_time;

        // GPU excels at highly parallel tasks
        let type_multiplier = match task.characteristics.computation_type {
            ComputationType::GeneralPurpose => 2.0,      // GPU not ideal for general tasks
            ComputationType::HighlyParallel => 0.3,     // Excellent for parallel workloads
            ComputationType::Reconfigurable => 1.5,     // Can handle but not specialized
            ComputationType::MemoryBound => 0.8,        // Good memory bandwidth helps
        };

        // Factor in parallelism (GPU benefits greatly from high parallelism)
        let parallelism_factor = task.characteristics.parallelism_factor as f64;
        let parallelism_bonus = if parallelism_factor > 100.0 {
            parallelism_factor.log2() / 10.0 // Massive parallelism bonus
        } else {
            1.0
        };

        // Data size affects performance (GPU handles large datasets well)
        let data_factor = (task.characteristics.data_size as f64 / 1_000_000.0).max(1.0);

        ((base_time as f64) * type_multiplier / parallelism_bonus * data_factor.sqrt()) as u64
    }

    /// Generate result data for GPU tile (potentially larger due to parallel processing).
    fn generate_result(&self, task: &Task) -> Vec<u8> {
        let result_size = (task.characteristics.data_size / 8).max(8); // GPU can handle larger outputs
        (0..result_size)
            .map(|i| ((task.id * 100 + i as u64) % 256) as u8)
            .collect()
    }

    /// Calculate load impact - GPU load increases significantly with parallel tasks.
    fn calculate_load_impact(&self, execution_time: u64, task: &Task) -> f32 {
        let base_impact = execution_time as f32 / 500.0; // GPU is fast but power-hungry

        // Highly parallel tasks create more load
        let parallelism_factor = if task.characteristics.parallelism_factor > 50 {
            1.5
        } else {
            1.0
        };

        (self.current_load + base_impact * parallelism_factor).min(1.0)
    }
}

impl ComputeTile for GpuTile {
    fn execute_task(&mut self, task: &Task) -> Result<Vec<u8>, String> {
        if !self.is_available() {
            return Err(format!("GPU Tile {} is not available (load: {:.2})", self.id, self.current_load));
        }

        let execution_time = self.calculate_execution_time(task);
        let result = self.generate_result(task);
        let new_load = self.calculate_load_impact(execution_time, task);

        self.update_load(new_load);

        println!("🚀 GPU Tile {} executed task {} in {} cycles, new load: {:.2}",
                self.id, task.id, execution_time, new_load);

        Ok(result)
    }

    fn get_characteristics(&self) -> &HardwareCharacteristics {
        &self.characteristics
    }

    fn get_tile_type(&self) -> HardwareTileType {
        HardwareTileType::GPU
    }

    fn is_available(&self) -> bool {
        self.current_load < 0.95 // GPU can handle higher loads but becomes unstable
    }

    fn get_current_load(&self) -> f32 {
        self.current_load
    }

    fn update_load(&mut self, new_load: f32) {
        self.current_load = new_load.max(0.0).min(1.0);
        self.characteristics.current_load = self.current_load;
    }
}
