#[cfg(test)]
mod tests {
    use super::*;
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
