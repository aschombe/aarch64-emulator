mod assembler;
mod cpu;
mod debugger;
mod memory;
mod plugin;
mod syscall;
mod types;

use crate::assembler::asm_types::{AssemblyBlock, AssemblyContent, Data, SymbolTable};
use crate::assembler::assemble_multiple_files;
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED, Word};
use clap::Parser;
use std::sync::atomic::Ordering;

/// Command-line argument structure for the AArch64 interpreter.
#[derive(Parser, Debug)]
#[clap(author, version, about = "AArch64 Assembly Interpreter", long_about = None)]
struct EmuConfig {
    /// Assembly files (.s) to interpret
    #[clap(value_parser, required = true)]
    assembly_files: Vec<String>,

    /// Enables verbose execution tracing and syscall debug messages in run mode.
    #[clap(short, long)]
    verbose: bool,

    /// Enables the GDB-like interactive debugging mode (step-by-step).
    #[clap(short, long)]
    debug: bool,
    // Comma-separated list of dynamic plugins to load (.so, .dll, .dylib files).
    // #[clap(long, value_delimiter = ',')]
    // plugins: Vec<String>,
}

impl EmuConfig {
    /// Validates that exactly one input type (assembly set or binary) was provided.
    fn validate(&self) -> EmuResult<()> {
        if self.assembly_files.is_empty() {
            Err(EmuError::InternalError(
                "No input file specified. Use assembly file(s).".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

fn get_bytes_from_data_item(data_item: &Data) -> Vec<u8> {
    match data_item {
        Data::Quad(val) => val.to_le_bytes().to_vec(),
        Data::Word(val) => val.to_le_bytes().to_vec(),
        Data::Byte(val) => vec![*val],
        Data::QuadArr(arr) => arr.iter().flat_map(|&v| v.to_le_bytes().to_vec()).collect(),
        _ => Vec::new(),
    }
}

fn load_data_sections(
    cpu: &mut CpuState,
    data_blocks: &[AssemblyBlock],
    symbol_table: &SymbolTable,
) -> EmuResult<()> {
    if VERBOSE_ENABLED.load(Ordering::Relaxed) {
        println!("[DEBUG] Starting data load phase.");
    }
    for block in data_blocks {
        if let AssemblyContent::Data(data_items) = &block.content {
            let mut current_addr = *symbol_table.get(&block.label).ok_or_else(|| {
                EmuError::InternalError(format!("Data label {} address not found.", block.label))
            })?;
            if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                println!(
                    "[DEBUG] Loading data block '{}' at 0x{:X}",
                    block.label, current_addr
                );
            }
            for data_item in data_items {
                let bytes_to_write = get_bytes_from_data_item(data_item);
                if !bytes_to_write.is_empty() {
                    cpu.memory.write_bytes(current_addr, &bytes_to_write)?;
                    current_addr += bytes_to_write.len() as Word;
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), EmuError> {
    // 1. Parse Arguments using clap
    let config = EmuConfig::parse();
    config.validate()?;

    // Set global state for debug logging
    if config.verbose || config.debug {
        VERBOSE_ENABLED.store(true, Ordering::SeqCst);
        if config.verbose {
            println!("AArch64 Interpreter starting up. (VERBOSE MODE)");
        }
    } else {
        println!("AArch64 Interpreter starting up.");
    }

    let (program, data_blocks) = if !config.assembly_files.is_empty() {
        // --- Assembly Source File Execution (Assembler) ---
        assemble_multiple_files(&config.assembly_files)?
    } else {
        return Err(EmuError::InternalError(
            "No input file specified. Use assembly file(s).".to_string(),
        ));
    };

    // 3. Initialize the Interpreter
    let symbol_table = program.label_to_ip.clone();
    let mut cpu = CpuState::new(program);

    // 4. Load dynamic plugins based on CLI arguments
    // for plugin_path in &config.plugins {
    //     cpu.plugin_manager.load_lua_plugin(plugin_path)?;
    // }

    // 5. Load data into CPU memory
    load_data_sections(&mut cpu, &data_blocks, &symbol_table)?;

    println!("\n--- Starting Execution ---");

    // 6. Run based on mode (Debugger or Full Speed)
    let result = if config.debug {
        debugger::run_debugger(&mut cpu)
    } else {
        cpu.run()
    };

    match result {
        Ok(_) => {
            println!("\nProgram finished successfully.");
            if !config.debug {
                cpu.dump_state_full();
            }
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("Halt command received.") {
                return Ok(());
            }
            eprintln!("\nExecution failed with error: {:?}", e);
            cpu.dump_state_full();
            Err(e)
        }
    }
}
