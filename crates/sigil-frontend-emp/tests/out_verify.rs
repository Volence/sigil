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
    verify_out(items, &[reg], &[], callee_uncond_out)
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
