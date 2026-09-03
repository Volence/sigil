//! `eval` is asl's processor-neutral spelling of `SET`, and the one the Sonic
//! disassemblies write — on a Z80 `set` is a real bit instruction, so a shared
//! sound include cannot use it. These are the cases the byte-golden corpus
//! (`snippets_golden.txt`, blocks `t6_eval_*`) cannot carry, because asl
//! REFUSES them and the generator only records successful assemblies.
//!
//! Every expectation below was taken from asl 1.42 Beta [Bld 212]
//! (`x86_64-unknown-linux`, md5 `61e672562465725a8c102288a7da9098` — the copy
//! Sonic 1's own build uses), invoked `-xx -n -q -A -L -U -E -i .`.

use sigil_frontend_as::{assemble, Options};

/// asl reads the colon-less `NAME eval VALUE` form only when NAME sits in the
/// LABEL field, i.e. at column 0:
///
/// ```text
///        3/       0 :                         i    eval 5
/// > > > d1.asm(3):2: error #1200: unknown instruction
/// ```
///
/// An INDENTED `i eval 5` is an instruction named `i`, not an assignment — so
/// sigil must refuse it too, and must not bind `i`.
///
/// The gate matters because of what the ungated form does to a line that is not
/// an assignment at all. `eval` is an ordinary symbol name to asl, so an
/// indented `dc.b eval&$FF` presents as head `dc.b` with `eval` in the second
/// token; read as the label-column form it assigns a symbol named `dc.b` and
/// emits NOTHING — no bytes, no diagnostic, exit 0.
///
/// Its mutation is the removal of the `name_in_label_field` guard in
/// `exec_one`, not the removal of `eval` support: a build with no `eval` at all
/// also refuses this line, for the unrelated reason that `i` is not a mnemonic.
#[test]
fn indented_colonless_eval_is_an_instruction_not_an_assignment() {
    let src = "        cpu 68000\n        phase 0\n        i\teval 5\n        dc.b 1\n";
    let err = assemble(src, &Options::default())
        .expect_err("an indented colon-less `i eval 5` must be refused, as asl refuses it");
    assert!(
        err.iter().any(|d| d.message.contains('i')),
        "expected a diagnostic naming the head `i`, got: {err:?}"
    );
}

/// The same rule for the `set` spelling, which shares the arm.
#[test]
fn indented_colonless_set_is_an_instruction_not_an_assignment() {
    let src = "        cpu 68000\n        phase 0\n        i\tset 5\n        dc.b 1\n";
    assemble(src, &Options::default())
        .expect_err("an indented colon-less `i set 5` must be refused, as asl refuses it");
}

/// The concrete line the column rule protects: `eval` used as an ordinary
/// symbol, referenced from a `dc.b` operand. asl emits the label's low byte
/// (`09 03 01`); sigil must emit bytes rather than silently assigning a symbol
/// named `dc.b`. The byte values themselves are pinned in the golden corpus as
/// `t6_eval_is_an_ordinary_symbol_name`; what is asserted here is that the line
/// PRODUCES OUTPUT at all, which is the shape the silent failure had.
#[test]
fn eval_in_operand_position_still_emits() {
    let src = "        cpu 68000\n        phase 0\n        dc.b $09\neval    dc.b $03\n        dc.b eval&$FF\n";
    let module = assemble(src, &Options::default()).expect("assemble");
    let linked =
        sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(
        sigil_link::flatten(&linked, 0x00),
        vec![0x09, 0x03, 0x01],
        "`dc.b eval&$FF` must emit the label's low byte, not be read as an assignment"
    );
}

/// asl's operand form takes two operands (a third names a SEGMENT:
/// `eval f,1,2` is `#1961: unknown segment` on `2`, not a parse failure).
/// sigil implements the two-operand form and refuses anything else LOUDLY —
/// the corpus writes 68 two-operand `eval` lines and no three-operand one.
#[test]
fn eval_operand_form_needs_exactly_a_name_and_a_value() {
    for src in [
        "        cpu 68000\n        phase 0\n        eval e\n",
        "        cpu 68000\n        phase 0\n        eval f,1,2\n",
    ] {
        let err = assemble(src, &Options::default())
            .expect_err("a non-two-operand `eval` must be refused, never accepted quietly");
        // The DIRECTIVE's own diagnostic, quoting the spelling the author wrote.
        // Matching merely on "`eval`" would also match the
        // ``eval` is not a recognized 68000 mnemonic`` a build with no `eval`
        // support at all emits, which is the state this gate exists to catch.
        assert!(
            err.iter()
                .any(|d| d.message.contains("`eval` directive expects")),
            "expected the `eval` directive's own arity diagnostic, got: {err:?}"
        );
    }
}

/// A user macro named `eval` beats the builtin, exactly as asl's
/// macro-beats-builtin rule requires, and `!eval` forces the builtin past it.
/// Byte values are pinned by probe: the macro path emits `$41`, the forced
/// path assigns and emits `$07`.
#[test]
fn a_user_macro_named_eval_beats_the_directive() {
    let src = "eval\tmacro x\n        dc.b x+$40\n        endm\n        cpu 68000\n        phase 0\n        eval 1\n";
    let module = assemble(src, &Options::default()).expect("assemble");
    let linked =
        sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(sigil_link::flatten(&linked, 0x00), vec![0x41]);

    let forced = "eval\tmacro x\n        dc.b x+$40\n        endm\n        cpu 68000\n        phase 0\n        !eval z,7\n        dc.b z\n";
    let module = assemble(forced, &Options::default()).expect("assemble");
    let linked =
        sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(sigil_link::flatten(&linked, 0x00), vec![0x07]);
}
