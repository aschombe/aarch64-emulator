// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{Immediate, MovType, OpCode, Operand};
use crate::cpu::InterpretedProgram;
use crate::types::VERBOSE_ENABLED;

/// Optimizes the instructions of a program in-place
pub fn optimize(prog: &mut InterpretedProgram) {
    remove_redundant_movs(prog);
    remove_consecutive_movs(prog);
    simplify_arithmetic_operations(prog);
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

/// Removes consecutive MOV instructions that overwrite each other
fn remove_consecutive_movs(prog: &mut InterpretedProgram) {
    let mut optimized_instructions = Vec::new();
    let mut skip_next = false;

    for i in 0..prog.instructions.len() {
        if skip_next {
            skip_next = false;
            continue;
        }

        let current_ir = &prog.instructions[i];

        if i + 1 < prog.instructions.len() {
            let next_ir = &prog.instructions[i + 1];

            if let (OpCode::MOV(_), OpCode::MOV(_)) = (&current_ir.opcode, &next_ir.opcode) {
                if let ([Operand::Reg(dest1), _], [Operand::Reg(dest2), _]) =
                    (&current_ir.operands[..], &next_ir.operands[..])
                {
                    if dest1 == dest2 {
                        if VERBOSE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                            println!(
                                "[OPTIMIZER] Removed redundant instruction: {}",
                                current_ir.to_asm_string()
                            );
                        }
                        skip_next = true;
                        continue;
                    }
                }
            }
        }

        optimized_instructions.push(current_ir.clone());
    }

    prog.instructions = optimized_instructions;
}

/// Simplifies arithmetic operations with zero
/// E.g add x1, x2, 0 -> mov x1, x2
fn simplify_arithmetic_operations(prog: &mut InterpretedProgram) {
    for ir in &mut prog.instructions {
        match &ir.opcode {
            OpCode::ADD | OpCode::SUB => {
                if let [
                    Operand::Reg(dest),
                    Operand::Reg(src),
                    Operand::Imm(Immediate::Lit(0)),
                ] = &ir.operands[..]
                {
                    ir.opcode = OpCode::MOV(MovType::Normal);
                    ir.operands = vec![Operand::Reg(*dest), Operand::Reg(*src)];
                    if VERBOSE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                        println!(
                            "[OPTIMIZER] Simplified instruction to MOV: {}",
                            ir.to_asm_string()
                        );
                    }
                }
            }
            _ => {}
        }
    }
}
