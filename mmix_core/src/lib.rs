/// MMIX simulator
pub mod execute;
pub mod instruction;
pub mod machine;
pub mod memory;
pub mod register;

pub use instruction::{RawInst, Timing, op, timing, name, NAME_TABLE};
pub use machine::Machine;
pub use memory::Memory;
pub use register::{GeneralRegisters, SpecialRegister, SpecialRegisters};
