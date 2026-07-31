# G9 — Ground_Move_Cap probe-direction d7 high-word clear (object-bank parcel)

**Class BA (byte-gate-blind hazard fix) / census effort S, re-rated by the
2026-07-31 design note for its bank-head position.** Executed by the overseer
directly (S-effort; A3 precedent). Chain len 9. Design note:
`2026-07-31-g9-object-bank-parcel-design.md`.

## The fix

One `moveq #0, d7` before `move.b (a1,d2.w), d7` in `Ground_Move_Cap`'s
`.dir_fwd` (games/sonic4/player/player_ground.emp): the probe-direction code
loads as a BYTE but is consumed as a WORD by the `.off_*`/`.cancel_*`
`move.w`/`tst.w` sites. +2 bytes at the object-bank HEAD section.

## THE STRUCTURAL HEADLINE: the rows-6/58 cascade is dead — proven live

G9 was PARKED because a +2 at the bank head collided ~15 hand-pinned resume
orgs (`resolve_layout` colliding-pins failure, the pin-tax the campaign twice
refused). The design step ruled LEAN COMPUTED; B-0 pulled that forward. This
parcel is the first LIVE +2 at the bank head since: **both shapes built first
try, zero hand-bumps.** Shift census (debug): 1243 ROM symbols unmoved, +2 ×19
(player_ground tail), then align-cascade +4 ×45 / +8 ×11 / +0x10 ×183; TOTAL
SIZE UNCHANGED both shapes (the tail pad absorbed the cascade). What was a
15-org hand-ruling in July is now: build it.

- plain `6cf74e65/412127` (was 7ec2137a/412127) · debug `16615e46/421958`
  (was 229446d4/421958).

## A/B (debug shape, the collision drive — the design note's reachability prep,
now the standing grounded-movement scene)

- **Reachability + the benign claim made concrete (witness, no register
  injection — ledger row 21):** breakpoint at the decode. OLD (`0x1079C`):
  6 natural hits, **D7 = 0x00000001 every time** — bits 8–31 zero, so the word
  consume is benign, but NOT the assumed slot-counter 0: the entry value is 1,
  one dirty bit in 8–15 away from a mis-decode. The hazard was real, the fix
  earns its bytes. NEW (`0x1079E`, post-moveq): 5 hits, **D7 = 0 by
  construction**. Evidence `golden/ab/g9/witness_{OLD,NEW}.json`.
- **State identity (code-point anchors fc 220/280/340 at
  GameState_OJZScroll_Update 0x5E42C, double-runs deterministic both sides):**
  VRAM/CRAM/VSRAM/reg-file **byte-identical at every anchor** (only `ram_crc`
  differs). Full-RAM diff = 27–30 bytes, **every one classified as the
  layout-pointer class**: reg-encoded DMA-queue source bytes ($96xx/$95xx
  fields at stride 14 in DMA_Important) and stored ROM pointers
  (Parallax_Current_Config low byte, mappings/table pointers at $FFACxx…),
  each shifted by exactly its section's census delta (+8/+0x10). No stack-only
  story needed — the classification is field-level and closes to zero
  unexplained bytes. Evidence `golden/ab/g9/manifest_coll_{OLD,NEW}.json` +
  `coll_*_run*/ram_f*.bin`.
- Cost: 4 cyc per grounded-moving frame (one moveq). No profiler delta claim
  needed at this magnitude; the parcel is a correctness fix (BA), not PF.

## Oracle intel (new, minor)

`wait_for_break` can return while the system is still running (a race after
the prior `step`): the follow-up `registers` call then samples a LIVE PC — we
observed off-breakpoint samples at `0x10792` interleaved with genuine bp hits.
Genuine hits are identified by PC == the bp address; filter on that. (The
witness runner does.)

## Rulings

1. **KEEP.** BA-class hazard fix, sanctioned by the census + design note;
   behavior-identical today (proven: VDP identity + all-RAM-diffs-classified +
   the witness), load-bearing the day any dispatch path enters with dirty
   d7[8:15].
2. The design note's open question (hand-bump vs computed) is RESOLVED by
   B-0 in the field: computed placement carried the bank-head +2 with zero
   ceremony. The G9 ledger row closes.

## Step-3 / step-5 / neither

- **Step-3:** none new (the parcel exercised existing machinery).
- **Step-5:** the witness observed entry d7=1, not the note's assumed 0 —
  worth a future pass over other byte-load/word-consume sites in the object
  bank (the same latent shape may exist elsewhere); candidate for a
  diagnostics-mechanism sweep rather than hand-audit.
- **Neither:** the shift-census technique (diff the two .lst symbol tables,
  bucket by delta) is a 5-line one-shot that turns "did the layout do what
  packing promised" into a table; worth folding into the A/B protocol notes.

## Ripple record (strict-suite failures chased to conscious updates)

The +0x10 cascade tripped 5 hand-typed spot-check surfaces, each updated
consciously (values re-derived from the failing assertion + the shift census,
then re-proven by the tests' own value-level checks):
- `keystone_flip_relocation::DEFORM_PTR_OFF` 0x11410→0x11420 (the fold-vs-
  placement validator re-proves the emitted longword equals the placed VMA).
- `native_offcanonical_full` load-bearing: config_a HeightMaps 0x257c6→0x257d6;
  config_b BusError 0x423c0→0x423d0, HeightMaps 0x25720→0x25730, EndOfRom
  0x43470→0x43480.
- `test_p2_player_states_port::p2_air_undoctored_compile_equals_the_reference_window`
  gained the B-0 zero-pad tolerance (strict equality on the code prefix +
  zero-verified pad tail) — player_air's pin len now spans 6 bytes of align pad
  because its base shifted +2 while the next section shifted +8 (the load_art
  precedent from wave-c).
Final: strict **2861/0/4**; fresh builds reproduce the chain-9 CRCs (fixpoint).
