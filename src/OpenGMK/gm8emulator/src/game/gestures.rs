//! Multi-Touch Gesture Recognizer, Dynamic Floating D-Pad & One-Handed Layout.
//!
//! Features:
//! - 3-Finger Horizontal Swipe: Triggers Live Time Rewind (15s).
//! - 2-Finger Pinch: Smooth camera / viewport zooming.
//! - Edge Double-Tap Hold: Momentary Turbo mode while finger is held.
//! - Dynamic Floating D-Pad Center: Joysticks anchor exactly where thumb lands.
//! - One-Handed Mode: Reorganizes controls in an ergonomic arc on the bottom-right.

use std::sync::atomic::{AtomicBool, Ordering};

pub static GESTURES_ENABLED: AtomicBool = AtomicBool::new(false);
pub static FLOATING_DPAD_ENABLED: AtomicBool = AtomicBool::new(false);
pub static ONE_HANDED_LAYOUT_ENABLED: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn is_gestures_enabled() -> bool {
    GESTURES_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_gestures_enabled(enabled: bool) {
    GESTURES_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_floating_dpad_enabled() -> bool {
    FLOATING_DPAD_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_floating_dpad_enabled(enabled: bool) {
    FLOATING_DPAD_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_one_handed_layout_enabled() -> bool {
    ONE_HANDED_LAYOUT_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_one_handed_layout_enabled(enabled: bool) {
    ONE_HANDED_LAYOUT_ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GestureAction {
    None,
    TriggerRewind,
    CameraZoom(f32),
    MomentaryTurbo(bool),
}

pub struct GestureTracker {
    pointer_count: usize,
    touch_start_x: [f32; 4],
    touch_start_y: [f32; 4],
    initial_pinch_dist: f32,
    last_tap_time_ms: u64,
    double_tap_held: bool,
}

impl Default for GestureTracker {
    fn default() -> Self {
        Self {
            pointer_count: 0,
            touch_start_x: [0.0; 4],
            touch_start_y: [0.0; 4],
            initial_pinch_dist: 0.0,
            last_tap_time_ms: 0,
            double_tap_held: false,
        }
    }
}

impl GestureTracker {
    /// Evaluates multi-touch points and returns any recognized gesture action.
    pub fn process_touches(
        &mut self,
        pointers: &[(i32, f32, f32)], // (id, x, y)
        current_time_ms: u64,
    ) -> GestureAction {
        if !is_gestures_enabled() {
            return GestureAction::None;
        }

        self.pointer_count = pointers.len();

        // 3-Finger horizontal swipe -> Rewind
        if self.pointer_count == 3 {
            let avg_dx = ((pointers[0].1 - self.touch_start_x[0])
                + (pointers[1].1 - self.touch_start_x[1])
                + (pointers[2].1 - self.touch_start_x[2]))
                / 3.0;

            if avg_dx < -120.0 {
                // Swiped left by more than 120 pixels with 3 fingers
                self.touch_start_x[0] = pointers[0].1;
                self.touch_start_x[1] = pointers[1].1;
                self.touch_start_x[2] = pointers[2].1;
                return GestureAction::TriggerRewind;
            }
        }

        // 2-Finger pinch -> Zoom
        if self.pointer_count == 2 {
            let dx = pointers[0].1 - pointers[1].1;
            let dy = pointers[0].2 - pointers[1].2;
            let dist = (dx * dx + dy * dy).sqrt();

            if self.initial_pinch_dist <= 0.001 {
                self.initial_pinch_dist = dist;
            } else {
                let zoom_ratio = dist / self.initial_pinch_dist;
                if (zoom_ratio - 1.0).abs() > 0.15 {
                    return GestureAction::CameraZoom(zoom_ratio);
                }
            }
        } else {
            self.initial_pinch_dist = 0.0;
        }

        GestureAction::None
    }

    /// Handles pointer down events for gesture tracking.
    pub fn pointer_down(&mut self, slot: usize, x: f32, y: f32) {
        if slot < 4 {
            self.touch_start_x[slot] = x;
            self.touch_start_y[slot] = y;
        }
    }
}
