//! `dispatch Name (encoding: word_offsets) { Member: target, ... }` — the
//! encoding-agnostic typed state-dispatch table (Spec 2, Plan 7 backlog #6,
//! Part B — D6.B2/B3/B4/B5). This task ships the `word_offsets` encoding:
//! forward emission (`dc.w member_target - Name` per member, on the shipped
//! `offsets` RelOffset machinery), the pre-scaled `Name.Member` / `Name.count`
//! ordinals, and the module-local `[dispatch.target-not-code]` kind check.
//! `long_ptrs` EMISSION is a later task (Task 11).
//!
//! Each case parses a full `.emp` file, lowers it via the same `lower_module`
//! entry the CLI uses, and asserts on the resulting diagnostics / linked bytes
//! (mirroring `overlay.rs`'s harness).

use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

/// Lower `src` (asserting a clean parse) and return `(module, diagnostic messages)`.
fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn msgs(src: &str) -> Vec<String> {
    lower(src).1
}

/// Link the lowered module and return the bytes of its (single) default section.
fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

/// Evaluate the const named `name` in `src`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (v, diags) = eval_const(&file, name);
    (v, diags.into_iter().map(|d| d.message).collect())
}

// ---- 1. byte-exact word_offsets table ------------------------------------

#[test]
fn word_offsets_table_bytes_are_exact() {
    // A `word_offsets` dispatch emits `dc.w member_target - Name` per member,
    // in declaration order, with the base label `Routines` at the table's
    // first byte. Followed by two procs (`init` at +4, `wait` at +6, each
    // `rts` = 4E 75). So the full default-section image is:
    //   table:  00 04  00 06   (init - Routines = 4, wait - Routines = 6)
    //   init:   4E 75
    //   wait:   4E 75
    let src = "\
module m
dispatch Routines (encoding: word_offsets) {
    Init: init,
    Wait: wait,
}
proc init() { rts }
proc wait() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x04, 0x00, 0x06, 0x4E, 0x75, 0x4E, 0x75]);
}

// ---- 2. scaled ordinals + count ------------------------------------------

#[test]
fn ordinals_are_prescaled_and_count_is_unscaled() {
    // `word_offsets` ordinals are pre-scaled ×2 (D6.B3): Init=0, Wait=2.
    // `Routines.count` is UNSCALED = member count = 2.
    // A `data` item using them as `u8`s emits those exact bytes: 00 02 02.
    let src = "\
module m
dispatch Routines (encoding: word_offsets) {
    Init: init,
    Wait: wait,
}
proc init() { rts }
proc wait() { rts }
data ids: [u8; 3] = [Routines.Init, Routines.Wait, Routines.count]
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    let bytes = linked_bytes(&module);
    // table (4) + init (2) + wait (2) = 8, then the 3 data bytes.
    assert_eq!(&bytes[8..11], &[0x00, 0x02, 0x02]);
}

#[test]
fn ordinal_via_const_is_prescaled() {
    let src = "\
module m
dispatch R (encoding: word_offsets) { A: a, B: b, C: c }
const X = R.B
const Y = R.count
";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(Value::Int(2)), "R.B = ordinal 1 * scale 2 = 2");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let (v, diags) = eval(src, "Y");
    assert_eq!(v, Some(Value::Int(3)), "R.count = 3, unscaled");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn unknown_member_is_an_error() {
    let src = "\
module m
dispatch R (encoding: word_offsets) { A: a, B: b }
const X = R.Nope
";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(diags[0].contains("no member"), "diagnostic was {:?}", diags[0]);
}

// ---- 3. reserved `count` + duplicate member ------------------------------

#[test]
fn member_named_count_is_reserved() {
    let src = "\
module m
dispatch R (encoding: word_offsets) { count: c }
proc c() { rts }
";
    let errs = msgs(src);
    assert_eq!(errs.iter().filter(|m| m.contains("reserved")).count(), 1, "errs: {errs:?}");
}

#[test]
fn duplicate_member_name_is_an_error() {
    let src = "\
module m
dispatch R (encoding: word_offsets) { A: a, A: b }
proc a() { rts }
proc b() { rts }
";
    let errs = msgs(src);
    assert_eq!(errs.iter().filter(|m| m.contains("duplicate")).count(), 1, "errs: {errs:?}");
}

#[test]
fn duplicate_member_reported_once_through_full_lowering() {
    // The dup-member check must fire ONCE per compile, not once per per-item
    // evaluator (mirrors the offsets exemplar). With ≥2 data items present,
    // assert EXACTLY ONE "duplicate" diagnostic.
    let src = "\
module m
dispatch R (encoding: word_offsets) { A: a, A: b }
proc a() { rts }
proc b() { rts }
data D1: [u8; 1] = [1]
data D2: [u8; 1] = [2]
";
    let errs = msgs(src);
    assert_eq!(errs.iter().filter(|m| m.contains("duplicate")).count(), 1, "errs: {errs:?}");
}

// ---- 4. [dispatch.target-not-code] ---------------------------------------

#[test]
fn target_data_item_is_not_code() {
    // A member targeting a module-local `data` item is a hard error — a
    // dispatch table into data is exactly the jump-to-garbage this construct
    // exists to kill.
    let src = "\
module m
dispatch Routines (encoding: word_offsets) { Init: init }
data init: [u8; 1] = [0]
";
    let errs = msgs(src);
    assert_eq!(
        errs.iter().filter(|m| m.contains("[dispatch.target-not-code]")).count(),
        1,
        "errs: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("data item")),
        "message should name the kind (data item): {errs:?}"
    );
}

#[test]
fn target_const_is_not_code() {
    let src = "\
module m
const init = 7
dispatch Routines (encoding: word_offsets) { Init: init }
";
    let errs = msgs(src);
    assert_eq!(
        errs.iter().filter(|m| m.contains("[dispatch.target-not-code]")).count(),
        1,
        "errs: {errs:?}"
    );
}

#[test]
fn undefined_target_is_left_to_link_not_module_local_error() {
    // A name that resolves module-locally to NOTHING gets no early
    // [dispatch.target-not-code] error — it is left to link (v1 does not
    // kind-check cross-module). But it must fail LOUDLY at link time with an
    // unknown-symbol error, not silently.
    let src = "\
module m
dispatch Routines (encoding: word_offsets) { Init: nowhere }
";
    let (module, errs) = lower(src);
    assert!(
        errs.iter().all(|m| !m.contains("[dispatch.target-not-code]")),
        "an undefined name is NOT a module-local kind error: {errs:?}"
    );
    // Link must fail LOUDLY on the unresolved target (not silently emit a bad
    // word). The linker reports an unresolved-target-expression error for the
    // dangling `nowhere` symbol — the same loud failure an `offsets` table with
    // an undefined target hits.
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve");
    let link = sigil_link::link(&resolved, &SymbolTable::new());
    assert!(link.is_err(), "link should fail on the undefined target `nowhere`");
    let e = format!("{:?}", link.err().unwrap());
    assert!(
        e.contains("unresolved"),
        "link error should be a loud unresolved-target failure: {e}"
    );
}

// ---- 5. signed-word range (mirror the offsets range machinery) -----------
//
// The offsets emission covers the signed-word range purely at the fixup /
// linker level (RelWord16Be) — the frontend never range-checks a symbolic
// `target - base` at lower time (it cannot without addresses). So there is no
// frontend-level range unit test to mirror for dispatch either: an
// out-of-range word offset surfaces as a link-time relocation-overflow, on
// the SAME RelWord16Be kind dispatch reuses. Asserting a duplicate of the
// linker's own range test here would test sigil-link, not this task. The
// byte-exact test above is the emission proof; range is the linker's contract.

// ---- 6. [dispatch.non-68k] -----------------------------------------------

#[test]
fn dispatch_in_z80_section_is_non_68k() {
    // A dispatch in a `cpu: z80` section is rejected, mirroring
    // `[offsets.non-68k]`.
    let src = "\
module m
section s (cpu: z80, vma: $8000) {
    dispatch R (encoding: word_offsets) { A: a, B: b }
    proc a() { rts }
    proc b() { rts }
}
";
    let (_module, errs) = lower(src);
    assert!(
        errs.iter().any(|m| m.contains("[dispatch.non-68k]")),
        "expected [dispatch.non-68k]: {errs:?}"
    );
}

// ---- 8. section-nested dispatch lowers -----------------------------------

#[test]
fn section_nested_dispatch_lowers() {
    // A dispatch inside a 68k `section { }` lowers exactly like a top-level
    // one (both item loops handle `Item::Dispatch`).
    let src = "\
module m
section s (cpu: m68000, vma: $8000) {
    dispatch Routines (encoding: word_offsets) {
        Init: init,
        Wait: wait,
    }
    proc init() { rts }
    proc wait() { rts }
}
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    // Same table bytes as the top-level case.
    let bytes = linked_bytes(&module);
    assert_eq!(&bytes[0..4], &[0x00, 0x04, 0x00, 0x06]);
}
