// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::{
    AssemblyBlock, AssemblyContent, Condition, Data, Immediate, InstructionIR, Offset, OpCode,
    Operand, Reg,
};
use crate::types::{EmuError, EmuResult, Word};
use std::collections::HashSet;

/// Mangle local labels with filename prefix, but keep global labels unmangled
fn mangle_label(label: &str, filename: &str, is_global: bool) -> String {
    if is_global {
        label.to_string()
    } else {
        let safe = filename.replace(['/', '\\', '.'], "_");
        format!("{}_{}", safe, label)
    }
}

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
        || s.starts_with("0o")
}

fn is_label(s: &str) -> bool {
    let s = s.trim_start_matches('=');
    !s.is_empty() && s.chars().next().unwrap().is_alphabetic() && !is_register(s)
}

/// Cleans a vector of lines, stripping out block comments (/* ... */) AND single-line comments (//).
fn clean_source_code(lines: &[String]) -> Vec<String> {
    let mut cleaned_lines = Vec::new();
    let mut in_block_comment = false;

    for line in lines {
        let line = line
            .replace('\u{00A0}', " ")
            .replace('\r', "")
            .replace('\u{0009}', " ");
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

        let trimmed_line = new_line.trim();
        if !trimmed_line.is_empty() || in_block_comment {
            cleaned_lines.push(new_line.trim_end().to_string());
        }
    }
    cleaned_lines
}

pub struct AsmParser;

impl AsmParser {
    fn parse_immediate(
        &self,
        token: &str,
        filename: &str,
        global_labels: &HashSet<String>,
    ) -> EmuResult<Immediate> {
        if let Some(label) = token.strip_prefix(":lo12:") {
            if is_label(label) {
                let is_global = global_labels.contains(label);
                let mangled = mangle_label(label, filename, is_global);
                return Ok(Immediate::Lo12Lbl(mangled));
            } else {
                return Err(EmuError::InternalError(format!(
                    "Invalid label after :lo12:: {}",
                    label
                )));
            }
        }
        let clean_token = token.trim_start_matches(|c| c == '#' || c == '=').trim();
        if is_numeric(clean_token) {
            let val = if clean_token.starts_with("0x") || clean_token.starts_with("0X") {
                i64::from_str_radix(&clean_token[2..], 16)
            } else if clean_token.starts_with("0b") || clean_token.starts_with("0B") {
                i64::from_str_radix(&clean_token[2..], 2)
            } else if clean_token.starts_with("0o") || clean_token.starts_with("0O") {
                i64::from_str_radix(&clean_token[2..], 8)
            } else {
                clean_token.parse::<i64>()
            };
            match val {
                Ok(v) => Ok(Immediate::Lit(v)),
                Err(_) => Err(EmuError::InternalError(format!(
                    "Invalid number format: {}",
                    clean_token
                ))),
            }
        } else if is_label(clean_token) {
            let is_global = global_labels.contains(clean_token);
            let mangled = mangle_label(clean_token, filename, is_global);
            Ok(Immediate::Lbl(mangled))
        } else {
            Err(EmuError::InternalError(format!(
                "Invalid immediate value: {}",
                token
            )))
        }
    }

    fn parse_operand(
        &self,
        token: &str,
        filename: &str,
        global_labels: &HashSet<String>,
    ) -> EmuResult<Operand> {
        let token = token.trim();
        if token.is_empty() {
            return Err(EmuError::InternalError("Operand is empty.".to_string()));
        }

        // Matches [reg, imm]! exactly (no whitespace assumptions).
        if token.ends_with('!') && token.starts_with('[') {
            // Remove trailing '!' safely, and parse inside the brackets
            let bracket_end = token.rfind(']').ok_or_else(|| {
                EmuError::InternalError(format!("Malformed pre-indexed token: {}", token))
            })?;
            let bracket_content = &token[1..bracket_end];
            let parts: Vec<&str> = bracket_content.split(',').map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(EmuError::InternalError(format!(
                    "Invalid pre-indexed address format: {}",
                    token
                )));
            }
            let reg = parse_reg(parts[0])?;
            let imm = self.parse_immediate(parts[1], filename, global_labels)?;
            return Ok(Operand::Offset(Offset::PreIndexed(reg, imm)));
        }

        // PATCH FOR POST-INDEXED ([sp], imm) is in parse_instruction

        // Existing logic for [reg] and [reg, imm]
        if token.starts_with('[') && token.ends_with(']') {
            let inner = token.trim_matches(|c| c == '[' || c == ']').trim();
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            match parts.len() {
                1 => {
                    if is_register(parts[0]) {
                        return Ok(Operand::Offset(Offset::Ind2(parse_reg(parts[0])?)));
                    } else {
                        return Ok(Operand::Offset(Offset::Ind1(self.parse_immediate(
                            parts[0],
                            filename,
                            global_labels,
                        )?)));
                    }
                }
                2 => {
                    let reg = parse_reg(parts[0])?;
                    if is_register(parts[1]) {
                        return Ok(Operand::Offset(Offset::Ind4(reg, parse_reg(parts[1])?)));
                    } else {
                        return Ok(Operand::Offset(Offset::Ind3(
                            reg,
                            self.parse_immediate(parts[1], filename, global_labels)?,
                        )));
                    }
                }
                _ => {
                    return Err(EmuError::InternalError(format!(
                        "Invalid offset format: {}",
                        token
                    )));
                }
            }
        }
        if token.starts_with('#')
            || token.starts_with('=')
            || is_numeric(token)
            || is_label(token)
            || token.to_lowercase().starts_with(":lo12:")
        {
            return Ok(Operand::Imm(self.parse_immediate(
                token,
                filename,
                global_labels,
            )?));
        }
        if is_register(token) {
            return Ok(Operand::Reg(parse_reg(token)?));
        }
        Err(EmuError::InternalError(format!(
            "Unrecognized token/operand: {}",
            token
        )))
    }

    fn parse_instruction(
        &self,
        line: &str,
        filename: &str,
        global_labels: &HashSet<String>,
    ) -> EmuResult<InstructionIR> {
        let cleaned_line = line.trim();
        let mut parts = cleaned_line.split_whitespace();
        let full_mnemonic = parts.next().unwrap_or("").to_uppercase();
        if full_mnemonic.is_empty() {
            return Err(EmuError::InternalError(
                "Empty instruction line.".to_string(),
            ));
        }
        let rest_of_line_parts: Vec<&str> = parts.collect();
        if rest_of_line_parts.is_empty() {
            let opcode = self.full_mnemonic_to_opcode(&full_mnemonic)?;
            if matches!(opcode, OpCode::NOP | OpCode::RET) {
                return Ok(InstructionIR {
                    opcode,
                    operands: Vec::new(),
                });
            } else {
                return Err(EmuError::InternalError(format!(
                    "Instruction '{}' requires operands.",
                    full_mnemonic
                )));
            }
        }
        let operands_str = rest_of_line_parts.join(" ");
        let mut token_assembly = String::new();
        let mut in_brackets = false;
        for c in operands_str.chars() {
            match c {
                '[' => {
                    if !token_assembly.ends_with('|') {
                        token_assembly.push('|');
                    }
                    in_brackets = true;
                    token_assembly.push(c);
                }
                ']' => {
                    token_assembly.push(c);
                    in_brackets = false;
                    token_assembly.push('|');
                }
                ',' if !in_brackets => {
                    token_assembly.push('|');
                }
                c if c.is_whitespace() && !in_brackets => {
                    if !token_assembly.ends_with('|') {
                        token_assembly.push('|');
                    }
                }
                _ => {
                    token_assembly.push(c);
                }
            }
        }
        let tokens: Vec<String> = token_assembly
            .split('|')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        let opcode = self.full_mnemonic_to_opcode(&full_mnemonic)?;
        let mut operands = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            // Handle post-indexed addressing: [reg], imm
            // Only if: token i == [reg] and token i+1 is numeric/imm
            if tokens[i].starts_with('[') && tokens[i].ends_with(']') && i + 1 < tokens.len() {
                let base_inner = tokens[i]
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim();
                if is_register(base_inner)
                    && (tokens[i + 1].starts_with('#') || is_numeric(&tokens[i + 1]))
                {
                    let reg = parse_reg(base_inner)?;
                    let imm = self.parse_immediate(&tokens[i + 1], filename, global_labels)?;
                    operands.push(Operand::Offset(Offset::PostIndexed(reg, imm)));
                    i += 2;
                    continue;
                }
            }
            // Otherwise, normal handling
            operands.push(self.parse_operand(&tokens[i], filename, global_labels)?);
            i += 1;
        }
        Ok(InstructionIR { opcode, operands })
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
            "CS" | "HS" => Ok(Condition::Cs),
            "CC" | "LO" => Ok(Condition::Cc),
            "MI" => Ok(Condition::Mi),
            "PL" => Ok(Condition::Pl),
            "VS" => Ok(Condition::Vs),
            "VC" => Ok(Condition::Vc),
            "HI" => Ok(Condition::Hi),
            "LS" => Ok(Condition::Ls),
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
                    Some('0') => 0,
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

    /// Parses data definition directives (.quad, .string, .ascii, .asciiz, .skip, .int, etc.)
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
            ".byte" => {
                let values_str = parts[1..].join(" ");
                let values: Result<Vec<u8>, _> = values_str
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().parse::<u8>())
                    .collect();
                match values {
                    Ok(v) => Ok(Data::ByteArr(v)),
                    Err(_) => Err(EmuError::InternalError(format!(
                        "Invalid .byte values on line {}: {}",
                        original_line_number, line_content
                    ))),
                }
            }

            ".single" | ".float" => {
                let values_str = parts[1..].join(" ");
                let values: Result<Vec<f32>, _> = values_str
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().parse::<f32>())
                    .collect();
                match values {
                    Ok(v) => Ok(Data::FloatArr(v)),
                    Err(_) => Err(EmuError::InternalError(format!(
                        "Invalid .single/.float values on line {}: {}",
                        original_line_number, line_content
                    ))),
                }
            }

            ".double" | ".doubleword" => {
                let values_str = parts[1..].join(" ");
                let values: Result<Vec<f64>, _> = values_str
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().parse::<f64>())
                    .collect();
                match values {
                    Ok(v) => Ok(Data::DoubleArr(v)),
                    Err(_) => Err(EmuError::InternalError(format!(
                        "Invalid .double/.doubleword values on line {}: {}",
                        original_line_number, line_content
                    ))),
                }
            }

            ".quad" | ".dword" => {
                // Get the string containing all values after the directive, preserving spaces between values
                let values_str = parts[1..].join(" ");

                // Split by comma, aggressively trimming each resulting value.
                let values: Result<Vec<i64>, _> = values_str
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| {
                        let trimmed = s.trim();

                        if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                            // Parse as hexadecimal (base 16) after stripping the "0x" prefix
                            i64::from_str_radix(&trimmed[2..], 16)
                        } else {
                            // Parse as decimal (base 10)
                            trimmed.parse::<i64>()
                        }
                    })
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
            ".string" | ".ascii" | ".asciiz" | ".asciz" => {
                // Find index of first quote to get exact literal including spaces
                let quote_pos = line_content.find('"').ok_or_else(|| {
                    EmuError::InternalError(format!(
                        "Missing opening quote on line {}",
                        original_line_number
                    ))
                })?;
                let literal = &line_content[quote_pos..]; // from first quote to end

                let mut bytes = self.parse_string_literal(literal)?;
                if directive == ".asciiz" || directive == ".asciz" || directive == ".string" {
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

            ".fill" => {
                // .fill <repeat>, <size>, <value>
                let fill_args_line = parts[1..].join(" ");
                let fill_args: Vec<&str> = fill_args_line.split(',').map(str::trim).collect();

                let repeat = fill_args
                    .get(0)
                    .and_then(|x| x.parse::<usize>().ok())
                    .unwrap_or(0);
                let size = fill_args
                    .get(1)
                    .and_then(|x| x.parse::<usize>().ok())
                    .unwrap_or(1);
                let value = fill_args
                    .get(2)
                    .and_then(|x| x.parse::<u8>().ok())
                    .unwrap_or(0);

                let total_bytes = repeat * size;
                // Fill the array
                let mut arr = Vec::with_capacity(total_bytes);
                for _ in 0..repeat {
                    for _ in 0..size {
                        arr.push(value);
                    }
                }
                Ok(Data::ByteArr(arr))
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

            ".balign" => {
                if parts.len() < 2 {
                    return Err(EmuError::InternalError(format!(
                        ".balign directive requires an alignment argument on line {}.",
                        original_line_number
                    )));
                }
                let alignment = parts[1].parse::<usize>().map_err(|_| {
                    EmuError::InternalError(format!(
                        "Invalid .balign alignment argument on line {}: {}",
                        original_line_number, parts[1]
                    ))
                })?;
                Ok(Data::Align(alignment))
            }

            _ => Err(EmuError::InternalError(format!(
                "Unimplemented data definition on line {}: {}",
                original_line_number, line_content
            ))),
        }
    }

    /// Converts a full mnemonic (e.g., "ADD", "B.EQ") into its OpCode enum variant.
    fn full_mnemonic_to_opcode(&self, full_mnemonic: &str) -> EmuResult<OpCode> {
        let full_mnemonic = full_mnemonic.to_uppercase();
        let base_mnemonic = full_mnemonic.split('.').next().unwrap().to_uppercase();

        match full_mnemonic.as_str() {
            "MOV" => Ok(OpCode::MOV),
            "ADD" => Ok(OpCode::ADD),
            "SUB" => Ok(OpCode::SUB),
            "MUL" => Ok(OpCode::MUL),
            "UDIV" => Ok(OpCode::UDIV),
            "SDIV" => Ok(OpCode::SDIV),
            "ADDS" => Ok(OpCode::ADDS),
            "SUBS" => Ok(OpCode::SUBS),
            "MULS" => Ok(OpCode::MULS),
            "UDIVS" => Ok(OpCode::UDIVS),
            "SDIVS" => Ok(OpCode::SDIVS),
            "AND" => Ok(OpCode::AND),
            "ANDS" => Ok(OpCode::ANDS),
            "ORR" => Ok(OpCode::ORR),
            "EOR" => Ok(OpCode::EOR),
            "NOT" => Ok(OpCode::NOT),
            "LSL" => Ok(OpCode::LSL),
            "LSR" => Ok(OpCode::LSR),
            "ASR" => Ok(OpCode::ASR),
            "ADR" => Ok(OpCode::ADR),
            "ADRP" => Ok(OpCode::ADRP),
            "LDR" => Ok(OpCode::LDR),
            "LDP" => Ok(OpCode::LDP),
            "LDRB" => Ok(OpCode::LDRB),
            "LDRH" => Ok(OpCode::LDRH),
            "LDRSB" => Ok(OpCode::LDRSB),
            "LDRSH" => Ok(OpCode::LDRSH),
            "STR" => Ok(OpCode::STR),
            "STP" => Ok(OpCode::STP),
            "STRB" => Ok(OpCode::STRB),
            "STRH" => Ok(OpCode::STRH),
            "BL" => Ok(OpCode::BL),
            "BR" => Ok(OpCode::BR),
            // Conditional branches
            "B.EQ" | "BEQ" => Ok(OpCode::B(Condition::Eq)),
            "B.NE" | "BNE" => Ok(OpCode::B(Condition::Ne)),
            "B.LT" | "BLT" => Ok(OpCode::B(Condition::Lt)),
            "B.LE" | "BLE" => Ok(OpCode::B(Condition::Le)),
            "B.GT" | "BGT" => Ok(OpCode::B(Condition::Gt)),
            "B.GE" | "BGE" => Ok(OpCode::B(Condition::Ge)),
            "B.AL" | "BAL" => Ok(OpCode::B(Condition::Al)),
            "B.CS" | "BCS" | "B.HS" | "BHS" => Ok(OpCode::B(Condition::Cs)),
            "B.CC" | "BCC" | "B.LO" | "BLO" => Ok(OpCode::B(Condition::Cc)),
            "B.MI" | "BMI" => Ok(OpCode::B(Condition::Mi)),
            "B.PL" | "BPL" => Ok(OpCode::B(Condition::Pl)),
            "B.VS" | "BVS" => Ok(OpCode::B(Condition::Vs)),
            "B.VC" | "BVC" => Ok(OpCode::B(Condition::Vc)),
            "B.HI" | "BHI" => Ok(OpCode::B(Condition::Hi)),
            "B.LS" | "BLS" => Ok(OpCode::B(Condition::Ls)),
            // Other Control
            "RET" => Ok(OpCode::RET),
            "CMP" => Ok(OpCode::CMP),
            "CBZ" => Ok(OpCode::CBZ),
            "CBNZ" => Ok(OpCode::CBNZ),
            "SVC" => Ok(OpCode::SVC),
            "NOP" => Ok(OpCode::NOP),

            _ if base_mnemonic == "B" => {
                let condition = self.parse_condition(&full_mnemonic)?;
                Ok(OpCode::B(condition))
            }
            _ => Err(EmuError::InternalError(format!(
                "Unknown mnemonic: {}",
                full_mnemonic
            ))),
        }
    }

    /// Main entry point for parsing assembly source lines into IR blocks.
    pub fn parse_assembly_to_ir(
        &self,
        lines: &Vec<String>,
        filename: &str,
        global_labels: &HashSet<String>,
    ) -> EmuResult<(
        Vec<AssemblyBlock>,
        Vec<(InstructionIR, usize)>,
        HashSet<String>,
    )> {
        let cleaned_source_lines = clean_source_code(lines);
        let mut blocks = Vec::new();
        let mut current_block = AssemblyBlock {
            label: "".to_string(),
            _is_entry: false,
            content: AssemblyContent::Text(Vec::new()),
        };
        let mut current_section = "text"; // DEFAULT STARTS AS .text, not "none"
        let mut global_entry_flag = false;
        let mut instruction_line_map = Vec::new();
        let mut extern_labels: HashSet<String> = HashSet::new();

        for (i, line) in cleaned_source_lines.iter().enumerate() {
            let original_line_number = i + 1;
            let line_content = line.trim();

            if line_content.is_empty() {
                continue;
            }
            if line_content.starts_with('.') {
                if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
                    blocks.push(current_block.clone());
                }
                if line_content.starts_with(".data") {
                    current_section = "data";
                } else if line_content.starts_with(".text") {
                    current_section = "text";
                } else if line_content.starts_with(".bss") {
                    current_section = "bss";
                }
                if line_content.starts_with(".global") || line_content.starts_with(".globl") {
                    let _label = line_content
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if _label == "_start" {
                        global_entry_flag = true;
                    }
                }
                if line_content.starts_with(".extern") {
                    let label = line_content
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if !label.is_empty() {
                        let is_global = global_labels.contains(&label);
                        extern_labels.insert(mangle_label(&label, filename, is_global));
                    }
                    continue;
                }
                current_block = AssemblyBlock {
                    label: format!(".section_{}", current_section),
                    _is_entry: global_entry_flag,
                    content: AssemblyContent::Text(Vec::new()),
                };
                continue;
            }
            if line_content.contains(':') && !line_content.contains(":lo12:") {
                if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
                    blocks.push(current_block.clone());
                }
                let parts: Vec<&str> = line_content.splitn(2, ':').collect();
                let raw_label = parts[0].trim().to_string();
                let is_global = global_labels.contains(&raw_label);
                let label = mangle_label(&raw_label, filename, is_global);
                let rest_of_line = parts.get(1).map(|s| s.trim()).unwrap_or("");
                let is_entry_flag_for_new_block = global_entry_flag || raw_label == "_start";
                current_block = AssemblyBlock {
                    label: label.clone(),
                    _is_entry: is_entry_flag_for_new_block,
                    content: match current_section {
                        "text" => AssemblyContent::Text(Vec::new()),
                        "data" => AssemblyContent::Data(Vec::new()),
                        "bss" => AssemblyContent::Bss(0),
                        _ => {
                            // Safest fallback: treat as .text
                            AssemblyContent::Text(Vec::new())
                        }
                    },
                };
                if !rest_of_line.is_empty() {
                    match &mut current_block.content {
                        AssemblyContent::Text(insns) => {
                            match self.parse_instruction(rest_of_line, filename, global_labels) {
                                Ok(ir) => {
                                    instruction_line_map.push((ir.clone(), original_line_number));
                                    insns.push(ir);
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
                continue;
            }
            match &mut current_block.content {
                AssemblyContent::Text(insns) => {
                    match self.parse_instruction(line_content, filename, global_labels) {
                        Ok(ir) => {
                            instruction_line_map.push((ir.clone(), original_line_number));
                            insns.push(ir);
                        }
                        Err(e) => return Err(e),
                    }
                }
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
        if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
            blocks.push(current_block.clone());
        }
        Ok((blocks, instruction_line_map, extern_labels))
    }
}
