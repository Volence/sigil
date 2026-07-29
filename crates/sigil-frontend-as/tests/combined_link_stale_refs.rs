//! Regression: the CROSS-SEAM variant of the stale-fold defect (`stale_fold_repro`
//! covers the IN-PASS variant that T3 closed).
//!
//! When an AS module references a symbol that is UNRESOLVED at assemble time (an
//! `.emp`-side / sibling-module target — the mixed whole-ROM build), the front
//! end DEFERS the `jsr`/`jmp` as a length-variable `Fragment::JmpJsrSym` at the
//! abs.w BASELINE (4 bytes). The COMBINED link then resolves that target (from
//! the other seam) and `resolve_layout` GROWS the fragment abs.w→abs.l (+2),
//! SHIFTING every following label. The linker shifts the section's LABEL table
//! correctly — but a PC-relative branch / `dc.l` whose target the front end
//! BAKED to an absolute `Expr::Int` at the baseline does NOT follow that shift,
//! so it resolves to a stale (pre-growth) address.
//!
//! Symptom on the real player port: ~30 local-label `.s` branches past a
//! cross-seam `jsr` resolved to displacement 0 (op+2) — a hard
//! `bra.s/Bcc.s displacement is 0` link error. The fix keeps a fixup target that
//! names a section LABEL symbolic on the deferral pass, so the linker resolves it
//! against its own relaxation-shifted table.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{SymbolTable, SymbolValue};

/// Assemble `asm`, then link it against a symbol table that defines the
/// cross-seam target `External` at a HIGH address (forcing the deferred `jsr` to
/// grow abs.w→abs.l), and return the flattened bytes.
fn assemble_link_flatten(asm: &str) -> Vec<u8> {
    let module = assemble(asm, &Options::default()).expect("assemble");
    let mut stubs = SymbolTable::new();
    stubs.define("External", SymbolValue::Int(0x0001_2345));
    let resolved = sigil_link::resolve_layout(&module.sections, &stubs, true)
        .expect("resolve_layout (combined)");
    let linked = sigil_link::link(&resolved, &stubs).expect("link (combined)");
    sigil_link::flatten(&linked, 0x00)
}

/// A phased block whose first instruction is an UNRESOLVED cross-seam `jsr`
/// (deferred to a `JmpJsrSym`), followed by a forward `.s` branch skipping a
/// 2-byte `nop`, then a `dc.l Skip`. `phase` (not `org`) sidesteps
/// `resolve_layout`'s `Org + JmpJsrSym` guard, exactly as `stale_fold_repro`.
///
/// Grown layout (jsr = abs.l, 6 bytes) — LMA offsets:
///   0: jsr External (6)   6: tst.b d0 (2)   8: bne.s Skip (2, disp @ 9)
///   10: nop (2)           12: Skip: dc.l Skip (4)   16: rts (2)
/// So the correct `bne.s` displacement is `Skip - (bne+2)` = `$C - $A` = 2, and
/// `dc.l Skip` must equal Skip's grown VMA `$1000C`.
const SRC: &str = "\
        cpu 68000
        phase $10000
        jsr     External
        tst.b   d0
        bne.s   Skip
        nop
Skip:
        dc.l    Skip
        rts
";

#[test]
fn forward_branch_past_grown_cross_seam_jsr_is_not_stale() {
    let bytes = assemble_link_flatten(SRC);
    // Grown jsr is abs.l (0x4EB9), so the `bne.s` disp byte sits at LMA 9.
    // Pre-fix this baked the pre-growth target and resolved to 0 (op+2) — a hard
    // `displacement is 0` link error; the fix keeps it symbolic → disp 2.
    assert_eq!(
        bytes[9], 0x02,
        "bne.s past a width-grown cross-seam jsr must keep displacement 2, not the stale 0"
    );
}

#[test]
fn dc_l_past_grown_cross_seam_jsr_is_not_stale() {
    let bytes = assemble_link_flatten(SRC);
    // `dc.l Skip` at LMA 12 must equal Skip's GROWN VMA ($1000C), not the
    // baseline ($1000A).
    assert_eq!(
        &bytes[12..16],
        &[0x00, 0x01, 0x00, 0x0C],
        "dc.l past a width-grown cross-seam jsr must resolve to the shifted label ($1000C)"
    );
}
