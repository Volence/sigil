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

/// Every `[cycles.*]` diagnostic, whatever the id or level.
fn cycle_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.message.contains("[cycles."))
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

/// The 68000 twin of [`z80`].
fn m68k(attrs: &str, body: &str) -> Vec<Diagnostic> {
    lower_cpu(&format!("module m\n{attrs}\nproc f() {{\n{body}}}\n"), Cpu::M68000)
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

/// Padding the short arm until the two match is what the proof is FOR. The pair
/// that still differs fires; the pair that has been balanced proves.
#[test]
fn cycles_exact_proves_a_padded_pair() {
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

/// A single-path body is trivially exact — and the twin proves the checker is
/// awake on the same shape the moment a second path appears.
#[test]
fn a_straight_line_is_trivially_exact() {
    assert_silent(&z80("@cycles_exact", "    nop\n    nop\n    ret\n"));
    assert_one(
        &z80("@cycles_exact", "    jp z, .skip\n    nop\n.skip:\n    nop\n    ret\n"),
        "cycles.path-mismatch",
    );
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

/// A `djnz` whose target is not a label of this body takes its counting leg OUT
/// of the proc, so the path's cost continues somewhere this walk cannot see. The
/// refusal names that — the structural fact, not the downstream symptom that its
/// 13/8 split has nowhere to be routed.
#[test]
fn a_djnz_leg_out_of_the_body_is_refused() {
    assert_one(
        &z80("@budget(cycles: 100)", "    djnz Elsewhere\n    ret\n"),
        "cycles.unbounded-transfer",
    );
}

/// A split-cost conditional whose leg leaves the body earns the STRUCTURAL
/// refusal, and earns it FIRST: `charged_edges` refuses on the first leaving edge,
/// before it counts edges or consults the cost table. So `[cycles.ambiguous-branch]`
/// is not what a body gets here — `[cycles.unbounded-transfer]` is, and it names
/// what actually happens: the leg leaves the proc.
///
/// What this does NOT measure is the edge COUNT. Because the structural refusal
/// comes first, a shape presenting a single leaving edge would earn the same id and
/// this sweep would stay green — so the invariant that leaves the ambiguous-branch
/// guard inputless is pinned crate-side instead, off the edge builders themselves
/// (`cycle_budget.rs`'s `mod tests`,
/// `a_split_cost_terminator_presents_exactly_two_edges`).
///
/// Swept over every Z80 and 68k split-cost terminator in every target shape,
/// including the three a raw `label_target` lookup would drop the leg for. A
/// counted sweep, so it cannot pass by measuring nothing; the `ret z` twin that
/// MEASURES proves the walk is awake on the same instruction class.
#[test]
fn a_split_cost_conditional_is_refused_before_its_edges_are_counted() {
    let z80_shapes = [
        "    djnz Elsewhere\n    ret\n",   // leg out of the body
        "    djnz .done\n    ret\n.done:\n", // leg to a body-closing label
        "    djnz Elsewhere\n",            // leg out, and no fall-through
        "    jr z, Elsewhere\n    ret\n",
        "    jr z, .done\n    ret\n.done:\n",
        "    jr z, Elsewhere\n",
        "    ret z\n",
    ];
    let mut swept = 0;
    for body in z80_shapes {
        assert_one(&z80("@budget(cycles: 100)", body), "cycles.unbounded-transfer");
        swept += 1;
    }
    let m68k_shapes = [
        "    beq Elsewhere\n    rts\n",
        "    beq Elsewhere\n",
        "    dbra d0, Elsewhere\n    rts\n",
    ];
    for body in m68k_shapes {
        assert_one(&m68k("@budget(cycles: 100)", body), "cycles.unbounded-transfer");
        swept += 1;
    }
    assert_eq!(swept, 10, "the sweep covered {swept} shapes");

    // The twin: a `ret cc` that stays inside the body routes its 11/5 split
    // across two edges and MEASURES.
    assert_silent(&z80("@budget(cycles: 100)", "    ret z\n    ret\n"));
}

/// A CONDITIONAL return at the end of a body returns on its taken edge and runs
/// off the end on the other. Only the first ends a path; charging both would
/// close an escaping path at zero cost and report a bound that is TOO LOW.
#[test]
fn a_tail_conditional_return_is_refused() {
    assert_one(
        &z80("@budget(cycles: 100)", "    ld a, 1\n    ret z\n"),
        "cycles.unbounded-transfer",
    );
    // The twin: give the fall-through a return of its own and it measures.
    assert_silent(&z80("@budget(cycles: 22)", "    ld a, 1\n    ret z\n    ret\n"));
}

/// Data spliced into the code stream emits bytes that DECODE if control reaches
/// them, and the control-flow model steps over them. A body carrying any is
/// refused rather than costed as if the bytes were free.
#[test]
fn inline_data_is_refused() {
    assert_silent(&z80("@budget(cycles: 100)", "    nop\n    ret\n"));
    assert_one(
        &z80("@budget(cycles: 100)", "    nop\n    dc.b $00, $00\n    ret\n"),
        "cycles.inline-data",
    );
}

/// A proc inside a `section` is still a proc, and its contract is still checked —
/// which is where every Z80 proc in the engine lives.
#[test]
fn a_section_scoped_proc_carries_its_budget() {
    let src = "module m\n\
               section s (cpu: z80, vma: $0000) {\n\
               @budget(cycles: 13)\n\
               proc f() {\n    nop\n    ret\n}\n\
               }\n";
    assert_silent(&lower_cpu(&src.replace("cycles: 13", "cycles: 14"), Cpu::Z80));
    let d = lower_cpu(src, Cpu::Z80);
    assert_eq!(with_id(&d, "cycles.over-budget").len(), 1, "{d:?}");
}

/// A declared `falls_into` names its successor, so the refusal says THAT rather
/// than describing it as an unknown escape.
#[test]
fn a_declared_fallthrough_is_refused_by_its_own_name() {
    let src = "module m\nproc g() {\n    ret\n}\n\
               @budget(cycles: 100)\n\
               proc f() falls_into g {\n    nop\n}\n";
    let d = lower_cpu(src, Cpu::Z80);
    let f = with_id(&d, "cycles.unbounded-transfer");
    assert_eq!(f.len(), 1, "{d:?}");
    assert!(f[0].message.contains("`g`"), "names the successor: {}", f[0].message);
}

/// Two declarations of one contract are two claims where the reader sees one.
#[test]
fn a_repeated_declaration_is_reported() {
    let d = z80("@budget(cycles: 100)\n@budget(cycles: 1)", "    nop\n    ret\n");
    assert_eq!(with_id(&d, "cycles.form").len(), 1, "the repeat is named: {d:?}");
}

/// A misspelled attribute is a proof that silently disappears, so the name is a
/// closed set.
#[test]
fn a_misspelled_attribute_is_loud() {
    let (_f, errs) = parse_str("module m\n@cycles_exakt\nproc f() {\n    ret\n}\n");
    assert!(
        errs.iter().any(|d| d.message.contains("[attr.unknown]")),
        "a typo does not silently drop the proof: {errs:?}"
    );
}

/// A call costs whatever its callee costs, which is not a fact about this proc.
///
/// The CONDITIONAL form is the one that matters beyond the obvious: `call cc`
/// carries a SPLIT cost (17/10) over a SINGLE edge — it calls and comes back, so
/// its only successor is the fall-through — which is the exact shape
/// `[cycles.ambiguous-branch]` exists to refuse. It never gets there because the
/// call bail is the first statement of `charged_edges`. This pins which of the two
/// refusals a `call cc` earns, so a reordering that broke the ambiguous-branch
/// guard's inputlessness would show up as a changed id here.
#[test]
fn a_call_is_refused() {
    assert_one(&z80("@budget(cycles: 100)", "    call Helper\n    ret\n"), "cycles.opaque-call");
    assert_one(
        &z80("@budget(cycles: 100)", "    call nz, Helper\n    ret\n"),
        "cycles.opaque-call",
    );
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
///
/// The fixture has been `rlca`, then `ldir`, and each in turn became priced,
/// leaving this asserting that a KNOWN op was unknown. `rlc (ix+0)` is chosen to
/// end that: the DDCB shift column has no encoding in this assembler at all
/// (`encode_cb_shift` refuses an indexed target), so it CANNOT become priced,
/// the cost table's coverage guard is keyed on what the encoder accepts, and it
/// does not accept this. The accompanying `[lower.z80-unsupported]` is not a
/// cycle finding and `assert_one` filters on `[cycles.`, so it does not disturb
/// the count.
#[test]
fn an_off_table_op_is_refused() {
    assert_silent(&z80("@budget(cycles: 100)", "    nop\n    ret\n"));
    assert_one(&z80("@budget(cycles: 100)", "    rlc (ix+0)\n    ret\n"), "cycles.unknown-op");
    // The TWIN that separates "off the table" from "refused for another reason":
    // `ldir` used to be this test's fixture and is now priced, as a split cost.
    // It is still refused, a block repeat has one edge and two costs, but under
    // the other id, so a walk that answered `unknown-op` for everything could not
    // pass both halves.
    assert_one(&z80("@budget(cycles: 100)", "    ldir\n    ret\n"), "cycles.ambiguous-branch");
    // And the single-step sibling MEASURES: ldi 16 + ret 10 = 26.
    assert_silent(&z80("@budget(cycles: 26)", "    ldi\n    ret\n"));
}

/// A 68k `@budget` measures through the M68000UM table: the ceiling that admits
/// the worst path holds, one cycle under it fires naming both numbers.
#[test]
fn a_68k_budget_bounds_the_worst_path() {
    // moveq 4 + rts 16 = 20, through the M68000UM table.
    assert_silent(&m68k("@budget(cycles: 20)", "    moveq #1, d0\n    rts\n"));
    let d = m68k("@budget(cycles: 19)", "    moveq #1, d0\n    rts\n");
    let f = assert_one(&d, "cycles.over-budget");
    assert!(f.message.contains("20"), "names the measured cost: {}", f.message);
    assert!(f.message.contains("19"), "and the declared budget: {}", f.message);
}

/// An UNSIZED 68k conditional relaxes `.s`/`.w` at link time, so its
/// fall-through is charged the `.w` ceiling: the budget that admits the ceiling
/// holds, one cycle less fires.
#[test]
fn an_unsized_68k_conditional_is_charged_its_word_fall_through() {
    // taken: 10 + 16 = 26; fall-through ceiling: 12 + 4 + 16 = 32.
    let body = "    beq .skip\n    moveq #1, d0\n.skip:\n    rts\n";
    assert_silent(&m68k("@budget(cycles: 32)", body));
    let d = m68k("@budget(cycles: 31)", body);
    let f = assert_one(&d, "cycles.over-budget");
    assert!(f.message.contains("32"), "{}", f.message);
}

/// A ceiling charge holds a budget but cannot prove an equality: `@cycles_exact`
/// over a linker-relaxed `jbra` refuses by the offender's name, while the budget
/// declared beside it still concludes.
#[test]
fn a_ceiling_refuses_an_exactness_proof_but_holds_a_budget() {
    // jbra charged its dearest rung (jmp abs.l, 12) + rts 16 = 28.
    let body = "    jbra .join\n.join:\n    rts\n";
    assert_silent(&m68k("@budget(cycles: 28)", body));
    let d = m68k("@cycles_exact", body);
    let f = assert_one(&d, "cycles.inexact-cost");
    assert!(f.message.contains("jbra"), "names the offender: {}", f.message);
    let both = m68k("@budget(cycles: 28)\n@cycles_exact", body);
    assert_eq!(cycle_diags(&both).len(), 1, "the budget half still holds: {both:?}");
    assert_eq!(with_id(&both, "cycles.inexact-cost").len(), 1);
}

/// A computed dispatch — `jmp .table(a1)`, the DMA-queue drain shape — is
/// refused by its own name: the destination set is data, and the walk does not
/// enumerate what the program text does not name.
#[test]
fn a_68k_computed_dispatch_is_refused_by_its_own_name() {
    let d = m68k("@budget(cycles: 670)", "    jmp .table(a1)\n.table:\n    rts\n");
    let f = assert_one(&d, "cycles.computed-transfer");
    assert!(f.message.contains("jmp"), "{}", f.message);
    assert!(f.message.contains("COMPUTED"), "says what it refused: {}", f.message);
}

/// Every 68000 call spelling is opaque, the auto-reaching `jbsr` included.
#[test]
fn a_68k_call_is_refused() {
    let d = m68k(
        "@budget(cycles: 100)",
        "    jbsr .helper\n    rts\n.helper:\n    rts\n",
    );
    assert_eq!(with_id(&d, "cycles.opaque-call").len(), 1, "{d:?}");
}

/// A 68k back edge is refused like a Z80 one — `dbf`, the counting idiom, is a
/// loop before it is anything else.
#[test]
fn a_68k_loop_is_refused() {
    let d = m68k("@budget(cycles: 100)", ".loop:\n    nop\n    bra .loop\n");
    assert_eq!(with_id(&d, "cycles.unbounded-loop").len(), 1, "{d:?}");
    let d = m68k(
        "@budget(cycles: 100)",
        ".loop:\n    nop\n    dbf d0, .loop\n    rts\n",
    );
    assert_eq!(with_id(&d, "cycles.unbounded-loop").len(), 1, "{d:?}");
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
        errs.iter().any(|d| d.message.contains("[cycles.form]")),
        "a positional budget is rejected: {errs:?}"
    );
}

/// `@cycles_exact` proves whatever cost the paths share, so it takes no argument.
#[test]
fn cycles_exact_takes_no_arguments() {
    let (_f, errs) = parse_str("module m\n@cycles_exact(195)\nproc f() {\n    ret\n}\n");
    assert!(
        errs.iter().any(|d| d.message.contains("[cycles.form]")),
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

/// A ceiling that does not fold reports the form error, and the walk does NOT
/// then also complain about a body it was never able to judge.
#[test]
fn an_unfoldable_budget_reports_the_form_and_stops() {
    let d = lower_cpu(
        "module m\n@budget(cycles: NotAConst)\nproc f() {\n    call Helper\n    ret\n}\n",
        Cpu::Z80,
    );
    assert_eq!(with_id(&d, "cycles.form").len(), 1, "the broken ceiling is named: {d:?}");
    assert!(with_id(&d, "cycles.opaque-call").is_empty(), "and the body is not judged: {d:?}");
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

// ===========================================================================
// Enumerated dispatch — `targets(...)` (enumerated-dispatch design)
// ===========================================================================

/// Every `[dispatch.*]` diagnostic, whatever the id.
fn dispatch_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.message.contains("[dispatch.")).collect()
}

/// A 68k module whose proc `f` carries the given attribute line and body,
/// clobbering a1/a5 the way the drain shape does.
fn dispatch_mod(attrs: &str, body: &str) -> Vec<Diagnostic> {
    lower_cpu(
        &format!("module m\n{attrs}\nproc f () clobbers(a1/a5) {{\n{body}}}\n"),
        Cpu::M68000,
    )
}

/// The dma-drain shape in miniature, full pipeline: a computed `jmp` enumerated
/// over three local labels, the drain groups falling through each other. The
/// dearest arm is `jmp`(10) + two `move.l`(20) + `rts`(16) = 66.
const DISPATCH_BODY: &str = "\
        jmp     .table(a1) targets(.done, .drain_2, .drain_1)\n\
    .table:\n\
        rts\n\
    .drain_2:\n\
        move.l  (a1)+, (a5)\n\
    .drain_1:\n\
        move.l  (a1)+, (a5)\n\
        rts\n\
    .done:\n\
        rts\n";

/// End-to-end (spec §5 positive pin): the clause resolves through the label
/// scope, the walk sees THROUGH the dispatch, and a budget pinned at the measured
/// ceiling verifies clean — no cycle finding, no dispatch refusal.
#[test]
fn an_enumerated_dispatch_verifies_its_budget() {
    let d = dispatch_mod("@budget(cycles: 66)", DISPATCH_BODY);
    assert_silent(&d);
    assert!(dispatch_diags(&d).is_empty(), "no refusal on a well-formed clause: {:?}", dispatch_diags(&d));
}

/// One under the ceiling and the walk reports the measured 66 — proof the number
/// came from the enumerated arms, not the prose.
#[test]
fn an_enumerated_dispatch_over_budget_names_its_cost() {
    let d = dispatch_mod("@budget(cycles: 65)", DISPATCH_BODY);
    let f = assert_one(&d, "cycles.over-budget");
    assert!(f.message.contains("66"), "names the measured worst path: {}", f.message);
}

/// Without the clause the SAME jmp is the pre-form refusal: a computed transfer
/// the walk will not put a number on.
#[test]
fn without_the_clause_the_dispatch_still_refuses() {
    let body = DISPATCH_BODY.replacen(" targets(.done, .drain_2, .drain_1)", "", 1);
    let d = dispatch_mod("@budget(cycles: 66)", &body);
    assert_one(&d, "cycles.computed-transfer");
}

/// The refusals, each a full-pipeline probe with the corpus spelling.
#[test]
fn targets_on_a_call_refuses_end_to_end() {
    let d = dispatch_mod("", "        jsr (a1) targets(.done)\n    .done:\n        rts\n");
    let r = dispatch_diags(&d);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("[dispatch.targets-on-call]"), "{}", r[0].message);
    assert_eq!(r[0].level, Level::Error);
}

#[test]
fn targets_on_a_direct_jmp_refuses_end_to_end() {
    let d = dispatch_mod("", "        jmp .done targets(.done)\n    .done:\n        rts\n");
    let r = dispatch_diags(&d);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("[dispatch.targets-redundant]"), "{}", r[0].message);
}

#[test]
fn an_unknown_target_refuses_end_to_end() {
    let d = dispatch_mod("", "        jmp .table(a1) targets(.typo)\n    .table:\n        rts\n");
    let r = dispatch_diags(&d);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("[dispatch.target-unknown]"), "{}", r[0].message);
}

#[test]
fn a_nonlocal_target_refuses_end_to_end() {
    let d = dispatch_mod("", "        jmp .table(a1) targets(SomeGlobal)\n    .table:\n        rts\n");
    let r = dispatch_diags(&d);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("[dispatch.target-nonlocal]"), "{}", r[0].message);
}

#[test]
fn a_duplicate_target_refuses_end_to_end() {
    let d = dispatch_mod(
        "",
        "        jmp .table(a1) targets(.done, .done)\n    .table:\n        rts\n    .done:\n        rts\n",
    );
    let r = dispatch_diags(&d);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("[dispatch.target-duplicate]"), "{}", r[0].message);
}

/// The empty set is a contradiction, refused at PARSE (before lowering) — so the
/// probe reads the parse diagnostics directly.
#[test]
fn an_empty_target_set_refuses_at_parse() {
    let (_f, perrs) = parse_str("module m\nproc f () {\n    jmp .table(a1) targets()\n.table:\n    rts\n}\n");
    assert!(
        perrs.iter().any(|d| d.message.contains("[dispatch.targets-empty]")),
        "parse diagnostics: {perrs:?}"
    );
}

/// §1 composition: `as ContractType` and `targets(...)` are orthogonal and parse
/// together on one instruction (as bounds CLOBBER, targets bounds GO).
#[test]
fn as_type_and_targets_compose_in_the_grammar() {
    let (_f, d) =
        parse_str("module m\nproc f () {\n    jmp .t(a1) as Foo targets(.a, .b)\n.a:\n    rts\n.b:\n    rts\n}\n");
    assert!(d.is_empty(), "`as` and `targets` should parse together: {d:?}");
}

/// The clause is checked on a DISPATCH-table inline body (not just a named proc):
/// the buf funnels through the same `lower_code_buf` chokepoint. A `targets(.typo)`
/// there must refuse, not silently carry.
#[test]
fn a_dispatch_inline_body_checks_its_targets() {
    let d = lower_cpu(
        "module m\ndispatch R (encoding: word_offsets) {\n    Go: { jmp .t(a1) targets(.typo)\n.t:\n    rts },\n}\n",
        Cpu::M68000,
    );
    let r = dispatch_diags(&d);
    assert!(
        r.iter().any(|x| x.message.contains("[dispatch.target-unknown]")),
        "dispatch inline body must check its targets: {r:?}"
    );
}

/// The clause is checked on a SCRIPT body (the third funnel through
/// `lower_code_buf`). A `targets(.typo)` there must refuse.
#[test]
fn a_script_body_checks_its_targets() {
    let src = "module m\n\
newtype ScriptPc = u16\n\
struct S (size: $24) {\n\
    _pad0: [u8; $20],\n\
    resume: ScriptPc @ $20,\n\
    _pad1: [u8; 2] @ $22,\n\
}\n\
script s (a0: *S) (encoding: word_offsets) shows Draw {\n\
    jmp .t(a1) targets(.typo)\n\
.t:\n\
    yield\n\
}\n";
    let d = lower_cpu(src, Cpu::M68000);
    let r = dispatch_diags(&d);
    assert!(
        r.iter().any(|x| x.message.contains("[dispatch.target-unknown]")),
        "script body must check its targets: {r:?}"
    );
}

/// A `dc` directive lowers to inline DATA, never a `CodeItem::Instr`, so it would
/// skip the enumerated-dispatch check — a stray `targets(...)` on it is refused
/// loudly rather than silently dropped.
#[test]
fn targets_on_dc_data_is_refused() {
    let d = dispatch_mod("", "        dc.w 5 targets(.x)\n");
    assert!(
        d.iter().any(|x| x.message.contains("[dispatch.targets-on-data]")),
        "dc must refuse a targets clause: {:?}",
        d.iter().map(|x| &x.message).collect::<Vec<_>>()
    );
}

/// A trailing label (one that closes the body) named as a target is refused
/// end-to-end: a fall-off is not a landing point.
#[test]
fn a_trailing_target_refuses_end_to_end() {
    let d = dispatch_mod(
        "",
        "        jmp .t(a1) targets(.done)\n    .t:\n        rts\n    .done:\n",
    );
    let r = dispatch_diags(&d);
    assert!(
        r.iter().any(|x| x.message.contains("[dispatch.target-trailing]")),
        "trailing-label target must refuse: {r:?}"
    );
}
