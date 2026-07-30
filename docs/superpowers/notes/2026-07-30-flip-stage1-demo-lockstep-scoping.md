# 2026-07-30 — FLIP STAGE 1 · demo lockstep — scoping + the plain-golden drift

Status: **demo layout characterized; the native demo driver is a real sub-project
(demo-specific pins + engine-only registry), scoped here. Golden correction (overseer
own-run, RESOLVED): the PLAIN demo golden `2b71b37d/88738` was a STALE-ARTIFACT FALSE
BASELINE — there was NO drift. The first TRUE plain demo capture is `18c64002/90776`.**
No demo driver code yet — this de-risks it for the build session (analogous to the S1.4
investigation note).

## GOLDEN CORRECTION — the plain demo "drift" was a stale artifact (overseer own-run)

There was no drift. The overseer built a fresh DETACHED worktree at the OLD commit
`34023be` (the design-era baseline) and it ALSO builds demo plain = `18c64002/90776`,
byte-identical to the current tree. The `2b71b37d/88738` figure entered the record TWICE
as a stale artifact: (1) the design porter (aeon read-only) CRC'd a pre-existing
`demo.bin` lying in the main checkout; (2) the overseer's own "verification" ran
`GAME=demo ./build.sh`, but **`build.sh` takes the game POSITIONALLY** (`build.sh:4`,
`GAME="${1:-sonic4}"`), so the env var was silently ignored, sonic4 rebuilt, and the same
STALE `demo.bin` got re-CRC'd. The debug run used the positional form (`./build.sh demo`),
so `b0475a59/91584` was a REAL build and correctly "held."

**RULING (overseer):** the plain demo golden bar is **`18c64002/90776`** — the FIRST true
capture, NOT a re-capture of a drifted value. `b0475a59/91584` debug stands. PROVENANCE
must document the cause (stale-artifact false baseline, corrected by the two-commit fresh
derivation) so the history is honest.

**FREEZE-SCRIPT HARDENING (overseer, for the freeze stage):** the one-step re-capture
script must ALWAYS BUILD each ROM fresh via the POSITIONAL game arg (`./build.sh demo`,
`DEBUG=1 ./build.sh demo`) before capturing — NEVER CRC an existing file — and, if cheap,
assert the build actually ran (artifact mtime newer than the invocation, or a build-log
marker) so a stale-capture of this exact class is structurally impossible.

## The golden facts (aeon `bcb8f64`, freshly rebuilt both shapes)

`build.sh demo` / `DEBUG=1 build.sh demo`, deterministic (double-built):

| ROM | assembled (deb2 @0x11224) | appendix | full file | golden (design era) | status |
|---|---|---|---|---|---|
| demo.bin (plain) | **70180 / 0x11224** | 20596 B | **18c64002 / 90776** | 2b71b37d / 88738 | **DRIFTED** (appendix +2038) |
| demo.debug.bin | **70180 / 0x11224** | 21404 B | **b0475a59 / 91584** | b0475a59 / 91584 | HELD (byte-identical) |

Both shapes DO carry a real deb2 appendix at `0x11224` (`de b2 04 02`; `$1A4` bumped to
size−1 to span it — the same convsym pipeline as sonic4; the design note's demo appendix
figures were CORRECT, my mid-session "$1A4 says no appendix" was a misread — `$1A4` is the
POST-append END, i.e. appendix-PRESENT). The **assembled demo (70180) is byte-stable**
across the golden era → now; only the PLAIN symbol set grew (+2038 appendix bytes), drifting
the plain full-file CRC. This is exactly the OQ-A3 appendix-drift the design anticipated:
under the split-golden model it is a NON-issue (assembled == asl is the stable anchor; the
full-file golden re-captures to current asl). **Action: the frozen plain demo golden must
re-capture to 18c64002/90776** (debug stays b0475a59/91584). Both go into the freeze stage's
one-step re-capture, so this is automatic there.

## What the native demo driver requires (the sub-project)

Demo (`games/demo/main.asm`) is **sound-OFF** (`build.conf` `SOUND_DRIVER_ENABLED:=0`) and
supplies its OWN game side as residual AS (`demo_box.asm` objects, `demo_data.asm`,
`demo_state.asm`, `config/{constants,game,ram}.asm`); `gameEngineBlockIncludes` is EMPTY
(sonic4 put `player_sensors` there). It `include`s `engine/engine.inc`, so all 53 engine
gates apply. So the native demo build = **native `.emp` ENGINE modules + residual-AS demo
game side**, sound stack OFF.

**The blocker: demo's engine layout is ENTIRELY different from sonic4's** (sound-off + the
empty engine-block shift everything). Measured (asl demo vs asl sonic4, plain): EntryPoint
0x200 (same, boot) but GameLoop 0xAAE vs 0x239A, AnimateSprite 0x1520 vs 0x2F3C, BG_Init
0x3D1A vs 0x60BE, BusError 0x10174 vs 0x5CAB0 (the error handler sits right after the object
bank in demo). `ObjCodeBase` 0x10000 (fixed `org`). So **`pins.rs` (sonic4 layout) does NOT
apply to demo** — the native demo driver needs a DEMO pin set.

Concretely, the demo driver is a parameterized sibling of `native.rs`'s sonic4 driver:
1. **A demo pin table** — parse `demo.lst` / `demo.debug.lst` for the same region-boundary
   labels `pins.rs` uses (BOOT = EntryPoint..BootData, …), both shapes → demo region
   bases/lens. (846 in-range C symbols exist; demo layout is well-formed.) Analogous to
   `repin`, demo-scoped. This is the bulk of the work.
2. **A demo registry** — the ENGINE `.emp` modules only (drop every `games.sonic4.*`). Demo
   has NO game-specific `.emp` (demo objects stay residual AS). Re-examine the sonic4-specific
   bits: the OBJDEFS `"text"`-section guard (demo has no objdefs.emp), `AS_OWNED_KEYSTONES`
   (demo's are demo_box/demo_state, different), the drift-guard allowlist (demo gates off the
   same engine `.asm`, so likely the same VRAM_PLANE_B_BYTES/CAM_SCREEN_HALF set — verify),
   the embed paths, and `GAME_CAMERA_JUMP_LOCK` (demo's `config/game.asm` may not define it).
3. **The demo AS side** — `assemble_native_all_gates_as_side` rooted at `games/demo/main.asm`,
   `SOUND_DRIVER_ENABLED=0`, NO DAC/MT/SFX BINCLUDE gates, engine code gates ON, `__DEBUG__`
   per shape. `ensure_generated` is a NO-OP for demo (no sound stack).
4. **Gates** — reuse the S1.4 split-golden machinery verbatim (`build_native_full_file`,
   `convsym_resolve`, presence/determinism/t24): native demo assembled == asl demo (70180
   both shapes, the stable anchor); native demo full = sigil-canonical; freeze the current
   asl demo goldens (18c64002/90776 · b0475a59/91584 — the assembled-bar witnesses).

**Effort:** comparable to a fresh mini-driver — the demo pin derivation + the registry
re-examination are each a real chunk with byte-identity iteration (each sonic4-specific
hardcoding may fight). Not a re-pin; a parameterization. Flagged as its own build session so
it lands byte-passing rather than rushed (the valve). The S1.4 machinery + this scoping are
the head start; nothing here is a blocker, only unbuilt.

## Sequencing note

Demo is a Stage-2 precondition (Volence: lockstep). It precedes the golden freeze (which
covers all six ROMs incl. the re-captured plain demo golden). No aeon change needed (demo
builds through `sigil-frontend-as` for its game side — the PERMANENT residual-AS path).
