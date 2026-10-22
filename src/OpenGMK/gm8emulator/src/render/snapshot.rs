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

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdKind {
    Sprite = 0,
    Rect = 1,
    Line = 2,
    Text = 3,
    Tilemap = 4,
    Particle = 5,
    SurfaceBlit = 6,
    Raw = 7,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Normal = 0,
    Add = 1,
    Subtract = 2,
    Max = 3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DrawCmd {
    pub sort_key: u64,        // (depth_biased:u32 << 32) | seq:u32
    pub kind: CmdKind,
    pub blend: BlendMode,
    pub atlas_layer: u8,
    pub flags: u8,
    pub a: [f32; 8],          // payload: x,y,xs,ys,ang,u0,v0,u1,v1
    pub color: u32,
    pub extra: u32,
}

pub struct FrameSnapshot {
    pub frame_id: u64,
    pub view_x: f32,
    pub view_y: f32,
    pub view_w: f32,
    pub view_h: f32,
    pub cmds: Vec<DrawCmd>,
    pub text_arena: Vec<u8>,
    pub static_layers_visible: Vec<u16>,
}

impl FrameSnapshot {
    pub fn with_capacity(cmd_cap: usize) -> Self {
        Self {
            frame_id: 0,
            view_x: 0.0,
            view_y: 0.0,
            view_w: 640.0,
            view_h: 480.0,
            cmds: Vec::with_capacity(cmd_cap),
            text_arena: Vec::with_capacity(2048),
            static_layers_visible: Vec::with_capacity(32),
        }
    }

    #[inline]
    pub fn begin(&mut self, frame_id: u64, vx: f32, vy: f32, vw: f32, vh: f32) {
        self.frame_id = frame_id;
        self.view_x = vx;
        self.view_y = vy;
        self.view_w = vw;
        self.view_h = vh;
        self.cmds.clear();
        self.text_arena.clear();
        self.static_layers_visible.clear();
    }
}

const DIRTY_BIT: u8 = 0b0100;
const IDX_MASK: u8 = 0b0011;

/// Truly lock-free, zero-mutex triple-buffered snapshot queue.
pub struct SpscSnapshotQueue {
    slots: [std::cell::UnsafeCell<Option<RenderStateSnapshot>>; 3],
    middle: std::sync::atomic::AtomicU8,
    back_idx: std::sync::atomic::AtomicU8,
    front_idx: std::sync::atomic::AtomicU8,
}

unsafe impl Sync for SpscSnapshotQueue {}

impl SpscSnapshotQueue {
    pub const fn new() -> Self {
        const INIT: std::cell::UnsafeCell<Option<RenderStateSnapshot>> = std::cell::UnsafeCell::new(None);
        Self {
            slots: [INIT, INIT, INIT],
            middle: std::sync::atomic::AtomicU8::new(1),
            back_idx: std::sync::atomic::AtomicU8::new(0),
            front_idx: std::sync::atomic::AtomicU8::new(2),
        }
    }

    /// Publishes a snapshot from logic thread without locks. Reuses preallocated entities capacity.
    pub fn publish(&self, snapshot: RenderStateSnapshot) {
        let back = self.back_idx.load(Ordering::Relaxed);
        unsafe {
            *self.slots[back as usize].get() = Some(snapshot);
        }
        let prev = self.middle.swap(back | DIRTY_BIT, Ordering::AcqRel);
        self.back_idx.store(prev & IDX_MASK, Ordering::Release);
    }

    /// Reads latest snapshot in consumer thread if a fresh frame is ready. Zero-copy swap.
    pub fn read_latest(&self) -> Option<RenderStateSnapshot> {
        if self.middle.load(Ordering::Relaxed) & DIRTY_BIT == 0 {
            return None;
        }
        let front = self.front_idx.load(Ordering::Relaxed);
        let prev = self.middle.swap(front, Ordering::AcqRel);
        let new_front = prev & IDX_MASK;
        self.front_idx.store(new_front, Ordering::Release);
        unsafe {
            (*self.slots[new_front as usize].get()).clone()
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
