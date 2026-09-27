//! Everyday 68000 spellings a hack author writes in new code for a community
//! disassembly, which the reference asl assembles and sigil's AS route used to
//! refuse (or, for `swap.w`, accept where asl refuses).
//!
//! 1. The mnemonics `chk.w`, `illegal`, `link`, `unlk`, `reset`, `rtr`, `stop`,
//!    `trapv`.
//! 2. `.b` as the short branch suffix (`beq.b`, `bra.b`, `bsr.b`).
//! 3. `andi`/`ori`/`eori` to `ccr` and `sr` with no suffix (byte for `ccr`,
//!    word for `sr`), and with the suffix asl accepts.
//! 4. asl's operand-driven aliases: `add`/`sub` to an address register are
//!    `adda`/`suba`, `eor #imm,Dn` is `eori`.
//! 5. A `.w`/`.l` suffix on an absolute written without parentheses
//!    (`jmp $1234.w`), in either case.
//! 6. `binclude` with an unquoted path, and its offset and length operands.
//! 7. `swap` with a `.b`, `.w` or `.s` suffix is refused.
//!
//! ## Provenance
//!
//! Every expected byte string is the hex of asl's own image, never sigil's:
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run as
//!
//! ```text
//! . docs/superpowers/notes/asl-reference/asl_ref.sh
//! asl_run -xx -n -q -A -L -U -i .. <probe>.asm      # ASL_EXIT=0
//! p2bin <probe>.p <probe>.bin
//! ```
//!
//! by `run_asl.sh` in `docs/superpowers/notes/2026-09-27-as-author-forms-exact-probes/`,
//! whose `mkprobes.sh` writes each probe: the one form alone in a file under
//! `cpu 68000`. Each row below names its probe; its source line is that probe's
//! verbatim. A refusal row cites a run that exited 2 and reads only the
//! accept-or-refuse verdict from it, never a byte.
//!
//! ## Deliberate divergences from asl, named
//!
//! - `chk.l d0,d1` assembles under asl as `4300` (probe `chk_l`), which is the
//!   MC68020's long form and does not decode on a 68000. sigil refuses it.
//! - `chk d0,d1` (no suffix) assembles under asl as `4380` (probe `chk_bare`).
//!   sigil refuses it as it refuses unsuffixed `muls`/`divs`: the unsuffixed
//!   default is an open owner question for all of them together.

use sigil_frontend_as::{assemble_root_located, Options};

/// The files a `binclude` probe reads, as `mkprobes.sh` writes them.
const BLOBS: &[(&str, &[u8])] = &[
    ("blob.bin", &[0x12, 0x34, 0x56, 0x78]),
    ("sub/blob2.bin", &[0xAB, 0xCD]),
    ("MixCase.bin", &[0x5A]),
];

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("sub")).expect("sub dir");
    for (name, bytes) in BLOBS {
        std::fs::write(dir.path().join(name), bytes).expect("write blob");
    }
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    let m = match assemble_root_located(&path, &Options::default()) {
        Ok(m) => m,
        Err(f) => return Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    };
    let msgs =
        |d: Vec<sigil_span::Diagnostic>| d.iter().map(|d| d.message.clone()).collect::<Vec<_>>();
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(msgs)?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).map_err(msgs)?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

/// `cpu 68000` then the probe's lines, the shape `mkprobes.sh` writes.
fn probe(lines: &str) -> String {
    format!("\tcpu 68000\n{lines}\n")
}

/// Assemble each `(probe, source, asl hex)` row and require asl's bytes.
fn assert_rows(rows: &[(&str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, asl_hex) in rows {
        match assemble(&probe(src)) {
            Ok(b) => {
                let got: String = b.iter().map(|x| format!("{x:02X}")).collect();
                if got != *asl_hex {
                    bad.push(format!("{name}: asl {asl_hex}, sigil {got}"));
                }
            }
            Err(d) => bad.push(format!("{name}: asl {asl_hex}, sigil refused: {d:?}")),
        }
    }
    assert!(bad.is_empty(), "rows that differ from asl:\n{}", bad.join("\n"));
}

/// Assemble each `(probe, source, needle)` row and require a refusal whose text
/// contains the needle, so a neighbouring refusal path cannot stand in for it.
fn assert_refusals(rows: &[(&str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, needle) in rows {
        match assemble(&probe(src)) {
            Ok(b) => bad.push(format!("{name}: asl refuses, sigil emitted {b:02X?}")),
            Err(d) => {
                if !d.iter().any(|m| m.contains(needle)) {
                    bad.push(format!("{name}: refused, but no diagnostic names `{needle}`: {d:?}"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "rows asl refuses that sigil did not refuse as expected:\n{}", bad.join("\n"));
}

#[test]
fn the_eight_mnemonics_assemble_to_asls_bytes() {
    assert_rows(&[
        ("chk_w", "\tchk.w\td0,d1", "4380"),
        ("chk_w_ind", "\tchk.w\t(a0),d1", "4390"),
        ("chk_imm", "\tchk.w\t#$100,d1", "43BC0100"),
        ("illegal", "\tillegal", "4AFC"),
        ("link", "\tlink\ta6,#8", "4E560008"),
        ("link_neg", "\tlink\ta6,#-8", "4E56FFF8"),
        ("link_w", "\tlink.w\ta6,#-8", "4E56FFF8"),
        ("link_min", "\tlink\ta6,#-32768", "4E568000"),
        ("link_big", "\tlink\ta6,#32768", "4E568000"),
        ("link_ffff", "\tlink\ta6,#$FFFF", "4E56FFFF"),
        ("unlk", "\tunlk\ta6", "4E5E"),
        ("unlk_sp", "\tunlk\tsp", "4E5F"),
        ("reset", "\treset", "4E70"),
        ("rtr", "\trtr", "4E77"),
        ("stop", "\tstop\t#$2700", "4E722700"),
        ("stop_neg", "\tstop\t#-1", "4E72FFFF"),
        ("trapv", "\ttrapv", "4E76"),
    ]);
}

#[test]
fn the_eight_mnemonics_refuse_what_asl_refuses() {
    assert_refusals(&[
        // asl `#1130 invalid operand size`.
        ("chk_b", "\tchk.b\td0,d1", "only `chk.w`"),
        ("chk_s", "\tchk.s\td0,d1", "only `chk.w`"),
        ("link_b", "\tlink.b\ta6,#-8", "word only"),
        ("link_s", "\tlink.s\ta6,#-8", "word only"),
        ("unlk_w", "\tunlk.w\ta6", "no size field"),
        ("unlk_l", "\tunlk.l\ta6", "no size field"),
        // asl `#1500 instruction not supported on 68000`.
        ("link_l", "\tlink.l\ta6,#-8", "word only"),
        // asl `#1100 useless attribute`.
        ("reset_w", "\treset.w", "no size field"),
        ("rtr_w", "\trtr.w", "no size field"),
        ("rtr_l", "\trtr.l", "no size field"),
        ("trapv_w", "\ttrapv.w", "no size field"),
        ("illegal_w", "\tillegal.w", "no size field"),
        ("stop_w", "\tstop.w\t#$2700", "no size field"),
        ("stop_l", "\tstop.l\t#$2700", "no size field"),
        // asl `#1350 addressing mode not allowed here`.
        ("link_dn", "\tlink\td0,#8", "An,#imm"),
        ("unlk_dn", "\tunlk\td0", "An operand"),
        // asl `#1120 addressing mode must be immediate`.
        ("stop_dn", "\tstop\td0", "#imm"),
        // asl `#1320 range overflow`.
        ("stop_big", "\tstop\t#$12345", "out of range"),
        // asl `#1110 wrong number of operands`.
        ("rtr_op", "\trtr\td0", "0 operands"),
    ]);
}

/// `chk.l` (asl `4300`, the 68020's form) and unsuffixed `chk` (asl `4380`)
/// stay refused; see the module doc.
#[test]
fn chk_long_and_unsuffixed_chk_stay_refused() {
    assert_refusals(&[
        ("chk_l", "\tchk.l\td0,d1", "MC68020"),
        ("chk_bare", "\tchk\td0,d1", "explicit size suffix"),
    ]);
}

#[test]
fn dot_b_is_the_short_branch() {
    assert_rows(&[
        ("beq_b", "\tbeq.b\tt\n\tnop\nt:", "67024E71"),
        ("bra_b", "\tbra.b\tt\n\tnop\nt:", "60024E71"),
        ("bsr_b", "\tbsr.b\tt\n\tnop\nt:", "61024E71"),
        ("bra_b_back", "t:\tnop\n\tbra.b\tt", "4E7160FC"),
        // The `.s` spelling of the same probe, for the pair.
        ("beq_s", "\tbeq.s\tt\n\tnop\nt:", "67024E71"),
    ]);
    // asl `#1370 jump distance too big`.
    assert_refusals(&[("bra_b_far", "\tbra.b\tt\n\tds.b\t200\nt:\tnop", "out of range")]);
}

#[test]
fn immediates_to_ccr_and_sr_take_their_one_size() {
    assert_rows(&[
        ("andi_ccr", "\tandi\t#$FE,ccr", "023C00FE"),
        ("ori_ccr", "\tori\t#1,ccr", "003C0001"),
        ("eori_ccr", "\teori\t#1,ccr", "0A3C0001"),
        ("andi_sr", "\tandi\t#$F8FF,sr", "027CF8FF"),
        ("ori_sr", "\tori\t#$0700,sr", "007C0700"),
        ("eori_sr", "\teori\t#$2000,sr", "0A7C2000"),
        ("andi_sr_ff", "\tandi\t#$FF,sr", "027C00FF"),
        ("andi_w_sr_ff", "\tandi.w\t#$FF,sr", "027C00FF"),
        ("andi_b_ccr", "\tandi.b\t#$FE,ccr", "023C00FE"),
        ("eori_b_ccr", "\teori.b\t#1,ccr", "0A3C0001"),
        ("andi_w_sr", "\tandi.w\t#$F8FF,sr", "027CF8FF"),
        ("ori_w_sr", "\tori.w\t#$0700,sr", "007C0700"),
        ("eori_w_sr", "\teori.w\t#$2000,sr", "0A7C2000"),
        ("andi_ccr_up", "\tANDI\t#$FE,CCR", "023C00FE"),
    ]);
    assert_refusals(&[
        // asl `#1130 invalid operand size`.
        ("andi_w_ccr", "\tandi.w\t#$FE,ccr", "byte only"),
        ("andi_w_ccr_up", "\tANDI.W\t#$FE,CCR", "byte only"),
        ("eori_l_ccr", "\teori.l\t#1,ccr", "byte only"),
        ("andi_b_sr", "\tandi.b\t#$FF,sr", "word only"),
        ("andi_l_sr", "\tandi.l\t#$FF,sr", "word only"),
        ("ori_b_sr", "\tori.b\t#1,sr", "word only"),
        // asl `#1320 range overflow`.
        ("andi_ccr_big", "\tandi\t#$1FE,ccr", "out of range"),
        ("andi_sr_big", "\tandi\t#$1FFFF,sr", "out of range"),
    ]);
}

#[test]
fn add_sub_to_an_address_register_and_eor_immediate_are_asls_aliases() {
    assert_rows(&[
        ("add_w_dn_an", "\tadd.w\td0,a1", "D2C0"),
        ("add_l_dn_an", "\tadd.l\td0,a1", "D3C0"),
        ("sub_w_dn_an", "\tsub.w\td0,a1", "92C0"),
        ("sub_l_dn_an", "\tsub.l\td0,a1", "93C0"),
        ("add_w_mem_an", "\tadd.w\t(a0),a1", "D2D0"),
        ("add_w_imm_an", "\tadd.w\t#1,a1", "D2FC0001"),
        ("add_w_an_an", "\tadd.w\ta0,a1", "D2C8"),
        ("add_w_dn_sp", "\tadd.w\td0,sp", "DEC0"),
        ("sub_w_imm_an", "\tsub.w\t#1,a1", "92FC0001"),
        ("eor_w_imm_dn", "\teor.w\t#1,d0", "0A400001"),
        ("eor_b_imm_dn", "\teor.b\t#1,d0", "0A000001"),
        ("eor_l_imm_dn", "\teor.l\t#1,d0", "0A8000000001"),
    ]);
    assert_refusals(&[
        // asl `#1130 invalid operand size`: `adda` has no byte form.
        ("add_b_dn_an", "\tadd.b\td0,a1", "word/long only"),
        // asl `#1350`: there is no `anda`.
        ("and_w_dn_an", "\tand.w\td0,a1", "address-register destination"),
    ]);
}

#[test]
fn swap_refuses_every_suffix_asl_refuses() {
    assert_rows(&[
        ("swap", "\tswap\td0", "4840"),
        ("swap_l", "\tswap.l\td0", "4840"),
    ]);
    // asl `#1130 invalid operand size` for all three.
    assert_refusals(&[
        ("swap_w", "\tswap.w\td0", "`swap.w` is refused"),
        ("swap_b", "\tswap.b\td0", "`swap.b` is refused"),
        ("swap_s", "\tswap.s\td0", "`swap.s` is refused"),
    ]);
}

/// The forms the owner question AS-UNSIZED-DEFAULTS covers stay refused: none
/// of this parcel's rewrites may start accepting them.
#[test]
fn the_unsized_defaults_stay_out_of_scope() {
    assert_refusals(&[
        ("move_unsized", "\tmove\t#1,d0", "explicit size suffix"),
        ("bra_unsized", "\tbra\tt\n\tnop\nt:", "explicit size suffix"),
        ("bra_l", "\tbra.l\tt\n\tnop\nt:", "branch size suffix"),
        ("ds_bare", "\tds\t2", "not a recognized 68000 mnemonic"),
    ]);
}
