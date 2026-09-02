//! Parity between the TWO copies of the 68000 condition-code table.
//!
//! # The gap this closes
//!
//! `m68k_cond` — the map from a condition-code spelling to its 4-bit cc field —
//! exists twice, independently:
//!
//! - `crates/sigil-frontend-as/src/eval.rs` — unit-tested in place
//!   (`m68k_cond_parses_all_16_condition_codes`).
//! - `crates/sigil-frontend-emp/src/lower/code.rs` — **the copy the shipping
//!   front end uses**, and the copy nothing unit-tested.
//!
//! Its doc comment says *"Mirrors AS's `m68k_cond`"*, and until this file nothing
//! checked that it does. A wrong or dropped arm in the `.emp` copy was caught only
//! by the external corpus happening to contain the spelling — which is coverage
//! that a pinned corpus would freeze into a permanent sole witness.
//!
//! # What is compared, and how
//!
//! Both copies are private `fn`s, so this test does not call them; it compares
//! their **observable effect**, which is the stronger comparison — a table that
//! agreed textually but was reached differently would still pass a source-level
//! diff and fail here. For every spelling, the same instruction is assembled
//! through both front ends and the emitted bytes compared, across all three
//! families that consume a condition code:
//!
//! | family | encoder | base word (from `sigil-isa/src/m68k.rs`) |
//! |---|---|---|
//! | `s<cc> d0` | `encode_single_ea` | `0x50C0 \| (cc<<8) \| ea` — `d0` is ea `0x00` |
//! | `db<cc> d0,tgt` | `encode_dbcc` | `0x50C8 \| (cc<<8) \| dn`, then a disp word |
//! | `b<cc>.s tgt` | `encode_branch` | `0x6000 \| (cc<<8)`, disp in the low byte |
//!
//! Each family is asserted **twice**: the two front ends must agree with each
//! other, and both must agree with an expectation derived from those base words
//! and the `Cond` enum's documented nibble. Agreement alone would be satisfied by
//! two copies that are identically wrong.
//!
//! `bt`/`bf` are excluded from the branch family only. Both front ends reach them
//! by the same generic `strip_prefix('b')` path, so they would encode to `0x6000`
//! and `0x6100` — `bra` and `bsr`. That is a shared property of both copies rather
//! than a parity difference, and testing it here would read as a blessing of a
//! spelling no 68000 assembler is expected to offer. The `s`/`db` families cover
//! `t` and `f`.

use sigil_backend_m68k::m68k::Cond;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Every spelling both tables claim to accept: the 16 ISA condition codes plus
/// the two unsigned-branch aliases `hs` (== `cc`) and `lo` (== `cs`).
const SPELLINGS: [(&str, Cond); 18] = [
    ("t", Cond::T),
    ("f", Cond::F),
    ("hi", Cond::Hi),
    ("ls", Cond::Ls),
    ("cc", Cond::Cc),
    ("cs", Cond::Cs),
    ("hs", Cond::Cc),
    ("lo", Cond::Cs),
    ("ne", Cond::Ne),
    ("eq", Cond::Eq),
    ("vc", Cond::Vc),
    ("vs", Cond::Vs),
    ("pl", Cond::Pl),
    ("mi", Cond::Mi),
    ("ge", Cond::Ge),
    ("lt", Cond::Lt),
    ("gt", Cond::Gt),
    ("le", Cond::Le),
];

fn cc_nibble(c: Cond) -> u16 {
    let cc = c.cc();
    assert!(cc <= 0xF, "{c:?} discriminant {cc:#x} is not a 4-bit cc field");
    cc
}

fn as_image(src: &str) -> Vec<u8> {
    let opts = sigil_frontend_as::Options::default();
    let module = sigil_frontend_as::assemble(src, &opts).expect("AS assemble");
    let linked = sigil_link::link(&module.sections, &SymbolTable::new()).expect("AS link");
    sigil_link::flatten(&linked, 0x00)
}

fn emp_image(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "emp parse: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<&str> = diags
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| d.message.as_str())
        .collect();
    assert!(errs.is_empty(), "emp lower: {errs:?}");
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// `rts` = `4E75`, `nop` = `4E71` — the fixed-opcode frame around each probe.
const NOP: [u8; 2] = [0x4E, 0x71];
const RTS: [u8; 2] = [0x4E, 0x75];

/// Run one probe through both front ends and against a derived expectation.
///
/// `want` is built by the caller from the ISA base word, never from a corpus
/// image; `as_src`/`emp_src` are the same logical program in the two syntaxes.
fn assert_both(label: &str, as_src: &str, emp_src: &str, want: &[u8]) {
    let a = as_image(as_src);
    let e = emp_image(emp_src);
    assert_eq!(a, e, "{label}: the two `m68k_cond` copies disagree (AS vs .emp)");
    assert_eq!(a, want, "{label}: both front ends disagree with the derived encoding");
}

/// `s<cc> d0` — `encode_single_ea`: `0x50C0 | (cc<<8) | ea`, `d0` = ea `0x00`.
/// Covers all 18 spellings including `t`/`f`, which the branch family cannot.
#[test]
fn scc_parity_over_all_eighteen_spellings() {
    for (w, c) in SPELLINGS {
        let word = 0x50C0u16 | (cc_nibble(c) << 8);
        let mut want = word.to_be_bytes().to_vec();
        want.extend_from_slice(&RTS);
        assert_both(
            &format!("s{w}"),
            &format!("\tcpu 68000\n\ts{w}\td0\n\trts\n"),
            &format!("module m\nproc p() {{\n    s{w}  d0\n    rts\n}}\n"),
            &want,
        );
    }
}

/// `db<cc> d0,tgt` — `encode_dbcc`: `0x50C8 | (cc<<8) | dn` (dn = 0), then a
/// fixed 16-bit displacement word. The `dbcc` opcode is at offset 2 (after the
/// framing `nop`), the displacement is measured from `instruction_address + 2`
/// = 4, and `tgt` is at 0, so the word is `-4` = `FF FC`.
#[test]
fn dbcc_parity_over_all_eighteen_spellings() {
    for (w, c) in SPELLINGS {
        let word = 0x50C8u16 | (cc_nibble(c) << 8);
        let mut want = NOP.to_vec();
        want.extend_from_slice(&word.to_be_bytes());
        want.extend_from_slice(&(-4i16).to_be_bytes());
        want.extend_from_slice(&RTS);
        assert_both(
            &format!("db{w}"),
            &format!("\tcpu 68000\ntgt:\n\tnop\n\tdb{w}\td0,tgt\n\trts\n"),
            &format!(
                "module m\nproc p() {{\n.tgt:\n    nop\n    db{w}  d0, .tgt\n    rts\n}}\n"
            ),
            &want,
        );
    }
}

/// `b<cc>.s tgt` — `encode_branch`: `0x6000 | (cc<<8)` with the signed 8-bit
/// displacement in the low byte. Same layout as `dbcc` above, so the byte is
/// `-4` = `FC`. `t`/`f` excluded (see the module header).
#[test]
fn bcc_short_parity_over_the_sixteen_branch_spellings() {
    for (w, c) in SPELLINGS.iter().filter(|(w, _)| *w != "t" && *w != "f") {
        let word = 0x6000u16 | (cc_nibble(*c) << 8);
        let mut want = NOP.to_vec();
        want.extend_from_slice(&[word.to_be_bytes()[0], (-4i8) as u8]);
        want.extend_from_slice(&RTS);
        assert_both(
            &format!("b{w}.s"),
            &format!("\tcpu 68000\ntgt:\n\tnop\n\tb{w}.s\ttgt\n\trts\n"),
            &format!(
                "module m\nproc p() {{\n.tgt:\n    nop\n    b{w}.s  .tgt\n    rts\n}}\n"
            ),
            &want,
        );
    }
}

/// `b<cc>.w tgt` — the same base word with a zero low byte and a following
/// displacement word (`FF FC`). Exercises the `Size::W` arm of `encode_branch`
/// through both tables, so a copy that were right for short branches and wrong
/// for word branches is still caught.
#[test]
fn bcc_word_parity_over_the_sixteen_branch_spellings() {
    for (w, c) in SPELLINGS.iter().filter(|(w, _)| *w != "t" && *w != "f") {
        let word = 0x6000u16 | (cc_nibble(*c) << 8);
        let mut want = NOP.to_vec();
        want.extend_from_slice(&[word.to_be_bytes()[0], 0x00]);
        want.extend_from_slice(&(-4i16).to_be_bytes());
        want.extend_from_slice(&RTS);
        assert_both(
            &format!("b{w}.w"),
            &format!("\tcpu 68000\ntgt:\n\tnop\n\tb{w}.w\ttgt\n\trts\n"),
            &format!(
                "module m\nproc p() {{\n.tgt:\n    nop\n    b{w}.w  .tgt\n    rts\n}}\n"
            ),
            &want,
        );
    }
}

/// The alias pair, stated as its own assertion so a table that dropped `hs`/`lo`
/// entirely — rather than mis-mapping them — fails with the reason named.
/// `hs` (higher-or-same) IS carry-clear and `lo` (lower) IS carry-set on the
/// 68000, so they must be byte-identical to `cc`/`cs`, in both front ends.
#[test]
fn hs_lo_are_exact_aliases_of_cc_cs_in_both_front_ends() {
    for (alias, canon) in [("hs", "cc"), ("lo", "cs")] {
        let as_alias = as_image(&format!("\tcpu 68000\n\ts{alias}\td0\n\trts\n"));
        let as_canon = as_image(&format!("\tcpu 68000\n\ts{canon}\td0\n\trts\n"));
        assert_eq!(as_alias, as_canon, "AS: s{alias} must equal s{canon}");
        let emp_alias = emp_image(&format!("module m\nproc p() {{\n    s{alias}  d0\n    rts\n}}\n"));
        let emp_canon = emp_image(&format!("module m\nproc p() {{\n    s{canon}  d0\n    rts\n}}\n"));
        assert_eq!(emp_alias, emp_canon, ".emp: s{alias} must equal s{canon}");
        assert_eq!(as_alias, emp_alias, "s{alias}: AS and .emp disagree");
    }
}

/// Rejection parity: a word that is not a condition code must be refused by BOTH
/// front ends. A copy that grew a spurious arm would still pass every acceptance
/// test above; only the negative side catches it.
#[test]
fn a_non_condition_suffix_is_refused_by_both_front_ends() {
    for w in ["xx", "banana", "zz", "q"] {
        // Deliberately NOT wrapped in `catch_unwind`: the refusal must be a
        // returned `Err`, not a panic. A panicking front end would otherwise be
        // scored as a correct rejection.
        let opts = sigil_frontend_as::Options::default();
        let as_r = sigil_frontend_as::assemble(&format!("\tcpu 68000\n\ts{w}\td0\n\trts\n"), &opts);
        assert!(as_r.is_err(), "AS accepted `s{w}` as a condition-code form");

        let src = format!("module m\nproc p() {{\n    s{w}  d0\n    rts\n}}\n");
        let (file, _) = parse_str(&src);
        let (_, diags) = lower_module(
            &file,
            &LowerOptions {
                initial_cpu: Cpu::M68000,
                include_root: None,
                embed_base: None,
                defines: vec![],
            },
        );
        assert!(
            diags.iter().any(|d| d.level == Level::Error),
            ".emp accepted `s{w}` as a condition-code form"
        );
    }
}
