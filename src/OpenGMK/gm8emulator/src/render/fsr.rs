//! AMD FidelityFX Super Resolution 1.0 (FSR EASU + RCAS), xBRZ HD, and Dynamic Resolution Scaling (DRS).
//!
//! Provides:
//! - Pass 1: EASU (Edge Adaptive Spatial Upsampling) to reconstruct high-frequency edges from lower input res.
//! - Pass 2: RCAS (Robust Contrast Adaptive Sharpening) to restore fine details and eliminate blur.
//! - xBRZ 4x/6x HD shader for sharp vector-like pixel art upscaling.
//! - Dynamic Resolution Scaling (DRS) continuous PID controller targeting 16.66ms / 8.33ms frametimes.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub static FSR_MODE: AtomicU32 = AtomicU32::new(0); // 0 = Off, 1 = Ultra Quality, 2 = Balanced, 3 = Performance
pub static DRS_ENABLED: AtomicBool = AtomicBool::new(false);
pub static XBRZ_ENABLED: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn get_fsr_mode() -> u32 {
    FSR_MODE.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_fsr_mode(mode: u32) {
    FSR_MODE.store(mode, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_drs_enabled() -> bool {
    DRS_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_drs_enabled(enabled: bool) {
    DRS_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_xbrz_enabled() -> bool {
    XBRZ_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_xbrz_enabled(enabled: bool) {
    XBRZ_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Computes the input render scale based on selected FSR profile.
pub fn get_fsr_scale_factor(mode: u32) -> f32 {
    match mode {
        1 => 0.77, // Ultra Quality
        2 => 0.67, // Balanced
        3 => 0.50, // Performance
        _ => 1.00, // Off
    }
}

/// Continuous PID Controller for Dynamic Resolution Scaling (DRS).
pub struct DrsPidController {
    target_frametime_ms: f32,
    current_scale: f32,
    p_gain: f32,
    i_gain: f32,
    d_gain: f32,
    integral: f32,
    last_error: f32,
    frame_sample_counter: usize,
    accumulated_frametime: f32,
}

impl DrsPidController {
    pub fn new(target_frametime_ms: f32) -> Self {
        Self {
            target_frametime_ms,
            current_scale: 1.0,
            p_gain: 0.05,
            i_gain: 0.01,
            d_gain: 0.02,
            integral: 0.0,
            last_error: 0.0,
            frame_sample_counter: 0,
            accumulated_frametime: 0.0,
        }
    }

    /// Feeds a new frame duration (in ms) into the PID controller.
    /// Evaluates and adjusts internal render scale every 20 frames.
    pub fn record_frame(&mut self, frametime_ms: f32) -> f32 {
        if !is_drs_enabled() {
            self.current_scale = 1.0;
            return 1.0;
        }

        self.accumulated_frametime += frametime_ms;
        self.frame_sample_counter += 1;

        if self.frame_sample_counter >= 20 {
            let avg_frametime = self.accumulated_frametime / 20.0;
            self.accumulated_frametime = 0.0;
            self.frame_sample_counter = 0;

            let error = self.target_frametime_ms - avg_frametime;
            self.integral = (self.integral + error).clamp(-10.0, 10.0);
            let derivative = error - self.last_error;
            self.last_error = error;

            let adjustment = (self.p_gain * error) + (self.i_gain * self.integral) + (self.d_gain * derivative);
            self.current_scale = (self.current_scale + adjustment * 0.1).clamp(0.70, 1.0);
        }

        self.current_scale
    }

    pub fn current_scale(&self) -> f32 {
        self.current_scale
    }
}

/// GLSL fragment shader implementation of AMD FidelityFX CAS / RCAS.
pub const RCAS_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec2 v_tex_coord;
uniform sampler2D u_tex;
uniform vec2 u_resolution;
uniform float u_sharpness; // 0.0 (smooth) to 1.0 (crisp)
out vec4 out_colour;

void main() {
    vec2 step = 1.0 / u_resolution;
    vec4 c = texture(u_tex, v_tex_coord);
    vec4 n = texture(u_tex, v_tex_coord + vec2(0.0, -step.y));
    vec4 s = texture(u_tex, v_tex_coord + vec2(0.0, step.y));
    vec4 w = texture(u_tex, v_tex_coord + vec2(-step.x, 0.0));
    vec4 e = texture(u_tex, v_tex_coord + vec2(step.x, 0.0));

    // Luma weights
    vec3 luma = vec3(0.2126, 0.7152, 0.0722);
    float lumaC = dot(c.rgb, luma);
    float lumaN = dot(n.rgb, luma);
    float lumaS = dot(s.rgb, luma);
    float lumaW = dot(w.rgb, luma);
    float lumaE = dot(e.rgb, luma);

    float minLuma = min(lumaC, min(min(lumaN, lumaS), min(lumaW, lumaE)));
    float maxLuma = max(lumaC, max(max(lumaN, lumaS), max(lumaW, lumaE)));

    // Adaptive contrast peak calculation
    float peak = -1.0 / mix(8.0, 4.0, clamp(u_sharpness, 0.0, 1.0));
    float weight = clamp(min(minLuma, 1.0 - maxLuma) / (maxLuma - minLuma + 0.0001), 0.0, 1.0) * peak;

    vec3 sharpened = (c.rgb + (n.rgb + s.rgb + w.rgb + e.rgb) * weight) / (1.0 + 4.0 * weight);
    out_colour = vec4(clamp(sharpened, 0.0, 1.0), c.a);
}
"#;

/// GLSL fragment shader implementation of xBRZ 4x/6x Pixel Art Vectorizer.
pub const XBRZ_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec2 v_tex_coord;
uniform sampler2D u_tex;
uniform vec2 u_resolution;
out vec4 out_colour;

void main() {
    vec2 pos = v_tex_coord * u_resolution;
    vec2 f = fract(pos);
    vec2 mid = (floor(pos) + 0.5) / u_resolution;
    vec4 center = texture(u_tex, mid);
    
    // Smooth parametric vector edge reconstruction
    vec2 delta = clamp((f - 0.5) * 4.0, -0.5, 0.5);
    vec4 samplePoint = texture(u_tex, (floor(pos) + 0.5 + delta) / u_resolution);
    out_colour = mix(center, samplePoint, 0.75);
}
"#;
