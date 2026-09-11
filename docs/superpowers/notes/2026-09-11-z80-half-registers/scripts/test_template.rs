//! The Z80's undocumented index-register halves under `cpu z80undoc`, against
//! the reference asl, probe by probe.
//!
//! Every expectation is asl's own answer: the table at the bottom is generated
//! (`gen_test.py`, beside the probes in
//! `docs/superpowers/notes/2026-09-11-z80-half-registers/`) from one probe file
//! per construct, each assembled by the pinned asl (md5
//! 61e672562465725a8c102288a7da9098) through `asl_run`. A probe's bytes are the
//! whole byte column of its listing, taken only from an exit-0 run with a
//! complete pass footer; a probe asl refused carries `None` and no bytes. Each
//! test assembles the probe's exact source through the front end and requires
//! the same image, or a refusal where asl refused.
//!
//! What the groups stand guard over, each a half-fix that would otherwise pass:
//!
//!   * `ixl`/`iyl` recognised without `ixu`/`iyu`/`ixh`/`iyh`, or the reverse:
//!     the spelling and `ld` groups name all four halves in every position;
//!   * the right register behind the wrong prefix (IX written, IY encoded):
//!     every expectation carries its `DD` or `FD`;
//!   * a half accepted where asl refuses it, above all beside `h` or `l`, where
//!     `ld ixl,h` would assemble silently as `ld ixl,ixu` (`DD 6C`): the forms
//!     group, and the dedicated refusal test that also checks the reason;
//!   * only `ld` taught: the arithmetic group runs all eight accumulator
//!     operations in both operand counts, and `inc`/`dec`;
//!   * the halves read as registers where asl reads a symbol, or the reverse:
//!     the symbol group defines a symbol of the half's name and asks each
//!     operand position;
//!   * the mode ignored or lost: the halves exist only under `z80undoc`, the
//!     mode survives `save`/`restore`, and `MOMCPU`/`MOMCPUNAME` report it.
//!
//! Probes outside this parcel, where sigil still differs from asl for reasons
//! that are not the halves, are left out of the table by name in `gen_test.py`
//! and listed in the note: `sll`, one-operand `add`/`adc`/`sbc` on a plain
//! register or immediate, upper-case documented registers, and a `save` with no
//! `restore`.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

/// Assemble one probe's source through parse, lower, link and flatten.
fn build(src: &str) -> Result<Vec<u8>, String> {
    let module = assemble(src, &Options::default()).map_err(|d| format!("{d:?}"))?;
    let linked =
        sigil_link::link(&module.sections, &SymbolTable::new()).map_err(|d| format!("{d:?}"))?;
    sigil_link::flatten(&linked, 0x00).map_err(|d| format!("{d:?}"))
}

/// The probe's name without its round (`m1/ld_a_ixl` is `ld_a_ixl`).
fn stem(name: &str) -> &str {
    name.split_once('/').map_or(name, |(_, s)| s)
}

/// Each group and the probe-name prefixes it owns. Every probe belongs to
/// exactly one group (`every_probe_is_checked_by_exactly_one_group`).
const GROUPS: &[(&str, &[&str])] = &[
    ("spellings", &["sp_", "spd_", "ctl_"]),
    ("ld", &["ld_", "ldi_", "ldineg_", "ldibig_", "ldisym_", "ldifwd_", "rng_"]),
    (
        "arithmetic",
        &["alu2_", "alu1_", "alud_", "alu_add_hl_", "alu1r_", "alu1i_", "alu2b_", "inc_", "dec_"],
    ),
    ("forms", &["cb_", "cbb_", "push_", "pop_", "in_", "out_", "ex_", "mix_", "nosym_"]),
    (
        "symbol",
        &[
            "jp_", "expr_", "exprl_", "paren_", "pareni_", "db_", "dbnosym_", "ldimm_", "label_",
            "plus_", "neg_", "undsym_", "usym_", "sym_", "lab_",
        ],
    ),
    ("mode", &["doc_", "docsym_", "mode_", "root68k_"]),
    ("momcpu", &["momcpu_", "momcpuname_", "momcpunamez_"]),
    ("sonic2", &["s2_"]),
];

fn group_of(name: &str) -> Vec<&'static str> {
    GROUPS
        .iter()
        .filter(|(_, prefixes)| prefixes.iter().any(|p| stem(name).starts_with(p)))
        .map(|(g, _)| *g)
        .collect()
}

/// Every probe of `group` assembles to asl's image, or is refused where asl
/// refused it. Reports every disagreement at once.
fn agree(group: &str) {
    let members: Vec<_> = PROBES.iter().filter(|(n, _, _)| group_of(n).contains(&group)).collect();
    assert!(!members.is_empty(), "group `{group}` has no probes, so it checks nothing");
    assert!(
        members.iter().any(|(_, _, want)| want.is_some()),
        "group `{group}` has no probe asl accepted, so no byte of it is checked"
    );
    let mut wrong = Vec::new();
    for (name, src, want) in &members {
        match (build(src), want) {
            (Ok(got), Some(w)) if got == *w => {}
            (Err(_), None) => {}
            (Ok(got), Some(w)) => wrong.push(format!("{name}: asl {w:02X?}, sigil {got:02X?}")),
            (Ok(got), None) => {
                wrong.push(format!("{name}: asl REFUSES, sigil assembled {got:02X?}"))
            }
            (Err(e), Some(w)) => wrong.push(format!("{name}: asl {w:02X?}, sigil refused: {e}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} `{group}` probes disagree with asl:\n{}",
        wrong.len(),
        members.len(),
        wrong.join("\n")
    );
}

/// `ixl`, `ixu`, `ixh`, `iyl`, `iyu`, `iyh` in any case, as a source and as a
/// destination; the near-miss spellings (`lx`, `xh`, `ixlo`, `ix.l`) are not
/// registers.
#[test]
fn every_spelling_asl_accepts_names_the_register_asl_names() {
    agree("spellings");
}

/// `ld` with a half in every position: from and to `a`..`e`, between the two
/// halves of one index register, and from an 8-bit immediate (a literal, a
/// negative, a character, a symbol, a forward reference, and the range edges).
#[test]
fn ld_takes_a_half_in_every_position_asl_allows() {
    agree("ld");
}

/// The eight accumulator operations with a half, in both operand counts
/// (one-operand `add`, `adc` and `sbc` refused), and `inc`/`dec`.
#[test]
fn the_arithmetic_and_logic_forms_take_a_half() {
    agree("arithmetic");
}

/// CB shifts and bit operations on a half, `push`/`pop`, `in`/`out`, `ex`,
/// 16-bit pairs, `i`/`r`, wrong operand counts: all refused, as asl refuses.
#[test]
fn a_half_in_a_form_asl_refuses_is_refused() {
    agree("forms");
}

/// With a symbol of the half's name defined, the bare name is still the
/// register in a register position (`ld b,ixl` is `DD 45`, not `ld b,5`), and
/// the symbol inside an expression, inside parentheses, as a bit number and as
/// a jump target.
#[test]
fn a_symbol_named_like_a_half_is_read_where_asl_reads_it() {
    agree("symbol");
}

/// The halves exist only under `z80undoc`: under plain `z80` the names are
/// symbols, and the mode follows `cpu` lines and `save`/`restore`, including
/// Sonic 2's shape (a 68000 root switching to `z80undoc` and back).
#[test]
fn the_halves_exist_only_under_z80undoc() {
    agree("mode");
}

/// `MOMCPU` is `$80DC` and `MOMCPUNAME` is `"Z80UNDOC"` under `z80undoc`,
/// whatever case the directive is written in.
#[test]
fn momcpu_and_momcpuname_report_the_undocumented_z80() {
    agree("momcpu");
}

/// The seven distinct forms of Sonic 2's twelve half-register lines.
#[test]
fn sonic_2_s_twelve_lines_assemble_as_asl_assembles_them() {
    agree("sonic2");
}

/// A half beside `h` or `l`, or beside a half of the other index register, is
/// refused, and refused for that reason, not for an unrelated one. The first
/// would otherwise encode silently as a different instruction: under the
/// prefix `h` and `l` ARE the halves, so `ld ixl,h` is `DD 6C`, `ld ixl,ixu`.
#[test]
fn a_half_beside_h_l_or_the_other_index_register_is_refused_for_that_reason() {
    const HALVES: [&str; 4] = ["ixl", "ixu", "iyl", "iyu"];
    let mut cases: Vec<(String, &str)> = Vec::new();
    for h in HALVES {
        for r in ["h", "l"] {
            cases.push((format!("m1/ld_{r}_{h}"), "index-register half"));
            cases.push((format!("m1/ld_{h}_{r}"), "index-register half"));
        }
        for h2 in HALVES {
            if h[..2] != h2[..2] {
                cases.push((format!("m1/ld_{h}_{h2}"), "different index registers"));
            }
        }
    }
    assert_eq!(cases.len(), 24, "16 h/l mixes and 8 cross-register pairs");
    let mut wrong = Vec::new();
    for (name, reason) in &cases {
        let (_, src, want) = PROBES
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("no probe {name} in the table"));
        assert!(want.is_none(), "{name}: the table says asl accepted it; this test is about refusals");
        match build(src) {
            Ok(got) => wrong.push(format!("{name}: sigil assembled {got:02X?}, asl refuses")),
            Err(e) if !e.contains(reason) => {
                wrong.push(format!("{name}: refused, but not because of the half ({reason}): {e}"))
            }
            Err(_) => {}
        }
    }
    assert!(wrong.is_empty(), "{} of {}:\n{}", wrong.len(), cases.len(), wrong.join("\n"));
}

/// No probe in the table goes unchecked, and none is checked twice.
#[test]
fn every_probe_is_checked_by_exactly_one_group() {
    let bad: Vec<String> = PROBES
        .iter()
        .filter(|(n, _, _)| group_of(n).len() != 1)
        .map(|(n, _, _)| format!("{n}: groups {:?}", group_of(n)))
        .collect();
    assert!(bad.is_empty(), "probes not in exactly one group:\n{}", bad.join("\n"));
}

/// (probe, its source, asl's image or `None` where asl refused). Generated from
/// the probe files and asl's listings; do not edit by hand.
#[rustfmt::skip]
const PROBES: &[(&str, &str, Option<&[u8]>)] = &[
// @PROBES@
];
