//! Contract-grammar v2 — the whole-corpus contract walk end to end: parse
//! synthetic `.emp` modules → `analyze_corpus` → closure + firings. Exercises
//! the AST→ProcNode wiring the pure closure tests can't (call-edge extraction,
//! indirect-site bounds, extern leaves, the §11 Q4 collision).

use sigil_frontend_emp::corpus_contracts::analyze_corpus;
use sigil_frontend_emp::parse_str;

/// Parse each source into a `File` (demanding clean parse) and analyze.
fn analyze(srcs: &[&str]) -> sigil_frontend_emp::corpus_contracts::ContractReport {
    let files: Vec<_> = srcs
        .iter()
        .map(|s| {
            let (f, diags) = parse_str(s);
            assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
            f
        })
        .collect();
    analyze_corpus(&files)
}

/// A firing on `(proc, reg)` is present.
fn fires(r: &sigil_frontend_emp::corpus_contracts::ContractReport, proc: &str, reg: &str) -> bool {
    r.firings.iter().any(|f| f.proc == proc && f.reg.as_deref() == Some(reg))
}

/// A proc that writes a register outside its declared `clobbers` fires (the
/// transitive lint subsumes the local one for a direct write).
#[test]
fn direct_under_declaration_fires_over_corpus() {
    let r = analyze(&[
        "module m\nproc P () clobbers(d0) {\n moveq #0, d0\n moveq #1, d7\n rts }\n",
    ]);
    assert!(fires(&r, "P", "d7"), "firings: {:?}", r.firings);
    assert!(!fires(&r, "P", "d0"), "d0 is declared, must not fire");
}

/// The number of `[call.flag-result-unused]` firings on `proc` calling `callee`.
fn flag_fires(
    r: &sigil_frontend_emp::corpus_contracts::ContractReport,
    proc: &str,
    callee: &str,
) -> usize {
    r.flag_firings.iter().filter(|f| f.proc == proc && f.callee == callee).count()
}

/// An extern declaring `out(carry: dropped)` whose caller CONSUMES the carry
/// (`bcs`) produces no flag firing — the wired end-to-end happy path (contract
/// from a decl, CFG over the caller's evaluated body).
#[test]
fn flag_result_consumed_over_corpus_passes() {
    let r = analyze(&[
        "module m\n\
         extern proc Queue (d1) clobbers(d0) out(carry: dropped)\n\
         proc Caller () clobbers(d0-d1) {\n\
             jbsr Queue\n\
             bcs .done\n\
             moveq #0, d0\n\
         .done:\n\
             rts\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Queue"), 0, "flag firings: {:?}", r.flag_firings);
}

/// The same extern whose caller DROPS the carry (overwrites CC and returns)
/// fires `[call.flag-result-unused]` — the Palette_Dirty / load_art bug class,
/// caught through the whole-corpus wiring.
#[test]
fn flag_result_dropped_over_corpus_fires() {
    let r = analyze(&[
        "module m\n\
         extern proc Queue (d1) clobbers(d0) out(carry: dropped)\n\
         proc Caller () clobbers(d0-d1) {\n\
             jbsr Queue\n\
             moveq #0, d0\n\
             rts\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Queue"), 1, "flag firings: {:?}", r.flag_firings);
}

/// `@discards(dropped)` on the call is the explicit opt-out — no firing even
/// though the carry is dropped.
#[test]
fn flag_result_discarded_over_corpus_passes() {
    let r = analyze(&[
        "module m\n\
         extern proc Queue (d1) clobbers(d0) out(carry: dropped)\n\
         proc Caller () clobbers(d0-d1) {\n\
             jbsr Queue @discards(dropped)\n\
             moveq #0, d0\n\
             rts\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Queue"), 0, "flag firings: {:?}", r.flag_firings);
}

/// A caller with a tight contract that CALLS a scribbler is charged the
/// scribbler's writes transitively (the whole point of §1).
#[test]
fn transitive_callee_leak_fires_over_corpus() {
    let r = analyze(&[
        "module m\n\
         proc Caller () clobbers(d0) {\n moveq #0, d0\n jbsr Scribbler\n rts }\n\
         proc Scribbler () clobbers(d1) {\n moveq #1, d1\n rts }\n",
    ]);
    // Caller declares only d0 but transitively clobbers d1 via Scribbler.
    assert!(fires(&r, "Caller", "d1"), "firings: {:?}", r.firings);
}

/// An `extern proc` leaf charges its declared clobbers to its callers (§3): a
/// caller of `VSync_Wait () clobbers(d0)` that declares `clobbers()` fires d0.
#[test]
fn extern_leaf_charges_callers() {
    let r = analyze(&[
        "module m\n\
         extern proc VSync_Wait () clobbers(d0)\n\
         proc Frame () clobbers() {\n jbsr VSync_Wait\n rts }\n",
    ]);
    assert!(fires(&r, "Frame", "d0"), "firings: {:?}", r.firings);
    assert_eq!(r.extern_count, 1);
}

/// A BOUNDED indirect dispatch (`jsr (a1) as HBlankHandler`) charges only the
/// bound's clobbers — NOT ⊤ — so a proc declaring exactly that set does not fire.
#[test]
fn bounded_indirect_is_not_top() {
    let r = analyze(&[
        "module m\n\
         type HBlankHandler = proc () clobbers(d0, d1, a0)\n\
         proc Dispatch () clobbers(d0, d1, a0) {\n jsr (a1) as HBlankHandler\n rts }\n",
    ]);
    assert!(r.firings.is_empty(), "bounded dispatch should not fire: {:?}", r.firings);
    assert_eq!(r.contract_type_count, 1);
}

/// An UNBOUNDED indirect dispatch (`jsr (a1)` with no `as`) makes the proc's
/// effect ⊤ — a bounded `clobbers` contract on it fires unbounded.
#[test]
fn unbounded_indirect_fires_unbounded() {
    let r = analyze(&[
        "module m\nproc Dispatch () clobbers(d0) {\n jsr (a1)\n rts }\n",
    ]);
    assert!(
        r.firings.iter().any(|f| f.proc == "Dispatch" && f.unbounded),
        "firings: {:?}",
        r.firings
    );
}

/// S2-D6 U2 — the DIRECT `jsr Foo` / `bsr Foo` call edge (companion to the
/// `jbsr` edge in `transitive_callee_leak_fires_over_corpus`): a plain `jsr` to a
/// resolvable proc symbol unions the callee's declared write surface into the
/// caller's effective set. `bsr` is the same edge (all of jsr/jbsr/bsr resolve).
#[test]
fn direct_jsr_and_bsr_call_edges_propagate() {
    let r = analyze(&[
        "module m\n\
         proc ViaJsr () clobbers(d0) {\n moveq #0, d0\n jsr Scribbler\n rts }\n\
         proc ViaBsr () clobbers(d0) {\n moveq #0, d0\n bsr Scribbler\n rts }\n\
         proc Scribbler () clobbers(d1) {\n moveq #1, d1\n rts }\n",
    ]);
    assert!(fires(&r, "ViaJsr", "d1"), "direct jsr edge must charge d1: {:?}", r.firings);
    assert!(fires(&r, "ViaBsr", "d1"), "bsr edge must charge d1: {:?}", r.firings);
}

// S2-D6 U2 — the "contract invoke" call edge. `invoke Iface.hook` lowers to an
// absolute-long `jsr (sym).l` (proven in the `game_contract` tests:
// `bound_hook_emits_absolute_jsr`), which then propagates its target's clobbers
// through the SAME direct-call path proven by `direct_jsr_and_bsr_call_edges`.
// It is NOT exercised end-to-end here: the corpus contract walk uses the EMPTY
// interface env, under which an `invoke` emits nothing, so there is no edge in
// the gate to charge — and the corpus's only abs-long calls are the contract-less
// vendored debugger entries. Wiring the invoke edge into the closure (env-threaded
// + resolvable-vs-⊤ abs-long handling) is L1 game-contract-seam work. See the
// scope note on `call_target_sym` in corpus_contracts.rs.

/// S2-D6 U4 — the `@allow("clobbers.unanalyzable", "<reason>")` escape hatch: a
/// genuinely-unanalyzable computed dispatch opts OUT of the `unbounded` firing.
/// The companion `unbounded_indirect_fires_unbounded` proves the SAME site fires
/// WITHOUT the annotation, so this pins the suppression is load-bearing (not a
/// site that never fired).
#[test]
fn unanalyzable_allow_suppresses_the_unbounded_firing() {
    let r = analyze(&[
        "module m\n\
         @allow(\"clobbers.unanalyzable\", \"raw trampoline: target set is open by design\")\n\
         proc Dispatch () clobbers(d0) {\n jsr (a1)\n rts }\n",
    ]);
    assert!(
        !r.firings.iter().any(|f| f.proc == "Dispatch" && f.unbounded),
        "the U4 allow must suppress the unbounded firing: {:?}",
        r.firings
    );
    // And the annotation is LISTED in force (audited, never silent).
    assert!(
        r.unanalyzable_allows.iter().any(|(p, reason)| p == "Dispatch" && !reason.is_empty()),
        "the in-force annotation must be listed: {:?}",
        r.unanalyzable_allows
    );
}

/// S2-D6 U4 — the hatch suppresses ONLY the ⊤/unbounded case, never a CONCRETE
/// register under-declaration. A proc that both makes an unbounded call AND
/// directly writes an undeclared register still fires the concrete register.
#[test]
fn unanalyzable_allow_does_not_silence_concrete_leak() {
    let r = analyze(&[
        "module m\n\
         @allow(\"clobbers.unanalyzable\", \"open trampoline\")\n\
         proc Dispatch () clobbers(d0) {\n moveq #0, d7\n jsr (a1)\n rts }\n",
    ]);
    // The unbounded firing is suppressed, but d7 (a concrete direct write) is not
    // — except a ⊤ effective set has no named regs, so the concrete write is
    // subsumed by ⊤. To prove the concrete path independently, use a bounded call
    // so the effect is a named set, not ⊤:
    let r2 = analyze(&[
        "module m\n\
         @allow(\"clobbers.unanalyzable\", \"open trampoline\")\n\
         proc P () clobbers(d0) {\n moveq #0, d7\n rts }\n",
    ]);
    let _ = r; // (the ⊤ case is covered by the suppression test above)
    assert!(
        fires(&r2, "P", "d7"),
        "a concrete undeclared write must still fire despite the allow: {:?}",
        r2.firings
    );
}

/// Does the caller's D1c `[call.live-clobbered]` fire on `(proc, callee, reg)`?
fn live_clobbered(
    r: &sigil_frontend_emp::corpus_contracts::ContractReport,
    proc: &str,
    callee: &str,
    reg: &str,
) -> bool {
    r.live_clobbered_firings
        .iter()
        .any(|f| f.proc == proc && f.callee == callee && f.reg == reg)
}

/// S2-D6 ACCEPTANCE BAR 3 — THE DOCTORED-a5 NEGATIVE PROBE. The sweep's
/// load-bearing example, proven fireable: a synthetic of the tile_cache a5/a6
/// hoist chain. `TileCache_FillColumn` hoists `a5` (the Nametable base) OUT of
/// the loop and relies on it SURVIVING each `TileCache_DecompressBlock` call —
/// which is only safe because DecompressBlock's license (`d0-d7/a0/a2-a4`)
/// EXCLUDES a5. If S4LZ_DecompressDict (DecompressBlock's inner callee) ever
/// gained an undeclared a5 write, the hoisted a5 would be corrupted between the
/// set and the read — silent collision-plane corruption. The byte gate cannot
/// see this; this lint MUST. The DOCTORED version fires the CALLER's D1c; the
/// undoctored CONTROL is clean.
#[test]
fn doctored_a5_write_fails_the_callers_lint_the_negative_probe() {
    // DOCTORED: DecompressBlock's inner S4LZ callee secretly writes a5 (its
    // declared license excludes a5/a6). The caller hoisted a5 and reads it after
    // the call → D1c live-clobbered on the caller.
    let doctored = "module engine.tile_cache\n\
        proc FillColumn () clobbers(d0-d7/a0-a6) {\n\
        \x20   lea Tile_Cache_Nametable, a5\n\
        \x20   jbsr DecompressBlock\n\
        \x20   movea.l a5, a2\n\
        \x20   move.l (a2), d0\n\
        \x20   rts\n}\n\
        proc DecompressBlock () clobbers(d0-d7/a0/a2-a4) {\n\
        \x20   jbsr S4LZ_DecompressDict\n\
        \x20   rts\n}\n\
        proc S4LZ_DecompressDict () clobbers(d0-d3) {\n\
        \x20   lea Garbage, a5\n\
        \x20   rts\n}\n";
    let r = analyze(&[doctored]);
    // The CALLER's lint fails: a5 is live across DecompressBlock (which now
    // transitively clobbers a5 via the doctored S4LZ) and read after.
    assert!(
        live_clobbered(&r, "FillColumn", "DecompressBlock", "a5"),
        "the doctored a5 write MUST fail the caller's D1c lint: live={:?} firings={:?}",
        r.live_clobbered_firings,
        r.firings
    );
    // And the doctored callee's OWN lint fires the a5 under-declaration too (the
    // transitive closure catches the leak into DecompressBlock's contract).
    assert!(
        fires(&r, "S4LZ_DecompressDict", "a5"),
        "the doctored S4LZ must fire clobber-undeclared on a5: {:?}",
        r.firings
    );

    // CONTROL: S4LZ preserves a5 (does not touch it). The hoist is safe again —
    // the caller's D1c is silent and no a5 leak anywhere. This proves the probe
    // is meaningful (not a site that always fires).
    let control = "module engine.tile_cache\n\
        proc FillColumn () clobbers(d0-d7/a0-a6) {\n\
        \x20   lea Tile_Cache_Nametable, a5\n\
        \x20   jbsr DecompressBlock\n\
        \x20   movea.l a5, a2\n\
        \x20   move.l (a2), d0\n\
        \x20   rts\n}\n\
        proc DecompressBlock () clobbers(d0-d7/a0/a2-a4) {\n\
        \x20   jbsr S4LZ_DecompressDict\n\
        \x20   rts\n}\n\
        proc S4LZ_DecompressDict () clobbers(d0-d3) {\n\
        \x20   moveq #0, d0\n\
        \x20   rts\n}\n";
    let rc = analyze(&[control]);
    assert!(
        !live_clobbered(&rc, "FillColumn", "DecompressBlock", "a5"),
        "the control (a5 preserved) must NOT fire the caller's D1c: {:?}",
        rc.live_clobbered_firings
    );
    assert!(
        !fires(&rc, "S4LZ_DecompressDict", "a5"),
        "the control must not fire an a5 under-declaration: {:?}",
        rc.firings
    );
}

/// A preserves-only contract type bounds clobbers to everything-not-preserved:
/// `ObjRoutine preserves(a0, d7)` lets a target clobber the rest, so a dispatcher
/// declaring the full register file minus nothing does not fire, but a0/d7 stay
/// protected (not charged).
#[test]
fn preserves_only_type_bounds_complement() {
    let r = analyze(&[
        "module m\n\
         type ObjRoutine = proc (a0: *Sst) preserves(a0, d7)\n\
         proc Run () clobbers(d0-d6/a1-a6) {\n jsr (a1) as ObjRoutine\n rts }\n",
    ]);
    // a0 and d7 are preserved by the bound, so they are never charged; the rest
    // (d0-d6/a1-a6) is exactly declared → no firing.
    assert!(r.firings.is_empty(), "firings: {:?}", r.firings);
}

/// A name declared BOTH `extern proc` and `proc` collides (§11 Q4).
#[test]
fn extern_proc_collision_flagged() {
    let r = analyze(&[
        "module a\nextern proc Shared () clobbers(d0)\n",
        "module b\nproc Shared () clobbers(d0) {\n rts }\n",
    ]);
    assert!(
        r.extern_collisions.iter().any(|(n, _)| n == "Shared"),
        "collisions: {:?}",
        r.extern_collisions
    );
}

/// An `extern proc` with an `out` register charges that register to its callers
/// too — an out result is WRITTEN by the callee (the S4LZ in-out cursor case),
/// so a caller relying on it across the call is wrong. The extern leaf's
/// effective set is clobbers ∪ out.
#[test]
fn extern_out_register_charges_callers() {
    let r = analyze(&[
        "module m\n\
         extern proc Decompress (a0, a1) clobbers(d0) out(a1)\n\
         proc Caller () clobbers(d0) {\n jbsr Decompress\n rts }\n",
    ]);
    // Caller is charged a1 (the extern's out cursor) but declares only d0.
    assert!(fires(&r, "Caller", "a1"), "firings: {:?}", r.firings);
}

/// Declared + movem-VERIFIED preserves is subtracted (the D2.32 fast path): a
/// proc that writes a0/a1 but movem-saves/restores them, declaring
/// preserves(a0/a1), fires nothing — the registers do not escape it.
#[test]
fn declared_verified_preserves_subtracts_over_corpus() {
    let r = analyze(&[
        "module m\n\
         proc P () clobbers() preserves(a0/a1) {\n\
             movem.l a0-a1, -(sp)\n\
             lea Foo, a0\n\
             lea Bar, a1\n\
             movem.l (sp)+, a0-a1\n\
             rts }\n",
    ]);
    assert!(r.firings.is_empty(), "verified preserves must subtract: {:?}", r.firings);
}

/// Declared but UNVERIFIABLE preserves does NOT subtract: an individual-push
/// save (no movem pair) leaves the D2.32 slice unable to prove preservation, so
/// the register stays in `effective` and fires (and the declared preserves is
/// itself a D2.32 error at its own site — subtracting on an unproven claim would
/// be unsound). This is the row-1030 individual-push class → G3.
#[test]
fn verified_individual_push_preserves_subtracts() {
    // The G3 upgrade (§5): individual-push preserves is now VERIFIABLE, so a0 is
    // subtracted from `effective` and does NOT fire — the AllocDynamic shape. (In
    // G1/G2 the D2.32 movem-only slice could not verify this; it fired.)
    let r = analyze(&[
        "module m\n\
         proc P () clobbers() preserves(a0) {\n\
             move.l a0, -(sp)\n\
             lea Foo, a0\n\
             movea.l (sp)+, a0\n\
             rts }\n",
    ]);
    assert!(
        !fires(&r, "P", "a0"),
        "individual-push preserves is §5-verified → a0 subtracted → must NOT fire: {:?}",
        r.firings
    );
}

#[test]
fn genuinely_unverifiable_preserves_does_not_subtract() {
    // A declared preserves whose proof BAILS (computed sp) is unverifiable → not
    // subtracted → a0 still fires. (A wrong contract earns [proc.preserves-
    // unverifiable] at lowering; the closure conservatively keeps a0.)
    let r = analyze(&[
        "module m\n\
         proc P () clobbers() preserves(a0) {\n\
             move.l a0, -(sp)\n\
             lea Foo, a0\n\
             adda.w #4, sp\n\
             movea.l (sp)+, a0\n\
             rts }\n",
    ]);
    assert!(
        fires(&r, "P", "a0"),
        "an unverifiable (bailed) preserves must NOT subtract → a0 fires: {:?}",
        r.firings
    );
}

// === D1b WARN→ERROR flip — the gate's teeth, hermetic ======================

/// A `[call.input-undefined]` firing on `(proc, callee, reg)` is present.
fn input_fires(
    r: &sigil_frontend_emp::corpus_contracts::ContractReport,
    proc: &str,
    callee: &str,
    reg: &str,
) -> bool {
    r.input_firings
        .iter()
        .any(|f| f.proc == proc && f.callee == callee && f.reg == reg)
}

/// FLIP-GATE RED-TEST (brief §2.6): a caller invoking a callee whose register
/// param is UNDEFINED on the path produces a `[call.input-undefined]` firing — so
/// the ERROR gate (`input_firings` empty) would REJECT it. This is the synthetic
/// undefined-input the corpus gate is a permanent absence-of.
#[test]
fn flip_gate_rejects_undefined_input() {
    let r = analyze(&[
        "module m\n\
         proc Callee (d0: u16) clobbers() { rts }\n\
         proc Caller () clobbers() {\n\
             jbsr Callee\n\
             rts }\n",
    ]);
    assert!(
        input_fires(&r, "Caller", "Callee", "d0"),
        "d0 is Callee's param, undefined in Caller → D1b must fire: {:?}",
        r.input_firings
    );
}

/// VERIFIED-CREDIT IS LOAD-BEARING (brief §2.2, the FindStagedBlock shape as a
/// permanent regression): `Liar` DECLARES `out(d0)` but only produces d0 on one
/// return (the `.skip` path leaves it unproduced — an existence-lie). `Consumer`
/// relies on that out to define d0 for a later `jbsr NeedsD0`. Under VERIFIED
/// credit the lie is NOT credited ⇒ d0 undefined at NeedsD0 ⇒ D1b FIRES. MUTATION:
/// reverting D1b to DECLARED credit (crediting the unverified out) suppresses this
/// firing — so it pins the flip's whole point.
#[test]
fn flip_gate_verified_credit_is_load_bearing() {
    let r = analyze(&[
        "module m\n\
         proc Liar (d1: u16) clobbers(d3) out(d0) {\n\
             cmp.w #0, d1\n\
             beq .skip\n\
             move.l d1, d0\n\
             rts\n\
         .skip:\n\
             rts }\n\
         proc NeedsD0 (d0: u16) clobbers() { rts }\n\
         proc Consumer () clobbers(d1/d3) {\n\
             moveq #0, d1\n\
             jbsr Liar\n\
             jbsr NeedsD0\n\
             rts }\n",
    ]);
    assert!(
        input_fires(&r, "Consumer", "NeedsD0", "d0"),
        "Liar's out(d0) is an existence-lie (unproduced on .skip) → verified credit \
         withholds it → d0 undefined at NeedsD0 → D1b fires: {:?}",
        r.input_firings
    );
}

// ===========================================================================
// §5 — the CALLEE-PRESERVES ORACLE + the defer→closure final-authority split
// (t30). The `TestChurnObj_Main` shape: a caller save/restores a register then
// makes a trailing call. The per-file byte gate DEFERS; the corpus closure is
// the FINAL AUTHORITY — it credits a callee that PROVABLY preserves the register,
// and fires when the callee genuinely clobbers it.
// ===========================================================================

/// FINAL AUTHORITY, positive: the callee `Del` provably preserves a0 (it saves +
/// restores a0 around its own body), so the closure credits the caller's trailing
/// `jsr Del` and the caller's `preserves(a0)` clears — NO firing. The caller could
/// not prove this per-file (it deferred); the closure does.
#[test]
fn oracle_credits_preserving_trailing_callee() {
    let r = analyze(&[
        "module m\n\
         proc Caller () clobbers(d0) preserves(a0) {\n\
             move.l  a0, -(sp)\n\
             lea     Foo, a0\n\
             movea.l (sp)+, a0\n\
             jbsr    Del\n\
             rts\n\
         }\n\
         proc Del () clobbers(d0) preserves(a0) {\n\
             move.l  a0, -(sp)\n\
             lea     Bar, a0\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
    ]);
    assert!(!fires(&r, "Caller", "a0"), "a preserving trailing callee → no firing: {:?}", r.firings);
    assert!(!fires(&r, "Del", "a0"), "Del itself preserves a0: {:?}", r.firings);
}

/// FINAL AUTHORITY, negative: the callee `Trash` genuinely CLOBBERS a0 (writes it,
/// no restore, no preserves). The caller's trailing `jbsr Trash` therefore does NOT
/// round-trip a0 — the per-file gate DEFERRED, but the closure, the final authority,
/// FIRES `Caller`/a0. Nothing genuinely unprovable slips through.
#[test]
fn closure_fires_when_trailing_callee_clobbers() {
    let r = analyze(&[
        "module m\n\
         proc Caller () clobbers(d0) preserves(a0) {\n\
             move.l  a0, -(sp)\n\
             lea     Foo, a0\n\
             movea.l (sp)+, a0\n\
             jbsr    Trash\n\
             rts\n\
         }\n\
         proc Trash () clobbers(d0/a0) {\n\
             lea     Bar, a0\n\
             rts\n\
         }\n",
    ]);
    assert!(fires(&r, "Caller", "a0"), "a clobbering trailing callee must fire on the caller: {:?}", r.firings);
}

// ---------------------------------------------------------------------------
// rung-2 §13.3 sub-part 3 — the Z80 cross-proc caller-must-consume flag check.
// The 68k register-contract closure skips (cpu: z80) modules, but the
// flag-result must-use check is inherently cross-proc, so a SEPARATE Cpu::Z80
// pass routes Z80 procs' flag callees + bodies into check_flag_unused. Corpus
// shape: PsgVolEnv_Resolve declares out(carry: found); a caller `jr c`s on it.
// ---------------------------------------------------------------------------

/// A Z80 caller that CONSUMES the callee's `out(carry:)` result (`jr c`) over
/// the whole-corpus walk — no flag firing (the wired Z80 happy path).
#[test]
fn z80_flag_result_consumed_over_corpus_passes() {
    let r = analyze(&[
        "module m (cpu: z80)\n\
         section s (cpu: z80, vma: $0) {\n\
           proc Resolve () out(carry: found) {\n\
               scf\n\
               ret\n\
           }\n\
           proc Caller () {\n\
               call Resolve\n\
               jr c, .miss\n\
               ret\n\
           .miss:\n\
               ret\n\
           }\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Resolve"), 0, "flag firings: {:?}", r.flag_firings);
}

/// The same Z80 callee whose caller ABANDONS the carry (returns without testing
/// it) fires `[call.flag-result-unused]` through the Z80 corpus routing — the
/// psg-header bug class, now cross-proc-caught.
#[test]
fn z80_flag_result_dropped_over_corpus_fires() {
    let r = analyze(&[
        "module m (cpu: z80)\n\
         section s (cpu: z80, vma: $0) {\n\
           proc Resolve () out(carry: found) {\n\
               scf\n\
               ret\n\
           }\n\
           proc Caller () {\n\
               call Resolve\n\
               ld a, 0\n\
               ret\n\
           }\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Resolve"), 1, "flag firings: {:?}", r.flag_firings);
}

/// A bounded indirect dispatch charges the bound's `out` registers as well as its
/// `clobbers` — an `out` register is WRITTEN by whatever target is installed, so
/// a caller holding a live value in it across the dispatch is wrong. The live
/// corpus shape is `player_sensors.emp`'s `SensorProbe … clobbers(d3-d5/a1)
/// out(d0, d1, d2)` reached through `jsr (a2) as SensorProbe`.
///
/// This matters beyond permissiveness: the same `effective` set feeds
/// `preserves::find_dead_saves`, so a bound narrower than the truth makes a
/// load-bearing save look dead.
#[test]
fn bounded_indirect_charges_the_bounds_out() {
    let r = analyze(&[
        "module m\n\
         type SensorProbe = proc () clobbers(d3) out(d0)\n\
         proc Dispatch () clobbers(d3) {\n jsr (a2) as SensorProbe\n rts }\n",
    ]);
    assert!(fires(&r, "Dispatch", "d0"), "the bound's out must be charged: {:?}", r.firings);
}

/// A CONDITIONAL bound out is charged too — the register is written on the cc
/// edge, so from the caller's side it is destroyed on every edge (the same
/// conservative reading `extern_node` takes).
#[test]
fn bounded_indirect_charges_a_conditional_bound_out() {
    let r = analyze(&[
        "module m\n\
         type Alloc = proc () clobbers(d0) out(a1 if eq)\n\
         proc Dispatch () clobbers(d0) {\n jsr (a2) as Alloc\n rts }\n",
    ]);
    assert!(fires(&r, "Dispatch", "a1"), "the bound's conditional out must be charged: {:?}", r.firings);
}

/// Declaring the charged out keeps the dispatch clean — the fix widens the bound,
/// it does not make bounded dispatch unusable.
#[test]
fn bounded_indirect_with_the_out_declared_is_clean() {
    let r = analyze(&[
        "module m\n\
         type SensorProbe = proc () clobbers(d3) out(d0)\n\
         proc Dispatch () clobbers(d3, d0) {\n jsr (a2) as SensorProbe\n rts }\n",
    ]);
    assert!(r.firings.is_empty(), "declared bound out must not fire: {:?}", r.firings);
}

/// The conditional/unconditional out split is keyed on CANONICAL register names.
/// `out(sp if eq)` expands to `a7` in the out reglist, so a RAW-text subtraction
/// leaves `a7` in the UNCONDITIONAL set — crediting a conditional result as a
/// definition on every return path, the false-negative polarity D1b must-def and
/// the §6 taint-kill run on.
#[test]
fn conditional_out_spelled_sp_is_not_credited_unconditionally() {
    let r = analyze(&[
        "module m\n\
         extern proc Probe () out(sp if eq)\n\
         proc Caller () clobbers(d0) {\n jbsr Probe\n rts }\n",
    ]);
    let verified = r.verified_uncond_out.get("Probe").cloned().unwrap_or_default();
    assert!(
        !verified.contains("a7"),
        "`out(sp if eq)` must not be credited as an unconditional out: {verified:?}"
    );
}

/// The wall: an UNCONDITIONAL `out(sp)` IS credited (the canonicalization must
/// not swallow a genuine unconditional result).
#[test]
fn unconditional_out_spelled_sp_is_still_credited() {
    let r = analyze(&[
        "module m\n\
         extern proc Probe () out(sp)\n\
         proc Caller () clobbers(d0) {\n jbsr Probe\n rts }\n",
    ]);
    let verified = r.verified_uncond_out.get("Probe").cloned().unwrap_or_default();
    assert!(verified.contains("a7"), "`out(sp)` must stay an unconditional out: {verified:?}");
}

/// THE `[comptime.unresolved]` LEDGER REPRO (gap-ledger `[bprime-4-gates lens B]`
/// row, executable). A statement-`if` whose condition references a name the
/// define set does not resolve evaluates to `Poison` and discards BOTH arms:
/// the guarded `moveq #1, d2` write is invisible to the closure (no firing),
/// and nothing reaches `dropped_instrs` (arms are discarded whole, not
/// dropped). The ONLY thing that sees the failure is the unresolved surface,
/// which names the exact proc and name. Supplying the define — the SAME source
/// — empties the surface and reveals the hidden write as a real closure firing:
/// the two halves together are the class this surface catches and the drop
/// count structurally cannot.
#[test]
fn a_poisoned_statement_if_is_invisible_at_zero_drops_and_the_surface_names_it() {
    let src = "module m\n\
         proc Foo () clobbers(d0) {\n\
             moveq #0, d0\n\
             if NOPE_UNDEFINED == 0 {\n\
                 moveq #1, d2\n\
             }\n\
             rts\n\
         }\n";

    // Without the define: zero drops, zero firings, one surface row.
    let r = analyze(&[src]);
    assert!(!fires(&r, "Foo", "d2"), "the discarded arm's d2 write must be invisible: {:?}", r.firings);
    assert_eq!(r.dropped_instrs, 0, "a poisoned condition drops nothing: {:?}", r.dropped_by_proc);
    let rows: Vec<(&str, &str)> =
        r.comptime_unresolved.iter().map(|(p, n, _)| (p.as_str(), n.as_str())).collect();
    assert_eq!(
        rows,
        vec![("Foo", "NOPE_UNDEFINED")],
        "the surface must name exactly the proc and the unresolved name"
    );

    // With the define: the surface empties and the hidden write becomes a real
    // undeclared-clobber firing (`NOPE_UNDEFINED == 0` is true, the arm lowers).
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let r = sigil_frontend_emp::corpus_contracts::analyze_corpus_with(
        &[file],
        &[("NOPE_UNDEFINED".to_string(), 0)],
    );
    assert!(r.comptime_unresolved.is_empty(), "resolved: {:?}", r.comptime_unresolved);
    assert!(fires(&r, "Foo", "d2"), "the revealed arm's d2 write must now fire: {:?}", r.firings);
}

/// The EXPRESSION-`if` sibling of the statement repro, in the shape where it
/// really bites: a comptime CODE-EMITTING fn (the emit-tool idiom) whose gate
/// condition fails to resolve. The `if` statement is silently skipped (`Poison`
/// selects no branch), the fn FALLS THROUGH to `return Code.empty()`, the call
/// splices nothing — the guarded `moveq #1, d2` never exists, at zero drops.
/// The surface names the reference through the same eval-site harvest,
/// attributed to the proc whose body evaluation reached it.
///
/// Honest boundary, pinned by construction here: the harvest records a
/// RESOLUTION MISS, and label-value positions (an instruction immediate, a data
/// initializer, a call argument) resolve an unknown bareword as a deferred link
/// Label instead — so a condition evaluated UNDER one of those contexts is
/// outside this surface's reach (the build rejects it as a type error, one-file
/// and loud; the corpus statement-`if` sites all evaluate outside label ctx).
#[test]
fn a_poisoned_expression_if_inside_a_comptime_fn_lands_on_the_surface() {
    let src = "module m\n\
         comptime fn emit_thing() -> Code {\n\
             if WHO_KNOWS == 1 {\n\
                 return asm {\n\
                     moveq #1, d2\n\
                 }\n\
             }\n\
             return Code.empty()\n\
         }\n\
         proc Bar () clobbers(d0) {\n\
             moveq #0, d0\n\
             emit_thing()\n\
             rts\n\
         }\n";
    let r = analyze(&[src]);
    assert!(!fires(&r, "Bar", "d2"), "the ungated Code never spliced: {:?}", r.firings);
    assert_eq!(r.dropped_instrs, 0, "nothing drops — the arm never existed: {:?}", r.dropped_by_proc);
    let rows: Vec<(&str, &str)> =
        r.comptime_unresolved.iter().map(|(p, n, _)| (p.as_str(), n.as_str())).collect();
    assert_eq!(
        rows,
        vec![("Bar", "WHO_KNOWS")],
        "the expression-if condition's unresolved name must surface"
    );
}

/// THE BUILD-TIER HALF of the two-stage doctrine, pinned: an unresolved name in a
/// comptime condition is a ONE-FILE fact, and the build path makes it a HARD
/// ERROR (`unknown name`) — the evaluator reports at the resolution miss and
/// lowering surfaces every eval diagnostic. The corpus surface exists for the
/// fact a single build can never check: that every OTHER shipped shape's define
/// set also reaches the analysis. So the build path is the first tier and the
/// per-shape corpus gate the second; if this test ever fails, the corpus gate
/// has become the ONLY line of defence and the tiering needs re-adjudication.
#[test]
fn an_unresolved_condition_name_is_a_hard_error_in_the_build_path() {
    use sigil_frontend_emp::lower::{lower_module, LowerOptions};
    let src = "module m\n\
         proc Foo () clobbers(d0) {\n\
             moveq #0, d0\n\
             if NOPE_UNDEFINED == 0 {\n\
                 moveq #1, d2\n\
             }\n\
             rts\n\
         }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: sigil_ir::backend::Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errors: Vec<&str> = diags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        errors.iter().any(|m| m.contains("unknown name `NOPE_UNDEFINED`")),
        "the build must hard-fail naming the unresolved condition name: {errors:?}"
    );
}

/// The MEMO SIDE DOOR is closed: a const/equ whose initializer failed resolves
/// to a MEMOIZED `Poison`, returned without re-entering the initializer — so a
/// later `if FOO == 1` would find nothing newly logged to harvest, resurrecting
/// the arm-invisibility hole through the memo. The const READ site therefore
/// logs the const's own name whenever the resolved value is `Poison`. Two
/// conditions on the same poisoned equ: the first surfaces both the
/// initializer's miss and the read; the second — served purely from the memo —
/// must still surface the read.
#[test]
fn a_memoized_poison_const_read_in_a_condition_still_lands_on_the_surface() {
    let src = "module m\n\
         equ FOO = MISSING_NAME\n\
         proc Baz () clobbers(d0) {\n\
             moveq #0, d0\n\
             if FOO == 1 {\n\
                 moveq #1, d3\n\
             }\n\
             if FOO == 1 {\n\
                 moveq #1, d4\n\
             }\n\
             rts\n\
         }\n";
    let r = analyze(&[src]);
    assert!(!fires(&r, "Baz", "d3") && !fires(&r, "Baz", "d4"), "arms discarded: {:?}", r.firings);
    assert_eq!(r.dropped_instrs, 0, "nothing drops: {:?}", r.dropped_by_proc);
    let foo_reads =
        r.comptime_unresolved.iter().filter(|(p, n, _)| p == "Baz" && n == "FOO").count();
    assert_eq!(
        foo_reads, 2,
        "BOTH poisoned reads must surface — one row per condition, the second \
         served from the memo: {:?}",
        r.comptime_unresolved.iter().map(|(p, n, _)| (p.as_str(), n.as_str())).collect::<Vec<_>>()
    );
    assert!(
        r.comptime_unresolved.iter().any(|(p, n, _)| p == "Baz" && n == "MISSING_NAME"),
        "the initializer's own miss surfaces on first resolution: {:?}",
        r.comptime_unresolved
    );
}

/// LOOPS join the surface: a `while` whose condition poisons stops silently
/// (zero iterations), and a `for` whose iterable poisons skips its whole body —
/// the same arm-invisibility a poisoned `if` has, at zero drops. Both loop
/// heads harvest their resolution misses exactly as the `if` sites do.
#[test]
fn a_poisoned_loop_head_lands_on_the_surface() {
    let src = "module m\n\
         comptime fn spin() -> Code {\n\
             while WHILE_GATE == 1 {\n\
                 return asm {\n\
                     moveq #1, d5\n\
                 }\n\
             }\n\
             for i in 0..FOR_BOUND {\n\
                 return asm {\n\
                     moveq #1, d6\n\
                 }\n\
             }\n\
             return Code.empty()\n\
         }\n\
         proc Qux () clobbers(d0) {\n\
             moveq #0, d0\n\
             spin()\n\
             rts\n\
         }\n";
    let r = analyze(&[src]);
    assert!(!fires(&r, "Qux", "d5") && !fires(&r, "Qux", "d6"), "no arm spliced: {:?}", r.firings);
    assert_eq!(r.dropped_instrs, 0, "nothing drops: {:?}", r.dropped_by_proc);
    let rows: Vec<(&str, &str)> =
        r.comptime_unresolved.iter().map(|(p, n, _)| (p.as_str(), n.as_str())).collect();
    assert_eq!(
        rows,
        vec![("Qux", "FOR_BOUND"), ("Qux", "WHILE_GATE")],
        "both loop heads' unresolved names must surface"
    );
}

// ===========================================================================
// §6 partial-width — conservative v1, pinned THROUGH the implementing path.
// ===========================================================================

/// A `.w`-round-tripping proc that declares `preserves(d5.w)`.
const WORD_CALLEE: &str = "module m\n\
     proc WCallee () clobbers(d0) preserves(d5.w) {\n\
     \x20   move.w d5, -(sp)\n\
     \x20   moveq #0, d5\n\
     \x20   move.w (sp)+, d5\n\
     \x20   rts\n\
     }\n";

/// THE PERMISSION HALF of conservative v1, driven through the real path: a
/// `preserves(dN.w)` licenses clobbering the FULL dN, so the declaring proc must
/// NOT fire `[proc.clobber-undeclared]` on d5 — even though d5 is written, is not
/// in `clobbers`, and is deliberately NOT credited to `verified_preserves`.
///
/// This pins the mechanism (`declared_clobbers` gaining the word-facet registers):
/// with that extension removed, `effective` still contains d5 (a local write the
/// facet never subtracts) while `allowed` loses it, and this fires.
#[test]
fn a_word_facet_licenses_the_full_register_as_a_clobber() {
    let r = analyze(&[WORD_CALLEE]);
    assert!(
        !fires(&r, "WCallee", "d5"),
        "a `preserves(d5.w)` licenses d5 as a clobber — it must not fire: {:?}",
        r.firings
    );
    // Non-vacuity: the walk DID see this proc and its d5 write. The identical proc
    // WITHOUT the facet fires, so the assertion above is about the facet and not
    // about an empty analysis.
    let bare = analyze(&["module m\n\
         proc WCallee () clobbers(d0) {\n\
         \x20   move.w d5, -(sp)\n\
         \x20   moveq #0, d5\n\
         \x20   move.w (sp)+, d5\n\
         \x20   rts\n\
         }\n"]);
    assert!(
        fires(&bare, "WCallee", "d5"),
        "without the facet the same d5 write IS an undeclared clobber: {:?}",
        bare.firings
    );
}

/// THE NO-CREDIT HALF of conservative v1: to every consumer a `.w`-preserving
/// callee is a FULL clobber of d5, so a caller that declares `preserves(d5)` across
/// the call is not credited and fires. Nothing about the callee's word facet may
/// weaken a consumer's verdict (spec §3).
#[test]
fn a_caller_gets_no_full_credit_through_a_word_preserving_callee() {
    let r = analyze(&[
        WORD_CALLEE,
        "module n\nproc Caller () clobbers(d0) preserves(d5) {\n jbsr WCallee\n rts }\n",
    ]);
    assert!(
        fires(&r, "Caller", "d5"),
        "a `.w`-preserving callee is a full clobber to its caller: {:?}",
        r.firings
    );
}

/// The contract SURFACE records the facet (spec §3): `preserves(dN.w)` is
/// distinguishable from a plain `clobbers(dN)` downstream, even though the two are
/// deliberately identical to the closure's verdicts.
#[test]
fn the_word_facet_is_recorded_on_the_contract_surface() {
    let r = analyze(&[WORD_CALLEE]);
    assert_eq!(
        r.word_preserve_claims,
        vec![("WCallee".to_string(), "d5".to_string())],
        "the facet is recorded as a claim, not erased into the clobber set"
    );
    // And a plain clobber of the same register records NO claim — the surface
    // genuinely distinguishes them.
    let bare = analyze(&["module m\nproc P () clobbers(d0, d5) {\n moveq #0, d5\n rts }\n"]);
    assert!(
        bare.word_preserve_claims.is_empty(),
        "a plain clobber is not a word-facet claim: {:?}",
        bare.word_preserve_claims
    );
}

/// The word-facet OBLIGATION gate fires when the claim is false: a proc declaring
/// `preserves(d5.w)` whose body does not round-trip the low word is reported by
/// `word_preserve_firings`. The corpus assert-empty gate is only meaningful because
/// this path is reachable.
#[test]
fn a_false_word_facet_claim_fires_the_obligation_gate() {
    let r = analyze(&[
        "module m\nproc Liar () clobbers(d0) preserves(d5.w) {\n moveq #0, d5\n rts }\n",
    ]);
    assert!(
        r.word_preserve_firings.iter().any(|(p, reg, _)| p == "Liar" && reg == "d5.w"),
        "a false `.w` claim must fire the obligation gate: {:?}",
        r.word_preserve_firings
    );
}

/// A `[proc.out-unverified]` firing on `(proc, reg)` is present.
fn out_fires(
    r: &sigil_frontend_emp::corpus_contracts::ContractReport,
    proc: &str,
    reg: &str,
) -> bool {
    r.out_firings.iter().any(|f| f.proc == proc && f.reg == reg)
}

/// THE SITE-2 PLUMBING GUARD, one layer above the fixpoint's own
/// `falls_into_successor_credit_reaches_the_fixpoint`: the `falls_into` successor
/// name must travel from the proc table into the PER-PROC out check, not only
/// into the verified-out fixpoint.
///
/// The fixpoint's copy decides which procs end up VERIFIED; this one decides
/// whether a proc's OWN claim is charged at its fall-off.
///
/// The coverage is ASYMMETRIC, measured rather than assumed. A `None` mutant at
/// the per-proc call site reddens the fall-off assertion below and nothing else in
/// the tree. A `None` mutant at the FIXPOINT's call site also reddens this test —
/// through the verified-map presence assertion, not the fall-off one — as well as
/// `out_width.rs::falls_into_credit_is_capped_at_the_successors_declared_width`.
/// So this test covers both arguments while `out_verify.rs`'s
/// `falls_into_successor_credit_reaches_the_fixpoint` covers neither call site,
/// only the function parameter it hand-builds.
///
/// THE CORPUS CANNOT WITNESS THIS. No 68k proc declares both `falls_into` and an
/// `out`, so the argument is dead over every shipped shape and a `None` mutant
/// passes all seven byte gates and the whole closure corpus. `PsgVolEnv_Resolve`
/// is the only proc of that shape anywhere and it is Z80, which the closure tier
/// excludes twice over (`proc_bufs` skips `(cpu: z80)` modules and `Reg::from_name`
/// rejects Z80 spellings). This test is therefore the ONLY thing standing under
/// the argument.
///
/// The polarities vary the SUCCESSOR'S PRODUCTION, not the `falls_into` flag.
/// Varying the flag alone would keep passing under an implementation that ignores
/// the successor entirely — it would pin the vacuity as the contract.
#[test]
fn falls_into_successor_credit_reaches_the_per_proc_out_check() {
    // P produces a1 nowhere and runs off its end into Q; Q produces a1 locally.
    // The claim must be credited from Q's verified out.
    let credited = analyze(&[
        "module m\n\
         proc P () clobbers(d0) out(a1) falls_into Q {\n\
        \x20   moveq #0, d0\n\
         }\n\
         proc Q () clobbers() out(a1) {\n\
        \x20   lea Slot, a1\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        !out_fires(&credited, "P", "a1"),
        "P's fall-off must be credited from Q's verified out(a1): {:?}",
        credited.out_firings
    );
    // "No firing" is also what a proc the walk never parsed produces, and what a
    // checker that stopped charging outs entirely produces. A name in the
    // verified map states the claim was EXAMINED and CARRIED.
    assert!(
        credited.verified_uncond_out.get("P").is_some_and(|s| s.contains("a1")),
        "P's a1 must be carried in the verified map, not merely absent from the \
         firings: {:?}",
        credited.verified_uncond_out
    );

    // NON-VACUITY, and the reason the successor is provably consulted: the same
    // body whose successor does NOT produce a1 must still fire. Without this the
    // assertion above would also pass on a surface that never populates at all.
    let barren = analyze(&[
        "module m\n\
         proc P () clobbers(d0) out(a1) falls_into Q {\n\
        \x20   moveq #0, d0\n\
         }\n\
         proc Q () clobbers(d0) out(d0) {\n\
        \x20   moveq #0, d0\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        out_fires(&barren, "P", "a1"),
        "a successor that does not produce a1 leaves P's claim unproven: {:?}",
        barren.out_firings
    );
}

/// THE OUT-RESIDUE SURFACE READS THE VERIFIED MAP, NOT THE DECLARED ONE.
///
/// `D` produces `a1` nowhere and tails into `S`; `S` DECLARES `out(a1)` but never
/// produces it. Under DECLARED credit the tail hands `D` its `a1` and `D` vanishes
/// from the residue. Under VERIFIED credit `S`'s own claim is unproven, so it
/// credits nothing and `D` stands. `D` firing is the entire discriminator.
///
/// This is a SYNTHETIC and belongs here rather than in the corpus. The corpus
/// witness for this fact was `Art_Decompress::out(a1)` grounding in
/// `S4LZ_Decompress`, and before that `Collision_GetType::out(d0)` — the latter
/// went silently inert when aeon fused its callee into a leaf, passing while
/// detecting nothing. Both had the same defect: their discriminating power lived
/// in another repo and decayed when that repo moved. No row in the present residue
/// carries the shape at all — every one writes its own register and sources none
/// of it from a callee, tail or successor — so a corpus witness is not merely
/// fragile here, it is unavailable.
#[test]
fn the_out_residue_surface_uses_verified_credit_not_declared() {
    // S declares out(a1) and produces nothing, so its claim is unverifiable and
    // it credits nothing to the tail that grounds in it.
    let unverified_source = analyze(&[
        "module m\n\
         proc D () clobbers(d0) out(a1) {\n\
        \x20   jbra S\n\
         }\n\
         proc S () clobbers(d0) out(a1) {\n\
        \x20   moveq #0, d0\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        out_fires(&unverified_source, "D", "a1"),
        "D's out(a1) grounds ONLY in S's declared-but-unverified out, so under verified \
         credit it must stand in the residue; its absence means the surface is reading \
         the declared map: {:?}",
        unverified_source.out_firings
    );

    // THE OPPOSITE POLARITY, and the non-vacuity control: the same D over a source
    // that DOES produce a1 must not fire. Without this, D firing above would be
    // equally consistent with a surface that fires on every declared out.
    let verified_source = analyze(&[
        "module m\n\
         proc D () clobbers(d0) out(a1) {\n\
        \x20   jbra S\n\
         }\n\
         proc S () clobbers(d0) out(a1) {\n\
        \x20   lea Slot, a1\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        !out_fires(&verified_source, "D", "a1"),
        "a tail into a VERIFIED producer must discharge D's claim: {:?}",
        verified_source.out_firings
    );

    // TWO HETEROGENEOUS TAILS — the shape the retired corpus witness carried and
    // the single-tail cases above cannot state. `Art_Decompress` dispatched to a
    // verified producer on one arm and an unverified declarer on the other, and
    // the row proved credit is accumulated PER EDGE: one uncrediting tail leaves
    // the claim unproven no matter what the sibling arm supplies. A union over
    // the tail targets would discharge D here and is exactly what this excludes.
    let mixed_tails = analyze(&[
        "module m\n\
         proc D () clobbers(d0) out(a1) {\n\
        \x20   tst.b Flag\n\
        \x20   bne .other\n\
        \x20   jbra V\n\
        \x20.other:\n\
        \x20   jbra S\n\
         }\n\
         proc V () clobbers(d0) out(a1) {\n\
        \x20   lea Slot, a1\n\
        \x20   rts\n\
         }\n\
         proc S () clobbers(d0) out(a1) {\n\
        \x20   moveq #0, d0\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        out_fires(&mixed_tails, "D", "a1"),
        "one uncrediting tail must leave the claim unproven even though the sibling \
         tail V produces a1 — credit is per edge, not a union: {:?}",
        mixed_tails.out_firings
    );
    assert!(
        mixed_tails.verified_uncond_out.get("V").is_some_and(|s| s.contains("a1")),
        "the sibling arm V must really be a verified producer, or the case above \
         degenerates into the single-tail one: {:?}",
        mixed_tails.verified_uncond_out
    );
}

/// THE CONDITIONAL HALF OF THE SAME SURFACE. `check_out` takes TWO credit maps —
/// unconditional and conditional — and they are independent arguments. The
/// unconditional one is covered above; swapping only the conditional one to the
/// DECLARED map compiles (the types are identical) and, before this test, was
/// green across the whole tree.
///
/// Same discriminator, conditional shape: `D` declares `out(a1 if eq)` grounded
/// only in `S`'s declared-but-unverified `out(a1 if eq)`. Under declared credit
/// the call discharges `D`; under verified credit `S` proves nothing and `D`
/// stands.
#[test]
fn the_conditional_out_credit_surface_also_uses_verified_credit() {
    let unverified_source = analyze(&[
        "module m\n\
         proc D () clobbers(d0/a1) out(a1 if eq) {\n\
        \x20   jbsr S\n\
        \x20   bne .fail\n\
        \x20   rts\n\
        \x20.fail:\n\
        \x20   rts\n\
         }\n\
         proc S () clobbers(d0/a1) out(a1 if eq) {\n\
        \x20   moveq #0, d0\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        out_fires(&unverified_source, "D", "a1"),
        "D's conditional out grounds ONLY in S's declared-but-unverified conditional \
         out, so under verified credit it must stand in the residue: {:?}",
        unverified_source.out_firings
    );

    // The opposite polarity: a source that really produces a1 on its success edge
    // discharges D. Without it, the assertion above would also pass on a surface
    // that fires every conditional out unconditionally.
    let verified_source = analyze(&[
        "module m\n\
         proc D () clobbers(d0/a1) out(a1 if eq) {\n\
        \x20   jbsr S\n\
        \x20   bne .fail\n\
        \x20   rts\n\
        \x20.fail:\n\
        \x20   rts\n\
         }\n\
         proc S (d0: u16) clobbers(a1) out(a1 if eq) {\n\
        \x20   tst.w d0\n\
        \x20   bne .miss\n\
        \x20   lea Slot, a1\n\
        \x20   rts\n\
        \x20.miss:\n\
        \x20   rts\n\
         }\n",
    ]);
    assert!(
        !out_fires(&verified_source, "D", "a1"),
        "a call into a source whose conditional out IS verified must discharge D's \
         claim: {:?}",
        verified_source.out_firings
    );
    // ...and S's conditional out must really BE verified, or the control above
    // passes because D was never charged rather than because it was credited.
    assert!(
        verified_source
            .verified_cond_out
            .get("S")
            .is_some_and(|v| v.iter().any(|(reg, _)| reg == "a1")),
        "S's out(a1 if eq) must be carried in the verified conditional map: {:?}",
        verified_source.verified_cond_out
    );
}
