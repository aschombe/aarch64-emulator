// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::types::Word;

/// Result type used by all ALU operations.
pub type AluResult = (Word, bool, bool);

/// Addition with carry and overflow detection.
pub fn add(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    let (result, carry) = if is_w {
        let (r, c) = (val_n as u32).overflowing_add(val_m as u32);
        (r as u64, c)
    } else {
        val_n.overflowing_add(val_m)
    };

    let overflow = if is_w {
        let sn = val_n as i32 as i64;
        let sm = val_m as i32 as i64;
        let sr = (result as u32 as i32) as i64;
        ((sn ^ sr) & (sm ^ sr)) < 0
    } else {
        let sn = val_n as i64;
        let sm = val_m as i64;
        let sr = result as i64;
        ((sn ^ sr) & (sm ^ sr)) < 0
    };

    (result, carry, overflow)
}

/// Subtraction with signed overflow and borrow handling.
pub fn sub(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if is_w {
        let (res, borrow) = (val_n as u32).overflowing_sub(val_m as u32);
        let res64 = res as u64;

        // ARM: Carry = NOT borrow
        let carry = !borrow;

        // Signed overflow detection
        let n_sign = ((val_n >> 31) & 1) != 0;
        let m_sign = ((val_m >> 31) & 1) != 0;
        let r_sign = ((res64 >> 31) & 1) != 0;
        let overflow = (n_sign != m_sign) && (n_sign != r_sign);

        (res64, carry, overflow)
    } else {
        let (res, borrow) = val_n.overflowing_sub(val_m);
        let carry = !borrow;

        let n_sign = ((val_n >> 63) & 1) != 0;
        let m_sign = ((val_m >> 63) & 1) != 0;
        let r_sign = ((res >> 63) & 1) != 0;
        let overflow = (n_sign != m_sign) && (n_sign != r_sign);

        (res, carry, overflow)
    }
}

/// Multiplication (no overflow flags).
pub fn mul(val_n: Word, val_m: Word, _is_w: bool) -> AluResult {
    (val_n.wrapping_mul(val_m), false, false)
}

/// Unsigned division.
pub fn udiv(val_n: Word, val_m: Word, _is_w: bool) -> AluResult {
    if val_m == 0 {
        (0, false, false)
    } else {
        (val_n / val_m, false, false)
    }
}

/// Signed division.
pub fn sdiv(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if val_m == 0 {
        (0, false, false)
    } else if is_w {
        let n = val_n as i32;
        let m = val_m as i32;
        (n.wrapping_div(m) as u32 as u64, false, false)
    } else {
        let n = val_n as i64;
        let m = val_m as i64;
        (n.wrapping_div(m) as u64, false, false)
    }
}

/// Bitwise AND.
pub fn and(val_n: Word, val_m: Word, _is_w: bool) -> AluResult {
    (val_n & val_m, false, false)
}

/// Bitwise OR.
pub fn orr(val_n: Word, val_m: Word, _is_w: bool) -> AluResult {
    (val_n | val_m, false, false)
}

/// Bitwise XOR.
pub fn eor(val_n: Word, val_m: Word, _is_w: bool) -> AluResult {
    (val_n ^ val_m, false, false)
}

/// Logical shift left.
pub fn lsl(val: Word, shift: Word, is_w: bool) -> AluResult {
    let sh = (shift & 0xFF) as u32;
    let mask = if is_w {
        0xFFFF_FFFF
    } else {
        0xFFFF_FFFF_FFFF_FFFF
    };
    ((val.wrapping_shl(sh) & mask), false, false)
}

/// Logical shift right.
pub fn lsr(val: Word, shift: Word, _is_w: bool) -> AluResult {
    let sh = (shift & 0xFF) as u32;
    (val.wrapping_shr(sh), false, false)
}

/// Arithmetic shift right.
pub fn asr(val: Word, shift: Word, is_w: bool) -> AluResult {
    let sh = (shift & 0xFF) as u32;
    if is_w {
        let signed = val as i32;
        ((signed.wrapping_shr(sh) as u32) as u64, false, false)
    } else {
        let signed = val as i64;
        ((signed.wrapping_shr(sh)) as u64, false, false)
    }
}
