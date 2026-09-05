//! The RADIX of `\{expr}` string interpolation, and the one place it must NOT
//! apply.
//!
//! asl renders an interpolated INTEGER as bare uppercase hexadecimal — no `$`,
//! no `0x`, no sign, no leading zeros — and it renders a NEGATIVE integer as its
//! 64-bit two's complement, so `-1` is sixteen `F`s rather than `-1`. Sigil
//! rendered decimal until this file existed, which is a divergence that reaches
//! ROM BYTES: `interp_text` also folds where a STRING SYMBOL is bound, so
//! `s := "\{n}"` then `dc.b s` emits the characters of the rendering.
//!
//! Expectations derived from asl 1.42 Beta [Bld 212]
//! (`s2disasm/build_tools/Linux-x86_64/asl`, md5
//! `0dee1f98e6480a4783d27ffd8b90896f`), probes committed under
//! `docs/superpowers/notes/2026-09-05-as-interp-radix-probes/` with each cell's
//! verbatim output and three-run stability.
//!
//! **THE DIGEST IS CITED BECAUSE THE VERSION STRING CANNOT IDENTIFY THE
//! BINARY**, and here that is not a formality: the build named above answers
//! differently on every run for any operand it declined to give a value, so
//! three identical runs from it were a statement about which operands happened
//! to resolve. `run.sh` now selects `61e672562465725a8c102288a7da9098` and
//! refuses anything else. All 13 probes were assembled under both builds and
//! their output is identical, so every row here stands.
//!
//! EVERY FIXTURE HERE USES A VALUE WHOSE HEX AND DECIMAL SPELLINGS DIFFER. The
//! reason this divergence survived for months is that every probe behind the
//! helper used a single-digit value, where the two renderings are the same
//! characters — a proof that could not fail. A one-digit fixture in this file is
//! a bug in the file.

use sigil_frontend_as::{assemble, Options};
use sigil_span::Diagnostic;

/// Assemble and flatten, for the cells whose evidence is BYTES.
fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default()).expect("assemble");
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Assemble through the FILE entry point, expecting success, and hand back the
/// warn-tier diagnostics — the only carrier for a `warning`'s rendered text.
fn warnings(src: &str) -> Vec<Diagnostic> {
    let dir = tmpdir();
    let path = dir.join("root.asm");
    std::fs::write(&path, src).expect("write probe");
    let a = sigil_frontend_as::assemble_root_located_warned(&path, &Options::default())
        .unwrap_or_else(|f| {
            panic!(
                "expected a SUCCESSFUL assembly, got {:?}",
                f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            )
        });
    std::fs::remove_dir_all(&dir).ok();
    a.warnings
}

/// Assemble, expecting REFUSAL, and hand back the diagnostic messages.
fn refusal(src: &str) -> Vec<String> {
    assemble(src, &Options::default())
        .err()
        .unwrap_or_else(|| panic!("expected a refusal, the source assembled"))
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// A per-test scratch directory, named with the clock and the thread id so two
/// tests running in parallel cannot land on the same path.
fn tmpdir() -> std::path::PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = std::env::temp_dir()
        .join("sigil_as_interp_radix")
        .join(format!("{n}-{:?}", std::thread::current().id()));
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n";

/// A POSITIVE integer renders as bare uppercase hex.
///
/// asl, probe `r1` (three identical runs): `v42 equ 42` / `message "d42=\{v42}"`
/// prints `d42=2A`, and alongside it `255` prints `FF`, `4095` prints `FFF`,
/// `10` prints `A` and `4660` prints `1234`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `42`/`255`/`4095`/`10`/`4660` if the
/// radix were decimal — which is what sigil emitted before this file — or
/// `$2A`/`0x2A`/`002A` if asl prefixed or padded. Each of the five values has a
/// hex spelling that differs from its decimal one in characters, length, or
/// both, and `4660`/`1234` is multi-digit in BOTH bases so a fixture that merely
/// looked "long enough" cannot pass by accident.
///
/// MUST FAIL on a decimal renderer, on a prefixed one, and on a zero-padded one.
#[test]
fn a_positive_integer_interpolates_as_bare_uppercase_hex() {
    for (value, rendered) in [
        ("42", "2A"),
        ("255", "FF"),
        ("4095", "FFF"),
        ("10", "A"),
        ("4660", "1234"),
    ] {
        let src = format!("{HEAD}v\tequ {value}\n\twarning \"v=\\{{v}}\"\n\tdc.b $11\n");
        let w = warnings(&src);
        assert_eq!(
            w[0].message,
            format!("[as.warning] v={rendered}"),
            "`{value}` must interpolate as `{rendered}`"
        );
    }
}

/// A NEGATIVE integer renders as its 64-BIT TWO'S COMPLEMENT in the same bare
/// uppercase hex — no minus sign, no truncation to the operand's width.
///
/// asl, probe `r2` (three identical runs):
///
/// ```text
/// m1=FFFFFFFFFFFFFFFF
/// m42=FFFFFFFFFFFFFFD6
/// m255=FFFFFFFFFFFFFF01
/// expr=FFFFFFFFFFFFFFFF
/// ```
///
/// from `n1 equ -1`, `n42 equ -42`, `n255 equ -255` and the literal expression
/// `\{0-1}`. Probe `r11` corroborates the WIDTH independently rather than by
/// reading the same digits again: `strlen("\{neg}")` with `neg := -1` is `10`
/// hex, i.e. sixteen characters.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `-1`/`-42` (a signed decimal),
/// `-1`/`-2A` (a signed hex), `FFFFFFFF` (32-bit two's complement — the width a
/// 68000 assembler is the likeliest to pick), or `FF` (the byte the value would
/// occupy). The probe distinguishes all four: only the 64-bit unsigned reading
/// produces sixteen digits, and only a two's-complement reading loses the sign.
///
/// MUST FAIL on a signed renderer, on a 32-bit-wide one, and on any width that
/// truncates.
#[test]
fn a_negative_integer_interpolates_as_64_bit_twos_complement_hex() {
    for (value, rendered) in [
        ("-1", "FFFFFFFFFFFFFFFF"),
        ("-42", "FFFFFFFFFFFFFFD6"),
        ("-255", "FFFFFFFFFFFFFF01"),
    ] {
        let src = format!("{HEAD}v\tequ {value}\n\twarning \"v=\\{{v}}\"\n\tdc.b $11\n");
        let w = warnings(&src);
        assert_eq!(
            w[0].message,
            format!("[as.warning] v={rendered}"),
            "`{value}` must interpolate as `{rendered}`"
        );
    }
    // The same value written as an EXPRESSION rather than read from a symbol, so
    // the rule is the renderer's and not the symbol table's.
    let w = warnings(&format!("{HEAD}\twarning \"e=\\{{0-1}}\"\n\tdc.b $11\n"));
    assert_eq!(w[0].message, "[as.warning] e=FFFFFFFFFFFFFFFF");
}

/// A value a `function` RETURNS obeys the same rule — the renderer sees an
/// integer and does not care where it came from.
///
/// asl, probe `r3` (three identical runs), with `twice function x,x*2` and
/// `add10 function x,x+10`:
///
/// ```text
/// f42=2A
/// f255=FF
/// fneg=FFFFFFFFFFFFFFD6
/// ```
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `42`/`255`/`-42` if a function
/// return were rendered decimal, which is the shape a "functions are a different
/// value channel" implementation would produce. `add10(245)` is deliberately an
/// ADDITION reaching 255 rather than a pass-through, so a function that silently
/// returned its argument would render `F5` and fail.
///
/// MUST FAIL if function returns are rendered on a different path from symbol
/// reads, and MUST FAIL if the negative case special-cases a sign back in.
#[test]
fn a_function_return_interpolates_in_the_same_hex() {
    let src = format!(
        "{HEAD}twice\tfunction x,x*2\nadd10\tfunction x,x+10\n\
         \twarning \"a=\\{{twice(21)}} b=\\{{add10(245)}} c=\\{{twice(0-21)}}\"\n\tdc.b $11\n"
    );
    let w = warnings(&src);
    assert_eq!(
        w[0].message, "[as.warning] a=2A b=FF c=FFFFFFFFFFFFFFD6",
        "a function return renders like any other integer"
    );
}

/// A LABEL's address interpolates in hex too, so the corpus's `$\{*}` idiom
/// ("print a `$` then the value") reads correctly.
///
/// asl, probe `r6` (three identical runs): a label 42 bytes into the section
/// prints `lbl=2A`, the location counter one byte later prints `pc=2B`, and
/// `here+here` prints `54`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `lbl=42`/`pc=43`/`sum=84` on a
/// decimal renderer. The `here+here` cell is there because `54` hex is `84`
/// decimal — a doubling that stays distinguishable rather than landing on a
/// palindrome.
///
/// MUST FAIL if addresses take a different rendering path from constants.
#[test]
fn a_label_address_interpolates_in_hex() {
    let src = format!(
        "{HEAD}\tdc.b $11\n\trept 41\n\tdc.b $22\n\tendm\nhere:\n\tdc.b $99\n\
         \twarning \"lbl=\\{{here}} pc=\\{{*}} sum=\\{{here+here}}\"\n"
    );
    let w = warnings(&src);
    assert_eq!(w[0].message, "[as.warning] lbl=2A pc=2B sum=54");
}

/// ZERO renders as one `0` — not `00`, not empty.
///
/// asl, probe `r12` (three identical runs): `z equ 0` prints `zero=0`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `0000000000000000`, if the
/// negative-value width were applied unconditionally rather than falling out of
/// the two's-complement bit pattern; or an empty string, if the renderer
/// suppressed leading zeros without a floor.
///
/// MUST FAIL on a fixed-width renderer.
#[test]
fn zero_interpolates_as_a_single_digit() {
    let w = warnings(&format!("{HEAD}z\tequ 0\n\twarning \"z=\\{{z}}\"\n\tdc.b $11\n"));
    assert_eq!(w[0].message, "[as.warning] z=0");
}

/// THE BYTE-REACHING CELL, and the reason this parcel had to prove ROM identity
/// rather than only compare message text. `interp_text` folds where a STRING
/// SYMBOL is BOUND, so the rendering becomes the symbol's characters and `dc.b`
/// emits them.
///
/// asl, probe `r11` (three identical runs): `n := 42`, `s := "\{n}"`, `dc.b s`
/// then `dc.b $ff` gives the image `32 41 ff` — the ASCII of `2A`. Its
/// `strlen(s)` is `2`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `34 32 ff` — the ASCII of `42` —
/// which is exactly what sigil emitted before this parcel. A one-digit value
/// would have emitted the same byte either way, which is how the divergence
/// stayed invisible.
///
/// MUST FAIL if the string-binding path renders on a different radix from the
/// message path.
#[test]
fn a_string_bound_from_an_interpolation_emits_the_hex_characters() {
    let src = format!("{HEAD}n\t:= 42\ns\t:= \"\\{{n}}\"\n\tdc.b s\n\tdc.b $ff\n");
    assert_eq!(
        bytes(&src),
        vec![0x32, 0x41, 0xff],
        "`s := \"\\{{n}}\"` with n = 42 must bind the characters `2A`, not `42`"
    );
}

/// The same for `equ`, which has its own string-binding branch beside `set`'s.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `34 32 ff`. The two branches are
/// separate code, so a fix applied to one and not the other passes the test
/// above and fails this one.
#[test]
fn an_equ_string_bound_from_an_interpolation_emits_the_hex_characters() {
    let src = format!("{HEAD}n\tequ 255\ns\tequ \"\\{{n}}\"\n\tdc.b s\n\tdc.b $ff\n");
    assert_eq!(
        bytes(&src),
        vec![0x46, 0x46, 0xff],
        "`s equ \"\\{{n}}\"` with n = 255 must bind the characters `FF`, not `255`"
    );
}

/// All four author-diagnostic directives share the helper, so all four move
/// together. `message` has no carrier a test can read, so it is asserted by the
/// one thing it does observably: it must not refuse.
///
/// asl, probe `r12` (three identical runs), `v equ 42`:
///
/// ```text
/// > > > r12.asm(7): warning: w=2A
/// > > > r12.asm(8): error: e=2A
/// > > > r12.asm(9): error: f=2A
/// ```
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `w=42`/`e=42`/`f=42`, if the fix
/// had been applied at one call site instead of in the shared helper.
///
/// MUST FAIL if `error` or `fatal` renders on a radix of its own.
#[test]
fn error_and_fatal_share_the_hex_rendering() {
    for directive in ["error", "fatal"] {
        let msgs = refusal(&format!(
            "{HEAD}v\tequ 42\n\t{directive} \"x=\\{{v}}\"\n\tdc.b $11\n"
        ));
        assert!(
            msgs.iter().any(|m| m.contains("x=2A")),
            "`{directive}` must render `2A`, got {msgs:?}"
        );
    }
}

/// THE PLACE THE HEX RULE MUST NOT REACH: `{expr}` symbol-name composition,
/// which is a DIFFERENT construct with a DIFFERENT radix. asl renders an integer
/// pasted into a NAME in DECIMAL, in the same source file where it renders the
/// same integer in a string in hex.
///
/// asl, probe `r9` (three identical runs): `n := 42` then `name_{n} equ $55`
/// defines `name_42` — reading it back through `\{name_42}` prints `55`, while
/// the earlier spelling of the probe read `name_2A` and got `symbol undefined`
/// plus exit 2.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `name_2A`, had the two constructs
/// shared a renderer — which is precisely the coupling a careless fix to
/// `interp_text` introduces, because the name path calls into it for its STRING
/// branch.
///
/// MUST FAIL if the integer name-composition path is routed through the string
/// interpolation renderer.
#[test]
fn symbol_name_composition_stays_decimal() {
    let src = format!("{HEAD}n\t:= 42\nname_{{n}}\tequ $55\n\tdc.b name_42\n");
    assert_eq!(
        bytes(&src),
        vec![0x55],
        "`name_{{n}}` with n = 42 must define `name_42`, not `name_2A`"
    );
}

/// …and the seam between the two: a `\{}` written INSIDE a string literal that
/// is itself a name-composition group renders in HEX, because it is the string
/// construct, nested.
///
/// asl, probe `r10` (three identical runs): `n := 42` then
/// `name_{"\{n}"} equ $55` defines `name_2A`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN? `name_42`, if the name path folded
/// nested interpolations on its own decimal rule instead of delegating to the
/// string renderer. The corpus depends on the distinction: `s2.macros.asm`
/// composes `zoneanimcount_{"\{zoneanimcur}"}` and `zone_id_{cur_zone_str}` with
/// `cur_zone_str := "\{cur_zone_id}"`, and the defining and reading sides only
/// agree because both go through the same renderer.
///
/// MUST FAIL if the name path stops delegating its string branch, which would
/// silently split the two sides of that corpus idiom apart at zone 10.
#[test]
fn a_nested_interpolation_inside_a_name_group_renders_in_hex() {
    let src = format!("{HEAD}n\t:= 42\nname_{{\"\\{{n}}\"}}\tequ $55\n\tdc.b name_2A\n");
    assert_eq!(
        bytes(&src),
        vec![0x55],
        "`name_{{\"\\{{n}}\"}}` with n = 42 must define `name_2A`"
    );
}
