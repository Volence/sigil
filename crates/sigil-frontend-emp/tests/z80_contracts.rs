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

// ---- ladder item 5 (§4.2): the push/pop `preserves` proof (z80_preserves) -----
//
// The SIBLING proof (ruling 2, 2026-07-29 countersign) — a Z80 proc declaring
// `preserves(rN)` is verified over the push/pop stack model. Every negative
// carries a positive control (t24).

/// (a) POSITIVE control: a proc that brackets a clobbering `call` with
/// `push hl / … / pop hl` PROVES `preserves(hl)` — no firing. The Z80 analog of
/// the 68k closure's transitive-clobber job (§1.2).
#[test]
fn z80_preserves_via_push_pop_bracket_proven() {
    let src = "module m in s (cpu: z80)\n\
               proc Callee() { ret }\n\
               proc P() preserves(hl) {\n\
                 push hl\n\
                 call Callee\n\
                 pop hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "push hl/call/pop hl must PROVE preserves(hl), got: {diags:?}"
    );
}

/// (b) NEGATIVE: a proc that writes `hl` and never restores it, yet declares
/// `preserves(hl)`, fires `[proc.preserves-unverifiable]`.
#[test]
fn z80_preserves_written_unrestored_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc Q() preserves(hl) {\n\
                 ld hl, 5\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    // The proof is unit-level, so a declared `preserves(hl)` fires on its halves
    // (`h`/`l`) — the register that is genuinely not preserved.
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")
            && d.contains("written and not restored")),
        "a written-unrestored preserves(hl) must fire, got: {diags:?}"
    );
}

/// (c) the `push R / pop R'` MOVE idiom (§1.3): `push ix / pop hl` copies ix→hl —
/// it PROVES `preserves(ix)` (ix never written) while CLOBBERING hl. The proof
/// credits ix and (in the sibling test below) rejects a `preserves(hl)` claim.
#[test]
fn z80_move_idiom_preserves_source_not_dest() {
    let ok = "module m in s (cpu: z80)\n\
              proc R() preserves(ix) {\n\
                push ix\n\
                pop hl\n\
                ret\n\
              }\n";
    assert!(
        !lower_diags(ok).iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "push ix/pop hl must PROVE preserves(ix)"
    );
    let bad = "module m in s (cpu: z80)\n\
               proc R() preserves(hl) {\n\
                 push ix\n\
                 pop hl\n\
                 ret\n\
               }\n";
    assert!(
        lower_diags(bad).iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("h")),
        "push ix/pop hl CLOBBERS hl — preserves(hl) must fire"
    );
}

/// (d) an unmodeled sp manipulation BAILS the proof — a declared preserve over it
/// is `[proc.preserves-unverifiable]`. Tested via `ld sp, hl` (a modeled sp
/// hazard); the rung-3 `ex (sp),hl` trampoline routes to the SAME bail once its
/// `(sp)` operand form lands (§4.2), so the bailout is proven now.
#[test]
fn z80_sp_hazard_bails_declared_preserve() {
    let src = "module m in s (cpu: z80)\n\
               proc S() preserves(hl) {\n\
                 push hl\n\
                 ld sp, hl\n\
                 pop hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "an unmodeled sp op must bail a declared preserve, got: {diags:?}"
    );
}

/// (e) SOUNDNESS (§13.4-E): a register clobbered ONLY on a conditional branch's
/// FALL-THROUGH path must NOT be verified preserved. `jr z, .skip` skips the
/// `ld a, 5` on the z-TRUE edge but FALLS THROUGH to it on the z-FALSE edge, so
/// `a` is clobbered on some path — a caller cannot rely on it, and
/// `preserves(a)` MUST fire. (Before the conditional two-way split, `z80_edges`
/// treated `jr cc` as unconditional and dropped the fall-through, wrongly
/// verifying the preserve — the empirical reproducer this pass recorded.)
#[test]
fn z80_conditional_jr_fallthrough_clobber_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(a) {\n\
                 jr z, .skip\n\
                 ld a, 5\n\
               .skip:\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a clobber on the jr-z FALL-THROUGH must fire preserves(a), got: {diags:?}"
    );
}

/// (e′) t24 POSITIVE control: an UNCONDITIONAL `jr .skip` stays SINGLE-edge — the
/// `ld a, 5` it jumps over is DEAD (nothing branches into it), so `a` is NOT
/// clobbered and `preserves(a)` HOLDS. Guards against the conditional two-way
/// split leaking a PHANTOM fall-through onto unconditional branches (which would
/// re-reach the dead `ld a, 5` and false-fire).
#[test]
fn z80_unconditional_jr_keeps_single_edge() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(a) {\n\
                 jr .skip\n\
                 ld a, 5\n\
               .skip:\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "unconditional jr skips the dead `ld a, 5` — preserves(a) must hold, got: {diags:?}"
    );
}

// ---- ladder item 6 (§3.2): module invariant(ix) INHERITANCE proof -------------

/// (a) POSITIVE control: `invariant: preserves(ix)` is inherited onto a proc with
/// NO explicit contract and PROVEN — no instruction writes ix (it is only read as
/// `(ix+d)`). No firing.
#[test]
fn module_invariant_ix_inherited_and_proven() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               proc P() {\n\
                 ld a, (ix+0)\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "invariant(ix) must be PROVEN for an ix-read-only proc, got: {diags:?}"
    );
}

/// (b) NEGATIVE: a proc that writes ix (`pop ix` with no matching save) fires
/// `[proc.preserves-unverifiable]` via the INHERITED invariant — the
/// psg-header-line-60 bug class, now a compile error (§3.2). The message names it
/// as a MODULE-invariant break.
#[test]
fn module_invariant_ix_broken_fires_via_inheritance() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               proc P() {\n\
                 pop ix\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")
            && d.contains("ix")
            && d.contains("invariant")),
        "breaking the inherited invariant(ix) must fire naming the module invariant, got: {diags:?}"
    );
}

// ---- ladder item 7 (§4.5): the t27-retirement transition ----------------------

/// The three landed Z80 modules take the VACUOUS pass: a `(cpu: z80)` proc with
/// NO declared `preserves` and NO module `invariant` has nothing to prove, so the
/// checker fires nothing — even `z80_init`'s unbalanced init-drain pops
/// (`pop ix/iy/de/hl/af/bc`, which would underflow-bail the proof) are silent
/// because there is no contract to verify. Item 7's stated outcome, executable.
#[test]
fn z80_init_style_drain_without_contract_is_silent() {
    let src = "module m in s (cpu: z80)\n\
               proc Z80_Idle() {\n\
                 pop ix\n\
                 pop iy\n\
                 pop de\n\
                 pop hl\n\
                 pop af\n\
                 pop bc\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves")),
        "a contract-less Z80 drain proc must be silent (item 7 vacuous pass), got: {diags:?}"
    );
}

// ---- gap 1 (§13.5): clobbers/out reglist validation via the Z80 regfile --------
//
// The rung-2 note §2/§2.2 designed `clobbers`/`out` to ride the CPU-aware
// recognizer, but items 1-8 wired only `preserves`/`invariant`/`out(carry:)` — so
// `check_clobbers`/`check_out` validated the reglist against the 68k universe and
// every `clobbers(af)`/`out(a)` on a Z80 proc fired `[proc.*-invalid]`. These wire
// the two remaining reglists to `expand_reglist(RegFile::Z80)`.

/// A Z80 proc's `clobbers(af, b)` (the `Psg_VolToAtten` shape, §1.1) validates
/// against the Z80 register file — NO `[proc.clobber-invalid]`.
#[test]
fn z80_clobbers_validates_against_z80_regfile() {
    let src = "module m in s (cpu: z80)\n\
               proc P() clobbers(af, b) preserves(c) {\n\
                 ld b, a\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.clobber-invalid]")),
        "Z80 clobbers(af, b) must validate against the Z80 regfile, got: {diags:?}"
    );
}

/// A Z80 proc's `out(a)` / `out(hl)` register result validates against the Z80
/// register file — NO `[proc.out-invalid]`. (`clobbers(f)` keeps the flag-scratch
/// half disjoint from the `a` result — no out∩clobbers overlap.)
#[test]
fn z80_out_reg_validates_against_z80_regfile() {
    let a_out = "module m in s (cpu: z80)\n\
                 proc P() out(a) clobbers(f) {\n\
                   ld a, (ix+0)\n\
                   sub 5\n\
                   ret\n\
                 }\n";
    assert!(
        !lower_diags(a_out).iter().any(|d| d.contains("[proc.out-invalid]")),
        "Z80 out(a) must validate"
    );
    let hl_out = "module m in s (cpu: z80)\n\
                  proc P() out(hl) {\n\
                    ld hl, 0\n\
                    ret\n\
                  }\n";
    assert!(
        !lower_diags(hl_out).iter().any(|d| d.contains("[proc.out-invalid]")),
        "Z80 out(hl) must validate"
    );
}

/// A Z80 `out`-declared register never written on any path must NOT false-fire
/// `[proc.out-unwritten]` — the Z80 write detector is the 68k heuristic (finds no
/// Z80 writes), so an unwritten claim is unverifiable, not false (honest, like
/// `preserves`). Before the wiring, an empty 68k `written` set fired unwritten on
/// EVERY Z80 out.
#[test]
fn z80_out_unwritten_not_false_fired() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(hl) {\n\
                 ld hl, 0\n\
                 ret\n\
               }\n";
    assert!(
        !lower_diags(src).iter().any(|d| d.contains("[proc.out-unwritten]")),
        "a Z80 out must not false-fire out-unwritten (write detection is 68k-only)"
    );
}

/// t24 NEGATIVE control: a 68k register name in a Z80 `clobbers`/`out` is still
/// `[proc.clobber-invalid]` / `[proc.out-invalid]` (`d0` is not a Z80 register).
#[test]
fn z80_clobbers_out_68k_name_still_invalid() {
    let clob = "module m in s (cpu: z80)\n\
                proc P() clobbers(d0) { ret }\n";
    assert!(
        lower_diags(clob).iter().any(|d| d.contains("[proc.clobber-invalid]") && d.contains("d0")),
        "clobbers(d0) in a Z80 module must stay invalid"
    );
    let out = "module m in s (cpu: z80)\n\
               proc P() out(d0) { ret }\n";
    assert!(
        lower_diags(out).iter().any(|d| d.contains("[proc.out-invalid]") && d.contains("d0")),
        "out(d0) in a Z80 module must stay invalid"
    );
}

/// t24 NEGATIVE control (the reverse): a Z80 register name in a 68k `clobbers` is
/// still `[proc.clobber-invalid]` — the frozen-68k behavior is unchanged.
#[test]
fn m68k_clobbers_z80_name_still_invalid() {
    let src = "module m in s\n\
               proc P() clobbers(af) { rts }\n";
    assert!(
        lower_diags(src).iter().any(|d| d.contains("[proc.clobber-invalid]")),
        "clobbers(af) in a 68k module must stay invalid (frozen 68k path)"
    );
}

// ---- gap 2 (§13.5): callee-declared-preserves oracle for z80_preserves ---------
//
// The sibling proof treated every `call` as clobber-all (no callee awareness), so
// `invariant(ix)` + every calling proc's `preserves` false-fired on psg's 5
// calling procs. This threads a callee-preserves map (each visible proc's declared
// `preserves` ∪ the module invariant; each `extern proc`'s declared `preserves`) —
// the Z80 analog of `preserves.rs`'s `CallPolicy::Oracle`.

/// A calling proc under `invariant(ix)` VERIFIES when the callee's own contract
/// preserves ix (a LOCAL callee, credited from its declared/inherited preserves).
/// Before the fix `call` clobbered all, so the caller fired.
#[test]
fn z80_call_to_ix_preserving_local_callee_verifies() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               proc Leaf() {\n\
                 ld a, (ix+0)\n\
                 ret\n\
               }\n\
               proc Caller() {\n\
                 call Leaf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a call to an ix-preserving local callee must verify invariant(ix), got: {diags:?}"
    );
}

/// An `extern proc`'s declared `preserves` is credited — the mechanism psg needs
/// for its cross-module callees (`Snd_ChanClass`/`Mod_ReArm`/`Mod_Advance`, all
/// ix-preserving). Before the fix the caller fired.
#[test]
fn z80_extern_callee_preserves_credited() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               extern proc Ext() preserves(ix)\n\
               proc Caller() {\n\
                 call Ext\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "an extern callee's declared preserves(ix) must be credited, got: {diags:?}"
    );
}

/// t24 NEGATIVE control: a call to a callee that does NOT preserve the register
/// still fires — the oracle credits only DECLARED preservation.
#[test]
fn z80_call_to_non_preserving_callee_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc Sub() clobbers(de) { ret }\n\
               proc Caller() preserves(de) {\n\
                 call Sub\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a call to a callee that does not preserve de must fire preserves(de), got: {diags:?}"
    );
}

/// t24 NEGATIVE control: an UNKNOWN (undeclared) callee stays conservative —
/// clobbers everything — so a caller's `preserves(de)` across it fires.
#[test]
fn z80_call_to_unknown_callee_conservative() {
    let src = "module m in s (cpu: z80)\n\
               proc Caller() preserves(de) {\n\
                 call SomethingUndeclared\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "an unknown callee must be conservative (clobbers de) — preserves(de) fires, got: {diags:?}"
    );
}

// ---- gap 3 (§13.5): the vacuous tail-jp pass — a soundness hole -----------------
//
// A proc ending in a tail `jp`/`jr` with NO local `ret` passed `preserves`
// VACUOUSLY (`!saw_return`), silently verifying a contract the body breaks. The
// tail transfer is now an EXIT: the preserved register must hold its entry value
// AT the jp AND the tail-callee must itself preserve it.

/// REPRODUCER (the watched fail): a proc that CLOBBERS the preserved register
/// (`push bc / pop ix` writes ix a foreign value) then tail-jumps out (`jp
/// External`) with no `ret` must FIRE. Before the fix `!saw_return` verified it.
#[test]
fn z80_tail_jp_clobber_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(ix) {\n\
                 push bc\n\
                 pop ix\n\
                 jp SomewhereExternal\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a tail-jp proc that clobbers the preserved ix must FIRE (not pass vacuously), got: {diags:?}"
    );
}

/// t24 POSITIVE control: a tail-jp proc that GENUINELY preserves ix (never writes
/// it) and tail-jumps to a callee that itself preserves ix still passes — the psg
/// `jp Psg_SetVolume` shape.
#[test]
fn z80_tail_jp_genuine_preserve_holds() {
    let src = "module m in s (cpu: z80)\n\
               extern proc Sink() preserves(ix)\n\
               proc P() preserves(ix) {\n\
                 ld a, (ix+0)\n\
                 jp Sink\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a tail-jp to an ix-preserving callee, ix untouched, must hold, got: {diags:?}"
    );
}

/// A tail-jp to an UNKNOWN external callee is conservative even when the body
/// preserves the register — the callee could clobber it (no contract). Fires.
#[test]
fn z80_tail_jp_unknown_callee_conservative() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(ix) {\n\
                 ld a, (ix+0)\n\
                 jp SomewhereUnknown\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a tail-jp to an unknown callee must be conservative — preserves(ix) fires, got: {diags:?}"
    );
}

// ======================================================================
// t36 — the `ex (sp),hl` sequencer trampoline (the demanded rung-3a feature).
// The `push hl` + `ex (sp),hl` + `ret` idiom is a COMPUTED DISPATCH through the
// return address, not a proc exit. The distinct `(sp)` operand form (Z80IndSp)
// lands here and routes to the SAME loud bail `ld sp` uses; the verdict
// tightening (ruling iii) restricts credit past the bail to module-invariant
// units only.
// ======================================================================

/// The `ex (sp),hl` operand form LANDS and bails a declared preserve — the real
/// trampoline spelling, replacing the `ld sp,hl` proxy of test (d). A non-invariant
/// register declared preserved across the swap is `[proc.preserves-unverifiable]`.
#[test]
fn z80_ex_sp_hl_trampoline_bails_declared_preserve() {
    let src = "module m in s (cpu: z80)\n\
               proc T() preserves(hl) {\n\
                 push hl\n\
                 ex (sp), hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "ex (sp),hl must bail a declared preserve, got: {diags:?}"
    );
}

/// THE TIGHTENING RED (ruling iii): a LOCALLY-UNTOUCHED, NON-invariant unit (`de`,
/// never written in this proc's body) declared preserved ACROSS the trampoline must
/// be REFUTED — the `ret` continues into a handler that could clobber `de`. This
/// fixture FALSE-PASSES against the pre-tightening verdict (a never-written unit
/// fell through to `all_returns_preserve`=true → silently Verified); post-fix it is
/// loudly `[proc.preserves-unverifiable]`. This is the demonstration the overseer
/// mandated.
#[test]
fn z80_trampoline_untouched_noninvariant_preserve_is_unverifiable() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               proc T() preserves(de) {\n\
                 push hl\n\
                 ex (sp), hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("de")),
        "a locally-untouched NON-invariant `de` preserved past the trampoline must be \
         Unverifiable (false-passes pre-tightening), got: {diags:?}"
    );
}

/// THE GREEN control: the MODULE INVARIANT (`ix`, never written locally + in the
/// invariant set) IS credited past the trampoline bail — verification-grade,
/// because the invariant is machine-checked on every dispatch target. A proc that
/// preserves ONLY the inherited invariant across the swap lowers clean.
#[test]
fn z80_trampoline_module_invariant_survives_bail() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               proc T() {\n\
                 push hl\n\
                 ex (sp), hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "the module invariant ix (never written, invariant-set) is creditable past \
         the trampoline bail, got: {diags:?}"
    );
}

/// The invariant is NOT blindly credited: a proc that WRITES `ix` locally
/// (`pop ix` — the move idiom clobbers it) then trampolines still fails the
/// invariant — ever_clobbered gates clause (a) of the tightening.
#[test]
fn z80_trampoline_locally_written_invariant_still_fires() {
    let src = "module m in s (cpu: z80, invariant: preserves(ix))\n\
               proc T() {\n\
                 push hl\n\
                 pop ix\n\
                 push hl\n\
                 ex (sp), hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("ix")),
        "ix written locally (pop ix) must fail the invariant even past a bail, got: {diags:?}"
    );
}

// ======================================================================
// The `falls_into` tail blind spot (the 4th mnemonic-table defect instance).
// A proc with a declared `falls_into` successor ends in FallOff, which used to
// checkpoint the tail as a plain exit WITHOUT consulting the successor's
// contract — so `preserves(rN)` passed even when the tail clobbers rN. The
// FallOff arm of a `falls_into` proc now applies the same logic the explicit
// `jr` tail (`Edge::TailOut`) takes: rN survives iff it holds its entry value AND
// the successor itself preserves it.
// ======================================================================

/// THE CATCH (synthetic `Psg_EmitDivisor`/`Psg_EmitDivisorTo` shape): `P`
/// preserves(b), never touches b in its own body, but FALLS INTO `S` whose tail
/// writes b (`ld b, a`). The successor's contract must be consulted —
/// `preserves(b)` is false. Before the fix this false-passed (the tail was never
/// looked at).
#[test]
fn z80_falls_into_tail_clobbering_successor_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(b) falls_into S {\n\
                 ld a, 1\n\
               }\n\
               proc S() clobbers(b) {\n\
                 ld b, a\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("preserves(b)")),
        "preserves(b) falling into a b-clobbering successor must fire, got: {diags:?}"
    );
}

/// THE POSITIVE TWIN: `P` preserves(b), never touches b, falls into `S` which
/// ALSO preserves(b) (and proves it — pure-`ld` body). The successor's contract
/// credits b, so `P` verifies. Mirrors the live `PsgVolEnv_Resolve →
/// VolEnv_ResolveScan` (c) seam that this fix makes genuinely prove.
#[test]
fn z80_falls_into_tail_preserving_successor_holds() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(b) falls_into S {\n\
                 ld a, 1\n\
               }\n\
               proc S() preserves(b) {\n\
                 ld a, 2\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "preserves(b) falling into a b-preserving successor must hold, got: {diags:?}"
    );
}

// ======================================================================
// Z80 `out(carry:) ∩ preserves(f/af)` overlap (a latent hole the Z80 branch of
// `check_out` skipped). A flag result lives in `f`, so a contract also
// PRESERVING the flags contradicts it. RULING: `clobbers(f)` + `out(carry:)`
// stays LEGAL (Z80 has no finer-than-`f` token — the only honest scratch
// spelling), diverging from the 68k sr.ccr rule.
// ======================================================================

/// `out(carry: ok) preserves(f)` — the flag result and the preserved flags
/// collide. Fires `[proc.out-preserves-overlap]`.
#[test]
fn z80_out_carry_preserves_f_overlap_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(carry: ok) preserves(f) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.out-preserves-overlap]")),
        "out(carry:) + preserves(f) must fire out-preserves-overlap, got: {diags:?}"
    );
}

/// `out(carry: ok) preserves(af)` — `af` expands to {a, f}, so the `f` half
/// collides with the flag result exactly as bare `f` does.
#[test]
fn z80_out_carry_preserves_af_overlap_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(carry: ok) preserves(af) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.out-preserves-overlap]")),
        "out(carry:) + preserves(af) must fire (af covers f), got: {diags:?}"
    );
}

/// NEGATIVE control: `out(carry: ok) preserves(a)` touches the accumulator, NOT
/// the flags — no overlap. (`scf` writes `f`, leaving `a` untouched, so
/// `preserves(a)` genuinely holds too.)
#[test]
fn z80_out_carry_preserves_a_no_overlap() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(carry: ok) preserves(a) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.out-preserves-overlap]")),
        "out(carry:) + preserves(a) is disjoint — no overlap, got: {diags:?}"
    );
}

/// THE RULING (negative control): `out(carry: ok) clobbers(f)` stays LEGAL — the
/// only honest Z80 spelling of "flags are scratch except the carry result". No
/// overlap of EITHER polarity fires.
#[test]
fn z80_out_carry_clobbers_f_stays_legal() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(carry: ok) clobbers(f) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.out-preserves-overlap]")
            || d.contains("[proc.out-clobbers-overlap]")),
        "out(carry:) + clobbers(f) must stay legal on Z80, got: {diags:?}"
    );
}

// ======================================================================
// The Z80 F-write model (`preserves(f)` false-pass hole). `z80_writes` modeled
// NO instruction as writing `f`, so `preserves(f) { scf; ret }` verified. `f` is
// now the complement of a flag-neutral allowlist: an instruction writes `f`
// unless provably flag-neutral (conservative — a forgotten neutral over-fires
// visibly rather than false-passing).
// ======================================================================

/// THE HOLE, closed: `scf` writes the flags, so `preserves(f)` is false. Before
/// the F-model this false-passed (no instruction wrote `f`).
#[test]
fn z80_preserves_f_scf_body_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(f) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("preserves(f)")),
        "preserves(f) across scf must fire, got: {diags:?}"
    );
}

/// POSITIVE control (`Psg_EnvCursorReset` shape): a memory-destination `ld`
/// (`ld (ix+0), 0`) writes NO register and never touches the flags, so
/// `preserves(af)` stays Verified. The F-model must not false-fire on the
/// flag-neutral allowlist.
#[test]
fn z80_preserves_af_pure_ld_body_holds() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(af) {\n\
                 ld (ix+0), 0\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "preserves(af) with a memory-dest ld body must hold, got: {diags:?}"
    );
}

/// POSITIVE control (`SndDrv_ISR` shape): a flag-writing body bracketed by
/// `push af`/`pop af` restores the entry flags, so `preserves(af)` holds — the
/// slot machinery models `pop af`'s `f` restore, not the F-write allowlist.
#[test]
fn z80_preserves_af_push_pop_bracket_holds() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(af) {\n\
                 push af\n\
                 scf\n\
                 pop af\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "preserves(af) with a push af/pop af bracket must hold, got: {diags:?}"
    );
}

/// The operand-sensitive `inc`/`dec` split, POSITIVE half: 16-bit `inc hl` (a
/// PAIR operand) touches no flags, so `preserves(f)` survives it.
#[test]
fn z80_preserves_f_16bit_inc_holds() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(f) {\n\
                 inc hl\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "16-bit inc hl leaves the flags — preserves(f) must hold, got: {diags:?}"
    );
}

/// The operand-sensitive `inc`/`dec` split, NEGATIVE half: 8-bit `inc b` WRITES
/// the flags, so `preserves(f)` is false across it.
#[test]
fn z80_preserves_f_8bit_inc_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(f) {\n\
                 inc b\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("preserves(f)")),
        "8-bit inc b writes the flags — preserves(f) must fire, got: {diags:?}"
    );
}

// ======================================================================
// Panel fix-up round (2026-08-05): the empty-body falls_into bypass, the djnz/
// ldir register-write gaps, the out_cond flag-rule bypass, and the exit-with-a-
// live-slot conservative bail.
// ======================================================================

/// M1 — empty-body `falls_into` bypass: `P` has NO instructions, so control flows
/// straight into `S`. `S` clobbers b, so `preserves(b)` is false. The instruction-
/// less early return must consult the successor oracle, not verify vacuously.
#[test]
fn z80_empty_body_falls_into_clobbering_successor_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(b) falls_into S {}\n\
               proc S() clobbers(b) {\n\
                 ld b, a\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("preserves(b)")),
        "an empty body falling into a b-clobbering successor must fire, got: {diags:?}"
    );
}

/// M1 POSITIVE twin: an empty body falling into a b-PRESERVING successor verifies —
/// the entry value flows straight through and the successor keeps it.
#[test]
fn z80_empty_body_falls_into_preserving_successor_holds() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(b) falls_into S {}\n\
               proc S() preserves(b) {\n\
                 ld a, 2\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "an empty body falling into a b-preserving successor must hold, got: {diags:?}"
    );
}

/// M2 — `djnz` writes its counter `b`. A `preserves(bc)` proc whose only b-writer is
/// the djnz loop must fire (before the arm, djnz wrote no register → false pass).
#[test]
fn z80_djnz_writes_counter_b_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(bc) {\n\
               .loop:\n\
                 djnz .loop\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("preserves(b)")),
        "djnz writes b — preserves(bc) must fire, got: {diags:?}"
    );
}

/// M3 — `ldir` writes the pointer/counter pairs bc, de, hl. A `preserves(hl)` proc
/// whose only hl-writer is a bare ldir must fire (before the arm, ldir wrote no
/// register → false pass). Isolates ldir: no other instruction touches hl.
#[test]
fn z80_ldir_writes_hl_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(hl) {\n\
                 ldir\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]") && d.contains("preserves(h)")),
        "ldir writes hl — preserves(hl) must fire, got: {diags:?}"
    );
}

/// S1 — a conditional register result (`out(rN if cc)`) reads its guard from the
/// flags, so `preserves(f)` contradicts it exactly as a flag result does. The Z80
/// out-flag rule must gate on `out_cond`, not only `out_flags`.
#[test]
fn z80_out_cond_preserves_f_overlap_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(a if eq) preserves(f) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.out-preserves-overlap]")),
        "out(a if eq) + preserves(f) must fire out-preserves-overlap, got: {diags:?}"
    );
}

/// S1 NEGATIVE probe: `out(a if eq) preserves(b)` — the conditional guard reads the
/// flags but `preserves(b)` claims a data register, not the flags. No overlap.
#[test]
fn z80_out_cond_preserves_b_no_overlap() {
    let src = "module m in s (cpu: z80)\n\
               proc P() out(a if eq) preserves(b) {\n\
                 scf\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.out-preserves-overlap]")),
        "out(a if eq) + preserves(b) is disjoint from the flags — no overlap, got: {diags:?}"
    );
}

/// S3 — an exit reached with a live push slot (a cross-seam `push` here / `pop` in
/// the successor) is conservatively UNVERIFIABLE: the local stack model cannot pair
/// the push. `push bc` then `ret` leaves bc on the stack at the exit, so a
/// locally-untouched `preserves(de)` is refused (it would false-pass without the bail).
#[test]
fn z80_exit_with_live_slot_bails() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(de) {\n\
                 push bc\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "an exit with a live push slot must bail (Unverifiable), got: {diags:?}"
    );
}

/// S3 POSITIVE control: a BALANCED `push bc`/`pop bc` leaves the stack empty at the
/// exit, so the bail does NOT fire and `preserves(de)` (de untouched) verifies.
#[test]
fn z80_exit_with_balanced_stack_holds() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(de) {\n\
                 push bc\n\
                 pop bc\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a balanced push/pop leaves the stack empty — preserves(de) must hold, got: {diags:?}"
    );
}

// ======================================================================
// The noreturn-tail exemption (the secondary item): `verify_z80_preserved`'s
// transfer-out arm consults `@noreturn` exactly as the 68k twin's
// `exit_diverges` does. A tail into a diverging target is not a return path, so
// it carries no preserve obligation. Every negative carries a positive control.
// ======================================================================

/// THE EXEMPTION (positive): `P preserves(de)` never writes `de` and tail-jumps
/// into a `@noreturn` handler. The diverging tail is not a return path, so
/// `preserves(de)` is NOT charged against it — no firing. Before the fix the tail
/// arm charged the callee oracle (`Dead` does not preserve `de`) and fired.
#[test]
fn z80_tail_into_noreturn_carries_no_preserve_obligation() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(de) {\n\
                 ld a, 1\n\
                 jp Dead\n\
               }\n\
               @noreturn\n\
               proc Dead() clobbers(af, bc, de, hl, ix, iy) {\n\
                 jp Dead\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a tail into a @noreturn handler is not a return path — preserves(de) must not \
         be charged against it, got: {diags:?}"
    );
}

/// t24 NEGATIVE control: the SAME shape with a NON-noreturn tail callee that
/// clobbers `de` still fires — the exemption spares only DIVERGING tails, not
/// ordinary ones. This is the mutant-revert of the fix (drop the `@noreturn`).
#[test]
fn z80_tail_into_ordinary_clobbering_callee_still_fires() {
    let src = "module m in s (cpu: z80)\n\
               proc P() preserves(de) {\n\
                 ld a, 1\n\
                 jp Live\n\
               }\n\
               proc Live() clobbers(af, bc, de, hl, ix, iy) {\n\
                 ld d, a\n\
                 ret\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.contains("[proc.preserves-unverifiable]")),
        "a tail into an ordinary de-clobbering callee is a real return path — \
         preserves(de) must still fire, got: {diags:?}"
    );
}
