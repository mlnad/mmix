/// MMIX Instructions
/// [OP, X, Y, Z] or [OP, X, YZ]
#[derive(Debug, Clone, Copy)]
pub struct RawInst {
    pub op: u8,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl RawInst {
    pub fn decode(word: u32) -> Self {
        Self {
            op: ((word >> 24) & 0xFF) as u8,
            x: ((word >> 16) & 0xFF) as u8,
            y: ((word >> 8) & 0xFF) as u8,
            z: (word & 0xFF) as u8,
        }
    }

    pub fn encode(&self) -> u32 {
        ((self.op as u32) << 24)
            | ((self.x as u32) << 16)
            | ((self.y as u32) << 8)
            | (self.z as u32)
    }

    /// Y*256 + Z, used for immediate/branch offsets
    pub fn yz(&self) -> u16 {
        ((self.y as u16) << 8) | self.z as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_fields() {
        let inst = RawInst::decode(0xAB_CD_EF_01);
        assert_eq!(inst.op, 0xAB);
        assert_eq!(inst.x, 0xCD);
        assert_eq!(inst.y, 0xEF);
        assert_eq!(inst.z, 0x01);
    }

    #[test]
    fn encode_roundtrip() {
        let word = 0x20_01_02_03u32;
        let inst = RawInst::decode(word);
        assert_eq!(inst.encode(), word);
    }

    #[test]
    fn yz_value() {
        let inst = RawInst::decode(0x00_00_12_34);
        assert_eq!(inst.yz(), 0x1234);
    }

    #[test]
    fn encode_all_ff() {
        let inst = RawInst::decode(0xFFFFFFFF);
        assert_eq!(inst.op, 0xFF);
        assert_eq!(inst.x, 0xFF);
        assert_eq!(inst.y, 0xFF);
        assert_eq!(inst.z, 0xFF);
        assert_eq!(inst.yz(), 0xFFFF);
        assert_eq!(inst.encode(), 0xFFFFFFFF);
    }

    #[test]
    fn encode_zero() {
        let inst = RawInst::decode(0);
        assert_eq!(inst.op, 0);
        assert_eq!(inst.x, 0);
        assert_eq!(inst.y, 0);
        assert_eq!(inst.z, 0);
        assert_eq!(inst.encode(), 0);
    }

    #[test]
    fn opcode_constants_no_overlap() {
        // Verify a sampling of opcode constants are distinct
        let ops = [
            op::ADD,
            op::SUB,
            op::MUL,
            op::DIV,
            op::CMP,
            op::NEG,
            op::AND,
            op::OR,
            op::XOR,
            op::SL,
            op::SR,
            op::SRU,
            op::LDB,
            op::LDO,
            op::STB,
            op::STO,
            op::SETH,
            op::SETL,
            op::BZ,
            op::JMP,
            op::GO,
            op::GET,
            op::PUT,
            op::TRAP,
        ];
        for i in 0..ops.len() {
            for j in (i + 1)..ops.len() {
                assert_ne!(ops[i], ops[j], "opcode conflict at index {} and {}", i, j);
            }
        }
    }
}

pub mod op {
    pub const ADD: u8 = 0x20;
    pub const ADDI: u8 = 0x21;
    pub const SUB: u8 = 0x24;
    pub const SUBI: u8 = 0x25;
    pub const MUL: u8 = 0x28;
    pub const MULI: u8 = 0x29;
    pub const DIV: u8 = 0x2C;
    pub const DIVI: u8 = 0x2D;
    pub const CMP: u8 = 0x30;
    pub const CMPI: u8 = 0x31;
    pub const NEG: u8 = 0x34;
    pub const NEGI: u8 = 0x35;

    // 逻辑/移位
    pub const AND: u8 = 0xC8;
    pub const OR: u8 = 0xC0;
    pub const XOR: u8 = 0xC6;
    pub const SL: u8 = 0x38;
    pub const SLI: u8 = 0x39;
    pub const SR: u8 = 0x3C;
    pub const SRI: u8 = 0x3D;
    pub const SRU: u8 = 0x3E;
    pub const SRUI: u8 = 0x3F;

    // 内存
    pub const LDB: u8 = 0x80;
    pub const LDBI: u8 = 0x81;
    pub const LDW: u8 = 0x84;
    pub const LDWI: u8 = 0x85;
    pub const LDT: u8 = 0x88;
    pub const LDTI: u8 = 0x89;
    pub const LDO: u8 = 0x8C;
    pub const LDOI: u8 = 0x8D;
    pub const LDBU: u8 = 0x82;
    pub const LDBUI: u8 = 0x83;
    pub const STB: u8 = 0xA0;
    pub const STBI: u8 = 0xA1;
    pub const STW: u8 = 0xA4;
    pub const STWI: u8 = 0xA5;
    pub const STT: u8 = 0xA8;
    pub const STTI: u8 = 0xA9;
    pub const STO: u8 = 0xAC;
    pub const STOI: u8 = 0xAD;

    // 常量加载
    pub const SETH: u8 = 0xE0;
    pub const SETMH: u8 = 0xE1;
    pub const SETML: u8 = 0xE2;
    pub const SETL: u8 = 0xE3;
    pub const ORH: u8 = 0xE8;
    pub const ORMH: u8 = 0xE9;
    pub const ORML: u8 = 0xEA;
    pub const ORL: u8 = 0xEB;

    // 分支
    pub const BZ: u8 = 0x42;
    pub const BZB: u8 = 0x43;
    pub const BNZ: u8 = 0x4A;
    pub const BNZB: u8 = 0x4B;
    pub const BP: u8 = 0x44;
    pub const BN: u8 = 0x48;
    pub const BNN: u8 = 0x46;
    pub const JMP: u8 = 0xF0;
    pub const JMPB: u8 = 0xF1;
    pub const GO: u8 = 0x9E;
    pub const GOI: u8 = 0x9F;

    // 特殊
    pub const GET: u8 = 0xFE;
    pub const PUT: u8 = 0xF6;
    pub const PUTI: u8 = 0xF7;
    pub const TRAP: u8 = 0x00;

    // 条件赋值（部分）
    pub const CSZ: u8 = 0x60;
    pub const CSZI: u8 = 0x61;
    pub const CSNZ: u8 = 0x6A;
    pub const CSNZI: u8 = 0x6B;

    // GETA
    pub const GETA: u8 = 0xF4;
    pub const GETAB: u8 = 0xF5;
}
