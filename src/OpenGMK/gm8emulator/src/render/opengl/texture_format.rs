//! GPU Texture Compression formats (ETC2 & ASTC) for Mobile Hardware Acceleration.
//!
//! ETC2 is a standard format across all OpenGL ES 3.0+ hardware.
//! ASTC provides high-efficiency adaptive block sizes (4x4, 8x8) reducing VRAM consumption by ~60%.

use super::gl;
use super::gl::types::*;

pub const GL_COMPRESSED_RGB8_ETC2: GLenum = 0x9274;
pub const GL_COMPRESSED_RGBA8_ETC2_EAC: GLenum = 0x9278;
pub const GL_COMPRESSED_RGBA_ASTC_4x4_KHR: GLenum = 0x93B0;
pub const GL_COMPRESSED_RGBA_ASTC_6x6_KHR: GLenum = 0x93B4;
pub const GL_COMPRESSED_RGBA_ASTC_8x8_KHR: GLenum = 0x93B7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    None,
    Etc2Rgb,
    Etc2Rgba,
    Astc4x4,
    Astc8x8,
}

pub struct TextureCompressionSupport {
    pub etc2_supported: bool,
    pub astc_supported: bool,
}

static SUPPORT: std::sync::OnceLock<TextureCompressionSupport> = std::sync::OnceLock::new();

impl TextureCompressionSupport {
    pub unsafe fn detect(gl: &gl::Gl) -> &'static Self {
        SUPPORT.get_or_init(|| {
            let mut etc2 = false;
            let mut astc = false;

            #[cfg(target_os = "android")]
            {
                // In OpenGL ES 3.0+, ETC2 is standard and guaranteed
                etc2 = true;
            }

            // Query extensions string
            let ext_ptr = gl.GetString(gl::EXTENSIONS);
            if !ext_ptr.is_null() {
                if let Ok(ext_str) = std::ffi::CStr::from_ptr(ext_ptr.cast()).to_str() {
                    if ext_str.contains("GL_KHR_texture_compression_astc_ldr")
                        || ext_str.contains("GL_OES_texture_compression_astc")
                    {
                        astc = true;
                    }
                    if ext_str.contains("GL_OES_compressed_ETC2_RGB8_texture")
                        || ext_str.contains("GL_OES_compressed_ETC2_RGBA8_texture")
                    {
                        etc2 = true;
                    }
                }
            }

            eprintln!(
                "[texture_compression] Capacidades GPU detectadas: ETC2={}, ASTC={}",
                etc2, astc
            );

            Self {
                etc2_supported: etc2,
                astc_supported: astc,
            }
        })
    }

    /// Calculates compressed payload size in bytes for a given dimension and format.
    pub fn calculate_size(width: u32, height: u32, format: CompressionFormat) -> usize {
        match format {
            CompressionFormat::None => (width * height * 4) as usize,
            CompressionFormat::Etc2Rgb => {
                let blocks_x = (width + 3) / 4;
                let blocks_y = (height + 3) / 4;
                (blocks_x * blocks_y * 8) as usize
            },
            CompressionFormat::Etc2Rgba => {
                let blocks_x = (width + 3) / 4;
                let blocks_y = (height + 3) / 4;
                (blocks_x * blocks_y * 16) as usize
            },
            CompressionFormat::Astc4x4 => {
                let blocks_x = (width + 3) / 4;
                let blocks_y = (height + 3) / 4;
                (blocks_x * blocks_y * 16) as usize
            },
            CompressionFormat::Astc8x8 => {
                let blocks_x = (width + 7) / 8;
                let blocks_y = (height + 7) / 8;
                (blocks_x * blocks_y * 16) as usize
            },
        }
    }
}
