//! Custom Memory Allocators
//!
//! Custom allocator implementations for improved memory performance:
//! - Jemalloc allocator wrapper
//! - Mimalloc allocator wrapper
//! - Custom arena-based allocator
//! - Performance monitoring allocators

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::NonNull;
use serde::{Deserialize, Serialize};

/// Trait for custom allocators
pub trait Allocator: Send + Sync {
    /// Allocate memory with the given layout
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError>;

    /// Deallocate memory at the given pointer with the given layout
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);

    /// Try to grow the allocation at ptr from old_layout to new_layout
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<u8>, AllocError> {
        // Default implementation: allocate new and copy
        let new_ptr = self.allocate(new_layout)?;
        std::ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), old_layout.size());
        self.deallocate(ptr, old_layout);
        Ok(new_ptr)
    }

    /// Try to shrink the allocation at ptr from old_layout to new_layout
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<u8>, AllocError> {
        // Default implementation: allocate new and copy
        let new_ptr = self.allocate(new_layout)?;
        std::ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), new_layout.size());
        self.deallocate(ptr, old_layout);
        Ok(new_ptr)
    }

    /// Get allocator statistics
    fn stats(&self) -> AllocatorStats;
}

/// Allocation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocError {
    /// Out of memory
    OutOfMemory,
    /// Alignment is invalid
    InvalidAlignment,
    /// Size is invalid
    InvalidSize,
}

// Removed unstable AllocError implementation - using stable error handling instead

/// Allocator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocatorStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub bytes_allocated: u64,
    pub bytes_deallocated: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_time_ns: u64,
    pub deallocation_time_ns: u64,
}

/// Jemalloc allocator wrapper
#[cfg(feature = "jemalloc")]
pub struct JemallocAllocator {
    stats: std::sync::Mutex<AllocatorStats>,
}

#[cfg(feature = "jemalloc")]
impl JemallocAllocator {
    pub fn new() -> Self {
        Self {
            stats: std::sync::Mutex::new(AllocatorStats {
                allocations: 0,
                deallocations: 0,
                bytes_allocated: 0,
                bytes_deallocated: 0,
                current_usage: 0,
                peak_usage: 0,
                allocation_time_ns: 0,
                deallocation_time_ns: 0,
            }),
        }
    }
}

#[cfg(feature = "jemalloc")]
impl Allocator for JemallocAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let start = std::time::Instant::now();

        // Use tikv-jemallocator
        let result = unsafe { tikv_jemallocator::Jemalloc.allocate(layout) };

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.allocation_time_ns += elapsed;

        match result {
            Ok(ptr) => {
                stats.allocations += 1;
                stats.bytes_allocated += layout.size() as u64;
                stats.current_usage += layout.size() as u64;
                stats.peak_usage = stats.peak_usage.max(stats.current_usage);
                Ok(ptr)
            }
            Err(_) => Err(AllocError::OutOfMemory),
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let start = std::time::Instant::now();

        tikv_jemallocator::Jemalloc.deallocate(ptr, layout);

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.deallocations += 1;
        stats.bytes_deallocated += layout.size() as u64;
        stats.current_usage = stats.current_usage.saturating_sub(layout.size() as u64);
        stats.deallocation_time_ns += elapsed;
    }

    fn stats(&self) -> AllocatorStats {
        self.stats.lock().unwrap().clone()
    }
}

/// Mimalloc allocator wrapper
#[cfg(feature = "mimalloc")]
pub struct MimallocAllocator {
    stats: std::sync::Mutex<AllocatorStats>,
}

#[cfg(feature = "mimalloc")]
impl MimallocAllocator {
    pub fn new() -> Self {
        Self {
            stats: std::sync::Mutex::new(AllocatorStats {
                allocations: 0,
                deallocations: 0,
                bytes_allocated: 0,
                bytes_deallocated: 0,
                current_usage: 0,
                peak_usage: 0,
                allocation_time_ns: 0,
                deallocation_time_ns: 0,
            }),
        }
    }
}

#[cfg(feature = "mimalloc")]
impl Allocator for MimallocAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let start = std::time::Instant::now();

        // Use mimalloc
        let result = unsafe { mimalloc::MiMalloc.allocate(layout) };

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.allocation_time_ns += elapsed;

        match result {
            Ok(ptr) => {
                stats.allocations += 1;
                stats.bytes_allocated += layout.size() as u64;
                stats.current_usage += layout.size() as u64;
                stats.peak_usage = stats.peak_usage.max(stats.current_usage);
                Ok(ptr)
            }
            Err(_) => Err(AllocError::OutOfMemory),
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let start = std::time::Instant::now();

        mimalloc::MiMalloc.deallocate(ptr, layout);

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.deallocations += 1;
        stats.bytes_deallocated += layout.size() as u64;
        stats.current_usage = stats.current_usage.saturating_sub(layout.size() as u64);
        stats.deallocation_time_ns += elapsed;
    }

    fn stats(&self) -> AllocatorStats {
        self.stats.lock().unwrap().clone()
    }
}

/// Custom arena-based allocator
pub struct CustomAllocator {
    arena: std::sync::Mutex<Arena>,
    stats: std::sync::Mutex<AllocatorStats>,
}

struct Arena {
    memory: Vec<u8>,
    offset: usize,
    allocations: Vec<Allocation>,
}

struct Allocation {
    offset: usize,
    size: usize,
    layout: Layout,
}

impl CustomAllocator {
    pub fn new() -> Self {
        Self {
            arena: std::sync::Mutex::new(Arena {
                memory: Vec::new(),
                offset: 0,
                allocations: Vec::new(),
            }),
            stats: std::sync::Mutex::new(AllocatorStats {
                allocations: 0,
                deallocations: 0,
                bytes_allocated: 0,
                bytes_deallocated: 0,
                current_usage: 0,
                peak_usage: 0,
                allocation_time_ns: 0,
                deallocation_time_ns: 0,
            }),
        }
    }

    /// Reset the arena (deallocate all)
    pub fn reset(&self) {
        let mut arena = self.arena.lock().unwrap();
        arena.offset = 0;
        arena.allocations.clear();

        let mut stats = self.stats.lock().unwrap();
        stats.deallocations += stats.allocations;
        stats.bytes_deallocated += stats.current_usage;
        stats.current_usage = 0;
        stats.allocations = 0;
        stats.bytes_allocated = 0;
    }
}

impl Allocator for CustomAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let start = std::time::Instant::now();
        let mut arena = self.arena.lock().unwrap();

        // Align the offset
        let aligned_offset = (arena.offset + layout.align() - 1) & !(layout.align() - 1);

        // Check if we need to grow the arena
        if aligned_offset + layout.size() > arena.memory.len() {
            let new_size = (aligned_offset + layout.size()).max(arena.memory.len() * 2).max(4096);
            arena.memory.resize(new_size, 0);
        }

        if aligned_offset + layout.size() > arena.memory.len() {
            return Err(AllocError::OutOfMemory);
        }

        let ptr = unsafe { NonNull::new_unchecked(arena.memory.as_mut_ptr().add(aligned_offset)) };
        arena.allocations.push(Allocation {
            offset: aligned_offset,
            size: layout.size(),
            layout,
        });
        arena.offset = aligned_offset + layout.size();

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.allocations += 1;
        stats.bytes_allocated += layout.size() as u64;
        stats.current_usage += layout.size() as u64;
        stats.peak_usage = stats.peak_usage.max(stats.current_usage);
        stats.allocation_time_ns += elapsed;

        Ok(ptr)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        // Arena allocator doesn't deallocate individual blocks
        // Use reset() to deallocate all at once
        let start = std::time::Instant::now();
        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.deallocation_time_ns += elapsed;
    }

    fn stats(&self) -> AllocatorStats {
        self.stats.lock().unwrap().clone()
    }
}

/// Performance monitoring allocator wrapper
pub struct MonitoringAllocator<A: Allocator> {
    inner: A,
    stats: std::sync::Mutex<AllocatorStats>,
}

impl<A: Allocator> MonitoringAllocator<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            stats: std::sync::Mutex::new(AllocatorStats {
                allocations: 0,
                deallocations: 0,
                bytes_allocated: 0,
                bytes_deallocated: 0,
                current_usage: 0,
                peak_usage: 0,
                allocation_time_ns: 0,
                deallocation_time_ns: 0,
            }),
        }
    }
}

impl<A: Allocator> Allocator for MonitoringAllocator<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let start = std::time::Instant::now();
        let result = self.inner.allocate(layout);
        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.allocation_time_ns += elapsed;

        if let Ok(ptr) = &result {
            stats.allocations += 1;
            stats.bytes_allocated += layout.size() as u64;
            stats.current_usage += layout.size() as u64;
            stats.peak_usage = stats.peak_usage.max(stats.current_usage);
        }

        result
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let start = std::time::Instant::now();
        self.inner.deallocate(ptr, layout);
        let elapsed = start.elapsed().as_nanos() as u64;

        let mut stats = self.stats.lock().unwrap();
        stats.deallocations += 1;
        stats.bytes_deallocated += layout.size() as u64;
        stats.current_usage = stats.current_usage.saturating_sub(layout.size() as u64);
        stats.deallocation_time_ns += elapsed;
    }

    fn stats(&self) -> AllocatorStats {
        let mut combined_stats = self.inner.stats();
        let monitoring_stats = self.stats.lock().unwrap();

        combined_stats.allocations += monitoring_stats.allocations;
        combined_stats.deallocations += monitoring_stats.deallocations;
        combined_stats.bytes_allocated += monitoring_stats.bytes_allocated;
        combined_stats.bytes_deallocated += monitoring_stats.bytes_deallocated;
        combined_stats.current_usage += monitoring_stats.current_usage;
        combined_stats.peak_usage = combined_stats.peak_usage.max(monitoring_stats.peak_usage);
        combined_stats.allocation_time_ns += monitoring_stats.allocation_time_ns;
        combined_stats.deallocation_time_ns += monitoring_stats.deallocation_time_ns;

        combined_stats
    }
}

/// Global allocator implementations for use with #[global_allocator]

#[cfg(feature = "jemalloc")]
#[global_allocator]
static JEMALLOC_GLOBAL: JemallocGlobalAlloc = JemallocGlobalAlloc;

#[cfg(feature = "jemalloc")]
struct JemallocGlobalAlloc;

#[cfg(feature = "jemalloc")]
unsafe impl GlobalAlloc for JemallocGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        tikv_jemallocator::Jemalloc.alloc(layout).unwrap_or(std::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        tikv_jemallocator::Jemalloc.deallocate(NonNull::new_unchecked(ptr), layout)
    }
}

#[cfg(feature = "mimalloc")]
#[global_allocator]
static MIMALLOC_GLOBAL: MimallocGlobalAlloc = MimallocGlobalAlloc;

#[cfg(feature = "mimalloc")]
struct MimallocGlobalAlloc;

#[cfg(feature = "mimalloc")]
unsafe impl GlobalAlloc for MimallocGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        mimalloc::MiMalloc.alloc(layout).unwrap_or(std::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        mimalloc::MiMalloc.deallocate(NonNull::new_unchecked(ptr), layout)
    }
}

/// Convenience functions for allocator selection
pub fn create_jemalloc_allocator() -> Option<Box<dyn Allocator>> {
    #[cfg(feature = "jemalloc")]
    {
        Some(Box::new(JemallocAllocator::new()))
    }
    #[cfg(not(feature = "jemalloc"))]
    {
        None
    }
}

pub fn create_mimalloc_allocator() -> Option<Box<dyn Allocator>> {
    #[cfg(feature = "mimalloc")]
    {
        Some(Box::new(MimallocAllocator::new()))
    }
    #[cfg(not(feature = "mimalloc"))]
    {
        None
    }
}

pub fn create_custom_allocator() -> Box<dyn Allocator> {
    Box::new(CustomAllocator::new())
}

pub fn create_monitored_allocator<A: Allocator + 'static>(inner: A) -> Box<dyn Allocator> {
    Box::new(MonitoringAllocator::new(inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_allocator() {
        let allocator = CustomAllocator::new();
        let layout = Layout::from_size_align(64, 8).unwrap();

        // Allocate some memory
        let ptr = allocator.allocate(layout).unwrap();
        assert!(!ptr.as_ptr().is_null());

        // Check stats
        let stats = allocator.stats();
        assert_eq!(stats.allocations, 1);
        assert_eq!(stats.current_usage, 64);

        // Reset arena
        allocator.reset();
        let stats_after = allocator.stats();
        assert_eq!(stats_after.current_usage, 0);
    }

    #[test]
    fn test_monitoring_allocator() {
        let inner = CustomAllocator::new();
        let allocator = MonitoringAllocator::new(inner);
        let layout = Layout::from_size_align(128, 8).unwrap();

        // Allocate some memory
        let ptr = allocator.allocate(layout).unwrap();
        assert!(!ptr.as_ptr().is_null());

        // Check stats
        let stats = allocator.stats();
        assert!(stats.allocations >= 1);
        assert!(stats.current_usage >= 128);
        assert!(stats.allocation_time_ns > 0);
    }
}
