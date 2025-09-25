// Merkle DAG: vm.MemorySystem
// Defines the interface for the memory system.
pub trait MemorySystem {
    fn read(&self, address: u64) -> u8;
    fn write(&mut self, address: u64, value: u8);
    fn size(&self) -> usize;
    fn read_slice(&self, address: u64, len: usize) -> Option<&[u8]>;
    fn write_slice(&mut self, address: u64, data: &[u8]);
    fn allocate(&mut self, size: usize) -> Option<MemoryHandle>;
    fn deallocate(&mut self, handle: MemoryHandle);
    fn get_memory_handle(&self, handle: &MemoryHandle) -> Option<&[u8]>;
    fn get_memory_handle_mut(&mut self, handle: &MemoryHandle) -> Option<&mut [u8]>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryHandle {
    pub start_address: u64,
    pub size: usize,
}

pub struct MemoryArena {
    memory: Vec<u8>,
    allocated_blocks: Vec<(MemoryHandle, bool)>, // (handle, is_free)
    free_blocks: Vec<MemoryHandle>,
}
pub struct MemorySystemImpl {
    memory: Vec<u8>,
}

impl MemorySystemImpl {
    /// Creates a new memory system with a given size in bytes.
    pub fn new(size: usize) -> Self {
        Self {
            memory: vec![0; size],
        }
    }
}

impl MemorySystem for MemorySystemImpl {
    fn read(&self, address: u64) -> u8 {
        // Basic bounds checking
        self.memory.get(address as usize).copied().unwrap_or(0)
    }

    fn write(&mut self, address: u64, value: u8) {
        if let Some(mem) = self.memory.get_mut(address as usize) {
            *mem = value;
        }
    }

    fn size(&self) -> usize {
        self.memory.len()
    }

    fn read_slice(&self, address: u64, len: usize) -> Option<&[u8]> {
        let start = address as usize;
        let end = start.saturating_add(len);
        if end > self.memory.len() {
            return None;
        }
        self.memory.get(start..end)
    }

    fn write_slice(&mut self, address: u64, data: &[u8]) {
        let start = address as usize;
        let end = start.saturating_add(data.len());
        if end > self.memory.len() {
            return;
        }
        if let Some(slice) = self.memory.get_mut(start..end) {
            slice.copy_from_slice(data);
        }
    }

    fn allocate(&mut self, size: usize) -> Option<MemoryHandle> {
        // Simple first-fit allocation
        for i in 0..self.memory.len() - size + 1 {
            if self.memory[i..i + size].iter().all(|&b| b == 0) {
                // Found free block
                let handle = MemoryHandle {
                    start_address: i as u64,
                    size,
                };
                // Mark as allocated
                for j in i..i + size {
                    self.memory[j] = 1; // Mark as allocated
                }
                return Some(handle);
            }
        }
        None // No free block found
    }

    fn deallocate(&mut self, handle: MemoryHandle) {
        let start = handle.start_address as usize;
        let end = start + handle.size;
        if end <= self.memory.len() {
            for i in start..end {
                self.memory[i] = 0; // Mark as free
            }
        }
    }

    fn get_memory_handle(&self, handle: &MemoryHandle) -> Option<&[u8]> {
        let start = handle.start_address as usize;
        let end = start + handle.size;
        if end <= self.memory.len() {
            Some(&self.memory[start..end])
        } else {
            None
        }
    }

    fn get_memory_handle_mut(&mut self, handle: &MemoryHandle) -> Option<&mut [u8]> {
        let start = handle.start_address as usize;
        let end = start + handle.size;
        if end <= self.memory.len() {
            Some(&mut self.memory[start..end])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let mem = MemorySystemImpl::new(1024);
        assert_eq!(mem.size(), 1024);
    }

    #[test]
    fn test_memory_read_write() {
        let mut mem = MemorySystemImpl::new(1024);

        // Test initial state (should be zeros)
        assert_eq!(mem.read(0), 0);
        assert_eq!(mem.read(100), 0);

        // Test writing and reading back
        mem.write(50, 42);
        assert_eq!(mem.read(50), 42);

        // Test multiple writes
        mem.write(100, 255);
        mem.write(200, 128);
        assert_eq!(mem.read(100), 255);
        assert_eq!(mem.read(200), 128);
        assert_eq!(mem.read(50), 42); // Other locations unchanged
    }

    #[test]
    fn test_memory_bounds_checking() {
        let mut mem = MemorySystemImpl::new(100);

        // Writing within bounds
        mem.write(99, 123);
        assert_eq!(mem.read(99), 123);

        // Writing out of bounds (should not panic, just ignored)
        mem.write(100, 255); // One past the end
        mem.write(1000, 255); // Way out of bounds

        // Reading out of bounds should return 0
        assert_eq!(mem.read(100), 0);
        assert_eq!(mem.read(1000), 0);
    }

    #[test]
    fn test_memory_byte_operations() {
        let mut mem = MemorySystemImpl::new(10);

        // Test all byte values
        for i in 0..=255u8 {
            mem.write(i as u64 % 10, i);
            assert_eq!(mem.read(i as u64 % 10), i);
        }
    }

    #[test]
    fn test_memory_slice_operations() {
        let mut mem = MemorySystemImpl::new(1024);
        let data_to_write: &[u8] = &[1, 2, 3, 4, 5];

        // Test successful write and read
        mem.write_slice(100, data_to_write);
        let read_data = mem.read_slice(100, 5).unwrap();
        assert_eq!(read_data, data_to_write);

        // Test out-of-bounds write (should not panic)
        mem.write_slice(1020, data_to_write); // This would go out of bounds
        let read_data_after_overflow = mem.read_slice(1020, 5);
        assert!(read_data_after_overflow.is_none());

        // Test out-of-bounds read
        let read_data_out_of_bounds = mem.read_slice(1024, 1);
        assert!(read_data_out_of_bounds.is_none());
    }

    #[test]
    fn test_memory_allocation() {
        let mut mem = MemorySystemImpl::new(1024);

        // Test allocation
        let handle = mem.allocate(100).expect("Failed to allocate memory");
        assert_eq!(handle.size, 100);
        assert_eq!(handle.start_address, 0);

        // Test allocation of allocated memory (should fail)
        let handle2 = mem.allocate(100);
        assert!(handle2.is_none());

        // Test deallocation
        mem.deallocate(handle);

        // Test re-allocation of freed memory
        let handle3 = mem.allocate(100).expect("Failed to re-allocate memory");
        assert_eq!(handle3.start_address, 0);
    }

    #[test]
    fn test_memory_handle_operations() {
        let mut mem = MemorySystemImpl::new(1024);
        let handle = mem.allocate(100).expect("Failed to allocate memory");

        // Test getting memory handle
        let data = mem.get_memory_handle(&handle).expect("Failed to get memory handle");
        assert_eq!(data.len(), 100);

        // Test getting mutable memory handle
        let data_mut = mem.get_memory_handle_mut(&handle).expect("Failed to get mutable memory handle");
        assert_eq!(data_mut.len(), 100);

        // Test modifying through handle
        data_mut[0] = 42;
        let data_after = mem.get_memory_handle(&handle).expect("Failed to get memory handle");
        assert_eq!(data_after[0], 42);
    }
}
