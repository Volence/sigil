//! `+` over STRING operands, and the two things that ride with it: a
//! `function` body's parameters reaching inside a string literal, and a
//! `\{expr}` interpolation inside a string that arrives at bytes through
//! something other than a bare literal.
//!
//! Row AS-STRING-PLUS-SILENT. What made this worth a parcel is that three of
//! the four shapes below were silently WRONG rather than refused: sigil exited
//! 0 and wrote different bytes from asl, so nothing in a build said so.
//!
//! | source | asl | sigil before |
//! |---|---|---|
//! | `dc.b "-"+"x"` | `2D 78` | `A5`, exit 0 |
//! | `f function number,"a"+"b"` / `dc.b f(1)` | `61 62` | `C3`, exit 0 |
//! | `f function number,"$\{abs(number)}"` / `dc.b f(-5)` | `24 35` | the 15 bytes of the source text, exit 0 |
//! | `dc.b substr("$\{abs(-5)}",0,0)` | `24 35` | the 11 bytes of the source text, exit 0 |
//!
//! The consequence: Sonic 1's `signedToString`
//! (`s1disasm/MacroSetup.asm(221)`) did not assemble.
//!
//! # Provenance
//!
//! Every fixture here IS a probe file asl assembled. Source, listing and
//! verdict are read from
//! `docs/superpowers/notes/2026-09-12-as-missing-builtins/probes/` at test
//! time, so the text sigil is tested on cannot drift from the text the oracle
//! answered, and the expected image is REBUILT FROM asl's LISTING rather than
//! copied out of it by hand. The oracle is asl 1.42 Beta Bld 212,
//! `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run through `asl_ref.sh`'s `asl_run`
//! with `-xx -n -q -A -L -U -i .`.
//!
//! [`builds`] first requires the probe's recorded `ASL_EXIT=0`: a run carrying
//! any error is not a source of values for the lines that did assemble, so a
//! listing is only read when the run was clean.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | `+` concatenates, but only for two bare literals | `a_chain_of_concatenations_folds_left_to_right` |
//! | `+` concatenates any pair, numeric `+` caught with it | `plus_over_non_strings_stays_numeric` |
//! | a function parameter substituted only inside `\{…}` | `a_function_parameter_reaches_inside_a_string_literal` |
//! | a parameter substituted as a SUBSTRING rather than a whole word | `a_function_parameter_reaches_inside_a_string_literal` |
//! | the argument pasted unparenthesised | `a_function_parameter_reaches_inside_a_string_literal` |
//! | interpolation run for a bare literal only, not for a computed string | `an_interpolation_is_folded_in_a_computed_string` |
//! | a string-valued function result not bindable by `set` | `a_string_valued_function_result_binds_to_a_symbol` |
//! | the whole feature, minus `sgn`/`substr` (which already worked) | `sonic_1_signed_to_string_assembles` |

// REASON: the doc comments quote asl's listing rows verbatim, and asl separates
// a row's byte column from its echoed source with a TAB. The tabs are the
// evidence, so they are not reflowed to spaces (the same call `charset.rs`
// makes). Scoped to this test file.
#![allow(clippy::tabs_in_doc_comments)]

use std::path::PathBuf;

use sigil_frontend_as::{assemble_root_located, Options};

fn probe_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-12-as-missing-builtins/probes")
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
/// The continuation rows are load-bearing here and not a detail: asl prints at
/// most six bytes per row, and `signed`'s `dc.b "\{sgn(-5)}"` is SIXTEEN bytes
/// (`FFFFFFFFFFFFFFFF`, a 64-bit -1 in asl's hex interpolation), so a reader
/// that took only the numbered rows would expect six and call a correct
/// assembler wrong.
///
/// The byte column is the text between ` : ` and the tab that starts the echoed
/// source; a row whose column is not hex (`(MACRO)`, `="-$5"`, `=>TRUE`)
/// carries no bytes. Reading stops at the symbol table.
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

/// Assemble and link one probe. `Ok` is the image, `Err` the messages of
/// whichever stage refused it.
fn sigil_image(name: &str) -> Result<Vec<u8>, Vec<String>> {
    let path = probe_dir().join(format!("{name}.asm"));
    let m = assemble_root_located(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked =
        sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).map_err(|e| vec![format!("{e:?}")])?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
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

/// `+` between two string literals is CONCATENATION, not the sum of their
/// packed character codes. `v_concat_lit.lst`, exit 0:
///
/// ```text
///        4/       0 : 2D78                	dc.b "-"+"x"
/// ```
///
/// `2D 78`, two bytes. sigil read each one-character string as its character
/// code and added them, writing the one byte `A5` and exiting 0. The two
/// readings are distinguishable in the LENGTH as well as the value, which is
/// why this probe uses two characters that do not sum to either of them.
///
/// `v_concat_substr` is the same law with a computed left operand
/// (`substr("-",0,1)+"x"`, also `2D78`), so a fix that special-cased a pair of
/// literal tokens does not satisfy it.
#[test]
fn plus_between_two_strings_concatenates() {
    builds("v_concat_lit");
    builds("v_concat_substr");
}

/// The chain and its edges, `v_concat_chain.lst`, exit 0:
///
/// ```text
///        4/       0 : 6162 6364 6566 EE   	dc.b "ab"+"cd"+"ef",$EE
///        5/       7 : 41EE                	dc.b "A"+"",$EE
///        6/       9 : 42EE                	dc.b ""+"B",$EE
///        7/       B : EE                  	dc.b ""+"",$EE
///        8/       C : 6162 63EE           	dc.b ("a"+"b")+"c",$EE
///        9/      10 : 34EE                	dc.b "\{strlen("ab"+"cd")}",$EE
///       12/      16 : 656C 2D78 79EE      	dc.b substr("hello",1,2)+"-"+lowstring("XY"),$EE
/// ```
///
/// Three operands, so a fix that handles exactly one `+` is red. The empty
/// string is an identity on both sides and `""+""` contributes nothing at all
/// (line 7 is the `$EE` alone). Line 9 is the one that proves the result is a
/// STRING and not merely bytes at a data directive: `strlen` of it is 4.
#[test]
fn a_chain_of_concatenations_folds_left_to_right() {
    builds("v_concat_chain");
}

/// `+` over operands that are not strings stays ARITHMETIC. The same probe,
/// exit 0:
///
/// ```text
///       10/      12 : 03EE                	dc.b 1+2,$EE
///       11/      14 : 62EE                	dc.b "a"+1,$EE
/// ```
///
/// These are the negative rows, and they are in the acceptance set on purpose:
/// the risk in teaching `+` about strings is that it stops being addition. A
/// one-character string plus an integer is asl's packed-character arithmetic
/// (`"a"+1` is `b`), which sigil already reached through its numeric path, and
/// this test exists to keep it there. `1+2` is `03` and not a concatenation of
/// two digits.
///
/// This test shares a probe with the one above and is deliberately not merged
/// into it: a single test over `v_concat_chain` would let a reader see one
/// failure and not know which half moved.
#[test]
fn plus_over_non_strings_stays_numeric() {
    let want = listing_image("v_concat_chain");
    let got = sigil_image("v_concat_chain").expect("v_concat_chain assembles");
    // `dc.b 1+2,$EE` and `dc.b "a"+1,$EE` are at $12..$16 in asl's listing.
    assert_eq!(&got[0x12..0x16], &want[0x12..0x16], "the numeric `+` rows moved");
    assert_eq!(&got[0x12..0x16], &[0x03, 0xEE, 0x62, 0xEE], "asl: 03 EE 62 EE");
}

/// A `function` body's parameters reach INSIDE a string literal, whole word,
/// and the argument is pasted PARENTHESISED. `v_fn_str_body.lst`, exit 0:
///
/// ```text
///        5/       0 : 2833 29EE           	dc.b fa(3),$EE          fa function n,"n"
///        7/       4 : 33EE                	dc.b fb(3),$EE          fb function n,"\{n}"
///        9/       6 : 2833 2920 6973      	dc.b fc(3),$EE          fc function n,"n is \{n}"
///                C : 2033 EE
///       11/       F : 6E6F 6E65 33EE      	dc.b fd(3),$EE          fd function n,"none\{n}"
///       13/      15 : 34EE                	dc.b fe(3),$EE          fe function num,"\{num+1}"
///       15/      17 : 63EE                	dc.b ff(2),$EE          ff function n,substr("abcdef",n,1)
/// ```
///
/// Three separable claims, each with its own row, because a fix can satisfy any
/// one of them alone:
///
/// * `fa` is `(3)` and not `3`: the pasted text carries its parentheses, which
///   is what keeps `fe`'s `\{num+1}` reading `(3)+1` and not a re-lex of
///   whatever the argument's own spelling would have meant in context.
/// * `fd` is `none3` and not `(3)one3`: the boundary is a whole WORD, so the
///   `n` inside `none` is not a parameter occurrence.
/// * `fb` and `fe` are the interpolating rows, `fa` and `fc` the plain-text
///   ones, so a fix that substituted only inside `\{…}` leaves `fa` red.
///
/// sigil before this parcel emitted the literal source text for every row: `6E`
/// for `fa(3)` and the six bytes of `\{n}` for `fb(3)`, exit 0.
#[test]
fn a_function_parameter_reaches_inside_a_string_literal() {
    builds("v_fn_str_body");
}

/// A `\{expr}` inside a string literal is folded even when the string arrives
/// at the data directive through a COMPUTED expression rather than as a bare
/// literal token. `v_interp_substr.lst`, exit 0:
///
/// ```text
///        4/       0 : 2435 EE             	dc.b substr("$\{abs(-5)}",0,0),$EE
///        5/       3 : 6135 62EE           	dc.b lowstring("A\{abs(-5)}B"),$EE
///        6/       7 : 2D24 35EE           	dc.b "-"+"$\{abs(-5)}",$EE
///        7/       B : 3521 EE             	dc.b "\{abs(-5)}"+"!",$EE
/// ```
///
/// The first two rows carry no `+` at all, and they were silently wrong before
/// this parcel: sigil wrote the eleven bytes of the source text `$\{abs(-5)}`
/// and exited 0. So the interpolation gap is a defect in its own right that
/// this parcel's fix closes, and not a consequence of `+`. It is guarded here
/// rather than left to `v_concat`'s coverage precisely because `+` is not what
/// causes it.
#[test]
fn an_interpolation_is_folded_in_a_computed_string() {
    builds("v_interp_substr");
    builds("v_concat");
}

/// A function whose body is a string, called and bound with `set`, and the
/// symbol emitted. `v_fn_full_str.lst`, exit 0:
///
/// ```text
///        4/       0 :                     f function number,substr("-",0,-sgn(number))+"$\{abs(number)}"
///        5/       0 : ="-$5"               S set f(-5)
///        6/       0 : 2D24 35             	dc.b S
/// ```
///
/// asl's symbol table carries `S : "-$5"`, so the binding is string-typed and
/// the three bytes come from it. sigil before this parcel refused with
/// `unresolved symbol S`: the `set` string branch probed `eval_str` without
/// expanding the `function` call first, so `f(-5)` was not a string to it and
/// fell through to the integer path, which had nothing to resolve.
///
/// This is the one shape of the five that failed LOUDLY, and it is still worth
/// a test of its own: a fix aimed only at the byte-emitting sites leaves it
/// exactly as it was.
#[test]
fn a_string_valued_function_result_binds_to_a_symbol() {
    builds("v_fn_full_str");
    builds("v_fn_concat_const");
    builds("v_fn_interp_only");
}

/// The whole feature, on Sonic 1's real definition. `signed.lst`, exit 0:
///
/// ```text
///        4/       0 :                     signedToString function number,substr("-",0,-sgn(number))+"$\{abs(number)}"
///        5/       0 : 2D24 35             	dc.b signedToString(-5)
///        6/       3 : 2435                	dc.b signedToString(5)
///        7/       5 : 2D24 30             	dc.b signedToString(0)
///        8/       8 : 2D24 3132 33        	dc.b signedToString(-$123)
///        9/       D : 4646 4646 4646      	dc.b "\{sgn(-5)}"
///                13 : 4646 4646 4646
///                19 : 4646 4646
///       10/      1D : 3430                	dc.b "\{bitcnt(-1)}"
/// ```
///
/// Two rows here are easy to read wrong, and both are asl's answer rather than
/// a typo. `signedToString(0)` is `-$0` and not `$0`: asl's `substr(s,p,0)`
/// means "from `p` to the END", so the zero length that `-sgn(0)` produces
/// keeps the minus sign rather than dropping it. And line 9 is SIXTEEN bytes of
/// `46`, not the six the numbered row shows: `sgn(-5)` is -1 and asl renders an
/// interpolated integer as 64-bit uppercase hex, `FFFFFFFFFFFFFFFF`.
#[test]
fn sonic_1_signed_to_string_assembles() {
    builds("signed");
}

/// The control. `signed_sgn` is the `sgn`/`substr` half of `signedToString`
/// with the `+` removed, and it assembled correctly BEFORE this parcel:
///
/// ```text
///        5/       0 : 2D                  	dc.b signPrefix(-5)
///        6/       1 : 2D                  	dc.b signPrefix(0)
///        7/       2 : 2D24                	dc.b signPrefix(-$123),"$"
///        8/       4 : 2D78                	dc.b substr("-",0,-sgn(-5)),"x"
/// ```
///
/// It is in this file so that the parcel's own suite says whether the machinery
/// it extended still answers what it already answered. A change to `eval_str`
/// that taught it `+` at the cost of the plain `substr` path would be green
/// everywhere else here and red only on this.
#[test]
fn the_substr_half_that_already_worked_still_works() {
    builds("signed_sgn");
}
