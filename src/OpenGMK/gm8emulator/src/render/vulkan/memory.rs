//! Contiguous GPU memory allocator and vertex buffer management.

pub struct VulkanMemoryManager {
    pub staging_allocated_bytes: usize,
    pub vram_usage_bytes: usize,
}

impl VulkanMemoryManager {
    pub fn new() -> Self {
        Self {
            staging_allocated_bytes: 0,
            vram_usage_bytes: 0,
        }
    }

    pub fn trim(&mut self) {
        self.staging_allocated_bytes = 0;
    }
}
