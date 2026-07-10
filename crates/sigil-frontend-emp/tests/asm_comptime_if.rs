//! Statement-position comptime `if` in proc/asm bodies (tranche 5, H1 —
//! mt_bank's define-conditional pattern extended to CODE, the game_loop
//! port's `ifdef SOUND_DRIVER_ENABLED` carrier).
//!
//! The grammar: `if cond { asm... } [else if ... | else { asm... }]` at the
//! same statement position as labels/instructions. The condition must
//! evaluate to a comptime bool or int (an int is truthy when nonzero, so a
//! bare `-D FLAG=1` define works unadorned); the CHOSEN branch's statements
//! lower inline against the enclosing body's label scope, and the unchosen
//! branch is never lowered. `if` can never shadow an instruction (S2-D1
//! reserves it; no 68k/Z80 mnemonic is named `if`) nor a future runtime
//! form (S2-D15's control-flow-sugar "no").
//!
//! Script bodies: a comptime `if` parses there too (a `yield` is a
//! `ScriptStmt`, so it can never nest inside one by construction), but a
//! LABEL defined under an `if` is refused — a resume/branch target must
//! exist unconditionally.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` for the 68k with the given `-D` defines, asserting a
/// clean parse. Returns the module and the lowering diagnostics.
fn lower_with(src: &str, defines: &[(&str, i128)]) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
        },
    )
}

/// Link a lowered `Module` to a flat image (the `lower_proc.rs` helper).
fn flatten(module: &Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

/// The game_loop shape: a define-gated instruction between unconditional
/// ones. ON: the branch's bytes appear in place; OFF: they vanish — no
/// padding, no trace.
#[test]
fn define_gates_instruction_bytes() {
    let src = "module m\n\
               proc f() clobbers(d0, d1) {\n\
               \tmoveq #0, d0\n\
               \tif FLAG == 1 {\n\
               \tmoveq #1, d1\n\
               \t}\n\
               \trts\n\
               }\n";
    let (module, diags) = lower_with(src, &[("FLAG", 1)]);
    assert!(diags.iter().all(|d| d.level != Level::Error), "ON diags: {diags:?}");
    assert_eq!(flatten(&module), vec![0x70, 0x00, 0x72, 0x01, 0x4E, 0x75]);

    let (module, diags) = lower_with(src, &[("FLAG", 0)]);
    assert!(diags.iter().all(|d| d.level != Level::Error), "OFF diags: {diags:?}");
    assert_eq!(flatten(&module), vec![0x70, 0x00, 0x4E, 0x75]);
}

/// A bare int condition is truthy when nonzero (`if FLAG { ... }` — the
/// unadorned define spelling).
#[test]
fn bare_int_condition_is_truthy_nonzero() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tif FLAG {\n\
               \tmoveq #0, d0\n\
               \t}\n\
               \trts\n\
               }\n";
    let (module, _) = lower_with(src, &[("FLAG", 2)]);
    assert_eq!(flatten(&module), vec![0x70, 0x00, 0x4E, 0x75]);
    let (module, _) = lower_with(src, &[("FLAG", 0)]);
    assert_eq!(flatten(&module), vec![0x4E, 0x75]);
}

/// `else` and `else if` chains choose exactly one branch; the `else` may sit
/// on the closing brace's line or the next line (both spell the same
/// statement).
#[test]
fn else_and_else_if_choose_one_branch() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tif MODE == 2 {\n\
               \tmoveq #2, d0\n\
               \t} else if MODE == 1 {\n\
               \tmoveq #1, d0\n\
               \t}\n\
               \telse {\n\
               \tmoveq #0, d0\n\
               \t}\n\
               \trts\n\
               }\n";
    for (mode, imm) in [(2i128, 0x02u8), (1, 0x01), (7, 0x00)] {
        let (module, diags) = lower_with(src, &[("MODE", mode)]);
        assert!(diags.iter().all(|d| d.level != Level::Error), "MODE={mode}: {diags:?}");
        assert_eq!(
            flatten(&module),
            vec![0x70, imm, 0x4E, 0x75],
            "MODE={mode} must select exactly one branch"
        );
    }
}

/// A label inside the CHOSEN branch is an ordinary label of the enclosing
/// body — referable from outside the `if`.
#[test]
fn label_in_chosen_branch_resolves() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tjbra .done\n\
               \tif FLAG == 1 {\n\
               \tmoveq #0, d0\n\
               \t.done:\n\
               \t}\n\
               \trts\n\
               }\n";
    let (module, diags) = lower_with(src, &[("FLAG", 1)]);
    assert!(diags.iter().all(|d| d.level != Level::Error), "diags: {diags:?}");
    // jbra .done relaxes to bra.s +2 (over the moveq): 60 02 70 00 4E 75.
    assert_eq!(flatten(&module), vec![0x60, 0x02, 0x70, 0x00, 0x4E, 0x75]);
}

/// A reference to a label whose defining branch was NOT chosen fails loudly
/// (the label resolves in scope but is never defined — a link-level error,
/// not a silent fall-through).
#[test]
fn label_in_unchosen_branch_is_loud() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tjbra .done\n\
               \tif FLAG == 1 {\n\
               \tmoveq #0, d0\n\
               \t.done:\n\
               \t}\n\
               \trts\n\
               }\n";
    let (module, diags) = lower_with(src, &[("FLAG", 0)]);
    let lower_err = diags.iter().any(|d| d.level == Level::Error);
    let resolve_err =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).is_err();
    assert!(
        lower_err || resolve_err,
        "a reference into an unchosen branch must fail loudly, got neither a \
         lowering error ({diags:?}) nor a resolve error"
    );
}

/// A non-comptime condition is the `[asm.if-not-comptime]` error.
#[test]
fn non_comptime_condition_errors() {
    let src = "module m\n\
               proc f() {\n\
               \tif \"strings are not conditions\" {\n\
               \trts\n\
               \t}\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower_with(src, &[]);
    assert!(
        has_tag(&diags, "[asm.if-not-comptime]"),
        "expected [asm.if-not-comptime], got: {diags:?}"
    );
}

/// Nested `if`s lower inside-out (the H2 mirror's nested-ifdef shape,
/// spelled either nested or `&&`-flattened).
#[test]
fn nested_ifs_compose() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tif A == 1 {\n\
               \tif B == 1 {\n\
               \tmoveq #3, d0\n\
               \t}\n\
               \t}\n\
               \trts\n\
               }\n";
    let (module, _) = lower_with(src, &[("A", 1), ("B", 1)]);
    assert_eq!(flatten(&module), vec![0x70, 0x03, 0x4E, 0x75]);
    let (module, _) = lower_with(src, &[("A", 1), ("B", 0)]);
    assert_eq!(flatten(&module), vec![0x4E, 0x75]);
    let (module, _) = lower_with(src, &[("A", 0), ("B", 1)]);
    assert_eq!(flatten(&module), vec![0x4E, 0x75]);
}

/// Script bodies: a label under a comptime `if` is refused (resume/branch
/// targets must exist unconditionally).
#[test]
fn script_label_under_if_refused() {
    let src = "module m\n\
               newtype ScriptPc = u16\n\
               struct S (size: $24) {\n\
               \t_pad0: [u8; $20],\n\
               \tresume: ScriptPc @ $20,\n\
               \t_pad1: [u8; 2] @ $22,\n\
               }\n\
               proc done() { rts }\n\
               script s (a0: *S) (encoding: word_offsets) shows done {\n\
               \tif FLAG == 1 {\n\
               \t.park:\n\
               \t}\n\
               \tyield\n\
               }\n";
    let (_module, diags) = lower_with(src, &[("FLAG", 1)]);
    assert!(
        diags.iter().any(|d| d.level == Level::Error
            && d.message.contains("inside a comptime `if`")),
        "expected the script label-under-if refusal, got: {diags:?}"
    );
}

/// Adversarial nesting is DIAGNOSED, not a stack overflow (the parser's
/// paren-bomb guard, shared with `stmt_block`/`loop`).
#[test]
fn deep_nesting_is_diagnosed_not_fatal() {
    let mut body = String::new();
    for _ in 0..200 {
        body.push_str("\tif FLAG == 1 {\n");
    }
    body.push_str("\tnop\n");
    for _ in 0..200 {
        body.push_str("\t}\n");
    }
    let src = format!("module m\nproc f() {{\n{body}\trts\n}}\n");
    let (_file, diags) = sigil_frontend_emp::parse_str(&src);
    assert!(
        diags.iter().any(|d| d.message.contains("block nesting too deep")),
        "expected the nesting-depth diagnostic, got: {diags:?}"
    );
}

/// The same label spelled `export .x:` in one arm and `.x:` in the other is
/// an error (the scope maps a name to ONE symbol — the flavors can't coexist).
#[test]
fn export_flag_disagreement_across_arms_errors() {
    let src = "module m\n\
               proc f() {\n\
               \tif FLAG == 1 {\n\
               \texport .done:\n\
               \t} else {\n\
               \t.done:\n\
               \t}\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower_with(src, &[("FLAG", 1)]);
    assert!(
        diags.iter().any(|d| d.level == Level::Error
            && d.message.contains("both `export` and non-`export`")),
        "expected the export-flavor disagreement error, got: {diags:?}"
    );
}

/// A statement on the same line after the closing brace is diagnosed, like
/// every other statement form's trailing junk.
#[test]
fn same_line_statement_after_brace_is_diagnosed() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tif FLAG == 1 { nop } moveq #0, d0\n\
               \trts\n\
               }\n";
    let (_file, diags) = sigil_frontend_emp::parse_str(src);
    assert!(
        diags.iter().any(|d| d.message.contains("expected end of line after `}`")),
        "expected the trailing-junk diagnostic, got: {diags:?}"
    );
}

/// `else` tolerates a newline on either side (`}\nelse\n{` parses).
#[test]
fn else_with_newlines_on_both_sides_parses() {
    let src = "module m\n\
               proc f() clobbers(d0) {\n\
               \tif FLAG == 1 {\n\
               \tmoveq #1, d0\n\
               \t}\n\
               \telse\n\
               \t{\n\
               \tmoveq #0, d0\n\
               \t}\n\
               \trts\n\
               }\n";
    let (module, diags) = lower_with(src, &[("FLAG", 0)]);
    assert!(diags.iter().all(|d| d.level != Level::Error), "diags: {diags:?}");
    assert_eq!(flatten(&module), vec![0x70, 0x00, 0x4E, 0x75]);
}
