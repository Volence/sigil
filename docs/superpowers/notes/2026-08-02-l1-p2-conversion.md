# L1 P2 — the game-contract CONVERSION — close packet

Branch `l1-p2-conversion` (paired: sigil + aeon). Spec
`specs/2026-08-02-l1-game-contract-design.md` §8 P2. Gated on P1's countersign.
The engine now DECLARES the `Game` interface; both games BIND it in `.emp`
manifests; the boot/game_loop/camera mirrors die; `games/*/config/game.asm` ×2
are DELETED. No merge — gate-green branches + this packet for the overseer.

Strict suite **2941 / 0 / 4** (2938 baseline + 3 new whole-build probes; no
tests net-retired — the isolated port oracles were RE-POINTED in place).
`refreeze --check` OK (tip `l1-p2-game-contract-conversion`, chain len **19**).
`repin --check`: **pins.rs unchanged** (Config-A is not a pinned canonical shape).

## What flipped where

### Engine (aeon)
- **`engine/system/game_contract.emp`** (NEW, module `engine.game_contract`) —
  the `pub interface Game`. Pure declaration, emits nothing. Members:
  `const CAMERA_JUMP_LOCK: bool`, `const ENTRY_ID: u8`, `proc entry: GameState`,
  `hook boot_hook () clobbers(d0-d4/a0-a1) = empty`,
  `hook debug_tick () clobbers(d0-d7/a0-a6) = empty`.
- **`engine/system/boot.emp`** — the row-45 mirror block (22-byte inline
  ping+autoplay) → `invoke Game.boot_hook`; the handoff → `move.l #Game.entry,
  (Game_State).w` + `move.b #Game.ENTRY_ID, (Game_State_ID).w`. LOCKSTEP comment
  gone.
- **`engine/system/game_loop.emp`** — the row-9 mirror (`jsr Debug_MusicToggle`)
  → `invoke Game.debug_tick`; LOCKSTEP comment gone. `GameState` is now
  `pub type` (see the hoist note).
- **`engine/level/camera.emp`** — the four `GAME_CAMERA_JUMP_LOCK` sites →
  `Game.CAMERA_JUMP_LOCK` (bool form): the proc-body `if Game.CAMERA_JUMP_LOCK`,
  and the three comptime-fn sites `if !Game.CAMERA_JUMP_LOCK` (see the deviation
  note).

### Game (aeon)
- **`games/sonic4/config/game.emp`** (NEW, module `games.sonic4.game`) — the one
  `implement Game`: `CAMERA_JUMP_LOCK = true`, `ENTRY_ID = GS_OJZ_SCROLL_TEST`
  (`use`d), `entry = GameState_OJZScroll_Init`, and — under
  `if SOUND_DEBUG_HOTKEYS == 1 && SOUND_DRIVER_ENABLED == 1` —
  `boot_hook = SoundTest_BootPing`, `debug_tick = Debug_MusicToggle`.
- **`games/demo/config/game.emp`** (NEW, module `games.demo.game`) — the minimal
  manifest: `CAMERA_JUMP_LOCK = false`, `ENTRY_ID = GS_DEMO`,
  `entry = GameState_Demo_Init`; no hook binds.
- **`games/sonic4/debug/game_debug.emp`** — `SoundTest_BootPing` added: the
  ping+autoplay body moved VERBATIM out of boot.emp (`moveq #$3C,d0` / `bsr.w
  Sound_Ping` / `moveq #SONG_MOVINGTRUCKS,d0 as SongId` / `bsr.w Sound_PlayMusic`
  / `move.b #1,(Dbg_Music_On).w` / `rts`), where `SONG_MOVINGTRUCKS`/`Dbg_Music_On`
  are in-family names. `as SongId` added (the corpus enforces the typed slot in
  this module; byte-neutral comptime bless).

### Deletions + shrink (aeon)
- `games/sonic4/config/game.asm` and `games/demo/config/game.asm` **DELETED** —
  the last game-authored `.asm` carrying semantics.
- `games/sonic4/game_root.asm` + `games/demo/game_root.asm` drop the game.asm
  include + its combo-matrix comment block (both keep `debugger.asm`; sonic4
  keeps the mt_syms include).

### Harness (sigil)
- **`crates/sigil-harness/src/native.rs`** — `GameProfile` gains
  `manifest_module`; `synthetic_entry_src` `use`s `engine.game_contract` + the
  manifest (both reachable so the bind pass sees interface + implement). The ×4
  `("GAME_CAMERA_JUMP_LOCK", n)` emp_define hardcodes **RETIRED** (all four
  profiles) — the manifest is the single source.
- **`crates/sigil-harness/src/test_support.rs`** — `game_contract_env` helper
  (build an `InterfaceEnv` from synthetic interface+implement sources) for the
  isolated port oracles.
- **`crates/sigil-harness/m1c_root.asm`** — drops the game.asm include (matches
  `game_root.asm`; the vector table names no contract symbol).

### Compiler wiring (sigil) — how the whole build resolves the contract
- **`resolve/mod.rs` `build_program_with`** now runs the P1 `bind` pass over the
  reachable module set (ambient-prepending `implement` modules so their binding
  values read `use`d game consts — the deviation-3 fix) and threads the resolved
  `InterfaceEnv` into every module's lowering via the new
  `lower_module_with_region_ends_and_contracts`. A contract-free build yields the
  empty env — byte-identical to the whole pre-L1 corpus.
- **`lower/mod.rs`** — `contracts` threaded into `lower_item_guard` /
  `lower_equ_item` (and thus `eval_item_guard` / the new
  `eval_const_with_root_and_contracts`), which `seed_interfaces` — so a TOP-LEVEL
  comptime `ensure`/`equ`/`comptime fn` can read an interface member (the
  camera.emp deviation-2 fix). `seed_interfaces(empty)` is a no-op, so every
  contract-free path is unchanged.

## GameState pub-hoist home

`GameState` stays declared in **`engine.game_loop`** as `pub type GameState =
proc () clobbers(d0-d7/a0-a6)` — its dispatch-use home (`jsr (a0) as GameState`).
Contract types are module-local by the `use` import model (imports don't carry
`type = proc` decls), so the interface's `proc entry: GameState` resolves it via
the bind pass's whole-module-set type lookup (`find_contract_type` over
`all_items`), not a cross-module import. "Hoist" = made `pub`; moving the decl
into `game_contract.emp` would strand game_loop's own dispatch use.

## D6 final clobber bounds (verified against the real impls)

- **`boot_hook clobbers(d0-d4/a0-a1)`** — WIDENED from the spec's proposed
  `d0-d1/a0-a1`. `SoundTest_BootPing`'s honest closure is `d0-d4/a0-a1`:
  `Sound_PlayMusic` (sound_api.emp) clobbers `d0-d4/a0-a1`, whose d2-d4 leak past
  the ping's own d0/a0. Boot-time every register is free, so the widened bound is
  honest and the impl satisfies it exactly.
- **`debug_tick clobbers(d0-d7/a0-a6)`** — ⊤ as proposed. `Debug_MusicToggle`
  clobbers `d0-d4/a0-a1` ⊆ ⊤. Runs beside the GameState dispatch's own ⊤ bound.

## P1-deviation resolutions

- **Comptime-fn member refs (deviation 2):** chose option (a) — extend
  member-ref resolution to top-level comptime scope. `contracts` threaded into
  the `ensure`/`equ` eval entry points + `seed_interfaces`. camera.emp's three
  guard/equ comptime fns read `Game.CAMERA_JUMP_LOCK` directly; NO `-D` side
  channel. Additive and in the P1 architecture's spirit; every contract-free
  caller passes the empty env and is unchanged.
- **Bind cross-module scope (deviation 3):** `build_program_with`
  ambient-prepends each `implement` module before bind, so
  `ENTRY_ID = GS_OJZ_SCROLL_TEST` (a `use`d const) folds. Other reachable modules
  contribute plain items (proc/extern contracts for the hook-signature check, the
  `type = proc` contract types) at no ambient cost.

## Byte-identity ledger

**FIVE targets byte-identical to chain-18** (real builds + strict gates):
- s4 `5f72b9c3`/412134 · s4.debug `e6171a80`/421970 (aeon `build.sh` / `DEBUG=1`)
- demo `55b70266`/90576 · demo.debug `6487a47c`/93073 (aeon `build.sh demo`)
- config_b `947e4c57`/303555 (`config_b_anchor_matches_golden`)

**Config-A moved, re-frozen (anchors held):**

| | full_crc | full_size | anchor_crc | anchor_end |
|---|---|---|---|---|
| OLD (chain-18) | `f92f0333` | 422305 | `19b793ec` | 0x5f5f2 |
| NEW (chain-19) | `818bb109` | 422321 | `1ecd3443` | 0x5f5f2 |

- **anchor_end UNCHANGED** (0x5f5f2 / EndOfRom) — the assembled ROM region length
  held; every genuine org anchor held (`validate_placement` passed during the
  build; ANY anchor move would have been `[map.anchor-absent]`/`[map.undeclared-
  island]`). `config_a_anchor_matches_golden` byte-compares the whole
  `[0, EndOfRom)` window against the re-frozen golden — green.
- **anchor_crc CHANGED** — the internal re-layout: boot's 22-byte inline body
  became a 6-byte `jsr SoundTest_BootPing` (@ROM 0x396), the body (+rts, 20 bytes)
  now lives in game_debug (@0x647A), and the vectors + link-address immediates
  within the affected chained run shifted (e.g. `AnimateSprite` 0x3534→0x353C).
- **full_size +16** — the deb2 symbol appendix grew by the one new
  `SoundTest_BootPing` symbol.
- Re-freeze via the standing `refreeze --freeze` (rebuild 6 ROMs → re-derive
  off-canonical size tables → repin → provenance). Only `config_a.bin` +
  `config_a.txt` changed; the 5 identical goldens re-froze byte-identical;
  pins.rs unchanged. Chain 18 → 19.

## Test retirements / re-points (each named)

No test net-retired; the isolated single-module port oracles were RE-POINTED to
supply a synthetic contract env (`game_contract_env`), since the whole-build
native gates (`native_rom`, `native_offcanonical_*`) now exercise the real
conversion end-to-end:
- `camera_port.rs` (×4) — env with `CAMERA_JUMP_LOCK` bound; `-D
  GAME_CAMERA_JUMP_LOCK` dropped.
- `boot_port.rs` (×2) — env binds `ENTRY_ID`/`entry`; value seam renames
  `Game_Entry`→`GameState_OJZScroll_Init`, drops the `GAME_ENTRY_ID` equ (now a
  comptime const).
- `game_loop_port.rs` (×4) + `game_debug_port.rs` (×2) — env for
  `Game.debug_tick` (unbound for the canonical shape; bound to Debug_MusicToggle
  for the two-module-flip). `game_debug_port` adds `Sound_Ping` to its cross-seam
  carriers (SoundTest_BootPing's new call).
- `tranche5_negative_probes.rs` (×3 game_loop probes) — the env reproduces the
  MANIFEST's `if SOUND_DEBUG_HOTKEYS == 1 && SOUND_DRIVER_ENABLED == 1` bind from
  the SAME defines (the hotkeys gating moved engine→manifest).
- `slot_type_corpus.rs` — fixed by `as SongId` on SoundTest_BootPing (no test
  change).
- `native_offcanonical_full.rs` — the config_a `AnimateSprite` spot-check
  0x3534→0x353C (the re-layout slide; whole-anchor compare proves the golden).
- `m1c_vector_table.rs` — passes once `m1c_root.asm` drops the game.asm include.

New tests: **`crates/sigil-frontend-emp/tests/game_contract_build.rs`** (+3):
`whole_build_rejects_a_hook_that_clobbers_too_much` (THE required whole-build
negative probe — a manifest binding a hook with excess clobbers fails the BUILD
via `build_program_open_embed`, not just a unit `bind`),
`whole_build_rejects_an_unimplemented_interface`,
`whole_build_binds_clean_and_invoke_lowers_to_a_jsr` (+ its empty-impl
discriminator).

## Kill-rows closed

- **Row 9** (game_loop.emp gameDebugTick mirror) — CLOSED: `invoke
  Game.debug_tick`; the engine no longer names `Debug_MusicToggle`.
- **Row 45** (boot.emp gameBootHook mirror) — CLOSED: `invoke Game.boot_hook`;
  the ping+autoplay body is game-side.
- **Row 90** (Game_Entry / GAME_ENTRY_ID equalates) — CLOSED: `#Game.entry`
  (link ref to `GameState_OJZScroll_Init`) + `#Game.ENTRY_ID` (comptime const);
  the equalates and their file are deleted.

## step-3 / step-5 / neither

- **step-3 (retrospect / language asks):** the whole-build bind wiring makes the
  hook-signature contract a BUILD guarantee, not a remembered-to-run unit check —
  the drift class the row-9/45 combo matrices contained is now unrepresentable.
  One follow-up for the ledger: the hook-signature check silently SKIPS when the
  bound proc's contract isn't in the manifest's bind scope (an unbounded
  boundary); in the aeon build game_debug IS reachable at the hotkeys shape, so
  the real binds ARE checked — but a manifest that binds a hook to a proc whose
  module isn't reachable would skip. Tightening this to require the bound proc's
  contract be visible is an additive L5-adjacent follow-up.
- **step-5 (engine optimize):** the boot inline body → single boot-time `jsr` is
  a size-neutral behavior-preserving relocation (Config-A only); zero per-frame
  cost change. No other engine optimization in scope.
- **neither:** the `build_program_with`-runs-`bind` choice (vs. threading the env
  from the harness) makes the contract automatic for BOTH the native driver and
  the CLI whole-program path, and keeps the empty-env no-op structural — the
  reason the whole pre-L1 corpus stays byte-identical without per-test opt-out.
