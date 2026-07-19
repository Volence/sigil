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
    verify_out(items, &[reg], &[], &BTreeSet::new(), callee_uncond_out)
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

// === 3. no-param-seed for a PURE out (Finding 2) ==========================
// A register declared `out` but NOT a param gets NO seed — production must come
// from a write on the path. (The param∩out in-out seed is §Bucket-3 below.)

/// Verify a single UNCONDITIONAL `out(reg)` with the proc's own params passed
/// (needed for the in-out seed). `params` are register spellings.
fn status_uncond_params(
    src: &str,
    proc: &str,
    reg: Reg,
    params: &[Reg],
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) -> OutStatus {
    let all = eval_all(src);
    let items = all.get(proc).unwrap_or_else(|| panic!("no proc {proc}"));
    let pset: BTreeSet<String> = params.iter().map(|r| r.to_string()).collect();
    verify_out(items, &[reg], &[], &pset, callee_uncond_out)
        .remove(&reg)
        .expect("status for the checked reg")
}

/// A PURE cursor `out(a4)` (a4 is NOT a param) produced by `lea` on the main
/// path but early-exiting BEFORE the write on the bail path ⇒ FIRES (no seed;
/// production must come from a write). Finding 2, pure-out form.
#[test]
fn unadvanced_pure_out_cursor_fires() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a4) {\n\
             tst.b   Flag\n\
             beq     .bail\n\
             lea     Table, a4\n\
             rts\n\
         .bail:\n\
             rts\n\
         }\n",
        "P",
        Reg::A4,
        &map(&[]),
    );
    assert!(is_unverified(&s), "pure out(a4) un-produced on the bail path → Unverified, got {s:?}");
}

/// The version that produces a4 on EVERY path verifies — the `lea` write is the
/// production (an address-register write), no seed involved.
#[test]
fn produced_pure_out_cursor_on_all_paths_verifies() {
    let s = status_uncond(
        "module m\n\
         proc P () clobbers(d0) out(a4) {\n\
             tst.b   Flag\n\
             beq     .other\n\
             lea     A, a4\n\
             rts\n\
         .other:\n\
             lea     B, a4\n\
             rts\n\
         }\n",
        "P",
        Reg::A4,
        &map(&[]),
    );
    assert!(is_produced(&s), "a4 lea'd on every path → Produced, got {s:?}");
}

// === Bucket 3: in-out accumulator seed (param∩out, read on some path) =======
// A register declared BOTH a param and an out is an IN-OUT accumulator: its
// INPUT is a valid output (unchanged-on-empty is a valid zero-append; a partial
// advance like `(a4)+`/`addq.b` preserves the already-full input value). It is
// seeded produced-at-entry — but ONLY when it is a genuine input (READ on some
// path). A param∩out register never read is suspect (a fake param) and STILL
// fires.

/// The InsertSpriteMasks shape: `out(a4, d5)` where BOTH are params, READ +
/// advanced on the loop, and returned UNCHANGED on the empty (`.done`) path ⇒
/// VERIFIES (the in-out seed credits the input as a valid output).
#[test]
fn inout_param_seed_verifies() {
    let src = "module m\n\
        proc P (a4: *u8, d5: u16) clobbers(d0) out(a4, d5) {\n\
            cmpi.w  #100, d5\n\
            bge     .done\n\
            move.w  d0, (a4)+\n\
            addq.b  #1, d5\n\
        .done:\n\
            rts\n\
        }\n";
    let a4 = status_uncond_params(src, "P", Reg::A4, &[Reg::A4, Reg::D5], &map(&[]));
    let d5 = status_uncond_params(src, "P", Reg::D5, &[Reg::A4, Reg::D5], &map(&[]));
    assert!(is_produced(&a4), "in-out a4 (read+advanced, unchanged on empty) → Produced, got {a4:?}");
    assert!(is_produced(&d5), "in-out d5 (read+advanced, unchanged on empty) → Produced, got {d5:?}");
}

/// Trap (guardrail): a param∩out register that is NEVER READ is a fake param —
/// it is NOT seeded and STILL FIRES. A mutation seeding every param∩out
/// unconditionally makes this go green (the read-guard is load-bearing).
#[test]
fn param_out_never_read_still_fires() {
    let s = status_uncond_params(
        "module m\n\
         proc P (a4: *u8) clobbers(d0) out(a4) {\n\
             moveq   #0, d0\n\
             rts\n\
         }\n",
        "P",
        Reg::A4,
        &[Reg::A4],
        &map(&[]),
    );
    assert!(is_unverified(&s), "param∩out a4 never read → not seeded → Unverified, got {s:?}");
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
    verify_out(items, &[], &[(reg, cc.to_string())], &BTreeSet::new(), callee_uncond_out)
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
