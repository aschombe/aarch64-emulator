// use crate::types::{EmuError, EmuResult, MEMORY_SIZE, STACK_START, STACK_TOP, Word};
use crate::types::{
    DATA_BASE, DATA_SIZE, EmuError, EmuResult, HEAP_BASE, HEAP_SIZE, MEMORY_SIZE, RODATA_BASE,
    RODATA_SIZE, STACK_SIZE, STACK_START, TEXT_BASE, TEXT_SIZE, Word,
};
use byteorder::{ByteOrder, LittleEndian};

#[derive(Clone, Debug)]
pub struct MemoryRegion {
    pub name: &'static str,
    pub base: u64,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub ram: Vec<u8>,
    pub regions: Vec<MemoryRegion>,
}

impl Memory {
    pub fn new() -> Self {
        let mut mem = Memory {
            ram: vec![0; MEMORY_SIZE as usize],
            regions: vec![],
        };

        // Define memory regions
        mem.regions.push(MemoryRegion {
            name: "text",
            base: TEXT_BASE,
            size: TEXT_SIZE,
        });
        mem.regions.push(MemoryRegion {
            name: "rodata",
            base: RODATA_BASE,
            size: RODATA_SIZE,
        });
        mem.regions.push(MemoryRegion {
            name: "data",
            base: DATA_BASE,
            size: DATA_SIZE,
        });
        mem.regions.push(MemoryRegion {
            name: "heap",
            base: HEAP_BASE,
            size: HEAP_SIZE,
        });
        mem.regions.push(MemoryRegion {
            name: "stack",
            base: STACK_START,
            size: STACK_SIZE,
        });

        mem
    }

    // In Memory implementation
    pub fn read_c_string(&self, addr: Word) -> Result<String, EmuError> {
        let mut buf = Vec::new();
        let mut cur_addr = addr;
        loop {
            let b = self.read_byte(cur_addr)?;
            if b == 0 {
                break;
            }
            buf.push(b);
            cur_addr += 1;
        }
        String::from_utf8(buf).map_err(|e| EmuError::InternalError(format!("UTF8 Error: {}", e)))
    }

    /// Given an address range, find the region name it belongs to if any
    pub fn find_region(&self, base_addr: u64, size: u64) -> Option<&'static str> {
        for region in &self.regions {
            if base_addr >= region.base && (base_addr + size) <= (region.base + region.size) {
                return Some(region.name);
            }
        }
        None
    }

    /// Add a dynamic region linked by name
    pub fn add_dynamic_region(&mut self, name: &'static str, base: u64, size: u64) {
        self.regions.push(MemoryRegion { name, base, size });
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

    pub fn data_section(&self) -> Option<&[u8]> {
        self.get_region_slice("data")
    }

    pub fn rodata_section(&self) -> Option<&[u8]> {
        self.get_region_slice("rodata")
    }

    pub fn text_section(&self) -> Option<&[u8]> {
        self.get_region_slice("text")
    }

    pub fn stack_section(&self) -> Option<&[u8]> {
        self.get_region_slice("stack")
    }

    pub fn heap_section(&self) -> Option<&[u8]> {
        self.get_region_slice("heap")
    }

    fn get_region_slice(&self, name: &str) -> Option<&[u8]> {
        self.regions.iter().find(|r| r.name == name).and_then(|r| {
            let start = r.base as usize;
            let end = start + r.size as usize;
            self.ram.get(start..end)
        })
    }

    /// Helper to get the byte slice for a given address and length, checking bounds
    fn get_slice_mut(&mut self, addr: Word, len: usize) -> EmuResult<&mut [u8]> {
        self.check_bounds(addr, len)?;
        let start_index = addr as usize;
        let end_index = start_index
            .checked_add(len)
            .ok_or_else(|| EmuError::MemoryAccessViolation(addr))?;
        Ok(&mut self.ram[start_index..end_index])
    }

    /// Helper to get an immutable byte slice, checking bounds
    fn get_slice(&self, addr: Word, len: usize) -> EmuResult<&[u8]> {
        self.check_bounds(addr, len)?;
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
