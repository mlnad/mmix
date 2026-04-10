/// MMIX executor
use crate::{
    instruction::RawInst,
    opcodes::op,
    machine::Machine,
    register::SpecialRegister,
};

impl Machine {
    /// 取指 → 解码 → 执行，返回 Ok(()) 或错误
    pub fn step(&mut self) -> Result<(), String> {
        if self.halted {
            return Err("Machine halted".into());
        }

        let pc = self.pc;
        let word = self.memory.read_u32(pc);
        let inst = RawInst::decode(word);
        let mut next_pc = pc.wrapping_add(4);

        // 辅助：取第二操作数（寄存器或立即数，奇数 opcode = 立即数）
        let is_imm = inst.op & 1 != 0;

        match inst.op {
            // ---- 有符号算术 ----
            op::ADD | op::ADDI => {
                let a = self.general.get(inst.y);
                let b = if is_imm {
                    inst.z as u64
                } else {
                    self.general.get(inst.z)
                };
                self.general.set(inst.x, a.wrapping_add(b));
            }
            op::SUB | op::SUBI => {
                let a = self.general.get(inst.y);
                let b = if is_imm {
                    inst.z as u64
                } else {
                    self.general.get(inst.z)
                };
                self.general.set(inst.x, a.wrapping_sub(b));
            }
            op::MUL | op::MULI => {
                let a = self.general.get(inst.y) as i64;
                let b = if is_imm {
                    inst.z as i64
                } else {
                    self.general.get(inst.z) as i64
                };
                self.general.set(inst.x, a.wrapping_mul(b) as u64);
            }
            op::DIV | op::DIVI => {
                let a = self.general.get(inst.y) as i64;
                let b = if is_imm {
                    inst.z as i64
                } else {
                    self.general.get(inst.z) as i64
                };
                if b == 0 {
                    self.general.set(inst.x, 0);
                    self.special.set(SpecialRegister::Rr, a as u64);
                } else {
                    self.general.set(inst.x, (a / b) as u64);
                    self.special.set(SpecialRegister::Rr, (a % b) as u64);
                }
            }
            op::CMP | op::CMPI => {
                let a = self.general.get(inst.y) as i64;
                let b = if is_imm {
                    inst.z as i64
                } else {
                    self.general.get(inst.z) as i64
                };
                let result = if a < b {
                    -1i64 as u64
                } else if a > b {
                    1
                } else {
                    0
                };
                self.general.set(inst.x, result);
            }
            op::NEG | op::NEGI => {
                let b = if is_imm {
                    inst.z as u64
                } else {
                    self.general.get(inst.z)
                };
                let y_val = inst.y as u64;
                self.general.set(inst.x, y_val.wrapping_sub(b));
            }

            // ---- 无符号算术 ----
            op::ADDU | op::ADDUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a.wrapping_add(b));
            }
            op::SUBU | op::SUBUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a.wrapping_sub(b));
            }
            op::MULU | op::MULUI => {
                let a = self.general.get(inst.y) as u128;
                let b = if is_imm { inst.z as u128 } else { self.general.get(inst.z) as u128 };
                let product = a * b;
                self.general.set(inst.x, product as u64);
                self.special.set(SpecialRegister::Rh, (product >> 64) as u64);
            }
            op::DIVU | op::DIVUI => {
                let d = self.special.get(SpecialRegister::Rd);
                let a_hi = self.special.get(SpecialRegister::Rh);
                let a_lo = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                if b == 0 || a_hi >= b {
                    // 按 Knuth 规定：若 rD >= b 则商=rD，余=a
                    self.general.set(inst.x, d);
                    self.special.set(SpecialRegister::Rr, a_lo);
                } else {
                    let dividend = ((a_hi as u128) << 64) | (a_lo as u128);
                    let divisor = b as u128;
                    self.general.set(inst.x, (dividend / divisor) as u64);
                    self.special.set(SpecialRegister::Rr, (dividend % divisor) as u64);
                }
            }
            op::CMPU | op::CMPUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                let result = if a < b { -1i64 as u64 } else if a > b { 1 } else { 0 };
                self.general.set(inst.x, result);
            }
            op::NEGU | op::NEGUI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                let y_val = inst.y as u64;
                self.general.set(inst.x, y_val.wrapping_sub(b));
            }

            // ---- 缩放加法 ----
            op::_2ADDU | op::_2ADDUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a.wrapping_mul(2).wrapping_add(b));
            }
            op::_4ADDU | op::_4ADDUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a.wrapping_mul(4).wrapping_add(b));
            }
            op::_8ADDU | op::_8ADDUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a.wrapping_mul(8).wrapping_add(b));
            }
            op::_16ADDU | op::_16ADDUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a.wrapping_mul(16).wrapping_add(b));
            }

            // ---- 移位 ----
            op::SL | op::SLI => {
                let a = self.general.get(inst.y);
                let b = if is_imm {
                    inst.z as u64
                } else {
                    self.general.get(inst.z)
                };
                self.general.set(inst.x, a.wrapping_shl(b as u32));
            }
            op::SLU | op::SLUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if b >= 64 { 0 } else { a << b });
            }
            op::SR | op::SRI => {
                let a = self.general.get(inst.y) as i64;
                let b = if is_imm {
                    inst.z as u64
                } else {
                    self.general.get(inst.z)
                };
                self.general.set(inst.x, (a >> (b & 63)) as u64);
            }
            op::SRU | op::SRUI => {
                let a = self.general.get(inst.y);
                let b = if is_imm {
                    inst.z as u64
                } else {
                    self.general.get(inst.z)
                };
                self.general.set(inst.x, a.wrapping_shr(b as u32));
            }

            // ---- 逻辑 ----
            op::AND | op::ANDI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a & b);
            }
            op::OR | op::ORI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a | b);
            }
            op::XOR | op::XORI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a ^ b);
            }
            op::ORN | op::ORNI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a | !b);
            }
            op::NOR | op::NORI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, !(a | b));
            }
            op::ANDN | op::ANDNI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, a & !b);
            }
            op::NAND | op::NANDI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, !(a & b));
            }
            op::NXOR | op::NXORI => {
                let a = self.general.get(inst.y);
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, !(a ^ b));
            }

            // ---- 内存加载 ----
            op::LDB | op::LDBI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u8(addr) as i8 as i64 as u64;
                self.general.set(inst.x, val);
            }
            op::LDBU | op::LDBUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u8(addr) as u64;
                self.general.set(inst.x, val);
            }
            op::LDW | op::LDWI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u16(addr) as i16 as i64 as u64;
                self.general.set(inst.x, val);
            }
            op::LDWU | op::LDWUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u16(addr) as u64;
                self.general.set(inst.x, val);
            }
            op::LDT | op::LDTI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u32(addr) as i32 as i64 as u64;
                self.general.set(inst.x, val);
            }
            op::LDTU | op::LDTUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u32(addr) as u64;
                self.general.set(inst.x, val);
            }
            op::LDO | op::LDOI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u64(addr);
                self.general.set(inst.x, val);
            }
            op::LDOU | op::LDOUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                let val = self.memory.read_u64(addr);
                self.general.set(inst.x, val);
            }

            // ---- 内存存储 ----
            op::STB | op::STBI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u8(addr, self.general.get(inst.x) as u8);
            }
            op::STBU | op::STBUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u8(addr, self.general.get(inst.x) as u8);
            }
            op::STW | op::STWI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u16(addr, self.general.get(inst.x) as u16);
            }
            op::STWU | op::STWUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u16(addr, self.general.get(inst.x) as u16);
            }
            op::STT | op::STTI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u32(addr, self.general.get(inst.x) as u32);
            }
            op::STTU | op::STTUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u32(addr, self.general.get(inst.x) as u32);
            }
            op::STO | op::STOI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u64(addr, self.general.get(inst.x));
            }
            op::STOU | op::STOUI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.memory.write_u64(addr, self.general.get(inst.x));
            }

            // ---- 常量加载 ----
            op::SETH => self.general.set(inst.x, (inst.yz() as u64) << 48),
            op::SETMH => self.general.set(inst.x, (inst.yz() as u64) << 32),
            op::SETML => self.general.set(inst.x, (inst.yz() as u64) << 16),
            op::SETL => self.general.set(inst.x, inst.yz() as u64),
            op::INCH => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v.wrapping_add((inst.yz() as u64) << 48));
            }
            op::INCMH => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v.wrapping_add((inst.yz() as u64) << 32));
            }
            op::INCML => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v.wrapping_add((inst.yz() as u64) << 16));
            }
            op::INCL => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v.wrapping_add(inst.yz() as u64));
            }
            op::ORH => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v | ((inst.yz() as u64) << 48));
            }
            op::ORMH => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v | ((inst.yz() as u64) << 32));
            }
            op::ORML => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v | ((inst.yz() as u64) << 16));
            }
            op::ORL => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v | (inst.yz() as u64));
            }
            op::ANDNH => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v & !((inst.yz() as u64) << 48));
            }
            op::ANDNMH => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v & !((inst.yz() as u64) << 32));
            }
            op::ANDNML => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v & !((inst.yz() as u64) << 16));
            }
            op::ANDNL => {
                let v = self.general.get(inst.x);
                self.general.set(inst.x, v & !(inst.yz() as u64));
            }

            // ---- 分支 ----
            op::BN | op::BNB => {
                if (self.general.get(inst.x) as i64) < 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BZ | op::BZB => {
                if self.general.get(inst.x) == 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BP | op::BPB => {
                if (self.general.get(inst.x) as i64) > 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BOD | op::BODB => {
                if self.general.get(inst.x) & 1 != 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BNN | op::BNNB => {
                if (self.general.get(inst.x) as i64) >= 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BNZ | op::BNZB => {
                if self.general.get(inst.x) != 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BNP | op::BNPB => {
                if (self.general.get(inst.x) as i64) <= 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::BEV | op::BEVB => {
                if self.general.get(inst.x) & 1 == 0 {
                    let offset = self.branch_offset(&inst);
                    next_pc = (pc as i64).wrapping_add(offset) as u64;
                }
            }
            op::JMP | op::JMPB => {
                let offset = self.branch_offset(&inst);
                next_pc = (pc as i64).wrapping_add(offset) as u64;
            }
            op::GO | op::GOI => {
                let addr = self.calc_addr(inst.y, inst.z, is_imm);
                self.general.set(inst.x, next_pc);
                next_pc = addr;
            }

            // ---- GETA ----
            op::GETA | op::GETAB => {
                let offset = self.branch_offset(&inst);
                self.general
                    .set(inst.x, (pc as i64).wrapping_add(offset) as u64);
            }

            // ---- 条件赋值 CSxx: if cond then $X ← $Z/$Z ----
            op::CSN | op::CSNI => {
                if (self.general.get(inst.y) as i64) < 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSZ | op::CSZI => {
                if self.general.get(inst.y) == 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSP | op::CSPI => {
                if (self.general.get(inst.y) as i64) > 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSOD | op::CSODI => {
                if self.general.get(inst.y) & 1 != 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSNN | op::CSNNI => {
                if (self.general.get(inst.y) as i64) >= 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSNZ | op::CSNZI => {
                if self.general.get(inst.y) != 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSNP | op::CSNPI => {
                if (self.general.get(inst.y) as i64) <= 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }
            op::CSEV | op::CSEVI => {
                if self.general.get(inst.y) & 1 == 0 {
                    let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                    self.general.set(inst.x, b);
                }
            }

            // ---- 零或赋值 ZSxx: $X ← (cond ? $Z/Z : 0) ----
            op::ZSN | op::ZSNI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if (self.general.get(inst.y) as i64) < 0 { b } else { 0 });
            }
            op::ZSZ | op::ZSZI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if self.general.get(inst.y) == 0 { b } else { 0 });
            }
            op::ZSP | op::ZSPI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if (self.general.get(inst.y) as i64) > 0 { b } else { 0 });
            }
            op::ZSOD | op::ZSODI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if self.general.get(inst.y) & 1 != 0 { b } else { 0 });
            }
            op::ZSNN | op::ZSNNI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if (self.general.get(inst.y) as i64) >= 0 { b } else { 0 });
            }
            op::ZSNZ | op::ZSNZI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if self.general.get(inst.y) != 0 { b } else { 0 });
            }
            op::ZSNP | op::ZSNPI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if (self.general.get(inst.y) as i64) <= 0 { b } else { 0 });
            }
            op::ZSEV | op::ZSEVI => {
                let b = if is_imm { inst.z as u64 } else { self.general.get(inst.z) };
                self.general.set(inst.x, if self.general.get(inst.y) & 1 == 0 { b } else { 0 });
            }

            // ---- GET/PUT ----
            op::GET => {
                if let Some(sr) = SpecialRegister::from_encoding(inst.z) {
                    self.general.set(inst.x, self.special.get(sr));
                } else {
                    return Err(format!("GET: invalid special register {}", inst.z));
                }
            }
            op::PUT | op::PUTI => {
                if let Some(sr) = SpecialRegister::from_encoding(inst.x) {
                    let val = if is_imm {
                        inst.z as u64
                    } else {
                        self.general.get(inst.z)
                    };
                    self.special.set(sr, val);
                } else {
                    return Err(format!("PUT: invalid special register {}", inst.x));
                }
            }

            // ---- TRAP ----
            op::TRAP => {
                self.handle_trap(inst.x, inst.y, inst.z)?;
            }

            _ => {
                return Err(format!(
                    "Unimplemented opcode: {:#04x} at PC={:#x}",
                    inst.op, pc
                ));
            }
        }

        let t = crate::opcodes::timing(inst.op);
        self.oops += t.v;
        self.mems += t.mu;

        self.pc = next_pc;
        Ok(())
    }

    fn calc_addr(&self, y: u8, z: u8, is_imm: bool) -> u64 {
        let base = self.general.get(y);
        let offset = if is_imm {
            z as u64
        } else {
            self.general.get(z)
        };
        base.wrapping_add(offset)
    }

    fn branch_offset(&self, inst: &RawInst) -> i64 {
        let raw = inst.yz() as u64;
        let is_backward = inst.op & 1 != 0;
        if is_backward {
            -((0x10000 - raw as i64) * 4)
        } else {
            (raw as i64) * 4
        }
    }

    fn handle_trap(&mut self, x: u8, y: u8, z: u8) -> Result<(), String> {
        match (x, y, z) {
            // TRAP 0,0,0 — Halt
            (0, 0, 0) => {
                self.halted = true;
                Ok(())
            }
            // TRAP 0,1,1 — Fputs to stdout (simplified)
            (0, 1, 1) => {
                let mut addr = self.general.get(255);
                loop {
                    let ch = self.memory.read_u8(addr);
                    if ch == 0 {
                        break;
                    }
                    self.output_buffer.push(ch);
                    addr = addr.wrapping_add(1);
                }
                Ok(())
            }
            _ => Err(format!("Unimplemented TRAP {},{},{}", x, y, z)),
        }
    }

    /// Load raw bytes into memory at the given base address
    pub fn load_raw(&mut self, base_addr: u64, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            self.memory.write_u8(base_addr.wrapping_add(i as u64), b);
        }
    }

    /// Set the program entry point (PC)
    pub fn set_entry(&mut self, addr: u64) {
        self.pc = addr;
    }

    pub fn run(&mut self) -> Result<(), String> {
        while !self.halted {
            self.step()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::RawInst;
    use crate::opcodes::op;

    /// Helper: create a machine, write one instruction at addr 0, set PC=0, step once
    fn exec_one(opcode: u8, x: u8, y: u8, z: u8) -> Machine {
        let mut m = Machine::new();
        let word = RawInst {
            op: opcode,
            x,
            y,
            z,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        m
    }

    /// Helper: write multiple instructions starting at addr, set PC=addr, run until halt or error
    fn exec_program(base: u64, insts: &[(u8, u8, u8, u8)]) -> Machine {
        let mut m = Machine::new();
        for (i, &(opcode, x, y, z)) in insts.iter().enumerate() {
            let word = RawInst {
                op: opcode,
                x,
                y,
                z,
            }
            .encode();
            m.memory.write_u32(base + (i as u64) * 4, word);
        }
        m.set_entry(base);
        m
    }

    // ==== Arithmetic ====

    #[test]
    fn add_reg_reg() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        m.general.set(2, 20);
        // ADD $0, $1, $2
        let word = RawInst {
            op: op::ADD,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 30);
    }

    #[test]
    fn add_immediate() {
        let mut m = Machine::new();
        m.general.set(1, 100);
        // ADDI $0, $1, 55
        let word = RawInst {
            op: op::ADDI,
            x: 0,
            y: 1,
            z: 55,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 155);
    }

    #[test]
    fn add_wrapping() {
        let mut m = Machine::new();
        m.general.set(1, u64::MAX);
        m.general.set(2, 1);
        let word = RawInst {
            op: op::ADD,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0); // wrapping
    }

    #[test]
    fn sub_reg_reg() {
        let mut m = Machine::new();
        m.general.set(1, 50);
        m.general.set(2, 20);
        let word = RawInst {
            op: op::SUB,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 30);
    }

    #[test]
    fn sub_immediate() {
        let mut m = Machine::new();
        m.general.set(1, 100);
        let word = RawInst {
            op: op::SUBI,
            x: 0,
            y: 1,
            z: 10,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 90);
    }

    #[test]
    fn mul_positive() {
        let mut m = Machine::new();
        m.general.set(1, 6);
        m.general.set(2, 7);
        let word = RawInst {
            op: op::MUL,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 42);
    }

    #[test]
    fn mul_immediate() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        let word = RawInst {
            op: op::MULI,
            x: 0,
            y: 1,
            z: 5,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 50);
    }

    #[test]
    fn div_positive() {
        let mut m = Machine::new();
        m.general.set(1, 42);
        m.general.set(2, 5);
        let word = RawInst {
            op: op::DIV,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 8);
        assert_eq!(m.special.get(SpecialRegister::Rr), 2); // remainder
    }

    #[test]
    fn div_by_zero() {
        let mut m = Machine::new();
        m.general.set(1, 42);
        m.general.set(2, 0);
        let word = RawInst {
            op: op::DIV,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0);
        assert_eq!(m.special.get(SpecialRegister::Rr), 42);
    }

    #[test]
    fn cmp_less() {
        let mut m = Machine::new();
        m.general.set(1, 5);
        m.general.set(2, 10);
        let word = RawInst {
            op: op::CMP,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0) as i64, -1);
    }

    #[test]
    fn cmp_equal() {
        let mut m = Machine::new();
        m.general.set(1, 7);
        m.general.set(2, 7);
        let word = RawInst {
            op: op::CMP,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0);
    }

    #[test]
    fn cmp_greater() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        m.general.set(2, 5);
        let word = RawInst {
            op: op::CMP,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 1);
    }

    #[test]
    fn neg() {
        let m = exec_one(op::NEG, 0, 0, 1); // NEG $0, 0, $1 — but $1=0, so result = 0-0 = 0
        assert_eq!(m.general.get(0), 0);
    }

    // ==== Shift ====

    #[test]
    fn shift_left() {
        let mut m = Machine::new();
        m.general.set(1, 1);
        m.general.set(2, 4);
        let word = RawInst {
            op: op::SL,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 16);
    }

    #[test]
    fn shift_left_imm() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF);
        let word = RawInst {
            op: op::SLI,
            x: 0,
            y: 1,
            z: 8,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0xFF00);
    }

    #[test]
    fn shift_right_arithmetic() {
        let mut m = Machine::new();
        m.general.set(1, (-16i64) as u64); // negative number
        let word = RawInst {
            op: op::SRI,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0) as i64, -4); // arithmetic shift preserves sign
    }

    #[test]
    fn shift_right_unsigned() {
        let mut m = Machine::new();
        m.general.set(1, 0x8000_0000_0000_0000);
        let word = RawInst {
            op: op::SRUI,
            x: 0,
            y: 1,
            z: 1,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0x4000_0000_0000_0000);
    }

    // ==== Logic ====

    #[test]
    fn and_reg() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF00);
        m.general.set(2, 0x0FF0);
        let word = RawInst {
            op: op::AND,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0x0F00);
    }

    #[test]
    fn or_reg() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF00);
        m.general.set(2, 0x00FF);
        let word = RawInst {
            op: op::OR,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0xFFFF);
    }

    #[test]
    fn xor_reg() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF00);
        m.general.set(2, 0xFFFF);
        let word = RawInst {
            op: op::XOR,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0x00FF);
    }

    // ==== Memory load/store ====

    #[test]
    fn ldo_sto_roundtrip() {
        let mut m = Machine::new();
        m.general.set(0, 0xDEAD_BEEF_CAFE_BABE);
        m.general.set(1, 0x1000); // base address
        // STO $0, $1, 0
        let sto = RawInst {
            op: op::STOI,
            x: 0,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, sto);
        // LDO $2, $1, 0
        let ldo = RawInst {
            op: op::LDOI,
            x: 2,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(4, ldo);
        m.set_entry(0);
        m.step().unwrap(); // STO
        m.step().unwrap(); // LDO
        assert_eq!(m.general.get(2), 0xDEAD_BEEF_CAFE_BABE);
    }

    #[test]
    fn stb_ldb_signed() {
        let mut m = Machine::new();
        m.general.set(0, 0xFF); // will be stored as byte 0xFF = -1 signed
        m.general.set(1, 0x2000);
        let stb = RawInst {
            op: op::STBI,
            x: 0,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, stb);
        let ldb = RawInst {
            op: op::LDBI,
            x: 2,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(4, ldb);
        m.set_entry(0);
        m.step().unwrap();
        m.step().unwrap();
        // LDB sign-extends: 0xFF -> -1 -> 0xFFFF_FFFF_FFFF_FFFF
        assert_eq!(m.general.get(2) as i64, -1);
    }

    #[test]
    fn stb_ldbu_unsigned() {
        let mut m = Machine::new();
        m.general.set(0, 0xFF);
        m.general.set(1, 0x2000);
        let stb = RawInst {
            op: op::STBI,
            x: 0,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, stb);
        let ldbu = RawInst {
            op: op::LDBUI,
            x: 2,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(4, ldbu);
        m.set_entry(0);
        m.step().unwrap();
        m.step().unwrap();
        assert_eq!(m.general.get(2), 0xFF); // zero-extended
    }

    // ==== Constant loading ====

    #[test]
    fn setl() {
        let m = exec_one(op::SETL, 0, 0x12, 0x34);
        assert_eq!(m.general.get(0), 0x1234);
    }

    #[test]
    fn seth() {
        let m = exec_one(op::SETH, 0, 0x00, 0x01);
        assert_eq!(m.general.get(0), 0x0001_0000_0000_0000);
    }

    #[test]
    fn setmh() {
        let m = exec_one(op::SETMH, 0, 0x00, 0x01);
        assert_eq!(m.general.get(0), 0x0000_0001_0000_0000);
    }

    #[test]
    fn setml() {
        let m = exec_one(op::SETML, 0, 0x00, 0x01);
        assert_eq!(m.general.get(0), 0x0000_0000_0001_0000);
    }

    #[test]
    fn set_full_64bit_via_seth_ormh_orml_orl() {
        // Build 0xDEAD_BEEF_CAFE_BABE using four instructions
        let mut m = exec_program(
            0,
            &[
                (op::SETH, 0, 0xDE, 0xAD), // $0 = 0xDEAD_0000_0000_0000
                (op::ORMH, 0, 0xBE, 0xEF), // $0 |= 0x0000_BEEF_0000_0000
                (op::ORML, 0, 0xCA, 0xFE), // $0 |= 0x0000_0000_CAFE_0000
                (op::ORL, 0, 0xBA, 0xBE),  // $0 |= 0x0000_0000_0000_BABE
                (op::TRAP, 0, 0, 0),       // halt
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(0), 0xDEAD_BEEF_CAFE_BABE);
    }

    // ==== Branches ====

    #[test]
    fn bz_taken() {
        // $0 = 0 (default), BZ $0, +2 instructions forward => skip to addr 8
        let mut m = exec_program(
            0,
            &[
                (op::BZ, 0, 0x00, 0x02),   // if $0==0, jump to pc + 2*4 = 8
                (op::SETL, 1, 0x00, 0xFF), // $1 = 0xFF (should be skipped)
                (op::TRAP, 0, 0, 0),       // halt
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0); // SETL was skipped
    }

    #[test]
    fn bz_not_taken() {
        let mut m = exec_program(
            0,
            &[
                (op::SETL, 0, 0x00, 0x01), // $0 = 1
                (op::BZ, 0, 0x00, 0x03),   // if $0==0, jump (not taken)
                (op::SETL, 1, 0x00, 0xFF), // $1 = 0xFF (should execute)
                (op::TRAP, 0, 0, 0),       // halt
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0xFF);
    }

    #[test]
    fn bnz_taken() {
        let mut m = exec_program(
            0,
            &[
                (op::SETL, 0, 0x00, 0x01), // $0 = 1
                (op::BNZ, 0, 0x00, 0x02),  // if $0!=0, jump to pc + 2*4
                (op::SETL, 1, 0x00, 0xFF), // should be skipped
                (op::TRAP, 0, 0, 0),
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0);
    }

    #[test]
    fn jmp_forward() {
        let mut m = exec_program(
            0,
            &[
                (op::JMP, 0x00, 0x00, 0x02), // jump forward 2 instructions
                (op::SETL, 1, 0x00, 0xFF),   // should be skipped
                (op::TRAP, 0, 0, 0),
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0);
    }

    #[test]
    fn go_saves_return_addr() {
        let mut m = Machine::new();
        m.general.set(1, 100); // target address
        // GO $0, $1, 0 at addr 0 => $0 = 4 (next pc), jump to addr 100
        let word = RawInst {
            op: op::GOI,
            x: 0,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, word);
        // Put TRAP at addr 100
        let halt = RawInst {
            op: op::TRAP,
            x: 0,
            y: 0,
            z: 0,
        }
        .encode();
        m.memory.write_u32(100, halt);
        m.set_entry(0);
        m.run().unwrap();
        assert_eq!(m.general.get(0), 4); // saved return address
    }

    // ==== GETA ====

    #[test]
    fn geta() {
        // GETA $0, 5 at addr 0 => $0 = 0 + 5*4 = 20
        let m = exec_one(op::GETA, 0, 0x00, 0x05);
        assert_eq!(m.general.get(0), 20);
    }

    // ==== Conditional set ====

    #[test]
    fn csz_taken() {
        let mut m = Machine::new();
        m.general.set(1, 0); // condition: zero
        m.general.set(2, 42);
        let word = RawInst {
            op: op::CSZ,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 42);
    }

    #[test]
    fn csz_not_taken() {
        let mut m = Machine::new();
        m.general.set(0, 99); // pre-existing value
        m.general.set(1, 1); // condition: non-zero
        m.general.set(2, 42);
        let word = RawInst {
            op: op::CSZ,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 99); // unchanged
    }

    #[test]
    fn csnz_taken() {
        let mut m = Machine::new();
        m.general.set(1, 5); // condition: non-zero
        m.general.set(2, 77);
        let word = RawInst {
            op: op::CSNZ,
            x: 0,
            y: 1,
            z: 2,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 77);
    }

    // ==== GET/PUT ====

    #[test]
    fn get_put_special_register() {
        let mut m = exec_program(
            0,
            &[
                (op::PUTI, 0, 0, 42), // PUT rA, 42 (rA encoding=0, x=0)
                (op::GET, 1, 0, 0),   // GET $1, rA
                (op::TRAP, 0, 0, 0),
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(1), 42);
    }

    #[test]
    fn put_invalid_register() {
        let mut m = Machine::new();
        let word = RawInst {
            op: op::PUT,
            x: 200,
            y: 0,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        assert!(m.step().is_err());
    }

    // ==== TRAP ====

    #[test]
    fn trap_halt() {
        let mut m = exec_program(0, &[(op::SETL, 0, 0x00, 0x01), (op::TRAP, 0, 0, 0)]);
        m.run().unwrap();
        assert!(m.halted);
        assert_eq!(m.general.get(0), 1);
    }

    #[test]
    fn trap_unimplemented() {
        let mut m = Machine::new();
        let word = RawInst {
            op: op::TRAP,
            x: 99,
            y: 99,
            z: 99,
        }
        .encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        assert!(m.step().is_err());
    }

    // ==== PC advancement ====

    #[test]
    fn pc_advances_by_4() {
        let m = exec_one(op::SETL, 0, 0, 0);
        assert_eq!(m.pc, 4);
    }

    // ==== Unimplemented opcode ====

    #[test]
    fn unimplemented_opcode_errors() {
        let mut m = Machine::new();
        let word = RawInst {
            op: 0x01,
            x: 0,
            y: 0,
            z: 0,
        }
        .encode(); // 0x01 is not implemented
        m.memory.write_u32(0, word);
        m.set_entry(0);
        let result = m.step();
        assert!(result.is_err());
    }

    // ==== Step after halt ====

    #[test]
    fn step_after_halt_errors() {
        let mut m = Machine::new();
        m.halted = true;
        assert!(m.step().is_err());
    }

    // ==== Multi-instruction program: compute factorial(5) ====

    #[test]
    fn factorial_5() {
        // Compute 5! = 120 using a loop
        // $0 = n = 5, $1 = result = 1
        // loop: result *= n; n--; if n > 0 goto loop
        // BPB (0x45) backward branch: offset = -((0x10000 - yz)*4)
        // target addr 8, branch at addr 16 => need offset = -8
        // -((0x10000 - yz)*4) = -8 => yz = 0xFFFE => y=0xFF, z=0xFE
        let mut m = exec_program(
            0,
            &[
                (op::SETL, 0, 0x00, 0x05), // addr 0: $0 = 5
                (op::SETL, 1, 0x00, 0x01), // addr 4: $1 = 1
                (op::MUL, 1, 1, 0),        // addr 8: $1 *= $0
                (op::SUBI, 0, 0, 1),       // addr 12: $0--
                (0x45, 0, 0xFF, 0xFE),     // addr 16: BPB $0, back to addr 8
                (op::TRAP, 0, 0, 0),       // addr 20: halt
            ],
        );
        m.run().unwrap();
        assert_eq!(m.general.get(1), 120);
    }

    // ==== Load/store word and tetra ====

    #[test]
    fn stw_ldw_signed() {
        let mut m = Machine::new();
        m.general.set(0, 0x8001); // as i16 this is negative
        m.general.set(1, 0x3000);
        let stw = RawInst {
            op: op::STWI,
            x: 0,
            y: 1,
            z: 0,
        }
        .encode();
        let ldw = RawInst {
            op: op::LDWI,
            x: 2,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, stw);
        m.memory.write_u32(4, ldw);
        m.set_entry(0);
        m.step().unwrap();
        m.step().unwrap();
        // LDW sign-extends: 0x8001 as i16 = -32767
        assert_eq!(m.general.get(2) as i64, -32767);
    }

    #[test]
    fn stt_ldt_signed() {
        let mut m = Machine::new();
        m.general.set(0, 0x8000_0001);
        m.general.set(1, 0x4000);
        let stt = RawInst {
            op: op::STTI,
            x: 0,
            y: 1,
            z: 0,
        }
        .encode();
        let ldt = RawInst {
            op: op::LDTI,
            x: 2,
            y: 1,
            z: 0,
        }
        .encode();
        m.memory.write_u32(0, stt);
        m.memory.write_u32(4, ldt);
        m.set_entry(0);
        m.step().unwrap();
        m.step().unwrap();
        assert_eq!(m.general.get(2) as i64, 0x8000_0001u32 as i32 as i64);
    }

    // ==== load_raw / set_entry helpers ====

    #[test]
    fn load_raw_and_run() {
        let mut m = Machine::new();
        let halt = RawInst {
            op: op::TRAP,
            x: 0,
            y: 0,
            z: 0,
        };
        let set = RawInst {
            op: op::SETL,
            x: 5,
            y: 0,
            z: 42,
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&set.encode().to_be_bytes());
        bytes.extend_from_slice(&halt.encode().to_be_bytes());
        m.load_raw(0x100, &bytes);
        m.set_entry(0x100);
        m.run().unwrap();
        assert_eq!(m.general.get(5), 42);
    }

    // ==== P0: Unsigned arithmetic ====

    #[test]
    fn addu_reg() {
        let mut m = Machine::new();
        m.general.set(1, u64::MAX);
        m.general.set(2, 3);
        let word = RawInst { op: op::ADDU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 2); // wraps
    }

    #[test]
    fn subu_imm() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        let word = RawInst { op: op::SUBUI, x: 0, y: 1, z: 3 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 7);
    }

    #[test]
    fn mulu_sets_rh() {
        let mut m = Machine::new();
        m.general.set(1, u64::MAX);
        m.general.set(2, 2);
        let word = RawInst { op: op::MULU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), u64::MAX.wrapping_mul(2));
        assert_eq!(m.special.get(SpecialRegister::Rh), 1);
    }

    #[test]
    fn divu_basic() {
        let mut m = Machine::new();
        m.general.set(1, 42);
        m.general.set(2, 5);
        // rD defaults to 0, rH defaults to 0
        let word = RawInst { op: op::DIVU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 8);
        assert_eq!(m.special.get(SpecialRegister::Rr), 2);
    }

    #[test]
    fn cmpu_unsigned() {
        let mut m = Machine::new();
        m.general.set(1, u64::MAX); // unsigned: largest
        m.general.set(2, 1);
        let word = RawInst { op: op::CMPU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 1); // MAX > 1 unsigned
    }

    #[test]
    fn negu() {
        let m = exec_one(op::NEGU, 0, 5, 3);
        // $0 = Y - $Z = 5 - $3, but $3=0 so result=5
        assert_eq!(m.general.get(0), 5);
    }

    // ==== P0: Scaled addition ====

    #[test]
    fn _2addu() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        m.general.set(2, 5);
        let word = RawInst { op: op::_2ADDU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 25); // 10*2+5
    }

    #[test]
    fn _4addu_imm() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        let word = RawInst { op: op::_4ADDUI, x: 0, y: 1, z: 3 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 43); // 10*4+3
    }

    #[test]
    fn _8addu() {
        let mut m = Machine::new();
        m.general.set(1, 5);
        m.general.set(2, 1);
        let word = RawInst { op: op::_8ADDU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 41); // 5*8+1
    }

    #[test]
    fn _16addu() {
        let mut m = Machine::new();
        m.general.set(1, 3);
        m.general.set(2, 7);
        let word = RawInst { op: op::_16ADDU, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 55); // 3*16+7
    }

    // ==== P0: Complemented logic ====

    #[test]
    fn orn() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF00);
        m.general.set(2, 0xFF00);
        let word = RawInst { op: op::ORN, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0xFF00 | !0xFF00u64);
    }

    #[test]
    fn nor() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF);
        m.general.set(2, 0xFF00);
        let word = RawInst { op: op::NOR, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), !(0xFF | 0xFF00));
    }

    #[test]
    fn andn() {
        let mut m = Machine::new();
        m.general.set(1, 0xFFFF);
        m.general.set(2, 0x0F0F);
        let word = RawInst { op: op::ANDN, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0xFFFF & !0x0F0F);
    }

    #[test]
    fn nand() {
        let mut m = Machine::new();
        m.general.set(1, 0xFF);
        m.general.set(2, 0x0F);
        let word = RawInst { op: op::NAND, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), !(0xFF & 0x0F));
    }

    #[test]
    fn nxor() {
        let mut m = Machine::new();
        m.general.set(1, 0xAA);
        m.general.set(2, 0x55);
        let word = RawInst { op: op::NXOR, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), !(0xAA ^ 0x55));
    }

    // ==== P0: SLU ====

    #[test]
    fn slu_large_shift() {
        let mut m = Machine::new();
        m.general.set(1, 1);
        let word = RawInst { op: op::SLUI, x: 0, y: 1, z: 64 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0); // shift >= 64 gives 0
    }

    // ==== P0: Unsigned loads ====

    #[test]
    fn ldwu() {
        let mut m = Machine::new();
        m.general.set(1, 0x2000);
        m.memory.write_u16(0x2000, 0x8001);
        let word = RawInst { op: op::LDWUI, x: 0, y: 1, z: 0 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0x8001); // zero-extended, not sign-extended
    }

    #[test]
    fn ldtu() {
        let mut m = Machine::new();
        m.general.set(1, 0x3000);
        m.memory.write_u32(0x3000, 0x8000_0001);
        let word = RawInst { op: op::LDTUI, x: 0, y: 1, z: 0 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0x8000_0001u64); // zero-extended
    }

    #[test]
    fn ldou() {
        let mut m = Machine::new();
        m.general.set(1, 0x4000);
        m.memory.write_u64(0x4000, 0xDEAD_BEEF_CAFE_BABE);
        let word = RawInst { op: op::LDOUI, x: 0, y: 1, z: 0 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0xDEAD_BEEF_CAFE_BABE);
    }

    // ==== P0: Unsigned stores ====

    #[test]
    fn stbu_ldbu_roundtrip() {
        let mut m = Machine::new();
        m.general.set(0, 0xAB);
        m.general.set(1, 0x5000);
        let st = RawInst { op: op::STBUI, x: 0, y: 1, z: 0 }.encode();
        let ld = RawInst { op: op::LDBUI, x: 2, y: 1, z: 0 }.encode();
        m.memory.write_u32(0, st);
        m.memory.write_u32(4, ld);
        m.set_entry(0);
        m.step().unwrap();
        m.step().unwrap();
        assert_eq!(m.general.get(2), 0xAB);
    }

    // ==== P0: INCx / ANDNx ====

    #[test]
    fn inch() {
        let mut m = Machine::new();
        m.general.set(0, 0x0001_0000_0000_0000);
        let word = RawInst { op: op::INCH, x: 0, y: 0x00, z: 0x01 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0x0002_0000_0000_0000);
    }

    #[test]
    fn incl() {
        let mut m = Machine::new();
        m.general.set(0, 100);
        let word = RawInst { op: op::INCL, x: 0, y: 0x00, z: 50 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 150);
    }

    #[test]
    fn andnl() {
        let mut m = Machine::new();
        m.general.set(0, 0xFFFF_FFFF_FFFF_FFFF);
        let word = RawInst { op: op::ANDNL, x: 0, y: 0x00, z: 0xFF }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), !0xFF_u64);
    }

    // ==== P0: Branches BOD/BNP/BEV ====

    #[test]
    fn bod_taken() {
        let mut m = exec_program(0, &[
            (op::SETL, 0, 0x00, 0x03),   // $0 = 3 (odd)
            (op::BOD, 0, 0x00, 0x02),     // BOD $0, +2 => skip to addr 12
            (op::SETL, 1, 0x00, 0xFF),    // should be skipped
            (op::TRAP, 0, 0, 0),
        ]);
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0);
    }

    #[test]
    fn bev_taken() {
        let mut m = exec_program(0, &[
            (op::SETL, 0, 0x00, 0x04),   // $0 = 4 (even)
            (op::BEV, 0, 0x00, 0x02),     // BEV $0, +2 => skip to addr 12
            (op::SETL, 1, 0x00, 0xFF),
            (op::TRAP, 0, 0, 0),
        ]);
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0);
    }

    #[test]
    fn bnp_taken() {
        let mut m = exec_program(0, &[
            // $0 = 0 by default, 0 <= 0, so BNP taken
            (op::BNP, 0, 0x00, 0x02),     // BNP $0, +2 => skip to addr 12
            (op::SETL, 1, 0x00, 0xFF),
            (op::TRAP, 0, 0, 0),
        ]);
        m.run().unwrap();
        assert_eq!(m.general.get(1), 0);
    }

    // ==== P0: Conditional set CSN/CSP/CSOD/CSNN/CSNP/CSEV ====

    #[test]
    fn csn_taken() {
        let mut m = Machine::new();
        m.general.set(1, (-5i64) as u64); // negative
        m.general.set(2, 42);
        let word = RawInst { op: op::CSN, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 42);
    }

    #[test]
    fn csn_not_taken() {
        let mut m = Machine::new();
        m.general.set(0, 99);
        m.general.set(1, 5); // positive, not negative
        m.general.set(2, 42);
        let word = RawInst { op: op::CSN, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 99); // unchanged
    }

    #[test]
    fn csp_taken() {
        let mut m = Machine::new();
        m.general.set(1, 5); // positive
        m.general.set(2, 77);
        let word = RawInst { op: op::CSP, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 77);
    }

    #[test]
    fn csod_taken() {
        let mut m = Machine::new();
        m.general.set(1, 7); // odd
        m.general.set(2, 33);
        let word = RawInst { op: op::CSOD, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 33);
    }

    #[test]
    fn csev_not_taken() {
        let mut m = Machine::new();
        m.general.set(0, 99);
        m.general.set(1, 7); // odd, not even
        m.general.set(2, 33);
        let word = RawInst { op: op::CSEV, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 99); // unchanged
    }

    // ==== P0: Zero-or-set ZSxx ====

    #[test]
    fn zsn_positive_gives_zero() {
        let mut m = Machine::new();
        m.general.set(1, 5); // positive
        m.general.set(2, 42);
        let word = RawInst { op: op::ZSN, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0); // not negative, so 0
    }

    #[test]
    fn zsn_negative_gives_value() {
        let mut m = Machine::new();
        m.general.set(1, (-3i64) as u64); // negative
        m.general.set(2, 42);
        let word = RawInst { op: op::ZSN, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 42);
    }

    #[test]
    fn zsz() {
        let mut m = Machine::new();
        // $1 = 0
        m.general.set(2, 99);
        let word = RawInst { op: op::ZSZ, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 99); // $1 == 0, so take value
    }

    #[test]
    fn zsp() {
        let mut m = Machine::new();
        m.general.set(1, 10);
        m.general.set(2, 55);
        let word = RawInst { op: op::ZSP, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 55); // $1 > 0
    }

    #[test]
    fn zsod_even_gives_zero() {
        let mut m = Machine::new();
        m.general.set(1, 4); // even
        m.general.set(2, 42);
        let word = RawInst { op: op::ZSOD, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 0); // even, not odd
    }

    #[test]
    fn zsev_even_gives_value() {
        let mut m = Machine::new();
        m.general.set(1, 4); // even
        m.general.set(2, 42);
        let word = RawInst { op: op::ZSEV, x: 0, y: 1, z: 2 }.encode();
        m.memory.write_u32(0, word);
        m.set_entry(0);
        m.step().unwrap();
        assert_eq!(m.general.get(0), 42); // even
    }
}
