//! T5 (Plan 4) — `patch` / `bind` → back-patch + Fixup (§6.4, D-P4.10). Drives
//! the [`PatchTable`] lowering primitive directly (the emit-forward-bind-later
//! mechanism), since the surface does not yet let a comptime `patch` / `bind`
//! statement sit in a section-emission position — see the `lower::patch` module
//! doc for the emission-context decision and the flagged surface gap (T6/T7).
//!
//! Covers: an integer `bind` back-patches the reserved bytes in 68k big-endian
//! order; a slot never bound is `[patch.unbound]`; a second `bind` is
//! `[patch.double-bound]`; a `bind` of an unknown name is `[patch.unknown]`.

use sigil_frontend_emp::lower::patch::{BindValue, PatchTable};
use sigil_ir::backend::Cpu;
use sigil_span::{SourceId, Span};

fn span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

#[test]
fn int_bind_backpatches_reserved_bytes_big_endian() {
    // `patch p: u16` reserves 2 zero bytes; `bind p = 0x1234` fills them with the
    // 68k big-endian encoding [0x12, 0x34] at the slot's offset.
    let mut t = PatchTable::new(Cpu::M68000);
    t.emit_bytes(&[0xAA]); // some ordinary emission before the slot
    let slot_off = t.len();
    t.patch("p", 2, span());
    t.emit_bytes(&[0xBB]); // ordinary emission after the slot
    t.bind("p", BindValue::Int(0x1234), span());

    let (bytes, fixups, diags) = t.finish();
    assert!(diags.is_empty(), "no diagnostics expected, got: {diags:?}");
    assert!(fixups.is_empty(), "an integer bind records no fixup");
    assert_eq!(slot_off, 1, "slot lands right after the leading byte");
    assert_eq!(bytes, vec![0xAA, 0x12, 0x34, 0xBB]);
}

#[test]
fn int_bind_is_little_endian_in_a_z80_section() {
    // The back-patch reuses T2's byte-order logic: a Z80 section is little-endian,
    // so 0x1234 lands as [0x34, 0x12].
    let mut t = PatchTable::new(Cpu::Z80);
    t.patch("p", 2, span());
    t.bind("p", BindValue::Int(0x1234), span());
    let (bytes, _fixups, diags) = t.finish();
    assert!(diags.is_empty(), "no diagnostics expected, got: {diags:?}");
    assert_eq!(bytes, vec![0x34, 0x12]);
}

#[test]
fn symbol_bind_records_a_fixup_over_the_reserved_hole() {
    // A `bind p = Target` (a symbol) leaves the bytes zero and records an Abs32Be
    // fixup at the slot offset for the linker to resolve.
    let mut t = PatchTable::new(Cpu::M68000);
    t.patch("p", 4, span());
    t.bind("p", BindValue::Sym("Target".into()), span());
    let (bytes, fixups, diags) = t.finish();
    assert!(diags.is_empty(), "no diagnostics expected, got: {diags:?}");
    assert_eq!(bytes, vec![0, 0, 0, 0], "the reserved hole stays zero");
    assert_eq!(fixups.len(), 1);
    assert_eq!(fixups[0].offset, 0);
}

#[test]
fn z80_symbol_bind_is_patch_unsupported() {
    // A symbol `bind` needs a representable absolute-fixup kind; a Z80 section has
    // none (it needs a windowed pointer, deferred), so it is `[patch.unsupported]`.
    let mut t = PatchTable::new(Cpu::Z80);
    t.patch("p", 2, span());
    t.bind("p", BindValue::Sym("Target".into()), span());
    let (_bytes, fixups, diags) = t.finish();
    assert!(fixups.is_empty(), "no fixup is recorded for an unsupported bind");
    assert!(
        diags.iter().any(|d| d.message.contains("[patch.unsupported]")),
        "expected a [patch.unsupported] diagnostic, got: {diags:?}"
    );
}

#[test]
fn unbound_patch_is_patch_unbound() {
    // A `patch` never followed by a `bind` is `[patch.unbound]`, naming the slot.
    let mut t = PatchTable::new(Cpu::M68000);
    t.patch("size", 2, span());
    let (_bytes, _fixups, diags) = t.finish();
    assert!(
        diags.iter().any(|d| d.message.contains("[patch.unbound]") && d.message.contains("size")),
        "expected a [patch.unbound] diagnostic naming `size`, got: {diags:?}"
    );
}

#[test]
fn double_bind_is_patch_double_bound() {
    // A second `bind` of the same slot is `[patch.double-bound]`.
    let mut t = PatchTable::new(Cpu::M68000);
    t.patch("p", 2, span());
    t.bind("p", BindValue::Int(1), span());
    t.bind("p", BindValue::Int(2), span());
    let (bytes, _fixups, diags) = t.finish();
    assert!(
        diags.iter().any(|d| d.message.contains("[patch.double-bound]") && d.message.contains('p')),
        "expected a [patch.double-bound] diagnostic, got: {diags:?}"
    );
    // The first bind stands; the second is rejected (does not overwrite).
    assert_eq!(bytes, vec![0x00, 0x01]);
}

#[test]
fn bind_of_unknown_name_is_patch_unknown() {
    // A `bind` naming no reserved slot is `[patch.unknown]`.
    let mut t = PatchTable::new(Cpu::M68000);
    t.bind("nope", BindValue::Int(0), span());
    let (_bytes, _fixups, diags) = t.finish();
    assert!(
        diags.iter().any(|d| d.message.contains("[patch.unknown]") && d.message.contains("nope")),
        "expected a [patch.unknown] diagnostic naming `nope`, got: {diags:?}"
    );
}
