use std::collections::HashSet;
use std::io::{self, Write};

use crate::assembler::asm_types::InstructionIR;
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};

// Defines the last execution command used for repetition
#[derive(Debug, Clone, Copy)]
enum LastCommand {
    Step,
    Continue,
    None,
}

struct DebuggerState {
    breakpoints: HashSet<usize>, // Stores 1-based source line numbers
    last_command: LastCommand,
    running_continuously: bool,
    last_executed_insn: Option<InstructionIR>,
}

/// Visualizes the source code around the current IP, marking current instruction and breakpoints
fn visualize_source(cpu: &CpuState, dbg: &DebuggerState, radius: usize) {
    let source_lines = &cpu.program.source_lines;
    let source_map = &cpu.program.source_map;

    // Get the 1-based line number corresponding to the current IP
    let current_line_num = source_map.get(cpu.ip).cloned().unwrap_or(0);

    if current_line_num == 0 || source_lines.is_empty() {
        println!("[SOURCE] Source map unavailable or IP out of bounds.");
        return;
    }

    // Convert to 0-based index for array access
    let current_line_index = current_line_num.saturating_sub(1);

    let max_lines = source_lines.len();

    let start_line_index = current_line_index.saturating_sub(radius);
    let end_line_index = (current_line_index + radius).min(max_lines.saturating_sub(1));

    println!(
        "\n--- CODE --- [Breakpoints: {:?}]",
        dbg.breakpoints.iter().collect::<Vec<_>>()
    );

    for line_index in start_line_index..=end_line_index {
        let actual_line_num = line_index + 1; // 1-based line number

        // 1. Current Instruction Marker
        let ip_marker = if line_index == current_line_index {
            "==>"
        } else {
            "   "
        };

        // 2. Breakpoint Marker
        let break_marker = if dbg.breakpoints.contains(&(actual_line_num)) {
            "B"
        } else {
            " "
        };

        let line_content = &source_lines[line_index];

        println!(
            "{}{} {:04} | {}",
            break_marker, ip_marker, actual_line_num, line_content
        );
    }
    println!("------------");
}

/// Helper function to handle GDB-like debugger commands
fn handle_debug_command(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    command: &str,
) -> EmuResult<(bool, LastCommand)> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    // Breakpoint commands
    if let Some(cmd) = parts.first() {
        if cmd.starts_with('b') || cmd.starts_with('d') {
            let target = parts.get(1).map(|s| *s);
            if target.is_none() {
                println!("Error: Breakpoint command requires a line number or IP index.");
                return Ok((false, dbg.last_command));
            }

            let target_str = target.unwrap();
            let target_line = target_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid Line number: {}", target_str))
            })?;

            if cmd == &"break" || cmd == &"b" {
                dbg.breakpoints.insert(target_line);
                println!(
                    "Breakpoint {} set at line {}",
                    dbg.breakpoints.len(),
                    target_line
                );
            } else if cmd == &"delete" || cmd == &"d" {
                if dbg.breakpoints.remove(&target_line) {
                    println!("Breakpoint at line {} deleted.", target_line);
                } else {
                    println!("No breakpoint found at line {}.", target_line);
                }
            }
            return Ok((false, dbg.last_command));
        }
    }

    let result = match parts.as_slice() {
        // Execution control
        ["s"] | ["step"] => Ok((true, LastCommand::Step)),
        ["c"] | ["continue"] => Ok((true, LastCommand::Continue)),

        // Exit commands
        ["q"] | ["quit"] | ["exit"] => Err(EmuError::InternalError(
            "Halt command received.".to_string(),
        )),

        // State examination
        ["info", "reg"] | ["i", "r"] => {
            cpu.dump_state_full();
            Ok((false, dbg.last_command))
        }
        ["x", format_type, addr_str] => {
            let size = format_type
                .trim_start_matches('/')
                .parse::<usize>()
                .map_err(|_| {
                    EmuError::InternalError(format!("Invalid memory size format: {}", format_type))
                })?;
            let addr = Word::from_str_radix(addr_str.trim_start_matches("0x"), 16)
                .map_err(|_| EmuError::InternalError(format!("Invalid address: {}", addr_str)))?;

            match cpu.memory.read_bytes(addr, size) {
                Ok(bytes) => println!("0x{:016X}: {:?}", addr, bytes),
                Err(EmuError::MemoryAccessViolation(_)) => {
                    println!("Error: Invalid memory address or size: 0x{:X}", addr)
                }
                Err(e) => return Err(e),
            }
            Ok((false, dbg.last_command))
        }

        // Code visualization
        ["l"] | ["list"] | ["code"] => {
            visualize_source(cpu, dbg, 5);
            Ok((false, dbg.last_command))
        }

        // Help command
        ["h"] | ["help"] | ["?"] => {
            println!("Commands:");
            println!("  [ENTER]/s: Repeat last command ('s' or 'c').");
            println!("  s/step: Execute one instruction.");
            println!("  c/continue: Run continuously.");
            println!("  l/list: Print current code context.");
            println!("  b/break <Line>: Set breakpoint at Line Number.");
            println!("  d/delete <Line>: Delete breakpoint.");
            println!("  info reg: Dump all register values.");
            println!("  x /<size> <addr>: Examine memory.");
            Ok((false, dbg.last_command))
        }
        _ => {
            println!("Unknown command: {}. Type 'h' or 'help'.", command);
            Ok((false, dbg.last_command))
        }
    }?;

    Ok(result)
}

/// Runs the interpreter in gdb-like debugger mode
pub fn run_debugger(cpu: &mut CpuState) -> Result<(), EmuError> {
    let max_instructions = cpu.program.instructions.len();

    let mut dbg = DebuggerState {
        breakpoints: HashSet::new(),
        last_command: LastCommand::Step,
        running_continuously: false,
        last_executed_insn: None,
    };

    // Initial display
    visualize_source(cpu, &dbg, 5);
    cpu.dump_state_full(); // Show initial register state clearly
    println!("Debugger mode activated. Type 'h' or 'help' for commands. Default command is 's'.");

    while cpu.ip < max_instructions {
        let ir_insn = cpu
            .program
            .instructions
            .get(cpu.ip)
            .ok_or_else(|| EmuError::InternalError(format!("Invalid IP: {}", cpu.ip)))?
            .clone();

        let mut input = String::new();
        let mut should_execute = false;
        let mut command_to_execute = String::new();

        // Breakpoint check
        let current_line_num = cpu.program.source_map.get(cpu.ip).cloned().unwrap_or(0);
        if dbg.breakpoints.contains(&current_line_num) {
            println!(
                "\n[BREAKPOINT HIT] at Line {} (IP {})!",
                current_line_num, cpu.ip
            );
            dbg.running_continuously = false;
        }

        // 1. Get command or repeat last command if not running continuously
        if dbg.running_continuously && !dbg.breakpoints.contains(&current_line_num) {
            command_to_execute = match dbg.last_command {
                LastCommand::Step => "s".to_string(),
                LastCommand::Continue => "c".to_string(),
                _ => "s".to_string(),
            };
        } else {
            // Wait for input
            print!("(aarch64-dbg) ");
            io::stdout().flush().expect("Failed to flush stdout");
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let input_command = input.trim();

            // Command repetition logic
            if input_command.is_empty() {
                command_to_execute = match dbg.last_command {
                    LastCommand::Step => "s".to_string(),
                    LastCommand::Continue => "c".to_string(),
                    LastCommand::None => "s".to_string(),
                };
            } else {
                command_to_execute = input_command.to_string();
            }
        }

        // 2. Dispatch Command
        let (do_exec, history_cmd) = match handle_debug_command(cpu, &mut dbg, &command_to_execute)
        {
            Ok(result) => result,
            Err(EmuError::InternalError(msg)) if msg.contains("Halt command received.") => {
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        should_execute = do_exec;

        // 3. Update continuous state and history
        if do_exec {
            dbg.last_command = history_cmd;
            if let LastCommand::Continue = history_cmd {
                dbg.running_continuously = true;
                should_execute = true;
            } else {
                dbg.running_continuously = false;
            }
        }

        // 4. Execute and Update State
        if should_execute {
            let halt = cpu.execute_instruction_ir(&ir_insn)?;

            dbg.last_executed_insn = Some(ir_insn);

            if halt {
                println!("Program halted due to exit syscall.");
                cpu.dump_state_full();
                return Ok(());
            }

            cpu.ip += 1;

            // 5. Display resulting state
            if !dbg.running_continuously {
                visualize_source(cpu, &dbg, 5);
            }
        }

        // Final exit check
        if cpu.ip >= max_instructions {
            break;
        }
    }

    println!("Program finished.");
    cpu.dump_state_full();
    Ok(())
}
