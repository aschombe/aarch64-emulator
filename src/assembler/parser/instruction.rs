// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::operand::parse_operand;
use crate::assembler::asm_types::Operand;
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
            ',' if !in_brackets => token_assembly.push('|'),
            c if c.is_whitespace() && !in_brackets => {
                if !token_assembly.ends_with('|') {
                    token_assembly.push('|');
                }
            }
            _ => token_assembly.push(c),
        }
    }

    // STEP 1: initial tokens
    let tokens: Vec<String> = token_assembly
        .split('|')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|tok| tok.to_string())
        .collect();

    // STEP 2: Merge reg + shift/extend (+ amount)
    fn is_register(tok: &str) -> bool {
        tok.starts_with('x') || tok.starts_with('w')
    }
    let mut merged_tokens = Vec::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        if is_register(&token) {
            if let Some(next) = iter.peek() {
                let next_lower = next.to_ascii_lowercase();
                let is_mod = next_lower.starts_with("lsl")
                    || next_lower.starts_with("lsr")
                    || next_lower.starts_with("asr")
                    || next_lower.starts_with("ror")
                    || next_lower.starts_with("uxtb")
                    || next_lower.starts_with("uxth")
                    || next_lower.starts_with("uxtw")
                    || next_lower.starts_with("sxtb")
                    || next_lower.starts_with("sxth")
                    || next_lower.starts_with("sxtw");
                if is_mod {
                    let mod_token = iter.next().unwrap();
                    if let Some(next2) = iter.peek() {
                        if next2.starts_with('#') || next2.chars().all(|c| c.is_ascii_digit()) {
                            let amt_token = iter.next().unwrap();
                            merged_tokens.push(format!("{}, {} {}", token, mod_token, amt_token));
                        } else {
                            merged_tokens.push(format!("{}, {}", token, mod_token));
                        }
                    } else {
                        merged_tokens.push(format!("{}, {}", token, mod_token));
                    }
                    continue;
                }
            }
        }
        merged_tokens.push(token);
    }
    // STEP 3: Postindex split ([x1], w5, sxtw #2 => [x1], w5, sxtw #2)
    let mut final_tokens = Vec::new();
    for t in merged_tokens {
        if let Some(closing_idx) = t.find("]") {
            if closing_idx + 1 < t.len() {
                let (l, r) = t.split_at(closing_idx + 1);
                let rest = r.trim_start_matches(',').trim();
                if rest.is_empty() {
                    final_tokens.push(l.trim().to_string());
                } else {
                    final_tokens.push(l.trim().to_string());
                    final_tokens.push(rest.to_string());
                }
            } else {
                final_tokens.push(t);
            }
        } else {
            final_tokens.push(t);
        }
    }
    let tokens: Vec<String> = final_tokens
        .into_iter()
        .filter(|tok| tok.trim() != "!")
        .filter(|tok| !tok.trim().is_empty())
        .collect();

    // --- Conditional Select (CSEL, CSINC, etc) ---
    let condsel_mnemonics = [
        "CSEL", "CSINC", "CSINV", "CSNEG", "CSET", "CSETM", "CINC", "CINV", "CNEG",
    ];
    if condsel_mnemonics.contains(&full_mnemonic.as_str()) {
        if tokens.len() < 2 {
            return Err(EmuError::InternalError(format!(
                "Instruction '{}' requires at least two operands.",
                full_mnemonic
            )));
        }
        let condition_token = tokens.last().unwrap();
        let cond = parse_csel_like_condition(condition_token)?;
        let opcode = match full_mnemonic.as_str() {
            "CSEL" => OpCode::CSEL(cond),
            "CSINC" => OpCode::CSINC(cond),
            "CSINV" => OpCode::CSINV(cond),
            "CSNEG" => OpCode::CSNEG(cond),
            "CSET" => OpCode::CSET(cond),
            "CSETM" => OpCode::CSETM(cond),
            "CINC" => OpCode::CINC(cond),
            "CINV" => OpCode::CINV(cond),
            "CNEG" => OpCode::CNEG(cond),
            _ => unreachable!(),
        };
        let mut operands = Vec::new();
        for token in &tokens[0..tokens.len() - 1] {
            operands.push(parse_operand(token, filename, global_labels, equ_map)?);
        }
        return Ok(InstructionIR { opcode, operands });
    }
    // --- Conditional Compare (CCMP, CCMN) ---
    let condcmp_mnemonics = ["CCMP", "CCMN"];
    if condcmp_mnemonics.contains(&full_mnemonic.as_str()) {
        if tokens.len() < 4 {
            return Err(EmuError::InternalError(format!(
                "Instruction '{}' requires at least four operands.",
                full_mnemonic
            )));
        }
        let condition_token = tokens.last().unwrap();
        let cond = parse_csel_like_condition(condition_token)?;
        let ops: Vec<_> = tokens[..tokens.len() - 1]
            .iter()
            .map(|tok| parse_operand(tok, filename, global_labels, equ_map))
            .collect::<Result<_, _>>()?;
        let opcode = match full_mnemonic.as_str() {
            "CCMP" => match (&ops[1], &ops[2]) {
                (Operand::Imm(_), Operand::Imm(_)) => OpCode::CCMPImm(cond),
                (Operand::Reg(_), Operand::Imm(_)) => OpCode::CCMPReg(cond),
                _ => return Err(EmuError::InternalError("Invalid operands for CCMP".into())),
            },
            "CCMN" => match (&ops[1], &ops[2]) {
                (Operand::Imm(_), Operand::Imm(_)) => OpCode::CCMNImm(cond),
                (Operand::Reg(_), Operand::Imm(_)) => OpCode::CCMNReg(cond),
                _ => return Err(EmuError::InternalError("Invalid operands for CCMN".into())),
            },
            _ => unreachable!(),
        };
        return Ok(InstructionIR {
            opcode,
            operands: ops,
        });
    }

    // Default (classic model)
    let opcode = full_mnemonic_to_opcode(&full_mnemonic)?;
    let mut operands = Vec::new();
    for token in tokens.iter() {
        operands.push(parse_operand(token, filename, global_labels, equ_map)?);
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
        "NV" => Ok(Condition::Nv),
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

pub fn parse_csel_like_condition(condition_str: &str) -> EmuResult<Condition> {
    let condition_str = condition_str.trim().to_uppercase();
    let fake_mnemonic = format!("B.{}", condition_str);
    parse_condition(&fake_mnemonic)
}

pub fn full_mnemonic_to_opcode(full_mnemonic: &str) -> EmuResult<OpCode> {
    let full_mnemonic = full_mnemonic.to_uppercase();
    match full_mnemonic.as_str() {
        "MOV" => Ok(OpCode::MOV(MovType::Normal)),
        "MOVK" => Ok(OpCode::MOV(MovType::K)),
        "MOVZ" => Ok(OpCode::MOV(MovType::Z)),
        "MOVN" => Ok(OpCode::MOV(MovType::N)),
        "NEG" => Ok(OpCode::NEG),
        "NEGS" => Ok(OpCode::NEGS),

        "ADD" => Ok(OpCode::ADD),
        "SUB" => Ok(OpCode::SUB),
        "MUL" => Ok(OpCode::MUL),
        "UMULL" => Ok(OpCode::UMULL),
        "SMULL" => Ok(OpCode::SMULL),
        "UMULH" => Ok(OpCode::UMULH),
        "SMULH" => Ok(OpCode::SMULH),
        "MADD" => Ok(OpCode::MADD),
        "MSUB" => Ok(OpCode::MSUB),
        "MNEG" => Ok(OpCode::MNEG),
        "SMADDL" => Ok(OpCode::SMADDL),
        "SMSUBL" => Ok(OpCode::SMSUBL),
        "SMNEGL" => Ok(OpCode::SMNEGL),
        "UMADDL" => Ok(OpCode::UMADDL),
        "UMSUBL" => Ok(OpCode::UMSUBL),
        "UMNEGL" => Ok(OpCode::UMNEGL),
        "CLS" => Ok(OpCode::CLS),
        "CLZ" => Ok(OpCode::CLZ),
        "CNT" => Ok(OpCode::CNT),
        "CTZ" => Ok(OpCode::CTZ),
        "RBIT" => Ok(OpCode::RBIT),
        "REV" => Ok(OpCode::REV),
        "REV16" => Ok(OpCode::REV16),
        "REV32" => Ok(OpCode::REV32),
        "REV64" => Ok(OpCode::REV64),

        "UDIV" => Ok(OpCode::UDIV),
        "SDIV" => Ok(OpCode::SDIV),
        "ADDS" => Ok(OpCode::ADDS),
        "SUBS" => Ok(OpCode::SUBS),
        "MULS" => Ok(OpCode::MULS),
        "UDIVS" => Ok(OpCode::UDIVS),
        "SDIVS" => Ok(OpCode::SDIVS),
        "SMAX" => Ok(OpCode::SMAX),
        "SMIN" => Ok(OpCode::SMIN),
        "UMAX" => Ok(OpCode::UMAX),
        "UMIN" => Ok(OpCode::UMIN),
        "SWP" => Ok(OpCode::SWP),
        "SWPB" => Ok(OpCode::SWPB),
        "SWPH" => Ok(OpCode::SWPH),
        "SWPP" => Ok(OpCode::SWPP),
        "ABS" => Ok(OpCode::ABS),
        "ADC" => Ok(OpCode::ADC),
        "ADCS" => Ok(OpCode::ADCS),
        "SBC" => Ok(OpCode::SBC),
        "SBCS" => Ok(OpCode::SBCS),
        "NGC" => Ok(OpCode::NGC),
        "NGCS" => Ok(OpCode::NGCS),
        "AND" => Ok(OpCode::AND),
        "ANDS" => Ok(OpCode::ANDS),
        "ORR" => Ok(OpCode::ORR),
        "EOR" => Ok(OpCode::EOR),
        "NOT" => Ok(OpCode::NOT),
        "LSL" => Ok(OpCode::LSL),
        "LSR" => Ok(OpCode::LSR),
        "ASR" => Ok(OpCode::ASR),
        "ROR" => Ok(OpCode::ROR),
        "BIC" => Ok(OpCode::BIC),
        "BICS" => Ok(OpCode::BICS),
        "EON" => Ok(OpCode::EON),
        "ORN" => Ok(OpCode::ORN),
        "MVN" => Ok(OpCode::MVN),

        "SXTB" => Ok(OpCode::SXTB),
        "SXTH" => Ok(OpCode::SXTH),
        "SXTW" => Ok(OpCode::SXTW),
        "UXTB" => Ok(OpCode::UXTB),
        "UXTH" => Ok(OpCode::UXTH),
        "UXTW" => Ok(OpCode::UXTW),

        "ADR" => Ok(OpCode::ADR),
        "ADRP" => Ok(OpCode::ADRP),
        "LDR" => Ok(OpCode::LDR),
        "LDP" => Ok(OpCode::LDP),
        "LDPSW" => Ok(OpCode::LDPSW),
        "LDRB" => Ok(OpCode::LDRB),
        "LDRH" => Ok(OpCode::LDRH),
        "LDRSB" => Ok(OpCode::LDRSB),
        "LDRSH" => Ok(OpCode::LDRSH),
        "LDRSW" => Ok(OpCode::LDRSW),
        "LDUR" => Ok(OpCode::LDUR),
        "LDURB" => Ok(OpCode::LDURB),
        "LDURH" => Ok(OpCode::LDURH),
        "LDURSB" => Ok(OpCode::LDURSB),
        "LDURSH" => Ok(OpCode::LDURSH),
        "LDURSW" => Ok(OpCode::LDURSW),
        "STR" => Ok(OpCode::STR),
        "STP" => Ok(OpCode::STP),
        "STRB" => Ok(OpCode::STRB),
        "STRH" => Ok(OpCode::STRH),
        "STUR" => Ok(OpCode::STUR),
        "STURB" => Ok(OpCode::STURB),
        "STURH" => Ok(OpCode::STURH),
        "BL" => Ok(OpCode::BL),
        "BR" => Ok(OpCode::BR),
        "BLR" => Ok(OpCode::BLR),
        "B.EQ" | "BEQ" => Ok(OpCode::B(Condition::Eq)),
        "B.NE" | "BNE" => Ok(OpCode::B(Condition::Ne)),
        "B.LT" | "BLT" => Ok(OpCode::B(Condition::Lt)),
        "B.LE" | "BLE" => Ok(OpCode::B(Condition::Le)),
        "B.GT" | "BGT" => Ok(OpCode::B(Condition::Gt)),
        "B.GE" | "BGE" => Ok(OpCode::B(Condition::Ge)),
        "B.AL" | "BAL" => Ok(OpCode::B(Condition::Al)),
        "B.NV" | "BNV" => Ok(OpCode::B(Condition::Nv)),
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
        "CMN" => Ok(OpCode::CMN),
        "TST" => Ok(OpCode::TST),
        "CBZ" => Ok(OpCode::CBZ),
        "CBNZ" => Ok(OpCode::CBNZ),
        "TBZ" => Ok(OpCode::TBZ),
        "TBNZ" => Ok(OpCode::TBNZ),
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
