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
use crate::lower::{expand_reglist_regs, proc_written_registers};
use crate::value::{CodeItem, CodeOperand, Reg};
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
    /// Names declared BOTH `extern proc` and `proc` (§11 Q4) — with the extern's
    /// span (the mirror that should be deleted when the callee ports).
    pub extern_collisions: Vec<(String, Span)>,
    /// How many procs (incl. externs) the walk collected.
    pub proc_count: usize,
    /// How many extern-proc leaves.
    pub extern_count: usize,
    /// How many contract types.
    pub contract_type_count: usize,
}

/// Analyze the parsed corpus: build the proc/extern/contract-type maps, run the
/// closure, and collect firings + collisions.
pub fn analyze_corpus(files: &[ast::File]) -> ContractReport {
    let mut nodes: BTreeMap<String, ProcNode> = BTreeMap::new();
    let mut types: BTreeMap<String, RegEffect> = BTreeMap::new();
    let mut extern_names: BTreeSet<String> = BTreeSet::new();
    let mut proc_names: BTreeSet<String> = BTreeSet::new();
    let mut extern_spans: BTreeMap<String, Span> = BTreeMap::new();
    let mut counter: u32 = 0;

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

    ContractReport { closure, firings, extern_collisions, proc_count, extern_count, contract_type_count }
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
) {
    for item in items {
        match item {
            Item::Proc(p) => {
                proc_names.insert(p.name.clone());
                nodes.insert(p.name.clone(), proc_node(p, file, counter));
            }
            Item::ExternProc(e) => {
                extern_names.insert(e.name.clone());
                extern_spans.insert(e.name.clone(), e.span);
                nodes.insert(e.name.clone(), extern_node(e));
            }
            Item::ContractType(t) => {
                types.insert(t.name.clone(), contract_type_bound(t));
            }
            Item::Section(s) => collect_items(
                &s.items, file, nodes, types, extern_names, proc_names, extern_spans, counter,
            ),
            _ => {}
        }
    }
}

/// Build a [`ProcNode`] from a body-bearing `proc` decl.
fn proc_node(p: &ProcDecl, file: &ast::File, counter: &mut u32) -> ProcNode {
    let (buf, _diags, next) =
        crate::eval::eval_proc_body(file, &p.name, &p.params, &p.body, p.span, *counter, Cpu::M68000, &[]);
    *counter = next;

    let mut local_writes = BTreeSet::new();
    let mut direct_callees = Vec::new();
    if let Some(buf) = &buf {
        // Local writes — `a7` filtered as stack discipline (census caveat 5).
        local_writes = proc_written_registers(buf).into_iter().filter(|r| r != "a7").collect();
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

    ProcNode {
        local_writes,
        direct_callees,
        indirect_sites: collect_indirect_sites(&p.body),
        is_extern: false,
        declared_clobbers: expand_reglist_regs(p.clobbers.as_deref().unwrap_or(&[])),
        params: param_regs_typed(&p.params),
        out: expand_reglist_regs(p.out.as_deref().unwrap_or(&[])),
        has_clobber_contract: p.clobbers.is_some(),
    }
}

/// Build a leaf [`ProcNode`] from an `extern proc` decl (§3).
fn extern_node(e: &ExternProcDecl) -> ProcNode {
    ProcNode {
        is_extern: true,
        declared_clobbers: sig_clobbers(&e.sig),
        params: sig_param_regs(&e.sig),
        out: expand_reglist_regs(e.sig.out.as_deref().unwrap_or(&[])),
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

/// The `Sym` target of a call-shaped instruction, if its sole operand is a bare
/// symbol (a DIRECT call `jsr Foo`); `None` for an indirect `jsr (aN)` (whose
/// operand is register-based) or a non-call.
fn call_target_sym(ops: &[CodeOperand]) -> Option<String> {
    match ops {
        [CodeOperand::Sym(name)] => Some(name.clone()),
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
