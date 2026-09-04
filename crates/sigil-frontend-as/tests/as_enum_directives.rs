//! AS's `enum` / `nextenum` / `enumconf` enumeration directives.
//!
//! WHY SEVERAL ITEMS BELOW CARRY `#[allow(clippy::tabs_in_doc_comments)]`. The
//! `text` blocks are asl listings pasted verbatim, and asl separates its columns
//! with TABS. Those tabs are the evidence: what these comments assert is what the
//! reference assembler PRINTED, so respacing them into four spaces would quietly
//! restate the claim about output asl never produced. The waiver is per-item and
//! deliberately NOT a file-scoped `#![allow]`: a new test here that grows a tab by
//! accident should still trip the lint and be looked at, which a file-wide allow
//! would silently absorb.
//!
//! The model is one running counter and one running step: a member assigns the
//! counter, then the counter advances by the step read AT THAT MOMENT. `enum`
//! resets the counter to 0 before binding its members; `nextenum` continues from
//! wherever the previous enumeration left it; `enumconf` sets the step and
//! touches nothing else.
//!
//! Every expectation below is read off the listing of `asl` 1.42 Beta Bld 212
//! — S1's own binary at `s1disasm/build_tools/Linux-x86_64/asl`, run with S1's
//! own flags, `-xx -n -q -A -L -U -i .` — for the identical source text. The
//! probes are committed under `docs/superpowers/notes/2026-09-04-as-enum-probes`
//! and `run.sh` there reruns any of them. The listing annotation `=$X..$Y` on an
//! `enum`/`nextenum` line is AS reporting the first and last value the line
//! bound, and it is quoted beside each expectation.
//!
//! These directives build the sound driver's pitch and note tables in
//! `sound/_smps2asm_inc.asm` (shared verbatim by S1 and S2) and the whole
//! object-RAM offset vocabulary in `s2.constants.asm`.

use sigil_frontend_as::{assemble, Options};

/// `padding off` keeps a `dc.b` image packed; `phase 0` puts the section at
/// address 0 so an image offset IS the PC.
const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n";

/// Assemble AND LINK. An enum member that was never bound survives the front end
/// as a deferred fixup, so `assemble` alone can return `Ok` and a byte assertion
/// on it would be vacuous — the link is what refuses an unbound name.
fn image(body: &str) -> Vec<u8> {
    let src = format!("{HEAD}{body}");
    let m = assemble(&src, &Options::default())
        .unwrap_or_else(|e| panic!("did not assemble:\n{src}\n{e:?}"));
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .unwrap_or_else(|e| panic!("did not resolve:\n{src}\n{e:?}"));
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .unwrap_or_else(|e| panic!("did not link:\n{src}\n{e:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Whether the source is refused by the front end.
fn refused(body: &str) -> bool {
    assemble(&format!("{HEAD}{body}"), &Options::default()).is_err()
}

/// Probe q1. `enum` restarts, `nextenum` continues.
///
/// ```text
///        4/       0 : =$1..$3              	enum a=1,b,c
///        5/       0 : 0102 03             	dc.b a,b,c
///        6/       3 : =$10..$12            	enum d=$10,e,f
///        7/       3 : 1011 12             	dc.b d,e,f
///        8/       6 : =$13..$14            	nextenum g,h
///        9/       6 : 1314                	dc.b g,h
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn enum_restarts_and_nextenum_continues() {
    let got = image(
        "\tenum a=1,b,c\n\tdc.b a,b,c\n\
         \tenum d=$10,e,f\n\tdc.b d,e,f\n\
         \tnextenum g,h\n\tdc.b g,h\n",
    );
    assert_eq!(
        got,
        vec![0x01, 0x02, 0x03, 0x10, 0x11, 0x12, 0x13, 0x14],
        "asl listing: =$1..$3, =$10..$12, =$13..$14"
    );
}

/// Probe q5. With no explicit start a member list begins at 0.
///
/// ```text
///        4/       0 : =$0..$2              	enum a,b,c
///        5/       0 : 0001 02             	dc.b a,b,c
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn default_start_is_zero() {
    assert_eq!(image("\tenum a,b,c\n\tdc.b a,b,c\n"), vec![0x00, 0x01, 0x02]);
}

/// Probe q6. A leading `nextenum` with no `enum` ahead of it is NOT an error;
/// the counter is simply its initial 0. This is not a corner case — it is how
/// a continuation reads before anything has set the counter.
///
/// ```text
///        4/       0 : =$0..$1              	nextenum q,r
///        5/       0 : 0001                	dc.b q,r
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn bare_leading_nextenum_starts_at_zero_without_diagnostic() {
    assert!(!refused("\tnextenum q,r\n\tdc.b q,r\n"));
    assert_eq!(image("\tnextenum q,r\n\tdc.b q,r\n"), vec![0x00, 0x01]);
}

/// Probe q11. `enum` genuinely RESETS the counter rather than defaulting its
/// start to the running value — this is the only thing separating it from
/// `nextenum`.
///
/// ```text
///        4/       0 : =$5..$6              	enum a=5,b
///        5/       0 : =$0..$1              	enum c,d
///        6/       0 : 0506 0001           	dc.b a,b,c,d
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn enum_resets_the_counter() {
    assert_eq!(
        image("\tenum a=5,b\n\tenum c,d\n\tdc.b a,b,c,d\n"),
        vec![0x05, 0x06, 0x00, 0x01],
        "asl listing: =$5..$6 then =$0..$1"
    );
}

/// Probe q2. `enumconf` sets the STEP. This is the corpus's octave stride: the
/// pitch table opens `enumconf $C` to walk twelve semitones per entry.
///
/// ```text
///        4/       0 :                     	enumconf $C
///        5/       0 : =$88..$A0            	enum a=$88,b,c
///        6/       0 : 8894 A0             	dc.b a,b,c
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn enumconf_sets_the_step() {
    assert_eq!(
        image("\tenumconf $C\n\tenum a=$88,b,c\n\tdc.b a,b,c\n"),
        vec![0x88, 0x94, 0xA0],
        "asl listing: =$88..$A0"
    );
}

/// Probe q12. The step persists across `enum` — only `enumconf` changes it.
///
/// ```text
///        5/       0 : =$5..$8              	enum a=5,b
///        6/       0 : =$0..$3              	enum c,d
///        7/       0 : =$6..$9              	nextenum e,f
///        8/       0 : 0508 0003 0609      	dc.b a,b,c,d,e,f
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn step_persists_across_enum() {
    assert_eq!(
        image("\tenumconf 3\n\tenum a=5,b\n\tenum c,d\n\tnextenum e,f\n\tdc.b a,b,c,d,e,f\n"),
        vec![0x05, 0x08, 0x00, 0x03, 0x06, 0x09]
    );
}

/// Probes q7 and q9. A negative step counts down and a zero step stands still;
/// neither is an error.
///
/// ```text
///        4/       0 :                     	enumconf -1
///        5/       0 : =$5..$3              	enum a=5,b,c
///        6/       0 : 0504 03             	dc.b a,b,c
///
///        4/       0 :                     	enumconf 0
///        5/       0 : =$5..$5              	enum a=5,b,c
///        6/       0 : 0505 05             	dc.b a,b,c
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn step_may_be_negative_or_zero() {
    assert_eq!(
        image("\tenumconf -1\n\tenum a=5,b,c\n\tdc.b a,b,c\n"),
        vec![0x05, 0x04, 0x03]
    );
    assert_eq!(
        image("\tenumconf 0\n\tenum a=5,b,c\n\tdc.b a,b,c\n"),
        vec![0x05, 0x05, 0x05]
    );
}

/// Probe q3. An explicit `name=expr` mid-list sets the counter, so the member
/// AFTER it continues from that value and not from the one the list would have
/// reached on its own.
///
/// This is the row the corpus's note table is built on: `nCs0,nDb0=nCs0,nD0`
/// gives the flat the sharp's value without costing the table a slot, and
/// `nD0` must still land one semitone above. Get this wrong and every note
/// above the first enharmonic shifts.
///
/// ```text
///        4/       0 : =$80..$83            	enum a=$80,b,c=b,d,e
///        5/       0 : 8081 8182 83        	dc.b a,b,c,d,e
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn explicit_member_value_moves_the_counter() {
    assert_eq!(
        image("\tenum a=$80,b,c=b,d,e\n\tdc.b a,b,c,d,e\n"),
        vec![0x80, 0x81, 0x81, 0x82, 0x83],
        "asl listing: =$80..$83"
    );
}

/// Probe q4. The step is read at the moment of each advance and NEVER
/// re-applied, so an `enumconf` between two enumerations cannot reach back into
/// a counter that has already moved.
///
/// A reading where `nextenum` resumes at `last + current_step` gives `c=$5`.
/// AS gives `c=$8`, because the counter was advanced past `b` by the step in
/// force on the `enum` line.
///
/// ```text
///        5/       0 : =$0..$4              	enum a=0,b
///        6/       0 :                     	enumconf 1
///        7/       0 : =$8..$9              	nextenum c,d
///        8/       0 : 0004 0809           	dc.b a,b,c,d
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn enumconf_is_not_retroactive() {
    assert_eq!(
        image("\tenumconf 4\n\tenum a=0,b\n\tenumconf 1\n\tnextenum c,d\n\tdc.b a,b,c,d\n"),
        vec![0x00, 0x04, 0x08, 0x09],
        "asl binds c=$8, not $5 — listing: =$0..$4 then =$8..$9"
    );
}

/// Probe q9 (member ahead of its own line). An enum member is an ordinary
/// two-pass symbol: a reference ABOVE the `enum` line that binds it resolves.
///
/// ```text
///        4/       0 : 07                  	dc.b z
///        5/       1 : =$7                  	enum z=7
/// ```
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only - see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn member_resolves_above_its_own_enum_line() {
    assert_eq!(image("\tdc.b z\n\tenum z=7\n"), vec![0x07]);
}

/// Probe q15. The start is an arbitrary expression, not a literal.
#[test]
fn start_may_be_an_expression() {
    assert_eq!(
        image("k\tEQU 3\n\tenum a=k*2,b\n\tdc.b a,b\n"),
        vec![0x06, 0x07]
    );
}

/// An empty member list and a stepless `enumconf` are refusals, as they are in
/// AS (`error #1110: wrong number of operands`).
#[test]
fn empty_operand_lists_are_refused() {
    assert!(refused("\tenum\n\tdc.b 0\n"), "bare `enum` must be refused");
    assert!(
        refused("\tenumconf\n\tdc.b 0\n"),
        "stepless `enumconf` must be refused"
    );
}

/// The corpus's own pitch and note tables, copied VERBATIM out of
/// `s1disasm/sound/_smps2asm_inc.asm` lines 22-34 — the file S1 and S2 share.
/// The full table runs to line 47; this is its head, through the third octave
/// continuation, which is where every construct the table uses has appeared.
///
/// This is the construct in the shape that actually ships: an `enumconf $C`
/// octave stride, two `enum` restarts, an `enumconf 1`, then one `enum
/// nRst=$80` followed by sixteen `nextenum` continuations whose enharmonics are
/// all explicit mid-list assignments.
///
/// The expectation is the byte image `asl` + `p2bin` produce for the identical
/// text over all 53 names. A wrong step, a non-resetting `enum`, or a counter
/// that ignores an explicit member would each shift a suffix of it.
#[test]
fn corpus_pitch_and_note_tables_match_asl() {
    const TABLE: &str = "\tenumconf\t$C\n\
\tenum\t\tsmpsPitch10lo=$88,smpsPitch09lo,smpsPitch08lo,smpsPitch07lo,smpsPitch06lo\n\
\tnextenum\tsmpsPitch05lo,smpsPitch04lo,smpsPitch03lo,smpsPitch02lo,smpsPitch01lo\n\
\tenum\t\tsmpsPitch00=$00,smpsPitch01hi,smpsPitch02hi,smpsPitch03hi,smpsPitch04hi\n\
\tnextenum\tsmpsPitch05hi,smpsPitch06hi,smpsPitch07hi,smpsPitch08hi,smpsPitch09hi\n\
\tnextenum\tsmpsPitch10hi\n\
\tenumconf\t1\n\
\tenum\t\tnRst=$80\n\
\tnextenum\tnC0,nCs0,nDb0=nCs0,nD0,nDs0,nEb0=nDs0,nE0,nFb0=nE0,nEs0,nF0=nEs0\n\
\tnextenum\tnFs0,nGb0=nFs0,nG0,nGs0,nAb0=nGs0,nA0,nAs0,nBb0=nAs0,nB0,nCb1=nB0,nBs0\n\
\tnextenum\tnC1=nBs0,nCs1,nDb1=nCs1,nD1,nDs1,nEb1=nDs1,nE1,nFb1=nE1,nEs1,nF1=nEs1\n";

    // The names in binding order, and the values asl's listing gives them.
    let names = [
        "smpsPitch10lo",
        "smpsPitch09lo",
        "smpsPitch08lo",
        "smpsPitch07lo",
        "smpsPitch06lo",
        "smpsPitch05lo",
        "smpsPitch04lo",
        "smpsPitch03lo",
        "smpsPitch02lo",
        "smpsPitch01lo",
        "smpsPitch00",
        "smpsPitch01hi",
        "smpsPitch02hi",
        "smpsPitch03hi",
        "smpsPitch04hi",
        "smpsPitch05hi",
        "smpsPitch06hi",
        "smpsPitch07hi",
        "smpsPitch08hi",
        "smpsPitch09hi",
        "smpsPitch10hi",
        "nRst", "nC0", "nCs0", "nDb0", "nD0", "nDs0", "nEb0", "nE0", "nFb0", "nEs0", "nF0",
        "nFs0", "nGb0", "nG0", "nGs0", "nAb0", "nA0", "nAs0", "nBb0", "nB0", "nCb1", "nBs0",
        "nC1", "nCs1", "nDb1", "nD1", "nDs1", "nEb1", "nE1", "nFb1", "nEs1", "nF1",
    ];
    #[rustfmt::skip]
    let expect: Vec<u8> = vec![
        // enumconf $C — the octave stride
        0x88, 0x94, 0xA0, 0xAC, 0xB8, 0xC4, 0xD0, 0xDC, 0xE8, 0xF4,
        0x00, 0x0C, 0x18, 0x24, 0x30, 0x3C, 0x48, 0x54, 0x60, 0x6C, 0x78,
        // enumconf 1 — the chromatic note table, enharmonics sharing values
        0x80, 0x81, 0x82, 0x82, 0x83, 0x84, 0x84, 0x85, 0x85, 0x86, 0x86,
        0x87, 0x87, 0x88, 0x89, 0x89, 0x8A, 0x8B, 0x8B, 0x8C, 0x8C, 0x8D,
        0x8D, 0x8E, 0x8E, 0x8F, 0x90, 0x90, 0x91, 0x91, 0x92, 0x92,
    ];
    assert_eq!(names.len(), expect.len(), "test table is self-consistent");

    let body: String = TABLE
        .chars()
        .chain(
            names
                .iter()
                .flat_map(|n| format!("\tdc.b {n}\n").chars().collect::<Vec<_>>()),
        )
        .collect();
    assert_eq!(
        image(&body),
        expect,
        "the corpus's own pitch and note table must reproduce asl byte for byte"
    );
}
