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
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
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

// ---- 5. AS parity ------------------------------------------------------------
// The AS-vs-emp byte-parity vector lives in `crates/sigil-cli/tests/
// align_as_parity.rs` — the crate-graph contamination safeguard (crate_graph.rs
// invariant (c)) allows only sigil-cli/sigil-harness to depend on
// sigil-frontend-as, so the cross-frontend comparison runs there.
