use std::collections::{HashMap, HashSet};
use crate::directive::{Directive, DirectiveTable};
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
    let directives = DirectiveTable::new();
    let lines: Vec<&str> = source.lines().collect();

    // --- Pass 1: collect labels and advance the location counter (lc) ---
    //
    // `lc` is always an *absolute* memory address.  When no LOC directive has
    // been seen yet it starts at 0 (matching previous behaviour).
    // `entry_addr` records the address of the first LOC, which is where the
    // loader will map the first byte of `bytes`.
    let mut labels: HashMap<String, u64> = HashMap::new();
    let mut lc: u64 = 0;
    let mut entry_addr: Option<u64> = None;

    for (line_idx, &raw_line) in lines.iter().enumerate() {
        let line = strip_comment(raw_line);
        if line.is_empty() {
            continue;
        }

        let (label, rest) = extract_label(line, &mnemonics);

        // Duplicate-label check applies to every line before anything else.
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
            // Label-only line: symbol gets current lc.
            if let Some(lbl) = label {
                labels.insert(lbl.to_string(), lc);
            }
            continue;
        }

        let mnem_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let mnem = rest[..mnem_end].to_uppercase();
        let args_str = rest[mnem_end..].trim();

        match directives.get(&mnem) {
            // IS: the label itself is the defined symbol; no bytes emitted.
            Some(Directive::Is) => {
                let lbl = label.ok_or_else(|| AssembleError {
                    line: line_idx,
                    message: "IS requires a label".into(),
                })?;
                let val = resolve_label_or_number(args_str, &labels, lc)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;
                labels.insert(lbl.to_string(), val);
                continue;
            }

            // LOC: set the location counter; record entry_addr on first use.
            Some(Directive::Loc) => {
                let new_lc = resolve_label_or_number(args_str, &labels, lc)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;
                lc = new_lc;
                if entry_addr.is_none() {
                    entry_addr = Some(lc);
                }
                // A label on a LOC line resolves to the *new* lc.
                if let Some(lbl) = label {
                    labels.insert(lbl.to_string(), lc);
                }
                continue;
            }

            _ => {}
        }

        // For all other lines: register any label at the current lc.
        if let Some(lbl) = label {
            labels.insert(lbl.to_string(), lc);
        }

        // Advance lc by however many bytes this line emits.
        match directives.get(&mnem) {
            Some(Directive::Byte)  => { lc += count_data_bytes(args_str, 1, line_idx)?; }
            Some(Directive::Wyde)  => { lc += count_data_bytes(args_str, 2, line_idx)?; }
            Some(Directive::Tetra) => { lc += count_data_bytes(args_str, 4, line_idx)?; }
            Some(Directive::Octa)  => { lc += count_data_bytes(args_str, 8, line_idx)?; }
            _                      => { lc += 4; } // every real instruction is 4 bytes
        }
    }

    // --- Pass 2: emit bytes ---
    //
    // `cur_offset` tracks the absolute address of the next byte to be written.
    // It begins at `entry_addr` (0 if no LOC was seen), so that all debug-info
    // addresses stored in `line_to_offset` / `offset_to_line` are absolute and
    // match the PC values the Machine will produce at runtime.
    let entry_addr_val = entry_addr.unwrap_or(0);
    let mut bytes: Vec<u8> = Vec::new();
    let mut line_to_offset: HashMap<usize, u64> = HashMap::new();
    let mut offset_to_line: HashMap<u64, usize> = HashMap::new();
    let mut cur_offset: u64 = entry_addr_val;

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

        match directives.get(&mnem) {
            // IS was fully resolved in pass 1.
            Some(Directive::Is) => { continue; }

            // LOC: validate forward-only movement, then zero-pad the gap.
            Some(Directive::Loc) => {
                let new_addr = resolve_label_or_number(args_str, &labels, cur_offset)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;
                if new_addr < cur_offset {
                    return Err(AssembleError {
                        line: line_idx,
                        message: format!(
                            "LOC {:#x} is before current position {:#x}",
                            new_addr, cur_offset
                        ),
                    });
                }
                let gap = (new_addr - cur_offset) as usize;
                bytes.extend(std::iter::repeat(0u8).take(gap));
                cur_offset = new_addr;
                continue;
            }

            Some(Directive::Byte) => {
                let data = emit_data(args_str, 1, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Wyde) => {
                let data = emit_data(args_str, 2, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Tetra) => {
                let data = emit_data(args_str, 4, line_idx)?;
                line_to_offset.insert(line_idx, cur_offset);
                offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Octa) => {
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
        entry_addr: entry_addr_val,
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

fn count_data_bytes(args: &str, unit_size: u64, line: usize) -> Result<u64, AssembleError> {
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
