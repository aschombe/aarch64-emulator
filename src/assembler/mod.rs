pub mod asm_types;
pub mod data_loader;
pub mod parser;

use crate::cpu::InterpretedProgram;
use crate::types::{EmuError, EmuResult, Word};
use asm_types::{AssemblyBlock, AssemblyContent, Data, InstructionIR, SymbolTable};
use parser::AsmParser;
use std::fs;

// Start address for data in simulated memory
const DATA_SECTION_START: Word = 0x200000;

/// The main entry point for assembling multiple source files.
/// This function coordinates parsing, symbol resolution, and flattening.
pub fn assemble_multiple_files(
    file_paths: &[String],
) -> EmuResult<(InterpretedProgram, Vec<AssemblyBlock>)> {
    let mut all_ir_blocks = Vec::new();
    let mut all_raw_lines = Vec::new();
    let mut ir_to_line_map = Vec::new(); // Collects (IR, Line Number) tuples

    // 1. Process and Merge All Files
    for path in file_paths {
        println!("Assembling file: {}", path);

        let raw_content = fs::read_to_string(path)
            .map_err(|e| EmuError::IoError(format!("Failed to open {}: {}", path, e)))?;
        let current_raw_lines: Vec<String> = raw_content.lines().map(|s| s.to_string()).collect();

        let line_offset = all_raw_lines.len(); // Current line count before merging

        // Parse the current file (returns IR blocks and temporary line map)
        let (current_ir_blocks, current_line_map) =
            AsmParser.parse_assembly_to_ir(&current_raw_lines)?;

        // Merge lines for the final program structure
        all_raw_lines.extend(current_raw_lines);

        // Offset and merge the IR blocks and line map
        for (ir, line_num) in current_line_map {
            ir_to_line_map.push((ir, line_num + line_offset));
        }

        all_ir_blocks.extend(current_ir_blocks);
    }

    // 2. Resolve Symbols and Flatten (Single Pass over Merged IR)
    let (instructions, label_to_ip, entry_ip, data_blocks, source_map) =
        flatten_and_resolve(&all_ir_blocks, &ir_to_line_map)?;

    println!(
        "Assembly complete. {} instructions generated.",
        instructions.len()
    );

    let program = InterpretedProgram {
        instructions,
        label_to_ip,
        entry_ip,
        source_map,
        source_lines: all_raw_lines,
    };

    Ok((program, data_blocks))
}

/// Returns the size in bytes of a given data item.
fn get_data_size(data_item: &Data) -> Word {
    match data_item {
        Data::Quad(_) => 8,
        Data::Word(_) => 4,
        Data::Byte(_) => 1,
        _ => 0,
    }
}

/// Flattens blocks, calculates Instruction Pointer (IP) indices, and extracts source map data.
fn flatten_and_resolve(
    ir_blocks: &Vec<AssemblyBlock>,
    ir_to_line_map: &Vec<(InstructionIR, usize)>,
) -> EmuResult<(
    Vec<InstructionIR>,
    SymbolTable,
    usize,
    Vec<AssemblyBlock>,
    Vec<usize>,
)> {
    let mut instructions = Vec::new();
    let mut data_blocks = Vec::new();
    let mut label_to_ip = SymbolTable::new();
    let mut ip_to_line_map = Vec::new();
    let mut current_ip = 0usize;
    let mut current_data_addr: Word = DATA_SECTION_START;
    let mut entry_ip = 0usize;

    // ---------- PASS 1: DATA blocks ----------
    for block in ir_blocks
        .iter()
        .filter(|b| matches!(&b.content, AssemblyContent::Data(_)))
    {
        if label_to_ip.contains_key(&block.label) {
            return Err(EmuError::InternalError(format!(
                "Duplicate label definition: {}",
                block.label
            )));
        }

        // Assign base address
        label_to_ip.insert(block.label.clone(), current_data_addr);

        // Compute size
        let block_size: Word = match &block.content {
            AssemblyContent::Data(data_items) => data_items
                .iter()
                .map(|d| match d {
                    Data::QuadArr(v) => (v.len() as Word) * 8,
                    Data::Quad(_) => 8,
                    Data::Word(_) => 4,
                    Data::ByteArr(v) => v.len() as Word,
                    Data::Byte(_) => 1,
                    _ => 0,
                })
                .sum(),
            _ => 0,
        };

        current_data_addr += block_size;
        data_blocks.push(block.clone());
    }

    // ---------- PASS 2: TEXT blocks ----------
    for block in ir_blocks
        .iter()
        .filter(|b| matches!(&b.content, AssemblyContent::Text(_)))
    {
        if label_to_ip.contains_key(&block.label) {
            return Err(EmuError::InternalError(format!(
                "Duplicate label definition: {}",
                block.label
            )));
        }

        label_to_ip.insert(block.label.clone(), current_ip as Word);

        if block.label == "_start" {
            entry_ip = current_ip;
        }

        if let AssemblyContent::Text(block_instructions) = &block.content {
            current_ip += block_instructions.len();
        }
    }

    // ---------- PASS 3: instruction flattening ----------
    for (ir, line_num) in ir_to_line_map {
        instructions.push(ir.clone());
        ip_to_line_map.push(*line_num);
    }

    Ok((
        instructions,
        label_to_ip,
        entry_ip,
        data_blocks,
        ip_to_line_map,
    ))
}

// fn flatten_and_resolve(
//     ir_blocks: &Vec<AssemblyBlock>,
//     ir_to_line_map: &Vec<(InstructionIR, usize)>, // This map is the result of the parser
// ) -> EmuResult<(
//     Vec<InstructionIR>,
//     SymbolTable,
//     usize,
//     Vec<AssemblyBlock>,
//     Vec<usize>,
// )> {
//     let mut instructions = Vec::new();
//     let mut data_blocks = Vec::new();
//     let mut label_to_ip = SymbolTable::new();
//     let mut ip_to_line_map = Vec::new(); // Maps IP to original source line numbers
//     let mut current_ip = 0;
//     let mut current_data_addr: Word = DATA_SECTION_START;
//     let mut entry_ip = 0;
//
//     // Pass 1, calculate IPs and data addresses
//     for block in ir_blocks.iter() {
//         if label_to_ip.contains_key(&block.label) {
//             return Err(EmuError::InternalError(format!(
//                 "Duplicate label definition: {}",
//                 block.label
//             )));
//         }
//
//         match &block.content {
//             AssemblyContent::Text(block_instructions) => {
//                 label_to_ip.insert(block.label.clone(), current_ip as Word);
//                 if block.label == "_start" {
//                     entry_ip = current_ip;
//                 }
//
//                 current_ip += block_instructions.len();
//             }
//             AssemblyContent::Data(data_items) => {
//                 label_to_ip.insert(block.label.clone(), current_data_addr);
//
//                 let block_size: Word = data_items
//                     .iter()
//                     .map(|data_item| match data_item {
//                         Data::QuadArr(arr) => arr.len() as Word * 8,
//                         _ => get_data_size(data_item),
//                     })
//                     .sum();
//
//                 current_data_addr += block_size;
//                 data_blocks.push(block.clone());
//             }
//         }
//     }
//
//     // Pass 2, flatten instructions and build source map
//     for instruction_with_line in ir_to_line_map {
//         let (instruction, line_num) = instruction_with_line;
//         instructions.push(instruction.clone());
//         ip_to_line_map.push(*line_num);
//     }
//
//     Ok((
//         instructions,
//         label_to_ip,
//         entry_ip,
//         data_blocks,
//         ip_to_line_map,
//     ))
// }
