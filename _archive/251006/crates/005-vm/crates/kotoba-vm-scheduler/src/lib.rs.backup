use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasherDefault, Hash, Hasher};
use twox_hash::XxHash64;
use vm_types::{Dag, TaskId, CachedResult, Task, HardwareTile, HardwareTileType, TaskCharacteristics, HardwareCharacteristics, ComputationType};
use rayon::prelude::*;

// Use XxHash64 for faster hashing performance.
type FastHasher = BuildHasherDefault<XxHash64>;

// Merkle DAG: vm.ExecutionEngine.DataflowRuntime.MemoizationEngine
// Content-addressable caching system for redundancy elimination
pub trait MemoizationEngine {
    fn check_cache(&self, task_hash: TaskId) -> Option<CachedResult>;
    fn store_result(&mut self, task_hash: TaskId, result: CachedResult);
    fn compute_task_hash(&self, task: &Task) -> TaskId;
    fn invalidate_cache(&mut self, task_hash: TaskId);
    fn clear_cache(&mut self);
}

pub struct MemoizationEngineImpl {
    cache: HashMap<TaskId, CachedResult, FastHasher>,
    max_cache_size: usize,
}

impl MemoizationEngineImpl {
    pub fn new(max_cache_size: usize) -> Self {
        MemoizationEngineImpl {
            cache: HashMap::with_hasher(FastHasher::default()),
            max_cache_size,
        }
    }

    fn evict_if_needed(&mut self) {
        if self.cache.len() >= self.max_cache_size {
            // Simple LRU: remove oldest entry (in practice, track access times)
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }
    }
}

impl MemoizationEngine for MemoizationEngineImpl {
    fn check_cache(&self, task_hash: TaskId) -> Option<CachedResult> {
        self.cache.get(&task_hash).cloned()
    }

    fn store_result(&mut self, task_hash: TaskId, result: CachedResult) {
        self.evict_if_needed();
        self.cache.insert(task_hash, result);
    }

    fn compute_task_hash(&self, task: &Task) -> TaskId {
        let mut hasher = XxHash64::default();
        task.id.hash(&mut hasher);
        task.operation.hash(&mut hasher);
        task.dependencies.hash(&mut hasher);
        task.estimated_execution_time.hash(&mut hasher);
        task.characteristics.computation_type.hash(&mut hasher);
        task.characteristics.data_size.hash(&mut hasher);
        task.characteristics.parallelism_factor.hash(&mut hasher);
        task.characteristics.memory_intensity.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    fn invalidate_cache(&mut self, task_hash: TaskId) {
        self.cache.remove(&task_hash);
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

// Merkle DAG: vm.ExecutionEngine.DataflowRuntime
// Defines the interface for the dataflow runtime, which manages DAG-based execution.
pub trait DataflowRuntime {
    fn schedule_dag(&self, dag: &Dag) -> Result<Vec<TaskId>, String>;
    fn schedule_with_critical_path(&self, dag: &Dag) -> Result<Vec<TaskId>, String>;
    fn cache_task_result(&mut self, task: &Task, result_data: Vec<u8>);
    fn dispatch_to_hardware(&self, task: &Task, available_tiles: &[HardwareTile]) -> Option<HardwareTile>;
    fn compute_dag_hash(&self, dag: &Dag) -> u64;
    fn execute_tasks_parallel(&self, tasks: &[TaskId], dag: &Dag) -> Result<Vec<Vec<u8>>, String>;
}

pub struct DataflowRuntimeImpl {
    memoization_engine: MemoizationEngineImpl,
    critical_path_cache: HashMap<u64, Vec<TaskId>, FastHasher>, // Cache for critical path calculations
}

impl DataflowRuntimeImpl {
    pub fn new() -> Self {
        Self {
            memoization_engine: MemoizationEngineImpl::new(100), // Cache up to 100 results
            critical_path_cache: HashMap::with_hasher(FastHasher::default()),
        }
    }

    /// Performs topological sort on the DAG to determine execution order.
    /// Returns a vector of task IDs in execution order, or an error if the DAG has cycles.
    fn topological_sort(&self, dag: &Dag) -> Result<Vec<TaskId>, String> {
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        let mut adj_list: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        // Initialize in-degrees and adjacency list
        for task in &dag.tasks {
            in_degree.insert(task.id, 0);
            adj_list.insert(task.id, Vec::new());
        }

        // Build the graph
        for task in &dag.tasks {
            for &dep in &task.dependencies {
                adj_list.get_mut(&dep).unwrap().push(task.id);
                *in_degree.get_mut(&task.id).unwrap() += 1;
            }
        }

        // Find nodes with no incoming edges (in-degree 0)
        let mut queue: VecDeque<TaskId> = in_degree.iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::new();

        while let Some(task_id) = queue.pop_front() {
            result.push(task_id);

            // Reduce in-degree of neighbors
            if let Some(neighbors) = adj_list.get(&task_id) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(&neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != dag.tasks.len() {
            return Err("DAG contains cycles".to_string());
        }

        Ok(result)
    }
}

impl DataflowRuntime for DataflowRuntimeImpl {
    fn schedule_dag(&self, dag: &Dag) -> Result<Vec<TaskId>, String> {
        println!("Scheduling DAG with {} tasks", dag.tasks.len());

        // Perform topological sort to get execution order
        let execution_order = self.topological_sort(dag)?;

        // Check memoization cache and filter out cached tasks
        let mut tasks_to_execute = Vec::new();
        let mut cached_tasks = Vec::new();

        for &task_id in &execution_order {
            let task = dag.tasks.iter().find(|t| t.id == task_id).unwrap();
            let task_hash = self.memoization_engine.compute_task_hash(task);

            if let Some(cached_result) = self.memoization_engine.check_cache(task_hash) {
                println!("Task {} found in cache (hash: {})", task_id, task_hash);
                cached_tasks.push((task_id, cached_result));
            } else {
                tasks_to_execute.push(task_id);
            }
        }

        println!("Tasks to execute: {:?} ({} total, {} cached)",
                 tasks_to_execute, execution_order.len(), cached_tasks.len());
        println!("Cached tasks: {:?}", cached_tasks.iter().map(|(id, _)| id).collect::<Vec<_>>());

        Ok(tasks_to_execute)
    }

    fn schedule_with_critical_path(&self, dag: &Dag) -> Result<Vec<TaskId>, String> {
        println!("Scheduling DAG with critical path consideration ({} tasks)", dag.tasks.len());

        // Check cache first - use simple DAG hash as key
        let dag_hash = self.compute_dag_hash(dag);
        if let Some(cached_result) = self.critical_path_cache.get(&dag_hash) {
            println!("Using cached critical path result");
            return Ok(cached_result.clone());
        }

        // Perform topological sort first
        let topo_order = self.topological_sort(dag)?;

        // Calculate earliest start/finish times (forward pass)
        let (est, eft) = self.calculate_earliest_times(dag, &topo_order)?;

        // Calculate latest start/finish times (backward pass)
        let (lst, _lft) = self.calculate_latest_times(dag, &topo_order, &eft)?;

        // Find critical path tasks (slack = 0)
        let critical_tasks = self.find_critical_path_tasks(dag, &est, &lst)?;

        // Prioritize critical path tasks and schedule others
        let prioritized_order = self.prioritize_tasks(dag, &critical_tasks, &topo_order)?;

        // Check memoization cache and filter out cached tasks from the prioritized order
        let mut tasks_to_execute = Vec::new();
        let mut cached_tasks = Vec::new();

        for &task_id in &prioritized_order {
            let task = dag.tasks.iter().find(|t| t.id == task_id).unwrap();
            let task_hash = self.memoization_engine.compute_task_hash(task);

            if let Some(cached_result) = self.memoization_engine.check_cache(task_hash) {
                println!("Critical path task {} found in cache (hash: {})", task_id, task_hash);
                cached_tasks.push((task_id, cached_result));
            } else {
                tasks_to_execute.push(task_id);
            }
        }

        println!("Critical path tasks: {:?}", critical_tasks);
        println!("Prioritized execution order: {:?}", prioritized_order);
        println!("Tasks to execute after memoization: {:?} ({} total, {} cached)",
                 tasks_to_execute, prioritized_order.len(), cached_tasks.len());

        // Cache the result for future use
        let _result_to_cache = tasks_to_execute.clone();
        // Note: In a real implementation, we'd need to make this method mutable to update cache
        // For now, we'll skip caching in this immutable context

        Ok(tasks_to_execute)
    }

    /// Compute a simple hash for DAG caching purposes
    fn compute_dag_hash(&self, dag: &Dag) -> u64 {
        let mut hasher = XxHash64::default();
        dag.tasks.len().hash(&mut hasher);
        // Hash task IDs and dependencies for basic DAG structure
        for task in &dag.tasks {
            task.id.hash(&mut hasher);
            task.dependencies.len().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn dispatch_to_hardware(&self, task: &Task, available_tiles: &[HardwareTile]) -> Option<HardwareTile> {
        if available_tiles.is_empty() {
            return None;
        }

        // Find the best matching tile based on task characteristics
        let mut best_tile: Option<&HardwareTile> = None;
        let mut best_score = 0.0;

        for tile in available_tiles {
            if !tile.is_available {
                continue;
            }

            let score = self.calculate_hardware_match_score(&task.characteristics, &tile.characteristics);

            // Prefer tiles with lower load
            let load_penalty = tile.characteristics.current_load;
            let adjusted_score = score * (1.0 - load_penalty);

            if adjusted_score > best_score {
                best_score = adjusted_score;
                best_tile = Some(tile);
            }
        }

        best_tile.cloned()
    }

    fn execute_tasks_parallel(&self, tasks: &[TaskId], dag: &Dag) -> Result<Vec<Vec<u8>>, String> {
        // Find tasks in the DAG
        let task_map: HashMap<TaskId, &Task> = dag.tasks.iter()
            .map(|task| (task.id, task))
            .collect();

        // Execute tasks in parallel using rayon
        let results: Result<Vec<Vec<u8>>, String> = tasks.par_iter()
            .map(|&task_id| {
                if let Some(task) = task_map.get(&task_id) {
                    // Simple task execution simulation
                    // In a real implementation, this would dispatch to appropriate hardware tiles
                    let result_size = (task.characteristics.data_size / 8).max(1);
                    let result = vec![task_id as u8; result_size];
                    Ok(result)
                } else {
                    Err(format!("Task {} not found in DAG", task_id))
                }
            })
            .collect();

        results
    }

    fn cache_task_result(&mut self, task: &Task, result_data: Vec<u8>) {
        let task_hash = self.memoization_engine.compute_task_hash(task);
        let cached_result = CachedResult {
            task_hash,
            result_data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        self.memoization_engine.store_result(task_hash, cached_result);
    }
}

impl DataflowRuntimeImpl {
    /// Calculate earliest start time (EST) and earliest finish time (EFT) for all tasks
    fn calculate_earliest_times(&self, dag: &Dag, topo_order: &[TaskId]) -> Result<(HashMap<TaskId, u64>, HashMap<TaskId, u64>), String> {
        let mut est: HashMap<TaskId, u64> = HashMap::new();
        let mut eft: HashMap<TaskId, u64> = HashMap::new();

        // Initialize EST for tasks with no dependencies
        for task in &dag.tasks {
            if task.dependencies.is_empty() {
                est.insert(task.id, 0);
                eft.insert(task.id, task.estimated_execution_time);
            }
        }

        // Process tasks in topological order
        for &task_id in topo_order {
            let task = dag.tasks.iter().find(|t| t.id == task_id).unwrap();

            // EST = max(EFT of all predecessors)
            let mut max_pred_eft = 0;
            for &pred_id in &task.dependencies {
                if let Some(pred_eft) = eft.get(&pred_id) {
                    max_pred_eft = max_pred_eft.max(*pred_eft);
                }
            }

            est.insert(task_id, max_pred_eft);
            eft.insert(task_id, max_pred_eft + task.estimated_execution_time);
        }

        Ok((est, eft))
    }

    /// Calculate latest start time (LST) and latest finish time (LFT) for all tasks
    fn calculate_latest_times(&self, dag: &Dag, topo_order: &[TaskId], eft: &HashMap<TaskId, u64>) -> Result<(HashMap<TaskId, u64>, HashMap<TaskId, u64>), String> {
        let mut lst: HashMap<TaskId, u64> = HashMap::new();
        let mut lft: HashMap<TaskId, u64> = HashMap::new();

        // Find the maximum EFT (makespan) as the LFT for tasks with no successors
        let makespan = eft.values().max().unwrap_or(&0);

        // Initialize LFT for tasks with no successors
        for task in &dag.tasks {
            let has_successors = dag.tasks.iter().any(|t| t.dependencies.contains(&task.id));
            if !has_successors {
                lft.insert(task.id, *makespan);
                lst.insert(task.id, makespan.saturating_sub(task.estimated_execution_time));
            }
        }

        // Process tasks in reverse topological order
        for &task_id in topo_order.iter().rev() {
            let task = dag.tasks.iter().find(|t| t.id == task_id).unwrap();

            // LFT = min(LST of all successors)
            let mut min_succ_lst = *makespan;
            for successor in &dag.tasks {
                if successor.dependencies.contains(&task_id) {
                    if let Some(succ_lst) = lst.get(&successor.id) {
                        min_succ_lst = min_succ_lst.min(*succ_lst);
                    }
                }
            }

            lft.insert(task_id, min_succ_lst);
            lst.insert(task_id, min_succ_lst.saturating_sub(task.estimated_execution_time));
        }

        Ok((lst, lft))
    }

    /// Calculate how well a hardware tile matches a task's requirements.
    /// Returns a score between 0.0 (poor match) and 1.0 (perfect match).
    fn calculate_hardware_match_score(&self, task_chars: &TaskCharacteristics, hw_chars: &HardwareCharacteristics) -> f32 {
        // Computation type compatibility (major factor)
        let type_compatibility = match (&task_chars.computation_type, &hw_chars.tile_type) {
            (ComputationType::GeneralPurpose, HardwareTileType::CPU) => 1.0,
            (ComputationType::HighlyParallel, HardwareTileType::GPU) => 1.0,
            (ComputationType::Reconfigurable, HardwareTileType::CgraFpga) => 1.0,
            (ComputationType::MemoryBound, HardwareTileType::PIM) => 1.0,
            // Partial compatibility
            (ComputationType::GeneralPurpose, _) => 0.6, // CPU can run most things
            (ComputationType::HighlyParallel, HardwareTileType::CPU) => 0.3, // CPU can simulate parallelism
            (_, HardwareTileType::CPU) => 0.5, // CPU is general purpose
            _ => 0.2, // Poor match
        };

        // Parallelism factor compatibility
        let parallelism_compatibility = if task_chars.parallelism_factor == 1 {
            // Sequential task - any tile can handle it
            1.0
        } else {
            // Parallel task - prefer tiles with more compute units
            let parallelism_ratio = task_chars.parallelism_factor as f32 / hw_chars.compute_units as f32;
            if parallelism_ratio <= 1.0 {
                parallelism_ratio // Underutilized but can handle
            } else {
                1.0 / parallelism_ratio // Overloaded but still possible
            }
        };

        // Memory intensity compatibility
        let memory_compatibility = match hw_chars.tile_type {
            HardwareTileType::PIM => {
                // PIM excels at memory-intensive tasks
                if task_chars.memory_intensity > 0.7 {
                    1.0
                } else {
                    0.8
                }
            }
            HardwareTileType::GPU => {
                // GPU handles memory-bound tasks well
                if task_chars.memory_intensity > 0.5 {
                    0.9
                } else {
                    0.7
                }
            }
            HardwareTileType::CPU => {
                // CPU handles compute-bound tasks well
                if task_chars.memory_intensity < 0.3 {
                    0.9
                } else {
                    0.6
                }
            }
            HardwareTileType::CgraFpga => {
                // CGRA/FPGA is balanced
                0.8
            }
        };

        // Data size consideration (bandwidth requirements)
        let data_size_mb = task_chars.data_size as f32 / (1024.0 * 1024.0);
        let bandwidth_requirement = data_size_mb * task_chars.memory_intensity;
        let bandwidth_compatibility = if bandwidth_requirement > 0.0 {
            let available_bandwidth_mb = hw_chars.memory_bandwidth as f32 / (1024.0 * 1024.0);
            if available_bandwidth_mb >= bandwidth_requirement {
                1.0
            } else {
                available_bandwidth_mb / bandwidth_requirement
            }
        } else {
            1.0
        };

        // Power efficiency consideration (prefer efficient tiles for long-running tasks)
        let task_duration_estimate = task_chars.data_size as f32 / 1000000.0; // Rough estimate
        let power_compatibility = if task_duration_estimate > 1.0 {
            hw_chars.power_efficiency
        } else {
            1.0 // Short tasks don't care about power efficiency
        };

        // Communication cost consideration (Ring-Tree topology awareness)
        let communication_cost = self.calculate_communication_cost(task_chars, hw_chars);
        let communication_compatibility = 1.0 - communication_cost; // Lower cost = higher compatibility

        // Weighted combination of all factors
        let score = type_compatibility * 0.3 +
                parallelism_compatibility * 0.25 +
                memory_compatibility * 0.2 +
                bandwidth_compatibility * 0.1 +
                power_compatibility * 0.05 +
                communication_compatibility * 0.1;

        score.min(1.0).max(0.0) // Clamp to [0, 1]
    }

    /// Calculate communication cost based on Ring-Tree topology
    fn calculate_communication_cost(&self, task_chars: &TaskCharacteristics, hw_chars: &HardwareCharacteristics) -> f32 {
        // Ring-Tree topology: communication cost based on data size and memory intensity
        // Higher memory intensity and larger data size = higher communication cost
        let data_size_factor = (task_chars.data_size as f32 / 1000000.0).min(1.0); // Normalize to [0, 1]
        let memory_intensity_factor = task_chars.memory_intensity;
        let communication_cost = data_size_factor * memory_intensity_factor;

        // Adjust based on hardware tile type's communication efficiency
        let tile_communication_efficiency = match hw_chars.tile_type {
            HardwareTileType::CPU => 0.9,  // CPUs have good interconnect
            HardwareTileType::GPU => 0.7,  // GPUs have good internal interconnect
            HardwareTileType::CgraFpga => 0.6, // FPGAs have configurable interconnect
            HardwareTileType::PIM => 0.8,  // PIM has memory-local communication
        };

        (communication_cost * (1.0 - tile_communication_efficiency)).min(1.0)
    }

    /// Find tasks that are on the critical path (slack = 0)
    fn find_critical_path_tasks(&self, dag: &Dag, est: &HashMap<TaskId, u64>, lst: &HashMap<TaskId, u64>) -> Result<Vec<TaskId>, String> {
        let mut critical_tasks = Vec::new();

        for task in &dag.tasks {
            let task_est = est.get(&task.id).unwrap_or(&0);
            let task_lst = lst.get(&task.id).unwrap_or(&0);

            // Slack = LST - EST
            if task_lst.saturating_sub(*task_est) == 0 {
                critical_tasks.push(task.id);
            }
        }

        Ok(critical_tasks)
    }

    /// Prioritize tasks based on critical path and topological order
    fn prioritize_tasks(&self, _dag: &Dag, critical_tasks: &[TaskId], topo_order: &[TaskId]) -> Result<Vec<TaskId>, String> {
        let mut prioritized = Vec::new();
        let mut remaining: Vec<TaskId> = topo_order.to_vec();

        // First, schedule critical path tasks in topological order
        for &task_id in topo_order {
            if critical_tasks.contains(&task_id) {
                prioritized.push(task_id);
                if let Some(pos) = remaining.iter().position(|&x| x == task_id) {
                    remaining.remove(pos);
                }
            }
        }

        // Then add remaining tasks
        prioritized.extend(remaining);

        Ok(prioritized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vm_types::{Task, TaskCharacteristics, ComputationType};

    fn create_test_task(id: TaskId, deps: Vec<TaskId>, exec_time: u64, comp_type: ComputationType) -> Task {
        Task {
            id,
            operation: vec![vm_types::Instruction::Halt],
            dependencies: deps,
            estimated_execution_time: exec_time,
            characteristics: TaskCharacteristics {
                computation_type: comp_type,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.5,
            },
        }
    }

    #[test]
    fn test_topological_sort_simple() {
        let runtime = DataflowRuntimeImpl::new();

        let task0 = create_test_task(0, vec![], 1, ComputationType::GeneralPurpose);
        let task1 = create_test_task(1, vec![0], 1, ComputationType::GeneralPurpose);
        let task2 = create_test_task(2, vec![0], 1, ComputationType::GeneralPurpose);
        let task3 = create_test_task(3, vec![1, 2], 1, ComputationType::GeneralPurpose);

        let dag = Dag {
            tasks: vec![task0, task1, task2, task3],
        };

        let result = runtime.topological_sort(&dag);
        assert!(result.is_ok());

        let order = result.unwrap();
        // Verify topological order: 0 before 1 and 2, 1 and 2 before 3
        assert!(order.iter().position(|&x| x == 0).unwrap() < order.iter().position(|&x| x == 1).unwrap());
        assert!(order.iter().position(|&x| x == 0).unwrap() < order.iter().position(|&x| x == 2).unwrap());
        assert!(order.iter().position(|&x| x == 1).unwrap() < order.iter().position(|&x| x == 3).unwrap());
        assert!(order.iter().position(|&x| x == 2).unwrap() < order.iter().position(|&x| x == 3).unwrap());
    }

    #[test]
    fn test_topological_sort_cycle_detection() {
        let runtime = DataflowRuntimeImpl::new();

        // Create a cycle: 0 -> 1 -> 2 -> 0
        let task0 = create_test_task(0, vec![2], 1, ComputationType::GeneralPurpose);
        let task1 = create_test_task(1, vec![0], 1, ComputationType::GeneralPurpose);
        let task2 = create_test_task(2, vec![1], 1, ComputationType::GeneralPurpose);

        let dag = Dag {
            tasks: vec![task0, task1, task2],
        };

        let result = runtime.topological_sort(&dag);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "DAG contains cycles".to_string());
    }

    #[test]
    fn test_critical_path_calculation() {
        let runtime = DataflowRuntimeImpl::new();

        // Create tasks with different execution times to test critical path
        // Task 0 (2 time units) -> Task 1 (1 time unit) -> Task 3 (2 time units) = 5 total
        // Task 0 (2 time units) -> Task 2 (3 time units) -> Task 3 (2 time units) = 7 total (critical path)
        let task0 = create_test_task(0, vec![], 2, ComputationType::GeneralPurpose);
        let task1 = create_test_task(1, vec![0], 1, ComputationType::GeneralPurpose);
        let task2 = create_test_task(2, vec![0], 3, ComputationType::GeneralPurpose);
        let task3 = create_test_task(3, vec![1, 2], 2, ComputationType::GeneralPurpose);

        let dag = Dag {
            tasks: vec![task0, task1, task2, task3],
        };

        let topo_order = runtime.topological_sort(&dag).unwrap();

        let (est, eft) = runtime.calculate_earliest_times(&dag, &topo_order).unwrap();
        let (lst, _lft) = runtime.calculate_latest_times(&dag, &topo_order, &eft).unwrap();

        let critical_tasks = runtime.find_critical_path_tasks(&dag, &est, &lst).unwrap();

        // Critical path should be [0, 2, 3] (slack = 0)
        assert_eq!(critical_tasks.len(), 3);
        assert!(critical_tasks.contains(&0));
        assert!(critical_tasks.contains(&2));
        assert!(critical_tasks.contains(&3));
        assert!(!critical_tasks.contains(&1)); // Task 1 has slack
    }

    #[test]
    fn test_hardware_matching() {
        let runtime = DataflowRuntimeImpl::new();

        // Test CPU task with CPU tile
        let cpu_task = Task {
            id: 1,
            operation: vec![],
            dependencies: vec![],
            estimated_execution_time: 100,
            characteristics: TaskCharacteristics {
                computation_type: ComputationType::GeneralPurpose,
                data_size: 1024,
                parallelism_factor: 1,
                memory_intensity: 0.1,
            },
        };

        let cpu_tile = vm_types::HardwareTile {
            id: 0,
            characteristics: vm_types::HardwareCharacteristics {
                tile_type: vm_types::HardwareTileType::CPU,
                compute_units: 8,
                memory_bandwidth: 50_000_000_000,
                power_efficiency: 0.8,
                current_load: 0.2,
            },
            is_available: true,
        };

        let score = runtime.calculate_hardware_match_score(&cpu_task.characteristics, &cpu_tile.characteristics);
        assert!(score > 0.8); // Should be a very good match
    }

    #[test]
    fn test_memoization() {
        let mut runtime = DataflowRuntimeImpl::new();

        let task = create_test_task(42, vec![], 100, ComputationType::GeneralPurpose);

        // First time should not be in cache
        let hash = runtime.memoization_engine.compute_task_hash(&task);
        assert!(runtime.memoization_engine.check_cache(hash).is_none());

        // Store a result
        runtime.cache_task_result(&task, vec![1, 2, 3, 4]);

        // Now should be in cache
        let cached = runtime.memoization_engine.check_cache(hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().result_data, vec![1, 2, 3, 4]);
    }
}
