# R-A1 ring-cull boundary — live confirmation (oracle)

**Date:** 2026-07-13. **Observation-only** (no code changes). Closes the ledger's
R-A1 handoff row (campaign-gap-ledger.md ~989). ROM: fresh `s4.debug.bin` off
master `101dd06` (A2 latch merged), md5 `0c1c6fab…`. Scene-pin hook
(`Debug_Scene_Freeze`) is the enabler.

## Result: all 8 cull boundaries match the derivation EXACTLY — zero off-by-one. X=0 mask guard confirmed unreachable (defensive dead code).

## Method

OJZScroll, settled + paused; camera pinned `Camera_X=96 / Camera_Y=144` (player
at world (256,256) = screen centre). One controlled ring at `Ring_Buffer[0]`
(sentinel section/list ids `$FF`), `Ring_Count=1`, `Debug_Scene_Freeze=1`; all
writes landed while paused, then press-only 4-frame steps. `screenX = engine_X −
Camera_X`; SAT X = screenX+120, SAT Y = screenY+120 (post camera-bias fold).
Active sprite = the SAT **link chain** from sprite 0 (stale non-chained VRAM
entries persist and MUST be ignored — the player's `link=0` when no ring is
appended is the culled signal). Player marker = tile `0xA3F8` at fixed SAT
X=280; ring = tile `0xA3E8..0xA3F4` (anim frames 0–3). Cull math read from
`rings.asm` DrawRings: X `d0=screenX+8` cull if `>336`; Y `d0=screenY+8` cull if
`>240` → draws iff screenX∈[−8,328], screenY∈[−8,232].

## X boundary (engine_Y=184 / screenY=40 fixed)

| screenX | engine_X | predicted | OBSERVED | SAT evidence (active chain) |
|---|---|---|---|---|
| −9  | `$0057` | culled | **CULLED** | sprite0=player `link=0`; no ring appended |
| −8  | `$0058` | draws  | **DRAWS**  | player`link→1` → ring `0xA3EC` at **SAT X=`$70`=112** (=screenX+120) |
| 328 | `$01A8` | draws  | **DRAWS**  | ring `0xA3EC` at **SAT X=`$1C0`=448** |
| 329 | `$01A9` | culled | **CULLED** | player `link=0`; no ring (entry1 X=448 is stale from the 328 run) |

## Y boundary (engine_X=196 / screenX=100 fixed → SAT X=`$DC`=220)

| screenY | engine_Y | predicted | OBSERVED | SAT evidence (active chain) |
|---|---|---|---|---|
| −9  | `$0087` | culled | **CULLED** | player `link=0`; no ring |
| −8  | `$0088` | draws  | **DRAWS**  | ring `0xA3F4` at **SAT Y=`$70`=112** (=screenY+120), SAT X=`$DC` |
| 232 | `$0178` | draws  | **DRAWS**  | ring `0xA3F4` at **SAT Y=`$160`=352**, SAT X=`$DC` |
| 233 | `$0179` | culled | **CULLED** | player `link=0`; no ring (entry1 stale from the 232 run) |

Every transition fires on the exact predicted pixel. The three-round paper proof
(cull-math + byte gate + SAT-emit read) is now live-confirmed. Calibration ring
(screenX=100, screenY=40) emitted at SAT (220,160) = (screenX+120, screenY+120),
validating the fold + SAT read before the boundary sweep.

## X=0 SAT-mask guard probe

`tst.w d2 / bne .x_ok / moveq #1,d2` (rings.asm:188) forces SAT X 0→1 to dodge VDP
first-column sprite masking. SAT X = `d2` = screenX+120, so **SAT X=0 ⟺
screenX=−120** — the unique value. Constructed it: `engine_X=$FFE8` (−24),
Camera_X=96 → screenX=−120 → `d2=0` *if it drew*. **OBSERVED: CULLED** (player
`link=0`, no ring in chain) — the X cull (`d0=screenX+8=−112`, unsigned `$FF90 >
336`) skips the ring before the SAT write, so the guard never executes. For any
DRAWN ring screenX∈[−8,328] → d2∈[112,448], never 0. **The guard is unreachable
post-cull → defensive dead code.** No reachable path found; the cull and the mask
do NOT interact in an unmodeled way.

**Recommendation (no change made — observation session):** give the X=0 guard a
site-comment `defensive — unreachable post-cull (SAT X = screenX+120 ∈ [112,448]
for drawn rings; screenX=−120 is always culled)`. Optional micro-cleanup: the
guard could be removed for −4 B, but it's a defensible cheap insurance against a
future cull-window change — the comment is the lighter call.

## Ledger

- R-A1 ring-cull live-confirm handoff row → **CLOSED** (all boundaries confirmed,
  no bug). The parent R-A1 row's "verified by derivation, live confirmation owed"
  is now fully live-verified.
- Bug-2 (grounded-wall-push) stays OPEN (needs a terrain level; unrelated).

Nothing merged, no pushes.
