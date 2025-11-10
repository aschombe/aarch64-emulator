// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{Condition, Immediate, Offset, OpCode, Operand, SelectOp};
use crate::cpu::{alu, control_flow::InstructionControl, state::CpuState};
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
    fn execute_adc(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool>;
    fn execute_sbc(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool>;
    fn execute_ngc(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool>;
    fn execute_abs(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_madd(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_msub(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_mneg(&mut self, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_wide_3op(
        &mut self,
        operands: &[Operand],
        op: fn(Word, Word, Word) -> Word,
    ) -> EmuResult<bool>;
    fn execute_wide_2op(
        &mut self,
        operands: &[Operand],
        op: fn(Word, Word) -> Word,
    ) -> EmuResult<bool>;
    fn execute_bitop(&mut self, operands: &[Operand], op: fn(u64, bool) -> u64) -> EmuResult<bool>;
    fn execute_csel_like(
        &mut self,
        operands: &[Operand],
        cond: crate::assembler::asm_types::Condition,
        op: SelectOp,
    ) -> EmuResult<bool>;
    fn execute_ccmp_reg(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool>;
    fn execute_ccmp_imm(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool>;
    fn execute_ccmn_reg(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool>;
    fn execute_ccmn_imm(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool>;
    fn execute_mvn(&mut self, operands: &[Operand]) -> EmuResult<bool>;
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

    fn execute_adc(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let val1 = self.resolve_operand_source(src1)?;
                let val2 = self.resolve_operand_source(src2)?;
                let carry = self.get_carry_flag();
                let (result, carry_out, overflow) = alu::adc(val1, val2, carry, is_w);
                self.set_reg_with_width(rd, result, is_w);
                if update_flags {
                    self.update_cpsr_nzcv(result, carry_out, overflow, false, is_w);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid ADC operands.".to_string())),
        }
    }

    fn execute_sbc(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let val1 = self.resolve_operand_source(src1)?;
                let val2 = self.resolve_operand_source(src2)?;
                let carry = self.get_carry_flag();
                let (result, carry_out, overflow) = alu::sbc(val1, val2, carry, is_w);
                self.set_reg_with_width(rd, result, is_w);
                if update_flags {
                    self.update_cpsr_nzcv(result, carry_out, overflow, true, is_w);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid SBC operands.".to_string())),
        }
    }

    fn execute_ngc(&mut self, operands: &[Operand], update_flags: bool) -> EmuResult<bool> {
        match operands {
            [dest, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let val2 = self.resolve_operand_source(src2)?;
                let carry = self.get_carry_flag();
                let (result, carry_out, overflow) = alu::ngc(val2, carry, is_w);
                self.set_reg_with_width(rd, result, is_w);
                if update_flags {
                    self.update_cpsr_nzcv(result, carry_out, overflow, true, is_w);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid NGC operands.".to_string())),
        }
    }

    fn execute_abs(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest, src] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let value = self.resolve_operand_source(src)?;
                let result = if is_w {
                    let sv = value as i32;
                    if sv == i32::MIN {
                        sv as u32 as u64
                    } else {
                        sv.abs() as u32 as u64
                    }
                } else {
                    let sv = value as i64;
                    if sv == i64::MIN {
                        sv as u64
                    } else {
                        sv.abs() as u64
                    }
                };
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(
                "Invalid operands for ABS".to_string(),
            )),
        }
    }

    fn execute_madd(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2, src3] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let n = self.resolve_operand_source(src1)?;
                let m = self.resolve_operand_source(src2)?;
                let a = self.resolve_operand_source(src3)?;
                let result = alu::madd(n, m, a, is_w);
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid MADD operands".to_string())),
        }
    }
    fn execute_msub(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2, src3] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let n = self.resolve_operand_source(src1)?;
                let m = self.resolve_operand_source(src2)?;
                let a = self.resolve_operand_source(src3)?;
                let result = alu::msub(n, m, a, is_w);
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid MSUB operands".to_string())),
        }
    }
    fn execute_mneg(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let n = self.resolve_operand_source(src1)?;
                let m = self.resolve_operand_source(src2)?;
                let result = alu::mneg(n, m, is_w);
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid MNEG operands".to_string())),
        }
    }
    // Same style for wide variants
    fn execute_wide_3op(
        &mut self,
        operands: &[Operand],
        op: fn(Word, Word, Word) -> Word,
    ) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2, src3] => {
                let (rd, _is_w) = self.resolve_operand_dest(dest)?;
                let n = self.resolve_operand_source(src1)?;
                let m = self.resolve_operand_source(src2)?;
                let a = self.resolve_operand_source(src3)?;
                let result = op(n, m, a);
                self.set_reg_with_width(rd, result, false); // These always write 64-bit
                Ok(false)
            }
            _ => Err(EmuError::InternalError(
                "Invalid wide 3-op operands".to_string(),
            )),
        }
    }
    fn execute_wide_2op(
        &mut self,
        operands: &[Operand],
        op: fn(Word, Word) -> Word,
    ) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, _is_w) = self.resolve_operand_dest(dest)?;
                let n = self.resolve_operand_source(src1)?;
                let m = self.resolve_operand_source(src2)?;
                let result = op(n, m);
                self.set_reg_with_width(rd, result, false);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(
                "Invalid wide 2-op operands".to_string(),
            )),
        }
    }

    fn execute_bitop(&mut self, operands: &[Operand], op: fn(u64, bool) -> u64) -> EmuResult<bool> {
        match operands {
            [dest, src] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let val = self.resolve_operand_source(src)?;
                let result = op(val, is_w);
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid bit op operands".into())),
        }
    }

    fn execute_csel_like(
        &mut self,
        operands: &[Operand],
        cond: Condition,
        op: SelectOp,
    ) -> EmuResult<bool> {
        match operands {
            [dest, src1, src2] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let val1 = self.resolve_operand_source(src1)?;
                let val2 = self.resolve_operand_source(src2)?;
                let select_first = self.check_condition(cond);
                let chosen = match op {
                    SelectOp::Sel => {
                        if select_first {
                            val1
                        } else {
                            val2
                        }
                    }
                    SelectOp::Inc => {
                        if select_first {
                            val1
                        } else {
                            val2.wrapping_add(1)
                                & if is_w {
                                    0xFFFF_FFFF
                                } else {
                                    0xFFFF_FFFF_FFFF_FFFF
                                }
                        }
                    }
                    SelectOp::Inv => {
                        if select_first {
                            val1
                        } else {
                            !val2
                                & if is_w {
                                    0xFFFF_FFFF
                                } else {
                                    0xFFFF_FFFF_FFFF_FFFF
                                }
                        }
                    }
                    SelectOp::Neg => {
                        if select_first {
                            val1
                        } else {
                            (!val2).wrapping_add(1)
                                & if is_w {
                                    0xFFFF_FFFF
                                } else {
                                    0xFFFF_FFFF_FFFF_FFFF
                                }
                        }
                    }
                    SelectOp::Set => {
                        if select_first {
                            val1
                        } else {
                            if is_w {
                                0xFFFF_FFFF
                            } else {
                                0xFFFF_FFFF_FFFF_FFFF
                            }
                        }
                    }
                    SelectOp::Setm => {
                        if select_first {
                            val1
                        } else {
                            0
                        }
                    }
                    SelectOp::IncTrue => {
                        if select_first {
                            val1.wrapping_add(1)
                                & if is_w {
                                    0xFFFF_FFFF
                                } else {
                                    0xFFFF_FFFF_FFFF_FFFF
                                }
                        } else {
                            val2
                        }
                    }
                    SelectOp::InvTrue => {
                        if select_first {
                            !val1
                                & if is_w {
                                    0xFFFF_FFFF
                                } else {
                                    0xFFFF_FFFF_FFFF_FFFF
                                }
                        } else {
                            val2
                        }
                    }
                    SelectOp::NegTrue => {
                        if select_first {
                            (!val1).wrapping_add(1)
                                & if is_w {
                                    0xFFFF_FFFF
                                } else {
                                    0xFFFF_FFFF_FFFF_FFFF
                                }
                        } else {
                            val2
                        }
                    }
                };
                self.set_reg_with_width(rd, chosen, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(
                "Invalid operands for conditional select".into(),
            )),
        }
    }

    fn execute_ccmp_reg(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool> {
        // [Rn, Rm, #nzcv]
        if let [op1, op2, Operand::Imm(Immediate::Lit(nzcv))] = operands {
            let is_w = matches!(op1, Operand::Reg(r) if r.is_w_register());
            let condition_true = self.check_condition(cond);
            if condition_true {
                let lhs = self.resolve_operand_source(op1)?;
                let rhs = self.resolve_operand_source(op2)?;
                let (res, carry, overflow) = alu::sub(lhs, rhs, is_w);
                self.update_cpsr_nzcv(res, carry, overflow, true, is_w);
            } else {
                // Set flags to literal nzcv (lowest 4 bits)
                let mut cpsr = self.cpsr.borrow_mut();
                *cpsr = (*cpsr & !0xF0000000) | ((*nzcv as u64 & 0xF) << 28);
            }
            Ok(false)
        } else {
            Err(EmuError::InternalError("Bad CCMP reg ops".into()))
        }
    }

    fn execute_ccmp_imm(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool> {
        // [Rn, #imm5, #nzcv]
        if let [
            op1,
            Operand::Imm(Immediate::Lit(imm)),
            Operand::Imm(Immediate::Lit(nzcv)),
        ] = operands
        {
            let is_w = matches!(op1, Operand::Reg(r) if r.is_w_register());
            let condition_true = self.check_condition(cond);
            if condition_true {
                let lhs = self.resolve_operand_source(op1)?;
                let rhs = *imm as u64;
                let (res, carry, overflow) = alu::sub(lhs, rhs, is_w);
                self.update_cpsr_nzcv(res, carry, overflow, true, is_w);
            } else {
                let mut cpsr = self.cpsr.borrow_mut();
                *cpsr = (*cpsr & !0xF0000000) | ((*nzcv as u64 & 0xF) << 28);
            }
            Ok(false)
        } else {
            Err(EmuError::InternalError("Bad CCMP imm ops".into()))
        }
    }

    fn execute_ccmn_reg(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool> {
        // [Rn, Rm, #nzcv]
        if let [op1, op2, Operand::Imm(Immediate::Lit(nzcv))] = operands {
            let is_w = matches!(op1, Operand::Reg(r) if r.is_w_register());
            let condition_true = self.check_condition(cond);
            if condition_true {
                let lhs = self.resolve_operand_source(op1)?;
                let rhs = self.resolve_operand_source(op2)?;
                let (res, carry, overflow) = alu::add(lhs, rhs, is_w);
                self.update_cpsr_nzcv(res, carry, overflow, false, is_w);
            } else {
                let mut cpsr = self.cpsr.borrow_mut();
                *cpsr = (*cpsr & !0xF0000000) | ((*nzcv as u64 & 0xF) << 28);
            }
            Ok(false)
        } else {
            Err(EmuError::InternalError("Bad CCMN reg ops".into()))
        }
    }

    fn execute_ccmn_imm(&mut self, operands: &[Operand], cond: Condition) -> EmuResult<bool> {
        if let [
            op1,
            Operand::Imm(Immediate::Lit(imm)),
            Operand::Imm(Immediate::Lit(nzcv)),
        ] = operands
        {
            let is_w = matches!(op1, Operand::Reg(r) if r.is_w_register());
            let condition_true = self.check_condition(cond);
            if condition_true {
                let lhs = self.resolve_operand_source(op1)?;
                let rhs = *imm as u64;
                let (res, carry, overflow) = alu::add(lhs, rhs, is_w);
                self.update_cpsr_nzcv(res, carry, overflow, false, is_w);
            } else {
                let mut cpsr = self.cpsr.borrow_mut();
                *cpsr = (*cpsr & !0xF0000000) | ((*nzcv as u64 & 0xF) << 28);
            }
            Ok(false)
        } else {
            Err(EmuError::InternalError("Bad CCMN imm ops".into()))
        }
    }

    fn execute_mvn(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest, src] => {
                let (rd, is_w) = self.resolve_operand_dest(dest)?;
                let srcval = self.resolve_operand_source(src)?;
                let (result, _, _) = alu::mvn(srcval, is_w);
                self.set_reg_with_width(rd, result, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid MVN operands".to_string())),
        }
    }
}
