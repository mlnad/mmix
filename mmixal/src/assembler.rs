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

/// All mutable state shared between assembler passes.
///
/// Fields are grouped by which pass writes them and which reads them:
///
/// | Field            | Written by | Read by      |
/// |------------------|------------|--------------|
/// | `labels`         | pass 1     | pass 2+      |
/// | `entry_addr`     | pass 1     | pass 2+      |
/// | `bytes`          | pass 2     | `assemble()` |
/// | `line_to_offset` | pass 2     | `assemble()` |
/// | `offset_to_line` | pass 2     | `assemble()` |
///
/// # Adding a new pass
///
/// 1. Add any needed fields to `PassContext`.
/// 2. Write `fn passN_name(ctx: &mut PassContext) -> Result<(), AssembleError>`.
/// 3. Call `passN_name(&mut ctx)?;` at the right position in `assemble()`.
pub(crate) struct PassContext {
    // ── read-only after new() ─────────────────────────────────────────────
    pub lines:      Vec<String>,
    pub directives: DirectiveTable,
    pub optable:    HashMap<&'static str, InstrEntry>,
    pub mnemonics:  HashSet<String>,

    // ── pass 1 output → pass 2 input ─────────────────────────────────────
    /// Symbol table: label / IS-constant → absolute address or value.
    pub labels:     HashMap<String, u64>,
    /// Address of the first `LOC`; `None` means no `LOC` was seen.
    pub entry_addr: Option<u64>,

    // ── pass 2 output → `assemble()` result ──────────────────────────────
    pub bytes:          Vec<u8>,
    pub line_to_offset: HashMap<usize, u64>,
    pub offset_to_line: HashMap<u64, usize>,
}

impl PassContext {
    fn new(source: &str) -> Self {
        let optable   = build_opcode_table();
        let mnemonics = build_mnemonic_set(&optable);
        Self {
            lines:          source.lines().map(str::to_owned).collect(),
            directives:     DirectiveTable::new(),
            optable,
            mnemonics,
            labels:         HashMap::new(),
            entry_addr:     None,
            bytes:          Vec::new(),
            line_to_offset: HashMap::new(),
            offset_to_line: HashMap::new(),
        }
    }
}

/// Scan every source line to build `ctx.labels` and set `ctx.entry_addr`.
///
/// The location counter (`lc`) is local to this pass — an absolute memory
/// address starting at 0 until a `LOC` directive is seen.  No bytes are
/// emitted; only the symbol table and entry address are produced.
///
/// **Output**: `ctx.labels`, `ctx.entry_addr`
fn pass_collect_labels(ctx: &mut PassContext) -> Result<(), AssembleError> {
    let mut lc: u64 = 0;

    for line_idx in 0..ctx.lines.len() {
        let raw_line = &ctx.lines[line_idx];
        let line     = strip_comment(raw_line);
        if line.is_empty() {
            continue;
        }

        let (label_opt, rest) = extract_label(line, &ctx.mnemonics);
        let label_owned: Option<String> = label_opt.map(str::to_owned);

        // Duplicate-label check applies to every line before anything else.
        if let Some(ref lbl) = label_owned {
            if ctx.labels.contains_key(lbl.as_str()) {
                return Err(AssembleError {
                    line:    line_idx,
                    message: format!("duplicate label '{}'", lbl),
                });
            }
        }

        let rest = rest.trim();
        if rest.is_empty() {
            // Label-only line: symbol gets current lc.
            if let Some(lbl) = label_owned { ctx.labels.insert(lbl, lc); }
            continue;
        }

        let mnem_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let mnem     = rest[..mnem_end].to_uppercase();
        let args_str = rest[mnem_end..].trim();

        // `.cloned()` ends the borrow of `ctx.directives` before we mutate
        // `ctx.labels` / `ctx.entry_addr` inside the match arms.
        match ctx.directives.get(&mnem) {
            // IS: the label itself is the defined symbol; no bytes emitted.
            Some(Directive::Is) => {
                let lbl = label_owned.ok_or_else(|| AssembleError {
                    line: line_idx, message: "IS requires a label".into(),
                })?;
                let val = resolve_label_or_number(args_str, &ctx.labels, lc)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;
                ctx.labels.insert(lbl, val);
                continue;
            }

            // LOC: set the location counter; record entry_addr on first use.
            Some(Directive::Loc) => {
                let new_lc = resolve_label_or_number(args_str, &ctx.labels, lc)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;
                lc = new_lc;
                if ctx.entry_addr.is_none() {
                    ctx.entry_addr = Some(lc);
                }
                // A label on a LOC line resolves to the *new* lc.
                if let Some(lbl) = label_owned { ctx.labels.insert(lbl, lc); }
                continue;
            }

            _ => {}
        }

        // For all other lines: register any label at the current lc.
        if let Some(lbl) = label_owned { ctx.labels.insert(lbl, lc); }

        // Advance lc by however many bytes this line emits.
        match ctx.directives.get(&mnem) {
            Some(Directive::Byte)  => { lc += count_data_bytes(args_str, 1, line_idx)?; }
            Some(Directive::Wyde)  => { lc += count_data_bytes(args_str, 2, line_idx)?; }
            Some(Directive::Tetra) => { lc += count_data_bytes(args_str, 4, line_idx)?; }
            Some(Directive::Octa)  => { lc += count_data_bytes(args_str, 8, line_idx)?; }
            _                      => { lc += 4; } // every real instruction is 4 bytes
        }
    }
    Ok(())
}

/// Emit machine-code bytes and populate the debug-info address maps.
///
/// Reads `ctx.labels` and `ctx.entry_addr` (written by pass 1).
/// `cur_offset` is local and starts at `entry_addr` (or 0), so every address
/// written into `line_to_offset` / `offset_to_line` is absolute and matches
/// the PC values the Machine produces at runtime.
///
/// **Input**:  `ctx.labels`, `ctx.entry_addr`
/// **Output**: `ctx.bytes`, `ctx.line_to_offset`, `ctx.offset_to_line`
fn pass_emit_code(ctx: &mut PassContext) -> Result<(), AssembleError> {
    let mut cur_offset: u64 = ctx.entry_addr.unwrap_or(0);

    for line_idx in 0..ctx.lines.len() {
        let raw_line = &ctx.lines[line_idx];
        let line     = strip_comment(raw_line);
        if line.is_empty() {
            continue;
        }

        let (_label, rest) = extract_label(line, &ctx.mnemonics);
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }

        let mnem_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let mnem     = rest[..mnem_end].to_uppercase();
        let args_str = rest[mnem_end..].trim();

        match ctx.directives.get(&mnem) {
            // IS was fully resolved in pass 1.
            Some(Directive::Is) => { continue; }

            // LOC: validate forward-only movement, then zero-pad the gap.
            Some(Directive::Loc) => {
                let new_addr = resolve_label_or_number(args_str, &ctx.labels, cur_offset)
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
                ctx.bytes.extend(std::iter::repeat(0u8).take(gap));
                cur_offset = new_addr;
                continue;
            }

            Some(Directive::Byte) => {
                let data = emit_data(args_str, 1, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Wyde) => {
                let data = emit_data(args_str, 2, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Tetra) => {
                let data = emit_data(args_str, 4, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Octa) => {
                let data = emit_data(args_str, 8, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.bytes.extend_from_slice(&data);
                continue;
            }

            _ => {}
        }

        ctx.line_to_offset.insert(line_idx, cur_offset);
        ctx.offset_to_line.insert(cur_offset, line_idx);

        // `InstrEntry` is Copy, so dereferencing gives an owned value and
        // releases the borrow of `ctx.optable` before we access `ctx.labels`.
        let entry = *ctx.optable.get(mnem.as_str())
            .ok_or_else(|| AssembleError {
                line:    line_idx,
                message: format!("unknown instruction '{}'", mnem),
            })?;

        let args: Vec<&str> = if args_str.is_empty() {
            vec![]
        } else {
            args_str.split(',').collect()
        };

        let word = match entry {
            InstrEntry::Real { base_op, format } => {
                encode_instruction(base_op, format, &args, cur_offset, &ctx.labels, line_idx)?
            }
            InstrEntry::Alias(lowering) => {
                encode_alias(lowering, &args, cur_offset, &ctx.labels, line_idx)?
            }
        };
        ctx.bytes.extend_from_slice(&word.to_be_bytes());
        cur_offset += 4;
    }
    Ok(())
}

/// Assemble MMIX source text into a binary together with debug metadata.
///
/// The pipeline is a sequence of named passes over [`PassContext`].  Each pass
/// is called directly in dependency order so the data flow and call sequence
/// are explicit.
///
/// To add a new pass:
/// 1. Add any needed fields to `PassContext`.
/// 2. Write `fn pass_<name>(ctx: &mut PassContext) -> Result<(), AssembleError>`.
/// 3. Insert `pass_<name>(&mut ctx)?;` at the right position below.
pub fn assemble(source: &str) -> Result<AssembleResult, AssembleError> {
    let mut ctx = PassContext::new(source);
    pass_collect_labels(&mut ctx)?;
    pass_emit_code(&mut ctx)?;
    Ok(AssembleResult {
        bytes:          ctx.bytes,
        line_to_offset: ctx.line_to_offset,
        offset_to_line: ctx.offset_to_line,
        entry_addr:     ctx.entry_addr.unwrap_or(0),
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
