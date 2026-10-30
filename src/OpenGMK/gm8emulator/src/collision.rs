//! 64-bit Bitmasking and ARM NEON vectorized collision detection routines.

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// Evaluates branchless bitwise overlap between two 64-bit mask words under an active validity mask.
#[inline(always)]
pub fn check_overlap_u64(w1: u64, w2: u64, mask: u64) -> bool {
    ((w1 & w2) & mask) != 0
}

/// Evaluates branchless 128-bit block intersection using ARM NEON SIMD intrinsics when available.
#[inline(always)]
pub fn check_overlap_128(w1: [u64; 2], w2: [u64; 2], mask: [u64; 2]) -> bool {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let v1 = vcombine_u64(vcreate_u64(w1[0]), vcreate_u64(w1[1]));
        let v2 = vcombine_u64(vcreate_u64(w2[0]), vcreate_u64(w2[1]));
        let vm = vcombine_u64(vcreate_u64(mask[0]), vcreate_u64(mask[1]));
        let vand1 = vandq_u8(vreinterpretq_u8_u64(v1), vreinterpretq_u8_u64(v2));
        let vand2 = vandq_u8(vand1, vreinterpretq_u8_u64(vm));
        vmaxvq_u8(vand2) != 0
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        ((w1[0] & w2[0]) & mask[0]) != 0 || ((w1[1] & w2[1]) & mask[1]) != 0
    }
}

/// Checks bitmask collision between two continuous Box<[u64]> bitmasks given row dimensions and shifts.
#[inline(always)]
pub fn check_row_overlap(row_a: u64, row_b: u64, shift_x: u32) -> bool {
    let overlap = if shift_x == 0 {
        row_a & row_b
    } else if shift_x < 64 {
        row_a & (row_b >> shift_x)
    } else {
        0
    };
    overlap != 0
}
