// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::operand::parse_operand;
use crate::assembler::asm_types::{Condition, InstructionIR, MovType, OpCode};
use crate::types::{EmuError, EmuResult};
use std::collections::HashSet;

/// Parses a line of assembly code into an InstructionIR structure.
pub fn parse_instruction(
    line: &str,
    filename: &str,
    global_labels: &HashSet<String>,
    equ_map: &std::collections::HashMap<String, i64>,
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
        let opcode = full_mnemonic_to_opcode(&full_mnemonic)?;
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
    let opcode = full_mnemonic_to_opcode(&full_mnemonic)?;
    let mut operands = Vec::new();
    for token in tokens.into_iter() {
        operands.push(parse_operand(&token, filename, global_labels, equ_map)?);
    }

    Ok(InstructionIR { opcode, operands })
}

pub fn parse_condition(full_mnemonic: &str) -> EmuResult<Condition> {
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

pub fn full_mnemonic_to_opcode(full_mnemonic: &str) -> EmuResult<OpCode> {
    let full_mnemonic = full_mnemonic.to_uppercase();
    match full_mnemonic.as_str() {
        "MOV" => Ok(OpCode::MOV(MovType::Normal)),
        "MOVK" => Ok(OpCode::MOV(MovType::K)),
        "MOVZ" => Ok(OpCode::MOV(MovType::Z)),
        "MOVN" => Ok(OpCode::MOV(MovType::N)),
        "ADD" => Ok(OpCode::ADD),
        "SUB" => Ok(OpCode::SUB),
        "MUL" => Ok(OpCode::MUL),
        "UMULL" => Ok(OpCode::UMULL),
        "SMULL" => Ok(OpCode::SMULL),
        "UMULH" => Ok(OpCode::UMULH),
        "SMULH" => Ok(OpCode::SMULH),
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
        "RET" => Ok(OpCode::RET),
        "CMP" => Ok(OpCode::CMP),
        "CBZ" => Ok(OpCode::CBZ),
        "CBNZ" => Ok(OpCode::CBNZ),
        "SVC" => Ok(OpCode::SVC),
        "NOP" => Ok(OpCode::NOP),
        _ => {
            let base_mnemonic = full_mnemonic.split('.').next().unwrap().to_uppercase();
            if base_mnemonic == "B" {
                let condition = parse_condition(&full_mnemonic)?;
                Ok(OpCode::B(condition))
            } else {
                Err(EmuError::InternalError(format!(
                    "Unknown mnemonic: {}",
                    full_mnemonic
                )))
            }
        }
    }
}
