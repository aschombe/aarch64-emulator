// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License.

use crate::assembler::asm_types::{
    AssemblyBlock, AssemblyContent, Condition, Data, Immediate, InstructionIR, MovType, Offset,
    OpCode, Operand, OperandWithShiftExtend, Reg, ShiftOrExtendKind,
};
use crate::cpu::InterpretedProgram;
use crate::types::{EmuError, EmuResult, Word};

use bad64::{Imm, Op, Operand as Bad64Operand, Shift, decode};

use elf::{ElfBytes, endian::AnyEndian};
use std::collections::{HashMap, HashSet};

// Helper to get section name from section header
fn section_name<'a>(elf: &ElfBytes<'a, AnyEndian>, sh_name: u32) -> Option<&'a str> {
    let strtab_index = elf.ehdr.e_shstrndx as usize;
    let shdrs = elf.section_headers()?;
    let strtab_hdr = shdrs.get(strtab_index).ok()?;
    let (strtab, _) = elf.section_data(&strtab_hdr).ok()?;
    let off = sh_name as usize;
    let end = strtab[off..]
        .iter()
        .position(|b| *b == 0)
        .map(|p| off + p)?;
    std::str::from_utf8(&strtab[off..end]).ok()
}

/// Loads an ELF file and decodes its .text and .data sections
pub fn parse_elf(path: &str) -> EmuResult<(InterpretedProgram, Vec<AssemblyBlock>)> {
    // Parse ELF file
    let file_data =
        std::fs::read(path).map_err(|e| EmuError::IoError(format!("ELF read failed: {}", e)))?;
    let elf = ElfBytes::<AnyEndian>::minimal_parse(&file_data)
        .map_err(|e| EmuError::InternalError(format!("ELF parse failed: {:?}", e)))?;

    // Locate section headers and .text
    let shdrs = elf
        .section_headers()
        .ok_or_else(|| EmuError::InternalError("No section headers in ELF file.".to_string()))?;
    let text_shdr = shdrs
        .iter()
        .find(|sh| {
            section_name(&elf, sh.sh_name)
                .map(|n| n == ".text")
                .unwrap_or(false)
        })
        .ok_or_else(|| EmuError::InternalError("`.text` section not found".to_string()))?;

    // Extract .text bytes
    let (text_bytes, _) = elf
        .section_data(&text_shdr)
        .map_err(|_| EmuError::InternalError("No .text data found".to_string()))?;

    // Extract .data, .rodata, .bss sections
    let mut data_blocks = Vec::new();
    for sh in shdrs.iter() {
        // .data/.rodata: contains actual bytes
        if let Some(name) = section_name(&elf, sh.sh_name) {
            if name == ".data" || name == ".rodata" {
                let (bytes, _) = elf
                    .section_data(&sh)
                    .map_err(|_| EmuError::InternalError(format!("No {} data found", name)))?;
                let mut items = Vec::new();
                // Chop into quad/word/byte as appropriate (here, as bytes):
                for b in bytes {
                    items.push(Data::Byte(*b));
                }
                data_blocks.push(AssemblyBlock {
                    label: name.to_string(),
                    _is_entry: false,
                    content: AssemblyContent::Data(items),
                });
            }
            // .bss: uninitialized, size-only!
            if name == ".bss" {
                let sz = sh.sh_size as Word;
                data_blocks.push(AssemblyBlock {
                    label: name.to_string(),
                    _is_entry: false,
                    content: AssemblyContent::Bss(sz),
                });
            }
        }
    }

    // Compute entry point (offset into .text in instructions)
    let entry_addr = elf.ehdr.e_entry as usize;
    let text_addr = text_shdr.sh_addr as usize;
    let entry_ip = if entry_addr >= text_addr {
        (entry_addr - text_addr) / 4
    } else {
        0
    };

    // Decode instructions using disarm64
    let mut instructions = Vec::new();
    for (i, instr_bytes) in text_bytes.chunks(4).enumerate() {
        if instr_bytes.len() < 4 {
            break;
        }
        let word = u32::from_le_bytes([
            instr_bytes[0],
            instr_bytes[1],
            instr_bytes[2],
            instr_bytes[3],
        ]);
        let addr = text_shdr.sh_addr as u64 + (i * 4) as u64;
        let ir = decode_bad64_to_ir(word, addr)?;
        instructions.push(ir);
    }

    Ok((
        InterpretedProgram {
            instructions,
            label_to_ip: HashMap::new(),
            label_is_addr: HashMap::new(),
            entry_ip,
            extern_labels: HashSet::new(),
            text_base: text_shdr.sh_addr,
            files: Vec::new(),
            ip_map: Vec::new(),
        },
        data_blocks,
    ))
}

fn bad64_reg_to_ir(reg: bad64::Reg) -> EmuResult<Reg> {
    match reg {
        bad64::Reg::X0 => Ok(Reg::X0),
        bad64::Reg::W0 => Ok(Reg::W0),
        bad64::Reg::X1 => Ok(Reg::X1),
        bad64::Reg::W1 => Ok(Reg::W1),
        bad64::Reg::X2 => Ok(Reg::X2),
        bad64::Reg::W2 => Ok(Reg::W2),
        bad64::Reg::X3 => Ok(Reg::X3),
        bad64::Reg::W3 => Ok(Reg::W3),
        bad64::Reg::X4 => Ok(Reg::X4),
        bad64::Reg::W4 => Ok(Reg::W4),
        bad64::Reg::X5 => Ok(Reg::X5),
        bad64::Reg::W5 => Ok(Reg::W5),
        bad64::Reg::X6 => Ok(Reg::X6),
        bad64::Reg::W6 => Ok(Reg::W6),
        bad64::Reg::X7 => Ok(Reg::X7),
        bad64::Reg::W7 => Ok(Reg::W7),
        bad64::Reg::X8 => Ok(Reg::X8),
        bad64::Reg::W8 => Ok(Reg::W8),
        bad64::Reg::X9 => Ok(Reg::X9),
        bad64::Reg::W9 => Ok(Reg::W9),
        bad64::Reg::X10 => Ok(Reg::X10),
        bad64::Reg::W10 => Ok(Reg::W10),
        bad64::Reg::X11 => Ok(Reg::X11),
        bad64::Reg::W11 => Ok(Reg::W11),
        bad64::Reg::X12 => Ok(Reg::X12),
        bad64::Reg::W12 => Ok(Reg::W12),
        bad64::Reg::X13 => Ok(Reg::X13),
        bad64::Reg::W13 => Ok(Reg::W13),
        bad64::Reg::X14 => Ok(Reg::X14),
        bad64::Reg::W14 => Ok(Reg::W14),
        bad64::Reg::X15 => Ok(Reg::X15),
        bad64::Reg::W15 => Ok(Reg::W15),
        bad64::Reg::X16 => Ok(Reg::X16),
        bad64::Reg::W16 => Ok(Reg::W16),
        bad64::Reg::X17 => Ok(Reg::X17),
        bad64::Reg::W17 => Ok(Reg::W17),
        bad64::Reg::X18 => Ok(Reg::X18),
        bad64::Reg::W18 => Ok(Reg::W18),
        bad64::Reg::X19 => Ok(Reg::X19),
        bad64::Reg::W19 => Ok(Reg::W19),
        bad64::Reg::X20 => Ok(Reg::X20),
        bad64::Reg::W20 => Ok(Reg::W20),
        bad64::Reg::X21 => Ok(Reg::X21),
        bad64::Reg::W21 => Ok(Reg::W21),
        bad64::Reg::X22 => Ok(Reg::X22),
        bad64::Reg::W22 => Ok(Reg::W22),
        bad64::Reg::X23 => Ok(Reg::X23),
        bad64::Reg::W23 => Ok(Reg::W23),
        bad64::Reg::X24 => Ok(Reg::X24),
        bad64::Reg::W24 => Ok(Reg::W24),
        bad64::Reg::X25 => Ok(Reg::X25),
        bad64::Reg::W25 => Ok(Reg::W25),
        bad64::Reg::X26 => Ok(Reg::X26),
        bad64::Reg::W26 => Ok(Reg::W26),
        bad64::Reg::X27 => Ok(Reg::X27),
        bad64::Reg::W27 => Ok(Reg::W27),
        bad64::Reg::X28 => Ok(Reg::X28),
        bad64::Reg::W28 => Ok(Reg::W28),
        bad64::Reg::X29 => Ok(Reg::X29),
        bad64::Reg::W29 => Ok(Reg::W29),
        bad64::Reg::X30 => Ok(Reg::X30),
        bad64::Reg::W30 => Ok(Reg::W30),
        bad64::Reg::XZR => Ok(Reg::XZR),
        bad64::Reg::WZR => Ok(Reg::WZR),
        bad64::Reg::SP => Ok(Reg::SP),
        bad64::Reg::WSP => Ok(Reg::SP),
        _ => Err(EmuError::InternalError(format!(
            "Unsupported register: {:?}",
            reg
        ))),
    }
}

fn bad64_opcode_to_ir(op: Op) -> EmuResult<OpCode> {
    match op {
        Op::MOV => Ok(OpCode::MOV(MovType::Normal)),
        Op::MOVK => Ok(OpCode::MOV(MovType::K)),
        Op::MOVZ => Ok(OpCode::MOV(MovType::Z)),
        Op::MOVN => Ok(OpCode::MOV(MovType::N)),
        Op::NEG => Ok(OpCode::NEG),
        Op::NEGS => Ok(OpCode::NEGS),
        Op::ADR => Ok(OpCode::ADR),
        Op::ADRP => Ok(OpCode::ADRP),
        Op::ADD => Ok(OpCode::ADD),
        Op::SUB => Ok(OpCode::SUB),
        Op::MUL => Ok(OpCode::MUL),
        Op::UMULL => Ok(OpCode::UMULL),
        Op::SMULL => Ok(OpCode::SMULL),
        Op::UMULH => Ok(OpCode::UMULH),
        Op::SMULH => Ok(OpCode::SMULH),
        Op::MADD => Ok(OpCode::MADD),
        Op::MSUB => Ok(OpCode::MSUB),
        Op::MNEG => Ok(OpCode::MNEG),
        Op::SMADDL => Ok(OpCode::SMADDL),
        Op::SMSUBL => Ok(OpCode::SMSUBL),
        Op::SMNEGL => Ok(OpCode::SMNEGL),
        Op::UMADDL => Ok(OpCode::UMADDL),
        Op::UMSUBL => Ok(OpCode::UMSUBL),
        Op::UMNEGL => Ok(OpCode::UMNEGL),
        Op::CLS => Ok(OpCode::CLS),
        Op::CLZ => Ok(OpCode::CLZ),
        Op::CNT => Ok(OpCode::CNT),
        Op::CTZ => Ok(OpCode::CTZ),
        Op::RBIT => Ok(OpCode::RBIT),
        Op::REV => Ok(OpCode::REV),
        Op::REV16 => Ok(OpCode::REV16),
        Op::REV32 => Ok(OpCode::REV32),
        Op::REV64 => Ok(OpCode::REV64),
        // Skipping CSEL family, CCMP/CCMN for now
        Op::UDIV => Ok(OpCode::UDIV),
        Op::SDIV => Ok(OpCode::SDIV),
        Op::ADDS => Ok(OpCode::ADDS),
        Op::SUBS => Ok(OpCode::SUBS),
        // Op::MULS => Ok(OpCode::MULS), // MULS not yet supported in bad64
        Op::SMAX => Ok(OpCode::SMAX),
        Op::SMIN => Ok(OpCode::SMIN),
        Op::UMAX => Ok(OpCode::UMAX),
        Op::UMIN => Ok(OpCode::UMIN),
        // Op::UDIVS => Ok(OpCode::UDIVS), // UDIVS not yet supported in bad64
        // Op::SDIVS => Ok(OpCode::SDIVS), // SDIVS not yet supported in bad64
        Op::SWP => Ok(OpCode::SWP),
        Op::SWPB => Ok(OpCode::SWPB),
        Op::SWPH => Ok(OpCode::SWPH),
        Op::SWPP => Ok(OpCode::SWPP),
        Op::ABS => Ok(OpCode::ABS),
        Op::ADC => Ok(OpCode::ADC),
        Op::ADCS => Ok(OpCode::ADCS),
        Op::SBC => Ok(OpCode::SBC),
        Op::SBCS => Ok(OpCode::SBCS),
        Op::NGC => Ok(OpCode::NGC),
        Op::NGCS => Ok(OpCode::NGCS),
        Op::AND => Ok(OpCode::AND),
        Op::ANDS => Ok(OpCode::ANDS),
        Op::ORR => Ok(OpCode::ORR),
        Op::EOR => Ok(OpCode::EOR),
        Op::BIC => Ok(OpCode::BIC),
        Op::BICS => Ok(OpCode::BICS),
        Op::EON => Ok(OpCode::EON),
        Op::ORN => Ok(OpCode::ORN),
        Op::NOT => Ok(OpCode::NOT),
        Op::LSL => Ok(OpCode::LSL),
        Op::LSR => Ok(OpCode::LSR),
        Op::ASR => Ok(OpCode::ASR),
        Op::ROR => Ok(OpCode::ROR),
        Op::MVN => Ok(OpCode::MVN),

        Op::SXTB => Ok(OpCode::SXTB),
        Op::SXTH => Ok(OpCode::SXTH),
        Op::SXTW => Ok(OpCode::SXTW),
        Op::UXTB => Ok(OpCode::UXTB),
        Op::UXTH => Ok(OpCode::UXTH),
        Op::UXTW => Ok(OpCode::UXTW),

        Op::LDR => Ok(OpCode::LDR),
        Op::LDP => Ok(OpCode::LDP),
        Op::LDPSW => Ok(OpCode::LDPSW),
        Op::LDRB => Ok(OpCode::LDRB),
        Op::LDRH => Ok(OpCode::LDRH),
        Op::LDRSB => Ok(OpCode::LDRSB),
        Op::LDRSH => Ok(OpCode::LDRSH),
        Op::LDRSW => Ok(OpCode::LDRSW),
        Op::LDUR => Ok(OpCode::LDUR),
        Op::LDURB => Ok(OpCode::LDURB),
        Op::LDURSB => Ok(OpCode::LDURSB),
        Op::LDURH => Ok(OpCode::LDURH),
        Op::LDURSH => Ok(OpCode::LDURSH),
        Op::LDURSW => Ok(OpCode::LDURSW),
        Op::STR => Ok(OpCode::STR),
        Op::STP => Ok(OpCode::STP),
        Op::STRB => Ok(OpCode::STRB),
        Op::STRH => Ok(OpCode::STRH),
        Op::STUR => Ok(OpCode::STUR),
        Op::STURB => Ok(OpCode::STURB),
        Op::STURH => Ok(OpCode::STURH),
        Op::B => Ok(OpCode::B(Condition::Al)),
        Op::B_AL => Ok(OpCode::B(Condition::Al)),
        Op::B_CC => Ok(OpCode::B(Condition::Cc)),
        Op::B_CS => Ok(OpCode::B(Condition::Cs)),
        Op::B_EQ => Ok(OpCode::B(Condition::Eq)),
        Op::B_GE => Ok(OpCode::B(Condition::Ge)),
        Op::B_GT => Ok(OpCode::B(Condition::Gt)),
        Op::B_HI => Ok(OpCode::B(Condition::Hi)),
        Op::B_LE => Ok(OpCode::B(Condition::Le)),
        Op::B_LS => Ok(OpCode::B(Condition::Ls)),
        Op::B_LT => Ok(OpCode::B(Condition::Lt)),
        Op::B_MI => Ok(OpCode::B(Condition::Mi)),
        Op::B_NE => Ok(OpCode::B(Condition::Ne)),
        Op::B_NV => Ok(OpCode::B(Condition::Nv)),
        Op::B_PL => Ok(OpCode::B(Condition::Pl)),
        Op::B_VC => Ok(OpCode::B(Condition::Vc)),
        Op::B_VS => Ok(OpCode::B(Condition::Vs)),
        Op::BL => Ok(OpCode::BL),
        Op::BR => Ok(OpCode::BR),
        Op::BLR => Ok(OpCode::BLR),
        Op::RET => Ok(OpCode::RET),
        Op::CMP => Ok(OpCode::CMP),
        Op::CMN => Ok(OpCode::CMN),
        Op::TST => Ok(OpCode::TST),
        Op::CBZ => Ok(OpCode::CBZ),
        Op::CBNZ => Ok(OpCode::CBNZ),
        Op::TBZ => Ok(OpCode::TBZ),
        Op::TBNZ => Ok(OpCode::TBNZ),
        Op::SVC => Ok(OpCode::SVC),
        Op::NOP => Ok(OpCode::NOP),
        _ => {
            return Err(EmuError::InternalError(format!(
                "Unsupported instruction: {:?}",
                op
            )));
        }
    }
}

fn extract_imm_value(imm: &Imm) -> i64 {
    match imm {
        Imm::Signed(i) => *i,
        Imm::Unsigned(u) => *u as i64, // Lossless if u64 fits in i64; otherwise handle overflow as needed
    }
}

fn extract_shift_extend_type_and_amount(shift: &Option<Shift>) -> Option<(ShiftOrExtendKind, u8)> {
    match shift {
        Some(s) => match s {
            Shift::LSL(amount) => Some((ShiftOrExtendKind::LSL, *amount as u8)),
            Shift::LSR(amount) => Some((ShiftOrExtendKind::LSR, *amount as u8)),
            Shift::ASR(amount) => Some((ShiftOrExtendKind::ASR, *amount as u8)),
            Shift::ROR(amount) => Some((ShiftOrExtendKind::ROR, *amount as u8)),
            Shift::MSL(amount) => Some((ShiftOrExtendKind::MSL, *amount as u8)),
            Shift::UXTB(amount) => Some((ShiftOrExtendKind::UXTB, *amount as u8)),
            Shift::UXTH(amount) => Some((ShiftOrExtendKind::UXTH, *amount as u8)),
            Shift::UXTW(amount) => Some((ShiftOrExtendKind::UXTW, *amount as u8)),
            Shift::UXTX(amount) => Some((ShiftOrExtendKind::UXTW, *amount as u8)),
            Shift::SXTB(amount) => Some((ShiftOrExtendKind::SXTB, *amount as u8)),
            Shift::SXTH(amount) => Some((ShiftOrExtendKind::SXTH, *amount as u8)),
            Shift::SXTW(amount) => Some((ShiftOrExtendKind::SXTW, *amount as u8)),
            Shift::SXTX(amount) => Some((ShiftOrExtendKind::SXTW, *amount as u8)),
        },
        None => None,
    }
}

// Decodes a single Bad64 instruction word into InstructionIR
fn decode_bad64_to_ir(word: u32, addr: u64) -> EmuResult<InstructionIR> {
    let instr = decode(word, addr)
        .map_err(|e| EmuError::InternalError(format!("Decode error: {:?}", e)))?;

    let opcode = bad64_opcode_to_ir(instr.op())?;

    let operands = match instr.op() {
        Op::B_AL
        | Op::B_CC
        | Op::B_CS
        | Op::B_EQ
        | Op::B_GE
        | Op::B_GT
        | Op::B_HI
        | Op::B_LE
        | Op::B_LS
        | Op::B_LT
        | Op::B_MI
        | Op::B_NE
        | Op::B_NV
        | Op::B_PL
        | Op::B_VC
        | Op::B_VS
        | Op::BL
        | Op::BR
        | Op::BLR
        | Op::CBZ
        | Op::CBNZ
        | Op::TBZ
        | Op::TBNZ => {
            // For bad64, first operand should be the branch target (imm, reg, or address)
            instr
                .operands()
                .iter()
                .map(|op| {
                    match op {
                        // Branch target is an immediate value (address offset or absolute)
                        Bad64Operand::Imm32 { imm, .. } | Bad64Operand::Imm64 { imm, .. } => {
                            let val = extract_imm_value(imm);
                            Ok(Operand::Imm(Immediate::Lit(val)))
                        }
                        // Some branches may take a register (indirect branch)
                        Bad64Operand::Reg { reg, .. } => {
                            let r = bad64_reg_to_ir(*reg)?;
                            Ok(Operand::Reg(r))
                        }
                        // Labels as branch targets
                        Bad64Operand::Label(imm) => {
                            let val = extract_imm_value(imm);
                            Ok(Operand::Imm(Immediate::Lit(val)))
                        }
                        _ => Err(EmuError::InternalError(format!(
                            "Unsupported branch operand form: {:?}",
                            op
                        ))),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        _ => instr
            .operands()
            .iter()
            .map(|op| match op {
                Bad64Operand::Imm32 { imm, shift } => {
                    let (shift_type, amount) = match extract_shift_extend_type_and_amount(shift) {
                        Some((stype, amt)) => (stype, amt),
                        None => {
                            return Ok(Operand::Imm(Immediate::Lit(extract_imm_value(imm))));
                        }
                    };
                    Ok(Operand::ImmWithShift(
                        extract_imm_value(imm),
                        shift_type,
                        amount,
                    ))
                }
                Bad64Operand::Imm64 { imm, shift } => {
                    let (shift_type, amount) = match extract_shift_extend_type_and_amount(shift) {
                        Some((stype, amt)) => (stype, amt),
                        None => {
                            return Ok(Operand::Imm(Immediate::Lit(extract_imm_value(imm))));
                        }
                    };
                    Ok(Operand::ImmWithShift(
                        extract_imm_value(imm),
                        shift_type,
                        amount,
                    ))
                }
                Bad64Operand::FImm32(_fimm) => Err(EmuError::InternalError(
                    "FImm32 operands not supported.".to_string(),
                )),
                Bad64Operand::ShiftReg { reg, shift } => {
                    let base_reg = bad64_reg_to_ir(*reg)?;
                    let (modifier, amount) =
                        match extract_shift_extend_type_and_amount(&Some(*shift)) {
                            Some((stype, amt)) => (Some(stype), amt),
                            None => (None, 0),
                        };
                    Ok(Operand::RegWithMod(Box::new(OperandWithShiftExtend {
                        base: Operand::Reg(base_reg),
                        modifier,
                        amount,
                    })))
                }
                Bad64Operand::QualReg { reg, qual } => Err(EmuError::InternalError(
                    "QualReg operands not yet supported".to_string(),
                )),
                Bad64Operand::Reg { reg, arrspec: _ } => {
                    let r = bad64_reg_to_ir(*reg)?;
                    Ok(Operand::Reg(r))
                }
                Bad64Operand::MultiReg { regs, arrspec: _ } => Err(EmuError::InternalError(
                    "MultiReg operands not yet supported".to_string(),
                )),
                Bad64Operand::SysReg(_sysreg) => Err(EmuError::InternalError(
                    "SysReg operands not supported.".to_string(),
                )),
                Bad64Operand::MemReg(reg) => {
                    let base_reg = bad64_reg_to_ir(*reg)?;
                    Ok(Operand::Offset(Offset::Ind2(base_reg)))
                }
                Bad64Operand::MemOffset {
                    reg,
                    offset,
                    mul_vl: _,
                    arrspec: _,
                } => {
                    let base_reg = bad64_reg_to_ir(*reg)?;
                    let offset = Operand::Imm(Immediate::Lit(extract_imm_value(offset)));
                    Ok(Operand::Offset(Offset::Ind3(
                        base_reg,
                        match offset {
                            Operand::Imm(immediate) => immediate,
                            _ => {
                                return Err(EmuError::InternalError(
                                    "Expected immediate for MemOffset".to_string(),
                                ));
                            }
                        },
                    )))
                }
                Bad64Operand::MemPreIdx { reg, imm } => {
                    let base_reg = bad64_reg_to_ir(*reg)?;
                    let offset = Operand::Imm(Immediate::Lit(extract_imm_value(imm)));
                    Ok(Operand::Offset(Offset::PreIndexed(
                        base_reg,
                        match offset {
                            Operand::Imm(immediate) => immediate,
                            _ => {
                                return Err(EmuError::InternalError(
                                    "Expected immediate for pre-index offset".to_string(),
                                ));
                            }
                        },
                    )))
                }
                Bad64Operand::MemPostIdxReg(regs) => {
                    let base_reg = bad64_reg_to_ir(regs[0])?;
                    let offset_reg = bad64_reg_to_ir(regs[1])?;
                    Ok(Operand::Offset(Offset::PostIndexedReg(
                        base_reg,
                        Box::new(Operand::Reg(offset_reg)),
                    )))
                }
                Bad64Operand::MemPostIdxImm { reg, imm } => {
                    let base_reg = bad64_reg_to_ir(*reg)?;
                    let offset = Operand::Imm(Immediate::Lit(extract_imm_value(imm)));
                    Ok(Operand::Offset(Offset::PostIndexed(
                        base_reg,
                        match offset {
                            Operand::Imm(immediate) => immediate,
                            _ => {
                                return Err(EmuError::InternalError(
                                    "Expected immediate for post-index offset".to_string(),
                                ));
                            }
                        },
                    )))
                }
                Bad64Operand::MemExt {
                    regs,
                    shift,
                    arrspec: _,
                } => {
                    let base_reg = bad64_reg_to_ir(regs[0])?;
                    let offset_operand = if regs.len() > 1 {
                        let offset_reg = bad64_reg_to_ir(regs[1])?;
                        Operand::Reg(offset_reg)
                    } else {
                        Operand::Imm(Immediate::Lit(0))
                    };
                    let (modifier, amount) = match extract_shift_extend_type_and_amount(shift) {
                        Some((stype, amt)) => (Some(stype), amt),
                        None => (None, 0),
                    };
                    let operand_with_mod = OperandWithShiftExtend {
                        base: offset_operand,
                        modifier,
                        amount,
                    };
                    Ok(Operand::Offset(Offset::Ind5(
                        base_reg,
                        Box::new(Operand::RegWithMod(Box::new(operand_with_mod))),
                    )))
                }
                Bad64Operand::SmeTile {
                    tile,
                    slice,
                    arrspec,
                    reg,
                    imm,
                } => Err(EmuError::InternalError(
                    "SmeTile operands not supported.".to_string(),
                )),
                Bad64Operand::AccumArray { reg, imm } => Err(EmuError::InternalError(
                    "AccumArray operands not yet supported".to_string(),
                )),
                Bad64Operand::IndexedElement {
                    regs,
                    arrspec: _,
                    imm,
                } => {
                    let base_reg = bad64_reg_to_ir(regs[0])?;
                    let offset = Operand::Imm(Immediate::Lit(extract_imm_value(imm)));
                    Ok(Operand::Offset(Offset::Ind3(
                        base_reg,
                        match offset {
                            Operand::Imm(immediate) => immediate,
                            _ => {
                                return Err(EmuError::InternalError(
                                    "Expected immediate for IndexedElement".to_string(),
                                ));
                            }
                        },
                    )))
                }
                // Bad64Operand::Label(imm) => Err(EmuError::InternalError(
                //     "Label operands not yet supported".to_string(),
                // )),
                Bad64Operand::Label(imm) => match imm {
                    Imm::Signed(val) => Ok(Operand::Imm(Immediate::Lit(*val))),
                    Imm::Unsigned(val) => Ok(Operand::Imm(Immediate::Lit(*val as i64))),
                },
                Bad64Operand::ImplSpec { o0, o1, cm, cn, o2 } => Err(EmuError::InternalError(
                    "ImplSpec operands not yet supported".to_string(),
                )),
                Bad64Operand::Cond(cond) => Err(EmuError::InternalError(
                    "Cond operands not yet supported".to_string(),
                )),
                Bad64Operand::Name(name) => Err(EmuError::InternalError(
                    "Name operands not yet supported".to_string(),
                )),
                Bad64Operand::StrImm { str, imm } => Err(EmuError::InternalError(
                    "StrImm operands not yet supported".to_string(),
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(InstructionIR { opcode, operands })
}
