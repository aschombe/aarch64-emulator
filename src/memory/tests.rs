// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_back() {
        let mut mem = Memory::new(0x1000);
        mem.write_u64(0x8, 0xBEEFCAFE);
        assert_eq!(mem.read_u64(0x8), 0xBEEFCAFE);
    }
}
