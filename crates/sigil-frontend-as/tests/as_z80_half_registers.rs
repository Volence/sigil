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
    ("m1/sp_ixl", "\tcpu z80undoc\n\torg 0\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m1/sp_ixu", "\tcpu z80undoc\n\torg 0\n\tld a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/sp_ixh", "\tcpu z80undoc\n\torg 0\n\tld a,ixh\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/sp_iyl", "\tcpu z80undoc\n\torg 0\n\tld a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x7D, 0x00])),
    ("m1/sp_iyu", "\tcpu z80undoc\n\torg 0\n\tld a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/sp_iyh", "\tcpu z80undoc\n\torg 0\n\tld a,iyh\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/sp_IXL", "\tcpu z80undoc\n\torg 0\n\tld a,IXL\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m1/sp_IXU", "\tcpu z80undoc\n\torg 0\n\tld a,IXU\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/sp_IXH", "\tcpu z80undoc\n\torg 0\n\tld a,IXH\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/sp_IYL", "\tcpu z80undoc\n\torg 0\n\tld a,IYL\n\tnop\n\tend\n", Some(&[0xFD, 0x7D, 0x00])),
    ("m1/sp_IYU", "\tcpu z80undoc\n\torg 0\n\tld a,IYU\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/sp_IYH", "\tcpu z80undoc\n\torg 0\n\tld a,IYH\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/sp_IxL", "\tcpu z80undoc\n\torg 0\n\tld a,IxL\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m1/sp_iXu", "\tcpu z80undoc\n\torg 0\n\tld a,iXu\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/sp_Iyh", "\tcpu z80undoc\n\torg 0\n\tld a,Iyh\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/sp_lx", "\tcpu z80undoc\n\torg 0\n\tld a,lx\n\tnop\n\tend\n", None),
    ("m1/sp_hx", "\tcpu z80undoc\n\torg 0\n\tld a,hx\n\tnop\n\tend\n", None),
    ("m1/sp_xl", "\tcpu z80undoc\n\torg 0\n\tld a,xl\n\tnop\n\tend\n", None),
    ("m1/sp_xh", "\tcpu z80undoc\n\torg 0\n\tld a,xh\n\tnop\n\tend\n", None),
    ("m1/sp_ly", "\tcpu z80undoc\n\torg 0\n\tld a,ly\n\tnop\n\tend\n", None),
    ("m1/sp_hy", "\tcpu z80undoc\n\torg 0\n\tld a,hy\n\tnop\n\tend\n", None),
    ("m1/sp_yl", "\tcpu z80undoc\n\torg 0\n\tld a,yl\n\tnop\n\tend\n", None),
    ("m1/sp_yh", "\tcpu z80undoc\n\torg 0\n\tld a,yh\n\tnop\n\tend\n", None),
    ("m1/sp_ixlo", "\tcpu z80undoc\n\torg 0\n\tld a,ixlo\n\tnop\n\tend\n", None),
    ("m1/sp_ixhi", "\tcpu z80undoc\n\torg 0\n\tld a,ixhi\n\tnop\n\tend\n", None),
    ("m1/sp_ixdotl", "\tcpu z80undoc\n\torg 0\n\tld a,ix.l\n\tnop\n\tend\n", None),
    ("m1/sp_ixdoth", "\tcpu z80undoc\n\torg 0\n\tld a,ix.h\n\tnop\n\tend\n", None),
    ("m1/spd_ixh", "\tcpu z80undoc\n\torg 0\n\tld ixh,a\n\tnop\n\tend\n", Some(&[0xDD, 0x67, 0x00])),
    ("m1/spd_iyh", "\tcpu z80undoc\n\torg 0\n\tld iyh,a\n\tnop\n\tend\n", Some(&[0xFD, 0x67, 0x00])),
    ("m1/spd_IXH", "\tcpu z80undoc\n\torg 0\n\tld IXH,a\n\tnop\n\tend\n", Some(&[0xDD, 0x67, 0x00])),
    ("m1/doc_ixl", "\tcpu z80\n\torg 0\n\tld a,ixl\n\tnop\n\tend\n", None),
    ("m1/docsym_ixl", "\tcpu z80\n\torg 0\nixl equ 5\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m1/undsym_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m1/doc_ixu", "\tcpu z80\n\torg 0\n\tld a,ixu\n\tnop\n\tend\n", None),
    ("m1/docsym_ixu", "\tcpu z80\n\torg 0\nixu equ 5\n\tld a,ixu\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m1/undsym_ixu", "\tcpu z80undoc\n\torg 0\nixu equ 5\n\tld a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/doc_iyl", "\tcpu z80\n\torg 0\n\tld a,iyl\n\tnop\n\tend\n", None),
    ("m1/docsym_iyl", "\tcpu z80\n\torg 0\niyl equ 5\n\tld a,iyl\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m1/undsym_iyl", "\tcpu z80undoc\n\torg 0\niyl equ 5\n\tld a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x7D, 0x00])),
    ("m1/doc_iyu", "\tcpu z80\n\torg 0\n\tld a,iyu\n\tnop\n\tend\n", None),
    ("m1/docsym_iyu", "\tcpu z80\n\torg 0\niyu equ 5\n\tld a,iyu\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m1/undsym_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/doc_ixh", "\tcpu z80\n\torg 0\n\tld a,ixh\n\tnop\n\tend\n", None),
    ("m1/docsym_ixh", "\tcpu z80\n\torg 0\nixh equ 5\n\tld a,ixh\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m1/undsym_ixh", "\tcpu z80undoc\n\torg 0\nixh equ 5\n\tld a,ixh\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/ld_a_ixl", "\tcpu z80undoc\n\torg 0\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m1/ld_ixl_a", "\tcpu z80undoc\n\torg 0\n\tld ixl,a\n\tnop\n\tend\n", Some(&[0xDD, 0x6F, 0x00])),
    ("m1/ld_b_ixl", "\tcpu z80undoc\n\torg 0\n\tld b,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x45, 0x00])),
    ("m1/ld_ixl_b", "\tcpu z80undoc\n\torg 0\n\tld ixl,b\n\tnop\n\tend\n", Some(&[0xDD, 0x68, 0x00])),
    ("m1/ld_c_ixl", "\tcpu z80undoc\n\torg 0\n\tld c,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x4D, 0x00])),
    ("m1/ld_ixl_c", "\tcpu z80undoc\n\torg 0\n\tld ixl,c\n\tnop\n\tend\n", Some(&[0xDD, 0x69, 0x00])),
    ("m1/ld_d_ixl", "\tcpu z80undoc\n\torg 0\n\tld d,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x55, 0x00])),
    ("m1/ld_ixl_d", "\tcpu z80undoc\n\torg 0\n\tld ixl,d\n\tnop\n\tend\n", Some(&[0xDD, 0x6A, 0x00])),
    ("m1/ld_e_ixl", "\tcpu z80undoc\n\torg 0\n\tld e,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x5D, 0x00])),
    ("m1/ld_ixl_e", "\tcpu z80undoc\n\torg 0\n\tld ixl,e\n\tnop\n\tend\n", Some(&[0xDD, 0x6B, 0x00])),
    ("m1/ld_h_ixl", "\tcpu z80undoc\n\torg 0\n\tld h,ixl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_h", "\tcpu z80undoc\n\torg 0\n\tld ixl,h\n\tnop\n\tend\n", None),
    ("m1/ld_l_ixl", "\tcpu z80undoc\n\torg 0\n\tld l,ixl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_l", "\tcpu z80undoc\n\torg 0\n\tld ixl,l\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixl,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x6D, 0x00])),
    ("m1/ld_ixl_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixl,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x6C, 0x00])),
    ("m1/ld_ixl_iyl", "\tcpu z80undoc\n\torg 0\n\tld ixl,iyl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_iyu", "\tcpu z80undoc\n\torg 0\n\tld ixl,iyu\n\tnop\n\tend\n", None),
    ("m1/ldi_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixl,5\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x05, 0x00])),
    ("m1/ldineg_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixl,-1\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0xFF, 0x00])),
    ("m1/ldibig_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixl,256\n\tnop\n\tend\n", None),
    ("m1/ldisym_ixl", "\tcpu z80undoc\n\torg 0\nval equ 7Fh\n\tld ixl,val\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x7F, 0x00])),
    ("m1/ldifwd_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixl,fwd\nfwd equ 12h\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x12, 0x00])),
    ("m1/ld_ixl_indhl", "\tcpu z80undoc\n\torg 0\n\tld ixl,(hl)\n\tnop\n\tend\n", None),
    ("m1/ld_indhl_ixl", "\tcpu z80undoc\n\torg 0\n\tld (hl),ixl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_indix", "\tcpu z80undoc\n\torg 0\n\tld ixl,(ix+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indix_ixl", "\tcpu z80undoc\n\torg 0\n\tld (ix+1),ixl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_indiy", "\tcpu z80undoc\n\torg 0\n\tld ixl,(iy+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indiy_ixl", "\tcpu z80undoc\n\torg 0\n\tld (iy+1),ixl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_mem", "\tcpu z80undoc\n\torg 0\n\tld ixl,(1234h)\n\tnop\n\tend\n", None),
    ("m1/ld_mem_ixl", "\tcpu z80undoc\n\torg 0\n\tld (1234h),ixl\n\tnop\n\tend\n", None),
    ("m1/ld_ixl_i", "\tcpu z80undoc\n\torg 0\n\tld ixl,i\n\tnop\n\tend\n", None),
    ("m1/ld_i_ixl", "\tcpu z80undoc\n\torg 0\n\tld i,ixl\n\tnop\n\tend\n", None),
    ("m1/alu2_add_ixl", "\tcpu z80undoc\n\torg 0\n\tadd a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x85, 0x00])),
    ("m1/alu1_add_ixl", "\tcpu z80undoc\n\torg 0\n\tadd ixl\n\tnop\n\tend\n", None),
    ("m1/alu2_adc_ixl", "\tcpu z80undoc\n\torg 0\n\tadc a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x8D, 0x00])),
    ("m1/alu1_adc_ixl", "\tcpu z80undoc\n\torg 0\n\tadc ixl\n\tnop\n\tend\n", None),
    ("m1/alu2_sub_ixl", "\tcpu z80undoc\n\torg 0\n\tsub a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x95, 0x00])),
    ("m1/alu1_sub_ixl", "\tcpu z80undoc\n\torg 0\n\tsub ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x95, 0x00])),
    ("m1/alu2_sbc_ixl", "\tcpu z80undoc\n\torg 0\n\tsbc a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x9D, 0x00])),
    ("m1/alu1_sbc_ixl", "\tcpu z80undoc\n\torg 0\n\tsbc ixl\n\tnop\n\tend\n", None),
    ("m1/alu2_and_ixl", "\tcpu z80undoc\n\torg 0\n\tand a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xA5, 0x00])),
    ("m1/alu1_and_ixl", "\tcpu z80undoc\n\torg 0\n\tand ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xA5, 0x00])),
    ("m1/alu2_xor_ixl", "\tcpu z80undoc\n\torg 0\n\txor a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xAD, 0x00])),
    ("m1/alu1_xor_ixl", "\tcpu z80undoc\n\torg 0\n\txor ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xAD, 0x00])),
    ("m1/alu2_or_ixl", "\tcpu z80undoc\n\torg 0\n\tor a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xB5, 0x00])),
    ("m1/alu1_or_ixl", "\tcpu z80undoc\n\torg 0\n\tor ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xB5, 0x00])),
    ("m1/alu2_cp_ixl", "\tcpu z80undoc\n\torg 0\n\tcp a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xBD, 0x00])),
    ("m1/alu1_cp_ixl", "\tcpu z80undoc\n\torg 0\n\tcp ixl\n\tnop\n\tend\n", Some(&[0xDD, 0xBD, 0x00])),
    ("m1/alud_add_ixl_a", "\tcpu z80undoc\n\torg 0\n\tadd ixl,a\n\tnop\n\tend\n", None),
    ("m1/alud_add_b_ixl", "\tcpu z80undoc\n\torg 0\n\tadd b,ixl\n\tnop\n\tend\n", None),
    ("m1/alu_add_hl_ixl", "\tcpu z80undoc\n\torg 0\n\tadd hl,ixl\n\tnop\n\tend\n", None),
    ("m1/inc_ixl", "\tcpu z80undoc\n\torg 0\n\tinc ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x2C, 0x00])),
    ("m1/dec_ixl", "\tcpu z80undoc\n\torg 0\n\tdec ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x2D, 0x00])),
    ("m1/cb_rlc_ixl", "\tcpu z80undoc\n\torg 0\n\trlc ixl\n\tnop\n\tend\n", None),
    ("m1/cb_rrc_ixl", "\tcpu z80undoc\n\torg 0\n\trrc ixl\n\tnop\n\tend\n", None),
    ("m1/cb_rl_ixl", "\tcpu z80undoc\n\torg 0\n\trl ixl\n\tnop\n\tend\n", None),
    ("m1/cb_rr_ixl", "\tcpu z80undoc\n\torg 0\n\trr ixl\n\tnop\n\tend\n", None),
    ("m1/cb_sla_ixl", "\tcpu z80undoc\n\torg 0\n\tsla ixl\n\tnop\n\tend\n", None),
    ("m1/cb_sra_ixl", "\tcpu z80undoc\n\torg 0\n\tsra ixl\n\tnop\n\tend\n", None),
    ("m1/cb_srl_ixl", "\tcpu z80undoc\n\torg 0\n\tsrl ixl\n\tnop\n\tend\n", None),
    ("m1/cb_sll_ixl", "\tcpu z80undoc\n\torg 0\n\tsll ixl\n\tnop\n\tend\n", None),
    ("m1/cbb_bit_ixl", "\tcpu z80undoc\n\torg 0\n\tbit 0,ixl\n\tnop\n\tend\n", None),
    ("m1/cbb_res_ixl", "\tcpu z80undoc\n\torg 0\n\tres 0,ixl\n\tnop\n\tend\n", None),
    ("m1/cbb_set_ixl", "\tcpu z80undoc\n\torg 0\n\tset 0,ixl\n\tnop\n\tend\n", None),
    ("m1/push_ixl", "\tcpu z80undoc\n\torg 0\n\tpush ixl\n\tnop\n\tend\n", None),
    ("m1/pop_ixl", "\tcpu z80undoc\n\torg 0\n\tpop ixl\n\tnop\n\tend\n", None),
    ("m1/in_ixl", "\tcpu z80undoc\n\torg 0\n\tin ixl,(c)\n\tnop\n\tend\n", None),
    ("m1/out_ixl", "\tcpu z80undoc\n\torg 0\n\tout (c),ixl\n\tnop\n\tend\n", None),
    ("m1/jp_ixl", "\tcpu z80undoc\n\torg 0\n\tjp (ixl)\n\tnop\n\tend\n", None),
    ("m1/ex_ixl", "\tcpu z80undoc\n\torg 0\n\tex de,ixl\n\tnop\n\tend\n", None),
    ("m1/ld_a_ixu", "\tcpu z80undoc\n\torg 0\n\tld a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x7C, 0x00])),
    ("m1/ld_ixu_a", "\tcpu z80undoc\n\torg 0\n\tld ixu,a\n\tnop\n\tend\n", Some(&[0xDD, 0x67, 0x00])),
    ("m1/ld_b_ixu", "\tcpu z80undoc\n\torg 0\n\tld b,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x44, 0x00])),
    ("m1/ld_ixu_b", "\tcpu z80undoc\n\torg 0\n\tld ixu,b\n\tnop\n\tend\n", Some(&[0xDD, 0x60, 0x00])),
    ("m1/ld_c_ixu", "\tcpu z80undoc\n\torg 0\n\tld c,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x4C, 0x00])),
    ("m1/ld_ixu_c", "\tcpu z80undoc\n\torg 0\n\tld ixu,c\n\tnop\n\tend\n", Some(&[0xDD, 0x61, 0x00])),
    ("m1/ld_d_ixu", "\tcpu z80undoc\n\torg 0\n\tld d,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x54, 0x00])),
    ("m1/ld_ixu_d", "\tcpu z80undoc\n\torg 0\n\tld ixu,d\n\tnop\n\tend\n", Some(&[0xDD, 0x62, 0x00])),
    ("m1/ld_e_ixu", "\tcpu z80undoc\n\torg 0\n\tld e,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x5C, 0x00])),
    ("m1/ld_ixu_e", "\tcpu z80undoc\n\torg 0\n\tld ixu,e\n\tnop\n\tend\n", Some(&[0xDD, 0x63, 0x00])),
    ("m1/ld_h_ixu", "\tcpu z80undoc\n\torg 0\n\tld h,ixu\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_h", "\tcpu z80undoc\n\torg 0\n\tld ixu,h\n\tnop\n\tend\n", None),
    ("m1/ld_l_ixu", "\tcpu z80undoc\n\torg 0\n\tld l,ixu\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_l", "\tcpu z80undoc\n\torg 0\n\tld ixu,l\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixu,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x65, 0x00])),
    ("m1/ld_ixu_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixu,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x64, 0x00])),
    ("m1/ld_ixu_iyl", "\tcpu z80undoc\n\torg 0\n\tld ixu,iyl\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_iyu", "\tcpu z80undoc\n\torg 0\n\tld ixu,iyu\n\tnop\n\tend\n", None),
    ("m1/ldi_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixu,5\n\tnop\n\tend\n", Some(&[0xDD, 0x26, 0x05, 0x00])),
    ("m1/ldineg_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixu,-1\n\tnop\n\tend\n", Some(&[0xDD, 0x26, 0xFF, 0x00])),
    ("m1/ldibig_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixu,256\n\tnop\n\tend\n", None),
    ("m1/ldisym_ixu", "\tcpu z80undoc\n\torg 0\nval equ 7Fh\n\tld ixu,val\n\tnop\n\tend\n", Some(&[0xDD, 0x26, 0x7F, 0x00])),
    ("m1/ldifwd_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixu,fwd\nfwd equ 12h\n\tnop\n\tend\n", Some(&[0xDD, 0x26, 0x12, 0x00])),
    ("m1/ld_ixu_indhl", "\tcpu z80undoc\n\torg 0\n\tld ixu,(hl)\n\tnop\n\tend\n", None),
    ("m1/ld_indhl_ixu", "\tcpu z80undoc\n\torg 0\n\tld (hl),ixu\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_indix", "\tcpu z80undoc\n\torg 0\n\tld ixu,(ix+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indix_ixu", "\tcpu z80undoc\n\torg 0\n\tld (ix+1),ixu\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_indiy", "\tcpu z80undoc\n\torg 0\n\tld ixu,(iy+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indiy_ixu", "\tcpu z80undoc\n\torg 0\n\tld (iy+1),ixu\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_mem", "\tcpu z80undoc\n\torg 0\n\tld ixu,(1234h)\n\tnop\n\tend\n", None),
    ("m1/ld_mem_ixu", "\tcpu z80undoc\n\torg 0\n\tld (1234h),ixu\n\tnop\n\tend\n", None),
    ("m1/ld_ixu_i", "\tcpu z80undoc\n\torg 0\n\tld ixu,i\n\tnop\n\tend\n", None),
    ("m1/ld_i_ixu", "\tcpu z80undoc\n\torg 0\n\tld i,ixu\n\tnop\n\tend\n", None),
    ("m1/alu2_add_ixu", "\tcpu z80undoc\n\torg 0\n\tadd a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x84, 0x00])),
    ("m1/alu1_add_ixu", "\tcpu z80undoc\n\torg 0\n\tadd ixu\n\tnop\n\tend\n", None),
    ("m1/alu2_adc_ixu", "\tcpu z80undoc\n\torg 0\n\tadc a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x8C, 0x00])),
    ("m1/alu1_adc_ixu", "\tcpu z80undoc\n\torg 0\n\tadc ixu\n\tnop\n\tend\n", None),
    ("m1/alu2_sub_ixu", "\tcpu z80undoc\n\torg 0\n\tsub a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x94, 0x00])),
    ("m1/alu1_sub_ixu", "\tcpu z80undoc\n\torg 0\n\tsub ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x94, 0x00])),
    ("m1/alu2_sbc_ixu", "\tcpu z80undoc\n\torg 0\n\tsbc a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x9C, 0x00])),
    ("m1/alu1_sbc_ixu", "\tcpu z80undoc\n\torg 0\n\tsbc ixu\n\tnop\n\tend\n", None),
    ("m1/alu2_and_ixu", "\tcpu z80undoc\n\torg 0\n\tand a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xA4, 0x00])),
    ("m1/alu1_and_ixu", "\tcpu z80undoc\n\torg 0\n\tand ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xA4, 0x00])),
    ("m1/alu2_xor_ixu", "\tcpu z80undoc\n\torg 0\n\txor a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xAC, 0x00])),
    ("m1/alu1_xor_ixu", "\tcpu z80undoc\n\torg 0\n\txor ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xAC, 0x00])),
    ("m1/alu2_or_ixu", "\tcpu z80undoc\n\torg 0\n\tor a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xB4, 0x00])),
    ("m1/alu1_or_ixu", "\tcpu z80undoc\n\torg 0\n\tor ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xB4, 0x00])),
    ("m1/alu2_cp_ixu", "\tcpu z80undoc\n\torg 0\n\tcp a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xBC, 0x00])),
    ("m1/alu1_cp_ixu", "\tcpu z80undoc\n\torg 0\n\tcp ixu\n\tnop\n\tend\n", Some(&[0xDD, 0xBC, 0x00])),
    ("m1/alud_add_ixu_a", "\tcpu z80undoc\n\torg 0\n\tadd ixu,a\n\tnop\n\tend\n", None),
    ("m1/alud_add_b_ixu", "\tcpu z80undoc\n\torg 0\n\tadd b,ixu\n\tnop\n\tend\n", None),
    ("m1/alu_add_hl_ixu", "\tcpu z80undoc\n\torg 0\n\tadd hl,ixu\n\tnop\n\tend\n", None),
    ("m1/inc_ixu", "\tcpu z80undoc\n\torg 0\n\tinc ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x24, 0x00])),
    ("m1/dec_ixu", "\tcpu z80undoc\n\torg 0\n\tdec ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x25, 0x00])),
    ("m1/cb_rlc_ixu", "\tcpu z80undoc\n\torg 0\n\trlc ixu\n\tnop\n\tend\n", None),
    ("m1/cb_rrc_ixu", "\tcpu z80undoc\n\torg 0\n\trrc ixu\n\tnop\n\tend\n", None),
    ("m1/cb_rl_ixu", "\tcpu z80undoc\n\torg 0\n\trl ixu\n\tnop\n\tend\n", None),
    ("m1/cb_rr_ixu", "\tcpu z80undoc\n\torg 0\n\trr ixu\n\tnop\n\tend\n", None),
    ("m1/cb_sla_ixu", "\tcpu z80undoc\n\torg 0\n\tsla ixu\n\tnop\n\tend\n", None),
    ("m1/cb_sra_ixu", "\tcpu z80undoc\n\torg 0\n\tsra ixu\n\tnop\n\tend\n", None),
    ("m1/cb_srl_ixu", "\tcpu z80undoc\n\torg 0\n\tsrl ixu\n\tnop\n\tend\n", None),
    ("m1/cb_sll_ixu", "\tcpu z80undoc\n\torg 0\n\tsll ixu\n\tnop\n\tend\n", None),
    ("m1/cbb_bit_ixu", "\tcpu z80undoc\n\torg 0\n\tbit 0,ixu\n\tnop\n\tend\n", None),
    ("m1/cbb_res_ixu", "\tcpu z80undoc\n\torg 0\n\tres 0,ixu\n\tnop\n\tend\n", None),
    ("m1/cbb_set_ixu", "\tcpu z80undoc\n\torg 0\n\tset 0,ixu\n\tnop\n\tend\n", None),
    ("m1/push_ixu", "\tcpu z80undoc\n\torg 0\n\tpush ixu\n\tnop\n\tend\n", None),
    ("m1/pop_ixu", "\tcpu z80undoc\n\torg 0\n\tpop ixu\n\tnop\n\tend\n", None),
    ("m1/in_ixu", "\tcpu z80undoc\n\torg 0\n\tin ixu,(c)\n\tnop\n\tend\n", None),
    ("m1/out_ixu", "\tcpu z80undoc\n\torg 0\n\tout (c),ixu\n\tnop\n\tend\n", None),
    ("m1/jp_ixu", "\tcpu z80undoc\n\torg 0\n\tjp (ixu)\n\tnop\n\tend\n", None),
    ("m1/ex_ixu", "\tcpu z80undoc\n\torg 0\n\tex de,ixu\n\tnop\n\tend\n", None),
    ("m1/ld_a_iyl", "\tcpu z80undoc\n\torg 0\n\tld a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x7D, 0x00])),
    ("m1/ld_iyl_a", "\tcpu z80undoc\n\torg 0\n\tld iyl,a\n\tnop\n\tend\n", Some(&[0xFD, 0x6F, 0x00])),
    ("m1/ld_b_iyl", "\tcpu z80undoc\n\torg 0\n\tld b,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x45, 0x00])),
    ("m1/ld_iyl_b", "\tcpu z80undoc\n\torg 0\n\tld iyl,b\n\tnop\n\tend\n", Some(&[0xFD, 0x68, 0x00])),
    ("m1/ld_c_iyl", "\tcpu z80undoc\n\torg 0\n\tld c,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x4D, 0x00])),
    ("m1/ld_iyl_c", "\tcpu z80undoc\n\torg 0\n\tld iyl,c\n\tnop\n\tend\n", Some(&[0xFD, 0x69, 0x00])),
    ("m1/ld_d_iyl", "\tcpu z80undoc\n\torg 0\n\tld d,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x55, 0x00])),
    ("m1/ld_iyl_d", "\tcpu z80undoc\n\torg 0\n\tld iyl,d\n\tnop\n\tend\n", Some(&[0xFD, 0x6A, 0x00])),
    ("m1/ld_e_iyl", "\tcpu z80undoc\n\torg 0\n\tld e,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x5D, 0x00])),
    ("m1/ld_iyl_e", "\tcpu z80undoc\n\torg 0\n\tld iyl,e\n\tnop\n\tend\n", Some(&[0xFD, 0x6B, 0x00])),
    ("m1/ld_h_iyl", "\tcpu z80undoc\n\torg 0\n\tld h,iyl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_h", "\tcpu z80undoc\n\torg 0\n\tld iyl,h\n\tnop\n\tend\n", None),
    ("m1/ld_l_iyl", "\tcpu z80undoc\n\torg 0\n\tld l,iyl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_l", "\tcpu z80undoc\n\torg 0\n\tld iyl,l\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_ixl", "\tcpu z80undoc\n\torg 0\n\tld iyl,ixl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_ixu", "\tcpu z80undoc\n\torg 0\n\tld iyl,ixu\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyl,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x6D, 0x00])),
    ("m1/ld_iyl_iyu", "\tcpu z80undoc\n\torg 0\n\tld iyl,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x6C, 0x00])),
    ("m1/ldi_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyl,5\n\tnop\n\tend\n", Some(&[0xFD, 0x2E, 0x05, 0x00])),
    ("m1/ldineg_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyl,-1\n\tnop\n\tend\n", Some(&[0xFD, 0x2E, 0xFF, 0x00])),
    ("m1/ldibig_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyl,256\n\tnop\n\tend\n", None),
    ("m1/ldisym_iyl", "\tcpu z80undoc\n\torg 0\nval equ 7Fh\n\tld iyl,val\n\tnop\n\tend\n", Some(&[0xFD, 0x2E, 0x7F, 0x00])),
    ("m1/ldifwd_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyl,fwd\nfwd equ 12h\n\tnop\n\tend\n", Some(&[0xFD, 0x2E, 0x12, 0x00])),
    ("m1/ld_iyl_indhl", "\tcpu z80undoc\n\torg 0\n\tld iyl,(hl)\n\tnop\n\tend\n", None),
    ("m1/ld_indhl_iyl", "\tcpu z80undoc\n\torg 0\n\tld (hl),iyl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_indix", "\tcpu z80undoc\n\torg 0\n\tld iyl,(ix+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indix_iyl", "\tcpu z80undoc\n\torg 0\n\tld (ix+1),iyl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_indiy", "\tcpu z80undoc\n\torg 0\n\tld iyl,(iy+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indiy_iyl", "\tcpu z80undoc\n\torg 0\n\tld (iy+1),iyl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_mem", "\tcpu z80undoc\n\torg 0\n\tld iyl,(1234h)\n\tnop\n\tend\n", None),
    ("m1/ld_mem_iyl", "\tcpu z80undoc\n\torg 0\n\tld (1234h),iyl\n\tnop\n\tend\n", None),
    ("m1/ld_iyl_i", "\tcpu z80undoc\n\torg 0\n\tld iyl,i\n\tnop\n\tend\n", None),
    ("m1/ld_i_iyl", "\tcpu z80undoc\n\torg 0\n\tld i,iyl\n\tnop\n\tend\n", None),
    ("m1/alu2_add_iyl", "\tcpu z80undoc\n\torg 0\n\tadd a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x85, 0x00])),
    ("m1/alu1_add_iyl", "\tcpu z80undoc\n\torg 0\n\tadd iyl\n\tnop\n\tend\n", None),
    ("m1/alu2_adc_iyl", "\tcpu z80undoc\n\torg 0\n\tadc a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x8D, 0x00])),
    ("m1/alu1_adc_iyl", "\tcpu z80undoc\n\torg 0\n\tadc iyl\n\tnop\n\tend\n", None),
    ("m1/alu2_sub_iyl", "\tcpu z80undoc\n\torg 0\n\tsub a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x95, 0x00])),
    ("m1/alu1_sub_iyl", "\tcpu z80undoc\n\torg 0\n\tsub iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x95, 0x00])),
    ("m1/alu2_sbc_iyl", "\tcpu z80undoc\n\torg 0\n\tsbc a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x9D, 0x00])),
    ("m1/alu1_sbc_iyl", "\tcpu z80undoc\n\torg 0\n\tsbc iyl\n\tnop\n\tend\n", None),
    ("m1/alu2_and_iyl", "\tcpu z80undoc\n\torg 0\n\tand a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xA5, 0x00])),
    ("m1/alu1_and_iyl", "\tcpu z80undoc\n\torg 0\n\tand iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xA5, 0x00])),
    ("m1/alu2_xor_iyl", "\tcpu z80undoc\n\torg 0\n\txor a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xAD, 0x00])),
    ("m1/alu1_xor_iyl", "\tcpu z80undoc\n\torg 0\n\txor iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xAD, 0x00])),
    ("m1/alu2_or_iyl", "\tcpu z80undoc\n\torg 0\n\tor a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xB5, 0x00])),
    ("m1/alu1_or_iyl", "\tcpu z80undoc\n\torg 0\n\tor iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xB5, 0x00])),
    ("m1/alu2_cp_iyl", "\tcpu z80undoc\n\torg 0\n\tcp a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xBD, 0x00])),
    ("m1/alu1_cp_iyl", "\tcpu z80undoc\n\torg 0\n\tcp iyl\n\tnop\n\tend\n", Some(&[0xFD, 0xBD, 0x00])),
    ("m1/alud_add_iyl_a", "\tcpu z80undoc\n\torg 0\n\tadd iyl,a\n\tnop\n\tend\n", None),
    ("m1/alud_add_b_iyl", "\tcpu z80undoc\n\torg 0\n\tadd b,iyl\n\tnop\n\tend\n", None),
    ("m1/alu_add_hl_iyl", "\tcpu z80undoc\n\torg 0\n\tadd hl,iyl\n\tnop\n\tend\n", None),
    ("m1/inc_iyl", "\tcpu z80undoc\n\torg 0\n\tinc iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x2C, 0x00])),
    ("m1/dec_iyl", "\tcpu z80undoc\n\torg 0\n\tdec iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x2D, 0x00])),
    ("m1/cb_rlc_iyl", "\tcpu z80undoc\n\torg 0\n\trlc iyl\n\tnop\n\tend\n", None),
    ("m1/cb_rrc_iyl", "\tcpu z80undoc\n\torg 0\n\trrc iyl\n\tnop\n\tend\n", None),
    ("m1/cb_rl_iyl", "\tcpu z80undoc\n\torg 0\n\trl iyl\n\tnop\n\tend\n", None),
    ("m1/cb_rr_iyl", "\tcpu z80undoc\n\torg 0\n\trr iyl\n\tnop\n\tend\n", None),
    ("m1/cb_sla_iyl", "\tcpu z80undoc\n\torg 0\n\tsla iyl\n\tnop\n\tend\n", None),
    ("m1/cb_sra_iyl", "\tcpu z80undoc\n\torg 0\n\tsra iyl\n\tnop\n\tend\n", None),
    ("m1/cb_srl_iyl", "\tcpu z80undoc\n\torg 0\n\tsrl iyl\n\tnop\n\tend\n", None),
    ("m1/cb_sll_iyl", "\tcpu z80undoc\n\torg 0\n\tsll iyl\n\tnop\n\tend\n", None),
    ("m1/cbb_bit_iyl", "\tcpu z80undoc\n\torg 0\n\tbit 0,iyl\n\tnop\n\tend\n", None),
    ("m1/cbb_res_iyl", "\tcpu z80undoc\n\torg 0\n\tres 0,iyl\n\tnop\n\tend\n", None),
    ("m1/cbb_set_iyl", "\tcpu z80undoc\n\torg 0\n\tset 0,iyl\n\tnop\n\tend\n", None),
    ("m1/push_iyl", "\tcpu z80undoc\n\torg 0\n\tpush iyl\n\tnop\n\tend\n", None),
    ("m1/pop_iyl", "\tcpu z80undoc\n\torg 0\n\tpop iyl\n\tnop\n\tend\n", None),
    ("m1/in_iyl", "\tcpu z80undoc\n\torg 0\n\tin iyl,(c)\n\tnop\n\tend\n", None),
    ("m1/out_iyl", "\tcpu z80undoc\n\torg 0\n\tout (c),iyl\n\tnop\n\tend\n", None),
    ("m1/jp_iyl", "\tcpu z80undoc\n\torg 0\n\tjp (iyl)\n\tnop\n\tend\n", None),
    ("m1/ex_iyl", "\tcpu z80undoc\n\torg 0\n\tex de,iyl\n\tnop\n\tend\n", None),
    ("m1/ld_a_iyu", "\tcpu z80undoc\n\torg 0\n\tld a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m1/ld_iyu_a", "\tcpu z80undoc\n\torg 0\n\tld iyu,a\n\tnop\n\tend\n", Some(&[0xFD, 0x67, 0x00])),
    ("m1/ld_b_iyu", "\tcpu z80undoc\n\torg 0\n\tld b,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x44, 0x00])),
    ("m1/ld_iyu_b", "\tcpu z80undoc\n\torg 0\n\tld iyu,b\n\tnop\n\tend\n", Some(&[0xFD, 0x60, 0x00])),
    ("m1/ld_c_iyu", "\tcpu z80undoc\n\torg 0\n\tld c,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x4C, 0x00])),
    ("m1/ld_iyu_c", "\tcpu z80undoc\n\torg 0\n\tld iyu,c\n\tnop\n\tend\n", Some(&[0xFD, 0x61, 0x00])),
    ("m1/ld_d_iyu", "\tcpu z80undoc\n\torg 0\n\tld d,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x54, 0x00])),
    ("m1/ld_iyu_d", "\tcpu z80undoc\n\torg 0\n\tld iyu,d\n\tnop\n\tend\n", Some(&[0xFD, 0x62, 0x00])),
    ("m1/ld_e_iyu", "\tcpu z80undoc\n\torg 0\n\tld e,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x5C, 0x00])),
    ("m1/ld_iyu_e", "\tcpu z80undoc\n\torg 0\n\tld iyu,e\n\tnop\n\tend\n", Some(&[0xFD, 0x63, 0x00])),
    ("m1/ld_h_iyu", "\tcpu z80undoc\n\torg 0\n\tld h,iyu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_h", "\tcpu z80undoc\n\torg 0\n\tld iyu,h\n\tnop\n\tend\n", None),
    ("m1/ld_l_iyu", "\tcpu z80undoc\n\torg 0\n\tld l,iyu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_l", "\tcpu z80undoc\n\torg 0\n\tld iyu,l\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_ixl", "\tcpu z80undoc\n\torg 0\n\tld iyu,ixl\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_ixu", "\tcpu z80undoc\n\torg 0\n\tld iyu,ixu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyu,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x65, 0x00])),
    ("m1/ld_iyu_iyu", "\tcpu z80undoc\n\torg 0\n\tld iyu,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x64, 0x00])),
    ("m1/ldi_iyu", "\tcpu z80undoc\n\torg 0\n\tld iyu,5\n\tnop\n\tend\n", Some(&[0xFD, 0x26, 0x05, 0x00])),
    ("m1/ldineg_iyu", "\tcpu z80undoc\n\torg 0\n\tld iyu,-1\n\tnop\n\tend\n", Some(&[0xFD, 0x26, 0xFF, 0x00])),
    ("m1/ldibig_iyu", "\tcpu z80undoc\n\torg 0\n\tld iyu,256\n\tnop\n\tend\n", None),
    ("m1/ldisym_iyu", "\tcpu z80undoc\n\torg 0\nval equ 7Fh\n\tld iyu,val\n\tnop\n\tend\n", Some(&[0xFD, 0x26, 0x7F, 0x00])),
    ("m1/ldifwd_iyu", "\tcpu z80undoc\n\torg 0\n\tld iyu,fwd\nfwd equ 12h\n\tnop\n\tend\n", Some(&[0xFD, 0x26, 0x12, 0x00])),
    ("m1/ld_iyu_indhl", "\tcpu z80undoc\n\torg 0\n\tld iyu,(hl)\n\tnop\n\tend\n", None),
    ("m1/ld_indhl_iyu", "\tcpu z80undoc\n\torg 0\n\tld (hl),iyu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_indix", "\tcpu z80undoc\n\torg 0\n\tld iyu,(ix+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indix_iyu", "\tcpu z80undoc\n\torg 0\n\tld (ix+1),iyu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_indiy", "\tcpu z80undoc\n\torg 0\n\tld iyu,(iy+1)\n\tnop\n\tend\n", None),
    ("m1/ld_indiy_iyu", "\tcpu z80undoc\n\torg 0\n\tld (iy+1),iyu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_mem", "\tcpu z80undoc\n\torg 0\n\tld iyu,(1234h)\n\tnop\n\tend\n", None),
    ("m1/ld_mem_iyu", "\tcpu z80undoc\n\torg 0\n\tld (1234h),iyu\n\tnop\n\tend\n", None),
    ("m1/ld_iyu_i", "\tcpu z80undoc\n\torg 0\n\tld iyu,i\n\tnop\n\tend\n", None),
    ("m1/ld_i_iyu", "\tcpu z80undoc\n\torg 0\n\tld i,iyu\n\tnop\n\tend\n", None),
    ("m1/alu2_add_iyu", "\tcpu z80undoc\n\torg 0\n\tadd a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x84, 0x00])),
    ("m1/alu1_add_iyu", "\tcpu z80undoc\n\torg 0\n\tadd iyu\n\tnop\n\tend\n", None),
    ("m1/alu2_adc_iyu", "\tcpu z80undoc\n\torg 0\n\tadc a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x8C, 0x00])),
    ("m1/alu1_adc_iyu", "\tcpu z80undoc\n\torg 0\n\tadc iyu\n\tnop\n\tend\n", None),
    ("m1/alu2_sub_iyu", "\tcpu z80undoc\n\torg 0\n\tsub a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x94, 0x00])),
    ("m1/alu1_sub_iyu", "\tcpu z80undoc\n\torg 0\n\tsub iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x94, 0x00])),
    ("m1/alu2_sbc_iyu", "\tcpu z80undoc\n\torg 0\n\tsbc a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x9C, 0x00])),
    ("m1/alu1_sbc_iyu", "\tcpu z80undoc\n\torg 0\n\tsbc iyu\n\tnop\n\tend\n", None),
    ("m1/alu2_and_iyu", "\tcpu z80undoc\n\torg 0\n\tand a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xA4, 0x00])),
    ("m1/alu1_and_iyu", "\tcpu z80undoc\n\torg 0\n\tand iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xA4, 0x00])),
    ("m1/alu2_xor_iyu", "\tcpu z80undoc\n\torg 0\n\txor a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xAC, 0x00])),
    ("m1/alu1_xor_iyu", "\tcpu z80undoc\n\torg 0\n\txor iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xAC, 0x00])),
    ("m1/alu2_or_iyu", "\tcpu z80undoc\n\torg 0\n\tor a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xB4, 0x00])),
    ("m1/alu1_or_iyu", "\tcpu z80undoc\n\torg 0\n\tor iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xB4, 0x00])),
    ("m1/alu2_cp_iyu", "\tcpu z80undoc\n\torg 0\n\tcp a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xBC, 0x00])),
    ("m1/alu1_cp_iyu", "\tcpu z80undoc\n\torg 0\n\tcp iyu\n\tnop\n\tend\n", Some(&[0xFD, 0xBC, 0x00])),
    ("m1/alud_add_iyu_a", "\tcpu z80undoc\n\torg 0\n\tadd iyu,a\n\tnop\n\tend\n", None),
    ("m1/alud_add_b_iyu", "\tcpu z80undoc\n\torg 0\n\tadd b,iyu\n\tnop\n\tend\n", None),
    ("m1/alu_add_hl_iyu", "\tcpu z80undoc\n\torg 0\n\tadd hl,iyu\n\tnop\n\tend\n", None),
    ("m1/inc_iyu", "\tcpu z80undoc\n\torg 0\n\tinc iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x24, 0x00])),
    ("m1/dec_iyu", "\tcpu z80undoc\n\torg 0\n\tdec iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x25, 0x00])),
    ("m1/cb_rlc_iyu", "\tcpu z80undoc\n\torg 0\n\trlc iyu\n\tnop\n\tend\n", None),
    ("m1/cb_rrc_iyu", "\tcpu z80undoc\n\torg 0\n\trrc iyu\n\tnop\n\tend\n", None),
    ("m1/cb_rl_iyu", "\tcpu z80undoc\n\torg 0\n\trl iyu\n\tnop\n\tend\n", None),
    ("m1/cb_rr_iyu", "\tcpu z80undoc\n\torg 0\n\trr iyu\n\tnop\n\tend\n", None),
    ("m1/cb_sla_iyu", "\tcpu z80undoc\n\torg 0\n\tsla iyu\n\tnop\n\tend\n", None),
    ("m1/cb_sra_iyu", "\tcpu z80undoc\n\torg 0\n\tsra iyu\n\tnop\n\tend\n", None),
    ("m1/cb_srl_iyu", "\tcpu z80undoc\n\torg 0\n\tsrl iyu\n\tnop\n\tend\n", None),
    ("m1/cb_sll_iyu", "\tcpu z80undoc\n\torg 0\n\tsll iyu\n\tnop\n\tend\n", None),
    ("m1/cbb_bit_iyu", "\tcpu z80undoc\n\torg 0\n\tbit 0,iyu\n\tnop\n\tend\n", None),
    ("m1/cbb_res_iyu", "\tcpu z80undoc\n\torg 0\n\tres 0,iyu\n\tnop\n\tend\n", None),
    ("m1/cbb_set_iyu", "\tcpu z80undoc\n\torg 0\n\tset 0,iyu\n\tnop\n\tend\n", None),
    ("m1/push_iyu", "\tcpu z80undoc\n\torg 0\n\tpush iyu\n\tnop\n\tend\n", None),
    ("m1/pop_iyu", "\tcpu z80undoc\n\torg 0\n\tpop iyu\n\tnop\n\tend\n", None),
    ("m1/in_iyu", "\tcpu z80undoc\n\torg 0\n\tin iyu,(c)\n\tnop\n\tend\n", None),
    ("m1/out_iyu", "\tcpu z80undoc\n\torg 0\n\tout (c),iyu\n\tnop\n\tend\n", None),
    ("m1/jp_iyu", "\tcpu z80undoc\n\torg 0\n\tjp (iyu)\n\tnop\n\tend\n", None),
    ("m1/ex_iyu", "\tcpu z80undoc\n\torg 0\n\tex de,iyu\n\tnop\n\tend\n", None),
    ("m1/ctl_ld_a_l", "\tcpu z80undoc\n\torg 0\n\tld a,l\n\tnop\n\tend\n", Some(&[0x7D, 0x00])),
    ("m1/ctl_ld_a_h", "\tcpu z80undoc\n\torg 0\n\tld a,h\n\tnop\n\tend\n", Some(&[0x7C, 0x00])),
    ("m1/ctl_ld_l_a", "\tcpu z80undoc\n\torg 0\n\tld l,a\n\tnop\n\tend\n", Some(&[0x6F, 0x00])),
    ("m1/ctl_ld_h_l", "\tcpu z80undoc\n\torg 0\n\tld h,l\n\tnop\n\tend\n", Some(&[0x65, 0x00])),
    ("m1/ctl_ld_l_5", "\tcpu z80undoc\n\torg 0\n\tld l,5\n\tnop\n\tend\n", Some(&[0x2E, 0x05, 0x00])),
    ("m1/ctl_add_a_l", "\tcpu z80undoc\n\torg 0\n\tadd a,l\n\tnop\n\tend\n", Some(&[0x85, 0x00])),
    ("m1/ctl_inc_l", "\tcpu z80undoc\n\torg 0\n\tinc l\n\tnop\n\tend\n", Some(&[0x2C, 0x00])),
    ("m1/ctl_dec_h", "\tcpu z80undoc\n\torg 0\n\tdec h\n\tnop\n\tend\n", Some(&[0x25, 0x00])),
    ("m1/ctl_ld_a_ix_1_", "\tcpu z80undoc\n\torg 0\n\tld a,(ix+1)\n\tnop\n\tend\n", Some(&[0xDD, 0x7E, 0x01, 0x00])),
    ("m2/expr_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld a,ixl+1\n\tnop\n\tend\n", Some(&[0x3E, 0x06, 0x00])),
    ("m2/exprl_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld a,1+ixl\n\tnop\n\tend\n", Some(&[0x3E, 0x06, 0x00])),
    ("m2/paren_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 1234h\n\tld a,(ixl)\n\tnop\n\tend\n", Some(&[0x3A, 0x34, 0x12, 0x00])),
    ("m2/pareni_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld a,(ixl)+1\n\tnop\n\tend\n", Some(&[0x3E, 0x06, 0x00])),
    ("m2/db_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tdb ixl\n\tnop\n\tend\n", Some(&[0x05, 0x00])),
    ("m2/dbnosym_ixl", "\tcpu z80undoc\n\torg 0\n\tdb ixl\n\tnop\n\tend\n", None),
    ("m2/jp_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 1234h\n\tjp ixl\n\tnop\n\tend\n", Some(&[0xC3, 0x34, 0x12, 0x00])),
    ("m2/ldimm_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld b,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x45, 0x00])),
    ("m2/label_ixl", "\tcpu z80undoc\n\torg 0\n\tixl: nop\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0x00, 0xDD, 0x7D, 0x00])),
    ("m2/plus_ixl", "\tcpu z80undoc\n\torg 0\n\tld a,+ixl\n\tnop\n\tend\n", None),
    ("m2/neg_ixl", "\tcpu z80undoc\n\torg 0\n\tld a,-ixl\n\tnop\n\tend\n", None),
    ("m2/expr_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld a,iyu+1\n\tnop\n\tend\n", Some(&[0x3E, 0x06, 0x00])),
    ("m2/exprl_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld a,1+iyu\n\tnop\n\tend\n", Some(&[0x3E, 0x06, 0x00])),
    ("m2/paren_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 1234h\n\tld a,(iyu)\n\tnop\n\tend\n", Some(&[0x3A, 0x34, 0x12, 0x00])),
    ("m2/pareni_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld a,(iyu)+1\n\tnop\n\tend\n", Some(&[0x3E, 0x06, 0x00])),
    ("m2/db_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tdb iyu\n\tnop\n\tend\n", Some(&[0x05, 0x00])),
    ("m2/dbnosym_iyu", "\tcpu z80undoc\n\torg 0\n\tdb iyu\n\tnop\n\tend\n", None),
    ("m2/jp_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 1234h\n\tjp iyu\n\tnop\n\tend\n", Some(&[0xC3, 0x34, 0x12, 0x00])),
    ("m2/ldimm_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld b,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x44, 0x00])),
    ("m2/label_iyu", "\tcpu z80undoc\n\torg 0\n\tiyu: nop\n\tld a,iyu\n\tnop\n\tend\n", Some(&[0x00, 0xFD, 0x7C, 0x00])),
    ("m2/plus_iyu", "\tcpu z80undoc\n\torg 0\n\tld a,+iyu\n\tnop\n\tend\n", None),
    ("m2/neg_iyu", "\tcpu z80undoc\n\torg 0\n\tld a,-iyu\n\tnop\n\tend\n", None),
    ("m2/mode_save_z80_restore", "\tcpu z80undoc\n\torg 0\n\tsave\n\tcpu z80\n\trestore\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m2/mode_save_68k_restore", "\tcpu z80undoc\n\torg 0\n\tsave\n\tcpu 68000\n\trestore\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m2/mode_z80_then_undoc", "\tcpu z80\n\torg 0\n\tcpu z80undoc\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m2/mode_undoc_then_z80", "\tcpu z80undoc\n\torg 0\n\tcpu z80\n\tld a,ixl\n\tnop\n\tend\n", None),
    ("m2/mode_undoc_then_z80_sym", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tcpu z80\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m2/mode_z80_save_undoc_restore", "\tcpu z80\n\torg 0\nixl equ 5\n\tsave\n\tcpu z80undoc\n\trestore\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0x3E, 0x05, 0x00])),
    ("m2/mode_undoc_upper", "\tcpu Z80UNDOC\n\torg 0\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m2/mode_undoc_mixed", "\tcpu Z80UnDoc\n\torg 0\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m2/momcpu_z80", "\tcpu z80\n\torg 0\n\tdw MOMCPU\n\tnop\n\tend\n", Some(&[0x80, 0x00, 0x00])),
    ("m2/momcpuname_z80", "\tcpu z80\n\torg 0\n\tif MOMCPUNAME=\"Z80UNDOC\"\n\tdb 1\n\telse\n\tdb 2\n\tendif\n\tnop\n\tnop\n\tend\n", Some(&[0x02, 0x00, 0x00])),
    ("m2/momcpunamez_z80", "\tcpu z80\n\torg 0\n\tif MOMCPUNAME=\"Z80\"\n\tdb 1\n\telse\n\tdb 2\n\tendif\n\tnop\n\tnop\n\tend\n", Some(&[0x01, 0x00, 0x00])),
    ("m2/momcpu_z80undoc", "\tcpu z80undoc\n\torg 0\n\tdw MOMCPU\n\tnop\n\tend\n", Some(&[0xDC, 0x80, 0x00])),
    ("m2/momcpuname_z80undoc", "\tcpu z80undoc\n\torg 0\n\tif MOMCPUNAME=\"Z80UNDOC\"\n\tdb 1\n\telse\n\tdb 2\n\tendif\n\tnop\n\tnop\n\tend\n", Some(&[0x01, 0x00, 0x00])),
    ("m2/momcpunamez_z80undoc", "\tcpu z80undoc\n\torg 0\n\tif MOMCPUNAME=\"Z80\"\n\tdb 1\n\telse\n\tdb 2\n\tendif\n\tnop\n\tnop\n\tend\n", Some(&[0x02, 0x00, 0x00])),
    ("m2/momcpu_Z80UNDOCu", "\tcpu Z80UNDOC\n\torg 0\n\tdw MOMCPU\n\tnop\n\tend\n", Some(&[0xDC, 0x80, 0x00])),
    ("m2/momcpuname_Z80UNDOCu", "\tcpu Z80UNDOC\n\torg 0\n\tif MOMCPUNAME=\"Z80UNDOC\"\n\tdb 1\n\telse\n\tdb 2\n\tendif\n\tnop\n\tnop\n\tend\n", Some(&[0x01, 0x00, 0x00])),
    ("m2/momcpunamez_Z80UNDOCu", "\tcpu Z80UNDOC\n\torg 0\n\tif MOMCPUNAME=\"Z80\"\n\tdb 1\n\telse\n\tdb 2\n\tendif\n\tnop\n\tnop\n\tend\n", Some(&[0x02, 0x00, 0x00])),
    ("m2/alu2b_add_ixl", "\tcpu z80undoc\n\torg 0\n\tadd b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu2b_adc_ixl", "\tcpu z80undoc\n\torg 0\n\tadc b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu1r_sub", "\tcpu z80undoc\n\torg 0\n\tsub b\n\tnop\n\tend\n", Some(&[0x90, 0x00])),
    ("m2/alu1r_sub_z80", "\tcpu z80\n\torg 0\n\tsub b\n\tnop\n\tend\n", Some(&[0x90, 0x00])),
    ("m2/alu1i_sub", "\tcpu z80undoc\n\torg 0\n\tsub 5\n\tnop\n\tend\n", Some(&[0xD6, 0x05, 0x00])),
    ("m2/alu2b_sub_ixl", "\tcpu z80undoc\n\torg 0\n\tsub b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu2b_sbc_ixl", "\tcpu z80undoc\n\torg 0\n\tsbc b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu1r_and", "\tcpu z80undoc\n\torg 0\n\tand b\n\tnop\n\tend\n", Some(&[0xA0, 0x00])),
    ("m2/alu1r_and_z80", "\tcpu z80\n\torg 0\n\tand b\n\tnop\n\tend\n", Some(&[0xA0, 0x00])),
    ("m2/alu1i_and", "\tcpu z80undoc\n\torg 0\n\tand 5\n\tnop\n\tend\n", Some(&[0xE6, 0x05, 0x00])),
    ("m2/alu2b_and_ixl", "\tcpu z80undoc\n\torg 0\n\tand b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu1r_xor", "\tcpu z80undoc\n\torg 0\n\txor b\n\tnop\n\tend\n", Some(&[0xA8, 0x00])),
    ("m2/alu1r_xor_z80", "\tcpu z80\n\torg 0\n\txor b\n\tnop\n\tend\n", Some(&[0xA8, 0x00])),
    ("m2/alu1i_xor", "\tcpu z80undoc\n\torg 0\n\txor 5\n\tnop\n\tend\n", Some(&[0xEE, 0x05, 0x00])),
    ("m2/alu2b_xor_ixl", "\tcpu z80undoc\n\torg 0\n\txor b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu1r_or", "\tcpu z80undoc\n\torg 0\n\tor b\n\tnop\n\tend\n", Some(&[0xB0, 0x00])),
    ("m2/alu1r_or_z80", "\tcpu z80\n\torg 0\n\tor b\n\tnop\n\tend\n", Some(&[0xB0, 0x00])),
    ("m2/alu1i_or", "\tcpu z80undoc\n\torg 0\n\tor 5\n\tnop\n\tend\n", Some(&[0xF6, 0x05, 0x00])),
    ("m2/alu2b_or_ixl", "\tcpu z80undoc\n\torg 0\n\tor b,ixl\n\tnop\n\tend\n", None),
    ("m2/alu1r_cp", "\tcpu z80undoc\n\torg 0\n\tcp b\n\tnop\n\tend\n", Some(&[0xB8, 0x00])),
    ("m2/alu1r_cp_z80", "\tcpu z80\n\torg 0\n\tcp b\n\tnop\n\tend\n", Some(&[0xB8, 0x00])),
    ("m2/alu1i_cp", "\tcpu z80undoc\n\torg 0\n\tcp 5\n\tnop\n\tend\n", Some(&[0xFE, 0x05, 0x00])),
    ("m2/alu2b_cp_ixl", "\tcpu z80undoc\n\torg 0\n\tcp b,ixl\n\tnop\n\tend\n", None),
    ("m2/rng_ixl__128", "\tcpu z80undoc\n\torg 0\n\tld ixl,-128\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x80, 0x00])),
    ("m2/rng_ixl__129", "\tcpu z80undoc\n\torg 0\n\tld ixl,-129\n\tnop\n\tend\n", None),
    ("m2/rng_ixl_255", "\tcpu z80undoc\n\torg 0\n\tld ixl,255\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0xFF, 0x00])),
    ("m2/rng_ixl_0FFh", "\tcpu z80undoc\n\torg 0\n\tld ixl,0FFh\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0xFF, 0x00])),
    ("m2/rng_ixl__A_", "\tcpu z80undoc\n\torg 0\n\tld ixl,'A'\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x41, 0x00])),
    ("m2/rng_l__128", "\tcpu z80undoc\n\torg 0\n\tld l,-128\n\tnop\n\tend\n", Some(&[0x2E, 0x80, 0x00])),
    ("m2/rng_l__129", "\tcpu z80undoc\n\torg 0\n\tld l,-129\n\tnop\n\tend\n", None),
    ("m2/rng_l_255", "\tcpu z80undoc\n\torg 0\n\tld l,255\n\tnop\n\tend\n", Some(&[0x2E, 0xFF, 0x00])),
    ("m2/rng_l_0FFh", "\tcpu z80undoc\n\torg 0\n\tld l,0FFh\n\tnop\n\tend\n", Some(&[0x2E, 0xFF, 0x00])),
    ("m2/rng_l__A_", "\tcpu z80undoc\n\torg 0\n\tld l,'A'\n\tnop\n\tend\n", Some(&[0x2E, 0x41, 0x00])),
    ("m2/mix_ld_hl_ixl", "\tcpu z80undoc\n\torg 0\n\tld hl,ixl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_hl", "\tcpu z80undoc\n\torg 0\n\tld ixl,hl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ix_ixl", "\tcpu z80undoc\n\torg 0\n\tld ix,ixl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_ix", "\tcpu z80undoc\n\torg 0\n\tld ixl,ix\n\tnop\n\tend\n", None),
    ("m2/mix_ld_bc_ixl", "\tcpu z80undoc\n\torg 0\n\tld bc,ixl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_sp_ixl", "\tcpu z80undoc\n\torg 0\n\tld sp,ixl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_r", "\tcpu z80undoc\n\torg 0\n\tld ixl,r\n\tnop\n\tend\n", None),
    ("m2/mix_ld_r_ixl", "\tcpu z80undoc\n\torg 0\n\tld r,ixl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_af", "\tcpu z80undoc\n\torg 0\n\tld ixl,af\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_c_", "\tcpu z80undoc\n\torg 0\n\tld ixl,(c)\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_nz", "\tcpu z80undoc\n\torg 0\n\tld ixl,nz\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_a", "\tcpu z80undoc\n\torg 0\n\tld (ixl),a\n\tnop\n\tend\n", None),
    ("m2/mix_ld_a_ixl_", "\tcpu z80undoc\n\torg 0\n\tld a,(ixl)\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl", "\tcpu z80undoc\n\torg 0\n\tld ixl\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_a_b", "\tcpu z80undoc\n\torg 0\n\tld ixl,a,b\n\tnop\n\tend\n", None),
    ("m2/mix_inc_ixl_a", "\tcpu z80undoc\n\torg 0\n\tinc ixl,a\n\tnop\n\tend\n", None),
    ("m2/mix_cp_ixl_a", "\tcpu z80undoc\n\torg 0\n\tcp ixl,a\n\tnop\n\tend\n", None),
    ("m2/mix_ld_iyh_iyl", "\tcpu z80undoc\n\torg 0\n\tld iyh,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x65, 0x00])),
    ("m2/mix_ld_ixh_ixu", "\tcpu z80undoc\n\torg 0\n\tld ixh,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x64, 0x00])),
    ("m2/mix_ld_ixl_iyh", "\tcpu z80undoc\n\torg 0\n\tld ixl,iyh\n\tnop\n\tend\n", None),
    ("m2/mix_ld_ixl_H", "\tcpu z80undoc\n\torg 0\n\tld ixl,H\n\tnop\n\tend\n", None),
    ("m2/mix_ld_L_ixu", "\tcpu z80undoc\n\torg 0\n\tld L,ixu\n\tnop\n\tend\n", None),
    ("m3/sym_ldhl_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld hl,ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldbc_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld bc,ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldix_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld ix,ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldsp_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld sp,ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldindhl_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld (hl),ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldindix_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld (ix+1),ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldmem_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld (1234h),ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldmemd_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld ixl,(1234h)\n\tnop\n\tend\n", None),
    ("m3/sym_ldhlmem_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld hl,(ixl)\n\tnop\n\tend\n", Some(&[0x2A, 0x05, 0x00, 0x00])),
    ("m3/sym_ldixd_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld a,(ix+ixl)\n\tnop\n\tend\n", Some(&[0xDD, 0x7E, 0x05, 0x00])),
    ("m3/sym_ldixd2_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld (ixl+ix),a\n\tnop\n\tend\n", None),
    ("m3/sym_ldhexpr_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld ixl,ixl+1\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x06, 0x00])),
    ("m3/sym_ldhparen_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld ixl,(ixl)\n\tnop\n\tend\n", None),
    ("m3/sym_addhl_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tadd hl,ixl\n\tnop\n\tend\n", None),
    ("m3/sym_adda_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tadd a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x85, 0x00])),
    ("m3/sym_sub1_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tsub ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x95, 0x00])),
    ("m3/sym_inc_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tinc ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x2C, 0x00])),
    ("m3/sym_ldh5_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld ixl,5\n\tnop\n\tend\n", Some(&[0xDD, 0x2E, 0x05, 0x00])),
    ("m3/sym_push_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tpush ixl\n\tnop\n\tend\n", None),
    ("m3/sym_bit_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tbit ixl,b\n\tnop\n\tend\n", Some(&[0xCB, 0x68, 0x00])),
    ("m3/sym_res_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tres ixl,b\n\tnop\n\tend\n", Some(&[0xCB, 0xA8, 0x00])),
    ("m3/sym_set_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tset ixl,(hl)\n\tnop\n\tend\n", Some(&[0xCB, 0xEE, 0x00])),
    ("m3/sym_im_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tim ixl\n\tnop\n\tend\n", None),
    ("m3/sym_rst_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\trst ixl\n\tnop\n\tend\n", None),
    ("m3/sym_call_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tcall ixl\n\tnop\n\tend\n", Some(&[0xCD, 0x05, 0x00, 0x00])),
    ("m3/sym_jpnz_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tjp nz,ixl\n\tnop\n\tend\n", Some(&[0xC2, 0x05, 0x00, 0x00])),
    ("m3/sym_callnz_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tcall nz,ixl\n\tnop\n\tend\n", Some(&[0xC4, 0x05, 0x00, 0x00])),
    ("m3/sym_retnz_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tret ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ina_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tin a,(ixl)\n\tnop\n\tend\n", Some(&[0xDB, 0x05, 0x00])),
    ("m3/sym_outa_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tout (ixl),a\n\tnop\n\tend\n", Some(&[0xD3, 0x05, 0x00])),
    ("m3/sym_ldia_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld i,ixl\n\tnop\n\tend\n", None),
    ("m3/sym_exsp_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tex (sp),ixl\n\tnop\n\tend\n", None),
    ("m3/sym_ldab_ixl", "\tcpu z80undoc\n\torg 0\nixl equ 5\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m3/lab_jr_ixl", "\tcpu z80undoc\n\torg 0\n\tjr ixl\nixl:\n\tnop\n\tend\n", Some(&[0x18, 0x00, 0x00])),
    ("m3/lab_djnz_ixl", "\tcpu z80undoc\n\torg 0\n\tdjnz ixl\nixl:\n\tnop\n\tend\n", Some(&[0x10, 0x00, 0x00])),
    ("m3/lab_jp_ixl", "\tcpu z80undoc\n\torg 0\n\tjp ixl\nixl:\n\tnop\n\tend\n", Some(&[0xC3, 0x03, 0x00, 0x00])),
    ("m3/lab_jrz_ixl", "\tcpu z80undoc\n\torg 0\n\tjr z,ixl\nixl:\n\tnop\n\tend\n", Some(&[0x28, 0x00, 0x00])),
    ("m3/sym_ldhl_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld hl,iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldbc_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld bc,iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldix_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld ix,iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldsp_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld sp,iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldindhl_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld (hl),iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldindix_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld (ix+1),iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldmem_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld (1234h),iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldmemd_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld iyu,(1234h)\n\tnop\n\tend\n", None),
    ("m3/sym_ldhlmem_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld hl,(iyu)\n\tnop\n\tend\n", Some(&[0x2A, 0x05, 0x00, 0x00])),
    ("m3/sym_ldixd_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld a,(ix+iyu)\n\tnop\n\tend\n", Some(&[0xDD, 0x7E, 0x05, 0x00])),
    ("m3/sym_ldixd2_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld (iyu+ix),a\n\tnop\n\tend\n", None),
    ("m3/sym_ldhexpr_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld iyu,iyu+1\n\tnop\n\tend\n", Some(&[0xFD, 0x26, 0x06, 0x00])),
    ("m3/sym_ldhparen_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld iyu,(iyu)\n\tnop\n\tend\n", None),
    ("m3/sym_addhl_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tadd hl,iyu\n\tnop\n\tend\n", None),
    ("m3/sym_adda_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tadd a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x84, 0x00])),
    ("m3/sym_sub1_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tsub iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x94, 0x00])),
    ("m3/sym_inc_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tinc iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x24, 0x00])),
    ("m3/sym_ldh5_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld iyu,5\n\tnop\n\tend\n", Some(&[0xFD, 0x26, 0x05, 0x00])),
    ("m3/sym_push_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tpush iyu\n\tnop\n\tend\n", None),
    ("m3/sym_bit_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tbit iyu,b\n\tnop\n\tend\n", Some(&[0xCB, 0x68, 0x00])),
    ("m3/sym_res_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tres iyu,b\n\tnop\n\tend\n", Some(&[0xCB, 0xA8, 0x00])),
    ("m3/sym_set_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tset iyu,(hl)\n\tnop\n\tend\n", Some(&[0xCB, 0xEE, 0x00])),
    ("m3/sym_im_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tim iyu\n\tnop\n\tend\n", None),
    ("m3/sym_rst_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\trst iyu\n\tnop\n\tend\n", None),
    ("m3/sym_call_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tcall iyu\n\tnop\n\tend\n", Some(&[0xCD, 0x05, 0x00, 0x00])),
    ("m3/sym_jpnz_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tjp nz,iyu\n\tnop\n\tend\n", Some(&[0xC2, 0x05, 0x00, 0x00])),
    ("m3/sym_callnz_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tcall nz,iyu\n\tnop\n\tend\n", Some(&[0xC4, 0x05, 0x00, 0x00])),
    ("m3/sym_retnz_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tret iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ina_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tin a,(iyu)\n\tnop\n\tend\n", Some(&[0xDB, 0x05, 0x00])),
    ("m3/sym_outa_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tout (iyu),a\n\tnop\n\tend\n", Some(&[0xD3, 0x05, 0x00])),
    ("m3/sym_ldia_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld i,iyu\n\tnop\n\tend\n", None),
    ("m3/sym_exsp_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tex (sp),iyu\n\tnop\n\tend\n", None),
    ("m3/sym_ldab_iyu", "\tcpu z80undoc\n\torg 0\niyu equ 5\n\tld a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m3/lab_jr_iyu", "\tcpu z80undoc\n\torg 0\n\tjr iyu\niyu:\n\tnop\n\tend\n", Some(&[0x18, 0x00, 0x00])),
    ("m3/lab_djnz_iyu", "\tcpu z80undoc\n\torg 0\n\tdjnz iyu\niyu:\n\tnop\n\tend\n", Some(&[0x10, 0x00, 0x00])),
    ("m3/lab_jp_iyu", "\tcpu z80undoc\n\torg 0\n\tjp iyu\niyu:\n\tnop\n\tend\n", Some(&[0xC3, 0x03, 0x00, 0x00])),
    ("m3/lab_jrz_iyu", "\tcpu z80undoc\n\torg 0\n\tjr z,iyu\niyu:\n\tnop\n\tend\n", Some(&[0x28, 0x00, 0x00])),
    ("m3/usym_ldaIXL", "\tcpu z80undoc\n\torg 0\nIXL equ 5\n\tld a,IXL\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m3/usym_ldaIYU", "\tcpu z80undoc\n\torg 0\nIYU equ 5\n\tld a,IYU\n\tnop\n\tend\n", Some(&[0xFD, 0x7C, 0x00])),
    ("m3/usym_dbIXL", "\tcpu z80undoc\n\torg 0\nIXL equ 5\n\tdb IXL\n\tnop\n\tend\n", Some(&[0x05, 0x00])),
    ("m3/nosym_bit", "\tcpu z80undoc\n\torg 0\n\tbit ixl,b\n\tnop\n\tend\n", None),
    ("m3/nosym_im", "\tcpu z80undoc\n\torg 0\n\tim ixl\n\tnop\n\tend\n", None),
    ("m3/nosym_rst", "\tcpu z80undoc\n\torg 0\n\trst ixl\n\tnop\n\tend\n", None),
    ("m3/nosym_ldhl", "\tcpu z80undoc\n\torg 0\n\tld hl,ixl\n\tnop\n\tend\n", None),
    ("m3/nosym_ldindhl", "\tcpu z80undoc\n\torg 0\n\tld (hl),ixl\n\tnop\n\tend\n", None),
    ("m4/root68k_iyl", "\tcpu 68000\n\torg 0\n\tmove.w #$1234,d0\n\tsave\n\tcpu z80undoc\n\tld a,iyl\n\trestore\n\tmove.w #$5678,d1\n\tnop\n\tend\n", Some(&[0x30, 0x3C, 0x12, 0x34, 0xFD, 0x7D, 0x32, 0x3C, 0x56, 0x78, 0x4E, 0x71])),
    ("m4/root68k_after", "\tcpu 68000\n\torg 0\n\tsave\n\tcpu z80undoc\n\tnop\n\trestore\n\tsave\n\tcpu z80\n\tld a,ixl\n\trestore\n\tnop\n\tend\n", None),
    ("m4/root68k_after_sym", "\tcpu 68000\n\torg 0\nixl equ 5\n\tsave\n\tcpu z80undoc\n\tnop\n\trestore\n\tsave\n\tcpu z80\n\tld a,ixl\n\trestore\n\tnop\n\tend\n", Some(&[0x00, 0x3E, 0x05, 0x00, 0x4E, 0x71])),
    ("m4/s2_0", "\tcpu z80undoc\n\torg 0\n\tld a,iyl\n\tnop\n\tend\n", Some(&[0xFD, 0x7D, 0x00])),
    ("m4/s2_1", "\tcpu z80undoc\n\torg 0\n\tadc a,iyu\n\tnop\n\tend\n", Some(&[0xFD, 0x8C, 0x00])),
    ("m4/s2_2", "\tcpu z80undoc\n\torg 0\n\tld e,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x5D, 0x00])),
    ("m4/s2_3", "\tcpu z80undoc\n\torg 0\n\tld d,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x54, 0x00])),
    ("m4/s2_4", "\tcpu z80undoc\n\torg 0\n\tld a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x7D, 0x00])),
    ("m4/s2_5", "\tcpu z80undoc\n\torg 0\n\tadc a,ixu\n\tnop\n\tend\n", Some(&[0xDD, 0x8C, 0x00])),
    ("m4/s2_6", "\tcpu z80undoc\n\torg 0\n\tadd a,ixl\n\tnop\n\tend\n", Some(&[0xDD, 0x85, 0x00])),
];
