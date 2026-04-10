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
            op::TRIP,
        ];
        for i in 0..ops.len() {
            for j in (i + 1)..ops.len() {
                assert_ne!(ops[i], ops[j], "opcode conflict at index {} and {}", i, j);
            }
        }
    }

    #[test]
    fn opcode_values_from_position() {
        // Opcodes are derived from table position, verify key values
        assert_eq!(op::TRAP, 0x00);
        assert_eq!(op::MUL, 0x18);
        assert_eq!(op::MULI, 0x19);
        assert_eq!(op::DIV, 0x1C);
        assert_eq!(op::ADD, 0x20);
        assert_eq!(op::ADDI, 0x21);
        assert_eq!(op::SUB, 0x24);
        assert_eq!(op::BN, 0x40);
        assert_eq!(op::BZ, 0x42);
        assert_eq!(op::BP, 0x44);
        assert_eq!(op::BNN, 0x48);
        assert_eq!(op::BNZ, 0x4A);
        assert_eq!(op::LDB, 0x80);
        assert_eq!(op::GO, 0x9E);
        assert_eq!(op::STB, 0xA0);
        assert_eq!(op::OR, 0xC0);
        assert_eq!(op::AND, 0xC8);
        assert_eq!(op::XOR, 0xC6);
        assert_eq!(op::SETH, 0xE0);
        assert_eq!(op::JMP, 0xF0);
        assert_eq!(op::GET, 0xFE);
        assert_eq!(op::TRIP, 0xFF);
    }

    #[test]
    fn name_table_entries() {
        assert_eq!(name(op::TRAP), "TRAP");
        assert_eq!(name(op::ADD), "ADD");
        assert_eq!(name(op::ADDI), "ADDI");
        assert_eq!(name(op::_2ADDU), "2ADDU");
        assert_eq!(name(op::_16ADDUI), "16ADDUI");
        assert_eq!(name(op::LDB), "LDB");
        assert_eq!(name(op::GET), "GET");
        assert_eq!(name(op::TRIP), "TRIP");
    }

    #[test]
    fn timing_table_entries() {
        assert_eq!(timing(op::ADD), Timing::new(1, 0));
        assert_eq!(timing(op::MUL), Timing::new(10, 0));
        assert_eq!(timing(op::DIV), Timing::new(60, 0));
        assert_eq!(timing(op::LDB), Timing::new(1, 1));
        assert_eq!(timing(op::TRAP), Timing::new(5, 0));
        assert_eq!(timing(op::SAVE), Timing::new(1, 20));
    }
}

/// Instruction timing cost, in units of υ (clock cycles) and μ (memory accesses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub v: u64,
    pub mu: u64,
}

impl Timing {
    pub const fn new(v: u64, mu: u64) -> Self {
        Self { v, mu }
    }
}

/// Return the estimated execution cost for a given opcode.
pub fn timing(opcode: u8) -> Timing {
    TIMING_TABLE[opcode as usize]
}

/// Return the mnemonic name for a given opcode.
pub fn name(opcode: u8) -> &'static str {
    NAME_TABLE[opcode as usize]
}

// MMIX Opcode Table — 16×16 grid matching Knuth's MMIX reference card.
//
// Each entry `NAME(υ, μ)` defines an opcode whose numeric value equals its position (0..255).
// The macro generates `TIMING_TABLE`, `NAME_TABLE`, and `pub mod op` automatically.
//
// Notation: υ = clock cycles, μ = memory accesses, π = branch misprediction penalty.
// Names starting with `_` (e.g. `_2ADDU`) have the underscore stripped in NAME_TABLE.
#[rustfmt::skip]
mmix_macros::define_opcodes! {
//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            TRAP(5,0),     FCMP(1,0),     FUN(1,0),      FEQL(1,0),     FADD(4,0),     FIX(4,0),      FSUB(4,0),     FIXU(4,0),
//  #0x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            FLOT(4,0),     FLOTI(4,0),    FLOTU(4,0),    FLOTUI(4,0),   SFLOT(4,0),    SFLOTI(4,0),   SFLOTU(4,0),   SFLOTUI(4,0),

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            FMUL(4,0),     FCMPE(4,0),    FUNE(1,0),     FEQLE(4,0),    FDIV(40,0),    FSQRT(40,0),   FREM(4,0),     FINT(4,0),
//  #1x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            MUL(10,0),     MULI(10,0),    MULU(10,0),    MULUI(10,0),   DIV(60,0),     DIVI(60,0),    DIVU(60,0),    DIVUI(60,0),

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            ADD(1,0),      ADDI(1,0),     ADDU(1,0),     ADDUI(1,0),    SUB(1,0),      SUBI(1,0),     SUBU(1,0),     SUBUI(1,0),
//  #2x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            _2ADDU(1,0),   _2ADDUI(1,0),  _4ADDU(1,0),   _4ADDUI(1,0),  _8ADDU(1,0),   _8ADDUI(1,0),  _16ADDU(1,0),  _16ADDUI(1,0)

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            CMP(1,0),      CMPI(1,0),     CMPU(1,0),     CMPUI(1,0),    NEG(1,0),      NEGI(1,0),     NEGU(1,0),     NEGUI(1,0),  
//  #3x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            SL(1,0),       SLI(1,0),      SLU(1,0),      SLUI(1,0),     SR(1,0),       SRI(1,0),      SRU(1,0),      SRUI(1,0),   

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            BN(1,0),       BNB(1,0),      BZ(1,0),       BZB(1,0),      BP(1,0),       BPB(1,0),      BOD(1,0),      BODB(1,0),   
//  #4x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            BNN(1,0),      BNNB(1,0),     BNZ(1,0),      BNZB(1,0),     BNP(1,0),      BNPB(1,0),     BEV(1,0),      BEVB(1,0),   

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            PBN(3,0),      PBNB(3,0),     PBZ(3,0),      PBZB(3,0),     PBP(3,0),      PBPB(3,0),     PBOD(3,0),     PBODB(3,0),  
//  #5x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            PBNN(3,0),     PBNNB(3,0),    PBNZ(3,0),     PBNZB(3,0),    PBNP(3,0),     PBNPB(3,0),    PBEV(3,0),     PBEVB(3,0),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            CSN(1,0),      CSNI(1,0),     CSZ(1,0),      CSZI(1,0),     CSP(1,0),      CSPI(1,0),     CSOD(1,0),     CSODI(1,0),  
//  #6x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            CSNN(1,0),     CSNNI(1,0),    CSNZ(1,0),     CSNZI(1,0),    CSNP(1,0),     CSNPI(1,0),    CSEV(1,0),     CSEVI(1,0),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            ZSN(1,0),      ZSNI(1,0),     ZSZ(1,0),      ZSZI(1,0),     ZSP(1,0),      ZSPI(1,0),     ZSOD(1,0),     ZSODI(1,0),  
//  #7x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            ZSNN(1,0),     ZSNNI(1,0),    ZSNZ(1,0),     ZSNZI(1,0),    ZSNP(1,0),     ZSNPI(1,0),    ZSEV(1,0),     ZSEVI(1,0),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            LDB(1,1),      LDBI(1,1),     LDBU(1,1),     LDBUI(1,1),    LDW(1,1),      LDWI(1,1),     LDWU(1,1),     LDWUI(1,1),  
//  #8x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            LDT(1,1),      LDTI(1,1),     LDTU(1,1),     LDTUI(1,1),    LDO(1,1),      LDOI(1,1),     LDOU(1,1),     LDOUI(1,1),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            LDSF(1,1),     LDSFI(1,1),    LDHT(1,1),     LDHTI(1,1),    CSWAP(2,2),    CSWAPI(2,2),   LDUNC(1,1),    LDUNCI(1,1), 
//  #9x     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            LDVTS(1,0),    LDVTSI(1,0),   PRELD(1,0),    PRELDI(1,0),   PREGO(1,0),    PREGOI(1,0),   GO(3,0),       GOI(3,0),    

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            STB(1,1),      STBI(1,1),     STBU(1,1),     STBUI(1,1),    STW(1,1),      STWI(1,1),     STWU(1,1),     STWUI(1,1),  
//  #Ax     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            STT(1,1),      STTI(1,1),     STTU(1,1),     STTUI(1,1),    STO(1,1),      STOI(1,1),     STOU(1,1),     STOUI(1,1),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            STSF(1,1),     STSFI(1,1),    STHT(1,1),     STHTI(1,1),    STCO(1,1),     STCOI(1,1),    STUNC(1,1),    STUNCI(1,1), 
//  #Bx     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            SYNCD(1,0),    SYNCDI(1,0),   PREST(1,0),    PRESTI(1,0),   SYNCID(1,0),   SYNCIDI(1,0),  PUSHGO(3,0),   PUSHGOI(3,0),

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            OR(1,0),       ORI(1,0),      ORN(1,0),      ORNI(1,0),     NOR(1,0),      NORI(1,0),     XOR(1,0),      XORI(1,0),   
//  #Cx     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            AND(1,0),      ANDI(1,0),     ANDN(1,0),     ANDNI(1,0),    NAND(1,0),     NANDI(1,0),    NXOR(1,0),     NXORI(1,0),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            BDIF(1,0),     BDIFI(1,0),    WDIF(1,0),     WDIFI(1,0),    TDIF(1,0),     TDIFI(1,0),    ODIF(1,0),     ODIFI(1,0),  
//  #Dx     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            MUX(1,0),      MUXI(1,0),     SADD(1,0),     SADDI(1,0),    MOR(1,0),      MORI(1,0),     MXOR(1,0),     MXORI(1,0),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            SETH(1,0),     SETMH(1,0),    SETML(1,0),    SETL(1,0),     INCH(1,0),     INCMH(1,0),    INCML(1,0),    INCL(1,0),   
//  #Ex     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            ORH(1,0),      ORMH(1,0),     ORML(1,0),     ORL(1,0),      ANDNH(1,0),    ANDNMH(1,0),   ANDNML(1,0),   ANDNL(1,0),  

//          #x0            #x1            #x2            #x3            #x4            #x5            #x6            #x7
            JMP(1,0),      JMPB(1,0),     PUSHJ(1,0),    PUSHJB(1,0),   GETA(1,0),     GETAB(1,0),    PUT(1,0),      PUTI(1,0),   
//  #Fx     #x8            #x9            #xA            #xB            #xC            #xD            #xE            #xF
            POP(3,0),      RESUME(5,0),   SAVE(1,20),    UNSAVE(1,20),  SYNC(1,0),     SWYM(1,0),     GET(1,0),      TRIP(5,0),   
}
