//! `align N` — the D2.29 item-position alignment opener (§4.8): pads the
//! current position to the next multiple of `N` with `$00` fill, exact AS
//! parity by construction (the AS front-end's `align` emits zero-fill; AS
//! `even` ports as `align 2`). `N` must comptime-evaluate to a positive int;
//! alignment at a provisional position is the loud `[align.provisional]`
//! error in v1. The compiler still never inserts IMPLICIT alignment — `align`
//! is the author's explicit, byte-visible act.
//!
//! Soundness refinement (recorded in the tranche notes): padding is computed
//! against the LOWERING-baseline position, and every `align` also records a
//! link-time congruence assertion on a hidden anchor label — if final
//! placement (D2.25 chaining / map regions) moves the section to a base that
//! breaks the alignment, the build fails loudly instead of shipping a
//! misaligned item.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

// ---- 1. padding arithmetic + zero fill --------------------------------------

#[test]
fn align_pads_to_boundary_with_zero_fill() {
    let src = "\
module m
data D1: [u8; 3] = [1, 2, 3]
align 4
data D2: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 2, 3, 0, 9]);
}

#[test]
fn align_at_aligned_position_emits_nothing() {
    let src = "\
module m
data D1: [u8; 4] = [1, 2, 3, 4]
align 4
data D2: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 2, 3, 4, 9]);
}

#[test]
fn align_2_is_the_even_translation() {
    let src = "\
module m
data D1: [u8; 1] = [1]
align 2
data D2: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 0, 9]);
}

#[test]
fn align_n_is_a_comptime_expression() {
    let src = "\
module m
const K = 2
data D1: [u8; 1] = [1]
align K * 2
data D2: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 0, 0, 0, 9]);
}

#[test]
fn align_respects_a_pinned_section_base() {
    // vma: $101 — align 2 from an odd PINNED base pads exactly one byte.
    let src = "\
module m
section s (vma: $101) {
    data D1: [u8; 1] = [1]
    align 2
    data D2: [u8; 1] = [9]
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 9], "1 at $101 (odd), pad 0? no: $102 is even");
}

// ---- 2. errors ---------------------------------------------------------------

#[test]
fn align_non_positive_is_error() {
    let (_, msgs) = lower("module m\nalign 0\n");
    assert!(
        msgs.iter().any(|m| m.contains("positive")),
        "align 0 must error naming the positive requirement: {msgs:?}"
    );
}

#[test]
fn align_after_relaxable_branch_is_align_provisional() {
    // An unsized `bra` in a new-style proc is a size-relaxable fragment, so
    // the position after the proc is PROVISIONAL — v1 refuses (D2.29).
    let src = "\
module m
proc p () {
    bra .done
    nop
.done:
    rts
}
align 4
data D: [u8; 1] = [9]
";
    let (_, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("[align.provisional]")),
        "expected [align.provisional]: {msgs:?}"
    );
}

// ---- 3. contextual-keyword rule (§10, the equ precedent) ----------------------

#[test]
fn align_stays_usable_as_an_ordinary_name() {
    let src = "\
module m
const align = 2
data D: [u8; 2] = [align, align + 1]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "`align` as a const name/expr still works: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![2, 3]);
}

// ---- 4. the congruence link-assert ------------------------------------------

#[test]
fn align_records_a_congruence_link_assert() {
    let src = "\
module m
data D1: [u8; 1] = [1]
align 2
data D2: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert!(
        !m.link_asserts.is_empty(),
        "every align records a link-time congruence assertion (placement drift is loud)"
    );
    // And the normal link path passes it.
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let ds = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &m.link_asserts);
    assert!(ds.is_empty(), "aligned layout passes its own assert: {ds:?}");
}

#[test]
fn placement_drift_fails_the_congruence_assert() {
    // THE refinement's point: padding computed at the lowering baseline, but
    // the section later PLACED at an incongruent base (simulating a map
    // region / chained-growth move) — the build must fail loudly, naming the
    // final address.
    let src = "\
module m
data D1: [u8; 1] = [1]
align 2
data D2: [u8; 1] = [9]
";
    let (mut m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // Move the whole section to an ODD base — a fair stand-in for
    // place_sections routing it to an odd map region.
    for s in &mut m.sections {
        s.lma = 0x101;
    }
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let ds = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &m.link_asserts);
    let fail = ds
        .iter()
        .find(|d| d.message.contains("align 2") && d.message.contains("final placement broke"))
        .unwrap_or_else(|| panic!("expected the congruence assert to fire: {ds:?}"));
    assert!(matches!(fail.level, sigil_span::Level::Error), "drift fails the build");
}

#[test]
fn align_eq_at_item_position_is_a_finite_error() {
    // Review C1 regression: `align = 5` is NOT an item (the `=` guard skips
    // the arm) — recovery must CONSUME it, not spin forever. This test's
    // existence is the proof of termination; the assertions pin the shape.
    let (f, perrs) = sigil_frontend_emp::parse_str("module m\nalign = 5\nconst A = 1\n");
    assert!(!perrs.is_empty(), "the stray line still errors");
    assert!(
        perrs.len() < 10,
        "a FINITE diagnostic list (the spin produced unbounded errors): {}",
        perrs.len()
    );
    assert_eq!(f.items.len(), 1, "the following const still parses");
}

#[test]
fn align_eq_inside_a_section_body_is_finite_too() {
    let (_, perrs) =
        sigil_frontend_emp::parse_str("module m\nsection s (vma: $100) {\nalign = 5\n}\n");
    assert!(!perrs.is_empty() && perrs.len() < 10, "finite: {}", perrs.len());
}

#[test]
fn align_works_in_z80_sections() {
    let src = "\
module m
section z (cpu: z80, vma: $0000) {
    data D1: [u8; 1] = [1]
    align 4
    data D2: [u8; 1] = [9]
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 0, 0, 0, 9]);
}

#[test]
fn consecutive_aligns_compose() {
    let src = "\
module m
data D1: [u8; 1] = [1]
align 2
align 4
data D2: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 0, 0, 0, 9]);
    assert_eq!(m.link_asserts.len(), 2, "each align records its own congruence assert");
}

#[test]
fn align_as_first_item_is_fine() {
    let src = "\
module m
align 4
data D: [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![9], "position 0 is already aligned");
}

#[test]
fn pub_align_is_rejected() {
    let (_, perrs) = sigil_frontend_emp::parse_str("module m\npub align 2\n");
    assert!(
        perrs.iter().any(|d| d.message.contains("`pub` is not valid")),
        "pub align must be rejected: {perrs:?}"
    );
}

// ---- 6. per-DATA-item `(align: N)` (L3) --------------------------------------
// The relaxation-aware alignment form: pins THIS item's base independent of what
// precedes it, and — unlike a bare `align` — a relaxation-INVARIANT `N` (word
// alignment, since every m68k relaxation delta is even) survives size-relaxable
// code earlier in the section, so the eager pad stays exact.

#[test]
fn data_item_align_pads_before_the_item() {
    let src = "\
module m
data D1: [u8; 1] = [1]
data D2 (align: 2): [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // D1 at 0 (odd next base) → one pad byte → D2 word-aligned.
    assert_eq!(linked_bytes(&m), vec![1, 0, 9]);
}

#[test]
fn data_item_align_is_zero_when_already_aligned() {
    let src = "\
module m
data D1: [u8; 2] = [1, 2]
data D2 (align: 2): [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 2, 9], "even base ⇒ zero pad ⇒ byte-neutral");
}

#[test]
fn data_item_align_survives_a_word_relaxable_with_a_real_pad() {
    // The crux: a size-relaxable `bra` sits earlier in the section, so a bare
    // `align` would be `[align.provisional]`. Word alignment is
    // relaxation-invariant (every 68k delta is even), so `(align: 2)` is EXEMPT
    // and lays a genuine pad byte before an odd-based item.
    let src = "\
module m
proc p () {
    bra .done
    nop
.done:
    rts
}
data Odd: [u8; 1] = [7]
data D (align: 2): [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(
        !msgs.iter().any(|s| s.contains("[align.provisional]")),
        "word alignment past a relaxable must NOT refuse: {msgs:?}"
    );
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let bytes = linked_bytes(&m);
    // proc p is even-length at baseline (bra.s 2 + nop 2 + rts 2 = 6), Odd makes
    // the next base odd, so `(align: 2)` inserts exactly one $00 pad before D.
    assert!(bytes.ends_with(&[7, 0, 9]), "expected a real pad byte before D: {bytes:?}");
}

#[test]
fn data_item_align_coarser_than_granularity_still_refuses_past_a_relaxable() {
    // `(align: 4)` is NOT relaxation-invariant (a +2 relaxation delta can change
    // an address mod 4), so the provisional wall still stands — the general
    // relaxation-aware-`align` fixpoint is the ledgered option, not this one.
    let src = "\
module m
proc p () {
    bra .done
    nop
.done:
    rts
}
data D (align: 4): [u8; 1] = [9]
";
    let (_, msgs) = lower(src);
    assert!(
        msgs.iter().any(|s| s.contains("[align.provisional]")),
        "align 4 past a relaxable is still provisional: {msgs:?}"
    );
}

#[test]
fn data_item_align_without_a_relaxable_pads_like_a_bare_align() {
    let src = "\
module m
data D1: [u8; 1] = [1]
data D2 (align: 4): [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(linked_bytes(&m), vec![1, 0, 0, 0, 9]);
}

#[test]
fn data_item_align_records_a_structural_congruence_assert() {
    let src = "\
module m
data D1: [u8; 1] = [1]
data D2 (align: 2): [u8; 1] = [9]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // Tagged `[layout.align]` (structural) → excluded from the twin-guard count,
    // like a `table item_align:` pad, not a user drift guard.
    let structural = m.link_asserts.iter().any(|a| {
        a.message.iter().any(|p| {
            matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("[layout.align]"))
        })
    });
    assert!(structural, "per-item align assert must carry the [layout.align] tag");
    // The aligned layout passes its own assert.
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let ds = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &m.link_asserts);
    assert!(ds.is_empty(), "aligned layout passes: {ds:?}");
}

#[test]
fn data_item_align_congruence_fails_on_placement_drift() {
    let src = "\
module m
data D1: [u8; 1] = [1]
data D2 (align: 2): [u8; 1] = [9]
";
    let (mut m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    for s in &mut m.sections {
        s.lma = 0x101; // an odd base a map region could route to
    }
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let ds = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &m.link_asserts);
    assert!(
        ds.iter().any(|d| d.message.contains("final placement broke")
            && matches!(d.level, sigil_span::Level::Error)),
        "placement drift must fail the per-item align congruence assert: {ds:?}"
    );
}

// ---- 5. AS parity ------------------------------------------------------------
// The AS-vs-emp byte-parity vector lives in `crates/sigil-cli/tests/
// align_as_parity.rs` — the crate-graph contamination safeguard (crate_graph.rs
// invariant (c)) allows only sigil-cli/sigil-harness to depend on
// sigil-frontend-as, so the cross-frontend comparison runs there.
