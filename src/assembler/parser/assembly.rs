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
        // let lines = preprocess_rept(lines);
        //
        // for (idx, line) in lines.iter().enumerate() {
        //     println!("Preprocessed line [{}]: '{}'", idx, line);
        // }

        let mut equ_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for line in lines.iter() {
            let trimmed = line.trim();
            // Handle .equ directives
            if trimmed.starts_with(".equ") {
                // .equ NAME, VALUE
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

        for (i, line) in cleaned_source_lines.iter().enumerate() {
            let original_line_number = i + 1;
            let line_content = line.trim();

            if line_content.is_empty() {
                continue;
            }

            if line_content.starts_with('.') {
                // Push any accumulated block
                if let Some(label) = current_label.take() {
                    match current_section {
                        "data" => {
                            blocks.push(AssemblyBlock {
                                label,
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Data(current_data.clone()),
                            });
                            current_data.clear();
                        }
                        "text" => {
                            if !current_text.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label,
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Text(current_text.clone()),
                                });
                            }
                            current_text.clear();
                        }
                        "bss" => {
                            if !current_data.is_empty() {
                                let size: usize =
                                    current_data.iter().map(|d| d.size_in_bytes()).sum();
                                blocks.push(AssemblyBlock {
                                    label,
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Bss(size as u64),
                                });
                            }
                            current_data.clear();
                        }
                        _ => {}
                    }
                }

                // Section changes
                if line_content.starts_with(".data") {
                    current_section = "data";
                } else if line_content.starts_with(".text") {
                    current_section = "text";
                } else if line_content.starts_with(".bss") {
                    current_section = "bss";
                }

                // .global/.extern handling (unchanged from your code)
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
                    continue;
                }
                // Don't treat section as a label, clear current_label
                current_label = None;
                continue;
            }

            // LABEL line
            if line_content.contains(':') && !line_content.contains(":lo12:") {
                // Push prior block
                if let Some(label) = current_label.take() {
                    match current_section {
                        "data" => {
                            blocks.push(AssemblyBlock {
                                label,
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Data(current_data.clone()),
                            });
                            current_data.clear();
                        }
                        "text" => {
                            blocks.push(AssemblyBlock {
                                label,
                                _is_entry: current_is_entry_flag,
                                content: AssemblyContent::Text(current_text.clone()),
                            });
                            current_text.clear();
                        }
                        // Add bss if needed
                        _ => {}
                    }
                }
                // Split label and immediate data
                let parts: Vec<&str> = line_content.splitn(2, ':').collect();
                let raw_label = parts[0].trim().to_string();
                let is_global = global_labels.contains(&raw_label);
                let label = mangle_label(&raw_label, filename, is_global);
                let is_entry_flag_for_new_block = global_entry_flag || raw_label == entry_label;
                current_label = Some(label);
                current_is_entry_flag = is_entry_flag_for_new_block;

                let rest_of_line = parts.get(1).map(|s| s.trim()).unwrap_or("");
                if current_section == "data" && !rest_of_line.is_empty() {
                    if let Ok(data) =
                        parse_data_definition(rest_of_line, original_line_number, &equ_map)
                    {
                        current_data.push(data);
                    }
                }
                continue;
            }

            // DATA SECTION data line
            if current_section == "data" && current_label.is_some() {
                match parse_data_definition(line_content, original_line_number, &equ_map) {
                    Ok(data_item) => current_data.push(data_item),
                    Err(e) => return Err(e),
                }
                continue;
            }

            // TEXT SECTION code line
            if current_section == "text" && current_label.is_some() {
                match parse_instruction(line_content, filename, global_labels, &equ_map) {
                    Ok(ir) => {
                        instruction_line_map.push((ir.clone(), original_line_number));
                        current_text.push(ir);
                    }
                    Err(e) => return Err(e),
                }
                continue;
            }
        }

        // Final block flush (important!)
        if let Some(label) = current_label.take() {
            match current_section {
                "data" => {
                    if !current_data.is_empty() {
                        blocks.push(AssemblyBlock {
                            label,
                            _is_entry: current_is_entry_flag,
                            content: AssemblyContent::Data(current_data.clone()),
                        });
                    }
                }
                "text" => {
                    if !current_text.is_empty() {
                        blocks.push(AssemblyBlock {
                            label,
                            _is_entry: current_is_entry_flag,
                            content: AssemblyContent::Text(current_text.clone()),
                        });
                    }
                }
                _ => {}
            }
        }
        Ok((blocks, instruction_line_map, extern_labels))
    }
}
