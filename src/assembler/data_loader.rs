use crate::assembler::asm_types::{AssemblyBlock, AssemblyContent, Data};
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};

fn align_up(val: Word, align: Word) -> Word {
    if align == 0 {
        val
    } else {
        (val + align - 1) & !(align - 1)
    }
}

pub fn load_data_into_cpu(cpu: &mut CpuState, data_blocks: &[AssemblyBlock]) -> EmuResult<()> {
    for block in data_blocks {
        let base_addr = *cpu.program.label_to_ip.get(&block.label).ok_or_else(|| {
            EmuError::InternalError(format!("Missing address for label '{}'", block.label))
        })?;
        let mut offset: Word = 0;

        match &block.content {
            AssemblyContent::Data(items) => {
                for item in items {
                    // align offset based on data type
                    offset = match item {
                        Data::Quad(_) | Data::QuadArr(_) => align_up(offset, 8),
                        Data::Word(_) | Data::IntArr(_) => align_up(offset, 4),
                        _ => align_up(offset, 1),
                    };

                    match item {
                        Data::QuadArr(vals) => {
                            for v in vals {
                                cpu.memory
                                    .borrow_mut()
                                    .write_bytes(base_addr + offset, &v.to_le_bytes())?;
                                offset += 8;
                            }
                        }
                        Data::Quad(v) => {
                            cpu.memory
                                .borrow_mut()
                                .write_bytes(base_addr + offset, &v.to_le_bytes())?;
                            offset += 8;
                        }
                        Data::Word(v) => {
                            let val = *v as u32;
                            cpu.memory
                                .borrow_mut()
                                .write_bytes(base_addr + offset, &val.to_le_bytes())?;
                            offset += 4;
                        }
                        Data::IntArr(vals) => {
                            for v in vals {
                                let val = *v as u32;
                                cpu.memory
                                    .borrow_mut()
                                    .write_bytes(base_addr + offset, &val.to_le_bytes())?;
                                offset += 4;
                            }
                        }
                        Data::ByteArr(bytes) => {
                            cpu.memory
                                .borrow_mut()
                                .write_bytes(base_addr + offset, bytes)?;
                            offset += bytes.len() as u64;
                        }
                        Data::Byte(b) => {
                            cpu.memory
                                .borrow_mut()
                                .write_bytes(base_addr + offset, &[*b])?;
                            offset += 1;
                        }
                        _ => {}
                    }
                }

                let region_name = cpu
                    .memory
                    .borrow()
                    .find_region(base_addr, offset)
                    .unwrap_or("unknown");
                cpu.memory
                    .borrow_mut()
                    .add_dynamic_region(region_name, base_addr, offset);
            }
            AssemblyContent::Bss(size) => {
                cpu.memory
                    .borrow_mut()
                    .write_bytes(base_addr, &vec![0u8; *size as usize])?;
                let region_name = cpu
                    .memory
                    .borrow()
                    .find_region(base_addr, *size)
                    .unwrap_or("unknown");
                cpu.memory
                    .borrow_mut()
                    .add_dynamic_region(region_name, base_addr, *size);
            }
            _ => {}
        }
    }
    Ok(())
}
