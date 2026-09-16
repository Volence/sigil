//! asl's STRING TYPING in the numeric evaluator: rows
//! `AS-STRING-PLUS-NUMERIC-CONTEXT`, `AS-STRING-PLUS-INT` and
//! `AS-STRING-SYMBOL-INT-SLOT`, which share one root cause.
//!
//! The failure the first row names is a SILENT one, which is why it is worth a
//! test file of its own: `move.w #"a"+"b",d0` assembled at exit 0 as
//! `303C 00C3`, the sum of the packed character codes, where asl writes
//! `303C 6162`, the concatenation packed. Nothing announced it, and every
//! downstream check inherited the silence.
//!
//! | source | asl | the wrong reading it replaced |
//! |---|---|---|
//! | `move.w #"a"+"b",d0` | `303C 6162` | `303C 00C3` |
//! | `move.l #"ab"+"cd",d0` | `203C 6162 6364` | `203C 0000 C4C6` |
//! | `move.w "a"+"b",d0` | `3038 6162` | `3038 00C3` |
//! | `dc.b "ab"+1` | `61 63` | a `dc.b` range refusal |
//! | `move.w #S2,d0`, `S2 equ "ab"` | `303C 6162` | `unresolved symbol S2` |
//!
//! # The three rules under test
//!
//! **R1.** `+` is the ONLY operator that propagates stringness, so the type
//! turns on the ROOT of the operator tree.
//! [`a_non_plus_operator_makes_it_an_integer`] and
//! [`the_root_operator_decides_the_type`].
//!
//! **R2.** A string renders one element PER CHARACTER in a data directive and
//! PACKS in an integer slot. [`an_all_string_plus_packs_in_an_integer_slot`]
//! and [`a_string_symbol_reads_as_its_literal_in_every_slot`].
//!
//! **R3.** `+`'s value: string+string concatenates, string+integer is packed
//! arithmetic rendered in the MINIMAL bytes the sum needs.
//! [`a_string_plus_an_integer_is_packed_arithmetic`] and
//! [`the_length_is_the_sums_minimal_bytes_not_the_strings`].
//!
//! # Provenance
//!
//! Every fixture is a probe file asl assembled, and the expected image is
//! REBUILT FROM asl's LISTING at test time rather than copied out of it by
//! hand, so the text sigil is tested on cannot drift from the text the oracle
//! answered. Probes, listings and recorded exit statuses are
//! `docs/superpowers/notes/2026-09-15-as-string-numeric-typing/probes/`; the
//! matrix they were read off is the note beside that directory.
//!
//! The oracle is asl 1.42 Beta Bld 212,
//! `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run through `asl_ref.sh`'s `asl_run`
//! with `-xx -n -q -A -L -U -i .`. [`builds`] first requires the probe's
//! recorded `ASL_EXIT=0`: a run carrying any error is not a source of values
//! for the lines that DID assemble, so a listing is only read when the run was
//! clean.
//!
//! # The shapes with no oracle value at all
//!
//! Two families are asserted as REFUSALS against sigil alone, with no asl byte
//! compared, because asl has no dependable answer for them and the tests say so
//! in their own words: a `string + integer` outside 1 to 4 characters (asl
//! emits nothing at exit 0, or a different value on every run) and the same
//! under a non-identity `charset`. See
//! [`a_string_plus_integer_asl_declines_is_refused_not_guessed`].
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | `+` concatenates in `dc.b` only, not in an integer slot | `an_all_string_plus_packs_in_an_integer_slot` |
//! | the packing applied to the whole operand LIST, not per operand | `an_all_string_plus_packs_in_an_integer_slot` |
//! | string+integer keeps the STRING's length | `the_length_is_the_sums_minimal_bytes_not_the_strings` |
//! | string+integer takes `max(len, needed)` | `the_length_is_the_sums_minimal_bytes_not_the_strings` |
//! | any `+` in the expression makes it string-typed | `the_root_operator_decides_the_type` |
//! | a non-`+` operator propagates stringness | `a_non_plus_operator_makes_it_an_integer` |
//! | a string symbol resolved BEFORE the integer environment | `a_string_symbol_reads_as_its_literal_in_every_slot` |
//! | a string symbol packed into `dc.w`/`dc.l` instead of refused | `a_string_symbol_in_wide_data_stays_loud` |
//! | a declined string+integer answered instead of refused | `a_string_plus_integer_asl_declines_is_refused_not_guessed` |
//! | a refusal routed as "not a string" and folded numerically | `a_string_plus_integer_asl_declines_is_refused_not_guessed` |

// REASON: the doc comments quote asl's listing rows verbatim, and asl separates
// a row's byte column from its echoed source with a TAB. The tabs are the
// evidence, so they are not reflowed to spaces. Scoped to this test file.
#![allow(clippy::tabs_in_doc_comments)]

use std::path::PathBuf;

use sigil_frontend_as::{assemble_root_located, Options};

fn probe_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-15-as-string-numeric-typing/probes")
}

fn read(name: &str, ext: &str) -> String {
    let path = probe_dir().join(format!("{name}.{ext}"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()))
}

/// asl's exit status for the probe, as `asl_run` recorded it.
fn asl_exit(name: &str) -> i32 {
    let out = read(name, "asl.out");
    let line = out
        .lines()
        .find_map(|l| l.strip_prefix("ASL_EXIT="))
        .unwrap_or_else(|| panic!("{name}.asl.out records no ASL_EXIT"));
    line.trim().parse().expect("ASL_EXIT is a number")
}

/// The image asl's listing shows: every `address : bytes` row, the unnumbered
/// continuation rows of a long line included, placed at its address.
///
/// The byte column is the text between ` : ` and the tab that starts the echoed
/// source; a row whose column is not hex (`="ab"`, `(MACRO)`) carries no bytes.
/// Reading stops at the symbol table, which is where the `="ab"` rows would
/// otherwise reappear as prose.
fn listing_image(name: &str) -> Vec<u8> {
    let mut image = Vec::new();
    for line in read(name, "lst").lines() {
        if line.contains("Symbol Table") {
            break;
        }
        let Some((left, right)) = line.split_once(" : ") else { continue };
        let Some(addr) = left
            .split_whitespace()
            .last()
            .and_then(|a| usize::from_str_radix(a, 16).ok())
        else {
            continue;
        };
        let column = right.split('\t').next().unwrap_or("").trim();
        if column.is_empty() || !column.chars().all(|c| c.is_ascii_hexdigit() || c == ' ') {
            continue;
        }
        let hex: String = column.chars().filter(|c| *c != ' ').collect();
        assert!(hex.len().is_multiple_of(2), "{name}.lst: odd byte column in {line:?}");
        for (k, pair) in hex.as_bytes().chunks(2).enumerate() {
            let byte = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
            if image.len() <= addr + k {
                image.resize(addr + k + 1, 0);
            }
            image[addr + k] = byte;
        }
    }
    image
}

fn link_image(m: sigil_ir::Module) -> Result<Vec<u8>, Vec<String>> {
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked =
        sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).map_err(|e| vec![format!("{e:?}")])?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

/// Assemble and link one committed probe.
fn sigil_image(name: &str) -> Result<Vec<u8>, Vec<String>> {
    let path = probe_dir().join(format!("{name}.asm"));
    let m = assemble_root_located(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    link_image(m)
}

/// asl assembled the probe cleanly, and sigil's image is the one its listing
/// shows, byte for byte.
#[track_caller]
fn builds(name: &str) {
    assert_eq!(
        asl_exit(name),
        0,
        "{name}: not a clean asl run, so its listing is no source of values"
    );
    let want = listing_image(name);
    assert!(!want.is_empty(), "{name}: the listing shows no bytes");
    match sigil_image(name) {
        Ok(got) if got == want => {}
        Ok(got) => {
            let at = got
                .iter()
                .zip(&want)
                .position(|(g, w)| g != w)
                .unwrap_or(got.len().min(want.len()));
            let end = |v: &[u8]| v[at.min(v.len())..(at + 8).min(v.len())].to_vec();
            panic!(
                "{name}: sigil's image differs from asl's listing at ${at:X} (asl {} bytes, sigil {}):\n  asl   {:02X?}\n  sigil {:02X?}",
                want.len(),
                got.len(),
                end(&want),
                end(&got)
            );
        }
        Err(m) => panic!("{name}: asl assembles it, sigil refused: {m:?}"),
    }
}

/// asl REFUSED the probe (its recorded exit is not 0), and sigil refuses it too.
#[track_caller]
fn refused(name: &str) {
    assert_ne!(asl_exit(name), 0, "{name}: asl assembled it, so it is not a refusal probe");
    if let Ok(got) = sigil_image(name) {
        panic!("{name}: asl refuses it, sigil built {got:02X?}");
    }
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const Z80_HEAD: &str = "\tcpu z80undoc\n\tpadding off\n\torg 0\n";

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    assemble_with(HEAD, body)
}

fn assemble_with(head: &str, body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => link_image(m),
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

#[track_caller]
fn diags(body: &str) -> Vec<String> {
    match assemble(body) {
        Ok(b) => panic!("expected a refusal for {body:?}, got bytes: {b:02X?}"),
        Err(d) => d,
    }
}

/// R2's integer-slot rendering, and THE SILENT DEFECT THIS PARCEL EXISTS FOR.
///
/// `t_plus_int_ctx.lst`, exit 0:
///
/// ```text
///        4/       0 : 303C 6162           	move.w #"a"+"b",d0
///        5/       4 : 203C 6162 6364      	move.l #"ab"+"cd",d0
///        6/       A : 3038 6162           	move.w "a"+"b",d0
///        7/       E : 2039 6162 6364      	move.l "abcd",d0
/// ```
///
/// An all-string `+` CONCATENATES and the concatenation is then packed, so
/// `#"a"+"b"` is `6162`. The arithmetic reading sums the packed codes and gives
/// `00C3`, which is what sigil wrote, at exit 0, with no diagnostic.
///
/// The absolute-address row is here deliberately: it is the same rendering as
/// the immediate (`3038 6162`), so a fix that reached only `#` operands leaves
/// it wrong. So is the two-literal `move.l "abcd",d0`, which packs four
/// characters with no `+` in it at all and must not be perturbed.
#[test]
fn an_all_string_plus_packs_in_an_integer_slot() {
    builds("t_plus_int_ctx");
}

/// R3's arithmetic, `AS-STRING-PLUS-INT`. `t_plus_int_arith.lst`, exit 0:
///
/// ```text
///        4/       0 : 62EE                	dc.b "a"+1,$EE
///        5/       2 : 6163 EE             	dc.b "ab"+1,$EE
///        6/       5 : 6162 64EE           	dc.b "abc"+1,$EE
///        7/       9 : 6162 6365 EE        	dc.b "abcd"+1,$EE
///        8/       E : 6163 EE             	dc.b 1+"ab",$EE
///        9/      11 : 6260 EE             	dc.b "aa"+255,$EE
///       10/      14 : 6262 EE             	dc.b "ab"+256,$EE
/// ```
///
/// Pack, add, re-render. `"aa"+255` is `62 60`, so the carry crosses the
/// characters and this is one number and not a per-character add; `"ab"+256`
/// is `62 62`, which a per-character reading cannot produce at all. `1+"ab"`
/// is the same as `"ab"+1`, so the rule is commutative.
///
/// The one-character case `"a"+1` = `62` agreed BEFORE this parcel, by
/// accident: the numeric path's packed code plus one is the same byte. It is
/// kept because a fix that regresses it would otherwise go unnoticed.
#[test]
fn a_string_plus_an_integer_is_packed_arithmetic() {
    builds("t_plus_int_arith");
}

/// R3's LENGTH rule, and THE ONE THE LEDGER HAD WRONG.
///
/// The ledger booked "packed arithmetic that keeps the string's length". That
/// is right for every case anyone had tried and wrong in general.
/// `t_plus_len_minimal.lst`, exit 0:
///
/// ```text
///        4/       0 : 62EE                	dc.b "\x00a"+1,$EE
///        5/       2 : 6163 EE             	dc.b "\x00ab"+1,$EE
///        6/       5 : 0100 0000 EE        	dc.b "\xff\xff\xff"+1,$EE
///        7/       A : EE                  	dc.b "a"+(0-97),$EE
///        8/       B : FFFF FFFF EE        	dc.b "a"+(-98),$EE
///        9/      10 : FFFF FF35 EE        	dc.b "a"+(0-300),$EE
///       10/      15 : FFFF EC32 EE        	dc.b "ab"+(0-30000),$EE
///       11/      1A : 6161 EE             	dc.b "ab"+(0-1),$EE
///       12/      1D : 0160 EE             	dc.b "a"+255,$EE
///       13/      20 : FF01 FFFF EE        	dc.b "\xfe\xff\xff\xff"+$20000,$EE
/// ```
///
/// Row 4 is the discriminator and the reason this test exists: the string is
/// TWO characters, the sum is `0x62`, and asl emits ONE byte. A
/// length-preserving rule emits `00 62`. Row 5 says the same with the zero in
/// front of a longer tail.
///
/// The rule is the MINIMAL number of whole bytes the sum needs, which the other
/// rows pin from every side: a carry GROWS it three to four (row 6), a sum of
/// zero keeps NO bytes and is the empty string (row 7, where `max(len, needed)`
/// would emit one), and a NEGATIVE sum takes all four as the 32-bit two's
/// complement (rows 8 to 10, which are -1, -203 and -5070). Row 13 is a
/// four-byte sum that is NOT a carry case, so the four bytes are the value's
/// own and not a floor.
///
/// Row 12 is the one that cannot be read as an integer at all: `"a"+255` is
/// `01 60`, two bytes, where the integer `0x160` would be a `dc.b` range
/// refusal. Its partner is [`the_root_operator_decides_the_type`].
///
/// WHAT IS DELIBERATELY ABSENT, and it was in this probe until the test caught
/// it: a sum needing FIVE bytes. `dc.b "\xff\xff\xff\xff"+1,$EE` is `00 EE`,
/// and the first draft of this parcel wrapped to 32 bits and emitted the `$EE`
/// alone. Widening the window to match would have been the wrong repair,
/// because that region is not an answer, see
/// [`a_string_plus_integer_asl_declines_is_refused_not_guessed`], which asserts
/// the refusal instead.
#[test]
fn the_length_is_the_sums_minimal_bytes_not_the_strings() {
    builds("t_plus_len_minimal");
}

/// `AS-STRING-SYMBOL-INT-SLOT`: a string symbol behaves EXACTLY as its literal,
/// in every slot. `t_strsym_slots.lst`, exit 0, with `S1 equ "a"`,
/// `S2 equ "ab"`, `S4 equ "abcd"`:
///
/// ```text
///        7/       0 : 61EE                	dc.b S1,$EE
///        8/       2 : 6162 EE             	dc.b S2,$EE
///        9/       5 : 6163 EE             	dc.b S2+1,$EE
///       10/       8 : 6161 62EE           	dc.b S1+S2,$EE
///       11/       C : 303C 0061           	move.w #S1,d0
///       12/      10 : 303C 6162           	move.w #S2,d0
///       13/      14 : 203C 6162 6364      	move.l #S4,d0
///       14/      1A : 3038 6162           	move.w S2,d0
///       15/      1E : 303C 6163           	move.w #S2+1,d0
///       16/      22 : 6161                	dc.w S2-1
/// ```
///
/// Not one cell differs from the same expression written with the literal,
/// which is what makes this a RESOLUTION gap and not a semantics one: sigil had
/// no route from the name to the value and said `unresolved symbol S2`.
///
/// The last row is the one that keeps the resolution honest. `dc.w S2-1` has a
/// `-` at its root, so it is an INTEGER to asl (`6161`) and reaches `dc.w`
/// legitimately, while `dc.w S2` does not
/// ([`a_string_symbol_in_wide_data_stays_loud`]). A fix that made every string
/// symbol string-typed everywhere would refuse this row.
#[test]
fn a_string_symbol_reads_as_its_literal_in_every_slot() {
    builds("t_strsym_slots");
}

/// R1: every operator but `+` packs a string operand and yields an INTEGER.
/// `t_nonplus_ops.lst`, exit 0:
///
/// ```text
///        4/       0 : 303C 6161           	move.w #"ab"-1,d0
///        5/       4 : 303C C2C4           	move.w #"ab"*2,d0
///        6/       8 : 303C 9E9E           	move.w #-"ab",d0
///        7/       C : 303C 0062           	move.w #"ab"&$00FF,d0
///        8/      10 : 303C 0061           	move.w #"ab">>8,d0
///        9/      14 : 303C 9E9D           	move.w #~"ab",d0
///       10/      18 : 303C 30B1           	move.w #"ab"/2,d0
///       11/      1C : 303C 6163           	move.w #"ab"|1,d0
///       12/      20 : 303C 6162           	move.w #"ab"+1-1,d0
/// ```
///
/// Unary `-` and `~` are in the list because they are the operators most likely
/// to be given stringness by a fix that reasons "a string operand makes a
/// string result". The last row is the root rule in an integer slot: `+` then
/// `-` leaves an integer, and `0x6162 + 1 - 1` is `6162` either way, which is
/// why the discriminating case for the root rule lives in `dc.b`.
#[test]
fn a_non_plus_operator_makes_it_an_integer() {
    builds("t_nonplus_ops");
}

/// The ROOT of the operator tree decides the type, not the presence of a `+`.
///
/// This is the other half of `dc.b "a"+255,$EE` = `01 60 EE`
/// ([`the_length_is_the_sums_minimal_bytes_not_the_strings`], row 11). Put a
/// `-0` after it and the root becomes `-`, the value becomes the integer
/// `0x160`, and `dc.b` refuses it. `t_root_is_minus.asl.out`:
///
/// ```text
///     > > > t_root_is_minus.asm(4): error #1320: range overflow
///     ASL_EXIT=2
/// ```
///
/// So the SAME arithmetic is two bytes or a refusal depending only on which
/// operator ends up at the root. An implementation that splits at every
/// top-level `+` (which is what this parcel replaced) cannot tell the pair
/// apart and emits `01 60 EE` for both.
#[test]
fn the_root_operator_decides_the_type() {
    refused("t_root_is_minus");
}

/// A string of length 0, or 5 and longer, has no integer value: asl's
/// `error #1141: expected integer, but got string`, exit 2 in both probes.
/// Sigil refuses both.
///
/// This is asl's own window and not one chosen here, and it is distinct from a
/// 1-to-4 character string that simply does not FIT the slot, which is an
/// ordinary range complaint (asserted below).
#[test]
fn a_string_with_no_packed_value_is_refused_in_an_integer_slot() {
    refused("t_str_too_long_imm");
    refused("t_empty_str_imm");
    // AND REFUSED AS THAT, not as something else. Asserting only "sigil
    // refuses" is too weak to hold the window: widening MAX_PACKED_CHARS to 8
    // leaves both probes refused, because a 5-character string then packs to a
    // 40-bit value the immediate's own RANGE check rejects. The refusal would
    // have moved from asl's #1141 to asl's #1320 with nothing going red, so
    // these two name the reason.
    for (body, chars) in [("\tmove.w #\"abcde\",d0", "5-character"), ("\tmove.w #\"\",d0", "0-character")] {
        let d = diags(body).join(" ");
        assert!(
            d.contains(chars) && d.contains("no integer value in this slot"),
            "{body:?}: must say WHY it has no integer value, got {d:?}"
        );
        assert!(
            !d.contains("out of range"),
            "{body:?}: this is asl's #1141, not a range complaint: {d:?}"
        );
    }
}

/// A string that packs but does not fit is a RANGE complaint, not a
/// not-an-integer one. asl separates them (`#1320` against `#1141`) and so does
/// sigil, so a reader is told which of the two things went wrong.
#[test]
fn a_packed_string_too_wide_for_its_slot_is_a_range_complaint() {
    let d = diags("\tmove.w #\"abc\",d0").join(" ");
    assert!(
        d.contains("out of range"),
        "a 3-character string in a word immediate is a range complaint, got {d:?}"
    );
    let d = diags("\tmove.w #\"a\"+(0-97),d0").join(" ");
    assert!(
        d.contains("0-character string"),
        "an empty computed string names itself as such, got {d:?}"
    );
}

/// A `string + integer` asl DECLINES is refused, never answered.
///
/// MEASURED 2026-09-15, and this is a shape the REFERENCE build is silently
/// wrong on at EXIT 0: `dc.b "abcde"+1,$EE` emits NOTHING AT ALL with no
/// diagnostic, `dc.b "abcdefgh"+1,$EE` swallows the `$EE` with it, and five
/// consecutive runs of `move.w #"abcde"+1,d0` returned `5605`, `0000`, `564D`,
/// `5608` and `55C6`, with an accepted `move.w #$1234,d0` above it, three runs
/// all returned `1234`, the stale-slot echo.
///
/// So there is NO asl byte to compare against, and that is exactly why this
/// test asserts a refusal rather than a value: the alternative is not "match
/// asl" but "invent a value and call it asl's". No probe is committed for these
/// for the same reason.
///
/// THE SECOND ASSERTION IS THE LOAD-BEARING ONE. A refusal must not be routed
/// as "this is not a string", because the integer path would then pack it and
/// emit a plausible wrong byte at exit 0, which is
/// `AS-STRING-PLUS-NUMERIC-CONTEXT` rebuilt one level up. Both the data
/// directive and the integer slot are checked, because they are different call
/// sites and only one of them was wired first.
#[test]
fn a_string_plus_integer_asl_declines_is_refused_not_guessed() {
    for body in [
        // The OPERAND does not pack: a 5-character string, whatever the addend
        // (`"abcde"+0` emits nothing too, so it is the pack and not the sum).
        "\tdc.b \"abcde\"+1,$EE",
        "\tdc.b \"abcdefgh\"+1,$EE",
        "\tmove.w #\"abcde\"+1,d0",
        "\tdc.b \"\"+1,$EE",
        // The SUM does not fit four bytes. asl is stable here and still not
        // answering: `"\xff\xff\xff\xff"` plus 1, 2 and 256 gives `00`, `01`
        // and `FF`, one low byte each, while `"abcde"+1`, the same five-byte
        // class, gives nothing at all. No rule explains both, and the rule
        // that explains the four-byte cases explains neither.
        "\tdc.b \"\\xff\\xff\\xff\\xff\"+1,$EE",
        "\tdc.b \"\\xff\\xff\\xff\\xff\"+256,$EE",
    ] {
        let d = diags(body).join(" ");
        assert!(
            d.contains("1 to 4 character string"),
            "{body:?}: refused, but not as a declined string+integer: {d:?}"
        );
    }
    // Under a non-identity code page the arithmetic runs on the MAPPED bytes
    // (`charset 'a',$11` gives `dc.b "ab"+1` = `11 63`), but no probe can say
    // whether asl maps the RESULT bytes a second time, and the two readings
    // emit different bytes. Refused rather than guessed.
    let d = diags("\tcharset 'a',$11\n\tdc.b \"ab\"+1,$EE").join(" ");
    assert!(
        d.contains("non-identity `charset`"),
        "string+integer under a charset is refused in its own words, got {d:?}"
    );
    // And the page itself still works: this is a refusal of the ARITHMETIC, not
    // of strings under a `charset`.
    assert_eq!(
        assemble("\tcharset 'a',$11\n\tdc.b \"ab\",$EE").expect("a plain string still maps"),
        vec![0x11, 0x62, 0xEE]
    );
}

/// A string SYMBOL in `dc.w`/`dc.l`/`dw` stays LOUD, and this is the guard on
/// the fix rather than on the defect.
///
/// asl renders a string per character at these widths (`dc.w S2` is
/// `0061 0062`), which sigil does not implement and refuses by name
/// (`STRING_IN_WIDE_DATA`). That refusal used to key on a string LITERAL still
/// standing in the operand, which was the whole population, because a string
/// with no literal in it could not resolve at all.
///
/// It can now. Once `resolve_str_packed` answers for a string symbol, `dc.w S2`
/// would reach the numeric fold, pack to `6162`, and assemble CLEANLY where asl
/// writes two zero-extended words. That is this parcel's own defect class, re-created
/// by its own fix. Every width is checked because the guard is three call
/// sites, not one.
#[test]
fn a_string_symbol_in_wide_data_stays_loud() {
    for w in ["dc.w", "dc.l", "dw"] {
        for expr in ["S2", "S2+1", "S1"] {
            let d = diags(&format!("S1 equ \"a\"\nS2 equ \"ab\"\n\t{w} {expr}")).join(" ");
            assert!(
                d.contains("wider than a byte"),
                "{w} {expr}: a string-typed operand must be refused by name, got {d:?}"
            );
        }
    }
    // The integer-typed neighbour is NOT caught: `S2-1` has a `-` at its root,
    // so asl gives the single word `6161` and so does sigil. A guard that
    // refused this would be refusing a value it gets right.
    assert_eq!(
        assemble("S2 equ \"ab\"\n\tdc.w S2-1").expect("an integer-typed root still folds"),
        vec![0x61, 0x61]
    );
}

/// An ordinary numeric `+` is untouched, which is what keeps every corpus byte
/// identical.
///
/// The typing probe answers "not a string" for these and the integer path runs
/// exactly as it did, so this is the population guard for the whole parcel: a
/// change that captured plain arithmetic would turn the corpus red, and this
/// says so in one test rather than in a ROM diff.
#[test]
fn plus_over_non_strings_stays_numeric() {
    assert_eq!(assemble("\tdc.b 1+2,$EE").expect("numeric"), vec![0x03, 0xEE]);
    assert_eq!(assemble("N equ 5\n\tdc.b N+2,$EE").expect("symbol"), vec![0x07, 0xEE]);
    assert_eq!(
        assemble("\tmove.w #1+2,d0").expect("immediate"),
        vec![0x30, 0x3C, 0x00, 0x03]
    );
    // A nameless forward label is spelled `+` in AS, so it must not be read as
    // an addition with an empty left operand.
    assert_eq!(
        assemble("\tbra.s +\n\tnop\n+\n\tnop").expect("nameless label"),
        vec![0x60, 0x02, 0x4E, 0x71, 0x4E, 0x71]
    );
}

/// A REGISTER in operand position is never a string symbol, however the name is
/// bound elsewhere. `AS-STRING-SYMBOL-INT-SLOT`'s sharp edge, and a regression
/// this parcel introduced and the corpus byte gate caught.
///
/// `s2disasm/s2.asm:14504` writes `l := lowstring("char")` inside an `irpc`, so
/// `l` is a live string-valued symbol for the rest of the assembly, and
/// `s2.sounddriver.asm` is Z80 and writes `ld l,(ix+zTrack.Detune)` 148 times.
/// With the operand packing asked unconditionally, the REGISTER was rewritten
/// into the packed character `l` had last been assigned and s2 gained 24 errors
/// reading `Ld, ops: [Imm8(99), Indexed { reg: Ix, disp: 3 }]` (99 is `'c'`).
///
/// asl settles it by POSITION: it peels the addressing mode before it evaluates
/// anything. So the same name is a register here and a symbol in an expression,
/// and the last assertion is the one that keeps the fix honest: `dc.b l` two
/// lines below that `:=` is the STRING, and a guard in the string evaluator
/// would have traded one corpus regression for another.
///
/// The expected encodings are the Z80 and 68000 ones for the register forms,
/// which is the point: any byte at all here means the operand was read as a
/// register, and the defect produced no bytes but an `unsupported form`.
#[test]
fn a_register_in_operand_position_is_never_a_string_symbol() {
    // `ld l,(ix+3)` = DD 6E 03, and `ld l,a` = 6F. Both operands matter: the
    // whole-operand scan is what covers the register inside `(ix+3)` too.
    assert_eq!(
        assemble_with(Z80_HEAD, "l := \"c\"\n\tld l,(ix+3)\n\tld l,a")
            .expect("a Z80 register operand is a register, whatever `l` is bound to"),
        vec![0xDD, 0x6E, 0x03, 0x6F]
    );
    // The 68000 half, so the guard is not quietly Z80-only: `d0`/`a0` are
    // register spellings there and `l` is an ordinary symbol.
    assert_eq!(
        assemble("d0 := \"c\"\n\tmove.w d0,d1").expect("a 68000 register operand is a register"),
        vec![0x32, 0x00]
    );
    // AND THE OTHER HALF OF asl's POSITIONAL RULE, which is what makes this a
    // guard on the operand path alone: in a DATA directive the same name is the
    // STRING. s2 writes exactly this two lines below its `:=`, and it is in the
    // 68000 half of s2, which is why the assertion is too.
    //
    // The Z80 spelling of this line is refused (`bad byte expression`), and
    // that is PRE-EXISTING and not this parcel's: measured identical on the
    // baseline binary and on this one, so a Z80 `dc.b l` never worked. It is
    // ledgered rather than fixed here.
    assert_eq!(
        assemble("l := \"c\"\n\tdc.b l,$EE").expect("`dc.b l` is the string"),
        vec![0x63, 0xEE]
    );
}
