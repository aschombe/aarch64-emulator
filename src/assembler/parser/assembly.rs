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
                            if !current_data.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label,
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Data(current_data.clone()),
                                });
                                current_data.clear();
                            }
                        }
                        "text" => {
                            if !current_text.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label,
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Text(current_text.clone()),
                                });
                                current_text.clear();
                            }
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
            if line_content.ends_with(':') && !line_content.contains(":lo12:") {
                // Push prior block
                if let Some(label) = current_label.take() {
                    match current_section {
                        "data" => {
                            if !current_data.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label,
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Data(current_data.clone()),
                                });
                                current_data.clear();
                            }
                        }
                        "text" => {
                            if !current_text.is_empty() {
                                blocks.push(AssemblyBlock {
                                    label,
                                    _is_entry: current_is_entry_flag,
                                    content: AssemblyContent::Text(current_text.clone()),
                                });
                                current_text.clear();
                            }
                        }
                        _ => {}
                    }
                }
                // Set new label context
                let raw_label = line_content.trim_end_matches(':').trim().to_string();
                let is_global = global_labels.contains(&raw_label);
                let label = mangle_label(&raw_label, filename, is_global);
                let is_entry_flag_for_new_block = global_entry_flag || raw_label == entry_label;
                current_label = Some(label);
                current_is_entry_flag = is_entry_flag_for_new_block;
                continue;
            }

            // DATA SECTION data line
            if current_section == "data" && current_label.is_some() {
                match parse_data_definition(line_content, original_line_number) {
                    Ok(data_item) => current_data.push(data_item),
                    Err(e) => return Err(e),
                }
                continue;
            }

            // TEXT SECTION code line
            if current_section == "text" && current_label.is_some() {
                match parse_instruction(line_content, filename, global_labels) {
                    Ok(ir) => {
                        instruction_line_map.push((ir.clone(), original_line_number));
                        current_text.push(ir);
                    }
                    Err(e) => return Err(e),
                }
                continue;
            }

            // BSS or other handling as needed...
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
