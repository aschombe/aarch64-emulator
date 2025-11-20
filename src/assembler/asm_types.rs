// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

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
    Lo12Lbl(String),
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
    X29,
    X30,
    SP,
    WSP,
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
    WZR,
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
            Reg::X29 | Reg::W29 => 29,
            Reg::X30 | Reg::W30 | Reg::LR => 30,
            Reg::XZR | Reg::W31 | Reg::WZR => 31,
            Reg::SP | Reg::WSP => 32,
        }
    }

    pub fn is_w_register(self) -> bool {
        matches!(
            self,
            Reg::W0
                | Reg::W1
                | Reg::W2
                | Reg::W3
                | Reg::W4
                | Reg::W5
                | Reg::W6
                | Reg::W7
                | Reg::W8
                | Reg::W9
                | Reg::W10
                | Reg::W11
                | Reg::W12
                | Reg::W13
                | Reg::W14
                | Reg::W15
                | Reg::W16
                | Reg::W17
                | Reg::W18
                | Reg::W19
                | Reg::W20
                | Reg::W21
                | Reg::W22
                | Reg::W23
                | Reg::W24
                | Reg::W25
                | Reg::W26
                | Reg::W27
                | Reg::W28
                | Reg::W29
                | Reg::W30
                | Reg::W31
                | Reg::WZR
                | Reg::WSP
        )
    }
}

/// Enum representing different addressing modes for memory access
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offset {
    Ind1(Immediate),                   // [imm]
    Ind2(Reg),                         // [reg]
    Ind3(Reg, Immediate),              // [reg, imm]
    Ind4(Reg, Reg),                    // [reg, reg]
    Ind5(Reg, Box<Operand>),           // [reg, reg/imm with shift/extend]
    PreIndexed(Reg, Immediate),        // [reg, imm]!
    PostIndexed(Reg, Immediate),       // [reg], imm
    PreIndexedReg(Reg, Box<Operand>),  // [reg, reg/imm with shift/extend]!
    PostIndexedReg(Reg, Box<Operand>), // [reg], reg/imm with shift/extend
}

/// Enum representing different types of operands in an instruction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Imm(Immediate),
    ImmWithShift(i64, ShiftOrExtendKind, u8),
    Reg(Reg),
    Offset(Offset),
    RegWithMod(Box<OperandWithShiftExtend>),
}

/// Enum representing condition codes for conditional instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Al,
    Nv,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Cs,
    Hs,
    Cc,
    Lo,
    Mi,
    Pl,
    Vs,
    Vc,
    Hi,
    Ls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovType {
    Normal,
    K, // MOVK
    Z, // MOVZ
    N, // MOVN
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftOrExtendKind {
    LSL,
    LSR,
    ASR,
    ROR,
    MSL,
    UXTB,
    UXTH,
    UXTW,
    UXTX,
    SXTB,
    SXTH,
    SXTW,
    SXTX,
}

/// Struct representing an operand with an optional shift or extend modifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperandWithShiftExtend {
    pub base: Operand,                       // Register/imm
    pub modifier: Option<ShiftOrExtendKind>, // None means no modifier
    pub amount: u8,                          // Amount (if relevant)
}

/// Enum representing the various operation codes (opcodes) for instructions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCode {
    MOV(MovType),
    NEG,
    NEGS,
    ADR,
    ADRP,
    ADD,
    SUB,
    MUL,
    UMULL,
    SMULL,
    UMULH,
    SMULH,
    MADD,   // Rd = Rn * Rm + Ra (signed/unsigned detected by context)
    MSUB,   // Rd = Rn * Rm - Ra
    MNEG,   // Rd = -(Rn * Rm)
    SMADDL, // signed multiply-add long (wide)
    SMSUBL, // signed multiply-subtract long
    SMNEGL, // signed multiply-negate long
    UMADDL, // unsigned multiply-add long
    UMSUBL, // unsigned multiply-subtract long
    UMNEGL, // unsigned multiply-negate long
    CLS,
    CLZ,
    CNT,
    CTZ,
    RBIT,
    REV,
    REV16,
    REV32,
    REV64,

    CSEL(Condition),
    CSINC(Condition),
    CSINV(Condition),
    CSNEG(Condition),
    CSET(Condition),
    CSETM(Condition),
    CINC(Condition),
    CINV(Condition),
    CNEG(Condition),

    CCMPReg(Condition), // For CCMP <Rn>, <Rm>, #nzcv, <cond>
    CCMPImm(Condition), // For CCMP <Rn>, #imm, #nzcv, <cond>
    CCMNReg(Condition), // For CCMN <Rn>, <Rm>, #nzcv, <cond>
    CCMNImm(Condition), // For CCMN <Rn>, #imm, #nzcv, <cond>

    UDIV,
    SDIV,
    ADDS,
    SUBS,
    MULS,
    SMAX,
    SMIN,
    UMAX,
    UMIN,
    UDIVS,
    SDIVS,
    SWP,
    SWPB,
    SWPH,
    SWPP,
    ABS,
    ADC,
    ADCS,
    SBC,
    SBCS,
    NGC,
    NGCS,
    AND,
    ANDS,
    ORR,
    EOR,
    BIC,
    BICS,
    EON,
    ORN,
    NOT,
    LSL,
    LSR,
    ASR,
    ROR,
    MVN,

    SXTB,
    SXTH,
    SXTW,
    SXTX,
    UXTB,
    UXTH,
    UXTW,
    UXTX,

    LDR,
    LDP,
    LDPSW,
    LDRB,
    LDRH,
    LDRSB,
    LDRSH,
    LDRSW,
    LDUR,
    LDURB,
    LDURSB,
    LDURH,
    LDURSH,
    LDURSW,
    STR,
    STP,
    STRB,
    STRH,
    STUR,
    STURB,
    STURH,
    B(Condition),
    BL,
    BR,
    BLR,
    RET,
    CMP,
    CMN,
    TST,
    CBZ,
    CBNZ,
    TBZ,
    TBNZ,
    SVC,
    NOP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectOp {
    Sel,
    Inc,
    Inv,
    Neg,
    Set,
    Setm,
    IncTrue,
    InvTrue,
    NegTrue,
}

/// Struct representing a single instruction in the intermediate representation (IR)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionIR {
    pub opcode: OpCode,
    pub operands: Vec<Operand>,
}

impl InstructionIR {
    /// Converts the instruction to its assembly string representation
    pub fn to_asm_string(&self) -> String {
        let opcode_str = format!("{:?}", self.opcode).to_lowercase();
        let operands_str: Vec<String> = self
            .operands
            .iter()
            .map(|op| format!("{:?}", op).to_lowercase())
            .collect();
        format!("{} {}", opcode_str, operands_str.join(", "))
    }
}

/// Enum representing different types of data in the data section
#[derive(Debug, Clone)]
pub enum Data {
    Align(usize),
    Quad(Quad),
    QuadArr(Vec<Quad>),
    Word(Wword),
    WordArr(Vec<Wword>),
    Byte(u8),
    ByteArr(Vec<u8>),
    IntArr(Vec<i32>),
    DoubleArr(Vec<f64>),
    FloatArr(Vec<f32>),
}

impl Data {
    /// Returns the size in bytes of the data item
    pub fn size_in_bytes(&self) -> usize {
        match self {
            Data::Align(_) => 0, // Alignment does not occupy space
            Data::Quad(_) => 8,
            Data::QuadArr(arr) => arr.len() * 8,
            Data::Word(_) => 4,
            Data::WordArr(arr) => arr.len() * 4,
            Data::Byte(_) => 1,
            Data::ByteArr(arr) => arr.len(),
            Data::IntArr(arr) => arr.len() * 4,
            Data::DoubleArr(arr) => arr.len() * 8,
            Data::FloatArr(arr) => arr.len() * 4,
        }
    }
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
    pub _is_entry: bool,
    pub content: AssemblyContent,
    pub base_addr: Word,
}
