// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{InstructionIR, OpCode, Operand};
use crate::cpu::InterpretedProgram;
use crate::types::VERBOSE_ENABLED;

/// Optimizes the instructions of a program in-place
pub fn optimize(prog: &mut InterpretedProgram) {
    remove_redundant_movs(prog);
}

/// Removes redundant 'mov reg, reg' instructions
fn remove_redundant_movs(prog: &mut InterpretedProgram) {
    prog.instructions.retain(|ir| match &ir.opcode {
        OpCode::MOV(_) => {
            if let [Operand::Reg(dest), Operand::Reg(src)] = &ir.operands[..] {
                if dest == src {
                    if VERBOSE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                        println!(
                            "[OPTIMIZER] Removed redundant instruction: {}",
                            ir.to_asm_string()
                        );
                    }
                    false
                } else {
                    true
                }
            } else {
                true
            }
        }
        _ => true,
    });
}
