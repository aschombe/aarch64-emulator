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
        // Always mangle, just like block creation
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
