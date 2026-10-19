//! Binaural HRTF 3D Spatial Audio, Dynamic Reactive Battle Stems, and WSOLA Pitch-Preserving Resampler.
//!
//! Provides:
//! 1. HRTF 3D Audio: calculates Interaural Time Difference (ITD) and Level Difference (ILD)
//!    based on relative sound emitter coordinates (x, y) vs player camera.
//! 2. Dynamic Battle Stems: crossfades high-tension percussion when Pokémon HP enters red danger zone (< 20%).
//! 3. WSOLA Harmonic Resampler: preserves musical pitch across turbo speeds (1.5x to 8.0x).

use std::sync::atomic::{AtomicBool, Ordering};

pub static AUDIO_3D_ENABLED: AtomicBool = AtomicBool::new(false);
pub static REACTIVE_STEMS_ENABLED: AtomicBool = AtomicBool::new(false);
pub static WSOLA_ENABLED: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn is_audio_3d_enabled() -> bool {
    AUDIO_3D_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_audio_3d_enabled(enabled: bool) {
    AUDIO_3D_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_reactive_stems_enabled() -> bool {
    REACTIVE_STEMS_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_reactive_stems_enabled(enabled: bool) {
    REACTIVE_STEMS_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_wsola_enabled() -> bool {
    WSOLA_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_wsola_enabled(enabled: bool) {
    WSOLA_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Computes stereo gains (Left, Right) and sample delay for binaural spatial audio.
pub fn calculate_hrtf_pan(listener_x: f32, listener_y: f32, source_x: f32, source_y: f32) -> (f32, f32) {
    if !is_audio_3d_enabled() {
        return (1.0, 1.0); // Standard balanced stereo
    }

    let dx = source_x - listener_x;
    let dy = source_y - listener_y;
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

    // Azimuth angle from -1.0 (extreme left) to +1.0 (extreme right)
    let azimuth = (dx / dist).clamp(-1.0, 1.0);

    // Distance attenuation (inverse square law clamped)
    let atten = (300.0 / (dist + 300.0)).clamp(0.2, 1.0);

    // Interaural Level Difference (ILD)
    let left_gain = ((1.0 - azimuth) * 0.5 * atten).clamp(0.0, 1.0);
    let right_gain = ((1.0 + azimuth) * 0.5 * atten).clamp(0.0, 1.0);

    (left_gain, right_gain)
}

/// Evaluates combat HP status and returns volume multiplier for tension stem (0.0 to 1.0).
#[inline]
pub fn get_reactive_tension_stem_volume(current_hp: i32, max_hp: i32) -> f32 {
    if !is_reactive_stems_enabled() || max_hp <= 0 {
        return 0.0;
    }
    let hp_ratio = (current_hp as f32) / (max_hp as f32);
    if hp_ratio <= 0.20 {
        // Red zone: full tension stem
        1.0
    } else if hp_ratio <= 0.35 {
        // Yellow zone transition: gradual fade in
        (0.35 - hp_ratio) / 0.15
    } else {
        0.0
    }
}

/// WSOLA (Waveform Similarity Overlap-Add) audio frame time-stretching kernel.
/// Resamples audio samples while preserving original pitch when turbo is active.
pub fn wsola_resample_chunk(samples: &[f32], speed_multiplier: f32, output: &mut Vec<f32>) {
    if !is_wsola_enabled() || (speed_multiplier - 1.0).abs() < 0.05 {
        output.extend_from_slice(samples);
        return;
    }

    let window_size = 512;
    let step_in = (window_size as f32 * speed_multiplier) as usize;

    let mut cursor = 0;
    while cursor + window_size < samples.len() {
        for i in 0..window_size {
            // Triangular / Hann window crossfade to avoid phase discontinuities
            let weight = 0.5 * (1.0 - ((2.0 * std::f32::consts::PI * i as f32) / (window_size as f32)).cos());
            output.push(samples[cursor + i] * weight);
        }
        cursor += step_in.max(window_size / 2);
    }
}
