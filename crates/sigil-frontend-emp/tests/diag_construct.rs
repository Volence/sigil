//! Statement-position `assert` / `raise_error` grammar (diagnostics construct,
//! spec `2026-07-11-emp-diagnostics-construct-design.md` §3, §5). Task 2 of the
//! build: AST variants + parsing ONLY — no desugar/lowering (that is Task 3).
//!
//! These tests exercise the parser directly (`parse_str` → `File` → proc body),
//! the same shape as `parser_bodies.rs`. They assert on the [`AsmStmt::Assert`]
//! / [`AsmStmt::RaiseError`] variants and on the parse-time steering
//! diagnostics named in spec §5 (missing width, unknown cond, the
//! `consoleprogram` two-argument form).

use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::parse_str;

/// Parse `src`, assert a CLEAN parse, return the file.
fn ok(src: &str) -> File {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "unexpected parse diagnostics: {diags:?}");
    file
}

/// Parse `src` expecting AT LEAST ONE diagnostic; return the concatenated
/// messages so a test can assert the fix is NAMED (spec §5).
fn err_msgs(src: &str) -> String {
    let (_file, diags) = parse_str(src);
    assert!(!diags.is_empty(), "expected a parse diagnostic, got none for: {src:?}");
    diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("\n")
}

fn first_proc(f: &File) -> &ProcDecl {
    for it in &f.items {
        if let Item::Proc(p) = it {
            return p;
        }
    }
    panic!("no proc in file");
}

/// Wrap a single asm statement in a minimal proc so it parses in body position.
fn body_stmt(stmt: &str) -> String {
    format!("module m\nproc p () {{\n\t{stmt}\n}}\n")
}

// ---- assert: cmp form, all three widths ------------------------------------

#[test]
fn assert_byte_cmp_form() {
    let f = ok(&body_stmt("assert.b d4, eq, #0"));
    let p = first_proc(&f);
    let AsmStmt::Assert { width, src, src_spelling, cond, dest, .. } = &p.body[0] else {
        panic!("expected Assert, got {:?}", p.body[0]);
    };
    assert_eq!(*width, Width::B);
    assert_eq!(src_spelling, "d4");
    assert!(matches!(&**src, Operand::Plain { .. }));
    assert_eq!(cond, "eq");
    let (_, dest_spelling) = dest.as_ref().expect("cmp form has a dest");
    assert_eq!(dest_spelling, "#0");
}

#[test]
fn assert_word_cmp_symbol_immediate() {
    let f = ok(&body_stmt("assert.w d1, lo, #MAX_LIST_ENTRIES"));
    let p = first_proc(&f);
    let AsmStmt::Assert { width, src_spelling, cond, dest, .. } = &p.body[0] else {
        panic!("expected Assert, got {:?}", p.body[0]);
    };
    assert_eq!(*width, Width::W);
    assert_eq!(src_spelling, "d1");
    assert_eq!(cond, "lo");
    // The symbol immediate survives verbatim — no `# Symbol`, no re-spelling.
    assert_eq!(dest.as_ref().unwrap().1, "#MAX_LIST_ENTRIES");
}

#[test]
fn assert_long_cmp_form() {
    let f = ok(&body_stmt("assert.l a0, ne, #0"));
    let p = first_proc(&f);
    let AsmStmt::Assert { width, src_spelling, cond, dest, .. } = &p.body[0] else {
        panic!("expected Assert, got {:?}", p.body[0]);
    };
    assert_eq!(*width, Width::L);
    assert_eq!(src_spelling, "a0");
    assert_eq!(cond, "ne");
    assert_eq!(dest.as_ref().unwrap().1, "#0");
}

// ---- assert: tst form (no dest) --------------------------------------------

#[test]
fn assert_tst_form_no_dest() {
    let f = ok(&body_stmt("assert.w d1, eq"));
    let p = first_proc(&f);
    let AsmStmt::Assert { width, src_spelling, cond, dest, .. } = &p.body[0] else {
        panic!("expected Assert, got {:?}", p.body[0]);
    };
    assert_eq!(*width, Width::W);
    assert_eq!(src_spelling, "d1");
    assert_eq!(cond, "eq");
    assert!(dest.is_none(), "tst form has no dest");
}

// ---- the #Object_RAM verbatim-spelling retrofit rule (spec §4.4) -----------

#[test]
fn assert_symbol_immediate_survives_verbatim() {
    // core.emp's Debug_AssertObjLoop shape: `#Object_RAM` must round-trip.
    let f = ok(&body_stmt("assert.l a0, hs, #Object_RAM"));
    let p = first_proc(&f);
    let AsmStmt::Assert { dest, .. } = &p.body[0] else { panic!() };
    assert_eq!(dest.as_ref().unwrap().1, "#Object_RAM");
}

// ---- raise_error -----------------------------------------------------------

#[test]
fn raise_error_captures_fstring() {
    let f = ok(&body_stmt("raise_error \"X%<endl>Got: %<.b d0>\""));
    let p = first_proc(&f);
    let AsmStmt::RaiseError { fstring, .. } = &p.body[0] else {
        panic!("expected RaiseError, got {:?}", p.body[0]);
    };
    assert_eq!(fstring, "X%<endl>Got: %<.b d0>");
}

// ---- negatives: the message NAMES THE FIX (spec §5) ------------------------

#[test]
fn missing_width_errors() {
    let msg = err_msgs(&body_stmt("assert d4, eq, #0"));
    assert!(
        msg.to_lowercase().contains("width") || msg.contains(".b"),
        "missing-width error must name the width: {msg}"
    );
}

#[test]
fn unknown_cond_lists_the_sixteen_codes() {
    let msg = err_msgs(&body_stmt("assert.b d4, xx, #0"));
    // The steering error enumerates every legal code.
    for code in ["eq", "ne", "cs", "cc", "pl", "mi", "hi", "hs", "ls", "lo", "gt", "ge", "le", "lt", "vs", "vc"] {
        assert!(msg.contains(code), "unknown-cond error must list `{code}`: {msg}");
    }
}

#[test]
fn raise_error_second_arg_is_a_steering_error() {
    // The `consoleprogram` two-argument form is out of scope (spec §5).
    let msg = err_msgs(&body_stmt("raise_error \"boom\", consoleprogram"));
    assert!(
        msg.contains("consoleprogram") || msg.to_lowercase().contains("one string"),
        "second-arg error must steer to the single-string form: {msg}"
    );
}
