//! Memory Pool Implementation
//!
//! Efficient memory pooling for reduced allocation overhead:
//! - Object pooling for frequently allocated types
//! - Slab allocation for small objects
//! - Arena allocation for temporary data
//! - Custom allocation strategies

use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

/// Memory pool for efficient allocations
pub struct MemoryPool {
    slabs: Mutex<HashMap<usize, SlabAllocator>>,
    arenas: Mutex<Vec<Arena>>,
    total_capacity: usize,
    used_memory: Mutex<usize>,
}

impl MemoryPool {
    /// Create a new memory pool with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            slabs: Mutex::new(HashMap::new()),
            arenas: Mutex::new(Vec::new()),
            total_capacity: capacity,
            used_memory: Mutex::new(0),
        }
    }

    /// Allocate memory from the pool
    pub fn allocate(&self, size: usize) -> Result<MemoryBlock, Box<dyn std::error::Error>> {
        let mut used_memory = self.used_memory.lock().unwrap();

        if *used_memory + size > self.total_capacity {
            return Err("Memory pool capacity exceeded".into());
        }

        // Try to allocate from existing slab
        if let Some(slab) = self.slabs.lock().unwrap().get_mut(&size) {
            if let Some(block) = slab.allocate() {
                *used_memory += size;
                return Ok(MemoryBlock {
                    ptr: block,
                    size,
                    pool: Some(Arc::new(self.clone())),
                });
            }
        }

        // Create new slab if needed
        let mut slabs = self.slabs.lock().unwrap();
        slabs.entry(size).or_insert_with(|| SlabAllocator::new(size, 1024)); // 1024 objects per slab

        if let Some(block) = slabs.get_mut(&size).unwrap().allocate() {
            *used_memory += size;
            Ok(MemoryBlock {
                ptr: block,
                size,
                pool: Some(Arc::new(self.clone())),
            })
        } else {
            Err("Failed to allocate from memory pool".into())
        }
    }

    /// Create a new arena for temporary allocations
    pub fn create_arena(&self, initial_capacity: usize) -> ArenaHandle {
        let mut arenas = self.arenas.lock().unwrap();
        let arena = Arena::new(initial_capacity);
        let index = arenas.len();
        arenas.push(arena);

        ArenaHandle {
            pool: Arc::new(self.clone()),
            arena_index: index,
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let slabs = self.slabs.lock().unwrap();
        let arenas = self.arenas.lock().unwrap();
        let used_memory = *self.used_memory.lock().unwrap();

        let total_allocated = slabs.values().map(|slab| slab.allocated_count()).sum::<usize>();
        let total_available = slabs.values().map(|slab| slab.available_count()).sum::<usize>();

        PoolStats {
            total_capacity: self.total_capacity,
            used_memory,
            available_memory: self.total_capacity - used_memory,
            utilization: used_memory as f64 / self.total_capacity as f64,
            slab_count: slabs.len(),
            arena_count: arenas.len(),
            total_allocated,
            total_available,
            allocation_efficiency: if total_allocated > 0 {
                total_available as f64 / total_allocated as f64
            } else {
                1.0
            },
        }
    }

    /// Analyze pool usage patterns
    pub fn analyze(&self) -> PoolAnalysis {
        let stats = self.stats();
        let slabs = self.slabs.lock().unwrap();

        let mut size_distribution = HashMap::new();
        for (size, slab) in slabs.iter() {
            size_distribution.insert(*size, slab.allocated_count());
        }

        let top_sizes: Vec<_> = size_distribution.iter()
            .map(|(size, count)| (*size, *count))
            .collect();
        let mut sorted_sizes = top_sizes.clone();
        sorted_sizes.sort_by(|a, b| b.1.cmp(&a.1));

        PoolAnalysis {
            stats,
            size_distribution,
            top_allocation_sizes: sorted_sizes.into_iter().take(10).collect(),
            fragmentation_ratio: self.calculate_fragmentation(),
            recommendations: self.generate_recommendations(),
        }
    }

    /// Deallocate memory back to pool
    fn deallocate(&self, ptr: NonNull<u8>, size: usize) {
        let mut used_memory = self.used_memory.lock().unwrap();
        *used_memory = used_memory.saturating_sub(size);

        // Return to appropriate slab
        if let Some(slab) = self.slabs.lock().unwrap().get_mut(&size) {
            slab.deallocate(ptr);
        }
    }

    /// Calculate memory fragmentation
    fn calculate_fragmentation(&self) -> f64 {
        let slabs = self.slabs.lock().unwrap();
        if slabs.is_empty() {
            return 0.0;
        }

        let total_allocated: usize = slabs.values().map(|slab| slab.allocated_count() * slab.object_size).sum();
        let total_used: usize = slabs.values().map(|slab| slab.used_count() * slab.object_size).sum();

        if total_allocated == 0 {
            0.0
        } else {
            1.0 - (total_used as f64 / total_allocated as f64)
        }
    }

    /// Generate optimization recommendations
    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let stats = self.stats();

        if stats.utilization < 0.3 {
            recommendations.push("Low pool utilization detected. Consider reducing pool size.".to_string());
        }

        if stats.allocation_efficiency < 0.7 {
            recommendations.push("Low allocation efficiency. Consider adjusting slab sizes.".to_string());
        }

        let fragmentation = self.calculate_fragmentation();
        if fragmentation > 0.3 {
            recommendations.push("High memory fragmentation. Consider defragmentation or different allocation strategy.".to_string());
        }

        recommendations
    }
}

impl Clone for MemoryPool {
    fn clone(&self) -> Self {
        // Note: This creates a new pool with the same capacity
        // In practice, you might want to share the actual pool
        Self::new(self.total_capacity)
    }
}

/// Memory block allocated from pool
pub struct MemoryBlock {
    ptr: NonNull<u8>,
    size: usize,
    pool: Option<Arc<MemoryPool>>,
}

impl MemoryBlock {
    /// Get pointer to the allocated memory
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Get slice to the allocated memory
    pub fn as_slice(&self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Get the size of the allocation
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for MemoryBlock {
    fn drop(&mut self) {
        if let Some(ref pool) = self.pool {
            pool.deallocate(self.ptr, self.size);
        } else {
            // Fallback to system deallocation
            unsafe {
                dealloc(self.ptr.as_ptr(), Layout::from_size_align(self.size, 8).unwrap());
            }
        }
    }
}

/// Slab allocator for fixed-size objects
struct SlabAllocator {
    object_size: usize,
    objects_per_slab: usize,
    slabs: Vec<Slab>,
}

struct Slab {
    memory: NonNull<u8>,
    free_list: Vec<NonNull<u8>>,
    allocated_count: usize,
}

impl SlabAllocator {
    fn new(object_size: usize, objects_per_slab: usize) -> Self {
        Self {
            object_size,
            objects_per_slab,
            slabs: Vec::new(),
        }
    }

    fn allocate(&mut self) -> Option<NonNull<u8>> {
        // Try to find free object in existing slabs
        for slab in self.slabs.iter_mut() {
            if let Some(ptr) = slab.free_list.pop() {
                slab.allocated_count += 1;
                return Some(ptr);
            }
        }

        // Allocate new slab
        self.allocate_new_slab()?;
        self.slabs.last_mut()?.allocate()
    }

    fn deallocate(&mut self, ptr: NonNull<u8>) {
        for slab in self.slabs.iter_mut() {
            if slab.contains(ptr) {
                slab.free_list.push(ptr);
                slab.allocated_count -= 1;
                break;
            }
        }
    }

    fn allocate_new_slab(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let slab_size = self.object_size * self.objects_per_slab;
        let layout = Layout::from_size_align(slab_size, 8)?;
        let memory = unsafe { alloc(layout) };

        if memory.is_null() {
            return Err("Failed to allocate slab memory".into());
        }

        let mut free_list = Vec::new();
        for i in 0..self.objects_per_slab {
            let offset = i * self.object_size;
            let ptr = unsafe { NonNull::new(memory.add(offset)) };
            if let Some(ptr) = ptr {
                free_list.push(ptr);
            }
        }

        self.slabs.push(Slab {
            memory: NonNull::new(memory).unwrap(),
            free_list,
            allocated_count: 0,
        });

        Ok(())
    }

    fn allocated_count(&self) -> usize {
        self.slabs.iter().map(|slab| slab.allocated_count).sum()
    }

    fn available_count(&self) -> usize {
        self.slabs.iter().map(|slab| slab.free_list.len()).sum()
    }

    fn used_count(&self) -> usize {
        self.allocated_count()
    }
}

impl Slab {
    fn allocate(&mut self) -> Option<NonNull<u8>> {
        self.free_list.pop().map(|ptr| {
            self.allocated_count += 1;
            ptr
        })
    }

    fn contains(&self, ptr: NonNull<u8>) -> bool {
        let start = self.memory.as_ptr() as usize;
        let end = start + (1024 * 64); // Assume max slab size for simplicity
        let ptr_addr = ptr.as_ptr() as usize;
        ptr_addr >= start && ptr_addr < end
    }
}

impl Drop for SlabAllocator {
    fn drop(&mut self) {
        for slab in self.slabs.drain(..) {
            unsafe {
                dealloc(slab.memory.as_ptr(), Layout::from_size_align(
                    self.object_size * self.objects_per_slab, 8
                ).unwrap());
            }
        }
    }
}

/// Arena allocator for temporary allocations
pub struct Arena {
    memory: Vec<u8>,
    offset: usize,
}

impl Arena {
    fn new(capacity: usize) -> Self {
        Self {
            memory: vec![0; capacity],
            offset: 0,
        }
    }

    pub fn allocate(&mut self, size: usize) -> Result<NonNull<u8>, Box<dyn std::error::Error>> {
        if self.offset + size > self.memory.len() {
            return Err("Arena out of memory".into());
        }

        let ptr = &mut self.memory[self.offset] as *mut u8;
        self.offset += size;

        Ok(NonNull::new(ptr).unwrap())
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }

    pub fn used_memory(&self) -> usize {
        self.offset
    }

    pub fn total_capacity(&self) -> usize {
        self.memory.len()
    }
}

/// Handle to an arena for safe access
pub struct ArenaHandle {
    pool: Arc<MemoryPool>,
    arena_index: usize,
}

impl ArenaHandle {
    pub fn allocate(&self, size: usize) -> Result<ArenaBlock, Box<dyn std::error::Error>> {
        let arenas = self.pool.arenas.lock().unwrap();
        if let Some(arena) = arenas.get(self.arena_index) {
            // Note: This is simplified - in practice you'd need mutable access
            Err("Arena allocation not fully implemented".into())
        } else {
            Err("Invalid arena index".into())
        }
    }

    pub fn reset(&self) {
        if let Some(arena) = self.pool.arenas.lock().unwrap().get_mut(self.arena_index) {
            arena.reset();
        }
    }
}

/// Memory block from arena allocation
pub struct ArenaBlock {
    ptr: NonNull<u8>,
    size: usize,
    arena: ArenaHandle,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_capacity: usize,
    pub used_memory: usize,
    pub available_memory: usize,
    pub utilization: f64,
    pub slab_count: usize,
    pub arena_count: usize,
    pub total_allocated: usize,
    pub total_available: usize,
    pub allocation_efficiency: f64,
}

/// Pool analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAnalysis {
    pub stats: PoolStats,
    pub size_distribution: HashMap<usize, usize>,
    pub top_allocation_sizes: Vec<(usize, usize)>,
    pub fragmentation_ratio: f64,
    pub recommendations: Vec<String>,
}

/// Convenience functions for memory pool usage
pub fn create_pool(capacity_mb: usize) -> MemoryPool {
    MemoryPool::new(capacity_mb * 1024 * 1024)
}

pub fn allocate_from_pool(pool: &MemoryPool, size: usize) -> Result<MemoryBlock, Box<dyn std::error::Error>> {
    pool.allocate(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_allocation() {
        let pool = MemoryPool::new(1024 * 1024); // 1MB pool

        // Allocate some memory
        let block = pool.allocate(1024).unwrap();
        assert_eq!(block.size(), 1024);

        // Check stats
        let stats = pool.stats();
        assert_eq!(stats.used_memory, 1024);
        assert!(stats.utilization > 0.0);

        // Block should be automatically deallocated on drop
        drop(block);

        let stats_after = pool.stats();
        assert_eq!(stats_after.used_memory, 0);
    }

    #[test]
    fn test_pool_capacity_limits() {
        let pool = MemoryPool::new(4096); // 4KB pool

        // This should work
        let _block1 = pool.allocate(2048).unwrap();

        // This should fail
        let result = pool.allocate(4096);
        assert!(result.is_err());
    }
}
