//! `dc.w <local-label> - <local-label>` — the in-proc self-relative offset
//! word (the AS inline jump-table idiom: `dc.w .target - .base`). Extends the
//! EXISTING symbol-difference semantic (already shipped for `offsets`/`dispatch`
//! and the `extern("X") - extern("Y")` data form, both lowering to
//! `Cell::RelOffset`) to proc-LOCAL labels in `dc` data position.
//!
//! Ratified 2026-07-29 (t38, Option A) under six constraints, each a test here:
//!   1. SUB of TWO labels only — `label + label`, `label + int`, `int - label`
//!      all keep the loud label-arithmetic error.
//!   2. data position, `dc.w` only — `dc.b`/`dc.l` reject (a self-rel offset is
//!      a signed word); outside `dc` a label difference is untouched.
//!   3. same-section: proc-LOCAL labels only (structurally same-section). A
//!      global/cross-module difference is NOT this form (falls through to the
//!      loud error; `offsets`/`dispatch` own the cross-module table).
//!   4. word-range overflow errors (the RelWord16Be i16 check), never truncates.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

// ---- constraint 5 RED: the demanded feature (the .case_table shape) ----------

#[test]
fn self_rel_dc_w_emits_reloffset_words() {
    // A minimal inline self-relative table: `.t` is the base, `.a`/`.b` the
    // (forward, positive) targets — the Player_SensorSurface `.case_table`
    // shape reduced to two entries. Layout: table 4 bytes at 0; `.a` at 4,
    // `.b` at 8. So word0 = .a-.t = 4, word1 = .b-.t = 8; bodies follow.
    let src = "\
module m
proc Foo () clobbers(d2) {
.t:
        dc.w    .a - .t
        dc.w    .b - .t
.a:
        moveq   #1, d2
        rts
.b:
        moveq   #2, d2
        rts
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(
        linked_bytes(&m),
        vec![0x00, 0x04, 0x00, 0x08, 0x74, 0x01, 0x4E, 0x75, 0x74, 0x02, 0x4E, 0x75],
    );
}

#[test]
fn self_rel_negative_offset_two_complement() {
    // Target BEFORE base (a backward reference): `.a` at 0, the table at 4.
    // word = .a - .t = -4 = 0xFFFC.
    let src = "\
module m
proc Foo () clobbers(d2) {
.a:
        moveq   #1, d2
        rts
.t:
        dc.w    .a - .t
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![0x74, 0x01, 0x4E, 0x75, 0xFF, 0xFC]);
}

// ---- constraint 1: SUB of TWO labels ONLY ------------------------------------

#[test]
fn label_plus_label_still_rejects() {
    let src = "\
module m
proc Foo () clobbers(d2) {
.t:
        dc.w    .a + .t
.a:
        rts
}
";
    let (_m, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("not defined for") && m.contains("label")),
        "`label + label` must keep the loud label-arithmetic error: {msgs:?}"
    );
}

#[test]
fn label_plus_int_still_rejects() {
    let src = "\
module m
proc Foo () clobbers(d2) {
.t:
        dc.w    .t + 2
.a:
        rts
}
";
    let (_m, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("not defined for") && m.contains("label")),
        "`label + int` must keep the loud label-arithmetic error: {msgs:?}"
    );
}

#[test]
fn int_minus_label_still_rejects() {
    let src = "\
module m
proc Foo () clobbers(d2) {
.t:
        dc.w    2 - .t
.a:
        rts
}
";
    let (_m, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("not defined for") && m.contains("label")),
        "`int - label` must keep the loud label-arithmetic error: {msgs:?}"
    );
}

// ---- constraint 2: dc.w only -------------------------------------------------

#[test]
fn self_rel_dc_l_rejects() {
    let src = "\
module m
proc Foo () clobbers(d2) {
.t:
        dc.l    .a - .t
.a:
        rts
}
";
    let (_m, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("dc.self-rel") || m.contains("dc.w")),
        "a self-rel offset is `dc.w`-only; `dc.l` must reject: {msgs:?}"
    );
}

#[test]
fn self_rel_dc_b_rejects() {
    let src = "\
module m
proc Foo () clobbers(d2) {
.t:
        dc.b    .a - .t
.a:
        rts
}
";
    let (_m, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("dc.self-rel") || m.contains("dc.w")),
        "a self-rel offset is `dc.w`-only; `dc.b` must reject: {msgs:?}"
    );
}

// ---- constraint 3: proc-LOCAL labels only (same-section) ---------------------

#[test]
fn global_label_difference_rejects() {
    // Two GLOBAL labels (not proc-local) — a cross-section-risky difference is
    // NOT this form. It falls through to the loud label-arithmetic error, never
    // a silent fold. `offsets`/`dispatch` own the cross-module table case.
    let src = "\
module m
data Base: [u8; 1] = [0]
data Tgt:  [u8; 1] = [0]
proc Foo () clobbers(d2) {
        dc.w    Tgt - Base
        rts
}
";
    let (_m, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("not defined for") && m.contains("label")),
        "a global-label difference must keep the loud label-arithmetic error (no silent cross-section fold): {msgs:?}"
    );
}

// ---- constraint 4: word-range overflow errors, never truncates ---------------

#[test]
fn self_rel_overflow_diagnoses() {
    // A target pushed past +0x7FFF from the base overflows the signed word. The
    // feature emits `Cell::RelOffset` → the linker's RelWord16Be i16 range check
    // fires (an Error, not a truncated fold — the RelWord16Be write never masks).
    // The gap is a `dc.b` string byte-run of 0x8000 bytes: `.t` at 0, table word
    // 2 bytes, filler 0x8000, so `.a` lands at 0x8002 → offset 0x8002 > 0x7FFF.
    let filler: String = "A".repeat(0x8000);
    let src = format!(
        "module m\n\
         proc Foo () clobbers(d2) {{\n\
         .t:\n\
                 dc.w    .a - .t\n\
                 dc.b    \"{filler}\"\n\
         .a:\n\
                 rts\n\
         }}\n"
    );
    let (file, perrs) = parse_str(&src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let lower_msgs: Vec<String> = diags.into_iter().map(|d| d.message).collect();
    assert!(lower_msgs.is_empty(), "clean lower expected (overflow is a link check): {lower_msgs:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let link_res = sigil_link::link(&resolved, &SymbolTable::new());
    let err = link_res.err().expect("a >i16 self-rel offset must be a link Error, not a truncated fold");
    assert!(
        err.iter().any(|d| d.message.contains("out of") && d.message.contains("word")),
        "the overflow error must name the signed-word range: {err:?}"
    );
}
