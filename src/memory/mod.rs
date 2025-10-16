use crate::types::{EmuError, EmuResult, Word};
use byteorder::{ByteOrder, LittleEndian};

const MEMORY_SIZE: usize = 0x4000_0000; // 1 GB RAM (2^30 bytes)

pub struct Memory {
    pub ram: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            ram: vec![0; MEMORY_SIZE],
        }
    }

    /// Helper to get the byte slice for a given address and length, checking bounds
    fn get_slice_mut(&mut self, addr: Word, len: usize) -> EmuResult<&mut [u8]> {
        let start_index = addr as usize;
        let end_index = start_index
            .checked_add(len)
            .ok_or_else(|| EmuError::MemoryAccessViolation(addr))?;

        if end_index > MEMORY_SIZE {
            return Err(EmuError::MemoryAccessViolation(addr + len as Word));
        }

        Ok(&mut self.ram[start_index..end_index])
    }

    /// Helper to get an immutable byte slice, checking bounds
    fn get_slice(&self, addr: Word, len: usize) -> EmuResult<&[u8]> {
        let start_index = addr as usize;
        let end_index = start_index
            .checked_add(len)
            .ok_or_else(|| EmuError::MemoryAccessViolation(addr))?;

        if end_index > MEMORY_SIZE {
            return Err(EmuError::MemoryAccessViolation(addr + len as Word));
        }

        Ok(&self.ram[start_index..end_index])
    }

    /// Writes a slice of bytes to the specified virtual address
    pub fn write_bytes(&mut self, addr: Word, data: &[u8]) -> EmuResult<()> {
        let len = data.len();
        let slice = self.get_slice_mut(addr, len)?;
        slice.copy_from_slice(data);
        Ok(())
    }

    /// Reads a slice of bytes from the specified virtual address
    pub fn read_bytes(&self, addr: Word, len: usize) -> EmuResult<&[u8]> {
        self.get_slice(addr, len)
    }

    /// Reads a 64-bit Word (8 bytes) from memory at the specified address (Little-Endian)
    pub fn read_word(&self, addr: Word) -> EmuResult<Word> {
        let bytes = self.get_slice(addr, 8)?;

        // Little-Endian conversion from 8 bytes to a u64
        Ok(LittleEndian::read_u64(bytes))
    }

    /// Writes a 64-bit Word (8 bytes) to memory at the specified address (Little-Endian)
    pub fn write_word(&mut self, addr: Word, val: Word) -> EmuResult<()> {
        let bytes = self.get_slice_mut(addr, 8)?;

        // Little-Endian conversion from u64 to 8 bytes
        LittleEndian::write_u64(bytes, val);
        Ok(())
    }
}
