use std::fmt;
use std::sync::atomic::AtomicBool;

/// Flag set by CLI arguments to enable detailed output tracing
pub static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

// Memory constants
// pub const MEMORY_SIZE: u64 = 0x4000_0000; // 1 GB total simulated RAM
// pub const STACK_SIZE: u64 = 0x800000; // 8 MB stack
// pub const STACK_TOP: u64 = MEMORY_SIZE; // top of address space
// pub const STACK_START: u64 = STACK_TOP - STACK_SIZE; // bottom of the stack (grows downward)

// mem.regions.push(MemoryRegion {
//     name: "text",
//     base: 0x0,
//     size: 0x100000,
// });
// mem.regions.push(MemoryRegion {
//     name: "rodata",
//     base: 0x100000,
//     size: 0x200000,
// });
// mem.regions.push(MemoryRegion {
//     name: "data",
//     base: 0x300000,
//     size: 0x100000,
// });
// mem.regions.push(MemoryRegion {
//     name: "heap",
//     base: 0x400000,
//     size: 0x100000,
// });
// mem.regions.push(MemoryRegion {
//     name: "stack",
//     base: STACK_START,
//     size: STACK_TOP - STACK_START,
// });

// Memory region constants
pub const TEXT_BASE: u64 = 0x0;
pub const TEXT_SIZE: u64 = 0x100000; // 1 MB
pub const RODATA_BASE: u64 = TEXT_BASE + TEXT_SIZE; // 0x100000
pub const RODATA_SIZE: u64 = 0x200000; // 2 MB
pub const DATA_BASE: u64 = RODATA_BASE + RODATA_SIZE; // 0x300000
pub const DATA_SIZE: u64 = 0x100000; // 1 MB
pub const HEAP_BASE: u64 = DATA_BASE + DATA_SIZE; // 0x400000
pub const HEAP_SIZE: u64 = 0x100000; // 1 MB
pub const STACK_TOP: u64 = 0x4000_0000; // 1 GB
pub const STACK_SIZE: u64 = 0x800000; // 8 MB
pub const STACK_START: u64 = STACK_TOP - STACK_SIZE; // 0x3F800000
pub const MEMORY_SIZE: u64 = 0x4000_0000; // 1 GB total simulated RAM

// Type aliases
pub type Word = u64; // 64-bit data/address word
// pub type Instruction = u32; // 32-bit instruction word

// Custom error type for the emulator
#[derive(Debug, Clone)]
pub enum EmuError {
    /// PC tried to access memory outside the defined RAM boundary
    MemoryAccessViolation(Word),
    /// Stack smashing detected
    StackSmashDetected(Word),
    /// Division by zero detected
    DivisionByZero,
    ///// An instruction IR was invalid or unimplemented
    // InvalidInstructionIR(String),
    /// Syscall requested is not implemented
    UnimplementedSyscall(String),
    /// General internal failure
    InternalError(String),
    /// I/O failure (e.g., file not found, memory mapping error)
    IoError(String),
    ///// Plugin-related error
    // PluginError(String),
}

// User-friendly error messages
impl fmt::Display for EmuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmuError::MemoryAccessViolation(addr) => {
                write!(f, "Memory Access Violation at 0x{:X}", addr)
            }
            EmuError::StackSmashDetected(addr) => {
                write!(f, "Stack Smash Detected at 0x{:X}", addr)
            }
            EmuError::DivisionByZero => write!(f, "Division by zero"),
            // EmuError::InvalidInstructionIR(msg) => write!(f, "Invalid Instruction IR: {}", msg),
            EmuError::UnimplementedSyscall(num) => write!(f, "Unimplemented Syscall: {}", num),
            EmuError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
            EmuError::IoError(msg) => write!(f, "I/O Error: {}", msg),
            // EmuError::PluginError(msg) => write!(f, "Plugin Error: {}", msg),
        }
    }
}

impl From<mlua::Error> for EmuError {
    fn from(err: mlua::Error) -> Self {
        EmuError::InternalError(format!("Lua error: {}", err))
    }
}

impl From<std::io::Error> for EmuError {
    fn from(err: std::io::Error) -> EmuError {
        EmuError::InternalError(format!("IO error: {}", err))
    }
}

pub type EmuResult<T> = Result<T, EmuError>;
