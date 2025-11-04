// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::Data;
use crate::types::{EmuError, EmuResult};

pub fn parse_string_literal(literal: &str) -> EmuResult<Vec<u8>> {
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

pub fn parse_data_definition(line_content: &str, original_line_number: usize) -> EmuResult<Data> {
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
            let values_str = parts[1..].join(" ");
            let values: Result<Vec<i64>, _> = values_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let trimmed = s.trim();
                    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                        i64::from_str_radix(&trimmed[2..], 16)
                    } else {
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
        ".string" | ".ascii" | ".asciiz" => {
            let quote_pos = line_content.find('"').ok_or_else(|| {
                EmuError::InternalError(format!(
                    "Missing opening quote on line {}",
                    original_line_number
                ))
            })?;
            let literal = &line_content[quote_pos..];
            let mut bytes = parse_string_literal(literal)?;
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
            let size = parts[1].parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!(
                    "Invalid .skip size argument on line {}: {}",
                    original_line_number, parts[1]
                ))
            })?;
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
