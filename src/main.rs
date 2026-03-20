use mmix::{Machine, SpecialRegister};

fn main() {
    let mut machine = Machine::new();

    machine.special.set(SpecialRegister::Ra, 0x1234);
    machine.general.set(0, 0x1);
    machine.memory.write_u64(0, 0xdead_beef_cafe_babe);

    println!("Special registers: {:?}", machine.special);
    println!("General r0: {:#x}", machine.general.get(0));
    println!("Mem[0..8]: {:#x}", machine.memory.read_u64(0));
}
