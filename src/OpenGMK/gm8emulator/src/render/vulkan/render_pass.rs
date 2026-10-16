//! Render pass and framebuffer abstraction for Vulkan.

pub struct VulkanRenderPass {
    pub render_pass_handle: usize,
}

impl VulkanRenderPass {
    pub fn new() -> Self {
        Self {
            render_pass_handle: 1,
        }
    }
}
