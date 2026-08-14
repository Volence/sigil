//! Rung-4 t40: the `ensure(cycles(L1, L2) == N)` eager channel end-to-end.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

fn errs(body: &str) -> Vec<String> {
    let src = format!(
        "module m\nsection s (cpu: z80, vma: $0) {{\n  proc P() {{\n{body}\n    ret\n  }}\n}}\n"
    );
    let (file, perrs) = parse_str(&src);
    if perrs.iter().any(|d| d.level == Level::Error) {
        return perrs.iter().filter(|d| d.level == Level::Error).map(|d| format!("PARSE: {}", d.message)).collect();
    }
    let (_m, ds) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    ds.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

// Two nops = 8 T-states; the ensure passes → no diagnostics (the positive control).
#[test]
fn cycles_passes_on_correct_count() {
    let e = errs("    .a:\n    nop\n    nop\n    .b:\n    ensure(cycles(.a, .b) == 8, \"drift\")");
    assert!(e.is_empty(), "expected clean, got {e:?}");
}

// A doctored expectation (12 ≠ 8) fails the ensure with ITS message (pad-drift class).
#[test]
fn cycles_doctored_fires_the_ensure() {
    let e = errs("    .a:\n    nop\n    nop\n    .b:\n    ensure(cycles(.a, .b) == 12, \"drift\")");
    assert!(e.iter().any(|m| m.contains("drift")), "expected the ensure to fire, got {e:?}");
}

// A `jr cc` inside the span is the hard ambiguous-branch bail (jp-not-jr discipline).
#[test]
fn cycles_jr_cc_in_span_bails() {
    let e = errs("    .a:\n    jr z, .a\n    .b:\n    ensure(cycles(.a, .b) == 0, \"x\")");
    assert!(e.iter().any(|m| m.contains("[cycles.ambiguous-branch]")), "got {e:?}");
}

// An off-table op inside the span is the unknown-op bail.
#[test]
fn cycles_off_table_bails() {
    // A real Z80 op with no cost entry — see the note in `cycle_budget.rs`.
    let e = errs("    .a:\n    ldir\n    .b:\n    ensure(cycles(.a, .b) == 0, \"x\")");
    assert!(e.iter().any(|m| m.contains("[cycles.unknown-op]")), "got {e:?}");
}

// `jp cc` is the POSITIVE control for the ambiguous bail — fixed 10, passes.
#[test]
fn cycles_jp_cc_is_the_positive_control() {
    let e = errs("    .a:\n    jp z, .a\n    .b:\n    ensure(cycles(.a, .b) == 10, \"x\")");
    assert!(e.is_empty(), "expected clean (jp cc = 10), got {e:?}");
}

// --- pad_to_cycles (t40 step-2 modernization) ---

// The pad emits (target - measured)/4 nops: pad_to_cycles(20, 4) = 4 nops = 16
// T-states, cross-checked by the following cycles() span (the emit count IS the
// span cost). Clean → the derived count matches.
#[test]
fn pad_to_cycles_emits_the_derived_count() {
    let e = errs("    .a:\n    pad_to_cycles(20, 4)\n    .b:\n    ensure(cycles(.a, .b) == 16, \"pad\")");
    assert!(e.is_empty(), "expected clean (pad = 4 nops = 16 T), got {e:?}");
}

// The pad is DERIVED from a live cycles() measurement of the prefix: 2 nops (8 T)
// measured, pad to 20 → 3 nops (12 T), total span = 20. Proves the pad tracks the
// prefix automatically (a future prefix edit re-derives the pad).
#[test]
fn pad_to_cycles_derives_from_a_cycles_span() {
    let e = errs(
        "    .a:\n    nop\n    nop\n    .p:\n    pad_to_cycles(20, cycles(.a, .p))\n    .b:\n    ensure(cycles(.a, .b) == 20, \"pad\")",
    );
    assert!(e.is_empty(), "expected clean (8 measured + 12 pad = 20), got {e:?}");
}

// A target unreachable with nop granularity (pad = 6, not a multiple of 4) is a loud
// error — nop padding cannot hit it.
#[test]
fn pad_to_cycles_non_multiple_of_4_errors() {
    let e = errs("    .a:\n    pad_to_cycles(10, 4)\n    ret");
    assert!(e.iter().any(|m| m.contains("not a\n                     multiple of 4") || m.contains("multiple of 4")), "got {e:?}");
}

// A measured cost that already exceeds the target (no pad fits) is a loud error.
#[test]
fn pad_to_cycles_over_budget_errors() {
    let e = errs("    .a:\n    pad_to_cycles(4, 20)\n    ret");
    assert!(e.iter().any(|m| m.contains("exceeds the target")), "got {e:?}");
}

// --- pad_to_cycles DENSE mode (wave-4 Z80 reclaim) ---

// An unconditional `jr Label` is 12 T-states — the dense pad's unit, and the arm
// that was MISSING from the table (it used to fall through to `[cycles.unknown-op]`).
#[test]
fn cycles_unconditional_jr_is_twelve() {
    let e = errs("    .a:\n    jr .b\n    .b:\n    ensure(cycles(.a, .b) == 12, \"jr\")");
    assert!(e.is_empty(), "expected clean (jr = 12 T), got {e:?}");
}

// The doctored control for the arm above: 12 is the real cost, so 10 must fire.
#[test]
fn cycles_unconditional_jr_doctored_fires() {
    let e = errs("    .a:\n    jr .b\n    .b:\n    ensure(cycles(.a, .b) == 10, \"jrdrift\")");
    assert!(e.iter().any(|m| m.contains("jrdrift")), "expected the ensure to fire, got {e:?}");
}

// DENSE with an exact multiple of 12: 84 T = 7 `jr`, no nop remainder. The
// following cycles() span re-measures the emitted pad, so a wrong split fails here.
#[test]
fn pad_to_cycles_dense_exact_multiple_of_twelve() {
    let e = errs(
        "    .a:\n    pad_to_cycles(84, 0, dense: true)\n    .b:\n    ensure(cycles(.a, .b) == 84, \"pad\")",
    );
    assert!(e.is_empty(), "expected clean (7 jr = 84 T), got {e:?}");
}

// DENSE with a sub-12 remainder: 76 T = 6 `jr` (72) + 1 `nop` (4).
#[test]
fn pad_to_cycles_dense_with_nop_remainder() {
    let e = errs(
        "    .a:\n    pad_to_cycles(76, 0, dense: true)\n    .b:\n    ensure(cycles(.a, .b) == 76, \"pad\")",
    );
    assert!(e.is_empty(), "expected clean (6 jr + 1 nop = 76 T), got {e:?}");
}

// `dense: false` and the positional form are both accepted, and `false` is the
// unchanged nop-only shape (20 T = 5 nops, NOT 1 jr + 2 nops).
#[test]
fn pad_to_cycles_dense_false_is_the_sparse_shape() {
    let e = errs(
        "    .a:\n    pad_to_cycles(20, 0, dense: false)\n    .b:\n    ensure(cycles(.a, .b) == 20, \"pad\")",
    );
    assert!(e.is_empty(), "expected clean, got {e:?}");
    let e = errs(
        "    .a:\n    pad_to_cycles(20, 0, true)\n    .b:\n    ensure(cycles(.a, .b) == 20, \"pad\")",
    );
    assert!(e.is_empty(), "positional dense should work, got {e:?}");
}

// The multiple-of-4 validation still governs DENSE (12 is a multiple of 4, so the
// jr/nop split can only be exact when `rem` already is).
#[test]
fn pad_to_cycles_dense_still_requires_multiple_of_four() {
    let e = errs("    .a:\n    pad_to_cycles(10, 0, dense: true)\n    ret");
    assert!(e.iter().any(|m| m.contains("multiple of 4")), "got {e:?}");
}

// A misspelled keyword is LOUD — silently ignoring it would emit the sparse pad the
// caller did not ask for.
#[test]
fn pad_to_cycles_wrong_keyword_errors() {
    let e = errs("    .a:\n    pad_to_cycles(20, 0, sense: true)\n    ret");
    assert!(e.iter().any(|m| m.contains("the third argument is `dense`")), "got {e:?}");
}

// A non-bool `dense` is loud too.
#[test]
fn pad_to_cycles_non_bool_dense_errors() {
    let e = errs("    .a:\n    pad_to_cycles(20, 0, dense: 1)\n    ret");
    assert!(e.iter().any(|m| m.contains("must be a bool")), "got {e:?}");
}

// Two dense pads in ONE body mint DISTINCT hidden labels — a collision would be a
// duplicate-symbol link failure, not a silent miscount.
#[test]
fn pad_to_cycles_dense_labels_do_not_collide() {
    let e = errs(
        "    .a:\n    pad_to_cycles(24, 0, dense: true)\n    .b:\n    pad_to_cycles(24, 0, dense: true)\n    .c:\n    ensure(cycles(.a, .c) == 48, \"pad\")",
    );
    assert!(e.is_empty(), "expected clean, got {e:?}");
}
