use std::fmt;
use std::sync::atomic::AtomicBool;

/// Flag set by CLI arguments to enable detailed output tracing
pub static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

// Memory constants
pub const MEMORY_SIZE: u64 = 0x4000_0000; // 1 GB total simulated RAM
pub const STACK_SIZE: u64 = 0x800000; // 8 MB stack
pub const STACK_TOP: u64 = MEMORY_SIZE; // top of address space
pub const STACK_START: u64 = STACK_TOP - STACK_SIZE; // bottom of the stack (grows downward)

// Type aliases
pub type Word = u64; // 64-bit data/address word
pub type Instruction = u32; // 32-bit instruction word

// Custom error type for the emulator
#[derive(Debug, Clone)]
pub enum EmuError {
    /// PC tried to access memory outside the defined RAM boundary
    MemoryAccessViolation(Word),
    /// Stack smashing detected
    StackSmashDetected(Word),
    /// Division by zero detected
    DivisionByZero,
    /// An instruction IR was invalid or unimplemented
    InvalidInstructionIR(String),
    /// Syscall requested is not implemented
    UnimplementedSyscall(Word),
    /// General internal failure
    InternalError(String),
    /// I/O failure (e.g., file not found, memory mapping error)
    IoError(String),
    /// Plugin-related error
    PluginError(String),
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
            EmuError::InvalidInstructionIR(msg) => write!(f, "Invalid Instruction IR: {}", msg),
            EmuError::UnimplementedSyscall(num) => write!(f, "Unimplemented Syscall: {}", num),
            EmuError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
            EmuError::IoError(msg) => write!(f, "I/O Error: {}", msg),
            EmuError::PluginError(msg) => write!(f, "Plugin Error: {}", msg),
        }
    }
}

pub type EmuResult<T> = Result<T, EmuError>;
