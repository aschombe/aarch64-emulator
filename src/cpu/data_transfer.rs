// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{Immediate, MovType, Offset, OpCode, Operand};
use crate::cpu;
use crate::cpu::state::CpuState;
use crate::types::{EmuError, EmuResult, Word};

/// Trait for data transfer instructions: MOV, LDR/STR, ADR/ADRP.
pub trait InstructionDataTransfer {
    fn execute_mov(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_adr(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_ldr(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_str(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool>;
    fn resolve_label_address(&self, label: &str) -> EmuResult<Word>;
    fn resolve_offset_address(&mut self, offset: &Offset) -> EmuResult<Word>;
}

impl InstructionDataTransfer for CpuState {
    fn execute_mov(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match opcode {
            OpCode::MOV(MovType::Normal) => match operands {
                [dest_op, src_op] => {
                    let (rd_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let src_val = self.resolve_operand_source(src_op)?;
                    self.set_reg_with_width(rd_id, src_val, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError(format!(
                    "Invalid MOV operands: {:?}",
                    operands
                ))),
            },
            OpCode::MOV(MovType::K) => match operands {
                [dest_op, Operand::Imm(Immediate::Lit(v))] => {
                    let (rd_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let value = if is_w {
                        *v as Word & 0xFFFF_FFFF
                    } else {
                        *v as Word
                    };
                    self.set_reg_with_width(rd_id, value, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError(format!(
                    "Invalid MOV K operands: {:?}",
                    operands
                ))),
            },
            OpCode::MOV(MovType::Z) => match operands {
                [dest_op, Operand::Imm(Immediate::Lit(v))] => {
                    let (rd_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let value = if is_w {
                        *v as Word & 0xFFFF_FFFF
                    } else {
                        *v as Word
                    };
                    self.set_reg_with_width(rd_id, value, is_w);
                    let mut cpsr = *self.cpsr.borrow();
                    if value == 0 {
                        cpsr |= cpu::flags::Z_FLAG;
                    } else {
                        cpsr &= !cpu::flags::Z_FLAG;
                    }
                    *self.cpsr.borrow_mut() = cpsr;
                    Ok(false)
                }
                _ => Err(EmuError::InternalError(format!(
                    "Invalid MOV Z operands: {:?}",
                    operands
                ))),
            },
            OpCode::MOV(MovType::N) => match operands {
                [dest_op, Operand::Imm(Immediate::Lit(v))] => {
                    let (rd_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let value = if is_w {
                        *v as Word & 0xFFFF_FFFF
                    } else {
                        *v as Word
                    };
                    self.set_reg_with_width(rd_id, value, is_w);
                    let mut cpsr = *self.cpsr.borrow();
                    let sign_bit = if is_w { 31 } else { 63 };
                    if (value >> sign_bit) & 1 == 1 {
                        cpsr |= cpu::flags::N_FLAG;
                    } else {
                        cpsr &= !cpu::flags::N_FLAG;
                    }
                    *self.cpsr.borrow_mut() = cpsr;
                    Ok(false)
                }
                _ => Err(EmuError::InternalError(format!(
                    "Invalid MOV N operands: {:?}",
                    operands
                ))),
            },
            _ => Err(EmuError::InternalError(format!(
                "Invalid MOV opcode: {:?}",
                opcode
            ))),
        }
    }

    fn execute_adr(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest_op, Operand::Imm(Immediate::Lbl(label))] => {
                let (rd_id, is_w) = self.resolve_operand_dest(dest_op)?;

                let raw = match self.program.label_to_ip.get(label) {
                    Some(x) => *x as Word,
                    None => {
                        if self.program.extern_labels.contains(label) {
                            return Err(EmuError::InternalError(format!(
                                "Attempted to call or branch to extern function '{}' but it was not defined in any input file.",
                                label
                            )));
                        } else {
                            return Err(EmuError::InternalError(format!(
                                "Undefined label: {}",
                                label
                            )));
                        }
                    }
                };

                let is_addr = *self.program.label_is_addr.get(label).unwrap_or(&false);

                // For text labels, return IR index (raw); for data, use full address.
                let target_addr = if is_addr {
                    // Data label: use loaded address (as before)
                    match opcode {
                        OpCode::ADR => raw,
                        OpCode::ADRP => raw & 0xFFFF_FFFF_FFFF_F000,
                        _ => {
                            return Err(EmuError::InternalError(format!(
                                "Invalid ADR opcode: {:?}",
                                opcode
                            )));
                        }
                    }
                } else {
                    // Text label: always return IR index
                    raw
                };

                self.set_reg_with_width(rd_id, target_addr, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid ADR operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_ldr(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match opcode {
            OpCode::LDR => match operands {
                [dest_op, Operand::Offset(offset)] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let value = self.memory.borrow().read_word(effective_addr)?;
                    self.set_reg_with_width(rt_id, value, is_w);
                    Ok(false)
                }
                [dest_op, Operand::Imm(Immediate::Lbl(label))] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let address_value = self.resolve_label_address(label)?;
                    self.set_reg_with_width(rt_id, address_value, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid LDR operands".into())),
            },
            OpCode::LDP => match operands {
                [dest_op1, dest_op2, Operand::Offset(offset)] => {
                    let (rt1_id, is_w1) = self.resolve_operand_dest(dest_op1)?;
                    let (rt2_id, is_w2) = self.resolve_operand_dest(dest_op2)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let value1 = self.memory.borrow().read_word(effective_addr)?;
                    let value2 = self.memory.borrow().read_word(effective_addr + 8)?;
                    self.set_reg_with_width(rt1_id, value1, is_w1);
                    self.set_reg_with_width(rt2_id, value2, is_w2);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid LDP operands".into())),
            },
            OpCode::LDRB => match operands {
                [dest_op, Operand::Offset(offset)] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let byte_value = self.memory.borrow().read_byte(effective_addr)? as Word;
                    self.set_reg_with_width(rt_id, byte_value & 0xFF, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid LDRB operands".into())),
            },
            OpCode::LDRH => match operands {
                [dest_op, Operand::Offset(offset)] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let halfword_value =
                        self.memory.borrow().read_halfword(effective_addr)? as Word;
                    self.set_reg_with_width(rt_id, halfword_value & 0xFFFF, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid LDRH operands".into())),
            },
            OpCode::LDRSB => match operands {
                [dest_op, Operand::Offset(offset)] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let byte_value = self.memory.borrow().read_byte(effective_addr)? as i8;
                    let sign_extended = byte_value as i64 as Word;
                    self.set_reg_with_width(rt_id, sign_extended, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid LDRSB operands".into())),
            },
            OpCode::LDRSH => match operands {
                [dest_op, Operand::Offset(offset)] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let halfword_value = self.memory.borrow().read_halfword(effective_addr)? as i16;
                    let sign_extended = halfword_value as i32 as Word;
                    self.set_reg_with_width(rt_id, sign_extended, is_w);
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid LDRSH operands".into())),
            },
            _ => Err(EmuError::InternalError("Invalid LDR opcode".into())),
        }
    }

    fn execute_str(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match opcode {
            OpCode::STR => match operands {
                [source_op, Operand::Offset(offset)] => {
                    let value = self.resolve_operand_source(source_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    self.memory.borrow_mut().write_word(effective_addr, value)?;
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid STR operands".into())),
            },
            OpCode::STP => match operands {
                [source_op1, source_op2, Operand::Offset(offset)] => {
                    let value1 = self.resolve_operand_source(source_op1)?;
                    let value2 = self.resolve_operand_source(source_op2)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    self.memory
                        .borrow_mut()
                        .write_word(effective_addr, value1)?;
                    self.memory
                        .borrow_mut()
                        .write_word(effective_addr + 8, value2)?;
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid STP operands".into())),
            },
            OpCode::STRB => match operands {
                [source_op, Operand::Offset(offset)] => {
                    let value = self.resolve_operand_source(source_op)? as u8;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    self.memory.borrow_mut().write_byte(effective_addr, value)?;
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid STRB operands".into())),
            },
            OpCode::STRH => match operands {
                [source_op, Operand::Offset(offset)] => {
                    let value = self.resolve_operand_source(source_op)? as u16;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    self.memory
                        .borrow_mut()
                        .write_halfword(effective_addr, value as u32)?;
                    Ok(false)
                }
                _ => Err(EmuError::InternalError("Invalid STRH operands".into())),
            },
            _ => Err(EmuError::InternalError("Invalid STR opcode".into())),
        }
    }

    fn resolve_label_address(&self, label: &str) -> EmuResult<Word> {
        let raw = match self.program.label_to_ip.get(label) {
            Some(x) => *x as Word,
            None => {
                if self.program.extern_labels.contains(label) {
                    return Err(EmuError::InternalError(format!(
                        "Attempted to call or branch to extern function '{}' but it was not defined in any input file.",
                        label
                    )));
                } else {
                    return Err(EmuError::InternalError(format!(
                        "Undefined label: {}",
                        label
                    )));
                }
            }
        };

        let is_addr = *self.program.label_is_addr.get(label).unwrap_or(&false);

        Ok(if is_addr {
            raw
        } else {
            self.ip_to_virtual_addr(raw)
        })
    }

    fn resolve_offset_address(&mut self, offset: &Offset) -> EmuResult<Word> {
        let resolve_immediate_value = |imm: &Immediate, cpu: &mut CpuState| -> EmuResult<Word> {
            match imm {
                Immediate::Lit(v) => Ok(*v as Word),
                Immediate::Lbl(label) => {
                    let raw = match cpu.program.label_to_ip.get(label) {
                        Some(x) => *x as Word,
                        None => {
                            if cpu.program.extern_labels.contains(label) {
                                return Err(EmuError::InternalError(format!(
                                    "Attempted to call or branch to extern function '{}' but it was not defined in any input file.",
                                    label
                                )));
                            } else {
                                return Err(EmuError::InternalError(format!(
                                    "Undefined label: {}",
                                    label
                                )));
                            }
                        }
                    };

                    let is_addr = *cpu.program.label_is_addr.get(label).unwrap_or(&false);
                    Ok(if is_addr {
                        raw
                    } else {
                        cpu.ip_to_virtual_addr(raw)
                    })
                }
                Immediate::Lo12Lbl(label) => {
                    let raw = match cpu.program.label_to_ip.get(label) {
                        Some(x) => *x as Word,
                        None => {
                            if cpu.program.extern_labels.contains(label) {
                                return Err(EmuError::InternalError(format!(
                                    "Attempted to call or branch to extern function '{}' but it was not defined in any input file.",
                                    label
                                )));
                            } else {
                                return Err(EmuError::InternalError(format!(
                                    "Undefined label: {}",
                                    label
                                )));
                            }
                        }
                    };

                    let is_addr = *cpu.program.label_is_addr.get(label).unwrap_or(&false);
                    let full_addr = if is_addr {
                        raw
                    } else {
                        cpu.ip_to_virtual_addr(raw)
                    };
                    Ok(full_addr & 0xFFF)
                }
            }
        };

        match offset {
            Offset::Ind1(imm) => resolve_immediate_value(imm, self),
            Offset::Ind2(base_reg) => Ok(self.get_reg(base_reg.to_id())),
            Offset::Ind3(base_reg, imm) => {
                let base_addr = self.get_reg(base_reg.to_id());
                let offset_val = resolve_immediate_value(imm, self)?;
                Ok(base_addr.wrapping_add(offset_val))
            }
            Offset::Ind4(base_reg, index_reg) => {
                let base_addr = self.get_reg(base_reg.to_id());
                let idx_val = self.get_reg(index_reg.to_id());
                Ok(base_addr.wrapping_add(idx_val))
            }
            Offset::PreIndexed(base_reg, imm) => {
                let reg_id = base_reg.to_id();
                let base_addr = self.get_reg(reg_id);
                let offset_val = resolve_immediate_value(imm, self)?;
                let new_base = base_addr.wrapping_add(offset_val);
                self.set_reg(reg_id, new_base);
                Ok(new_base)
            }
            Offset::PostIndexed(base_reg, imm) => {
                let reg_id = base_reg.to_id();
                let base_addr = self.get_reg(reg_id);
                let offset_val = resolve_immediate_value(imm, self)?;
                let new_base = base_addr.wrapping_add(offset_val);
                self.set_reg(reg_id, new_base);
                Ok(base_addr)
            }
        }
    }
}
