use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED, Word};
use std::convert::TryInto;
use std::sync::atomic::Ordering;

// AArch64 Linux Syscall Numbers
const SYS_WRITE: Word = 64;
const SYS_EXIT: Word = 93;

/// Executes the system call defined by register X8 in the CPU state.
///
/// Returns:
/// - Ok(true) if the emulator should halt (e.g., for SYS_EXIT).
/// - Ok(false) if execution should continue.
/// - Err otherwise.
pub fn handle_syscall(state: &mut CpuState) -> EmuResult<bool> {
    let syscall_num = state.get_reg(8); // Syscall number is in X8

    match syscall_num {
        SYS_WRITE => {
            // write(fd, buf, count)
            let fd = state.get_reg(0);
            let buf_addr = state.get_reg(1);
            let count = state.get_reg(2) as usize;

            if count == 0 {
                state.set_reg(0, 0);
                return Ok(false);
            }

            // Only handle stdout (1) and stderr (2)
            if fd == 1 || fd == 2 {
                match state.memory.read_bytes(buf_addr, count) {
                    Ok(bytes) => {
                        // let is_ascii_text = bytes.iter().all(|&b| {
                        //     b.is_ascii_graphic() || b.is_ascii_whitespace() || b == b'\n'
                        // });

                        let is_ascii_text = bytes.iter().all(|&b| b.is_ascii());

                        if is_ascii_text {
                            // Pretty-print ASCII output
                            let text = String::from_utf8_lossy(bytes);
                            print!("[SYSCALL WRITE] {}", text);
                        } else if count == 8 {
                            // If exactly 8 bytes, interpret as u64
                            let val = u64::from_le_bytes(bytes.try_into().unwrap());
                            println!("[SYSCALL WRITE] {}", val);
                        } else {
                            // Fallback: hex dump
                            print!("[SYSCALL WRITE - RAW BYTES]");
                            for b in bytes {
                                print!(" {:02X}", b);
                            }
                            println!();
                        }
                    }
                    Err(e) => {
                        eprintln!("[SYSCALL ERROR] SYS_WRITE failed to read memory: {:?}", e);
                        state.set_reg(0, !0u64);
                        return Err(e);
                    }
                }
            } else {
                if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                    eprintln!("[SYSCALL WARNING] Write call to unsupported FD: {}", fd);
                }
            }

            // Return bytes written in X0
            state.set_reg(0, count as u64);
            Ok(false)
        }
        SYS_EXIT => {
            let exit_code = state.get_reg(0) as i32;
            if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                println!("[SYSCALL EXIT] Exiting with code: {}", exit_code);
            }

            Ok(true)
        }

        _ => Err(EmuError::InternalError(format!(
            "Unimplemented system call number: {}",
            syscall_num
        ))),
    }
}
