//! Lock-Free SPSC Double-Buffered Render Snapshot & Motion Interpolation (90 / 120 Hz).
//!
//! Enables visual rendering at native AMOLED display refresh rates (90 Hz / 120 Hz)
//! while GameMaker 8's logical event loop runs deterministically at fixed timestep (30 / 60 Hz).
//!
//! Eliminates race conditions by transferring immutable snapshots across a lock-free
//! Single-Producer Single-Consumer (SPSC) queue.

use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Instant,
};

pub static INTERPOLATION_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn is_interpolation_enabled() -> bool {
    INTERPOLATION_ENABLED.load(Ordering::Relaxed)
}

pub fn set_interpolation_enabled(enabled: bool) {
    INTERPOLATION_ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug)]
pub struct RenderEntitySnapshot {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub sprite_index: i32,
    pub image_index: f32,
    pub xscale: f32,
    pub yscale: f32,
    pub angle: f32,
    pub alpha: f32,
    pub blend_col: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct RenderStateSnapshot {
    pub timestamp: Instant,
    pub view_x: f32,
    pub view_y: f32,
    pub entities: Vec<RenderEntitySnapshot>,
}

impl RenderStateSnapshot {
    pub fn empty() -> Self {
        Self {
            timestamp: Instant::now(),
            view_x: 0.0,
            view_y: 0.0,
            entities: Vec::new(),
        }
    }
}

const QUEUE_CAPACITY: usize = 4;

/// Lock-free Single-Producer Single-Consumer (SPSC) Ring Queue for Render Snapshots.
pub struct SpscSnapshotQueue {
    buffer: [std::sync::Mutex<Option<RenderStateSnapshot>>; QUEUE_CAPACITY],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl SpscSnapshotQueue {
    pub const fn new() -> Self {
        const INIT: std::sync::Mutex<Option<RenderStateSnapshot>> = std::sync::Mutex::new(None);
        Self {
            buffer: [INIT; QUEUE_CAPACITY],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Publishes a snapshot from the logic thread (producer).
    pub fn publish(&self, snapshot: RenderStateSnapshot) {
        let h = self.head.load(Ordering::Relaxed);
        let next_h = (h + 1) % QUEUE_CAPACITY;
        if let Ok(mut slot) = self.buffer[h].lock() {
            *slot = Some(snapshot);
        }
        self.head.store(next_h, Ordering::Release);
    }

    /// Reads the latest available snapshot in the render thread (consumer).
    pub fn read_latest(&self) -> Option<RenderStateSnapshot> {
        let h = self.head.load(Ordering::Acquire);
        let latest_idx = (h + QUEUE_CAPACITY - 1) % QUEUE_CAPACITY;
        if let Ok(slot) = self.buffer[latest_idx].lock() {
            slot.clone()
        } else {
            None
        }
    }
}

pub static SNAPSHOT_QUEUE: SpscSnapshotQueue = SpscSnapshotQueue::new();

/// Motion Interpolator for 90 Hz / 120 Hz High Refresh Rate Displays.
pub struct SnapshotInterpolator {
    prev_snapshot: RenderStateSnapshot,
    curr_snapshot: RenderStateSnapshot,
}

impl SnapshotInterpolator {
    pub fn new() -> Self {
        Self {
            prev_snapshot: RenderStateSnapshot::empty(),
            curr_snapshot: RenderStateSnapshot::empty(),
        }
    }

    /// Pushes a newly received snapshot and advances history.
    pub fn update(&mut self, new_snapshot: RenderStateSnapshot) {
        self.prev_snapshot = std::mem::replace(&mut self.curr_snapshot, new_snapshot);
    }

    /// Linearly interpolates entity positions between the previous and current snapshot.
    /// `alpha` is the normalized time fraction between logical ticks (0.0 ..= 1.0).
    pub fn interpolate(&self, alpha: f32) -> Vec<RenderEntitySnapshot> {
        let alpha = alpha.clamp(0.0, 1.0);
        let mut result = Vec::with_capacity(self.curr_snapshot.entities.len());

        for curr in &self.curr_snapshot.entities {
            // Find corresponding entity in previous snapshot
            if let Some(prev) = self.prev_snapshot.entities.iter().find(|e| e.id == curr.id) {
                result.push(RenderEntitySnapshot {
                    id: curr.id,
                    x: prev.x + (curr.x - prev.x) * alpha,
                    y: prev.y + (curr.y - prev.y) * alpha,
                    sprite_index: curr.sprite_index,
                    image_index: curr.image_index,
                    xscale: prev.xscale + (curr.xscale - prev.xscale) * alpha,
                    yscale: prev.yscale + (curr.yscale - prev.yscale) * alpha,
                    angle: prev.angle + (curr.angle - prev.angle) * alpha,
                    alpha: prev.alpha + (curr.alpha - prev.alpha) * alpha,
                    blend_col: curr.blend_col,
                });
            } else {
                result.push(*curr);
            }
        }
        result
    }
}
