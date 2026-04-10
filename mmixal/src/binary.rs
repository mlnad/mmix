use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use crate::{AssembleResult, DebugInfo};

const MAGIC_BIN: &[u8; 8] = b"MMIXBIN\0";
const MAGIC_DBG: &[u8; 8] = b"MMIXDBG\0";

/// Save an assembled program to a .mmb file with embedded debug information.
///
/// Format:
///   Header: "MMIXBIN\0" (8) + entry_addr (u64 BE) + code_length (u64 BE)
///   Code:   code_length bytes of machine code
///   Debug:  "MMIXDBG\0" (8) + mapping entries + source file name + source lines
pub fn save_mmb(
    path: &Path,
    asm_result: &AssembleResult,
    source: &str,
    source_file: &str,
) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;

    // Header
    f.write_all(MAGIC_BIN)?;
    f.write_all(&asm_result.entry_addr.to_be_bytes())?;
    f.write_all(&(asm_result.bytes.len() as u64).to_be_bytes())?;

    // Code
    f.write_all(&asm_result.bytes)?;

    // Debug section
    f.write_all(MAGIC_DBG)?;

    // Mapping entries: sorted by line index for deterministic output
    let mut entries: Vec<(usize, u64)> = asm_result.line_to_offset.iter()
        .map(|(&line, &offset)| (line, offset))
        .collect();
    entries.sort_by_key(|&(line, _)| line);

    f.write_all(&(entries.len() as u32).to_be_bytes())?;
    for &(line_idx, offset) in &entries {
        f.write_all(&(line_idx as u32).to_be_bytes())?;
        f.write_all(&offset.to_be_bytes())?;
    }

    // Source file name
    let name_bytes = source_file.as_bytes();
    f.write_all(&(name_bytes.len() as u32).to_be_bytes())?;
    f.write_all(name_bytes)?;

    // Source lines
    let lines: Vec<&str> = source.lines().collect();
    f.write_all(&(lines.len() as u32).to_be_bytes())?;
    for line in &lines {
        let line_bytes = line.as_bytes();
        f.write_all(&(line_bytes.len() as u32).to_be_bytes())?;
        f.write_all(line_bytes)?;
    }

    Ok(())
}

/// Load a .mmb file. Returns (entry_addr, code_bytes, Option<DebugInfo>).
///
/// If the file contains a debug section after the code, it is parsed.
/// Otherwise DebugInfo is None.
pub fn load_mmb(path: &Path) -> io::Result<(u64, Vec<u8>, Option<DebugInfo>)> {
    let data = std::fs::read(path)?;
    let mut pos = 0;

    // Header
    if data.len() < 24 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too short for header"));
    }
    if &data[pos..pos + 8] != MAGIC_BIN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid magic: not a .mmb file"));
    }
    pos += 8;

    let entry_addr = u64::from_be_bytes(data[pos..pos + 8].try_into().unwrap());
    pos += 8;

    let code_length = u64::from_be_bytes(data[pos..pos + 8].try_into().unwrap()) as usize;
    pos += 8;

    if data.len() < pos + code_length {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file truncated: code section incomplete"));
    }

    let code = data[pos..pos + code_length].to_vec();
    pos += code_length;

    // Check for debug section
    if data.len() < pos + 8 || &data[pos..pos + 8] != MAGIC_DBG {
        return Ok((entry_addr, code, None));
    }
    pos += 8;

    // Parse debug section
    let debug_info = parse_debug_section(&data, &mut pos)?;
    Ok((entry_addr, code, Some(debug_info)))
}

fn read_u32(data: &[u8], pos: &mut usize) -> io::Result<u32> {
    if data.len() < *pos + 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected end of debug section"));
    }
    let val = u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(val)
}

fn read_u64(data: &[u8], pos: &mut usize) -> io::Result<u64> {
    if data.len() < *pos + 8 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected end of debug section"));
    }
    let val = u64::from_be_bytes(data[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(val)
}

fn read_bytes<'a>(data: &'a [u8], pos: &mut usize, len: usize) -> io::Result<&'a [u8]> {
    if data.len() < *pos + len {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected end of debug section"));
    }
    let slice = &data[*pos..*pos + len];
    *pos += len;
    Ok(slice)
}

fn parse_debug_section(data: &[u8], pos: &mut usize) -> io::Result<DebugInfo> {
    // Mapping entries
    let n = read_u32(data, pos)? as usize;
    let mut line_to_offset = HashMap::new();
    let mut offset_to_line = HashMap::new();
    for _ in 0..n {
        let line_idx = read_u32(data, pos)? as usize;
        let offset = read_u64(data, pos)?;
        line_to_offset.insert(line_idx, offset);
        offset_to_line.insert(offset, line_idx);
    }

    // Source file name
    let name_len = read_u32(data, pos)? as usize;
    let name_bytes = read_bytes(data, pos, name_len)?;
    let source_file = String::from_utf8(name_bytes.to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid UTF-8 in source file name: {}", e)))?;

    // Source lines
    let m = read_u32(data, pos)? as usize;
    let mut source_lines = Vec::with_capacity(m);
    for _ in 0..m {
        let line_len = read_u32(data, pos)? as usize;
        let line_bytes = read_bytes(data, pos, line_len)?;
        let line = String::from_utf8(line_bytes.to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid UTF-8 in source line: {}", e)))?;
        source_lines.push(line);
    }

    Ok(DebugInfo {
        line_to_offset,
        offset_to_line,
        source_file,
        source_lines,
    })
}
