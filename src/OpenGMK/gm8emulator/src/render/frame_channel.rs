//! Lock-free Triple Buffer Frame Channel with Atomic Index Exchange (Zero-Copy / Zero-Alloc).
//!
//! Eliminates thread contention between Game Logic thread and Render Loop.
//! Producers write to an exclusive back slot without locks.
//! Consumers acquire the latest completed front slot without waiting.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const IDX_MASK: u8 = 0b0011;
const DIRTY: u8 = 0b0100;

#[repr(align(64))]
struct Slot<T>(UnsafeCell<T>);

#[repr(align(64))]
struct PaddedAtomic(AtomicU8);

struct Shared<T> {
    slots: [Slot<T>; 3],
    middle: PaddedAtomic, // index of middle slot | DIRTY bit if new frame ready
}

unsafe impl<T: Send> Sync for Shared<T> {}

pub struct FrameProducer<T> {
    shared: Arc<Shared<T>>,
    back: u8,
}

pub struct FrameConsumer<T> {
    shared: Arc<Shared<T>>,
    front: u8,
}

pub fn frame_channel<T: Send>(make: impl Fn() -> T) -> (FrameProducer<T>, FrameConsumer<T>) {
    let shared = Arc::new(Shared {
        slots: [
            Slot(UnsafeCell::new(make())),
            Slot(UnsafeCell::new(make())),
            Slot(UnsafeCell::new(make())),
        ],
        middle: PaddedAtomic(AtomicU8::new(1)),
    });
    (
        FrameProducer { shared: shared.clone(), back: 0 },
        FrameConsumer { shared, front: 2 },
    )
}

impl<T> FrameProducer<T> {
    /// Slot exclusively owned by producer. Callers reuse internal allocations (clear() without dealloc).
    #[inline]
    pub fn back_mut(&mut self) -> &mut T {
        unsafe { &mut *self.shared.slots[self.back as usize].0.get() }
    }

    /// Publishes the back slot and retrieves the previous middle slot. Lock-free & wait-free.
    #[inline]
    pub fn publish(&mut self) {
        let prev = self.shared.middle.0.swap(self.back | DIRTY, Ordering::AcqRel);
        self.back = prev & IDX_MASK;
    }
}

impl<T> FrameConsumer<T> {
    /// Returns true if a fresh unread frame was acquired.
    #[inline]
    pub fn try_acquire(&mut self) -> bool {
        if self.shared.middle.0.load(Ordering::Relaxed) & DIRTY == 0 {
            return false;
        }
        let prev = self.shared.middle.0.swap(self.front, Ordering::AcqRel);
        self.front = prev & IDX_MASK;
        true
    }

    /// Accesses the acquired front frame slot.
    #[inline]
    pub fn front(&self) -> &T {
        unsafe { &*self.shared.slots[self.front as usize].0.get() }
    }
}

/// SPSC Ring buffer for control events that cannot be lost (BakeTiles, TileDirty, Resize).
const CTRL_CAP: usize = 32;

pub enum ControlEvent {
    BakeTiles { depth: i32 },
    TileDirty { depth: i32 },
    Resize { width: u32, height: u32 },
    ConfigChanged,
}

pub struct ControlChannel {
    events: [std::sync::Mutex<Option<ControlEvent>>; CTRL_CAP],
    head: std::sync::atomic::AtomicUsize,
    tail: std::sync::atomic::AtomicUsize,
}

impl ControlChannel {
    pub const fn new() -> Self {
        const INIT: std::sync::Mutex<Option<ControlEvent>> = std::sync::Mutex::new(None);
        Self {
            events: [INIT; CTRL_CAP],
            head: std::sync::atomic::AtomicUsize::new(0),
            tail: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn send(&self, event: ControlEvent) {
        let h = self.head.load(Ordering::Relaxed);
        let next = (h + 1) % CTRL_CAP;
        if let Ok(mut slot) = self.events[h].lock() {
            *slot = Some(event);
        }
        self.head.store(next, Ordering::Release);
    }

    pub fn try_recv(&self) -> Option<ControlEvent> {
        let t = self.tail.load(Ordering::Relaxed);
        let h = self.head.load(Ordering::Acquire);
        if t == h {
            return None;
        }
        let event = if let Ok(mut slot) = self.events[t].lock() {
            slot.take()
        } else {
            None
        };
        self.tail.store((t + 1) % CTRL_CAP, Ordering::Release);
        event
    }
}

pub static CONTROL_CHANNEL: ControlChannel = ControlChannel::new();
