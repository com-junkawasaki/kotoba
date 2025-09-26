use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use twox_hash::XxHash64;
use kotoba_vm_types::{TaskId, CachedResult, Task};

// Use XxHash64 for faster hashing performance.
type FastHasher = BuildHasherDefault<XxHash64>;

/// Merkle DAG: vm.ExecutionEngine.DataflowRuntime.MemoizationEngine
/// Content-addressable caching system for redundancy elimination
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

    /// Compute a hash for a task based on its operation and dependencies
    pub fn hash_task(&self, task: &Task) -> TaskId {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        task.id.hash(&mut hasher);
        task.operation.hash(&mut hasher);
        task.dependencies.hash(&mut hasher);
        hasher.finish()
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
        self.hash_task(task)
    }

    fn invalidate_cache(&mut self, task_hash: TaskId) {
        self.cache.remove(&task_hash);
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for MemoizationEngineImpl {
    fn default() -> Self {
        Self::new(1000) // Default cache size of 1000 entries
    }
}
