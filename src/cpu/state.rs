// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::{
    FileSource, SourceMapEntry,
    asm_types::{Immediate, InstructionIR, OpCode, Operand, SelectOp, SymbolTable},
};

use crate::cpu::{
    alu, control_flow::InstructionControl, data_processing::InstructionDataProcessing,
    data_transfer::InstructionDataTransfer, flags::V_FLAG,
};

use crate::memory::Memory;
use crate::plugin::PluginManager;
use crate::types::{EmuError, EmuResult, STACK_TOP, TEXT_BASE, Word};
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

    pub did_branch: Cell<bool>,
}

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

    pub fn resolve_operand_source(&self, operand: &Operand) -> EmuResult<Word> {
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

    pub fn resolve_operand_dest(&self, operand: &Operand) -> EmuResult<(usize, bool)> {
        match operand {
            Operand::Reg(r) => Ok((r.to_id(), r.is_w_register())),
            _ => Err(EmuError::InternalError(format!(
                "Invalid dest operand: {:?}",
                operand
            ))),
        }
    }

    /// Converts instruction index to a virtual address in text segment
    pub fn ip_to_virtual_addr(&self, idx: Word) -> Word {
        TEXT_BASE + idx * 4 // Assuming 4 bytes per instruction
    }

    pub fn execute_instruction_ir(&mut self, ir_insn: &InstructionIR) -> EmuResult<bool> {
        match &ir_insn.opcode {
            // Data Processing (ALU, CMP, NEG)
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
            OpCode::MADD => self.execute_madd(&ir_insn.operands),
            OpCode::MSUB => self.execute_msub(&ir_insn.operands),
            OpCode::MNEG => self.execute_mneg(&ir_insn.operands),
            OpCode::SMADDL => self.execute_wide_3op(&ir_insn.operands, alu::smaddl),
            OpCode::SMSUBL => self.execute_wide_3op(&ir_insn.operands, alu::smsubl),
            OpCode::SMNEGL => self.execute_wide_2op(&ir_insn.operands, alu::smnegl),
            OpCode::UMADDL => self.execute_wide_3op(&ir_insn.operands, alu::umaddl),
            OpCode::UMSUBL => self.execute_wide_3op(&ir_insn.operands, alu::umsubl),
            OpCode::UMNEGL => self.execute_wide_2op(&ir_insn.operands, alu::umnegl),

            OpCode::CLS => self.execute_bitop(&ir_insn.operands, alu::cls),
            OpCode::CLZ => self.execute_bitop(&ir_insn.operands, alu::clz),
            OpCode::CTZ => self.execute_bitop(&ir_insn.operands, alu::ctz),
            OpCode::CNT => self.execute_bitop(&ir_insn.operands, alu::cnt),
            OpCode::RBIT => self.execute_bitop(&ir_insn.operands, alu::rbit),
            OpCode::REV => self.execute_bitop(&ir_insn.operands, alu::rev),
            OpCode::REV16 => self.execute_bitop(&ir_insn.operands, alu::rev16),
            OpCode::REV32 => self.execute_bitop(&ir_insn.operands, alu::rev32),
            OpCode::REV64 => self.execute_bitop(&ir_insn.operands, alu::rev),

            OpCode::UDIV => self.execute_binary_op(&ir_insn.operands, alu::udiv, false, false),
            OpCode::SDIV => self.execute_binary_op(&ir_insn.operands, alu::sdiv, false, false),
            OpCode::NEG => self.execute_neg(&ir_insn.operands, false),
            OpCode::NEGS => self.execute_neg(&ir_insn.operands, true),
            OpCode::ABS => self.execute_abs(&ir_insn.operands),
            OpCode::ADC => self.execute_adc(&ir_insn.operands, false),
            OpCode::ADCS => self.execute_adc(&ir_insn.operands, true),
            OpCode::SBC => self.execute_sbc(&ir_insn.operands, false),
            OpCode::SBCS => self.execute_sbc(&ir_insn.operands, true),
            OpCode::NGC => self.execute_ngc(&ir_insn.operands, false),
            OpCode::NGCS => self.execute_ngc(&ir_insn.operands, true),

            OpCode::AND => self.execute_binary_op(&ir_insn.operands, alu::and, false, false),
            OpCode::ANDS => self.execute_binary_op(&ir_insn.operands, alu::and, true, false),
            OpCode::ORR => self.execute_binary_op(&ir_insn.operands, alu::orr, false, false),
            OpCode::EOR => self.execute_binary_op(&ir_insn.operands, alu::eor, false, false),
            OpCode::LSL => self.execute_binary_op(&ir_insn.operands, alu::lsl, false, false),
            OpCode::LSR => self.execute_binary_op(&ir_insn.operands, alu::lsr, false, false),
            OpCode::ASR => self.execute_binary_op(&ir_insn.operands, alu::asr, false, false),
            OpCode::ROR => self.execute_binary_op(&ir_insn.operands, alu::ror, false, false),
            OpCode::CMP => self.execute_cmp(&ir_insn.operands),
            OpCode::CMN => self.execute_cmn(&ir_insn.operands),
            OpCode::TST => self.execute_tst(&ir_insn.operands),
            OpCode::SMAX => self.execute_minmax(&ir_insn.operands, true, true),
            OpCode::SMIN => self.execute_minmax(&ir_insn.operands, true, false),
            OpCode::UMAX => self.execute_minmax(&ir_insn.operands, false, true),
            OpCode::UMIN => self.execute_minmax(&ir_insn.operands, false, false),

            // Data Transfer (MOV, LDR/STR, ADR/ADRP)
            OpCode::MOV(kind) => self.execute_mov(OpCode::MOV(*kind), &ir_insn.operands),
            OpCode::SWP => self.execute_swp(OpCode::SWP, &ir_insn.operands),
            OpCode::SWPB => self.execute_swp(OpCode::SWPB, &ir_insn.operands),
            OpCode::SWPH => self.execute_swp(OpCode::SWPH, &ir_insn.operands),
            OpCode::SWPP => self.execute_swp(OpCode::SWPP, &ir_insn.operands),
            OpCode::ADR => self.execute_adr(OpCode::ADR, &ir_insn.operands),
            OpCode::ADRP => self.execute_adr(OpCode::ADRP, &ir_insn.operands),
            OpCode::LDR => self.execute_ldr(OpCode::LDR, &ir_insn.operands),
            OpCode::LDP => self.execute_ldr(OpCode::LDP, &ir_insn.operands),
            OpCode::LDPSW => self.execute_ldr(OpCode::LDPSW, &ir_insn.operands),
            OpCode::LDRB => self.execute_ldr(OpCode::LDRB, &ir_insn.operands),
            OpCode::LDRH => self.execute_ldr(OpCode::LDRH, &ir_insn.operands),
            OpCode::LDRSB => self.execute_ldr(OpCode::LDRSB, &ir_insn.operands),
            OpCode::LDRSH => self.execute_ldr(OpCode::LDRSH, &ir_insn.operands),
            OpCode::LDRSW => self.execute_ldr(OpCode::LDRSW, &ir_insn.operands),
            OpCode::LDUR => self.execute_ldr(OpCode::LDUR, &ir_insn.operands),
            OpCode::LDURB => self.execute_ldr(OpCode::LDURB, &ir_insn.operands),
            OpCode::LDURH => self.execute_ldr(OpCode::LDURH, &ir_insn.operands),
            OpCode::LDURSB => self.execute_ldr(OpCode::LDURSB, &ir_insn.operands),
            OpCode::LDURSH => self.execute_ldr(OpCode::LDURSH, &ir_insn.operands),
            OpCode::LDURSW => self.execute_ldr(OpCode::LDURSW, &ir_insn.operands),
            OpCode::STR => self.execute_str(OpCode::STR, &ir_insn.operands),
            OpCode::STP => self.execute_str(OpCode::STP, &ir_insn.operands),
            OpCode::STRB => self.execute_str(OpCode::STRB, &ir_insn.operands),
            OpCode::STRH => self.execute_str(OpCode::STRH, &ir_insn.operands),
            OpCode::STUR => self.execute_str(OpCode::STUR, &ir_insn.operands),
            OpCode::STURB => self.execute_str(OpCode::STURB, &ir_insn.operands),
            OpCode::STURH => self.execute_str(OpCode::STURH, &ir_insn.operands),

            OpCode::CSEL(cond) => self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::Sel),
            OpCode::CSINC(cond) => self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::Inc),
            OpCode::CSINV(cond) => self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::Inv),
            OpCode::CSNEG(cond) => self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::Neg),
            OpCode::CSET(cond) => self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::Set),
            OpCode::CSETM(cond) => self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::Setm),
            OpCode::CINC(cond) => {
                self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::IncTrue)
            }
            OpCode::CINV(cond) => {
                self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::InvTrue)
            }
            OpCode::CNEG(cond) => {
                self.execute_csel_like(&ir_insn.operands, *cond, SelectOp::NegTrue)
            }

            // Control Flow (B, BL, BR, RET, SVC)
            OpCode::B(condition) => self.execute_branch(OpCode::B(*condition), &ir_insn.operands),
            OpCode::BL => self.execute_branch(OpCode::BL, &ir_insn.operands),
            OpCode::BR => self.execute_branch(OpCode::BR, &ir_insn.operands),
            OpCode::BLR => self.execute_branch(OpCode::BLR, &ir_insn.operands),
            OpCode::CBZ => self.execute_branch(OpCode::CBZ, &ir_insn.operands),
            OpCode::CBNZ => self.execute_branch(OpCode::CBNZ, &ir_insn.operands),
            OpCode::TBZ => self.execute_branch(OpCode::TBZ, &ir_insn.operands),
            OpCode::TBNZ => self.execute_branch(OpCode::TBNZ, &ir_insn.operands),
            OpCode::RET => self.execute_ret(),
            OpCode::SVC => self.execute_svc(&ir_insn.operands),
            OpCode::NOP => Ok(false),
            _ => Err(EmuError::InvalidInstructionIR(format!(
                "{:?}",
                ir_insn.opcode
            ))),
        }
    }
}

#[cfg(test)]
impl CpuState {
    // Mock implementation for testing
    pub fn mock() -> Self {
        use crate::memory::Memory;
        use crate::types::STACK_TOP;
        use std::collections::{HashMap, HashSet};

        CpuState {
            registers: RefCell::new([0; 32]),
            sp: RefCell::new(STACK_TOP),
            cpsr: RefCell::new(0),
            memory: RefCell::new(Memory::new()),
            program: InterpretedProgram {
                instructions: vec![],
                label_to_ip: HashMap::new(),
                label_is_addr: HashMap::new(),
                entry_ip: 0,
                extern_labels: HashSet::new(),
                files: vec![],
                ip_map: vec![],
            },
            ip: RefCell::new(0),
            vfs: None,
            plugin_manager: None,
            did_branch: Cell::new(false),
        }
    }
}
