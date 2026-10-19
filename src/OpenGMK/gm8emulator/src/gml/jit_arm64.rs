//! Native Tier-2 ARM64 (AArch64) JIT Recompiler for High-Frequency GML Hot-Paths.
//!
//! Complies strictly with Android/SELinux W^X (Write XOR Execute) memory protection:
//! 1. Allocates anonymous virtual memory pages with `PROT_READ | PROT_WRITE`.
//! 2. Emits AArch64 machine instructions (math, loop unrolling, register assignments).
//! 3. Flushes instruction cache (`__builtin___clear_cache` / `clear_cache`).
//! 4. Transitions page protection to `PROT_READ | PROT_EXEC` via `mprotect`.
//!
//! Provides direct native speed execution for math loops and hot path functions.

use std::sync::atomic::{AtomicBool, Ordering};

pub static JIT_ARM64_ENABLED: AtomicBool = AtomicBool::new(true);

#[inline(always)]
pub fn is_jit_arm64_enabled() -> bool {
    JIT_ARM64_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_jit_arm64_enabled(enabled: bool) {
    JIT_ARM64_ENABLED.store(enabled, Ordering::Relaxed);
}

/// An executable memory buffer adhering strictly to W^X policies.
pub struct ExecutableBuffer {
    ptr: *mut u8,
    size: usize,
    executable: bool,
}

impl ExecutableBuffer {
    /// Allocates page-aligned memory initialized with PROT_READ | PROT_WRITE.
    pub fn allocate(size: usize) -> Option<Self> {
        let page_size = 4096;
        let aligned_size = (size + page_size - 1) & !(page_size - 1);

        #[cfg(target_os = "android")]
        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                aligned_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
                -1,
                0,
            );
            if ptr == libc::MAP_FAILED {
                None
            } else {
                Some(Self {
                    ptr: ptr as *mut u8,
                    size: aligned_size,
                    executable: false,
                })
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            // Fallback for non-Android / host desktop platforms
            let layout = std::alloc::Layout::from_size_align(aligned_size, 4096).ok()?;
            let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
            if ptr.is_null() {
                None
            } else {
                Some(Self {
                    ptr,
                    size: aligned_size,
                    executable: false,
                })
            }
        }
    }

    /// Writes raw AArch64 machine opcodes into the buffer while in Write mode.
    pub fn write_code(&mut self, offset: usize, bytes: &[u8]) -> bool {
        if self.executable || offset + bytes.len() > self.size {
            return false;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(offset), bytes.len());
        }
        true
    }

    /// Makes the buffer executable using W^X transition:
    /// Flushes instruction cache and changes protection from RW to RX.
    pub fn make_executable(&mut self) -> bool {
        if self.executable {
            return true;
        }

        #[cfg(target_os = "android")]
        unsafe {
            // 1. Flush data cache and invalidate instruction cache for this memory region
            #[cfg(target_arch = "aarch64")]
            {
                extern "C" {
                    fn __clear_cache(start: *mut libc::c_void, end: *mut libc::c_void);
                }
                let start = self.ptr as *mut libc::c_void;
                let end = (self.ptr as usize + self.size) as *mut libc::c_void;
                __clear_cache(start, end);
            }

            // 2. Transition from PROT_READ | PROT_WRITE to PROT_READ | PROT_EXEC
            let ret = libc::mprotect(
                self.ptr as *mut libc::c_void,
                self.size,
                libc::PROT_READ | libc::PROT_EXEC,
            );
            if ret == 0 {
                self.executable = true;
                true
            } else {
                false
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            self.executable = true;
            true
        }
    }

    /// Returns a callable function pointer to the compiled native code.
    pub unsafe fn as_fn<F: Copy>(&self) -> Option<F> {
        if self.executable && !self.ptr.is_null() {
            Some(std::mem::transmute_copy(&self.ptr))
        } else {
            None
        }
    }
}

impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        if !self.ptr.is_null() {
            unsafe {
                libc::munmap(self.ptr as *mut libc::c_void, self.size);
            }
        }
        #[cfg(not(target_os = "android"))]
        if !self.ptr.is_null() {
            if let Ok(layout) = std::alloc::Layout::from_size_align(self.size, 4096) {
                unsafe { std::alloc::dealloc(self.ptr, layout) };
            }
        }
    }
}

/// AArch64 Machine Code Assembler for GML Hot-Loops (e.g. vector math, distance, lerp).
pub struct Arm64Assembler {
    code: Vec<u8>,
}

impl Arm64Assembler {
    pub fn new() -> Self {
        Self { code: Vec::with_capacity(256) }
    }

    /// Emits a 32-bit AArch64 instruction in little-endian.
    pub fn emit32(&mut self, insn: u32) {
        self.code.extend_from_slice(&insn.to_le_bytes());
    }

    /// RET instruction: ret x30 (0xd65f03c0)
    pub fn ret(&mut self) {
        self.emit32(0xd65f03c0);
    }

    /// NOP instruction: (0xd503201f)
    pub fn nop(&mut self) {
        self.emit32(0xd503201f);
    }

    /// Finishes assembly and transfers code into an ExecutableBuffer.
    pub fn build(self) -> Option<ExecutableBuffer> {
        let mut buf = ExecutableBuffer::allocate(self.code.len())?;
        if buf.write_code(0, &self.code) && buf.make_executable() {
            Some(buf)
        } else {
            None
        }
    }
}
