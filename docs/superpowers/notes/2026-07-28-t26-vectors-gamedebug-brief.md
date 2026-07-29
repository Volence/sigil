# t26 BRIEF — vectors.asm + game_debug.asm (two lanes)

**Overseer: Fable (direct dispatch). Date: 2026-07-28.** Bases at cut: aeon `d696792` /
sigil `8b0d29a`. Canonical plain `c51342d0`/421041 · debug `992d9e7d`/429102; strict
baseline **2620/0**. Census input: the 2026-07-28 game-side recon (agent, adjudicated);
its three framing corrections are ACCEPTED (backlog is ~20 code files + 4 config, not
~10; game_debug kills row 33 ONLY, row 45 is game.asm's gameBootHook; vectors.asm is
ENGINE-owned) and ONE census claim is OVERRULED BY OVERSEER VERIFICATION: the census
read `Debug_MusicToggle` "at $55A4" in the plain listing as emission — those are AS
listing ECHOES of skipped ifdef lines (address frozen, no byte column).
**game_debug.asm emits ZERO bytes in BOTH canonical shapes** (`SOUND_DEBUG_HOTKEYS`
defaults 0, build.sh:59) — it is the sound_debug config-shape class, minus the parser
blockers (plain 68k code, ordinary constructs).

## LANE A — `engine/system/vectors.asm` → `vectors.emp` (engine; the dc.l shakedown)

48 lines, `org 0`, exactly $000-$0FF (64 × dc.l), `__BUDGET_VECTORS`. Every target an
engine symbol, most already .emp-owned (EntryPoint, BusError..ErrorTrap/ErrorExcept
from t25, VBlank_Handler, HBlank_Vector_Slot, NullInterrupt) + SYSTEM_STACK. Source
shape-invariant; the VALUES differ per shape via the link (normal). The region's fixed
256-byte size makes the header.inc boundary structurally byte-neutral.
- **P-A1 (step 0, blocking):** the t25 `dc.l <label>` capability vs vectors.asm's FOUR
  comma-list lines (4 labels per dc.l, lines 40-47). Probe first; if comma-lists are
  unsupported, that is the lane's one demanded feature (TDD, small).
- Region gate: windowed `[0, $100)` both shapes + gate-off dual rebuild exact. This
  region PRECEDES EVERYTHING — a byte delta here is impossible by construction ($100
  fixed) but state the bar anyway.
- NullInterrupt stays engine.inc-inline (out of scope); ErrorTrap/ErrorExcept resolve
  against error_handler.emp module-to-module under both gates — the first .emp→.emp
  vector reference; prove both gate states.

## LANE B — `games/sonic4/debug/game_debug.asm` → `game_debug.emp` (the first game-side .emp CODE module)

117 lines, ONE proc (`Debug_MusicToggle`) + `Dbg_SfxIdTable` (8 bytes), whole file
under `SOUND_DEBUG_HOTKEYS && SOUND_DRIVER_ENABLED`. Seam: 4 call sites into
sound_api.emp (`Sound_PlayMusic/PlaySFX/PlayRing/StopMusic` — the proven class);
reads game RAM `Dbg_Music_On`/`Dbg_Sfx_Sel` (config/ram.asm:8-9) + game consts
`SONG_*`/`SFXID_*`/`BUTTON_*` (the game-contract symbol surface — mirror class, jot
rows as needed). Engine-side caller: `game_loop.emp:37` `jsr Debug_MusicToggle`
inside a comptime `if SOUND_DEBUG_HOTKEYS == 1 && SOUND_DRIVER_ENABLED == 1` arm over
the ungated extern decl at game_loop.emp:13 — **KILL ROW 33's target: the decl dies
same-commit when the module exists** (work the extern-vs-import split per gate state:
gate-OFF at the hotkeys shape must still resolve against the AS twin's symbol).
- **Proof machinery = the t21-named off-canonical twin-parity class** (AS-side ROM as
  oracle for comptime arms with no reference ROM; vblank's mirror-shape gate is the
  precedent): reference build `DEBUG=1 SOUND_DEBUG_HOTKEYS=1`, windowed byte gate at
  that shape, + CANONICAL-EMPTINESS proofs both shapes (t22
  `..._plain_region_is_empty` template, here for both). If this machinery balloons
  past LEAN: STOP and report (t25 rule) — do not build a monument.
- game_debug sits in the engine block BELOW $10000, unshielded — at the hotkeys shape
  its emission shifts everything downstream; the off-canonical oracle build absorbs
  that by construction. The $8000 bank-shift bar applies AT THE HOTKEYS SHAPE — state
  the check.

## EXPECTED BYTE MOVEMENT: ZERO (both canonical shapes, every commit)

Lane A is structurally fixed-size; lane B emits nothing canonically. Nonzero canonical
delta = STOP-and-report. EndOfRom 0x5DB60/0x5F65A unchanged.

## LOOP + PANEL + BARS

Full loop, LEAN, lanes ordered A then B. Step 2 lawful targets: essentially none in
lane A (pure data; comment/format only); lane B normal (branches/widths at the hotkeys
shape ride the off-canonical oracle — byte-locked there, so relaxations are LOGGED not
taken unless the oracle build proves them in lockstep). PANEL: **A1 + B1 + C2; C1
INACTIVE** (vector table = data; hotkey handler = human-timescale debug path — recorded)
**; C3 INACTIVE** (no VDP/DMA/Z80-bus content; sound_api calls are the proven seam —
recorded). Standing bars: positive controls on every negative probe; strict paired at
the branch trees; worktrees `port-tranche26`; editor-dir rsync on the fresh aeon
worktree; per-Bash cd; no cross-repo git compounds; checkpoints (a) after steps 0-2
STOP / (b) after passes+panel STOP / (c) merge gate = overseer. Ledger duties at
close: kill row 33 KILLED; row 35/45 NOT touched (state why); census corrections
recorded (the ~20-file backlog figure supersedes "~10" in prior notes); the
config-shape proof class gets a named row if new machinery ships.
