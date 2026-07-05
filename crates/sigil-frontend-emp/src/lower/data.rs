//! The `Value::Data` serializer (Spec 2, Plan 4 — T2, D-P4.5): walk a checked,
//! CPU-neutral [`DataBuf`] and commit it to image bytes plus [`Fixup`]s for a
//! section of a given [`Cpu`]. This is where Plan 3's structured cells finally
//! pick a byte order and turn a symbolic reference into a relocation — the
//! load-bearing Plan 3 → Plan 4 seam.
//!
//! Byte order is section-CPU driven (§4.5 / §7.2): M68000 is big-endian, Z80 is
//! little-endian. Fixup-kind selection reads (`width`, section CPU, `windowed`)
//! per the D-P4.5 table:
//!
//! | context                          | FixupKind                              |
//! |----------------------------------|----------------------------------------|
//! | 68000, width 4                   | `Abs32Be`                              |
//! | 68000, width 2                   | `Abs16Be`                              |
//! | Z80, windowed (`winptr`)         | `BankPtr16Le`                          |
//! | Z80, un-windowed 68k pointer     | ERROR `[cross-cpu.unwindowed-pointer]` |
//!
//! `BankPtr16Be` (a 68k reference to a bank pointer) is deferred to T6 (D-P4.7):
//! it does NOT exist in Core's [`FixupKind`] yet, so a case that would need it
//! diagnoses rather than emit a wrong kind.

use crate::value::{Cell, DataBuf};
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Fixup, FixupKind};
use sigil_span::{Diagnostic, Level, Span};

/// Serialize a checked [`DataBuf`] to image bytes plus fixups (offsets relative
/// to the start of this buffer, i.e. within the `DataFragment` it becomes) for a
/// section encoded for `cpu`. `span` locates any diagnostic against the owning
/// `data` item. Returns the bytes, the fixups, and any diagnostics produced
/// while selecting a fixup kind (only the un-windowed-pointer / deferred-kind
/// cases produce one; ranges were already checked in Plan 3).
pub(super) fn stream_data(
    buf: &DataBuf,
    cpu: Cpu,
    span: Span,
) -> (Vec<u8>, Vec<Fixup>, Vec<Diagnostic>) {
    let mut bytes = Vec::with_capacity(buf.size);
    let mut fixups = Vec::new();
    let mut diags = Vec::new();

    for cell in &buf.cells {
        match cell {
            Cell::Scalar { value, width, .. } => {
                let w = *width as usize;
                // The low `width` bytes in the section CPU's byte order. Ranges
                // were already checked in Plan 3, so truncating to `width` is
                // exact (a defensive re-check is unnecessary here).
                let be = &value.to_be_bytes()[16 - w..];
                match cpu {
                    Cpu::M68000 => bytes.extend_from_slice(be),
                    // Z80 is little-endian: reverse the big-endian window.
                    Cpu::Z80 => bytes.extend(be.iter().rev().copied()),
                }
            }
            // Single bytes have no byte order — order-neutral, emit verbatim.
            Cell::Bytes(v) => bytes.extend_from_slice(v),
            Cell::SymRef { name, width, windowed } => {
                let Some(kind) = fixup_kind(cpu, *width, *windowed, name, span, &mut diags) else {
                    // No representable kind: the diagnostic is already recorded.
                    // Still reserve `width` bytes so downstream sizes line up.
                    bytes.resize(bytes.len() + *width as usize, 0);
                    continue;
                };
                // The reserved hole is sized from the fixup kind so the two never
                // drift (`kind.byte_width()` ties them together); this must equal
                // the cell's own `width`.
                debug_assert_eq!(*width as u32, kind.byte_width());
                fixups.push(Fixup {
                    kind,
                    offset: bytes.len() as u32,
                    target: Expr::Sym(name.clone()),
                });
                bytes.resize(bytes.len() + kind.byte_width() as usize, 0);
            }
        }
    }

    (bytes, fixups, diags)
}

/// Select the [`FixupKind`] for a `SymRef` from (`width`, section CPU,
/// `windowed`) per the D-P4.5 table. Returns `None` (after recording a
/// diagnostic) for a case with no representable kind: an un-windowed pointer in
/// a Z80 section (`[cross-cpu.unwindowed-pointer]`, §7.2), or a shape deferred
/// to a later task (`BankPtr16Be`, T6 / D-P4.7; a bare Z80-local `Abs16Le`).
fn fixup_kind(
    cpu: Cpu,
    width: u8,
    windowed: bool,
    name: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Option<FixupKind> {
    match (cpu, width, windowed) {
        // A 68k absolute pointer: width picks Abs32/Abs16 (both big-endian).
        (Cpu::M68000, 4, false) => Some(FixupKind::Abs32Be),
        (Cpu::M68000, 2, false) => Some(FixupKind::Abs16Be),
        // A 68k reference to a bank pointer would be `BankPtr16Be` — deferred.
        (Cpu::M68000, 2, true) => {
            // TODO(T6): BankPtr16Be — a 68k reference to a windowed bank pointer.
            diags.push(err(
                span,
                format!(
                    "[cross-cpu.unsupported] windowed pointer to `{name}` in a 68000 section \
                     needs BankPtr16Be, which is not yet supported"
                ),
            ));
            None
        }
        // A Z80 windowed bank pointer: little-endian 16-bit window offset.
        (Cpu::Z80, 2, true) => Some(FixupKind::BankPtr16Le),
        // A 68k-address constant in Z80 data is an error unless explicitly
        // windowed via `winptr(sym)` — the convsym z-filter class is
        // unrepresentable (§7.2).
        (Cpu::Z80, _, false) => {
            diags.push(err(
                span,
                format!(
                    "[cross-cpu.unwindowed-pointer] un-windowed pointer to `{name}` in a Z80 \
                     section: a 68k-address pointer needs an explicit winptr(sym)"
                ),
            ));
            None
        }
        // Totality guard: every (width, cpu, windowed) shape T2 actually
        // produces is matched above, so this arm is currently unreachable. It
        // is NOT the Z80-local `dc.w` case — a width-2 un-windowed Z80 ref
        // matches `(Cpu::Z80, _, false)` above and yields
        // `[cross-cpu.unwindowed-pointer]`. Supporting a genuine Z80-local
        // 16-bit label ref (an `Abs16Le` that does not yet exist) will require
        // T6 to SPLIT the `(Z80, _, false)` arm by width, not to route here.
        _ => {
            diags.push(err(
                span,
                format!(
                    "[lower.unsupported-pointer] no fixup kind for a width-{width} \
                     {}pointer to `{name}` in this section",
                    if windowed { "windowed " } else { "" }
                ),
            ));
            None
        }
    }
}

/// Build an error diagnostic at `span`.
fn err(span: Span, message: String) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary: span }
}
