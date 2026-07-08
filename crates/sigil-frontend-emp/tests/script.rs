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
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
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
        yield .tick
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
    // body: nop, then a loop containing [.tick label, subq, bare yield, yield .tick]
    assert_eq!(s.body.len(), 2);
    let sigil_frontend_emp::ast::ScriptStmt::Loop { body, .. } = &s.body[1] else {
        panic!("expected loop, got {:?}", s.body[1])
    };
    assert_eq!(body.len(), 4);
    assert!(matches!(&body[2],
        sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: None, .. }));
    let sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: Some(l), .. } = &body[3] else {
        panic!("expected yield .tick, got {:?}", body[3])
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
    yield done
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
