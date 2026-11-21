//! A6 (step 0 soundness): the FRONT-END differential. The same instruction is
//! spelled for the `.emp` front end and for the AS (asl-compatible) front end,
//! both are assembled and linked, and the outcomes are compared:
//!
//! - both produce bytes, and the bytes are equal ............ [`Outcome::SameBytes`]
//! - both refuse ............................................. [`Outcome::BothRefuse`]
//! - anything else (one refuses, or the bytes differ) ........ [`Outcome::Disagree`]
//!
//! A `Disagree` is a bug in one of the two front ends, unless a named row of
//! [`EXCLUSIONS`] explains it. Sigil finding 2 (the `.emp` route silently
//! truncating `move.b #256, d1` to `123C 0000` while the AS route refused it)
//! is exactly one such row of this matrix, and this gate is what would have
//! caught it.
//!
//! # The generator
//!
//! [`cases_m68k`] is deterministic and exhaustive over its own tables, never
//! pruned to make the gate pass: a family x size x EA combination that BOTH
//! front ends refuse is a `BothRefuse` and is counted.
//!
//! - **Families:** every row of `sigil_isa::m68k::ALL_FAMILY_NAMES` that takes
//!   an operand ([`NO_OPERAND_FAMILIES`] lists the seven that take none). The
//!   ten families whose second (or first) operand IS `sr`/`ccr`/`usp` hold that
//!   operand fixed ([`fixed_operand`]); every other family is tried with every
//!   source against every destination, including none.
//! - **Sizes:** [`SIZES`]: `.b`/`.w`/`.l`, plus `.s` (the short branch the
//!   aeon build emits) and the unsized spelling.
//! - **Sources:** [`src_table`] per size: each register/memory EA class once
//!   (with the boundary values of its own field for `(d16,An)`, `(d8,An,Xn)`,
//!   `(xxx).w`, `(xxx).l`), and the immediates `{lo-1, lo, -1, 0, 1, hi, hi+1}`
//!   with `(lo, hi)` from the size's Rust bounds ([`imm_bounds`]).
//! - **Destinations:** [`DST_TABLE`], each EA class once, and none.
//!
//! Z80 ([`cases_z80`]): `ld r,n`, `ld rr,nn`, `cp n` with the `i8`/`u8` and
//! `i16`/`u16` sets.
//!
//! This file lives under `tests/support/` (cargo builds no target from a
//! subdirectory) and is shared by `sigil-cli/tests/frontend_differential.rs`
//! and `sigil-harness/tests/frontend_differential_coverage.rs` through
//! `#[path]`, the `sigil-isa/tests/support/capstone_diff.rs` precedent: one
//! generator, one definition of the triple, so the coverage test measures the
//! generator the differential runs. Each including crate root binds `m68k`
//! (`use sigil_backend_m68k::m68k;` / `use sigil_isa::m68k;`).

#![allow(dead_code)]

use crate::m68k::ALL_FAMILY_NAMES;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;
use std::collections::{BTreeMap, BTreeSet};

// ── the case ────────────────────────────────────────────────────────────────

/// One 68000 effective address as the generator spells it. The first nine are
/// the brief's table; `Idx` .. `Usp` were added in Step 3 because the aeon
/// build emits them (the coverage test named the missing triples).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ea {
    D(u8),
    A(u8),
    Ind(u8),
    PostInc(u8),
    PreDec(u8),
    Disp(i16, u8),
    AbsW(u16),
    AbsL(u32),
    Imm(i64),
    /// `d8(An,Dn.w)`.
    Idx(i8, u8, u8),
    /// `tgt(pc)`, the frame's backward label.
    PcDisp,
    /// `tgt(pc,d0.w)`.
    PcIdx,
    /// A `movem` register list: bit 0..7 = d0..d7, bit 8..15 = a0..a7.
    RegList(u16),
    /// The frame's label as a branch target.
    Label,
    Sr,
    Ccr,
    Usp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub family: &'static str,
    pub size: Option<char>,
    pub src: Ea,
    pub dst: Option<Ea>,
}

/// A Z80 case: `mnemonic reg, n` (or `mnemonic n` when `reg` is `None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Z80Case {
    pub mnemonic: &'static str,
    pub reg: Option<&'static str>,
    pub imm: i64,
}

// ── the generator's tables ──────────────────────────────────────────────────

/// The families that take no operand at all, so there is no source to vary.
/// The coverage test holds this list to the build: an emitted instruction of
/// one of these families with an operand fails it.
pub const NO_OPERAND_FAMILIES: &[&str] = &["nop", "rts", "rte", "illegal", "reset", "rtr", "trapv"];

/// The spelled sizes. `.b`/`.w`/`.l` are the brief's; `.s` is the short
/// branch (`bra.s`, `bcc.s`), which the aeon build emits; `None` is the unsized
/// spelling, which is how most source writes `lea`, `moveq`, `exg`, `btst`.
pub const SIZES: &[Option<char>] = &[None, Some('b'), Some('w'), Some('l'), Some('s')];

/// The bounds a `#imm` of a size must sit in: the signed minimum to the
/// unsigned maximum of the field, from the Rust types (the brief's rule; also
/// both front ends' rule, `m68k_imm_bounds` / `imm_bounds`).
pub fn imm_bounds(size: char) -> (i64, i64) {
    match size {
        'b' => (i8::MIN as i64, u8::MAX as i64),
        'w' => (i16::MIN as i64, u16::MAX as i64),
        'l' => (i32::MIN as i64, u32::MAX as i64),
        other => panic!("no immediate bounds for size `.{other}`"),
    }
}

/// `{lo-1, lo, -1, 0, 1, hi, hi+1}` for one `(lo, hi)`.
fn imm_set(size: char) -> [i64; 7] {
    let (lo, hi) = imm_bounds(size);
    [lo - 1, lo, -1, 0, 1, hi, hi + 1]
}

/// The immediates tried at a spelled size. An unsized spelling has no one
/// field, so it gets the union of the three sets; `.s` (a branch) has none of
/// its own and gets the byte set.
pub fn imm_values(size: Option<char>) -> Vec<i64> {
    let sets: &[char] = match size {
        None => &['b', 'w', 'l'],
        Some('s') => &['b'],
        Some(c) => std::slice::from_ref(match c {
            'b' => &'b',
            'w' => &'w',
            'l' => &'l',
            other => panic!("unknown size `.{other}`"),
        }),
    };
    let set: BTreeSet<i64> = sets.iter().flat_map(|&s| imm_set(s)).collect();
    set.into_iter().collect()
}

/// The non-immediate sources: each EA class once, with the boundary values of
/// its own field where it has one.
pub const SRC_EAS: &[Ea] = &[
    Ea::D(1),
    Ea::A(2),
    Ea::Ind(3),
    Ea::PostInc(4),
    Ea::PreDec(5),
    Ea::Disp(i16::MIN, 1),
    Ea::Disp(0, 1),
    Ea::Disp(i16::MAX, 1),
    Ea::AbsW(0x0000),
    Ea::AbsW(0x7FFF),
    Ea::AbsW(0x8000),
    Ea::AbsW(0xFFFF),
    Ea::AbsL(0x0000_0000),
    Ea::AbsL(0x00FF_FFFF),
    Ea::AbsL(0xFFFF_FFFF),
    Ea::Idx(i8::MIN, 2, 3),
    Ea::Idx(0, 2, 3),
    Ea::Idx(i8::MAX, 2, 3),
    Ea::PcDisp,
    Ea::PcIdx,
    Ea::RegList(0x070F),
    Ea::Label,
];

/// Every source tried at `size`: [`SRC_EAS`] then the immediates.
pub fn src_table(size: Option<char>) -> Vec<Ea> {
    let mut v = SRC_EAS.to_vec();
    v.extend(imm_values(size).into_iter().map(Ea::Imm));
    v
}

/// Every destination: each EA class once, then none.
pub const DST_TABLE: &[Option<Ea>] = &[
    Some(Ea::D(6)),
    Some(Ea::A(4)),
    Some(Ea::Ind(1)),
    Some(Ea::PostInc(2)),
    Some(Ea::PreDec(3)),
    Some(Ea::Disp(8, 4)),
    Some(Ea::AbsW(0x1234)),
    Some(Ea::AbsL(0x00FF_8000)),
    Some(Ea::Idx(4, 5, 6)),
    Some(Ea::PcDisp),
    Some(Ea::PcIdx),
    Some(Ea::RegList(0x0301)),
    Some(Ea::Label),
    Some(Ea::Imm(0)),
    None,
];

/// The families whose `sr`/`ccr`/`usp` operand IS the family: the other
/// operand varies, this one is held. `(is_source, operand)`.
pub fn fixed_operand(family: &str) -> Option<(bool, Ea)> {
    Some(match family {
        "move-to-sr" | "andi-sr" | "ori-sr" | "eori-sr" => (false, Ea::Sr),
        "move-to-ccr" | "andi-ccr" | "ori-ccr" | "eori-ccr" => (false, Ea::Ccr),
        "move-to-usp" => (false, Ea::Usp),
        "move-from-sr" => (true, Ea::Sr),
        "move-from-usp" => (true, Ea::Usp),
        _ => return None,
    })
}

/// The mnemonic a family is spelled with. The condition families take one
/// condition (`ne`); the condition spelling itself is `m68k_cond_parity.rs`'s.
pub fn mnemonic(family: &str) -> &str {
    match family {
        "scc" => "sne",
        "bcc" => "bne",
        "dbcc" => "dbne",
        "move-to-sr" | "move-from-sr" | "move-to-ccr" | "move-to-usp" | "move-from-usp" => "move",
        "andi-ccr" | "andi-sr" => "andi",
        "ori-ccr" | "ori-sr" => "ori",
        "eori-ccr" | "eori-sr" => "eori",
        f => f,
    }
}

/// The families the generator covers, in `ALL_FAMILY_NAMES` order.
pub fn families() -> Vec<&'static str> {
    ALL_FAMILY_NAMES.iter().copied().filter(|f| !NO_OPERAND_FAMILIES.contains(f)).collect()
}

pub fn cases_m68k() -> Vec<Case> {
    let mut out = Vec::new();
    for family in families() {
        for &size in SIZES {
            match fixed_operand(family) {
                Some((true, fixed)) => {
                    for dst in DST_TABLE {
                        out.push(Case { family, size, src: fixed, dst: *dst });
                    }
                }
                Some((false, fixed)) => {
                    for src in src_table(size) {
                        out.push(Case { family, size, src, dst: Some(fixed) });
                    }
                }
                None => {
                    for src in src_table(size) {
                        for dst in DST_TABLE {
                            out.push(Case { family, size, src, dst: *dst });
                        }
                    }
                }
            }
        }
    }
    out
}

/// The brief's name for the 68000 generator.
pub fn cases() -> Vec<Case> {
    cases_m68k()
}

/// The number of cases the tables above define, computed from the tables
/// themselves (never typed, and not `cases_m68k().len()`): per family and
/// size, `|src| x |dst|`, or one side when the other is held.
pub fn expected_case_count() -> usize {
    let mut n = 0;
    for family in families() {
        for &size in SIZES {
            let srcs = SRC_EAS.len() + imm_values(size).len();
            n += match fixed_operand(family) {
                Some((true, _)) => DST_TABLE.len(),
                Some((false, _)) => srcs,
                None => srcs * DST_TABLE.len(),
            };
        }
    }
    n
}

pub const Z80_R8: &[&str] = &["a", "b", "c", "d", "e", "h", "l"];
pub const Z80_R16: &[&str] = &["bc", "de", "hl", "sp", "ix", "iy"];

fn z80_set(lo: i64, hi: i64) -> [i64; 7] {
    [lo - 1, lo, -1, 0, 1, hi, hi + 1]
}

pub fn cases_z80() -> Vec<Z80Case> {
    let n8 = z80_set(i8::MIN as i64, u8::MAX as i64);
    let n16 = z80_set(i16::MIN as i64, u16::MAX as i64);
    let mut out = Vec::new();
    for &r in Z80_R8 {
        for &imm in &n8 {
            out.push(Z80Case { mnemonic: "ld", reg: Some(r), imm });
        }
    }
    for &r in Z80_R16 {
        for &imm in &n16 {
            out.push(Z80Case { mnemonic: "ld", reg: Some(r), imm });
        }
    }
    for &imm in &n8 {
        out.push(Z80Case { mnemonic: "cp", reg: None, imm });
    }
    out
}

/// From the tables: one value set per `ld r,n` register, per `ld rr,nn`
/// register pair, and one for `cp n`.
pub fn expected_z80_case_count() -> usize {
    z80_set(0, 0).len() * (Z80_R8.len() + Z80_R16.len() + 1)
}

// ── spelling ────────────────────────────────────────────────────────────────

/// One operand, for either front end. The only difference between the two
/// spellings is the frame label: `.tgt` (a proc-local label) in `.emp`, `tgt`
/// in AS.
fn ea_text(ea: &Ea, emp: bool) -> String {
    let tgt = if emp { ".tgt" } else { "tgt" };
    match *ea {
        Ea::D(n) => format!("d{n}"),
        Ea::A(n) => format!("a{n}"),
        Ea::Ind(n) => format!("(a{n})"),
        Ea::PostInc(n) => format!("(a{n})+"),
        Ea::PreDec(n) => format!("-(a{n})"),
        Ea::Disp(d, n) => format!("{d}(a{n})"),
        // A short absolute address names a sign-extended word: $8000..$FFFF
        // is spelled as the address it reaches, $FFFF8000..$FFFFFFFF.
        Ea::AbsW(w) if w >= 0x8000 => format!("(${:08X}).w", 0xFFFF_0000u32 | w as u32),
        Ea::AbsW(w) => format!("(${w:04X}).w"),
        Ea::AbsL(a) => format!("(${a:08X}).l"),
        Ea::Imm(v) => format!("#{v}"),
        Ea::Idx(d, an, dn) => format!("{d}(a{an},d{dn}.w)"),
        Ea::PcDisp => format!("{tgt}(pc)"),
        Ea::PcIdx => format!("{tgt}(pc,d0.w)"),
        Ea::RegList(mask) => reglist_text(mask),
        Ea::Label => tgt.to_string(),
        Ea::Sr => "sr".into(),
        Ea::Ccr => "ccr".into(),
        Ea::Usp => "usp".into(),
    }
}

fn reglist_text(mask: u16) -> String {
    let mut parts = Vec::new();
    for (bank, base) in [('d', 0u16), ('a', 8u16)] {
        let mut i = 0;
        while i < 8 {
            if mask & (1 << (base + i)) != 0 {
                let start = i;
                while i + 1 < 8 && mask & (1 << (base + i + 1)) != 0 {
                    i += 1;
                }
                parts.push(if start == i {
                    format!("{bank}{start}")
                } else {
                    format!("{bank}{start}-{bank}{i}")
                });
            }
            i += 1;
        }
    }
    parts.join("/")
}

/// The instruction line alone, e.g. `move.b #256, d1`.
fn inst_text(c: &Case, emp: bool) -> String {
    let mut s = mnemonic(c.family).to_string();
    if let Some(z) = c.size {
        s.push('.');
        s.push(z);
    }
    s.push(' ');
    s.push_str(&ea_text(&c.src, emp));
    if let Some(d) = &c.dst {
        s.push_str(", ");
        s.push_str(&ea_text(d, emp));
    }
    s
}

/// The case as a human reads it in a report: the AS spelling of the line.
pub fn label(c: &Case) -> String {
    inst_text(c, false)
}

/// The `.emp` program: the instruction framed by a backward label on a `nop`
/// (the branch, `dbcc` and pc-relative target) and an `rts`. The proc allows
/// `stack.unbalanced`: the differential compares one instruction at a time, so a
/// `link` is necessarily alone in its frame, which it leaves open.
pub fn emp_spelling(c: &Case) -> String {
    format!("module m\n@allow(\"stack.unbalanced\")\nproc f() {{\n.tgt:\n    nop\n    {}\n    rts\n}}\n", inst_text(c, true))
}

/// The AS program: the same frame.
pub fn as_spelling(c: &Case) -> String {
    format!("\tcpu 68000\ntgt:\n\tnop\n\t{}\n\trts\n", inst_text(c, false))
}

fn z80_inst(c: &Z80Case) -> String {
    match c.reg {
        Some(r) => format!("{} {r}, {}", c.mnemonic, c.imm),
        None => format!("{} {}", c.mnemonic, c.imm),
    }
}

pub fn z80_label(c: &Z80Case) -> String {
    z80_inst(c)
}

/// Z80 values are spelled in decimal: under `cpu z80` the AS front end lexes a
/// bare `$` as the program counter, as asl does.
pub fn z80_emp_spelling(c: &Z80Case) -> String {
    format!("module m (cpu: z80)\nproc f() {{\n    {}\n    ret\n}}\n", z80_inst(c))
}

pub fn z80_as_spelling(c: &Z80Case) -> String {
    format!("\tcpu z80\n\t{}\n\tret\n", z80_inst(c))
}

// ── the two front ends ──────────────────────────────────────────────────────

/// A front-end panic is an outcome too (a refusal no author can read), and it
/// must not take the whole matrix down with it.
fn guarded(f: impl FnOnce() -> Result<Vec<u8>, Vec<String>> + std::panic::UnwindSafe) -> Result<Vec<u8>, Vec<String>> {
    match std::panic::catch_unwind(f) {
        Ok(r) => r,
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic>".into());
            Err(vec![format!("PANIC: {msg}")])
        }
    }
}

/// The link environment both frames see: one symbol defined OUTSIDE the
/// module, named `usp`, the way an AS module or another unit could define it.
/// `.emp` cannot declare `usp` itself (`[name.reserved-register]`), so this is
/// the one way a `usp` symbol reaches an `.emp` operand. Before the A6 fix round
/// a bare `usp` operand resolved to it and `move.l a2, usp` assembled
/// `21F8 1234` (a move to `($1234).w`); asl assembles `4E62`. With the symbol
/// in scope the gate itself sees that direction.
pub const USP_SYMBOL_VALUE: i64 = 0x1234;

fn link_env() -> SymbolTable {
    let mut t = SymbolTable::new();
    t.define("usp", sigil_ir::SymbolValue::Int(USP_SYMBOL_VALUE));
    t
}

/// `.emp`: `parse_str` + `lower_module` + resolve + link + flatten. The bytes,
/// or every ERROR message.
pub fn emp_assemble(src: &str) -> Result<Vec<u8>, Vec<String>> {
    let src = src.to_string();
    guarded(move || {
        let (file, perrs) = parse_str(&src);
        let perrs: Vec<String> =
            perrs.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
        if !perrs.is_empty() {
            return Err(perrs);
        }
        let (m, diags) = lower_module(
            &file,
            &LowerOptions {
                roots: None,
                initial_cpu: Cpu::M68000,
                include_root: None,
                embed_base: None,
                defines: vec![],
            },
        );
        let errs: Vec<String> =
            diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
        if !errs.is_empty() {
            return Err(errs);
        }
        let env = link_env();
        let resolved = sigil_link::resolve_layout(&m.sections, &env, true)
            .map_err(|e| vec![format!("resolve_layout: {e:?}")])?;
        let linked = sigil_link::link(&resolved, &env)
            .map_err(|e| vec![format!("link: {e:?}")])?;
        sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
    })
}

/// AS: `sigil_frontend_as::assemble` + link + flatten.
pub fn as_assemble(src: &str) -> Result<Vec<u8>, Vec<String>> {
    let src = src.to_string();
    guarded(move || {
        let opts = sigil_frontend_as::Options::default();
        let module = sigil_frontend_as::assemble(&src, &opts)
            .map_err(|ds| ds.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
        let linked = sigil_link::link(&module.sections, &link_env())
            .map_err(|e| vec![format!("link: {e:?}")])?;
        sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
    })
}

// ── comparison ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    SameBytes,
    BothRefuse,
    Disagree { emp: Result<Vec<u8>, Vec<String>>, as_: Result<Vec<u8>, Vec<String>> },
}

fn panicked(r: &Result<Vec<u8>, Vec<String>>) -> bool {
    matches!(r, Err(m) if m.iter().any(|s| s.starts_with("PANIC: ")))
}

/// A front-end panic is never agreement: it is a `Disagree` whatever the
/// other side did, and no exclusion names a panic, so it fails the gate.
fn outcome(emp: Result<Vec<u8>, Vec<String>>, as_: Result<Vec<u8>, Vec<String>>) -> Outcome {
    if panicked(&emp) || panicked(&as_) {
        return Outcome::Disagree { emp, as_ };
    }
    match (&emp, &as_) {
        (Ok(a), Ok(b)) if a == b => Outcome::SameBytes,
        (Err(_), Err(_)) => Outcome::BothRefuse,
        _ => Outcome::Disagree { emp, as_ },
    }
}

pub fn compare(c: &Case) -> Outcome {
    outcome(emp_assemble(&emp_spelling(c)), as_assemble(&as_spelling(c)))
}

pub fn compare_z80(c: &Z80Case) -> Outcome {
    outcome(emp_assemble(&z80_emp_spelling(c)), as_assemble(&z80_as_spelling(c)))
}

/// A sized case whose immediate source lies outside its size's bounds.
pub fn is_out_of_range(c: &Case) -> bool {
    match (c.size, c.src) {
        (Some(z @ ('b' | 'w' | 'l')), Ea::Imm(v)) => {
            let (lo, hi) = imm_bounds(z);
            v < lo || v > hi
        }
        _ => false,
    }
}

// ── named exclusions ────────────────────────────────────────────────────────

/// `(pattern, reason)`. A pattern is the NAME of a predicate in
/// [`exclusion_matches`] (a value-range or message class cannot be a text
/// glob). Each predicate is narrow: it names the families, the exact `.emp`
/// message class and the side that refuses, so a NEW disagreement (above all
/// the finding-2 direction, `.emp` bytes where AS refuses, or two different
/// byte strings) is never absorbed by an old row. The two rows where `.emp`
/// assembles and AS refuses also prove the `.emp` bytes equal the AS route's
/// bytes for the asl-legal spelling, so neither can hide a wrong byte. Every
/// row must match at least one `Disagree` (`every_exclusion_is_used`).
///
/// "asl" below is the real asl 1.42 Bld 212 (`sonic_hack/tools/as/asl`),
/// probed on every disagreement of the matrix (Task 9 report): the AS route
/// matched asl's verdict and bytes on all rows these exclusions name.
/// Rows 2-9 are `.emp` refusing a form asl and the AS route assemble: a
/// capability gap, never a wrong byte (booked, `docs/QUEUE.md`
/// `EMP-FRONTEND-DIFF-GAPS`).
pub const EXCLUSIONS: &[(&str, &str)] = &[
    (
        "bit-number-past-width",
        "A bit instruction's immediate bit number outside 0..=31 (Dn) or 0..=7 (memory). \
         `.emp` refuses it with `[lower.bit-number]` (A1, Task 3); the AS route accepts it as \
         asl does (asl assembles `btst #40, d0` to `0800 0028` with only `warning: bit number \
         will be truncated`; the 68000 takes the number modulo 32 / 8, MC68000 PRM BTST).",
    ),
    (
        "emp-requires-a-size-suffix",
        "An unsized spelling of a form whose size `.emp` does not default (`m68k_default_size`, \
         `lower/code.rs`: shifts/rotates on memory, `move`/`andi`/`ori`/`eori` to sr/ccr, \
         `bchg`, `addx`, `subx`, `negx`). `.emp` refuses `instruction needs an explicit size suffix`; asl \
         and the AS route default the size (word; `bchg` byte on memory, long on Dn).",
    ),
    (
        "emp-no-implicit-variant",
        "asl (and the AS route) rewrite `add`/`sub <ea>,An` to `adda`/`suba` and `eor #imm,Dn` to \
         `eori`; `.emp` does not (`Add|Sub requires a Dn operand`, `... with an address-register \
         destination is not encodable, write adda/suba`, `eor requires Dn source`).",
    ),
    (
        "emp-label-operand-gap",
        "A frame label in an operand position `.emp` cannot yet defer to the linker: a straight-line \
         symbolic operand (`symbolic operand ... is not yet supported`), a symbolic absolute operand \
         beside another extension word or a second unpinned one (`[lower.abs-sym-operand]`). \
         asl and the AS route resolve the label.",
    ),
    (
        "branch-b-suffix",
        "`bra.b`/`bsr.b`/`bcc.b`: asl and the AS route read `.b` as the short form (`.s`); `.emp` \
         accepts only `.s`/`.w` (`branch size suffix must be .s or .w`).",
    ),
    (
        "unsized-branch-aeon-pin",
        "An unsized `bra`/`bsr`/`bcc`: the AS route refuses it by AEON POLICY (`frontend-as/src/eval.rs`, \
         `branch needs an explicit size suffix (.s or .w), Aeon pins branch width, no relaxation`), \
         not as asl does (asl relaxes it and assembles). `.emp` assembles it; the predicate proves the \
         `.emp` bytes equal the AS route's `.s` spelling of the same branch.",
    ),
    (
        "sized-swap",
        "`swap.b/.w/.s dN`: asl refuses (`error: invalid operand size`, probed) and the AS route \
         refuses as asl does; `.emp` ignores the suffix and assembles `swap`. The predicate proves \
         the `.emp` bytes equal the AS route's unsized `swap`, so no byte is wrong; the laxness is \
         booked (`EMP-FRONTEND-DIFF-GAPS`).",
    ),
];

fn is_bit_family(f: &str) -> bool {
    matches!(f, "btst" | "bset" | "bclr" | "bchg")
}

fn is_branch_family(f: &str) -> bool {
    matches!(f, "bra" | "bsr" | "bcc")
}

/// The families row `emp-requires-a-size-suffix` may name: those `.emp` has no
/// default size for. Closed, so a NEW unsized refusal is a new disagreement.
pub const SIZE_DEFAULT_GAP_FAMILIES: &[&str] = &[
    "addx", "subx", "negx", "asl", "asr", "lsl", "lsr", "rol", "ror", "roxl", "roxr", "bchg",
    "andi-ccr", "andi-sr", "ori-ccr", "ori-sr", "eori-ccr", "eori-sr",
    "move-to-sr", "move-from-sr", "move-to-ccr", "move-to-usp", "move-from-usp",
];

/// The families row `emp-label-operand-gap` may name.
pub const LABEL_GAP_FAMILIES: &[&str] = &["move", "movem"];

fn emp_err_only(emp: &Result<Vec<u8>, Vec<String>>) -> Option<&[String]> {
    match emp {
        Err(m) if !m.is_empty() => Some(m),
        _ => None,
    }
}

fn has_label_operand(c: &Case) -> bool {
    let is_label = |e: &Ea| matches!(e, Ea::Label | Ea::PcDisp | Ea::PcIdx);
    is_label(&c.src) || c.dst.as_ref().is_some_and(is_label)
}

/// The AS route's bytes for `c` respelled at `size`.
fn as_bytes_at(c: &Case, size: Option<char>) -> Option<Vec<u8>> {
    as_assemble(&as_spelling(&Case { size, ..c.clone() })).ok()
}

/// Whether the excluded class `pattern` explains this disagreement. Unknown
/// pattern names are a bug in [`EXCLUSIONS`] and panic.
pub fn exclusion_matches(pattern: &str, c: &Case, o: &Outcome) -> bool {
    let Outcome::Disagree { emp, as_ } = o else { return false };
    // Rows 2-9: `.emp` refuses, AS assembles, and EVERY `.emp` message is of
    // the row's class.
    let emp_refuses_as_assembles = |class: &dyn Fn(&str) -> bool| {
        as_.is_ok() && emp_err_only(emp).is_some_and(|m| m.iter().all(|s| class(s)))
    };
    match pattern {
        "bit-number-past-width" => {
            let Ea::Imm(v) = c.src else { return false };
            let hi = match c.dst {
                Some(Ea::D(_)) => 31,
                Some(_) => 7,
                None => return false,
            };
            is_bit_family(c.family)
                && !(0..=hi).contains(&v)
                && emp_refuses_as_assembles(&|s| s.starts_with("[lower.bit-number]"))
        }
        "emp-requires-a-size-suffix" => {
            c.size.is_none()
                && SIZE_DEFAULT_GAP_FAMILIES.contains(&c.family)
                && emp_refuses_as_assembles(&|s| s == "instruction needs an explicit size suffix (.b/.w/.l)")
        }
        "emp-no-implicit-variant" => {
            let to_an = matches!(c.dst, Some(Ea::A(_)));
            let eor_imm = matches!(c.src, Ea::Imm(_)) && matches!(c.dst, Some(Ea::D(_)));
            match c.family {
                "add" | "sub" if to_an => emp_refuses_as_assembles(&|s| {
                    s.starts_with("unsupported form: Add requires a Dn operand, got ")
                        || s.starts_with("unsupported form: Sub requires a Dn operand, got ")
                        || s.starts_with("unsupported form: Add.W with an address-register destination")
                        || s.starts_with("unsupported form: Add.L with an address-register destination")
                        || s.starts_with("unsupported form: Sub.W with an address-register destination")
                        || s.starts_with("unsupported form: Sub.L with an address-register destination")
                }),
                "eor" if eor_imm => emp_refuses_as_assembles(&|s| {
                    s.starts_with("unsupported form: eor requires Dn source, got Imm(")
                }),
                _ => false,
            }
        }
        "emp-label-operand-gap" => {
            LABEL_GAP_FAMILIES.contains(&c.family)
                && has_label_operand(c)
                && emp_refuses_as_assembles(&|s| {
                    (s.starts_with("symbolic operand `")
                        && s.contains("` in a straight-line instruction is not yet supported"))
                        || s.starts_with("[lower.abs-sym-operand] ")
                })
        }
        "branch-b-suffix" => {
            is_branch_family(c.family)
                && c.size == Some('b')
                && emp_refuses_as_assembles(&|s| s == "branch size suffix must be `.s` or `.w`")
        }
        "unsized-branch-aeon-pin" => {
            is_branch_family(c.family)
                && c.size.is_none()
                && matches!(as_, Err(m) if m.iter().all(|s| s.starts_with(
                    "branch needs an explicit size suffix (.s or .w), Aeon pins branch width")))
                && matches!(emp, Ok(b) if Some(b) == as_bytes_at(c, Some('s')).as_ref())
        }
        "sized-swap" => {
            c.family == "swap"
                && c.size.is_some()
                && matches!(as_, Err(m) if m.iter().all(|s| s.contains("is refused, as asl refuses it (invalid operand size)")))
                && matches!(emp, Ok(b) if Some(b) == as_bytes_at(c, None).as_ref())
        }
        other => panic!("EXCLUSIONS names an unknown pattern `{other}`"),
    }
}

/// The first exclusion row explaining `o`, if any.
pub fn excluded_by(c: &Case, o: &Outcome) -> Option<usize> {
    EXCLUSIONS.iter().position(|(p, _)| exclusion_matches(p, c, o))
}

// ── running ─────────────────────────────────────────────────────────────────

/// The per-family tally a run prints.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    pub same: usize,
    pub refuse: usize,
    pub excluded: usize,
    pub disagree: usize,
}

/// One run's result.
pub struct RunResult {
    pub n: usize,
    /// `Disagree` outcomes no exclusion explains.
    pub bad: Vec<(Case, Outcome)>,
    /// How many `Disagree` outcomes each EXCLUSIONS row explained.
    pub exclusion_hits: Vec<usize>,
    pub per_family: BTreeMap<&'static str, Tally>,
}

/// Compare every case, in parallel (each case is an independent pair of
/// assemblies), and classify.
pub fn run_all(cases: Vec<Case>) -> RunResult {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(32);
    let chunk = cases.len().div_ceil(threads).max(1);
    let outcomes: Vec<Outcome> = std::thread::scope(|s| {
        let handles: Vec<_> = cases
            .chunks(chunk)
            .map(|part| {
                std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn_scoped(s, move || part.iter().map(compare).collect::<Vec<_>>())
                    .expect("spawn")
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("worker panicked")).collect()
    });
    let mut r = RunResult {
        n: cases.len(),
        bad: Vec::new(),
        exclusion_hits: vec![0; EXCLUSIONS.len()],
        per_family: BTreeMap::new(),
    };
    for (c, o) in cases.into_iter().zip(outcomes) {
        let t = r.per_family.entry(c.family).or_default();
        match &o {
            Outcome::SameBytes => t.same += 1,
            Outcome::BothRefuse => t.refuse += 1,
            Outcome::Disagree { .. } => match excluded_by(&c, &o) {
                Some(i) => {
                    t.excluded += 1;
                    r.exclusion_hits[i] += 1;
                }
                None => {
                    t.disagree += 1;
                    r.bad.push((c, o));
                }
            },
        }
    }
    r
}

/// The brief's `run`: `(n, bad)`.
pub fn run(cases: Vec<Case>) -> (usize, Vec<(Case, Outcome)>) {
    let r = run_all(cases);
    (r.n, r.bad)
}

fn show(r: &Result<Vec<u8>, Vec<String>>) -> String {
    match r {
        Ok(b) => format!("bytes {}", b.iter().map(|x| format!("{x:02X}")).collect::<String>()),
        Err(m) => format!("refused {m:?}"),
    }
}

/// Every unexplained disagreement, one line each (capped), plus the count.
pub fn report(bad: &[(Case, Outcome)]) -> String {
    let mut s = format!("{} front-end disagreement(s) not covered by EXCLUSIONS:\n", bad.len());
    for (c, o) in bad.iter().take(200) {
        if let Outcome::Disagree { emp, as_ } = o {
            s.push_str(&format!("  `{}`: emp {} | AS {}\n", label(c), show(emp), show(as_)));
        }
    }
    if bad.len() > 200 {
        s.push_str(&format!("  ... and {} more\n", bad.len() - 200));
    }
    s
}

pub fn report_z80(bad: &[(Z80Case, Outcome)]) -> String {
    let mut s = format!("{} Z80 front-end disagreement(s):\n", bad.len());
    for (c, o) in bad {
        if let Outcome::Disagree { emp, as_ } = o {
            s.push_str(&format!("  `{}`: emp {} | AS {}\n", z80_label(c), show(emp), show(as_)));
        }
    }
    s
}

// ── coverage triples ────────────────────────────────────────────────────────

/// An EA's class name, as the coverage triples spell it.
pub fn ea_class(ea: &Ea) -> &'static str {
    match ea {
        Ea::D(_) => "Dn",
        Ea::A(_) => "An",
        Ea::Ind(_) => "(An)",
        Ea::PostInc(_) => "(An)+",
        Ea::PreDec(_) => "-(An)",
        Ea::Disp(..) => "(d16,An)",
        Ea::AbsW(_) => "(xxx).w",
        Ea::AbsL(_) => "(xxx).l",
        Ea::Imm(_) => "#imm",
        Ea::Idx(..) => "(d8,An,Xn)",
        Ea::PcDisp => "(d16,PC)",
        Ea::PcIdx => "(d8,PC,Xn)",
        Ea::RegList(_) => "reglist",
        Ea::Label => "disp",
        Ea::Sr => "sr",
        Ea::Ccr => "ccr",
        Ea::Usp => "usp",
    }
}

/// `(family, spelled size, EA class per operand)` of one case.
pub fn case_triple(c: &Case) -> (String, Option<char>, Vec<&'static str>) {
    let mut ops = vec![ea_class(&c.src)];
    if let Some(d) = &c.dst {
        ops.push(ea_class(d));
    }
    (c.family.to_string(), c.size, ops)
}

/// The triple of every generated case.
pub fn case_triples() -> BTreeSet<(String, Option<char>, Vec<&'static str>)> {
    cases_m68k().iter().map(case_triple).collect()
}
