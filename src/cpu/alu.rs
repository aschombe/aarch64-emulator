// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::types::Word;

/// Result type used by all ALU operations.
pub type AluResult = (Word, bool, bool);

/// Addition with carry and overflow detection.
pub fn add(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    let (raw_result, carry) = if is_w {
        let (r, c) = (val_n as u32).overflowing_add(val_m as u32);
        (r as u64, c)
    } else {
        val_n.overflowing_add(val_m)
    };

    let result = raw_result;

    let overflow = if is_w {
        let n_i = val_n as i32;
        let m_i = val_m as i32;
        let r_i = raw_result as i32;
        ((n_i >= 0) == (m_i >= 0)) && ((n_i >= 0) != (r_i >= 0))
    } else {
        let n_i = val_n as i64;
        let m_i = val_m as i64;
        let r_i = raw_result as i64;
        ((n_i >= 0) == (m_i >= 0)) && ((n_i >= 0) != (r_i >= 0))
    };

    (result, carry, overflow)
}

/// Subtraction with signed overflow and borrow handling.
pub fn sub(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if is_w {
        let n_32 = val_n as u32;
        let m_32 = val_m as u32;

        let (res_32, borrow) = n_32.overflowing_sub(m_32);
        let res64 = res_32 as u64;

        let carry = !borrow;

        let n_i = n_32 as i32;
        let m_i = m_32 as i32;
        let r_i = res_32 as i32;

        let m_i_comp = m_i.wrapping_neg();
        let overflow = ((n_i >= 0) == (m_i_comp >= 0)) && ((n_i >= 0) != (r_i >= 0));

        (res64, carry, overflow)
    } else {
        let (res, borrow) = val_n.overflowing_sub(val_m);
        let carry = !borrow;

        let n_i = val_n as i64;
        let m_i = val_m as i64;
        let r_i = res as i64;

        let m_i_comp = m_i.wrapping_neg();
        let overflow = ((n_i >= 0) == (m_i_comp >= 0)) && ((n_i >= 0) != (r_i >= 0));

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

/// Unsigned multiplication producing double-width result.
pub fn umull(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if is_w {
        let n = val_n as u32 as u64;
        let m = val_m as u32 as u64;
        (n.wrapping_mul(m), false, false)
    } else {
        (val_n.wrapping_mul(val_m), false, false)
    }
}

/// Signed multiplication producing double-width result.
pub fn smull(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if is_w {
        let n = val_n as i32 as i64;
        let m = val_m as i32 as i64;
        (n.wrapping_mul(m) as u64, false, false)
    } else {
        let n = val_n as i64;
        let m = val_m as i64;
        (n.wrapping_mul(m) as u64, false, false)
    }
}

/// Unsigned multiplication high half.
pub fn umulh(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if is_w {
        let n = val_n as u32 as u64;
        let m = val_m as u32 as u64;
        let product = n.wrapping_mul(m);
        (product >> 32, false, false)
    } else {
        let n_hi = val_n >> 32;
        let n_lo = val_n & 0xFFFF_FFFF;
        let m_hi = val_m >> 32;
        let m_lo = val_m & 0xFFFF_FFFF;

        let cross = n_hi
            .wrapping_mul(m_lo)
            .wrapping_add(n_lo.wrapping_mul(m_hi));
        let high = n_hi.wrapping_mul(m_hi).wrapping_add(cross >> 32);

        (high, false, false)
    }
}

/// Signed multiplication high half.
pub fn smulh(val_n: Word, val_m: Word, is_w: bool) -> AluResult {
    if is_w {
        let n = val_n as i32 as i64;
        let m = val_m as i32 as i64;
        let product = n.wrapping_mul(m);
        (product as u64 >> 32, false, false)
    } else {
        let n_i = val_n as i64;
        let m_i = val_m as i64;
        let product = n_i.wrapping_mul(m_i);
        (product as u64 >> 32, false, false)
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

/// Rotate right.
pub fn ror(val: Word, shift: Word, is_w: bool) -> AluResult {
    let sh = (shift & 0xFF) as u32;
    if is_w {
        let v = val as u32;
        ((v.rotate_right(sh) as u32) as u64, false, false)
    } else {
        (val.rotate_right(sh), false, false)
    }
}

/// Minimum or maximum of two values.
pub fn minmax(a: u64, b: u64, is_w: bool, is_signed: bool, is_max: bool) -> u64 {
    if is_signed {
        if is_w {
            let a = a as i32;
            let b = b as i32;
            if is_max {
                a.max(b) as u32 as u64
            } else {
                a.min(b) as u32 as u64
            }
        } else {
            let a = a as i64;
            let b = b as i64;
            if is_max {
                a.max(b) as u64
            } else {
                a.min(b) as u64
            }
        }
    } else {
        if is_w {
            let a = a as u32;
            let b = b as u32;
            if is_max {
                a.max(b) as u64
            } else {
                a.min(b) as u64
            }
        } else {
            if is_max { a.max(b) } else { a.min(b) }
        }
    }
}

pub fn adc(val_n: Word, val_m: Word, carry_in: Word, is_w: bool) -> (Word, bool, bool) {
    let (sum1, carry1): (u64, bool) = if is_w {
        let (r, c) = (val_n as u32).overflowing_add(val_m as u32);
        (r as u64, c)
    } else {
        val_n.overflowing_add(val_m)
    };
    let (sum2, carry2): (u64, bool) = if is_w {
        let (r, c) = (sum1 as u32).overflowing_add(carry_in as u32);
        (r as u64, c)
    } else {
        sum1.overflowing_add(carry_in)
    };
    let result = if is_w { sum2 as u32 as u64 } else { sum2 };
    let carry = carry1 || carry2;
    let overflow = if is_w {
        let n = val_n as i32;
        let m = val_m as i32;
        // let c = carry_in as i32;
        let r = result as i32;
        ((n >= 0) == (m >= 0)) && ((n >= 0) != (r >= 0))
    } else {
        let n = val_n as i64;
        let m = val_m as i64;
        // let c = carry_in as i64;
        let r = result as i64;
        ((n >= 0) == (m >= 0)) && ((n >= 0) != (r >= 0))
    };
    (result, carry, overflow)
}

pub fn sbc(val_n: Word, val_m: Word, carry_in: Word, is_w: bool) -> (Word, bool, bool) {
    let borrow = if carry_in == 1 { 0 } else { 1 };
    let to_subtract = val_m + borrow;
    let (result, carry, overflow) = if is_w {
        let (res, carry) = (val_n as u32).overflowing_sub(to_subtract as u32);
        let n = val_n as i32;
        let m = to_subtract as i32;
        let r = res as i32;
        let overflow = ((n >= 0) == (-(m) >= 0)) && ((n >= 0) != (r >= 0));
        (res as u64, !carry, overflow)
    } else {
        let (res, carry) = val_n.overflowing_sub(to_subtract);
        let n = val_n as i64;
        let m = to_subtract as i64;
        let r = res as i64;
        let overflow = ((n >= 0) == (-(m) >= 0)) && ((n >= 0) != (r >= 0));
        (res, !carry, overflow)
    };
    (result, carry, overflow)
}

pub fn ngc(val_m: Word, carry_in: Word, is_w: bool) -> (Word, bool, bool) {
    // Like sbc but val_n == 0
    sbc(0, val_m, carry_in, is_w)
}
