//! A symbol spelled like a 68000 register (`A1:`, `sp equ 5`) can be DEFINED,
//! but an expression that names it reads the REGISTER, as asl does.
//!
//! ## asl's rule, measured
//!
//! Reference asl, md5 `61e672562465725a8c102288a7da9098`, through
//! `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run -xx -n -q -A -L -U
//! -i .`, over the 124 probes of
//! `docs/superpowers/notes/2026-09-25-as-register-spelled-label/` (the note beside
//! that directory tables every one):
//!
//! 1. On the 68000 a name spelled `d0`..`d7`, `a0`..`a7` or `sp`, in any case, is
//!    the register in every expression, whatever symbol of that name exists. The
//!    definition is accepted and `ifdef` / `defined()` see it; `#A1+2`,
//!    `#$$x-A1`, `A1(pc)`, `bra.w A1`, `org A1+8`, `if A1=$1200`, `X set A1+2`
//!    and the rest are refused (`#1145`, `#1146`, `#10000`).
//! 2. `usp`, `sr`, `ccr` and `pc` labels read as ordinary values, and so does
//!    any name that merely contains a register spelling (`A1x`, `A1.l`,
//!    `$$A1`). Under `cpu z80` no name is an expression register.
//! 3. A register where a whole operand stands is an addressing mode:
//!    `move.w A1,d0` is `3009` with `A1:` defined.
//!
//! ## Provenance of the bytes
//!
//! Every expected byte string below is asl's own image from a probe asl exited 0
//! on with no diagnostic at all, then its `p2bin -p=0`, as hex from `$1200`, where
//! every probe orgs. The probe sources are the `*_SRC` constants verbatim. A
//! refusal row quotes no asl byte: an asl run with any error is not a source of
//! values.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble `body` as `probe.asm` and hand back the image, or every diagnostic
/// as `(probe.asm(line), message)`.
fn assemble(body: &str) -> Result<Vec<u8>, Vec<(String, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let stubs = sigil_ir::SymbolTable::new();
            let located = |d: Vec<sigil_span::Diagnostic>| -> Vec<(String, String)> {
                d.into_iter().map(|d| ("<link>".to_string(), d.message)).collect()
            };
            let resolved = sigil_link::resolve_layout(&m.sections, &stubs, true).map_err(located)?;
            let linked = sigil_link::link(&resolved, &stubs).map_err(located)?;
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let label = match f.sources.label(d.primary) {
                    Some(full) => {
                        let base = full.rsplit_once('/').map_or(full.clone(), |(_, b)| b.to_string());
                        base.split_once(')').map_or(base.clone(), |(upto, _)| format!("{upto})"))
                    }
                    None => "<no source location>".to_string(),
                };
                (label, d.message.clone())
            })
            .collect()),
    }
}

/// Require asl's bytes from `$1200` and zeros below it.
fn assert_asl(what: &str, src: &str, asl_hex: &str) {
    let got = match assemble(src) {
        Ok(b) => b,
        Err(d) => panic!("{what}: expected asl's bytes {asl_hex}, refused: {d:?}"),
    };
    assert!(got.len() > 0x1200, "{what}: image ends at {:#x}, below $1200", got.len());
    assert!(got[..0x1200].iter().all(|&b| b == 0), "{what}: nonzero byte below $1200");
    let hex: String = got[0x1200..].iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, asl_hex, "{what}: sigil's image from $1200 differs from asl's");
}

/// THE sentence when a symbol of the register's spelling is defined, spelled out
/// rather than imported, because the text is the contract.
fn shadowed(name: &str) -> String {
    format!(
        "`{name}` is a register, not a value: expected an integer, floating point number or \
         string. A symbol named `{name}` is defined, but in an expression that spelling is the \
         68000 register, as it is to asl, so the symbol's value cannot be read here: rename the \
         symbol"
    )
}

/// Require a refusal carrying `shadowed(name)` on `line`, from the front end
/// (with a source line), never from the linker.
fn assert_shadowed(what: &str, src: &str, line: u32, name: &str) {
    let diags = match assemble(src) {
        Ok(b) => panic!("{what}: asl refuses this, sigil assembled {} bytes", b.len()),
        Err(d) => d,
    };
    let want = (format!("probe.asm({line})"), shadowed(name));
    assert!(diags.contains(&want), "{what}: want {want:?}, got {diags:?}");
}

const I09_SRC: &str = "\tcpu 68000\n\torg $1200\nA1:\tnop\n$$x:\tnop\n\tmove.w\t#$$x-A1,d0\n";

/// The booked shape. asl exit 3, `#10000 internal error`: the difference of a
/// label and a register. Before this, sigil read `A1` as the label and wrote
/// `303C 0002`.
#[test]
fn the_booked_i09_shape_is_refused_as_a_register() {
    assert_shadowed("i09", I09_SRC, 5, "A1");
}

/// Every consumer that reached a value for the label before, each one measured
/// refused by asl. A row per consumer, because the property is produced at one
/// place and consumed at many.
#[test]
fn every_consumer_of_a_value_reads_the_register() {
    let l = "\tcpu 68000\n\torg $1200\nA1:\tnop\nLab:\tnop\n";
    let rows: &[(&str, &str, u32, &str)] = &[
        ("imm", "\tmove.w\t#A1,d0\n", 5, "A1"),
        ("imm plus", "\tmove.w\t#A1+2,d0\n", 5, "A1"),
        ("imm difference", "\tmove.w\t#Lab-A1,d0\n", 5, "A1"),
        ("imm paren", "\tmove.w\t#(A1),d0\n", 5, "A1"),
        ("dc.w plus", "\tdc.w\tA1+2\n", 5, "A1"),
        ("dc.l difference", "\tdc.l\tLab-A1\n", 5, "A1"),
        ("dc.l with a forward label", "\tdc.l\tA1+Fwd\nFwd:\tnop\n", 5, "A1"),
        ("displacement", "\tmove.w\tA1(a0),d0\n", 5, "A1"),
        ("pc-relative", "\tlea\tA1(pc),a0\n", 5, "A1"),
        ("pc-relative indexed", "\tlea\tA1(pc,d0.w),a0\n", 5, "A1"),
        ("movem pc-relative", "\tmovem.l\tA1(pc),d0-d1\n", 5, "A1"),
        ("bra.w", "\tbra.w\tA1\n", 5, "A1"),
        ("dbf", "\tdbf\td0,A1\n", 5, "A1"),
        ("absolute plus", "\tmove.w\tA1+2,d0\n", 5, "A1"),
        ("jsr plus", "\tjsr\tA1+2\n", 5, "A1"),
        ("org", "\torg\tA1+8\n\tnop\n", 5, "A1"),
        ("set right-hand side", "X\tset\tA1+2\n\tdc.w\tX\n", 5, "A1"),
        ("if", "\tif A1=$1200\n\tnop\n\tendif\n", 5, "A1"),
    ];
    for (what, use_, line, name) in rows {
        assert_shadowed(what, &format!("{l}{use_}"), *line, name);
    }
}

/// The spellings and the case: `sp`, a data register, lower case, and a symbol
/// read in the other case from its definition. All asl `#1145`.
#[test]
fn every_register_spelling_in_either_case_reads_the_register() {
    for r in ["a1", "D0", "d0", "SP", "sp", "A7"] {
        let src = format!("\tcpu 68000\n\torg $1200\n{r}:\tnop\n\tmove.w\t#{r}+2,d0\n");
        assert_shadowed(r, &src, 4, r);
    }
    let src = "\tcpu 68000\n\torg $1200\nA1\tequ\t5\n\tmove.w\t#A1+2,d0\n";
    assert_shadowed("an equ, not a label", src, 4, "A1");
}

/// A symbol defined in a macro body and read in the same body, and one read
/// ABOVE its definition. asl `#1145` for both.
#[test]
fn a_body_label_and_a_forward_read_are_registers_too() {
    let body = "\tcpu 68000\n\torg $1200\nM\tmacro\nA1:\tnop\n\tmove.w\t#A1+2,d0\n\tendm\n\tM\n";
    let diags = assemble(body).expect_err("asl refuses a register read in a macro body");
    assert!(
        diags.iter().any(|(_, m)| m == &shadowed("A1")),
        "macro body: want the shadowed-register sentence, got {diags:?}"
    );
    let fwd = "\tcpu 68000\n\torg $1200\n\tmove.w\t#A1+2,d0\nA1:\tnop\n";
    let diags = assemble(fwd).expect_err("asl refuses a register read above the definition");
    assert!(
        diags.iter().any(|(l, m)| l == "probe.asm(3)" && m.starts_with("`A1` is a register, not a value")),
        "forward read: want the register sentence on line 3, got {diags:?}"
    );
}

/// THE ACCEPT SIDE, each with asl's own bytes: a definition nobody reads, the
/// bare register operand, the symbol-name tests, the four spellings asl does
/// not treat as registers, names that only contain one, and the Z80.
#[test]
fn what_asl_accepts_still_assembles_with_asl_bytes() {
    let h = "\tcpu 68000\n\torg $1200\n";
    let rows: &[(&str, String, &str)] = &[
        ("label defined, never read", format!("{h}A1:\tnop\n\tnop\n"), "4e714e71"),
        ("equ defined, never read", format!("{h}A1\tequ\t5\n\tnop\n"), "4e71"),
        ("bare operand is register direct", format!("{h}A1:\tnop\nLab:\tnop\n\tmove.w\tA1,d0\n"), "4e714e713009"),
        ("ifdef sees the symbol", format!("{h}A1:\tnop\nLab:\tnop\n\tifdef A1\n\tnop\n\tendif\n"), "4e714e714e71"),
        ("defined() sees the symbol", format!("{h}A1:\tnop\nLab:\tnop\n\tif defined(A1)\n\tnop\n\tendif\n"), "4e714e714e71"),
        ("usp label", format!("{h}USP:\tnop\n\tmove.w\t#USP+2,d0\n"), "4e71303c1202"),
        ("ccr label", format!("{h}CCR:\tnop\n\tmove.w\t#CCR+2,d0\n"), "4e71303c1202"),
        ("sr label", format!("{h}SR:\tnop\n\tdc.w\tSR+2\n"), "4e711202"),
        ("pc label", format!("{h}PC:\tnop\n\tdc.w\tPC+2\n"), "4e711202"),
        ("contains a register spelling", format!("{h}A1x:\tnop\n\tmove.w\t#A1x+2,d0\n\tdc.w\tA1x\n"), "4e71303c12021200"),
        ("local under a register-spelled label", format!("{h}A1:\tnop\n.l:\tnop\n\tmove.w\t#A1.l+2,d0\n\tdc.w\tA1.l\n"), "4e714e71303c12041202"),
        ("temp symbol spelled with a register", format!("{h}Lab:\tnop\n$$A1:\tnop\n\tmove.w\t#$$A1+2,d0\n\tdc.w\t$$A1\n"), "4e714e71303c12041202"),
        ("z80 hl label", "\tcpu z80\n\torg 1200h\nhl:\tnop\n\tdw\thl+2\n".to_string(), "000212"),
        ("z80 sp label", "\tcpu z80\n\torg 1200h\nsp:\tnop\n\tld\tde,sp+2\n".to_string(), "00110212"),
    ];
    for (what, src, hex) in rows {
        assert_asl(what, src, hex);
    }
}
