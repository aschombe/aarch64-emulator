use crate::types::{EmuError, EmuResult, MEMORY_SIZE, STACK_START, STACK_TOP, Word};
use byteorder::{ByteOrder, LittleEndian};

#[derive(Clone, Debug)]
pub struct Memory {
    pub ram: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            ram: vec![0; MEMORY_SIZE as usize],
        }
    }

    fn check_bounds(&self, addr: u64, size: usize) -> EmuResult<()> {
        if addr
            .checked_add(size as u64)
            .filter(|&end| end <= MEMORY_SIZE)
            .is_none()
        {
            return Err(EmuError::MemoryAccessViolation(addr));
        }
        Ok(())
    }

    fn check_stack_bounds(&self, addr: u64, size: usize) -> EmuResult<()> {
        if addr < STACK_START
            || addr
                .checked_add(size as u64)
                .map_or(true, |end| end > STACK_TOP)
        {
            return Err(EmuError::StackSmashDetected(addr));
        }
        Ok(())
    }

    /// Helper to get the byte slice for a given address and length, checking bounds
    fn get_slice_mut(&mut self, addr: Word, len: usize) -> EmuResult<&mut [u8]> {
        self.check_bounds(addr, len)?;
        // self.check_stack_bounds(addr, len)?;
        let start_index = addr as usize;
        let end_index = start_index
            .checked_add(len)
            .ok_or_else(|| EmuError::MemoryAccessViolation(addr))?;
        Ok(&mut self.ram[start_index..end_index])
    }

    /// Helper to get an immutable byte slice, checking bounds
    fn get_slice(&self, addr: Word, len: usize) -> EmuResult<&[u8]> {
        self.check_bounds(addr, len)?;
        // self.check_stack_bounds(addr, len)?;
        let start_index = addr as usize;
        let end_index = start_index
            .checked_add(len)
            .ok_or_else(|| EmuError::MemoryAccessViolation(addr))?;
        Ok(&self.ram[start_index..end_index])
    }

    /// Writes a slice of bytes to the specified virtual address
    pub fn write_bytes(&mut self, addr: Word, data: &[u8]) -> EmuResult<()> {
        let len = data.len();
        let slice = self.get_slice_mut(addr, len)?;
        slice.copy_from_slice(data);
        Ok(())
    }

    /// Writes a single byte to the specified virtual address
    pub fn write_byte(&mut self, addr: Word, byte: u8) -> EmuResult<()> {
        let slice = self.get_slice_mut(addr, 1)?;
        slice[0] = byte;
        Ok(())
    }

    /// Reads a slice of bytes from the specified virtual address
    pub fn read_bytes(&self, addr: Word, len: usize) -> EmuResult<&[u8]> {
        self.get_slice(addr, len)
    }

    /// Reads a single byte from the specified virtual address
    pub fn read_byte(&self, addr: Word) -> EmuResult<u8> {
        let bytes = self.get_slice(addr, 1)?;
        Ok(bytes[0])
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
