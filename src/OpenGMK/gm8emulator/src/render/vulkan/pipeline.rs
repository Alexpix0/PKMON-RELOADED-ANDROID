//! Graphics pipeline and shader state management for Vulkan.

pub struct VulkanPipeline {
    pub pipeline_handle: usize,
    pub blend_enabled: bool,
}

impl VulkanPipeline {
    pub fn new() -> Self {
        Self {
            pipeline_handle: 1,
            blend_enabled: true,
        }
    }
}
