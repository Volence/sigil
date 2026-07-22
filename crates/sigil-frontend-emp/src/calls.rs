//! Contract-grammar v2 §6 / G4 — the caller-side INPUT + LIVENESS checks.
//!
//! Two per-call-site diagnostics keyed off the callee's (declared or transitive)
//! contract, both over the SAME lightweight CFG G2 built
//! ([`crate::flag_check::Cfg`], spec §11 Q1 — reused, real joins, never
//! straight-line):
//!
//! - **[call.input-undefined]** (D1b): every register param of the callee must
//!   have a reaching definition at the call site on EVERY path in the caller. A
//!   forward MUST-def (all-paths intersection) dataflow from the caller's own
//!   params; a param outside the def set on some path fires.
//! - **[call.live-clobbered]** (D1c): a value defined before the call and read
//!   after it, held in a register the callee EFFECTIVELY clobbers. This is
//!   pass-3's seatbelt: the exact mistake a contract-trusting hoist could make.
//!
//! **Modeling stance (soundness).** Both checks are FALSE-NEGATIVE-leaning — the
//! error-tier house stance (never fire on a case not modeled). A "read" of a
//! register is `mentioned in an operand AND not written by that instruction`
//! (the [`crate::flag_check`] convention): a read-modify-write destination
//! (`add.l d0, d1` — d1 read-then-written) is therefore NOT counted as a read of
//! d1, and a register passed as a LATER call's argument is not counted as a read
//! either (D1c does not model callee inputs at intervening sites). D1c models the
//! caller's own save/restore: a `movem <ea>, <reglist>` LOAD (reglist as
//! destination) redefines its registers, so a `movem.l d5/d7,-(sp)` … call …
//! `movem.l (sp)+, d5/d7` around a clobbering call correctly does NOT fire (the
//! value is preserved by the caller). D1b's must-def is likewise firing-safe: any
//! write — including a movem load — defines a register, so it over-approximates
//! "defined" and only fires when a param is genuinely undefined on some path.
//! Indirect dispatch input-coverage (the bound's params) is logged, not checked
//! here (§4 site machinery; the object-pointer input is always live).

use crate::closure::RegEffect;
use crate::flag_check::{conditional_out_edge_credits, Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::value::{CodeItem, CodeOperand};
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Call mnemonics whose target RETURNS (a `jsr`/`jbsr`/`bsr`) — the only ones a
/// D1c "read after the call" analysis walks past.
const CALL_MNEMONICS: [&str; 3] = ["jsr", "jbsr", "bsr"];
/// Tail-transfer mnemonics: control leaves this proc for the target, which still
/// READS its inputs — so D1b checks them too (D1c does not: nothing in THIS proc
/// runs after a tail transfer).
const TAIL_MNEMONICS: [&str; 3] = ["jmp", "bra", "jbra"];

/// The bare GLOBAL-symbol target of a DIRECT call/tail (`jbsr Foo` / `jbra Foo`),
/// or `None` for an indirect (`jsr (a1)`), a non-call, or a local-label target
/// (hygiene mangles those with `$`). Mirrors [`crate::corpus_contracts`]'s
/// `call_target_sym` so the callee key matches the contract maps.
fn direct_target<'a>(mnem: &str, ops: &'a [CodeOperand]) -> Option<&'a str> {
    if !CALL_MNEMONICS.contains(&mnem) && !TAIL_MNEMONICS.contains(&mnem) {
        return None;
    }
    match ops {
        [CodeOperand::Sym(name)] if !name.contains('$') => Some(name.as_str()),
        _ => None,
    }
}

/// SHARED call-aware primitive for the caller-side ERROR gates (D1b must-def §6
/// invalid-path). If `(mnem, ops)` is a DIRECT CALL to a known callee, its
/// UNCONDITIONAL `out()` set — the registers it redefines with a produced value
/// on EVERY return edge. **must-def** credits these as DEFINITIONS; **§6** credits
/// them as taint-KILLING redefines. Both consume the SAME fact through this one
/// function so the two gates cannot silently drift on what `out()` means.
/// UNCONDITIONAL only: a conditional `out(rM if cc)` is trash on its `!cc` edge
/// and is NEVER a redefine — `callee_uncond_out` must already exclude the
/// conditional-out registers (the corpus caller subtracts them). Tails do not
/// return, so calls only.
pub(crate) fn call_unconditional_outs<'a>(
    mnem: &str,
    ops: &[CodeOperand],
    callee_uncond_out: &'a BTreeMap<String, BTreeSet<String>>,
) -> Option<&'a BTreeSet<String>> {
    if !CALL_MNEMONICS.contains(&mnem) {
        return None;
    }
    callee_uncond_out.get(direct_target(mnem, ops)?)
}

/// The register-name write set of an instruction (canonical `d0`..`a7`
/// spellings). Combines the shared [`instr_written_regs`] detector (dest register
/// plus auto-inc/dec bases) with the movem LOAD form. A movem whose register list
/// is the DESTINATION (the last operand, e.g. `movem.l (sp)+, d5/d7`) writes every
/// listed register. [`instr_written_regs`] expands only NON-stack movem-load
/// reglists (it exempts `(sp)+` restores as clobber-lint preserve-discipline), so
/// a caller's movem-RESTORE of a saved register would otherwise look like a
/// read-after-the-call without a redefine — a false live-clobber. This must-def /
/// live-clobber analysis needs the ISA-true set (a restore DOES redefine), so it
/// mask-expands every movem reglist itself (idempotent with the detector for
/// non-stack loads; the BTreeSet dedupes). Crediting the reglist load as a write
/// makes a save/restore around a call correctly NOT fire (the corpus tile_cache/
/// sprites pattern). `a7` is left in — the def/live sets that consume this never
/// contain `a7`.
///
/// **must-write vs may-write (S2-D6):** [`instr_written_regs`] counts EVERY
/// `db<cc>` counter as a may-write (correct for the clobber lint — a `dbeq`
/// MIGHT decrement dN on its cc-false path). But this feeds a MUST-write /
/// redefine analysis (D1b definitions, production collection, D1c redefine): a
/// CONDITIONAL `dbeq/dbne/…` does NOT decrement dN on its cc-satisfied exit, so it
/// does NOT DEFINE dN on every path, and `dbt` never decrements at all. Crediting
/// those as a definition is the spec-§3-forbidden over-credit on the flip-bound
/// D1b. Only `dbf`/`dbra` (condition F — always decrements) unconditionally write
/// their counter, so the conditional family's counter is stripped back here.
fn written_names(mnem: &str, ops: &[CodeOperand]) -> BTreeSet<String> {
    let mut regs: BTreeSet<String> =
        instr_written_regs(mnem, ops).into_iter().map(|r| r.to_string()).collect();
    // Strip the may-write dbcc counter for the CONDITIONAL family (everything but
    // the unconditional `dbf`/`dbra`): its first-operand counter is not a
    // must-definition. The counter is a dbcc's ONLY register write, so removing
    // `ops.first()` cannot drop another effect's contribution.
    if mnem.starts_with("db") && mnem != "dbf" && mnem != "dbra" {
        if let Some(CodeOperand::Reg(r)) = ops.first() {
            regs.remove(&r.to_string());
        }
    }
    // A `movem` load writes its reglist when the reglist is the DESTINATION (last
    // operand); a `movem` STORE (`movem d5/d7,-(sp)`, reglist first) reads them.
    if let Some(CodeOperand::RegList(mask)) = ops.last() {
        for r in crate::preserves::expand_mask(*mask) {
            regs.insert(r.to_string());
        }
    }
    regs
}

/// One `[call.input-undefined]` firing (D1b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFiring {
    pub proc: String,
    pub callee: String,
    pub reg: String,
    pub span: Span,
}

/// One `[call.live-clobbered]` firing (D1c).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveClobberFiring {
    pub proc: String,
    pub callee: String,
    pub reg: String,
    pub span: Span,
}

/// The first instruction item index (the proc entry), or `None` for an empty
/// body.
fn entry_index(items: &[CodeItem]) -> Option<usize> {
    items.iter().position(|it| matches!(it, CodeItem::Instr { .. }))
}

/// Forward MUST-def dataflow: at each reachable instruction, the set of registers
/// DEFINED on EVERY path from entry (the caller's own params are defined on
/// entry). Join is INTERSECTION (all-paths); the transfer adds each
/// instruction's write set. Registers unreachable from entry are absent from the
/// map. A monotone shrinking fixpoint on a finite lattice → terminates.
fn must_defined_in(
    cfg: &Cfg,
    entry: usize,
    params: &BTreeSet<String>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    edge_credit: &BTreeMap<(usize, usize), BTreeSet<String>>,
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut in_def: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    in_def.insert(entry, params.clone());
    let mut work: VecDeque<usize> = VecDeque::from([entry]);
    while let Some(idx) = work.pop_front() {
        let mut out = in_def[&idx].clone();
        if let Some((mnem, ops)) = cfg.instr(idx) {
            out.extend(written_names(mnem, ops));
            // Credit a CALL's callee UNCONDITIONAL `out()` as a definition: a
            // plain `out(rN)` guarantees rN holds a produced value on every
            // return, so it is defined on every edge leaving the call — sound
            // under must-def's intersection join. `callee_out` here is the
            // UNCONDITIONAL subset ONLY (the caller subtracts conditional-out
            // registers — a parser quirk folds `out(rN if cc)`'s rN into the
            // reglist, so the raw `node.out` is NOT safe to credit). Crediting a
            // conditional out would over-approximate must-def = a FALSE NEGATIVE
            // on an ERROR gate; edge-sensitive success-edge crediting is deferred
            // to spec. Via the SHARED primitive §6 also consumes.
            if let Some(outs) = call_unconditional_outs(mnem, ops, callee_out) {
                out.extend(outs.iter().cloned());
            }
        }
        for edge in cfg.edges(idx) {
            let Edge::Follow(succ) = edge else { continue };
            // Edge-sensitive conditional-out credit (item #2): a callee's
            // `out(rN if cc)` is credited DEFINED only on the caller's
            // provably-cc-SUCCESS edge (`(idx, succ)` from the shared edge-ID
            // primitive). Applied as a per-EDGE transfer that re-joins by
            // intersection at `succ` — NOT a global "rN defined post-call" fact
            // (§3): at a merge reached from this success edge AND a non-producing
            // predecessor, the intersection below correctly drops rN.
            let edge_out = match edge_credit.get(&(idx, succ)) {
                None => out.clone(),
                Some(extra) => out.union(extra).cloned().collect(),
            };
            let changed = match in_def.get(&succ) {
                None => {
                    in_def.insert(succ, edge_out);
                    true
                }
                Some(existing) => {
                    let merged: BTreeSet<String> =
                        existing.intersection(&edge_out).cloned().collect();
                    if merged != *existing {
                        in_def.insert(succ, merged);
                        true
                    } else {
                        false
                    }
                }
            };
            if changed {
                work.push_back(succ);
            }
        }
    }
    in_def
}

/// `[call.input-undefined]` (D1b): at every DIRECT call/tail to a callee that
/// declares register-param inputs, each param must be DEFINED on every path from
/// the caller's entry to the call. A param outside the must-def set on some path
/// fires. Indirect dispatch is not checked here (its bound's params ride the §4
/// site machinery; the object-pointer input is always live — logged scope).
pub fn check_input_undefined(
    proc_name: &str,
    params: &BTreeSet<String>,
    items: &[CodeItem],
    callee_params: &BTreeMap<String, BTreeSet<String>>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    cond_callees: &BTreeMap<String, Vec<(String, String)>>,
) -> Vec<InputFiring> {
    let Some(entry) = entry_index(items) else { return Vec::new() };
    let cfg = Cfg::build(items);
    // Item #2: a callee's conditional `out(rN if cc)` is credited as a definition
    // only on the caller's provably-cc-success edge (the shared edge-ID primitive,
    // §4). `callee_out` here is the UNCONDITIONAL subset (the corpus subtracts the
    // conditional-out registers), so must-def's unconditional credit stays sound
    // and the conditional credit is applied edge-sensitively below.
    let edge_credit = conditional_out_edge_credits(&cfg, items, cond_callees);
    let defined = must_defined_in(&cfg, entry, params, callee_out, &edge_credit);

    let mut firings = Vec::new();
    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        let Some(callee) = direct_target(mnemonic, ops) else { continue };
        let Some(inputs) = callee_params.get(callee) else { continue };
        if inputs.is_empty() {
            continue;
        }
        // Unreachable call sites (absent from the def map) are not analyzed.
        let Some(here) = defined.get(&idx) else { continue };
        for reg in inputs {
            if !here.contains(reg) {
                firings.push(InputFiring {
                    proc: proc_name.to_string(),
                    callee: callee.to_string(),
                    reg: reg.clone(),
                    span: *span,
                });
            }
        }
    }
    firings.sort_by(|a, b| (&a.callee, &a.reg, a.span.start).cmp(&(&b.callee, &b.reg, b.span.start)));
    firings
}

/// The whole tracked register file (`d0`..`d7` + `a0`..`a6`), the universe a ⊤
/// effective set clobbers. `a7` is stack discipline, never a held value.
fn register_universe() -> Vec<String> {
    (0..8).map(|n| format!("d{n}")).chain((0..7).map(|n| format!("a{n}"))).collect()
}

/// Does callee `c` DESTROY a caller's held value in `reg` — `reg` is in its
/// effective clobber set (post verified-preserves subtraction) and is NOT its
/// declared UNCONDITIONAL output? Only an UNCONDITIONAL `out(reg)` excuses the
/// clobber (it is genuinely the produced result on ALL return edges, so a read
/// after the call sees that result, not a stale held value). A CONDITIONAL
/// `out(reg if cc)` does NOT excuse it: `reg` is trash on the `!cc` edge AND the
/// produced result on the cc edge, so the caller's OLD held value in `reg` is
/// destroyed on EVERY path — excusing it would be a silent D1c miss (§4, Finding
/// 4). Hence this reads `callee_uncond_out` (the conditional-out registers
/// already subtracted), NOT the full `callee_out`. An unknown callee (`None`) is
/// not claimed to clobber (holes are gated elsewhere; never fire across an
/// unknown here).
///
/// KNOWN FALSE POSITIVES (observe-only, documented 2026-07-19): this simple close
/// is edge-blind, so a register read that is really a conditional callee's
/// PRODUCED value on the cc-success edge looks like a destroyed held value. Two
/// same-class sites, both verified FP (the read is the produced/moved value, not
/// a stale held one), both on the ungated D1c. (1) `TileCache_FillRow @
/// TileCache_FindStagedBlock :: a1` — FillRow reads `a1` only on FindStagedBlock's
/// eq-success edge (valid there) or after an intervening `DecompressBlock`
/// (unconditional `out(a1)`, a redefine). (2) `Load_Object @ AllocDynamic :: a1` —
/// new since the item-#2 Bucket-1 relabel made `AllocDynamic out(a1 if eq)`: the
/// `a1` read after the call is AllocDynamic's produced new-SST pointer; the old
/// held template was restored into `a2` (`movem.l (sp)+, d0-d2/a2`), not `a1`; the
/// alloc-fail path restores `a1` and returns without reading it. D1c is
/// observe-only, so neither breaks a gate. Item #2 deliberately does NOT couple
/// D1c to its edge primitive (§4, Finding 5): a degrade-to-miss on a `valid_edge`
/// bail would be worse than the FP. An edge-precise D1c that never degrades on
/// bail is separate future work.
fn destroys_value(
    effective: &BTreeMap<String, RegEffect>,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
    c: &str,
    reg: &str,
) -> bool {
    let Some(e) = effective.get(c) else { return false };
    if callee_uncond_out.get(c).is_some_and(|o| o.contains(reg)) {
        return false;
    }
    e.top || e.regs.contains(reg)
}

/// The registers a resolved instruction MENTIONS in an operand (any position,
/// including an indirect base or index) — the read candidates before subtracting
/// the write set. Mirrors `flag_check::regs_mentioned` (kept local so the two
/// caller-side modules stay decoupled).
fn regs_mentioned(ops: &[CodeOperand]) -> BTreeSet<String> {
    let mut regs = BTreeSet::new();
    for op in ops {
        match op {
            CodeOperand::Reg(r)
            | CodeOperand::Ind(r)
            | CodeOperand::PreDec(r)
            | CodeOperand::PostInc(r)
            | CodeOperand::DispInd { reg: r, .. } => {
                regs.insert(r.to_string());
            }
            CodeOperand::IndIdx { reg, xn, .. } => {
                regs.insert(reg.to_string());
                regs.insert(xn.to_string());
            }
            _ => {}
        }
    }
    regs
}

/// Forward MAY-def dataflow: at each reachable instruction, the set of registers
/// that HOLD A VALUE entering it (defined on SOME path — a value that could be
/// clobbered). Gen = each instruction's explicit writes; a call additionally gens
/// its declared OUTPUTS (a produced result is a defined value). Join is UNION;
/// entry seeds the caller's own params. Nothing kills — a value written anywhere
/// upstream still "exists" (whether it survives an intervening clobbering call is
/// the liveness pass's job, not this one's). Monotone growing fixpoint on a
/// finite lattice → terminates.
fn may_defined_in(
    cfg: &Cfg,
    entry: usize,
    params: &BTreeSet<String>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut in_def: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    in_def.insert(entry, params.clone());
    let mut work: VecDeque<usize> = VecDeque::from([entry]);
    while let Some(idx) = work.pop_front() {
        let mut out = in_def[&idx].clone();
        if let Some((mnem, ops)) = cfg.instr(idx) {
            if CALL_MNEMONICS.contains(&mnem) {
                if let Some(c) = direct_target(mnem, ops) {
                    if let Some(o) = callee_out.get(c) {
                        out.extend(o.iter().cloned());
                    }
                }
            } else {
                out.extend(written_names(mnem, ops));
            }
        }
        for edge in cfg.edges(idx) {
            let Edge::Follow(succ) = edge else { continue };
            let changed = match in_def.get(&succ) {
                None => {
                    in_def.insert(succ, out.clone());
                    true
                }
                Some(existing) => {
                    if out.is_subset(existing) {
                        false
                    } else {
                        let merged: BTreeSet<String> = existing.union(&out).cloned().collect();
                        in_def.insert(succ, merged);
                        true
                    }
                }
            };
            if changed {
                work.push_back(succ);
            }
        }
    }
    in_def
}

/// Is `reg` LIVE across the call at `call_idx` — read on some path from the
/// call's successors before it is redefined? A redefine is an explicit write to
/// `reg` OR an intervening call that WRITES `reg` (any effective-set member — a
/// scratch clobber OR a declared output; an indirect / unknown intervening call
/// conservatively ends liveness). A carrier passed as a later call's argument is
/// NOT modeled as a read (false-negative-leaning, the house stance for an
/// error-tier check). Visited-set for CFG joins/loops.
fn live_after_call(
    cfg: &Cfg,
    effective: &BTreeMap<String, RegEffect>,
    call_idx: usize,
    reg: &str,
) -> bool {
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    for e in cfg.edges(call_idx) {
        if let Edge::Follow(i) = e {
            queue.push_back(i);
        }
    }
    while let Some(idx) = queue.pop_front() {
        if !visited.insert(idx) {
            continue;
        }
        let Some((mnem, ops)) = cfg.instr(idx) else { continue };
        // An intervening RETURNING call: it either REDEFINES reg (a redefine — this
        // path is dead) or preserves it (transparent — the value flows on). A
        // redefine is ANY write of reg — a scratch clobber OR a declared OUTPUT
        // (an `out(reg)` call produces a fresh value, so a later read reads the
        // produced value, not the held one). So the kill test is effective-set
        // MEMBERSHIP (which includes outputs), not `destroys_value` (which
        // excludes outputs — that is only for the FIRE decision below).
        if CALL_MNEMONICS.contains(&mnem) {
            let kills = match direct_target(mnem, ops) {
                None => true, // indirect: unknown effect, conservatively kills
                Some(c) => match effective.get(c) {
                    None => true, // unresolved: conservatively kills
                    Some(e) => e.top || e.regs.contains(reg),
                },
            };
            if kills {
                continue; // reg redefined by the intervening call — dead here
            }
            // Preserved: fall through the call to its successors.
            for e in cfg.edges(idx) {
                if let Edge::Follow(i) = e {
                    queue.push_back(i);
                }
            }
            continue;
        }
        let written = written_names(mnem, ops);
        let mentioned = regs_mentioned(ops);
        if mentioned.contains(reg) && !written.contains(reg) {
            return true; // a genuine read of the held value
        }
        if written.contains(reg) {
            continue; // redefined — dead on this path
        }
        for e in cfg.edges(idx) {
            if let Edge::Follow(i) = e {
                queue.push_back(i);
            }
        }
    }
    false
}

/// `[call.live-clobbered]` (D1c): a value defined before a returning call and
/// read after it, held in a register the callee EFFECTIVELY clobbers (post
/// verified-preserves subtraction, minus its declared outputs). This is pass-3's
/// seatbelt — the exact mistake a contract-trusting register hoist could make.
pub fn check_live_clobbered(
    proc_name: &str,
    params: &BTreeSet<String>,
    items: &[CodeItem],
    effective: &BTreeMap<String, RegEffect>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<LiveClobberFiring> {
    let Some(entry) = entry_index(items) else { return Vec::new() };
    let cfg = Cfg::build(items);
    // may-def keeps the FULL `callee_out` — a conditional out IS a produced value
    // there, and a may-def over-approximation is firing-safe. Only the FIRE
    // decision (`destroys_value`) switches to `callee_uncond_out` (Finding 4).
    let defined = may_defined_in(&cfg, entry, params, callee_out);

    let mut firings = Vec::new();
    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        if !CALL_MNEMONICS.contains(&mnemonic.as_str()) {
            continue; // only returning calls have a "read after" in this proc
        }
        let Some(callee) = direct_target(mnemonic, ops) else { continue };
        let Some(e) = effective.get(callee) else { continue };
        let Some(here) = defined.get(&idx) else { continue };
        // Candidate clobbered registers (⊤ ⇒ the whole file), minus outputs.
        let candidates: Vec<String> =
            if e.top { register_universe() } else { e.regs.iter().cloned().collect() };
        for reg in candidates {
            if !destroys_value(effective, callee_uncond_out, callee, &reg) {
                continue; // it is a declared UNCONDITIONAL output, not a destroyed value
            }
            if here.contains(&reg) && live_after_call(&cfg, effective, idx, &reg) {
                firings.push(LiveClobberFiring {
                    proc: proc_name.to_string(),
                    callee: callee.to_string(),
                    reg: reg.clone(),
                    span: *span,
                });
            }
        }
    }
    firings.sort_by(|a, b| (&a.callee, &a.reg, a.span.start).cmp(&(&b.callee, &b.reg, b.span.start)));
    firings
}
