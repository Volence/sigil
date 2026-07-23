# Dry-panel calibration run — sprites.emp (outcome + triage rulings)

> **STATUS: TRIAGED 2026-07-23 (overseer).** Full six-lens report delivered in-session
> (A1·B1·B2·C1·C2·C3 on engine/objects/sprites.emp @ bd9ddf2, read-only, independent
> subagents). This note records the calibration verdict, the overseer's claim
> verifications, and where each finding landed. **INSTRUMENT RULING: composition
> RATIFIED UNCHANGED for t18** — the C-family's honest near-null on a mature file is a
> feature (no manufactured findings); yield concentrated in A/B/C2 (structural + latent
> classes), exactly the dry-panel design intent; four blind lenses converged on the same
> two structures (SAT/mapping layout · frame-resolution idiom) = real signal, not noise.
> Nothing rose to must-fix — a live dry claim would NOT have re-opened the cycle, only
> filed step-4/5 backlog. Cost/benefit confirmed.

## Overseer verifications (run before triage)

- **C3-2 RESOLVED CLEAN:** SpriteMask_Y's ram.asm declaration documents the VDP-Y (+128)
  contract; corpus has ZERO writers today (mask feature has no producer). No bug. But the
  class is real — **screen-Y vs VDP-Y (+128 bias) already bit once (PB2)** → ledgered as
  a VdpY/ScreenY domain-type candidate (item-13 orbit).
- **C3-3 VERIFIED CLEAN:** DrawRings emits exactly 8 bytes/entry (2+1+1+2+2), link d5
  incremented once per entry, skip-path touches neither — the sprites terminator's
  whole-entry dependency holds.
- **C2-1 CONFIRMED:** `lsl.w #6` at :147/:225 hardcodes SPRITES_PER_BAND*2=64 (comment
  tracks the symbol, the shift doesn't). Latent, gate-blind (both twins share it).
- **C2-2 CONFIRMED incl. comment discrepancy:** the 8-byte clears' "pad byte" comments
  contradict ram.asm's "already even — no pad needed". Comment-vs-truth nit + latent
  size coupling.

## Findings → dispositions (gap-ledger rows added same commit)

| Finding | Disposition |
|---|---|
| B2-1 SAT-emit primitive duplicated (sprites Emit_ObjectPieces + rings DrawRings; verbatim link-protocol + .x_ok X=0 clauses) | **LEDGER (top actionable):** shared-construct promotion, splice-style (byte-neutral when emitted bytes identical); rides the next sprites/rings touch or a small construct batch |
| C2-1/2/3/4 stride-hardening cluster | **LEDGER:** ensure() drift-locks (lsl#6↔SPRITES_PER_BAND*2 · clear sizes↔PRIORITY_BANDS/SCANLINE_BANDS · lsr#5↔SCREEN_HEIGHT/SCANLINE_BANDS · mask size/height agreement) + fix the false pad comments. Byte-neutral; small hardening rider |
| A1 record-over-streaming-cursor | **LEDGER (language ask):** can .emp records serve a post-increment cursor walk with named offsets (the -5(a4)/-6(a3) class)? Design question for the language track |
| A3 sub-word RAM access (Sprite_Cycle_Counter+1) | **LEDGER (small):** named `_lo` const per the row-1068 shared-const precedent — buildable anytime |
| C3-2 resolution spawn | **LEDGER (item-13 orbit):** VdpY/ScreenY newtype pair — the +128-bias class, PB2 precedent |
| A2 (MappingFrame/VramTile untyped) | Already covered — item-13 wave-1 rows (confirmation, not discovery) |
| B1-1 clear_longs, B1-2 frame_piece_count holdout, C1-2 lea hoist, A5/B2-2/B2-3 | Noted here; no rows — sub-bar or byte-changing-for-nothing; revisit only riding a natural touch |

## Calibration data banked for the rule

- C1/C3 honest near-null on a well-tended file = the lenses don't manufacture findings.
- Blind-lens convergence (2 structures, 3-4 lenses each) = agreement will be meaningful
  when the panel fires on a real port.
- Expected t18 yield profile: A/B/C2 structural+latent findings; C1/C3 real signal only
  where the port itself introduces new perf/timing surface.
