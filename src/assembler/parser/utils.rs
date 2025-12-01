// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::Reg;
use crate::types::{EmuError, EmuResult};

pub fn mangle_label(label: &str, filename: &str, is_global: bool) -> String {
    if is_global {
        label.to_string()
    } else {
        // Use just the file stem (no directories, no extensions)
        let safe = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
            .to_string();
        format!("{}_{}", safe, label)
    }
}

pub fn is_register(s: &str) -> bool {
    let s = s.to_lowercase();
    (s.starts_with('x') && s.len() <= 3 && s[1..].chars().all(|c| c.is_digit(10)))
        || (s.starts_with('w') && s.len() <= 3 && s[1..].chars().all(|c| c.is_digit(10)))
        || s == "sp"
        || s == "lr"
        || s == "xzr"
        || s == "wzr"
}

pub fn parse_reg(s: &str) -> EmuResult<Reg> {
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
        "X22" => Ok(Reg::X22),
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
        _ => Err(EmuError::AssemblerError {
            message: format!("Invalid register name: {}", s),
        }),
    }
}

pub fn is_numeric(s: &str) -> bool {
    let s = s.trim_start_matches('#');
    s.starts_with(|c: char| c.is_digit(10) || c == '-')
        || s.starts_with("0x")
        || s.starts_with("0b")
        || s.starts_with("0o")
}

pub fn is_label(s: &str) -> bool {
    let s = s.trim_start_matches('=');
    !s.is_empty() && s.chars().next().unwrap().is_alphabetic() && !is_register(s)
}

pub fn clean_source_code(lines: &[String]) -> Vec<String> {
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
            } else if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                in_block_comment = true;
            } else if c == '/' && chars.peek() == Some(&'/') {
                break;
            } else {
                new_line.push(c);
            }
        }
        let trimmed_line = new_line.trim();
        if !trimmed_line.is_empty() || in_block_comment {
            cleaned_lines.push(new_line.trim_end().to_string());
        }
    }
    cleaned_lines
}

// pub fn preprocess_rept(lines: &[String]) -> Vec<String> {
//     let mut out: Vec<String> = Vec::new();
//     let mut i = 0;
//     while i < lines.len() {
//         let line = lines[i].trim();
//         if line.starts_with(".rept") {
//             let tokens: Vec<&str> = line.split_whitespace().collect();
//             let count: usize = tokens
//                 .get(1)
//                 .and_then(|tok| tok.parse::<usize>().ok())
//                 .unwrap_or(1);
//             let mut rept_block = Vec::new();
//             i += 1;
//             while i < lines.len() && !lines[i].trim().starts_with(".endr") {
//                 rept_block.push(lines[i].clone());
//                 i += 1;
//             }
//             // Skip over the .endr
//             i += 1;
//             for _ in 0..count {
//                 out.extend(rept_block.clone());
//             }
//         } else {
//             out.push(lines[i].clone());
//             i += 1;
//         }
//     }
//     out
// }
