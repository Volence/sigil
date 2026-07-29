//! Z80 rung-2 contracts — the register-contract vocabulary, the module-scope
//! `invariant` class, and the push/pop `preserves` proof, scoped to what the
//! rung-2 corpus (`sound_psg.asm`/`sound_fm.asm`) demands. Design note:
//! `docs/superpowers/notes/2026-07-29-z80-rung2-contracts.md`. Every negative
//! control carries a positive control (the t24 rule).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::regfile::{expand_reglist, RegFile};
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Lower `src` (68k default; a Z80 module opts in) and return the diagnostics.
fn lower_diags(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    diags.into_iter().map(|d| d.message).collect()
}

/// Expand a comma-enumerated reglist under `rf`, collecting the unit set and any
/// error reasons — the unit-test lens on [`expand_reglist`].
fn expand(segs: &[(&str, Option<&str>)], rf: RegFile) -> (Vec<String>, Vec<String>) {
    let owned: Vec<(String, Option<String>)> =
        segs.iter().map(|(lo, hi)| (lo.to_string(), hi.map(|h| h.to_string()))).collect();
    let mut errs = Vec::new();
    let set = expand_reglist(&owned, rf, |reason| errs.push(reason));
    let mut units: Vec<String> = set.into_iter().collect();
    units.sort();
    (units, errs)
}

// ---- ladder item 1: the Z80 reglist recognizer (§2) ------------------------

/// A pair name EXPANDS to its 8-bit halves: `de` → `{d, e}` (§1.1).
#[test]
fn z80_pair_expands_to_halves() {
    let (units, errs) = expand(&[("de", None)], RegFile::Z80);
    assert_eq!(units, vec!["d", "e"]);
    assert!(errs.is_empty(), "clean pair expansion: {errs:?}");
}

/// A register HALF is an independent unit: `clobbers(af, b)` lists `{a, f, b}`
/// and leaves `c` unlisted — the pair split that makes `clobbers(af, b)` +
/// `preserves(c)` expressible (`Psg_VolToAtten`, §1.1).
#[test]
fn z80_half_split_leaves_sibling_unlisted() {
    let (units, errs) = expand(&[("af", None), ("b", None)], RegFile::Z80);
    assert_eq!(units, vec!["a", "b", "f"]);
    assert!(!units.contains(&"c".to_string()), "c stays unlisted");
    assert!(errs.is_empty(), "{errs:?}");
}

/// `ix`/`iy` are index UNITS (not pairs — no half-split): `preserves(bc, ix)` →
/// `{b, c, ix}`.
#[test]
fn z80_index_is_a_unit() {
    let (units, errs) = expand(&[("bc", None), ("ix", None)], RegFile::Z80);
    assert_eq!(units, vec!["b", "c", "ix"]);
    assert!(errs.is_empty(), "{errs:?}");
}

/// Negative control (t24): a 68k register name in a Z80 reglist is
/// `[contract.unknown-register]` — `clobbers(d0)` under a Z80 module.
#[test]
fn z80_unknown_register_d0_errors() {
    let (_units, errs) = expand(&[("d0", None)], RegFile::Z80);
    assert!(
        errs.iter().any(|e| e.contains("[contract.unknown-register]") && e.contains("d0")),
        "expected unknown-register for a 68k name in a Z80 reglist, got: {errs:?}"
    );
}

/// The reverse (t24 positive control the negative pairs with): a Z80 register
/// name in a 68k reglist is the SAME `[contract.unknown-register]` — `af` under
/// the 68k file. Proves the recognizer is genuinely CPU-parametric, not a
/// one-sided allow-list.
#[test]
fn m68k_unknown_register_af_errors() {
    let (_units, errs) = expand(&[("af", None)], RegFile::M68k);
    assert!(
        errs.iter().any(|e| e.contains("[contract.unknown-register]") && e.contains("af")),
        "expected unknown-register for a Z80 name in a 68k reglist, got: {errs:?}"
    );
}

/// A well-formed 68k reglist still expands cleanly through the same seam (the
/// positive control for the reverse-direction negative): `d0`, `a1` are units.
#[test]
fn m68k_known_registers_expand() {
    let (units, errs) = expand(&[("d0", None), ("a1", None)], RegFile::M68k);
    assert_eq!(units, vec!["a1", "d0"]);
    assert!(errs.is_empty(), "{errs:?}");
}

/// Z80 reglists ENUMERATE — no ordinal range form (the step-2 range rule is
/// 68k-scoped, §2.1). `preserves(b-l)` under Z80 is a loud reason, not a silent
/// wrong expansion.
#[test]
fn z80_range_form_rejected() {
    let (_units, errs) = expand(&[("b", Some("l"))], RegFile::Z80);
    assert!(
        errs.iter().any(|e| e.to_lowercase().contains("enumerate")),
        "expected a no-range-form reason for a Z80 range, got: {errs:?}"
    );
}

// ---- ladder item 6 (§3, ruling 4): the module-scope `invariant` grammar ------
//
// The module-header attribute form (ruling 4), reusing `ModuleDecl.attrs` beside
// `cpu:`. This landing is the GRAMMAR + reglist validation (the forward-compat
// slot, giving item 1's recognizer a production consumer); the INHERITANCE PROOF
// — every proc actually preserving `ix` — rides the Z80 contract checker (the
// push/pop `preserves` proof), which is gated on the ruling-2 preserves decision.

/// `module m (cpu: z80, invariant: preserves(ix))` parses and lowers with NO
/// error — the ratified attribute form (ruling 4).
#[test]
fn module_invariant_preserves_ix_accepted() {
    let src = "module m (cpu: z80, invariant: preserves(ix))\n\
               section s (cpu: z80, vma: $0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let diags = lower_diags(src);
    assert!(diags.is_empty(), "invariant: preserves(ix) must lower clean, got: {diags:?}");
}

/// Negative control (t24): a 68k register in a Z80 module's invariant reglist is
/// the same `[contract.unknown-register]` a proc reglist gives — the invariant
/// clause is genuinely validated, not silently swallowed.
#[test]
fn module_invariant_bad_register_errors() {
    let src = "module m (cpu: z80, invariant: preserves(d0))\n\
               section s (cpu: z80, vma: $0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[contract.unknown-register]") && d.contains("d0")),
        "expected an unknown-register error for `d0` in a Z80 invariant, got: {diags:?}"
    );
}

/// The value-bound form `invariant: holds(de == $4001)` (§3.4) is REPRESENTED —
/// accepted but not wired (the rung-4 DAC-loop spelling; the grammar is
/// forward-compatible now).
#[test]
fn module_invariant_holds_value_form_represented() {
    let src = "module m (cpu: z80, invariant: holds(de == $4001))\n\
               section s (cpu: z80, vma: $0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let diags = lower_diags(src);
    assert!(diags.is_empty(), "invariant: holds(...) must be accepted (represented), got: {diags:?}");
}

/// A malformed invariant clause (neither `preserves(...)` nor `holds(...)`) is a
/// loud error, not silent tolerance.
#[test]
fn module_invariant_malformed_errors() {
    let src = "module m (cpu: z80, invariant: ix)\n\
               section s (cpu: z80, vma: $0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("invariant must be")),
        "expected a malformed-invariant error, got: {diags:?}"
    );
}
