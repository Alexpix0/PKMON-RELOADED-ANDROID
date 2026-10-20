//! Graphics pipeline, batching and descriptor state management.

pub struct VulkanPipeline {
    pub pipeline_handle: usize,
    pub ui_pipeline_handle: usize,
    pub blend_enabled: bool,
    pub cached_descriptor_set: usize,
    pub ui_quad_batch_count: usize,
}

impl VulkanPipeline {
    pub fn new() -> Self {
        Self {
            pipeline_handle: 1,
            ui_pipeline_handle: 2,
            blend_enabled: true,
            cached_descriptor_set: 1,
            ui_quad_batch_count: 0,
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
}
