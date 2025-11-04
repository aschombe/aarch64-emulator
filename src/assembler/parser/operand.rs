// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::immediate::parse_immediate;
use super::utils::{is_label, is_numeric, is_register, parse_reg};
use crate::assembler::asm_types::{Offset, Operand};
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
    if token.contains("],") {
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
