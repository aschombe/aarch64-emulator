// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::asm_types::Data;
use crate::types::{EmuError, EmuResult, Snippet};
use num_traits::{Num, NumCast};

fn parse_char_literal(s: &str) -> Option<i64> {
    let t = s.trim();
    let stripped = if t.starts_with("'") && t.ends_with("'") && t.len() >= 3 {
        &t[1..t.len() - 1]
    } else {
        ""
    };
    if !stripped.is_empty() {
        Some(if stripped.len() == 1 {
            stripped.chars().next().unwrap() as i64
        } else if stripped.starts_with("\\") && stripped.len() == 2 {
            match &stripped[1..] {
                "n" => b'\n' as i64,
                "r" => b'\r' as i64,
                "t" => b'\t' as i64,
                "0" => b'\0' as i64,
                "'" => b'\'' as i64,
                "\"" => b'"' as i64,
                "\\" => b'\\' as i64,
                _ => return None,
            }
        } else {
            return None;
        })
    } else {
        None
    }
}

// Parses an integer from a string
// Supports: decimal, hexadecimal (0x), binary (0b), and octal (0o) formats
fn parse_int_with_bases<T: std::str::FromStr + Num + NumCast>(s: &str) -> Option<T> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        T::from_str_radix(&s[2..], 16).ok()
    } else if s.starts_with("0b") || s.starts_with("0B") {
        T::from_str_radix(&s[2..], 2).ok()
    } else if s.starts_with("0o") || s.starts_with("0O") {
        T::from_str_radix(&s[2..], 8).ok()
    } else {
        s.parse::<T>().ok()
    }
}

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
                    return Err(EmuError::AssemblerError {
                        message: format!("Unknown escape sequence:"),
                        snippet: Some(Snippet::new(0, format!("Escape sequence: \\{}", other))),
                    });
                }
                None => {
                    // TODO: make this one better
                    return Err(EmuError::AssemblerError {
                        message: "Incomplete escape sequence at end of string.".to_string(),
                        snippet: Some(Snippet::new(0, "Incomplete escape sequence".to_string())),
                    });
                }
            };
            bytes.push(escaped_char);
        } else {
            bytes.push(c as u8);
        }
    }
    Ok(bytes)
}

pub fn parse_data_definition(
    line_content: &str,
    original_line_number: usize,
    equ_map: &std::collections::HashMap<String, i64>,
) -> EmuResult<Data> {
    // Remove anything after a comment (// or ;)
    let line_content = line_content
        .split("//")
        .next()
        .unwrap_or("")
        .split(";")
        .next()
        .unwrap_or("")
        .trim();

    let parts: Vec<&str> = line_content.split_whitespace().collect();
    let directive = if !parts.is_empty() {
        parts[0].to_lowercase()
    } else {
        "".to_string()
    };

    if directive == ".byte" && parts.len() == 2 {
        let value_str = parts[1].trim();
        let value = if let Some(val) = equ_map.get(value_str) {
            *val as u8
        } else if let Some(c) = parse_char_literal(value_str) {
            c as u8
        } else if let Some(v) = parse_int_with_bases::<u8>(value_str) {
            v
        } else {
            return Err(EmuError::AssemblerError {
                message: format!("Invalid .byte value:"),
                snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
            });
        };
        return Ok(Data::ByteArr(vec![value]));
    }

    if parts.len() < 2 {
        return Err(EmuError::AssemblerError {
            message: format!("Data directive '{}' missing arguments", directive),
            snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
        });
    }

    match directive.as_str() {
        ".byte" => {
            let values_str = parts[1..].join(" ");
            let values: Result<Vec<u8>, _> = values_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let s_trim = s.trim();
                    if let Some(val) = equ_map.get(s_trim) {
                        Ok(*val as u8)
                    } else if let Some(c) = parse_char_literal(s_trim) {
                        Ok(c as u8)
                    } else if let Some(v) = parse_int_with_bases::<u8>(s_trim) {
                        Ok(v)
                    } else {
                        Err(())
                    }
                })
                .collect();
            match values {
                Ok(v) => Ok(Data::ByteArr(v)),
                Err(_) => Err(EmuError::AssemblerError {
                    message: format!("Invalid .byte values:"),
                    snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
                }),
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
                Err(_) => Err(EmuError::AssemblerError {
                    message: format!("Invalid .single/.float values:"),
                    snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
                }),
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
                Err(_) => Err(EmuError::AssemblerError {
                    message: format!("Invalid .double/.doubleword values:"),
                    snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
                }),
            }
        }
        ".quad" | ".dword" => {
            let values_str = parts[1..].join(" ");
            let values: Result<Vec<i64>, _> = values_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let trimmed = s.trim();
                    if let Some(val) = equ_map.get(trimmed) {
                        Ok(*val)
                    } else if let Some(c) = parse_char_literal(trimmed) {
                        Ok(c as i64)
                    } else if let Some(v) = parse_int_with_bases::<i64>(trimmed) {
                        Ok(v)
                    } else if let Some(v) = parse_int_with_bases::<u64>(trimmed) {
                        Ok(v as i64) // Interpret bit pattern, may be negative.
                    } else {
                        Err(())
                    }
                })
                .collect();
            match values {
                Ok(v) => Ok(Data::QuadArr(v)),
                Err(_) => Err(EmuError::AssemblerError {
                    message: format!("Invalid .quad/.dword values:"),
                    snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
                }),
            }
        }
        ".word" | ".int" => {
            let values_str = parts[1..].join(" ");
            let values: Result<Vec<i32>, _> = values_str
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let s_trim = s.trim();
                    if let Some(val) = equ_map.get(s_trim) {
                        Ok(*val as i32)
                    } else if let Some(c) = parse_char_literal(s_trim) {
                        Ok(c as i32)
                    } else if let Some(v) = parse_int_with_bases::<i32>(s_trim) {
                        Ok(v)
                    } else if let Some(v) = parse_int_with_bases::<u32>(s_trim) {
                        Ok(v as i32)
                    } else {
                        Err(())
                    }
                })
                .collect();
            match values {
                Ok(v) => Ok(Data::IntArr(v)),
                Err(_) => Err(EmuError::AssemblerError {
                    message: format!("Invalid .word/.int values:",),
                    snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
                }),
            }
        }
        ".string" | ".ascii" | ".asciiz" | ".asciz" => {
            let quote_pos = line_content
                .find('"')
                .ok_or_else(|| EmuError::AssemblerError {
                    message: format!("Missing opening quote for string literal:"),
                    snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
                })?;
            let literal = &line_content[quote_pos..];
            let mut bytes = parse_string_literal(literal)?;
            // Add null terminator for .asciiz/.asciz/.string
            if directive == ".asciiz" || directive == ".asciz" || directive == ".string" {
                bytes.push(0);
            }
            Ok(Data::ByteArr(bytes))
        }
        ".skip" | ".space" => {
            let arg = parts[1].trim();
            let size = if let Some(val) = equ_map.get(arg) {
                *val as usize
            } else {
                parse_int_with_bases::<usize>(arg).unwrap_or(0)
            };
            Ok(Data::ByteArr(vec![0u8; size]))
        }
        ".fill" => {
            let fill_args_line = parts[1..].join(" ");
            let fill_args: Vec<&str> = fill_args_line.split(',').map(str::trim).collect();
            let repeat = fill_args
                .get(0)
                .and_then(|x| {
                    equ_map
                        .get(*x)
                        .copied()
                        .or_else(|| parse_int_with_bases::<i64>(x))
                        .map(|n| n as i64)
                })
                .unwrap_or(0) as usize;
            let size = fill_args
                .get(1)
                .and_then(|x| {
                    equ_map
                        .get(*x)
                        .copied()
                        .or_else(|| parse_int_with_bases::<i64>(x))
                        .map(|n| n as i64)
                })
                .unwrap_or(1) as usize;
            let value = fill_args
                .get(2)
                .and_then(|x| {
                    equ_map
                        .get(*x)
                        .copied()
                        .or_else(|| parse_int_with_bases::<i64>(x))
                        .map(|n| n as i64)
                })
                .unwrap_or(0) as u8;
            let total_bytes = repeat * size;
            let mut arr = Vec::with_capacity(total_bytes);
            for _ in 0..repeat {
                for _ in 0..size {
                    arr.push(value);
                }
            }
            Ok(Data::ByteArr(arr))
        }
        ".balign" => {
            let arg = parts[1].trim();
            let alignment = if let Some(val) = equ_map.get(arg) {
                *val as usize
            } else {
                parse_int_with_bases::<usize>(arg).unwrap_or(0)
            };
            Ok(Data::Align(alignment))
        }
        _ => Err(EmuError::AssemblerError {
            message: format!("Unknown data directive '{}':", directive),
            snippet: Some(Snippet::new(original_line_number, line_content.to_string())),
        }),
    }
}
