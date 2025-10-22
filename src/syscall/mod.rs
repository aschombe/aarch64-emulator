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

            if fd == 1 || fd == 2 {
                // Clone bytes into an owned Vec<u8> to drop borrow before mutable uses
                let read_result: EmuResult<Vec<u8>> = {
                    let mem_ref = state.memory.borrow();
                    mem_ref
                        .read_bytes(buf_addr, count)
                        .map(|slice| slice.to_vec())
                };

                match read_result {
                    Ok(bytes) => {
                        let is_ascii_text = bytes.iter().all(|&b| b.is_ascii());
                        if is_ascii_text {
                            print!("[SYSCALL WRITE] ");
                            for &b in &bytes {
                                match b {
                                    b'\n' => print!("\\n\n"),
                                    b'\r' => print!("\\r"),
                                    b'\t' => print!("\\t"),
                                    0x20..=0x7E => print!("{}", b as char),
                                    _ => print!("\\x{:02X}", b),
                                }
                            }
                            println!();
                        } else if count == 8 {
                            let val = u64::from_le_bytes(bytes.try_into().unwrap());
                            println!("[SYSCALL WRITE] {}", val);
                        } else {
                            println!("[SYSCALL WRITE - RAW BYTES]");
                            for b in &bytes {
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
            } else if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                eprintln!("[SYSCALL WARNING] Write call to unsupported FD: {}", fd);
            }

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
