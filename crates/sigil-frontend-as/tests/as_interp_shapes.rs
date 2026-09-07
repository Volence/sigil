//! What `\{expr}` interpolation pastes, per operand SHAPE, against asl.
//!
//! Every expectation here is a line asl wrote to stdout for a `message`
//! carrying the same `\{...}`, three identical runs each, reference build md5
//! `61e672562465725a8c102288a7da9098`, probes `p1b`, `p2s`, `p3`, `p6b` and
//! `p7s` under `docs/superpowers/notes/2026-09-07-as-message-interp-probes/`.
//! The cells are read here through `warning`, which shares `interp_string`
//! with `message`, so the assertion is on the interpolated text and not on
//! which stream carries it (`as_message_stdout.rs` covers the stream).
//!
//! Three value types, three renderings:
//!
//! * an INTEGER pastes uppercase hex (`42` is `2A`), the rule
//!   `as_interp_radix.rs` pins;
//! * a FLOAT-typed expression pastes decimal through asl's own `FloatString`
//!   (`%.15e`, then cut to 18 characters, then written out when it fits), and
//!   an integral float has no point (`1024.0` is `1024`), which is what makes
//!   `\{x/1.0}` the corpus's spelling for "print this in decimal";
//! * a STRING symbol pastes its characters.
//!
//! The float rows include the cells where `FloatString` is not a `%g`: the
//! length cut drops digits BEFORE the last one and keeps it, so `1/7.0` is
//! `0.14285714285718`. Those rows are the ones that would pass a `%g` port
//! wrongly and are the reason the port is byte-for-byte.

use sigil_frontend_as::Options;

/// Assemble through the file entry point, expecting success, and hand back
/// each `warning` line's text with the `[as.warning] ` prefix removed.
fn warning_texts(src: &str) -> Vec<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, src).expect("write probe");
    let a = sigil_frontend_as::assemble_root_located_warned(&path, &Options::default())
        .unwrap_or_else(|f| {
            panic!(
                "expected a SUCCESSFUL assembly, got {:?}",
                f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            )
        });
    a.warnings
        .iter()
        .map(|d| {
            d.message
                .strip_prefix("[as.warning] ")
                .unwrap_or(&d.message)
                .to_string()
        })
        .collect()
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

/// One `warning` per row, in source order; the rows are (expression, asl's
/// rendering).
fn check(defs: &str, rows: &[(&str, &str)]) {
    let mut src = String::from(HEAD);
    src.push_str(defs);
    for (i, (expr, _)) in rows.iter().enumerate() {
        src.push_str(&format!("\twarning \"r{i} \\{{{expr}}}\"\n"));
    }
    src.push_str("\tdc.b 1\n\tend\n");
    let got = warning_texts(&src);
    let want: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(i, (_, out))| format!("r{i} {out}"))
        .collect();
    assert_eq!(got, want, "source:\n{src}");
}

#[test]
fn integer_shapes_paste_uppercase_hex() {
    check(
        "n equ 42\nh equ $FF\n",
        &[
            ("42", "2A"),
            ("$FF", "FF"),
            ("n", "2A"),
            ("(n+3)*2", "5A"),
            ("-1", "FFFFFFFFFFFFFFFF"),
            ("0", "0"),
            ("7/2", "3"),
            ("*", "0"),
            ("(*)&$FFFFFFFF", "0"),
            ("3<4", "1"),
        ],
    );
}

#[test]
fn two_interpolations_in_one_string() {
    check("n equ 42\nh equ $FF\n", &[("n} and \\{h", "2A and FF")]);
}

#[test]
fn a_float_division_pastes_decimal_and_an_integral_float_has_no_point() {
    check(
        "n equ 42\n",
        &[
            ("(2048-1024)/1024.0", "1"),
            ("1536/1024.0", "1.5"),
            ("n/1.0", "42"),
            ("(n)/1.0", "42"),
            ("$400/1024.0", "1"),
            ("1024.0", "1024"),
            ("3.5", "3.5"),
            ("2.5*4", "10"),
            ("0.0", "0"),
            ("-2.5", "-2.5"),
            ("-1024.0", "-1024"),
            ("0.1", "0.1"),
            ("0.1+0.2", "0.3"),
            ("123456789.5", "123456789.5"),
            ("1234.5678", "1234.5678"),
        ],
    );
}

#[test]
fn a_float_symbol_pastes_its_value() {
    check(
        "fs equ 2.5\nfi equ 1024.0\n",
        &[("fs", "2.5"), ("fi", "1024"), ("fs*2", "5"), ("fi/2", "512")],
    );
}

#[test]
fn a_comparison_over_floats_pastes_an_integer() {
    check("", &[("3.5<4", "1"), ("3.5>4", "0")]);
}

/// The rows a `%g`-style port renders differently: the 18-character cut keeps
/// the final rounded digit and removes the ones before it.
#[test]
fn a_long_float_is_cut_the_way_asl_cuts_it() {
    check(
        "",
        &[
            ("1/3.0", "0.33333333333333"),
            ("2/3.0", "0.66666666666666"),
            ("1/7.0", "0.14285714285718"),
            ("2/7.0", "0.28571428571427"),
            ("1/9.0", "0.11111111111111"),
            ("22/7.0", "3.142857142857143"),
            ("100.0/3", "33.3333333333334"),
            ("1000000.0/3", "333333.333333333"),
            ("3.14159265358979", "3.14159265358979"),
            ("2.718281828459045", "2.718281828459045"),
            ("9.99999999999999", "9.999999999999989"),
            ("0.99999999999999999", "1"),
            ("-1/3.0", "-0.3333333333333"),
            ("0.1+0.7", "0.79999999999999"),
            ("4.0/3", "1.333333333333333"),
            ("10.0/3", "3.333333333333333"),
            ("5.0/3", "1.666666666666667"),
            ("1.23456789012345", "1.23456789012345"),
            ("123456789.123456789", "123456789.123458"),
            ("0.1234567890123456789", "0.12345678901237"),
            ("123456789012345.678", "123456789012370"),
            ("1234567890123456.0", "1234567890123600"),
            ("12345678901234567.0", "12345678901237000"),
            ("123456789012345678.0", "123456789012370000"),
            ("12345678901234567890.0", "1.2345678901237E19"),
        ],
    );
}

/// The exponent survives exactly when the written-out number would not fit in
/// 18 characters, on either side of the point.
#[test]
fn a_float_keeps_its_exponent_only_when_written_out_it_would_not_fit() {
    check(
        "",
        &[
            ("1.0e14", "100000000000000"),
            ("1.0e15", "1000000000000000"),
            ("1.0e16", "10000000000000000"),
            ("1.0e17", "100000000000000000"),
            ("1.0e18", "1E18"),
            ("1.0e19", "1E19"),
            ("1.0e20", "1E20"),
            ("1.5e20", "1.5E20"),
            ("1.25e21", "1.25E21"),
            ("1.0e21", "1E21"),
            ("1.0e22", "1E22"),
            ("1.0e100", "1E100"),
            ("-1.0e21", "-1E21"),
            ("1.0e-4", "0.0001"),
            ("1.0e-5", "0.00001"),
            ("1.0e-6", "0.000001"),
            ("1.0e-7", "0.0000001"),
            ("1.5e-7", "0.00000015"),
            ("1.0e-8", "0.00000001"),
            ("1.5e-8", "0.000000015"),
            ("1.0e-9", "0.000000001"),
            ("1.0e-10", "0.0000000001"),
            ("-1.0e-10", "-0.0000000001"),
            ("1.0e-15", "0.000000000000001"),
            ("1.0e-20", "9.999999999999E-21"),
            ("0.01", "0.01"),
            ("0.001", "0.001"),
            ("0.0001", "0.0001"),
            ("0.00001", "0.00001"),
            ("1000000.0", "1000000"),
            ("10000000.0", "10000000"),
            ("123456.0", "123456"),
            ("1234567.0", "1234567"),
            ("12345678.0", "12345678"),
        ],
    );
}

#[test]
fn a_string_symbol_pastes_its_characters() {
    check("s equ \"abc\"\nt := \"xyz\"\n", &[("s", "abc"), ("t", "xyz")]);
}

/// The two corpus shapes that mix a string symbol with numeric
/// interpolations in one line (`s2.macros.asm(120)` and `(225)`), in asl's
/// words (probe `p3`).
#[test]
fn a_string_symbol_beside_numeric_interpolations() {
    let src = format!(
        "{HEAD}s equ \"abc\"\nn equ 42\n\
         \twarning \"soundBank \\{{s}} has $\\{{$8000+n-*}} bytes free at end.\"\n\
         \twarning \"Table \\{{s}} has \\{{n/1.0}} entries, but it should have \\{{(n)/1.0}} entries\"\n\
         \tdc.b 1\n\tend\n"
    );
    assert_eq!(
        warning_texts(&src),
        vec![
            "soundBank abc has $802A bytes free at end.".to_string(),
            "Table abc has 42 entries, but it should have 42 entries".to_string(),
        ]
    );
}

/// `\{$}` is the Z80 program counter under `cpu z80` (s1disasm's
/// `Uncompressed driver size: \{$}h bytes.`); under `cpu 68000` asl refuses
/// it as an invalid symbol name (probe `p1`), so the cell is Z80-only.
#[test]
fn the_z80_program_counter_interpolates_under_cpu_z80() {
    let src = "\tcpu z80\n\torg 0\n\tnop\n\tnop\n\tnop\n\twarning \"Uncompressed driver size: \\{$}h bytes.\"\n\tdb 1\n\tend\n";
    assert_eq!(warning_texts(src), vec!["Uncompressed driver size: 3h bytes.".to_string()]);
}

/// The corpus site that fires (`s2.asm(91272)`), in its own words, against
/// asl's pass-2 line for the same program (probe `p5b`).
#[test]
fn the_s2disasm_rom_size_line_renders_as_asl_renders_it() {
    let src = format!(
        "{HEAD}StartOfRom:\n\tdc.b 1\n\tdc.w EndOfRom-*\npaddingSoFar equ 3\n\
         \twarning \"ROM size is $\\{{EndOfRom-StartOfRom}} bytes (\\{{(EndOfRom-StartOfRom)/1024.0}} KiB). About $\\{{paddingSoFar}} bytes are padding. \"\n\
         \tdc.b 5,6\nEndOfRom:\n\tend\n"
    );
    assert_eq!(
        warning_texts(&src),
        vec!["ROM size is $5 bytes (0.0048828125 KiB). About $3 bytes are padding. ".to_string()]
    );
}

/// A string symbol pasted through a `\{}` nested inside `{}` name composition:
/// `zoneanimcount_{"\{zoneanimcur}"} = zoneanimcount-1` in `s2.macros.asm(246)`
/// and `sonic3k.macros.asm(160)`. Before the string branch existed the `\{}`
/// stayed verbatim, the composed name carried a backslash, and the line was an
/// `unexpected character` error at every expansion (11 on s2disasm, 24 on
/// skdisasm) where asl assembles it clean.
#[test]
fn a_string_symbol_composes_a_name_through_a_nested_interpolation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    let src = format!(
        "{HEAD}cur := \"abc\"\ncount_{{\"\\{{cur}}\"}} = 5\n\tdc.b count_abc\n\tend\n"
    );
    std::fs::write(&path, &src).expect("write probe");
    let a = sigil_frontend_as::assemble_root_located_warned(&path, &Options::default())
        .unwrap_or_else(|f| {
            panic!(
                "expected a SUCCESSFUL assembly, got {:?}",
                f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            )
        });
    let resolved =
        sigil_link::resolve_layout(&a.module.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(sigil_link::flatten(&linked, 0x00).unwrap(), vec![0x05]);
}
