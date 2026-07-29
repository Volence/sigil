# 2026-07-29 — t38 verifier feature: the immediate-sp-cleanup idiom in preserves.rs

Porter: t38 continuation. Follows `2026-07-29-t38-step1-blocker.md` (the dc.w
feature) and precedes the green step-2 opening commit. Overseer-RULED (not
re-litigated here): teach `crates/sigil-frontend-emp/src/preserves.rs` the
immediate-sp-cleanup idiom so the 4 player_sensors probe cores' honest
`preserves(a0)` VERIFIES. Manual-honor annotations were REJECTED (standing
Fm_ReparkDac precedent) — the fix is a real dataflow model, not a trust escape.

## The blocked idiom (probe_core, player_sensors.emp)

```
        move.l  a0, -(sp)          // a0 saved across Collision_GetType
        move.w  d3, -(sp)          // layer word stashed beneath it
        bsr.s   .cell              // → bsr.w Collision_GetType (clobbers a0)
.done:
        addq.l  #2, sp             // drop the layer word (immediate sp-INCREASE)
        movea.l (sp)+, a0          // restore a0
        rts
```

`preserves.rs` bailed on ALL explicit sp arithmetic (the `Reg(A7)` operand read as
an sp escape), so `preserves(a0)` was `[proc.preserves-unverifiable]` and the
closure conservatively propagated GetType's a0-clobber through the cores →
**10 transitive a0 firings** in `corpus_closure_residue_is_empty` (the 4 cores +
Player_SensorWallDir + PState_Ground/Roll/Spindash + Ground_Move/_Cap).

## The model (constraints = tests, `tests/preserves.rs`)

- **Slot representation** now carries the TRUE pushed byte width
  (`struct Slot { reg: Option<Reg>, bytes: u8 }`) — the prerequisite the old
  `type Slot = Option<Reg>` lacked. `.l`=4, `.w`=2, byte-on-a7=2, each movem
  member its own size (`slot_bytes`).
- **`immediate_sp_cleanup_bytes`** recognizes ONLY `add/addq/adda #N, sp`
  (immediate source, sp dest, sp-INCREASE). `apply_sp_cleanup` drops whole slots
  totaling exactly N bytes, wired into `transfer` BEFORE the sp_hazard bail.
  - C1 control: `subq/suba #N,sp` (sp-DECREASE, scratch alloc) stays bailing;
    `adda dN,sp` (computed/register) stays bailing.
  - C2: a partial-slot drop (N lands mid-slot) bails loudly.
  - C3: over-drop soundness needs NO special case — dropping the slot that held a
    saved reg removes it; a later pop reads a different slot → the entry-bit model
    REFUTES the preserve (`NotPreserved`, not merely unverifiable).
  - C4: word/movem-member widths are TRUE — `addq #2` over a word slot verifies;
    over a long slot it is a partial-slot bail.
  - C5: the honest probe-core shape (save/cleanup/restore across a call) verifies
    under ClobberAll; the same shape with the restore removed does not (non-vacuity).
- `find_dead_saves` (ds_transfer) is UNCHANGED — it keeps bailing on sp arithmetic
  (the safe direction for a code-cutting worklist; never report a false dead-save).

## Outcome

New tests: 9 (`sp_cleanup_*`), 5 genuinely red-then-green + 4 controls green from
start. Two existing tests updated for honesty under the new semantics:
`written_without_restore_not_preserved` (now `NotPreserved` not a bail — comment
fixed) and `computed_sp_is_unverifiable` (body switched to a REGISTER operand
`adda.w d1,sp` so it genuinely tests computed sp, its immediate form now being the
modeled cleanup). Full `sigil-frontend-emp` suite: **1730 / 0** (was 1721 + 9).

The 10 firings clear at the step-2 commit via honest `preserves(a0)` on the 4
probe cores (verified by this model), NOT via widened clobbers.
