// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{Immediate, Offset, OpCode, Operand};
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
    fn execute_cmn(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_tst(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_neg(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool>;
    fn execute_swp(
        &mut self,
        opcode: crate::assembler::asm_types::OpCode,
        operands: &[Operand],
    ) -> EmuResult<bool>;
    fn execute_minmax(
        &mut self,
        operands: &[Operand],
        is_signed: bool,
        is_max: bool,
    ) -> EmuResult<bool>;
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

    fn execute_cmn(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [Operand::Reg(r1), Operand::Imm(imm)] => {
                let a = self.get_reg(r1.to_id());
                let b = match imm {
                    Immediate::Lit(val) => *val as Word,
                    Immediate::Lbl(_) | Immediate::Lo12Lbl(_) => {
                        return Err(EmuError::InternalError(format!(
                            "CMN does not support label immediates"
                        )));
                    }
                };
                let res = a.wrapping_add(b);
                let is_w = r1.is_w_register();
                let (_ignored_result, carry, overflow) = alu::add(a, b, is_w);
                self.update_cpsr_nzcv(res, carry, overflow, false, is_w);
                Ok(false)
            }
            [Operand::Reg(r1), Operand::Reg(r2)] => {
                let a = self.get_reg(r1.to_id());
                let b = self.get_reg(r2.to_id());
                let res = a.wrapping_add(b);
                let is_w = r1.is_w_register();
                let (_ignored_result, carry, overflow) = alu::add(a, b, is_w);
                self.update_cpsr_nzcv(res, carry, overflow, false, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid CMN operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_tst(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [src1, src2] => {
                let val_n = self.resolve_operand_source(src1)?;
                let val_m = self.resolve_operand_source(src2)?;
                let is_w = matches!(src1, Operand::Reg(r) if r.is_w_register());
                let (result, carry, overflow) = alu::and(val_n, val_m, is_w);
                self.update_cpsr_nzcv(result, carry, overflow, false, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Bad TST operands".into())),
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

    fn execute_swp(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match opcode {
            OpCode::SWP => match operands {
                [dest_op, src_op, Operand::Offset(Offset::Ind2(addr_reg))] => {
                    let (dest_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let src_val = self.resolve_operand_source(src_op)?;
                    let addr = self.get_reg(addr_reg.to_id());

                    let old_val = self.memory.borrow().read_word(addr)?;
                    self.memory.borrow_mut().write_word(addr, src_val)?;
                    self.set_reg_with_width(dest_id, old_val, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid SWP operands".into())),
            },
            OpCode::SWPB => match operands {
                [dest_op, src_op, Operand::Offset(Offset::Ind2(addr_reg))] => {
                    let (dest_id, _is_w) = self.resolve_operand_dest(dest_op)?;
                    let src_val = (self.resolve_operand_source(src_op)? & 0xFF) as u8;
                    let addr = self.get_reg(addr_reg.to_id());

                    let old_val = self.memory.borrow().read_byte(addr)? as Word;
                    self.memory.borrow_mut().write_byte(addr, src_val)?;
                    self.set_reg_with_width(dest_id, old_val, true);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid SWPB operands".into())),
            },
            OpCode::SWPH => match operands {
                [dest_op, src_op, Operand::Offset(Offset::Ind2(addr_reg))] => {
                    let (dest_id, _is_w) = self.resolve_operand_dest(dest_op)?;
                    let src_val = (self.resolve_operand_source(src_op)? & 0xFFFF) as u16;
                    let addr = self.get_reg(addr_reg.to_id());

                    let old_val = self.memory.borrow().read_halfword(addr)? as Word;
                    self.memory
                        .borrow_mut()
                        .write_halfword(addr, src_val as u32)?;
                    self.set_reg_with_width(dest_id, old_val, true);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid SWPH operands".into())),
            },
            OpCode::SWPP => match operands {
                [
                    dest_op1,
                    dest_op2,
                    src_op1,
                    src_op2,
                    Operand::Offset(Offset::Ind2(addr_reg)),
                ] => {
                    let (rd1, _) = self.resolve_operand_dest(dest_op1)?;
                    let (rd2, _) = self.resolve_operand_dest(dest_op2)?;
                    let src1 = self.resolve_operand_source(src_op1)?;
                    let src2 = self.resolve_operand_source(src_op2)?;
                    let base_addr = self.get_reg(addr_reg.to_id());

                    let old1 = self.memory.borrow().read_word(base_addr)?;
                    let old2 = self.memory.borrow().read_word(base_addr + 8)?;
                    self.memory.borrow_mut().write_word(base_addr, src1)?;
                    self.memory.borrow_mut().write_word(base_addr + 8, src2)?;
                    self.set_reg_with_width(rd1, old1, false);
                    self.set_reg_with_width(rd2, old2, false);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid SWPP operands".into())),
            },
            _ => Err(EmuError::InternalError("Invalid SWP opcode".into())),
        }
    }

    fn execute_minmax(
        &mut self,
        operands: &[Operand],
        is_signed: bool,
        is_max: bool,
    ) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let a = self.resolve_operand_source(src1)?;
                let b = self.resolve_operand_source(src2)?;
                let result = alu::minmax(a, b, is_w, is_signed, is_max);
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(
                "Invalid operands for min/max.".to_string(),
            )),
        }
    }
}
