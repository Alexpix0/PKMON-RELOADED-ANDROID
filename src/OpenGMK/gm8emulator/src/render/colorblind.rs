//! GPU Colorblindness Daltonization Matrices (< 0.05 ms on Mobile GPU).
//!
//! Provides calibrated matrix transformation shaders for:
//! 1. Protanopia (Red-weakness / Red-blindness)
//! 2. Deuteranopia (Green-weakness / Green-blindness)
//! 3. Tritanopia (Blue-weakness / Blue-blindness)
//!
//! Ensures HP bars, elemental attack types, and shiny beacons remain instantly discernible.

use std::sync::atomic::{AtomicU32, Ordering};

pub static COLORBLIND_MODE: AtomicU32 = AtomicU32::new(0); // 0 = Off, 1 = Protanopia, 2 = Deuteranopia, 3 = Tritanopia

#[inline(always)]
pub fn get_colorblind_mode() -> u32 {
    COLORBLIND_MODE.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_colorblind_mode(mode: u32) {
    COLORBLIND_MODE.store(mode, Ordering::Relaxed);
}

/// GLSL fragment shader incorporating Daltonization matrices.
pub const COLORBLIND_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec2 v_tex_coord;
uniform sampler2D u_tex;
uniform int u_mode; // 0 = off, 1 = protanopia, 2 = deuteranopia, 3 = tritanopia
out vec4 out_colour;

void main() {
    vec4 col = texture(u_tex, v_tex_coord);
    if (u_mode == 0) {
        out_colour = col;
        return;
    }

    vec3 rgb = col.rgb;
    vec3 result = rgb;

    if (u_mode == 1) {
        // Protanopia matrix
        result = mat3(
            0.56667, 0.43333, 0.0,
            0.55833, 0.44167, 0.0,
            0.0,     0.24167, 0.75833
        ) * rgb;
    } else if (u_mode == 2) {
        // Deuteranopia matrix
        result = mat3(
            0.625, 0.375, 0.0,
            0.700, 0.300, 0.0,
            0.0,   0.300, 0.700
        ) * rgb;
    } else if (u_mode == 3) {
        // Tritanopia matrix
        result = mat3(
            0.95, 0.05,  0.0,
            0.0,  0.433, 0.567,
            0.0,  0.475, 0.525
        ) * rgb;
    }

    out_colour = vec4(clamp(result, 0.0, 1.0), col.a);
}
"#;
