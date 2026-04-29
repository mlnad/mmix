use std::collections::{HashMap, HashSet};
use crate::directive::{Directive, DirectiveTable};
use crate::encode::*;
use crate::parse::*;
use crate::{AssembleResult, AssembleError, Segment};

/// Reserved segment symbols (MMIX Architecture §1.3.2).
///
/// These are pre-seeded into the label table before pass 1 begins.  Any
/// attempt to redefine them with `IS` or a plain label is a hard error.
const RESERVED_SYMBOLS: &[(&str, u64)] = &[
    ("Text_Segment",  0x0000_0000_0000_0000),
    ("Data_Segment",  0x2000_0000_0000_0000),
    ("Pool_Segment",  0x4000_0000_0000_0000),
    ("Stack_Segment", 0x6000_0000_0000_0000),
];

/// Return the MMIX segment index (0=Text, 1=Data, 2=Pool, 3=Stack, ≥4=Kernel)
/// for an absolute address.
///
/// MMIX divides the 64-bit space into 8 equal octants with the high 3 bits.
/// The four user segments occupy octants 0–3 (high bit clear).
#[inline]
fn mmix_segment_of(addr: u64) -> u8 {
    (addr >> 61) as u8
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

/// Strip comment (% or ;) and trim
fn strip_comment(line: &str) -> &str {
    let line = if let Some(pos) = line.find('%') { &line[..pos] } else { line };
    let line = if let Some(pos) = line.find(';') { &line[..pos] } else { line };
    line.trim()
}

fn build_predefined_labels() -> HashMap<String, u64> {
    RESERVED_SYMBOLS.iter()
        .map(|&(name, addr)| (name.to_owned(), addr))
        .collect()
}

/// All mutable state shared between assembler passes.
///
/// Fields are grouped by which pass writes them and which reads them:
///
/// | Field            | Written by | Read by      |
/// |------------------|------------|--------------|
/// | `labels`         | pass 1     | pass 2+      |
/// | `entry_addr`     | pass 1     | pass 2+      |
/// | `seg_bufs`       | pass 2     | `assemble()` |
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
    /// Per-segment byte buffers keyed by MMIX segment index.
    seg_bufs:           HashMap<u8, Segment>,
    /// MMIX segment index currently being emitted into.
    cur_seg_idx:        u8,
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
            labels:         build_predefined_labels(),
            entry_addr:     None,
            seg_bufs:       HashMap::new(),
            cur_seg_idx:    0,
            line_to_offset: HashMap::new(),
            offset_to_line: HashMap::new(),
        }
    }

    /// Materialize non-empty segment buffers into sorted output segments.
    fn finish_segments(&mut self) -> Vec<Segment> {
        let mut out = Vec::new();
        for (_idx, seg) in std::mem::take(&mut self.seg_bufs) {
            out.push(seg);
        }
        out.sort_unstable_by_key(|s| s.base);
        out
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

        // Duplicate / reserved-symbol check applies to every line before anything else.
        if let Some(ref lbl) = label_owned {
            let is_reserved = RESERVED_SYMBOLS.iter().any(|&(n, _)| n == lbl.as_str());
            if is_reserved {
                return Err(AssembleError {
                    line:    line_idx,
                    message: format!("'{}' is a reserved segment symbol and cannot be redefined", lbl),
                });
            }
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
                let is_reserved = RESERVED_SYMBOLS.iter().any(|&(n, _)| n == lbl.as_str());
                if is_reserved {
                    return Err(AssembleError {
                        line:    line_idx,
                        message: format!("'{}' is a reserved segment symbol and cannot be redefined", lbl),
                    });
                }
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
/// `cur_offset` tracks the absolute address of the next byte. Each MMIX
/// segment has its own grow-only buffer, so `LOC` can switch between segments
/// and later return to a previous segment as long as it doesn't move backward
/// inside that same segment.
///
/// **Input**:  `ctx.labels`, `ctx.entry_addr`
/// **Output**: `ctx.seg_bufs`, `ctx.line_to_offset`, `ctx.offset_to_line`
fn pass_emit_code(ctx: &mut PassContext) -> Result<Vec<Segment>, AssembleError> {
    let entry = ctx.entry_addr.unwrap_or(0);
    let mut cur_offset: u64 = entry;
    ctx.cur_seg_idx = mmix_segment_of(entry);
    ctx.seg_bufs.insert(ctx.cur_seg_idx, Segment { base: entry, bytes: Vec::new() });

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

            // LOC: move the location counter.
            //
            // Progress is checked per MMIX segment. Moving backward inside the
            // same segment is rejected; switching to another segment and later
            // returning to this one is allowed.
            Some(Directive::Loc) => {
                let new_addr = resolve_label_or_number(args_str, &ctx.labels, cur_offset)
                    .map_err(|e| AssembleError { line: line_idx, message: e })?;

                let new_seg_idx = mmix_segment_of(new_addr);
                let seg = ctx.seg_bufs.entry(new_seg_idx)
                    .or_insert_with(|| Segment { base: new_addr, bytes: Vec::new() });
                let seg_end = seg.base + seg.bytes.len() as u64;

                if new_addr < seg_end {
                    return Err(AssembleError {
                        line: line_idx,
                        message: format!(
                            "LOC {:#x} is before current position in segment ({:#x})",
                            new_addr, seg_end
                        ),
                    });
                }
                if new_addr > seg_end {
                    let gap = (new_addr - seg_end) as usize;
                    seg.bytes.extend(std::iter::repeat(0u8).take(gap));
                }

                ctx.cur_seg_idx = new_seg_idx;
                cur_offset = new_addr;
                continue;
            }

            Some(Directive::Byte) => {
                let data = emit_data(args_str, 1, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.seg_bufs.get_mut(&ctx.cur_seg_idx).expect("active segment must exist")
                    .bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Wyde) => {
                let data = emit_data(args_str, 2, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.seg_bufs.get_mut(&ctx.cur_seg_idx).expect("active segment must exist")
                    .bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Tetra) => {
                let data = emit_data(args_str, 4, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.seg_bufs.get_mut(&ctx.cur_seg_idx).expect("active segment must exist")
                    .bytes.extend_from_slice(&data);
                continue;
            }
            Some(Directive::Octa) => {
                let data = emit_data(args_str, 8, line_idx)?;
                ctx.line_to_offset.insert(line_idx, cur_offset);
                ctx.offset_to_line.insert(cur_offset, line_idx);
                cur_offset += data.len() as u64;
                ctx.seg_bufs.get_mut(&ctx.cur_seg_idx).expect("active segment must exist")
                    .bytes.extend_from_slice(&data);
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
        ctx.seg_bufs.get_mut(&ctx.cur_seg_idx).expect("active segment must exist")
            .bytes.extend_from_slice(&word.to_be_bytes());
        cur_offset += 4;
    }

    Ok(ctx.finish_segments())
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
    let segments = pass_emit_code(&mut ctx)?;
    Ok(AssembleResult {
        segments,
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
