use std::collections::HashMap;

mod assembler;
pub mod binary;

pub use assembler::assemble;

#[derive(Debug)]
pub struct AssembleResult {
    pub bytes: Vec<u8>,
    /// source line index -> byte offset in `bytes`
    pub line_to_offset: HashMap<usize, u64>,
    /// byte offset -> source line index
    pub offset_to_line: HashMap<u64, usize>,
    pub entry_addr: u64,
}

#[derive(Debug)]
pub struct AssembleError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for AssembleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line + 1, self.message)
    }
}

/// Debug information embedded in .mmb files, mapping binary offsets to source lines.
#[derive(Debug)]
pub struct DebugInfo {
    /// source line index -> byte offset in code
    pub line_to_offset: HashMap<usize, u64>,
    /// byte offset -> source line index
    pub offset_to_line: HashMap<u64, usize>,
    /// source file name
    pub source_file: String,
    /// embedded source code lines
    pub source_lines: Vec<String>,
}

#[cfg(test)]
mod tests;
