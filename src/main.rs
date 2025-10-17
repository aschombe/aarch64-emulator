mod assembler;
mod cpu;
mod debugger;
mod memory;
mod plugin;
mod syscall;
mod types;

use crate::assembler::{assemble_multiple_files, data_loader::load_data_into_cpu};
use crate::cpu::CpuState;
use crate::plugin::PluginManager;
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED};
use clap::Parser;
use std::cell::RefCell;
use std::rc::Rc;
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

    /// Comma-separated list of Lua plugin file paths to load.
    #[clap(long, value_delimiter = ',')]
    plugins: Vec<String>,
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

fn main() -> Result<(), EmuError> {
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
        assemble_multiple_files(&config.assembly_files)?
    } else {
        return Err(EmuError::InternalError(
            "No input file specified. Use assembly file(s).".to_string(),
        ));
    };

    let mut cpu: CpuState;
    if !config.plugins.is_empty() {
        let plugin_manager = Rc::new(RefCell::new(PluginManager::new(Vec::new())));
        cpu = CpuState::new(program, Some(Rc::clone(&plugin_manager)));

        // Load assembled data into CPU memory
        load_data_into_cpu(&mut cpu, &data_blocks)?;

        // Initialize lua if plugin manager present
        plugin_manager
            .borrow_mut()
            .init_lua(Rc::new(RefCell::new(cpu.clone())))
            .expect("Failed to initialize Lua context.");

        // Load Lua plugins
        for plugin_path in &config.plugins {
            plugin_manager
                .borrow_mut()
                .load_lua_plugin(plugin_path)
                .expect("Failed to load Lua plugin");
        }

        // Call on_plugin_load if present
        plugin_manager.borrow_mut().on_plugin_load()?;
    } else {
        cpu = CpuState::new(program, None);

        // Load assembled data into CPU memory
        load_data_into_cpu(&mut cpu, &data_blocks)?;
    };

    println!("\n--- Starting Execution ---");

    let result = if config.debug {
        debugger::run_debugger(&mut cpu)
    } else {
        cpu.run()
    };

    // Call on_plugin_unload if present
    if let Some(plugin_manager) = &cpu.plugin_manager {
        plugin_manager.borrow_mut().on_plugin_unload()?;
    }

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
