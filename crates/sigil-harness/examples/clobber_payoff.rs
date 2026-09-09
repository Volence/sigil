//! `clobber_payoff` — what the over-declared `clobbers(...)` sets cost, measured
//! at the CONSUMING end.
//!
//! The producer half (how many procs declare a register they never touch) is a
//! count of declarations and says nothing about cost: a declaration is not code.
//! The consuming half is the question — how many register saves and restores in
//! the shipped image exist only because a callee's contract claims a clobber it
//! does not perform.
//!
//! Both halves are printed per shipped shape:
//!
//! - **producer**: `declared(P) \ effective(P)` per proc, where `effective` is
//!   the corpus closure's transitive set (`localWrites ∪ callees' effective ∪
//!   indirect bounds − verifiedPreserves`), so a register in this difference is
//!   one the proc's own emitted code provably never writes.
//! - **consumer**: the `[proc.dead-save]` worklist from the same analysis — a
//!   verified save/restore pair around calls that all preserve the register.
//!   Every save an over-declaration could have motivated is in this list, because
//!   an over-declared register is by definition absent from `effective` and
//!   `find_dead_saves` reads `effective`.
//!
//! Reference tree: `AEON_DIR`, or the sibling aeon checkout. Read-only.

use sigil_frontend_emp::ast::{Item, ProcDecl};
use sigil_frontend_emp::corpus_contracts::{analyze_corpus_with_contracts, bind_corpus_interfaces};
use sigil_frontend_emp::parse_str;
use sigil_harness::native;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// The register file the closure ranges over, in canonical order.
fn universe() -> Vec<String> {
    (0..8).map(|n| format!("d{n}")).chain((0..8).map(|n| format!("a{n}"))).collect()
}

/// Expand a declared reglist (`d0-d3/a1`) to canonical register spellings. The
/// same `lo-hi` / `lo` segment shape the census renders, expanded over the
/// canonical `d0..d7,a0..a7` order the closure keys on.
fn expand(segs: &[(String, Option<String>)]) -> BTreeSet<String> {
    let uni = universe();
    let mut out = BTreeSet::new();
    for (lo, hi) in segs {
        match hi {
            None => {
                out.insert(lo.clone());
            }
            Some(h) => {
                let (Some(i), Some(j)) =
                    (uni.iter().position(|r| r == lo), uni.iter().position(|r| r == h))
                else {
                    // Not a register range this walk understands: keep the
                    // endpoints rather than silently dropping the segment.
                    out.insert(lo.clone());
                    out.insert(h.clone());
                    continue;
                };
                for r in &uni[i.min(j)..=i.max(j)] {
                    out.insert(r.clone());
                }
            }
        }
    }
    out.remove("a7");
    out
}

/// Every proc in the file tree, with its DECLARED clobbers (`None` = no
/// contract). `extern proc` leaves are collected separately: the closure fixes
/// their `effective` AT the declaration, so "declared but not effective" is not
/// even expressible for them.
fn collect(
    items: &[Item],
    procs: &mut BTreeMap<String, Option<BTreeSet<String>>>,
    externs: &mut BTreeSet<String>,
    falls: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            Item::Proc(p) => {
                let p: &ProcDecl = p;
                procs.insert(p.name.clone(), p.clobbers.as_ref().map(|v| expand(v)));
                if p.falls_into.is_some() {
                    falls.insert(p.name.clone());
                }
            }
            Item::ExternProc(e) => {
                externs.insert(e.name.clone());
            }
            Item::Section(s) => collect(&s.items, procs, externs, falls),
            _ => {}
        }
    }
}

fn main() {
    let aeon = std::env::var("AEON_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| sigil_harness::test_support::aeon_dir());
    if !aeon.exists() {
        eprintln!("FATAL: no reference tree at {}", aeon.display());
        std::process::exit(2);
    }
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    let files: Vec<_> =
        paths.iter().map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0).collect();

    let mut declared: BTreeMap<String, Option<BTreeSet<String>>> = BTreeMap::new();
    let mut externs: BTreeSet<String> = BTreeSet::new();
    let mut falls: BTreeSet<String> = BTreeSet::new();
    for f in &files {
        collect(&f.items, &mut declared, &mut externs, &mut falls);
    }
    println!("reference tree: {}", aeon.display());
    println!("files: {}   procs: {}   extern procs: {}", files.len(), declared.len(), externs.len());
    println!(
        "procs declaring clobbers: {}",
        declared.values().filter(|v| v.is_some()).count()
    );

    for (label, profile) in native::shipped_shapes() {
        let defines = native::shape_defines(&profile, &aeon).expect("shape defines");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, profile.game_module_prefix());
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: L1 bind errors"
        );
        let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);

        let mut over: Vec<(String, Vec<String>, usize)> = Vec::new();
        let mut top_procs = 0usize;
        // Two classes of over-declaration that are NOT a tight-contract
        // opportunity, counted apart so the residue is the real population:
        //  - `empty`: the proc's body writes NOTHING in this shape (a comptime
        //    `if DEBUG == 1` body compiled out), so its whole declared set reads
        //    as over-declared and describes the OTHER shape.
        //  - `falls`: the proc declares `falls_into SUCC`. This bucket is the
        //    standing witness that the fall-through EDGE is modelled: the closure
        //    charges the successor's whole effect to the falling proc (one of its
        //    node edges), so a proc landing here would mean a pair whose successor
        //    genuinely writes less than the head declares. It read 16 on every
        //    shape while the edge was missing, and reads 0 with it.
        let mut over_empty = 0usize;
        let mut over_falls = 0usize;
        for (name, decl) in &declared {
            let Some(decl) = decl else { continue };
            let Some(eff) = r.closure.effective.get(name) else { continue };
            if eff.top {
                top_procs += 1;
                continue; // ⊤: nothing is "not effective"
            }
            let diff: Vec<String> =
                decl.iter().filter(|d| !eff.regs.contains(*d)).cloned().collect();
            if !diff.is_empty() {
                if eff.regs.is_empty() {
                    over_empty += 1;
                } else if falls.contains(name) {
                    over_falls += 1;
                }
                over.push((name.clone(), diff, decl.len()));
            }
        }
        let pairs: usize = over.iter().map(|(_, d, _)| d.len()).sum();
        println!("\n== shape {label} ==");
        println!(
            "  PRODUCER  over-declaring procs: {} of {} declaring (⊤-effective procs, unmeasurable: {})",
            over.len(),
            declared.values().filter(|v| v.is_some()).count(),
            top_procs
        );
        println!("  PRODUCER  over-declared (proc, register) pairs: {pairs}");
        println!(
            "  PRODUCER  of those procs: {over_empty} write nothing in this shape, {over_falls} declare `falls_into` (expected 0: the successor's effect is charged to the head); residue = {}",
            over.len() - over_empty - over_falls
        );
        for (name, diff, declared_n) in &over {
            println!("    {name:<34} declares {declared_n:>2}, never writes {:>2}: {}", diff.len(), diff.join("/"));
        }
        println!("  CONSUMER  [proc.dead-save] firings: {}", r.dead_saves.len());
        for d in &r.dead_saves {
            println!("    {:<34} save {:?}  around {}", d.proc, d.reg, d.callees.join(", "));
        }
    }

    price();
}

/// What one save/restore costs to EXECUTE, priced by sigil's own 68000 timing
/// classifier (`m68k_cycles::instr_cost` over `sigil_isa::m68k_cycles`) — the
/// same table the `@budget` path walk charges. Byte counts are the encoding
/// forms: a single opcode word for the `move`/`movea` pair, opcode + mask word
/// for a `movem`.
fn price() {
    use sigil_frontend_emp::m68k_cycles::instr_cost;
    use sigil_frontend_emp::value::{CodeOperand as O, Reg, Width};

    // d0-d7/a0-a6 and the same minus one register, to price a movem's MARGINAL
    // register: shrinking a movem frees cycles but never a byte.
    let full: u16 = 0x7FFF;
    let minus_one: u16 = 0x7FFE;

    // One priced instruction: source text, byte length, operands, width, and the cost-table
    // family it is looked up under. Named because a five-field tuple is unreadable inline.
    type PriceRow = (&'static str, u32, Vec<O>, Option<Width>, &'static str);
    let rows: Vec<PriceRow> = vec![
        ("move.l  a0,-(sp)", 2, vec![O::Reg(Reg::A0), O::PreDec(Reg::A7)], Some(Width::L), "move"),
        ("movea.l (sp),a0", 2, vec![O::Ind(Reg::A7), O::Reg(Reg::A0)], Some(Width::L), "movea"),
        ("movea.l (sp)+,a0", 2, vec![O::PostInc(Reg::A7), O::Reg(Reg::A0)], Some(Width::L), "movea"),
        ("movem.l d0-d7/a0-a6,-(sp)", 4, vec![O::RegList(full), O::PreDec(Reg::A7)], Some(Width::L), "movem"),
        ("movem.l (sp)+,d0-d7/a0-a6", 4, vec![O::PostInc(Reg::A7), O::RegList(full)], Some(Width::L), "movem"),
        ("movem.l <14 regs>,-(sp)", 4, vec![O::RegList(minus_one), O::PreDec(Reg::A7)], Some(Width::L), "movem"),
        ("movem.l (sp)+,<14 regs>", 4, vec![O::PostInc(Reg::A7), O::RegList(minus_one)], Some(Width::L), "movem"),
    ];
    println!("\n== per-instruction price (sigil m68k_cycles) ==");
    for (label, bytes, ops, size, mnem) in rows {
        println!("  {label:<28} {bytes} bytes   {:?}", instr_cost(mnem, size, &ops));
    }
}
