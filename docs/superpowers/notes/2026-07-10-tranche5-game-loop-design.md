# Tranche 5 step 0 — game_loop hazard rulings (2026-07-10)

The handoff's two hazard classes, reconned against the real tree and ruled.
game_loop.asm is 18 emitted bytes in both pinned shapes (GameLoop 16 +
GameState_Idle rts 2), but the rulings are the point, not the bytes.

## Recon facts (all verified in-tree)

- `SOUND_DRIVER_ENABLED`: build.sh defaults it **ON** (`${SOUND_DRIVER_ENABLED:-1}`);
  both PROVENANCE pins (`907a9029…` / `7148f938…`) are standard builds, so
  **both reference shapes have the drain call**. The demo game builds
  sound-off via its build.conf (demo/main.asm:6) — and takes the AS twin
  anyway (below).
- `SOUND_DEBUG_HOTKEYS`: build.sh defaults it **OFF** (`:-0`, independent
  env opt-in, requires DEBUG=1). Neither pinned shape sets it, so
  **`gameDebugTick` expands to ZERO bytes in both reference shapes**.
- sonic4's `gameDebugTick` body (games/sonic4/config/game.asm:66): a single
  `jsr Debug_MusicToggle` under `ifdef SOUND_DEBUG_HOTKEYS` +
  `ifdef SOUND_DRIVER_ENABLED`. NOT inlined code — already a call.
  The demo's body (games/demo/config/game.asm:29): empty, unconditionally.
- The gate site is **engine.inc:136** — the engine-side gate pattern is
  already established (controllers :121, math :139, collision_lookup :165),
  including the per-shape org resume and the "gate define must never be set
  for other games" law. Nothing new to invent.
- Define pass-through machinery exists end-to-end: mt_bank.emp's `DEBUG`
  comptime-if + `placed_module_sections(..., defines)` in the harness.

## Ruling H1 — the `ifdef SOUND_DRIVER_ENABLED` line

**Comptime `if` over a `SOUND_DRIVER_ENABLED` define, mt_bank pattern.**
The .emp emits `jbsr Sound_DrainSfxRing` when the define is 1, nothing
when 0. The gate define (`SIGIL_EMP_GAME_LOOP`) stays sonic4-shape-only
per the engine-gate law — the demo build keeps the AS twin, so the .emp
never has to serve the demo's shape through the gate.

**OFF-shape testing**: the full-ROM byte gates only exercise define=1
(both pins are sound-on; the engine.inc resume orgs are sound-on-shape
addresses, so a sound-off mixed build is out of gate reach by
construction). The define=0 spelling gets a **module-level byte test**
sigil-side: emitted section bytes == the 12-byte no-drain loop. Not a gap
— the same class as mt_bank's DEBUG=0 leg, which is also module-tested.

## Ruling H2 — `gameDebugTick` (the game-contract macro seam)

**Option (d), not in the handoff's list: mirror sonic4's macro EXPANSION
under the same comptime-if** — `if SOUND_DEBUG_HOTKEYS == 1 &&
SOUND_DRIVER_ENABLED == 1 { jsr Debug_MusicToggle }`. The AS body
deliberately says `jsr` (abs.l, placement-free) rather than `bsr.w` even
though the target is a fixed label; the .emp mirrors the EXPANSION
verbatim, so bare `jsr` it stays — with a comment naming the
placement-freedom rationale so the jbra/jbsr rule's reviewers don't
"fix" it. Zero bytes in both pinned shapes,
byte-exact in all four define combinations against the AS twin.

Why not the handoff's options:
- (a) split the file — pre-rejected by the handoff (gate-on shape loses
  the hook entirely).
- (b) macro → proc contract change — NOT byte-neutral (adds a `jsr` in
  the hotkeys-off shapes where the macro emits nothing), and it rewrites
  the engine↔game contract for BOTH games; that's structural-change
  territory (Volence has structural work queued; out of campaign scope).
- (c) `.emp` extern-macro construct — new language surface for a site
  whose active body is already a plain call; no real consumer demands it
  (tenet: no machinery before demand). If a future game-contract macro
  with a NON-TRIVIAL body needs porting, THAT is the construct's demand
  moment — ledgered, not built.

**The cost of (d), named**: game_loop.emp bakes sonic4's hook body into
an engine-owned file. That is a twin-scaffolding mirror of
games/sonic4/config/game.asm's macro body → **kill-list row lands in the
same commit** (kill condition: a ratified .emp game-contract-hook
mechanism — same Spec-5 neighborhood as kill-row-4 stage 2 — or GameLoop
migrating game-side). Drift guard: the hotkeys-on shape can't be
full-ROM-gated (no pinned reference), so the mirror gets a module-level
byte test per define combination; a comment at the AS macro body points
back at the .emp mirror (same-commit, lockstep law).

## Non-hazard notes

- `movea.l (Game_State).w, a0` / `jsr (a0)` — bare `jsr` is CORRECT
  (computed target; the jbra/jbsr rule reserves bare forms for exactly
  this). `bra.s GameLoop` self-loop → `jbra` (auto-reach picks .s).
- `bsr.w VSync_Wait` / `bsr.w Sound_DrainSfxRing` → `jbsr` (auto-reach
  picks .w; byte gate confirms).
- Port order within the tranche: game_loop FIRST (its drain reference
  crosses the seam to AS-side sound_api), THEN sound_api flips that
  reference to .emp↔.emp through the shared link — exercising both
  directions of the seam in one tranche, deliberately.

## Test plan (TDD order)

1. Module-level byte tests, all four define combos (11/01/10/00 —
   expansion goes 16/16-4/12+6/12 bytes… derive exact vectors from the AS
   twin assembled standalone with each define set).
2. Mixed-ROM gates both pinned shapes (define=1, hotkeys absent).
3. Negative probes: unknown-symbol steering when `Sound_DrainSfxRing` is
   misspelled; gate-off neutrality sha256 ×3.
4. Kill-list row + AS-side lockstep comment, same commit.
