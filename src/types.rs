// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use std::fmt;
use std::sync::atomic::AtomicBool;

/// Flag set by CLI arguments to enable detailed output tracing
pub static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

// Memory region constants
pub const IMAGE_BASE: u64 = 0x000000;
pub const TEXT_BASE: u64 = IMAGE_BASE;
pub const TEXT_SIZE: u64 = 0x0010000; // 64 KB code + rodata + data
pub const DATA_BASE: u64 = TEXT_BASE + TEXT_SIZE;
pub const DATA_SIZE: u64 = 0x0008000; // 32 KB writable data
pub const BSS_BASE: u64 = DATA_BASE + DATA_SIZE;
pub const BSS_SIZE: u64 = 0x0008000; // 32 KB zeroed bss
pub const HEAP_BASE: u64 = BSS_BASE + BSS_SIZE;
pub const HEAP_SIZE: u64 = 0x0040000; // 256 KB heap
pub const STACK_TOP: u64 = 0x0080_0000; // 8 MB top of stack
pub const STACK_SIZE: u64 = 0x00080000; // 512 KB stack
pub const STACK_START: u64 = STACK_TOP - STACK_SIZE;
pub const MEMORY_SIZE: u64 = 0x0100_0000; // total 16 MB of emulated RAM
// Type aliases
pub type Word = u64; // 64-bit data/address word
// pub type Instruction = u32; // 32-bit instruction word

// // Custom error type for the emulator
// #[derive(Debug, Clone)]
// pub enum EmuError {
//     /// PC tried to access memory outside the defined RAM boundary
//     MemoryAccessViolation(Word),
//     /// Stack smashing detected
//     StackSmashDetected(Word),
//     /// Division by zero detected
//     DivisionByZero,
//     /// An instruction IR was invalid or unimplemented
//     InvalidInstructionIR(String),
//     /// Syscall requested is not implemented
//     UnimplementedSyscall(String),
//     /// General internal failure
//     InternalError(String),
//     /// I/O failure (e.g., file not found, memory mapping error)
//     IoError(String),
//     /// Plugin-related error
//     PluginError(String),
// }
//
// impl fmt::Display for EmuError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             EmuError::MemoryAccessViolation(addr) => write!(
//                 f,
//                 "Error: Tried to access an invalid memory address (0x{:X}).\n\
//                  This address is outside the emulator's allowed memory regions.",
//                 addr
//             ),
//             EmuError::StackSmashDetected(addr) => write!(
//                 f,
//                 "Error: Stack smash detected at address 0x{:X}.\n\
//                  This usually indicates a buffer overflow or stack corruption.",
//                 addr
//             ),
//             EmuError::DivisionByZero => {
//                 write!(f, "Error: Division by zero encountered during execution.")
//             }
//             EmuError::InvalidInstructionIR(msg) => write!(
//                 f,
//                 "Error: Encountered an invalid or unimplemented instruction IR.\n\
//                  Details: {}\n\
//                  Please check your assembly source for correctness.",
//                 msg
//             ),
//             EmuError::UnimplementedSyscall(num) => write!(
//                 f,
//                 "Error: System call '{}' is not implemented in this emulator.",
//                 num
//             ),
//             EmuError::InternalError(msg) => write!(
//                 f,
//                 "Internal error:\n  {}\n\
//                  Please check your assembly source and report this message if it seems like a bug.",
//                 msg
//             ),
//             EmuError::IoError(msg) => write!(
//                 f,
//                 "File I/O error:\n  {}\n\
//                  Please check file paths and permissions.",
//                 msg
//             ),
//             EmuError::PluginError(msg) => write!(
//                 f,
//                 "Plugin error:\n  {}\n\
//                  Please ensure the plugin is compatible and correctly installed.",
//                 msg
//             ),
//         }
//     }
// }
//
// impl From<mlua::Error> for EmuError {
//     fn from(err: mlua::Error) -> Self {
//         EmuError::InternalError(format!("Lua error: {}", err))
//     }
// }
//
// impl From<std::io::Error> for EmuError {
//     fn from(err: std::io::Error) -> EmuError {
//         EmuError::InternalError(format!("IO error: {}", err))
//     }
// }

#[derive(Debug, Clone)]
pub enum EmuError {
    // Core/runtime
    MemoryAccessViolation {
        addr: Word,
    },
    StackSmashDetected {
        addr: Word,
    },
    DivisionByZero,

    // Loading / files / CLI
    FileError {
        path: Option<String>,
        message: String,
    },
    Utf8Error {
        message: String,
    },
    NoInputFilesProvided,
    CustomEntryPointNotFound {
        symbol: String,
    },

    // Assembler & ELF & parser (all front‑end issues)
    AssemblerError {
        message: String,
    },

    // CPU execution / IR / state
    CpuError {
        message: String,
    },

    // Debugger
    DebuggerError {
        message: String,
    },

    // Syscalls
    SyscallError {
        message: String,
    },

    // Plugins
    PluginError {
        message: String,
    },
}

impl fmt::Display for EmuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmuError::MemoryAccessViolation { addr } => write!(
                f,
                "Error: Tried to access an invalid memory address (0x{:X}).\n\
                 This address is outside the emulator's allowed memory regions.",
                addr
            ),
            EmuError::StackSmashDetected { addr } => write!(
                f,
                "Error: Stack smash detected at address 0x{:X}.\n\
                 This usually indicates a buffer overflow or stack corruption.",
                addr
            ),
            EmuError::DivisionByZero => {
                write!(f, "Error: Division by zero encountered during execution.")
            }
            EmuError::FileError { path, message } => {
                if let Some(p) = path {
                    write!(f, "File error ({}): {}", p, message)
                } else {
                    write!(f, "File error: {}", message)
                }
            }
            EmuError::Utf8Error { message } => {
                write!(f, "UTF-8 error: {}", message)
            }
            EmuError::NoInputFilesProvided => {
                write!(f, "Error: No input files were provided.")
            }
            EmuError::CustomEntryPointNotFound { symbol } => {
                write!(f, "Error: Custom entry point '{}' not found.", symbol)
            }
            EmuError::AssemblerError { message } => {
                write!(f, "Assembler error: {}", message)
            }
            EmuError::CpuError { message } => {
                write!(f, "CPU error: {}", message)
            }
            EmuError::DebuggerError { message } => {
                write!(f, "Debugger error: {}", message)
            }
            EmuError::SyscallError { message } => {
                write!(f, "Syscall error: {}", message)
            }
            EmuError::PluginError { message } => {
                write!(f, "Plugin error: {}", message)
            }
        }
    }
}

impl From<mlua::Error> for EmuError {
    fn from(err: mlua::Error) -> Self {
        EmuError::PluginError {
            message: format!("Lua error: {}", err),
        }
    }
}

impl From<std::io::Error> for EmuError {
    fn from(err: std::io::Error) -> EmuError {
        EmuError::FileError {
            path: None,
            message: format!("IO error: {}", err),
        }
    }
}

pub type EmuResult<T> = Result<T, EmuError>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_debug_format() {
        let err = EmuError::PluginError {
            message: "test".into(),
        };
        assert!(format!("{:?}", err).contains("test"));
    }
}
