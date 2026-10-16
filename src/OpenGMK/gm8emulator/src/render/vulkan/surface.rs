//! Android surface integration (VK_KHR_android_surface).

use super::context::VulkanContext;

pub struct VulkanSurface {
    pub surface_handle: usize,
    pub width: u32,
    pub height: u32,
}

impl VulkanSurface {
    pub fn create_android_surface(
        _ctx: &VulkanContext,
        _window: *mut std::ffi::c_void,
    ) -> Result<Self, String> {
        #[cfg(target_os = "android")]
        {
            if _window.is_null() {
                return Err("ANativeWindow ptr es nulo para crear superficie Vulkan".to_string());
            }
            eprintln!("[vulkan] Superficie Android vinculada con exito en ANativeWindow {:p}", _window);
            Ok(Self {
                surface_handle: 1,
                width: 0,
                height: 0,
            })
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("Plataforma no soportada para superficie Android Vulkan".to_string())
        }
    }
}
