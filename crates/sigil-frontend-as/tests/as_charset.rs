//! `charset`: AS's code page, the character-to-byte step.
//!
//! A source that spells its text in the game's own font rather than in ASCII
//! says so with `charset`. Sonic 1's level select is written that way: `G` is
//! `$17`, not `$47`, across the 504 bytes between `LevelMenuText` and the reset
//! at `sonic.asm(2675)`.
//!
//! # Why most of this file is not about Sonic 1
//!
//! **Sonic 1 exercises ONE of the three consumers, so a two-thirds fix is
//! undetectable on it.** Its menu text is `dc.b "..."`, a string in a DATA
//! directive. It never writes a mapped string in an EXPRESSION, and its own
//! `charset` operands, though written as `'...'` character constants, never
//! index a character an earlier `charset` moved. An implementation that reaches
//! only `directive_db` emits Sonic 1's 504 bytes correctly, draws no
//! diagnostic, and is wrong. The census that scoped this work measured the
//! trap directly: 0 `dc.b` strings follow the bare `charset` reset in
//! `sonic.asm` and 45 exist elsewhere in the corpus.
//!
//! So the tests below deliberately exercise what the corpus cannot, and every
//! expected value is quoted from the reference listing rather than asserted.
//!
//! ## Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
//! `Macro Assembler 1.42 Beta [Bld 212]`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `-cpu 68000 -L -q -A`. Probe
//! sources and full listings: `docs/superpowers/notes/2026-09-09-as-charset.md`.
//!
//! **Every value here was read out of a run that exited 0.** That is not
//! pedantry: this build substitutes stable-but-invented answers for a shape it
//! declines, so a listing from a run carrying any error is not a source of
//! values even for the lines that assembled. Two probes in the note were
//! re-run for exactly this reason after an unrelated line on them drew an
//! error, and one (`p5`) had to be rewritten outright because its own construct
//! confounded it (see `save_and_restore_do_not_bracket_the_page`).
//!
//! ## The three consumers
//!
//! | # | site | shape | exercised by Sonic 1 |
//! |---|---|---|---|
//! | 1 | `eval.rs::directive_db` | `dc.b "AB"` | yes |
//! | 2 | `expr.rs::string_to_int` | `move.w #"AB",d0` | no |
//! | 3 | `lexer.rs` character constant | `dc.l 'INIT'` | no |
//!
//! asl has a fourth that sigil does not: its wide data directives distribute a
//! string operand and translate each character (`dc.w "AB"` under a live page is
//! `0011 0042`). sigil refuses that shape outright (`STRING_IN_WIDE_DATA`), so
//! it is not a consumer here; if it is ever implemented it is one on day one.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble_named(files: &[(&str, &str)], root: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        std::fs::write(dir.path().join(name), body).expect("write probe");
    }
    let path = dir.path().join(root);
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    assemble_named(&[("probe.asm", body)], "probe.asm")
}

fn bytes(body: &str) -> Vec<u8> {
    match assemble(body) {
        Ok(b) => b,
        Err(d) => panic!("expected bytes, got diagnostics: {d:?}"),
    }
}

fn diags(body: &str) -> Vec<String> {
    match assemble(body) {
        Ok(b) => panic!("expected a refusal, got bytes: {b:02X?}"),
        Err(d) => d,
    }
}

const HEAD: &str = "\tcpu 68000\n\torg 0\n";

// ---------------------------------------------------------------------------
// CONSUMER 1: a string in a DATA directive. The one Sonic 1 exercises.
// ---------------------------------------------------------------------------

/// Sonic 1's own preamble, `sonic.asm` 2616-2624 verbatim, then its own text
/// and its own reset at 2675.
///
/// Note the FORMS: five two-operand `charset SRC,TGT` lines and three
/// three-operand `charset LO,HI,BASE` lines, not eight of the latter.
///
/// asl, probe `s1.asm`, exit 0, `0 errors`:
///
/// ```text
///      12/       0 : 1722 1515 1EFF      	dc.b "GREEN HILL ZONE  STAGE 1"
///                6 : 1819 1C1C FF10
///                C : 1F1E 15FF FF23
///               12 : 2411 1715 FF01
///      13/      18 : FFFF FFFF FFFF      	dc.b "                 STAGE 2"
///               1E : FFFF FFFF FFFF
///               24 : FFFF FFFF FF23
///               2A : 2411 1715 FF02
///      14/      30 : 231F 251E 14FF      	dc.b "SOUND TEST"
///               36 : 2415 2324
///      15/      3A : 0A0D 0B0C           	dc.b "$>-="
///      17/      3E :                     	charset
///      18/      3E : 4752 4545 4E        	dc.b "GREEN"
/// ```
///
/// The last two lines are the RESET, and they are the half of this feature
/// Sonic 1 cannot check: `sonic.asm` has 0 `dc.b` strings after its own
/// `charset` reset, so an unimplemented reset is invisible there. `4752 4545
/// 4E` is plain ASCII `GREEN`, so the page really did go back.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn the_sonic_1_preamble_matches_the_reference_listing() {
    let src = format!(
        "{HEAD}\
\tcharset ' ', $FF\n\
\tcharset '0','9',$00\n\
\tcharset '$', $0A\n\
\tcharset '-', $0B\n\
\tcharset '=', $0C\n\
\tcharset '>', $0D\n\
\tcharset 'Y','Z',$0F\n\
\tcharset 'A','X',$11\n\
\tdc.b \"GREEN HILL ZONE  STAGE 1\"\n\
\tdc.b \"                 STAGE 2\"\n\
\tdc.b \"SOUND TEST\"\n\
\tdc.b \"$>-=\"\n\
\tcharset\n\
\tdc.b \"GREEN\"\n\
\tend\n"
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x17, 0x22, 0x15, 0x15, 0x1E, 0xFF, 0x18, 0x19, 0x1C, 0x1C, 0xFF, 0x10, 0x1F, 0x1E,
            0x15, 0xFF, 0xFF, 0x23, 0x24, 0x11, 0x17, 0x15, 0xFF, 0x01, //
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0x23, 0x24, 0x11, 0x17, 0x15, 0xFF, 0x02, //
            0x23, 0x1F, 0x25, 0x1E, 0x14, 0xFF, 0x24, 0x15, 0x23, 0x24, //
            0x0A, 0x0D, 0x0B, 0x0C, //
            0x47, 0x52, 0x45, 0x45, 0x4E,
        ]
    );
}

// ---------------------------------------------------------------------------
// CONSUMER 2: a string in an EXPRESSION. Sonic 1 never writes one under a
// live page, so nothing in the corpus can tell this apart from unimplemented.
// ---------------------------------------------------------------------------

/// The packing and the page COMPOSE: the page supplies the byte, the packing
/// stacks the bytes. asl, probe `t2.asm`, exit 0:
///
/// ```text
///       3/       0 : 4142                	dc.b "AB"
///       4/       2 : 303C 4142           	move.w #"AB",d0
///       5/       6 :                     	charset 'A','X',$11
///       6/       6 : 1112                	dc.b "AB"
///       7/       8 : 303C 1112           	move.w #"AB",d0
///       8/       C :                     	charset
///       9/       C : 4142                	dc.b "AB"
///      10/       E : 303C 4142           	move.w #"AB",d0
/// ```
///
/// The `303C 1112` is the whole point of this test. An implementation wired
/// only into `directive_db` writes `303C 4142` there, emits no diagnostic, and
/// passes every check Sonic 1 can offer.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_string_in_an_expression_packs_the_mapped_bytes() {
    let src = format!(
        "{HEAD}\
\tdc.b \"AB\"\n\
\tmove.w #\"AB\",d0\n\
\tcharset 'A','X',$11\n\
\tdc.b \"AB\"\n\
\tmove.w #\"AB\",d0\n\
\tcharset\n\
\tdc.b \"AB\"\n\
\tmove.w #\"AB\",d0\n\
\tend\n"
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x41, 0x42, //
            0x30, 0x3C, 0x41, 0x42, //
            0x11, 0x12, //
            0x30, 0x3C, 0x11, 0x12, // <== the mapped packing
            0x41, 0x42, //
            0x30, 0x3C, 0x41, 0x42,
        ]
    );
}

// ---------------------------------------------------------------------------
// CONSUMER 3: the `'...'` character constant. Missed by the seam note, and
// also invisible on Sonic 1.
// ---------------------------------------------------------------------------

/// A character constant is a packed INTEGER in every width, never a character
/// sequence, and its characters go through the page.
///
/// asl, probe `p15.asm`, exit 0, with `charset $49,$11` ('I' -> $11) live:
///
/// ```text
///       4/       0 : 114E 1154           	dc.l 'INIT'
///       5/       4 : 203C 114E 1154      	move.l #'INIT',d0
///       6/       A : 41                  	dc.b 'A'
///       7/       C : 114E                	dc.w 'IN'
/// ```
///
/// The mapping is written `$49,$11` rather than `'I',$11` on purpose: with a
/// character literal there the line would be self-referential (see
/// `a_charset_operand_reads_through_the_live_page`) and this probe would be
/// measuring two things at once.
///
/// The `dc.b 'A'` reading `41` is the control: `A` is not in the mapped set, so
/// a test that only checked the mapped characters could not tell a working page
/// from one that rewrote everything.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_character_constant_packs_the_mapped_bytes() {
    let src = format!(
        "{HEAD}\
\tcharset $49,$11\n\
\tdc.l 'INIT'\n\
\tmove.l #'INIT',d0\n\
\tdc.b 'A'\n\
\tdc.w 'IN'\n\
\tcharset\n\
\tend\n"
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x11, 0x4E, 0x11, 0x54, //
            0x20, 0x3C, 0x11, 0x4E, 0x11, 0x54, //
            0x41, 0x00, // dc.b 'A' then the auto-even pad
            0x11, 0x4E,
        ]
    );
}

// ---------------------------------------------------------------------------
// The forms, and the operand rule behind them.
// ---------------------------------------------------------------------------

/// A `charset` operand is an ordinary integer expression, so a character
/// literal written there is itself translated through the page that is live at
/// that moment. This is the rule a reader is most likely to guess wrong, and
/// getting it wrong is silent: it changes WHICH entry a second `charset` moves.
///
/// asl, probe `p2b.asm`, exit 0:
///
/// ```text
///       3/       0 :                     	charset 'A',$11
///       4/       0 : 11                  	dc.b 'A'
///       5/       1 :                     	charset 'A',$20
///       6/       1 : 11                  	dc.b 'A'
///       7/       2 : 11                  	dc.b $11
/// ```
///
/// and the CONTROL, probe `p3.asm`, exit 0, the same two lines with a raw index:
///
/// ```text
///       4/       0 :                     	charset 'A',$11
///       5/       0 :                     	charset $41,$20
///       6/       0 : 20                  	dc.b 'A'
/// ```
///
/// Without the control the first listing is equally well explained by "a second
/// `charset` on the same character is ignored", which is a different rule that
/// happens to produce the same byte.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_charset_operand_reads_through_the_live_page() {
    let selfref = format!(
        "{HEAD}\
\tcharset 'A',$11\n\
\tdc.b 'A'\n\
\tcharset 'A',$20\n\
\tdc.b 'A'\n\
\tdc.b $11\n\
\tend\n"
    );
    assert_eq!(bytes(&selfref), vec![0x11, 0x11, 0x11]);

    let raw = format!(
        "{HEAD}\
\tcharset 'A',$11\n\
\tcharset $41,$20\n\
\tdc.b 'A'\n\
\tend\n"
    );
    assert_eq!(bytes(&raw), vec![0x20]);
}

/// The two-operand form with a STRING target assigns consecutive entries, and
/// those characters are the one place a `charset` does NOT translate.
///
/// asl, probe `p2b.asm` line 12 and probe `p8.asm`, both exit 0:
///
/// ```text
///      12/       4 :                     	charset 'a',"xyz"
///      13/       4 : 7879 7A             	dc.b "abc"
///
///       5/       0 :                     	charset $7A,$05
///       6/       0 :                     	charset 'a',"z"
///       7/       0 : 7A                  	dc.b "a"
/// ```
///
/// The second listing is the discriminator: `z` is mapped to `$05` before the
/// `charset` that uses `"z"` as a target, and `a` still comes out `$7A`. A raw
/// target, not a translated one.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_string_target_assigns_consecutive_raw_bytes() {
    let consecutive = format!("{HEAD}\tcharset 'a',\"xyz\"\n\tdc.b \"abc\"\n\tend\n");
    assert_eq!(bytes(&consecutive), vec![0x78, 0x79, 0x7A]);

    let raw_target = format!(
        "{HEAD}\
\tcharset $7A,$05\n\
\tcharset 'a',\"z\"\n\
\tdc.b \"a\"\n\
\tend\n"
    );
    assert_eq!(bytes(&raw_target), vec![0x7A]);
}

/// Two `charset`s over DIFFERENT ranges accumulate rather than replace, and an
/// unmapped character passes through untouched.
///
/// asl, probe `p1.asm`, exit 0:
///
/// ```text
///       8/       8 :                     	charset 'A','X',$11
///      13/      10 :                     	charset '-', $0B
///      14/      10 : 0B11                	dc.b "-A"
///      16/      12 : 7A                  	dc.b "z"
/// ```
///
/// `0B11` is the accumulation: the second `charset` added `-` without dropping
/// `A`. `7A` is the passthrough.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_second_charset_accumulates_and_an_unmapped_character_passes_through() {
    let src = format!(
        "{HEAD}\
\tcharset 'A','X',$11\n\
\tcharset '-', $0B\n\
\tdc.b \"-A\"\n\
\tdc.b \"z\"\n\
\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x0B, 0x11, 0x7A]);
}

/// The three-operand form's TARGET wraps at $FF.
///
/// asl, probe `p7.asm`, exit 0:
///
/// ```text
///      11/       3 :                     	charset $41,$43,$FE
///      12/       3 : FEFF 00             	dc.b "ABC"
/// ```
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_range_target_wraps_at_the_top_of_the_page() {
    let src = format!("{HEAD}\tcharset $41,$43,$FE\n\tdc.b \"ABC\"\n\tend\n");
    assert_eq!(bytes(&src), vec![0xFE, 0xFF, 0x00]);
}

// ---------------------------------------------------------------------------
// Scope. Four questions with answers in the oracle, and guessing one is how a
// silently-wrong byte gets in.
// ---------------------------------------------------------------------------

/// The page reaches INTO an `include`, and a `charset` inside the include leaks
/// back OUT. It also survives a macro body the same way. Neither construct
/// scopes it.
///
/// asl, probe `p4.asm` (include) and `p3.asm` (macro), both exit 0:
///
/// ```text
///      10/       1 :                     	charset 'A',$11
///      11/       1 :                     	include "p4inc.asm"
/// (1)    1/       1 : 11                  	dc.b "A"
/// (1)    2/       2 :                     	charset 'B',$22
/// (1)    3/       2 : 22                  	dc.b "B"
///      12/       3 : 22                  	dc.b "B"
///
///      23/       6 : 41                  	dc.b "A"
///      24/       7 : (MACRO)              	mset
///      24/       7 :                             charset 'A',$11
///      24/       7 : 11                          dc.b "A"
///      25/       8 : 11                  	dc.b "A"
/// ```
///
/// The `22` on line 12 and the `11` on line 25 are the leaks.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn the_page_crosses_include_and_macro_boundaries_in_both_directions() {
    let inc = "\tdc.b \"A\"\n\tcharset 'B',$22\n\tdc.b \"B\"\n";
    let root = format!(
        "{HEAD}\
\tcharset 'A',$11\n\
\tinclude \"inc.asm\"\n\
\tdc.b \"B\"\n\
\tend\n"
    );
    assert_eq!(
        assemble_named(&[("root.asm", &root), ("inc.asm", inc)], "root.asm").expect("bytes"),
        vec![0x11, 0x22, 0x22]
    );

    let macro_src = format!(
        "{HEAD}\
mset\tmacro\n\
\tcharset 'A',$11\n\
\tdc.b \"A\"\n\
\tendm\n\
\tdc.b \"A\"\n\
\tmset\n\
\tdc.b \"A\"\n\
\tend\n"
    );
    assert_eq!(bytes(&macro_src), vec![0x41, 0x11, 0x11]);
}

/// `save`/`restore` do NOT bracket the page.
///
/// asl, probe `p7.asm`, exit 0:
///
/// ```text
///       3/       0 :                     	charset $41,$11
///       4/       0 : 11                  	dc.b "A"
///       5/       1 :                     	save
///       6/       1 :                     	charset $41,$44
///       7/       1 : 44                  	dc.b "A"
///       8/       2 : ALL                  	restore
///       9/       2 : 44                  	dc.b "A"
/// ```
///
/// The RAW `$41` index is load-bearing, and this probe is in the file twice for
/// that reason. Spelled `charset 'A',$44`, the inner line is inert -- by then
/// `'A'` evaluates to `$11`, so it remaps index `$11` -- and the first attempt
/// at this probe read `11 / 11 / 11` and was written down as "save/restore
/// brackets the page". It does not; the probe was measuring its own
/// self-reference.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn save_and_restore_do_not_bracket_the_page() {
    let src = format!(
        "{HEAD}\
\tcharset $41,$11\n\
\tdc.b \"A\"\n\
\tsave\n\
\tcharset $41,$44\n\
\tdc.b \"A\"\n\
\trestore\n\
\tdc.b \"A\"\n\
\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x11, 0x44, 0x44]);
}

/// The page IS reset to the identity at the start of every pass.
///
/// The probe has to be built so the two answers differ: the file must END with
/// a non-identity page live AND take more than one pass, so a page that carried
/// across would make the FIRST line read differently on the second pass. asl,
/// probe `t3.asm`, exit 0, `2 passes`:
///
/// ```text
///       3/       0 : 41                  	dc.b "A"
///       4/       1 :                     	charset 'A',$11
///       5/       1 : 11                  	dc.b "A"
///       6/       2 : 0002                	dc.w Later-*
///       7/       4 :                     Later:
/// ```
///
/// `41` on the last pass, with the page still dirty at the end of the previous
/// one and no `charset` reset anywhere in the file, is the finding. The listing
/// reports the LAST pass, so a page that survived would print `11` here.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn every_pass_starts_from_the_identity_page() {
    let src = format!(
        "{HEAD}\
\tdc.b \"A\"\n\
\tcharset 'A',$11\n\
\tdc.b \"A\"\n\
\tdc.w Later-*\n\
Later:\n\
\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x41, 0x11, 0x00, 0x02]);
}

// ---------------------------------------------------------------------------
// Refusals. asl applies NO part of a refused mapping; neither does this.
// ---------------------------------------------------------------------------

/// asl, probes `p8.asm` and `p10.asm`:
///
/// ```text
///      16/       2 :                     	charset 'A'
/// > > > p8.asm(16): error: wrong number of operands
///
///       4/       0 :                     	charset $61,$62,$63,$64
/// > > > p10.asm(4): error: wrong number of operands
/// ```
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn only_zero_two_or_three_operands() {
    assert_eq!(
        diags(&format!("{HEAD}\tcharset 'A'\n\tend\n")),
        vec!["charset takes 0, 2 or 3 operands, not 1"]
    );
    assert_eq!(
        diags(&format!("{HEAD}\tcharset $61,$62,$63,$64\n\tend\n")),
        vec!["charset takes 0, 2 or 3 operands, not 4"]
    );
}

/// An index or target outside `0..=255`, and a range that runs backwards.
///
/// asl, probes `p10.asm`, `p11.asm` and `p5.asm`:
///
/// ```text
/// > > > p10.asm(3): error: range overflow          charset $100,$11
/// > > > p6.asm(18): error: range overflow          charset $41,$1FF
/// > > > p11.asm(11): error: range overflow         charset $41,-1
/// > > > p5.asm(12): error: range underflow         charset 'C','A',$70
/// ```
///
/// Each of those runs then shows the mapping was NOT applied: `p5.asm` line 13
/// reads `dc.b "ABC"` as `4142 43`, plain ASCII.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn an_out_of_range_operand_is_refused_and_applies_nothing() {
    for (line, want) in [
        ("\tcharset $100,$11\n", "charset operand 256 out of range 0..=255"),
        ("\tcharset $41,$1FF\n", "charset operand 511 out of range 0..=255"),
        ("\tcharset $41,-1\n", "charset operand -1 out of range 0..=255"),
        ("\tcharset 'C','A',$70\n", "charset range $43..$41 runs backwards"),
    ] {
        let src = format!("{HEAD}{line}\tend\n");
        assert_eq!(diags(&src), vec![want.to_string()], "for `{}`", line.trim());
    }
}

/// An operand that never resolves is REFUSED, and a FORWARD-referenced one is
/// not. Two answers from one construct, and conflating them costs either a
/// silent wrong byte or a refused legal source.
///
/// asl, probes `p16.asm` (exit 2) and `p17.asm` (exit 0):
///
/// ```text
/// > > > p16.asm(4):10: error: symbol undefined
///       4/       0 :                     	charset NeverDefined,$11
///       5/       0 : 41                  	dc.b "A"
///
///       4/       0 :                     	charset Later,$11
///       5/       0 : 11                  	dc.b "A"
///       7/       1 : =$41                 Later	equ $41
/// ```
///
/// The `41` in the first is the reason this test exists rather than being left
/// to the general unresolved-symbol machinery. Before `charset_index` grew its
/// own `None` arm, sigil emitted that same `41` **at exit 0 with no
/// diagnostic** — plain ASCII where the source asked for the game's font, which
/// is a silent wrong byte and not a refusal. Neither the Sonic 1 census nor the
/// four-shape ROM gate can see one.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn an_unresolved_operand_is_refused_and_a_forward_reference_is_not() {
    assert_eq!(
        diags(&format!("{HEAD}\tcharset NeverDefined,$11\n\tdc.b \"A\"\n\tend\n")),
        vec!["unresolved charset operand"]
    );
    let forward = format!(
        "{HEAD}\
\tcharset Later,$11\n\
\tdc.b \"A\"\n\
Later:\tequ $41\n\
\tend\n"
    );
    assert_eq!(bytes(&forward), vec![0x11]);
}

/// A string target that would run past `$FF` is refused WHOLE: not even the
/// in-range prefix lands.
///
/// asl, probe `p13.asm`:
///
/// ```text
/// > > > p13.asm(4): error: range overflow
///       4/       0 :                     	charset $FE,"ABC"
///       5/       0 : FE                  	dc.b "\xfe"
/// ```
///
/// The `FE` on line 5 is what makes this a claim about ATOMICITY rather than
/// about the refusal: had asl applied the two in-range entries before giving
/// up, `$FE` would map to `A` and that byte would read `41`.
// REASON: the doc comment above quotes asl listings verbatim, and asl separates
// its listing columns with TABS. The tabs ARE the evidence: reflowing them to
// spaces would silently edit a reference assembler's output that later parcels
// compare against. Scoped to this item, never crate wide.
#[allow(clippy::tabs_in_doc_comments)]
#[test]
fn a_string_target_past_the_end_of_the_page_is_refused_whole() {
    let src = format!("{HEAD}\tcharset $FE,\"ABC\"\n\tend\n");
    assert_eq!(
        diags(&src),
        vec!["charset string target runs past the end of the code page"]
    );
}
