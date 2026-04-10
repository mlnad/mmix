/// MMIX register model
use mmix_macros::define_special_registers;

define_special_registers! {
    pub struct SpecialRegisters {
        ra = 21 : "arithmetic status register",
        rb = 0 : "bootstrap register (trip)",
        rc = 8 : "cycle counter",
        rd = 1 : "dividend register",
        re = 2 : "epsilon register",
        rf = 22 : "failure location register",
        rg = 19 : "global threshold register",
        rh = 3 : "himult register",
        ri = 12 : "interval counter",
        rj = 4 : "return-jump register",
        rk = 15 : "interrupt mask register",
        rl = 20 : "local threshold register",
        rm = 5 : "multiplex mask register",
        rn = 9 : "serial number",
        ro = 10 : "register stack offset",
        rp = 23 : "prediction register",
        rq = 16 : "interrupt request register",
        rr = 6 : "remainder register",
        rs = 11 : "register stack pointer",
        rt = 13 : "trap address register",
        ru = 17 : "usage counter",
        rv = 18 : "virtual translation register",
        rw = 24 : "where-interrupted register (trip)",
        rx = 25 : "execution register (trip)",
        ry = 26 : "Y operand (trip)",
        rz = 27 : "Z operand (trip)",
        rbb = 7 : "bootstrap register (trap)",
        rtt = 14 : "dynamic trap address register",
        rww = 28 : "where-interrupted register (dynamic trap)",
        rxx = 29 : "execution register (trap)",
        ryy = 30 : "Y operand (trap)",
        rzz = 31 : "Z operand (trap)",
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
    }

    #[test]
    fn special_reg_count() {
        assert_eq!(SpecialRegister::COUNT, SpecialRegister::ALL.len());
        assert_eq!(SpecialRegister::COUNT, 32);
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
