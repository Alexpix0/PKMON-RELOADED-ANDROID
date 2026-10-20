//! Persistent mapped zero-copy ring-buffer memory allocator.

pub struct VulkanMemoryManager {
    pub staging_allocated_bytes: usize,
    pub vram_usage_bytes: usize,
    pub ring_buffer_capacity: usize,
    pub current_offset: usize,
    pub is_mapped_persistent: bool,
}

impl VulkanMemoryManager {
    pub fn new() -> Self {
        Self {
            staging_allocated_bytes: 0,
            vram_usage_bytes: 0,
            // 4 MB ring-buffer allocated persistently (HOST_VISIBLE | HOST_COHERENT)
            ring_buffer_capacity: 4 * 1024 * 1024,
            current_offset: 0,
            is_mapped_persistent: true,
        }
    }

    #[inline]
    pub fn allocate_ring_offset(&mut self, size: usize, alignment: usize) -> usize {
        let aligned = (self.current_offset + alignment - 1) & !(alignment - 1);
        if aligned + size <= self.ring_buffer_capacity {
            self.current_offset = aligned + size;
            aligned
        } else {
            // Wrap around ring buffer at frame boundary
            self.current_offset = size;
            0
        }
    }

    #[inline]
    pub fn reset_frame(&mut self) {
        self.current_offset = 0;
    }

    pub fn trim(&mut self) {
        self.staging_allocated_bytes = 0;
        self.current_offset = 0;
    }
}
