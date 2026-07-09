//! `script name(params) (encoding: E) [shows label] { ScriptStmt* }` — the
//! ratified coroutine construct (Spec 2, Plan 7 #9b — D9.2/D9.6, rulings
//! R9b.1–R9b.12). A `script` desugars to a HIDDEN dispatch-encoded resume
//! table at its name plus ONE flattened proc-shaped body (`yield` saves a
//! typed resume point + exits via the per-frame epilogue; `loop {}` becomes a
//! hidden label + `jbra` back).
//!
//! Each case parses a full `.emp` file, lowers it via the same `lower_module`
//! entry the CLI uses, and asserts on the resulting diagnostics / linked bytes.

// The `lower`/`msgs`/`linked_bytes` helpers below mirror `tests/dispatch.rs`
// (lines 14-51): same lowering entry, same single-section link harness. Kept
// verbatim so the two suites stay in lockstep; Task 2's byte tests use them.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

/// Lower `src` (asserting a clean parse) and return `(module, diagnostic messages)`.
#[allow(dead_code)]
fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    (module, diags.into_iter().map(|d| d.message).collect())
}

#[allow(dead_code)]
fn msgs(src: &str) -> Vec<String> {
    lower(src).1
}

/// Link the lowered module and return the bytes of its (single) default section.
#[allow(dead_code)]
fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

// ---- 1. parsing (Plan 7 #9b — R9b.1) --------------------------------------

#[test]
fn script_decl_parses_with_loop_yield_and_shows() {
    let src = "\
module m
script brain (a0: *S) (encoding: word_offsets) shows Draw_Sprite {
    nop
    loop {
        .tick:
        subq.b  #1, d0
        yield
        yield shows .tick
    }
}
";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let Some(sigil_frontend_emp::ast::Item::Script(s)) = file.items.first() else {
        panic!("expected Item::Script, got {:?}", file.items.first())
    };
    assert_eq!(s.name, "brain");
    assert_eq!(s.params.len(), 1);
    assert!(matches!(s.encoding, sigil_frontend_emp::ast::DispatchEncoding::WordOffsets));
    let ep = s.epilogue.as_ref().expect("shows clause");
    assert_eq!((ep.name.as_str(), ep.local), ("Draw_Sprite", false));
    // body: nop, then a loop containing [.tick label, subq, bare yield,
    // yield shows .tick] — the D2.30(a) per-site epilogue spelling
    assert_eq!(s.body.len(), 2);
    let sigil_frontend_emp::ast::ScriptStmt::Loop { body, .. } = &s.body[1] else {
        panic!("expected loop, got {:?}", s.body[1])
    };
    assert_eq!(body.len(), 4);
    assert!(matches!(&body[2],
        sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: None, .. }));
    let sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: Some(l), .. } = &body[3] else {
        panic!("expected yield shows .tick, got {:?}", body[3])
    };
    assert_eq!((l.name.as_str(), l.local), ("tick", true));
}

#[test]
fn deep_loop_nesting_is_an_error_not_an_abort() {
    // Mirror of parser_bodies.rs::deep_block_nesting_is_an_error_not_an_abort:
    // `loop {` nested past MAX_EXPR_DEPTH must produce a diagnostic (and keep
    // parsing following items), not recurse until the process aborts.
    let opens = "loop {\n".repeat(600);
    let closes = "}\n".repeat(600);
    let src = format!(
        "module m\nscript s (a0: *S) (encoding: word_offsets) shows done {{\n\
         {opens}{closes}}}\nconst GOOD: u8 = 1\n"
    );
    let (f, diags) = parse_str(&src);
    assert!(!diags.is_empty());
    assert!(
        diags.iter().any(|d| d.message.contains("nesting too deep")),
        "expected a nesting-depth diagnostic, got: {diags:?}"
    );
    assert!(diags.len() < 50, "diagnostic flood: {}", diags.len());
    assert!(f
        .items
        .iter()
        .any(|i| matches!(i, sigil_frontend_emp::ast::Item::Const(c) if c.name == "GOOD")));
}

#[test]
fn yield_tolerates_same_line_close() {
    // Parity with instruction lines (`{ nop }` parses): a `}` may close the
    // body on the same line as a `yield`.
    let src = "\
module m
script s (a0: *S) (encoding: word_offsets) shows done { yield }
";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let Some(sigil_frontend_emp::ast::Item::Script(s)) = file.items.first() else {
        panic!("expected Item::Script, got {:?}", file.items.first())
    };
    assert_eq!(s.body.len(), 1);
    assert!(matches!(&s.body[0],
        sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: None, .. }));
}

#[test]
fn script_requires_encoding_attr() {
    let src = "\
module m
script s (a0: *S) {
    yield
}
";
    let (_, perrs) = parse_str(src);
    let msgs: Vec<_> = perrs.iter().map(|d| d.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("encoding")),
        "expected the dispatch-style required-encoding error, got: {msgs:?}"
    );
}

// ---- 2. lowering: hidden table + resume segments (R9b.2/R9b.5/R9b.6) ------

const SCRIPT_TYPES: &str = "\
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
";

#[test]
fn one_yield_word_offsets_byte_exact() {
    // Probe A (see plan): table [entry=+4, resume1=+14]; yield stores the
    // ×2 ordinal (#2) into resume ($20(a0)) then jbra's the epilogue.
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
    yield
    rts
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x0E, // table
            0x4E, 0x71, // nop
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20, // move.w #2,$20(a0)
            0x60, 0x02, // jbra done → bra.s +2
            0x4E, 0x75, // __resume$1: rts
            0x4E, 0x75, // done: rts
        ]
    );
}

#[test]
fn one_yield_long_ptrs_byte_exact() {
    // Probe B: 4-byte rows; the stored ordinal scales ×4 (#4).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: long_ptrs) shows done {{
    nop
    yield
    rts
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x12, // table
            0x4E, 0x71,
            0x31, 0x7C, 0x00, 0x04, 0x00, 0x20,
            0x60, 0x02,
            0x4E, 0x75,
            0x4E, 0x75,
        ]
    );
}

#[test]
fn loop_desugars_to_label_plus_jbra_back() {
    // Probe C: yield's resume point is the loop-bottom jbra, which jumps
    // back to the hidden loop label (bra.s −12).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    loop {{
        nop
        yield
    }}
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x0E,
            0x4E, 0x71,
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20,
            0x60, 0x02,
            0x60, 0xF4, // __resume$1: jbra .__loop$0 → bra.s −12
            0x4E, 0x75,
        ]
    );
}

// ---- 3. diagnostics (R9b.3/R9b.5/R9b.9) ------------------------------------

#[test]
fn bare_yield_without_epilogue_errors() {
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) {{
    yield
}}
"
    );
    let msgs = msgs(&src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("[script.no-epilogue]")).count(),
        1,
        "msgs: {msgs:?}"
    );
}

#[test]
fn script_without_scriptpc_field_errors() {
    let src = "\
module m
struct S (size: 2) { x: u16 }
script brain (a0: *S) (encoding: word_offsets) shows done {
    yield shows done
}
proc done () { rts }
";
    let msgs = msgs(src);
    assert!(
        msgs.iter().any(|m| m.contains("[script.no-resume-slot]")),
        "msgs: {msgs:?}"
    );
}

#[test]
fn script_fallthrough_warns() {
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
}}
proc done () {{ rts }}
"
    );
    let msgs = msgs(&src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("[script.fallthrough]")).count(),
        1,
        "msgs: {msgs:?}"
    );
}

// ---- 4. Task 3 coverage: overrides, hygiene, guards, edges ----------------
//
// These pin behavior the spec review already adversarially probed (see the
// plan's "Pre-derived byte references" + the nested-loop/override/zero-yield
// reference points quoted in the Task 3 section) — most are expected to be
// first-run green.

#[test]
fn yield_per_site_epilogue_overrides_shows() {
    // `shows done` + a per-site `yield shows other` override (D2.30(a) — the
    // retired bare-label spelling produced the SAME bytes; this test is the
    // equivalence proof): the jbra must target
    // `other`, not `done`. Layout is identical to Probe A (nop / yield / rts)
    // up through the resume label, but with TWO procs after the script body
    // (`done` first, then `other`) so the override is observable:
    //   table:  00 04  00 0E                     (entry=+4, resume1=+14)
    //   +4  nop:                4E 71
    //   +6  move.w #2,$20(a0):  31 7C 00 02 00 20  (ordinal 1*2)
    //   +12 jbra other:         60 ??              bra.s, PC+2=14
    //   +14 __resume$1: rts:    4E 75
    //   +16 done: rts:          4E 75
    //   +18 other: rts:         4E 75
    // jbra's target is `other` at +18: disp = 18 - (12+2) = 4 → 60 04.
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
    yield shows other
    rts
}}
proc done () {{ rts }}
proc other () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x0E, // table
            0x4E, 0x71, // nop
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20, // move.w #2,$20(a0)
            0x60, 0x04, // jbra other → bra.s +4 (targets `other` at +18, not `done` at +16)
            0x4E, 0x75, // __resume$1: rts
            0x4E, 0x75, // done: rts
            0x4E, 0x75, // other: rts
        ]
    );
}

#[test]
fn yield_local_epilogue_resolves() {
    // `yield shows .fin` — a dot-local per-site epilogue defined LATER in the SAME
    // script (at its end). Single-hygiene-scope flattening means the label is
    // visible to the yield above it, same as any proc-local forward reference.
    //
    // Byte-length derivation (first pass; corrected after a real run — see
    // note below): table(4) + nop(2) + store(6) + jbra(?) + resume(0) + rts(2).
    // The naive guess is jbra = 2 bytes (bra.s) for 16 total. The ACTUAL
    // linked length is 18: `__resume$1` and `.fin` are BOTH zero-width labels
    // sitting at the exact byte immediately after the jbra, so the jbra's
    // rung-0 (bra.s) displacement would be exactly 0 — the reserved 0x00
    // word-form escape byte, which is UNENCODABLE as a short branch (pinned
    // by `sigil-link`'s own `ladder_skips_bra_s_on_disp_zero` unit test).
    // Relaxation therefore skips straight to rung 1 (bra.w, 4 bytes), adding
    // 2 bytes over the naive guess: 16 + 2 = 18. This is a real, previously
    // documented linker behavior (not a script-lowering bug), so the
    // assertion below pins the corrected value with the arithmetic shown.
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) {{
    nop
    yield shows .fin
    .fin:
    rts
}}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module).len(), 18);
}

#[test]
fn nested_loops_get_distinct_labels() {
    // `loop { loop { nop / yield } }`: the inner and outer loops must mint
    // DISTINCT `__loop$d` labels (loop_count incremented before the nested
    // walk) so the two back-edge jbras land on different targets:
    //   table:  00 04  00 0E                      (entry=+4, resume1=+14)
    //   +4  __loop$0: / __loop$1: (coincide)  nop: 4E 71
    //   +6  move.w #2,$20(a0):                31 7C 00 02 00 20
    //   +12 jbra done:                        60 04   (bra.s, PC+2=14, target=18)
    //   +14 __resume$1: jbra .__loop$1:       60 F4   (bra.s, PC+2=16, target=4 → -12)
    //   +16 jbra .__loop$0:                   60 F2   (bra.s, PC+2=18, target=4 → -14)
    //   +18 done: rts:                        4E 75
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    loop {{
        loop {{
            nop
            yield
        }}
    }}
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x0E, // table
            0x4E, 0x71, // nop
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20, // move.w #2,$20(a0)
            0x60, 0x04, // jbra done → bra.s +4
            0x60, 0xF4, // __resume$1: jbra .__loop$1 → bra.s -12
            0x60, 0xF2, // jbra .__loop$0 → bra.s -14
            0x4E, 0x75, // done: rts
        ]
    );
}

#[test]
fn user_label_crosses_yield_boundary() {
    // THE load-bearing hygiene property (R9b's single-eval rationale): a user
    // label defined BEFORE a yield is referenced by a `jbra` AFTER it, in the
    // resume segment. Single flattened-body evaluation means this resolves
    // like any ordinary proc-local forward/backward reference — no separate
    // per-segment scope to trip over.
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    .tick:
    nop
    yield
    jbra .tick
}}
proc done () {{ rts }}
"
    );
    let (_module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
}

#[test]
fn zero_yield_script_emits_entry_only_table() {
    // No `yield` at all: the hidden table has exactly ONE row (member 0, the
    // entry segment) — `00 02` (word_offsets, 1 row, body starts at +2) —
    // followed by the body's single `rts` (4E 75).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) {{
    rts
}}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02, 0x4E, 0x75]);
}

#[test]
fn script_in_z80_section_errors() {
    // Mirror of `dispatch_in_z80_section_is_non_68k` (dispatch.rs): the guard
    // fires BEFORE resume-slot discovery / any body work, so the struct `S`
    // need not even be well-formed — the section's CPU alone is enough to
    // refuse. Must not panic.
    let src = "\
module m
section s (cpu: z80, vma: $8000) {
    script brain (a0: *S) (encoding: word_offsets) shows done {
        yield
    }
}
";
    let msgs = msgs(src);
    assert!(
        msgs.iter().any(|m| m.contains("[script.non-68k]")),
        "expected [script.non-68k]: {msgs:?}"
    );
}

#[test]
fn script_under_as_compat_silences_fallthrough() {
    // Mirror of `as_compat_silences_undeclared_fallthrough` (lower_proc.rs):
    // `@as_compat` right after `module m` silences the WARNING-level
    // modernization lint — the same body that warns `[script.fallthrough]`
    // in `script_fallthrough_warns` above stays quiet here.
    let src = format!(
        "module m\n@as_compat\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
}}
proc done () {{ rts }}
"
    );
    let msgs = msgs(&src);
    assert!(
        !msgs.iter().any(|m| m.contains("[script.fallthrough]")),
        "@as_compat must silence [script.fallthrough]: {msgs:?}"
    );
}

#[test]
fn ambiguous_resume_slot_errors() {
    // A struct with TWO `ScriptPc` fields makes the resume slot ambiguous —
    // `[script.ambiguous-resume-slot]`, not a panic or a silent pick.
    let src = "\
module m
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    resume2: ScriptPc @ $22,
}
script brain (a0: *S) (encoding: word_offsets) shows done {
    yield
}
proc done () { rts }
";
    let msgs = msgs(src);
    assert!(
        msgs.iter().any(|m| m.contains("[script.ambiguous-resume-slot]")),
        "expected [script.ambiguous-resume-slot]: {msgs:?}"
    );
}

#[test]
fn resume_width_errors() {
    // `newtype ScriptPc = u32` — a 4-byte field fails the "the slot is a word"
    // width check: `[script.resume-width]`.
    let src = "\
module m
newtype ScriptPc = u32
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
}
script brain (a0: *S) (encoding: word_offsets) shows done {
    yield
}
proc done () { rts }
";
    let msgs = msgs(src);
    assert!(
        msgs.iter().any(|m| m.contains("[script.resume-width]")),
        "expected [script.resume-width]: {msgs:?}"
    );
}

#[test]
fn comptime_call_inside_script_expands() {
    // A statement-position comptime call inside a script body goes through the
    // same `asm{}`-instantiation machinery as in a proc/dispatch body (mirror
    // of `inline_body_statement_comptime_call_expands`, dispatch.rs). No
    // `yield` at all, so the table is entry-only (`00 02`) and the body is the
    // comptime fn's expansion (`rts` = 4E 75).
    let src = "\
module m
comptime fn epi() -> Code {
    return asm {
        rts
    }
}
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
}
script brain (a0: *S) (encoding: word_offsets) {
    epi()
}
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02, 0x4E, 0x75]);
}

// ---- D2.30(a): the per-site epilogue is `yield shows <label>` -----------------

#[test]
fn retired_bare_label_yield_is_a_targeted_error() {
    // `yield <label>` was retired at the pre-freeze audit (it misread as a
    // resume target) — the error must TEACH both replacement spellings.
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    yield other
}}
proc done () {{ rts }}
proc other () {{ rts }}
"
    );
    let (_, perrs) = parse_str(&src);
    assert!(
        perrs.iter().any(|d| d.message.contains("yield shows") && d.message.contains("yield .")),
        "the retired form names both replacements: {perrs:?}"
    );
}

// ---- D2.30(b): `yield .label` — the named resume -------------------------------

#[test]
fn yield_dot_label_stores_target_ordinal() {
    // `yield .watch`: frame over, next frame continues at `.watch` — the
    // stored word is `.watch`'s member ordinal (×2), NOT a fresh site resume.
    //   table:  00 04  00 06            (entry=+4, watch=+6)
    //   +4  nop:                4E 71
    //   +6  .watch: nop:        4E 71
    //   +8  move.w #2,$20(a0):  31 7C 00 02 00 20   (ordinal 1×2)
    //   +14 jbra done:          60 00 00 02          (bra.s disp would be 0 —
    //                            the documented rung-skip → bra.w; done=+18)
    //   +18 done: rts:          4E 75
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
    .watch:
    nop
    yield .watch
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x06, // table: entry, .watch
            0x4E, 0x71, // nop
            0x4E, 0x71, // .watch: nop
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20, // move.w #2,$20(a0)
            0x60, 0x00, 0x00, 0x02, // jbra done (bra.w — disp-0 rung skip)
            0x4E, 0x75, // done: rts
        ]
    );
}

#[test]
fn repeated_yield_dot_same_label_shares_a_member() {
    // Two `yield .watch` join ONE table member ("becomes or joins") — the
    // table stays 2 rows (first word 00 04 = entry right after 4 table bytes).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    .watch:
    nop
    yield .watch
    yield .watch
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    let bytes = linked_bytes(&module);
    assert_eq!(&bytes[0..2], &[0x00, 0x04], "table is 4 bytes = 2 rows (entry + watch)");
}

#[test]
fn yield_dot_creates_no_site_resume_point() {
    // A bare yield mints its own member; a named yield does NOT — mixing one
    // of each with a shared target gives exactly 3 rows (entry, site, watch):
    // first table word = 6 (the body starts after 3 rows × 2 bytes).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    .watch:
    nop
    yield
    yield .watch
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    let bytes = linked_bytes(&module);
    assert_eq!(&bytes[0..2], &[0x00, 0x06], "3 rows: entry + the bare yield's site + watch");
}

#[test]
fn yield_dot_undefined_label_is_an_error() {
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    yield .nowhere
}}
proc done () {{ rts }}
"
    );
    let msgs = msgs(&src);
    assert!(
        msgs.iter().any(|m| m.contains("nowhere")),
        "the missing resume target is named: {msgs:?}"
    );
}

#[test]
fn yield_dot_scales_by_encoding() {
    // long_ptrs: the stored word is the ×4 ordinal (still one word — D9.3).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: long_ptrs) shows done {{
    .watch:
    nop
    yield .watch
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    let bytes = linked_bytes(&module);
    // long_ptrs table = 2 rows × 4 bytes = 8; store imm = 1×4 = #4.
    // store at +10: 31 7C 00 04 00 20.
    assert_eq!(&bytes[10..16], &[0x31, 0x7C, 0x00, 0x04, 0x00, 0x20]);
}

// ---- D2.30(c): `wait_frames #N, <slot>` — the declarative pure park ------------

const WAIT_TYPES: &str = "\
newtype ScriptPc = u16
struct S (size: $24) {
    timer: u8,
    _pad0: [u8; $1F] @ 1,
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
";

#[test]
fn wait_frames_expands_to_exactly_the_tick_idiom() {
    // The MUST of D2.30(c): `wait_frames` is a pure compiler expansion of the
    // documented tick idiom — the declarative line and the hand-written
    // spelling produce IDENTICAL bytes (same frame accounting: #64 parks 63
    // drawn frames and proceeds on the 64th tick).
    let sugar = format!(
        "module m\n{WAIT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    wait_frames #64, timer(a0)
}}
proc done () {{ rts }}
"
    );
    let hand = format!(
        "module m\n{WAIT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    move.b  #64, timer(a0)
    .tick:
    subq.b  #1, timer(a0)
    beq     .park_done
    yield   .tick
    .park_done:
}}
proc done () {{ rts }}
"
    );
    let (m1, e1) = lower(&sugar);
    assert!(e1.is_empty(), "sugar lowers clean: {e1:?}");
    let (m2, e2) = lower(&hand);
    assert!(e2.is_empty(), "hand spelling lowers clean: {e2:?}");
    assert_eq!(linked_bytes(&m1), linked_bytes(&m2), "byte-identical expansion");
}

#[test]
fn wait_frames_u16_slot_uses_word_width() {
    let types = "\
newtype ScriptPc = u16
struct S (size: $24) {
    timer16: u16,
    _pad0: [u8; $1E] @ 2,
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
";
    let sugar = format!(
        "module m\n{types}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    wait_frames #300, timer16(a0)
}}
proc done () {{ rts }}
"
    );
    let hand = format!(
        "module m\n{types}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    move.w  #300, timer16(a0)
    .tick:
    subq.w  #1, timer16(a0)
    beq     .park_done
    yield   .tick
    .park_done:
}}
proc done () {{ rts }}
"
    );
    let (m1, e1) = lower(&sugar);
    assert!(e1.is_empty(), "u16 slot lowers clean: {e1:?}");
    let (m2, e2) = lower(&hand);
    assert!(e2.is_empty(), "{e2:?}");
    assert_eq!(linked_bytes(&m1), linked_bytes(&m2));
}

#[test]
fn wait_frames_literal_zero_is_an_error() {
    // #0 would underflow-park ~2^width frames — refuse when comptime-visible.
    let src = format!(
        "module m\n{WAIT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    wait_frames #0, timer(a0)
}}
proc done () {{ rts }}
"
    );
    let msgs = msgs(&src);
    assert!(
        msgs.iter().any(|m| m.contains("wait_frames") && m.contains("0")),
        "the zero park is refused: {msgs:?}"
    );
}

#[test]
fn wait_frames_unknown_slot_field_is_an_error() {
    let src = format!(
        "module m\n{WAIT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    wait_frames #4, nosuch(a0)
}}
proc done () {{ rts }}
"
    );
    let msgs = msgs(&src);
    assert!(
        msgs.iter().any(|m| m.contains("nosuch")),
        "the missing slot field is named: {msgs:?}"
    );
}

#[test]
fn wait_frames_needs_an_epilogue() {
    let src = format!(
        "module m\n{WAIT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) {{
    wait_frames #4, timer(a0)
}}
"
    );
    let msgs = msgs(&src);
    assert!(
        msgs.iter().any(|m| m.contains("[script.no-epilogue]")),
        "a park draws every frame — it needs the epilogue: {msgs:?}"
    );
}

#[test]
fn wait_frames_outside_a_script_stays_loud() {
    // In a proc body `wait_frames` is not a statement (scripts only) — the
    // ordinary not-a-mnemonic path refuses it.
    let src = "\
module m
proc p () {
    wait_frames #4, d0
}
";
    let msgs = msgs(src);
    assert!(
        msgs.iter().any(|m| m.contains("wait_frames")),
        "loud outside scripts: {msgs:?}"
    );
}

#[test]
fn wait_frames_slot_can_live_in_an_overlay() {
    // The acceptance shape: the park timer is a `vars …: sst_custom` OVERLAY
    // field, not a direct Sst field — width resolution uses the same field
    // space ordinary operands do (D6.A3).
    // (An overlay needs its window field on the base struct — the prelude's
    // `Sst.sst_custom` shape, reproduced locally.)
    let src = "\
module m
newtype ScriptPc = u16
struct S (size: $24) {
    sst_custom: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
vars V: sst_custom {
    timer: u8,
}
script brain (a0: *S) (encoding: word_offsets) shows done {
    wait_frames #8, timer(a0)
}
proc done () { rts }
"
    .to_string();
    let (_, errs) = lower(&src);
    assert!(errs.is_empty(), "overlay slot resolves: {errs:?}");
}
