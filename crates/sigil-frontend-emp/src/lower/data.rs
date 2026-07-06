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
//! | 68000, windowed (`winptr`)       | `BankPtr16Be` (T6, D-P4.7)             |
//! | Z80, un-windowed 68k pointer     | ERROR `[cross-cpu.unwindowed-pointer]` |
//!
//! `BankPtr16Be` (a 68k reference to a Z80 bank pointer) was added in T6 (D-P4.7)
//! alongside its Core [`FixupKind`] variant — the big-endian counterpart of
//! `BankPtr16Le`.
//!
//! A [`Cell::RelOffset`] (an offset-table entry) does NOT go through
//! `fixup_kind`: it always emits a fixed-width `RelWord16Be` (68k big-endian
//! signed word) carrying a symbol *difference* `target - base`; a Z80 section is
//! the `[offsets.non-68k]` error.

use crate::value::{Cell, DataBuf};
use sigil_ir::backend::Cpu;
use sigil_ir::expr::BinOp;
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
                bytes.extend(encode_scalar(*value, *width, cpu));
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
                    target: sym_target(name, *windowed),
                });
                bytes.resize(bytes.len() + kind.byte_width() as usize, 0);
            }
            Cell::RelOffset { base, target } => {
                // 68k big-endian signed word only (first cut); Z80 diagnosed.
                if cpu != Cpu::M68000 {
                    diags.push(err(
                        span,
                        "[offsets.non-68k] an offset table is a 68k word-offset idiom; \
                         Z80 offset tables are not supported"
                            .to_string(),
                    ));
                    bytes.resize(bytes.len() + 2, 0);
                    continue;
                }
                // The reserved hole is tied to the fixup kind's width so the two
                // never drift, mirroring the `SymRef` arm's invariant.
                debug_assert_eq!(2, FixupKind::RelWord16Be.byte_width() as usize);
                fixups.push(Fixup {
                    kind: FixupKind::RelWord16Be,
                    offset: bytes.len() as u32,
                    target: Expr::Binary {
                        op: BinOp::Sub,
                        lhs: Box::new(Expr::Sym(target.clone())),
                        rhs: Box::new(Expr::Sym(base.clone())),
                    },
                });
                bytes.resize(bytes.len() + 2, 0);
            }
        }
    }

    (bytes, fixups, diags)
}

/// Serialize the low `width` bytes of `value` in `cpu`'s byte order (§4.5 / §7.2:
/// M68000 big-endian, Z80 little-endian). Ranges are checked in Plan 3, so
/// truncating to `width` is exact. Shared by the [`stream_data`] scalar path and
/// T5's `patch`/`bind` back-patch ([`super::patch`]) so both commit endianness
/// through ONE routine.
pub(super) fn encode_scalar(value: i128, width: u8, cpu: Cpu) -> Vec<u8> {
    let be = &value.to_be_bytes()[16 - width as usize..];
    match cpu {
        Cpu::M68000 => be.to_vec(),
        // Z80 is little-endian: reverse the big-endian window.
        Cpu::Z80 => be.iter().rev().copied().collect(),
    }
}

/// Select the [`FixupKind`] for a `SymRef` from (`width`, section CPU,
/// `windowed`) per the D-P4.5 table. Returns `None` (after recording a
/// diagnostic) for a case with no representable kind: an un-windowed pointer in
/// a Z80 section (`[cross-cpu.unwindowed-pointer]`, §7.2), or a shape still
/// deferred to a later task (a bare Z80-local `Abs16Le`). The 68k windowed
/// pointer (`BankPtr16Be`, T6 / D-P4.7) is now represented.
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
        // A 68k reference to a Z80 bank pointer: the big-endian counterpart of
        // `BankPtr16Le` (§7.2 / D-P4.7). Added in T6 alongside the Core kind.
        (Cpu::M68000, 2, true) => Some(FixupKind::BankPtr16Be),
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

/// The fixup target for a `SymRef`. A plain (un-windowed) reference is the bare
/// symbol; a WINDOWED (`winptr`) reference applies the SFX bank-window mask —
/// `(addr & 0x7FFF) | 0x8000` — matching AS `sfx_winptr`
/// (`SFX_WIN_MASK=0x7FFF`, `SFX_WIN_BASE=0x8000`) and the linker's own
/// `BankPtr16Le`/`BankPtr16Be` test convention. The mask maps a 68k-ROM-blob
/// address (e.g. `$6569A → $D69A`) into the z80's `$8000..$FFFF` window; it is
/// idempotent for a symbol that already resolves inside the window (a z80 label
/// in a `vma:$8000` section), so it is safe to apply unconditionally to every
/// windowed symref (both LE and BE kinds).
fn sym_target(name: &str, windowed: bool) -> Expr {
    let sym = Expr::Sym(name.to_string());
    if !windowed {
        return sym;
    }
    // (addr & 0x7FFF) | 0x8000
    Expr::Binary {
        op: BinOp::Or,
        lhs: Box::new(Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(sym),
            rhs: Box::new(Expr::Int(0x7FFF)),
        }),
        rhs: Box::new(Expr::Int(0x8000)),
    }
}

/// Build an error diagnostic at `span`.
fn err(span: Span, message: String) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary: span }
}

#[cfg(test)]
mod rel_offset_tests {
    use super::*;
    use crate::value::{Cell, DataBuf};
    use sigil_ir::backend::Cpu;
    use sigil_ir::{expr::BinOp, Expr, FixupKind};
    use sigil_span::{SourceId, Span};

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn rel_offset_emits_relword16be_symbol_difference() {
        let mut buf = DataBuf::empty();
        buf.push(Cell::RelOffset { base: "Tbl".into(), target: "Frame0".into() });
        let (bytes, fixups, diags) = stream_data(&buf, Cpu::M68000, span());
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        assert_eq!(bytes, vec![0x00, 0x00], "reserves a 2-byte hole");
        assert_eq!(fixups.len(), 1);
        assert_eq!(fixups[0].kind, FixupKind::RelWord16Be);
        assert_eq!(fixups[0].offset, 0);
        assert_eq!(
            fixups[0].target,
            Expr::Binary {
                op: BinOp::Sub,
                lhs: Box::new(Expr::Sym("Frame0".into())),
                rhs: Box::new(Expr::Sym("Tbl".into())),
            }
        );
    }

    #[test]
    fn rel_offset_in_z80_section_diagnoses() {
        let mut buf = DataBuf::empty();
        buf.push(Cell::RelOffset { base: "Tbl".into(), target: "Frame0".into() });
        let (bytes, _fixups, diags) = stream_data(&buf, Cpu::Z80, span());
        assert_eq!(bytes.len(), 2, "still reserves the hole so sizes line up");
        assert!(diags.iter().any(|d| d.message.contains("offset table")), "got: {diags:?}");
    }
}
