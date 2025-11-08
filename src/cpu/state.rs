// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{
    Condition, Immediate, InstructionIR, MovType, Offset, OpCode, Operand, SymbolTable,
};
use crate::assembler::{FileSource, SourceMapEntry};
use crate::cpu::alu;
use crate::memory::Memory;
use crate::plugin::PluginManager;
use crate::syscall;
use crate::types::{EmuError, EmuResult, STACK_TOP, Word};
use crate::vfs::VirtualFileSystem;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub struct InterpretedProgram {
    pub instructions: Vec<InstructionIR>,
    pub label_to_ip: SymbolTable,
    pub label_is_addr: std::collections::HashMap<String, bool>,
    pub entry_ip: usize,
    pub extern_labels: std::collections::HashSet<String>,

    pub files: Vec<FileSource>,
    pub ip_map: Vec<SourceMapEntry>,
}

#[derive(Clone)]
pub struct CpuState {
    pub registers: RefCell<[Word; 32]>,
    pub sp: RefCell<Word>,
    pub cpsr: RefCell<Word>,
    pub memory: RefCell<Memory>,
    pub program: InterpretedProgram,
    pub ip: RefCell<usize>,
    pub vfs: Option<VirtualFileSystem>,
    pub plugin_manager: Option<Rc<RefCell<PluginManager>>>,

    did_branch: Cell<bool>,
}

pub const N_FLAG: Word = 1 << 31; // Negative flag
pub const Z_FLAG: Word = 1 << 30; // Zero flag
pub const C_FLAG: Word = 1 << 29; // Carry flag
pub const V_FLAG: Word = 1 << 28; // Overflow flag
pub const Q_FLAG: Word = 1 << 27; // Saturation flag

impl CpuState {
    pub fn new(
        program: InterpretedProgram,
        plugin_manager: Option<Rc<RefCell<PluginManager>>>,
        vfs: Option<VirtualFileSystem>,
    ) -> Self {
        let cpu = CpuState {
            registers: RefCell::new([0; 32]),
            sp: RefCell::new(STACK_TOP),
            cpsr: RefCell::new(0),
            memory: RefCell::new(Memory::new()),
            ip: RefCell::new(program.entry_ip),
            program,
            vfs,
            plugin_manager,
            did_branch: Cell::new(false),
        };
        cpu.registers.borrow_mut()[30] = 0;
        cpu
    }

    pub fn step_instruction(&mut self) -> EmuResult<()> {
        if self.halted() {
            return Err(EmuError::InternalError("CPU already halted".into()));
        }

        let current_ip = *self.ip.borrow();
        let ir_insn = self
            .program
            .instructions
            .get(current_ip)
            .cloned()
            .ok_or_else(|| EmuError::InternalError("IP out of range".into()))?;

        let halted = self.execute_instruction_ir(&ir_insn)?;
        if halted {
            *self.cpsr.borrow_mut() |= V_FLAG;
        } else {
            if !self.did_branch.get() {
                *self.ip.borrow_mut() += 1;
            }
            self.did_branch.set(false);
        }
        Ok(())
    }

    pub fn current_instruction(&self) -> EmuResult<InstructionIR> {
        let current_ip = *self.ip.borrow();
        self.program
            .instructions
            .get(current_ip)
            .cloned()
            .ok_or_else(|| {
                EmuError::InternalError(format!(
                    "Instruction pointer out of bounds: {}",
                    current_ip
                ))
            })
    }

    pub fn halted(&self) -> bool {
        *self.cpsr.borrow() & V_FLAG != 0
    }

    pub fn run(&mut self) -> EmuResult<()> {
        let total_insns = self.program.instructions.len();

        while *self.ip.borrow() < total_insns {
            // Pre PC increment hook
            if let Some(pm_rc) = &self.plugin_manager {
                let mut pm = pm_rc.borrow_mut();
                pm.pre_pc_increment(self)?;
            }

            let current_ip = *self.ip.borrow();
            let ir_insn = self
                .program
                .instructions
                .get(current_ip)
                .cloned()
                .ok_or_else(|| {
                    EmuError::InternalError(format!("Invalid instruction pointer: {}", current_ip))
                })?;

            let halted = self.execute_instruction_ir(&ir_insn)?;
            if halted {
                return Ok(());
            }

            // Only increment IP if no branch or return changed control flow
            if !self.did_branch.get() {
                *self.ip.borrow_mut() += 1;
            }

            // Reset flag for next instruction
            self.did_branch.set(false);

            // Post PC increment hook
            if let Some(pm_rc) = &self.plugin_manager {
                let mut pm = pm_rc.borrow_mut();
                pm.post_pc_increment(self)?;
            }
        }

        Ok(())
    }

    pub fn get_reg(&self, id: usize) -> Word {
        if id == 31 {
            0
        } else if id == 32 {
            *self.sp.borrow()
        } else {
            self.registers.borrow()[id]
        }
    }

    pub fn set_reg(&mut self, id: usize, value: Word) {
        self.set_reg_with_width(id, value, false);
    }

    pub fn set_reg_with_width(&mut self, id: usize, value: Word, is_w: bool) {
        if id == 31 {
            // XZR/WZR - write ignored
        } else if id == 32 {
            *self.sp.borrow_mut() = value;
        } else {
            if is_w {
                let masked_value = value & 0xFFFF_FFFF;
                self.registers.borrow_mut()[id] = masked_value;
            } else {
                self.registers.borrow_mut()[id] = value;
            }
        }
    }

    pub fn dump_state_full(&self) {
        println!("--- REGISTER STATE ---");
        for i in 0..=31 {
            print!("X{:02}: 0x{:016X}  ", i, self.registers.borrow()[i]);
            if (i + 1) % 4 == 0 {
                println!();
            }
        }
        println!(
            " SP: 0x{:016X} CPSR: 0x{:08X}",
            *self.sp.borrow(),
            *self.cpsr.borrow()
        );
    }

    fn update_cpsr_nzcv(
        &mut self,
        result: Word,
        carry: bool,
        overflow: bool,
        is_sub: bool,
        is_w: bool,
    ) {
        let mut new_state = 0;

        let sign_bit = if is_w { 31 } else { 63 }; // Check bit 31 for W-reg op, 63 for X-reg op

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

    fn check_condition(&self, cond: Condition) -> bool {
        let cpsr = *self.cpsr.borrow();
        let n = (cpsr & N_FLAG) != 0;
        let z = (cpsr & Z_FLAG) != 0;
        let v = (cpsr & V_FLAG) != 0;
        match cond {
            Condition::Eq => z,
            Condition::Ne => !z,
            Condition::Ge => n == v,
            Condition::Lt => n != v,
            Condition::Gt => !z && (n == v),
            Condition::Le => z || (n != v),
            Condition::Al => true,

            Condition::Cs | Condition::Hs => (cpsr & C_FLAG) != 0,
            Condition::Cc | Condition::Lo => (cpsr & C_FLAG) == 0,
            Condition::Mi => n,
            Condition::Pl => !n,
            Condition::Vs => v,
            Condition::Vc => !v,
            Condition::Hi => (cpsr & C_FLAG) != 0 && !z,
            Condition::Ls => (cpsr & C_FLAG) == 0 || z,
        }
    }

    fn resolve_operand_source(&self, operand: &Operand) -> EmuResult<Word> {
        match operand {
            Operand::Reg(r) => {
                let val = self.get_reg(r.to_id());
                Ok(if r.is_w_register() {
                    val & 0xFFFF_FFFF // Only the bottom 32 bits are relevant for W-reg source
                } else {
                    val // X-registers provide the full 64 bits
                })
            }
            Operand::Imm(Immediate::Lit(v)) => Ok(*v as Word),
            Operand::Imm(Immediate::Lbl(label)) => {
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
            Operand::Imm(Immediate::Lo12Lbl(label)) => {
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
                let full_addr = if is_addr {
                    raw
                } else {
                    self.ip_to_virtual_addr(raw)
                };
                // Mask off lower 12 bits for a literal value
                Ok(full_addr & 0xFFF)
            }
            _ => Err(EmuError::InternalError(format!(
                "Unsupported source operand: {:?}",
                operand
            ))),
        }
    }

    fn resolve_operand_dest(&self, operand: &Operand) -> EmuResult<(usize, bool)> {
        match operand {
            Operand::Reg(r) => Ok((r.to_id(), r.is_w_register())),
            _ => Err(EmuError::InternalError(format!(
                "Invalid dest operand: {:?}",
                operand
            ))),
        }
    }

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
                    self.update_cpsr_nzcv(result, carry, overflow, is_sub, is_w); // <-- NEW: Pass is_w
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

    pub fn execute_instruction_ir(&mut self, ir_insn: &InstructionIR) -> EmuResult<bool> {
        match &ir_insn.opcode {
            OpCode::ADD => self.execute_binary_op(&ir_insn.operands, alu::add, false, false),
            OpCode::ADDS => self.execute_binary_op(&ir_insn.operands, alu::add, true, false),
            OpCode::SUB => self.execute_binary_op(&ir_insn.operands, alu::sub, false, true),
            OpCode::SUBS => self.execute_binary_op(&ir_insn.operands, alu::sub, true, true),
            OpCode::MUL => self.execute_binary_op(&ir_insn.operands, alu::mul, false, false),
            OpCode::MULS => self.execute_binary_op(&ir_insn.operands, alu::mul, true, false),
            OpCode::UMULL => self.execute_binary_op(&ir_insn.operands, alu::umull, false, false),
            OpCode::SMULL => self.execute_binary_op(&ir_insn.operands, alu::smull, false, false),
            OpCode::UMULH => self.execute_binary_op(&ir_insn.operands, alu::umulh, false, false),
            OpCode::SMULH => self.execute_binary_op(&ir_insn.operands, alu::smulh, false, false),
            OpCode::UDIV => self.execute_binary_op(&ir_insn.operands, alu::udiv, false, false),
            OpCode::SDIV => self.execute_binary_op(&ir_insn.operands, alu::sdiv, false, false),
            OpCode::AND => self.execute_binary_op(&ir_insn.operands, alu::and, false, false),
            OpCode::ANDS => self.execute_binary_op(&ir_insn.operands, alu::and, true, false),
            OpCode::ORR => self.execute_binary_op(&ir_insn.operands, alu::orr, false, false),
            OpCode::EOR => self.execute_binary_op(&ir_insn.operands, alu::eor, false, false),
            OpCode::LSL => self.execute_binary_op(&ir_insn.operands, alu::lsl, false, false),
            OpCode::LSR => self.execute_binary_op(&ir_insn.operands, alu::lsr, false, false),
            OpCode::ASR => self.execute_binary_op(&ir_insn.operands, alu::asr, false, false),
            OpCode::CMP => self.execute_cmp(&ir_insn.operands),
            OpCode::MOV(kind) => self.execute_mov(OpCode::MOV(*kind), &ir_insn.operands),
            OpCode::ADR => self.execute_adr(OpCode::ADR, &ir_insn.operands),
            OpCode::ADRP => self.execute_adr(OpCode::ADRP, &ir_insn.operands),
            OpCode::LDR => self.execute_ldr(OpCode::LDR, &ir_insn.operands),
            OpCode::LDP => self.execute_ldr(OpCode::LDP, &ir_insn.operands),
            OpCode::LDRB => self.execute_ldr(OpCode::LDRB, &ir_insn.operands),
            OpCode::LDRH => self.execute_ldr(OpCode::LDRH, &ir_insn.operands),
            OpCode::LDRSB => self.execute_ldr(OpCode::LDRSB, &ir_insn.operands),
            OpCode::LDRSH => self.execute_ldr(OpCode::LDRSH, &ir_insn.operands),
            OpCode::STR => self.execute_str(OpCode::STR, &ir_insn.operands),
            OpCode::STP => self.execute_str(OpCode::STP, &ir_insn.operands),
            OpCode::STRB => self.execute_str(OpCode::STRB, &ir_insn.operands),
            OpCode::STRH => self.execute_str(OpCode::STRH, &ir_insn.operands),
            OpCode::B(condition) => self.execute_branch(OpCode::B(*condition), &ir_insn.operands),
            OpCode::BL => self.execute_branch(OpCode::BL, &ir_insn.operands),
            OpCode::BR => self.execute_branch(OpCode::BR, &ir_insn.operands),
            OpCode::CBZ => self.execute_branch(OpCode::CBZ, &ir_insn.operands),
            OpCode::CBNZ => self.execute_branch(OpCode::CBNZ, &ir_insn.operands),
            OpCode::RET => self.execute_ret(),
            OpCode::SVC => self.execute_svc(&ir_insn.operands),
            OpCode::NOP => Ok(false),
            _ => Err(EmuError::UnimplementedSyscall(format!(
                "{:?}",
                ir_insn.opcode
            ))),
        }
    }

    fn execute_branch(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            // B / B.cond / BL
            [Operand::Imm(Immediate::Lbl(label))] => {
                // HYBRID: Check if label is a code (index), not address
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

                if let OpCode::B(condition) = opcode {
                    let take = self.check_condition(condition);
                    if !take {
                        return Ok(false);
                    }
                }
                if let OpCode::BL = opcode {
                    // Pre BL hook
                    if let Some(pm_rc) = &self.plugin_manager {
                        let mut pm = pm_rc.borrow_mut();
                        pm.run_hooks("pre_bl", self)?;
                    }

                    let return_addr = *self.ip.borrow() + 1;
                    self.set_reg(30, return_addr as Word);
                }
                *self.ip.borrow_mut() = target_ip;
                self.did_branch.set(true);
                if let OpCode::BL = opcode {
                    // Post BL hook
                    if let Some(pm_rc) = &self.plugin_manager {
                        let mut pm = pm_rc.borrow_mut();
                        pm.run_hooks("post_bl", self)?;
                    }
                }
                Ok(false)
            }

            // CBZ / CBNZ
            [Operand::Reg(reg), Operand::Imm(Immediate::Lbl(label))] => {
                // HYBRID: Only allow code label
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

                let condition_met = match opcode {
                    OpCode::CBZ => reg_val == 0,
                    OpCode::CBNZ => reg_val != 0,
                    _ => {
                        return Err(EmuError::InternalError(format!(
                            "Invalid conditional branch opcode: {:?}",
                            opcode
                        )));
                    }
                };
                if condition_met {
                    let target_ip = match self.program.label_to_ip.get(label) {
                        // pm.run_hooks("pre_pc_increment", self)?;
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
                    *self.ip.borrow_mut() = target_ip.saturating_sub(1);
                }
                Ok(false)
            }

            // BR
            [Operand::Reg(reg)] if matches!(opcode, OpCode::BR) => {
                let target_ip = self.get_reg(reg.to_id()) as usize;
                *self.ip.borrow_mut() = target_ip;
                self.did_branch.set(true);
                Ok(false)
            }

            _ => Err(EmuError::InternalError(format!(
                "Invalid branch operands: {:?}",
                operands
            ))),
        }
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

    pub fn execute_svc(&mut self, _ops: &[Operand]) -> EmuResult<bool> {
        // let sys_call_num = self.get_reg(8);

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
                    // Update Z flag
                    let mut cpsr = *self.cpsr.borrow();
                    if value == 0 {
                        cpsr |= Z_FLAG;
                    } else {
                        cpsr &= !Z_FLAG;
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
                    // Update N flag
                    let mut cpsr = *self.cpsr.borrow();
                    let sign_bit = if is_w { 31 } else { 63 };
                    if (value >> sign_bit) & 1 == 1 {
                        cpsr |= N_FLAG;
                    } else {
                        cpsr &= !N_FLAG;
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

                let target_addr = match opcode {
                    OpCode::ADR => {
                        let target_ip_addr = if is_addr {
                            raw
                        } else {
                            self.ip_to_virtual_addr(raw)
                        };
                        target_ip_addr
                    }
                    OpCode::ADRP => {
                        let target_ip_addr = if is_addr {
                            raw
                        } else {
                            self.ip_to_virtual_addr(raw)
                        };
                        target_ip_addr & 0xFFFF_FFFF_FFFF_F000
                    }
                    _ => {
                        return Err(EmuError::InternalError(format!(
                            "Invalid ADR opcode: {:?}",
                            opcode
                        )));
                    }
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

    /// Converts instruction index to a virtual address in text segment
    fn ip_to_virtual_addr(&self, idx: Word) -> Word {
        crate::types::TEXT_BASE + idx * 4 // Assuming 4 bytes per instruction
    }

    fn execute_ldr(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match opcode {
            OpCode::LDR => match operands {
                // Case 1: Standard Register/Immediate Offset Load (LDR Xt, [Xn] or [Xn, #imm])
                [dest_op, Operand::Offset(offset)] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                    let effective_addr = self.resolve_offset_address(offset)?;
                    let value = self.memory.borrow().read_word(effective_addr)?;
                    self.set_reg_with_width(rt_id, value, is_w);
                    Ok(false)
                }

                // Case 2: Literal Load Pseudoinstruction (LDR Xt, =label)
                [dest_op, Operand::Imm(Immediate::Lbl(label))] => {
                    let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;

                    let address_value = self.resolve_label_address(label)?;

                    self.set_reg_with_width(rt_id, address_value, is_w); // Load the ADDRESS
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
            Offset::Ind1(imm) => resolve_immediate_value(imm, self), // [imm]
            Offset::Ind2(base_reg) => Ok(self.get_reg(base_reg.to_id())), // [reg]
            Offset::Ind3(base_reg, imm) => {
                // [reg, imm]
                let base_addr = self.get_reg(base_reg.to_id());
                let offset_val = resolve_immediate_value(imm, self)?;
                Ok(base_addr.wrapping_add(offset_val))
            }
            Offset::Ind4(base_reg, index_reg) => {
                // [reg, reg]
                let base_addr = self.get_reg(base_reg.to_id());
                let idx_val = self.get_reg(index_reg.to_id());
                Ok(base_addr.wrapping_add(idx_val))
            }
            Offset::PreIndexed(base_reg, imm) => {
                // [reg, imm]!
                let reg_id = base_reg.to_id();
                let base_addr = self.get_reg(reg_id);
                let offset_val = resolve_immediate_value(imm, self)?;
                let new_base = base_addr.wrapping_add(offset_val);
                self.set_reg(reg_id, new_base); // Writeback
                Ok(new_base) // Effective address is the new base
            }
            Offset::PostIndexed(base_reg, imm) => {
                // [reg], imm
                let reg_id = base_reg.to_id();
                let base_addr = self.get_reg(reg_id);
                let offset_val = resolve_immediate_value(imm, self)?;
                let new_base = base_addr.wrapping_add(offset_val);
                self.set_reg(reg_id, new_base); // Writeback
                Ok(base_addr) // Effective address is the *original* base
            }
        }
    }
}
