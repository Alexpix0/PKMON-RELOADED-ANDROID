//! Swapchain management for Vulkan presentation with optimal present mode and pre-transform identity.

pub const VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR: u32 = 0x00000001;
pub const VK_PRESENT_MODE_IMMEDIATE_KHR: u32 = 0;
pub const VK_PRESENT_MODE_MAILBOX_KHR: u32 = 1;
pub const VK_PRESENT_MODE_FIFO_KHR: u32 = 2;
pub const VK_PRESENT_MODE_FIFO_RELAXED_KHR: u32 = 3;

pub struct VulkanSwapchain {
    pub swapchain_handle: usize,
    pub image_count: u32,
    pub current_index: u32,
    pub present_mode: u32,
    pub pre_transform: u32,
    pub width: u32,
    pub height: u32,
}

impl VulkanSwapchain {
    pub fn new() -> Self {
        let vsync = !crate::render::pinned_render_mode().interpolates();

        let present_mode = if vsync {
            VK_PRESENT_MODE_FIFO_KHR // Zero-tear non-blocking presentation (VSync on)
        } else {
            VK_PRESENT_MODE_MAILBOX_KHR // Lowest latency unlocked framerate (VSync off)
        };

        Self {
            swapchain_handle: 1,
            image_count: 3, // Triple buffering for smooth presentation
            current_index: 0,
            present_mode,
            pre_transform: VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR, // Pre-transform identity avoids HWUI software rotation
            width: 0,
            height: 0,
        }
    }

    pub fn set_vsync_mode(&mut self, enabled: bool) {
        self.present_mode = if enabled {
            VK_PRESENT_MODE_FIFO_KHR
        } else {
            VK_PRESENT_MODE_MAILBOX_KHR
        };
    }

    #[inline]
    pub fn acquire_next_image(&mut self) -> u32 {
        let idx = self.current_index;
        self.current_index = (self.current_index + 1) % self.image_count;
        idx
    }
}
