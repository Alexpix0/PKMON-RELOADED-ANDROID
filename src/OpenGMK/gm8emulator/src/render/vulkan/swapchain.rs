//! Swapchain management for Vulkan presentation.

pub struct VulkanSwapchain {
    pub swapchain_handle: usize,
    pub image_count: u32,
    pub current_index: u32,
}

impl VulkanSwapchain {
    pub fn new() -> Self {
        Self {
            swapchain_handle: 1,
            image_count: 3, // Triple buffering
            current_index: 0,
        }
    }
}
