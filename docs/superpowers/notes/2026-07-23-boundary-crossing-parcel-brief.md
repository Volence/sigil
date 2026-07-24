# PARCEL BRIEF — boundary-crossing transitions (window-slide + B6/B3/B1 inside B2)

**Dispatch: overseer-cut 2026-07-23, Volence-ordered (runs in parallel with the
item-13 wave-1 parcel — file-disjoint; NOT with sprites-hardening).**
**Canonical sources (read before cutting):** gap-ledger row ~1408 (the parcel
anchor — structure + technique constraints), notes/2026-07-23-t18-loop-findings-
parallax-transition.md (B1/B2/B3 characterization + B3 fix sketch),
notes/2026-07-23-t18-dry-panel-packet.md (B6).

**Scope class: BYTE-CHANGING + BEHAVIOR-CHANGING** (the first deliberate behavior
fixes on ported parallax code). Both twins lockstep per fix; per-commit 5-site
ripple + re-pin + full paired strict; PROVENANCE re-baseline at merge.
**Branch:** `parallax-transition-parcel` both repos, seeded worktrees, AEON_DIR
pinned to the branch tree.

## Order of work (per the ratified anchor)

0. **Crossing-drive rig (the capability, built once).** Deterministic oracle
   choreography that drives the camera across a section boundary firing
   `Parallax_StartTransition`. Technique constraints are BINDING (row 1408):
   drive the SCROLL TARGET (player/camera-target poke so Camera_Update scrolls
   itself) or the Game_Entry-flip soak scene; NEVER held-input-through-a-
   breakpoint (§D terminal wedge); findings logged even on a null result.
   Deliverable: a repeatable protocol note (poke script + expected observables).
1. **B6 — promote-frame CC-clobber fix (the easy first win, independent).**
   The `move.l #0,Parallax_Target_Config` Z-clobber before `beq .no_config`;
   re-test d0 explicitly. Crossing A/B: before = one-frame parallax freeze on
   the promotion frame; after = none.
2. **Window-slide mask-migration observation** (the carried Phase-2.5 rider) —
   observe one real EntityWindow_Slide + Entity_Loaded_Masks migration with the
   rig; close the rider row with the observation record.
3. **B2 mode-contract design pass — GATE CHECKPOINT BEFORE CUTTING.** B2 defines
   the transition state machine ("active config" coherence across builder/DMA-
   length/VSRAM-mode consumers, ≤16-frame tear today). Deliver the design note
   to the gate; B3 + B1 land inside its ruling.
4. **B3 — frames-remaining-aware ramp** (sketch in the loop-findings note; the
   geometric (15/16)^16 ≈ 0.356 residual + .snap_b pop). Crossing A/B: pop gone,
   convergence by frame 0.
5. **B1 — re-cross cancel branch.** Crossing A/B: re-entering the current
   config's section mid-transition cancels the staged transition.

## Bars

- Each fix = own byte-changing commit + rig A/B evidence (before-repro AND
  after-clean) + rebuilt canonical stated from fresh dual builds.
- The rig protocol itself is packet material (it becomes the standing capability
  for all future transition work).
- Out of scope: the mixed-provenance lint candidate (ledgered), any parallax
  perf work (t18 closed it), wave-2 types.

## Acceptance

All five items closed (or honestly stopped with logged findings); full paired
strict green at every commit; close packet per house format; merge via the
sequential queue (fetch-first origin precondition; coordinate with the item-13
parcel's merge — whoever is ready first goes first, the second re-verifies on
the moved master; byte-changing ⇒ PROVENANCE re-baseline at merge).
