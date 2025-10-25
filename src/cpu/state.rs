use crate::assembler::asm_types::{
    Condition, Immediate, InstructionIR, Offset, OpCode, Operand, SymbolTable,
};
use crate::cpu::alu;
use crate::memory::Memory;
use crate::plugin::PluginManager;
use crate::syscall;
use crate::types::{EmuError, EmuResult, STACK_TOP, Word};
use crate::vfs::VirtualFileSystem;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct InterpretedProgram {
    pub instructions: Vec<InstructionIR>,
    pub label_to_ip: SymbolTable,
    pub entry_ip: usize,
    pub source_map: Vec<usize>,
    pub source_lines: Vec<String>,
}

#[derive(Clone)]
pub struct CpuState {
    pub registers: RefCell<[Word; 32]>,
    pub sp: RefCell<Word>,
    pub pstate: RefCell<Word>,
    pub memory: RefCell<Memory>,
    pub program: InterpretedProgram,
    pub ip: RefCell<usize>,
    pub vfs: Option<VirtualFileSystem>,
    pub plugin_manager: Option<Rc<RefCell<PluginManager>>>,
}

pub const N_FLAG: Word = 1 << 31;
pub const Z_FLAG: Word = 1 << 30;
pub const C_FLAG: Word = 1 << 29;
pub const V_FLAG: Word = 1 << 28;

impl CpuState {
    pub fn new(
        program: InterpretedProgram,
        plugin_manager: Option<Rc<RefCell<PluginManager>>>,
        vfs: Option<VirtualFileSystem>,
    ) -> Self {
        let cpu = CpuState {
            registers: RefCell::new([0; 32]),
            sp: RefCell::new(STACK_TOP),
            pstate: RefCell::new(0),
            memory: RefCell::new(Memory::new()),
            ip: RefCell::new(program.entry_ip),
            program,
            vfs,
            plugin_manager,
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
            *self.pstate.borrow_mut() |= V_FLAG;
        } else {
            *self.ip.borrow_mut() += 1;
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
        *self.pstate.borrow() & V_FLAG != 0
    }

    pub fn run(&mut self) -> EmuResult<()> {
        let total_insns = self.program.instructions.len();

        while *self.ip.borrow() < total_insns {
            // Pre PC increment hook
            if let Some(pm_rc) = &self.plugin_manager {
                {
                    let regs_snapshot = *self.registers.borrow();
                    let mem_snapshot = self.memory.borrow().clone();

                    let _ = drop(regs_snapshot);
                    let _ = drop(mem_snapshot);
                }

                let mut pm = pm_rc.borrow_mut();
                let skip =
                    pm.pre_pc_increment(&self.registers.borrow(), self.memory.borrow().clone())?;
                if skip {
                    *self.ip.borrow_mut() += 1;
                    continue;
                }
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

            *self.ip.borrow_mut() += 1;

            // Post PC increment hook
            if let Some(pm_rc) = &self.plugin_manager {
                {
                    let regs_snapshot = *self.registers.borrow();
                    let mem_snapshot = self.memory.borrow().clone();

                    let _ = drop(regs_snapshot);
                    let _ = drop(mem_snapshot);
                }

                let mut pm = pm_rc.borrow_mut();
                pm.post_pc_increment(&self.registers.borrow(), self.memory.borrow().clone())?;
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
                self.registers.borrow_mut()[id] = value & 0xFFFF_FFFF;
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
        println!("SP: 0x{:016X}", *self.sp.borrow());
    }

    fn update_pstate_nzcv(&mut self, result: Word, carry: bool, overflow: bool, is_sub: bool) {
        let mut new_state = 0;
        if (result >> 63) & 1 == 1 {
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
        *self.pstate.borrow_mut() = new_state;
    }

    fn check_condition(&self, cond: Condition) -> bool {
        let pstate = *self.pstate.borrow();
        let n = (pstate & N_FLAG) != 0;
        let z = (pstate & Z_FLAG) != 0;
        let v = (pstate & V_FLAG) != 0;
        match cond {
            Condition::Eq => z,
            Condition::Ne => !z,
            Condition::Ge => n == v,
            Condition::Lt => n != v,
            Condition::Gt => !z && (n == v),
            Condition::Le => z || (n != v),
            Condition::Al => true,
        }
    }

    fn resolve_operand_source(&self, operand: &Operand) -> EmuResult<Word> {
        match operand {
            Operand::Reg(r) => {
                let val = self.get_reg(r.to_id());
                Ok(if r.is_w_register() {
                    val & 0xFFFF_FFFF
                } else {
                    val
                })
            }
            Operand::Imm(Immediate::Lit(v)) => Ok(*v as Word),
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
                let val_m = self.resolve_operand_source(src2)?;
                let (result, carry, overflow) = opfunc(val_n, val_m, is_w);
                self.set_reg_with_width(rd, result, is_w);

                if update_flags {
                    self.update_pstate_nzcv(result, carry, overflow, is_sub);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Bad binary op".into())),
        }
    }

    fn execute_cmp(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        if let [op1, op2] = operands {
            let val_n = self.resolve_operand_source(op1)?;
            let val_m = self.resolve_operand_source(op2)?;
            let is_w = matches!(op1, Operand::Reg(r) if r.is_w_register());
            let (res, carry, overflow) = alu::sub(val_n, val_m, is_w);
            self.update_pstate_nzcv(res, carry, overflow, true);
            Ok(false)
        } else {
            Err(EmuError::InternalError("Invalid CMP".into()))
        }
    }

    pub fn execute_instruction_ir(&mut self, ir_insn: &InstructionIR) -> EmuResult<bool> {
        match ir_insn.opcode {
            OpCode::ADD => self.execute_binary_op(&ir_insn.operands, alu::add, false, false),
            OpCode::ADDS => self.execute_binary_op(&ir_insn.operands, alu::add, true, false),
            OpCode::SUB => self.execute_binary_op(&ir_insn.operands, alu::sub, false, true),
            OpCode::SUBS => self.execute_binary_op(&ir_insn.operands, alu::sub, true, true),
            OpCode::MUL => self.execute_binary_op(&ir_insn.operands, alu::mul, false, false),
            OpCode::MULS => self.execute_binary_op(&ir_insn.operands, alu::mul, true, false),
            OpCode::UDIV => self.execute_binary_op(&ir_insn.operands, alu::udiv, false, false),
            OpCode::SDIV => self.execute_binary_op(&ir_insn.operands, alu::sdiv, false, false),
            OpCode::AND => self.execute_binary_op(&ir_insn.operands, alu::and, false, false),
            OpCode::ORR => self.execute_binary_op(&ir_insn.operands, alu::orr, false, false),
            OpCode::EOR => self.execute_binary_op(&ir_insn.operands, alu::eor, false, false),
            OpCode::LSL => self.execute_binary_op(&ir_insn.operands, alu::lsl, false, false),
            OpCode::LSR => self.execute_binary_op(&ir_insn.operands, alu::lsr, false, false),
            OpCode::ASR => self.execute_binary_op(&ir_insn.operands, alu::asr, false, false),
            OpCode::CMP => self.execute_cmp(&ir_insn.operands),
            OpCode::MOV => self.execute_mov(&ir_insn.operands),
            OpCode::ADR => self.execute_adr(&ir_insn.operands),
            OpCode::LDR => self.execute_ldr(&ir_insn.operands),
            OpCode::LDRB => self.execute_ldrb(&ir_insn.operands),
            OpCode::STR => self.execute_str(&ir_insn.operands),
            OpCode::STRB => self.execute_strb(&ir_insn.operands),
            OpCode::B(cond) => self.execute_branch(OpCode::B(cond), &ir_insn.operands),
            OpCode::BL => self.execute_branch(OpCode::BL, &ir_insn.operands),
            OpCode::CBZ => self.execute_branch(OpCode::CBZ, &ir_insn.operands),
            OpCode::CBNZ => self.execute_branch(OpCode::CBNZ, &ir_insn.operands),
            OpCode::RET => self.execute_ret(),
            OpCode::SVC => self.execute_svc(&ir_insn.operands),
            _ => Err(EmuError::UnimplementedSyscall(format!(
                "{:?}",
                ir_insn.opcode
            ))),
        }
    }

    fn execute_branch(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [Operand::Imm(Immediate::Lbl(label))] => {
                if let OpCode::B(condition) = opcode {
                    if !self.check_condition(condition) {
                        return Ok(false);
                    }
                }

                let target_ip =
                    *self.program.label_to_ip.get(label).ok_or_else(|| {
                        EmuError::InternalError(format!("Undefined label: {}", label))
                    })? as usize;

                // Pre BL hook
                if let OpCode::BL = opcode {
                    if let Some(pm_rc) = &self.plugin_manager {
                        {
                            let regs_snapshot = *self.registers.borrow();
                            let mem_snapshot = self.memory.borrow().clone();

                            let _ = drop(regs_snapshot);
                            let _ = drop(mem_snapshot);
                        }

                        let mut pm = pm_rc.borrow_mut();
                        let _ = pm.pre_bl(
                            &self.registers.borrow(),
                            self.memory.borrow().clone(),
                            target_ip as u64,
                        )?;
                    }

                    self.set_reg(30, (target_ip) as Word);
                }

                *self.ip.borrow_mut() = target_ip.checked_sub(1).unwrap_or(0);

                // Post BL hook
                if let OpCode::BL = opcode {
                    if let Some(pm_rc) = &self.plugin_manager {
                        {
                            let regs_snapshot = *self.registers.borrow();
                            let mem_snapshot = self.memory.borrow().clone();

                            let _ = drop(regs_snapshot);
                            let _ = drop(mem_snapshot);
                        }

                        let mut pm = pm_rc.borrow_mut();
                        let _ = pm.post_bl(
                            &self.registers.borrow(),
                            self.memory.borrow().clone(),
                            target_ip as u64,
                        )?;
                    }
                }

                Ok(false)
            }

            [Operand::Reg(reg), Operand::Imm(Immediate::Lbl(label))] => {
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
                    let target_ip = *self.program.label_to_ip.get(label).ok_or_else(|| {
                        EmuError::InternalError(format!("Undefined label: {}", label))
                    })? as usize;

                    *self.ip.borrow_mut() = target_ip.saturating_sub(1);
                }

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
            {
                let regs_snapshot = *self.registers.borrow();
                let mem_snapshot = self.memory.borrow().clone();

                let _ = drop(regs_snapshot);
                let _ = drop(mem_snapshot);
            }

            let mut pm = pm_rc.borrow_mut();
            let skip = pm.pre_ret(&self.registers.borrow(), self.memory.borrow().clone())?;
            if skip {
                return Ok(false);
            }
        }

        let lr_value = self.get_reg(30);
        if lr_value == 0 {
            return Err(EmuError::InternalError(
                "LR is zero on RET; cannot return.".to_string(),
            ));
        }

        *self.ip.borrow_mut() = lr_value as usize;
        *self.ip.borrow_mut() -= 1;

        // Post RET hook
        if let Some(pm_rc) = &self.plugin_manager {
            {
                let regs_snapshot = *self.registers.borrow();
                let mem_snapshot = self.memory.borrow().clone();

                let _ = drop(regs_snapshot);
                let _ = drop(mem_snapshot);
            }

            let mut pm = pm_rc.borrow_mut();
            pm.post_ret(&self.registers.borrow(), self.memory.borrow().clone())?;
        }

        Ok(false)
    }

    pub fn execute_svc(&mut self, _ops: &[Operand]) -> EmuResult<bool> {
        let sys_call_num = self.get_reg(8);

        // Pre syscall hook
        if let Some(pm_rc) = &self.plugin_manager {
            {
                let regs_snapshot = *self.registers.borrow();
                let mem_snapshot = self.memory.borrow().clone();

                let _ = drop(regs_snapshot);
                let _ = drop(mem_snapshot);
            }

            let mut pm = pm_rc.borrow_mut();
            let skip_syscall = pm.pre_syscall(
                &self.registers.borrow(),
                self.memory.borrow().clone(),
                sys_call_num,
            )?;
            if skip_syscall {
                return Ok(false);
            }
        }

        let halt = syscall::handle_syscall(self)?;

        // Post syscall hook
        if let Some(pm_rc) = &self.plugin_manager {
            {
                let regs_snapshot = *self.registers.borrow();
                let mem_snapshot = self.memory.borrow().clone();

                let _ = drop(regs_snapshot);
                let _ = drop(mem_snapshot);
            }

            let mut pm = pm_rc.borrow_mut();
            pm.post_syscall(
                &self.registers.borrow(),
                self.memory.borrow().clone(),
                sys_call_num,
            )?;
        }

        Ok(halt)
    }

    fn execute_mov(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
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
        }
    }

    fn execute_adr(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest_op, Operand::Imm(Immediate::Lbl(label))] => {
                let (rd_id, is_w) = self.resolve_operand_dest(dest_op)?;
                let target_addr = self.program.label_to_ip.get(label).ok_or_else(|| {
                    EmuError::InternalError(format!("Undefined label: {}", label))
                })?;

                self.set_reg_with_width(rd_id, *target_addr, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid ADR operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_ldr(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest_op, Operand::Offset(offset)] => {
                let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                let effective_addr = self.resolve_offset_address(offset)?;
                let value = self.memory.borrow().read_word(effective_addr)?;
                self.set_reg_with_width(rt_id, value, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid LDR operands".into())),
        }
    }

    fn execute_ldrb(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest_op, Operand::Offset(offset)] => {
                let (rt_id, is_w) = self.resolve_operand_dest(dest_op)?;
                let effective_addr = self.resolve_offset_address(offset)?;
                let byte_value = self.memory.borrow().read_byte(effective_addr)? as Word;
                self.set_reg_with_width(rt_id, byte_value & 0xFF, is_w);
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid LDRB operands".into())),
        }
    }

    fn execute_str(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [source_op, Operand::Offset(offset)] => {
                let value = self.resolve_operand_source(source_op)?;
                let effective_addr = self.resolve_offset_address(offset)?;
                self.memory.borrow_mut().write_word(effective_addr, value)?;
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid STR operands".into())),
        }
    }

    fn execute_strb(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [source_op, Operand::Offset(offset)] => {
                let value = self.resolve_operand_source(source_op)? as u8;
                let effective_addr = self.resolve_offset_address(offset)?;
                self.memory.borrow_mut().write_byte(effective_addr, value)?;
                Ok(false)
            }
            _ => Err(EmuError::InternalError("Invalid STRB operands".into())),
        }
    }

    fn resolve_offset_address(&self, offset: &Offset) -> EmuResult<Word> {
        match offset {
            Offset::Ind1(Immediate::Lbl(label)) => {
                Ok(*self.program.label_to_ip.get(label).ok_or_else(|| {
                    EmuError::InternalError(format!("Address label '{}' not found", label))
                })?)
            }
            Offset::Ind1(Immediate::Lit(val)) => Ok(*val as Word),

            Offset::Ind2(base_reg) => Ok(self.get_reg(base_reg.to_id())),

            Offset::Ind3(base_reg, Immediate::Lit(offset_val)) => {
                let base_addr = self.get_reg(base_reg.to_id());
                Ok(base_addr.wrapping_add(*offset_val as u64))
            }

            Offset::Ind3(base_reg, Immediate::Lbl(label)) => {
                let base_addr = self.get_reg(base_reg.to_id());
                let lbl_addr = *self.program.label_to_ip.get(label).ok_or_else(|| {
                    EmuError::InternalError(format!("Label '{}' not found in offset", label))
                })?;
                Ok(base_addr.wrapping_add(lbl_addr))
            }

            Offset::Ind4(base_reg, index_reg) => {
                let base_addr = self.get_reg(base_reg.to_id());
                let idx_val = self.get_reg(index_reg.to_id());
                Ok(base_addr.wrapping_add(idx_val))
            }

            _ => Err(EmuError::InternalError(format!(
                "Unimplemented offset addressing: {:?}",
                offset
            ))),
        }
    }
}
