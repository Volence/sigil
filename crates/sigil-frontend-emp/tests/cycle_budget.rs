//! §4-cycles / S2-D7(c) — cycle budgets: `@budget(cycles: N)` and
//! `@cycles_exact`.
//!
//! A budget is a DECLARED contract, so nothing here fires on a proc that did not
//! ask. What every test below is really about is the other half: which shapes the
//! walk REFUSES to put a number on. The findings are ERROR-tier and unsuppressible,
//! so an unsound bound would break correct code AND vouch for a claim the machine
//! cannot keep — the refusals are the feature, and each gets a probe paired with a
//! passing twin so a green cannot mean the checker is dead.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` for `cpu`, asserting it parsed cleanly.
fn lower_cpu(src: &str, cpu: Cpu) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: cpu, include_root: None, embed_base: None, defines: vec![] },
    );
    diags
}

/// Every `[cycles.*]` / `[budget.*]` diagnostic, whatever the id or level.
fn cycle_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.message.contains("[cycles.") || d.message.contains("[budget."))
        .collect()
}

/// Every diagnostic carrying lint id `id`.
fn with_id<'a>(diags: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
    let tag = format!("[{id}]");
    diags.iter().filter(|d| d.message.contains(&tag)).collect()
}

/// A Z80 module with one proc carrying `attrs`.
fn z80(attrs: &str, body: &str) -> Vec<Diagnostic> {
    lower_cpu(&format!("module m\n{attrs}\nproc f() {{\n{body}}}\n"), Cpu::Z80)
}

/// Assert `src` lowers with no cycle finding at all.
fn assert_silent(diags: &[Diagnostic]) {
    let d = cycle_diags(diags);
    assert!(d.is_empty(), "expected no cycle finding, got: {d:?}");
}

/// Assert exactly one finding, with id `id`, at error tier.
fn assert_one<'a>(diags: &'a [Diagnostic], id: &str) -> &'a Diagnostic {
    let all = cycle_diags(diags);
    assert_eq!(all.len(), 1, "expected exactly one cycle finding, got: {all:?}");
    let firings = with_id(diags, id);
    assert_eq!(firings.len(), 1, "expected `[{id}]`, got: {all:?}");
    assert_eq!(firings[0].level, Level::Error, "cycle findings are error tier (U-spec §6)");
    firings[0]
}

// A body whose two arms cost 24 and 20 T-states: `jp cc` is 10 either outcome,
// the fall-through adds a 4-T `nop`, and `ret` is 10.
const UNEVEN: &str = "    jp z, .skip\n    nop\n.skip:\n    ret\n";

// ===========================================================================
// The budget itself
// ===========================================================================

/// The ceiling is compared against the WORST path, not the best or the average.
#[test]
fn the_budget_bounds_the_worst_path() {
    assert_silent(&z80("@budget(cycles: 24)", UNEVEN));
    let d = z80("@budget(cycles: 23)", UNEVEN);
    let f = assert_one(&d, "cycles.over-budget");
    assert!(f.message.contains("24"), "the finding names the measured cost: {}", f.message);
    assert!(f.message.contains("23"), "and the declared budget: {}", f.message);
}

/// A budget met exactly is met.
#[test]
fn a_budget_met_exactly_is_silent() {
    // ld a, n (7) + ret (10) = 17.
    assert_silent(&z80("@budget(cycles: 17)", "    ld a, 1\n    ret\n"));
    assert_one(&z80("@budget(cycles: 16)", "    ld a, 1\n    ret\n"), "cycles.over-budget");
}

/// A proc that declares nothing is not walked — an unboundable body stays silent
/// until someone claims a budget for it.
#[test]
fn an_undeclared_proc_is_never_walked() {
    assert_silent(&z80("", "    call Helper\n    ret\n"));
    assert_silent(&z80("", ".loop:\n    nop\n    jp .loop\n"));
}

// ===========================================================================
// `@cycles_exact`
// ===========================================================================

/// Unequal paths fire, and the finding names both extremes.
#[test]
fn cycles_exact_fires_on_unequal_paths() {
    let d = z80("@cycles_exact", UNEVEN);
    let f = assert_one(&d, "cycles.path-mismatch");
    assert!(f.message.contains("20"), "names the cheap path: {}", f.message);
    assert!(f.message.contains("24"), "names the dear path: {}", f.message);
}

/// Padding the short arm to match is what the proof is FOR: the same body, with
/// the branch arm padded, proves.
#[test]
fn cycles_exact_proves_a_padded_pair() {
    // taken: jp cc 10 + nop 4 + ret 10 = 24 ; not-taken: 10 + nop 4 + ret 10 = 24.
    let padded = "    jp z, .skip\n\
                  \x20   nop\n\
                  \x20   jp .join\n\
                  .skip:\n\
                  \x20   nop\n\
                  .join:\n\
                  \x20   ret\n";
    // taken 10 + 4 + 10 = 24 ; not-taken 10 + 4 + 10 + 10 = 34.
    let d = z80("@cycles_exact", padded);
    let f = assert_one(&d, "cycles.path-mismatch");
    assert!(f.message.contains("24") && f.message.contains("34"), "{}", f.message);
    // Spelling the join as `jr` (12 T rather than `jp`'s 10) and filling the short
    // arm to four `nop`s balances both at 36 — the same substitution
    // `pad_to_cycles`' dense mode makes, here machine-checked instead of counted.
    let balanced = "    jp z, .skip\n\
                    \x20   nop\n\
                    \x20   jr .join\n\
                    .skip:\n\
                    \x20   nop\n\
                    \x20   nop\n\
                    \x20   nop\n\
                    \x20   nop\n\
                    .join:\n\
                    \x20   ret\n";
    assert_silent(&z80("@cycles_exact", balanced));
}

/// A single-path body is trivially exact.
#[test]
fn a_straight_line_is_trivially_exact() {
    assert_silent(&z80("@cycles_exact", "    nop\n    nop\n    ret\n"));
}

/// Both attributes on one proc report independently off the same walk.
#[test]
fn both_attributes_report_together() {
    let d = z80("@budget(cycles: 20)\n@cycles_exact", UNEVEN);
    assert_eq!(with_id(&d, "cycles.over-budget").len(), 1, "{d:?}");
    assert_eq!(with_id(&d, "cycles.path-mismatch").len(), 1, "{d:?}");
}

// ===========================================================================
// The refusals — each paired with the twin that proves the checker is awake
// ===========================================================================

/// A loop has no longest path, and the refusal says so rather than measuring one
/// iteration and calling it a bound.
#[test]
fn a_loop_is_refused() {
    assert_silent(&z80("@budget(cycles: 100)", "    nop\n    ret\n"));
    assert_one(
        &z80("@budget(cycles: 100)", ".loop:\n    nop\n    jp .loop\n"),
        "cycles.unbounded-loop",
    );
}

/// A `djnz` counting loop is the same refusal — its trip count is a runtime fact.
#[test]
fn a_djnz_loop_is_refused() {
    assert_one(
        &z80("@budget(cycles: 100)", ".loop:\n    nop\n    djnz .loop\n    ret\n"),
        "cycles.unbounded-loop",
    );
}

/// A `djnz` whose target is NOT a local label presents only one edge, so its
/// 13/8 split cannot be routed — the walk refuses rather than charging one of the
/// two numbers to the edge it does have.
#[test]
fn an_unroutable_split_is_refused() {
    assert_one(
        &z80("@budget(cycles: 100)", "    djnz Elsewhere\n    ret\n"),
        "cycles.ambiguous-branch",
    );
}

/// A call costs whatever its callee costs, which is not a fact about this proc.
#[test]
fn a_call_is_refused() {
    assert_one(&z80("@budget(cycles: 100)", "    call Helper\n    ret\n"), "cycles.opaque-call");
}

/// A tail transfer out leaves the accounted region.
#[test]
fn a_tail_transfer_out_is_refused() {
    assert_one(
        &z80("@budget(cycles: 100)", "    nop\n    jp Elsewhere\n"),
        "cycles.unbounded-transfer",
    );
}

/// Control running off the end of the body is the same hole as a tail transfer:
/// it continues into whatever the section places next.
#[test]
fn a_fall_off_the_end_is_refused() {
    assert_one(&z80("@budget(cycles: 100)", "    nop\n    nop\n"), "cycles.unbounded-transfer");
}

/// An op outside the T-state table has no assignable cost, and adding a default
/// would be the one thing worse than refusing.
#[test]
fn an_off_table_op_is_refused() {
    assert_silent(&z80("@budget(cycles: 100)", "    nop\n    ret\n"));
    assert_one(&z80("@budget(cycles: 100)", "    rlca\n    ret\n"), "cycles.unknown-op");
}

/// There is no 68000 timing model, and a 68k proc carrying a budget is told that
/// rather than measured against the Z80 table.
#[test]
fn a_68k_body_is_refused_by_name() {
    let d = lower_cpu(
        "module m\n@budget(cycles: 100)\nproc f() {\n    nop\n    rts\n}\n",
        Cpu::M68000,
    );
    let f = assert_one(&d, "cycles.unmodeled-cpu");
    assert!(f.message.contains("68000"), "the refusal names the CPU: {}", f.message);
}

/// A refusal REPLACES the conclusions: an unmeasurable proc never also reports a
/// number, because there is no number it could honestly report.
#[test]
fn a_refusal_replaces_the_conclusions() {
    let d = z80("@budget(cycles: 1)\n@cycles_exact", "    call Helper\n    ret\n");
    let all = cycle_diags(&d);
    assert_eq!(all.len(), 1, "one refusal, no budget verdict: {all:?}");
    assert_eq!(with_id(&d, "cycles.opaque-call").len(), 1, "{d:?}");
}

// ===========================================================================
// Tier: no softening, no suppression
// ===========================================================================

/// `@as_compat` does NOT soften a budget. Unlike `[stack.*]` — which reads raw
/// ported assembly nobody annotated — a budget only exists because its author
/// wrote it, so there is no faithful-port case to protect.
#[test]
fn as_compat_does_not_soften_a_budget() {
    let d = lower_cpu(
        &format!("module m\n@as_compat\n@budget(cycles: 23)\nproc f() {{\n{UNEVEN}}}\n"),
        Cpu::Z80,
    );
    let f = with_id(&d, "cycles.over-budget");
    assert_eq!(f.len(), 1, "{d:?}");
    assert_eq!(f[0].level, Level::Error, "a declared budget never softens");
}

/// `@allow` does not suppress one either — the honest escape is to delete the
/// attribute, which is visible in the source; `@allow` would leave a claim
/// standing that nothing checks.
#[test]
fn allow_does_not_suppress_a_budget() {
    let d = lower_cpu(
        &format!(
            "module m\n@allow(\"cycles.over-budget\")\n@budget(cycles: 23)\n\
             proc f() {{\n{UNEVEN}}}\n"
        ),
        Cpu::Z80,
    );
    assert_eq!(with_id(&d, "cycles.over-budget").len(), 1, "{d:?}");
}

// ===========================================================================
// The declaration's own form
// ===========================================================================

/// The budget's unit is named by keyword, so a later budget over a different
/// resource reads unambiguously beside this one.
#[test]
fn budget_requires_its_resource_keyword() {
    let (_f, errs) = parse_str("module m\n@budget(100)\nproc f() {\n    ret\n}\n");
    assert!(
        errs.iter().any(|d| d.message.contains("[budget.form]")),
        "a positional budget is rejected: {errs:?}"
    );
}

/// `@cycles_exact` proves whatever cost the paths share, so it takes no argument.
#[test]
fn cycles_exact_takes_no_arguments() {
    let (_f, errs) = parse_str("module m\n@cycles_exact(195)\nproc f() {\n    ret\n}\n");
    assert!(
        errs.iter().any(|d| d.message.contains("[budget.form]")),
        "an argument to `@cycles_exact` is rejected: {errs:?}"
    );
}

/// A budget may be a comptime expression, not just a literal — the ceiling is
/// usually derived (a sample clock, a scanline count), and spelling it as
/// arithmetic is what keeps the derivation visible.
#[test]
fn the_budget_may_be_a_comptime_expression() {
    let src = "module m\nconst CLOCK = 24\n@budget(cycles: CLOCK - 1)\n\
               proc f() {\n    jp z, .skip\n    nop\n.skip:\n    ret\n}\n";
    let d = lower_cpu(src, Cpu::Z80);
    let f = assert_one(&d, "cycles.over-budget");
    assert!(f.message.contains("23"), "the folded ceiling is reported: {}", f.message);
}

/// A ceiling that does not fold reports the form error once, and the walk does
/// not then also complain about the body.
#[test]
fn an_unfoldable_budget_reports_once() {
    let d = lower_cpu(
        "module m\n@budget(cycles: NotAConst)\nproc f() {\n    call Helper\n    ret\n}\n",
        Cpu::Z80,
    );
    assert!(with_id(&d, "cycles.opaque-call").is_empty(), "no second complaint: {d:?}");
}

// ===========================================================================
// The shared model
// ===========================================================================

/// The walk and the `cycles(L1, L2)` comptime builtin read the SAME T-state
/// table, so a span costed both ways agrees. A second table could drift; this
/// pins that there is not one.
#[test]
fn the_walk_agrees_with_the_cycles_builtin() {
    // `ensure` proves the span is 18 T; the budget proves the whole proc — the
    // same span plus a 10-T `ret` — is 28. Both numbers come off `instr_cost`.
    let src = "module m\n@budget(cycles: 28)\nproc f() {\n\
               .start:\n\
               \x20   ld a, (hl)\n\
               \x20   ld (de), a\n\
               \x20   nop\n\
               .end:\n\
               \x20   ret\n\
               \x20   ensure(cycles(.start, .end) == 18, \"the shared table\")\n\
               }\n";
    assert_silent(&lower_cpu(src, Cpu::Z80));
    let d = lower_cpu(&src.replace("cycles: 28", "cycles: 27"), Cpu::Z80);
    assert_eq!(with_id(&d, "cycles.over-budget").len(), 1, "{d:?}");
}

/// An outcome-split conditional charges its taken and not-taken costs to their
/// OWN edges — the fact the straight-line builtin cannot use and refuses on.
/// `jr cc` is 12 taken / 7 not-taken, so the two arms differ by 1 T, not by 5.
#[test]
fn a_split_conditional_charges_each_edge_separately() {
    let body = "    jr z, .skip\n    nop\n.skip:\n    ret\n";
    // taken: 12 + 10 = 22 ; not-taken: 7 + 4 + 10 = 21.
    assert_silent(&z80("@budget(cycles: 22)", body));
    assert_one(&z80("@budget(cycles: 21)", body), "cycles.over-budget");
    let d = z80("@cycles_exact", body);
    let f = assert_one(&d, "cycles.path-mismatch");
    assert!(f.message.contains("21") && f.message.contains("22"), "{}", f.message);
}
