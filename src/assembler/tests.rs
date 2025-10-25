// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::parser::AsmParser;

    #[test]
    fn parses_simple_mov() {
        let lines = vec!["mov x0, #1".to_string()];
        let parser = AsmParser;
        let (_, map) = parser.parse_assembly_to_ir(&lines).unwrap();
        assert_eq!(map[0].0.opcode, OpCode::MOV);
        assert!(matches!(map[0].0.operands[0], Operand::Reg(Reg::X0)));
    }
}
