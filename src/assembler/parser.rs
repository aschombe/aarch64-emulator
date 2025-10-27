// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{
    AssemblyBlock, AssemblyContent, Condition, Data, Immediate, InstructionIR, Offset, OpCode,
    Operand, Reg,
};
use crate::types::{EmuError, EmuResult, Word};

fn is_register(s: &str) -> bool {
    let s = s.to_lowercase();
    (s.starts_with('x') && s.len() <= 3 && s[1..].chars().all(|c| c.is_digit(10)))
        || (s.starts_with('w') && s.len() <= 3 && s[1..].chars().all(|c| c.is_digit(10)))
        || s == "sp"
        || s == "lr"
        || s == "xzr"
        || s == "wzr"
}

fn parse_reg(s: &str) -> EmuResult<Reg> {
    let s = s.to_uppercase();
    match s.as_str() {
        "X0" => Ok(Reg::X0),
        "X1" => Ok(Reg::X1),
        "X2" => Ok(Reg::X2),
        "X3" => Ok(Reg::X3),
        "X4" => Ok(Reg::X4),
        "X5" => Ok(Reg::X5),
        "X6" => Ok(Reg::X6),
        "X7" => Ok(Reg::X7),
        "X8" => Ok(Reg::X8),
        "X9" => Ok(Reg::X9),
        "X10" => Ok(Reg::X10),
        "X11" => Ok(Reg::X11),
        "X12" => Ok(Reg::X12),
        "X13" => Ok(Reg::X13),
        "X14" => Ok(Reg::X14),
        "X15" => Ok(Reg::X15),
        "X16" => Ok(Reg::X16),
        "X17" => Ok(Reg::X17),
        "X18" => Ok(Reg::X18),
        "X19" => Ok(Reg::X19),
        "X20" => Ok(Reg::X20),
        "X21" => Ok(Reg::X21),
        "X22" => Ok(Reg::W22),
        "X23" => Ok(Reg::X23),
        "X24" => Ok(Reg::X24),
        "X25" => Ok(Reg::X25),
        "X26" => Ok(Reg::X26),
        "X27" => Ok(Reg::X27),
        "X28" => Ok(Reg::X28),
        "X29" => Ok(Reg::X29),
        "X30" => Ok(Reg::LR),
        "LR" => Ok(Reg::LR),
        "X31" => Ok(Reg::XZR),
        "XZR" => Ok(Reg::XZR),
        "W0" => Ok(Reg::W0),
        "W1" => Ok(Reg::W1),
        "W2" => Ok(Reg::W2),
        "W3" => Ok(Reg::W3),
        "W4" => Ok(Reg::W4),
        "W5" => Ok(Reg::W5),
        "W6" => Ok(Reg::W6),
        "W7" => Ok(Reg::W7),
        "W8" => Ok(Reg::W8),
        "W9" => Ok(Reg::W9),
        "W10" => Ok(Reg::W10),
        "W11" => Ok(Reg::W11),
        "W12" => Ok(Reg::W12),
        "W13" => Ok(Reg::W13),
        "W14" => Ok(Reg::W14),
        "W15" => Ok(Reg::W15),
        "W16" => Ok(Reg::W16),
        "W17" => Ok(Reg::W17),
        "W18" => Ok(Reg::W18),
        "W19" => Ok(Reg::W19),
        "W20" => Ok(Reg::W20),
        "W21" => Ok(Reg::W21),
        "W22" => Ok(Reg::W22),
        "W23" => Ok(Reg::W23),
        "W24" => Ok(Reg::W24),
        "W25" => Ok(Reg::W25),
        "W26" => Ok(Reg::W26),
        "W27" => Ok(Reg::W27),
        "W28" => Ok(Reg::W28),
        "W29" => Ok(Reg::W29),
        "W30" => Ok(Reg::W30),
        "W31" => Ok(Reg::W31),
        "WZR" => Ok(Reg::W31),
        "SP" => Ok(Reg::SP),

        _ => Err(EmuError::InternalError(format!(
            "Invalid register name: {}",
            s
        ))),
    }
}

fn is_numeric(s: &str) -> bool {
    let s = s.trim_start_matches('#');
    s.starts_with(|c: char| c.is_digit(10) || c == '-')
        || s.starts_with("0x")
        || s.starts_with("0b")
}

fn is_label(s: &str) -> bool {
    !s.is_empty() && s.chars().next().unwrap().is_alphabetic() && !is_register(s)
}

/// Cleans a vector of lines, stripping out block comments (/* ... */) AND single-line comments (//).
fn clean_source_code(lines: &[String]) -> Vec<String> {
    let mut cleaned_lines = Vec::new();
    let mut in_block_comment = false;

    for line in lines {
        let mut new_line = String::new();
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if in_block_comment {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    in_block_comment = false;
                }
            } else {
                if c == '/' && chars.peek() == Some(&'*') {
                    chars.next();
                    in_block_comment = true;
                } else if c == '/' && chars.peek() == Some(&'/') {
                    break;
                } else {
                    new_line.push(c);
                }
            }
        }

        if !new_line.trim().is_empty() || in_block_comment {
            cleaned_lines.push(new_line);
        }
    }

    cleaned_lines
}

pub struct AsmParser;

impl AsmParser {
    fn parse_immediate(&self, token: &str) -> EmuResult<Immediate> {
        let clean_token = token.trim_start_matches('#');

        if is_numeric(clean_token) {
            match clean_token.parse::<i64>() {
                Ok(val) => Ok(Immediate::Lit(val)),
                Err(_) => Err(EmuError::InternalError(format!(
                    "Invalid number format: {}",
                    clean_token
                ))),
            }
        } else if is_label(clean_token) {
            Ok(Immediate::Lbl(clean_token.to_string()))
        } else {
            Err(EmuError::InternalError(format!(
                "Invalid immediate value: {}",
                token
            )))
        }
    }

    fn clean_offset_parts(&self, token: &str) -> Option<Vec<String>> {
        let content = token.trim_matches(|c| c == '[' || c == ']').trim();
        if content.is_empty() {
            return None;
        }

        let parts: Vec<String>;

        if content.contains(',') {
            parts = content
                .splitn(2, ',')
                .map(|s| s.trim().to_string())
                .collect();
        } else if content.contains(' ') {
            parts = content.split_whitespace().map(|s| s.to_string()).collect();
        } else {
            parts = vec![content.to_string()];
        }

        Some(parts.into_iter().filter(|s| !s.is_empty()).collect())
    }

    fn parse_condition(&self, full_mnemonic: &str) -> EmuResult<Condition> {
        let parts: Vec<&str> = full_mnemonic.split('.').collect();

        let cond_str = if parts.len() > 1 {
            parts[1].to_uppercase()
        } else {
            return Ok(Condition::Al);
        };

        match cond_str.as_str() {
            "EQ" => Ok(Condition::Eq),
            "NE" => Ok(Condition::Ne),
            "LT" => Ok(Condition::Lt),
            "LE" => Ok(Condition::Le),
            "GT" => Ok(Condition::Gt),
            "GE" => Ok(Condition::Ge),
            "AL" => Ok(Condition::Al),
            _ => Err(EmuError::InternalError(format!(
                "Invalid condition code: {}",
                cond_str
            ))),
        }
    }

    /// Parses a string literal (e.g., "Hello\nWorld") into a byte vector.
    fn parse_string_literal(&self, literal: &str) -> EmuResult<Vec<u8>> {
        let literal = literal.trim().trim_matches('"');
        let mut bytes = Vec::new();
        let mut chars = literal.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                let escaped_char = match chars.next() {
                    Some('n') => b'\n',
                    Some('t') => b'\t',
                    Some('r') => b'\r',
                    Some('\\') => b'\\',
                    Some('"') => b'"',
                    Some(other) => {
                        return Err(EmuError::InternalError(format!(
                            "Unknown escape sequence: \\{}",
                            other
                        )));
                    }
                    None => {
                        return Err(EmuError::InternalError(
                            "Incomplete escape sequence at end of string.".to_string(),
                        ));
                    }
                };
                bytes.push(escaped_char);
            } else {
                bytes.push(c as u8);
            }
        }
        Ok(bytes)
    }

    /// Parses data definition directives (.quad, .string, .ascii, .asciiz, .skip, .int, .word).
    fn parse_data_definition(
        &self,
        line_content: &str,
        original_line_number: usize,
    ) -> EmuResult<Data> {
        let parts: Vec<&str> = line_content.split_whitespace().collect();
        let directive = parts[0].to_lowercase();

        if parts.len() < 2 {
            return Err(EmuError::InternalError(format!(
                "Data directive '{}' requires arguments on line {}.",
                directive, original_line_number
            )));
        }

        match directive.as_str() {
            ".quad" => {
                // Join everything after directive into one string without removing spaces inside numbers
                let values_str = parts[1..].join(" ");
                let values: Result<Vec<i64>, _> = values_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().parse::<i64>())
                    .collect();

                match values {
                    Ok(v) => Ok(Data::QuadArr(v)),
                    Err(_) => Err(EmuError::InternalError(format!(
                        "Invalid .quad values on line {}: {}",
                        original_line_number, line_content
                    ))),
                }
            }

            // Handle string directives (.string, .asciiz) by extracting the literal substring starting at first quote
            ".string" | ".ascii" | ".asciiz" => {
                // Find index of first quote to get exact literal including spaces
                let quote_pos = line_content.find('"').ok_or_else(|| {
                    EmuError::InternalError(format!(
                        "Missing opening quote on line {}",
                        original_line_number
                    ))
                })?;
                let literal = &line_content[quote_pos..]; // from first quote to end

                let mut bytes = self.parse_string_literal(literal)?;
                if directive == ".asciiz" || directive == ".string" {
                    bytes.push(0);
                }
                Ok(Data::ByteArr(bytes))
            }

            ".skip" => {
                if parts.len() < 2 {
                    return Err(EmuError::InternalError(format!(
                        ".skip directive requires a size argument on line {}.",
                        original_line_number
                    )));
                }
                // Parse size and optional fill byte (default 0)
                let size = parts[1].parse::<usize>().map_err(|_| {
                    EmuError::InternalError(format!(
                        "Invalid .skip size argument on line {}: {}",
                        original_line_number, parts[1]
                    ))
                })?;

                // For now, just zero-fill the skip space as a byte array
                Ok(Data::ByteArr(vec![0u8; size]))
            }

            ".word" | ".int" => {
                let values_str = parts[1..].join(" ");
                let values: Result<Vec<i32>, _> = values_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().parse::<i32>())
                    .collect();
                match values {
                    Ok(v) => Ok(Data::IntArr(v)),
                    Err(_) => Err(EmuError::InternalError(format!(
                        "Invalid .word/.int values on line {}: {}",
                        original_line_number, line_content
                    ))),
                }
            }

            ".space" => {
                if parts.len() < 2 {
                    return Err(EmuError::InternalError(format!(
                        ".space directive requires a size argument on line {}.",
                        original_line_number
                    )));
                }
                let size = parts[1].parse::<usize>().map_err(|_| {
                    EmuError::InternalError(format!(
                        "Invalid .space size argument on line {}: {}",
                        original_line_number, parts[1]
                    ))
                })?;
                Ok(Data::ByteArr(vec![0u8; size]))
            }

            _ => Err(EmuError::InternalError(format!(
                "Unimplemented data definition on line {}: {}",
                original_line_number, line_content
            ))),
        }
    }

    fn parse_operand(&self, token: &str) -> EmuResult<Operand> {
        let token = token.trim();
        if token.is_empty() {
            return Err(EmuError::InternalError("Operand is empty.".to_string()));
        }

        if token.starts_with('[') && token.ends_with(']') {
            let parts = self.clean_offset_parts(token).ok_or_else(|| {
                EmuError::InternalError(format!("Empty offset operand: {}", token))
            })?;

            let offset = match parts.as_slice() {
                [p1] => {
                    if is_register(p1) {
                        Offset::Ind2(parse_reg(p1)?)
                    } else {
                        Offset::Ind1(self.parse_immediate(p1)?)
                    }
                }
                [p1, p2] => {
                    let reg1 = parse_reg(p1)?;
                    if is_register(p2) {
                        Offset::Ind4(reg1, parse_reg(p2)?)
                    } else {
                        Offset::Ind3(reg1, self.parse_immediate(p2)?)
                    }
                }
                _ => {
                    return Err(EmuError::InternalError(format!(
                        "Invalid offset format: {}",
                        token
                    )));
                }
            };
            return Ok(Operand::Offset(offset));
        }

        if token.starts_with('#') || is_numeric(token) || is_label(token) {
            return Ok(Operand::Imm(self.parse_immediate(token)?));
        }

        if is_register(token) {
            return Ok(Operand::Reg(parse_reg(token)?));
        }

        Err(EmuError::InternalError(format!(
            "Unrecognized token/operand: {}",
            token
        )))
    }

    /// Parses a single line of assembly (non-directive/non-label) into an InstructionIR.
    fn parse_instruction(&self, line: &str) -> EmuResult<InstructionIR> {
        let cleaned_line = line.trim().replace(',', " ");
        if cleaned_line.is_empty() {
            return Err(EmuError::InternalError("Empty line.".to_string()));
        }

        let mut tokens: Vec<String> = Vec::new();
        let mut current_token = String::new();
        let mut in_offset = false;

        for part in cleaned_line.split_whitespace() {
            if part.starts_with('[') {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                in_offset = true;
                current_token.push_str(part);
            } else if in_offset {
                current_token.push(' ');
                current_token.push_str(part);

                if part.ends_with(']') {
                    tokens.push(current_token.clone());
                    current_token.clear();
                    in_offset = false;
                }
            } else {
                tokens.push(part.to_string());
            }
        }
        if !current_token.is_empty() {
            tokens.push(current_token);
        }
        tokens.retain(|s| !s.is_empty());

        if tokens.is_empty() {
            return Err(EmuError::InternalError(
                "Empty instruction line.".to_string(),
            ));
        }

        let full_mnemonic = tokens[0].to_uppercase();
        let base_mnemonic = full_mnemonic.split('.').next().unwrap().to_uppercase();

        let opcode = match full_mnemonic.as_str() {
            "MOV" => OpCode::MOV,
            "ADD" => OpCode::ADD,
            "SUB" => OpCode::SUB,
            "MUL" => OpCode::MUL,
            "UDIV" => OpCode::UDIV,
            "SDIV" => OpCode::SDIV,
            "ADDS" => OpCode::ADDS,
            "SUBS" => OpCode::SUBS,
            "MULS" => OpCode::MULS,
            "UDIVS" => OpCode::UDIVS,
            "SDIVS" => OpCode::SDIVS,
            "AND" => OpCode::AND,
            "ANDS" => OpCode::ANDS,
            "ORR" => OpCode::ORR,
            "EOR" => OpCode::EOR,
            "NOT" => OpCode::NOT,
            "LSL" => OpCode::LSL,
            "LSR" => OpCode::LSR,
            "ASR" => OpCode::ASR,
            "ADR" => OpCode::ADR,
            "LDR" => OpCode::LDR,
            "LDRB" => OpCode::LDRB,
            "LDRH" => OpCode::LDRH,
            "LDRSB" => OpCode::LDRSB,
            "LDRSH" => OpCode::LDRSH,
            "STR" => OpCode::STR,
            "STRB" => OpCode::STRB,
            "STRH" => OpCode::STRH,
            "BL" => OpCode::BL,
            "BR" => OpCode::BR,
            "B" => OpCode::B(Condition::Al),
            "B.EQ" => OpCode::B(Condition::Eq),
            "BEQ" => OpCode::B(Condition::Eq),
            "B.NE" => OpCode::B(Condition::Ne),
            "BNE" => OpCode::B(Condition::Ne),
            "B.LT" => OpCode::B(Condition::Lt),
            "BLT" => OpCode::B(Condition::Lt),
            "B.LE" => OpCode::B(Condition::Le),
            "BLE" => OpCode::B(Condition::Le),
            "B.GT" => OpCode::B(Condition::Gt),
            "BGT" => OpCode::B(Condition::Gt),
            "B.GE" => OpCode::B(Condition::Ge),
            "BGE" => OpCode::B(Condition::Ge),
            "RET" => OpCode::RET,
            "CMP" => OpCode::CMP,
            "CBZ" => OpCode::CBZ,
            "CBNZ" => OpCode::CBNZ,
            "SVC" => OpCode::SVC,
            "NOP" => OpCode::NOP,

            _ if base_mnemonic == "B" => OpCode::B(self.parse_condition(&full_mnemonic)?),

            _ => {
                return Err(EmuError::InternalError(format!(
                    "Unknown mnemonic: {}",
                    full_mnemonic
                )));
            }
        };

        let mut operands = Vec::new();
        for token in &tokens[1..] {
            operands.push(self.parse_operand(token)?);
        }

        Ok(InstructionIR { opcode, operands })
    }

    /// Main entry point for parsing assembly source lines into IR blocks.
    pub fn parse_assembly_to_ir(
        &self,
        lines: &Vec<String>,
    ) -> EmuResult<(Vec<AssemblyBlock>, Vec<(InstructionIR, usize)>)> {
        let cleaned_source_lines = clean_source_code(lines);

        let mut blocks = Vec::new();
        let mut current_block = AssemblyBlock {
            label: "".to_string(),
            _is_entry: false,
            content: AssemblyContent::Text(Vec::new()),
        };
        let mut current_section = "none";
        let mut global_entry_flag = false;

        let mut instruction_line_map = Vec::new();

        for (i, line) in cleaned_source_lines.iter().enumerate() {
            let original_line_number = i + 1;

            let line_content = line.trim();

            if line_content.is_empty() {
                continue;
            }

            if line_content.starts_with('.') {
                if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
                    blocks.push(current_block);
                }

                if line_content.starts_with(".data") {
                    current_section = "data";
                } else if line_content.starts_with(".text") {
                    current_section = "text";
                } else if line_content.starts_with(".bss") {
                    current_section = "bss";
                }

                if line_content.starts_with(".global") {
                    let label = line_content
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if label == "_start" {
                        global_entry_flag = true;
                    }
                }

                current_block = AssemblyBlock {
                    label: format!(".section_{}", current_section),
                    _is_entry: global_entry_flag,
                    content: AssemblyContent::Text(Vec::new()),
                };
            } else if line_content.contains(':') {
                if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
                    blocks.push(current_block);
                }

                let parts: Vec<&str> = line_content.splitn(2, ':').collect();
                let label = parts[0].trim().to_string();
                let rest_of_line = parts.get(1).map(|s| s.trim()).unwrap_or("");

                let is_entry_flag_for_new_block = global_entry_flag || label == "_start";

                current_block = AssemblyBlock {
                    label: label.clone(),
                    _is_entry: is_entry_flag_for_new_block,
                    content: match current_section {
                        "text" => AssemblyContent::Text(Vec::new()),
                        "data" => AssemblyContent::Data(Vec::new()),
                        "bss" => AssemblyContent::Bss(0),
                        _ => {
                            return Err(EmuError::InternalError(format!(
                                "Label '{}' defined outside .text or .data section.",
                                label
                            )));
                        }
                    },
                };

                // If content immediately follows the label (e.g., `vec1: .quad...`), process it here
                if !rest_of_line.is_empty() {
                    match &mut current_block.content {
                        AssemblyContent::Text(insns) => {
                            match self.parse_instruction(rest_of_line) {
                                Ok(ir) => {
                                    instruction_line_map.push((ir.clone(), original_line_number));
                                    insns.push(ir);
                                }
                                Err(EmuError::InternalError(msg)) => {
                                    return Err(EmuError::InternalError(format!(
                                        "Parsing failed on line {}: {} -> {}",
                                        original_line_number, rest_of_line, msg
                                    )));
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        AssemblyContent::Data(data_defs) => {
                            let parts: Vec<&str> = rest_of_line.split_whitespace().collect();
                            if parts.len() < 2 {
                                return Err(EmuError::InternalError(format!(
                                    "Data definition incomplete on line {}.",
                                    original_line_number
                                )));
                            }

                            match self.parse_data_definition(rest_of_line, original_line_number) {
                                Ok(data) => data_defs.push(data),
                                Err(e) => return Err(e),
                            }
                        }
                        AssemblyContent::Bss(size) => {
                            let parts: Vec<&str> = rest_of_line.split_whitespace().collect();
                            if parts.is_empty() {
                                continue;
                            }

                            if parts[0].to_lowercase() == ".skip" && parts.len() >= 2 {
                                let skip_size = parts[1].parse::<Word>().map_err(|_| {
                                    EmuError::InternalError(format!(
                                        "Invalid .skip size on line {}: {}",
                                        original_line_number, parts[1]
                                    ))
                                })?;
                                *size = skip_size;
                            } else {
                                return Err(EmuError::InternalError(format!(
                                    "Invalid .bss directive on line {}: {}",
                                    original_line_number, rest_of_line
                                )));
                            }
                        }
                    }
                }
            } else {
                // This block handles instructions/data definitions that span a new line
                match &mut current_block.content {
                    AssemblyContent::Text(insns) => match self.parse_instruction(line_content) {
                        Ok(ir) => {
                            instruction_line_map.push((ir.clone(), original_line_number));
                            insns.push(ir);
                        }
                        Err(EmuError::InternalError(msg)) => {
                            return Err(EmuError::InternalError(format!(
                                "Parsing failed on line {}: {} -> {}",
                                original_line_number, line_content, msg
                            )));
                        }
                        Err(e) => return Err(e),
                    },
                    AssemblyContent::Data(data_defs) => {
                        let parts: Vec<&str> = line_content.split_whitespace().collect();
                        if parts.len() < 2 {
                            return Err(EmuError::InternalError(format!(
                                "Data definition incomplete on line {}.",
                                original_line_number
                            )));
                        }

                        match self.parse_data_definition(line_content, original_line_number) {
                            Ok(data) => data_defs.push(data),
                            Err(e) => return Err(e),
                        }
                    }
                    AssemblyContent::Bss(size) => {
                        let parts: Vec<&str> = line_content.split_whitespace().collect();
                        if parts.is_empty() {
                            continue;
                        }

                        if parts[0].to_lowercase() == ".skip" && parts.len() >= 2 {
                            let skip_size = parts[1].parse::<u64>().map_err(|_| {
                                EmuError::InternalError(format!(
                                    "Invalid .skip size on line {}: {}",
                                    original_line_number, parts[1]
                                ))
                            })?;
                            *size = skip_size;
                        } else {
                            return Err(EmuError::InternalError(format!(
                                "Invalid .bss directive on line {}: {}",
                                original_line_number, line_content
                            )));
                        }
                    }
                }
            }
        }

        if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
            blocks.push(current_block);
        }

        Ok((blocks, instruction_line_map))
    }
}
