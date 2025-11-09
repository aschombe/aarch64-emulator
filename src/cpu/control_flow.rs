// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{Condition, Immediate, OpCode, Operand};
use crate::cpu::state::CpuState;
use crate::types::{EmuError, EmuResult, Word};
use crate::{cpu, syscall};

/// Trait for control flow instructions: branches, calls, returns, and SVC.
pub trait InstructionControl {
    fn check_condition(&self, cond: Condition) -> bool;
    fn execute_branch(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool>;
    fn execute_ret(&mut self) -> EmuResult<bool>;
    fn execute_svc(&mut self, operands: &[Operand]) -> EmuResult<bool>;
}

impl InstructionControl for CpuState {
    fn check_condition(&self, cond: Condition) -> bool {
        let cpsr = *self.cpsr.borrow();
        let n = (cpsr & cpu::flags::N_FLAG) != 0;
        let z = (cpsr & cpu::flags::Z_FLAG) != 0;
        let v = (cpsr & cpu::flags::V_FLAG) != 0;
        let c = (cpsr & cpu::flags::C_FLAG) != 0;
        match cond {
            Condition::Eq => z,
            Condition::Ne => !z,
            Condition::Ge => n == v,
            Condition::Lt => n != v,
            Condition::Gt => !z && (n == v),
            Condition::Le => !(!z && (n == v)),
            Condition::Al | Condition::Nv => true,

            Condition::Cs | Condition::Hs => c,
            Condition::Cc | Condition::Lo => !c,
            Condition::Mi => n,
            Condition::Pl => !n,
            Condition::Vs => v,
            Condition::Vc => !v,
            Condition::Hi => c && !z,
            Condition::Ls => !(c && !z),
        }
    }

    fn execute_branch(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        // B/B.cond/BL
        if let OpCode::B(condition_opt) = opcode {
            if let [Operand::Imm(Immediate::Lbl(label))] = operands {
                if *self.program.label_is_addr.get(label).unwrap_or(&false) {
                    return Err(EmuError::InternalError(format!(
                        "Branch to data label not allowed: {}",
                        label
                    )));
                }

                // B.cond: check condition
                if !self.check_condition(condition_opt) {
                    return Ok(false);
                }

                let target_ip = match self.program.label_to_ip.get(label) {
                    Some(x) => *x as usize,
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
                *self.ip.borrow_mut() = target_ip;
                self.did_branch.set(true);
                return Ok(false);
            }
        }
        if let OpCode::BL = opcode {
            if let [Operand::Imm(Immediate::Lbl(label))] = operands {
                if *self.program.label_is_addr.get(label).unwrap_or(&false) {
                    return Err(EmuError::InternalError(format!(
                        "Branch to data label not allowed: {}",
                        label
                    )));
                }
                let target_ip = match self.program.label_to_ip.get(label) {
                    Some(x) => *x as usize,
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
                if let Some(pm_rc) = &self.plugin_manager {
                    let mut pm = pm_rc.borrow_mut();
                    pm.run_hooks("pre_bl", self)?;
                }
                let return_addr = *self.ip.borrow() + 1;
                self.set_reg(30, return_addr as Word);
                *self.ip.borrow_mut() = target_ip;
                self.did_branch.set(true);
                if let Some(pm_rc) = &self.plugin_manager {
                    let mut pm = pm_rc.borrow_mut();
                    pm.run_hooks("post_bl", self)?;
                }
                return Ok(false);
            }
        }
        // CBZ/CBNZ
        if matches!(opcode, OpCode::CBZ | OpCode::CBNZ) {
            if let [Operand::Reg(reg), Operand::Imm(Immediate::Lbl(label))] = operands {
                if *self.program.label_is_addr.get(label).unwrap_or(&false) {
                    return Err(EmuError::InternalError(format!(
                        "Conditional branch to data label not allowed: {}",
                        label
                    )));
                }
                let raw_val = self.get_reg(reg.to_id());
                let reg_val = if reg.is_w_register() {
                    raw_val & 0xFFFF_FFFF
                } else {
                    raw_val
                };
                let should_branch = match opcode {
                    OpCode::CBZ => reg_val == 0,
                    OpCode::CBNZ => reg_val != 0,
                    _ => false,
                };
                if should_branch {
                    let target_ip = match self.program.label_to_ip.get(label) {
                        Some(x) => *x as usize,
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
                    *self.ip.borrow_mut() = target_ip;
                    self.did_branch.set(true);
                }
                return Ok(false);
            }
        }
        // TBZ/TBNZ
        if matches!(opcode, OpCode::TBZ | OpCode::TBNZ) {
            if let [
                Operand::Reg(reg),
                Operand::Imm(Immediate::Lit(bit_pos)),
                Operand::Imm(Immediate::Lbl(label)),
            ] = operands
            {
                if *self.program.label_is_addr.get(label).unwrap_or(&false) {
                    return Err(EmuError::InternalError(format!(
                        "TBZ/TBNZ to data label not allowed: {}",
                        label
                    )));
                }
                let raw_val = self.get_reg(reg.to_id());
                let reg_val = if reg.is_w_register() {
                    raw_val & 0xFFFF_FFFF
                } else {
                    raw_val
                };
                if *bit_pos >= if reg.is_w_register() { 32 } else { 64 } {
                    return Err(EmuError::InternalError(format!(
                        "Bit position {} out of range for register {:?}",
                        bit_pos, reg
                    )));
                }
                let bit_set = (reg_val & (1 << bit_pos)) != 0;
                let should_branch = match opcode {
                    OpCode::TBZ => !bit_set,
                    OpCode::TBNZ => bit_set,
                    _ => false,
                };
                if should_branch {
                    let target_ip = match self.program.label_to_ip.get(label) {
                        Some(x) => *x as usize,
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
                    *self.ip.borrow_mut() = target_ip;
                    self.did_branch.set(true);
                }
                return Ok(false);
            }
        }
        // BR (indirect)
        // if let OpCode::BR = opcode {
        //     if let [Operand::Reg(reg)] = operands {
        //         let target_ip = self.get_reg(reg.to_id()) as usize;
        //         *self.ip.borrow_mut() = target_ip;
        //         self.did_branch.set(true);
        //         return Ok(false);
        //     }
        // }
        Err(EmuError::InternalError(format!(
            "Invalid branch operands: {:?} {:?}",
            opcode, operands
        )))
    }

    fn execute_ret(&mut self) -> EmuResult<bool> {
        // Pre RET hook
        if let Some(pm_rc) = &self.plugin_manager {
            let mut pm = pm_rc.borrow_mut();
            pm.run_hooks("pre_ret", self)?;
        }

        let lr_value = self.get_reg(30);
        if lr_value == 0 {
            return Err(EmuError::InternalError(
                "LR is zero on RET; cannot return.".to_string(),
            ));
        }

        *self.ip.borrow_mut() = lr_value as usize;
        self.did_branch.set(true);

        // Post RET hook
        if let Some(pm_rc) = &self.plugin_manager {
            let mut pm = pm_rc.borrow_mut();
            pm.run_hooks("post_ret", self)?;
        }

        Ok(false)
    }

    fn execute_svc(&mut self, _ops: &[Operand]) -> EmuResult<bool> {
        // Pre syscall hook
        if let Some(pm_rc) = &self.plugin_manager {
            let mut pm = pm_rc.borrow_mut();
            pm.run_hooks("pre_syscall", self)?;
        }

        let halt = syscall::handle_syscall(self)?;

        // Post syscall hook
        if let Some(pm_rc) = &self.plugin_manager {
            let mut pm = pm_rc.borrow_mut();
            pm.run_hooks("post_syscall", self)?;
        }

        Ok(halt)
    }
}
