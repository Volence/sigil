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
    let e = errs("    .a:\n    rlca\n    .b:\n    ensure(cycles(.a, .b) == 0, \"x\")");
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
