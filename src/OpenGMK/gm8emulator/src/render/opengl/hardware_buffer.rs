//! Safe Encapsulated AHardwareBuffer Zero-Copy Staging for Android (API 26+).
//!
//! Provides direct DMA-accessible shared memory between CPU and GPU.
//! Dynamically loads symbols from `libandroid.so` via `dlsym` to guarantee 100% safety
//! and zero regressions on older Android versions or desktop operating systems.

use std::{ffi::c_void, ptr};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AHardwareBufferDesc {
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub format: u32,
    pub usage: u64,
    pub stride: u32,
    pub rfu0: u32,
    pub rfu1: u64,
}

// Android HardwareBuffer formats and usage flags
pub const AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM: u32 = 1;
pub const AHARDWAREBUFFER_USAGE_CPU_READ_OFTEN: u64 = 0x02;
pub const AHARDWAREBUFFER_USAGE_CPU_WRITE_OFTEN: u64 = 0x20;
pub const AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE: u64 = 0x100;

type AHardwareBufferAllocateFn = unsafe extern "C" fn(*const AHardwareBufferDesc, *mut *mut c_void) -> i32;
type AHardwareBufferReleaseFn = unsafe extern "C" fn(*mut c_void);
type AHardwareBufferLockFn = unsafe extern "C" fn(*mut c_void, u64, i32, *const c_void, *mut *mut c_void) -> i32;
type AHardwareBufferUnlockFn = unsafe extern "C" fn(*mut c_void, *mut i32) -> i32;
type AHardwareBufferDescribeFn = unsafe extern "C" fn(*const c_void, *mut AHardwareBufferDesc);

struct HardwareBufferApi {
    allocate: AHardwareBufferAllocateFn,
    release: AHardwareBufferReleaseFn,
    lock: AHardwareBufferLockFn,
    unlock: AHardwareBufferUnlockFn,
    describe: AHardwareBufferDescribeFn,
}

static API: std::sync::OnceLock<Option<HardwareBufferApi>> = std::sync::OnceLock::new();

fn get_api() -> Option<&'static HardwareBufferApi> {
    API.get_or_init(|| {
        #[cfg(target_os = "android")]
        unsafe {
            let lib = libc::dlopen(b"libandroid.so\0".as_ptr() as *const libc::c_char, libc::RTLD_NOW);
            if lib.is_null() {
                return None;
            }
            let alloc = libc::dlsym(lib, b"AHardwareBuffer_allocate\0".as_ptr() as *const libc::c_char);
            let rel = libc::dlsym(lib, b"AHardwareBuffer_release\0".as_ptr() as *const libc::c_char);
            let lock = libc::dlsym(lib, b"AHardwareBuffer_lock\0".as_ptr() as *const libc::c_char);
            let unlock = libc::dlsym(lib, b"AHardwareBuffer_unlock\0".as_ptr() as *const libc::c_char);
            let desc = libc::dlsym(lib, b"AHardwareBuffer_describe\0".as_ptr() as *const libc::c_char);

            if alloc.is_null() || rel.is_null() || lock.is_null() || unlock.is_null() || desc.is_null() {
                return None;
            }

            eprintln!("[ahardwarebuffer] API NDK nativa vinculada exitosamente. Zero-Copy activado.");
            Some(HardwareBufferApi {
                allocate: std::mem::transmute(alloc),
                release: std::mem::transmute(rel),
                lock: std::mem::transmute(lock),
                unlock: std::mem::transmute(unlock),
                describe: std::mem::transmute(desc),
            })
        }
        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }).as_ref()
}

/// Unified staging buffer that uses AHardwareBuffer when available, with automatic heap fallback.
pub struct StagingBuffer {
    buffer_ptr: *mut c_void,
    heap_fallback: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl StagingBuffer {
    pub fn is_hardware_supported() -> bool {
        get_api().is_some()
    }

    pub fn new(width: u32, height: u32) -> Self {
        if let Some(api) = get_api() {
            let desc = AHardwareBufferDesc {
                width,
                height,
                layers: 1,
                format: AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM,
                usage: AHARDWAREBUFFER_USAGE_CPU_WRITE_OFTEN
                    | AHARDWAREBUFFER_USAGE_CPU_READ_OFTEN
                    | AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE,
                stride: 0,
                rfu0: 0,
                rfu1: 0,
            };
            let mut handle: *mut c_void = ptr::null_mut();
            let res = unsafe { (api.allocate)(&desc, &mut handle) };
            if res == 0 && !handle.is_null() {
                let mut out_desc = desc;
                unsafe { (api.describe)(handle, &mut out_desc) };
                return Self {
                    buffer_ptr: handle,
                    heap_fallback: None,
                    width,
                    height,
                    stride: if out_desc.stride > 0 { out_desc.stride } else { width },
                };
            }
        }

        // Safe fallback to CPU vector
        Self {
            buffer_ptr: ptr::null_mut(),
            heap_fallback: Some(vec![0u8; (width * height * 4) as usize]),
            width,
            height,
            stride: width,
        }
    }

    /// Access mapped slice for zero-copy writing.
    pub fn with_slice_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8], u32) -> R,
    {
        if !self.buffer_ptr.is_null() {
            if let Some(api) = get_api() {
                let mut mapped: *mut c_void = ptr::null_mut();
                let res = unsafe {
                    (api.lock)(
                        self.buffer_ptr,
                        AHARDWAREBUFFER_USAGE_CPU_WRITE_OFTEN,
                        -1,
                        ptr::null(),
                        &mut mapped,
                    )
                };
                if res == 0 && !mapped.is_null() {
                    let len = (self.stride * self.height * 4) as usize;
                    let slice = unsafe { std::slice::from_raw_parts_mut(mapped as *mut u8, len) };
                    let ret = f(slice, self.stride);
                    let mut fence = -1;
                    unsafe { (api.unlock)(self.buffer_ptr, &mut fence) };
                    return ret;
                }
            }
        }

        let slice = self.heap_fallback.as_mut().unwrap();
        f(slice.as_mut_slice(), self.stride)
    }

    /// Read buffer as slice for texture upload.
    pub fn as_slice(&self) -> Option<&[u8]> {
        self.heap_fallback.as_deref()
    }
}

impl Drop for StagingBuffer {
    fn drop(&mut self) {
        if !self.buffer_ptr.is_null() {
            if let Some(api) = get_api() {
                unsafe { (api.release)(self.buffer_ptr) };
                self.buffer_ptr = ptr::null_mut();
            }
        }
    }
}
