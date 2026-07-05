//! M1.D T0.2 — hermetic reproducer for the stale-fold / stale-LMA defect (F2).
//!
//! ## The defect
//!
//! The front-end advances the cursor by a FIXED 4 bytes per bare `jmp`/`jsr`
//! site (`eval.rs`, the `emit_fragment(JmpJsrSym, advance = 4)` path) — i.e. it
//! assumes the abs.w (2-word) form for every one. It then folds nearly every
//! label reference at those *baseline* offsets. The linker's `resolve_layout`
//! later picks the real abs.w/abs.l width per `asl_width_rule` and, when a
//! `jmp`/`jsr` grows W→L, re-lowers that `JmpJsrSym` AND shifts label offsets —
//! but it clones every `Data` fragment unchanged. So:
//!
//! 1. **Stale fold:** a `dc.l Label` the front-end already folded to a baseline
//!    value keeps that stale value even though `Label`'s real address moved by
//!    the width growth. (`resolve_layout` fixes the label *definition* but not
//!    the already-emitted `dc.l` bytes.)
//! 2. **Stale downstream LMA (the half the handoff did not flag):** `phys_base`
//!    accumulated the *baseline* section length, so every following section's
//!    LMA is short by the total growth. `resolve_layout` never re-flows section
//!    LMAs.
//!
//! Both are guaranteed to fire on the real object bank (`org $10000`, 11
//! `jmp`/`jsr` that grow abs.w→abs.l, +22 bytes total; the Z80 driver lands 22
//! bytes early).
//!
//! ## The fix (T3, landed)
//!
//! T3 moved width selection into the front-end pass loop: it picks abs.w/abs.l
//! per pass from the current env, advances the cursor by the TRUE width, and
//! lets the existing `env == prev` convergence absorb the growth — exactly as
//! asl's own repeat-until-stable does. Folds and `phys_base` are now true by
//! construction, and both halves of the defect close.
//!
//! ## This file
//!
//! These tests assert the **asl-correct** bytes/LMAs (verified live against
//! `asl 1.42` at authoring time). Before T3 they failed (that was the defect);
//! now that the front-end selects the true jmp/jsr width, they pass. The defect
//! explanation above is kept as historical rationale for what they pin.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

/// Assemble → resolve_layout → link → flatten, mirroring `asl_snippets.rs`.
fn assemble_flatten(asm: &str) -> Vec<u8> {
    let module = assemble(asm, &Options::default()).expect("assemble");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Assemble → resolve_layout only; return the resolved sections for LMA checks.
fn assemble_resolve(asm: &str) -> Vec<sigil_ir::Section> {
    let module = assemble(asm, &Options::default()).expect("assemble");
    sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout")
}

/// A section phased to `$10000` so its labels resolve `> $8000`, forcing the
/// bare `jmp` to grow abs.w→abs.l. `phase` (unlike `org`) sets the base via a
/// displacement and emits no `Org` fragment, so this exercises the width-growth
/// path WITHOUT tripping resolve_layout's `Org + JmpJsrSym` guard — isolating
/// the stale-fold defect itself. `AfterJmp` sits immediately after the grown
/// `jmp`; the `dc.l AfterJmp` is the folded reference that goes stale.
const SINGLE: &str = "\
        cpu 68000
        phase $10000
        jmp AfterJmp
AfterJmp:
        dc.l AfterJmp
";

/// Same, with a second section after `dephase` to expose the stale downstream
/// LMA: the phased block is jmp(6)+dc.l(4)=10 physical bytes once the jmp grows,
/// so `SecondSection` belongs at physical `$0A`. The front-end places it at `$8`
/// (baseline jmp=4) and resolve_layout never re-flows it.
const TWO: &str = "\
        cpu 68000
        phase $10000
        jmp AfterJmp
AfterJmp:
        dc.l AfterJmp
        dephase
SecondSection:
        dc.l SecondSection
";

// asl 1.42 (`-cpu 68000 -q -L -U`) on SINGLE emits: 4EF9 0001 0006 0001 0006.
// The grown jmp targets $10006 (correct) and the dc.l ALSO resolves to $10006.
const SINGLE_ASL_CORRECT: &[u8] = &[0x4E, 0xF9, 0x00, 0x01, 0x00, 0x06, 0x00, 0x01, 0x00, 0x06];

#[test]
fn dc_l_after_grown_jmp_folds_correctly() {
    assert_eq!(
        assemble_flatten(SINGLE),
        SINGLE_ASL_CORRECT,
        "the dc.l after a width-grown jmp must fold to the shifted label ($10006), not the baseline"
    );
}

#[test]
fn downstream_section_lma_reflows_after_growth() {
    let resolved = assemble_resolve(TWO);
    let second = resolved
        .iter()
        .find(|s| s.labels.iter().any(|l| l.name == "SecondSection"))
        .expect("SecondSection section present");
    // asl places SecondSection at physical $0A (10): the phased block is 10 bytes
    // once the jmp grows to abs.l.
    assert_eq!(
        second.lma, 0x0A,
        "downstream section LMA must account for jmp/jsr width growth"
    );
}
