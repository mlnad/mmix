/// MMIX memory model
///
use std::collections::BTreeMap;

pub struct Memory {
    data: BTreeMap<u64, u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn read_u8(&self, addr: u64) -> u8 {
        *self.data.get(&addr).unwrap_or(&0)
    }

    pub fn write_u8(&mut self, addr: u64, value: u8) {
        self.data.insert(addr, value);
    }

    pub fn read_u64_le(&self, addr: u64) -> u64 {
        let mut acc = 0u64;
        for offset in 0..8 {
            let byte = self.read_u8(addr.wrapping_add(offset));
            acc |= (byte as u64) << (offset * 8);
        }
        acc
    }

    pub fn write_u64_le(&mut self, addr: u64, value: u64) {
        for offset in 0..8 {
            let byte = ((value >> (offset * 8)) & 0xff) as u8;
            self.write_u8(addr.wrapping_add(offset), byte);
        }
    }

    /// Read a 16-bit value in big-endian byte order
    pub fn read_u16(&self, addr: u64) -> u16 {
        let hi = self.read_u8(addr) as u16;
        let lo = self.read_u8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    /// Read a 32-bit value in big-endian byte order
    pub fn read_u32(&self, addr: u64) -> u32 {
        let mut acc = 0u32;
        for i in 0..4u64 {
            acc = (acc << 8) | self.read_u8(addr.wrapping_add(i)) as u32;
        }
        acc
    }

    /// Read a 64-bit value in big-endian byte order
    pub fn read_u64(&self, addr: u64) -> u64 {
        let mut acc = 0u64;
        for i in 0..8u64 {
            acc = (acc << 8) | self.read_u8(addr.wrapping_add(i)) as u64;
        }
        acc
    }

    /// Write a 16-bit value in big-endian byte order
    pub fn write_u16(&mut self, addr: u64, val: u16) {
        self.write_u8(addr, (val >> 8) as u8);
        self.write_u8(addr.wrapping_add(1), val as u8);
    }

    /// Write a 32-bit value in big-endian byte order
    pub fn write_u32(&mut self, addr: u64, val: u32) {
        for i in 0..4u64 {
            self.write_u8(addr.wrapping_add(i), (val >> (24 - i * 8)) as u8);
        }
    }

    /// Write a 64-bit value in big-endian byte order
    pub fn write_u64(&mut self, addr: u64, val: u64) {
        for i in 0..8u64 {
            self.write_u8(addr.wrapping_add(i), (val >> (56 - i * 8)) as u8);
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_uninitialized_returns_zero() {
        let m = Memory::new();
        assert_eq!(m.read_u8(0), 0);
        assert_eq!(m.read_u8(u64::MAX), 0);
        assert_eq!(m.read_u64(0x1000), 0);
    }

    #[test]
    fn byte_roundtrip() {
        let mut m = Memory::new();
        m.write_u8(100, 0xAB);
        assert_eq!(m.read_u8(100), 0xAB);
        assert_eq!(m.read_u8(101), 0);
    }

    #[test]
    fn u16_big_endian() {
        let mut m = Memory::new();
        m.write_u16(0, 0x1234);
        assert_eq!(m.read_u8(0), 0x12);
        assert_eq!(m.read_u8(1), 0x34);
        assert_eq!(m.read_u16(0), 0x1234);
    }

    #[test]
    fn u32_big_endian() {
        let mut m = Memory::new();
        m.write_u32(0, 0x12345678);
        assert_eq!(m.read_u8(0), 0x12);
        assert_eq!(m.read_u8(1), 0x34);
        assert_eq!(m.read_u8(2), 0x56);
        assert_eq!(m.read_u8(3), 0x78);
        assert_eq!(m.read_u32(0), 0x12345678);
    }

    #[test]
    fn u64_big_endian() {
        let mut m = Memory::new();
        m.write_u64(0, 0x0123_4567_89AB_CDEF);
        assert_eq!(m.read_u8(0), 0x01);
        assert_eq!(m.read_u8(7), 0xEF);
        assert_eq!(m.read_u64(0), 0x0123_4567_89AB_CDEF);
    }

    #[test]
    fn u64_le_roundtrip() {
        let mut m = Memory::new();
        m.write_u64_le(0x100, 0x0123_4567_89AB_CDEF);
        assert_eq!(m.read_u64_le(0x100), 0x0123_4567_89AB_CDEF);
        // LE: least significant byte at lowest address
        assert_eq!(m.read_u8(0x100), 0xEF);
        assert_eq!(m.read_u8(0x107), 0x01);
    }

    #[test]
    fn write_overwrite() {
        let mut m = Memory::new();
        m.write_u8(0, 0x11);
        m.write_u8(0, 0x22);
        assert_eq!(m.read_u8(0), 0x22);
    }

    #[test]
    fn u32_sub_read_from_u64() {
        let mut m = Memory::new();
        m.write_u64(0, 0x0102_0304_0506_0708);
        assert_eq!(m.read_u32(0), 0x0102_0304);
        assert_eq!(m.read_u32(4), 0x0506_0708);
        assert_eq!(m.read_u16(2), 0x0304);
    }
}
