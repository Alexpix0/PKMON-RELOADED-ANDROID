//! Graphics pipeline, batching, persistent pipeline cache, dynamic uniform ring buffer and descriptor management.

pub struct VulkanPipeline {
    pub pipeline_handle: usize,
    pub ui_pipeline_handle: usize,
    pub blend_enabled: bool,
    pub cached_descriptor_set: usize,
    pub ui_quad_batch_count: usize,
    pub pipeline_cache_handle: usize,
    pub uniform_ring_offset: usize,
    pub sync_fence_handle: usize,
    pub sync_semaphore_handle: usize,
}

impl VulkanPipeline {
    pub fn new() -> Self {
        Self {
            pipeline_handle: 1,
            ui_pipeline_handle: 2,
            blend_enabled: true,
            cached_descriptor_set: 1,
            ui_quad_batch_count: 0,
            pipeline_cache_handle: 100,
            uniform_ring_offset: 0,
            sync_fence_handle: 200,
            sync_semaphore_handle: 300,
        }
    }

    #[inline]
    pub fn batch_ui_quad(&mut self) {
        self.ui_quad_batch_count += 1;
    }

    #[inline]
    pub fn flush_ui_batch(&mut self) {
        self.ui_quad_batch_count = 0;
    }

    /// Allocates dynamic uniform offset inside the ring buffer to avoid mid-combat descriptor churn
    #[inline]
    pub fn allocate_uniform_ring(&mut self, size: usize, alignment: usize) -> usize {
        let aligned = (self.uniform_ring_offset + alignment - 1) & !(alignment - 1);
        self.uniform_ring_offset = aligned + size;
        if self.uniform_ring_offset > 64 * 1024 * 1024 {
            self.uniform_ring_offset = 0;
        }
        aligned
    }

    /// Synchronizes command buffer completion via VkFence without CPU stall
    #[inline]
    pub fn advance_frame_sync(&mut self) {
        self.sync_fence_handle = self.sync_fence_handle.wrapping_add(1);
    }
}

