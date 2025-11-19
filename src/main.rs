// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use aarch64_emulator::{
    assembler::{
        assemble_multiple_files, data_loader::load_data_into_cpu, elf_loader, optimizer::optimize,
    },
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
#[clap(author, version, about = "AArch64 Assembly Interpreter", long_about = None, disable_version_flag = true)]
struct EmuConfig {
    /// Assembly files (.s) to interpret
    #[clap(value_parser, required = true)]
    assembly_files: Vec<String>,

    /// Enables verbose execution tracing and syscall debug messages in run mode
    #[clap(short, long)]
    verbose: bool,

    /// Enables the GDB-like interactive TUI debugger. Cannot be used with plugins
    #[clap(short, long, conflicts_with = "plugins")]
    debug: bool,

    /// Comma-separated list of Lua plugin file paths to load. Cannot be used with debug
    #[clap(short, long, value_delimiter = ',', conflicts_with = "debug")]
    plugins: Vec<String>,

    /// Folder path to give the emulated program access to the files within. Mounts to VFS root ('/')
    #[clap(short, long)]
    filesystem: Option<String>,

    /// Specify a custom entry point label
    #[clap(short, long, default_value = "_start")]
    entry: String,

    /// Enable optimization passes during assembly
    #[clap(short, long, conflicts_with = "debug")]
    optimize: bool,

    /// Path to a pre-compiled ELF binary to parse and execute
    #[clap(short, long, conflicts_with_all = ["debug", "plugins", "assembly_files", "optimize", "entry"])]
    binary: String,
}

impl EmuConfig {
    /// Validates that assembly files are provided.
    fn validate(&self) -> EmuResult<()> {
        if self.assembly_files.is_empty() && self.binary.is_empty() {
            Err(EmuError::InternalError(
                "No input file(s) specified.".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = start() {
        eprintln!("\n================ ERROR ================\n");
        eprintln!("{}", e);
        eprintln!("\n=======================================\n");
        std::process::exit(1);
    }
}

/// Entry point for the AArch64 interpreter.
fn start() -> Result<(), EmuError> {
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

    if !config.binary.is_empty() {
        let (program, data_blocks) = elf_loader::parse_elf(&config.binary)?;
        let cpu = Rc::new(RefCell::new(CpuState::new(program, None, None)));
        // Load data segments into CPU memory
        // load_data_into_cpu(&mut cpu.borrow_mut(), &data_blocks)?;
        println!("\n--- Starting Execution ---");
        let result = cpu.borrow_mut().run();
        return match result {
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
                cpu.borrow().dump_state_full();
                Err(e)
            }
        };
    }

    // Assemble input assembly files
    let (mut program, data_blocks) = if !config.assembly_files.is_empty() {
        assemble_multiple_files(&config.assembly_files, &config.entry)?
    } else {
        return Err(EmuError::InternalError(
            "No input file specified. Use assembly file(s).".to_string(),
        ));
    };

    // Override entry point if custom entry label is specified
    if config.entry != "_start" {
        let custom_entry_ip = program
            .label_to_ip
            .get(&config.entry)
            .copied()
            .ok_or_else(|| {
                EmuError::InternalError(format!(
                    "Custom entry point '{}' not found in assembled program",
                    config.entry
                ))
            })?;
        program.entry_ip = custom_entry_ip as usize;
        if config.verbose {
            println!(
                "Using custom entry point: {} at IP {}",
                config.entry, custom_entry_ip
            );
        }
    }

    // Optimization pass
    if config.optimize {
        if config.verbose {
            println!("\n--- Running Optimization Passes ---");
        }
        optimize(&mut program);
        if config.verbose {
            println!("--- Optimization Passes Complete ---\n");
        }
    }

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
                .load_lua_plugin(plugin_path, plugin_name)
                .expect("Failed to load Lua plugin");
        }

        // call on_plugin_load for each plugin
        plugin_manager.borrow_mut().on_plugin_load(&cpu.borrow())?;
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
            cpu.borrow().dump_state_full();
            Err(e)
        }
    };

    // Call on_plugin_unload if present
    if let Some(plugin_manager) = &cpu.borrow().plugin_manager {
        {
            println!("\n--- Unloading Plugins ---");

            plugin_manager
                .borrow_mut()
                .on_plugin_unload(&cpu.borrow())?;

            println!("--- Plugins Unloaded ---");
        }
    }

    exec_res
}
