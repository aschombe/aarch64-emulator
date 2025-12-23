// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

pub mod asm_types;
pub mod data_loader;
pub mod elf_loader;
pub mod label_pass;
pub mod optimizer;
pub mod parser;

use crate::assembler::label_pass::collect_labels;
use crate::types::{DATA_BASE, EmuError, EmuResult, Word};
use std::collections::HashMap;

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
pub fn assemble_multiple_files(
    file_paths: &[String],
    entry_label: &str,
) -> EmuResult<(InterpretedProgram, Vec<AssemblyBlock>)> {
    use std::collections::HashSet;
    use std::fs;

    // Pass 1: Collect global labels from all files
    let mut global_labels: HashSet<String> = HashSet::new();
    let mut file_sources: Vec<(String, Vec<String>)> = Vec::new();

    let equ_map: HashMap<String, i64> = HashMap::new();

    for path in file_paths {
        let raw_content = fs::read_to_string(path).map_err(|e| EmuError::FileError {
            path: Some(path.clone()),
            message: format!("Failed to open {}: {}", path, e),
        })?;
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
        let label_map = collect_labels(lines, path, &global_labels, &equ_map);
        all_label_map.extend(label_map);
    }

    let mut all_ir_blocks = Vec::new();
    let mut ir_to_line_map = Vec::new();
    let mut files = Vec::new();
    let mut all_extern_labels: HashSet<String> = HashSet::new();

    // Pass 2: Parse each file into IR blocks
    for (path, lines) in &file_sources {
        let (ir_blocks, line_map, extern_labels) = parser.parse_assembly_to_ir(
            &lines,
            path,
            &global_labels,
            &all_label_map,
            entry_label,
        )?;
        all_ir_blocks.extend(ir_blocks);
        ir_to_line_map.extend(line_map);
        all_extern_labels.extend(extern_labels);
        files.push(FileSource {
            filename: path.clone(),
            lines: lines.clone(),
        });
    }

    let (instructions, label_to_ip, label_is_addr, entry_ip, data_blocks, source_map) =
        flatten_and_resolve(&all_ir_blocks, &ir_to_line_map, entry_label)?;

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
        text_base: 0,
    };

    Ok((program, data_blocks))
}

/// Flattens blocks, calculates Instruction Pointer (IP) indices, and extracts source map data.
fn flatten_and_resolve(
    ir_blocks: &Vec<AssemblyBlock>,
    ir_to_line_map: &Vec<(InstructionIR, usize)>,
    entry_label: &str,
) -> EmuResult<(
    Vec<InstructionIR>,
    SymbolTable,
    std::collections::HashMap<String, bool>,
    usize,
    Vec<AssemblyBlock>,
    Vec<usize>,
)> {
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
    let mut entry_ip = 0usize;

    // First pass finds all data/bss labels
    for block in sorted_blocks.iter() {
        if [".data", ".text", ".bss", ".rodata"].contains(&block.label.as_str()) {
            continue;
        }
        match &block.content {
            AssemblyContent::Data(_) | AssemblyContent::Bss(_) => {
                // We'll update these addresses in the main loop,
                // but register them now so they exist
                if !label_to_ip.contains_key(&block.label) {
                    label_to_ip.insert(block.label.clone(), 0); // placeholder
                }
            }
            _ => {}
        }
    }

    // Second pass assigns addresses and instruction pointers
    let mut current_data_addr_main: Word = DATA_BASE;
    for block in sorted_blocks.iter() {
        if label_to_ip.contains_key(&block.label)
            && matches!(block.content, AssemblyContent::Text(_))
        {
            return Err(EmuError::AssemblerError {
                message: format!("Duplicate label definition: {}", block.label),
                snippet: None, // TODO: add snippet
            });
        }

        match &block.content {
            // TEXT SECTION: code label, not address
            AssemblyContent::Text(instrs) => {
                label_to_ip.insert(block.label.clone(), current_ip as Word);
                label_is_addr.insert(block.label.clone(), false);
                if block._is_entry {
                    entry_ip = current_ip;
                }
                current_ip += instrs.len();
            }

            // DATA SECTION: address
            AssemblyContent::Data(items) => {
                label_to_ip.insert(block.label.clone(), current_data_addr_main);
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
                current_data_addr_main += block_size;
                data_blocks.push(block.clone());
            }

            // BSS SECTION: address
            AssemblyContent::Bss(size) => {
                label_to_ip.insert(block.label.clone(), current_data_addr_main);
                label_is_addr.insert(block.label.clone(), true);
                current_data_addr_main += *size;
            }
        }
    }

    // Set entry point to the specified entry label
    if label_to_ip.contains_key(entry_label) {
        entry_ip = *label_to_ip.get(entry_label).unwrap() as usize;
    } else if entry_ip == 0 {
        return Err(EmuError::CustomEntryPointNotFound {
            symbol: entry_label.to_string(),
        });
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
