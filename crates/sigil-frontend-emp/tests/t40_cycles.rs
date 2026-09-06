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
    // `rlc (ix+0)` — the DDCB shift column, which this assembler cannot encode
    // at all, so it cannot become priced later. See the note in `cycle_budget.rs`
    // for why the previous two fixtures (`rlca`, then `ldir`) both went stale.
    let e = errs("    .a:\n    rlc (ix+0)\n    .b:\n    ensure(cycles(.a, .b) == 0, \"x\")");
    assert!(e.iter().any(|m| m.contains("[cycles.unknown-op]")), "got {e:?}");
    // The twin: `ldir` is now PRICED, and refuses under the other id because its
    // two costs are outcome-keyed on a repeat that is not an edge.
    let r = errs("    .a:\n    ldir\n    .b:\n    ensure(cycles(.a, .b) == 0, \"x\")");
    assert!(r.iter().any(|m| m.contains("[cycles.ambiguous-branch]")), "got {r:?}");
    // And the single-step `ldi` MEASURES at a flat 16: no cycle bail, and the
    // ensure does not fire. Only `[cycles.*]` and the ensure's own message are
    // examined, because `ldi` has no `.emp` mnemonic spelling yet — the language
    // surface is the owner's to extend — so lowering says so, and that
    // diagnostic is not what this test is about.
    let s = errs("    .a:\n    ldi\n    .b:\n    ensure(cycles(.a, .b) == 16, \"LDI-DRIFT\")");
    assert!(
        !s.iter().any(|m| m.contains("[cycles.") || m.contains("LDI-DRIFT")),
        "expected `ldi` to sum at 16 with no cycle finding, got {s:?}"
    );
    // …and the same span at a WRONG expectation does fire, so the assertion
    // above is not passing because `cycles()` never produced a number.
    let w = errs("    .a:\n    ldi\n    .b:\n    ensure(cycles(.a, .b) == 15, \"LDI-DRIFT\")");
    assert!(w.iter().any(|m| m.contains("LDI-DRIFT")), "got {w:?}");
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

// --- CYCLES-CPU-GUARD: the timing builtins are Z80-only, and say so ---

/// The same driver as [`errs`] with NO `(cpu: z80)` on the section, so the proc is
/// 68000 code (`LowerOptions::initial_cpu`). Every body here is spelled in 68000
/// mnemonics — `rts`, not `ret`.
fn errs_m68k(body: &str) -> Vec<String> {
    let src = format!(
        "module m\nsection s (vma: $0) {{\n  proc P() {{\n{body}\n    rts\n  }}\n}}\n"
    );
    let (file, perrs) = parse_str(&src);
    if perrs.iter().any(|d| d.level == Level::Error) {
        return perrs
            .iter()
            .filter(|d| d.level == Level::Error)
            .map(|d| format!("PARSE: {}", d.message))
            .collect();
    }
    let (_m, ds) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    ds.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

// THE defect: `nop` is spelled the same on both chips and the Z80 table prices it
// at 4 T, so a 68000 nop pad used to SUCCEED and hand back 8 — a Z80 T-state count
// at 3.58 MHz presented as this proc's cost at 7.67 MHz. The guard must refuse.
#[test]
fn cycles_on_a_68000_proc_refuses() {
    let e = errs_m68k("    .a:\n    nop\n    nop\n    .b:\n    ensure(cycles(.a, .b) == 8, \"drift\")");
    assert!(
        e.iter().any(|m| m.contains("[cycles.wrong-cpu]")),
        "a 68000 `cycles()` must refuse, got {e:?}"
    );
    // And it must NOT be the old misdirection, which sent the author off to grow a
    // table that was never the problem.
    assert!(
        !e.iter().any(|m| m.contains("[cycles.unknown-op]")),
        "the refusal must name the CPU, not blame the table, got {e:?}"
    );
}

// The OTHER half of the defect, and the reason a generic bail was not good enough:
// a 68000 op the Z80 table has never heard of used to come back as
// `[cycles.unknown-op] … add it to z80_cycles`, sending the author to grow a table
// for a chip the code is not running on. The refusal must name the CPU instead.
#[test]
fn cycles_on_a_68000_proc_does_not_blame_the_z80_table() {
    let e = errs_m68k(
        "    .a:\n    moveq #0, d0\n    .b:\n    ensure(cycles(.a, .b) == 4, \"x\")",
    );
    assert!(
        e.iter().any(|m| m.contains("[cycles.wrong-cpu]")),
        "expected the CPU refusal, got {e:?}"
    );
    // The property is not "never says z80_cycles" — the refusal names that table on
    // purpose, as the one it reads. It is "never sends the author to GROW it": the
    // unknown-op advice is correct on a Z80 proc and misdirection on this one.
    assert!(
        !e.iter().any(|m| m.contains("[cycles.unknown-op]")),
        "the table bail must not be what a 68000 author is told, got {e:?}"
    );
    assert!(
        !e.iter().any(|m| m.contains("add it to `z80_cycles`")),
        "the diagnostic must not send the author to grow the Z80 table, got {e:?}"
    );
}

// `pad_to_cycles(dense: true)` EMITS Z80 `jr` — a mnemonic the 68000 does not have.
// The guard has to stop it before it splices a foreign instruction into the stream.
#[test]
fn pad_to_cycles_on_a_68000_proc_refuses() {
    let e = errs_m68k("    .a:\n    pad_to_cycles(84, 0, dense: true)");
    assert!(
        e.iter().any(|m| m.contains("[cycles.wrong-cpu]")),
        "a 68000 `pad_to_cycles` must refuse, got {e:?}"
    );
}

// The sparse shape emits `nop`s, which ARE valid 68000 — but at 4 68000 cycles, not
// 4 T-states, so the count is still derived in the wrong unit. Same refusal.
#[test]
fn pad_to_cycles_sparse_on_a_68000_proc_refuses_too() {
    let e = errs_m68k("    .a:\n    pad_to_cycles(20, 0)");
    assert!(
        e.iter().any(|m| m.contains("[cycles.wrong-cpu]")),
        "the sparse 68000 pad must refuse as well, got {e:?}"
    );
}

// The refusal must be ACTIONABLE, not merely loud: it names the builtin's chip, the
// proc's chip, and the whole-proc bound that DOES dispatch on CPU.
#[test]
fn the_wrong_cpu_refusal_names_the_actual_problem() {
    let e = errs_m68k("    .a:\n    nop\n    .b:\n    ensure(cycles(.a, .b) == 4, \"x\")");
    let m = e
        .iter()
        .find(|m| m.contains("[cycles.wrong-cpu]"))
        .unwrap_or_else(|| panic!("no wrong-cpu diagnostic in {e:?}"));
    for needle in ["Z80", "68000", "@budget(cycles:"] {
        assert!(m.contains(needle), "the refusal must mention {needle:?}; got {m:?}");
    }
}

// POSITIVE CONTROL, and the thing that would catch a guard that refuses everything:
// the identical span in a `(cpu: z80)` section still measures and still passes.
#[test]
fn the_guard_leaves_z80_spans_working() {
    let e = errs("    .a:\n    nop\n    nop\n    .b:\n    ensure(cycles(.a, .b) == 8, \"drift\")");
    assert!(e.is_empty(), "the Z80 path must be untouched, got {e:?}");
}
