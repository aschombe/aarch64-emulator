// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};
use crate::vfs::FileAccessMode;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

#[cfg(test)]
mod tests;

// AArch64 Linux Syscall Numbers
const SYS_OPENAT: Word = 56;
const SYS_CLOSE: Word = 57;
const SYS_LSEEK: Word = 62;
const SYS_READ: Word = 63;
const SYS_WRITE: Word = 64;
const SYS_EXIT: Word = 93;

/// Helper: escape control characters for readable log output
fn escape_string(input: &[u8]) -> String {
    let mut s = String::new();
    for &b in input {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02X}", b)),
        }
    }
    s
}

/// Executes a system call.
///
/// Returns:
/// - Ok(true) if execution should halt (e.g. SYS_EXIT)
/// - Ok(false) to continue
/// - Err() if the syscall causes emulator error
pub fn handle_syscall(state: &mut CpuState) -> EmuResult<bool> {
    let syscall_num = state.get_reg(8);

    match syscall_num {
        SYS_OPENAT => {
            let pathname_addr = state.get_reg(1);
            let flags = state.get_reg(2);

            let path_str = {
                let mem = state.memory.borrow();
                mem.read_c_string(pathname_addr)?
            };

            if let Some(vfs) = &mut state.vfs {
                match vfs.open(&path_str, flags) {
                    Ok(fd) => {
                        println!("[SYS_OPENAT] Opened '{}' -> fd={}", path_str, fd);
                        state.set_reg(0, fd as i64);
                    }
                    Err(e) => {
                        eprintln!("[SYS_OPENAT ERROR] '{}' open failed: {}", path_str, e);
                        state.set_reg(0, -1);
                    }
                }
            } else {
                eprintln!("[SYS_OPENAT WARNING] No VFS mounted");
                state.set_reg(0, -1);
            }

            Ok(false)
        }

        SYS_CLOSE => {
            let fd = state.get_reg(0);

            if let Some(vfs) = &mut state.vfs {
                match vfs.close(fd) {
                    Ok(_) => {
                        println!("[SYS_CLOSE] Closed fd={}", fd);
                        state.set_reg(0, 0);
                    }
                    Err(e) => {
                        eprintln!("[SYS_CLOSE ERROR] {}", e);
                        state.set_reg(0, -1);
                    }
                }
            } else {
                eprintln!("[SYS_CLOSE WARNING] No VFS mounted");
                state.set_reg(0, -1);
            }

            Ok(false)
        }

        SYS_READ => {
            let fd = state.get_reg(0);
            let buf_addr = state.get_reg(1);
            let count = state.get_reg(2) as usize;

            if let Some(vfs) = &mut state.vfs {
                let mut buf = vec![0u8; count];
                match vfs.read(fd, &mut buf) {
                    Ok(bytes_read) => {
                        {
                            let mut mem_ref = state.memory.borrow_mut();
                            mem_ref.write_bytes(buf_addr, &buf[..bytes_read])?;
                        }

                        let escaped = escape_string(&buf[..bytes_read]);
                        println!(
                            "[SYS_READ] FD {} -> Read {} bytes: \"{}\"",
                            fd, bytes_read, escaped
                        );
                        state.set_reg(0, bytes_read as i64);
                    }
                    Err(e) => {
                        eprintln!("[SYS_READ ERROR] {}", e);
                        state.set_reg(0, -1);
                    }
                }
            } else {
                eprintln!("[SYS_READ WARNING] No VFS mounted");
                state.set_reg(0, -1);
            }

            Ok(false)
        }

        SYS_WRITE => {
            let fd = state.get_reg(0);
            let buf_addr = state.get_reg(1);
            let count = state.get_reg(2) as usize;

            if count == 0 {
                println!("[SYS_WRITE] Zero-length write ignored");
                state.set_reg(0, 0);
                return Ok(false);
            }

            let bytes = {
                let mem_ref = state.memory.borrow();
                mem_ref.read_bytes(buf_addr, count)?.to_vec()
            };

            let escaped = escape_string(&bytes);

            // stdout/stderr
            if fd == 1 || fd == 2 {
                println!("[SYS_WRITE - STDOUT] {}", escaped);
                std::io::stdout().flush()?;
                state.set_reg(0, count as i64);
                return Ok(false);
            }

            // normal file write
            if let Some(vfs) = &mut state.vfs {
                if let Some(vfh) = vfs.fd_map.get_mut(&fd) {
                    if matches!(vfh.mode, FileAccessMode::ReadOnly) {
                        eprintln!("[SYS_WRITE ERROR] fd={} is read-only", fd);
                        state.set_reg(0, 0);
                        return Ok(false);
                    }

                    let mut file = OpenOptions::new().write(true).open(&vfh.host_path)?;
                    file.seek(SeekFrom::Start(vfh.position as u64))?;
                    file.write_all(&bytes)?;
                    vfh.position += count;

                    println!(
                        "[SYS_WRITE] fd={} -> Wrote {} bytes to '{}' | Data: \"{}\"",
                        fd,
                        count,
                        vfh.host_path.display(),
                        escaped
                    );

                    state.set_reg(0, count as i64);
                    return Ok(false);
                }
            }

            eprintln!("[SYS_WRITE WARNING] FD {} unsupported", fd);
            state.set_reg(0, 0);
            Ok(false)
        }

        SYS_LSEEK => {
            let fd = state.get_reg(0);
            let offset = state.get_reg(1) as i64;
            let whence = state.get_reg(2) as u32;

            if let Some(vfs) = &mut state.vfs {
                match vfs.lseek(fd, offset, whence) {
                    Ok(new_offset) => {
                        println!(
                            "[SYS_LSEEK] fd={} -> new position {} (whence={})",
                            fd, new_offset, whence
                        );
                        state.set_reg(0, new_offset as i64);
                    }
                    Err(e) => {
                        eprintln!("[SYS_LSEEK ERROR] {}", e);
                        state.set_reg(0, -1);
                    }
                }
            } else {
                eprintln!("[SYS_LSEEK WARNING] No VFS mounted");
                state.set_reg(0, -1);
            }

            Ok(false)
        }

        SYS_EXIT => {
            let code = state.get_reg(0) as i32;
            println!("[SYS_EXIT] Emulator exiting with code {}", code);
            Ok(true)
        }

        _ => {
            println!("[SYS] UNKNOWN syscall {}", syscall_num);
            Err(EmuError::SyscallError {
                message: format!("Unimplemented system call number: {}", syscall_num),
            })
        }
    }
}
