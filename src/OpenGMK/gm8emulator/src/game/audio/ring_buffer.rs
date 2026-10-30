//! Lock-Free Single-Producer Single-Consumer (SPSC) Audio Ring Buffer.

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LockFreeAudioRing {
    buffer: Box<[f32]>,
    capacity: usize,
    mask: usize,
    head: AtomicUsize, // Escrito por el emulador (productor)
    tail: AtomicUsize, // Leído por el callback de AAudio (consumidor)
}

unsafe impl Send for LockFreeAudioRing {}
unsafe impl Sync for LockFreeAudioRing {}

impl LockFreeAudioRing {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.next_power_of_two().max(512);
        let buffer = vec![0.0f32; cap].into_boxed_slice();
        Self {
            buffer,
            capacity: cap,
            mask: cap - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(always)]
    pub fn available_read(&self) -> usize {
        let h = self.head.load(Ordering::Acquire);
        let t = self.tail.load(Ordering::Relaxed);
        h.wrapping_sub(t)
    }

    #[inline(always)]
    pub fn available_write(&self) -> usize {
        let h = self.head.load(Ordering::Relaxed);
        let t = self.tail.load(Ordering::Acquire);
        self.capacity.saturating_sub(h.wrapping_sub(t))
    }

    #[inline(always)]
    pub fn write_slice(&self, samples: &[f32]) -> usize {
        let h = self.head.load(Ordering::Relaxed);
        let t = self.tail.load(Ordering::Acquire);
        let free = self.capacity.saturating_sub(h.wrapping_sub(t));
        let count = samples.len().min(free);
        if count == 0 {
            return 0;
        }

        let ptr = self.buffer.as_ptr() as *mut f32;
        for (i, &sample) in samples[..count].iter().enumerate() {
            let idx = (h.wrapping_add(i)) & self.mask;
            unsafe {
                *ptr.add(idx) = sample;
            }
        }

        self.head.store(h.wrapping_add(count), Ordering::Release);
        count
    }

    #[inline(always)]
    pub fn read_slice(&self, out: &mut [f32]) -> usize {
        let t = self.tail.load(Ordering::Relaxed);
        let h = self.head.load(Ordering::Acquire);
        let avail = h.wrapping_sub(t);
        let count = out.len().min(avail);
        if count == 0 {
            return 0;
        }

        let ptr = self.buffer.as_ptr();
        for (i, slot) in out[..count].iter_mut().enumerate() {
            let idx = (t.wrapping_add(i)) & self.mask;
            unsafe {
                *slot = *ptr.add(idx);
            }
        }

        self.tail.store(t.wrapping_add(count), Ordering::Release);
        count
    }

    pub fn reset(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
    }
}
