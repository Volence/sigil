//! The Stage-3 P5 ownership-flip export mechanism (Option A) — the
//! FOLD-IDENTITY proof the flip owes (bar 1).
//!
//! The flip harvests `engine.constants`'s resolved `pub const` values and
//! injects them as GUARDED AS `-D` defines so the residual AS reads the
//! `.emp`-owned constant at COMPTIME (a `ds` count, an `if` guard, a shifted
//! `dc.b`, a derived `=` equate — positions link-deferral cannot serve). For
//! that to be byte-neutral, a guarded-`-D`-injected value MUST fold BYTE-FOR-BYTE
//! identically to the same value written as an in-file `=` equate in every one
//! of those positions. These fixtures prove exactly that, and prove the
//! no-silent-shadowing collision guard (bar 2).

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{Cpu, SectionPlacement, SymbolTable};

/// Assemble `src` (optionally with `guarded`), link at a pinned base, return the
/// emitted bytes of the first non-empty section.
fn emit(src: &str, guarded: &[(&str, i64)]) -> Vec<u8> {
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![],
        guarded_defines: guarded.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        include_root: None,
    };
    let module = assemble(src, &opts).unwrap_or_else(|d| panic!("assemble: {d:?}"));
    let mut sections = module.sections;
    for sec in &mut sections {
        sec.lma = 0x0010_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    linked
        .sections
        .iter()
        .find(|s| !s.bytes.is_empty())
        .map(|s| s.bytes.clone())
        .unwrap_or_default()
}

/// The consumer body — every comptime position the residual AS reads an engine
/// constant in (ram.asm `ds` sizes + `if`; macros.asm shifted `dc.b`;
/// constants.asm's derived `=` equates). `K` is the flipped constant.
const CONSUMERS: &str = "\
    cpu 68000\n\
    DERIVED = K * 2 + 1\n\
    Body:\n\
    \tdc.b DERIVED\n\
    \tdc.b 1 << K_SHIFT\n\
    \tif K > 90\n\
    \t\tdc.b $AA\n\
    \tendif\n\
    Gap:\tds.b K\n\
    After:\tdc.b After - Gap\n\
    \tdc.w K\n";

#[test]
fn guarded_define_folds_identically_to_in_file_equate() {
    // In-file authorship: `K`/`K_SHIFT` are plain `=` equates.
    let in_file = format!("K = 96\nK_SHIFT = 3\n{CONSUMERS}");
    let a = emit(&in_file, &[]);

    // Flipped authorship: `K`/`K_SHIFT` injected as guarded `-D` defines, the
    // in-file `=` lines DELETED (exactly what the flip does to constants.asm).
    let b = emit(CONSUMERS, &[("K", 96), ("K_SHIFT", 3)]);

    assert_eq!(
        a, b,
        "a guarded -D-injected value must fold byte-identically to an in-file `=` \
         equate in every consuming position (ds count, if guard, shifted dc.b, \
         derived equate, dc.w) — got in-file {a:x?} vs injected {b:x?}"
    );
    // And the bytes are actually the expected fold (non-vacuous): DERIVED = 193
    // clamps to $C1; 1<<3 = $08; if 96>90 → $AA (the `if K > 90` comptime guard
    // fired on the injected value); then the `ds.b 96` reserved gap advanced the
    // location counter (After-Gap = 96 = $60, so the reserve read the injected K);
    // dc.w 96 = $00 $60. The reserved gap is a Space fragment, not materialized
    // bytes, so the emitted section is the 3 prefix + 3 tail bytes.
    assert_eq!(b, vec![0xC1, 0x08, 0xAA, 0x60, 0x00, 0x60], "fold bytes: {b:x?}");
}

#[test]
fn guarded_define_redefined_in_file_is_a_hard_collision() {
    // The t24 negative probe (bar 2): reintroducing an in-file `=` for a guarded
    // name must FAIL LOUD with `[defines.collision]`, never silently prefer a side.
    let src = format!("K = 96\nK_SHIFT = 3\n{CONSUMERS}");
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![],
        guarded_defines: vec![("K".into(), 96), ("K_SHIFT".into(), 3)],
        include_root: None,
    };
    let diags = assemble(&src, &opts).expect_err("a guarded name redefined in-file must fail");
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("[defines.collision]") && d.message.contains("`K`")),
        "expected a [defines.collision] error naming K, got {diags:?}"
    );
}

#[test]
fn ordinary_defines_keep_silent_override_semantics() {
    // The general `defines` channel (code gates, game-config overrides like
    // MAX_RING_BUFFER) MUST keep asl's silent-override — an in-file `=` of a
    // plain define wins, no collision. This is why guarded_defines is a
    // SEPARATE channel (config_a passes -D names that config/constants.asm also
    // defines in-file).
    let src = "cpu 68000\nX = 7\nB:\tdc.b X\n";
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![("X".into(), 99)], // ordinary define, same name as in-file
        guarded_defines: vec![],
        include_root: None,
    };
    let module = assemble(src, &opts).expect("ordinary define coexisting in-file must not error");
    let byte = module
        .sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .find_map(|f| match f {
            sigil_ir::Fragment::Data(d) => d.bytes.first().copied(),
            _ => None,
        });
    assert_eq!(byte, Some(7), "in-file `=` must win over an ordinary -D define (asl parity)");
}
