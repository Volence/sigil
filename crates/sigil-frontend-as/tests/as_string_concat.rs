//! `+` over STRING operands, and what rides with it: a `function` body's
//! parameters reaching inside a string literal, a `\{expr}` interpolation
//! folded where its literal is, and a quote inside an interpolation.
//!
//! Row AS-STRING-PLUS-SILENT. The failures these tests are aimed at are SILENT
//! ones: a build that exits 0 with bytes asl does not write.
//!
//! | source | asl | the wrong reading |
//! |---|---|---|
//! | `dc.b "-"+"x"` | `2D 78` | `A5`, the sum of the packed codes |
//! | `f function number,"a"+"b"` / `dc.b f(1)` | `61 62` | `C3` |
//! | `f function number,"$\{abs(number)}"` / `dc.b f(-5)` | `24 35` | the 15 bytes of the source text |
//! | `dc.b substr("$\{abs(-5)}",0,0)` | `24 35` | the 11 bytes of the source text |
//! | `dc.b lowstring("\{N}")` with `N equ $AB` | `61 62` | `35`, an interpolation of a different symbol, `n` |
//! | `dc.b s` with `s := "\\{n}"` | `5C 7B 6E 7D` | `35`, the value scanned for `\{` a second time |
//!
//! The corpus demand is Sonic 1's `signedToString`
//! (`s1disasm/MacroSetup.asm(221)`), which Sonic 1 calls inside `\{…}` in its
//! `error`/`warning` text.
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
//! | no concatenation at all | `sonic_1_signed_to_string_assembles` |
//! | a function parameter substituted only inside `\{…}` | `a_function_parameter_reaches_inside_a_string_literal` |
//! | a parameter substituted as a SUBSTRING rather than a whole word | `a_function_parameter_reaches_inside_a_string_literal` |
//! | the argument pasted unparenthesised | `a_function_parameter_reaches_inside_a_string_literal` |
//! | the argument pasted as its spelling, not its decimal value | `a_pasted_argument_is_its_decimal_value` |
//! | `_` or `.` counted as part of a parameter word | `a_parameter_word_is_letters_and_digits_only` |
//! | a string or float argument pasted into a literal unchecked | `a_string_or_float_argument_pasted_into_a_literal_is_refused` |
//! | interpolation folded for a bare literal only, not a computed string | `an_interpolation_is_folded_in_a_computed_string` |
//! | interpolation folded where the string lands, not at its literal | `an_interpolation_is_folded_where_its_literal_is` |
//! | a string VALUE scanned for `\{` a second time | `a_string_value_is_not_scanned_again_for_interpolation` |
//! | a call's string result bound by `equ` as a string | `a_function_result_bound_by_equ_keeps_its_integer_reading` |
//! | a call's result bound by `set` with its interpolation unfolded | `a_string_valued_function_bound_by_set_is_never_silently_wrong` |
//! | an interpolation's string probe run before its call is expanded | `a_string_valued_function_pastes_its_string_into_an_interpolation` |
//! | integer builtins not folded inside a constant fold | `a_string_valued_function_pastes_its_string_into_an_interpolation` |
//! | a literal ended at a quote inside `\{…}` | `a_quote_inside_an_interpolation_belongs_to_it` |
//! | an interpolation ended at its first `}`, inside a quoted `"}"` | `a_quote_inside_an_interpolation_belongs_to_it` |
//!
//! Every row was applied as a mutation to the committed tree and its test went
//! red: `2026-09-12-as-missing-builtins/mutations/string-concat/`.

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

/// asl assembled the probe cleanly, and sigil either writes the image its
/// listing shows or refuses the file. The one outcome this rejects is a
/// DIFFERENT image, which is a silent wrong answer; it is the guard for a shape
/// asl accepts and sigil has not built yet.
#[track_caller]
fn builds_or_refuses(name: &str) {
    assert_eq!(
        asl_exit(name),
        0,
        "{name}: not a clean asl run, so its listing is no source of values"
    );
    let want = listing_image(name);
    assert!(!want.is_empty(), "{name}: the listing shows no bytes");
    if let Ok(got) = sigil_image(name) {
        assert_eq!(
            got, want,
            "{name}: sigil built an image asl's listing does not show (asl {} bytes, sigil {})",
            want.len(),
            got.len()
        );
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

/// `+` between two string literals is CONCATENATION, not the sum of their
/// packed character codes. `v_concat_lit.lst`, exit 0:
///
/// ```text
///        4/       0 : 2D78                	dc.b "-"+"x"
/// ```
///
/// `2D 78`, two bytes. The arithmetic reading takes each one-character string
/// as its code and adds them, the one byte `A5`. The two readings differ in
/// LENGTH as well as value, which is why this probe uses two characters that
/// do not sum to either of them.
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
///        9/      10 : ="abcd"              T set "ab"+"cd"
///       10/      10 : 04EE                	dc.b strlen(T),$EE
///       13/      16 : 656C 2D78 79EE      	dc.b substr("hello",1,2)+"-"+lowstring("XY"),$EE
/// ```
///
/// Three operands, so a fix that handles exactly one `+` is red. The empty
/// string is an identity on both sides and `""+""` contributes nothing at all
/// (line 7 is the `$EE` alone). Lines 9 and 10 are the ones that prove the
/// result is a STRING and not merely bytes at a data directive: `set` binds
/// `"abcd"` and `strlen` of it is 4.
#[test]
fn a_chain_of_concatenations_folds_left_to_right() {
    builds("v_concat_chain");
}

/// `+` over operands that are not strings stays ARITHMETIC. The same probe,
/// exit 0:
///
/// ```text
///       11/      12 : 03EE                	dc.b 1+2,$EE
///       12/      14 : 62EE                	dc.b "a"+1,$EE
/// ```
///
/// These are the negative rows, and they are in the acceptance set on purpose:
/// the risk in teaching `+` about strings is that it stops being addition. A
/// one-character string plus an integer is asl's packed-character arithmetic
/// (`"a"+1` is `b`), which is the numeric path's answer, and this test keeps
/// it there. `1+2` is `03` and not a concatenation of two digits.
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
/// A body whose literal is left alone writes its source text for every row:
/// `6E` for `fa(3)` and the six bytes of `\{n}` for `fb(3)`.
#[test]
fn a_function_parameter_reaches_inside_a_string_literal() {
    builds("v_fn_str_body");
}

/// A function argument pasted into a body literal is the argument's VALUE,
/// written in DECIMAL. `v_fn_paste_value.lst`, exit 0, `fm function n,"n"`:
///
/// ```text
///        5/       0 : 2831 3229 EE        	dc.b fm(12),$EE
///        6/       5 : 2831 3629 EE        	dc.b fm($10),$EE
///        7/       A : 2833 29EE           	dc.b fm(1+2),$EE
///        8/       E : 282D 3529 EE        	dc.b fm(-5),$EE
///        9/      13 : 2833 3129 EE        	dc.b fm(n2),$EE
///       10/      18 : 2835 29EE           	dc.b fm(%101),$EE
///       11/      1C : 2832 3535 29EE      	dc.b fm(255),$EE
///       12/      22 : =$1F                 n2 equ $1F
/// ```
///
/// Pasting the argument's spelling gives `(1+2)` and `(n2)`, and pasting a
/// hex rendering gives `(10)` for `$10`. `n2` is defined BELOW its use, so the
/// value comes from a later pass.
#[test]
fn a_pasted_argument_is_its_decimal_value() {
    builds("v_fn_paste_value");
}

/// A parameter occurrence inside a literal is a run of LETTERS AND DIGITS, and
/// nothing else. `v_fn_word_edges.lst`, exit 0, each called with 3 (`fk` with
/// 1 and 2, `fl` with `n2`, `n2 equ 7`):
///
/// ```text
///        5/       0 : 612E 2833 2920      	dc.b fg(3),$EE      fg function n,"a.n n.b n"
///                 6 : 2833 292E 6220
///                 C : 2833 29EE
///        7/      10 : 2833 295F 3120      	dc.b fh(3),$EE      fh function n,"n_1 1n n1 _n"
///                16 : 316E 206E 3120
///                1C : 5F28 3329 EE
///        9/      21 : 4E2D 2833 29EE      	dc.b fi(3),$EE      fi function n,"N-n"
///       11/      27 : 2828 3329 29EE      	dc.b fj(3),$EE      fj function n,"(n)"
///       13/      2D : 2832 2920 2831      	dc.b fk(1,2),$EE    fk function n,m,"m n"
///                33 : 29EE
///       15/      35 : 3728 3729 EE        	dc.b fl(n2),$EE     fl function n,"\{n}n"
/// ```
///
/// `.` and `_` END a word here although both can spell an AS name: `a.n` is
/// `a.(3)` and `n_1` is `(3)_1`. A boundary that counts either of them as part
/// of a word leaves those rows alone. Case is exact under `-U` (`N-n`).
#[test]
fn a_parameter_word_is_letters_and_digits_only() {
    builds("v_fn_word_edges");
}

/// An argument with no integer to paste into a body literal is refused. A
/// STRING is an asl error there (`v_fn_paste_str`, `dc.b fm("ab")`, `error
/// #1020: invalid symbol name`, exit 2). A FLOAT asl pastes in a fixed exponent
/// form, `fm(2.5)` as `(2.5000000000000000E+00)` (`v_fn_paste_float`, exit 0);
/// sigil does not render that form, so it must either write asl's twenty-four
/// bytes or refuse, and never paste the float some other way.
///
/// The paste is textual right through an escape: `fq function n,"a\n b"` /
/// `dc.b fq(3)` makes `\(3)`, which asl refuses as `error #2010: invalid escape
/// sequence` (`v_fn_escape_letter`, exit 2). A scan that stepped over escape
/// sequences would leave `\n` alone and write a newline.
#[test]
fn a_string_or_float_argument_pasted_into_a_literal_is_refused() {
    refused("v_fn_paste_str");
    builds_or_refuses("v_fn_paste_float");
    refused("v_fn_escape_letter");
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
/// The first two rows carry no `+` at all. A data directive that does not fold
/// a computed string's interpolation writes the eleven source characters
/// `$\{abs(-5)}` for them, so the interpolation is guarded here in its own
/// right rather than left to `v_concat`'s coverage: `+` is not what it
/// depends on.
#[test]
fn an_interpolation_is_folded_in_a_computed_string() {
    builds("v_interp_substr");
    builds("v_concat");
}

/// A function whose body is a string reaches the bytes of a data directive as
/// that string. `v_fn_concat_const.lst` and `v_fn_interp_only.lst`, exit 0:
///
/// ```text
///        5/       0 : 6162                	dc.b f(1)      f function number,"a"+"b"
///        5/       0 : 2435                	dc.b f(-5)     f function number,"$\{abs(number)}"
/// ```
///
/// The arithmetic reading of the first gives the one byte `C3`, and a body
/// whose parameter is not bound inside its literal gives the literal's fifteen
/// source characters.
#[test]
fn a_string_valued_function_result_reaches_bytes() {
    builds("v_fn_concat_const");
    builds("v_fn_interp_only");
}

/// A function whose body is a ONE-character string, bound by `equ` and read in
/// integer slots, keeps its integer reading. `v_fn_str_equ_int.lst`, exit 0:
///
/// ```text
///        4/       0 :                     f function x,"a"
///        5/       0 : ="a"                 Z equ f(1)
///        6/       0 : 303C 0061           	move.w #Z,d0
///        7/       4 : 61EE                	dc.b Z,$EE
///        8/       6 : 62EE                	dc.b Z+1,$EE
/// ```
///
/// asl binds `Z` as the string `"a"` and reads it in an integer slot as its
/// packed code. sigil binds the call's value as an integer, which gives the
/// same bytes on every row. Binding it as a string gives a symbol that sigil's
/// integer paths do not read, so `move.w #Z,d0` and `dc.b Z+1` are refused:
/// that is the half-fix this test is here for.
#[test]
fn a_function_result_bound_by_equ_keeps_its_integer_reading() {
    builds("v_fn_str_equ_int");
}

/// A string-valued function bound by `set` and then emitted. asl binds the
/// string (`v_fn_full_str.lst`, exit 0):
///
/// ```text
///        5/       0 : ="-$5"               S set f(-5)
///        6/       0 : 2D24 35             	dc.b S
/// ```
///
/// sigil does not bind a call's string (see `a_function_result_bound_by_equ_
/// keeps_its_integer_reading` for why), so this shape is OPEN in the gap
/// ledger as `AS-STRING-FUNCTION-SET`. What this test holds is the direction
/// of the gap: sigil either writes asl's three bytes or refuses the file. Any
/// other image is a silent wrong answer and fails here, and building the
/// feature later turns this green without editing it.
#[test]
fn a_string_valued_function_bound_by_set_is_never_silently_wrong() {
    builds_or_refuses("v_fn_full_str");
}

/// A string VALUE is not scanned for `\{` a second time. `v_value_not_
/// rescanned.lst`, exit 0, `n equ 5` and `s := "\\{n}"` above these rows:
///
/// ```text
///        6/       0 : 5C7B 6E7D EE        	dc.b s,$EE
///        7/       5 : 5C7B 6E7D EE        	dc.b substr(s,0,0),$EE
///        8/       A : 2D5C 7B6E 7DEE      	dc.b "-"+s,$EE
///        9/      10 : 5C7B 6E7D EE        	dc.b substr("\\{n}",0,0),$EE
///       11/      15 : 5C7B 6E7D EE        	dc.b t,$EE          t set s
///       13/      1A : 5C7B 6E7D EE        	dc.b u,$EE          u set substr("\\{n}",0,0)
/// ```
///
/// `\\{n}` is an escaped backslash followed by `{n}`, and its value is those
/// four characters. A pass that folds interpolations over a string's VALUE
/// finds a `\{n}` in it and writes `35` in every row; so does a `set` that
/// folds over the value it binds.
#[test]
fn a_string_value_is_not_scanned_again_for_interpolation() {
    builds("v_value_not_rescanned");
}

/// A `\{expr}` is folded where its LITERAL is evaluated, before any string
/// operation runs on it. `v_interp_at_literal.lst`, exit 0, `n equ 5` and
/// `N equ $AB`:
///
/// ```text
///        6/       0 : 01EE                	dc.b strlen("\{n}"),$EE
///        7/       2 : 35EE                	dc.b substr("\{n}xy",0,1),$EE
///        8/       4 : 6162 EE             	dc.b lowstring("\{N}"),$EE
///        9/       7 : 41EE                	dc.b substr("ab\{N}",2,1),$EE
///       10/       9 : 03EE                	dc.b strlen("-"+"\{N}"),$EE
/// ```
///
/// Folding where the string lands instead gives `04`, `5C`, `35` (the
/// lowercased expression reads a different symbol, `n`), `5C` and `05`. The
/// `substr` rows use a nonzero length on purpose: `substr(s,0,0)` means "to the
/// end", which is the same under both readings. `v_interp_fwd` is the same
/// fold with the symbol defined BELOW its use, so a fold that only works on a
/// symbol already known is red there:
///
/// ```text
///        4/       0 : 35EE                	dc.b "\{Later}",$EE
///        5/       2 : 35EE                	dc.b substr("\{Later}",0,0),$EE
///        6/       4 : 2D35 EE             	dc.b "-"+"\{Later}",$EE
/// ```
#[test]
fn an_interpolation_is_folded_where_its_literal_is() {
    builds("v_interp_at_literal");
    builds("v_interp_fwd");
}

/// A quoted string inside `\{…}`, even one holding a `}`, belongs to the
/// interpolation's expression: it ends neither the interpolation nor the
/// literal around it. Three probes, each exit 0:
///
/// ```text
///        4/       0 : 32EE                	dc.b "\{strlen("ab")}",$EE
///        5/       2 : 34EE                	dc.b "\{strlen("ab"+"cd")}",$EE
///        4/       0 : 31EE                	dc.b "\{strlen("}")}",$EE
///        4/       0 : 33EE                	dc.b "\{strlen("a}b")}",$EE
/// ```
///
/// A literal scan that closes at the next quote cuts the first line into
/// `"\{strlen("`, `ab` and `")}"`. One that ends the interpolation at the
/// first `}` cuts the third and fourth lines inside `"}"` and `"a}b"`. The
/// second line is a concatenation inside an interpolation.
#[test]
fn a_quote_inside_an_interpolation_belongs_to_it() {
    builds("v_interp_nested_quote");
    builds("v_interp_brace_in_quote");
    builds("v_interp_brace_mid_quote");
}

/// A string-valued function called INSIDE an interpolation pastes its string.
/// This is the shape Sonic 1 itself uses `signedToString` in, inside `\{…}` in
/// `error`/`warning` text (`_Variables.asm` 430 and 486). `v_fn_in_interp.lst`,
/// exit 0:
///
/// ```text
///        5/       0 :                     	message "A\{signedToString(-5)}B"
///        6/       0 : 2D24 35EE           	dc.b "\{signedToString(-5)}",$EE
///        7/       4 : 2431 3233 EE        	dc.b "\{signedToString($123)}",$EE
/// ```
///
/// asl prints `A-$5B` for the `message`; the data rows go through the same
/// interpolation renderer and are what an image can show. A renderer that
/// probes for a string before expanding the call finds no string, and the
/// interpolation has no value.
#[test]
fn a_string_valued_function_pastes_its_string_into_an_interpolation() {
    builds("v_fn_in_interp");
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
/// with the `+` removed, so it exercises `substr` and `sgn` with no
/// concatenation anywhere:
///
/// ```text
///        5/       0 : 2D                  	dc.b signPrefix(-5)
///        6/       1 : 2D                  	dc.b signPrefix(0)
///        7/       2 : 2D24                	dc.b signPrefix(-$123),"$"
///        8/       4 : 2D78                	dc.b substr("-",0,-sgn(-5)),"x"
/// ```
///
/// A change to `eval_str` that taught it `+` at the cost of the plain `substr`
/// path is green everywhere else in this file and red only here.
#[test]
fn the_substr_and_sgn_half_assembles_without_concatenation() {
    builds("signed_sgn");
}
