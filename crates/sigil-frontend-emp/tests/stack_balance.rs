//! §4-stack / S2-D7(b) — stack discipline: `[stack.unbalanced]` and
//! `[stack.merge-mismatch]`.
//!
//! sp must be back at its entry value on every path to a return, and paths that
//! merge must agree on where sp is. The checker reads the SAME symbolic stack the
//! `preserves` entry-value proof walks (`preserves::check_stack_balance`), so it
//! inherits that model's soundness bailouts rather than restating them.
//!
//! **Half of this file is silence.** The findings are ERROR-tier, so a false
//! positive breaks the build on correct code — which makes "the checker stays
//! quiet wherever the stack model bails" the load-bearing property, not a footnote.
//! Every bailout class therefore gets a probe, and each probe is paired with its
//! hazard-free twin so a green cannot mean the checker is simply dead.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` for `cpu`, asserting it parsed cleanly. Returns the
/// lowering diagnostics.
fn lower_cpu(src: &str, cpu: Cpu) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: cpu, include_root: None, embed_base: None, defines: vec![] },
    );
    diags
}

fn lower(src: &str) -> Vec<Diagnostic> {
    lower_cpu(src, Cpu::M68000)
}

/// Every `[stack.*]` diagnostic, whatever the id or level.
fn stack_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.message.contains("[stack.")).collect()
}

/// Every diagnostic carrying lint id `id`.
fn with_id<'a>(diags: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
    let tag = format!("[{id}]");
    diags.iter().filter(|d| d.message.contains(&tag)).collect()
}

/// A body whose only defect is one unpopped long — the shape every unbalanced
/// probe below builds on.
const IMBALANCED: &str = "    move.l d0, -(sp)\n    rts\n";

/// A body whose two paths rejoin at different sp offsets: the `beq` edge skips a
/// push the fall-through takes, so `.skip` is reached at depth 0 and depth 4.
const MERGE_MISMATCH: &str = "    tst.w d0\n\
                              \x20   beq.s .skip\n\
                              \x20   move.l d0, -(sp)\n\
                              .skip:\n\
                              \x20   rts\n";

/// `body` wrapped in a module with one proc, under the module-level `attrs` lines
/// (empty for the default tier).
fn proc_src_with(attrs: &str, body: &str) -> String {
    format!("module m\n{attrs}proc f() {{\n{body}}}\n")
}

fn proc_src(body: &str) -> String {
    proc_src_with("", body)
}

// ===========================================================================
// The findings fire
// ===========================================================================

/// A push with no matching pop reaches `rts` with sp four bytes low.
#[test]
fn push_without_pop_is_unbalanced() {
    let diags = lower(&proc_src(IMBALANCED));
    let firings = with_id(&diags, "stack.unbalanced");
    assert_eq!(firings.len(), 1, "expected exactly one firing, got: {diags:?}");
    assert_eq!(firings[0].level, Level::Error, "the tier is error by default (U-spec §6)");
    assert!(
        firings[0].message.contains("4 bytes still on the stack"),
        "the message names the delta: {}",
        firings[0].message
    );
}

/// A branch that pushes on one edge only leaves the two paths at different sp
/// offsets where they rejoin.
#[test]
fn one_sided_push_is_a_merge_mismatch() {
    let diags = lower(&proc_src(MERGE_MISMATCH));
    let firings = with_id(&diags, "stack.merge-mismatch");
    assert_eq!(firings.len(), 1, "expected exactly one firing, got: {diags:?}");
    assert_eq!(firings[0].level, Level::Error, "the tier is error by default (U-spec §6)");
    // Past the merge the model is BAILED, so the `rts` beneath it reports nothing:
    // the depth there is whichever branch arrived first, which is not a fact.
    assert!(
        with_id(&diags, "stack.unbalanced").is_empty(),
        "a mismatched merge must not also charge the return below it: {diags:?}"
    );
}

/// A finding is a fact about the path that produced it, not about the merged
/// view. Here both branches reach the `rts` holding 6 bytes — a real imbalance on
/// each — while their slot GEOMETRY differs (long-then-word against the reverse),
/// which taints the merge. The imbalance is still reported, because it was true of
/// a path the model tracked exactly before any merge erased it.
#[test]
fn a_real_per_path_imbalance_survives_a_tainted_merge() {
    let diags = lower(&proc_src(
        "    tst.w d0\n         \x20   beq.s .other\n         \x20   move.l d0, -(sp)\n         \x20   move.w d1, -(sp)\n         \x20   bra.s .join\n         .other:\n         \x20   move.w d1, -(sp)\n         \x20   move.l d0, -(sp)\n         .join:\n         \x20   rts\n",
    ));
    let firings = with_id(&diags, "stack.unbalanced");
    assert_eq!(firings.len(), 1, "expected the per-path imbalance, got: {diags:?}");
    assert!(
        firings[0].message.contains("6 bytes still on the stack"),
        "the depth is the path's own, not a merged guess: {}",
        firings[0].message
    );
}

/// A loop body that pushes without popping arrives at its own head with a deeper
/// stack every iteration — the merge mismatch a back edge exposes.
#[test]
fn a_loop_that_grows_the_stack_is_a_merge_mismatch() {
    let diags = lower(&proc_src(
        "    moveq #3, d0\n\
         .loop:\n\
         \x20   move.w d1, -(sp)\n\
         \x20   dbf d0, .loop\n\
         \x20   rts\n",
    ));
    assert!(
        !with_id(&diags, "stack.merge-mismatch").is_empty(),
        "a stack-growing loop must be reported: {diags:?}"
    );
}

// ===========================================================================
// Correct stack discipline is silent
// ===========================================================================

/// The movem save/restore pair — the corpus's dominant shape.
#[test]
fn a_matched_movem_pair_is_silent() {
    let diags = lower(&proc_src(
        "    movem.l d0-d1/a2, -(sp)\n    movem.l (sp)+, d0-d1/a2\n    rts\n",
    ));
    assert!(stack_diags(&diags).is_empty(), "a balanced body must be silent: {diags:?}");
}

/// The immediate-cleanup idiom: a scratch stash dropped with `addq #N, sp` rather
/// than popped into a register.
#[test]
fn an_immediate_sp_cleanup_balances_a_push() {
    let diags = lower(&proc_src("    move.l d0, -(sp)\n    addq.l #4, sp\n    rts\n"));
    assert!(stack_diags(&diags).is_empty(), "an addq cleanup balances the push: {diags:?}");
}

/// A call nets zero on the stack — the return address it pushes is popped by its
/// own `rts`, the convention the shared model already encodes.
#[test]
fn a_call_nets_zero_on_the_stack() {
    let diags = lower(&proc_src(
        "    movem.l d0-d1, -(sp)\n    jsr Helper\n    movem.l (sp)+, d0-d1\n    rts\n",
    ));
    assert!(stack_diags(&diags).is_empty(), "a bracketed call is balanced: {diags:?}");
}

/// A loop whose body pushes and pops in step reaches its head at one sp offset.
#[test]
fn a_balanced_loop_is_silent() {
    let diags = lower(&proc_src(
        "    moveq #3, d0\n\
         .loop:\n\
         \x20   move.w d1, -(sp)\n\
         \x20   move.w (sp)+, d1\n\
         \x20   dbf d0, .loop\n\
         \x20   rts\n",
    ));
    assert!(stack_diags(&diags).is_empty(), "a balanced loop must be silent: {diags:?}");
}

/// A tail transfer out of the proc is not charged, for the reason the entry-value
/// proof gives: it may diverge (a noreturn error rail owes its caller nothing) and
/// nothing in the language marks that yet.
#[test]
fn a_tail_transfer_out_is_not_charged() {
    let diags = lower(&proc_src("    move.l d0, -(sp)\n    jmp ErrorRail\n"));
    assert!(
        stack_diags(&diags).is_empty(),
        "a transfer out of the proc is not a return: {diags:?}"
    );
}

/// 68k only. `preserves.rs` and the `Cfg` edge model it walks are the 68k pair;
/// the Z80 sibling (`z80_preserves`) has no stack-delta arm, so a Z80 body must
/// not be judged by 68k rules.
#[test]
fn a_z80_body_is_not_checked() {
    let diags = lower_cpu("module m\nproc f() {\n    push hl\n    ret\n}\n", Cpu::Z80);
    assert!(stack_diags(&diags).is_empty(), "the checker is 68k-only: {diags:?}");
}

// ===========================================================================
// THE SILENCE PROBES — one per soundness bailout class
// ===========================================================================

/// Assert that inserting `hazard` into an otherwise-reported body silences the
/// checker, having FIRST proven the same body without it fires. Both halves
/// matter: the first is the bailout doing its job, the second is what stops a
/// dead checker from passing as a careful one.
fn assert_bailout_silences(what: &str, hazard: &str) {
    let firing = lower(&proc_src(IMBALANCED));
    assert!(
        !stack_diags(&firing).is_empty(),
        "precondition for `{what}`: the hazard-free twin must fire, got: {firing:?}"
    );

    let bailed = lower(&proc_src(&format!("    move.l d0, -(sp)\n    {hazard}\n    rts\n")));
    let leaked = stack_diags(&bailed);
    assert!(
        leaked.is_empty(),
        "`{what}` makes the stack model untrustworthy — an ERROR-tier finding past it \
         would be a guess, got: {leaked:?}"
    );
}

/// A bare `a7` operand: sp's VALUE escapes into address math, so the slot map no
/// longer describes the only path to that memory.
#[test]
fn a_bare_sp_operand_silences_the_checker() {
    assert_bailout_silences("a bare a7 operand", "movea.l sp, a0");
}

/// A COMPUTED sp advance: the byte count is not a static immediate, so no delta
/// can be attributed to it. (Its immediate sibling, `addq #N, sp`, is modeled —
/// see `an_immediate_sp_cleanup_balances_a_push`.)
#[test]
fn a_computed_sp_advance_silences_the_checker() {
    assert_bailout_silences("a computed adda to sp", "adda.l d1, sp");
}

/// A displaced sp WRITE could alias a tracked slot. (The displaced sp READ is
/// exempt — a load cannot alter a slot's contents — which is why this probe uses
/// the store form.)
#[test]
fn a_displaced_sp_write_silences_the_checker() {
    assert_bailout_silences("a displaced sp write", "move.l d0, 2(sp)");
}

/// An INDEXED sp access lands at a run-time offset, so it could alias any slot.
#[test]
fn an_indexed_sp_access_silences_the_checker() {
    assert_bailout_silences("an indexed sp access", "move.l d0, 2(sp, d1.w)");
}

/// A pop that drains more than was pushed reaches into the caller's frame — the
/// model is inconsistent from there on, so it reports nothing rather than
/// reporting the depth it invented.
#[test]
fn a_pop_underflow_silences_the_checker() {
    let diags = lower(&proc_src("    move.l (sp)+, d0\n    move.l d0, -(sp)\n    rts\n"));
    assert!(
        stack_diags(&diags).is_empty(),
        "an underflowing pop leaves nothing to measure against: {diags:?}"
    );
}

/// `bsr` is spelled like a conditional branch and is a CALL. Reading it as a
/// branch splices the local target's body into this proc's flow at the CALLER's
/// state, so an internal subroutine's own `rts` gets charged the caller's stack.
#[test]
fn a_local_bsr_does_not_charge_its_helper_with_the_callers_stack() {
    let diags = lower(&proc_src(
        "    move.l d0, -(sp)\n         \x20   bsr.s .helper\n         \x20   move.l (sp)+, d0\n         \x20   rts\n         .helper:\n         \x20   addq.w #1, d1\n         \x20   rts\n",
    ));
    assert!(
        stack_diags(&diags).is_empty(),
        "the helper returns at ITS entry depth, not the caller's: {diags:?}"
    );
}

/// A declared `falls_into` continues into its successor rather than returning, so
/// the pair may legitimately share one frame across the boundary. Charging the
/// fall-off-end would reject the idiom with a message about an `rts` the body does
/// not contain.
#[test]
fn a_declared_fallthrough_end_is_not_a_return() {
    let diags = lower(
        "module m\n         proc a() falls_into b {\n    move.l d0, -(sp)\n}\n         proc b() {\n    move.l (sp)+, d0\n    rts\n}\n",
    );
    assert!(
        stack_diags(&diags).is_empty(),
        "a shared frame across a declared fallthrough is legitimate: {diags:?}"
    );
}

/// The wall for the test above: an UNDECLARED fall-off-end IS charged. Its
/// successor is whatever follows in the section, which nothing checked — so
/// leaving the stack dirty there is a real defect and stays reported.
#[test]
fn an_undeclared_fall_off_end_is_still_charged() {
    let diags = lower("module m\nproc a() {\n    move.l d0, -(sp)\n}\n");
    assert!(
        !with_id(&diags, "stack.unbalanced").is_empty(),
        "an undeclared fallthrough keeps its stack obligation: {diags:?}"
    );
}

/// A pop whose width disagrees with the slots beneath it leaves sp somewhere the
/// slot map cannot name. Counting SLOTS rather than bytes would report 2 bytes
/// still held here — a false positive on balanced code.
#[test]
fn a_width_mismatched_pop_silences_the_checker() {
    let diags = lower(&proc_src(
        "    move.w d0, -(sp)\n    move.w d1, -(sp)\n    move.l (sp)+, d2\n    rts\n",
    ));
    assert!(
        stack_diags(&diags).is_empty(),
        "a `.l` pop over two `.w` pushes is balanced on the machine: {diags:?}"
    );
}

/// A plain `(sp)` STORE overwrites the top slot's contents, so the entry-value
/// model no longer knows what is there.
#[test]
fn a_top_of_stack_store_silences_the_checker() {
    assert_bailout_silences("a plain (sp) store", "move.l d1, (sp)");
}

/// The wall for the probe above: a `(sp)` READ is NOT a hazard whatever the
/// mnemonic. A load cannot alter a slot, and the corpus reads the top of stack
/// through `adda.w (sp), a2` (`sprites.emp:257`) — hazarding that would bail
/// `Render_Sprites` for doing something entirely safe.
#[test]
fn a_top_of_stack_read_is_not_a_hazard() {
    let diags = lower(&proc_src(
        "    move.w d0, -(sp)\n         \x20   adda.w (sp), a2\n         \x20   move.w (sp)+, d0\n         \x20   rts\n",
    ));
    assert!(stack_diags(&diags).is_empty(), "a top-of-stack read is balanced: {diags:?}");
}

/// An sp used as the INDEX register of another base is still sp escaping into
/// address math — the `IndIdx { xn: A7 }` arm of the hazard.
#[test]
fn sp_as_an_index_register_silences_the_checker() {
    assert_bailout_silences("sp as an index register", "move.l d1, 2(a0, sp.w)");
}

/// An sp cleanup that drains more than was pushed reaches into the caller's frame.
#[test]
fn an_over_draining_cleanup_silences_the_checker() {
    assert_bailout_silences("an over-draining sp cleanup", "addq.l #8, sp");
}

/// An sp cleanup landing MID-SLOT cannot be represented — the model would have to
/// split a tracked slot in half.
#[test]
fn a_mid_slot_cleanup_silences_the_checker() {
    assert_bailout_silences("a mid-slot sp cleanup", "addq.l #2, sp");
}

// ===========================================================================
// Tier: @as_compat softening and @allow
// ===========================================================================

/// U-spec §6: `[stack.*]` softens to a WARNING under `@as_compat`. Unlike a
/// declared contract — which the author opted into — this gate reads raw ported
/// assembly nobody annotated, so a faithful port keeps the finding visible without
/// failing its build.
#[test]
fn as_compat_softens_the_finding_to_a_warning() {
    let diags = lower(&proc_src_with("@as_compat\n", IMBALANCED));
    let firings = with_id(&diags, "stack.unbalanced");
    assert_eq!(firings.len(), 1, "the finding is still reported: {diags:?}");
    assert_eq!(
        firings[0].level,
        Level::Warning,
        "@as_compat softens rather than silences: {}",
        firings[0].message
    );
}

/// `@as_compat` softens the MERGE-mismatch arm too — one attribute governs the
/// whole `[stack.*]` family, so a port cannot be half-softened.
#[test]
fn as_compat_softens_the_merge_mismatch_too() {
    let diags = lower(&proc_src_with("@as_compat\n", MERGE_MISMATCH));
    let firings = with_id(&diags, "stack.merge-mismatch");
    assert_eq!(firings.len(), 1, "expected the merge finding: {diags:?}");
    assert_eq!(
        firings[0].level,
        Level::Warning,
        "one attribute governs the whole family: {}",
        firings[0].message
    );
}

/// U-spec §6 gives `[stack.*]` an `@allow` escape, for the module that means it.
#[test]
fn allow_suppresses_the_finding() {
    let diags = lower(&proc_src_with("@allow(\"stack.unbalanced\")\n", IMBALANCED));
    assert!(with_id(&diags, "stack.unbalanced").is_empty(), "@allow opts out: {diags:?}");
}

/// The `@allow` is per-id, not per-family: allowing the unbalanced arm leaves the
/// merge arm reporting.
#[test]
fn allow_is_keyed_per_lint_id() {
    let diags = lower(&proc_src_with("@allow(\"stack.unbalanced\")\n", MERGE_MISMATCH));
    assert!(
        !with_id(&diags, "stack.merge-mismatch").is_empty(),
        "the merge arm keeps its own id: {diags:?}"
    );
}
