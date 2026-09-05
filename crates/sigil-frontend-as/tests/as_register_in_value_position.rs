//! A register written where a VALUE belongs gets ONE story, in those terms,
//! with a source location, wherever it was written.
//!
//! Before this the same mistake was told apart by at least six messages, three
//! of which reported it as a missing definition and two of which had no source
//! location at all:
//!
//! | written | said |
//! |---|---|
//! | `dc.l ig(a0)` (function ignores its parameter) | `` `a0` is a register: a function argument must be … `` |
//! | `dc.l us(a0)` (function uses it) | `unresolved long expression` |
//! | `dc.l a0+1` | `unresolved long expression` |
//! | `dc.l a0` | `unresolved symbol `a0` for fixup in section sec0 at offset 0` |
//! | `dc.w a0+1` | `unresolved target expression (dangling symbol(s) `a0`) for fixup …` |
//! | `move.w #a0,d0` | `unresolved symbol `a0` in operand` |
//! | `org a0` | `org needs a constant expression` |
//! | `ds.b a0` | `unresolved ds count` |
//!
//! The cause is one, not several: a register name is not in the symbol table,
//! so an expression holding one folds to Poison, and Poison is the SHAPE of a
//! forward reference. Every consumer of a Poison expression then did what a
//! forward reference deserves, which is to defer it or call it unresolved.
//!
//! THE POPULATION IS THE CONSUMING END. The property being fixed is produced at
//! one place (the expression) and consumed at fifteen, so a check at the
//! producer proves nothing about any of them. Every row below is a distinct
//! consumer of a Poison expression, and a new consumer added without the check
//! is a row this table does not yet have.
//!
//! WHAT EACH ROW MUST FAIL ON. Reverted to the pre-fix front end, `dc.l a0`,
//! `dc.b a0`, `dc.w a0`, `move.l #a0,d0` and `jsr a0` do not refuse AT ALL here
//! (`assemble_root_located` returns `Ok`, since the register leaves as a fixup
//! and only the LINKER refuses it, with a section and an offset and no line),
//! and the other rows refuse with a message that says "unresolved" and never
//! says "register". Asserting the front end refuses is therefore the same
//! assertion as "the answer carries a source location": a linker diagnostic
//! cannot reach this API.
//!
//! NOT asl fidelity, deliberately. Reference asl (md5
//! `61e672562465725a8c102288a7da9098`) exits 0 and SILENTLY EMITS NOTHING for
//! `dc.l a0`, `dc.l ig(a0)` and `dc.l us(a0)`: the listing shows no bytes and
//! the program counter does not advance, against a positive control
//! (`dc.l $12345678`) that shows `1234 5678` and advances it by 4. A `dc.l`
//! that emits zero bytes is the silent-wrong-answer class, which this project
//! does not adopt. The refusal stays; only the wording moved. asl DOES diagnose
//! the one row it can (`dc.l a0+1`, exit 2, `expected integer, floating point
//! number or string but got register`), and that wording is where the tail of
//! sigil's sentence comes from.

use sigil_frontend_as::{assemble_root_located, Options};

/// THE sentence, spelled out rather than imported, because the text is the
/// contract: a reworded message must red this file loudly rather than keep
/// matching on a fragment.
const A0: &str = "`a0` is a register, not a value: expected an integer, floating point number or string";
const A1: &str = "`a1` is a register, not a value: expected an integer, floating point number or string";
const SP: &str = "`sp` is a register, not a value: expected an integer, floating point number or string";
const D7: &str = "`d7` is a register, not a value: expected an integer, floating point number or string";
const UPPER_A0: &str = "`A0` is a register, not a value: expected an integer, floating point number or string";

/// Assemble `body` as a real file named `probe.asm`, and hand back every
/// diagnostic as `(file(line), message)` with the scratch directory trimmed off
/// the front so the expectation can be written down.
///
/// A real file rather than a string: `SourceMap::label` renders `file(line)`
/// only for a NAMED source, and the location half of this parcel is exactly
/// what an unnamed source cannot show. `<no source location>` stands in when a
/// diagnostic belongs to no source line at all, which is what a linker-stage
/// answer looks like and what every row here must not be.
fn refusal(body: &str) -> Vec<(String, String)> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    // A source that assembles CLEAN reports itself as one row rather than
    // panicking, so a table test enumerates every consumer that disagrees
    // instead of dying on the first. The sentinel matches no expectation, so
    // this cannot turn a miss into a pass. It is also the exact shape the
    // deferring consumers had before the fix: front end silent, linker refusing.
    let failure = match assemble_root_located(&path, &Options::default()) {
        Ok(_) => {
            return vec![(
                "<no refusal>".to_string(),
                "the front end assembled it clean and left it to the linker".to_string(),
            )]
        }
        Err(f) => f,
    };
    failure
        .diags
        .iter()
        .map(|d| {
            let label = match failure.sources.label(d.primary) {
                Some(full) => match full.rsplit_once('/') {
                    Some((_, base)) => base.to_string(),
                    None => full,
                },
                None => "<no source location>".to_string(),
            };
            (label, d.message.clone())
        })
        .collect()
}

/// Assemble `body` expecting SUCCESS, and hand back the linked bytes.
fn bytes(body: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    let module = match assemble_root_located(&path, &Options::default()) {
        Ok(m) => m,
        Err(f) => panic!(
            "expected an assembly, refused: {:?}",
            f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        ),
    };
    let linked =
        sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// One consumer of a Poison expression: the source that reaches it, the line
/// the register is written on, and the message that line must produce.
struct Row {
    what: &'static str,
    body: &'static str,
    line: u32,
    says: &'static str,
}

/// Every consumer of a Poison expression that can be handed a register.
///
/// Grouped by the path each one took before the fix: the DEFERRING consumers
/// (which shipped the register to the linker and so answered with no source
/// location at all), and the REPORTING ones (which answered on the right line
/// with the wrong story).
const ROWS: &[Row] = &[
    // Deferred to the linker: no file, no line.
    Row { what: "dc.b bare", body: "\tcpu 68000\n\tdc.b a0\n", line: 2, says: A0 },
    Row { what: "dc.b compound", body: "\tcpu 68000\n\tdc.b a0+1\n", line: 2, says: A0 },
    Row { what: "dc.w bare", body: "\tcpu 68000\n\tdc.w a0\n", line: 2, says: A0 },
    Row { what: "dc.w compound", body: "\tcpu 68000\n\tdc.w a0+1\n", line: 2, says: A0 },
    Row { what: "dc.l bare", body: "\tcpu 68000\n\tdc.l a0\n", line: 2, says: A0 },
    Row { what: "move.l #imm (imm32 deferral)", body: "\tcpu 68000\n\tmove.l\t#a0,d0\n", line: 2, says: A0 },
    Row { what: "jsr bare target", body: "\tcpu 68000\n\tjsr\ta0\n", line: 2, says: A0 },
    Row { what: "jmp compound target", body: "\tcpu 68000\n\tjmp\ta0+1\n", line: 2, says: A0 },
    Row { what: "equ then use", body: "\tcpu 68000\nX\tequ a0\n\tdc.l X\n", line: 2, says: A0 },
    // Reported on the line, with the wrong story.
    Row { what: "dc.l compound", body: "\tcpu 68000\n\tdc.l a0+1\n", line: 2, says: A0 },
    Row { what: "move.w #imm", body: "\tcpu 68000\n\tmove.w\t#a0,d0\n", line: 2, says: A0 },
    Row { what: "moveq #imm", body: "\tcpu 68000\n\tmoveq\t#a0,d0\n", line: 2, says: A0 },
    Row { what: "absolute EA", body: "\tcpu 68000\n\tmove.w\ta0+1,d0\n", line: 2, says: A0 },
    Row { what: "org", body: "\tcpu 68000\n\torg a0\n", line: 2, says: A0 },
    Row { what: "ds count", body: "\tcpu 68000\n\tds.b a0\n", line: 2, says: A0 },
    Row { what: "align", body: "\tcpu 68000\n\talign a0\n\tdc.l 1\n", line: 2, says: A0 },
    Row { what: "rept count", body: "\tcpu 68000\n\trept a0\n\tdc.l 1\n\tendr\n", line: 2, says: A0 },
    Row { what: "while condition", body: "\tcpu 68000\n\twhile a0\n\tdc.l 1\n\tendw\n", line: 2, says: A0 },
    Row { what: "if condition", body: "\tcpu 68000\n\tif a0\n\tdc.l 1\n\tendc\n", line: 2, says: A0 },
    // Function arguments: the body that USES its parameter and the body that
    // IGNORES it took different paths, and told different stories.
    Row {
        what: "function argument, body uses it",
        body: "\tcpu 68000\nus\tfunction p,p+1\n\tdc.l us(a0)\n",
        line: 3,
        says: A0,
    },
    Row {
        what: "function argument, body ignores it",
        body: "\tcpu 68000\nig\tfunction p,$100\n\tdc.l ig(a0)\n",
        line: 3,
        says: A0,
    },
    // Line 3, not line 4, and that is measured rather than chosen. The argument
    // is dropped by the INNER call, whose tokens come from `hu`'s body on line
    // 3, so that is the span the substituted expression carries. The reader gets
    // the definition that discards the argument rather than the call that
    // supplied it. It is a real line in the file either way, so the location
    // half of this parcel holds; which of the two lines serves a reader better
    // is a separate question, booked in the parcel note.
    Row {
        what: "function argument through a nested ignoring call",
        body: "\tcpu 68000\ngi\tfunction q,$100\nhu\tfunction p,gi(p)\n\tdc.l hu(a1)\n",
        line: 3,
        says: A1,
    },
    // The whole register set is one story, not a0's story.
    Row { what: "stack pointer", body: "\tcpu 68000\n\tdc.l sp\n", line: 2, says: SP },
    Row { what: "data register", body: "\tcpu 68000\n\tdc.l d7\n", line: 2, says: D7 },
    Row {
        what: "uppercase, under case-sensitive -U",
        body: "\tcpu 68000\n\tdc.l A0\n",
        line: 2,
        says: UPPER_A0,
    },
];

/// EVERY consumer answers with the same sentence, on the line that wrote the
/// register, and says it exactly once.
///
/// "Exactly once" is half the parcel: `org a0` used to answer with the register
/// message AND `org needs a constant expression`, which is two stories for one
/// fault and the second of them phrased as if a definition were missing.
#[test]
fn every_consumer_tells_the_same_story_on_the_right_line() {
    let mut wrong = Vec::new();
    for row in ROWS {
        let got = refusal(row.body);
        let want = (format!("probe.asm({})", row.line), row.says.to_string());
        if got.len() != 1 || got[0] != want {
            wrong.push(format!("{}: wanted exactly [{want:?}], got {got:?}", row.what));
        }
    }
    assert!(wrong.is_empty(), "{} of {} consumers disagree:\n{}", wrong.len(), ROWS.len(), wrong.join("\n"));
}

/// No consumer may reach for the vocabulary of a missing definition. `a0` is
/// not a symbol anyone forgot to define, and a reader told "unresolved" or
/// "undefined" goes hunting for one.
#[test]
fn no_consumer_calls_a_register_a_missing_definition() {
    let mut wrong = Vec::new();
    for row in ROWS {
        for (label, msg) in refusal(row.body) {
            let lower = msg.to_ascii_lowercase();
            if lower.contains("unresolved") || lower.contains("undefined") {
                wrong.push(format!("{}: {label}: {msg}", row.what));
            }
        }
    }
    assert!(wrong.is_empty(), "{} consumers still say it is a missing definition:\n{}", wrong.len(), wrong.join("\n"));
}

/// No consumer may answer without a source location. This is the assertion
/// `dc.l a0` failed hardest: it left the front end as a fixup and the LINKER
/// refused it, with a section and an offset and nothing a reader can open.
#[test]
fn no_consumer_answers_without_a_file_and_a_line() {
    let mut wrong = Vec::new();
    for row in ROWS {
        for (label, msg) in refusal(row.body) {
            if !label.starts_with("probe.asm(") {
                wrong.push(format!("{}: {label}: {msg}", row.what));
            }
        }
    }
    assert!(wrong.is_empty(), "{} answers carry no source location:\n{}", wrong.len(), wrong.join("\n"));
}

/// An expression holding BOTH a register and a genuinely undefined symbol still
/// names the register, and names it as a register.
#[test]
fn a_register_beside_an_undefined_symbol_is_still_reported_as_a_register() {
    let got = refusal("\tcpu 68000\n\tmove.w\t#a0+zz,d0\n");
    assert!(
        got.iter().any(|(l, m)| l == "probe.asm(2)" && m == A0),
        "the register must be named as a register, got {got:?}"
    );
}

/// Two registers in one expression are two names, not one.
#[test]
fn two_registers_in_one_expression_are_both_named() {
    let got = refusal("\tcpu 68000\n\tdc.l a0+a1\n");
    assert_eq!(
        got,
        vec![
            ("probe.asm(2)".to_string(), A0.to_string()),
            ("probe.asm(2)".to_string(), A1.to_string()),
        ],
    );
}

/// THE CONTROL that keeps this from being a ban on the spelling. The check
/// fires only on a name that does NOT resolve, so a program which defines a
/// symbol called `a0` assembles exactly as it did.
#[test]
fn a_symbol_that_happens_to_be_spelled_like_a_register_still_assembles() {
    assert_eq!(bytes("\tcpu 68000\na0\tequ 5\n\tdc.l a0\n"), vec![0x00, 0x00, 0x00, 0x05]);
}

/// THE CONTROL for the register's own syntax. `(a0)` is a register-indirect
/// effective address, which is a register in a REGISTER position, and nothing
/// here may touch it.
#[test]
fn a_register_in_a_register_position_is_untouched() {
    assert_eq!(bytes("\tcpu 68000\n\tlea\t(a0),a1\n"), vec![0x43, 0xD0]);
    assert_eq!(bytes("\tcpu 68000\n\tmove.w\td0,d1\n"), vec![0x32, 0x00]);
}

/// THE CONTROL for a genuinely undefined symbol, which is the other half of
/// what Poison means. A forward or cross-seam reference must still leave the
/// front end as a deferred fixup, and a compound one must still get the
/// unresolved-expression wording. Narrowing this fix into a blanket refusal of
/// Poison would break every cross-seam `.emp` reference in the tree.
#[test]
fn a_genuinely_undefined_symbol_keeps_its_own_story() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, "\tcpu 68000\n\tdc.l zz\n").expect("write");
    assert!(
        assemble_root_located(&path, &Options::default()).is_ok(),
        "a bare undefined symbol must still defer to the linker, not refuse here"
    );

    let got = refusal("\tcpu 68000\n\tdc.l zz+1\n");
    assert_eq!(
        got,
        vec![("probe.asm(2)".to_string(), "unresolved long expression".to_string())],
    );
}

/// THE CONTROL for z80, which is not in scope and must not be dragged in. A z80
/// program is free to define a symbol called `sp`, and asl answers `#1010
/// symbol undefined` for `dw hl` rather than a register diagnostic, so the
/// deferral stands.
#[test]
fn z80_register_names_are_not_expression_level_registers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, "\tcpu z80\n\tdw hl\n").expect("write");
    assert!(
        assemble_root_located(&path, &Options::default()).is_ok(),
        "z80 `dw hl` must still leave the front end as a deferred fixup"
    );
}
