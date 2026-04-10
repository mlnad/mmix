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
}
