// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::data::parse_data_definition;
use super::instruction::parse_instruction;
use super::utils::{clean_source_code, mangle_label};
use crate::assembler::asm_types::{AssemblyBlock, AssemblyContent, InstructionIR};
use crate::types::{EmuError, EmuResult, Word};
use std::collections::{HashMap, HashSet};

pub struct AsmParser;

impl AsmParser {
    /// Main entry point for parsing assembly source lines into IR blocks.
    pub fn parse_assembly_to_ir(
        &self,
        lines: &Vec<String>,
        filename: &str,
        global_labels: &HashSet<String>,
        _global_label_map: &HashMap<String, (String, usize)>,
    ) -> EmuResult<(
        Vec<AssemblyBlock>,
        Vec<(InstructionIR, usize)>,
        HashSet<String>,
    )> {
        let cleaned_source_lines = clean_source_code(lines);
        let mut blocks = Vec::new();
        let mut current_block = AssemblyBlock {
            label: "".to_string(),
            _is_entry: false,
            content: AssemblyContent::Text(Vec::new()),
        };
        let mut current_section = "text";
        let mut global_entry_flag = false;
        let mut instruction_line_map = Vec::new();
        let mut extern_labels: HashSet<String> = HashSet::new();

        for (i, line) in cleaned_source_lines.iter().enumerate() {
            let original_line_number = i + 1;
            let line_content = line.trim();

            if line_content.is_empty() {
                continue;
            }
            if line_content.starts_with('.') {
                if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
                    blocks.push(current_block.clone());
                }
                if line_content.starts_with(".data") {
                    current_section = "data";
                } else if line_content.starts_with(".text") {
                    current_section = "text";
                } else if line_content.starts_with(".bss") {
                    current_section = "bss";
                }
                if line_content.starts_with(".global") || line_content.starts_with(".globl") {
                    let _label = line_content
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if _label == "_start" {
                        global_entry_flag = true;
                    }
                }
                if line_content.starts_with(".extern") {
                    let label = line_content
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if !label.is_empty() {
                        let is_global = global_labels.contains(&label);
                        extern_labels.insert(mangle_label(&label, filename, is_global));
                    }
                    continue;
                }
                current_block = AssemblyBlock {
                    label: format!(".section_{}", current_section),
                    _is_entry: global_entry_flag,
                    content: AssemblyContent::Text(Vec::new()),
                };
                continue;
            }
            if line_content.contains(':') && !line_content.contains(":lo12:") {
                if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
                    blocks.push(current_block.clone());
                }
                let parts: Vec<&str> = line_content.splitn(2, ':').collect();
                let raw_label = parts[0].trim().to_string();
                let is_global = global_labels.contains(&raw_label);
                let label = mangle_label(&raw_label, filename, is_global);
                let rest_of_line = parts.get(1).map(|s| s.trim()).unwrap_or("");
                let is_entry_flag_for_new_block = global_entry_flag || raw_label == "_start";
                current_block = AssemblyBlock {
                    label: label.clone(),
                    _is_entry: is_entry_flag_for_new_block,
                    content: match current_section {
                        "text" => AssemblyContent::Text(Vec::new()),
                        "data" => AssemblyContent::Data(Vec::new()),
                        "bss" => AssemblyContent::Bss(0),
                        _ => AssemblyContent::Text(Vec::new()),
                    },
                };
                if !rest_of_line.is_empty() {
                    match &mut current_block.content {
                        AssemblyContent::Text(insns) => {
                            match parse_instruction(rest_of_line, filename, global_labels) {
                                Ok(ir) => {
                                    instruction_line_map.push((ir.clone(), original_line_number));
                                    insns.push(ir);
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        AssemblyContent::Data(data_defs) => {
                            let parts: Vec<&str> = rest_of_line.split_whitespace().collect();
                            if parts.len() < 2 {
                                return Err(EmuError::InternalError(format!(
                                    "Data definition incomplete on line {}.",
                                    original_line_number
                                )));
                            }
                            match parse_data_definition(rest_of_line, original_line_number) {
                                Ok(data) => data_defs.push(data),
                                Err(e) => return Err(e),
                            }
                        }
                        AssemblyContent::Bss(size) => {
                            let parts: Vec<&str> = rest_of_line.split_whitespace().collect();
                            if parts.is_empty() {
                                continue;
                            }
                            if parts[0].to_lowercase() == ".skip" && parts.len() >= 2 {
                                let skip_size = parts[1].parse::<Word>().map_err(|_| {
                                    EmuError::InternalError(format!(
                                        "Invalid .skip size on line {}: {}",
                                        original_line_number, parts[1]
                                    ))
                                })?;
                                *size = skip_size;
                            } else {
                                return Err(EmuError::InternalError(format!(
                                    "Invalid .bss directive on line {}: {}",
                                    original_line_number, rest_of_line
                                )));
                            }
                        }
                    }
                }
                continue;
            }
            match &mut current_block.content {
                AssemblyContent::Text(insns) => {
                    match parse_instruction(line_content, filename, global_labels) {
                        Ok(ir) => {
                            instruction_line_map.push((ir.clone(), original_line_number));
                            insns.push(ir);
                        }
                        Err(e) => return Err(e),
                    }
                }
                AssemblyContent::Data(data_defs) => {
                    let parts: Vec<&str> = line_content.split_whitespace().collect();
                    if parts.len() < 2 {
                        return Err(EmuError::InternalError(format!(
                            "Data definition incomplete on line {}.",
                            original_line_number
                        )));
                    }
                    match parse_data_definition(line_content, original_line_number) {
                        Ok(data) => data_defs.push(data),
                        Err(e) => return Err(e),
                    }
                }
                AssemblyContent::Bss(size) => {
                    let parts: Vec<&str> = line_content.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    if parts[0].to_lowercase() == ".skip" && parts.len() >= 2 {
                        let skip_size = parts[1].parse::<u64>().map_err(|_| {
                            EmuError::InternalError(format!(
                                "Invalid .skip size on line {}: {}",
                                original_line_number, parts[1]
                            ))
                        })?;
                        *size = skip_size;
                    } else {
                        return Err(EmuError::InternalError(format!(
                            "Invalid .bss directive on line {}: {}",
                            original_line_number, line_content
                        )));
                    }
                }
            }
        }
        if !current_block.label.is_empty() && !current_block.label.starts_with('.') {
            blocks.push(current_block.clone());
        }
        Ok((blocks, instruction_line_map, extern_labels))
    }
}
