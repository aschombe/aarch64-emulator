// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::cpu::state::CpuState;
use crate::types::Word;

pub const N_FLAG: Word = 1 << 31; // Negative flag
pub const Z_FLAG: Word = 1 << 30; // Zero flag
pub const C_FLAG: Word = 1 << 29; // Carry flag
pub const V_FLAG: Word = 1 << 28; // Overflow flag
pub const Q_FLAG: Word = 1 << 27; // Saturation flag

impl CpuState {
    pub fn update_cpsr_nzcv(
        &mut self,
        result: Word,
        carry: bool,
        overflow: bool,
        is_sub: bool,
        is_w: bool,
    ) {
        let mut new_state = 0;

        let sign_bit = if is_w { 31 } else { 63 };

        if (result >> sign_bit) & 1 == 1 {
            new_state |= N_FLAG;
        }
        if result == 0 {
            new_state |= Z_FLAG;
        }
        if is_sub {
            if !carry {
                new_state |= C_FLAG;
            }
        } else if carry {
            new_state |= C_FLAG;
        }
        if overflow {
            new_state |= V_FLAG;
        }
        *self.cpsr.borrow_mut() = new_state;
    }
}
