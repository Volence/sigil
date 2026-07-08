//! `dispatch Name (encoding: word_offsets) { Member: target, ... }` — the
//! encoding-agnostic typed state-dispatch table (Spec 2, Plan 7 backlog #6,
//! Part B — D6.B2/B3/B4/B5). Both v1 encodings ship: `word_offsets` forward
//! emission (`dc.w member_target - Name` per member, on the shipped `offsets`
//! RelOffset machinery) and `long_ptrs` (`dc.l target` Abs32 pointers, reusing
//! the struct-data label-pointer `SymRef` cell), plus the pre-scaled
//! `Name.Member` / `Name.count` ordinals (×2 / ×4) and the module-local
//! `[dispatch.target-not-code]` kind check.
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
fn long_ptrs_ordinals_are_prescaled_x4_and_count_is_unscaled() {
    // `long_ptrs` ordinals are pre-scaled ×4 (D6.B3): A=0, B=4, C=8.
    // `R.count` is UNSCALED = member count = 3. A `data` item reading
    // `[R.B, R.count, R.A]` as `u8`s emits 04 03 00. (The ordinals resolve via
    // `eval_path` regardless of the forward-emission task — live TODAY.)
    let src = "\
module m
dispatch R (encoding: long_ptrs) { A: x, B: y, C: z }
proc x() { rts }
proc y() { rts }
proc z() { rts }
data ids: [u8; 3] = [R.B, R.count, R.A]
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    let bytes = linked_bytes(&module);
    // The `data ids` item is emitted LAST, so its 3 bytes are the tail of the
    // image regardless of the forward table's size — this test pins the ×4
    // ordinals specifically, decoupled from the `long_ptrs` emission.
    assert_eq!(&bytes[bytes.len() - 3..], &[0x04, 0x03, 0x00]);
}

#[test]
fn empty_dispatch_builds_and_count_is_zero() {
    // An empty dispatch table builds with no diagnostics and emits no forward
    // bytes; `R.count` is 0. Assert via a data byte so the whole pipeline runs.
    let src = "\
module m
dispatch R (encoding: word_offsets) { }
data c: [u8; 1] = [R.count]
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    let bytes = linked_bytes(&module);
    // The table is 0 bytes, so the data byte is at offset 0.
    assert_eq!(bytes[0], 0x00, "R.count == 0");
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
fn target_offsets_table_is_not_code() {
    // A member targeting a module-local `offsets` table names "offset table".
    let src = "\
module m
offsets Tbl { A: init }
dispatch Routines (encoding: word_offsets) { Init: Tbl }
proc init() { rts }
";
    let errs = msgs(src);
    assert_eq!(
        errs.iter().filter(|m| m.contains("[dispatch.target-not-code]")).count(),
        1,
        "errs: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("offset table")),
        "message should name the kind (offset table): {errs:?}"
    );
}

#[test]
fn target_dispatch_table_is_not_code() {
    // A member targeting a module-local `dispatch` table (here a self-reference)
    // names "dispatch table".
    let src = "\
module m
dispatch R (encoding: word_offsets) { A: R }
";
    let errs = msgs(src);
    assert_eq!(
        errs.iter().filter(|m| m.contains("[dispatch.target-not-code]")).count(),
        1,
        "errs: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("dispatch table")),
        "message should name the kind (dispatch table): {errs:?}"
    );
}

#[test]
fn target_overlay_is_not_code() {
    // A member targeting a module-local overlay (`vars Name: window { .. }`)
    // names "overlay".
    let src = "\
module m
struct S (size: $8) { pad: [u8; 8] @ $0 }
vars V: pad { t: u8 }
dispatch Routines (encoding: word_offsets) { Init: V }
";
    let errs = msgs(src);
    assert_eq!(
        errs.iter().filter(|m| m.contains("[dispatch.target-not-code]")).count(),
        1,
        "errs: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("overlay")),
        "message should name the kind (overlay): {errs:?}"
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

// ---- 7. section-nested dispatch lowers -----------------------------------

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

// ---- 8. long_ptrs emission (D6.B2) ---------------------------------------

#[test]
fn long_ptrs_table_bytes_are_exact() {
    // A `long_ptrs` dispatch emits `dc.l target` (4-byte ABSOLUTE pointer,
    // big-endian, Abs32 fixup) per member in declaration order, base label
    // `Routines` at the table's first byte. Followed by two procs.
    // The harness default-section origin is 0 (the word_offsets test's image
    // `00 04 00 06...` = init−Routines = 4 with Routines@0), so absolute
    // addresses ARE the offsets:
    //   table (8 bytes): Routines@0 → init@8, wait@10 (0x0A)
    //     dc.l init = 00 00 00 08
    //     dc.l wait = 00 00 00 0A
    //   init@8:  4E 75
    //   wait@10: 4E 75
    let src = "\
module m
dispatch Routines (encoding: long_ptrs) {
    Init: init,
    Wait: wait,
}
proc init() { rts }
proc wait() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0A, 0x4E, 0x75, 0x4E, 0x75]
    );
}

#[test]
fn word_offsets_and_long_ptrs_coexist_per_decl() {
    // Two dispatch tables over the SAME two procs, one `word_offsets` and one
    // `long_ptrs`, in one module. Proves the encoding switch is per-decl and the
    // two encodings coexist. Layout (origin 0), tables then procs:
    //   W (word_offsets, 4 bytes) @ 0:  dc.w init−W, wait−W
    //   L (long_ptrs, 8 bytes)    @ 4:  dc.l init, wait
    //   init @ 12 (0x0C): 4E 75
    //   wait @ 14 (0x0E): 4E 75
    // W words: init−W = 12−0 = 0x0C, wait−W = 14−0 = 0x0E.
    // L longs: init = 0x0000000C, wait = 0x0000000E.
    let src = "\
module m
dispatch W (encoding: word_offsets) { Init: init, Wait: wait }
dispatch L (encoding: long_ptrs) { Init: init, Wait: wait }
proc init() { rts }
proc wait() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x0C, 0x00, 0x0E, // W: word offsets
            0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x0E, // L: long ptrs
            0x4E, 0x75, 0x4E, 0x75, // init, wait
        ]
    );
}

#[test]
fn long_ptrs_in_z80_section_is_non_68k() {
    // A `long_ptrs` dispatch in a `cpu: z80` section is rejected with the
    // generalized (encoding-neutral) `[dispatch.non-68k]` message — dispatch is
    // 68k-only for BOTH encodings in v1.
    let src = "\
module m
section s (cpu: z80, vma: $8000) {
    dispatch R (encoding: long_ptrs) { A: a, B: b }
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

// ---- 9. inline member bodies (Plan 7 #9a — D9.1) --------------------------

#[test]
fn inline_body_member_parses_and_lowers_clean() {
    // 9a resolves the seam reserved since #6: `Member: { … }` is sugar for an
    // anonymous per-member proc. Mixing body and label members is legal.
    let src = "\
module m
dispatch Routines (encoding: word_offsets) {
    Init: { rts },
    Wait: wait,
}
proc wait() { rts }
";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let errs: Vec<_> = diags.into_iter().map(|d| d.message).collect();
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
}

// ---- 9a T2: inline bodies lower as anonymous procs after the table -------

#[test]
fn inline_body_lowers_after_table_byte_exact() {
    // R9a.1: bodies lower immediately after the table, in member order. The
    // Init row points at the anonymous proc (+4); Wait at the named proc (+6):
    //   table:       00 04  00 06
    //   Init's body: 4E 75          (rts)
    //   wait:        4E 75          (rts)
    let src = "\
module m
dispatch Routines (encoding: word_offsets) {
    Init: { rts },
    Wait: wait,
}
proc wait() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x04, 0x00, 0x06, 0x4E, 0x75, 0x4E, 0x75]);
}

#[test]
fn inline_body_long_ptrs_byte_exact() {
    // Same shape under long_ptrs: 2 rows × 4 bytes, then the bodies.
    //   table: 00 00 00 08  00 00 00 0A
    //   A:     4E 75   b: 4E 75
    let src = "\
module m
dispatch R (encoding: long_ptrs) {
    A: { rts },
    B: b,
}
proc b() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0A, 0x4E, 0x75, 0x4E, 0x75]
    );
}

#[test]
fn inline_body_without_terminator_warns_fallthrough() {
    // R9a.4: a body that can reach its `}` without an unconditional terminator
    // warns [dispatch.body-fallthrough] (mirror of [proc.undeclared-fallthrough]).
    let src = "\
module m
dispatch R (encoding: word_offsets) {
    A: { nop },
}
";
    let msgs = msgs(src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("[dispatch.body-fallthrough]")).count(),
        1,
        "msgs: {msgs:?}"
    );
}

#[test]
fn empty_inline_body_emits_row_and_warns() {
    // An empty body is legal (its label sits at whatever follows) but cannot
    // terminate, so the fallthrough warning fires; the table row still emits.
    let src = "\
module m
dispatch R (encoding: word_offsets) {
    A: { },
}
";
    let (module, diags) = lower(src);
    assert!(diags.iter().any(|m| m.contains("[dispatch.body-fallthrough]")), "diags: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02]);
}

#[test]
fn section_nested_inline_body_lowers() {
    let src = "\
module m
section code (vma: $100) {
    dispatch R (encoding: word_offsets) {
        A: { rts },
    }
}
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02, 0x4E, 0x75]);
}
