// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::parser::AsmParser;

    #[test]
    fn parses_tbz_operands() {
        let line = "tbz x1, #7, label1".to_string();
        let parser = AsmParser;
        let (ir_blocks, _) = parser
            .parse_assembly_to_ir(
                &vec![line],
                "test.s",
                &HashSet::new(),
                &HashMap::new(),
                "_start",
            )
            .unwrap();
        let tbz_ir = match &ir_blocks[0].content {
            AssemblyContent::Text(instrs) => &instrs[0],
            _ => panic!("Expected text block"),
        };
        assert!(matches!(tbz_ir.opcode, OpCode::TBZ));
        assert!(matches!(tbz_ir.operands[0], Operand::Reg(Reg::X1)));
        assert!(matches!(
            tbz_ir.operands[1],
            Operand::Imm(Immediate::Lit(7))
        ));
        assert!(matches!(tbz_ir.operands[2], Operand::Imm(Immediate::Lbl(ref l)) if l == "label1"));
    }
}
