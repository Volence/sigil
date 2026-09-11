//! An `extern(name)` that names a symbol no module in the link defines is refused
//! by name, at the `extern()` call's own span, whatever its value feeds: a guard at
//! item position, a guard in a 68000 or Z80 proc body (the resident sound driver's
//! `ensure(cycles(..) >= X)` shape), a guard's message, or a comptime result
//! nothing reads. A name the link does define still resolves in every one of those
//! positions, and a guard over a defined name is decided rather than dropped.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{AssertKind, Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

const UNKNOWN_ID: &str = sigil_link::EXTERN_UNKNOWN_ID;

fn lower(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "lower errors: {errs:?}");
    m
}

/// The module's link asserts, decided against its own sections exactly as every
/// link composer decides them.
fn link_verdict(src: &str) -> Vec<Diagnostic> {
    let m = lower(src);
    let resolved = sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &m.link_asserts)
}

fn errors(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.level == Level::Error).collect()
}

/// Byte offset of the `n`th (0-based) occurrence of `needle` in `src`.
fn offset_of(src: &str, needle: &str, n: usize) -> u32 {
    src.match_indices(needle).nth(n).unwrap_or_else(|| panic!("`{needle}` #{n} not in source")).0 as u32
}

/// Exactly one refusal, naming `name`, at the byte offset where `extern("name")`
/// is written, and no second diagnostic for the guard that consumed the value.
fn assert_refused_once_at(diags: &[Diagnostic], src: &str, name: &str, at: u32) {
    let errs = errors(diags);
    assert_eq!(errs.len(), 1, "one refusal and nothing else expected: {diags:?}");
    let d = errs[0];
    assert!(d.message.starts_with(UNKNOWN_ID), "not the extern refusal: {}", d.message);
    assert!(d.message.contains(&format!("`{name}`")), "the refusal must name `{name}`: {}", d.message);
    assert_eq!(
        d.primary.start, at,
        "the refusal must point at the extern() call, got {:?} = {:?}",
        d.primary,
        &src[d.primary.start as usize..(d.primary.end as usize).min(src.len())]
    );
}

// ---- the unknown name is refused, in every position and module kind ----------

/// A body-position guard in a 68000 proc: the guard defers to link, and the
/// proc-body lowering used to drop the deferred assert, so the build was green.
#[test]
fn an_unknown_extern_in_a_68k_proc_body_guard_is_refused_at_the_reference() {
    let src = "module m\nsection s (cpu: m68000, vma: $8000) {\n  proc P () {\n    nop\n    \
               ensure(extern(\"NO_SUCH\") == 1, \"body guard\")\n    rts\n  }\n}\n";
    assert_refused_once_at(&link_verdict(src), src, "NO_SUCH", offset_of(src, "extern(", 0));
}

/// The reported shape: a Z80 const bound to `extern()` and read by a body-position
/// cycle guard. The refusal lands on the const's `extern()`, once.
#[test]
fn an_unknown_extern_const_read_by_a_z80_cycle_guard_is_refused_at_the_const() {
    let src = "module m (cpu: z80)\nconst FLOOR = extern(\"NO_SUCH_SYMBOL_LENS_PIN\")\n\
               section s (cpu: z80, vma: $0) {\n  proc P () {\n    .a:\n    nop\n    .b:\n    nop\n    \
               ensure(cycles(.a, .b) >= FLOOR, \"cycle guard a\")\n    \
               ensure(cycles(.a, .b) >= FLOOR, \"cycle guard b\")\n    ret\n  }\n}\n";
    assert_refused_once_at(
        &link_verdict(src),
        src,
        "NO_SUCH_SYMBOL_LENS_PIN",
        offset_of(src, "extern(", 0),
    );
}

/// An item-position guard was already decided at link; the refusal now names the
/// reference instead of the guard, in both module kinds.
#[test]
fn an_unknown_extern_in_an_item_guard_is_refused_at_the_reference() {
    for (cpu, header) in [("m68000", "module m\n"), ("z80", "module m (cpu: z80)\n")] {
        let src = format!(
            "{header}section s (cpu: {cpu}, vma: $0) {{\n  data L: u8 = 0\n}}\n\
             ensure(extern(\"NO_SUCH\") == 1, \"item guard\")\n"
        );
        assert_refused_once_at(&link_verdict(&src), &src, "NO_SUCH", offset_of(&src, "extern(", 0));
    }
}

/// Not only inside a guard: a comptime fn evaluates `extern()` and discards it,
/// called from a data initializer. Nothing consumes the value, and the name is
/// still refused.
#[test]
fn an_unknown_extern_whose_value_nothing_reads_is_refused() {
    let src = "module m\ncomptime fn f() -> int {\n  let x = extern(\"NO_SUCH\")\n  return 1\n}\n\
               section s (cpu: m68000, vma: $8000) {\n  data D: u8 = f()\n}\n";
    assert_refused_once_at(&link_verdict(src), src, "NO_SUCH", offset_of(src, "extern(", 0));
}

/// Not only in a guard's condition: a passing deferred guard's message is frozen at
/// defer time, and a placeholder naming an unknown symbol is refused. A placeholder
/// has no file span of its own, so the refusal points at the guard.
#[test]
fn an_unknown_extern_in_a_guard_message_is_refused_at_the_guard() {
    let src = "module m\nsection s (cpu: m68000, vma: $8000) {\n  data L: u8 = 0\n}\n\
               ensure(extern(\"L\") == extern(\"L\"), \"never shown {extern(\\\"NO_SUCH\\\")}\")\n";
    assert_refused_once_at(&link_verdict(src), src, "NO_SUCH", offset_of(src, "ensure(", 0));
}

// ---- a defined name still resolves, and its guard is decided -----------------

/// A defined label, read by guards at item position and in 68000 and Z80 proc
/// bodies: no diagnostic, and every guard and reference reached the link.
#[test]
fn a_defined_extern_resolves_in_every_position_and_module_kind() {
    let srcs = [
        "module m\nsection s (cpu: m68000, vma: $8000) {\n  data L: u16 = 0\n  proc P () {\n    nop\n    \
         ensure(extern(\"L\") == $8000, \"68k body\")\n    rts\n  }\n}\n\
         ensure(extern(\"L\") == $8000, \"68k item\")\n",
        "module m (cpu: z80)\nconst FLOOR = extern(\"T_FLOOR\")\nsection s (cpu: z80, vma: $0) {\n  \
         equ T_FLOOR = 4\n  proc P () {\n    .a:\n    nop\n    .b:\n    nop\n    \
         ensure(cycles(.a, .b) >= FLOOR, \"z80 body\")\n    ret\n  }\n}\n\
         ensure(extern(\"T_FLOOR\") == 4, \"z80 item\")\n",
    ];
    for src in srcs {
        let m = lower(src);
        let conditions = m.link_asserts.iter().filter(|a| a.kind == AssertKind::Condition).count();
        let references = m.link_asserts.iter().filter(|a| a.kind == AssertKind::ExternDefined).count();
        assert!(conditions >= 2, "both guards must reach the link: {:?}", m.link_asserts);
        assert!(references >= 1, "the extern() reference must reach the link: {:?}", m.link_asserts);
        let diags = link_verdict(src);
        assert!(errors(&diags).is_empty(), "a defined name must resolve: {diags:?}");
    }
}

/// Defined means defined, not nonzero: an equ of `0` passes the reference check.
#[test]
fn a_defined_extern_whose_value_is_zero_resolves() {
    let src = "module m\nsection s (cpu: m68000, vma: $8000) {\n  equ ZERO = 0\n  data L: u8 = 0\n}\n\
               ensure(extern(\"ZERO\") + 1 == 1, \"zero\")\n";
    let diags = link_verdict(src);
    assert!(errors(&diags).is_empty(), "an equ of 0 is defined: {diags:?}");
}

/// A body guard over a defined name is DECIDED at link: a false one fails the build
/// with its own message, where the proc-body lowering used to drop it.
#[test]
fn a_false_body_guard_over_a_defined_extern_fires() {
    for (cpu, header, ret) in [("m68000", "module m\n", "rts"), ("z80", "module m (cpu: z80)\n", "ret")] {
        let src = format!(
            "{header}section s (cpu: {cpu}, vma: $0) {{\n  equ SEVEN = 7\n  proc P () {{\n    nop\n    \
             ensure(extern(\"SEVEN\") == 8, \"body guard fired\")\n    {ret}\n  }}\n}}\n"
        );
        let diags = link_verdict(&src);
        let errs = errors(&diags);
        assert_eq!(errs.len(), 1, "{cpu}: the false body guard must fail: {diags:?}");
        assert_eq!(errs[0].message, "body guard fired", "{cpu}");
    }
}
