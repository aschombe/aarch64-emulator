// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use super::data::parse_data_definition;
use super::instruction::parse_instruction;
use super::utils::{clean_source_code, mangle_label};
use crate::assembler::Data;
use crate::assembler::asm_types::{AssemblyBlock, AssemblyContent, InstructionIR};
use crate::types::EmuResult;
use std::collections::{HashMap, HashSet};

pub struct AsmParser;

fn is_section_directive(line: &str) -> bool {
    matches!(
        line.trim(),
        ".data" | ".text" | ".bss" | ".section" | ".rodata"
    )
}

impl AsmParser {
    /// Main entry point for parsing assembly source lines into IR blocks.
    pub fn parse_assembly_to_ir(
        &self,
        lines: &Vec<String>,
        filename: &str,
        global_labels: &HashSet<String>,
        _global_label_map: &HashMap<String, (String, usize)>,
        entry_label: &str,
    ) -> EmuResult<(
        Vec<AssemblyBlock>,
        Vec<(InstructionIR, usize)>,
        HashSet<String>,
    )> {
        let mut equ_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for line in lines.iter() {
            let trimmed = line.trim();
            if trimmed.starts_with(".equ") {
                let tokens: Vec<&str> = trimmed.split_whitespace().collect();
                if tokens.len() >= 3 {
                    let name = tokens[1].trim_matches(',');
                    let val = tokens[2].trim_matches(',').parse::<i64>().unwrap_or(0);
                    equ_map.insert(name.to_string(), val);
                }
            }
        }
        let cleaned_source_lines = clean_source_code(&lines);
        let mut blocks = Vec::new();
        let mut instruction_line_map = Vec::new();
        let mut extern_labels: HashSet<String> = HashSet::new();
        let mut current_section = "text";
        let mut current_label: Option<String> = None;
        let mut current_data: Vec<Data> = Vec::new();
        let mut current_is_entry_flag = false;
        let mut current_text: Vec<InstructionIR> = Vec::new();
        let mut global_entry_flag = false;
        let mut i = 0;
        while i < cleaned_source_lines.len() {
            let line = &cleaned_source_lines[i];
            let original_line_number = i + 1;
            let line_content = line.trim();
            if line_content.is_empty() {
                i += 1;
                continue;
            }
            // Section change (flush block)
            if is_section_directive(line_content) {
                if let Some(label) = current_label.take() {
                    match current_section {
                        "data" => {
                            blocks.push(AssemblyBlock {
                                label: label.clone(),
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Data(current_data.clone()),
                                base_addr: 0,
                            });
                            current_data.clear();
                        }
                        "text" => {
                            if !current_text.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label: label.clone(),
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Text(current_text.clone()),
                                    base_addr: 0,
                                });
                                current_text.clear();
                            }
                        }
                        "bss" => {
                            let size: usize = current_data.iter().map(|d| d.size_in_bytes()).sum();
                            blocks.push(AssemblyBlock {
                                label: label.clone(),
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Bss(size as u64),
                                base_addr: 0,
                            });
                            current_data.clear();
                        }
                        _ => {}
                    }
                }
                current_label = None;
                if line_content.starts_with(".data") {
                    current_section = "data";
                } else if line_content.starts_with(".text") {
                    current_section = "text";
                } else if line_content.starts_with(".bss") {
                    current_section = "bss";
                }
                if line_content.starts_with(".global") || line_content.starts_with(".globl") {
                    let lbl = line_content
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if lbl == entry_label {
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
                    i += 1;
                    continue;
                }
                i += 1;
                continue;
            }
            // LABEL
            if line_content.contains(':') && !line_content.contains(":lo12:") {
                let mut _flushed_block = false;
                if let Some(label) = current_label.take() {
                    match current_section {
                        "data" => {
                            blocks.push(AssemblyBlock {
                                label: label.clone(),
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Data(current_data.clone()),
                                base_addr: 0,
                            });
                            current_data.clear();
                            _flushed_block = true;
                        }
                        "text" => {
                            if !current_text.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label: label.clone(),
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Text(current_text.clone()),
                                    base_addr: 0,
                                });
                                current_text.clear();
                                _flushed_block = true;
                            }
                        }
                        "bss" => {
                            let size: usize = current_data.iter().map(|d| d.size_in_bytes()).sum();
                            blocks.push(AssemblyBlock {
                                label: label.clone(),
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Bss(size as u64),
                                base_addr: 0,
                            });
                            current_data.clear();
                            _flushed_block = true;
                        }
                        _ => {}
                    }
                }
                let parts: Vec<&str> = line_content.splitn(2, ':').collect();
                let raw_label = parts[0].trim().to_string();
                let is_global = global_labels.contains(&raw_label);
                let label = mangle_label(&raw_label, filename, is_global);
                let is_entry_flag = global_entry_flag || raw_label == entry_label;
                current_label = Some(label);
                current_is_entry_flag = is_entry_flag;
                let rest_of_line = parts.get(1).map(|s| s.trim()).unwrap_or("");
                if (current_section == "data" || current_section == "bss")
                    && !rest_of_line.is_empty()
                {
                    let rest_directive = rest_of_line.trim_start();
                    if rest_directive.starts_with('.') {
                        if let Ok(data) =
                            parse_data_definition(rest_directive, original_line_number, &equ_map)
                        {
                            current_data.push(data);
                        }
                    }
                }
                i += 1;
                continue;
            }
            // DATA/BSS: accumulate .directives
            if (current_section == "data" || current_section == "bss") && current_label.is_some() {
                let directive_line = line_content.trim_start();
                if directive_line.starts_with('.') {
                    match parse_data_definition(directive_line, original_line_number, &equ_map) {
                        Ok(data_item) => {
                            current_data.push(data_item);
                        }
                        Err(e) => return Err(e),
                    }
                    i += 1;
                    continue;
                } else {
                }
            }
            // TEXT
            if current_section == "text" && current_label.is_some() {
                match parse_instruction(line_content, filename, global_labels, &equ_map) {
                    Ok(ir) => {
                        instruction_line_map.push((ir.clone(), original_line_number));
                        current_text.push(ir);
                    }
                    Err(e) => return Err(e),
                }
                i += 1;
                continue;
            }
            i += 1;
        }
        // Final block flush
        if let Some(label) = current_label.take() {
            match current_section {
                "data" => {
                    blocks.push(AssemblyBlock {
                        label,
                        _is_entry: current_is_entry_flag,
                        content: AssemblyContent::Data(current_data.clone()),
                        base_addr: 0,
                    });
                }
                "text" => {
                    if !current_text.is_empty() {
                        blocks.push(AssemblyBlock {
                            label,
                            _is_entry: current_is_entry_flag,
                            content: AssemblyContent::Text(current_text.clone()),
                            base_addr: 0,
                        });
                    }
                }
                "bss" => {
                    let size: usize = current_data.iter().map(|d| d.size_in_bytes()).sum();
                    blocks.push(AssemblyBlock {
                        label,
                        _is_entry: current_is_entry_flag,
                        content: AssemblyContent::Bss(size as u64),
                        base_addr: 0,
                    });
                }
                _ => {}
            }
        }

        Ok((blocks, instruction_line_map, extern_labels))
    }
}
