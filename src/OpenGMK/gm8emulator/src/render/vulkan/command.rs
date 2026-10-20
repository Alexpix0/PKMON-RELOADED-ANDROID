//! Command buffers, frames-in-flight and synchronization dispatch.

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct FrameSync {
    pub command_buffer: usize,
    pub image_available_semaphore: usize,
    pub render_finished_semaphore: usize,
    pub in_flight_fence: usize,
}

pub struct VulkanCommandBuffer {
    pub pool_handle: usize,
    pub frames: [FrameSync; MAX_FRAMES_IN_FLIGHT],
    pub current_frame: usize,
}

impl VulkanCommandBuffer {
    pub fn new() -> Self {
        Self {
            pool_handle: 1,
            frames: [
                FrameSync {
                    command_buffer: 1,
                    image_available_semaphore: 1,
                    render_finished_semaphore: 2,
                    in_flight_fence: 1,
                },
                FrameSync {
                    command_buffer: 2,
                    image_available_semaphore: 3,
                    render_finished_semaphore: 4,
                    in_flight_fence: 2,
                },
            ],
            current_frame: 0,
        }
    }

    #[inline]
    pub fn next_frame(&mut self) -> FrameSync {
        let sync = self.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        sync
    }
}
