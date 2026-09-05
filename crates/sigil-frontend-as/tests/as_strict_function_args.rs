//! AS `function` argument evaluation is STRICT: every actual argument is
//! calculated once at the call, whether or not the body mentions the parameter
//! it binds to.
//!
//! AS's manual states the rule in as many words (`doc_EN/as.tex`, `FUNCTION`):
//!
//! > When the function is called, all parameters are calculated once and are
//! > then inserted into the function's formula. […] The result's type may depend
//! > on the type of the input arguments as the arguments are textually inserted
//! > into the function's formula. For example […] may have an integer, a float,
//! > or even a string as result, depending on the argument's type!
//!
//! Three types are named — integer, float, string. A register is not one of
//! them, and an undefined symbol has no value to calculate.
//!
//! The reference build (md5 `61e672562465725a8c102288a7da9098`,
//! `s1disasm/build_tools/Linux-x86_64/asl`, flags `-xx -n -q -A -L -U -i .`)
//! refuses `#fi(zz)` with `error #1010: symbol undefined` and exit 2, where `fi`
//! is a function whose body ignores its parameter and `zz` is nowhere defined.
//! The measurement, its 33-shape population and the four asl digests are in
//! `docs/superpowers/notes/2026-09-05-asl-silent-decline-regime.md`.
//!
//! WHAT EACH TEST MUST FAIL ON. Under LAZY expansion — folding the body without
//! ever looking at an argument the body does not mention — the three refusal
//! tests below assemble clean and emit `30 3C 03 C7`, exit 0, no diagnostic. The
//! two acceptance tests are the other half of the bar: strictness that refuses
//! an argument which does calculate would be a regression, not a fix, and the
//! corpus (2056 `.asm` across s1disasm, s2disasm, skdisasm and aeon) is made
//! entirely of that shape.
//!
//! On the diagnostic for a register: asl's own message catalogue (`as.msg`,
//! beside the binary) carries `expected integer, floating point number or string
//! but got register`, and asl fires it (`#1145`) for `#1+a1`. A register in
//! value position is not a symbol anyone forgot to define, and a reader told
//! `unresolved symbol` goes looking for a missing definition.

use sigil_frontend_as::{assemble, Options};

/// The shared preamble: a function whose body USES its parameter, one that
/// IGNORES it, and one more to nest. `fi`'s body is the whole point — it is the
/// only shape under which an argument can go unevaluated.
const PRELUDE: &str = "\tcpu 68000\n\tphase 0\n\
                       fu\tfunction p,(p*7)+$100\n\
                       fi\tfunction p,$3C7\n\
                       gi\tfunction q,$100\n\
                       hu\tfunction p,gi(p)\n";

/// Assemble, expecting REFUSAL, and hand back the diagnostic messages.
fn refusal(src: &str) -> Vec<String> {
    let diags = assemble(src, &Options::default())
        .err()
        .unwrap_or_else(|| panic!("expected a refusal, the source assembled"));
    diags.into_iter().map(|d| d.message).collect()
}

/// Assemble, expecting SUCCESS, and hand back the linked bytes.
fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default())
        .unwrap_or_else(|d| panic!("expected an assembly, refused: {d:?}"));
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// `#fi(zz)` — `zz` is defined nowhere, and `fi`'s body never mentions the
/// parameter it binds to. asl refuses it, LOUDLY, `#1010 symbol undefined`.
///
/// MUST FAIL under lazy expansion: the body folds to `$3C7` without `zz` ever
/// reaching the evaluator, and the line assembles as `move.w #$3C7,d0`.
#[test]
fn an_undefined_symbol_is_refused_even_where_the_body_ignores_it() {
    let msgs = refusal(&format!("{PRELUDE}\tmove.w\t#fi(zz),d0\n"));
    assert!(
        msgs.iter().any(|m| m.contains("zz")),
        "the refusal must name the undefined symbol `zz` — got {msgs:?}"
    );
}

/// The same shape one call deeper: `hu` USES its parameter, but it spends it on
/// `gi`, which does not. A check that only looked at the outermost call would
/// pass `zz` straight through into a body that drops it.
///
/// MUST FAIL under lazy expansion: `hu(zz)` → `gi((zz))` → `$100`, clean.
#[test]
fn an_undefined_symbol_is_refused_through_a_nested_ignoring_call() {
    let msgs = refusal(&format!("{PRELUDE}\tmove.w\t#hu(zz),d0\n"));
    assert!(
        msgs.iter().any(|m| m.contains("zz")),
        "the refusal must name the undefined symbol `zz` — got {msgs:?}"
    );
}

/// `#fi(a1)` — an address register where AS requires an integer, float or
/// string. Both shipped asl behaviours here are defects (a stable carry-over
/// word on two builds, an uninitialized read on the other two, exit 0 on all
/// four), so this is not asl-matching: it is the refusal AS's manual, AS's
/// message catalogue and today's upstream all point at.
///
/// The message must say REGISTER. `unresolved symbol \`a1\`` is a true
/// statement about sigil's symbol table and a false one about the program.
///
/// MUST FAIL under lazy expansion: `a1` is never looked at, and the line
/// assembles as `move.w #$3C7,d0`.
#[test]
fn a_register_argument_is_refused_as_a_register_not_as_a_missing_symbol() {
    let msgs = refusal(&format!("{PRELUDE}\tmove.w\t#fi(a1),d0\n"));
    assert!(
        msgs.iter().any(|m| m.contains("register") && m.contains("a1")),
        "the refusal must name `a1` AS A REGISTER — got {msgs:?}"
    );
    assert!(
        !msgs.iter().any(|m| m.contains("unresolved symbol")),
        "`a1` is a register in value position, not a missing definition — got {msgs:?}"
    );
}

/// The other half of the bar. An argument that DOES calculate is untouched by
/// strictness — same bytes, no diagnostic — and this is the shape the entire
/// corpus is made of. `fi(5)` = `$3C7`, `fu(5)` = `(5*7)+$100` = `$123`.
#[test]
fn an_argument_that_calculates_is_unaffected_whether_or_not_the_body_uses_it() {
    assert_eq!(
        bytes(&format!("{PRELUDE}\tmove.w\t#fi(5),d0\n\tmove.w\t#fu(5),d0\n")),
        vec![0x30, 0x3C, 0x03, 0xC7, 0x30, 0x3C, 0x01, 0x23],
    );
}

/// A FORWARD reference is not an undefined symbol. AS resolves symbols over
/// several passes and a name defined later in the file is legal in an argument;
/// a strictness check that reported on the first pass would refuse most real
/// programs.
#[test]
fn a_forward_reference_in_an_ignored_argument_still_assembles() {
    assert_eq!(
        bytes(&format!("{PRELUDE}\tmove.w\t#fi(later),d0\nlater\t=\t7\n")),
        vec![0x30, 0x3C, 0x03, 0xC7],
    );
}
