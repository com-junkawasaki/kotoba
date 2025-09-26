use std::collections::HashMap;
use kotoba_vm_types::{Dag, TaskId, CachedResult, Task, HardwareTile, HardwareTileType, TaskCharacteristics, HardwareCharacteristics, ComputationType};
use crate::runtime::DataflowRuntime;
use crate::memoization::MemoizationEngine;

/// Merkle DAG: vm.ExecutionEngine.Scheduler
/// Advanced task scheduler with hardware-aware scheduling policies
pub trait Scheduler {
    fn schedule_dag(&mut self, dag: &Dag) -> Result<Schedule, String>;
    fn get_schedule_status(&self) -> ScheduleStatus;
    fn optimize_schedule(&mut self) -> Result<(), String>;
    fn get_performance_metrics(&self) -> PerformanceMetrics;
}

#[derive(Debug, Clone)]
pub enum ScheduleStatus {
    NotScheduled,
    Scheduling,
    Scheduled,
    Executing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub dag_id: String,
    pub task_assignments: HashMap<TaskId, HardwareTile>,
    pub execution_order: Vec<TaskId>,
    pub estimated_completion_time: f64,
    pub total_power_consumption: f64,
    pub resource_utilization: f64,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub throughput: f64,
    pub latency: f64,
    pub resource_efficiency: f64,
    pub power_efficiency: f64,
    pub load_balance: f64,
}

pub struct HardwareAwareScheduler {
    runtime: Box<dyn DataflowRuntime>,
    memo_engine: Box<dyn MemoizationEngine>,
    hardware_tiles: Vec<HardwareTile>,
    current_schedule: Option<Schedule>,
    scheduling_policy: SchedulingPolicy,
}

#[derive(Debug, Clone)]
pub enum SchedulingPolicy {
    /// Load balancing: assign tasks to least loaded hardware
    LoadBalancing,
    /// Performance optimized: assign tasks to fastest suitable hardware
    PerformanceOptimized,
    /// Power efficient: assign tasks to most power-efficient hardware
    PowerEfficient,
    /// Deadline aware: prioritize tasks with deadlines
    DeadlineAware,
}

impl HardwareAwareScheduler {
    pub fn new(runtime: Box<dyn DataflowRuntime>, memo_engine: Box<dyn MemoizationEngine>, hardware_tiles: Vec<HardwareTile>, policy: SchedulingPolicy) -> Self {
        HardwareAwareScheduler {
            runtime,
            memo_engine,
            hardware_tiles,
            current_schedule: None,
            scheduling_policy: policy,
        }
    }

    /// Find the best hardware tile for a task based on the scheduling policy
    fn select_best_hardware(&self, task: &Task) -> Option<&HardwareTile> {
        let suitable_tiles: Vec<_> = self.hardware_tiles.iter()
            .filter(|tile| {
                match task.characteristics.computation_type {
                    ComputationType::GeneralPurpose => matches!(tile.characteristics.tile_type, HardwareTileType::CPU),
                    ComputationType::HighlyParallel => matches!(tile.characteristics.tile_type, HardwareTileType::GPU),
                    ComputationType::Reconfigurable => matches!(tile.characteristics.tile_type, HardwareTileType::CgraFpga),
                    ComputationType::MemoryBound => matches!(tile.characteristics.tile_type, HardwareTileType::PIM),
                }
            })
            .collect();

        if suitable_tiles.is_empty() {
            return None;
        }

        match self.scheduling_policy {
            SchedulingPolicy::LoadBalancing => {
                // Select tile with minimum current load
                suitable_tiles.iter().min_by_key(|tile| (tile.characteristics.current_load * 1000.0) as i32).copied()
            }
            SchedulingPolicy::PerformanceOptimized => {
                // Select tile with highest compute units
                suitable_tiles.iter().max_by_key(|tile| tile.characteristics.compute_units).copied()
            }
            SchedulingPolicy::PowerEfficient => {
                // Select tile with lowest power consumption per compute unit
                suitable_tiles.iter().min_by_key(|tile| {
                    (tile.characteristics.power_efficiency * 1000.0) as i32
                }).copied()
            }
            SchedulingPolicy::DeadlineAware => {
                // Select tile with lowest latency for deadline-critical tasks
                suitable_tiles.iter().min_by_key(|tile| (tile.characteristics.current_load * 1000.0) as i32).copied()
            }
        }
    }

    /// Estimate execution time for a task on a hardware tile
    fn estimate_execution_time(&self, task: &Task, tile: &HardwareTile) -> f64 {
        // Base execution time
        let base_time = task.estimated_execution_time as f64;

        // Adjust for hardware characteristics
        let hardware_factor = match tile.characteristics.tile_type {
            HardwareTileType::CPU => 1.0,
            HardwareTileType::GPU => 0.25, // GPU is faster for parallelizable tasks
            HardwareTileType::CgraFpga => 0.1, // CGRA/FPGA is very fast for specialized workloads
            HardwareTileType::PIM => 0.5, // PIM varies
        };

        // Current load factor (more load = slower execution)
        let load_factor = 1.0 + (tile.characteristics.current_load as f64 * 0.1);

        base_time * hardware_factor * load_factor
    }

    /// Calculate total power consumption for a schedule
    fn calculate_power_consumption(&self, assignments: &HashMap<TaskId, &HardwareTile>) -> f64 {
        assignments.values().map(|tile| (tile.characteristics.power_efficiency as f64) * 10.0).sum()
    }

    /// Optimize task ordering for better cache locality and reduced communication
    fn optimize_task_order(&self, dag: &Dag) -> Vec<TaskId> {
        // Topological sort with optimizations
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        // Simple topological sort (in practice, use more sophisticated algorithms)
        for task_id in dag.get_all_task_ids() {
            if !visited.contains(&task_id) {
                self.dfs_visit(task_id, dag, &mut visited, &mut order);
            }
        }

        order.reverse(); // Reverse to get correct topological order
        order
    }

    fn dfs_visit(&self, task_id: TaskId, dag: &Dag, visited: &mut std::collections::HashSet<TaskId>, order: &mut Vec<TaskId>) {
        visited.insert(task_id);

        // Visit dependencies first
        if let Some(task) = dag.get_task(task_id) {
            for &dep_id in &task.dependencies {
                if !visited.contains(&dep_id) {
                    self.dfs_visit(dep_id, dag, visited, order);
                }
            }
        }

        order.push(task_id);
    }
}

impl Scheduler for HardwareAwareScheduler {
    fn schedule_dag(&mut self, dag: &Dag) -> Result<Schedule, String> {
        let mut assignments = HashMap::new();
        let mut execution_order = self.optimize_task_order(dag);
        let mut total_completion_time = 0.0;

        // Schedule each task
        for &task_id in &execution_order {
            if let Some(task) = dag.get_task(task_id) {
                if let Some(tile) = self.select_best_hardware(task) {
                    // Update hardware load
                    let execution_time = self.estimate_execution_time(task, tile);
                    total_completion_time += execution_time;

                    assignments.insert(task_id, tile.clone());
                } else {
                    return Err(format!("No suitable hardware for task {}", task_id));
                }
            }
        }

        let total_power = self.calculate_power_consumption(&assignments.iter()
            .map(|(&tid, tile)| (tid, tile))
            .collect());

        let resource_utilization = assignments.len() as f64 / (self.hardware_tiles.len() * 10) as f64; // Rough estimate

        let schedule = Schedule {
            dag_id: "dag_1".to_string(), // TODO: Generate unique ID
            task_assignments: assignments.iter().map(|(&tid, tile)| (tid, tile.clone())).collect(),
            execution_order,
            estimated_completion_time: total_completion_time,
            total_power_consumption: total_power,
            resource_utilization,
        };

        self.current_schedule = Some(schedule.clone());
        Ok(schedule)
    }

    fn get_schedule_status(&self) -> ScheduleStatus {
        match &self.current_schedule {
            None => ScheduleStatus::NotScheduled,
            Some(_) => ScheduleStatus::Scheduled,
        }
    }

    fn optimize_schedule(&mut self) -> Result<(), String> {
        if let Some(schedule) = &self.current_schedule {
            // Re-optimize the schedule (placeholder for advanced optimization)
            // In practice, this could involve genetic algorithms, linear programming, etc.
            Ok(())
        } else {
            Err("No schedule to optimize".to_string())
        }
    }

    fn get_performance_metrics(&self) -> PerformanceMetrics {
        match &self.current_schedule {
            Some(schedule) => {
                let throughput = schedule.task_assignments.len() as f64 / schedule.estimated_completion_time;
                let latency = schedule.estimated_completion_time;
                let resource_efficiency = schedule.resource_utilization;
                let power_efficiency = throughput / schedule.total_power_consumption;
                let load_balance = 0.8; // Placeholder for load balance calculation

                PerformanceMetrics {
                    throughput,
                    latency,
                    resource_efficiency,
                    power_efficiency,
                    load_balance,
                }
            }
            None => PerformanceMetrics {
                throughput: 0.0,
                latency: 0.0,
                resource_efficiency: 0.0,
                power_efficiency: 0.0,
                load_balance: 0.0,
            }
        }
    }

}


impl Default for HardwareAwareScheduler {
    fn default() -> Self {
        let runtime = Box::new(crate::runtime::DataflowRuntimeImpl::default());
        let memo_engine = Box::new(crate::memoization::MemoizationEngineImpl::default());
        let hardware_tiles = vec![
            HardwareTile {
                id: 0,
                characteristics: HardwareCharacteristics {
                    tile_type: HardwareTileType::CPU,
                    compute_units: 4,
                    memory_bandwidth: 25,
                    power_efficiency: 0.8,
                    current_load: 0.0,
                },
                is_available: true,
            },
            HardwareTile {
                id: 1,
                characteristics: HardwareCharacteristics {
                    tile_type: HardwareTileType::GPU,
                    compute_units: 8,
                    memory_bandwidth: 50,
                    power_efficiency: 0.6,
                    current_load: 0.0,
                },
                is_available: true,
            },
            HardwareTile {
                id: 2,
                characteristics: HardwareCharacteristics {
                    tile_type: HardwareTileType::CgraFpga,
                    compute_units: 16,
                    memory_bandwidth: 100,
                    power_efficiency: 0.9,
                    current_load: 0.0,
                },
                is_available: true,
            },
        ];

        Self::new(runtime, memo_engine, hardware_tiles, SchedulingPolicy::LoadBalancing)
    }
}
