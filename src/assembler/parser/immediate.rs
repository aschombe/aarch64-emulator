// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::utils::{is_label, is_numeric, mangle_label};
use crate::assembler::asm_types::Immediate;
use crate::types::{EmuError, EmuResult};
use std::collections::HashSet;

pub fn parse_immediate(
    token: &str,
    filename: &str,
    global_labels: &HashSet<String>,
    equ_map: &std::collections::HashMap<String, i64>,
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

    // Char literal support (e.g., 'A', #'A', '\n', #'\\')
    let stripped = if clean_token.starts_with("'")
        && clean_token.ends_with("'")
        && clean_token.len() >= 3
    {
        &clean_token[1..clean_token.len() - 1]
    } else if clean_token.starts_with("#'") && clean_token.ends_with("'") && clean_token.len() >= 4
    {
        &clean_token[2..clean_token.len() - 1]
    } else {
        ""
    };
    if !stripped.is_empty() {
        // Rudimentary escape support
        let ch = if stripped.len() == 1 {
            stripped.chars().next().unwrap()
        } else if stripped.starts_with("\\") && stripped.len() == 2 {
            match &stripped[1..] {
                "n" => '\n',
                "r" => '\r',
                "t" => '\t',
                "0" => '\0',
                "'" => '\'',
                "\"" => '"',
                "\\" => '\\',
                other => {
                    return Err(EmuError::InternalError(format!(
                        "Invalid char escape: \\{} in immediate '{}'",
                        other, token
                    )));
                }
            }
        } else {
            return Err(EmuError::InternalError(format!(
                "Invalid char literal (too many chars): '{}'",
                token
            )));
        };
        return Ok(Immediate::Lit(ch as i64));
    }

    if equ_map.contains_key(clean_token) {
        Ok(Immediate::Lit(*equ_map.get(clean_token).unwrap()))
    } else if is_numeric(clean_token) {
        let val = if clean_token.starts_with("0x") || clean_token.starts_with("0X") {
            u64::from_str_radix(&clean_token[2..], 16).map(|v| v as i64)
        } else if clean_token.starts_with("0b") || clean_token.starts_with("0B") {
            u64::from_str_radix(&clean_token[2..], 2).map(|v| v as i64)
        } else if clean_token.starts_with("0o") || clean_token.starts_with("0O") {
            u64::from_str_radix(&clean_token[2..], 8).map(|v| v as i64)
        } else {
            clean_token.parse::<i64>().map(|v| v)
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
