use crate::assembler::asm_types::{
    Condition, Immediate, InstructionIR, Offset, OpCode, Operand, SymbolTable,
};
use crate::memory::Memory;
use crate::plugin::PluginManager;
use crate::syscall;
use crate::types::{EmuError, EmuResult, STACK_TOP, Word};
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

    pub plugin_manager: Option<Rc<RefCell<PluginManager>>>,
}

pub const N_FLAG: Word = 1 << 31; // Negative
pub const Z_FLAG: Word = 1 << 30; // Zero
pub const C_FLAG: Word = 1 << 29; // Carry/Borrow
pub const V_FLAG: Word = 1 << 28; // Overflow

impl CpuState {
    pub fn new(
        program: InterpretedProgram,
        plugin_manager: Option<Rc<RefCell<PluginManager>>>,
    ) -> Self {
        let cpu = CpuState {
            registers: RefCell::new([0; 32]),
            sp: RefCell::new(0),
            pstate: RefCell::new(0),
            memory: RefCell::new(Memory::new()),
            ip: RefCell::new(program.entry_ip),
            program,
            plugin_manager,
        };

        *cpu.sp.borrow_mut() = STACK_TOP;
        cpu.registers.borrow_mut()[30] = 0;

        cpu
    }

    pub fn step_instruction(&mut self) -> EmuResult<()> {
        if self.halted() {
            return Err(EmuError::InternalError("CPU already halted".to_string()));
        }

        let current_ip = *self.ip.borrow();
        let ir_insn = self
            .program
            .instructions
            .get(current_ip)
            .ok_or_else(|| EmuError::InternalError("IP out of range".to_string()))?
            .clone();

        let halt = self.execute_instruction_ir(&ir_insn)?;

        if halt {
            *self.pstate.borrow_mut() |= V_FLAG; // Mark as halted
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
        // Here we check for V_FLAG or other halt indication
        *self.pstate.borrow() & V_FLAG != 0
    }

    fn op_add_logic(val_n: Word, val_m: Word) -> (Word, bool, bool) {
        let (res, carry) = val_n.overflowing_add(val_m);

        let n_sign = (val_n >> 63) & 1;
        let m_sign = (val_m >> 63) & 1;
        let res_sign = (res >> 63) & 1;

        let operands_have_same_sign = n_sign == m_sign;
        let result_sign_differs = n_sign != res_sign;

        let overflow = operands_have_same_sign && result_sign_differs;
        (res, carry, overflow)
    }

    fn op_sub_logic(val_n: Word, val_m: Word) -> (Word, bool, bool) {
        let (res, borrow) = val_n.overflowing_sub(val_m);

        let n_sign = (val_n >> 63) & 1;
        let m_sign = (val_m >> 63) & 1;
        let res_sign = (res >> 63) & 1;

        let operands_have_different_signs = n_sign != m_sign;
        let result_sign_differs = n_sign != res_sign;

        let overflow = operands_have_different_signs && result_sign_differs;

        (res, borrow, overflow)
    }

    fn op_mul(val_n: Word, val_m: Word) -> (Word, bool, bool) {
        (val_n.wrapping_mul(val_m), false, false)
    }
    fn op_and(val_n: Word, val_m: Word) -> (Word, bool, bool) {
        (val_n & val_m, false, false)
    }
    fn op_orr(val_n: Word, val_m: Word) -> (Word, bool, bool) {
        (val_n | val_m, false, false)
    }
    fn op_eor(val_n: Word, val_m: Word) -> (Word, bool, bool) {
        (val_n ^ val_m, false, false)
    }
    fn op_udiv(_val_n: Word, _val_m: Word) -> (Word, bool, bool) {
        (0, false, false)
    }
    fn op_sdiv(_val_n: Word, _val_m: Word) -> (Word, bool, bool) {
        (0, false, false)
    }
    fn op_lsl(val: Word, shift: Word) -> (Word, bool, bool) {
        (val.wrapping_shl(shift as u32), false, false)
    }
    fn op_lsr(val: Word, shift: Word) -> (Word, bool, bool) {
        (val.wrapping_shr(shift as u32), false, false)
    }
    fn op_asr(val: Word, shift: Word) -> (Word, bool, bool) {
        let signed_val = val as i64;
        let res = signed_val.wrapping_shr(shift as u32);
        (res as Word, false, false)
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
        if id == 31 {
            // XZR - do nothing
        } else if id == 32 {
            *self.sp.borrow_mut() = value;
        } else {
            self.registers.borrow_mut()[id] = value;
        }
    }

    pub fn dump_state_full(&self) {
        println!("--- REGISTER STATE ---");
        for i in 0..=31 {
            if (i != 0) && (i % 4 == 0) {
                print!("\n");
            }

            if i % 4 != 3 {
                print!("X{:02}: 0x{:016X} | ", i, self.registers.borrow()[i]);
            } else {
                print!("X{:02}: 0x{:016X}   ", i, self.registers.borrow()[i]);
            }
        }

        println!("\nLR (X30): 0x{:016X}", self.registers.borrow()[30]);
        println!("SP: 0x{:016X}", *self.sp.borrow());
        println!("----------------------");
    }

    // pub fn dump_state_interactive(&self, last_insn: Option<&InstructionIR>) {
    //     let pc_addr = self
    //         .program
    //         .label_to_ip
    //         .get("_start")
    //         .unwrap_or(&0)
    //         .checked_add(self.ip as u64 * 4)
    //         .unwrap_or(0);
    //
    //     println!("\n=========================================================");
    //     if let Some(insn) = last_insn {
    //         println!("= INSTRUCTION EXECUTED: {:?}", insn);
    //     } else {
    //         println!("= EMULATOR START");
    //     }
    //     println!("=========================================================");
    //
    //     println!("IP: {:04} | PC: 0x{:08X}", self.ip, pc_addr);
    //     self.dump_state_full();
    //
    //     if let Some(ir_insn) = self.program.instructions.get(self.ip) {
    //         println!("--> NEXT INSTRUCTION (IP {}): {:?}", self.ip, ir_insn);
    //     } else {
    //         println!("--> PROGRAM END REACHED.");
    //     }
    // }

    fn update_pstate_nzcv(
        &mut self,
        result: Word,
        carry_or_borrow: bool,
        overflow: bool,
        is_sub: bool,
    ) {
        *self.pstate.borrow_mut() = 0;
        if (result as i64) < 0 {
            *self.pstate.borrow_mut() |= N_FLAG;
        }
        if result == 0 {
            *self.pstate.borrow_mut() |= Z_FLAG;
        }
        if is_sub {
            if !carry_or_borrow {
                *self.pstate.borrow_mut() |= C_FLAG;
            }
        } else if carry_or_borrow {
            *self.pstate.borrow_mut() |= C_FLAG;
        }
        if overflow {
            *self.pstate.borrow_mut() |= V_FLAG;
        }
    }

    fn resolve_operand_source(&self, operand: &Operand) -> EmuResult<Word> {
        match operand {
            Operand::Reg(reg) => Ok(self.get_reg(reg.to_id())),
            Operand::Imm(Immediate::Lit(val)) => Ok(*val as Word),
            _ => Err(EmuError::InternalError(format!(
                "Unsupported operand source: {:?}",
                operand
            ))),
        }
    }

    fn resolve_operand_dest(&self, operand: &Operand) -> EmuResult<usize> {
        match operand {
            Operand::Reg(reg) => Ok(reg.to_id()),
            _ => Err(EmuError::InternalError(format!(
                "Invalid destination operand: {:?}",
                operand
            ))),
        }
    }

    fn resolve_offset_address(&self, offset: &Offset) -> EmuResult<Word> {
        match offset {
            Offset::Ind1(Immediate::Lbl(label)) => {
                Ok(*self.program.label_to_ip.get(label).ok_or_else(|| {
                    EmuError::InternalError(format!("Address label '{}' not found.", label))
                })?)
            }
            Offset::Ind2(base_reg) => Ok(self.get_reg(base_reg.to_id())),
            Offset::Ind3(base_reg, Immediate::Lit(offset_val)) => {
                let base_addr = self.get_reg(base_reg.to_id());
                Ok(base_addr.wrapping_add(*offset_val as Word))
            }
            Offset::Ind4(base_reg, index_reg) => {
                let base_addr = self.get_reg(base_reg.to_id());
                let index_val = self.get_reg(index_reg.to_id());
                Ok(base_addr.wrapping_add(index_val))
            }
            _ => Err(EmuError::InternalError(format!(
                "Unimplemented or Invalid offset addressing mode: {:?}",
                offset
            ))),
        }
    }

    fn execute_binary_op(
        &mut self,
        operands: &[Operand],
        op_func: fn(Word, Word) -> (Word, bool, bool),
        update_flags: bool,
        is_sub: bool,
    ) -> EmuResult<bool> {
        match operands {
            [dest_op, source1_op, source2_op] => {
                let rd_id = self.resolve_operand_dest(dest_op)?;
                let val_n = self.resolve_operand_source(source1_op)?;
                let val_m = self.resolve_operand_source(source2_op)?;

                // If its udiv or sdiv, check for division by zero
                if (op_func as usize) == (Self::op_udiv as usize)
                    || (op_func as usize) == (Self::op_sdiv as usize)
                {
                    if val_m == 0 {
                        return Err(EmuError::DivisionByZero);
                    }
                }

                let (result, carry_or_borrow, overflow) = op_func(val_n, val_m);
                self.set_reg(rd_id, result);

                if update_flags {
                    self.update_pstate_nzcv(result, carry_or_borrow, overflow, is_sub);
                }
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid binary operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_cmp(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [source1_op, source2_op] => {
                let val_n = self.resolve_operand_source(source1_op)?;
                let val_m = self.resolve_operand_source(source2_op)?;

                // CMP is SUBS XZR, Rn, Rm.
                let (result, borrow, overflow) = Self::op_sub_logic(val_n, val_m);

                self.update_pstate_nzcv(result, borrow, overflow, true);

                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid CMP operands: {:?}",
                operands
            ))),
        }
    }

    fn check_condition(&self, condition: Condition) -> bool {
        let pstate = *self.pstate.borrow();
        let n = (pstate & N_FLAG) != 0;
        let z = (pstate & Z_FLAG) != 0;
        let v = (pstate & V_FLAG) != 0;

        match condition {
            Condition::Al => true,
            Condition::Eq => z,
            Condition::Ne => !z,
            Condition::Gt => !z && (n == v),
            Condition::Ge => n == v,
            Condition::Lt => n != v,
            Condition::Le => z || (n != v),
        }
    }

    fn execute_mov(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest_op, src_op] => {
                let rd_id = self.resolve_operand_dest(dest_op)?;
                let src_val = self.resolve_operand_source(src_op)?;
                self.set_reg(rd_id, src_val);
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
                let rd_id = self.resolve_operand_dest(dest_op)?;
                let target_addr = self.program.label_to_ip.get(label).ok_or_else(|| {
                    EmuError::InternalError(format!("Undefined label: {}", label))
                })?;

                self.set_reg(rd_id, *target_addr);
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
                let rt_id = self.resolve_operand_dest(dest_op)?;
                let effective_addr = self.resolve_offset_address(offset)?;

                // let value = self.memory.read_word(effective_addr)?;
                let value = self.memory.borrow().read_word(effective_addr)?;
                self.set_reg(rt_id, value);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid LDR operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_ldrb(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [dest_op, Operand::Offset(offset)] => {
                let rt_id = self.resolve_operand_dest(dest_op)?;
                let effective_addr = self.resolve_offset_address(offset)?;

                // let byte_value = self.memory.read_byte(effective_addr)?;
                let byte_value = self.memory.borrow().read_byte(effective_addr)?;
                self.set_reg(rt_id, byte_value as Word);
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid LDRB operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_str(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [source_op, Operand::Offset(offset)] => {
                let rt_val = self.resolve_operand_source(source_op)?;
                let effective_addr = self.resolve_offset_address(offset)?;

                // self.memory.write_word(effective_addr, rt_val)?;
                self.memory
                    .borrow_mut()
                    .write_word(effective_addr, rt_val)?;
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid STR operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_strb(&mut self, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [source_op, Operand::Offset(offset)] => {
                let rt_val = self.resolve_operand_source(source_op)? as u8;
                let effective_addr = self.resolve_offset_address(offset)?;

                self.memory
                    .borrow_mut()
                    .write_byte(effective_addr, rt_val)?;
                Ok(false)
            }
            _ => Err(EmuError::InternalError(format!(
                "Invalid STRB operands: {:?}",
                operands
            ))),
        }
    }

    fn execute_branch(&mut self, opcode: OpCode, operands: &[Operand]) -> EmuResult<bool> {
        match operands {
            [Operand::Imm(Immediate::Lbl(label))] => {
                if let OpCode::B(condition) = opcode {
                    if !self.check_condition(condition) {
                        return Ok(false); // Condition not met, skip jump
                    }
                }

                let target_ip =
                    *self.program.label_to_ip.get(label).ok_or_else(|| {
                        EmuError::InternalError(format!("Undefined label: {}", label))
                    })? as usize;

                if let OpCode::BL = opcode {
                    if let Some(pm_rc) = &self.plugin_manager {
                        {
                            let regs_snapshot = *self.registers.borrow();
                            let mem_snapshot = self.memory.borrow().clone();

                            let _ = drop(regs_snapshot);
                            let _ = drop(mem_snapshot);
                        }

                        let mut pm = pm_rc.borrow_mut();
                        pm.pre_bl(
                            &self.registers.borrow(),
                            self.memory.borrow().clone(),
                            target_ip as u64,
                        )?;
                    }

                    // self.set_reg(30, (self.ip + 1) as Word);
                    self.set_reg(30, (target_ip) as Word);
                }

                *self.ip.borrow_mut() = target_ip.checked_sub(1).unwrap_or(0);

                if let OpCode::BL = opcode {
                    if let Some(pm_rc) = &self.plugin_manager {
                        {
                            let regs_snapshot = *self.registers.borrow();
                            let mem_snapshot = self.memory.borrow().clone();

                            let _ = drop(regs_snapshot);
                            let _ = drop(mem_snapshot);
                        }

                        let mut pm = pm_rc.borrow_mut();
                        pm.post_bl(
                            &self.registers.borrow(),
                            self.memory.borrow().clone(),
                            target_ip as u64,
                        )?;
                    }
                }

                Ok(false)
            }
            [Operand::Reg(reg), Operand::Imm(Immediate::Lbl(label))] => {
                let reg_val = self.get_reg(reg.to_id());
                let condition_met = match opcode {
                    OpCode::CBZ => reg_val == 0,
                    OpCode::CBNZ => reg_val != 0,
                    _ => {
                        return Err(EmuError::InternalError(format!(
                            "Invalid opcode for conditional branch with register: {:?}",
                            opcode
                        )));
                    }
                };

                if condition_met {
                    let target_ip = *self.program.label_to_ip.get(label).ok_or_else(|| {
                        EmuError::InternalError(format!("Undefined label: {}", label))
                    })? as usize;
                    *self.ip.borrow_mut() = target_ip.checked_sub(1).unwrap_or(0);
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
        if let Some(pm_rc) = &self.plugin_manager {
            {
                let regs_snapshot = *self.registers.borrow();
                let mem_snapshot = self.memory.borrow().clone();

                let _ = drop(regs_snapshot);
                let _ = drop(mem_snapshot);
            }

            let mut pm = pm_rc.borrow_mut();
            pm.pre_ret(&self.registers.borrow(), self.memory.borrow().clone())?;
        }

        let lr_value = self.get_reg(30);
        if lr_value == 0 {
            return Err(EmuError::InternalError(
                "LR is zero on RET; cannot return.".to_string(),
            ));
        }

        *self.ip.borrow_mut() = lr_value as usize;
        *self.ip.borrow_mut() -= 1;

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

    pub fn execute_svc(&mut self, _operands: &[Operand]) -> EmuResult<bool> {
        // Call pre-syscall hooks
        let sys_call_num = self.get_reg(8);

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

        // Call post-syscall hooks
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

    pub fn run(&mut self) -> EmuResult<()> {
        let total_insns = self.program.instructions.len();

        while *self.ip.borrow() < total_insns {
            // --- Phase 1: Pre-execution hooks ---
            if let Some(pm_rc) = &self.plugin_manager {
                pm_rc.borrow().run_hooks("pre_pc_increment").ok();
            }

            // --- Phase 2: Execute a single instruction ---
            let current_ip = *self.ip.borrow();
            let ir_insn = self
                .program
                .instructions
                .get(current_ip)
                .ok_or_else(|| {
                    EmuError::InternalError(format!("Invalid instruction pointer: {}", current_ip))
                })?
                .clone();

            let halted = self.execute_instruction_ir(&ir_insn)?;
            if halted {
                return Ok(());
            }

            *self.ip.borrow_mut() += 1;

            // --- Phase 3: Post-execution hooks ---
            if let Some(pm_rc) = &self.plugin_manager {
                pm_rc.borrow().run_hooks("post_pc_increment").ok();
            }
        }

        Ok(())
    }

    pub fn execute_instruction_ir(&mut self, ir_insn: &InstructionIR) -> EmuResult<bool> {
        let result = match ir_insn.opcode {
            OpCode::ADD => {
                self.execute_binary_op(&ir_insn.operands, Self::op_add_logic, false, false)
            }
            OpCode::ADDS => {
                self.execute_binary_op(&ir_insn.operands, Self::op_add_logic, true, false)
            }
            OpCode::SUB => {
                self.execute_binary_op(&ir_insn.operands, Self::op_sub_logic, false, true)
            }
            OpCode::SUBS => {
                self.execute_binary_op(&ir_insn.operands, Self::op_sub_logic, true, true)
            }
            OpCode::MUL => self.execute_binary_op(&ir_insn.operands, Self::op_mul, false, false),
            OpCode::MULS => self.execute_binary_op(&ir_insn.operands, Self::op_mul, true, false),
            OpCode::UDIV => self.execute_binary_op(&ir_insn.operands, Self::op_udiv, false, false),
            OpCode::SDIV => self.execute_binary_op(&ir_insn.operands, Self::op_sdiv, false, false),

            OpCode::AND => self.execute_binary_op(&ir_insn.operands, Self::op_and, false, false),
            OpCode::ORR => self.execute_binary_op(&ir_insn.operands, Self::op_orr, false, false),
            OpCode::EOR => self.execute_binary_op(&ir_insn.operands, Self::op_eor, false, false),

            OpCode::LSL => self.execute_binary_op(&ir_insn.operands, Self::op_lsl, false, false),
            OpCode::LSR => self.execute_binary_op(&ir_insn.operands, Self::op_lsr, false, false),
            OpCode::ASR => self.execute_binary_op(&ir_insn.operands, Self::op_asr, false, false),

            OpCode::MOV => self.execute_mov(&ir_insn.operands),
            OpCode::ADR => self.execute_adr(&ir_insn.operands),
            OpCode::LDR => self.execute_ldr(&ir_insn.operands),
            OpCode::LDRB => self.execute_ldrb(&ir_insn.operands),
            OpCode::STR => self.execute_str(&ir_insn.operands),
            OpCode::STRB => self.execute_strb(&ir_insn.operands),

            OpCode::CMP => self.execute_cmp(&ir_insn.operands),
            OpCode::B(cnd) => self.execute_branch(OpCode::B(cnd), &ir_insn.operands),
            OpCode::BL => self.execute_branch(OpCode::BL, &ir_insn.operands),
            OpCode::CBZ => self.execute_branch(OpCode::CBZ, &ir_insn.operands),
            OpCode::CBNZ => self.execute_branch(OpCode::CBNZ, &ir_insn.operands),
            OpCode::RET => self.execute_ret(),

            OpCode::SVC => self.execute_svc(&ir_insn.operands),

            _ => Err(EmuError::UnimplementedSyscall(format!(
                "Unimplemented OpCode: {:?}",
                ir_insn.opcode
            ))),
        };

        result
    }
}
