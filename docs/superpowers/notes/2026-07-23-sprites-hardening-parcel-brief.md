# PARCEL BRIEF — sprites structural-hardening (dry-panel calibration yield)

> **STATUS: PARKED — runs AFTER the conversion campaign completes (Volence,
> 2026-07-24).** Nothing here expires: the ledger rows it graduates (1355/1362/
> 1373) stay open, the files it touches (sprites.emp/rings.emp) are ported-and-
> stable with no remaining port scheduled against them, and the work is
> byte-neutral so it costs the same later. Natural slot: alongside or just
> before the post-asl-retirement optimization sweep (same files, same session
> economy). Do not dispatch before then without a fresh gate word.

**Source:** 2026-07-23 sprites.emp calibration run (6 lenses @ aeon bd9ddf2), triaged
at sigil c9c1552; graduates ledger rows ~1355 (SAT-emit shared construct), ~1362
(stride-hardening cluster), ~1373 (Sprite_Cycle_Counter low-byte const).
**Ruled actionable by the overseer 2026-07-23 (Volence's word) — third parcel in the
post-t18 queue.**

**Scope class: BYTE-NEUTRAL throughout** (no re-pin, no PROVENANCE change).
**Files:** sprites.emp, rings.emp, shared-construct home (frames.emp or new),
ram.asm comment only. `.asm` twins untouched — bytes must not move; the byte gate
is the verifier.

## P1 — SAT-emit shared construct (headline; ledger 1355)

Promote the 8-byte SAT entry-emit primitive to ONE comptime construct consumed by
sprites.emp `Emit_ObjectPieces` (all four flip variants) + rings.emp `DrawRings`.
Covers the verbatim-duplicated sub-clauses: the `addq.b #1,d5` / `move.b d5,(a4)+`
link protocol and the `bne .x_ok` / `moveq #1` X=0-avoidance. Parameterize size +
tile source only (the B2-1 contract: d5=link, a4=cursor).

**BAR:** byte-identical at every consumer site, both shapes, before/after — this is
a splice promotion, not a rewrite. If identity is unreachable at any site, that
site keeps its inline form with a logged reason (do not force).

## P2 — stride-hardening ensure() cluster (ledger 1362; C2-1/2/3/4)

Comptime assertions that derive or check the hand-encoded facts:

- `:147,:225` `lsl.w #6` ⇔ `SPRITES_PER_BAND*2 == 64`
- `:30-36` 8-byte clears ⇔ `PRIORITY_BANDS` / `SCANLINE_BANDS`+pad sizes
- `:325` `lsr.w #5` ⇔ `SCREEN_HEIGHT/SCANLINE_BANDS == 32`
- `:16-17` `SPRITE_MASK_SIZE` ⇔ `SPRITE_MASK_HEIGHT` agreement ((code+1)*8)

Zero emitted bytes. **NEGATIVE PIN required:** doctor one constant in a test → the
ensure() fires naming the site (the teeth proof).
Also: fix the `:31` "pad byte" comment vs ram.asm:245 (present-tense fact only).

## P3 — Sprite_Cycle_Counter low-byte const (ledger 1373)

Named `_lo` const replaces the literal `+1` + compensating comment. Byte-neutral
spelling; keep the `Sym+const` EA feature's width behavior identical.

## P4 — frame_piece_count holdout (B1-2) — REVIEWED decision, not mechanical

sprites reads `FRAME_PIECE_COUNT(a3)` inline vs the shared helper (frames.emp:23).
Addressing-mode mismatch means adoption may not be drop-in; adopt only if
byte-identical, else KEEP with the exception comment stating the real reason.

## OUT OF SCOPE (pre-ruled)

Newtypes (item-13 lane: MappingFrame/VramTile/VdpY-ScreenY); records-over-
streaming-cursors + frame-resolve construct (language-design asks, own pass);
C1-2 lea micro-opt (skip-logged — would break byte-neutrality); B1-1 clear_longs
swap (byte-changing + module-private).

## Acceptance

Full paired strict green; sprites_port + rings_port byte gates green (unchanged
ROMs); ensure() negative pin demonstrated; zero new clippy; ledger rows
1355/1362/1373 marked closed; close packet per house format (per-pass breakdown +
neither-bucket headlines).

**Sequencing: MUST NOT run concurrently with item-13** (shared sprites.emp). The
boundary-crossing transition parcel is file-disjoint and may run in parallel with
either.
