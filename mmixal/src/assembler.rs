use std::collections::{HashMap, HashSet};
use crate::encode::*;
use crate::parse::*;
use crate::{AssembleResult, AssembleError};

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

/// Strip comment (% or ;) and trim
fn strip_comment(line: &str) -> &str {
    let line = if let Some(pos) = line.find('%') { &line[..pos] } else { line };
    let line = if let Some(pos) = line.find(';') { &line[..pos] } else { line };
    line.trim()
}

pub fn assemble(source: &str) -> Result<AssembleResult, AssembleError> {
    let optable = build_opcode_table();
    let mnemonics = build_mnemonic_set(&optable);
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
        let (label, rest) = extract_label(line, &mnemonics);

        if let Some(lbl) = label {
            if labels.contains_key(lbl) {
                return Err(AssembleError {
                    line: line_idx,
                    message: format!("duplicate label '{}'", lbl),
                });
            }
        }

        let rest = rest.trim();
        if rest.is_empty() {
            if let Some(lbl) = label {
                labels.insert(lbl.to_string(), offset);
            }
            continue;
        }

        let mnem = rest.split_whitespace().next().unwrap().to_uppercase();

        // IS pseudo-instruction: defines a symbolic constant (no bytes emitted)
        if mnem == "IS" {
            if let Some(lbl) = label {
                let args_str = rest[mnem.len()..].trim();
                let val = resolve_label_or_number(args_str, &labels, offset)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;
                labels.insert(lbl.to_string(), val);
            } else {
                return Err(AssembleError {
                    line: line_idx,
                    message: "IS requires a label".into(),
                });
            }
            continue;
        }

        // For non-IS lines, register label at current offset
        if let Some(lbl) = label {
            labels.insert(lbl.to_string(), offset);
        }

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

        let (_label, rest) = extract_label(line, &mnemonics);
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }

        let mnem_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let mnem = rest[..mnem_end].to_uppercase();
        let args_str = rest[mnem_end..].trim();

        // Skip IS in pass 2 (already handled in pass 1)
        if mnem == "IS" {
            continue;
        }

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

        let entry = optable.get(mnem.as_str())
            .ok_or_else(|| AssembleError {
                line: line_idx,
                message: format!("unknown instruction '{}'", mnem),
            })?;

        let args: Vec<&str> = if args_str.is_empty() {
            vec![]
        } else {
            args_str.split(',').collect()
        };

        let word = match *entry {
            InstrEntry::Real { base_op, format } => {
                encode_instruction(base_op, format, &args, cur_offset, &labels, line_idx)?
            }
            InstrEntry::Alias(lowering) => {
                encode_alias(lowering, &args, cur_offset, &labels, line_idx)?
            }
        };
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

fn extract_label<'a>(line: &'a str, mnemonics: &HashSet<String>) -> (Option<&'a str>, &'a str) {
    let line_trimmed = line.trim_end();

    // If line starts with whitespace, no label
    if line_trimmed.is_empty() || line.starts_with(char::is_whitespace) {
        return (None, line_trimmed);
    }

    let first_word_end = line_trimmed.find(|c: char| c.is_whitespace()).unwrap_or(line_trimmed.len());
    let first_word = &line_trimmed[..first_word_end];

    // Check if it's a known mnemonic, alias, or pseudo-op (no label)
    let upper = first_word.trim_end_matches(':').to_uppercase();
    if mnemonics.contains(&upper) {
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
