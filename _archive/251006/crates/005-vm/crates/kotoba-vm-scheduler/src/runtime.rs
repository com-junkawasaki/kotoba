use std::collections::HashMap;
use kotoba_vm_types::{TaskId, CachedResult, Task, HardwareTile, HardwareTileType, TaskCharacteristics, HardwareCharacteristics, ComputationType};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::memoization::MemoizationEngine;

/// Merkle DAG: vm.ExecutionEngine.DataflowRuntime
/// Dataflow execution runtime for task scheduling and execution
pub trait DataflowRuntime {
    fn execute_task(&mut self, task_id: TaskId) -> Result<CachedResult, String>;
    fn get_task_status(&self, task_id: TaskId) -> TaskStatus;
    fn get_ready_tasks(&self) -> Vec<TaskId>;
    fn submit_task(&mut self, task: Task) -> TaskId;
    fn wait_for_completion(&self, task_id: TaskId) -> Result<CachedResult, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed(String),
}

pub struct DataflowRuntimeImpl {
    tasks: HashMap<TaskId, Task>,
    task_status: HashMap<TaskId, TaskStatus>,
    ready_queue: Vec<TaskId>,
    completed_tasks: HashMap<TaskId, CachedResult>,
    memo_engine: Box<dyn MemoizationEngine>,
    hardware_tiles: Vec<HardwareTile>,
}

impl DataflowRuntimeImpl {
    pub fn new(memo_engine: Box<dyn MemoizationEngine>, hardware_tiles: Vec<HardwareTile>) -> Self {
        DataflowRuntimeImpl {
            tasks: HashMap::new(),
            task_status: HashMap::new(),
            ready_queue: Vec::new(),
            completed_tasks: HashMap::new(),
            memo_engine,
            hardware_tiles,
        }
    }

    /// Check if all dependencies of a task are satisfied
    fn check_dependencies(&self, task_id: TaskId) -> bool {
        if let Some(task) = self.tasks.get(&task_id) {
            for &dep_id in &task.dependencies {
                if let Some(status) = self.task_status.get(&dep_id) {
                    if *status != TaskStatus::Completed {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Update task status and check for newly ready tasks
    fn update_task_status(&mut self, task_id: TaskId, new_status: TaskStatus) {
        self.task_status.insert(task_id, new_status.clone());

        // If task completed, check dependent tasks
        if new_status == TaskStatus::Completed {
            // Find all tasks that depend on this completed task
            for (tid, task) in &self.tasks {
                if task.dependencies.contains(&task_id) {
                    if self.check_dependencies(*tid) && !self.ready_queue.contains(tid) {
                        self.ready_queue.push(*tid);
                    }
                }
            }
        }
    }

    /// Execute a task on available hardware
    fn execute_on_hardware(&self, task: &Task) -> Result<CachedResult, String> {
        // Find suitable hardware tile
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
            return Err(format!("No suitable hardware for task {}", task.id));
        }

        // Select best tile based on load balancing
        let selected_tile = suitable_tiles.iter().min_by_key(|tile| (tile.characteristics.current_load * 1000.0) as i32).unwrap();

        // Simulate task execution
        // In real implementation, this would dispatch to actual hardware
        let execution_time = task.estimated_execution_time;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let result = CachedResult {
            task_hash: task.id,
            result_data: format!("Result of task {}", task.id).into_bytes(),
            timestamp,
        };

        Ok(result)
    }
}

impl DataflowRuntime for DataflowRuntimeImpl {
    fn execute_task(&mut self, task_id: TaskId) -> Result<CachedResult, String> {
        // Check cache first
        if let Some(cached) = self.memo_engine.check_cache(task_id) {
            self.completed_tasks.insert(task_id, cached.clone());
            self.update_task_status(task_id, TaskStatus::Completed);
            return Ok(cached);
        }

        // Check if task exists and is ready
        if let Some(task) = self.tasks.get(&task_id).cloned() {
            if !self.check_dependencies(task_id) {
                return Err("Dependencies not satisfied".to_string());
            }

            // Mark as running
            self.update_task_status(task_id, TaskStatus::Running);

            // Execute on hardware
            match self.execute_on_hardware(&task) {
                Ok(result) => {
                    // Store in cache
                    self.memo_engine.store_result(task_id, result.clone());
                    self.completed_tasks.insert(task_id, result.clone());
                    self.update_task_status(task_id, TaskStatus::Completed);
                    Ok(result)
                }
                Err(e) => {
                    self.update_task_status(task_id, TaskStatus::Failed(e.clone()));
                    Err(e)
                }
            }
        } else {
            Err(format!("Task {} not found", task_id))
        }
    }

    fn get_task_status(&self, task_id: TaskId) -> TaskStatus {
        self.task_status.get(&task_id).cloned().unwrap_or(TaskStatus::Pending)
    }

    fn get_ready_tasks(&self) -> Vec<TaskId> {
        self.ready_queue.clone()
    }

    fn submit_task(&mut self, task: Task) -> TaskId {
        let task_id = self.memo_engine.compute_task_hash(&task);
        self.tasks.insert(task_id, task);
        self.task_status.insert(task_id, TaskStatus::Pending);

        // Check if ready to execute
        if self.check_dependencies(task_id) {
            self.ready_queue.push(task_id);
        }

        task_id
    }

    fn wait_for_completion(&self, task_id: TaskId) -> Result<CachedResult, String> {
        loop {
            match self.get_task_status(task_id) {
                TaskStatus::Completed => {
                    return self.completed_tasks.get(&task_id)
                        .cloned()
                        .ok_or_else(|| "Task completed but result not found".to_string());
                }
                TaskStatus::Failed(e) => {
                    return Err(format!("Task failed: {}", e));
                }
                TaskStatus::Running | TaskStatus::Pending | TaskStatus::Ready => {
                    // Continue waiting
                    std::thread::yield_now();
                }
            }
        }
    }
}

impl Default for DataflowRuntimeImpl {
    fn default() -> Self {
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

        Self::new(memo_engine, hardware_tiles)
    }
}
