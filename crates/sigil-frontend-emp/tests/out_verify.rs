//! Contract-grammar v2 §G4.5 — verified `out()` by symbolic production tracking.
//!
//! A proc declaring `out(rN)` must PRODUCE rN on every required return path: a
//! full-width (`.l`) data-register write / `moveq`, any address-register write or
//! advance, a callee's UNCONDITIONAL `out(rN)` at a call, or a tail-transfer
//! target's UNCONDITIONAL `out(rN)` at a `Defer`. A param is NOT a production (no
//! seed). A `.w`/`.b` data write leaves the high word stale and does NOT verify
//! (mirrors `preserves`'s `is_long`). Conditional `out(rN if cc)` is obligated
//! only on the cc-success return paths; an UNKNOWN cc at a return keeps the
//! obligation (false-positive-leaning = sound).
//!
//! Every case is driven end-to-end from real `.emp` through `eval_proc_body`.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::out_verify::{verify_out, OutStatus};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, Reg};
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// Eval every proc in `src`, returning name → evaluated CodeItems.
fn eval_all(src: &str) -> BTreeMap<String, Vec<CodeItem>> {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse: {diags:?}");
    let mut out = BTreeMap::new();
    let mut counter = 0u32;
    for item in &file.items {
        if let Item::Proc(p) = item {
            let (buf, _d, next) =
                eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, counter, Cpu::M68000, &[]);
            counter = next;
            if let Some(buf) = buf {
                out.insert(p.name.clone(), buf.items);
            }
        }
    }
    out
}

/// A hand-built `callee_uncond_out` map from `(name, regs)` pairs.
fn map(entries: &[(&str, &[Reg])]) -> BTreeMap<String, BTreeSet<String>> {
    entries
        .iter()
        .map(|(n, regs)| (n.to_string(), regs.iter().map(|r| r.to_string()).collect()))
        .collect()
}

/// Verify a single UNCONDITIONAL `out(reg)` on the named proc's body.
fn status_uncond(
    src: &str,
    proc: &str,
    reg: Reg,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) -> OutStatus {
    let all = eval_all(src);
    let items = all.get(proc).unwrap_or_else(|| panic!("no proc {proc}"));
    let no_cond: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    verify_out(items, &[reg], &[], callee_uncond_out, &no_cond)
        .remove(&reg)
        .expect("status for the checked reg")
}

fn is_produced(s: &OutStatus) -> bool {
    matches!(s, OutStatus::Produced)
}
fn is_unverified(s: &OutStatus) -> bool {
    matches!(s, OutStatus::Unverified(_))
}

// === 1. still-fires + callee-sourced positive =============================

/// An unconditional `out(a1)` where a1 is produced on the success path but NOT
/// on the pool-exhausted return (`.full` does `moveq #1,d0; rts`, a1 untouched)
/// ⇒ FIRES. The AllocDynamic-shaped dishonest unconditional out.
#[test]
fn unproduced_on_some_return_fires() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             cmpi.w  #0, Flag\n\
             beq     .full\n\
             lea     Slot, a1\n\
             moveq   #0, d0\n\
             rts\n\
         .full:\n\
             moveq   #1, d0\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_unverified(&s), "a1 unproduced on the .full path → Unverified, got {s:?}");
}

/// `out(a1)` where a1 is produced by a callee's UNCONDITIONAL `out(a1)` at a
/// `jsr` on every path ⇒ verifies (the Load_Object←AllocDynamic shape).
#[test]
fn callee_sourced_out_verifies() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             jbsr    AllocDynamic\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[("AllocDynamic", &[Reg::A1])]),
    );
    assert!(is_produced(&s), "a1 from callee out(a1) → Produced, got {s:?}");
}

/// A produced-on-every-path unconditional out verifies: `lea` on both branches.
#[test]
fn produced_on_all_paths_verifies() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             cmpi.w  #0, Flag\n\
             beq     .other\n\
             lea     A, a1\n\
             rts\n\
         .other:\n\
             lea     B, a1\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_produced(&s), "a1 lea'd on every path → Produced, got {s:?}");
}

// === 2. width (Finding 1) =================================================

/// `out(d0)` produced only by a `.w` write leaves the high word stale ⇒ FIRES.
#[test]
fn word_write_of_data_reg_does_not_verify() {
    let s = status_uncond(
        "module m\n\
         proc P (d1: u16) clobbers() out(d0) {\n\
             move.w  d1, d0\n\
             rts\n\
         }\n",
        "P",
        Reg::D0,
        &map(&[]),
    );
    assert!(is_unverified(&s), "d0 written .w only → high word stale → Unverified, got {s:?}");
}

/// The SAME body with a `.l` write verifies (the width discriminator).
#[test]
fn long_write_of_data_reg_verifies() {
    let s = status_uncond(
        "module m\n\
         proc P (d1: u16) clobbers() out(d0) {\n\
             move.l  d1, d0\n\
             rts\n\
         }\n",
        "P",
        Reg::D0,
        &map(&[]),
    );
    assert!(is_produced(&s), "d0 written .l → Produced, got {s:?}");
}

/// `moveq` writes all 32 bits — a full-width production despite the byte
/// immediate (Section_FlatIDXY's `moveq #0,d0` clears the high word).
#[test]
fn moveq_is_full_width_production() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers() out(d0) {\n\
             moveq   #0, d0\n\
             rts\n\
         }\n",
        "P",
        Reg::D0,
        &map(&[]),
    );
    assert!(is_produced(&s), "moveq writes full 32 bits → Produced, got {s:?}");
}

// === 3. no-param-seed (Finding 2) =========================================

/// A cursor `out(a4)` advanced by `(a4)+` on the main path but early-exiting
/// BEFORE the advance on the bail path ⇒ FIRES (a param is not a production).
#[test]
fn unadvanced_cursor_param_fires() {
    let s = status_uncond(
        "module m\n\
         proc P (a4: *u8) clobbers(d0) out(a4) {\n\
             tst.b   Flag\n\
             beq     .bail\n\
             move.b  (a4)+, d0\n\
             rts\n\
         .bail:\n\
             rts\n\
         }\n",
        "P",
        Reg::A4,
        &map(&[]),
    );
    assert!(is_unverified(&s), "a4 un-advanced on the bail path → Unverified, got {s:?}");
}

/// The version that advances a4 on EVERY path verifies — the `(a4)+` advance is
/// the production (an address-register write).
#[test]
fn advanced_cursor_on_all_paths_verifies() {
    let s = status_uncond(
        "module m\n\
         proc P (a4: *u8) clobbers(d0) out(a4) {\n\
             tst.b   Flag\n\
             beq     .other\n\
             move.b  (a4)+, d0\n\
             rts\n\
         .other:\n\
             move.b  (a4)+, d0\n\
             rts\n\
         }\n",
        "P",
        Reg::A4,
        &map(&[]),
    );
    assert!(is_produced(&s), "a4 advanced on every path → Produced, got {s:?}");
}

// === 4. Defer (Finding 3) =================================================

/// `out(a1)` produced by a tail `jbra ProducesA1` where ProducesA1 declares
/// UNCONDITIONAL `out(a1)` ⇒ verifies (a tail transfer is a required return
/// path; the target's uncond out produces a1).
#[test]
fn tail_to_known_producer_verifies() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             jbra    ProducesA1\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[("ProducesA1", &[Reg::A1])]),
    );
    assert!(is_produced(&s), "tail to a known out(a1) producer → Produced, got {s:?}");
}

/// The same tail to a proc that does NOT declare `out(a1)` ⇒ FIRES.
#[test]
fn tail_to_non_producer_fires() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             jbra    NoA1\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[("NoA1", &[Reg::D0])]),
    );
    assert!(is_unverified(&s), "tail to a non-producer → Unverified, got {s:?}");
}

/// A tail to an UNRESOLVED/external symbol ⇒ cannot verify ⇒ FIRES.
#[test]
fn tail_to_external_symbol_fires() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             jbra    SomeExternalThing\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_unverified(&s), "tail to an external symbol → Unverified, got {s:?}");
}

// === 5. conditional out(rN if cc) — the FindStagedBlock shape ===============

/// Verify a single CONDITIONAL `out(reg if cc)` on the named proc's body.
fn status_cond(
    src: &str,
    proc: &str,
    reg: Reg,
    cc: &str,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) -> OutStatus {
    let all = eval_all(src);
    let items = all.get(proc).unwrap_or_else(|| panic!("no proc {proc}"));
    let no_cond: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    verify_out(items, &[], &[(reg, cc.to_string())], callee_uncond_out, &no_cond)
        .remove(&reg)
        .expect("status for the checked reg")
}

/// Verify a single UNCONDITIONAL `out(reg)` where a CALLEE declares a conditional
/// `out(cond_reg if cc)` (item #2's edge credit) — the Load_Object←AllocDynamic
/// cascade shape. `callee_uncond_out` are the callees' unconditional outs.
fn status_uncond_cond_callee(
    src: &str,
    proc: &str,
    reg: Reg,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
    cond_callees: &[(&str, Reg, &str)],
) -> OutStatus {
    let all = eval_all(src);
    let items = all.get(proc).unwrap_or_else(|| panic!("no proc {proc}"));
    let mut cond: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (callee, r, cc) in cond_callees {
        cond.entry(callee.to_string()).or_default().push((r.to_string(), cc.to_string()));
    }
    verify_out(items, &[reg], &[], callee_uncond_out, &cond)
        .remove(&reg)
        .expect("status for the checked reg")
}

/// The FindStagedBlock shape via `moveq`-fold: a1 produced on the hit (eq) path;
/// the `.miss` path does `moveq #1,d3` (Z clear ⇒ ne) then `rts` — a1 unproduced
/// but the return is provably `!eq`, so `out(a1 if eq)` VERIFIES.
#[test]
fn conditional_out_verifies_via_moveq_fold() {
    let s = status_cond(
        "module m\n\
         proc P (d0: u16) clobbers(d3) out(a1 if eq) {\n\
             cmp.w   #0, d0\n\
             bne     .miss\n\
             lea     Slot, a1\n\
             moveq   #0, d3\n\
             rts\n\
         .miss:\n\
             moveq   #1, d3\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        "eq",
        &map(&[]),
    );
    assert!(is_produced(&s), "a1 produced on eq, .miss is !eq → Produced, got {s:?}");
}

/// The SAME body declared UNCONDITIONAL `out(a1)` FIRES — a1 is not produced on
/// the `.miss` return (the existence-proof regression, both directions).
#[test]
fn same_body_unconditional_out_fires() {
    let s = status_uncond(
        "module m\n\
         proc P (d0: u16) clobbers(d3) out(a1) {\n\
             cmp.w   #0, d0\n\
             bne     .miss\n\
             lea     Slot, a1\n\
             moveq   #0, d3\n\
             rts\n\
         .miss:\n\
             moveq   #1, d3\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_unverified(&s), "unconditional out(a1) unproduced on .miss → Unverified, got {s:?}");
}

// === 6. cascade — a proc's out sourced from a CONDITIONAL callee out (item #2) ==
// The Load_Object←AllocDynamic-relabeled shape: P declares `out(a1)`; on the
// success path a1 is produced ONLY by AllocDynamic's `out(a1 if eq)` credited on
// the `bne .fail` fall-through (the eq-success edge); the fail path produces a1
// itself. Verifies WITH #2's edge credit; the same body WITHOUT it is the
// pre-relabel regression control (a1 unproduced on the success return).

/// WITH the conditional callee-out edge credit ⇒ a1 produced on both returns ⇒
/// verifies. This is the cascade that keeps Load_Object honest after the relabel.
#[test]
fn cascade_conditional_callee_out_verifies() {
    let s = status_uncond_cond_callee(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             jbsr    AllocDynamic\n\
             bne     .fail\n\
             rts\n\
         .fail:\n\
             movea.w d0, a1\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]), // AllocDynamic has NO unconditional out (it is `out(a1 if eq)`)
        &[("AllocDynamic", Reg::A1, "eq")],
    );
    assert!(is_produced(&s), "a1 produced on eq-success (credit) + fail (movea) → Produced, got {s:?}");
}

/// WITHOUT the conditional credit (edge-blind only) ⇒ a1 unproduced on the
/// eq-success return ⇒ FIRES. Proves #2's edge credit is load-bearing for the
/// cascade — the exact regression a bare AllocDynamic relabel would cause.
#[test]
fn cascade_without_conditional_credit_fires() {
    let s = status_uncond_cond_callee(
        "module m\n\
         proc P () clobbers(d0) out(a1) {\n\
             jbsr    AllocDynamic\n\
             bne     .fail\n\
             rts\n\
         .fail:\n\
             movea.w d0, a1\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
        &[], // no conditional credit supplied
    );
    assert!(is_unverified(&s), "no eq-success credit → a1 unproduced on success → Unverified, got {s:?}");
}

// === 7. movem LOAD production (the item-#2 cascade growth) ===================
// A `movem (sp)+, …aN` restore genuinely PRODUCES its reglist at full width;
// `produced_regs` now credits it (the Load_Object alloc-fail path). A movem STORE
// (reglist as SOURCE) must NOT be credited — it reads those regs.

/// A `movem.l (sp)+, d0-d2/a1` LOAD produces a1 at full width ⇒ `out(a1)`
/// verifies with no other producer. Without the movem-load growth a1 is
/// unproduced ⇒ this is the load-bearing test for the growth.
#[test]
fn movem_load_produces_reglist() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0-d2) out(a1) {\n\
             movem.l (sp)+, d0-d2/a1\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_produced(&s), "movem-load restores a1 (full-width production) → Produced, got {s:?}");
}

/// A `movem.w (sp)+, a1` LOAD produces a1 at FULL WIDTH even at `.w` — movem.w
/// sign-extends each word to 32 bits (unlike a plain `move.w`, which the width
/// filter drops). So `out(a1)` verifies. Guards guardrail 2 (full-width for both
/// sizes).
#[test]
fn movem_load_word_size_still_full_width() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers() out(a1) {\n\
             movem.w (sp)+, a1\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_produced(&s), "movem.w load is full-width (sign-extended) → Produced, got {s:?}");
}

/// A movem STORE — `movem.l d0-d2/a1, -(sp)` — has the reglist as the SOURCE
/// (first operand; `ops.last()` is the `-(sp)` predec, not a `RegList`). It READS
/// a1, it does NOT produce it ⇒ `out(a1)` FIRES. MUTATION: crediting a movem
/// store's reglist (dropping the `ops.last() == RegList` load-only guard) makes
/// this verify — so this asserts the store is NOT a production.
#[test]
fn movem_store_does_not_produce_reglist() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers() out(a1) {\n\
             movem.l d0-d2/a1, -(sp)\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        &map(&[]),
    );
    assert!(is_unverified(&s), "movem STORE reads a1, does not produce it → Unverified, got {s:?}");
}

/// Branch-split (guardrail 4's reusable primitive): a conditional out whose
/// `!cc` return is reached DIRECTLY after the branch with NO intervening `moveq`
/// — the taken `bne` edge must classify `.miss` as `!eq`. Verifies only if the
/// branch-split refinement is applied.
#[test]
fn conditional_out_verifies_via_branch_split() {
    let s = status_cond(
        "module m\n\
         proc P (d0: u16) clobbers() out(a1 if eq) {\n\
             tst.w   d0\n\
             bne     .miss\n\
             lea     Slot, a1\n\
             rts\n\
         .miss:\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        "eq",
        &map(&[]),
    );
    assert!(is_produced(&s), "branch-split classifies .miss as !eq → Produced, got {s:?}");
}

// === 6. trap for the ⊤-keeps-obligation conservatism (guardrail 3) ==========

/// A conditional `out(a1 if eq)` whose body does NOT produce a1 on a return
/// where the cc is ⊤ (clobbered by a non-`moveq` op — `add.w` — before the
/// `rts`) MUST FIRE: an UNKNOWN cc keeps the obligation. This is the load-bearing
/// conservatism — a mutation treating ⊤ as provably-`!eq` makes it go green.
#[test]
fn conditional_out_unknown_cc_at_return_fires() {
    let s = status_cond(
        "module m\n\
         proc P (d0: u16) clobbers(d3) out(a1 if eq) {\n\
             cmp.w   #0, d0\n\
             bne     .miss\n\
             lea     Slot, a1\n\
             moveq   #0, d3\n\
             rts\n\
         .miss:\n\
             add.w   #1, d3\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        "eq",
        &map(&[]),
    );
    assert!(
        is_unverified(&s),
        "a1 unproduced on .miss where eq is ⊤ (add.w clobbered it) → Unverified, got {s:?}"
    );
}

/// Companion: when a1 IS produced on BOTH paths, the conditional out verifies
/// regardless of the cc state (no obligation is ever unmet).
#[test]
fn conditional_out_produced_everywhere_verifies() {
    let s = status_cond(
        "module m\n\
         proc P (d0: u16) clobbers(d3) out(a1 if eq) {\n\
             cmp.w   #0, d0\n\
             bne     .other\n\
             lea     A, a1\n\
             rts\n\
         .other:\n\
             lea     B, a1\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        "eq",
        &map(&[]),
    );
    assert!(is_produced(&s), "a1 produced on every path → Produced, got {s:?}");
}

/// Adversarial probe (proof Finding 1's moveq-fold is NOT a silent miss): eq is
/// provably TRUE at the rts (`moveq #0,d3` sets Z) but a1 is NOT produced on that
/// return ⇒ MUST fire. The moveq-fold only DISCHARGES the obligation on a
/// cc-provably-FALSE return (where the caller cannot read the register); a
/// cc-TRUE return still carries the obligation. Guards against a regression where
/// the fold wrongly discharges a cc-true obligation.
#[test]
fn conditional_out_cc_true_at_return_unproduced_fires() {
    let s = status_cond(
        "module m\n\
         proc P (d0: u16) clobbers(d3) out(a1 if eq) {\n\
             cmp.w   #0, d0\n\
             bne     .miss\n\
             moveq   #0, d3\n\
             rts\n\
         .miss:\n\
             moveq   #1, d3\n\
             rts\n\
         }\n",
        "P",
        Reg::A1,
        "eq",
        &map(&[]),
    );
    assert!(is_unverified(&s), "eq TRUE at return but a1 unproduced -> MUST fire, got {s:?}");
}
