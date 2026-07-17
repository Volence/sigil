# Packet — sprites PB1/PB2 bugfix batch (→ Fable merge gate)

**Branch:** `fix/sprites-pb1-pb2` (both repos, seeded worktree)
**Date:** 2026-07-16
**Source:** the two correctness bugs surfaced by `aeon/docs/reviews/2026-07-16-emp-port-optimization-review.md` (sprites §4). Both Fable-verified against the code before this batch.

Two real bugs in `engine/objects/sprites.{emp,asm}`, twins in lockstep. Byte-CHANGING; region net-zero (PB1 −4, PB2 +4) so ROM size is unchanged both shapes.

---

## The fixes

### PB1 — frozen ghost sprites (`InitSpriteSystem`)
`InitSpriteSystem` (runs at the top of every frame, before RunObjects) cleared `Sprites_Rendered` every frame with `move.w d0, Sprites_Rendered`. That killed `Render_Sprites`' `.empty_table` had-sprites→none edge test (`tst Sprites_Rendered / beq .still_empty`): on a zero-sprite frame the counter was already 0 (cleared by init this same frame), so the transition never fired, the hidden SAT terminator was never written, and the previous frame's full SAT link chain persisted in VRAM → **sprites froze on screen**.

**Fix:** removed ONLY the per-frame `Sprites_Rendered` clear. Band-count + scanline-counter clears stay (genuinely per-frame). `Render_Sprites` now solely owns the value — `.done` writes `d5`, `.empty_table` writes 0, `.still_empty` leaves it. Cold-boot RAM clear seeds it to 0 for frame-one-ever (VRAM SAT also starts cleared, so `0 → .still_empty` is correct). A behavior comment documents the load-bearing "not reset here" invariant.

### PB2 — dead scanline budget (`Render_Sprites` band check)
The scanline-budget band check indexed `Scanline_Band_Sprites` from `d3`, which BOTH position paths bias to `screenY + 128` (the +128 SAT offset folded into the biased camera / added on the screen-coord path). The band math (`bmi` above-screen guard, `lsr.w #5` band index, `#SCANLINE_BANDS` below-screen guard) treated it as raw screen Y, so every band was shifted +4: sprites at screenY ≥ 96 were never budget-checked, 0–95 charged the wrong counter, above-screen sprites escaped the `bmi` guard — the soft heuristic was effectively non-functional over most of the screen.

**Fix:** `subi.w #VDP_SPRITE_Y_OFFSET, d0` unbiases `d0` to true screen Y before the guards/shift. `d3` stays biased for the downstream Emit position. Corrects the stale ":screen-relative Y" comment.

**Cost comparison (why subtract, not fold-into-bounds):** the subtract is one 4-byte `subi.w` (~8 cy), only on the rarely-taken budget branch (d5 ≥ SCANLINE_SPRITE_LIMIT=24). Folding −128 into the bounds needs two extra biased compare constants AND a band-index re-conversion for the array index — more instructions and more bytes. Subtract wins on both cycles and clarity.

---

## Commits

**aeon** (`master..HEAD`):
```
a3eef59 sprites PB1: stop clearing Sprites_Rendered every frame (frozen-ghost fix)
b0d8163 sprites PB2: unbias scanline-band index (dead-budget fix)
569df62 docs: track 2026-07-16 emp-port-optimization review (source of PB1/PB2)
```
**sigil** (`master..HEAD`):
```
e1e43d0 repin: DRAW_SPRITE -0x4 for sprites PB1/PB2 byte-changing fixes
3133c4a gap-ledger: sprites bugfix batch rows (review backlog + PB3/PB4)
```

---

## Byte gate (both shapes) + provenance

Region net-zero, so **no region base/len moved and no engine.inc gate orgs shift.** Only the one internal label between the two edit sites moves:
```
repin: 1 pin changed — DRAW_SPRITE  plain 0x2AB2→0x2AAE, debug 0x308C→0x3088 (Δ −0x4, −0x4)
```

Strict gate, `AEON_DIR` = the fix worktree, both shapes:
- `sprites_port` — **4/4 pass** (primary byte gate; .emp ≡ rebuilt .asm, plain + debug).
- `test_objects_port` — **2/2 pass** (the DRAW_SPRITE consumer; repin rerun hint).
- Full strict workspace suite — **all green, 0 failures** (incl. `repin_pins::pins_rs_is_current`).
- `clippy --workspace --all-targets` — **zero new warnings** (the 2 pre-existing warnings in `struct_field_disp_plus_n.rs` are identical on master; not introduced here).

Baseline was verified clean first (seeded worktree, `sprites_port` 4/4 against the unmodified build) before any edit.

**Provenance (fix/sprites-pb1-pb2, DEBUG-first then plain):**
| ROM | size | crc32 |
|---|---|---|
| plain `s4.bin` | 453087 | `e1ffead8` |
| debug `s4.debug.bin` | 461110 | `73038ec1` |

Baseline (master @ 21b0fcd, for the A/B): plain `453087/b335bdc6`, debug `461110/827e18c4`. **Sizes identical fixed↔baseline** (net-zero confirmed).

---

## Live verification (oracle, DEBUG shape)

Loaded ROMs hash-matched their build outputs; symbols from each build's `.lst`.

### Code sentinels (running ROM)
- **PB1:** fixed `InitSpriteSystem` = `…20C0 2080 4E75` — the two scanline `move.l`s go straight to `rts`; the removed `31C0 A130` (`move.w d0,(Sprites_Rendered)`) is **absent**. Baseline = `…2080 31C0A130 4E75` — clear **present** (bug confirmed).
- **PB2:** fixed budget check = `0C450018`(cmpi #24,d5) `6526`(blo) `3003`(move.w d3,d0) **`04400080`**(subi #128,d0) `6B1E`(bmi) `EA48`(lsr #5,d0) `0C400007`(cmpi #7,d0) — the unbiased band index, confirmed in ROM.

### PB1 mechanism A/B — the definitive live evidence
Same deterministic boot (reset → start×240), break at `Render_Sprites` entry (after this frame's `InitSpriteSystem` already ran), read `Sprites_Rendered`:

| | `Render_Sprites` entry addr | `Sprites_Rendered` at entry |
|---|---|---|
| **fixed** | `0x314A` | **`0x0003`** — survives init, holds prior frame's count |
| **baseline** | `0x314E` (+4, the shift) | **`0x0000`** — destroyed by init this frame |

This is the exact bug trigger: on the baseline the counter is gone before `.empty_table` can test it, so on a had-sprites→none frame it takes `.still_empty` and never terminates the SAT (ghost). On the fix it sees the prior nonzero count → writes the terminator. Combined with the source-verified `.empty_table` structure (`tst / beq .still_empty / else write terminator, Sprites_Rendered=0, dirty=1`) and the byte gate (ROM ≡ source), the terminator behavior is proven.

### Regression — normal-frame SAT unchanged
At the identical deterministic frame, `Sprite_Table_Buffer` (the SAT source, DMA'd verbatim to VRAM) is **byte-identical fixed↔baseline**:
```
00E80501 A3F80118 | 00850502 A3F0014C | 00850500 A3F0015C   (chain ends at sprite 2, link=0)
```
Light frame (`Sprites_Rendered=3` < 24 limit) → `Scanline_Band_Sprites` = all-zero on both: the budget path correctly early-outs, so PB2 is inert on normal frames and PB1 is behavior-neutral on non-empty frames. No regression.

### Lag frames
`Lag_Frame_Count` zeroed, 600 driven frames (right+down), both ROMs: **0 = 0**. No regression. (The demo boot has no max-speed scrollable level, so this is a no-regression confirmation, not a worst-case stress test — consistent with a net-zero-byte change that removes one per-frame store and adds one `subi` only on the rarely-taken budget branch.)

### Honest caveats — what the demo scene could NOT exercise live
The minimal demo boot carries persistent HUD/ring sprites via `DrawRings`, so `d5` never reaches 0 by zeroing object bands — a true **zero-sprite frame is unreachable**, so the `.empty_table` terminator-write could not be captured at the SAT level directly (the mechanism A/B above is the proof instead). Likewise the scene never reaches a **24+ piece overload**, so PB2's live band-charge distribution across bands 0–6 could not be observed (the code sentinel + byte gate carry PB2). Both need a crowd/transition scene not present in this boot; flagging rather than fabricating.

---

## Riders (byte-neutral)
- **aeon `569df62`** — tracked the previously-untracked `docs/reviews/2026-07-16-emp-port-optimization-review.md` (the review these fixes came from; the rest of its findings are step-5 optimization backlog).
- **sigil `3133c4a`** — 3 gap-ledger rows: (1) the review as the master unapplied step-5 backlog with its cross-file priority order; (2) **PB3** — the confirmed `sprSize` w/h swap in `engine/macros.asm:21`, latent until the first non-square use (sprites.emp doesn't inherit it), at-next-touch; (3) **PB4** — residual sprites.emp comment nits (`:27` pad wording + `.band_limit_pop` mask-skip note; the `:319` nit was closed by PB2), at-next-touch.

---

## Gate checklist for Fable
- [x] Both bugs fixed, both twins in lockstep, one commit per fix
- [x] Byte gate green both shapes, re-pinned (DRAW_SPRITE −4, net-zero region)
- [x] Full strict suite + clippy (zero new warnings), AEON_DIR at the worktree
- [x] Live: code sentinels (both ROMs) + PB1 mechanism A/B + regression SAT identical + lag 0=0
- [x] Riders committed (review doc + gap-ledger PB3/PB4)
- [ ] **Fable gate → merge** (aeon + sigil pushed together per campaign practice)

**Un-covered-by-scene (needs a crowd/transition scene, not this batch):** live SAT terminator capture on a real zero-sprite frame; live PB2 band-charge on a 24+ piece overload. Both carried by code sentinel + byte gate + mechanism A/B.
