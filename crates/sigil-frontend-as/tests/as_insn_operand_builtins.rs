//! The front-end builtin layer over a 68000 INSTRUCTION operand.
//!
//! `dc.b`/`dc.w`/`dc.l` route their operands through `expand_operand_builtins`:
//! user `function` calls, then `int(...)`/`sin(...)`, then the string builtins,
//! then string comparisons, each collapsing to a plain token before the
//! expression parser runs. An instruction operand ran the FIRST of those four
//! and none of the rest, so `dc.l strlen("ab")+Foo` assembled and
//! `move.l strlen("ab")+Foo,d0` did not.
//!
//! That is not a missing feature so much as a trap, and it is the same trap the
//! `dc.w`/`dc.l` widths were once in: a builtin that works in one operand
//! position and not another. Sonic 2 lands on it 518 times from one line,
//! `s2.macrosetup.asm:304`:
//!
//! ```text
//! 	jmp	(extractJmpToName("op")).l
//! ```
//!
//! # Provenance of every expected value here
//!
//! Reference assembler `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `asl -q -A -L -U <file>` and
//! **checked for exit status 0** before any byte was read out of the listing. An
//! earlier run of the same probe exited 2 on one extra line (`jmp val("Foo")`,
//! which asl rejects as `addressing mode not allowed here`) and its listing
//! still showed a full byte column for the ten lines that had assembled. Those
//! bytes were discarded and the line removed rather than quoted, because a
//! listing from a failing run is not a source of values. Probes and their
//! verbatim listings are committed under
//! `docs/superpowers/notes/2026-09-05-as-jmptos-518-block-probes/`.
//!
//! # Why these particular values
//!
//! * `strlen` is taken over a twelve-character string, not a one- or
//!   four-character one. A length of 1 is indistinguishable from a "found"
//!   boolean and a length of 0 from a failure that folded to zero.
//! * `val` is asked for BOTH `"$4142"` and `"4142"`, which are `$4142` and
//!   `$102E`. `val` is an expression evaluator, not a decimal parser, and this
//!   pair is the only kind of fixture that can tell those apart: a
//!   single-digit probe spells the same characters in both radices, which is
//!   how a live hex-versus-decimal divergence survived months in this repo.
//! * The two `jmp` targets in the corpus fixture sit at `$1000` and `$1008`, so
//!   an implementation that resolved every call to one symbol fails. A
//!   one-target probe could not have detected that.

use sigil_frontend_as::{assemble, Options};

/// Every fixture sits at `org $1000`, because `0` is a value a broken fold can
/// produce by accident.
const ORG: usize = 0x1000;

fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default()).unwrap_or_else(|d| {
        panic!(
            "expected a successful assembly, got {:?}",
            d.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
    });
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    let image = sigil_link::flatten(&linked, 0x00);
    assert!(
        image.len() > ORG,
        "the fixture emitted nothing at $1000: {} bytes total",
        image.len()
    );
    image[ORG..].to_vec()
}

/// The corpus line, with its own `function` and two distinct targets.
///
/// asl (md5 above, exit 0), `.../2026-09-05-as-jmptos-518-block-probes/jmptos.lst`:
///
/// ```text
///       13/    1008 : 4EF9 0000 1000      	jmp	(extractJmpToName("JmpTo_Foo")).l
///       14/    100E : 4EF9 0000 1008      	jmp	(extractJmpToName("JmpTo_Bar")).l
/// ```
#[test]
fn the_sonic_2_jump_table_generator() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "extractJmpToName function name,",
        "val(substr(name, strstr(name, \"_\") + 1, strlen(name)))\n",
        "Foo:\n",
        "\tnop\n\tnop\n\tnop\n\tnop\n",
        "Bar:\n",
        "\tjmp\t(extractJmpToName(\"JmpTo_Foo\")).l\n",
        "\tjmp\t(extractJmpToName(\"JmpTo_Bar\")).l\n",
    );
    assert_eq!(
        bytes(src),
        vec![
            0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71, // Foo: four nops
            0x4E, 0xF9, 0x00, 0x00, 0x10, 0x00, // jmp (Foo).l
            0x4E, 0xF9, 0x00, 0x00, 0x10, 0x08, // jmp (Bar).l
        ]
    );
}

/// The whole builtin layer over the operand positions an instruction has: a
/// long-absolute address, an immediate, and a bare absolute.
///
/// asl (md5 above, exit 0),
/// `.../2026-09-05-as-jmptos-518-block-probes/insn_operands.lst`:
///
/// ```text
///        4/    1000 : 4EF9 0000 1000      	jmp	(Foo).l
///        5/    1006 : 4EF9 0000 100C      	jmp	(strlen("abcdefghijkl")+Foo).l
///        6/    100C : 4EF9 0000 1000      	jmp	(val("Foo")).l
///        7/    1012 : 203C 0000 000C      	move.l	#strlen("abcdefghijkl"),d0
///        8/    1018 : 203C 0000 4142      	move.l	#val("$4142"),d0
///        9/    101E : 203C 0000 102E      	move.l	#val("4142"),d0
///       10/    1024 : 203C 0000 0003      	move.l	#int(3.7),d0
///       11/    102A : 2038 100C           	move.l	strlen("abcdefghijkl")+Foo,d0
/// ```
///
/// The last line is `abs.w`, not `abs.l`: the width rule is unchanged by this
/// parcel and the fixture would catch a change to it.
#[test]
fn builtins_in_every_instruction_operand_position() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "Foo:\n",
        "\tjmp\t(Foo).l\n",
        "\tjmp\t(strlen(\"abcdefghijkl\")+Foo).l\n",
        "\tjmp\t(val(\"Foo\")).l\n",
        "\tmove.l\t#strlen(\"abcdefghijkl\"),d0\n",
        "\tmove.l\t#val(\"$4142\"),d0\n",
        "\tmove.l\t#val(\"4142\"),d0\n",
        "\tmove.l\t#int(3.7),d0\n",
        "\tmove.l\tstrlen(\"abcdefghijkl\")+Foo,d0\n",
    );
    assert_eq!(
        bytes(src),
        vec![
            0x4E, 0xF9, 0x00, 0x00, 0x10, 0x00, //
            0x4E, 0xF9, 0x00, 0x00, 0x10, 0x0C, //
            0x4E, 0xF9, 0x00, 0x00, 0x10, 0x00, //
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x0C, //
            0x20, 0x3C, 0x00, 0x00, 0x41, 0x42, //
            0x20, 0x3C, 0x00, 0x00, 0x10, 0x2E, //
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x03, //
            0x20, 0x38, 0x10, 0x0C, //
        ]
    );
}

/// The inertness half, and the one that carries the shipping-build argument.
///
/// A name that is BOTH a symbol and the head of a `disp(An)` addressing mode is
/// the shape the builtin layer could plausibly have broken, because `val` is a
/// builtin head and `val(a0)` is a displacement. It is not broken, and it is not
/// broken for a structural reason rather than a lucky one: the expansion runs
/// over the same slices `expand_calls` already ran over, and those hold back a
/// trailing EA base group, so the `(a0)` is never offered to any builtin and the
/// `val` that precedes it is a bare identifier with no `(` after it.
///
/// asl (md5 above, exit 0),
/// `.../2026-09-05-as-jmptos-518-block-probes/disp_head.lst`:
///
/// ```text
///        7/    1000 : 2028 0004           	move.l	val(a0),d0
///        8/    1004 : 2030 8804           	move.l	strlen(a0,a0.l),d0
/// ```
#[test]
fn a_builtin_name_used_as_a_displacement_symbol_is_untouched() {
    let src = concat!(
        "\tcpu\t68000\n",
        "\torg\t$1000\n",
        "val:\tequ\t4\n",
        "strlen:\tequ\t4\n",
        "\tmove.l\tval(a0),d0\n",
        "\tmove.l\tstrlen(a0,a0.l),d0\n",
    );
    assert_eq!(
        bytes(src),
        vec![
            0x20, 0x28, 0x00, 0x04, //
            0x20, 0x30, 0x88, 0x04, //
        ]
    );
}
