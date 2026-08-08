//! Typed computed `jmp (aN) as Type` (bookmark ask 5,
//! `docs/superpowers/2026-08-06-bookmark-implementation-sketch.md` §6). The
//! `jsr (aN) as Type` dispatch bound already ships (game_loop.emp:42,
//! vblank.emp:45); this pins that the SAME `as ContractType` annotation on the
//! computed-TAIL spelling `jmp (aN)` carries the SAME semantics — it parses onto
//! the instruction, is byte-neutral, charges the bound's clobbers into the tail
//! caller's transitive-clobber closure, and (unbounded) makes the caller ⊤ —
//! and that it composes with `@resumable` (a resumable decoder's terminal
//! `jmp (a3) as Cont` continuation passes the stackless scan the untyped form
//! already passes).
//!
//! The dispatch-bound machinery is mnemonic-generic over CALL ∪ TAIL
//! (`parser::instr` attaches `as Type` after any instruction's operands;
//! `corpus_contracts::is_indirect_call` and `collect_indirect_sites` gate on
//! `CALL_MNEMONICS ∪ TAIL_MNEMONICS`; the closure charges every indirect site
//! uniformly), so the `jmp` spelling needs no bespoke lowering — these tests are
//! the executable proof of that parity, and the guard against a future change
//! that silently drops the tail half.

use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::corpus_contracts::analyze_corpus;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;

fn ok(src: &str) -> File {
    let (f, diags) = parse_str(src);
    assert!(diags.is_empty(), "unexpected parse diagnostics: {diags:?}");
    f
}

fn analyze(srcs: &[&str]) -> sigil_frontend_emp::corpus_contracts::ContractReport {
    let files: Vec<_> = srcs
        .iter()
        .map(|s| {
            let (f, diags) = parse_str(s);
            assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
            f
        })
        .collect();
    analyze_corpus(&files)
}

/// Link `src`'s single default section to a flat image.
fn flatten(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "lower errors: {diags:?}");
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

fn first_proc(f: &File) -> &ProcDecl {
    f.items.iter().find_map(|i| match i { Item::Proc(p) => Some(p), _ => None }).expect("a proc")
}

// ---- parse: the bound rides the `jmp` instruction --------------------------

#[test]
fn jmp_as_type_parses_the_dispatch_bound() {
    let f = ok("module engine.core\n\
                @noreturn\n\
                proc Tail () clobbers(d0-d7/a0-a6) {\n\
                    jmp (a1) as ObjRoutine\n\
                }\n");
    let p = first_proc(&f);
    let bounds: Vec<Option<String>> = p.body.iter().filter_map(|s| match s {
        AsmStmt::Instr(i) if i.mnemonic == vec![TextOrSplice::Text("jmp".into())] =>
            Some(i.dispatch_bound.clone()),
        _ => None,
    }).collect();
    assert_eq!(bounds, vec![Some("ObjRoutine".to_string())]);
}

// ---- byte-neutrality: the bound is pure metadata ---------------------------

#[test]
fn jmp_as_type_is_byte_neutral() {
    let plain = flatten("module m\n@noreturn\nproc P () clobbers(d0-d7/a0-a6) { jmp (a1) }\n");
    let bound = flatten("module m\n@noreturn\nproc P () clobbers(d0-d7/a0-a6) { jmp (a1) as ObjRoutine }\n");
    assert_eq!(bound, plain, "`as` dispatch bound on a jmp must not change emitted bytes");
}

// ---- closure parity: a bounded tail charges only the bound's clobbers -------

/// The `jsr` analog is `corpus_contracts::bounded_indirect_is_not_top`. A bounded
/// computed TAIL charges exactly the bound's write surface into the tail caller's
/// effective set — so a proc declaring precisely that set does not fire.
#[test]
fn bounded_jmp_charges_only_the_bound() {
    let r = analyze(&[
        "module m\n\
         type ObjRoutine = proc () clobbers(d0, d1, a0)\n\
         @noreturn\n\
         proc Dispatch () clobbers(d0, d1, a0) {\n jmp (a1) as ObjRoutine\n }\n",
    ]);
    assert!(r.firings.is_empty(), "bounded jmp dispatch should not fire: {:?}", r.firings);
    assert_eq!(r.contract_type_count, 1);
}

/// A tail caller declaring LESS than the bound fires transitively for the
/// leaked register — the bound really is charged (not ignored because it's a jmp).
#[test]
fn bounded_jmp_under_declaration_fires() {
    let r = analyze(&[
        "module m\n\
         type ObjRoutine = proc () clobbers(d0, d1, a0)\n\
         @noreturn\n\
         proc Dispatch () clobbers(d0) {\n jmp (a1) as ObjRoutine\n }\n",
    ]);
    assert!(
        r.firings.iter().any(|f| f.proc == "Dispatch"),
        "an under-declared bounded jmp must fire transitively: {:?}",
        r.firings
    );
}

/// An UNBOUNDED computed tail (`jmp (a1)` with no `as`) makes the caller ⊤ — the
/// same load-bearing fact as the `jsr` sibling.
#[test]
fn unbounded_jmp_is_top() {
    let r = analyze(&[
        "module m\n@noreturn\nproc Dispatch () clobbers(d0) {\n jmp (a1)\n }\n",
    ]);
    assert!(
        r.firings.iter().any(|f| f.proc == "Dispatch" && f.unbounded),
        "unbounded jmp must go ⊤: {:?}",
        r.firings
    );
}

// ---- compose with @resumable: the terminal typed continuation exit ----------

/// A `@resumable` decoder exits by a computed `jmp (a3)` continuation; the TYPED
/// spelling `jmp (a3) as Cont` must pass the stackless scan exactly as the untyped
/// form does (an `Ind(a3)` is not a stack operand; the `as Type` is metadata the
/// scan does not read).
#[test]
fn resumable_terminal_typed_jmp_passes_the_stackless_scan() {
    let src = "module m\n\
type Cont = proc () clobbers(d0-d2/a0-a3)\n\
@resumable\n\
pub proc ZX0R (a0: *u8, a1: *u8, a3: *u8) clobbers(d0-d2/a2) {\n\
    moveq #0, d0\n\
    move.b (a0)+, (a1)+\n\
    jmp (a3) as Cont\n\
}\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("[resumable.stack-op]")),
        "a typed continuation exit must pass the stackless scan: {diags:?}"
    );
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the whole resumable proc must lower clean: {diags:?}"
    );
}
