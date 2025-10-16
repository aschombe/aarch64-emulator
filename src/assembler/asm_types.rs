use crate::types::Word;
use std::collections::HashMap;

/// The Symbol Table maps labels (strings) to their resolved memory address (Word)
pub type SymbolTable = HashMap<String, Word>;

// Type aliases
pub type Quad = i64;
pub type Wword = i32;

// Operand definitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Immediate {
    Lit(Quad),
    Lbl(String),
}

/// Enum representing ARM64 registers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Reg {
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    X8,
    X9,
    X10,
    X11,
    X12,
    X13,
    X14,
    X15,
    X16,
    X17,
    X18,
    X19,
    X20,
    X21,
    X22,
    X23,
    X24,
    X25,
    X26,
    X27,
    X28,
    SP,
    LR,
    XZR,
    W0,
    W1,
    W2,
    W3,
    W4,
    W5,
    W6,
    W7,
    W8,
    W9,
    W10,
    W11,
    W12,
    W13,
    W14,
    W15,
    W16,
    W17,
    W18,
    W19,
    W20,
    W21,
    W22,
    W23,
    W24,
    W25,
    W26,
    W27,
    W28,
    W29,
    W30,
    W31,
}

impl Reg {
    /// Converts a Reg enum variant into its physical register index (0-31)
    pub fn to_id(self) -> usize {
        match self {
            Reg::X0 | Reg::W0 => 0,
            Reg::X1 | Reg::W1 => 1,
            Reg::X2 | Reg::W2 => 2,
            Reg::X3 | Reg::W3 => 3,
            Reg::X4 | Reg::W4 => 4,
            Reg::X5 | Reg::W5 => 5,
            Reg::X6 | Reg::W6 => 6,
            Reg::X7 | Reg::W7 => 7,
            Reg::X8 | Reg::W8 => 8,
            Reg::X9 | Reg::W9 => 9,
            Reg::X10 | Reg::W10 => 10,
            Reg::X11 | Reg::W11 => 11,
            Reg::X12 | Reg::W12 => 12,
            Reg::X13 | Reg::W13 => 13,
            Reg::X14 | Reg::W14 => 14,
            Reg::X15 | Reg::W15 => 15,
            Reg::X16 | Reg::W16 => 16,
            Reg::X17 | Reg::W17 => 17,
            Reg::X18 | Reg::W18 => 18,
            Reg::X19 | Reg::W19 => 19,
            Reg::X20 | Reg::W20 => 20,
            Reg::X21 | Reg::W21 => 21,
            Reg::X22 | Reg::W22 => 22,
            Reg::X23 | Reg::W23 => 23,
            Reg::X24 | Reg::W24 => 24,
            Reg::X25 | Reg::W25 => 25,
            Reg::X26 | Reg::W26 => 26,
            Reg::X27 | Reg::W27 => 27,
            Reg::X28 | Reg::W28 => 28,
            Reg::SP | Reg::W29 => 29,
            Reg::LR | Reg::W30 => 30,
            Reg::XZR | Reg::W31 => 31,
        }
    }
}

/// Enum representing different addressing modes for memory access
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offset {
    Ind1(Immediate),
    Ind2(Reg),
    Ind3(Reg, Immediate),
    Ind4(Reg, Reg),
}

/// Enum representing different types of operands in an instruction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Imm(Immediate),
    Reg(Reg),
    Offset(Offset),
}

/// Enum representing condition codes for conditional instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Al,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Enum representing the various operation codes (opcodes) for instructions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCode {
    MOV,
    ADR,
    ADD,
    SUB,
    MUL,
    UDIV,
    SDIV,
    ADDS,
    SUBS,
    MULS,
    UDIVS,
    SDIVS,
    AND,
    ORR,
    EOR,
    NOT,
    LSL,
    LSR,
    ASR,
    LDR,
    STR,
    LDRB,
    STRB,
    B(Condition),
    BL,
    RET,
    CMP,
    CBZ,
    CBNZ,
    SVC,
}

/// Struct representing a single instruction in the intermediate representation (IR)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionIR {
    pub opcode: OpCode,
    pub operands: Vec<Operand>,
}

/// Enum representing different types of data in the data section
#[derive(Debug, Clone)]
pub enum Data {
    Quad(Quad),
    QuadArr(Vec<Quad>),
    Word(Wword),
    WordArr(Vec<Wword>),
    Byte(u8),
    ByteArr(Vec<u8>),
    IntArr(Vec<i32>),
}

/// Enum representing the content of an assembly block, which can be either text (instructions) or
/// data
#[derive(Debug, Clone)]
pub enum AssemblyContent {
    Text(Vec<InstructionIR>),
    Data(Vec<Data>),
    Bss(Word),
}

/// Struct representing an assembly block, which includes a label, an entry flag, and the content
#[derive(Debug, Clone)]
pub struct AssemblyBlock {
    pub label: String,
    pub is_entry: bool,
    pub content: AssemblyContent,
}
