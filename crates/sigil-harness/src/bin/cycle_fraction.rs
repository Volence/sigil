//! `cycle_fraction` — how much of the shipped cartridge's code the cycle-timing
//! model can speak about (queue item CYCLE-FRACTION, 2026-08-27).
//!
//! A REPORT TOOL, not a gate: it prints numbers and always exits 0 unless the
//! corpus itself failed to read. Nothing here changes model behaviour; the two
//! cost tables and the budget walk are consulted exactly as the compiler
//! consults them.
//!
//! ## What it enumerates
//!
//! Every `proc` in the `.emp` modules a shipped shape ([`native::shipped_shapes`])
//! actually places, evaluated under that shape's own `-D` set
//! ([`native::shape_defines`]) and its game's bound L1 interface env — the same
//! three inputs the per-shape corpus gates use, because a define-free walk cannot
//! see inside a `if DEBUG == 1 { }` arm and would describe code the ROM does not
//! carry.
//!
//! Each evaluated `CodeBuf` is walked twice:
//!
//!   * **Layer A — instruction pricing.** Every [`CodeItem::Instr`] is priced by
//!     its CPU's table ([`m68k_cycles::instr_cost`] / [`z80_cycles::instr_cost`])
//!     and bucketed EXACT / CEILING / UNMODELED.
//!   * **Layer B — whole-proc reach.** [`cycle_budget::check_cycle_budget`] is run
//!     with an unreachable ceiling (`u64::MAX`), so `[cycles.over-budget]` cannot
//!     fire and every finding is a REFUSAL: the proc could not carry a
//!     `@budget(cycles:)` at all. Run again with `exact: true` for the
//!     `@cycles_exact` reach.
//!
//! Layer A answers "can the model price this instruction"; Layer B answers "can
//! the model state a bound for this proc". They are different questions and they
//! do not agree — which is the point of reporting both.

use sigil_frontend_emp::ast::{self, File as EmpFile, Item};
use sigil_frontend_emp::corpus_contracts::bind_corpus_interfaces;
use sigil_frontend_emp::eval::eval_proc_body_env;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, CodeOperand, Width};
use sigil_frontend_emp::{cycle_budget, m68k_cycles, z80_cycles};
use sigil_harness::native;
use sigil_ir::backend::Cpu;
use sigil_isa::m68k_cycles::CycleCost;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The three buckets. See the note for the exact definitions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Bucket {
    /// The table states a per-execution count it claims is the machine's.
    Exact,
    /// The table states an upper bound only (`exact: false`).
    Ceiling,
    /// The table refuses: no cost is assignable.
    Unmodeled,
}

/// One CPU's Layer-A tallies.
#[derive(Default, Clone)]
struct Tally {
    /// Instruction sites per bucket (denominator D1).
    instrs: BTreeMap<Bucket, u64>,
    /// Sub-counts, so a reader who disagrees with the branch ruling can re-derive.
    fixed_exact: u64,
    branch_exact: u64,
    fixed_ceiling: u64,
    branch_ceiling: u64,
    /// Procs per bucket, worst-bucket-wins (denominator D2).
    procs: BTreeMap<Bucket, u64>,
    /// Unmodeled mnemonics, counted.
    unmodeled: BTreeMap<String, u64>,
    /// Ceiling-priced mnemonics, counted.
    ceiling_by_mnemonic: BTreeMap<String, u64>,
    /// Exact-priced mnemonics, counted — the defect-hunt instrument: a
    /// data-dependent form appearing here is a number where the model owes a
    /// ceiling.
    exact_by_mnemonic: BTreeMap<String, u64>,
    /// The ceiling bucket split by cause (68000 only; the Z80 table has no
    /// inexact variant at all).
    ceiling_relax_only: u64,
    ceiling_data_dependent: u64,
    /// Procs with at least one instruction.
    procs_total: u64,
    /// Layer-B: procs with NO refusal under `@budget`, and under `@cycles_exact`.
    budgetable: u64,
    exactable: u64,
    /// Layer-B refusal reasons (first finding per proc), counted.
    refusals: BTreeMap<String, u64>,
    /// Procs that DECLARE `@budget(cycles:)` / `@cycles_exact` today — the reach
    /// actually claimed, against the reach available.
    declared: u64,
    /// The largest single proc, in instruction sites. Bounds the T-state total a
    /// `cycles(L1, L2)` straight-line span could ever reach in this corpus.
    max_instrs: u64,
    /// Example proc names per Layer-B refusal id, so a reader can re-derive the
    /// classification by opening the source rather than trusting the count.
    refusal_examples: BTreeMap<String, Vec<String>>,
    /// Example proc names per unmodeled mnemonic, same purpose.
    unmodeled_examples: BTreeMap<String, Vec<String>>,
}

/// Record `name` under `key`, keeping at most six examples.
fn note_example(map: &mut BTreeMap<String, Vec<String>>, key: &str, name: &str) {
    let v = map.entry(key.to_string()).or_default();
    if v.len() < 6 && !v.iter().any(|n| n == name) {
        v.push(name.to_string());
    }
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        for (k, v) in &other.instrs {
            *self.instrs.entry(*k).or_default() += v;
        }
        for (k, v) in &other.procs {
            *self.procs.entry(*k).or_default() += v;
        }
        for (k, v) in &other.unmodeled {
            *self.unmodeled.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &other.ceiling_by_mnemonic {
            *self.ceiling_by_mnemonic.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &other.exact_by_mnemonic {
            *self.exact_by_mnemonic.entry(k.clone()).or_default() += v;
        }
        self.ceiling_relax_only += other.ceiling_relax_only;
        self.ceiling_data_dependent += other.ceiling_data_dependent;
        for (k, v) in &other.refusals {
            *self.refusals.entry(k.clone()).or_default() += v;
        }
        self.fixed_exact += other.fixed_exact;
        self.branch_exact += other.branch_exact;
        self.fixed_ceiling += other.fixed_ceiling;
        self.branch_ceiling += other.branch_ceiling;
        self.procs_total += other.procs_total;
        self.declared += other.declared;
        self.max_instrs = self.max_instrs.max(other.max_instrs);
        self.budgetable += other.budgetable;
        self.exactable += other.exactable;
    }
    fn instr_total(&self) -> u64 {
        self.instrs.values().sum()
    }
}

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

/// `(cpu: z80)` on a module or a section header — mirrors `attr_cpu` in
/// `lower/mod.rs` and `module_is_z80` in `corpus_contracts`.
fn attrs_are_z80(attrs: &[(String, ast::Expr)]) -> bool {
    attrs.iter().any(|(name, expr)| {
        name == "cpu"
            && matches!(expr, ast::Expr::Path(p)
                if p.segments.last().is_some_and(|s| s.eq_ignore_ascii_case("z80")))
    })
}

/// PASS-1 corpus type environment — the declaration items the evaluator resolves
/// names against. A transliteration of `corpus_contracts::collect_env` (private
/// there); the item list is the same.
fn collect_env(items: &[Item], out: &mut Vec<Item>) {
    for item in items {
        match item {
            Item::Const(_)
            | Item::Equ(_)
            | Item::Enum(_)
            | Item::Bitfield(_)
            | Item::Struct(_)
            | Item::Offsets(_)
            | Item::Table(_)
            | Item::Dispatch(_)
            | Item::Vars(_)
            | Item::Data(_)
            | Item::ComptimeFn(_)
            | Item::Newtype(_)
            | Item::Context(_) => out.push(item.clone()),
            Item::Section(s) => collect_env(&s.items, out),
            _ => {}
        }
    }
}

/// Every proc in a file, paired with the CPU its enclosing section/module names.
fn procs_with_cpu<'a>(
    items: &'a [Item],
    cpu: Cpu,
    out: &mut Vec<(&'a ast::ProcDecl, Cpu)>,
) {
    for item in items {
        match item {
            Item::Proc(p) => out.push((p, cpu)),
            Item::Section(s) => {
                let inner = if attrs_are_z80(&s.attrs) { Cpu::Z80 } else { cpu };
                procs_with_cpu(&s.items, inner, out);
            }
            _ => {}
        }
    }
}

/// Is this 68000 ceiling charge purely the LINKER-RELAXATION ruling, or is the
/// underlying form data-dependent as well?
///
/// The discriminator is a re-price with the relaxation removed: every bare
/// symbolic operand is replaced by the SAME address written `.l` (`AbsSym { long:
/// true }` — the rung the ceiling already charges, so the NUMBER cannot move), and
/// an unsized conditional is given the `.w` width whose fall-through the ceiling
/// already charges. If the re-priced form is exact, the only thing the model did
/// not know was a width the linker will pick; if it is still inexact, the cost
/// varies with operand VALUES and no width pin would recover it.
///
/// `jbra`/`jra`/`jbsr` are relaxation by construction: the front end prices their
/// four-rung ladder before the ISA table is consulted, so there is no pinned form
/// to re-price into. They are the emp auto-reaching spellings of `bra`/`bsr`,
/// which price exactly at a pinned width.
fn ceiling_is_relaxation_only(mnemonic: &str, size: Option<Width>, ops: &[CodeOperand]) -> bool {
    if matches!(mnemonic, "jbra" | "jra" | "jbsr") {
        return true;
    }
    let pinned: Vec<CodeOperand> = ops
        .iter()
        .map(|o| match o {
            CodeOperand::Sym(s) => CodeOperand::AbsSym { target: s.clone(), long: true },
            CodeOperand::SymOff { sym, .. } => {
                CodeOperand::AbsSym { target: sym.clone(), long: true }
            }
            other => other.clone(),
        })
        .collect();
    // An absent width on a conditional branch IS the relax ladder, not a default,
    // so the pinned re-price supplies the dearer rung's width.
    let is_cond_branch = mnemonic.len() == 3
        && mnemonic.starts_with('b')
        && !matches!(mnemonic, "bra" | "bsr" | "bkp");
    let size = if size.is_none() && is_cond_branch { Some(Width::W) } else { size };
    matches!(
        m68k_cycles::instr_cost(mnemonic, size, &pinned),
        CycleCost::Fixed { exact: true, .. } | CycleCost::Branch { exact: true, .. }
    )
}

/// Layer A: one instruction's bucket, plus which table shape produced it.
fn price(cpu: Cpu, it: &CodeItem) -> Option<(Bucket, &'static str)> {
    let CodeItem::Instr { mnemonic, size, ops, .. } = it else { return None };
    Some(match cpu {
        Cpu::Z80 => match z80_cycles::instr_cost(mnemonic, ops) {
            z80_cycles::Cost::Fixed(_) => (Bucket::Exact, "fixed"),
            z80_cycles::Cost::Split { .. } => (Bucket::Exact, "branch"),
            z80_cycles::Cost::Unknown => (Bucket::Unmodeled, "refused"),
        },
        _ => match m68k_cycles::instr_cost(mnemonic, *size, ops) {
            CycleCost::Fixed { exact: true, .. } => (Bucket::Exact, "fixed"),
            CycleCost::Branch { exact: true, .. } => (Bucket::Exact, "branch"),
            CycleCost::Fixed { exact: false, .. } => (Bucket::Ceiling, "fixed"),
            CycleCost::Branch { exact: false, .. } => (Bucket::Ceiling, "branch"),
            CycleCost::Unmodeled => (Bucket::Unmodeled, "refused"),
        },
    })
}

fn pct(n: u64, d: u64) -> String {
    if d == 0 {
        return "   n/a".to_string();
    }
    format!("{:6.2}", 100.0 * n as f64 / d as f64)
}

fn bucket_row(label: &str, t: &Tally) -> String {
    let d1 = t.instr_total();
    let d2: u64 = t.procs.values().sum();
    let g = |b: Bucket| *t.instrs.get(&b).unwrap_or(&0);
    let p = |b: Bucket| *t.procs.get(&b).unwrap_or(&0);
    format!(
        "{label:<18} D1 instrs {d1:>7} | exact {:>7} ({}%)  ceiling {:>6} ({}%)  unmodeled {:>6} ({}%)\n\
         {:<18} D2 procs  {d2:>7} | exact {:>7} ({}%)  ceiling {:>6} ({}%)  unmodeled {:>6} ({}%)",
        g(Bucket::Exact),
        pct(g(Bucket::Exact), d1),
        g(Bucket::Ceiling),
        pct(g(Bucket::Ceiling), d1),
        g(Bucket::Unmodeled),
        pct(g(Bucket::Unmodeled), d1),
        "",
        p(Bucket::Exact),
        pct(p(Bucket::Exact), d2),
        p(Bucket::Ceiling),
        pct(p(Bucket::Ceiling), d2),
        p(Bucket::Unmodeled),
        pct(p(Bucket::Unmodeled), d2),
    )
}

fn main() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        eprintln!("error: no aeon tree at {} (set AEON_DIR)", aeon.display());
        std::process::exit(2);
    }
    // `SIGIL_CF_CARRIERS=1` defines the seam-1 banked `$8000` carriers as comptime
    // values while lowering the resident sound modules. OFF by default, matching the
    // blob EMIT: in the shipped build those symbols stay symbolic until link, and an
    // operand's SHAPE is what the cost table prices. The flag exists so the run can
    // be repeated under the other polarity and the answer compared — a bucket split
    // that moves is an instrument sensitivity, not a fact about the ROM.
    let carriers = std::env::var("SIGIL_CF_CARRIERS").is_ok();
    println!("banked carriers defined: {carriers}");

    // Whole-corpus parse, once. The bind env and the `@noreturn` set both need
    // every file, and the registry filter is applied to the WALK, not the parse.
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    let files: Vec<(PathBuf, EmpFile)> = paths
        .iter()
        .map(|p| (p.clone(), parse_str(&std::fs::read_to_string(p).unwrap()).0))
        .collect();
    let just_files: Vec<EmpFile> = files.iter().map(|(_, f)| f.clone()).collect();
    println!("corpus: {} .emp files under {}", files.len(), aeon.display());

    let noreturn: BTreeSet<String> = just_files
        .iter()
        .flat_map(|f| f.items.iter())
        .filter_map(|i| match i {
            Item::Proc(p) if p.is_noreturn() => Some(p.name.clone()),
            Item::ExternProc(e) if e.is_noreturn() => Some(e.name.clone()),
            _ => None,
        })
        .collect();

    // Module id -> parsed file.
    let by_id: BTreeMap<String, &EmpFile> = files
        .iter()
        .map(|(_, f)| (f.module.path.segments.join("."), f))
        .collect();

    let mut grand_68k = Tally::default();
    let mut grand_z80 = Tally::default();
    let mut per_shape: Vec<(String, Tally, Tally, usize, usize, usize, usize)> = Vec::new();

    for (label, profile) in native::shipped_shapes() {
        let defines = native::shape_defines(&profile, &aeon).expect("shape defines");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&just_files, &defines, profile.game_module_prefix());
        let bind_errs =
            bind_diags.iter().filter(|d| d.level == sigil_span::Level::Error).count();
        assert_eq!(bind_errs, 0, "shape `{label}`: {bind_errs} L1 bind errors");

        // THE SHIPPED MODULE SET: the shape's own placement registry, plus the
        // game RAM / manifest modules the synthetic entry pulls in. A module not
        // in this set contributes no bytes to this shape's ROM.
        let mut want: BTreeSet<String> =
            profile.registry.iter().map(|m| m.module_id.to_string()).collect();
        want.insert(profile.game_ram_module.to_string());
        want.insert(profile.manifest_module.to_string());
        let mut missing: Vec<String> = Vec::new();
        let mut walked: Vec<&EmpFile> = Vec::new();
        for id in &want {
            match by_id.get(id) {
                Some(f) => walked.push(f),
                None => missing.push(id.clone()),
            }
        }
        assert!(missing.is_empty(), "shape `{label}`: registry ids not in corpus: {missing:?}");

        // The ambient type environment is the WHOLE corpus (pass 1), matching
        // `analyze_corpus_with_contracts`: a walked module's field operands may
        // reference an imported struct that lives in a non-registry helper.
        let mut env: Vec<Item> = Vec::new();
        for f in &just_files {
            collect_env(&f.items, &mut env);
        }

        let mut t68 = Tally::default();
        let mut tz = Tally::default();
        let mut unresolved_bodies = 0usize;
        let mut dropped_instrs = 0usize;
        let mut empty_bodies = 0usize;
        let mut unresolved_conds = 0usize;
        let mut counter: u32 = 0;

        // THE SEAM-1 HALF. The five resident Z80 sound modules are linked by seam 1,
        // not placed by the registry, so a registry-only walk sees no Z80 code in any
        // sound-on shape. Each carries its own `-D` env (const seam + DEBUG), which is
        // why they ride a separate list rather than the shape's define set.
        let resident: Vec<(&'static str, EmpFile, Vec<(String, i128)>)> = if profile.sound_on {
            sigil_harness::seam1::resident_sound_modules(&aeon, profile.debug, carriers)
        } else {
            Vec::new()
        };
        // `(file, its -D env, its ambient declaration env)`. A registry module reads
        // the whole-corpus type environment (its field operands may name an imported
        // struct); a resident sound module is lowered standalone by seam 1 and gets
        // none, matching `lower_one`.
        let mut units: Vec<(&EmpFile, &[(String, i128)], &[Item])> =
            walked.iter().map(|f| (*f, &defines[..], &env[..])).collect();
        for (_rel, f, d) in &resident {
            units.push((f, &d[..], &[]));
        }

        for (f, defines, ambient) in &units {
            let mcpu = if attrs_are_z80(&f.module.attrs) { Cpu::Z80 } else { Cpu::M68000 };
            let mut procs = Vec::new();
            procs_with_cpu(&f.items, mcpu, &mut procs);
            for (p, cpu) in procs {
                let (buf, _d, next, dropped, unres) = eval_proc_body_env(
                    f,
                    &p.name,
                    &p.params,
                    &p.body,
                    p.span,
                    counter,
                    cpu,
                    defines,
                    ambient,
                    &iface_env,
                );
                unresolved_conds += unres.len();
                counter = next;
                dropped_instrs += dropped;
                let Some(buf) = buf else {
                    unresolved_bodies += 1;
                    continue;
                };
                let t = if cpu == Cpu::Z80 { &mut tz } else { &mut t68 };

                let mut worst = Bucket::Exact;
                let mut n_instr = 0u64;
                for it in &buf.items {
                    let Some((b, shape)) = price(cpu, it) else { continue };
                    n_instr += 1;
                    *t.instrs.entry(b).or_default() += 1;
                    let CodeItem::Instr { mnemonic, .. } = it else { unreachable!() };
                    match (b, shape) {
                        (Bucket::Exact, "fixed") => t.fixed_exact += 1,
                        (Bucket::Exact, "branch") => t.branch_exact += 1,
                        (Bucket::Ceiling, "fixed") => t.fixed_ceiling += 1,
                        (Bucket::Ceiling, "branch") => t.branch_ceiling += 1,
                        _ => {}
                    }
                    if b == Bucket::Unmodeled {
                        *t.unmodeled.entry(mnemonic.clone()).or_default() += 1;
                        note_example(&mut t.unmodeled_examples, mnemonic, &p.name);
                    }
                    if b == Bucket::Exact {
                        *t.exact_by_mnemonic.entry(mnemonic.clone()).or_default() += 1;
                    }
                    if b == Bucket::Ceiling {
                        *t.ceiling_by_mnemonic.entry(mnemonic.clone()).or_default() += 1;
                        let CodeItem::Instr { size, ops, .. } = it else { unreachable!() };
                        if ceiling_is_relaxation_only(mnemonic, *size, ops) {
                            t.ceiling_relax_only += 1;
                        } else {
                            t.ceiling_data_dependent += 1;
                        }
                    }
                    if b > worst {
                        worst = b;
                    }
                }
                if n_instr == 0 {
                    empty_bodies += 1;
                    continue;
                }
                t.procs_total += 1;
                t.max_instrs = t.max_instrs.max(n_instr);
                if p.attrs.iter().any(|a| a.name == "budget" || a.name == "cycles_exact") {
                    t.declared += 1;
                }
                *t.procs.entry(worst).or_default() += 1;

                // LAYER B — the whole-proc reach. `u64::MAX` makes `over-budget`
                // unreachable, so every finding is a structural refusal.
                let fs = cycle_budget::check_cycle_budget(
                    &buf.items,
                    cpu,
                    p.span,
                    Some(u64::MAX),
                    false,
                    &noreturn,
                );
                if fs.is_empty() {
                    t.budgetable += 1;
                } else {
                    let id = fs[0].kind.lint_id().to_string();
                    *t.refusals.entry(id.clone()).or_default() += 1;
                    note_example(&mut t.refusal_examples, &id, &p.name);
                }
                let fe = cycle_budget::check_cycle_budget(
                    &buf.items,
                    cpu,
                    p.span,
                    Some(u64::MAX),
                    true,
                    &noreturn,
                );
                // `path-mismatch` is a VERDICT (the paths differ), not a refusal:
                // the model spoke. Only a refusal means it could not.
                if fe.iter().all(|f| f.kind.lint_id() == "cycles.path-mismatch") {
                    t.exactable += 1;
                }
            }
        }

        println!("\n================ shape: {label} ================");
        println!(
            "modules walked {} (registry {} + seam-1 resident {})  |  proc bodies unresolved \
             {unresolved_bodies}  |  dropped instrs {dropped_instrs}  |  unresolved comptime \
             conds {unresolved_conds}  |  zero-instruction bodies {empty_bodies}",
            units.len(),
            walked.len(),
            resident.len()
        );
        println!("{}", bucket_row("  68000", &t68));
        println!(
            "    sub: fixed-exact {} branch-exact {} fixed-ceiling {} branch-ceiling {}",
            t68.fixed_exact, t68.branch_exact, t68.fixed_ceiling, t68.branch_ceiling
        );
        println!(
            "    ceiling cause: relaxation-only {} ({}%)  data-dependent {} ({}%)",
            t68.ceiling_relax_only,
            pct(t68.ceiling_relax_only, t68.ceiling_relax_only + t68.ceiling_data_dependent),
            t68.ceiling_data_dependent,
            pct(t68.ceiling_data_dependent, t68.ceiling_relax_only + t68.ceiling_data_dependent),
        );
        println!(
            "    layer B: budgetable {}/{}  cycles_exact-able {}/{}",
            t68.budgetable, t68.procs_total, t68.exactable, t68.procs_total
        );
        println!(
            "    declared today {}  |  largest proc {} instrs",
            t68.declared, t68.max_instrs
        );
        println!("{}", bucket_row("  Z80", &tz));
        println!(
            "    sub: fixed-exact {} branch-exact {} fixed-ceiling {} branch-ceiling {}",
            tz.fixed_exact, tz.branch_exact, tz.fixed_ceiling, tz.branch_ceiling
        );
        println!(
            "    layer B: budgetable {}/{}  cycles_exact-able {}/{}",
            tz.budgetable, tz.procs_total, tz.exactable, tz.procs_total
        );
        println!(
            "    declared today {}  |  largest proc {} instrs",
            tz.declared, tz.max_instrs
        );
        let mut both = t68.clone();
        both.add(&tz);
        println!("{}", bucket_row("  COMBINED", &both));

        grand_68k.add(&t68);
        grand_z80.add(&tz);
        per_shape.push((
            label.to_string(),
            t68,
            tz,
            walked.len(),
            unresolved_bodies,
            dropped_instrs,
            empty_bodies,
        ));
    }

    // The canonical shape alone (sonic4 debug is the widest: every arm on).
    println!("\n================ detail: unmodeled + ceiling mnemonics, canonical shape ================");
    for (label, t68, tz, ..) in &per_shape {
        if label != "sonic4 debug" {
            continue;
        }
        for (cpu, t) in [("68000", t68), ("Z80", tz)] {
            let mut u: Vec<_> = t.unmodeled.iter().collect();
            u.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            println!("{cpu} UNMODELED ({} distinct mnemonics):", u.len());
            for (m, n) in &u {
                let ex = t.unmodeled_examples.get(*m).map(|v| v.join(", ")).unwrap_or_default();
                println!("    {n:>6}  {m}   e.g. {ex}");
            }
            let mut c: Vec<_> = t.ceiling_by_mnemonic.iter().collect();
            c.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            println!("{cpu} CEILING ({} distinct mnemonics):", c.len());
            for (m, n) in &c {
                println!("    {n:>6}  {m}");
            }
            let mut x: Vec<_> = t.exact_by_mnemonic.iter().collect();
            x.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            println!("{cpu} EXACT ({} distinct mnemonics):", x.len());
            for (m, n) in &x {
                println!("    {n:>6}  {m}");
            }
            let mut r: Vec<_> = t.refusals.iter().collect();
            r.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            println!("{cpu} LAYER-B refusals (first finding per proc):");
            for (m, n) in &r {
                let ex = t.refusal_examples.get(*m).map(|v| v.join(", ")).unwrap_or_default();
                println!("    {n:>6}  {m}   e.g. {ex}");
            }
        }
    }

    println!("\n================ ALL SHAPES SUMMED (proc-instances, not distinct procs) ================");
    println!("{}", bucket_row("68000", &grand_68k));
    println!("{}", bucket_row("Z80", &grand_z80));
    let mut all = grand_68k.clone();
    all.add(&grand_z80);
    println!("{}", bucket_row("COMBINED", &all));
}
