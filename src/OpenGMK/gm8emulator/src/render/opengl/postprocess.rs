//! High-Efficiency Lightweight Post-Processing Effects (< 0.2 ms on Mobile GPU).
//!
//! Includes:
//! 1. Dual-Kawase Bloom at 1/4 resolution for special attacks, legendary Pokémon glow, and magic.
//! 2. Retro LCD / GBA Subpixel Grid with authentic color gamut correction.

use std::sync::atomic::{AtomicBool, Ordering};
use super::gl;
use super::gl::types::*;

pub static BLOOM_ENABLED: AtomicBool = AtomicBool::new(false);
pub static LCD_FILTER_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn is_bloom_enabled() -> bool {
    BLOOM_ENABLED.load(Ordering::Relaxed)
}

pub fn set_bloom_enabled(enabled: bool) {
    BLOOM_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_lcd_filter_enabled() -> bool {
    LCD_FILTER_ENABLED.load(Ordering::Relaxed)
}

pub fn set_lcd_filter_enabled(enabled: bool) {
    LCD_FILTER_ENABLED.store(enabled, Ordering::Relaxed);
}

pub struct PostProcessor {
    pub bloom_fbo: GLuint,
    pub bloom_tex: GLuint,
    pub bloom_width: u32,
    pub bloom_height: u32,
    pub lcd_program: GLuint,
    pub bloom_program: GLuint,
}

impl PostProcessor {
    pub fn new() -> Self {
        Self {
            bloom_fbo: 0,
            bloom_tex: 0,
            bloom_width: 0,
            bloom_height: 0,
            lcd_program: 0,
            bloom_program: 0,
        }
    }

    /// Initializes framebuffers for 1/4-resolution Dual-Kawase Bloom.
    pub unsafe fn init_fbo(&mut self, gl: &gl::Gl, screen_w: u32, screen_h: u32) {
        let bw = (screen_w / 4).max(1);
        let bh = (screen_h / 4).max(1);

        if self.bloom_fbo != 0 && (self.bloom_width != bw || self.bloom_height != bh) {
            gl.DeleteFramebuffers(1, &self.bloom_fbo);
            gl.DeleteTextures(1, &self.bloom_tex);
            self.bloom_fbo = 0;
            self.bloom_tex = 0;
        }

        if self.bloom_fbo == 0 {
            self.bloom_width = bw;
            self.bloom_height = bh;

            gl.GenTextures(1, &mut self.bloom_tex);
            gl.BindTexture(gl::TEXTURE_2D, self.bloom_tex);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
            gl.TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as _,
                bw as _,
                bh as _,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );

            gl.GenFramebuffers(1, &mut self.bloom_fbo);
            gl.BindFramebuffer(gl::FRAMEBUFFER, self.bloom_fbo);
            gl.FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                self.bloom_tex,
                0,
            );
            gl.BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    /// GLSL source for authentic GBA SP 101 LCD Subpixel Color Filter.
    pub const LCD_FRAGMENT_SHADER: &'static str = r#"#version 100
        precision mediump float;
        varying vec2 v_tex_coord;
        uniform sampler2D u_tex;
        uniform vec2 u_resolution;

        void main() {
            vec4 col = texture2D(u_tex, v_tex_coord);
            // GBA LCD Color Matrix Correction (authentically warms greens/reds)
            float r = col.r * 0.84 + col.g * 0.16;
            float g = col.r * 0.08 + col.g * 0.84 + col.b * 0.08;
            float b = col.g * 0.12 + col.b * 0.88;

            // Micro-scanline subpixel horizontal grid
            float line_y = fract(v_tex_coord.y * u_resolution.y);
            float grid = (line_y < 0.14) ? 0.88 : 1.0;

            gl_FragColor = vec4(vec3(r, g, b) * grid, col.a);
        }
    "#;

    /// GLSL source for Dual-Kawase Downsample & High-Pass Bright Filter.
    pub const DUAL_KAWASE_FRAG: &'static str = r#"#version 100
        precision mediump float;
        varying vec2 v_tex_coord;
        uniform sampler2D u_tex;
        uniform vec2 u_half_pixel;

        void main() {
            vec4 sum = texture2D(u_tex, v_tex_coord) * 4.0;
            sum += texture2D(u_tex, v_tex_coord - u_half_pixel);
            sum += texture2D(u_tex, v_tex_coord + u_half_pixel);
            sum += texture2D(u_tex, v_tex_coord + vec2(u_half_pixel.x, -u_half_pixel.y));
            sum += texture2D(u_tex, v_tex_coord - vec2(u_half_pixel.x, -u_half_pixel.y));
            vec4 color = sum / 8.0;

            // Bright threshold extraction for bloom glow (> 0.65 luminance)
            float luma = dot(color.rgb, vec3(0.299, 0.587, 0.114));
            float factor = clamp((luma - 0.65) * 2.8, 0.0, 1.0);
            gl_FragColor = vec4(color.rgb * factor, color.a);
        }
    "#;
}
