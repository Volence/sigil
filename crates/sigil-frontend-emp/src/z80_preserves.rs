//! Z80 push/pop `preserves` proof (rung-2 §4.2) — the SIBLING of
//! [`crate::preserves`] the overseer ratified (2026-07-29 countersign, item 5).
//!
//! Ruling 2 was amended on its own terms: the shared soundness bailouts the
//! original "don't duplicate" rationale protected are predominantly 68k-specific
//! (movem masks, linear-delta address arithmetic, sp-displacement hazards), while
//! the genuinely shared part — the CPU-agnostic CFG ([`crate::flag_check::Cfg`]) —
//! is ALREADY shared. So this sibling REUSES that CFG — including its Z80
//! successor model, [`Cfg::z80_edges`], which is the ONE place a Z80 edge is
//! classified — and duplicates nothing that matters; `preserves.rs` stays
//! byte-untouched.
//!
//! The proof mirrors `preserves.rs`'s dataflow SKELETON: a forward dataflow over
//! the shared CFG tracking a symbolic stack of slots (each tagged with which
//! register PAIR's entry value it holds) plus a per-register entry-value bit; a
//! `preserves(rN)` holds iff on every return path rN holds its entry value. The
//! conventions cited so future drift is findable:
//!
//! - **call clobbers all / nets zero on the stack** — mirrors `preserves.rs:344`
//!   (`transfer`'s `is_call` arm): a `call` clobbers every register's entry bit
//!   (no local callee contract) but pushes/pops its own return address, so it nets
//!   zero on our slot stack.
//! - **join meets entry bits by AND; a depth mismatch bails** — mirrors
//!   `preserves.rs:506` (`join`).
//! - **a bail is path-local and only matters if it reaches an exit** — mirrors
//!   `preserves.rs` (`bailed_reached_exit`).
//!
//! Z80 divergences from the 68k proof, by design (§4.2):
//! - every save is a `push rr` / `pop rr` PAIR (no movem, no single-register push);
//!   a pair slot bundles its two 8-bit halves.
//! - the `push R / pop R'` MOVE idiom (§1.3): `push ix / pop hl` copies ix→hl —
//!   the slot holds ix's value, so `pop hl` CLOBBERS hl (foreign value) and leaves
//!   ix untouched (never written). The slot/entry-bit model gets this for free.
//! - `ex (sp),hl` and any displaced/computed sp op is a REPRESENTED loud bailout
//!   (the rung-3 sequencer trampoline; wired-to-bail now so the proof stays sound
//!   the day it appears).

use crate::flag_check::{Cfg, Edge};
use crate::preserves::{exit_diverges, PreserveStatus};
use crate::value::{CodeItem, CodeOperand, Z80Pair, Z80Reg8};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The callee-declared-preserves oracle for the Z80 sibling — the analog of
/// `preserves.rs`'s `CallPolicy::Oracle` (`preserves.rs:77`) / `call_preserves`
/// (`preserves.rs:83`). A map from a visible callee name (a module proc, or an
/// `extern proc`) to the register UNITS its contract preserves: a local proc's
/// declared `preserves` UNIONED with the module `invariant`; an `extern proc`'s
/// declared `preserves`.
///
/// **Soundness.** A local callee's declared preservation is itself proven by THIS
/// SAME per-proc check (an inherited `invariant(ix)` makes every local proc prove
/// it preserves `ix`; a broken one fires and stops the build), and an `extern`
/// callee's is trusted by the extern convention — the same trust boundary the 68k
/// oracle's `effective` map draws. A caller's OWN direct write to a register always
/// clears its entry bit regardless of any callee credit, so crediting a callee can
/// never mask a local clobber. Unlike the 68k closure (which folds SCCs to
/// conservative), this per-proc map trusts declarations directly — the divergence
/// is recorded in the design note §13.5; it is sound for the reason above and psg
/// is acyclic.
pub type CalleePreserves = BTreeMap<String, BTreeSet<String>>;

/// Does a call/tail-transfer to `callee` (`None` = indirect / no symbol target)
/// provably PRESERVE the register UNIT `unit`, per the oracle? A callee ABSENT
/// from the map — unknown or indirect — preserves nothing (conservative
/// clobber-all), mirroring `callee_clobbers`' `None => true` (`preserves.rs:632`).
fn callee_preserves(map: &CalleePreserves, callee: Option<&str>, unit: &str) -> bool {
    callee.and_then(|c| map.get(c)).is_some_and(|s| s.contains(unit))
}

/// The Z80 register UNITS the proof tracks — the 8-bit halves plus the index
/// units (`sp` is stack discipline, the a7 analog, never tracked). Indexed
/// `a=0 … l=7, ix=8, iy=9`. Shared with [`crate::z80_out_verify`] so the out
/// production twin ranges over the SAME unit universe the preserve proof does.
pub(crate) const UNITS: [&str; 10] = ["a", "f", "b", "c", "d", "e", "h", "l", "ix", "iy"];
pub(crate) const NU: usize = 10;

/// The bail reason when an exit edge (`ret`/fall-off/tail-transfer) is reached with a
/// tracked push slot still live — a cross-seam `push` here / `pop` in the successor
/// the local stack model cannot pair. Conservatively unprovable (honest), so nothing
/// but a never-written module-invariant unit is creditable past it.
const STACK_AT_EXIT_BAIL: &str = "exit reached with a non-empty push stack (a cross-seam push/pop the local model cannot pair)";

/// The unit index of a register spelling, or `None` for a non-tracked name.
/// Shared with [`crate::z80_out_verify`] so both proofs map a contract-reglist
/// unit name onto the same index.
pub(crate) fn unit_idx(name: &str) -> Option<usize> {
    UNITS.iter().position(|u| *u == name)
}

/// The component units a register pair covers (`bc → {b,c}`, `af → {a,f}`;
/// `ix`/`iy` are single 16-bit units; `sp` covers nothing — stack discipline).
pub(crate) fn pair_units(p: Z80Pair) -> Vec<usize> {
    match p {
        Z80Pair::Bc => vec![idx("b"), idx("c")],
        Z80Pair::De => vec![idx("d"), idx("e")],
        Z80Pair::Hl => vec![idx("h"), idx("l")],
        Z80Pair::Af => vec![idx("a"), idx("f")],
        Z80Pair::Ix => vec![idx("ix")],
        Z80Pair::Iy => vec![idx("iy")],
        Z80Pair::Sp => vec![],
    }
}

/// Infallible [`unit_idx`] for the const unit names.
fn idx(name: &str) -> usize {
    unit_idx(name).expect("known unit")
}

/// The tracked unit an 8-bit register names. `f` (flags) is not a `Z80Reg8` — it
/// reaches the unit set only as the low half of the `af` pair (push/pop af).
fn reg8_unit(r: Z80Reg8) -> usize {
    match r {
        Z80Reg8::A => idx("a"),
        Z80Reg8::B => idx("b"),
        Z80Reg8::C => idx("c"),
        Z80Reg8::D => idx("d"),
        Z80Reg8::E => idx("e"),
        Z80Reg8::H => idx("h"),
        Z80Reg8::L => idx("l"),
    }
}

/// The register units a plain (non-push/pop/call) Z80 instruction WRITES — the
/// Z80 analog of `preserves.rs`'s `instr_written_regs`. Z80 is dest-FIRST
/// (`ld a, 5` writes `a`), unlike 68k dest-last. A curated, corpus-scoped
/// allowlist (false-negative-leaning like the 68k detector): a memory-destination
/// form (`ld (hl),a`, `set n,(ix+d)`) writes NO register. `sp`/`i`/`r` and shadow
/// units are not in the tracked universe.
fn z80_writes(mnem: &str, ops: &[CodeOperand]) -> Vec<usize> {
    let mut w = z80_writes_regs(mnem, ops);
    // The flag unit `f`, modeled as the COMPLEMENT of a flag-neutral allowlist
    // ([`z80_flag_neutral`]): an instruction WRITES `f` unless it is provably
    // flag-neutral. The conservative polarity is sound for `preserves(f)` — a
    // forgotten neutral mnemonic over-fires visibly rather than letting a
    // `preserves(f)` whose body writes the flags pass.
    if !z80_flag_neutral(mnem, ops) {
        w.push(idx("f"));
    }
    w
}

/// The REGISTER units (excluding `f`) a plain Z80 instruction writes — the curated,
/// false-negative-leaning allowlist. [`z80_writes`] layers the `f` model on top.
fn z80_writes_regs(mnem: &str, ops: &[CodeOperand]) -> Vec<usize> {
    let dst_units = |op: Option<&CodeOperand>| -> Vec<usize> {
        match op {
            Some(CodeOperand::Z80Reg8(r)) => vec![reg8_unit(*r)],
            Some(CodeOperand::Z80Pair(p)) => pair_units(*p),
            _ => vec![], // memory / index-mem / immediate destination writes no register
        }
    };
    match mnem {
        // dest-first two-operand + unary register writes.
        "ld" | "inc" | "dec" => dst_units(ops.first()),
        // Accumulator ALU (`add a,r` / `xor r`) writes `a`; the 16-bit
        // `add hl,rr` / `adc hl,rr` / `sbc hl,rr` / `add ix,rr` forms write the
        // destination PAIR (the first operand).
        "add" | "adc" | "sub" | "sbc" | "and" | "or" | "xor" => {
            match ops.first() {
                Some(CodeOperand::Z80Pair(p)) => pair_units(*p),
                _ => vec![idx("a")],
            }
        }
        // Accumulator-only writers.
        "neg" | "cpl" | "daa" | "rlca" | "rrca" | "rla" | "rra" => vec![idx("a")],
        // CB shifts/rotates write their (first) register operand; `(hl)`/`(ix+d)`
        // targets write memory (no register).
        "rlc" | "rrc" | "rl" | "rr" | "sla" | "sra" | "srl" | "sll" => dst_units(ops.first()),
        // `set`/`res` write their TARGET (second operand); `bit` writes nothing.
        "set" | "res" => dst_units(ops.get(1)),
        // `ex de,hl` writes de and hl; `ex af,af'` writes af.
        "ex" => match ops.first() {
            Some(CodeOperand::Z80Pair(Z80Pair::De)) => {
                let mut v = pair_units(Z80Pair::De);
                v.extend(pair_units(Z80Pair::Hl));
                v
            }
            Some(CodeOperand::Z80Pair(Z80Pair::Af)) | Some(CodeOperand::Z80AfShadow) => {
                pair_units(Z80Pair::Af)
            }
            _ => vec![],
        },
        // `exx` swaps the main {bc,de,hl} with the shadow bank — from the caller's
        // view the main bank reads clobbered (a lone `exx`; a balanced pair is
        // rung-4's shadow model, §4.3). Conservatively clobber bc/de/hl.
        "exx" => vec![idx("b"), idx("c"), idx("d"), idx("e"), idx("h"), idx("l")],
        // `djnz` decrements its counter `b` (dest-implicit) and branches — it writes
        // `b`. (Its flag-neutrality is handled in `z80_flag_neutral`; `djnz` leaves
        // the flags.)
        "djnz" => vec![idx("b")],
        // `ldir` — the block transfer `(hl++)→(de++), bc--` until bc=0: it writes the
        // pointer/counter pairs bc, de, hl. It is the ONLY assemblable block op in the
        // ISA enum (sigil-isa/z80.rs `Mnemonic`); the day the LD-block siblings
        // (`ldi`/`ldd`/`lddr`, same {bc,de,hl} writes-set) or the CP-block family
        // (`cpi`/`cpd`/`cpir`/`cpdr`, which write bc + hl but NOT de) become
        // assemblable, they join here. `ldir` is NOT flag-neutral (it clears P/V), so
        // its `f` write comes through the [`z80_flag_neutral`] complement above.
        "ldir" => vec![idx("b"), idx("c"), idx("d"), idx("e"), idx("h"), idx("l")],
        _ => vec![],
    }
}

/// The register UNITS a plain Z80 instruction PRODUCES — a value written on this
/// pass — for [`crate::z80_out_verify`]'s out-honesty dataflow. It is
/// [`z80_writes_regs`] (the register write set, WITHOUT the `f` flag unit: an out
/// register is never `f`) PLUS a `pop rr`, which loads its pair from the stack and
/// so writes each component register. `push` produces nothing (it reads a pair);
/// `call`/`rst` credit is the callee's declared out, applied by the caller in
/// [`crate::z80_out_verify`], not here. Sharing the write detector with the
/// preserve proof is what keeps a production claim and a clobber claim from
/// drifting apart on the same instruction.
pub(crate) fn z80_produced_units(mnem: &str, ops: &[CodeOperand]) -> Vec<usize> {
    let mut w = z80_writes_regs(mnem, ops);
    if mnem == "pop" {
        if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
            for u in pair_units(*p) {
                if !w.contains(&u) {
                    w.push(u);
                }
            }
        }
    }
    w
}

/// Does this Z80 mnemonic leave the flag register `f` UNTOUCHED? The complement of
/// the F-writer classification [`z80_writes`] layers on. The polarity is
/// conservative: an unmodeled mnemonic is treated as a flag WRITER (returns
/// `false`), so a forgotten neutral instruction over-fires visibly rather than
/// silently false-passing a `preserves(f)`. It shares nothing with
/// `flag_check.rs`'s CARRY model on purpose — carry is a strict subset of `f`, so a
/// single carry table would miss the flag-ONLY writers (`inc`/`dec`/`bit`),
/// reopening the hole.
///
/// Flag-neutral: `ld` (moves never touch flags — the ISA's `ld i,a`/`ld r,a`
/// (`LdIA`/`LdRA`) write `i`/`r`, not the flags. The flag-writing `ld a,i`/`ld a,r`
/// READ forms — which load `a` and set S/Z + copy IFF2 into P/V — are unassemblable
/// today (no `LdAI`/`LdAR` variant in the ISA `Mnemonic` enum) and MUST enter the
/// F-writer set the day those forms land; see the gap ledger's kill condition. This
/// is why `Psg_EnvCursorReset () preserves(af)` with a `ld (ix+d), 0` body stays green);
/// `push`/`pop` (the preserve idiom — `pop af`'s `f` load is modeled through the
/// slot machinery, not here, so `SndDrv_ISR preserves(af)` stays green via its
/// `push af`/`pop af` bracket); `ex`/`exx` (data moves); the control transfers
/// `jp`/`jr`/`djnz`/`call`/`rst`/`ret`/`reti`/`retn` (flags flow through their
/// edges/the callee oracle, not this local write); `set`/`res`/`out`/`nop`/
/// `di`/`ei`/`halt`/`im`. NOT neutral — `bit` (writes Z/H/N), every ALU/shift/
/// rotate, `scf`/`ccf`, `ldir`&block ops. 16-bit `inc`/`dec` of a PAIR is
/// flag-neutral; the 8-bit and memory forms write flags.
fn z80_flag_neutral(mnem: &str, ops: &[CodeOperand]) -> bool {
    match mnem {
        "ld" | "push" | "pop" | "ex" | "exx" | "jp" | "jr" | "djnz" | "call" | "rst" | "ret"
        | "reti" | "retn" | "set" | "res" | "out" | "nop" | "di" | "ei" | "halt" | "im" => true,
        // 16-bit inc/dec (a pair operand) leaves the flags; the 8-bit register and
        // memory forms write them.
        "inc" | "dec" => matches!(ops.first(), Some(CodeOperand::Z80Pair(_))),
        _ => false,
    }
}

/// The register UNITS a Z80 proc's OWN body writes DIRECTLY (its `local_writes`
/// for the transitive-clobber closure, [`crate::closure`]) — the union of
/// [`z80_writes`] over its instructions. `call`/`rst` (closure edges) and
/// `push`/`pop` (the preserve idiom) contribute nothing here, exactly as the
/// per-proc preserves proof treats them; the curated, false-negative-leaning
/// [`z80_writes`] detector is shared so the closure never drifts from the proof.
pub fn z80_written_registers(buf: &crate::value::CodeBuf) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for it in &buf.items {
        if let CodeItem::Instr { mnemonic, ops, .. } = it {
            for u in z80_writes(mnemonic, ops) {
                out.insert(UNITS[u].to_string());
            }
        }
    }
    out
}

/// True for a Z80 CALL (clobbers every register, nets zero on the stack).
fn is_call(mnem: &str) -> bool {
    matches!(mnem, "call" | "rst")
}

/// An unmodeled sp hazard → bail: `ex (sp),hl`/`ex (sp),ix` (the rung-3
/// trampoline alias), or a direct sp write that is not a push/pop
/// (`ld sp,…` / `inc sp` / `dec sp` / `add hl,sp`-into-sp — replaces or
/// recomputes the stack pointer). The clean stack forms are `push`/`pop` only.
fn sp_hazard(mnem: &str, ops: &[CodeOperand]) -> bool {
    // `ex (sp),hl` / `ex (sp),ix` — the top-of-stack swap (the rung-3 sequencer
    // trampoline: `push hl` + `ex (sp),hl` + `ret` = a computed dispatch through
    // the return address). The distinct `Z80IndSp` operand (t36) lands here so the
    // swap is a REPRESENTED loud bail — the pushed slot becomes the jump target
    // and `ret` leaves this proc's linear extent, so no `preserves` may be
    // credited past it except a module-invariant unit (see the verdict tightening).
    if mnem == "ex" && ops.iter().any(|o| matches!(o, CodeOperand::Z80IndSp)) {
        return true;
    }
    // A direct write to `sp` that is not a push/pop.
    match mnem {
        "ld" | "inc" | "dec" => {
            matches!(ops.first(), Some(CodeOperand::Z80Pair(Z80Pair::Sp)))
        }
        _ => false,
    }
}

/// A stack slot: the register PAIR pushed and each component unit's entry bit at
/// push time. `pop` of the SAME pair restores those bits; `pop` of a DIFFERENT
/// pair (the move idiom) clobbers the popped register instead.
#[derive(Clone, PartialEq, Eq)]
struct Slot {
    pair: Z80Pair,
    /// `(unit, entry-bit-at-push)` for each component unit of `pair`.
    saved: Vec<(usize, bool)>,
}

#[derive(Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Option<Slot>>,
    entry: [bool; NU],
    bailed: bool,
}

/// Verify `preserves(name)` for each register name in `check` over a Z80 proc's
/// evaluated CodeBuf `items`. One forward dataflow serves all checked registers.
/// A name outside the tracked Z80 universe is reported `Verified` vacuously (the
/// caller validated register spelling upstream).
///
/// `noreturn` is the corpus `@noreturn` symbol set: a tail transfer to a diverging
/// target (an `@noreturn` proc, or an `AssertDesugar` raise rail) is NOT a return
/// path — the callee never hands control back to P's caller — so it carries no
/// preserve obligation and clobbers nothing P's caller observes. This mirrors the
/// 68k twin ([`crate::preserves`] calls [`exit_diverges`] at its transfer-out
/// arm); without it, a `preserves(rN)` proc whose only exit tails into a noreturn
/// handler is charged for a path that never returns (over-obligation). Corpus-dead
/// today — no `preserves`-declaring Z80 proc tails into a noreturn target — so this
/// is byte- and residue-neutral, closing the gap before a proc of that shape lands.
pub fn verify_z80_preserved(
    items: &[CodeItem],
    check: &[String],
    invariant_regs: &[String],
    callee_map: &CalleePreserves,
    falls_into: Option<&str>,
    noreturn: &BTreeSet<String>,
) -> BTreeMap<String, PreserveStatus> {
    let checked: Vec<(String, usize)> =
        check.iter().filter_map(|n| unit_idx(n).map(|i| (n.clone(), i))).collect();
    // The module-invariant unit set (t36): the ONLY units creditable past a loud
    // bail, because the invariant is machine-checked on every module proc incl.
    // every dispatch target the bail transfers into.
    let invariant_units: Vec<usize> = invariant_regs.iter().filter_map(|n| unit_idx(n)).collect();

    let cfg = Cfg::build(items);
    let Some(entry_idx) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        // No instructions: the body touches nothing, so control flows straight to the
        // exit. A declared `falls_into` makes that exit continue into the successor's
        // frame — rN survives iff the successor preserves it (the same charge the
        // FallOff arm applies; an absent/unknown successor preserves nothing). Without
        // a `falls_into`, an empty body preserves everything.
        return check
            .iter()
            .map(|n| {
                let status = match falls_into {
                    Some(succ) if !callee_preserves(callee_map, Some(succ), n) => {
                        PreserveStatus::NotPreserved
                    }
                    _ => PreserveStatus::Verified,
                };
                (n.clone(), status)
            })
            .collect();
    };

    // Which units are EVER written — a plain write, a pop, or a call/tail-transfer
    // to a callee that does NOT provably preserve the unit (the callee oracle, gap
    // 2). Only a never-written unit stays preserved past a bailout (its value is
    // immutable) — mirrors `preserves.rs:127` (`ever_clobbered`). A call that
    // preserves a unit no longer marks it clobbered (the pre-oracle `has_call ⇒ all`
    // was over-conservative and would false-fire on a bail+call+return proc).
    let mut ever_clobbered = [false; NU];
    for it in items {
        if let CodeItem::Instr { mnemonic, ops, .. } = it {
            if is_call(mnemonic) {
                let callee = branch_sym(ops);
                for (u, name) in UNITS.iter().enumerate() {
                    if !callee_preserves(callee_map, callee, name) {
                        ever_clobbered[u] = true;
                    }
                }
            }
            // An external tail transfer (`jp`/`jr` to a symbol that is neither a
            // local label nor a local end-label) exits into the tail-callee, which
            // clobbers every unit it does not preserve. A local (end-)label jump is
            // an in-proc fall-off, not a tail call — excluded. A DIVERGING tail (an
            // `@noreturn` target / raise rail) never returns to P's caller, so it
            // clobbers nothing P's caller observes — excluded too.
            if matches!(mnemonic.as_str(), "jp" | "jr") && !exit_diverges(it, noreturn) {
                if let Some(t) = branch_sym(ops) {
                    if cfg.label_index(t).is_none() && !cfg.is_local_label(t) {
                        for (u, name) in UNITS.iter().enumerate() {
                            if !callee_preserves(callee_map, Some(t), name) {
                                ever_clobbered[u] = true;
                            }
                        }
                    }
                }
            }
            for u in z80_writes(mnemonic, ops) {
                ever_clobbered[u] = true;
            }
            if mnemonic == "pop" {
                if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
                    for u in pair_units(*p) {
                        ever_clobbered[u] = true;
                    }
                }
            }
        }
    }
    // A declared `falls_into` successor is a tail transfer exactly like an external
    // `jr`: the body's closing `}` continues into it, so it clobbers every unit it
    // does not preserve (mirrors the `jp`/`jr` external-transfer marking above). An
    // absent/unknown successor preserves nothing, via the oracle.
    if let Some(succ) = falls_into {
        for (u, name) in UNITS.iter().enumerate() {
            if !callee_preserves(callee_map, Some(succ), name) {
                ever_clobbered[u] = true;
            }
        }
    }

    let mut in_state: BTreeMap<usize, State> = BTreeMap::new();
    in_state.insert(entry_idx, State { stack: Vec::new(), entry: [true; NU], bailed: false });
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    let mut bail_reason: Option<String> = None;
    let mut bailed_reached_exit = false;
    let mut saw_exit = false;
    let mut all_returns_preserve: BTreeMap<usize, bool> =
        checked.iter().map(|(_, i)| (*i, true)).collect();

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        if !st.bailed {
            if let Some(reason) = transfer(&cfg, idx, &mut st, callee_map) {
                st.bailed = true;
                bail_reason.get_or_insert(reason);
            }
        }
        for edge in cfg.z80_edges(idx) {
            match edge {
                Edge::Follow(succ) => {
                    let changed = match in_state.get(&succ) {
                        None => {
                            in_state.insert(succ, st.clone());
                            true
                        }
                        Some(existing) => {
                            let mut merged = existing.clone();
                            join(&mut merged, &st);
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
                // A `ret`: the caller reads the register file from here —
                // checkpoint it. An early `ret` in a `falls_into` proc still
                // checkpoints as an exit (its return happens here, not in the tail).
                Edge::Return => {
                    saw_exit = true;
                    if st.bailed || !st.stack.is_empty() {
                        bailed_reached_exit = true;
                        if !st.bailed {
                            bail_reason.get_or_insert_with(|| STACK_AT_EXIT_BAIL.to_string());
                        }
                    } else {
                        for (_, i) in &checked {
                            if !st.entry[*i] {
                                *all_returns_preserve.get_mut(i).unwrap() = false;
                            }
                        }
                    }
                }
                // Control running off the end of the body. In a proc with a declared
                // `falls_into` successor, the closing `}` continues into that
                // successor's frame — semantically identical to an explicit `jr`
                // tail (`Edge::TailOut`): rN survives iff it holds its entry value here
                // AND the successor itself preserves it. Without a `falls_into`,
                // running off the end is a plain exit (checkpoint the entry bits).
                Edge::FallOff => {
                    saw_exit = true;
                    if st.bailed || !st.stack.is_empty() {
                        bailed_reached_exit = true;
                        if !st.bailed {
                            bail_reason.get_or_insert_with(|| STACK_AT_EXIT_BAIL.to_string());
                        }
                    } else if let Some(succ) = falls_into {
                        for (name, i) in &checked {
                            if !(st.entry[*i] && callee_preserves(callee_map, Some(succ), name)) {
                                *all_returns_preserve.get_mut(i).unwrap() = false;
                            }
                        }
                    } else {
                        for (_, i) in &checked {
                            if !st.entry[*i] {
                                *all_returns_preserve.get_mut(i).unwrap() = false;
                            }
                        }
                    }
                }
                // A transfer to a non-local target is an EXIT: this proc's real
                // return happens inside the callee. Both flavors arrive here — an
                // unconditional `jp`/`jr` tail, and the TAKEN leg of a conditional
                // `jr cc`/`jp cc`/`djnz` — and both are charged the same way, which
                // OVER-obligates a conditional leg rather than missing it, this
                // check's safe polarity. Closing the §13.4 vacuous `!saw_exit` hole
                // (gap 3) — a proc that ONLY transfers out used to pass `preserves`
                // vacuously, silently verifying a contract its body breaks. The proc
                // preserves rN across the transfer iff rN holds its entry value AT
                // the transfer AND the callee itself preserves rN (unknown/indirect
                // target → conservative clobber, via the oracle). Mirrors the
                // `Return`/`FallOff` arm plus the callee oracle.
                Edge::TailOut | Edge::BranchOut => {
                    // A DIVERGING transfer (an `@noreturn` target / raise rail) is
                    // not a return path — control never reaches P's caller — so it
                    // carries no preserve obligation. Same predicate the 68k twin
                    // uses; skip this edge (a sibling fall-through, if any, still
                    // charges). Without this, a preserve claim would be charged
                    // against a path that never returns.
                    if exit_diverges(&items[idx], noreturn) {
                        continue;
                    }
                    saw_exit = true;
                    if st.bailed || !st.stack.is_empty() {
                        bailed_reached_exit = true;
                        if !st.bailed {
                            bail_reason.get_or_insert_with(|| STACK_AT_EXIT_BAIL.to_string());
                        }
                    } else {
                        let tail_callee = cfg.instr(idx).and_then(|(_, ops)| branch_sym(ops));
                        for (name, i) in &checked {
                            if !(st.entry[*i] && callee_preserves(callee_map, tail_callee, name)) {
                                *all_returns_preserve.get_mut(i).unwrap() = false;
                            }
                        }
                    }
                }
            }
        }
    }

    checked
        .iter()
        .map(|(name, i)| {
            let status = if !saw_exit {
                PreserveStatus::Verified
            } else if bailed_reached_exit {
                // TIGHTENED (t36 ruling iii): once ANY bailed path reaches a return,
                // the `ret` continues execution INSIDE the caller-observed extent (the
                // trampoline dispatches into handler procs; an `ld sp` recomputes the
                // frame). A unit may be credited Verified ONLY if it is (a) never
                // written locally AND (b) in the module-invariant set — the invariant
                // is machine-checked on EVERY module proc including all dispatch
                // targets, so whatever the bail transfers into preserves it
                // (verification-grade, not local optimism). Every OTHER unit — incl. a
                // locally-untouched NON-invariant register (the silent-Verified hole a
                // handler could clobber) — is loudly Unverifiable, naming the bail.
                if !ever_clobbered[*i] && invariant_units.contains(i) {
                    PreserveStatus::Verified
                } else {
                    PreserveStatus::Unverifiable(
                        bail_reason.clone().unwrap_or_else(|| "unverifiable stack".to_string()),
                    )
                }
            } else if all_returns_preserve[i] {
                PreserveStatus::Verified
            } else {
                PreserveStatus::NotPreserved
            };
            (name.clone(), status)
        })
        .collect()
}

/// Apply instruction `idx`'s effect to `st`. `Some(reason)` on a soundness bail.
fn transfer(cfg: &Cfg, idx: usize, st: &mut State, callee_map: &CalleePreserves) -> Option<String> {
    let (mnem, ops) = cfg.instr(idx)?;

    // A call clobbers every register the callee does NOT provably preserve (the
    // callee oracle, gap 2 — the Z80 analog of `preserves.rs:411`'s
    // `call_preserves` loop); an unknown/indirect callee preserves nothing, so it
    // clobbers all (the pre-oracle behavior). It nets zero on the stack (its pushed
    // return address is popped by its own `ret`) — mirrors `preserves.rs:344`.
    if is_call(mnem) {
        let callee = branch_sym(ops);
        for (u, name) in UNITS.iter().enumerate() {
            if !callee_preserves(callee_map, callee, name) {
                st.entry[u] = false;
            }
        }
        return None;
    }

    if sp_hazard(mnem, ops) {
        return Some("unmodeled sp manipulation (ld/inc/dec sp or ex (sp),rr)".to_string());
    }

    // PUSH — save a pair slot capturing each component unit's entry bit.
    if mnem == "push" {
        if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
            let saved: Vec<(usize, bool)> = pair_units(*p).into_iter().map(|u| (u, st.entry[u])).collect();
            st.stack.push(Some(Slot { pair: *p, saved }));
        } else {
            st.stack.push(None); // pushing a non-pair (defensive) → opaque slot
        }
        return None;
    }

    // POP — restore into the popped pair. Popping more than was pushed underflows
    // into the caller's frame → bail (mirrors `preserves.rs:396`).
    if mnem == "pop" {
        if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
            let Some(slot) = st.stack.pop() else {
                return Some("stack underflow — pop drains more than was pushed".to_string());
            };
            for u in pair_units(*p) {
                // The popped register holds ITS entry value iff the slot held the
                // SAME pair's snapshot of that unit; a cross-pair pop (the move
                // idiom, `push ix / pop hl`) clobbers it (foreign value).
                st.entry[u] = slot
                    .as_ref()
                    .filter(|s| s.pair == *p)
                    .and_then(|s| s.saved.iter().find(|(su, _)| *su == u).map(|(_, b)| *b))
                    .unwrap_or(false);
            }
        } else {
            st.stack.pop();
        }
        return None;
    }

    // A plain instruction: every register write clears the unit's entry bit.
    for u in z80_writes(mnem, ops) {
        st.entry[u] = false;
    }
    None
}

/// The last bare-`Sym` operand of a branch/tail instruction (its target label).
fn branch_sym(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        _ => None,
    })
}

/// Join `other` into `acc` — entry bits meet by AND, slots meet pointwise, a
/// depth mismatch taints the merge `bailed`. Mirrors `preserves.rs:506`.
fn join(acc: &mut State, other: &State) {
    acc.bailed |= other.bailed;
    if acc.stack.len() != other.stack.len() {
        acc.bailed = true;
        return;
    }
    for (a, b) in acc.stack.iter_mut().zip(other.stack.iter()) {
        if *a != *b {
            *a = None;
        }
    }
    for i in 0..NU {
        acc.entry[i] = acc.entry[i] && other.entry[i];
    }
}
