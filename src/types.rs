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
