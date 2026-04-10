/// MMIX machine model
use crate::{
    memory::Memory,
    register::{GeneralRegisters, SpecialRegisters},
};

pub struct Machine {
    pub special: SpecialRegisters,
    pub general: GeneralRegisters,
    pub memory: Memory,
    pub halted: bool,
    pub output_buffer: Vec<u8>,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            special: SpecialRegisters::new(),
            general: GeneralRegisters::new(),
            memory: Memory::new(),
            halted: false,
            output_buffer: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.special = SpecialRegisters::new();
        self.general = GeneralRegisters::new();
        self.memory = Memory::new();
        self.halted = false;
        self.output_buffer.clear();
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
    use crate::register::SpecialRegister;

    #[test]
    fn new_machine_is_zeroed() {
        let m = Machine::new();
        assert_eq!(m.general.get(0), 0);
        assert_eq!(m.special.get(SpecialRegister::Ra), 0);
        assert_eq!(m.memory.read_byte(0), 0);
        assert!(!m.halted);
    }

    #[test]
    fn reset_clears_state() {
        let mut m = Machine::new();
        m.general.set(1, 999);
        m.special.set(SpecialRegister::Rh, 42);
        m.memory.write_byte(0x100, 0xFF);
        m.halted = true;

        m.reset();

        assert_eq!(m.general.get(1), 0);
        assert_eq!(m.special.get(SpecialRegister::Rh), 0);
        assert_eq!(m.memory.read_byte(0x100), 0);
        assert!(!m.halted);
    }

    #[test]
    fn default_equals_new() {
        let a = Machine::new();
        let b = Machine::default();
        assert_eq!(a.general.get(0), b.general.get(0));
        assert_eq!(a.halted, b.halted);
    }
}
