//! `[call.clobbers-incomplete]` — the Z80 transitive clobber closure (A5).
//!
//! The 68k register-contract closure ([`crate::closure`]) skips `(cpu: z80)`
//! modules: a Z80 proc's instructions carry no 68k effect. Z80 code gets the SAME
//! closure algorithm over its own unit domain (`a`, `f`, `b`, … `ix`, `iy`), with
//! nodes built here: each proc's local writes come from the shared
//! [`z80_written_registers`] detector (the one the per-proc preserves proof uses),
//! its direct edges from its `call`/`rst`/`jp`/`jr`/`djnz` targets, and its
//! declared `clobbers`/`out`/`preserves` from the declaration, expanded over the
//! Z80 register file. A proc's declared `clobbers` must be a SUPERSET of its
//! transitive effect (local writes ∪ reachable-callee effects − preserves).
//!
//! This closure was born in the seam-1 harness (`sigil-harness/src/seam1.rs`) over
//! the five resident sound modules. It lives here so the corpus walk
//! ([`crate::corpus_contracts`]) and therefore the BUILD gate run it too; the
//! harness keeps thin wrappers.
//!
//! SCOPE: the direct-call closure. A proc reached only through a COMPUTED dispatch
//! (aeon's sequencer opcode handlers, entered through `SeqOpcodeTable`'s
//! `ex (sp),hl; ret` trampoline) has a transitive effect that depends on an edge
//! the closure cannot traverse, so it cannot be soundly verified. Such a proc
//! DECLARES that: `@allow("clobbers.computed-dispatch", "<reason>")`. The exclusion
//! is by annotation, never by name, and the excluded set is reported beside the
//! firings so it stays audited.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{self, AsmStmt, ExternProcDecl, InstrLine, ProcDecl, TextOrSplice};
use crate::closure::{check_firings, compute_closure, Firing, ProcNode};
use crate::regfile::{expand_reglist, RegFile};
use crate::value::{CodeBuf, CodeItem, CodeOperand};
use crate::z80_preserves::z80_written_registers;
use sigil_span::{Diagnostic, Span};

/// The `@allow` rule a computed-dispatch-only Z80 proc declares.
pub const COMPUTED_DISPATCH_RULE: &str = "clobbers.computed-dispatch";

/// Expand a reglist over the Z80 register file. An unknown register is the
/// per-file gate's `[contract.unknown-register]`, not this closure's to report.
pub fn z80_units(segs: &[(String, Option<String>)]) -> BTreeSet<String> {
    expand_reglist(segs, RegFile::Z80, |_| {})
}

/// Whether `p` declares `@allow("clobbers.computed-dispatch", …)` on itself. Only
/// the proc's OWN attrs count: a module-level allow would exclude a whole module
/// from the closure, which is not what the rule means.
pub fn computed_dispatch_allowed(p: &ProcDecl) -> bool {
    p.attrs.iter().any(|a| {
        a.name == "allow" && a.args.first().and_then(ast::Arg::str_value) == Some(COMPUTED_DISPATCH_RULE)
    })
}

/// A module's `invariant: preserves(...)` register UNITS (empty when none): every
/// proc in the module inherits them as preserves (the reader mirrors
/// `lower/mod.rs`'s `validate_module_invariants`, which refuses a bad register).
pub fn module_invariant_units(file: &ast::File) -> BTreeSet<String> {
    let mut segs: Vec<(String, Option<String>)> = Vec::new();
    for (name, expr) in &file.module.attrs {
        if name != "invariant" {
            continue;
        }
        if let ast::Expr::Call { callee, args, .. } = expr {
            if callee.segments.last().map(String::as_str) == Some("preserves") {
                segs.extend(args.iter().filter_map(|a| match &a.value {
                    ast::Expr::Path(p) if p.segments.len() == 1 => Some((p.segments[0].clone(), None)),
                    _ => None,
                }));
            }
        }
    }
    z80_units(&segs)
}

/// The symbolic target of a Z80 transfer instruction (`call`/`rst`/`jp`/`jr`/
/// `djnz`, conditional or not): the last symbolic operand (a leading condition
/// code is skipped). A `.`-prefixed local label or a hygiene (`$`) symbol is not an
/// inter-proc edge, and a computed `jp (hl)` has no symbol, so both give `None`.
pub fn z80_transfer_target(mnemonic: &str, ops: &[CodeOperand]) -> Option<String> {
    if !matches!(mnemonic, "call" | "rst" | "jp" | "jr" | "djnz") {
        return None;
    }
    ops.iter().rev().find_map(|op| match op {
        CodeOperand::Sym(name) if !name.starts_with('.') && !name.contains('$') => Some(name.clone()),
        _ => None,
    })
}

/// The closure node of a body-bearing Z80 proc. `buf` is its evaluated body
/// (`None` when evaluation produced none: the node then has no writes and no
/// edges). `inv_units` are its module's invariant preserves. `clobbers_override`
/// replaces the declared `clobbers` (a doctored RED fixture); `None` reads `p`.
///
/// The declared `preserves` are subtracted as PROVEN: the per-file Z80 preserves
/// proof refuses a `preserves` it cannot verify, so a build that reaches this
/// closure has had every one checked.
pub fn z80_proc_node(
    p: &ProcDecl,
    buf: Option<&CodeBuf>,
    inv_units: &BTreeSet<String>,
    clobbers_override: Option<&[(String, Option<String>)]>,
) -> ProcNode {
    let mut local_writes = BTreeSet::new();
    let mut direct_callees = Vec::new();
    if let Some(buf) = buf {
        local_writes = z80_written_registers(buf);
        for ci in &buf.items {
            if let CodeItem::Instr { mnemonic, ops, .. } = ci {
                if let Some(t) = z80_transfer_target(mnemonic, ops) {
                    direct_callees.push(t);
                }
            }
        }
    }
    let declared_clobbers = match clobbers_override {
        Some(segs) => z80_units(segs),
        None => z80_units(p.clobbers.as_deref().unwrap_or(&[])),
    };
    let mut verified_preserves = z80_units(&p.preserves);
    verified_preserves.extend(inv_units.iter().cloned());
    ProcNode {
        local_writes,
        direct_callees,
        // A `falls_into T` proc flows into T with no transfer instruction, so the
        // declaration itself is the edge (the closure charges it like a tail).
        falls_into: p.falls_into.clone(),
        indirect_sites: Vec::new(),
        is_extern: false,
        declared_clobbers,
        params: BTreeSet::new(),
        out: z80_units(p.out.as_deref().unwrap_or(&[])),
        // The in-out facet is 68k-only.
        inout: BTreeSet::new(),
        has_clobber_contract: p.clobbers.is_some() || clobbers_override.is_some(),
        verified_preserves,
        // The §6 word facet is a 68k data-register form.
        word_preserves: BTreeSet::new(),
        requires: p.requires.iter().map(|(n, _)| n.clone()).collect(),
        grants: p.grants.iter().map(|(n, _)| n.clone()).collect(),
        unanalyzable_allowed: false,
        computed_dispatch: computed_dispatch_allowed(p),
        decl_span: Some(p.span),
    }
}

/// The closure node of a Z80 `extern proc`: a leaf whose effect is its declared
/// `clobbers ∪ out` (an `out` register is written by the callee).
pub fn z80_extern_node(e: &ExternProcDecl) -> ProcNode {
    let out = z80_units(e.sig.out.as_deref().unwrap_or(&[]));
    let mut effective = z80_units(e.sig.clobbers.as_deref().unwrap_or(&[]));
    effective.extend(out.iter().cloned());
    ProcNode {
        is_extern: true,
        declared_clobbers: effective,
        out,
        has_clobber_contract: e.sig.clobbers.is_some(),
        decl_span: Some(e.span),
        ..Default::default()
    }
}

/// The Z80 closure's full answer: every firing split by scope, the excluded procs,
/// and the holes.
#[derive(Clone, Debug, Default)]
pub struct Z80ClobberPartition {
    /// Firings on procs IN the direct-call closure's scope: each is a real
    /// transitive under-claim. Sorted (proc, register).
    pub in_scope: Vec<Firing>,
    /// Firings on `@allow("clobbers.computed-dispatch")` procs: out of scope, kept
    /// so a caller can show the exclusion filters real firings.
    pub excluded_firings: Vec<Firing>,
    /// Every proc carrying the allow, whether it fires or not. Sorted.
    pub excluded: Vec<String>,
    /// Direct callees that resolve to no Z80 proc or extern: holes in the closure.
    pub unresolved_callees: BTreeSet<String>,
}

/// Run the closure over `procs` and split its firings by the computed-dispatch
/// annotation.
pub fn z80_clobber_partition(procs: &BTreeMap<String, ProcNode>) -> Z80ClobberPartition {
    let closure = compute_closure(procs, &BTreeMap::new());
    let (excluded_firings, in_scope): (Vec<Firing>, Vec<Firing>) = check_firings(procs, &closure)
        .into_iter()
        .partition(|f| procs.get(&f.proc).is_some_and(|n| n.computed_dispatch));
    let excluded = procs
        .iter()
        .filter(|(_, n)| n.computed_dispatch)
        .map(|(k, _)| k.clone())
        .collect();
    Z80ClobberPartition {
        in_scope,
        excluded_firings,
        excluded,
        unresolved_callees: closure.unresolved_callees,
    }
}

/// `[call.clobbers-incomplete]` over a Z80 proc map: the in-scope firings, and the
/// names of the procs excluded by `@allow("clobbers.computed-dispatch")`.
pub fn z80_clobber_firings(procs: &BTreeMap<String, ProcNode>) -> (Vec<Firing>, Vec<String>) {
    let p = z80_clobber_partition(procs);
    (p.in_scope, p.excluded)
}

/// The spans of the `dc` data directives in a proc body, recursing comptime `if`
/// arms and `with` blocks (the same shape `collect_discarded` walks).
fn dc_spans(body: &[AsmStmt], out: &mut Vec<Span>) {
    for stmt in body {
        match stmt {
            AsmStmt::Instr(InstrLine { mnemonic, span, .. })
                if matches!(mnemonic.as_slice(), [TextOrSplice::Text(m)] if m == "dc") =>
            {
                out.push(*span)
            }
            AsmStmt::If { then, els, .. } => {
                dc_spans(then, out);
                if let Some(e) = els {
                    dc_spans(e, out);
                }
            }
            AsmStmt::With { body, .. } => dc_spans(body, out),
            _ => {}
        }
    }
}

/// Split a proc's dropped-instruction count into `(register_relevant, data_cells)`.
///
/// A dropped `dc` data cell (typically a cell whose operand names a link-time
/// constant the contract walk's define set does not carry, e.g. aeon's
/// `DacSampleTable` sample pointers) emits bytes and writes no register, so it
/// cannot hide an effect from this closure. Every other drop can. The evaluator
/// reports each drop with an `instruction dropped` diagnostic at the instruction's
/// own span, which is what identifies a drop as a `dc` here; a drop it reports any
/// other way stays register-relevant, the safe direction.
pub fn split_z80_drops(body: &[AsmStmt], dropped: usize, diags: &[Diagnostic]) -> (usize, usize) {
    if dropped == 0 {
        return (0, 0);
    }
    let mut spans = Vec::new();
    dc_spans(body, &mut spans);
    let data = diags
        .iter()
        .filter(|d| d.message.contains("instruction dropped") && spans.contains(&d.primary))
        .count()
        .min(dropped);
    (dropped - data, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(rs: &[&str]) -> BTreeSet<String> {
        rs.iter().map(|r| r.to_string()).collect()
    }

    fn node(writes: &[&str], callees: &[&str], clobbers: &[&str], dispatch: bool) -> ProcNode {
        ProcNode {
            local_writes: set(writes),
            direct_callees: callees.iter().map(|c| c.to_string()).collect(),
            declared_clobbers: set(clobbers),
            has_clobber_contract: true,
            computed_dispatch: dispatch,
            ..Default::default()
        }
    }

    #[test]
    fn an_underclaim_fires_through_its_carrier_and_the_allow_excludes_it() {
        let mut procs = BTreeMap::new();
        procs.insert("Q".to_string(), node(&["a", "b", "f"], &[], &["a", "b", "f"], false));
        procs.insert("P".to_string(), node(&[], &["Q"], &["a", "f"], false));
        let (firings, excluded) = z80_clobber_firings(&procs);
        assert_eq!(firings.len(), 1);
        assert_eq!(firings[0].proc, "P");
        assert_eq!(firings[0].reg.as_deref(), Some("b"));
        assert_eq!(firings[0].via.as_deref(), Some("Q"));
        assert!(excluded.is_empty());

        procs.get_mut("P").unwrap().computed_dispatch = true;
        let part = z80_clobber_partition(&procs);
        assert!(part.in_scope.is_empty());
        assert_eq!(part.excluded, vec!["P".to_string()]);
        assert_eq!(part.excluded_firings.len(), 1, "the excluded firing is kept, not lost");
    }

    #[test]
    fn an_allowed_proc_that_fires_nothing_is_still_listed_excluded() {
        let mut procs = BTreeMap::new();
        procs.insert("H".to_string(), node(&["a"], &[], &["a"], true));
        let (firings, excluded) = z80_clobber_firings(&procs);
        assert!(firings.is_empty());
        assert_eq!(excluded, vec!["H".to_string()]);
    }

    /// Through the corpus walk: a dropped `dc` cell is counted apart and does not
    /// blind the closure; a dropped instruction is register-relevant; an
    /// under-claim fires in scope and an annotated proc is excluded.
    #[test]
    fn the_corpus_walk_splits_drops_and_partitions_by_annotation() {
        let src = "module t.z (cpu: z80)
section z (cpu: z80) {
    pub proc Tab () clobbers() {
        dc.w    sizeof(NoSuchType)
        dc.b    0
    }
    pub proc Bad () clobbers(a, f) {
        ld      a, sizeof(NoSuchType)
        ld      b, 0
        ret
    }
    @allow(\"clobbers.computed-dispatch\", \"test\")
    pub proc Handler () clobbers(a, f) {
        ld      c, 0
        ret
    }
}
";
        let (file, diags) = crate::parse_str(src);
        assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "{diags:?}");
        let r = crate::corpus_contracts::analyze_corpus_with(&[file], &[]);
        assert_eq!(r.z80_dropped_data_cells, 1, "the dc cell is counted apart");
        assert_eq!(r.z80_dropped_instrs, 1, "the ld drop is register-relevant");
        assert_eq!(r.z80_dropped_by_proc, vec![("Bad".to_string(), 1)]);
        let fired: Vec<_> =
            r.z80_clobber_firings.iter().map(|f| (f.proc.as_str(), f.reg.as_deref())).collect();
        assert_eq!(fired, vec![("Bad", Some("b"))]);
        assert_eq!(r.z80_clobber_excluded, vec!["Handler".to_string()]);
        assert!(r.z80_clobber_unresolved.is_empty());
    }

    #[test]
    fn transfer_targets_skip_locals_hygiene_and_computed_jumps() {
        let sym = |s: &str| CodeOperand::Sym(s.to_string());
        assert_eq!(z80_transfer_target("call", &[sym("X")]), Some("X".to_string()));
        assert_eq!(z80_transfer_target("jr", &[sym(".loop")]), None);
        assert_eq!(z80_transfer_target("jp", &[sym("a$1")]), None);
        assert_eq!(z80_transfer_target("ld", &[sym("X")]), None);
    }
}
