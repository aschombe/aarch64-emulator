// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

pub mod asm_types;
pub mod data_loader;
pub mod label_pass;
pub mod parser;

use crate::assembler::label_pass::collect_labels;
use crate::types::{EmuError, EmuResult, Word};

#[cfg(test)]
mod tests;

use crate::cpu::InterpretedProgram;
use asm_types::{AssemblyBlock, AssemblyContent, Data, InstructionIR, SymbolTable};
use parser::AsmParser;

// Holds lines per file, for TEXT section only
#[derive(Clone)]
pub struct FileSource {
    pub filename: String,
    pub lines: Vec<String>,
}

// IP maps to file/line for source rendering
#[derive(Clone)]
pub struct SourceMapEntry {
    pub ip: usize,
    pub filename: String,
    pub line: usize,
}

/// The main entry point for assembling multiple source files.
/// This function coordinates parsing, symbol resolution, and flattening.
pub fn assemble_multiple_files(
    file_paths: &[String],
) -> EmuResult<(InterpretedProgram, Vec<AssemblyBlock>)> {
    use std::collections::HashSet;
    use std::fs;

    // ============= Pass 1: Gather all global labels and collect accurate label map using the parser ================
    let mut global_labels: HashSet<String> = HashSet::new();
    let mut file_sources: Vec<(String, Vec<String>)> = Vec::new();
    for path in file_paths {
        let raw_content = fs::read_to_string(path)
            .map_err(|e| EmuError::IoError(format!("Failed to open {}: {}", path, e)))?;
        let lines: Vec<String> = raw_content.lines().map(|s| s.to_string()).collect();
        for line in &lines {
            let line_content = line.trim();
            if line_content.starts_with(".global") || line_content.starts_with(".globl") {
                if let Some(label) = line_content.split_whitespace().nth(1) {
                    global_labels.insert(label.to_string());
                }
            }
        }
        file_sources.push((path.clone(), lines));
    }

    let parser = AsmParser;
    // Global label map for all files/sections/labels (with accurate offsets)
    let mut all_label_map = std::collections::HashMap::new();
    for (path, lines) in &file_sources {
        let label_map = collect_labels(lines, path, &global_labels);
        all_label_map.extend(label_map);
    }

    let mut all_ir_blocks = Vec::new();
    let mut ir_to_line_map = Vec::new();
    let mut files = Vec::new();
    let mut all_extern_labels: HashSet<String> = HashSet::new();

    // ============= Pass 2: Parse IR w/ access to all globals ================
    for (path, lines) in &file_sources {
        let (ir_blocks, line_map, extern_labels) =
            parser.parse_assembly_to_ir(&lines, path, &global_labels, &all_label_map)?;
        all_ir_blocks.extend(ir_blocks);
        ir_to_line_map.extend(line_map);
        all_extern_labels.extend(extern_labels);
        files.push(FileSource {
            filename: path.clone(),
            lines: lines.clone(),
        });
    }

    let (instructions, label_to_ip, label_is_addr, entry_ip, data_blocks, source_map) =
        flatten_and_resolve(&all_ir_blocks, &ir_to_line_map)?;

    let program = InterpretedProgram {
        instructions,
        label_to_ip,
        label_is_addr,
        entry_ip,
        files,
        ip_map: source_map
            .iter()
            .enumerate()
            .map(|(ip, line_num)| SourceMapEntry {
                ip,
                filename: String::new(),
                line: *line_num,
            })
            .collect(),
        extern_labels: all_extern_labels,
    };

    Ok((program, data_blocks))
}

/// Flattens blocks, calculates Instruction Pointer (IP) indices, and extracts source map data.
fn flatten_and_resolve(
    ir_blocks: &Vec<AssemblyBlock>,
    ir_to_line_map: &Vec<(InstructionIR, usize)>,
) -> EmuResult<(
    Vec<InstructionIR>,
    SymbolTable,
    std::collections::HashMap<String, bool>,
    usize,
    Vec<AssemblyBlock>,
    Vec<usize>,
)> {
    use crate::types::DATA_BASE;

    let mut sorted_blocks = ir_blocks.clone();
    sorted_blocks.sort_by_key(|b| match b.content {
        AssemblyContent::Text(_) => 0,
        AssemblyContent::Data(_) => 1,
        AssemblyContent::Bss(_) => 2,
    });

    let mut instructions = Vec::new();
    let mut data_blocks = Vec::new();
    let mut label_to_ip = SymbolTable::new();
    let mut label_is_addr = std::collections::HashMap::new();
    let mut ip_to_line_map = Vec::new();

    let mut current_ip = 0usize;
    let mut current_data_addr: Word = DATA_BASE;
    let mut entry_ip = 0usize;

    for block in sorted_blocks.iter() {
        if label_to_ip.contains_key(&block.label) {
            return Err(EmuError::InternalError(format!(
                "Duplicate label definition: {}",
                block.label
            )));
        }

        match &block.content {
            // TEXT SECTION: code label, not address
            AssemblyContent::Text(instrs) => {
                label_to_ip.insert(block.label.clone(), current_ip as Word);
                label_is_addr.insert(block.label.clone(), false);
                if block.label == "_start" {
                    entry_ip = current_ip;
                } else if block._is_entry && entry_ip == 0 {
                    entry_ip = current_ip;
                }
                current_ip += instrs.len();
            }

            // DATA SECTION: address
            AssemblyContent::Data(items) => {
                label_to_ip.insert(block.label.clone(), current_data_addr);
                label_is_addr.insert(block.label.clone(), true);
                let block_size: Word = items
                    .iter()
                    .map(|d| match d {
                        Data::Align(usz) => *usz as Word,
                        Data::Quad(_) => 8,
                        Data::Word(_) => 4,
                        Data::Byte(_) => 1,
                        Data::QuadArr(v) => (v.len() * 8) as Word,
                        Data::WordArr(v) => (v.len() * 4) as Word,
                        Data::IntArr(v) => (v.len() * 4) as Word,
                        Data::ByteArr(v) => v.len() as Word,
                        Data::DoubleArr(v) => (v.len() * 8) as Word,
                        Data::FloatArr(v) => (v.len() * 4) as Word,
                    })
                    .sum();
                current_data_addr += block_size;
                data_blocks.push(block.clone());
            }

            // BSS SECTION: address
            AssemblyContent::Bss(size) => {
                label_to_ip.insert(block.label.clone(), current_data_addr);
                label_is_addr.insert(block.label.clone(), true);
                current_data_addr += *size;
            }
        }
    }

    // Set entry point
    if label_to_ip.contains_key("_start") {
        entry_ip = *label_to_ip.get("_start").unwrap() as usize;
    } else if entry_ip == 0 && !label_to_ip.is_empty() {
        if let Some(first_text) = sorted_blocks
            .iter()
            .find(|b| matches!(b.content, AssemblyContent::Text(_)))
        {
            entry_ip = *label_to_ip.get(&first_text.label).unwrap_or(&0) as usize;
        }
    }

    for (ir, line_num) in ir_to_line_map {
        instructions.push(ir.clone());
        ip_to_line_map.push(*line_num);
    }

    Ok((
        instructions,
        label_to_ip,
        label_is_addr,
        entry_ip,
        data_blocks,
        ip_to_line_map,
    ))
}
