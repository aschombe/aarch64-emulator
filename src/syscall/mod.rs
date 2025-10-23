use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED, Word};
use crate::vfs::FileAccessMode;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::Ordering;

// AArch64 Linux Syscall Numbers
const SYS_OPENAT: Word = 56;
const SYS_CLOSE: Word = 57;
const SYS_LSEEK: Word = 62;
const SYS_READ: Word = 63;
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
        SYS_OPENAT => {
            // openat(dirfd, pathname, flags, mode)
            let pathname_addr = state.get_reg(1);
            let flags = state.get_reg(2);

            // Read the file path string from emulator memory
            let path_str = {
                let mem = state.memory.borrow();
                mem.read_c_string(pathname_addr)?
            };

            if let Some(vfs) = &mut state.vfs {
                match vfs.open(&path_str, flags) {
                    Ok(fd) => {
                        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                            println!("[SYS_OPENAT] Opened '{}' -> fd={}", path_str, fd);
                        }
                        state.set_reg(0, fd);
                    }
                    Err(e) => {
                        eprintln!("[SYS_OPENAT ERROR] Failed to open '{}': {}", path_str, e);
                        state.set_reg(0, u64::MAX);
                    }
                }
            } else {
                state.set_reg(0, u64::MAX);
            }
            Ok(false)
        }

        SYS_CLOSE => {
            let fd = state.get_reg(0);
            if let Some(vfs) = &mut state.vfs {
                match vfs.close(fd) {
                    Ok(_) => {
                        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                            println!("[SYS_CLOSE] Closed fd={}", fd);
                        }
                        state.set_reg(0, 0);
                    }
                    Err(e) => {
                        eprintln!("[SYS_CLOSE ERROR] {}", e);
                        state.set_reg(0, u64::MAX);
                    }
                }
            } else {
                state.set_reg(0, u64::MAX);
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
                        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                            let s = String::from_utf8_lossy(&buf[..bytes_read]);
                            println!("[SYS_READ] FD {} -> {} bytes: {}", fd, bytes_read, s);
                        }
                        state.set_reg(0, bytes_read as u64);
                    }
                    Err(e) => {
                        eprintln!("[SYS_READ ERROR] {}", e);
                        state.set_reg(0, u64::MAX);
                    }
                }
            } else {
                state.set_reg(0, u64::MAX);
            }
            Ok(false)
        }

        SYS_WRITE => {
            let fd = state.get_reg(0);
            let buf_addr = state.get_reg(1);
            let count = state.get_reg(2) as usize;

            if count == 0 {
                state.set_reg(0, 0);
                return Ok(false);
            }

            let bytes = {
                let mem_ref = state.memory.borrow();
                mem_ref.read_bytes(buf_addr, count)?.to_vec()
            };

            // stdout/stderr
            if fd == 1 || fd == 2 {
                let output_str = String::from_utf8_lossy(&bytes);
                print!("[SYS_WRITE] {}", output_str);
                std::io::stdout().flush()?;
                state.set_reg(0, count as u64);
                return Ok(false);
            }

            // Normal file write
            if let Some(vfs) = &mut state.vfs {
                if let Some(vfh) = vfs.fd_map.get_mut(&fd) {
                    if matches!(vfh.mode, FileAccessMode::ReadOnly) {
                        eprintln!("[SYS_WRITE ERROR] fd={} is read-only", fd);
                        state.set_reg(0, u64::MAX);
                        return Ok(false);
                    }

                    let mut file = OpenOptions::new().write(true).open(&vfh.host_path)?;
                    file.seek(SeekFrom::Start(vfh.position as u64))?;
                    file.write_all(&bytes)?;
                    vfh.position += count;

                    if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                        println!(
                            "[SYS_WRITE] fd={} -> wrote {} bytes to '{}'",
                            fd,
                            count,
                            vfh.host_path.display()
                        );
                    }

                    state.set_reg(0, count as u64);
                    return Ok(false);
                }
            }

            eprintln!("[SYS_WRITE WARNING] Unsupported FD {}", fd);
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
                        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                            println!("[SYS_LSEEK] FD {} -> new position {}", fd, new_offset);
                        }
                        state.set_reg(0, new_offset);
                    }
                    Err(e) => {
                        eprintln!("[SYS_LSEEK ERROR] {}", e);
                        state.set_reg(0, u64::MAX);
                    }
                }
            } else {
                state.set_reg(0, u64::MAX);
            }
            Ok(false)
        }

        SYS_EXIT => {
            let code = state.get_reg(0) as i32;
            if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                println!("[SYS_EXIT] Emulator exiting with code {}", code);
            }
            Ok(true)
        }

        _ => Err(EmuError::InternalError(format!(
            "Unimplemented system call number: {}",
            syscall_num
        ))),
    }
}
