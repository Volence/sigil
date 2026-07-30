//! Flip Stage 2 — the parallax `:=` relocation capability.
//!
//! A REASSIGNABLE set-symbol (`P_DBG := DeformTable`, AS `:=`/`set`) whose RHS
//! names a section LABEL must stay SYMBOLIC through placement, with per-use-site
//! SNAPSHOT semantics: each `dc.l P_DBG` binds the value `P_DBG` held at THAT
//! emission point, and that value — being a relocation-shiftable label address —
//! must follow the label when a width-grown cross-seam `jsr` shifts it. This is
//! the exact shape of `engine/parallax_macros.inc`: `P_DBG := deformBg` then
//! `dc.l P_DFG, P_DBG` inside a section the config_a/config_b chainer relocates.
//!
//! The mirror of `combined_link_stale_refs.rs`'s label-referencing-EQU test, but
//! with `:=` (a reassignable set) instead of `=` (a single-binding equate). The
//! equate exports ONE `equ_sym`; a set is redefined, so each use needs the
//! snapshot of what it held then — the SSA-style versioned shape.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{SymbolTable, SymbolValue};

/// Assemble `asm`, link against a symbol table defining the cross-seam target
/// `External` HIGH (forcing the deferred `jsr` to grow abs.w→abs.l, +2), flatten.
fn assemble_link_flatten(asm: &str) -> Vec<u8> {
    let module = assemble(asm, &Options::default()).expect("assemble");
    let mut stubs = SymbolTable::new();
    stubs.define("External", SymbolValue::Int(0x0001_2345));
    let resolved = sigil_link::resolve_layout(&module.sections, &stubs, true)
        .expect("resolve_layout (combined)");
    let linked = sigil_link::link(&resolved, &stubs).expect("link (combined)");
    sigil_link::flatten(&linked, 0x00)
}

/// A `:=` set-symbol bound to a label, `dc.l`'d past a width-grown cross-seam
/// `jsr`, then REASSIGNED to a plain int and `dc.l`'d again — proving both
/// relocation (use 1) and per-use snapshot semantics (use 2 is a plain 0).
///
/// Grown layout (jsr = abs.l, 6 bytes):
///   0: jsr External (6)   6: Handler: rts (2) = $10006
///   8: dc.l PSET (4)      12: dc.l PSET (4)
const SET_SRC: &str = "\
        cpu 68000
        phase $10000
        jsr     External
Handler:
        rts
PSET := Handler
        dc.l    PSET
PSET := 0
        dc.l    PSET
";

#[test]
fn dc_l_through_label_referencing_set_past_grown_jsr_is_not_stale() {
    let bytes = assemble_link_flatten(SET_SRC);
    // `PSET := Handler` → dc.l @8 must resolve to Handler's GROWN VMA ($10006),
    // not the baseline ($10004) baked at assemble time.
    assert_eq!(
        &bytes[8..12],
        &[0x00, 0x01, 0x00, 0x06],
        "dc.l of a label-referencing := set past a grown jsr must relocate to $10006"
    );
    // Snapshot: `PSET := 0` before the second use → that use is a plain 0.
    assert_eq!(
        &bytes[12..16],
        &[0x00, 0x00, 0x00, 0x00],
        "after `PSET := 0` the next dc.l binds the CURRENT (plain-int) value"
    );
}

/// Per-use SNAPSHOT with two DIFFERENT labels: `P := A; dc.l P; P := B; dc.l P`
/// must bind A then B — each use captures what the set held at that point.
///
/// Grown layout:
///   0: jsr (6)   6: A: rts = $10006   8: B: rts = $10008
///   10: dc.l P (=A)   14: dc.l P (=B)
const SNAPSHOT_SRC: &str = "\
        cpu 68000
        phase $10000
        jsr     External
A:
        rts
B:
        rts
P := A
        dc.l    P
P := B
        dc.l    P
";

#[test]
fn set_snapshot_binds_value_held_at_each_use_site() {
    let bytes = assemble_link_flatten(SNAPSHOT_SRC);
    assert_eq!(
        &bytes[10..14],
        &[0x00, 0x01, 0x00, 0x06],
        "first dc.l binds A's grown VMA ($10006)"
    );
    assert_eq!(
        &bytes[14..18],
        &[0x00, 0x01, 0x00, 0x08],
        "second dc.l binds B's grown VMA ($10008) — the snapshot after `P := B`"
    );
}

/// Set-symbol CHAINING: `Q := A; P := Q; dc.l P` — P inherits Q's symbolic
/// label binding transitively (the `PC_FG_T := fgTable` → `P_DFG := PC_FG_T`
/// chain in parallax_combine).
const CHAIN_SRC: &str = "\
        cpu 68000
        phase $10000
        jsr     External
A:
        rts
Q := A
P := Q
        dc.l    P
";

#[test]
fn set_chain_carries_label_symbolically() {
    let bytes = assemble_link_flatten(CHAIN_SRC);
    // Grown: jsr(6)@0, A:rts@6=$10006, dc.l P @8. P := Q := A must relocate.
    assert_eq!(
        &bytes[8..12],
        &[0x00, 0x01, 0x00, 0x06],
        "dc.l of a set that chains through another set to a label must relocate to $10006"
    );
}

/// t24 control: a `:=` bound to a PURE CONSTANT (no label) must STILL bake its
/// value byte-for-byte, unaffected by the relocation path.
const CONST_SRC: &str = "\
        cpu 68000
        phase $10000
        jsr     External
        rts
PC := $1234
        dc.l    PC
";

#[test]
fn const_set_still_bakes_verbatim() {
    let bytes = assemble_link_flatten(CONST_SRC);
    // jsr(6)@0, rts(2)@6, dc.l PC @8.
    assert_eq!(
        &bytes[8..12],
        &[0x00, 0x00, 0x12, 0x34],
        "a pure-constant := must still bake its value verbatim"
    );
}
