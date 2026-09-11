//! asl's `shared`, without a share file.
//!
//! Sonic 2 ends with `shared movewZ80CompSize` (`s2.asm(91275)`). asl, run
//! with `-c`, writes that symbol to a C header its build script reads to patch
//! the sound driver's compressed size after `p2bin`. Without `-c` asl assembles
//! the line and says it did nothing. sigil writes no share file, so it is asl
//! without `-c`: the line is accepted with a warning on every line, never
//! dropped silently, and its operands are not evaluated.
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
