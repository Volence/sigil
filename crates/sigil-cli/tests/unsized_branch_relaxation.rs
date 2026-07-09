//! Spec 2 · Plan 7 #8 — end-to-end byte-exactness proof for UNSIZED branch
//! relaxation (§5.4). An unsized `bra`/`bsr`/`Bcc` in a NON-`@as_compat` module
//! is sized by Core's relaxation over a two-rung `.s`→`.w` ladder; under
//! `@as_compat` it keeps the pre-§5.4 `[branch.missing-size]` pin requirement.
//! Explicit `.s`/`.w` stay pins everywhere.
//!
//! A `.emp` source is compiled through the FULL modern pipeline (parse →
//! `lower_module` → `resolve_layout` → `link` → `flatten`) and every asserted
//! byte is HAND-DERIVED (arithmetic in comments, house style) and — where the
//! resolved form has an AS spelling — cross-checked against Sigil's INDEPENDENT
//! AS front-end.
//!
//! The unsized ladder has NO far form (unlike `jbra`/`jbsr`): a conditional has
//! no unconditional `jmp` fallback, and §5.4 sizes `bra`/`bsr` uniformly the same
//! two rungs. Out of ±32K reach is Core's convergence error (`[branch.out-of-reach]`),
//! not a wider rung.
//!
//! ```text
//!   rung 0  <cc>.s  (6X dd)       PcRel8      @1   disp = target-(frag+2), i8, !=0
//!   rung 1  <cc>.w  (6X 00 ddDD)  PcRelDisp16 @2   disp = target-(frag+2), i16
//! ```

use sigil_frontend_as::{assemble, Options};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_link::LinkedImage;
use sigil_span::{Diagnostic, Level};

// ---------------------------------------------------------------------------
// Harness — mirrors `jbra_relaxation.rs`: link the modern pipeline through to a
// flat image, plus fallible variants that surface diagnostics (for the
// @as_compat pin error and the out-of-reach convergence error) rather than
// panicking.
// ---------------------------------------------------------------------------

fn emp_link(emp: &str) -> LinkedImage {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse errors: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "emp lower errors: {ldiags:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("emp resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("emp link failed: {d:?}"))
}

fn as_link(asm: &str) -> LinkedImage {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"))
}

fn emp_flat(emp: &str) -> Vec<u8> {
    sigil_link::flatten(&emp_link(emp), 0x00)
}
fn as_flat(asm: &str) -> Vec<u8> {
    sigil_link::flatten(&as_link(asm), 0x00)
}

/// Lowering diagnostics (parse must be clean) — for the `@as_compat` pin error,
/// which surfaces in `lower_module`, before layout runs.
fn emp_lower_diags(emp: &str) -> Vec<Diagnostic> {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse errors: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    lower_module(&file, &opts).1
}

/// The FIRST error diagnostic the pipeline produces, from lowering OR layout —
/// for the out-of-reach convergence error (which surfaces in `resolve_layout`).
fn emp_pipeline_err(emp: &str) -> Vec<Diagnostic> {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse errors: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == Level::Error) {
        return ldiags;
    }
    match sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true) {
        Ok(_) => vec![],
        Err(d) => d,
    }
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

// ---------------------------------------------------------------------------
// 1. Unsized `bne .near` → bne.s (rung 0) with a small forward displacement.
// ---------------------------------------------------------------------------

/// `bne` (cc=6, opcode high byte 0x66). A proc-local `.near` target 4 bytes
/// ahead, VMA 0:
///   0: bne .near   (rung 0: bne.s, 2 bytes, disp byte @ offset 1)
///   2: nop         (4E 71)
///   4: .near: rts  (4E 75)
/// disp = target - (frag+2) = 4 - 2 = 2, fits i8 and != 0 → bne.s. Byte 1 = 0x02.
#[test]
fn unsized_bne_near_forward_lands_bcc_s() {
    let emp = "module m\n\
               proc f() {\n\
                   bne .near\n\
                   nop\n\
               .near:\n\
                   rts\n\
               }\n";
    let want = vec![0x66, 0x02, 0x4E, 0x71, 0x4E, 0x75];
    assert_eq!(emp_flat(emp), want, "unsized bne .near → bne.s disp 2");
    // Independent AS cross-check: the resolved form is exactly `bne.s near`.
    let asm = "\tbne.s near\n\tnop\nnear:\n\trts\n";
    assert_eq!(as_flat(asm), want, "AS `bne.s near` equals the resolved bne bytes");
}

// ---------------------------------------------------------------------------
// 2. Unsized `bhi .back` → bhi.s with a NEGATIVE displacement (backward).
// ---------------------------------------------------------------------------

/// `bhi` (cc=2, opcode high byte 0x62). Backward branch to an earlier label:
///   0: .back: nop  (4E 71)
///   2: bhi .back   (rung 0: bhi.s, frag @ 2)
/// disp = target - (frag+2) = 0 - 4 = -4 → 0xFC. Image = `4E 71 62 FC`.
#[test]
fn unsized_bhi_backward_lands_bcc_s_negative() {
    let emp = "module m\n\
               proc f() {\n\
               .back:\n\
                   nop\n\
                   bhi .back\n\
               }\n";
    let want = vec![0x4E, 0x71, 0x62, 0xFC];
    assert_eq!(emp_flat(emp), want, "backward bhi → bhi.s disp -4");
    let asm = "back:\n\tnop\n\tbhi.s back\n";
    assert_eq!(as_flat(asm), want, "AS `bhi.s back` equals the resolved bhi bytes");
}

// ---------------------------------------------------------------------------
// 3. Far intra-module target → bne.w (out of i8 range, within i16).
// ---------------------------------------------------------------------------

/// A forward target padded 200 bytes away: too far for `.s`, within `.w`.
///   0: bne Far       (relaxes to rung 1: bne.w, 4 bytes)
///   4: Pad           (200 bytes of data)
///   204: Far: rts    (4E 75)
/// bne.s (2B) would put Far @ 202, disp = 202-2 = 200 > 127 → grow.
/// bne.w (4B) puts Far @ 204, disp = 204-2 = 202 = 0x00CA, fits i16 → bne.w.
/// Image head = `66 00 00 CA`, Far byte @ 204 = `4E 75`.
#[test]
fn unsized_bne_far_lands_bcc_w() {
    let pad = std::iter::repeat_n("$00", 200).collect::<Vec<_>>().join(", ");
    let emp = format!(
        "module m\n\
         proc f() {{\n\
             bne Far\n\
         }}\n\
         data Pad: [u8; 200] = [{pad}]\n\
         data Far: [u8; 2] = [$4E, $75]\n"
    );
    let bytes = emp_flat(&emp);
    // frag @ 0; Far @ 4 (bne.w) + 200 (pad) = 204. disp = 204 - 2 = 202 = 0x00CA.
    assert_eq!(&bytes[0..4], &[0x66, 0x00, 0x00, 0xCA], "unsized bne Far → bne.w disp 202");
    assert_eq!(&bytes[204..206], &[0x4E, 0x75], "Far data at offset 204");
    let asm = format!("\tbne.w Far\nPad:\n\tdc.b {pad}\nFar:\n\tdc.b $4E, $75\n");
    assert_eq!(as_flat(&asm), bytes, "AS `bne.w Far` equals the resolved bne image");
}

// ---------------------------------------------------------------------------
// 4. Unsized `bra`/`bsr` relax the SAME two rungs (§5.4 uniformity).
// ---------------------------------------------------------------------------

/// Unsized `bra .near` → bra.s (rung 0), the unconditional twin of case 1.
#[test]
fn unsized_bra_near_lands_bra_s() {
    let emp = "module m\n\
               proc f() {\n\
                   bra .near\n\
                   nop\n\
               .near:\n\
                   rts\n\
               }\n";
    let want = vec![0x60, 0x02, 0x4E, 0x71, 0x4E, 0x75];
    assert_eq!(emp_flat(emp), want, "unsized bra .near → bra.s disp 2");
    let asm = "\tbra.s near\n\tnop\nnear:\n\trts\n";
    assert_eq!(as_flat(asm), want, "AS `bra.s near` equals the resolved bra bytes");
}

/// Unsized `bsr Far` → bsr.w (rung 1): the call twin of case 3, 200-byte pad.
#[test]
fn unsized_bsr_far_lands_bsr_w() {
    let pad = std::iter::repeat_n("$00", 200).collect::<Vec<_>>().join(", ");
    let emp = format!(
        "module m\n\
         proc f() {{\n\
             bsr Far\n\
         }}\n\
         data Pad: [u8; 200] = [{pad}]\n\
         data Far: [u8; 2] = [$4E, $75]\n"
    );
    let bytes = emp_flat(&emp);
    // bsr opcode high byte 0x61; disp = 204 - 2 = 202 = 0x00CA.
    assert_eq!(&bytes[0..4], &[0x61, 0x00, 0x00, 0xCA], "unsized bsr Far → bsr.w disp 202");
    let asm = format!("\tbsr.w Far\nPad:\n\tdc.b {pad}\nFar:\n\tdc.b $4E, $75\n");
    assert_eq!(as_flat(&asm), bytes, "AS `bsr.w Far` equals the resolved bsr image");
}

// ---------------------------------------------------------------------------
// 5. disp-0 exclusion: an unsized branch to the immediately-following byte must
//    take the WORD rung (bra.s disp 0 is the 68000 word-form escape, unencodable).
// ---------------------------------------------------------------------------

/// `bra .next` where `.next` is the instruction immediately after the branch.
///   0: bra .next    (frag @ 0)
///   .next: rts      (the branch's own baseline end)
/// bra.s disp = target-(frag+2). At rung 0 (2 bytes) target would be @ 2, disp =
/// 2-2 = 0 → EXCLUDED (0x00 byte is the word-form escape). So the ladder grows to
/// rung 1 (bra.w, 4 bytes): target @ 4, disp = 4-2 = 2 = 0x0002. Image head =
/// `60 00 00 02`, then rts `4E 75`.
#[test]
fn unsized_bra_disp0_takes_word_rung() {
    let emp = "module m\n\
               proc f() {\n\
                   bra .next\n\
               .next:\n\
                   rts\n\
               }\n";
    let bytes = emp_flat(emp);
    assert_eq!(&bytes[0..4], &[0x60, 0x00, 0x00, 0x02], "disp-0 excludes bra.s → bra.w disp 2");
    assert_eq!(&bytes[4..6], &[0x4E, 0x75], "rts at offset 4");
}

// ---------------------------------------------------------------------------
// 6. MIXED proc: an unsized bne + a sized bne.s + a jbra, all correct at once.
// ---------------------------------------------------------------------------

/// One proc with three branch idioms to the SAME `.tgt` label; each must resolve
/// independently and correctly in a single layout:
///   0: bne .tgt     (UNSIZED → relaxes; rung 0 bne.s, disp = 8-2 = 6 → 66 06)
///   2: bne.s .tgt   (SIZED PIN → bne.s, disp = 8-4 = 4 → 66 04)
///   4: jbra .tgt    (emp auto-reach → bra.s, disp = 8-6 = 2 → 60 02)
///   6: nop          (4E 71)
///   8: .tgt: rts    (4E 75)
/// jbra reserves its 6-byte MAX span for placement, but the resolved form is the
/// 2-byte bra.s — so `.tgt` sits at the post-relaxation offset 8. Because all
/// three fragments end up 2 bytes each, layout does not shift and every disp is
/// measured against `.tgt` @ 8.
#[test]
fn mixed_unsized_sized_and_jbra_all_resolve() {
    let emp = "module m\n\
               proc f() {\n\
                   bne .tgt\n\
                   bne.s .tgt\n\
                   jbra .tgt\n\
                   nop\n\
               .tgt:\n\
                   rts\n\
               }\n";
    let bytes = emp_flat(emp);
    // Unsized bne @ 0 → bne.s disp = 8 - (0+2) = 6.
    assert_eq!(&bytes[0..2], &[0x66, 0x06], "unsized bne → bne.s disp 6");
    // Sized bne.s @ 2 (a PIN) → disp = 8 - (2+2) = 4.
    assert_eq!(&bytes[2..4], &[0x66, 0x04], "sized bne.s pin → disp 4");
    // jbra @ 4 → bra.s disp = 8 - (4+2) = 2.
    assert_eq!(&bytes[4..6], &[0x60, 0x02], "jbra → bra.s disp 2");
    assert_eq!(&bytes[6..10], &[0x4E, 0x71, 0x4E, 0x75], "nop then .tgt: rts");
}

// ---------------------------------------------------------------------------
// 7. Out-of-reach: an unsized bne to a target > 32K away → `[branch.out-of-reach]`.
// ---------------------------------------------------------------------------

/// A forward target padded past the i16 word-branch range. bne.w's disp word is
/// measured from the branch @ VMA 0; a target ~40000 bytes ahead exceeds ±32766,
/// so the ladder maxes at `.w` and still cannot reach → Core's convergence error,
/// through the REAL pipeline. The message names the signed distance.
#[test]
fn unsized_bne_out_of_reach_is_core_error() {
    let n = 40000usize; // > 0x7FFF (32767) — beyond bne.w reach.
    let pad = std::iter::repeat_n("$00", n).collect::<Vec<_>>().join(", ");
    let emp = format!(
        "module m\n\
         proc f() {{\n\
             bne Far\n\
         }}\n\
         data Pad: [u8; {n}] = [{pad}]\n\
         data Far: [u8; 2] = [$4E, $75]\n"
    );
    let errs = emp_pipeline_err(&emp);
    assert!(
        has_tag(&errs, "[branch.out-of-reach]"),
        "an unreachable unsized bne must be Core's out-of-reach error, got: {errs:?}"
    );
    // The message names the signed distance (a positive number here, forward)
    // and steers BOTH mnemonic classes (Core cannot see which built the ladder):
    // jbcc-deferred for a conditional, jbra/jbsr for an unconditional bra/bsr.
    assert!(
        errs.iter().any(|d| d.message.contains("bytes away")
            && d.message.contains("jbcc trampolines are deferred, D2.18")
            && d.message.contains("use jbra/jbsr instead")),
        "the out-of-reach message must name the distance and both steers, got: {errs:?}"
    );
}

// ---------------------------------------------------------------------------
// 8. `@as_compat`: an unsized branch keeps the `[branch.missing-size]` pin error,
//    VERBATIM; sized branches under `@as_compat` stay byte-identical.
// ---------------------------------------------------------------------------

/// Under `@as_compat` an unsized `bne` is a defect (a faithful AS port pins every
/// branch width) → `[branch.missing-size]`, the exact pre-§5.4 message.
#[test]
fn as_compat_unsized_branch_keeps_missing_size_error() {
    let emp = "module m\n@as_compat\n\
               proc f() {\n\
                   bne .tgt\n\
               .tgt:\n\
                   rts\n\
               }\n";
    let diags = emp_lower_diags(emp);
    assert!(
        diags.iter().any(|d| d.message.contains("[branch.missing-size]")
            && d.message.contains("Aeon pins branch width")),
        "under @as_compat an unsized branch must keep the verbatim missing-size error, got: {diags:?}"
    );
}

/// Under `@as_compat` an EXPLICITLY-sized branch lowers byte-identically to a
/// non-compat build (the attribute steers only the unsized-branch decision).
#[test]
fn as_compat_sized_branch_is_byte_identical() {
    let sized = "\
               proc f() {\n\
                   bne.s .tgt\n\
                   nop\n\
               .tgt:\n\
                   rts\n\
               }\n";
    let with_compat = format!("module m\n@as_compat\n{sized}");
    let without = format!("module m\n{sized}");
    // bne.s @ 0, disp = target-(frag+2) = 4-2 = 2 → `66 02`, nop, rts.
    let want = vec![0x66, 0x02, 0x4E, 0x71, 0x4E, 0x75];
    assert_eq!(emp_flat(&with_compat), want, "@as_compat sized bne.s bytes");
    assert_eq!(
        emp_flat(&with_compat),
        emp_flat(&without),
        "sized branch is byte-identical with/without @as_compat"
    );
}

// ---------------------------------------------------------------------------
// 9. Terminator: a proc ending in an unsized `bra X` must NOT warn undeclared
//    fallthrough (an unconditional `bra` terminates, sized or not — the analysis
//    keys on the mnemonic string "bra", so unsized inherits it).
// ---------------------------------------------------------------------------

#[test]
fn unsized_bra_terminates_proc_no_fallthrough_warning() {
    // `f` ends in unsized `bra X`; `X` is another proc. No `[proc.undeclared-
    // fallthrough]` warning — `bra` is an unconditional terminator regardless of
    // size. (X itself ends in rts, so it does not warn either.)
    let emp = "module m\n\
               proc f() {\n\
                   moveq #0, d0\n\
                   bra X\n\
               }\n\
               proc X() {\n\
                   rts\n\
               }\n";
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse errors: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    let (_module, diags) = lower_module(&file, &opts);
    assert!(
        !has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "a proc ending in unsized `bra` must not warn undeclared-fallthrough, got: {diags:?}"
    );
    // Sanity: the resolved image is a bra.s to X. f @ 0: moveq (70 00) + bra.s X;
    // X @ 4. bra.s @ 2, disp = 4 - (2+2) = 0 → EXCLUDED, grows to bra.w: X @ 6,
    // disp = 6 - (2+2) = 2. So image = 70 00 | 60 00 00 02 | 4E 75.
    let bytes = emp_flat(emp);
    assert_eq!(bytes, vec![0x70, 0x00, 0x60, 0x00, 0x00, 0x02, 0x4E, 0x75], "resolved image");
}
