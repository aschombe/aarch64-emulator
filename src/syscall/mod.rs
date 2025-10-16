use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED, Word};
use std::convert::TryInto;
use std::sync::atomic::Ordering;

// AArch64 Linux Syscall Numbers
const SYS_WRITE: Word = 64;

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
            // Write (64) signature: (int fd, const char *buf, size_t count)
            let fd = state.get_reg(0);
            let buf_addr = state.get_reg(1); // Pointer to the data buffer
            let count = state.get_reg(2) as usize; // Length of the message

            if count == 0 {
                state.set_reg(0, 0);
                return Ok(false);
            }

            if fd == 1 || fd == 2 {
                // 1. Read the required number of bytes from simulated memory
                match state.memory.read_bytes(buf_addr, count) {
                    Ok(bytes) => {
                        // 2. Convert the byte slice to a host string (handling invalid UTF-8 gracefully)
                        let output = String::from_utf8_lossy(bytes);

                        // 3. Output the string to the host console (stdout/stderr)
                        print!("{}", output);

                        // Note: Verbose logging for the string printing is now handled by the regular 'print' call.
                    }
                    Err(e) => {
                        eprintln!("[SYSCALL_ERROR] SYS_WRITE failed to read memory: {:?}", e);
                        state.set_reg(0, !0 as u64); // Set X0 to -1 (error)
                        return Err(e);
                    }
                }
            } else {
                if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                    eprintln!("[SYSCALL_WARNING] Write call to unsupported FD: {}", fd);
                }
            }

            // AArch64 convention: return value (bytes written) goes into X0
            state.set_reg(0, count as u64);
            Ok(false)
        }

        _ => Err(EmuError::InternalError(format!(
            "Unimplemented system call number: {}",
            syscall_num
        ))),
    }
}
