//! Contract-grammar v2 — the whole-corpus contract walk that feeds the
//! transitive closure ([`crate::closure`]).
//!
//! The closure is a pure algorithm over a name-keyed [`ProcNode`] map; THIS
//! module builds that map from the parsed `.emp` corpus (the §11 Q2 decision: a
//! whole-corpus FRONTEND pass, name-resolved, not a post-link pass — so it
//! reuses the real write detector [`crate::lower::proc_written_registers`] with
//! no drift, and source spans stay native). For each proc it derives:
//!
//! - **local writes** — from the proc's evaluated [`CodeBuf`] (same substrate as
//!   `emp_census`; `a7` stack-discipline filtered exactly as the census does),
//! - **direct callees** — the `Sym` targets of `jsr`/`bsr`/`jbsr` (calls, whose
//!   unresolved names are holes) and of `jmp`/`bra`/`jbra` (tail transfers, kept
//!   only when the target is a known proc so a local-label branch adds no edge),
//! - **indirect sites** — from the AST body: each `jsr (aN) [as Type]` call site
//!   contributes its declared bound (`Some(type)`) or `None` (⊤).
//!
//! Externs (§3) become closure leaves; contract types (§4) become clobber
//! bounds. The report also flags the §11 Q4 collision (a name declared BOTH as
//! `extern proc` and `proc`).

use crate::ast::{self, AsmStmt, ContractTypeDecl, ExternProcDecl, InstrLine, Item, Operand, ProcDecl, ProcSig, TextOrSplice};
use crate::closure::{check_firings, compute_closure, Closure, Firing, ProcNode, RegEffect};
use crate::flag_check::{check_flag_unused, check_result_invalid_path, FlagFiring};
use crate::lower::{expand_reglist_regs, proc_written_registers, verified_preserves_regs};
use crate::preserves::{find_dead_saves, DeadSave};
use crate::value::{CodeBuf, CodeItem, CodeOperand, Reg};
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet};

/// The register file the closure tracks — `d0`..`d7` + `a0`..`a6` (`a7`/sp is
/// stack discipline, never a clobber). This is the universe ⊤ ranges over and
/// the set a "preserves-only" contract type clobbers the complement of.
fn universe() -> BTreeSet<String> {
    (0..8).map(|n| format!("d{n}")).chain((0..7).map(|n| format!("a{n}"))).collect()
}

/// Mnemonics that CALL: an unresolved target is a hole (a missing `extern proc`).
const CALL_MNEMONICS: [&str; 3] = ["jsr", "bsr", "jbsr"];
/// Mnemonics that TAIL-TRANSFER: the target's effects become the caller's, but
/// an unresolved target (a local `.loop` label) is NOT a hole — so these edges
/// are kept only when the target resolves to a known proc.
const TAIL_MNEMONICS: [&str; 3] = ["jmp", "bra", "jbra"];

/// The corpus-wide contract analysis result.
#[derive(Debug, Default)]
pub struct ContractReport {
    /// Every proc's transitive `effective` clobber set.
    pub closure: Closure,
    /// The transitive under-declaration firings (§9), sorted (proc, reg).
    pub firings: Vec<Firing>,
    /// The §6 caller-side flag-result firings: `[call.flag-result-unused]` (a
    /// carry result abandoned on some path) and `[call.result-invalid-path]` (a
    /// conditional register result read on its invalid path), sorted (proc,
    /// callee, span).
    pub flag_firings: Vec<FlagFiring>,
    /// Names declared BOTH `extern proc` and `proc` (§11 Q4) — with the extern's
    /// span (the mirror that should be deleted when the callee ports).
    pub extern_collisions: Vec<(String, Span)>,
    /// How many procs (incl. externs) the walk collected.
    pub proc_count: usize,
    /// How many extern-proc leaves.
    pub extern_count: usize,
    /// How many contract types.
    pub contract_type_count: usize,
    /// The §6/D1d `[proc.dead-save]` firings: a verified save/restore of a
    /// register the bracketed callee (per the closure's VERIFIED `effective`
    /// set) provably preserves — the pass-3 dead-save worklist. Sorted
    /// (proc, reg, span).
    pub dead_saves: Vec<DeadSave>,
}

/// Analyze the parsed corpus with the canonical no-`-D` config (census-parity).
pub fn analyze_corpus(files: &[ast::File]) -> ContractReport {
    analyze_corpus_with(files, &[])
}

/// Analyze the parsed corpus under the given comptime `-D` defines: build the
/// proc/extern/contract-type maps, run the closure, and collect firings +
/// collisions. Comptime-`if` gating is config-sensitive, so the defines choose
/// which code paths lower (the plain canonical build is `SOUND_DRIVER_ENABLED=1`;
/// the census — and `analyze_corpus` — use no defines).
pub fn analyze_corpus_with(files: &[ast::File], defines: &[(String, i128)]) -> ContractReport {
    let mut nodes: BTreeMap<String, ProcNode> = BTreeMap::new();
    let mut types: BTreeMap<String, RegEffect> = BTreeMap::new();
    let mut extern_names: BTreeSet<String> = BTreeSet::new();
    let mut proc_names: BTreeSet<String> = BTreeSet::new();
    let mut extern_spans: BTreeMap<String, Span> = BTreeMap::new();
    let mut counter: u32 = 0;
    // §6 flag-result wiring: the flag / conditional-result contracts a callee
    // declares, keyed by name, plus each proc's evaluated CodeBuf + the call
    // sites carrying `@discards` (the caller-side check needs cross-module
    // contract knowledge, so it runs after the whole corpus is walked).
    let mut flag_callees: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut cond_callees: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut proc_bufs: Vec<ProcBuf> = Vec::new();

    for file in files {
        collect_items(
            &file.items,
            file,
            &mut nodes,
            &mut types,
            &mut extern_names,
            &mut proc_names,
            &mut extern_spans,
            &mut counter,
            &mut flag_callees,
            &mut cond_callees,
            &mut proc_bufs,
            defines,
        );
    }

    // §11 Q4: a name declared both `extern proc` and `proc` collides.
    let mut extern_collisions: Vec<(String, Span)> = extern_names
        .intersection(&proc_names)
        .map(|n| (n.clone(), extern_spans[n]))
        .collect();
    extern_collisions.sort_by(|a, b| a.0.cmp(&b.0));

    let extern_count = extern_names.len();
    let contract_type_count = types.len();
    let proc_count = nodes.len();

    let closure = compute_closure(&nodes, &types);
    let firings = check_firings(&nodes, &closure);

    // §6 caller-side flag checks, now that every callee's contract is known.
    let mut flag_firings: Vec<FlagFiring> = Vec::new();
    for pb in &proc_bufs {
        flag_firings.extend(check_flag_unused(&pb.name, &pb.buf.items, &flag_callees, &pb.discarded));
        flag_firings.extend(check_result_invalid_path(&pb.name, &pb.buf.items, &cond_callees));
    }
    // Deterministic order (proc, callee, flag); spans stay in encounter order
    // via the stable sort.
    flag_firings.sort_by(|a, b| {
        (&a.proc, &a.callee, &a.flag).cmp(&(&b.proc, &b.callee, &b.flag))
    });

    // D1d dead-save worklist: run over every proc's CodeBuf against the closure's
    // VERIFIED effective sets (never raw declared text — pass-3 cuts code on this).
    let mut dead_saves: Vec<DeadSave> = Vec::new();
    for pb in &proc_bufs {
        dead_saves.extend(find_dead_saves(&pb.name, &pb.buf.items, &closure.effective));
    }
    dead_saves.sort_by(|a, b| {
        (&a.proc, a.reg, a.span.start).cmp(&(&b.proc, b.reg, b.span.start))
    });

    ContractReport {
        closure,
        firings,
        flag_firings,
        extern_collisions,
        proc_count,
        extern_count,
        contract_type_count,
        dead_saves,
    }
}

/// A proc's evaluated CodeBuf + the call-site spans carrying `@discards`, held
/// for the §6 caller-side flag checks (run after the whole corpus is walked so
/// every callee's flag/conditional contract is known).
struct ProcBuf {
    name: String,
    buf: CodeBuf,
    discarded: Vec<Span>,
}

/// The set of status flags a decl's `out(carry: name)` clauses name.
fn flags_of(out_flags: &[ast::FlagResult]) -> BTreeSet<String> {
    out_flags.iter().map(|f| f.flag.clone()).collect()
}

/// The `(reg, cc)` pairs a decl's `out(rN if cc)` clauses name.
fn conds_of(out_cond: &[ast::CondResult]) -> Vec<(String, String)> {
    out_cond.iter().map(|c| (c.reg.clone(), c.cc.clone())).collect()
}

/// The spans of a proc body's call instructions carrying `@discards` (recursing
/// comptime-`if` branches, like [`collect_indirect_sites`]). A `@discards` inside
/// a comptime-fn template body is not seen (the AST-body limitation the walk
/// already carries for indirect sites); no corpus call site discards today.
fn collect_discarded(body: &[AsmStmt], out: &mut Vec<Span>) {
    for stmt in body {
        match stmt {
            AsmStmt::Instr(i) if i.discards.is_some() => out.push(i.span),
            AsmStmt::If { then, els, .. } => {
                collect_discarded(then, out);
                if let Some(e) = els {
                    collect_discarded(e, out);
                }
            }
            _ => {}
        }
    }
}

/// Recurse the item list (into `section {}` blocks), registering every proc /
/// extern proc / contract type.
#[allow(clippy::too_many_arguments)]
fn collect_items(
    items: &[Item],
    file: &ast::File,
    nodes: &mut BTreeMap<String, ProcNode>,
    types: &mut BTreeMap<String, RegEffect>,
    extern_names: &mut BTreeSet<String>,
    proc_names: &mut BTreeSet<String>,
    extern_spans: &mut BTreeMap<String, Span>,
    counter: &mut u32,
    flag_callees: &mut BTreeMap<String, BTreeSet<String>>,
    cond_callees: &mut BTreeMap<String, Vec<(String, String)>>,
    proc_bufs: &mut Vec<ProcBuf>,
    defines: &[(String, i128)],
) {
    for item in items {
        match item {
            Item::Proc(p) => {
                proc_names.insert(p.name.clone());
                let (node, buf) = proc_node(p, file, counter, defines);
                nodes.insert(p.name.clone(), node);
                // §6 flag / conditional contracts this proc exposes to callers.
                let flags = flags_of(&p.out_flags);
                if !flags.is_empty() {
                    flag_callees.insert(p.name.clone(), flags);
                }
                let conds = conds_of(&p.out_cond);
                if !conds.is_empty() {
                    cond_callees.insert(p.name.clone(), conds);
                }
                // Stash the CodeBuf + discard sites for the post-walk checks.
                if let Some(buf) = buf {
                    let mut discarded = Vec::new();
                    collect_discarded(&p.body, &mut discarded);
                    proc_bufs.push(ProcBuf { name: p.name.clone(), buf, discarded });
                }
            }
            Item::ExternProc(e) => {
                extern_names.insert(e.name.clone());
                extern_spans.insert(e.name.clone(), e.span);
                nodes.insert(e.name.clone(), extern_node(e));
                let flags = flags_of(&e.sig.out_flags);
                if !flags.is_empty() {
                    flag_callees.insert(e.name.clone(), flags);
                }
                let conds = conds_of(&e.sig.out_cond);
                if !conds.is_empty() {
                    cond_callees.insert(e.name.clone(), conds);
                }
            }
            Item::ContractType(t) => {
                types.insert(t.name.clone(), contract_type_bound(t));
            }
            Item::Section(s) => collect_items(
                &s.items, file, nodes, types, extern_names, proc_names, extern_spans, counter,
                flag_callees, cond_callees, proc_bufs, defines,
            ),
            _ => {}
        }
    }
}

/// Build a [`ProcNode`] from a body-bearing `proc` decl, returning the evaluated
/// CodeBuf too (for the §6 caller-side flag checks).
fn proc_node(
    p: &ProcDecl,
    file: &ast::File,
    counter: &mut u32,
    defines: &[(String, i128)],
) -> (ProcNode, Option<CodeBuf>) {
    let (buf, _diags, next) =
        crate::eval::eval_proc_body(file, &p.name, &p.params, &p.body, p.span, *counter, Cpu::M68000, defines);
    *counter = next;

    let mut local_writes = BTreeSet::new();
    let mut direct_callees = Vec::new();
    let mut verified_preserves = BTreeSet::new();
    if let Some(buf) = &buf {
        // Local writes — `a7` filtered as stack discipline (census caveat 5).
        local_writes = proc_written_registers(buf).into_iter().filter(|r| r != "a7").collect();
        // Provably-preserved registers (declared + D2.32 movem-verified).
        verified_preserves = verified_preserves_regs(p, buf);
        // Direct-call edges from the resolved CodeBuf (post-comptime accurate).
        for it in &buf.items {
            if let CodeItem::Instr { mnemonic, ops, .. } = it {
                if let Some(target) = call_target_sym(ops) {
                    if CALL_MNEMONICS.contains(&mnemonic.as_str())
                        || TAIL_MNEMONICS.contains(&mnemonic.as_str())
                    {
                        direct_callees.push(target);
                    }
                }
            }
        }
    }

    let node = ProcNode {
        local_writes,
        direct_callees,
        indirect_sites: collect_indirect_sites(&p.body),
        is_extern: false,
        declared_clobbers: expand_reglist_regs(p.clobbers.as_deref().unwrap_or(&[])),
        params: param_regs_typed(&p.params),
        out: expand_reglist_regs(p.out.as_deref().unwrap_or(&[])),
        has_clobber_contract: p.clobbers.is_some(),
        verified_preserves,
    };
    (node, buf)
}

/// Build a leaf [`ProcNode`] from an `extern proc` decl (§3). The leaf's
/// effective clobber set is `clobbers ∪ out`: an `out` register is WRITTEN by
/// the callee (an advanced in-out cursor like S4LZ's `a1`), so a caller relying
/// on it across the call is wrong and must be charged it — exactly as a
/// body-bearing proc's `local_writes` already includes its out-register writes.
fn extern_node(e: &ExternProcDecl) -> ProcNode {
    let out = expand_reglist_regs(e.sig.out.as_deref().unwrap_or(&[]));
    let mut effective = sig_clobbers(&e.sig);
    effective.extend(out.iter().cloned());
    ProcNode {
        is_extern: true,
        declared_clobbers: effective,
        params: sig_param_regs(&e.sig),
        out,
        has_clobber_contract: e.sig.clobbers.is_some(),
        ..Default::default()
    }
}

/// The clobber BOUND a contract type imposes on its dispatch targets (§4): the
/// registers a conforming target MAY clobber. An explicit `clobbers(...)` IS the
/// bound; a preserves-only type (e.g. `ObjRoutine preserves(a0, d7)`) bounds the
/// clobbers to everything-not-preserved (the whole register file minus its
/// preserves).
fn contract_type_bound(t: &ContractTypeDecl) -> RegEffect {
    let regs = match &t.sig.clobbers {
        Some(c) => expand_reglist_regs(c),
        None => {
            let preserved = expand_reglist_regs(&t.sig.preserves);
            universe().difference(&preserved).cloned().collect()
        }
    };
    RegEffect { top: false, regs }
}

/// A contract signature's clobbers as a register set.
fn sig_clobbers(sig: &ProcSig) -> BTreeSet<String> {
    expand_reglist_regs(sig.clobbers.as_deref().unwrap_or(&[]))
}

/// Register names of a `proc`'s params (spellings ARE registers, §5.1).
fn param_regs_typed(params: &[(String, ast::Type, Span)]) -> BTreeSet<String> {
    params.iter().filter_map(|(name, _, _)| reg_name(name)).collect()
}

/// Register names of a contract-signature's params (`Option<Type>`).
fn sig_param_regs(sig: &ProcSig) -> BTreeSet<String> {
    sig.params.iter().filter_map(|(name, _, _)| reg_name(name)).collect()
}

/// Canonicalize a param name to a register spelling, or `None` if it is not a
/// register (defensive — proc params are register spellings today).
fn reg_name(name: &str) -> Option<String> {
    Reg::from_name(name).map(|r| r.to_string())
}

/// The `Sym` target of a call/tail-shaped instruction, if its sole operand is a
/// bare GLOBAL symbol (a DIRECT call `jsr Foo` / a tail `jbra Foo`). `None` for
/// an indirect `jsr (aN)` (register-based operand), a non-call, or a LOCAL-label
/// target: hygiene mangles local labels as `$module$proc$label`, and a `bra`/
/// `jbra` to a local label (`.loop`) is intra-proc control flow, never a callee
/// — the `$` marks it so it is dropped from both the edge set and the
/// hole/unresolved report (a real proc/extern name never contains `$`).
fn call_target_sym(ops: &[CodeOperand]) -> Option<String> {
    match ops {
        [CodeOperand::Sym(name)] if !name.contains('$') => Some(name.clone()),
        _ => None,
    }
}

/// Scan a proc body (recursing comptime-`if` branches) for indirect call sites,
/// returning each site's declared bound: `Some(type)` for `jsr (aN) as Type`,
/// `None` for an unbounded `jsr (aN)`. A call whose target is a bare symbol
/// (direct) contributes no indirect site.
fn collect_indirect_sites(body: &[AsmStmt]) -> Vec<Option<String>> {
    let mut sites = Vec::new();
    walk_body_for_indirect(body, &mut sites);
    sites
}

fn walk_body_for_indirect(body: &[AsmStmt], sites: &mut Vec<Option<String>>) {
    for stmt in body {
        match stmt {
            AsmStmt::Instr(instr) => {
                if is_indirect_call(instr) {
                    sites.push(instr.dispatch_bound.clone());
                }
            }
            AsmStmt::If { then, els, .. } => {
                walk_body_for_indirect(then, sites);
                if let Some(e) = els {
                    walk_body_for_indirect(e, sites);
                }
            }
            _ => {}
        }
    }
}

/// True when an AST instruction is an indirect call/tail-transfer — a call-shaped
/// mnemonic whose first operand is a register-indirect EA (`jsr (a1)` /
/// `jsr (a0, d4.w)`), as opposed to a direct `jsr Foo`.
fn is_indirect_call(instr: &InstrLine) -> bool {
    let Some(m) = single_text(&instr.mnemonic) else { return false };
    if !CALL_MNEMONICS.contains(&m) && !TAIL_MNEMONICS.contains(&m) {
        return false;
    }
    matches!(instr.operands.first(), Some(Operand::Ind { .. }))
}

/// The mnemonic as a single literal string, or `None` if it is spliced.
fn single_text(mnemonic: &[TextOrSplice]) -> Option<&str> {
    match mnemonic {
        [TextOrSplice::Text(s)] => Some(s.as_str()),
        _ => None,
    }
}
