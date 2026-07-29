//! Z80 rung-2 contracts — the register-contract vocabulary, the module-scope
//! `invariant` class, and the push/pop `preserves` proof, scoped to what the
//! rung-2 corpus (`sound_psg.asm`/`sound_fm.asm`) demands. Design note:
//! `docs/superpowers/notes/2026-07-29-z80-rung2-contracts.md`. Every negative
//! control carries a positive control (the t24 rule).

use sigil_frontend_emp::regfile::{expand_reglist, RegFile};

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
