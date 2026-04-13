use super::*;
use mmix_core::op;

// ─── Basic instruction encoding ───

#[test]
fn assemble_single_add_reg() {
    // ADD $1,$2,$3
    let r = assemble("        ADD $1,$2,$3").unwrap();
    assert_eq!(r.bytes.len(), 4);
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::ADD as u32) << 24) | (1 << 16) | (2 << 8) | 3);
}

#[test]
fn assemble_add_immediate() {
    // ADD $1,$2,42  → uses ADDI opcode
    let r = assemble("        ADD $1,$2,42").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::ADDI as u32) << 24) | (1 << 16) | (2 << 8) | 42);
}

#[test]
fn assemble_sub_reg() {
    let r = assemble("        SUB $5,$10,$20").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SUB as u32) << 24) | (5 << 16) | (10 << 8) | 20);
}

#[test]
fn assemble_setl() {
    let r = assemble("        SETL $1,1000").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 1000);
}

#[test]
fn assemble_seth() {
    let r = assemble("        SETH $0,0xABCD").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETH as u32) << 24) | (0 << 16) | 0xABCD);
}

#[test]
fn assemble_neg_style() {
    // NEG $1,0,$2
    let r = assemble("        NEG $1,0,$2").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::NEG as u32) << 24) | (1 << 16) | (0 << 8) | 2);
}

#[test]
fn assemble_neg_immediate() {
    // NEG $1,0,5  → NEGI
    let r = assemble("        NEG $1,0,5").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::NEGI as u32) << 24) | (1 << 16) | (0 << 8) | 5);
}

#[test]
fn assemble_trap() {
    let r = assemble("        TRAP 0,0,0").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, (op::TRAP as u32) << 24);
}

#[test]
fn assemble_trap_fputs() {
    let r = assemble("        TRAP 0,1,1").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::TRAP as u32) << 24) | (0 << 16) | (1 << 8) | 1);
}

// ─── Labels ───

#[test]
fn assemble_label_forward_branch() {
    let src = "\
        BZ   $0,End\n\
        SETL $1,1\n\
End     TRAP 0,0,0";
    let r = assemble(src).unwrap();
    // BZ $0,End: forward by 2 instructions = offset 8, diff/4 = 2
    let word0 = u32::from_be_bytes(r.bytes[0..4].try_into().unwrap());
    assert_eq!(word0, ((op::BZ as u32) << 24) | (0 << 16) | 2);
}

#[test]
fn assemble_label_backward_branch() {
    let src = "\
Loop    SETL $1,1\n\
        BNZ  $1,Loop";
    let r = assemble(src).unwrap();
    // BNZ $1,Loop: backward by 4 bytes, diff = -4, offset=1, yz = 0x10000 - 1 = 0xFFFF
    let word1 = u32::from_be_bytes(r.bytes[4..8].try_into().unwrap());
    assert_eq!(word1, (((op::BNZ + 1) as u32) << 24) | (1 << 16) | 0xFFFF);
}

#[test]
fn assemble_label_with_colon() {
    let src = "\
Start:  SETL $1,42\n\
        TRAP 0,0,0";
    let r = assemble(src).unwrap();
    assert_eq!(r.bytes.len(), 8);
    let word0 = u32::from_be_bytes(r.bytes[0..4].try_into().unwrap());
    assert_eq!(word0, ((op::SETL as u32) << 24) | (1 << 16) | 42);
}

#[test]
fn assemble_jmp_forward() {
    let src = "\
        JMP  End\n\
        SETL $1,1\n\
End     TRAP 0,0,0";
    let r = assemble(src).unwrap();
    let word0 = u32::from_be_bytes(r.bytes[0..4].try_into().unwrap());
    assert_eq!(word0, ((op::JMP as u32) << 24) | 2);
}

#[test]
fn assemble_geta() {
    let src = "\
        GETA $255,Data\n\
Data    BYTE 0";
    let r = assemble(src).unwrap();
    let word0 = u32::from_be_bytes(r.bytes[0..4].try_into().unwrap());
    // Data is at offset 4, cur is 0, diff/4 = 1
    assert_eq!(word0, ((op::GETA as u32) << 24) | (255 << 16) | 1);
}

// ─── Data directives ───

#[test]
fn assemble_byte_values() {
    let r = assemble("        BYTE 1,2,3").unwrap();
    assert_eq!(r.bytes, vec![1, 2, 3]);
}

#[test]
fn assemble_byte_string() {
    let r = assemble("        BYTE \"AB\"").unwrap();
    assert_eq!(r.bytes, vec![b'A', b'B']);
}

#[test]
fn assemble_byte_string_with_escape() {
    let r = assemble("        BYTE \"\\n\\t\\0\"").unwrap();
    assert_eq!(r.bytes, vec![b'\n', b'\t', 0]);
}

#[test]
fn assemble_byte_string_with_null_terminator() {
    // The assembler treats the string as ending at the second quote,
    // so "Hi",0 is parsed as string "Hi" only (the ,0 is after the string literal parse).
    // To include a null terminator, use \0 inside the string.
    let r = assemble("Str     BYTE \"Hi\\0\"").unwrap();
    assert_eq!(r.bytes, vec![b'H', b'i', 0]);
}

#[test]
fn assemble_wyde_values() {
    let r = assemble("        WYDE 0x1234,0x5678").unwrap();
    assert_eq!(r.bytes, vec![0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn assemble_tetra_value() {
    let r = assemble("        TETRA 0xDEADBEEF").unwrap();
    assert_eq!(r.bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn assemble_octa_value() {
    let r = assemble("        OCTA 0x0102030405060708").unwrap();
    assert_eq!(r.bytes, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
}

// ─── Comments ───

#[test]
fn assemble_with_percent_comment() {
    let r = assemble("        SETL $1,99 % this is a comment").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 99);
}

#[test]
fn assemble_with_semicolon_comment() {
    let r = assemble("        SETL $1,99 ; comment here").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 99);
}

#[test]
fn assemble_blank_and_comment_lines() {
    let src = "\
% This is a comment\n\
\n\
        SETL $1,1\n\
; another comment\n\
        TRAP 0,0,0";
    let r = assemble(src).unwrap();
    assert_eq!(r.bytes.len(), 8);
}

// ─── Hex number formats ───

#[test]
fn assemble_hex_with_hash() {
    let r = assemble("        SETL $1,#FF").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 0xFF);
}

#[test]
fn assemble_hex_with_0x() {
    let r = assemble("        SETL $1,0xFF").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 0xFF);
}

// ─── Special registers (GET/PUT) ───

#[test]
fn assemble_get_special_reg() {
    let r = assemble("        GET $1,rA").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::GET as u32) << 24) | (1 << 16) | 21);
}

#[test]
fn assemble_put_special_reg() {
    let r = assemble("        PUT rA,$5").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::PUT as u32) << 24) | (21 << 16) | 5);
}

#[test]
fn assemble_put_immediate() {
    let r = assemble("        PUT rA,42").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::PUTI as u32) << 24) | (21 << 16) | 42);
}

// ─── @ symbol (current address) ───

#[test]
fn assemble_branch_to_self() {
    // BZ $0,@ should branch to itself (offset 0)
    let r = assemble("        BZ $0,@").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::BZ as u32) << 24) | (0 << 16) | 0);
}

#[test]
fn assemble_jmp_to_self() {
    // JMP @ should jump to itself (offset 0)
    let r = assemble("        JMP @").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::JMP as u32) << 24) | 0);
}

// ─── Line-to-offset mappings ───

#[test]
fn assemble_line_to_offset_mapping() {
    let src = "\
        SETL $1,1\n\
        SETL $2,2\n\
        ADD  $3,$1,$2";
    let r = assemble(src).unwrap();
    assert_eq!(r.line_to_offset[&0], 0);
    assert_eq!(r.line_to_offset[&1], 4);
    assert_eq!(r.line_to_offset[&2], 8);
    assert_eq!(r.offset_to_line[&0], 0);
    assert_eq!(r.offset_to_line[&4], 1);
    assert_eq!(r.offset_to_line[&8], 2);
}

#[test]
fn assemble_entry_addr_is_zero() {
    let r = assemble("        TRAP 0,0,0").unwrap();
    assert_eq!(r.entry_addr, 0);
}

// ─── Memory instructions ───

#[test]
fn assemble_ldo_reg() {
    let r = assemble("        LDO $1,$2,$3").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::LDO as u32) << 24) | (1 << 16) | (2 << 8) | 3);
}

#[test]
fn assemble_sto_immediate() {
    let r = assemble("        STO $1,$2,8").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::STOI as u32) << 24) | (1 << 16) | (2 << 8) | 8);
}

#[test]
fn assemble_go_reg() {
    let r = assemble("        GO $0,$1,$2").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::GO as u32) << 24) | (0 << 16) | (1 << 8) | 2);
}

// ─── Logic/shift ───

#[test]
fn assemble_and_or_xor() {
    let r = assemble("        AND $1,$2,$3\n        OR $4,$5,$6\n        XOR $7,$8,$9").unwrap();
    let w0 = u32::from_be_bytes(r.bytes[0..4].try_into().unwrap());
    let w1 = u32::from_be_bytes(r.bytes[4..8].try_into().unwrap());
    let w2 = u32::from_be_bytes(r.bytes[8..12].try_into().unwrap());
    assert_eq!(w0 >> 24, op::AND as u32);
    assert_eq!(w1 >> 24, op::OR as u32);
    assert_eq!(w2 >> 24, op::XOR as u32);
}

#[test]
fn assemble_shift_left_imm() {
    let r = assemble("        SL $1,$2,3").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SLI as u32) << 24) | (1 << 16) | (2 << 8) | 3);
}

// ─── Conditional set ───

#[test]
fn assemble_csz() {
    let r = assemble("        CSZ $1,$2,$3").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word >> 24, op::CSZ as u32);
}

#[test]
fn assemble_csnz_immediate() {
    let r = assemble("        CSNZ $1,$2,10").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word >> 24, op::CSNZI as u32);
}

// ─── Error cases ───

#[test]
fn assemble_unknown_instruction() {
    let e = assemble("        FOOBAR $1,$2,$3").unwrap_err();
    assert!(e.message.contains("unknown instruction"));
}

#[test]
fn assemble_duplicate_label() {
    let src = "X SETL $1,1\nX SETL $2,2";
    let e = assemble(src).unwrap_err();
    assert!(e.message.contains("duplicate label"));
}

#[test]
fn assemble_undefined_label() {
    let e = assemble("        BZ $0,Nowhere").unwrap_err();
    assert!(e.message.contains("undefined label"));
}

#[test]
fn assemble_wrong_operand_count() {
    let e = assemble("        ADD $1,$2").unwrap_err();
    assert!(e.message.contains("expected 3 operands"));
}

#[test]
fn assemble_invalid_register() {
    let e = assemble("        ADD $1,$2,bad").unwrap_err();
    assert!(e.message.contains("undefined label"));
}

#[test]
fn assemble_error_display() {
    let e = AssembleError { line: 5, message: "test error".into() };
    assert_eq!(format!("{}", e), "line 6: test error");
}

// ─── Full program (hello.mms equivalent) ───

#[test]
fn assemble_hello_world_program() {
    let src = "\
        SETL    $1,0\n\
        GETA    $255,String\n\
        TRAP    0,1,1\n\
        SETL    $2,3\n\
        SETL    $3,4\n\
        ADD     $4,$2,$3\n\
        SETL    $10,5\n\
Loop    SUB     $10,$10,1\n\
        BNZ     $10,Loop\n\
        TRAP    0,0,0\n\
String  BYTE    \"Hello, World!\\n\\0\"";
    let r = assemble(src).unwrap();

    // 10 instructions (40 bytes) + "Hello, World!\n\0" = 15 bytes
    assert_eq!(r.bytes.len(), 40 + 15);

    // First instruction: SETL $1,0
    let w0 = u32::from_be_bytes(r.bytes[0..4].try_into().unwrap());
    assert_eq!(w0 >> 24, op::SETL as u32);

    // Last instruction before data: TRAP 0,0,0
    let w9 = u32::from_be_bytes(r.bytes[36..40].try_into().unwrap());
    assert_eq!(w9, (op::TRAP as u32) << 24);

    // String data starts at offset 40
    assert_eq!(&r.bytes[40..53], b"Hello, World!");
    assert_eq!(r.bytes[53], b'\n');
    assert_eq!(r.bytes[54], 0);
}

// ─── Case insensitivity ───

#[test]
fn assemble_case_insensitive_mnemonic() {
    let r1 = assemble("        setl $1,1").unwrap();
    let r2 = assemble("        SETL $1,1").unwrap();
    assert_eq!(r1.bytes, r2.bytes);
}

// ─── Aliases ───

#[test]
fn assemble_set_register() {
    // SET $1,$2  -> ORI $1,$2,0
    let r = assemble("        SET $1,$2").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::ORI as u32) << 24) | (1 << 16) | (2 << 8));
}

#[test]
fn assemble_set_immediate() {
    // SET $1,1000  -> SETL $1,1000
    let r = assemble("        SET $1,1000").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 1000);
}

#[test]
fn assemble_set_case_insensitive() {
    let r = assemble("        set $3,$4").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::ORI as u32) << 24) | (3 << 16) | (4 << 8));
}

#[test]
fn assemble_lda() {
    // LDA $1,$2,$3  -> ADDU $1,$2,$3
    let r = assemble("        LDA $1,$2,$3").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::ADDU as u32) << 24) | (1 << 16) | (2 << 8) | 3);
}

#[test]
fn assemble_lda_immediate() {
    // LDA $1,$2,8  -> ADDUI $1,$2,8
    let r = assemble("        LDA $1,$2,8").unwrap();
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::ADDUI as u32) << 24) | (1 << 16) | (2 << 8) | 8);
}

// ─── IS pseudo-instruction ───

#[test]
fn assemble_is_constant() {
    // N IS 10 defines N=10, then SETL $1,N should use N as label resolving to 10
    // But IS defines a symbol whose value is 10, used as an immediate
    let src = "\
N       IS      10\n\
        SETL    $1,N";
    let r = assemble(src).unwrap();
    // IS produces no bytes, so only 4 bytes for SETL
    assert_eq!(r.bytes.len(), 4);
    // N=10 used as 16-bit immediate in SETL
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 10);
}

#[test]
fn assemble_is_no_bytes() {
    let src = "\
X       IS      42\n\
        TRAP    0,0,0";
    let r = assemble(src).unwrap();
    // IS produces no bytes
    assert_eq!(r.bytes.len(), 4);
}

#[test]
fn assemble_is_requires_label() {
    let e = assemble("        IS 10").unwrap_err();
    assert!(e.message.contains("IS requires a label"));
}

#[test]
fn assemble_is_hex_value() {
    let src = "\
MASK    IS      0xFF\n\
        AND     $1,$2,MASK";
    let r = assemble(src).unwrap();
    assert_eq!(r.bytes.len(), 4);
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    // MASK=255, used as immediate -> ANDI
    assert_eq!(word, ((op::ANDI as u32) << 24) | (1 << 16) | (2 << 8) | 0xFF);
}

#[test]
fn assemble_is_used_in_set() {
    // IS + SET alias together
    let src = "\
VAL     IS      42\n\
        SET     $1,VAL";
    let r = assemble(src).unwrap();
    assert_eq!(r.bytes.len(), 4);
    let word = u32::from_be_bytes(r.bytes[..4].try_into().unwrap());
    assert_eq!(word, ((op::SETL as u32) << 24) | (1 << 16) | 42);
}

// ─── Full program with aliases (fibonacci.mms) ───

#[test]
fn assemble_fibonacci_program() {
    let src = "\
N       IS      10\n\
\n\
        SETL    $0,0\n\
        SETL    $1,1\n\
        SETL    $2,N\n\
        SUB     $2,$2,2\n\
\n\
Loop    ADD     $3,$0,$1\n\
        SET     $0,$1\n\
        SET     $1,$3\n\
        SUB     $2,$2,1\n\
        BNZ     $2,Loop\n\
\n\
        TRAP    0,0,0";
    let r = assemble(src).unwrap();
    // IS produces no bytes; 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4 = 40 bytes (10 instructions)
    assert_eq!(r.bytes.len(), 40);

    // SET $0,$1 -> ORI $0,$1,0
    let w_set0 = u32::from_be_bytes(r.bytes[20..24].try_into().unwrap());
    assert_eq!(w_set0 >> 24, op::ORI as u32);

    // SET $1,$3 -> ORI $1,$3,0
    let w_set1 = u32::from_be_bytes(r.bytes[24..28].try_into().unwrap());
    assert_eq!(w_set1, ((op::ORI as u32) << 24) | (1 << 16) | (3 << 8));
}
