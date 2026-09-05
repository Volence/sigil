//! Z80 verified `out()` by symbolic production tracking — the SIBLING of
//! [`crate::out_verify`] for `(cpu: z80)` modules.
//!
//! A Z80 proc declaring `out(rN)` must PRODUCE rN on every required return path,
//! where "produce" means a value is written on THIS pass. This is a forward
//! MUST-produce dataflow over the SAME lightweight CFG the Z80 preserve proof
//! shares ([`crate::flag_check::Cfg`], read through its Z80 successor model
//! [`Cfg::z80_edges`]), modeled on [`crate::out_verify::verify_out`] — the 68k
//! twin — with the Z80 divergences below:
//!
//! - **Unit domain, not width.** A Z80 out is register-granular: `out(hl)` claims
//!   both halves `h` and `l`, `out(b, c)` claims `b` and `c`, `out(iy)` claims the
//!   16-bit index unit. There is no `.b`/`.w`/`.l` width facet — a unit is either
//!   produced or it is not — so the state is a produced-bit per tracked unit
//!   ([`crate::z80_preserves::UNITS`]), joined by MEET (a unit is produced at a
//!   merge only if produced on every incoming edge). The production detector is the
//!   shared [`crate::z80_preserves::z80_produced_units`] (a write, or a `pop rr`),
//!   so a production claim and a clobber claim never drift on the same instruction.
//! - **No param seed** (68k Finding 2): entry credits NOTHING; a production must
//!   come from a write / callee-out / tail-out / fall-into-out on the path. An
//!   `out(rN)` naming a register the body never writes FIRES.
//! - **Callee-out credit** at an UNCONDITIONAL `call`/`rst` via the shared
//!   verified-out map — a returning call credits the callee's VERIFIED out units. A
//!   `call cc, Foo` credits NOTHING: it returns on both its taken and not-taken
//!   paths (one fall-through edge), so Foo's out is produced only when the condition
//!   held, not on the fall-through — crediting it would bless production that may
//!   never run (over-fire direction). A tail `jp`/`jr` to a known proc credits its
//!   verified out at the [`Edge::TailOut`] exit (a tail transfer is a return of P
//!   from the caller's view — a required return path), and a declared `falls_into S`
//!   credits S's verified out at the [`Edge::FallOff`] end (the
//!   `PsgVolEnv_Resolve → VolEnv_ResolveScan` shape). An external / unresolved
//!   target credits nothing ⇒ any un-produced out fails there.
//! - **Exchanges move, never generate** ([`apply_exchange`]): production is
//!   gen-only, so `ex`/`exx` PERMUTE or INVALIDATE produced bits rather than
//!   setting them — `ex de,hl` swaps `d`↔`h`/`e`↔`l`, `exx` clears {b,c,d,e,h,l},
//!   `ex af,af'` clears `a`, and `ex (sp),rr` clears the pair. This is the sole
//!   place the produce side diverges from [`crate::z80_preserves`]'s clobber view
//!   (a swap DOES change those registers, so the clobber side counts them written).
//! - **Divergent tail** (the noreturn model, and where this twin does NOT
//!   over-obligate): a tail transfer to an `@noreturn` target — or an
//!   `AssertDesugar`-authored raise rail — never returns to P's caller, so it is
//!   not a return path and carries no out obligation. Same predicate the 68k twin
//!   uses ([`crate::preserves::exit_diverges`]).
//! - **BranchOut is not a required return path** — a conditional branch out of the
//!   body (a divergent handler / transitive tail) is skipped, mirroring the 68k
//!   twin. The `z80_edges` builder decides which of `TailOut`/`BranchOut` an exit
//!   is; this module holds no mnemonic table of its own.
//!
//! Soundness polarity: a dishonest out that verifies would falsely credit rN as a
//! reaching definition (the dangerous direction). So the verifier only blesses a
//! production PROVEN on all required return paths; when in doubt it FIRES. This is
//! a MUST analysis (produced-on-all-required-paths = meet join). What it proves is
//! that rN's declared unit holds a value written on this pass, NOT that the value
//! is semantically correct — value-provenance is out of scope, exactly as the 68k
//! twin's header states.
//!
//! The `out(carry: name)` FLAG result is NOT ranged over here: a flag is not a
//! tracked register unit, and the carry result is a separate contract consumed by
//! `[call.flag-result-unused]` (the caller-must-consume check) via the DECLARED
//! flag map. This module checks only the REGISTER out reglist — the `hl`/`b`/… part
//! of `out(hl, carry: found)` — and the carry half rides its own gate. No Z80 proc
//! declares the conditional-REGISTER form `out(rN if cc)` today; were one to
//! appear it would land in the reglist and be charged UNCONDITIONALLY here (the
//! safe polarity — an unconditional obligation over-charges a conditional out
//! rather than blessing it), which the collection guards against by measuring the
//! reglist a proc actually publishes.

use crate::flag_check::{Cfg, Edge};
use crate::preserves::exit_diverges;
use crate::value::{CodeItem, CodeOperand, Z80Pair};
use crate::z80_preserves::{pair_units, unit_idx, z80_produced_units, NU};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Proc name → the set of register-out UNIT names it declares (`out(hl)` →
/// `{h, l}`). Used for both the DECLARED map and the fixpoint's VERIFIED map.
pub type Z80OutMap = BTreeMap<String, BTreeSet<String>>;

/// The proof outcome for one declared output unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Z80OutStatus {
    /// Proven produced on every required return path.
    Produced,
    /// Some required return path does not produce it (a false `out()` claim).
    Unverified(String),
}

/// One `[proc.out-unverified]` firing for a Z80 proc: `proc` declares `out(unit)`
/// but the body does not PRODUCE `unit` on every required return path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Z80OutFiring {
    pub proc: String,
    pub unit: String,
    pub reason: String,
}

/// True for a Z80 CALL — credits its callee's verified out and nets zero on the
/// stack. `rst` is a fixed-vector call with the same return semantics.
fn is_call(mnem: &str) -> bool {
    matches!(mnem, "call" | "rst")
}

/// The bare-`Sym` target of a direct call / tail (`call Foo` / `jp Foo`), or
/// `None` for an indirect / computed / local-label target.
fn direct_target(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) if !name.contains('$') => Some(name.as_str()),
        _ => None,
    })
}

/// The abstract state at a program point: for each tracked unit, whether it is
/// MUST-produced on every path reaching here.
type State = [bool; NU];

/// Record a production of `unit` — gen-only: once produced upstream, a unit stays
/// produced (a later non-write does not un-produce it).
fn produce(st: &mut State, unit: usize) {
    st[unit] = true;
}

/// Credit `target`'s VERIFIED out units into `st` — one helper for all three
/// transfer-out charges (call, tail, `falls_into`), so a callee's out is credited
/// identically wherever control reaches it.
fn credit_target_outs(st: &mut State, target: &str, callee_verified_out: &Z80OutMap) {
    let Some(outs) = callee_verified_out.get(target) else { return };
    for name in outs {
        if let Some(u) = unit_idx(name) {
            produce(st, u);
        }
    }
}

/// Apply instruction `idx`'s effect to `st`: gen each produced unit, credit a
/// returning UNCONDITIONAL call's callee verified outs, and permute/invalidate
/// produced bits across an exchange.
fn transfer(cfg: &Cfg, idx: usize, st: &mut State, callee_verified_out: &Z80OutMap) {
    let Some((mnem, ops)) = cfg.instr(idx) else { return };
    if is_call(mnem) {
        // A `call cc, Foo` returns on BOTH its taken and its not-taken paths —
        // `z80_edges` models it as a single fall-through — so Foo's out is produced
        // only when the condition HELD, not on the fall-through the credit applies
        // to. Crediting it there would bless production that may never have run. Only
        // an UNCONDITIONAL `call` credits its callee (the over-fire direction: a
        // conditional call's out is treated as not produced).
        let leads_cc = matches!(ops.first(), Some(CodeOperand::Z80Cc(_)));
        if !leads_cc {
            if let Some(target) = direct_target(ops) {
                credit_target_outs(st, target, callee_verified_out);
            }
        }
        return;
    }
    // An exchange PERMUTES or INVALIDATES produced bits rather than generating them
    // (production is gen-only, so a cancelling `ex de,hl` pair would otherwise mark
    // hl produced while it holds the untouched entry value — Finding 2).
    if apply_exchange(mnem, ops, st) {
        return;
    }
    for u in z80_produced_units(mnem, ops) {
        produce(st, u);
    }
}

/// Apply a Z80 exchange to the PRODUCED-bit state, returning `true` when `mnem` is
/// an exchange (so the caller skips the gen-only path). This is the produce-side
/// dual of [`crate::z80_preserves`]'s clobber view, and it diverges from it on
/// purpose: a swap DOES change the destination registers (so the clobber/preserve
/// side counts them written), but it does not GENERATE a value on this pass, so
/// here it moves or discards produced bits instead of setting them:
///
/// - `ex de,hl` — a true permutation of the produced bits (`d`↔`h`, `e`↔`l`).
/// - `exx` — the main {b,c,d,e,h,l} bank swaps with the untracked shadow bank, so
///   their produced bits are CLEARED (the incoming shadow value is unproduced;
///   over-fire direction).
/// - `ex af,af'` — swaps `a` (and flags) with the shadow bank, so `a`'s produced
///   bit is CLEARED (`f` is never an out unit).
/// - `ex (sp),hl` / `ex (sp),ix` / `ex (sp),iy` — the pair takes the top-of-stack
///   value, which is unproduced, so the pair's produced bits are CLEARED.
fn apply_exchange(mnem: &str, ops: &[CodeOperand], st: &mut State) -> bool {
    let ui = |n: &str| unit_idx(n).expect("known unit");
    match mnem {
        "exx" => {
            for n in ["b", "c", "d", "e", "h", "l"] {
                st[ui(n)] = false;
            }
            true
        }
        "ex" => {
            match ops.first() {
                Some(CodeOperand::Z80Pair(Z80Pair::De)) => {
                    st.swap(ui("d"), ui("h"));
                    st.swap(ui("e"), ui("l"));
                }
                Some(CodeOperand::Z80Pair(Z80Pair::Af)) | Some(CodeOperand::Z80AfShadow) => {
                    st[ui("a")] = false;
                }
                // `ex (sp),rr` — the pair is whichever Z80Pair operand appears; clear
                // its units (the top-of-stack value it takes on is unproduced).
                _ => {
                    for op in ops {
                        if let CodeOperand::Z80Pair(p) = op {
                            for u in pair_units(*p) {
                                st[u] = false;
                            }
                        }
                    }
                }
            }
            true
        }
        _ => false,
    }
}

/// Join two produced-states on entry to the same node: the MUST meet — a unit is
/// produced only if BOTH paths produce it.
fn join(a: &State, b: &State) -> State {
    let mut out = *a;
    for i in 0..NU {
        out[i] = a[i] && b[i];
    }
    out
}

/// Verify each declared output unit of a Z80 proc over its evaluated CodeBuf.
/// `out_units` are the declared register-out unit names; `callee_verified_out` is
/// the VERIFIED-out map (call / tail-target / `falls_into` credit); `falls_into`
/// is the declared successor's name; `noreturn` is the corpus `@noreturn` set.
pub fn verify_z80_out(
    items: &[CodeItem],
    out_units: &[String],
    callee_verified_out: &Z80OutMap,
    falls_into: Option<&str>,
    noreturn: &BTreeSet<String>,
) -> BTreeMap<String, Z80OutStatus> {
    let checked: Vec<(String, usize)> =
        out_units.iter().filter_map(|n| unit_idx(n).map(|i| (n.clone(), i))).collect();
    let cfg = Cfg::build(items);

    let Some(entry_idx) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        // No body → the only exit is a fall-off with no instructions. A declared
        // `falls_into` continues into the successor's frame, so the claim is charged
        // against the successor's verified outs; without one, an empty body produces
        // nothing and every declared out FIRES (the no-production-from-nothing rule).
        let mut credit = [false; NU];
        if let Some(succ) = falls_into {
            credit_target_outs(&mut credit, succ, callee_verified_out);
        }
        return checked
            .iter()
            .map(|(name, i)| {
                let status = if credit[*i] {
                    Z80OutStatus::Produced
                } else {
                    Z80OutStatus::Unverified(format!(
                        "`{name}` not produced on a required return path"
                    ))
                };
                (name.clone(), status)
            })
            .collect();
    };

    let mut in_state: BTreeMap<usize, State> = BTreeMap::new();
    in_state.insert(entry_idx, [false; NU]);
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);

    // Per checked unit: produced on EVERY required return path so far.
    let mut ok: BTreeMap<usize, bool> = checked.iter().map(|(_, i)| (*i, true)).collect();

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx];
        transfer(&cfg, idx, &mut st, callee_verified_out);

        for edge in cfg.z80_edges(idx) {
            match edge {
                Edge::Follow(succ) => {
                    let changed = match in_state.get(&succ) {
                        None => {
                            in_state.insert(succ, st);
                            true
                        }
                        Some(existing) => {
                            let merged = join(existing, &st);
                            if merged != *existing {
                                in_state.insert(succ, merged);
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
                // A `ret` (incl. the taken edge of a `ret cc`): the caller reads the
                // register file from here, so the claim is due with no extra credit.
                Edge::Return => check_exit(&st, &checked, &mut ok),
                // Control running off the body's end. WITH a declared `falls_into S`
                // the closing `}` continues into S inside the same call, so rN is
                // produced iff this body produced it OR S's verified out produces it.
                // Without one, the fall-off is a plain return.
                Edge::FallOff => {
                    let mut credit = st;
                    if let Some(succ) = falls_into {
                        credit_target_outs(&mut credit, succ, callee_verified_out);
                    }
                    check_exit(&credit, &checked, &mut ok);
                }
                // A conditional branch out is not a required return path — skip it,
                // mirroring the 68k twin.
                Edge::BranchOut => {}
                Edge::TailOut => {
                    // ...UNLESS it DIVERGES. An `@noreturn` tail target or an
                    // `AssertDesugar` raise rail never returns to P's caller, so it is
                    // not a return path and carries no out obligation.
                    if exit_diverges(&items[idx], noreturn) {
                        continue;
                    }
                    // An unconditional tail transfer is a required return path: credit
                    // the tail target's verified out (a known proc producing rN); an
                    // external / unresolved target credits nothing ⇒ an un-produced
                    // out fails here.
                    let Some((_, ops)) = cfg.instr(idx) else { continue };
                    let mut credit = st;
                    if let Some(target) = direct_target(ops) {
                        credit_target_outs(&mut credit, target, callee_verified_out);
                    }
                    check_exit(&credit, &checked, &mut ok);
                }
            }
        }
    }

    checked
        .iter()
        .map(|(name, i)| {
            let status = if ok[i] {
                Z80OutStatus::Produced
            } else {
                Z80OutStatus::Unverified(format!("`{name}` not produced on a required return path"))
            };
            (name.clone(), status)
        })
        .collect()
}

/// At one required-return exit, mark any checked unit not present in `produced` as
/// failing.
fn check_exit(produced: &State, checked: &[(String, usize)], ok: &mut BTreeMap<usize, bool>) {
    for (_, i) in checked {
        if !produced[*i] {
            *ok.get_mut(i).unwrap() = false;
        }
    }
}

/// Run the Z80 `out()` production check for a proc and collect its firings.
pub fn check_z80_out(
    proc_name: &str,
    items: &[CodeItem],
    out_units: &[String],
    callee_verified_out: &Z80OutMap,
    falls_into: Option<&str>,
    noreturn: &BTreeSet<String>,
) -> Vec<Z80OutFiring> {
    verify_z80_out(items, out_units, callee_verified_out, falls_into, noreturn)
        .into_iter()
        .filter_map(|(unit, status)| match status {
            Z80OutStatus::Produced => None,
            Z80OutStatus::Unverified(reason) => {
                Some(Z80OutFiring { proc: proc_name.to_string(), unit, reason })
            }
        })
        .collect()
}

/// The VERIFIED-out fixpoint for the Z80 modules — the analog of
/// [`crate::out_verify::compute_verified_outs`]. A proc's out unit is VERIFIED iff
/// [`verify_z80_out`] proves it PRODUCED on every required return path when
/// callee / tail / `falls_into` credit is drawn ONLY from already-VERIFIED outs.
/// Computed as a least fixpoint: `extern proc` outs seed VERIFIED (boundary
/// axioms — no body to check); body procs start ⊥ (nothing verified) and grow
/// until stable.
///
/// **Monotone ⇒ terminates.** More verified credit can only turn a unit
/// Unverified→Produced (credit only adds produced bits in [`verify_z80_out`]),
/// never the reverse, so each proc's verified set grows monotonically, bounded by
/// its declared outs (finite). A round that adds nothing ends the loop; a hard
/// round cap turns any monotonicity regression into a panic rather than a hang. A
/// mutual/circular out-dependency that never grounds in a LOCAL production stays
/// UNVERIFIED — the correct conservative answer.
pub fn compute_verified_z80_outs(
    proc_items: &BTreeMap<String, &[CodeItem]>,
    declared_out: &Z80OutMap,
    extern_names: &BTreeSet<String>,
    falls_into: &BTreeMap<String, String>,
    noreturn: &BTreeSet<String>,
) -> Z80OutMap {
    let mut verified: Z80OutMap = declared_out
        .iter()
        .map(|(name, outs)| {
            let seed = if extern_names.contains(name) { outs.clone() } else { BTreeSet::new() };
            (name.clone(), seed)
        })
        .collect();

    let total_facts: usize = declared_out.values().map(|s| s.len()).sum();
    let cap = total_facts + 2;

    let mut round = 0usize;
    loop {
        let mut changed = false;
        for (name, items) in proc_items {
            let Some(units) = declared_out.get(name) else { continue };
            if units.is_empty() {
                continue;
            }
            let out_units: Vec<String> = units.iter().cloned().collect();
            let statuses = verify_z80_out(
                items,
                &out_units,
                &verified,
                falls_into.get(name).map(String::as_str),
                noreturn,
            );
            let new_v: BTreeSet<String> = statuses
                .iter()
                .filter(|(_, s)| matches!(s, Z80OutStatus::Produced))
                .map(|(u, _)| u.clone())
                .collect();
            if verified.get(name) != Some(&new_v) {
                verified.insert(name.clone(), new_v);
                changed = true;
            }
        }
        if !changed {
            break;
        }
        round += 1;
        assert!(
            round <= cap,
            "Z80 verified-out fixpoint did not stabilize in {cap} rounds, monotonicity regression"
        );
    }
    verified
}

/// The `[proc.out-unverified]` diagnostic text for one Z80 firing — one wording so
/// a reader meets the same diagnosis from every gate.
pub fn z80_out_message(f: &Z80OutFiring) -> String {
    format!(
        "[proc.out-unverified] `{}` declares `out({})` but does not PRODUCE `{}` on every \
         required return path, {}",
        f.proc, f.unit, f.unit, f.reason
    )
}
