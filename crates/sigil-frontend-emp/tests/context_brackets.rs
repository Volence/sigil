//! `[context.*]` — the DECLARED machine-state tier (contract unification §3):
//! the `context` item, the `with <ctx> { }` bracket and its three proofs, and
//! `requires(...)` propagation.
//!
//! The bracket layer sits ABOVE the inferred `[bus.*]` net, and the pair of
//! tests below is the reason it exists:
//! [`the_inference_tier_cannot_see_a_branch_out_of_a_hand_written_pair`] pins
//! the blind spot (a MUST lattice joins to `Unknown` at the merge and fires
//! nothing) and [`escape_fires_on_a_branch_out_of_the_region`] pins the same
//! shape caught. Every other test here pins one rule of the surface.

use sigil_frontend_emp::context::ContextFiringKind;
use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::z80_bus::BusFiringKind;

/// The engine's real bus bracket, inline (a context's acquire/release evaluate
/// in the CONSUMER's scope, so a cross-module context spells them out rather
/// than calling a module-private template).
const Z80_CTX: &str = "context z80_stopped {\n\
     \x20   acquire = asm { move.w #$0100, Z80_BUS_REQUEST\n\
     \x20                  .wait_z80:\n\
     \x20                   btst #0, Z80_BUS_REQUEST\n\
     \x20                   bne .wait_z80 }\n\
     \x20   release = asm { move.w #$0000, Z80_BUS_REQUEST }\n\
     }\n";

fn analyze(src: &str) -> ContractReport {
    let (f, diags) = parse_str(src);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "parse diagnostics: {errs:?}");
    analyze_corpus(&[f])
}

fn ctx_count(r: &ContractReport, proc: &str, kind: ContextFiringKind) -> usize {
    r.context_firings.iter().filter(|f| f.proc == proc && f.kind == kind).count()
}

fn bus_count(r: &ContractReport, proc: &str, kind: BusFiringKind) -> usize {
    r.bus_firings.iter().filter(|f| f.proc == proc && f.kind == kind).count()
}

// ---------------------------------------------------------------------------
// The headline pair: the inference tier's blind spot, and the bracket closing it
// ---------------------------------------------------------------------------

/// THE GAP. A branch out of a hand-written stop/start pair leaves the bus held
/// on one path — the hazard `bg.emp`'s header documents in prose ("a guard
/// branch taken from inside the bracket would leave the Z80 halted for the rest
/// of the level"). The `[bus.*]` MUST lattice CANNOT see it: at `.skip` the
/// held path meets the released path, the state falls to `Unknown`, and the
/// zero-false-positive stance means the `rts` fires nothing.
///
/// This test asserts the ABSENCE that motivates the whole construct, so it is
/// pinned rather than asserted in prose — if the inference tier ever grows teeth
/// here, this fails and the claim below must be re-derived.
#[test]
fn the_inference_tier_cannot_see_a_branch_out_of_a_hand_written_pair() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0) {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             tst.w   d0\n\
             bne     .skip\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
         .skip:\n\
             rts\n\
         }\n",
    );
    assert_eq!(
        r.bus_firings.iter().filter(|f| f.proc == "P").count(),
        0,
        "the inference tier is expected to be BLIND here (join = Unknown): {:?}",
        r.bus_firings
    );
}

/// THE CLOSURE. The same shape written as a bracket: the branch target sits past
/// the compiler-generated release, so the branch is a path that skips it —
/// `[context.escape]`, on every path, with no `Unknown` to bail on.
#[test]
fn escape_fires_on_a_branch_out_of_the_region() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0) {{\n\
             with z80_stopped {{\n\
                 tst.w   d0\n\
                 bne     .skip\n\
             }}\n\
         .skip:\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "P", ContextFiringKind::Escape),
        1,
        "{:?}",
        r.context_firings
    );
}

// ---------------------------------------------------------------------------
// The escape proof
// ---------------------------------------------------------------------------

/// A `rts` inside the bracketed body never reaches the release.
#[test]
fn escape_fires_on_a_return_inside_the_body() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0) {{\n\
             with z80_stopped {{\n\
                 tst.w   d0\n\
                 rts\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Escape), 1, "{:?}", r.context_firings);
}

/// A tail transfer to an EXTERNAL symbol leaves the proc — and the region —
/// without the release (`Edge::TailOut`).
#[test]
fn escape_fires_on_a_tail_transfer_out() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{\n\
                 jbra    Elsewhere\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Escape), 1, "{:?}", r.context_firings);
}

/// The happy path: a branch INSIDE the region (the `Sound_DrainSfxRing` shape —
/// an early-out that still lands on the release) is not an escape. Without this
/// the escape proof would be a range test that rejects every real bracket.
#[test]
fn a_branch_to_a_label_inside_the_region_is_not_an_escape() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0) {{\n\
             with z80_stopped {{\n\
                 tst.w   d0\n\
                 bne     .done\n\
                 moveq   #1, d0\n\
             .done:\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Escape), 0, "{:?}", r.context_firings);
    assert_eq!(r.context_regions, vec![("P".to_string(), "z80_stopped".to_string())]);
}

/// A `jbsr` out of the region RETURNS, so it is not an escape — the CFG models
/// a call as a fall-through. (`Read_Controllers` calls a local sub-block from
/// inside its bracket; a naive "any transfer out" rule would reject it.)
#[test]
fn a_call_out_of_the_region_is_not_an_escape() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{\n\
                 jbsr    .helper\n\
             }}\n\
             rts\n\
         .helper:\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Escape), 0, "{:?}", r.context_firings);
    // The region must EXIST to have been checked — an assert-absence over a
    // checker that saw nothing proves nothing.
    assert_eq!(r.context_regions.len(), 1, "{:?}", r.context_regions);
}

// ---------------------------------------------------------------------------
// entry-skip and reacquire
// ---------------------------------------------------------------------------

/// A branch from OUTSIDE into the region reaches the body without the acquire —
/// and then runs the release, freeing a context never taken.
#[test]
fn entry_skip_fires_on_a_branch_into_the_region() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0) {{\n\
             tst.w   d0\n\
             bne     .inside\n\
             with z80_stopped {{\n\
             .inside:\n\
                 moveq   #1, d0\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::EntrySkip), 1, "{:?}", r.context_firings);
}

/// Nesting the SAME acquired context is `[context.reacquire]`: the inner release
/// frees the outer hold (a bus request is not a counting lock).
#[test]
fn reacquire_fires_on_the_same_context_nested() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{\n\
                 with z80_stopped {{\n\
                     nop\n\
                 }}\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Reacquire), 1, "{:?}", r.context_firings);
}

/// Nesting DIFFERENT contexts is legal (§3.2) — the corpus's `ints_off` +
/// bus-hold shape. Neither the reacquire rule nor the escape proof may fire.
#[test]
fn nesting_different_contexts_is_legal() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         context ints_off {{\n\
             acquire = asm {{ move.w sr, -(sp)\n move.w #$2700, sr }}\n\
             release = asm {{ move.w (sp)+, sr }}\n\
         }}\n\
         pub proc P () clobbers() preserves(sr) {{\n\
             with ints_off {{\n\
                 with z80_stopped {{\n\
                     nop\n\
                 }}\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert!(r.context_firings.is_empty(), "{:?}", r.context_firings);
    let mut names: Vec<&str> = r.context_regions.iter().map(|(_, c)| c.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["ints_off", "z80_stopped"]);
}

// ---------------------------------------------------------------------------
// `requires` propagation
// ---------------------------------------------------------------------------

/// A call to a `requires(vblank)` callee from a proc that neither declares the
/// requirement nor grants it is `[context.unsatisfied]`.
#[test]
fn unsatisfied_fires_at_an_undischarged_call_site() {
    let r = analyze(
        "module m\n\
         context vblank { granted }\n\
         proc Callee () clobbers() requires(vblank) { rts }\n\
         pub proc Outside () clobbers() { jbsr Callee\n rts }\n",
    );
    assert_eq!(r.context_unsatisfied.len(), 1, "{:?}", r.context_unsatisfied);
    assert_eq!(r.context_unsatisfied[0].callee, "Callee");
    assert_eq!(r.context_unsatisfied[0].ctx, "vblank");
}

/// The three discharges (§3.3), and the propagation chain that makes the middle
/// one work: a `grants(vblank)` ROOT calls a `requires(vblank)` proc, which calls
/// another — the residue appears in the middle proc's own `requires`, so nothing
/// fires anywhere in the chain.
#[test]
fn requires_propagates_three_deep_from_a_grant_root() {
    let r = analyze(
        "module m\n\
         context vblank { granted }\n\
         proc Leaf () clobbers() requires(vblank) { rts }\n\
         proc Middle () clobbers() requires(vblank) { jbsr Leaf\n rts }\n\
         pub proc Root () clobbers() grants(vblank) { jbsr Middle\n rts }\n",
    );
    assert!(r.context_unsatisfied.is_empty(), "{:?}", r.context_unsatisfied);
    assert_eq!(
        r.context_claim_sites,
        vec![
            ("Leaf".to_string(), "requires".to_string(), "vblank".to_string()),
            ("Middle".to_string(), "requires".to_string(), "vblank".to_string()),
            ("Root".to_string(), "grants".to_string(), "vblank".to_string()),
        ]
    );
}

/// A BRACKET discharges a requirement over its own range, and only over it: the
/// same callee called from inside the bracket is fine and from after it fires.
/// (A bracket-wide discharge that ignored the range would make this vacuous.)
#[test]
fn a_bracket_discharges_a_requirement_over_its_range_only() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         proc Callee () clobbers() requires(z80_stopped) {{ rts }}\n\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{\n\
                 jbsr    Callee\n\
             }}\n\
             jbsr    Callee\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(r.context_unsatisfied.len(), 1, "{:?}", r.context_unsatisfied);
    assert_eq!(r.context_unsatisfied[0].proc, "P");
}

/// A `requires`/`grants` naming a context no module declares is reported: a
/// silently-ignored requirement reads as a checked claim and is not one.
#[test]
fn an_undeclared_context_reference_is_reported() {
    let r = analyze(
        "module m\n\
         proc Callee () clobbers() requires(no_such_ctx) { rts }\n",
    );
    assert_eq!(r.unknown_context_refs.len(), 1, "{:?}", r.unknown_context_refs);
    assert_eq!(r.unknown_context_refs[0].1, "no_such_ctx");
}

// ---------------------------------------------------------------------------
// The declared entry seed — the two tiers meeting
// ---------------------------------------------------------------------------

/// THE GAP (entry edition). A proc that runs with the bus already held — and
/// stops it AGAIN, or releases it and returns — is invisible to the inference
/// tier: entry is seeded `Unknown` because a caller may already hold the bus,
/// and that is not locally provable. Both bodies below are silent.
#[test]
fn an_unpaired_toggle_at_proc_entry_is_invisible_without_a_declaration() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc DoubleStopper () clobbers() {{\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }}\n\
         pub proc Releaser () clobbers() {{\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }}\n\
         pub proc Holder () clobbers() {{\n\
             with z80_stopped {{ nop }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        r.bus_firings.iter().filter(|f| f.proc != "Holder").count(),
        0,
        "entry must stay Unknown without a declaration: {:?}",
        r.bus_firings
    );
    // The bracket in `Holder` is what identifies `z80_stopped` as a bus-holding
    // context — asserted so the seed the NEXT test exercises cannot be vacuous.
    assert!(r.bus_contexts.contains("z80_stopped"), "bus contexts: {:?}", r.bus_contexts);
}

/// THE CLOSURE (entry edition). `requires(z80_stopped)` makes the entry state a
/// DECLARED fact — checked at every call site by `[context.unsatisfied]` — so the
/// SAME two bodies are analyzed from a definite `Held`: the re-stop is a
/// double-stop, and returning released breaks the contract the requirement states.
#[test]
fn a_declared_bus_requirement_seeds_the_inference_tier() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc DoubleStopper () clobbers() requires(z80_stopped) {{\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }}\n\
         pub proc Releaser () clobbers() requires(z80_stopped) {{\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }}\n\
         pub proc Holder () clobbers() {{\n\
             with z80_stopped {{ nop }}\n\
             rts\n\
         }}\n"
    ));
    assert!(r.bus_contexts.contains("z80_stopped"), "bus contexts: {:?}", r.bus_contexts);
    assert_eq!(
        bus_count(&r, "DoubleStopper", BusFiringKind::DoubleStop),
        1,
        "{:?}",
        r.bus_firings
    );
    assert_eq!(
        bus_count(&r, "Releaser", BusFiringKind::ReleasedAtReturn),
        1,
        "{:?}",
        r.bus_firings
    );
}

/// The seed must NOT re-polarise `[bus.stopped-at-return]` into a false positive:
/// a requiring proc that returns with the bus still held is behaving EXACTLY as
/// its contract says, and must stay silent. (Without this the declared seed would
/// fire on every correct adopter — the polarity the net does not take.)
#[test]
fn a_requiring_proc_that_returns_still_held_is_silent() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc Good () clobbers() requires(z80_stopped) {{\n\
             nop\n\
             rts\n\
         }}\n\
         pub proc Holder () clobbers() {{\n\
             with z80_stopped {{ nop }}\n\
             rts\n\
         }}\n"
    ));
    assert!(r.bus_contexts.contains("z80_stopped"), "bus contexts: {:?}", r.bus_contexts);
    assert_eq!(r.bus_firings.iter().filter(|f| f.proc == "Good").count(), 0, "{:?}", r.bus_firings);
    // …and the seed was actually applied: `Holder`'s bracket identified the
    // context, and `Good` declares it. Without this the silence is unearned.
    assert_eq!(r.context_claim_sites.len(), 1, "{:?}", r.context_claim_sites);
}

/// A GRANTED context never identifies as bus-holding (it splices no acquire), so
/// requiring one must NOT seed the bus net — the seed is keyed on what a bracket
/// EMITS, never on a name.
#[test]
fn a_granted_context_does_not_seed_the_bus_net() {
    let r = analyze(
        "module m\n\
         context vblank { granted }\n\
         pub proc P () clobbers() requires(vblank) {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert!(r.bus_contexts.is_empty(), "{:?}", r.bus_contexts);
    // The SECOND stop is a definite double-stop either way (the code made it
    // definite); what the granted context must not add is a HELD entry, which
    // would make the FIRST one fire too.
    assert_eq!(bus_count(&r, "P", BusFiringKind::DoubleStop), 1, "{:?}", r.bus_firings);
}

// ---------------------------------------------------------------------------
// The comptime gate
// ---------------------------------------------------------------------------

/// A FALSE gate lowers the body verbatim: no acquire, no release, no region — the
/// context genuinely is not held in that shape, so there is nothing to prove. The
/// same body under a TRUE gate brackets normally.
#[test]
fn a_false_gate_lowers_the_body_bare() {
    let off = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped if 0 {{ nop }}\n\
             rts\n\
         }}\n"
    ));
    assert!(off.context_regions.is_empty(), "{:?}", off.context_regions);
    assert!(off.bus_contexts.is_empty(), "{:?}", off.bus_contexts);

    let on = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped if 1 {{ nop }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(on.context_regions, vec![("P".to_string(), "z80_stopped".to_string())]);
}

/// A gated-OFF bracket still runs its body's own proofs: an inner bracket inside
/// the gated-off body is a real region. (A gate that skipped the body entirely
/// would silently drop code.)
#[test]
fn a_false_gate_still_lowers_nested_brackets() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped if 0 {{\n\
                 with z80_stopped {{ nop }}\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(r.context_regions, vec![("P".to_string(), "z80_stopped".to_string())]);
    assert!(r.context_firings.is_empty(), "{:?}", r.context_firings);
}

// ---------------------------------------------------------------------------
// Surface errors
// ---------------------------------------------------------------------------

/// A `with` on a GRANTED context is a steering error: there is no acquire/release
/// pair to bracket with.
#[test]
fn with_on_a_granted_context_is_an_error() {
    let diags = lower_diags(
        "module m\n\
         context vblank { granted }\n\
         section s {\n\
         pub proc P () clobbers() {\n\
             with vblank { nop }\n\
             rts\n\
         }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("[context.not-acquirable]")),
        "{diags:?}"
    );
}

/// A `with` naming no declared context is `[context.unknown]`.
#[test]
fn with_on_an_unknown_context_is_an_error() {
    let diags = lower_diags(
        "module m\n\
         section s {\n\
         pub proc P () clobbers() {\n\
             with nope { nop }\n\
             rts\n\
         }\n\
         }\n",
    );
    assert!(diags.iter().any(|d| d.message.contains("[context.unknown]")), "{diags:?}");
}

/// The per-file GATE — the site that actually fails a build — reports the escape
/// as an ERROR. `analyze_corpus` computes the same firings for the corpus gate;
/// this pins that the build-failing path is wired, not just the report.
#[test]
fn the_per_file_gate_reports_an_escape_as_an_error() {
    let diags = lower_diags(&format!(
        "module m\n{Z80_CTX}\
         section s {{\n\
         pub proc P () clobbers(d0) {{\n\
             with z80_stopped {{\n\
                 tst.w   d0\n\
                 bne     .skip\n\
             }}\n\
         .skip:\n\
             rts\n\
         }}\n\
         }}\n"
    ));
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[context.escape]")),
        "{diags:?}"
    );
}

/// Lower `src` and return every diagnostic (the build-failing path).
fn lower_diags(src: &str) -> Vec<sigil_span::Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(
        perrs.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse: {perrs:?}"
    );
    sigil_frontend_emp::lower::lower_module(
        &file,
        &sigil_frontend_emp::lower::LowerOptions {
            initial_cpu: sigil_ir::backend::Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    )
    .1
}

// ---------------------------------------------------------------------------
// Byte identity — the adoption guarantee
// ---------------------------------------------------------------------------

/// THE ADOPTION GUARANTEE: `with <ctx> { body }` emits EXACTLY the bytes the
/// manual `acquire ++ body ++ release` spelling emits. Corpus adoption is a
/// metadata change, and this is the unit-level statement of the ×7 byte bar the
/// parcel proves at ROM scale.
#[test]
fn a_bracket_emits_the_same_bytes_as_the_manual_pair() {
    // A self-contained context (a link-deferred `Z80_BUS_REQUEST` cannot resolve
    // in a standalone link); the bracket shape is the corpus's verbatim.
    let bracketed = linked(
        "module m\n\
         context z80_stopped {\n\
             acquire = asm { move.w #$0100, ($A11100).l\n\
                           .wait_z80:\n\
                             btst #0, ($A11100).l\n\
                             bne .wait_z80 }\n\
             release = asm { move.w #$0000, ($A11100).l }\n\
         }\n\
         section s {\n\
         pub proc P () clobbers(d0) {\n\
             with z80_stopped {\n\
                 moveq   #1, d0\n\
             }\n\
             rts\n\
         }\n\
         }\n",
    );
    let manual = linked(
        "module m\n\
         section s {\n\
         pub proc P () clobbers(d0) {\n\
             move.w  #$0100, ($A11100).l\n\
         .wait_z80:\n\
             btst    #0, ($A11100).l\n\
             bne     .wait_z80\n\
             moveq   #1, d0\n\
             move.w  #$0000, ($A11100).l\n\
             rts\n\
         }\n\
         }\n",
    );
    assert!(!manual.is_empty(), "the manual spelling produced no bytes");
    assert_eq!(bracketed, manual, "the bracket must be byte-identical to the manual pair");
}

/// Link `src`'s single section and return its bytes.
fn linked(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {perrs:?}");
    let (m, diags) = sigil_frontend_emp::lower::lower_module(
        &file,
        &sigil_frontend_emp::lower::LowerOptions {
            initial_cpu: sigil_ir::backend::Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lowering: {diags:?}"
    );
    let syms = sigil_ir::SymbolTable::new();
    let resolved =
        sigil_link::resolve_layout(&m.sections, &syms, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &syms).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

/// A loop label written immediately BEFORE a bracket resolves to the region's
/// FIRST instruction (a label targets the first instruction at or after it), and
/// branching back to it re-takes the whole acquire — not a skip. This is the
/// corpus's spin-probe idiom (`.await_slot:` + `with z80_stopped { … }` +
/// `bne .await_slot`); the rule is narrowed to targets PAST the acquire so that
/// idiom does not fire.
#[test]
fn a_loop_label_at_the_region_head_is_not_an_entry_skip() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0-d1) {{\n\
         .probe:\n\
             with z80_stopped {{\n\
                 move.b  d0, d1\n\
             }}\n\
             tst.b   d1\n\
             bne     .probe\n\
             rts\n\
         }}\n"
    ));
    assert!(r.context_firings.is_empty(), "{:?}", r.context_firings);
    assert_eq!(r.context_regions.len(), 1, "{:?}", r.context_regions);
}

// ---------------------------------------------------------------------------
// Panel regressions — each pins a hazard a lens constructed
// ---------------------------------------------------------------------------

/// A back-edge from the BODY into the acquire re-runs it with no matching
/// release. For `ints_off` that is an unbounded `move.w sr,-(sp)` stack leak;
/// for a bus context it is a re-request while held. The distinguishing fact
/// against the legitimate spin-probe above is where the edge STARTS: inside the
/// region, not after it.
#[test]
fn a_back_edge_into_the_acquire_is_a_reacquire() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0) {{\n\
         .spin:\n\
             with z80_stopped {{\n\
                 tst.b   d0\n\
                 bne     .spin\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "P", ContextFiringKind::Reacquire),
        1,
        "{:?}",
        r.context_firings
    );
}

/// The acquire's OWN internal spin (`bne .wait_z80`) branches into the acquire
/// range and must NOT read as a reacquire — otherwise every bracket of the
/// engine's bus context fires on itself.
#[test]
fn the_acquires_own_spin_is_not_a_reacquire() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{ nop }}\n\
             rts\n\
         }}\n"
    ));
    assert!(r.context_firings.is_empty(), "{:?}", r.context_firings);
    assert_eq!(r.context_regions.len(), 1, "{:?}", r.context_regions);
}

/// `bsr` is a CALL — control returns — so it is not a path out of the region.
/// The shared CFG classifies it as a conditional branch on mnemonic shape
/// (`b` + 3 chars), which would otherwise read an ordinary call as an escape.
#[test]
fn a_bsr_out_of_the_region_is_not_an_escape() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{\n\
                 bsr     Elsewhere\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Escape), 0, "{:?}", r.context_firings);
    assert_eq!(r.context_regions.len(), 1, "{:?}", r.context_regions);
}

/// An instruction that merely NAMES a local label in a data position is not a
/// branch into the region. The entry-skip scan reads "the last `Sym` operand",
/// so without a mnemonic gate a `lea .inner(pc), a0` would fire with a message
/// about a branch that does not exist.
#[test]
fn naming_an_in_region_label_in_a_data_operand_is_not_an_entry_skip() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(a0) {{\n\
             lea     .inner, a0\n\
             with z80_stopped {{\n\
                 nop\n\
             .inner:\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::EntrySkip), 0, "{:?}", r.context_firings);
    assert_eq!(r.context_regions.len(), 1, "{:?}", r.context_regions);
}

/// An EXPORTED label inside a region is an entry point this proc's item list
/// cannot see — it takes the stable `Owner.name` symbol, so any other proc can
/// branch straight past the acquire.
#[test]
fn an_exported_label_inside_the_region_is_an_entry_skip() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers() {{\n\
             with z80_stopped {{\n\
                 nop\n\
             export .mid:\n\
                 nop\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::EntrySkip), 1, "{:?}", r.context_firings);
}

/// `grants` of an ACQUIRED context is rejected — the mirror of `with` on a
/// granted one. Without this, the obvious spelling of "this proc establishes the
/// context" seeds the bus net from an unverifiable claim and makes the proc's own
/// compiler-generated acquire read as a double-take.
#[test]
fn granting_an_acquired_context_is_an_error() {
    let diags = lower_diags(&format!(
        "module m\n{Z80_CTX}\
         section s {{\n\
         pub proc P () clobbers() grants(z80_stopped) {{\n\
             with z80_stopped {{ nop }}\n\
             rts\n\
         }}\n\
         }}\n"
    ));
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[context.not-grantable]")),
        "{diags:?}"
    );
}

/// A `requires`/`grants` naming no declared context FAILS THE BUILD, not merely
/// the report: a silently-ignored requirement reads as a checked claim.
#[test]
fn an_undeclared_context_clause_fails_the_per_file_gate() {
    let diags = lower_diags(
        "module m\n\
         section s {\n\
         pub proc P () clobbers() requires(no_such_ctx) { rts }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[context.unknown]")),
        "{diags:?}"
    );
}

/// A context is identified as bus-holding by what its ACQUIRE splices, not by
/// what its body happens to contain. The corpus nests `with ints_off { with
/// z80_stopped { … } }`, so reading the body would make `ints_off` a bus context
/// and hand the first `requires(ints_off)` a bogus held entry — silencing
/// `[bus.vdp-write-unstopped]`, the crash class, for that whole proc.
#[test]
fn an_outer_bracket_nesting_a_bus_bracket_is_not_a_bus_context() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         context ints_off {{\n\
             acquire = asm {{ move.w sr, -(sp)\n move.w #$2700, sr }}\n\
             release = asm {{ move.w (sp)+, sr }}\n\
         }}\n\
         pub proc P () clobbers() preserves(sr) {{\n\
             with ints_off {{\n\
                 with z80_stopped {{ nop }}\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(r.context_regions.len(), 2, "{:?}", r.context_regions);
    assert!(r.bus_contexts.contains("z80_stopped"), "{:?}", r.bus_contexts);
    assert!(
        !r.bus_contexts.contains("ints_off"),
        "the OUTER bracket must not inherit the inner acquire: {:?}",
        r.bus_contexts
    );
}

/// The requirement gate's companion census: a discharged call site is RECORDED,
/// so an assert-empty over `context_unsatisfied` can be told apart from a walk
/// that examined no call site at all.
#[test]
fn a_discharged_requirement_is_censused() {
    let r = analyze(
        "module m\n\
         context vblank { granted }\n\
         proc Callee () clobbers() requires(vblank) { rts }\n\
         pub proc Root () clobbers() grants(vblank) { jbsr Callee\n rts }\n",
    );
    assert!(r.context_unsatisfied.is_empty(), "{:?}", r.context_unsatisfied);
    assert_eq!(
        r.context_discharged,
        vec![("Root".to_string(), "Callee".to_string(), "vblank".to_string())]
    );
}

// ---------------------------------------------------------------------------
// THE RTE FLAVOR — `released_by_rte` (§3.1's third context body)
//
// An interrupt handler that raises the mask for its body does not need to lower
// it again: `rte` reloads the whole SR from the frame the CPU pushed at entry, so
// the ordinary flavor's `move.w (sp)+, sr` restore writes a value the very next
// instruction discards. The flavor lets the release be NAMED rather than spelled
// — and then proves it is reached, because "the hardware releases this" is only
// true where control actually gets to an `rte`.
//
// Every test below asks one question about that proof. The pair that matters most
// is [`an_rte_released_bracket_ending_in_rte_is_clean`] (the shipping shape,
// silent) against [`an_rte_released_bracket_that_returns_with_rts_fires`] (the
// same bracket in a proc that returns the ordinary way, refused): a flavor that
// only ever went quiet would be a hole, not a feature.
// ---------------------------------------------------------------------------

/// Aeon's real flavored context, inline (same reason `Z80_CTX` is spelled out).
const RTE_CTX: &str = "context ints_off_until_rte {\n\
     \x20   acquire = asm { move.w #$2700, sr }\n\
     \x20   released_by_rte\n\
     }\n";

/// THE SHIPPING SHAPE IS SILENT. `Raster_HInt`'s exact structure: raise the mask,
/// save registers, work, restore registers, close the bracket, `rte`. No release
/// is spliced and none is missing — the `rte` immediately after the bracket IS
/// the release, and the checker finds it.
#[test]
fn an_rte_released_bracket_ending_in_rte_is_clean() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         pub proc H () clobbers() {{\n\
             with ints_off_until_rte {{\n\
                 move.w  #$8A00, VDP_CTRL\n\
             }}\n\
             rte\n\
         }}\n"
    ));
    assert_eq!(
        r.context_firings.iter().filter(|f| f.proc == "H").count(),
        0,
        "the flavored bracket's own shape must be clean: {:?}",
        r.context_firings
    );
    assert!(
        r.context_regions.iter().any(|(p, c)| p == "H" && c == "ints_off_until_rte"),
        "…and NOT because no region was recovered, the silence has to be a PASS: {:?}",
        r.context_regions
    );
}

/// THE POISON, AND THE WHOLE REASON THE FLAVOR IS A MECHANISM RATHER THAN A
/// WAIVER. The identical bracket in a proc that returns with `rts`: the mask is
/// raised and never lowered, because `rts` does not touch SR.
#[test]
fn an_rte_released_bracket_that_returns_with_rts_fires() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         pub proc H () clobbers() {{\n\
             with ints_off_until_rte {{\n\
                 move.w  #$8A00, VDP_CTRL\n\
             }}\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "H", ContextFiringKind::RteUndischarged),
        1,
        "an `rts` exit must fire `[context.rte-undischarged]`: {:?}",
        r.context_firings
    );
    assert_eq!(
        ctx_count(&r, "H", ContextFiringKind::Escape),
        0,
        "…and NOT as a plain escape, the two say different things: {:?}",
        r.context_firings
    );
}

/// `rtr` IS NOT `rte`, and the distinction is the flavor's whole premise. `rtr`
/// restores the CCR half from the stack and leaves the interrupt MASK exactly
/// where the handler left it — so a hold "discharged" by `rtr` is not discharged.
#[test]
fn an_rtr_exit_does_not_discharge_an_rte_released_hold() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         pub proc H () clobbers() {{\n\
             with ints_off_until_rte {{\n\
                 move.w  #$8A00, VDP_CTRL\n\
             }}\n\
             rtr\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "H", ContextFiringKind::RteUndischarged),
        1,
        "`rtr` restores CCR only, the mask stays raised: {:?}",
        r.context_firings
    );
}

/// AN INSTRUCTION BETWEEN THE BRACKET AND THE `rte` IS THE HOLD LEAKING, and it is
/// the mis-edit the flavor invites: close the bracket, do "one more thing", then
/// return. That one more thing runs at the raised mask the author believes the
/// bracket ended.
#[test]
fn work_between_an_rte_released_bracket_and_its_rte_fires() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         pub proc H () clobbers(d0) {{\n\
             with ints_off_until_rte {{\n\
                 move.w  #$8A00, VDP_CTRL\n\
             }}\n\
             moveq   #0, d0\n\
             rte\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "H", ContextFiringKind::RteUndischarged),
        1,
        "the fall-out must land ON the rte, not near it: {:?}",
        r.context_firings
    );
}

/// AN `rte` INSIDE THE BODY IS THE RELEASE TAKEN EARLY — legal, and the only
/// return that is. A multi-exit handler is the shape this permits, and it is the
/// shape the ordinary flavor structurally cannot have (one lexical region, one
/// spliced exit — `engine/irq.emp`'s header lists multi-exit brackets among the
/// sites that stay hand-spelled).
#[test]
fn an_rte_inside_an_rte_released_body_is_the_release() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         pub proc H () clobbers(d0) {{\n\
             with ints_off_until_rte {{\n\
                 tst.w   d0\n\
                 bne     .slow\n\
                 rte\n\
             .slow:\n\
                 move.w  #$8A00, VDP_CTRL\n\
             }}\n\
             rte\n\
         }}\n"
    ));
    assert_eq!(
        r.context_firings.iter().filter(|f| f.proc == "H").count(),
        0,
        "an in-body `rte` discharges its own path: {:?}",
        r.context_firings
    );
}

/// AN `rts` INSIDE THE BODY fires, on the same rule read the other way — the
/// early-exit arm written wrong, which is not visible from the closing brace.
#[test]
fn an_rts_inside_an_rte_released_body_fires() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         pub proc H () clobbers(d0) {{\n\
             with ints_off_until_rte {{\n\
                 tst.w   d0\n\
                 bne     .done\n\
                 rts\n\
             .done:\n\
                 move.w  #$8A00, VDP_CTRL\n\
             }}\n\
             rte\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "H", ContextFiringKind::RteUndischarged),
        1,
        "an in-body `rts` leaves the mask raised in the caller: {:?}",
        r.context_firings
    );
}

/// A TAIL TRANSFER OUT still fires. The hold would travel into the callee and come
/// back through ITS return, which this proc cannot see and cannot promise is an
/// `rte`.
#[test]
fn a_tail_transfer_out_of_an_rte_released_body_fires() {
    let r = analyze(&format!(
        "module m\n{RTE_CTX}\
         proc Elsewhere () clobbers() {{ rts }}\n\
         pub proc H () clobbers() {{\n\
             with ints_off_until_rte {{\n\
                 jbra    Elsewhere\n\
             }}\n\
             rte\n\
         }}\n"
    ));
    assert_eq!(
        ctx_count(&r, "H", ContextFiringKind::RteUndischarged),
        1,
        "a tail transfer carries the hold out of the proc: {:?}",
        r.context_firings
    );
}

/// THE ORDINARY FLAVOR IS UNTOUCHED — a bracket with a spliced release still fires
/// `Escape`, not the new kind. The rte rules are gated on the FLAVOR and not on
/// the presence of an `rte` anywhere, and this is what says so.
#[test]
fn the_ordinary_flavor_still_fires_plain_escape() {
    let r = analyze(&format!(
        "module m\n{Z80_CTX}\
         pub proc P () clobbers(d0) {{\n\
             with z80_stopped {{\n\
                 tst.w   d0\n\
                 bne     .skip\n\
             }}\n\
         .skip:\n\
             rts\n\
         }}\n"
    ));
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::Escape), 1, "{:?}", r.context_firings);
    assert_eq!(ctx_count(&r, "P", ContextFiringKind::RteUndischarged), 0, "{:?}", r.context_firings);
}

// ---- the DECLARATION-site rules -------------------------------------------

/// `release = …` AND `released_by_rte` is two answers to one question. Refused at
/// the decl, which is where the author can see both words.
#[test]
fn naming_both_release_and_released_by_rte_is_a_decl_error() {
    let (_f, diags) = sigil_frontend_emp::parse_str(
        "module m\n\
         context c {\n\
             acquire = asm { move.w #$2700, sr }\n\
             release = asm { move.w #$2000, sr }\n\
             released_by_rte\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("released_by_rte")),
        "expected a decl diagnostic naming the conflict: {diags:?}"
    );
}

/// `granted` AND `released_by_rte` is meaningless — a granted context has no
/// bracket, so there is no hold for an `rte` to end.
#[test]
fn granted_and_released_by_rte_is_a_decl_error() {
    let (_f, diags) = sigil_frontend_emp::parse_str(
        "module m\n\
         context c {\n\
             granted\n\
             released_by_rte\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("released_by_rte")),
        "expected a decl diagnostic: {diags:?}"
    );
}

/// `released_by_rte` WITHOUT an acquire is still the missing-half error — the
/// flavor replaces the release, never the acquire.
#[test]
fn released_by_rte_without_an_acquire_is_a_decl_error() {
    let (_f, diags) = sigil_frontend_emp::parse_str("module m\ncontext c {\n released_by_rte\n}\n");
    assert!(
        diags.iter().any(|d| d.message.contains("acquire")),
        "expected the missing-acquire error: {diags:?}"
    );
}
