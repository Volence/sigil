//! D-EQ.1 / D-EQ.2 — a comparison or an annotation that cannot be meaningfully
//! evaluated must REFUSE, never quietly produce a value.
//!
//! Three defects, one family, found live by the aeon item-5 comptime probe
//! (`docs/superpowers/probes/2026-09-02-item5-comptime-probe.md`, verdicts Q2-e,
//! Q2-D4 and Q1-L). The fixtures below are that probe's own shapes, transcribed
//! from `engine/effects/palette.emp` (`pal_variant`), `palette_dsl.emp`
//! (`variant()`), `raster_dsl.emp` (`first_mismatch`) and `ojz_effects.emp`
//! (the `pub data Variant_Water_Deep` twin), trimmed to the fields each row
//! actually reads:
//!
//! * **Q2-e — the always-RED guard.** A hand `pub data` symbol named inside an
//!   array literal resolves to a `Value::Label`, and label-vs-struct `!=` was
//!   always true, so `first_mismatch([Variant_Water_Deep], [variant(..)]) == -1`
//!   reported "index 0" for the EQUAL twin. A guard that cannot pass.
//! * **Q2-D4 — the same root, opposite sign.** `variant(..) == cycle_channel(..)`
//!   — two different struct types — was not refused; it evaluated FALSE, so a
//!   typo'd constructor read as a mismatch rather than as a type error.
//! * **Q1-L — the annotation that did not check.** A `[Label; 2]` parameter
//!   accepted a three-element argument and reported `.len == 3`; the length was
//!   caught only when a record built from it was emitted, blamed on the
//!   consumer's `pub data` line.
//!
//! THE TRAP THESE ROWS ARE BUILT AGAINST. The defect is a comparison stuck at
//! one constant, so a "fix" that merely stuck it at the OTHER constant — always
//! false instead of always true — would satisfy any single-twin test. Every row
//! here therefore runs BOTH twins IN ONE PROCESS: the equal twin must stay
//! green and a genuinely mismatched twin must go red, in the same test body. A
//! row that only exercises one polarity proves nothing about this family.
//!
//! AND WHY THEY ASSERT A REFUSAL RATHER THAN A BOOL. The two regression
//! directions are not equally dangerous. A refusal that decayed back to FALSE
//! reddens the builds that depend on it — the `parallax_dsl` fold guards fire,
//! and the failure is written down. A refusal that decayed to TRUE makes every
//! cross-type guard PASS, and a guard that passes leaves no artifact anywhere:
//! nothing downstream can report a defect whose whole signature is silence.
//! Asserting the PRESENCE of `[eq.cross-type]` (rather than an expected `false`)
//! is what makes these rows catch that direction — an always-true equality
//! leaves every `find`/`any` here empty, so the rows redden by construction
//! rather than by a bool that happened to be pinned the useful way round.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

/// Lower one `.emp` source and return its ERROR messages. Module-scope `ensure`s
/// elaborate here, which is what makes a guard's own text observable.
fn errors(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "expected a clean parse, got {perrs:?}");
    let (_module, diags): (_, Vec<Diagnostic>) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

/// The probe's fixture, minus the guards each row appends: the two struct types,
/// their constructors, `first_mismatch` verbatim from `raster_dsl.emp`, and the
/// hand `pub data` twin whose name is the thing that resolved to a label.
const PROBE: &str = "\
module m
struct pal_variant {
    v_shift_r: u8, v_bias_r: i8,
    v_shift_g: u8, v_bias_g: i8,
    v_shift_b: u8, v_bias_b: i8,
    v_lines:   u8,
    v_pad:     u8,
}
struct pal_cycle_channel {
    pc_line: u8, pc_first: u8, pc_count: u8, pc_period: u8,
}
pub comptime fn variant(shift_r: int = 0, shift_g: int = 0) -> pal_variant {
    return pal_variant{ v_shift_r: shift_r, v_bias_r: 0,
                        v_shift_g: shift_g, v_bias_g: 0,
                        v_shift_b: 0,       v_bias_b: 0,
                        v_lines: %1110,     v_pad: 0 }
}
pub comptime fn cycle_channel(line: int, first: int, count: int, period: int) -> pal_cycle_channel {
    return pal_cycle_channel{ pc_line: line, pc_first: first, pc_count: count, pc_period: period }
}
pub comptime fn first_mismatch(a: array, b: array) -> int {
    for i in 0..a.len {
        if i < b.len {
            if a[i] != b[i] { return i }
        }
    }
    return -1
}
pub data Variant_Water_Deep: pal_variant = variant(shift_r: 1, shift_g: 1)
";

fn probe_with(guards: &str) -> Vec<String> {
    errors(&format!("{PROBE}{guards}\n"))
}

// =============================================================================
// Q2-e — the always-red guard, and both twins of the guard that replaces it
// =============================================================================

#[test]
fn label_vs_struct_refuses_while_struct_vs_struct_still_answers_both_ways() {
    // Leg 1 — the EQUAL struct twin must stay GREEN. This is the leg that fails
    // if the fix merely moved the comparison from always-true to always-false.
    let equal = probe_with(
        "ensure(first_mismatch([variant(shift_r: 1, shift_g: 1)], [variant(shift_r: 1, shift_g: 1)]) == -1, \
         \"L1 equal struct twin reported a mismatch\")",
    );
    assert!(equal.is_empty(), "the equal struct twin must pass cleanly, got {equal:?}");

    // Leg 2 — a GENUINELY mismatched struct twin must go RED, in this same
    // process. Together with leg 1 this is the both-twins bar: the comparison
    // still discriminates, so it is a comparison and not a constant.
    let unequal = probe_with(
        "ensure(first_mismatch([variant(shift_r: 1, shift_g: 1)], [variant(shift_r: 1, shift_g: 0)]) == -1, \
         \"L2 unequal struct twin reported agreement\")",
    );
    assert!(
        unequal.iter().any(|m| m.contains("L2 unequal struct twin reported agreement")),
        "a one-field mutation must fire the guard, got {unequal:?}"
    );
    assert!(
        !unequal.iter().any(|m| m.contains("[eq.cross-type]")),
        "two `pal_variant` values are comparable — refusing them would fire on correct code: {unequal:?}"
    );

    // Leg 3 — the probe's own always-red shape, with the EQUAL twin on the right.
    // It used to report "index 0" here (a guard that could never pass); it must
    // now REFUSE, naming both sides.
    let hand_equal = probe_with(
        "ensure(first_mismatch([Variant_Water_Deep], [variant(shift_r: 1, shift_g: 1)]) == -1, \
         \"L3 hand twin via array literal\")",
    );
    let refusal = hand_equal
        .iter()
        .find(|m| m.contains("[eq.cross-type]"))
        .unwrap_or_else(|| panic!("label vs struct must refuse, got {hand_equal:?}"));
    assert!(
        refusal.contains("label `Variant_Water_Deep`") && refusal.contains("struct `pal_variant`"),
        "the refusal must name BOTH types, got {refusal:?}"
    );
    assert!(
        refusal.contains("always true"),
        "`!=` across classes was stuck at true — the diagnostic must say which constant: {refusal:?}"
    );

    // Leg 4 — the same shape with the UNEQUAL twin on the right refuses too. Legs
    // 3 and 4 are the probe's own discrimination: the answer never depended on
    // the struct's contents, so the refusal must not either. A fix that refused
    // only one of them would have re-created the defect with extra steps.
    let hand_unequal = probe_with(
        "ensure(first_mismatch([Variant_Water_Deep], [variant(shift_r: 1, shift_g: 0)]) == -1, \
         \"L4 hand twin via array literal\")",
    );
    assert!(
        hand_unequal.iter().any(|m| m.contains("[eq.cross-type]")),
        "label vs struct must refuse for the unequal twin as well, got {hand_unequal:?}"
    );
}

// =============================================================================
// Q2-D4 — the same root, opposite sign: two different struct types
// =============================================================================

#[test]
fn cross_struct_type_equality_refuses_while_same_type_answers_both_ways() {
    // Leg 1 — two different struct types. This was silently FALSE, so a typo'd
    // constructor on one side of an equality ensure read as a mismatch.
    let cross = probe_with(
        "ensure(variant(shift_r: 1, shift_g: 1) == cycle_channel(line: 2, first: 8, count: 4, period: 8), \
         \"D4 cross-type == said equal\")",
    );
    let refusal = cross
        .iter()
        .find(|m| m.contains("[eq.cross-type]"))
        .unwrap_or_else(|| panic!("cross-struct-type `==` must refuse, got {cross:?}"));
    assert!(
        refusal.contains("struct `pal_variant`") && refusal.contains("struct `pal_cycle_channel`"),
        "the refusal must name BOTH struct types, got {refusal:?}"
    );
    assert!(
        refusal.contains("always false"),
        "`==` across classes was stuck at false — the diagnostic must say so: {refusal:?}"
    );

    // Leg 2 — same-typed values still compare EQUAL cleanly...
    let same_equal = probe_with(
        "ensure(cycle_channel(line: 2, first: 8, count: 4, period: 8) == cycle_channel(line: 2, first: 8, count: 4, period: 8), \
         \"D4b same-type == said unequal\")",
    );
    assert!(same_equal.is_empty(), "same-typed equal structs must pass, got {same_equal:?}");

    // Leg 3 — ...and still compare UNEQUAL, in this same process. Without this
    // leg, an `==` that refused everything and a `==` stuck at false would look
    // identical from leg 2 alone.
    let same_unequal = probe_with(
        "ensure(cycle_channel(line: 2, first: 8, count: 4, period: 8) == cycle_channel(line: 2, first: 8, count: 4, period: 7), \
         \"D4c same-type == said equal\")",
    );
    assert!(
        same_unequal.iter().any(|m| m.contains("D4c same-type == said equal")),
        "a one-field mutation must fire the guard, got {same_unequal:?}"
    );
}

// =============================================================================
// The comparisons that must KEEP answering — a check that fires on correct code
// is worse than no check
// =============================================================================

#[test]
fn meaningful_inequality_never_refuses() {
    // `0` is how `.emp` spells an absent symbol in a pointer slot
    // (`variants: [Variant_Water_Deep, 0]`), so a label beside `0` is the
    // ordinary emptiness test and must ANSWER, both ways.
    let sentinel = probe_with(
        "ensure(first_mismatch([Variant_Water_Deep, 0], [Variant_Water_Deep, 0]) == -1, \
         \"S1 label/int sentinel pair disagreed with itself\")\n\
         ensure(first_mismatch([Variant_Water_Deep, 0], [Variant_Water_Deep, Variant_Water_Deep]) == 1, \
         \"S2 label vs the 0 that means empty did not report index 1\")",
    );
    assert!(sentinel.is_empty(), "the label/`0` slot idiom must not refuse: {sentinel:?}");

    // Arrays of different lengths, and enum-shaped differences, are meaningfully
    // unequal — the kinds have not decided the answer, the contents have.
    let lengths = probe_with(
        "ensure([1, 2] != [1, 2, 3], \"S3 arrays of different lengths compared equal\")\n\
         ensure([1, 2] == [1, 2], \"S4 identical arrays compared unequal\")\n\
         ensure(\"a\" != \"b\", \"S5 strings\")\n\
         ensure(1 == 1.0, \"S6 int/float promotion\")",
    );
    assert!(lengths.is_empty(), "ordinary comparisons must be untouched: {lengths:?}");
}

// =============================================================================
// The `unit` hint — the refusal keeps the diagnosis the guards used to carry
// =============================================================================

/// A fold-shaped fixture: `tail_if` is the pitfall catalogue's §1 shape — an `if`
/// in value position whose taken branch yields nothing — so `FOLDED` is `()` with
/// no diagnostic anywhere. `SOLID` is the same intent written as a flat
/// accumulator, which is the documented way out.
const FOLD: &str = "\
module m
comptime fn tail_if(n: int) -> int {
    return if n == 1 { 1 }
}
comptime fn solid(n: int) -> int {
    comptime var acc = 0
    if n == 1 { acc = 1 }
    return acc
}
const FOLDED = tail_if(n: 0)
const SOLID  = solid(n: 0)
";

fn fold_with(guards: &str) -> Vec<String> {
    errors(&format!("{FOLD}{guards}\n"))
}

#[test]
fn a_unit_operand_carries_the_fold_hint_in_both_orders() {
    // A guard like `T[0] == -16` compares int to int on every healthy tree and
    // reaches the refusal ONLY once a fold has happened — which is exactly when
    // its author's hand-written "a `()` here means the unit fold is back" advice
    // was meant to be read. The refusal has to carry that advice itself, or the
    // person meeting it never sees guidance written for that precise moment.
    //
    // BOTH ORDERS, because an asymmetry here is the same shape as the signature
    // defect this file already fixes: a check wired on one side only.
    for guard in [
        "ensure(FOLDED == 0, \"U1 folded twin\")",
        "ensure(0 == FOLDED, \"U2 folded twin, operands swapped\")",
    ] {
        let diags = fold_with(guard);
        let d = diags
            .iter()
            .find(|m| m.contains("[eq.cross-type]"))
            .unwrap_or_else(|| panic!("a folded `()` beside an int must refuse: {diags:?}"));
        assert!(
            d.contains("unit"),
            "the refusal must name the unit operand, got {d:?}"
        );
        assert!(
            d.contains("folds to `()` silently") && d.contains("§1"),
            "the refusal must carry the fold hint, got {d:?}"
        );
        // It is a lead, not a verdict — the compiler has established the operand's
        // KIND, never its provenance, and the wording must not overclaim.
        assert!(
            d.contains("LIKELY cause") && d.contains("other sources"),
            "the hint must read as a lead, not a diagnosis, got {d:?}"
        );
    }

    // The other twin, in this same process: the flat-accumulator spelling yields
    // a real int, so the identical guard compares in-class and passes silently.
    // Without this leg, a hint that attached to every comparison would look the
    // same from the rows above.
    let solid = fold_with("ensure(SOLID == 0, \"U3 accumulator twin rejected\")");
    assert!(solid.is_empty(), "the non-folded twin must pass cleanly, got {solid:?}");
}

#[test]
fn the_fold_hint_does_not_attach_to_other_cross_type_refusals() {
    // The hint is keyed to a `unit` operand. Every other refusal — the two this
    // file's earlier rows exercise — must stay free of it, or the guidance
    // becomes noise attached to comparisons it cannot explain.
    let struct_vs_struct = probe_with(
        "ensure(variant(shift_r: 1, shift_g: 1) == cycle_channel(line: 2, first: 8, count: 4, period: 8), \
         \"N1\")",
    );
    let label_vs_struct = probe_with(
        "ensure(first_mismatch([Variant_Water_Deep], [variant(shift_r: 1, shift_g: 1)]) == -1, \"N2\")",
    );
    for diags in [struct_vs_struct, label_vs_struct] {
        let d = diags
            .iter()
            .find(|m| m.contains("[eq.cross-type]"))
            .unwrap_or_else(|| panic!("expected a refusal to inspect, got {diags:?}"));
        assert!(
            !d.contains("§1") && !d.contains("LIKELY cause"),
            "a refusal with no `unit` operand must not carry the fold hint, got {d:?}"
        );
    }
}

// =============================================================================
// Q1-L — a `[T; N]` signature annotation is a length contract
// =============================================================================

/// The `[Label; 2]` chooser shape — the probe's `probe_variants_pair` — with no
/// consumer at all, so a diagnostic can only be blamed on the signature that
/// declared the length. The pair arrives as an ARGUMENT because that is the
/// position a bareword resolves to a label in (D-PP.3); `hand: array` is left
/// unconstrained so the return row measures only the return annotation.
const SIG: &str = "\
module m
pub comptime fn takes_pair(v: [Label; 2]) -> int { return v.len }
pub comptime fn makes_pair(hand: array) -> [Label; 2] {
    comptime var out = hand
    return out
}
proc target_a () { rts }
";

#[test]
fn array_length_annotation_is_checked_at_the_signature_both_ways() {
    // Leg 1 — the RIGHT length passes both the parameter and the return, cleanly.
    // Without this leg a check that refused every `[T; N]` signature would pass
    // legs 2 and 3.
    let ok = errors(&format!(
        "{SIG}ensure(takes_pair(v: [target_a, 0]) == 2, \"P1 correct pair rejected\")\n\
         ensure(makes_pair(hand: [target_a, 0]).len == 2, \"P2 correct return rejected\")\n"
    ));
    assert!(ok.is_empty(), "a correctly-sized pair must pass silently, got {ok:?}");

    // Leg 2 — a three-element argument to a `[Label; 2]` PARAMETER is refused at
    // the call, naming the parameter and the fn. It used to bind happily and
    // report `.len == 3`.
    let long_arg = errors(&format!(
        "{SIG}ensure(takes_pair(v: [target_a, 0, 0]) == 3, \"P3 unreachable\")\n"
    ));
    let d = long_arg
        .iter()
        .find(|m| m.contains("array length mismatch"))
        .unwrap_or_else(|| panic!("a 3-element arg to `[Label; 2]` must be refused, got {long_arg:?}"));
    assert!(
        d.contains("expected 2 element(s), got 3") && d.contains("parameter `v` of `takes_pair`"),
        "the parameter refusal must name the fn and the slot, got {d:?}"
    );

    // Leg 3 — the RETURN annotation checks at the returning fn. `ComptimeFnDecl`'s
    // `ret` was previously read by nothing in the crate.
    let long_ret =
        errors(&format!("{SIG}ensure(makes_pair(hand: [target_a, 0, 0]).len == 3, \"P4 unreachable\")\n"));
    let d = long_ret
        .iter()
        .find(|m| m.contains("array length mismatch"))
        .unwrap_or_else(|| panic!("a 3-element return from `-> [Label; 2]` must be refused, got {long_ret:?}"));
    assert!(
        d.contains("expected 2 element(s), got 3") && d.contains("the return type of `makes_pair`"),
        "the return refusal must name the fn, got {d:?}"
    );
}
