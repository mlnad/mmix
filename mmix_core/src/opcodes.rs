/// Operand format classification for each opcode.
///
/// Used by the assembler to determine how to parse and encode operands
/// without hardcoding per-opcode logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandFormat {
    /// $X, $Y, $Z | $X, $Y, Z — auto immediate variant (base_op + 1)
    ThreeReg,
    /// $X, YZ — 16-bit immediate (SETH, SETL, ORH, etc.)
    RegImm16,
    /// $X, YZ — branch relative (BZ, BNZ, GETA, etc.)
    Branch,
    /// XYZ — 24-bit relative (JMP)
    Jump,
    /// $X, Y, $Z | $X, Y, Z — NEG-style (Y is inline constant)
    NegStyle,
    /// X, Y, Z — three immediates (TRAP, TRIP)
    Trap,
    /// GET $X, special_reg
    Get,
    /// PUT special_reg, $Z | imm
    Put,
    /// POP X, YZ — special format
    Pop,
    /// No operands (RESUME, SAVE, UNSAVE, SYNC, SWYM)
    Special,
    /// PUSHJ/PUSHGO — register-push variants
    PushJ,
}

/// Return the operand format for a given opcode.
pub fn format(opcode: u8) -> OperandFormat {
    FORMAT_TABLE[opcode as usize]
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
    output {
        timing:  TIMING_TABLE,
        names:   NAME_TABLE,
        ops:     op,
        formats: FORMAT_TABLE,
    }

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            TRAP(5,0,Trap),        FCMP(1,0,ThreeReg),    FUN(1,0,ThreeReg),     FEQL(1,0,ThreeReg),    FADD(4,0,ThreeReg),    FIX(4,0,ThreeReg),     FSUB(4,0,ThreeReg),    FIXU(4,0,ThreeReg),
//  #0x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            FLOT(4,0,ThreeReg),    FLOTI(4,0,ThreeReg),   FLOTU(4,0,ThreeReg),   FLOTUI(4,0,ThreeReg),  SFLOT(4,0,ThreeReg),   SFLOTI(4,0,ThreeReg),  SFLOTU(4,0,ThreeReg),  SFLOTUI(4,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            FMUL(4,0,ThreeReg),    FCMPE(4,0,ThreeReg),   FUNE(1,0,ThreeReg),    FEQLE(4,0,ThreeReg),   FDIV(40,0,ThreeReg),   FSQRT(40,0,ThreeReg),  FREM(4,0,ThreeReg),    FINT(4,0,ThreeReg),
//  #1x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            MUL(10,0,ThreeReg),    MULI(10,0,ThreeReg),   MULU(10,0,ThreeReg),   MULUI(10,0,ThreeReg),  DIV(60,0,ThreeReg),    DIVI(60,0,ThreeReg),   DIVU(60,0,ThreeReg),   DIVUI(60,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            ADD(1,0,ThreeReg),     ADDI(1,0,ThreeReg),    ADDU(1,0,ThreeReg),    ADDUI(1,0,ThreeReg),   SUB(1,0,ThreeReg),     SUBI(1,0,ThreeReg),    SUBU(1,0,ThreeReg),    SUBUI(1,0,ThreeReg),
//  #2x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            _2ADDU(1,0,ThreeReg),  _2ADDUI(1,0,ThreeReg), _4ADDU(1,0,ThreeReg),  _4ADDUI(1,0,ThreeReg), _8ADDU(1,0,ThreeReg),  _8ADDUI(1,0,ThreeReg), _16ADDU(1,0,ThreeReg), _16ADDUI(1,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            CMP(1,0,ThreeReg),     CMPI(1,0,ThreeReg),    CMPU(1,0,ThreeReg),    CMPUI(1,0,ThreeReg),   NEG(1,0,NegStyle),     NEGI(1,0,NegStyle),    NEGU(1,0,NegStyle),    NEGUI(1,0,NegStyle),
//  #3x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            SL(1,0,ThreeReg),      SLI(1,0,ThreeReg),     SLU(1,0,ThreeReg),     SLUI(1,0,ThreeReg),    SR(1,0,ThreeReg),      SRI(1,0,ThreeReg),     SRU(1,0,ThreeReg),     SRUI(1,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            BN(1,0,Branch),        BNB(1,0,Branch),       BZ(1,0,Branch),        BZB(1,0,Branch),       BP(1,0,Branch),        BPB(1,0,Branch),       BOD(1,0,Branch),       BODB(1,0,Branch),
//  #4x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            BNN(1,0,Branch),       BNNB(1,0,Branch),      BNZ(1,0,Branch),       BNZB(1,0,Branch),     BNP(1,0,Branch),       BNPB(1,0,Branch),      BEV(1,0,Branch),       BEVB(1,0,Branch),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            PBN(3,0,Branch),       PBNB(3,0,Branch),      PBZ(3,0,Branch),       PBZB(3,0,Branch),     PBP(3,0,Branch),       PBPB(3,0,Branch),      PBOD(3,0,Branch),      PBODB(3,0,Branch),
//  #5x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            PBNN(3,0,Branch),      PBNNB(3,0,Branch),     PBNZ(3,0,Branch),      PBNZB(3,0,Branch),    PBNP(3,0,Branch),      PBNPB(3,0,Branch),     PBEV(3,0,Branch),      PBEVB(3,0,Branch),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            CSN(1,0,ThreeReg),     CSNI(1,0,ThreeReg),    CSZ(1,0,ThreeReg),     CSZI(1,0,ThreeReg),   CSP(1,0,ThreeReg),     CSPI(1,0,ThreeReg),    CSOD(1,0,ThreeReg),    CSODI(1,0,ThreeReg),
//  #6x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            CSNN(1,0,ThreeReg),    CSNNI(1,0,ThreeReg),   CSNZ(1,0,ThreeReg),    CSNZI(1,0,ThreeReg),  CSNP(1,0,ThreeReg),    CSNPI(1,0,ThreeReg),   CSEV(1,0,ThreeReg),    CSEVI(1,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            ZSN(1,0,ThreeReg),     ZSNI(1,0,ThreeReg),    ZSZ(1,0,ThreeReg),     ZSZI(1,0,ThreeReg),   ZSP(1,0,ThreeReg),     ZSPI(1,0,ThreeReg),    ZSOD(1,0,ThreeReg),    ZSODI(1,0,ThreeReg),
//  #7x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            ZSNN(1,0,ThreeReg),    ZSNNI(1,0,ThreeReg),   ZSNZ(1,0,ThreeReg),    ZSNZI(1,0,ThreeReg),  ZSNP(1,0,ThreeReg),    ZSNPI(1,0,ThreeReg),   ZSEV(1,0,ThreeReg),    ZSEVI(1,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            LDB(1,1,ThreeReg),     LDBI(1,1,ThreeReg),    LDBU(1,1,ThreeReg),    LDBUI(1,1,ThreeReg),  LDW(1,1,ThreeReg),     LDWI(1,1,ThreeReg),    LDWU(1,1,ThreeReg),    LDWUI(1,1,ThreeReg),
//  #8x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            LDT(1,1,ThreeReg),     LDTI(1,1,ThreeReg),    LDTU(1,1,ThreeReg),    LDTUI(1,1,ThreeReg),  LDO(1,1,ThreeReg),     LDOI(1,1,ThreeReg),    LDOU(1,1,ThreeReg),    LDOUI(1,1,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            LDSF(1,1,ThreeReg),    LDSFI(1,1,ThreeReg),   LDHT(1,1,ThreeReg),    LDHTI(1,1,ThreeReg),  CSWAP(2,2,ThreeReg),   CSWAPI(2,2,ThreeReg),  LDUNC(1,1,ThreeReg),   LDUNCI(1,1,ThreeReg),
//  #9x     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            LDVTS(1,0,ThreeReg),   LDVTSI(1,0,ThreeReg),  PRELD(1,0,ThreeReg),   PRELDI(1,0,ThreeReg), PREGO(1,0,ThreeReg),   PREGOI(1,0,ThreeReg),  GO(3,0,ThreeReg),      GOI(3,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            STB(1,1,ThreeReg),     STBI(1,1,ThreeReg),    STBU(1,1,ThreeReg),    STBUI(1,1,ThreeReg),  STW(1,1,ThreeReg),     STWI(1,1,ThreeReg),    STWU(1,1,ThreeReg),    STWUI(1,1,ThreeReg),
//  #Ax     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            STT(1,1,ThreeReg),     STTI(1,1,ThreeReg),    STTU(1,1,ThreeReg),    STTUI(1,1,ThreeReg),  STO(1,1,ThreeReg),     STOI(1,1,ThreeReg),    STOU(1,1,ThreeReg),    STOUI(1,1,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            STSF(1,1,ThreeReg),    STSFI(1,1,ThreeReg),   STHT(1,1,ThreeReg),    STHTI(1,1,ThreeReg),  STCO(1,1,ThreeReg),    STCOI(1,1,ThreeReg),   STUNC(1,1,ThreeReg),   STUNCI(1,1,ThreeReg),
//  #Bx     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            SYNCD(1,0,ThreeReg),   SYNCDI(1,0,ThreeReg),  PREST(1,0,ThreeReg),   PRESTI(1,0,ThreeReg), SYNCID(1,0,ThreeReg),  SYNCIDI(1,0,ThreeReg), PUSHGO(3,0,PushJ),     PUSHGOI(3,0,PushJ),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            OR(1,0,ThreeReg),      ORI(1,0,ThreeReg),     ORN(1,0,ThreeReg),     ORNI(1,0,ThreeReg),   NOR(1,0,ThreeReg),     NORI(1,0,ThreeReg),    XOR(1,0,ThreeReg),     XORI(1,0,ThreeReg),
//  #Cx     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            AND(1,0,ThreeReg),     ANDI(1,0,ThreeReg),    ANDN(1,0,ThreeReg),    ANDNI(1,0,ThreeReg),  NAND(1,0,ThreeReg),    NANDI(1,0,ThreeReg),   NXOR(1,0,ThreeReg),    NXORI(1,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            BDIF(1,0,ThreeReg),    BDIFI(1,0,ThreeReg),   WDIF(1,0,ThreeReg),    WDIFI(1,0,ThreeReg),  TDIF(1,0,ThreeReg),    TDIFI(1,0,ThreeReg),   ODIF(1,0,ThreeReg),    ODIFI(1,0,ThreeReg),
//  #Dx     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            MUX(1,0,ThreeReg),     MUXI(1,0,ThreeReg),    SADD(1,0,ThreeReg),    SADDI(1,0,ThreeReg),  MOR(1,0,ThreeReg),     MORI(1,0,ThreeReg),    MXOR(1,0,ThreeReg),    MXORI(1,0,ThreeReg),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            SETH(1,0,RegImm16),    SETMH(1,0,RegImm16),   SETML(1,0,RegImm16),   SETL(1,0,RegImm16),  INCH(1,0,RegImm16),    INCMH(1,0,RegImm16),   INCML(1,0,RegImm16),   INCL(1,0,RegImm16),
//  #Ex     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            ORH(1,0,RegImm16),     ORMH(1,0,RegImm16),    ORML(1,0,RegImm16),    ORL(1,0,RegImm16),   ANDNH(1,0,RegImm16),   ANDNMH(1,0,RegImm16),  ANDNML(1,0,RegImm16),  ANDNL(1,0,RegImm16),

//          #x0                    #x1                    #x2                    #x3                    #x4                    #x5                    #x6                    #x7
            JMP(1,0,Jump),         JMPB(1,0,Jump),        PUSHJ(1,0,PushJ),      PUSHJB(1,0,PushJ),   GETA(1,0,Branch),      GETAB(1,0,Branch),     PUT(1,0,Put),          PUTI(1,0,Put),
//  #Fx     #x8                    #x9                    #xA                    #xB                    #xC                    #xD                    #xE                    #xF
            POP(3,0,Pop),          RESUME(5,0,Special),   SAVE(1,20,Special),    UNSAVE(1,20,Special), SYNC(1,0,Special),     SWYM(1,0,Special),     GET(1,0,Get),          TRIP(5,0,Trap),
}

#[cfg(test)]
mod tests {
    use super::*;

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