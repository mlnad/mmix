use std::collections::BTreeMap;

use mmix_macros::define_special_registers;

define_special_registers! {
    pub struct SpecialRegisters {
        ra = 0 : "Arithmetic status register",
        rb = 1 : "Bootstrap register (trap)",
        rc = 2 : "Continuation register",
        rd = 3 : "Dividend register",
        re = 4 : "Epsilon register",
        rf = 5 : "Failure location register",
        rg = 6 : "Global threshold register",
        rh = 7 : "Himult register",
        ri = 8 : "Interval counter",
        rj = 9 : "Return-jump register",
        rk = 10 : "Interrupt mask register",
        rl = 11 : "Local threshold register",
        rm = 12 : "Multiplex mask register",
        rn = 13 : "Serial number",
        ro = 14 : "Register stack offset",
        rp = 15 : "Prediction register",
        rq = 16 : "Interrupt request register",
        rr = 17 : "Remainder register",
        rs = 18 : "Register stack pointer",
        rt = 19 : "Trap address register",
        ru = 20 : "Usage counter",
        rv = 21 : "Virtual translation register",
        rw = 22 : "Where-interrupted register (trip)",
        rx = 23 : "Execution register (trip)",
        ry = 24 : "Y operand (trip)",
        rz = 25 : "Z operand (trip)",
        rbb = 26 : "Bootstrap register (trap)",
        rtt = 27 : "Dynamic trap address register",
        rww = 28 : "Where-interrupted register (dynamic trap)",
        rxx = 29 : "Execution register (dynamic trap)",
        ryy = 30 : "Y operand (dynamic trap)",
        rzz = 31 : "Z operand (dynamic trap)"
    }
}

pub struct GeneralRegisters {
    regs: [u64; 256],
}

impl GeneralRegisters {
    pub fn new() -> Self {
        Self { regs: [0; 256] }
    }

    pub fn get(&self, index: u8) -> u64 {
        self.regs[index as usize]
    }

    pub fn set(&mut self, index: u8, value: u64) {
        self.regs[index as usize] = value;
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, u64)> + '_ {
        (0u16..256).map(|i| (i as u8, self.regs[i as usize]))
    }
}

impl Default for GeneralRegisters {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Memory {
    data: BTreeMap<u64, u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: BTreeMap::new() }
    }

    pub fn read_byte(&self, addr: u64) -> u8 {
        *self.data.get(&addr).unwrap_or(&0)
    }

    pub fn write_byte(&mut self, addr: u64, value: u8) {
        self.data.insert(addr, value);
    }

    pub fn read_u64_le(&self, addr: u64) -> u64 {
        let mut acc = 0u64;
        for offset in 0..8 {
            let byte = self.read_byte(addr.wrapping_add(offset));
            acc |= (byte as u64) << (offset * 8);
        }
        acc
    }

    pub fn write_u64_le(&mut self, addr: u64, value: u64) {
        for offset in 0..8 {
            let byte = ((value >> (offset * 8)) & 0xff) as u8;
            self.write_byte(addr.wrapping_add(offset), byte);
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Machine {
    pub special: SpecialRegisters,
    pub general: GeneralRegisters,
    pub memory: Memory,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            special: SpecialRegisters::new(),
            general: GeneralRegisters::new(),
            memory: Memory::new(),
        }
    }

    pub fn reset(&mut self) {
        self.special = SpecialRegisters::new();
        self.general = GeneralRegisters::new();
        self.memory = Memory::new();
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_registers_roundtrip() {
        let mut s = SpecialRegisters::new();
        s.set(SpecialRegister::Ra, 0x1234);
        assert_eq!(s.get(SpecialRegister::Ra), 0x1234);
    }

    #[test]
    fn special_register_encoding() {
        assert_eq!(SpecialRegister::Ra.encoding(), 0);
        assert_eq!(SpecialRegister::Rzz.encoding(), 31);
        assert_eq!(SpecialRegister::from_encoding(0), Some(SpecialRegister::Ra));
        assert_eq!(SpecialRegister::from_encoding(31), Some(SpecialRegister::Rzz));
        assert_eq!(SpecialRegister::from_encoding(255), None);
    }

    #[test]
    fn general_registers_roundtrip() {
        let mut g = GeneralRegisters::new();
        g.set(42, 0xdead_beef);
        assert_eq!(g.get(42), 0xdead_beef);
    }

    #[test]
    fn memory_roundtrip() {
        let mut m = Memory::new();
        m.write_u64_le(0, 0x0123_4567_89ab_cdef);
        assert_eq!(m.read_u64_le(0), 0x0123_4567_89ab_cdef);
        assert_eq!(m.read_byte(4), 0x67);
    }
}
