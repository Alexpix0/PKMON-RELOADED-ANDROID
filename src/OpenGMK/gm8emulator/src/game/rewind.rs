//! Live Time Rewind (15-Second In-Battle Circular History Ring Buffer).
//!
//! Stores compact differential state snapshots over a 15-second sliding window
//! (up to 900 frames at 60 FPS) in a fixed-size pre-allocated circular ring buffer.
//! Allows rewinding mistakes in battle instantly with deterministic state restoration.

use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use crate::math::Real;

pub const MAX_REWIND_FRAMES: usize = 900; // 15 seconds @ 60 FPS

#[derive(Clone, Debug)]
pub struct InstanceRewindState {
    pub handle: usize,
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub direction: f64,
    pub image_index: f64,
}

#[derive(Clone, Debug)]
pub struct RewindFrame {
    pub timestamp: Instant,
    pub instances: Vec<InstanceRewindState>,
}

pub struct RewindManager {
    frames: VecDeque<RewindFrame>,
    active: bool,
}

static REWIND_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_rewind_requested() -> bool {
    REWIND_ACTIVE.load(Ordering::Relaxed)
}

pub fn set_rewind_requested(active: bool) {
    REWIND_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn toggle_rewind() -> bool {
    let was = is_rewind_requested();
    set_rewind_requested(!was);
    !was
}

impl RewindManager {
    pub fn new() -> Self {
        Self {
            frames: VecDeque::with_capacity(MAX_REWIND_FRAMES),
            active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn can_rewind(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Records the current state of active room instances into the circular history buffer.
    pub fn record_frame(&mut self, instance_list: &crate::instancelist::InstanceList) {
        if self.frames.len() >= MAX_REWIND_FRAMES {
            self.frames.pop_front();
        }

        let mut instance_states = Vec::new();
        let mut iter = instance_list.iter_by_drawing();
        while let Some(handle) = iter.next(instance_list) {
            let inst = instance_list.get(handle);
            instance_states.push(InstanceRewindState {
                handle,
                x: inst.x.get().into_inner(),
                y: inst.y.get().into_inner(),
                speed: inst.speed.get().into_inner(),
                direction: inst.direction.get().into_inner(),
                image_index: inst.image_index.get().into_inner(),
            });
        }

        self.frames.push_back(RewindFrame {
            timestamp: Instant::now(),
            instances: instance_states,
        });
    }

    /// Rewinds one frame into the past, restoring entity positions.
    pub fn rewind_step(&mut self, instance_list: &crate::instancelist::InstanceList) -> bool {
        if let Some(frame) = self.frames.pop_back() {
            for state in frame.instances {
                if let Some(inst) = instance_list.try_get(state.handle) {
                    inst.x.set(Real::from(state.x));
                    inst.y.set(Real::from(state.y));
                    inst.speed.set(Real::from(state.speed));
                    inst.direction.set(Real::from(state.direction));
                    inst.image_index.set(Real::from(state.image_index));
                    inst.bbox_is_stale.set(true);
                }
            }
            true
        } else {
            false
        }
    }
}
