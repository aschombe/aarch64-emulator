use std::collections::HashSet;
use std::io::{self, Write};

use crate::assembler::asm_types::InstructionIR;
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};

#[derive(Debug, Clone, Copy)]
enum LastCommand {
    Step,
    Continue,
    _None,
}

struct DebuggerState {
    breakpoints: HashSet<usize>, // 1-based source line numbers
    last_command: LastCommand,
    running_continuously: bool,
    last_executed_insn: Option<InstructionIR>,
}

/// Visualize source lines around current IP. If IP is out-of-bounds we
/// fall back to showing context around the last executed instruction (if any),
/// otherwise start at top of file.
fn visualize_source(cpu: &CpuState, dbg: &DebuggerState, radius: usize) {
    let source_lines = &cpu.program.source_lines;
    let source_map = &cpu.program.source_map;

    let instr_idx_to_show = if cpu.ip < cpu.program.instructions.len() {
        cpu.ip
    } else if let Some(last) = &dbg.last_executed_insn {
        cpu.program
            .instructions
            .iter()
            .position(|i| i == last)
            .unwrap_or_else(|| cpu.program.instructions.len().saturating_sub(1))
    } else {
        0usize
    };

    // Map instruction index -> 1-based source line (if available)
    let current_line_num = source_map.get(instr_idx_to_show).cloned().unwrap_or(0);

    if current_line_num == 0 || source_lines.is_empty() {
        println!("[SOURCE] Source map unavailable or IP out of bounds.");
        return;
    }

    let current_line_index = current_line_num.saturating_sub(1);
    let max_lines = source_lines.len();
    let start_line_index = current_line_index.saturating_sub(radius);
    let end_line_index = (current_line_index + radius).min(max_lines.saturating_sub(1));

    let mut bps: Vec<_> = dbg.breakpoints.iter().cloned().collect();
    bps.sort_unstable();

    println!("\n--- CODE --- [Breakpoints: {:?}]", bps);

    for line_index in start_line_index..=end_line_index {
        let actual_line_num = line_index + 1; // 1-based

        let ip_marker = if line_index == current_line_index {
            "==>"
        } else {
            "   "
        };
        let break_marker = if dbg.breakpoints.contains(&actual_line_num) {
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

fn handle_debug_command(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    command: &str,
) -> EmuResult<(bool, LastCommand)> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    // Breakpoint and delete handling: "b <line>" / "break <line>", "d <line>" / "delete <line>"
    if let Some(cmd) = parts.first() {
        if cmd.starts_with('b') || cmd == &"break" || cmd.starts_with('d') || cmd == &"delete" {
            let target = parts.get(1).map(|s| *s);
            if target.is_none() {
                println!("Error: breakpoint command requires a line number.");
                return Ok((false, dbg.last_command));
            }
            let target_str = target.unwrap();

            let target_line = target_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid line number: {}", target_str))
            })?;

            if cmd.starts_with('b') || cmd == &"break" {
                dbg.breakpoints.insert(target_line);
                println!("Breakpoint set at source line {}", target_line);
            } else {
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
        ["s"] | ["step"] => Ok((true, LastCommand::Step)),
        ["c"] | ["continue"] => Ok((true, LastCommand::Continue)),
        ["q"] | ["quit"] | ["exit"] => Err(EmuError::InternalError(
            "Halt command received.".to_string(),
        )),

        // Register dump
        ["info", "reg"] | ["i", "r"] => {
            cpu.dump_state_full();
            Ok((false, dbg.last_command))
        }

        // examine memory: x /<size> <addr>
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

        // list / code
        ["l"] | ["list"] | ["code"] => {
            visualize_source(cpu, dbg, 5);
            Ok((false, dbg.last_command))
        }

        // help
        ["h"] | ["help"] | ["?"] => {
            println!("Commands:");
            println!("  [ENTER]/s: Repeat last command ('s' or 'c').");
            println!("  s/step: Execute one instruction.");
            println!("  c/continue: Run continuously.");
            println!("  l/list: Print current code context.");
            println!("  b/break <Line>: Set breakpoint at Line Number (source line).");
            println!("  d/delete <Line>: Delete breakpoint.");
            println!("  info reg: Dump all register values.");
            println!("  x /<size> <addr>: Examine memory.");
            Ok((false, dbg.last_command))
        }

        other => {
            let joined = other.join(" ");
            println!("Unknown command: {}. Type 'h' or 'help'.", joined);
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
    cpu.dump_state_full();
    println!("Debugger mode activated. Type 'h' or 'help' for commands. Default command is 's'.");

    while cpu.ip <= max_instructions {
        // Protect against invalid ip when program is done
        if cpu.ip >= max_instructions {
            println!("Program finished (IP at or past end).");
            cpu.dump_state_full();
            return Ok(());
        }

        // Compute current source line (safe): if mapping missing, use 0
        let current_line_num = cpu.program.source_map.get(cpu.ip).cloned().unwrap_or(0);

        // Breakpoint check uses source-line based breakpoints
        if current_line_num != 0 && dbg.breakpoints.contains(&current_line_num) {
            println!(
                "\n[BREAKPOINT HIT] at Line {} (IP {})!",
                current_line_num, cpu.ip
            );
            dbg.running_continuously = false;
        }

        // Decide command to run (respect continuous mode and breakpoints)
        let command_to_execute =
            if dbg.running_continuously && !dbg.breakpoints.contains(&current_line_num) {
                match dbg.last_command {
                    LastCommand::Step => "s".to_string(),
                    LastCommand::Continue => "c".to_string(),
                    LastCommand::_None => "s".to_string(),
                }
            } else {
                // Prompt user
                let mut input = String::new();
                print!("(aarch64-dbg) ");
                io::stdout().flush().expect("Failed to flush stdout");
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                let input_command = input.trim();
                if input_command.is_empty() {
                    match dbg.last_command {
                        LastCommand::Step => "s".to_string(),
                        LastCommand::Continue => "c".to_string(),
                        LastCommand::_None => "s".to_string(),
                    }
                } else {
                    input_command.to_string()
                }
            };
        // Dispatch command
        let (do_exec, history_cmd) = match handle_debug_command(cpu, &mut dbg, &command_to_execute)
        {
            Ok(r) => r,
            Err(EmuError::InternalError(msg)) if msg.contains("Halt command received.") => {
                println!("Debugger quitting.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error handling command: {:?}", e);
                continue;
            }
        };

        // Update history and continuous state only when we accepted an execution command
        if do_exec {
            dbg.last_command = history_cmd;
            dbg.running_continuously = matches!(history_cmd, LastCommand::Continue);
        }

        // Execute the instruction if requested (do_exec == true => either 's' or 'c')
        if do_exec {
            // Fetch the instruction at CURRENT ip (re-get in case anything changed)
            let ir_insn = cpu
                .program
                .instructions
                .get(cpu.ip)
                .ok_or_else(|| EmuError::InternalError(format!("Invalid IP: {}", cpu.ip)))?
                .clone();

            let halt = cpu.execute_instruction_ir(&ir_insn)?;

            // record last executed instruction so we can display it even when IP moves out-of-bounds
            dbg.last_executed_insn = Some(ir_insn);

            if halt {
                println!("Program halted due to exit syscall.");
                cpu.dump_state_full();
                // show context around last executed instruction
                visualize_source(cpu, &dbg, 5);
                return Ok(());
            }

            // Increment ip (cpu.execute_* handlers set ip as necessary; they usually set ip to target-1)
            cpu.ip = cpu.ip.saturating_add(1);

            // If we're not running continuously, show state & code context
            if !dbg.running_continuously {
                visualize_source(cpu, &dbg, 5);
                cpu.dump_state_full();
            }

            // Loop continues; next iteration will check breakpoints at the new ip
        } else {
            // command didn't execute an instruction (e.g., info reg, list, break), show context again
            visualize_source(cpu, &dbg, 5);
        }
    }

    println!("Debugger exiting.");
    cpu.dump_state_full();
    Ok(())
}
