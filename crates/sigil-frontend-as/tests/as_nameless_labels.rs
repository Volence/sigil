//! AS's NAMELESS TEMPORARY LABELS: `+`, `++`, `-`, `/`.
//!
//! A source writes a bare `+`, `-` or `/` in column 1 and branches to it with a
//! bare `+`/`++`/`-` operand. The construct carries 5,003 of the Sonic 2
//! disassembly's statements; without it the assembler drew 4,987 diagnostics
//! there and reached none of the code behind them.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
//! `Macro Assembler 1.42 Beta [Bld 212]`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `-q -A -L -U`. The probe sources
//! and their full listings are in
//! `docs/superpowers/notes/2026-09-09-as-nameless-labels.md`; each test below
//! quotes the listing rows its expectation comes from.
//!
//! **Every value here was read out of a run that exited 0**, and for this
//! construct that is not pedantry but the trap the first draft fell into: asl
//! STOPS ITERATING ITS PASSES once a line errors, so a listing from an errored
//! run prints unconverged forward branches -- `60FE`, a branch to itself -- for
//! lines that are perfectly fine. A probe whose LAST line was refused reported
//! that all three of its forward references resolved to their own address, and
//! the rules drafted off it had the forward direction backwards. Deleting the
//! one bad line turned the same file into `exit 0` and three distinct correct
//! answers.
//!
//! # The rules
//!
//! | | |
//! |---|---|
//! | definition `+` × m | forward counter += m; define the slot it lands on |
//! | definition `-` | backward counter += 1; define the slot it lands on |
//! | definition `/` | both counters += 1; define both slots |
//! | reference `+` × k | forward slot `fwd + k` |
//! | reference `-` × k | backward slot `bwd - k + 1` |
//!
//! Column 1 only, and `--`/`//` are refused. See `src/nameless.rs`.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble one source and return the flat image, or the diagnostics.
fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            // LINK diagnostics are returned, not panicked on. A nameless
            // reference deeper than the definitions behind it is a real refusal
            // that happens HERE rather than in the front end -- the slot is a
            // perfectly well-formed symbol that nothing defined -- and a helper
            // that panicked on it would make "refused at link" indistinguishable
            // from "the harness broke".
            match sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()) {
                Ok(linked) => Ok(sigil_link::flatten(&linked, 0x00).unwrap()),
                Err(d) => Err(d.iter().map(|d| d.message.clone()).collect()),
            }
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

/// The bytes an `org $1000` probe emits, from `$1000` for `len`.
fn at_1000(body: &str, len: usize) -> Vec<u8> {
    match assemble(body) {
        Ok(img) => img[0x1000..0x1000 + len].to_vec(),
        Err(d) => panic!("expected bytes, got diagnostics: {d:?}"),
    }
}

fn diags(body: &str) -> Vec<String> {
    match assemble(body) {
        Ok(b) => panic!("expected a refusal, got {} bytes: {:02X?}", b.len(), &b[..b.len().min(32)]),
        Err(d) => d,
    }
}

const HEAD: &str = "\tcpu\t68000\n\torg\t$1000\n";

// ---------------------------------------------------------------------------
// ORDINALS
// ---------------------------------------------------------------------------

/// `+` is the next forward-capable definition, `++` the second, `+++` the
/// third; `-` is the nearest preceding, `--` the second. A `+` definition is
/// forward-only, a `-` definition backward-only, and a `/` counts for BOTH --
/// which is what the interleaving here measures: the `-` at `$100A` is
/// invisible to `++`, and the `/` at `$100C` is what `++` and `-` both land on.
///
/// asl probe `ord2.asm`, exit 0:
///
/// ```text
///       4/    1000 : 6006                bra.s  +      ; -> $1008, the `+`
///       5/    1002 : 6008                bra.s  ++     ; -> $100C, the `/`
///       6/    1004 : 6008                bra.s  +++    ; -> $100E, the `+`
///       8/    1008 :                     +
///      10/    100A :                     -
///      12/    100C :                     /
///      14/    100E :                     +
///      17/    1010 : 60FA                bra.s  -      ; -> $100C, the `/`
///      18/    1012 : 60F6                bra.s  --     ; -> $100A, the `-`
/// ```
#[test]
fn ordinals_count_forward_and_backward_and_slash_counts_for_both() {
    let src = format!(
        "{HEAD}\
         \tbra.s\t+\n\
         \tbra.s\t++\n\
         \tbra.s\t+++\n\
         \tnop\n\
         +\n\
         \tnop\n\
         -\n\
         \tnop\n\
         /\n\
         \tnop\n\
         +\n\
         \tnop\n\
         \tbra.s\t-\n\
         \tbra.s\t--\n"
    );
    assert_eq!(
        at_1000(&src, 0x14),
        vec![
            0x60, 0x06, 0x60, 0x08, 0x60, 0x08, 0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71,
            0x4E, 0x71, 0x60, 0xFA, 0x60, 0xF6,
        ]
    );
}

/// A column-1 `++` consumes TWO forward slots rather than aliasing `+`.
///
/// This is the case that tells the two candidate models apart, and it is the
/// reason the definition side carries a count at all: under "a `++` definition
/// is another spelling of `+`" the `++` here would be slot 2 and `bra.s +++`
/// would be undefined. asl resolves `+++` to it, so it is slot 3 -- and slot 2
/// is then never defined by anything, which is exactly why a `bra.s ++` in this
/// same file is `error: symbol undefined`.
///
/// asl probe `q8.asm`, exit 0:
///
/// ```text
///       4/    1000 : 6004                bra.s  +      ; -> $1006, slot 1
///       5/    1002 : 6004                bra.s  +++    ; -> $1008, slot 3
///       7/    1006 :                     +
///       9/    1008 :                     ++
/// ```
#[test]
fn a_multi_plus_definition_consumes_that_many_slots() {
    let src = format!(
        "{HEAD}\tbra.s\t+\n\tbra.s\t+++\n\tnop\n+\n\tnop\n++\n\tnop\n"
    );
    assert_eq!(
        at_1000(&src, 0x0A),
        vec![0x60, 0x04, 0x60, 0x04, 0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71]
    );
}

/// The same-line case, BOTH directions, and the asymmetry is the counter rather
/// than a rule of its own: a column-1 definition has already advanced its
/// counter by the time the rest of that line dispatches, so a backward `-`
/// (slot `bwd`) lands on the label the line just defined and a forward `+`
/// (slot `fwd + 1`) lands one past it.
///
/// The backward half is 18 of the corpus's references -- `-\tdbf\td0,-`, one
/// line carrying both halves of the construct -- and before this feature those
/// 18 emitted no diagnostic at all, because the line died on its first token
/// and its operand was never reached.
///
/// asl probes `q3.asm` and `q4.asm`, both exit 0:
///
/// ```text
///       6/    1004 : 60FE                -  bra.s  -   ; -> $1004, its OWN label
///       4/    1000 : 6004                +  bra.s  +   ; -> $1006, the NEXT one
/// ```
#[test]
fn a_same_line_definition_is_visible_backward_and_not_forward() {
    let backward = format!("{HEAD}-\tnop\n\tnop\n-\tbra.s\t-\n");
    assert_eq!(
        at_1000(&backward, 6),
        vec![0x4E, 0x71, 0x4E, 0x71, 0x60, 0xFE]
    );

    let forward = format!("{HEAD}+\tbra.s\t+\n\tnop\n\tnop\n+\n\tnop\n");
    assert_eq!(
        at_1000(&forward, 8),
        vec![0x60, 0x04, 0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71]
    );
}

// ---------------------------------------------------------------------------
// THE COLUMN RULE, AND THE SPELLINGS asl REFUSES
// ---------------------------------------------------------------------------

/// An INDENTED `+` is not a label. asl answers `error: unknown instruction`
/// and does not define anything, so the forward reference above it has no
/// target at all:
///
/// ```text
///       4/    1000 : 60FE                bra.s  +
///       6/    1004 :                       +          ; indented
///       > > > q1.asm(6):2: error: unknown instruction
/// ```
///
/// Guarded in both directions in one test, because a rule that only ever fires
/// is indistinguishable from a rule that always fires.
#[test]
fn a_nameless_definition_must_sit_in_column_one() {
    let indented = format!("{HEAD}\tbra.s\t+\n\tnop\n\t+\n\tnop\n");
    let d = diags(&indented);
    assert!(
        !d.is_empty(),
        "an indented `+` must not define a label; asl calls it an unknown instruction"
    );

    let col1 = format!("{HEAD}\tbra.s\t+\n\tnop\n+\n\tnop\n");
    assert_eq!(at_1000(&col1, 4), vec![0x60, 0x02, 0x4E, 0x71]);
}

/// The multi-character definition is a `+` privilege, not a general one:
/// `--` and `//` in column 1 are both `error: invalid symbol name` to asl
/// (probes `q5.asm` line 6 and `q6.asm` line 6), while `++` is accepted.
#[test]
fn double_minus_and_double_slash_are_not_definitions() {
    for spelling in ["--", "//"] {
        let src = format!("{HEAD}{spelling}\n\tnop\n");
        let d = diags(&src);
        assert!(
            d.iter().any(|m| m.contains("not nameless labels")),
            "`{spelling}` in column 1 must be refused as asl refuses it; got {d:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// THE DISAMBIGUATION: `+` AND `-` ARE ALSO OPERATORS
// ---------------------------------------------------------------------------

/// The whole reason this feature is a reading of the operand parser rather than
/// a table entry, in one probe: every `+`/`-` expression shape asl ACCEPTS,
/// assembled together so a change to any one of them is a changed byte.
///
/// The three rows that carry the rule are `-Base`, `--Base` and `+-Base`. AS
/// splits an expression at the RIGHTMOST operator of the loosest tier present,
/// so in a leading run the LAST `+`/`-` is the binary operator and the rest is
/// the left-hand side: one `-` before an operand has nothing to its left and is
/// unary negation, two `-` split into `(nameless) - operand`.
///
/// asl probe `arith.asm`, exit 0, with `-` = `$1000`, `Base` = `$1020` and the
/// forward `+` = `$1084` (three distinct values, so a wrong reading cannot
/// coincide with a right one):
///
/// ```text
///   dc.l Base     0000 1020      dc.l (+)         0000 1084
///   dc.l $1234    0000 1234      dc.l (-)         0000 1000
///   dc.l +        0000 1084      dc.l (+)-Base    0000 0064
///   dc.l -        0000 1000      dc.l Base-(+)    FFFF FF9C
///   dc.l +-Base   0000 0064      dc.l 1+(+)       0000 1085
///   dc.l -Base    FFFF EFE0      dc.l (+)+1       0000 1085
///   dc.l --Base   FFFF FFE0      dc.l + -Base     0000 0064
///   dc.l 1+2      0000 0003      dc.l -  Base     FFFF EFE0
///   dc.l 1-2      FFFF FFFF      dc.l -(SIZE*2)   FFFF FF80
///   dc.l -1       FFFF FFFF
///   dc.l --1      0000 0FFF
/// ```
///
/// The addressing modes at the end are the other half of the guard: `(a5)+`,
/// `-(a5)`, `#-1` and `-4(a5)` all spell a `+` or `-` and none of them is a
/// nameless label. They are consumed structurally, before the expression parser
/// sees them, and this test is what says so.
#[test]
fn every_accepted_plus_minus_expression_matches_the_reference() {
    let src = "\tcpu\t68000\n\torg\t$1000\n\
        -\n\
        \tds.b\t$20\n\
        Base\tlabel\t*\n\
        \tdc.l\tBase\n\
        \tdc.l\t$1234\n\
        \tdc.l\t+\n\
        \tdc.l\t-\n\
        \tdc.l\t+-Base\n\
        \tdc.l\t-Base\n\
        \tdc.l\t--Base\n\
        \tdc.l\t1+2\n\
        \tdc.l\t1-2\n\
        \tdc.l\t-1\n\
        \tdc.l\t--1\n\
        \tdc.l\t(+)\n\
        \tdc.l\t(-)\n\
        \tdc.l\t(+)-Base\n\
        \tdc.l\tBase-(+)\n\
        \tdc.l\t1+(+)\n\
        \tdc.l\t(+)+1\n\
        \tdc.l\t+ -Base\n\
        \tdc.l\t-  Base\n\
        \tdc.l\t-(SIZE*2)\n\
        \tmove.b\t(a5)+,d0\n\
        \tmove.b\t-(a5),d0\n\
        \tmove.w\t#-1,d0\n\
        \tmove.w\t#-Base,d0\n\
        \tmove.l\t-4(a5),d0\n\
        \tds.b\t$4\n\
        +\n\
        \tnop\n\
        SIZE\tequ\t$40\n";
    let got = at_1000(src, 0x86);
    let want = hex(concat!(
        // $1000: the backward slot, then the $20 gap to `Base`.
        "00000000000000000000000000000000",
        "00000000000000000000000000000000",
        // $1020: the expression block.
        "00001020", "00001234", "00001084", "00001000", "00000064", "ffffefe0",
        "ffffffe0", "00000003", "ffffffff", "ffffffff", "00000fff", "00001084",
        "00001000", "00000064", "ffffff9c", "00001085", "00001085", "00000064",
        "ffffefe0", "ffffff80",
        // the addressing modes.
        "101d", "1025", "303cffff", "303cefe0", "202dfffc",
        // the $4 gap, then the forward slot's `nop`.
        "00000000", "4e71",
    ));
    assert_eq!(got, want);
}

/// `1+-2` is the row that shows sigil and asl already disagree about ordinary
/// arithmetic, and that this parcel did NOT change that.
///
/// asl splits `1+-2` at its rightmost `-`, is left with `1+`, and refuses it
/// (`error: wrong number of operands`). sigil reads it as `1 + (-2)` and has
/// since before nameless labels existed: the run there is one `+` followed by
/// one `-` before an operand, so the `-` takes the unary arm untouched by this
/// feature. Asserted so the divergence is a recorded fact rather than a
/// surprise, and so that closing it later is a deliberate, byte-changing
/// decision instead of an accident.
#[test]
fn sigil_still_accepts_the_expression_asl_refuses_here() {
    let src = format!("{HEAD}\tdc.l\t1+-2\n");
    assert_eq!(at_1000(&src, 4), vec![0xFF, 0xFF, 0xFF, 0xFF]);
}

// ---------------------------------------------------------------------------
// THE SHAPES THE CORPUS ACTUALLY WRITES
// ---------------------------------------------------------------------------

/// Every nameless-label shape in the Sonic 2 corpus, one instance of each, from
/// `docs/superpowers/notes/2026-09-05-s2-top-blocks-decompose-probes/nameless_shapes.asm`.
///
/// The four values are deliberately distinct, because a probe whose answers all
/// coincide proves nothing: `66FA` is a backward branch to a `/`, `6702` a
/// forward branch resolved through a macro ARGUMENT, `0016` twice a `dc.w`
/// difference reached as `+` and as `(+)`, and `0002` a PC displacement.
///
/// asl, exit 0, no diagnostic:
///
/// ```text
///      18/    2006 : 66FA                bne.s   -
///      21/    200A : 6702                        beq.s   +        (macro arg)
///      25/    200E : 0016                        dc.w    +-Base
///      26/    2010 : 0016                        dc.w    (+)-Base
///      28/    2012 : 123B 0002           move.b  +(pc,d0.w),d1
/// ```
///
/// `(+)` matters more than its one occurrence suggests: it is why this could
/// not be a bare-token special case, and the measurement note records that a
/// regex over bare tokens missed it and it was nearly lost from the count.
#[test]
fn the_four_corpus_shapes_match_the_reference() {
    let src = "\tcpu\t68000\n\torg\t$2000\n\
        mybranch macro dest\n\
        \tbeq.s\tdest\n\
        \tendm\n\
        myentry macro ptr\n\
        \tdc.w\tptr-Base\n\
        \tendm\n\
        Base\tlabel\t*\n\
        \tmoveq\t#$27,d0\n\
        /\n\
        \tmoveq\t#$31,d1\n\
        \ttst.b\td0\n\
        \tbne.s\t-\n\
        \ttst.b\td0\n\
        \tmybranch +\n\
        \tmoveq\t#$32,d1\n\
        +\n\
        \tmyentry +\n\
        \tmyentry (+)\n\
        \tmove.b\t+(pc,d0.w),d1\n\
        +\n\
        \tdc.b\t$11,$22,$33,$44\n\
        \trts\n";
    let img = match assemble(src) {
        Ok(i) => i,
        Err(d) => panic!("expected bytes, got diagnostics: {d:?}"),
    };
    assert_eq!(
        img[0x2000..0x201C].to_vec(),
        hex("702772314a0066fa4a006702723200160016123b0002112233444e75")
    );
}

/// A nameless label may share a line with a BLOCK OPENER, and the corpus writes
/// two of them (`s2.asm:9132` and `:21429`).
///
/// Block structure is decided by the head alone, so a line whose head the
/// scanner cannot read never reaches its block driver: the `rept` body was not
/// repeated and the line reported `rept` as an unrecognized mnemonic. Binding
/// the label matters as much as routing the line -- without it the `dbf d0,-`
/// underneath counts one definition too few, which is a branch to the wrong
/// address and no diagnostic at all.
///
/// asl probe `q9.asm`, exit 0:
///
/// ```text
///       4/    1000 : 7001                moveq  #1,d0
///       5/    1002 :                     -   rept 3
///       6/    1002 : 4E71                nop         (x3, $1002/$1004/$1006)
///       8/    1008 : 51C8 FFF8           dbf    d0,-     ; -8 -> $1002
/// ```
#[test]
fn a_nameless_label_may_open_a_rept_block() {
    let src = format!("{HEAD}\tmoveq\t#1,d0\n-   rept 3\n\tnop\n    endm\n\tdbf\td0,-\n");
    assert_eq!(
        at_1000(&src, 12),
        hex("70014e714e714e7151c8fff8")
    );
}

/// A LONE nameless label absorbs the pad on the line below it, exactly as a
/// lone NAMED label does.
///
/// This was the one design call in the parcel made on instinct rather than on
/// evidence -- the nameless definition was wired into
/// `absorb_pad_into_lone_label` because that is what a named label does -- so it
/// was measured afterwards, with the named twin in the SAME file so the two
/// answers are comparable rather than two separate probes.
///
/// asl probe `q12.asm`, exit 0:
///
/// ```text
///       5/    1000 : 11                  dc.b  $11
///       6/    1001 :                     Lone
///       7/    1001 : 00                  <padding>
///       7/    1002 : 2233                dc.w  $2233
///       8/    1004 : 44                  dc.b  $44
///       9/    1005 :                     -
///      10/    1005 : 00                  <padding>
///      10/    1006 : 5566                dc.w  $5566
///      11/    1008 : 60F8                bra.s Lone   ; -8 -> $1002, PAST the pad
///      12/    100A : 60FA                bra.s -      ; -6 -> $1006, PAST the pad
/// ```
///
/// Both labels moved. A probe using only `align` would NOT have settled it:
/// there (probes `q10`/`q11`) neither label moves, so named and nameless agree
/// for a reason that has nothing to do with this rule.
#[test]
fn a_lone_nameless_label_absorbs_a_pad_like_a_named_one() {
    let src = format!(
        "{HEAD}\
         \tdc.b\t$11\n\
         Lone\n\
         \tdc.w\t$2233\n\
         \tdc.b\t$44\n\
         -\n\
         \tdc.w\t$5566\n\
         \tbra.s\tLone\n\
         \tbra.s\t-\n"
    );
    assert_eq!(at_1000(&src, 12), hex("110022334400556660f860fa"));
}

/// A nameless reference deeper than the definitions behind it is an ordinary
/// UNDEFINED SYMBOL, not a silent zero. asl says `error: symbol undefined`;
/// the point of the assertion is that something is said at all, because the
/// backward slot arithmetic (`bwd - k + 1`) saturates rather than wrapping and
/// a saturating index is exactly the shape that quietly resolves to the wrong
/// row instead of to nothing.
#[test]
fn a_backward_reference_past_the_definitions_is_refused() {
    let src = format!("{HEAD}-\n\tnop\n\tbra.s\t---\n");
    let d = diags(&src);
    assert!(
        d.iter().any(|m| m.contains("nameless-#0")),
        "a `---` with one backward definition behind it must be refused, naming the \
         construct; got {d:?}"
    );
}

/// Decode a hex string into bytes; a malformed literal in a test is a test bug
/// and panics rather than silently comparing something shorter.
fn hex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd-length hex literal in a test");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex literal"))
        .collect()
}
