use crate::assembler::asm_types::{AssemblyBlock, AssemblyContent, Data};
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};

pub fn load_data_into_cpu(cpu: &mut CpuState, data_blocks: &[AssemblyBlock]) -> EmuResult<()> {
    for block in data_blocks {
        let base_addr = *cpu.program.label_to_ip.get(&block.label).ok_or_else(|| {
            EmuError::InternalError(format!("Missing address for label '{}'", block.label))
        })?;

        match &block.content {
            AssemblyContent::Data(items) => {
                let mut offset: Word = 0;

                for item in items {
                    match item {
                        Data::QuadArr(vals) => {
                            for v in vals {
                                cpu.memory
                                    .write_bytes(base_addr + offset, &v.to_le_bytes())?;
                                offset += 8;
                            }
                        }
                        Data::Quad(v) => {
                            cpu.memory
                                .write_bytes(base_addr + offset, &v.to_le_bytes())?;
                            offset += 8;
                        }
                        Data::Word(v) => {
                            let val = *v as u32;
                            cpu.memory
                                .write_bytes(base_addr + offset, &val.to_le_bytes())?;
                            offset += 4;
                        }
                        Data::ByteArr(bytes) => {
                            cpu.memory.write_bytes(base_addr + offset, bytes)?;
                            offset += bytes.len() as u64;
                        }
                        Data::Byte(b) => {
                            cpu.memory.write_bytes(base_addr + offset, &[*b])?;
                            offset += 1;
                        }
                        _ => {}
                    }
                }
            }
            AssemblyContent::Bss(size) => {
                // Do not copy bytes for .bss (uninitialized),
                // optionally zero memory if your simulator requires:
                cpu.memory
                    .write_bytes(base_addr, &vec![0u8; *size as usize])?;
            }
            _ => {}
        }
    }

    Ok(())
}
