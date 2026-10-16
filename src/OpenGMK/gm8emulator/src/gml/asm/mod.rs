//! AArch64 Assembly Dispatch for Ultra-Low-Latency GML Execution.

#[cfg(all(target_arch = "aarch64", target_os = "android"))]
core::arch::global_asm!(include_str!("dispatch_aarch64.s"));

#[cfg(all(target_arch = "aarch64", target_os = "android"))]
extern "C" {
    pub fn opengmk_dispatch_aarch64(
        stack_ptr: *mut usize,
        jump_table: *const usize,
        insn_ptr: *const u32,
        count: usize,
    ) -> usize;
}

#[cfg(not(all(target_arch = "aarch64", target_os = "android")))]
pub unsafe fn opengmk_dispatch_aarch64(
    _stack_ptr: *mut usize,
    _jump_table: *const usize,
    _insn_ptr: *const u32,
    _count: usize,
) -> usize {
    0
}

pub fn is_aarch64_dispatch_available() -> bool {
    cfg!(all(target_arch = "aarch64", target_os = "android"))
}
