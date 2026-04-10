use std::collections::HashMap;
use mmix::op;
use crate::{AssembleResult, AssembleError};

/// Map mnemonic -> (base opcode, OperandKind)
#[derive(Debug, Clone, Copy, PartialEq)]
enum OperandKind {
    /// $X, $Y, $Z  or  $X, $Y, Z  (auto immediate variant: base_op + 1)
    ThreeReg,
    /// $X, YZ  (16-bit immediate, e.g. SETH, SETL)
    RegImm16,
    /// $X, YZ  (branch relative: forward label or offset)
    Branch,
    /// TRAP X,Y,Z
    Trap,
    /// GET $X, special_reg_number
    Get,
    /// PUT special_reg_number, $Z or imm (PUT/PUTI)
    Put,
    /// JMP label (24-bit relative)
    Jump,
    /// $X, $Y, $Z or $X, $Y, Z  -- NEG style (Y is inline constant)
    NegStyle,
}

fn build_opcode_table() -> HashMap<&'static str, (u8, OperandKind)> {
    use OperandKind::*;
    let mut m = HashMap::new();

    // Arithmetic
    for &(name, base) in &[
        ("ADD", op::ADD), ("SUB", op::SUB), ("MUL", op::MUL), ("DIV", op::DIV),
        ("CMP", op::CMP),
        ("SL", op::SL), ("SR", op::SR), ("SRU", op::SRU),
        ("AND", op::AND), ("OR", op::OR), ("XOR", op::XOR),
        ("CSZ", op::CSZ), ("CSNZ", op::CSNZ),
    ] {
        m.insert(name, (base, ThreeReg));
    }

    m.insert("NEG", (op::NEG, NegStyle));

    // Memory
    for &(name, base) in &[
        ("LDB", op::LDB), ("LDBU", op::LDBU), ("LDW", op::LDW),
        ("LDT", op::LDT), ("LDO", op::LDO),
        ("STB", op::STB), ("STW", op::STW), ("STT", op::STT), ("STO", op::STO),
        ("GO", op::GO),
    ] {
        m.insert(name, (base, ThreeReg));
    }

    // Constant loads / ORx
    for &(name, base) in &[
        ("SETH", op::SETH), ("SETMH", op::SETMH), ("SETML", op::SETML), ("SETL", op::SETL),
        ("ORH", op::ORH), ("ORMH", op::ORMH), ("ORML", op::ORML), ("ORL", op::ORL),
    ] {
        m.insert(name, (base, RegImm16));
    }

    // Branches
    for &(name, base) in &[
        ("BZ", op::BZ), ("BNZ", op::BNZ), ("BP", op::BP), ("BN", op::BN), ("BNN", op::BNN),
        ("GETA", op::GETA),
    ] {
        m.insert(name, (base, Branch));
    }

    m.insert("JMP", (op::JMP, Jump));
    m.insert("TRAP", (op::TRAP, Trap));
    m.insert("GET", (op::GET, Get));
    m.insert("PUT", (op::PUT, Put));

    m
}

/// Parse a register operand like "$0" or "$255", returns register number
fn parse_reg(s: &str) -> Result<u8, String> {
    let s = s.trim();
    if !s.starts_with('$') {
        return Err(format!("expected register (e.g. $0), got '{}'", s));
    }
    s[1..].parse::<u8>().map_err(|_| format!("invalid register '{}'", s))
}

/// Parse a number: decimal or 0x hex
fn parse_number(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|e| format!("invalid hex '{}': {}", s, e))
    } else if s.starts_with('#') {
        u64::from_str_radix(&s[1..], 16).map_err(|e| format!("invalid hex '{}': {}", s, e))
    } else {
        // Try parsing as possibly-negative decimal
        if s.starts_with('-') {
            let v: i64 = s.parse().map_err(|e| format!("invalid number '{}': {}", s, e))?;
            Ok(v as u64)
        } else {
            s.parse::<u64>().map_err(|e| format!("invalid number '{}': {}", s, e))
        }
    }
}

/// Parse a register OR immediate. Returns (value, is_immediate).
fn parse_reg_or_imm(s: &str) -> Result<(u8, bool), String> {
    let s = s.trim();
    if s.starts_with('$') {
        Ok((parse_reg(s)?, false))
    } else {
        let v = parse_number(s)?;
        if v > 255 {
            return Err(format!("immediate {} out of range 0..255", v));
        }
        Ok((v as u8, true))
    }
}

/// Special register name -> encoding
fn parse_special_reg(s: &str) -> Result<u8, String> {
    let s = s.trim().to_lowercase();
    let mapping = [
        ("ra", 21), ("rb", 0), ("rc", 8), ("rd", 1), ("re", 2),
        ("rf", 22), ("rg", 19), ("rh", 3), ("ri", 12), ("rj", 4),
        ("rk", 15), ("rl", 20), ("rm", 5), ("rn", 9), ("ro", 10),
        ("rp", 23), ("rq", 16), ("rr", 6), ("rs", 11), ("rt", 13),
        ("ru", 17), ("rv", 18), ("rw", 24), ("rx", 25), ("ry", 26),
        ("rz", 27), ("rbb", 7), ("rtt", 14), ("rww", 28),
        ("rxx", 29), ("ryy", 30), ("rzz", 31),
    ];
    for &(name, enc) in &mapping {
        if s == name {
            return Ok(enc);
        }
    }
    // Also accept raw number
    s.parse::<u8>().map_err(|_| format!("unknown special register '{}'", s))
}

/// Strip comment (% or ;) and trim
fn strip_comment(line: &str) -> &str {
    let line = if let Some(pos) = line.find('%') { &line[..pos] } else { line };
    let line = if let Some(pos) = line.find(';') { &line[..pos] } else { line };
    line.trim()
}

pub fn assemble(source: &str) -> Result<AssembleResult, AssembleError> {
    let optable = build_opcode_table();
    let lines: Vec<&str> = source.lines().collect();

    // --- Pass 1: collect labels and compute offsets ---
    let mut labels: HashMap<String, u64> = HashMap::new();
    let mut offset: u64 = 0;

    for (line_idx, &raw_line) in lines.iter().enumerate() {
        let line = strip_comment(raw_line);
        if line.is_empty() {
            continue;
        }

        // Check for label: "Name  INSTR ..." or "Name:"
        let (label, rest) = extract_label(line);

        if let Some(lbl) = label {
            if labels.contains_key(lbl) {
                return Err(AssembleError {
                    line: line_idx,
                    message: format!("duplicate label '{}'", lbl),
                });
            }
            labels.insert(lbl.to_string(), offset);
        }

        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }

        let mnem = rest.split_whitespace().next().unwrap().to_uppercase();

        // Data pseudo-instructions
        match mnem.as_str() {
            "BYTE" => {
                let args = &rest[mnem.len()..].trim();
                offset += count_data_bytes(args, 1, line_idx)?;
            }
            "WYDE" => {
                let args = &rest[mnem.len()..].trim();
                offset += count_data_bytes(args, 2, line_idx)?;
            }
            "TETRA" => {
                let args = &rest[mnem.len()..].trim();
                offset += count_data_bytes(args, 4, line_idx)?;
            }
            "OCTA" => {
                let args = &rest[mnem.len()..].trim();
                offset += count_data_bytes(args, 8, line_idx)?;
            }
            _ => {
                // Regular instruction: always 4 bytes
                offset += 4;
            }
        }
    }

    // --- Pass 2: emit bytes ---
    let mut bytes: Vec<u8> = Vec::new();
    let mut line_to_offset: HashMap<usize, u64> = HashMap::new();
    let mut offset_to_line: HashMap<u64, usize> = HashMap::new();
    let mut cur_offset: u64 = 0;

    for (line_idx, &raw_line) in lines.iter().enumerate() {
        let line = strip_comment(raw_line);
        if line.is_empty() {
            continue;
        }

        let (_label, rest) = extract_label(line);
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }

        let mnem_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let mnem = rest[..mnem_end].to_uppercase();
        let args_str = rest[mnem_end..].trim();

        match mnem.as_str() {
            "BYTE" => {
                let data = emit_data(args_str, 1, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            "WYDE" => {
                let data = emit_data(args_str, 2, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            "TETRA" => {
                let data = emit_data(args_str, 4, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            "OCTA" => {
                let data = emit_data(args_str, 8, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            _ => {}
        }

        line_to_offset.insert(line_idx, cur_offset);
        offset_to_line.insert(cur_offset, line_idx);

        let (base_op, kind) = optable.get(mnem.as_str())
            .ok_or_else(|| AssembleError {
                line: line_idx,
                message: format!("unknown instruction '{}'", mnem),
            })?;

        let args: Vec<&str> = if args_str.is_empty() {
            vec![]
        } else {
            args_str.split(',').collect()
        };

        let word = encode_instruction(*base_op, *kind, &args, cur_offset, &labels, line_idx)?;
        bytes.extend_from_slice(&word.to_be_bytes());
        cur_offset += 4;
    }

    Ok(AssembleResult {
        bytes,
        line_to_offset,
        offset_to_line,
        entry_addr: 0,
    })
}

fn extract_label(line: &str) -> (Option<&str>, &str) {
    let line_trimmed = line.trim_end();

    // If line starts with whitespace, no label
    if line_trimmed.is_empty() || line.starts_with(char::is_whitespace) {
        return (None, line_trimmed);
    }

    let first_word_end = line_trimmed.find(|c: char| c.is_whitespace()).unwrap_or(line_trimmed.len());
    let first_word = &line_trimmed[..first_word_end];

    // Check if it's a known mnemonic or data directive (no label)
    let upper = first_word.trim_end_matches(':').to_uppercase();
    let known_mnemonics = [
        "ADD", "SUB", "MUL", "DIV", "CMP", "NEG",
        "SL", "SR", "SRU", "AND", "OR", "XOR",
        "CSZ", "CSNZ",
        "LDB", "LDBU", "LDW", "LDT", "LDO",
        "STB", "STW", "STT", "STO",
        "SETH", "SETMH", "SETML", "SETL",
        "ORH", "ORMH", "ORML", "ORL",
        "BZ", "BNZ", "BP", "BN", "BNN",
        "JMP", "GO", "GETA",
        "TRAP", "GET", "PUT",
        "BYTE", "WYDE", "TETRA", "OCTA",
    ];

    if known_mnemonics.contains(&upper.as_str()) {
        return (None, line_trimmed);
    }

    // It's a label
    let label = first_word.trim_end_matches(':');
    let rest = &line_trimmed[first_word_end..];
    (Some(label), rest)
}

fn count_data_bytes(args: &&str, unit_size: u64, line: usize) -> Result<u64, AssembleError> {
    if args.is_empty() {
        return Err(AssembleError { line, message: "data directive requires arguments".into() });
    }
    // Handle string literals
    if args.starts_with('"') {
        let s = parse_string_literal(args).map_err(|e| AssembleError { line, message: e })?;
        // Pad to unit_size boundary
        let len = s.len() as u64;
        let padded = ((len + unit_size - 1) / unit_size) * unit_size;
        return Ok(padded);
    }
    let count = args.split(',').count() as u64;
    Ok(count * unit_size)
}

fn emit_data(args: &str, unit_size: usize, line: usize) -> Result<Vec<u8>, AssembleError> {
    let mut out = Vec::new();
    if args.starts_with('"') {
        let s = parse_string_literal(args).map_err(|e| AssembleError { line, message: e })?;
        out.extend_from_slice(s.as_bytes());
        // Pad
        while out.len() % unit_size != 0 {
            out.push(0);
        }
        return Ok(out);
    }
    for part in args.split(',') {
        let val = parse_number(part.trim())
            .map_err(|e| AssembleError { line, message: e })?;
        match unit_size {
            1 => out.push(val as u8),
            2 => out.extend_from_slice(&(val as u16).to_be_bytes()),
            4 => out.extend_from_slice(&(val as u32).to_be_bytes()),
            8 => out.extend_from_slice(&val.to_be_bytes()),
            _ => unreachable!(),
        }
    }
    Ok(out)
}

fn parse_string_literal(s: &str) -> Result<String, String> {
    if !s.starts_with('"') {
        return Err("expected string literal".into());
    }
    let end = s[1..].find('"').ok_or("unterminated string literal")?;
    let inner = &s[1..1 + end];
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('0') => result.push('\0'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => return Err("trailing backslash".into()),
            }
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

fn resolve_label_or_number(s: &str, labels: &HashMap<String, u64>, cur_offset: u64) -> Result<u64, String> {
    let s = s.trim();
    if s == "@" {
        return Ok(cur_offset);
    }
    if s.starts_with('$') || s.starts_with('#') || s.starts_with("0x") || s.starts_with("0X")
        || s.starts_with('-') || s.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        parse_number(s)
    } else {
        labels.get(s).copied().ok_or_else(|| format!("undefined label '{}'", s))
    }
}

fn encode_instruction(
    base_op: u8,
    kind: OperandKind,
    args: &[&str],
    cur_offset: u64,
    labels: &HashMap<String, u64>,
    line: usize,
) -> Result<u32, AssembleError> {
    let err = |msg: String| AssembleError { line, message: msg };

    match kind {
        OperandKind::ThreeReg => {
            if args.len() != 3 {
                return Err(err(format!("expected 3 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let y = parse_reg(args[1]).map_err(|e| err(e))?;
            let (z, is_imm) = parse_reg_or_imm(args[2]).map_err(|e| err(e))?;
            let op = if is_imm { base_op + 1 } else { base_op };
            Ok(((op as u32) << 24) | ((x as u32) << 16) | ((y as u32) << 8) | (z as u32))
        }
        OperandKind::NegStyle => {
            if args.len() != 3 {
                return Err(err(format!("expected 3 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let y_val = parse_number(args[1].trim()).map_err(|e| err(e))? as u8;
            let (z, is_imm) = parse_reg_or_imm(args[2]).map_err(|e| err(e))?;
            let op = if is_imm { base_op + 1 } else { base_op };
            Ok(((op as u32) << 24) | ((x as u32) << 16) | ((y_val as u32) << 8) | (z as u32))
        }
        OperandKind::RegImm16 => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let yz = parse_number(args[1].trim()).map_err(|e| err(e))? as u16;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (yz as u32))
        }
        OperandKind::Branch => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let target = resolve_label_or_number(args[1], labels, cur_offset).map_err(|e| err(e))?;
            let diff = target as i64 - cur_offset as i64;
            if diff < 0 {
                // Backward branch
                let offset = ((-diff) / 4) as u64;
                let yz = (0x10000 - offset) as u16;
                Ok((((base_op + 1) as u32) << 24) | ((x as u32) << 16) | (yz as u32))
            } else {
                let yz = (diff / 4) as u16;
                Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (yz as u32))
            }
        }
        OperandKind::Jump => {
            if args.len() != 1 {
                return Err(err(format!("expected 1 operand, got {}", args.len())));
            }
            let target = resolve_label_or_number(args[0], labels, cur_offset).map_err(|e| err(e))?;
            let diff = target as i64 - cur_offset as i64;
            if diff < 0 {
                let offset = ((-diff) / 4) as u64;
                let xyz = (0x1000000u64 - offset) as u32 & 0xFFFFFF;
                Ok((((base_op + 1) as u32) << 24) | xyz)
            } else {
                let xyz = (diff / 4) as u32 & 0xFFFFFF;
                Ok(((base_op as u32) << 24) | xyz)
            }
        }
        OperandKind::Trap => {
            if args.len() != 3 {
                return Err(err(format!("expected 3 operands, got {}", args.len())));
            }
            let x = parse_number(args[0].trim()).map_err(|e| err(e))? as u8;
            let y = parse_number(args[1].trim()).map_err(|e| err(e))? as u8;
            let z = parse_number(args[2].trim()).map_err(|e| err(e))? as u8;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | ((y as u32) << 8) | (z as u32))
        }
        OperandKind::Get => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let sr = parse_special_reg(args[1]).map_err(|e| err(e))?;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (sr as u32))
        }
        OperandKind::Put => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let sr = parse_special_reg(args[0]).map_err(|e| err(e))?;
            let (z, is_imm) = parse_reg_or_imm(args[1]).map_err(|e| err(e))?;
            let op = if is_imm { base_op + 1 } else { base_op };
            Ok(((op as u32) << 24) | ((sr as u32) << 16) | (z as u32))
        }
    }
}
