//! Command buffers and command pool dispatch.

pub struct VulkanCommandBuffer {
    pub pool_handle: usize,
    pub buffer_handle: usize,
}

impl VulkanCommandBuffer {
    pub fn new() -> Self {
        Self {
            pool_handle: 1,
            buffer_handle: 1,
        }
    }
}
