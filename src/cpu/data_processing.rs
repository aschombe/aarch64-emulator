// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{Immediate, Operand};
use crate::cpu::alu;
use crate::cpu::state::CpuState;
use crate::types::{EmuError, EmuResult, Word};

/// Trait for data processing instructions: ALU ops, CMP, NEG.
pub trait InstructionDataProcessing {
    fn execute_binary_op(
        &mut self,
        operands: &[Operand],
        opfunc: fn(Word, Word, bool) -> (Word, bool, bool),
        update_flags: bool,
        is_sub: bool,
    ) -> EmuResult<bool>;
    fn execute_cmp(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_neg(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool>;
}

impl InstructionDataProcessing for CpuState {
    fn execute_binary_op(
        &mut self,
        operands: &[Operand],
        opfunc: fn(Word, Word, bool) -> (Word, bool, bool),
        update_flags: bool,
        is_sub: bool,
    ) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let val_n = self.resolve_operand_source(src1)?;
                let val_m = match src2 {
                    Operand::Imm(Immediate::Lit(v)) => *v as Word,
                    _ => self.resolve_operand_source(src2)?,
                };
                let (result, carry, overflow) = opfunc(val_n, val_m, is_w);
                self.set_reg_with_width(rd, result, is_w);

                if update_flags {
                    self.update_cpsr_nzcv(result, carry, overflow, is_sub, is_w);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Bad binary op".into())),
        }
    }

    fn execute_cmp(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        if let [src1, src2] = operands {
            let val_n = self.resolve_operand_source(src1)?;
            let val_m = self.resolve_operand_source(src2)?;
            let is_w = matches!(src1, Operand::Reg(r) if r.is_w_register());
            let (res, carry, overflow) = alu::sub(val_n, val_m, is_w);
            self.update_cpsr_nzcv(res, carry, overflow, true, is_w);

            Ok(false)
        } else {
            Err(EmuError::InternalError("Invalid CMP".into()))
        }
    }

    fn execute_neg(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool> {
        match operands {
            [dest, src] => {
                let (rdid, isw) = self.resolve_operand_dest(dest)?;
                let srcval = self.resolve_operand_source(src)?;
                // Perform subtraction: result = 0 - srcval
                let (result, carry, overflow) = alu::sub(0, srcval, isw);
                self.set_reg_with_width(rdid, result, isw);
                if update_flags {
                    self.update_cpsr_nzcv(result, carry, overflow, true, isw);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid operands for NEG/NEGS: {:?}",
                operands
            ))),
        }
    }
}
