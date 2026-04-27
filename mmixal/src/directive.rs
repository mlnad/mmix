use std::collections::HashMap;

/// All assembler pseudo-instructions (directives).
///
/// Directives are recognized during parsing but never emit machine-code words
/// directly; instead they control assembler state (location counter, symbol
/// table, raw data output).
///
/// Adding a new pseudo-instruction means adding a variant here and a handler
/// in [`DirectiveTable`] — the assembler never needs its own `match` arm for
/// the name string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Directive {
    /// `LOC expr` — set the location counter to an absolute address.
    ///
    /// The first `LOC` in a translation unit also determines `entry_addr`.
    Loc,

    /// `IS expr` — define a symbolic constant; no bytes emitted.
    ///
    /// Requires a label on the same line.  The label receives the value of
    /// `expr` rather than the current location counter.
    Is,

    /// `BYTE expr,...` — emit one byte per value (or a string literal).
    Byte,

    /// `WYDE expr,...` — emit one 16-bit big-endian word per value.
    Wyde,

    /// `TETRA expr,...` — emit one 32-bit big-endian word per value.
    Tetra,

    /// `OCTA expr,...` — emit one 64-bit big-endian word per value.
    Octa,
}

/// Lookup table for all supported assembler directives.
///
/// Centralizing the directive registry here means that:
/// * the assembler's main loop is a simple `directive_table.get(mnem)` lookup,
/// * new pseudo-instructions can be added without touching `assembler.rs`, and
/// * `build_mnemonic_set` in `encode.rs` automatically includes every directive
///   so that `extract_label` never misclassifies a directive as a label.
pub(crate) struct DirectiveTable {
    inner: HashMap<&'static str, Directive>,
}

impl DirectiveTable {
    pub(crate) fn new() -> Self {
        let mut m = HashMap::new();
        m.insert("LOC",   Directive::Loc);
        m.insert("IS",    Directive::Is);
        m.insert("BYTE",  Directive::Byte);
        m.insert("WYDE",  Directive::Wyde);
        m.insert("TETRA", Directive::Tetra);
        m.insert("OCTA",  Directive::Octa);
        Self { inner: m }
    }

    /// Look up a directive by its (already-uppercased) mnemonic.
    pub(crate) fn get(&self, name: &str) -> Option<Directive> {
        self.inner.get(name).copied()
    }

    /// Iterate over every directive name (all uppercase).
    pub(crate) fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.inner.keys().copied()
    }
}
