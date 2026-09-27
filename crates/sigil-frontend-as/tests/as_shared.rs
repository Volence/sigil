//! asl's `shared`, without a share file and with one.
//!
//! Sonic 2 ends with `shared movewZ80CompSize` (`s2.asm(91275)`). asl, run
//! with `-c`, writes that symbol to a C header its build script reads to patch
//! the sound driver's compressed size after `p2bin`. Without `-c` asl assembles
//! the line and says it did nothing. Without `Options::share_file` sigil is asl
//! without `-c`: the line is accepted with a warning on every line, never
//! dropped silently, and its operands are not evaluated. With it (sigil run as
//! `asl -c`), each name is recorded with its value at the line; the last two
//! tests below.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .` (no `-c`), one
//! construct per probe (`s7*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every one
//! exits 0 with `warning #30: no sharefile created, SHARED ignored`, once per
//! `shared` line, whatever its operands: a label, two labels, a `:=` variable,
//! an expression, a forward reference, an undefined name, or none at all.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | the line accepted silently | `every_shared_line_is_accepted_with_a_warning` |
//! | the line refused | `every_shared_line_is_accepted_with_a_warning` |
//! | the operands evaluated (an undefined name refused, which asl does not) | `the_operands_are_not_evaluated` |

use sigil_frontend_as::{assemble_root_located_warned, Options};
use sigil_span::Level;

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HEAD_Z80: &str = "\tcpu z80\n\torg 0\n";

/// Assemble; the image and the count of `shared` warnings.
fn assemble_with(head: &str, body: &str) -> Result<(Vec<u8>, usize), Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    let a = assemble_root_located_warned(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&a.module.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .map_err(|e| vec![format!("{e:?}")])?;
    let shared = a
        .warnings
        .iter()
        .filter(|d| d.level == Level::Warning && d.message.contains("share file"))
        .count();
    Ok((sigil_link::flatten(&linked, 0x00).unwrap(), shared))
}

/// Sonic 2's shape and asl's `s7_basic`: the bytes around the line are
/// untouched and there is exactly one warning. Two lines are two warnings.
#[test]
fn every_shared_line_is_accepted_with_a_warning() {
    assert_eq!(
        assemble_with(HEAD, "\tdc.w $1234\nFoo:\tmove.w #$0F64,d7\n\tshared Foo"),
        Ok((vec![0x12, 0x34, 0x3E, 0x3C, 0x0F, 0x64], 1))
    );
    assert_eq!(assemble_with(HEAD, "A:\tdc.b 1\n\tshared A\n\tshared A"), Ok((vec![0x01], 2)));
    assert_eq!(assemble_with(HEAD, "A:\tdc.b 1\nB:\tdc.b 2\n\tshared A,B"), Ok((vec![0x01, 0x02], 1)));
    assert_eq!(assemble_with(HEAD, "A:\tdc.b 1\n\tSHARED A"), Ok((vec![0x01], 1)));
    assert_eq!(assemble_with(HEAD_Z80, "A:\tdb 1\n\tshared A"), Ok((vec![0x01], 1)));
}

/// asl does not look at the operands without `-c`: an undefined name, an
/// expression, a forward reference, a variable and no operand at all are each
/// one warning and exit 0.
#[test]
fn the_operands_are_not_evaluated() {
    for body in [
        "\tshared Nope\n\tdc.b $EE",
        "A:\tdc.b 1\n\tshared A+1",
        "\tshared Later\nLater:\tdc.b 1",
        "V := 3\n\tshared V\n\tdc.b $EE",
        "\tshared\n\tdc.b $EE",
    ] {
        let got = assemble_with(HEAD, body);
        assert!(matches!(got, Ok((_, 1))), "asl accepts {body:?} with one warning; sigil: {got:?}");
    }
}

/// With a share file (asl's `-c`, `Options::share_file`): the names and their
/// values at the line, no warning. The values are the reference asl's own share
/// file for the same lines (probe `sh5`, md5 `61e672562465725a8c102288a7da9098`):
/// `A1 0x0` twice, `S1 0x3` though `S1` is rebound to 4 below the line, and
/// `Later 0x4`, a label defined after the line; a bare `shared` writes nothing.
/// `crates/sigil-cli/tests/asl_dropin.rs` compares the whole file with asl's.
#[test]
fn with_a_share_file_each_name_is_recorded_with_its_value_at_the_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(
        &path,
        "\tcpu 68000\n\tshared A1,A1\n\tshared\nA1:\tdc.w 1\nS1 set 3\n\tshared S1\nS1 set 4\n\tshared Later\n\tdc.w 0\nLater:\n",
    )
    .unwrap();
    let a = assemble_root_located_warned(&path, &Options { share_file: true, ..Options::default() })
        .unwrap_or_else(|f| panic!("{:?}", f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()));
    assert!(a.warnings.is_empty(), "{:?}", a.warnings);
    let got: Vec<(String, sigil_ir::expr::Expr)> = a.shared.into_iter().map(|s| (s.name, s.value)).collect();
    let int = sigil_ir::expr::Expr::Int;
    assert_eq!(
        got,
        vec![
            ("A1".to_string(), int(0)),
            ("A1".to_string(), int(0)),
            ("S1".to_string(), int(3)),
            ("Later".to_string(), int(4)),
        ]
    );
}

/// With a share file asl evaluates the operands, and refuses a name with no
/// value: `error #1010: symbol undefined` (probe `sh4`).
#[test]
fn with_a_share_file_an_undefined_name_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, "\tcpu 68000\n\tshared Nope\n\tdc.w 1\n").unwrap();
    let f = match assemble_root_located_warned(&path, &Options { share_file: true, ..Options::default() }) {
        Ok(_) => panic!("an undefined shared name was accepted"),
        Err(f) => f,
    };
    assert!(f.diags.iter().any(|d| d.message.contains("`shared Nope`: symbol undefined")), "{:?}", f.diags);
}
