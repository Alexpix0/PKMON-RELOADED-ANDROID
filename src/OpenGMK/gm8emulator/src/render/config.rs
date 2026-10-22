//! RenderConfig: Zero-alloc atomic configuration cache for OpenGMK render hot-loops.
//! Eliminates per-frame std::env::var calls in OpenGL ES and Vulkan paths.

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU8, Ordering};

pub struct RenderConfig {
    pub filter_mode: AtomicU8,      // 0 = nearest, 1 = linear, 2 = sharp, 3 = fxaa
    pub render_scale_q8: AtomicU16, // scale * 256 (e.g. 256 = 1.0x, 218 = 0.85x)
    pub bezel_active: AtomicBool,
    pub vsync_mode: AtomicU8,       // 0 = off, 1 = on, 2 = adaptive
    pub backend_requested: AtomicU8,// 0 = opengl, 1 = vulkan
    pub backend_actual: AtomicU8,   // 0 = opengl, 1 = vulkan
    pub fallback_active: AtomicBool,
    pub generation: AtomicU32,      // Incremented on every configuration update
}

impl RenderConfig {
    pub const fn new() -> Self {
        Self {
            filter_mode: AtomicU8::new(0),
            render_scale_q8: AtomicU16::new(256),
            bezel_active: AtomicBool::new(false),
            vsync_mode: AtomicU8::new(1),
            backend_requested: AtomicU8::new(0),
            backend_actual: AtomicU8::new(0),
            fallback_active: AtomicBool::new(false),
            generation: AtomicU32::new(0),
        }
    }

    /// Parses configuration once on boot from environment variables.
    /// Subsequent updates MUST arrive explicitly via JNI / SettingsBridge.
    pub fn init_from_env(&self) {
        if let Ok(filter) = std::env::var("OPENGMK_FILTER") {
            let mode = match filter.as_str() {
                "linear" => 1,
                "sharp" => 2,
                "fxaa" => 3,
                _ => 0,
            };
            self.filter_mode.store(mode, Ordering::Relaxed);
        }
        if let Ok(scale_str) = std::env::var("OPENGMK_RENDER_SCALE") {
            if let Ok(scale) = scale_str.parse::<f32>() {
                let q8 = (scale.clamp(0.25, 2.0) * 256.0) as u16;
                self.render_scale_q8.store(q8, Ordering::Relaxed);
            }
        }
        if let Ok(bezel) = std::env::var("OPENGMK_BEZEL") {
            self.bezel_active.store(bezel != "oled" && bezel != "black", Ordering::Relaxed);
        }
        if let Ok(backend) = std::env::var("OPENGMK_BACKEND") {
            let b = if backend.eq_ignore_ascii_case("vulkan") { 1 } else { 0 };
            self.backend_requested.store(b, Ordering::Relaxed);
        }
        if let Ok(vsync) = std::env::var("OPENGMK_VSYNC") {
            let v = match vsync.as_str() {
                "0" | "false" | "off" => 0,
                "2" | "adaptive" => 2,
                _ => 1,
            };
            self.vsync_mode.store(v, Ordering::Relaxed);
        }
        self.generation.fetch_add(1, Ordering::Release);
    }

    #[inline]
    pub fn is_linear_or_sharp(&self) -> bool {
        matches!(self.filter_mode.load(Ordering::Relaxed), 1 | 2 | 3)
    }

    #[inline]
    pub fn is_bezel_active(&self) -> bool {
        self.bezel_active.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn render_scale(&self) -> f32 {
        self.render_scale_q8.load(Ordering::Relaxed) as f32 / 256.0
    }

    pub fn set_backend_status(&self, requested_vulkan: bool, actual_vulkan: bool, fallback: bool) {
        self.backend_requested.store(if requested_vulkan { 1 } else { 0 }, Ordering::Relaxed);
        self.backend_actual.store(if actual_vulkan { 1 } else { 0 }, Ordering::Relaxed);
        self.fallback_active.store(fallback, Ordering::Relaxed);
        self.generation.fetch_add(1, Ordering::Release);
    }

    pub const GL_NEAREST: u32 = 0x2600;
    pub const GL_LINEAR: u32 = 0x2601;

    #[inline]
    pub fn filter_gl_enum(&self, interpolate_pixels: bool) -> u32 {
        match self.filter_mode.load(Ordering::Relaxed) {
            1 | 2 | 3 => Self::GL_LINEAR,
            0 => if interpolate_pixels { Self::GL_LINEAR } else { Self::GL_NEAREST },
            _ => Self::GL_NEAREST,
        }
    }
}

pub static CONFIG: RenderConfig = RenderConfig::new();
