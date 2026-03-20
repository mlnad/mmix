/// MMIX register model
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
        rzz = 31 : "Z operand (dynamic trap)",
        rpc = 32 : "Program counter (internal)"
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SpecialRegisters tests ----

    #[test]
    fn special_reg_new_all_zero() {
        let s = SpecialRegisters::new();
        for (reg, val) in s.iter() {
            assert_eq!(val, 0, "register {:?} should be zero", reg);
        }
    }

    #[test]
    fn special_reg_set_get_roundtrip() {
        let mut s = SpecialRegisters::new();
        s.set(SpecialRegister::Ra, 0x1234_5678_9abc_def0);
        assert_eq!(s.get(SpecialRegister::Ra), 0x1234_5678_9abc_def0);
    }

    #[test]
    fn special_reg_set_does_not_affect_others() {
        let mut s = SpecialRegisters::new();
        s.set(SpecialRegister::Rh, 42);
        assert_eq!(s.get(SpecialRegister::Ra), 0);
        assert_eq!(s.get(SpecialRegister::Rh), 42);
    }

    #[test]
    fn special_reg_encoding_roundtrip() {
        for reg in SpecialRegister::ALL {
            let enc = reg.encoding();
            assert_eq!(SpecialRegister::from_encoding(enc), Some(reg));
        }
    }

    #[test]
    fn special_reg_from_encoding_invalid() {
        assert_eq!(SpecialRegister::from_encoding(255), None);
    }

    #[test]
    fn special_reg_name() {
        assert_eq!(SpecialRegister::Ra.name(), "ra");
        assert_eq!(SpecialRegister::Rzz.name(), "rzz");
        assert_eq!(SpecialRegister::Rpc.name(), "rpc");
    }

    #[test]
    fn special_reg_count() {
        assert_eq!(SpecialRegister::COUNT, SpecialRegister::ALL.len());
        assert_eq!(SpecialRegister::COUNT, 33); // 26 + 6 doubled + rpc
    }

    #[test]
    fn special_reg_debug_format() {
        let s = SpecialRegisters::new();
        let dbg = format!("{:?}", s);
        assert!(dbg.contains("SpecialRegisters"));
        assert!(dbg.contains("ra"));
    }

    // ---- GeneralRegisters tests ----

    #[test]
    fn general_reg_new_all_zero() {
        let g = GeneralRegisters::new();
        for i in 0..=255u8 {
            assert_eq!(g.get(i), 0);
        }
    }

    #[test]
    fn general_reg_set_get_roundtrip() {
        let mut g = GeneralRegisters::new();
        g.set(0, 100);
        g.set(255, u64::MAX);
        assert_eq!(g.get(0), 100);
        assert_eq!(g.get(255), u64::MAX);
    }

    #[test]
    fn general_reg_set_does_not_affect_others() {
        let mut g = GeneralRegisters::new();
        g.set(42, 0xdead_beef);
        assert_eq!(g.get(41), 0);
        assert_eq!(g.get(43), 0);
        assert_eq!(g.get(42), 0xdead_beef);
    }

    #[test]
    fn general_reg_iter() {
        let mut g = GeneralRegisters::new();
        g.set(10, 999);
        let found: Vec<_> = g.iter().filter(|(_, v)| *v != 0).collect();
        assert_eq!(found, vec![(10u8, 999u64)]);
    }
}
