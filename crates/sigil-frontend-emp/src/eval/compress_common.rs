//! Shared plumbing for every comptime compression builtin (`zx0`, `s4lz`,
//! and the classic-format builtins in `classic_compress.rs`): flattening a
//! [`Value::Data`]'s cells to concrete bytes.
//!
//! Before this module existed, `zx0_from_data` (`sandbox.rs`) and
//! `s4lz`'s `flatten_data_buf` (`s4lz.rs`) each carried their own copy of
//! this exact cell-walk. Plan-7 #10 T2b adds SEVEN more builtins that all
//! need the identical prologue (rule of three long met — see the T2b task
//! spec's "shared plumbing" note), so this extracts the one true version.
//! Per-builtin diagnostic TEXT still varies (the `tag` parameter), matching
//! how `s4lz.rs` already parameterized its version for `s4lz` vs `s4lz.dict`.
use super::Evaluator;
use crate::value::{Cell, DataBuf};
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Flatten a [`DataBuf`]'s cells to raw bytes: `Cell::Bytes` extends
    /// directly; a width-1 `Cell::Scalar` contributes its one
    /// (range-checked) byte; a WIDER `Cell::Scalar` has no committed byte
    /// order yet (Plan 4's 68k-BE-vs-Z80-LE lowering decision) and a
    /// `Cell::SymRef`/`RelOffset`/`Expr` names something not yet resolved
    /// (also Plan 4) — all three are diagnostics here, never a panic, since
    /// a compression builtin can only compress concrete bytes.
    ///
    /// `tag` names the call site in the diagnostic code (e.g. `zx0`,
    /// `s4lz`, `s4lz.dict`, `kosinski`) so a diagnostic is always
    /// attributable to the builtin (and argument) that produced it.
    pub(crate) fn flatten_data_buf_tagged(&mut self, buf: &DataBuf, span: Span, tag: &str) -> Option<Vec<u8>> {
        let mut input = Vec::with_capacity(buf.size);
        for cell in &buf.cells {
            match cell {
                Cell::Bytes(b) => input.extend_from_slice(b),
                Cell::Scalar { value, width: 1, .. } => {
                    // Mirrors `byte`/`bytes`'s accepted range: a signed or
                    // unsigned reading of one byte. Reuse the SAME constants so
                    // the byte-domain sites cannot silently drift apart.
                    if !(super::builtins::BYTE_LO..=super::builtins::BYTE_HI).contains(value) {
                        self.error(
                            span,
                            format!("[{tag}.byte-range] {tag} input byte {value} does not fit 8 bits"),
                        );
                        return None;
                    }
                    input.push((*value & 0xFF) as u8);
                }
                Cell::Scalar { .. } => {
                    self.error(
                        span,
                        format!(
                            "[{tag}.byte-order] {tag} input has a multi-byte scalar with no committed \
                             byte order — build it from raw bytes (embed/bytes)"
                        ),
                    );
                    return None;
                }
                Cell::SymRef { .. } => {
                    self.error(span, format!("[{tag}.symbolic] {tag} input has an unresolved symbol reference"));
                    return None;
                }
                Cell::RelOffset { .. } => {
                    self.error(span, format!("[{tag}.symbolic] {tag} input has an unresolved offset-table entry"));
                    return None;
                }
                Cell::Expr { .. } => {
                    self.error(span, format!("[{tag}.symbolic] {tag} input has an unresolved link-expr value"));
                    return None;
                }
            }
        }
        Some(input)
    }
}
