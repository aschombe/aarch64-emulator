// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

pub mod asm_types;
pub mod data_loader;
pub mod parser;

use crate::types::{EmuError, EmuResult, Word};

#[cfg(test)]
mod tests;

use crate::cpu::InterpretedProgram;
use asm_types::{AssemblyBlock, AssemblyContent, Data, InstructionIR, SymbolTable};
use parser::AsmParser;
use std::collections::HashSet;
use std::fs;

/// The main entry point for assembling multiple source files.
/// This function coordinates parsing, symbol resolution, and flattening.
pub fn assemble_multiple_files(
    file_paths: &[String],
) -> EmuResult<(InterpretedProgram, Vec<AssemblyBlock>)> {
    let mut all_ir_blocks = Vec::new();
    let mut all_raw_lines = Vec::new();
    let mut ir_to_line_map = Vec::new(); // Collects (IR, Line Number) tuples
    let mut all_extern_labels: HashSet<String> = HashSet::new();

    // 1. Process and Merge All Files
    for path in file_paths {
        let raw_content = fs::read_to_string(path)
            .map_err(|e| EmuError::IoError(format!("Failed to open {}: {}", path, e)))?;
        let current_raw_lines: Vec<String> = raw_content.lines().map(|s| s.to_string()).collect();

        let line_offset = all_raw_lines.len(); // Current line count before merging

        // Parse the current file (returns IR blocks and temporary line map)
        let (current_ir_blocks, current_line_map, current_extern_labels) =
            AsmParser.parse_assembly_to_ir(&current_raw_lines)?;

        all_extern_labels.extend(current_extern_labels);

        // Merge lines for the final program structure
        all_raw_lines.extend(current_raw_lines);

        // Offset and merge the IR blocks and line map
        for (ir, line_num) in current_line_map {
            ir_to_line_map.push((ir, line_num + line_offset));
        }

        all_ir_blocks.extend(current_ir_blocks);
    }

    // 2. Resolve Symbols and Flatten (Single Pass over Merged IR)
    let (instructions, label_to_ip, label_is_addr, entry_ip, data_blocks, source_map) =
        flatten_and_resolve(&all_ir_blocks, &ir_to_line_map)?;

    println!(
        "Assembly complete. {} instructions generated.",
        instructions.len()
    );

    let program = InterpretedProgram {
        instructions,
        label_to_ip,
        label_is_addr,
        entry_ip,
        source_map,
        source_lines: all_raw_lines,
        extern_labels: all_extern_labels,
    };

    Ok((program, data_blocks))
}

/// Returns the size in bytes of a given data item.
// fn get_data_size(data_item: &Data) -> Word {
//     match data_item {
//         Data::Quad(_) => 8,
//         Data::Word(_) => 4,
//         Data::Byte(_) => 1,
//         _ => 0,
//     }
// }

/// Flattens blocks, calculates Instruction Pointer (IP) indices, and extracts source map data.
fn flatten_and_resolve(
    ir_blocks: &Vec<AssemblyBlock>,
    ir_to_line_map: &Vec<(InstructionIR, usize)>,
) -> EmuResult<(
    Vec<InstructionIR>,
    SymbolTable,
    std::collections::HashMap<String, bool>, // HYBRID: map label to 'is_addr'
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
    let mut label_is_addr = std::collections::HashMap::new(); // HYBRID: extra map
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
                label_is_addr.insert(block.label.clone(), false); // HYBRID
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
                label_is_addr.insert(block.label.clone(), true); // HYBRID
                let block_size: Word = items
                    .iter()
                    .map(|d| match d {
                        Data::Quad(_) => 8,
                        Data::Word(_) => 4,
                        Data::Byte(_) => 1,
                        Data::QuadArr(v) => (v.len() * 8) as Word,
                        Data::WordArr(v) => (v.len() * 4) as Word,
                        Data::IntArr(v) => (v.len() * 4) as Word,
                        Data::ByteArr(v) => v.len() as Word,
                    })
                    .sum();
                current_data_addr += block_size;
                data_blocks.push(block.clone());
            }

            // BSS SECTION: address
            AssemblyContent::Bss(size) => {
                label_to_ip.insert(block.label.clone(), current_data_addr);
                label_is_addr.insert(block.label.clone(), true); // HYBRID
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
        label_is_addr, // HYBRID
        entry_ip,
        data_blocks,
        ip_to_line_map,
    ))
}
