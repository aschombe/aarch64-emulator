// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use aarch64_emulator::{
    assembler::{assemble_multiple_files, data_loader::load_data_into_cpu},
    cpu::CpuState,
    debugger,
    plugin::PluginManager,
    types::{EmuError, EmuResult, VERBOSE_ENABLED},
    vfs::VirtualFileSystem,
};

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
    /// Cannot be used together with --plugins.
    #[clap(short, long, conflicts_with = "plugins")]
    debug: bool,

    /// Comma-separated list of Lua plugin file paths to load.
    /// Cannot be used together with --debug.
    #[clap(long, value_delimiter = ',', conflicts_with = "debug")]
    plugins: Vec<String>,

    /// Folder path to give the emulated program access to the files within.
    #[clap(short, long)]
    filesystem: Option<String>,
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

/// Entry point for the AArch64 interpreter.
fn main() -> Result<(), EmuError> {
    // Parse command-line arguments
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

    // Assemble input assembly files
    let (program, data_blocks) = if !config.assembly_files.is_empty() {
        assemble_multiple_files(&config.assembly_files)?
    } else {
        return Err(EmuError::InternalError(
            "No input file specified. Use assembly file(s).".to_string(),
        ));
    };

    let vfs = match &config.filesystem {
        Some(fs_path) => Some(VirtualFileSystem::new(fs_path)),
        None => None,
    };

    // Initialize CPU state
    let cpu: Rc<RefCell<CpuState>>;
    if !config.plugins.is_empty() {
        let plugin_manager = Rc::new(RefCell::new(PluginManager::new(Vec::new())));
        cpu = Rc::new(RefCell::new(CpuState::new(
            program,
            Some(Rc::clone(&plugin_manager)),
            vfs,
        )));

        // Load assembled data into CPU memory
        load_data_into_cpu(&mut cpu.borrow_mut(), &data_blocks)?;

        // Load Lua plugins
        for plugin_path in &config.plugins {
            let plugin_name = plugin_path
                .split('/')
                .last()
                .unwrap_or("unknown_plugin.lua")
                .split('.')
                .next()
                .unwrap_or("unknown_plugin")
                .to_uppercase();

            plugin_manager
                .borrow_mut()
                .load_lua_plugin(plugin_path, Rc::clone(&cpu), plugin_name)
                .expect("Failed to load Lua plugin");
        }

        // call on_plugin_load for each plugin
        {
            let regs_snapshot = *cpu.borrow().registers.borrow();
            let mem_snapshot = cpu.borrow().memory.borrow().clone();
            plugin_manager
                .borrow_mut()
                .on_plugin_load(&regs_snapshot, mem_snapshot)?;
        }
    } else {
        cpu = Rc::new(RefCell::new(CpuState::new(program, None, vfs)));

        // Load assembled data into CPU memory
        load_data_into_cpu(&mut cpu.borrow_mut(), &data_blocks)?;
    };

    println!("\n--- Starting Execution ---");

    let result = if config.debug {
        debugger::run_debugger(&mut cpu.borrow_mut())
    } else {
        cpu.borrow_mut().run()
    };

    let exec_res = match result {
        Ok(_) => {
            println!("\nProgram finished successfully.\n");
            if !config.debug {
                cpu.borrow().dump_state_full();
            }
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("Halt command received.") {
                return Ok(());
            }
            eprintln!("\nExecution failed with error: {:?}\n", e);
            cpu.borrow().dump_state_full();
            Err(e)
        }
    };

    // Call on_plugin_unload if present
    if let Some(plugin_manager) = &cpu.borrow().plugin_manager {
        {
            println!("\n--- Unloading Plugins ---");

            let regs_snapshot = *cpu.borrow().registers.borrow();
            let mem_snapshot = cpu.borrow().memory.borrow().clone();
            plugin_manager
                .borrow_mut()
                .on_plugin_unload(&regs_snapshot, mem_snapshot)?;

            println!("--- Plugins Unloaded ---");
        }
    }

    exec_res
}
