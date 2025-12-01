// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::immediate::parse_immediate;
use super::utils::{is_label, is_numeric, is_register, parse_reg};
use crate::assembler::asm_types::{
    Immediate, Offset, Operand, OperandWithShiftExtend, ShiftOrExtendKind,
};
use crate::types::{EmuError, EmuResult};
use std::collections::HashSet;

pub fn parse_operand(
    token: &str,
    filename: &str,
    global_labels: &HashSet<String>,
    equ_map: &std::collections::HashMap<String, i64>,
) -> EmuResult<Operand> {
    use regex::Regex;
    let token = token.trim();

    if token.is_empty() {
        return Err(EmuError::AssemblerError {
            message: "Operand is empty.".to_string(),
        });
    }

    // Pre-indexed ([reg, ...]!)
    if token.ends_with("]!") {
        let bracket_content = token.trim_end_matches("]!").trim_start_matches('[').trim();
        let parts: Vec<&str> = bracket_content.split(',').map(|s| s.trim()).collect();
        let reg = parse_reg(parts[0])?;
        if parts.len() == 2 && parts[1].starts_with('#') {
            let imm = parse_immediate(parts[1], filename, global_labels, &equ_map)?;
            return Ok(Operand::Offset(Offset::PreIndexed(reg, imm)));
        } else if parts.len() >= 2 {
            let offset_str = parts[1..].join(", ");
            let idx = parse_operand(&offset_str, filename, global_labels, equ_map)?;
            return Ok(Operand::Offset(Offset::PreIndexedReg(reg, Box::new(idx))));
        } else {
            return Err(EmuError::AssemblerError {
                message: format!("Invalid pre-indexed address format: '{}'", token),
            });
        }
    }

    // Post-indexed: handled in parse_instruction

    // Bracketed (memory operands)
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
                let idx_op = parse_operand(parts[1], filename, global_labels, equ_map)?;
                if let Operand::Imm(imm) = &idx_op {
                    return Ok(Operand::Offset(Offset::Ind3(reg, imm.clone())));
                } else if let Operand::Reg(r) = &idx_op {
                    return Ok(Operand::Offset(Offset::Ind4(reg, *r)));
                } else {
                    return Ok(Operand::Offset(Offset::Ind5(reg, Box::new(idx_op))));
                }
            }
            3 => {
                let reg = parse_reg(parts[0])?;
                let idx_token = format!("{}, {}", parts[1], parts[2]);
                let idx_op = parse_operand(&idx_token, filename, global_labels, equ_map)?;
                return Ok(Operand::Offset(Offset::Ind5(reg, Box::new(idx_op))));
            }
            _ => {
                return Err(EmuError::AssemblerError {
                    message: format!("Invalid offset format: '{}'", token),
                });
            }
        }
    }

    // Immediate + shift (e.g. "#42, lsl #16" or "label, lsl #16")
    let split_token = token.split(',').map(|s| s.trim()).collect::<Vec<_>>();
    if split_token.len() == 2 && (split_token[0].starts_with('#') || is_numeric(split_token[0])) {
        let imm_str = split_token[0];
        let mut mod_parts = split_token[1].split_whitespace();
        let modifier = mod_parts.next().unwrap_or("").to_lowercase();
        let amount: u8 = mod_parts
            .next()
            .and_then(|s| s.trim_start_matches('#').parse().ok())
            .unwrap_or(0);

        let modkind = match modifier.as_str() {
            "lsl" => Some(ShiftOrExtendKind::LSL),
            "lsr" => Some(ShiftOrExtendKind::LSR),
            "asr" => Some(ShiftOrExtendKind::ASR),
            "ror" => Some(ShiftOrExtendKind::ROR),
            _ => None,
        };
        if let Some(mk) = modkind {
            let imm = parse_immediate(imm_str, filename, global_labels, &equ_map)?;
            let value = match imm {
                Immediate::Lit(v) => v,
                Immediate::Lbl(ref s) | Immediate::Lo12Lbl(ref s) => {
                    return Err(EmuError::AssemblerError {
                        message: format!(
                            "Immediate with shift/extend not supported for label immediates: {}",
                            s
                        ),
                    });
                }
            };
            return Ok(Operand::ImmWithShift(value, mk, amount));
        }
    }

    // Register + shift/extend (including SXTW/UxTW/etc)
    if split_token.len() >= 2 && is_register(split_token[0]) {
        let base = split_token[0];
        let mod_amt = split_token[1..].join(" ");
        let mut mod_parts = mod_amt.split_whitespace();
        let modifier = mod_parts.next().unwrap_or("").to_lowercase();
        let amount: u8 = mod_parts
            .next()
            .and_then(|s| s.trim_start_matches('#').parse().ok())
            .unwrap_or(0);

        let modkind = match modifier.as_str() {
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
        if modkind.is_some() {
            let base_op = parse_operand(base, filename, global_labels, equ_map)?;
            return Ok(Operand::RegWithMod(Box::new(OperandWithShiftExtend {
                base: base_op,
                modifier: modkind,
                amount,
            })));
        }
    }

    // Basic register
    if is_register(token) {
        return Ok(Operand::Reg(parse_reg(token)?));
    }

    // Simple immediate/label
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
    if token.starts_with('\'') && token.ends_with('\'') && token.len() >= 3 {
        return Ok(Operand::Imm(parse_immediate(
            token,
            filename,
            global_labels,
            &equ_map,
        )?));
    }
    let re_label = Regex::new(r"^\.?[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    if re_label.is_match(token) {
        return Ok(Operand::Imm(parse_immediate(
            token,
            filename,
            global_labels,
            &equ_map,
        )?));
    }

    Err(EmuError::AssemblerError {
        message: format!("Unrecognized token/operand: {}", token),
    })
}
