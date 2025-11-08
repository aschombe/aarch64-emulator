// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

#[cfg(test)]
mod tests {
    use crate::assembler::asm_types::{Condition, Immediate, OpCode, Operand};
    use crate::cpu::alu::{add as op_add_logic, sub as op_sub_logic};
    use crate::cpu::exec_control::InstructionControl; // Import trait to enable execute_branch
    use crate::cpu::state::CpuState;

    #[test]
    fn add_and_sub_works() {
        assert_eq!(op_add_logic(5, 3, false).0, 8);
        assert_eq!(op_sub_logic(5, 3, false).0, 2);
    }

    #[test]
    fn branch_sets_ip_correctly() {
        let mut cpu = CpuState::mock();
        cpu.program.label_to_ip.insert("target".into(), 5);
        cpu.execute_branch(
            OpCode::B(Condition::Al),
            &[Operand::Imm(Immediate::Lbl("target".into()))],
        )
        .unwrap();
        assert_eq!(*cpu.ip.borrow(), 5);
    }
}
