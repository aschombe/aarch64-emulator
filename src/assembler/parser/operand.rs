// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::immediate::parse_immediate;
use super::utils::{is_label, is_numeric, is_register, parse_reg};
use crate::assembler::asm_types::{Offset, Operand, OperandWithShiftExtend, ShiftOrExtendKind};
use crate::types::{EmuError, EmuResult};
use std::collections::HashSet;

pub fn parse_operand(
    token: &str,
    filename: &str,
    global_labels: &HashSet<String>,
    equ_map: &std::collections::HashMap<String, i64>,
) -> EmuResult<Operand> {
    let token = token.trim();

    if token.is_empty() {
        return Err(EmuError::InternalError("Operand is empty.".to_string()));
    }

    let imm_mod_parts: Vec<&str> = token.split(',').map(|s| s.trim()).collect();
    if imm_mod_parts.len() >= 2 {
        let imm_token = imm_mod_parts[0];
        let mod_token = imm_mod_parts[1].to_lowercase();
        let mut amount: u8 = 0;
        if imm_mod_parts.len() == 3 {
            let amt_token = imm_mod_parts[2];
            amount = amt_token.trim_start_matches('#').parse().unwrap_or(0);
        }
        let modifier = match mod_token.as_str() {
            "lsl" => Some(ShiftOrExtendKind::LSL),
            "lsr" => Some(ShiftOrExtendKind::LSR),
            "asr" => Some(ShiftOrExtendKind::ASR),
            "ror" => Some(ShiftOrExtendKind::ROR),
            _ => None,
        };
        // For #imm plus LSL/LSR/ASR/ROR
        if imm_token.starts_with('#') && modifier.is_some() {
            // Parse immediate itself
            let base = parse_operand(imm_token, filename, global_labels, equ_map)?;
            // Compose extended/shifted immediate as a RegWithMod, or a custom ImmediateWithShift struct
            return Ok(Operand::RegWithMod(Box::new(OperandWithShiftExtend {
                base,
                modifier,
                amount,
            })));
            // OR: If you prefer, define a distinct Operand::ImmWithShift and use that instead
        }
    }

    // Register + shift/extend (x2, lsl #8) or (w5, sxtw #3), etc.
    let regmod_parts: Vec<&str> = token.split(',').map(|s| s.trim()).collect();
    if regmod_parts.len() >= 2 {
        let reg_token = regmod_parts[0];
        let mod_token = regmod_parts[1].to_lowercase();
        let mut amount: u8 = 0;
        if regmod_parts.len() == 3 {
            let amt_token = regmod_parts[2];
            amount = amt_token.trim_start_matches('#').parse().unwrap_or(0);
        }
        let modifier = match mod_token.as_str() {
            "lsl" => Some(ShiftOrExtendKind::LSL),
            "lsr" => Some(ShiftOrExtendKind::LSR),
            "asr" => Some(ShiftOrExtendKind::ASR),
            "ror" => Some(ShiftOrExtendKind::ROR),
            "uxtb" => Some(ShiftOrExtendKind::UXTB),
            "uxth" => Some(ShiftOrExtendKind::UXTH),
            "uxtw" => Some(ShiftOrExtendKind::UXTW),
            "sxtb" => Some(ShiftOrExtendKind::SXTB),
            "sxth" => Some(ShiftOrExtendKind::SXTH),
            "sxtw" => Some(ShiftOrExtendKind::SXTW),
            _ => None,
        };
        if modifier.is_some() && is_register(reg_token) {
            let base = parse_operand(reg_token, filename, global_labels, equ_map)?;
            return Ok(Operand::RegWithMod(Box::new(OperandWithShiftExtend {
                base,
                modifier,
                amount,
            })));
        }
    }

    // Bracketed addressing forms first to allow shift/extend in memory
    if token.starts_with('[') && token.ends_with(']') {
        let inner = token.trim_matches(|c| c == '[' || c == ']').trim();
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        match parts.len() {
            1 => {
                if is_register(parts[0]) {
                    return Ok(Operand::Offset(Offset::Ind2(parse_reg(parts[0])?)));
                } else {
                    return Ok(Operand::Offset(Offset::Ind1(parse_immediate(
                        parts[0],
                        filename,
                        global_labels,
                        &equ_map,
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
                        parse_immediate(parts[1], filename, global_labels, &equ_map)?,
                    )));
                }
            }
            3 => {
                // [base, index, mod/amount] -- supports shift/extend forms
                let reg = parse_reg(parts[0])?;
                let idx_mod = {
                    // Compose index+modifier (e.g., x3, lsl #2 or w6, sxtw #1)
                    let idx_token = format!(
                        "{}{}{}{}",
                        parts[1],
                        if parts[2].starts_with(|c: char| c.is_alphabetic()) {
                            ", "
                        } else {
                            " "
                        },
                        parts[2],
                        "", // extra space/sanity
                    );
                    parse_operand(&idx_token.trim(), filename, global_labels, equ_map)?
                };
                return Ok(Operand::Offset(Offset::Ind5(reg, Box::new(idx_mod))));
            }
            _ => {
                return Err(EmuError::InternalError(format!(
                    "Invalid offset format: {}",
                    token
                )));
            }
        }
    }

    // Post/pre index same as before
    if token.contains("] ,") {
        let parts: Vec<&str> = token.splitn(2, "],").map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err(EmuError::InternalError(format!(
                "Invalid post-indexed address format: {}",
                token
            )));
        }
        let base = parts[0].trim_start_matches('[').trim();
        let index = parts[1].trim();
        let reg = parse_reg(base)?;
        let imm = parse_immediate(index, filename, global_labels, &equ_map)?;
        return Ok(Operand::Offset(Offset::PostIndexed(reg, imm)));
    }
    if token.ends_with("]!") {
        let bracket_content = token.trim_end_matches("]!").trim_start_matches('[').trim();
        let parts: Vec<&str> = bracket_content.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err(EmuError::InternalError(format!(
                "Invalid pre-indexed address format: {}",
                token
            )));
        }
        let reg = parse_reg(parts[0])?;
        let imm = parse_immediate(parts[1], filename, global_labels, &equ_map)?;
        return Ok(Operand::Offset(Offset::PreIndexed(reg, imm)));
    }

    // Extended or shifted register outside brackets
    let mut split_token = token.split(',').map(|s| s.trim()).collect::<Vec<_>>();
    if split_token.len() == 2 {
        let reg_token = split_token[0];
        let rest = split_token[1];
        let mut rest_parts = rest.split_whitespace();
        let mod_str = rest_parts.next().unwrap_or("").to_lowercase();
        let amt_str = rest_parts.next();
        let amount: u8 = if let Some(amt) = amt_str {
            amt.trim_start_matches('#').parse().unwrap_or(0)
        } else {
            0
        };
        let modifier = match mod_str.as_str() {
            "lsl" => Some(ShiftOrExtendKind::LSL),
            "lsr" => Some(ShiftOrExtendKind::LSR),
            "asr" => Some(ShiftOrExtendKind::ASR),
            "ror" => Some(ShiftOrExtendKind::ROR),
            "uxtb" => Some(ShiftOrExtendKind::UXTB),
            "uxth" => Some(ShiftOrExtendKind::UXTH),
            "uxtw" => Some(ShiftOrExtendKind::UXTW),
            "sxtb" => Some(ShiftOrExtendKind::SXTB),
            "sxth" => Some(ShiftOrExtendKind::SXTH),
            "sxtw" => Some(ShiftOrExtendKind::SXTW),
            _ => None,
        };
        if modifier.is_some() {
            let base = parse_operand(reg_token, filename, global_labels, equ_map)?;
            return Ok(Operand::RegWithMod(Box::new(OperandWithShiftExtend {
                base,
                modifier,
                amount,
            })));
        }
    }
    // Space form (e.g., w2 lsl #3)
    let ws_split = token
        .split_whitespace()
        .map(|s| s.trim())
        .collect::<Vec<_>>();
    if ws_split.len() == 3 {
        let reg_token = ws_split[0];
        let mod_str = ws_split[1].to_lowercase();
        let amt_str = ws_split[2];
        let amount: u8 = amt_str.trim_start_matches('#').parse().unwrap_or(0);
        let modifier = match mod_str.as_str() {
            "lsl" => Some(ShiftOrExtendKind::LSL),
            "lsr" => Some(ShiftOrExtendKind::LSR),
            "asr" => Some(ShiftOrExtendKind::ASR),
            "ror" => Some(ShiftOrExtendKind::ROR),
            "uxtb" => Some(ShiftOrExtendKind::UXTB),
            "uxth" => Some(ShiftOrExtendKind::UXTH),
            "uxtw" => Some(ShiftOrExtendKind::UXTW),
            "sxtb" => Some(ShiftOrExtendKind::SXTB),
            "sxth" => Some(ShiftOrExtendKind::SXTH),
            "sxtw" => Some(ShiftOrExtendKind::SXTW),
            _ => None,
        };
        if modifier.is_some() {
            let base = parse_operand(reg_token, filename, global_labels, equ_map)?;
            return Ok(Operand::RegWithMod(Box::new(OperandWithShiftExtend {
                base,
                modifier,
                amount,
            })));
        }
    }

    // Char immediates, numbers, etc. (unchanged)
    if token.starts_with('\'') && token.ends_with('\'') && token.len() >= 3 {
        return Ok(Operand::Imm(parse_immediate(
            token,
            filename,
            global_labels,
            &equ_map,
        )?));
    }
    if token.starts_with('#')
        || token.starts_with('=')
        || is_numeric(token)
        || is_label(token)
        || token.to_lowercase().starts_with(":lo12:")
    {
        return Ok(Operand::Imm(parse_immediate(
            token,
            filename,
            global_labels,
            &equ_map,
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
