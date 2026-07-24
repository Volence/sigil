# TRANCHE 19 BRIEF — camera / bg / bg_anim conversion (LEAN tranche)

**Dispatch: overseer-cut 2026-07-24. Single-lane (merge queue EMPTY — t19 is
the only in-flight work; no coordination partner).** First tranche under the
corrected LEAN amendment (`ecb59d2`): in-tranche step 5 runs FULL
(interrogation + threshold-ruled cuts, the t18 H2 pattern); what t19 must NOT
do is spawn standalone optimization/hardening parcels on OTHER already-ported
files — those defer post-conversion (ledger rows instead).

**Canonical sources (read before cutting code):**
`docs/superpowers/notes/campaign-port-loop.md` (re-read at EVERY step boundary
— the changelog rule binds you to rulings ratified after your last read),
the t18 close packet (`notes/2026-07-23-t18-close-packet.md`) as the packet/
process template, and the campaign gap-ledger for the step-0 sweep.

## Scope

Port to `.emp` with full loop `0 → 1 → 2 → (3 → 4 → 5)* → 6 → merge`,
in this order:

1. **`engine/level/camera.asm`** (264 L; `Camera_Init`, `Camera_Update`) → `camera.emp`
2. **`engine/level/bg.asm`** (98 L; `BG_Init`) → `bg.emp`
3. **`engine/level/bg_anim.asm`** (131 L; `BgAnim_Init`, `BgAnim_Update`) → `bg_anim.emp`

Steps 0/1/2 run once PER FILE; the 3→4→5 loop and the dry-panel run over the
tranche; step 6 runs ONCE after the dry circuit; one close packet.

## Mechanics (the standing bars)

- **Branch:** `port-tranche19` both repos, worktrees. **Seed the aeon worktree's
  gitignored `games/sonic4/data/editor/` by rsync from the main checkout and
  verify CRC before trusting any build** (the +0x18000 padded-ROM trap).
- **Canonical (step-1 byte gate + gate-off neutrality): plain `0bfa5b79`/421161
  · debug `9d962703`/429204.** Build BOTH shapes per verification (one shape
  per `./build.sh` invocation; DEBUG=1 = debug only). Step-2+ byte changes pay
  the normal lockstep + re-pin + 5-site ripple; PROVENANCE re-baseline happens
  at merge.
- **Strict baseline: 2499/0** paired, `AEON_DIR` pointed at the BRANCH tree
  (paired-state gate — never aeon master). Shell cwd resets to the MAIN
  checkout on every Bash call — cd into the worktree on EVERY invocation; bare
  `cargo` in the wrong tree cost item-13 a re-run.
- Commit hygiene: explicit paths only (never `git add -u` in shared checkouts);
  check branch before every commit. Failures-first test reporting (never
  `grep|head` over suite output).
- **Emulator constraint (HARD): you must NOT drive the oracle MCP tools —
  subagent emulator calls deadlock.** All live verification (profiles, A/B,
  scroll drives) is run by the OVERSEER in the foreground session. When a step
  needs live data, STOP and return a named probe list (scene, pokes, measure
  windows, expected discriminator per probe) — results come back to you, then
  continue. Design probes with the standing techniques: scroll-TARGET drive /
  `Game_Entry`-flip soak / `Debug_Scene_Freeze` + camera poke (freeze skips
  `Camera_Update` + `EntityWindow_Scan` — that's WHY the poke survives; it
  also means a frozen scene cannot exercise the very code this tranche ports —
  camera-follow probes need the UN-frozen scroll-target drive). Never
  held-input-through-breakpoint. Frame-lock A/B to `Frame_Counter`.

## Step-0 hazard pre-sweep (overseer findings — VERIFY and complete the sweep;
the const-keyed trip-check over the kill-list/ledger is still owed)

**camera.asm**
- `GAME_CAMERA_JUMP_LOCK` conditional block references GAME-defined
  `_pl_state`/`PSTATE_JUMP`/`PSTATE_ROLLJUMP` — an engine/game-split
  conditional-assembly seam. Settle the `.emp` spelling in a step-0 design
  note (precedent check: how do ported files gate on game-defined symbols vs
  `DEBUG`?). The file's own comment states games without those symbols set
  the flag to 0 — the .emp must preserve that property (no unconditional
  reference to game symbols).
- `d4` is a reserved freeze-flag from the preamble through the X-clamp path to
  `.y_track` (site NOTE at :74). This is the conditional-register-reliance
  class the C2 lens exists for — contract-audit it, and treat any step-5
  reshuffle near it as high-risk.
- MEGA-ACT word-wrap comments (:135, :252) make claims about
  `act_descriptor`'s `ensure` guards — comment-claim-audit them against the
  descriptor as it NOW stands.
- Type layer: `Act_grid_w/h` reads and the `SECTION_SIZE_SHIFT` splits are
  shift/add chains — the value-flow test says LEDGER Coord-class candidates,
  don't force (Coord/Velocity are item-13 wave-2, A4-i-GATED, out of scope).
  GridX/GridY/SectionId exist where genuinely movable/comparable; log every
  untyped domain value per step-2 item 6 (a miss needs its logged why).

**bg.asm**
- VDP-heavy cold path: `sr=$2700` mask + `stopZ80` blocking copies. C3
  territory; typed VDP fns per the step-2 idiom list.
- The `move.w #$8F02, (VDP_CTRL).l` autoincrement site is an instance of the
  LEDGERED `set_vdp_reg` 4-site helper class (VDP_DATA/VDP_CTRL consolidation
  debt). This port's step 4 is a natural adjudication point: adopt/build if
  small and clean (byte-neutral), else ledger with demand incremented —
  name the decision in the packet either way.
- `BG_TILE_REGION_BYTES = VRAM_SPRITE_TABLE - BG_TILE_BASE_VRAM` and
  `VRAM_PLANE_B_BYTES` are VramAddr-class values — BUT the item-13 close
  ledgered `VRAM_PLANE_*` VramAddr typing as A4-i-GATED (address arithmetic)
  and `vram_bytes` as ledgered-not-created. Check those reopen rows BEFORE
  typing anything here; comptime-layer vocabulary only where it costs nothing.
- Capacity-guard comment block (:53-57) names build-tool expectations
  (`inject_editor_bg.py` 448) — comment-claim audit.

**bg_anim.asm**
- Calls `QueueDMA_Deferrable` — `dma_queue.asm` is UNPORTED, so this is a
  cross-`.asm` call seam (extern), fine as-is. When dma_queue ports in a later
  tranche it flips this symbol's ownership — that tranche owes the two-module
  link test, not this one; leave the seam clean for it.
- `BgAnim_Table` is runtime-read act data emitted by `inject_editor_bg.py`;
  the header comment's law — "nothing here may conditionally assemble on its
  symbols" — must survive the port verbatim as a real property.
- The record is walked by interleaved `(a3)+` pulls with hand-tracked offsets
  (the "banks at a3+6" magic comment, the `.skip_band addq #6,a3` resync, the
  3-word stack juggle around the two DMA queues + `.queue_full addq #6,sp`
  unwind). This cluster is simultaneously a step-3(a) ceremony candidate, a
  step-3(b) magic-number/name finding, and a C2 stack-balance hazard — expect
  the loop to spend its time here. A struct/offset-const spelling for the
  44-byte band record is a plausible step-4 build; the Python emitter is the
  OTHER twin of that layout — any shared-layout construct adds its
  twin-scaffolding kill row SAME COMMIT (the emitter drifts, the engine reads
  garbage).
- Driver select 0/1/2 (`Camera_X`/`Camera_Y`/`Frame_Counter`) is a closed
  vocabulary — domain-type scan candidate (comptime-side only; the runtime
  table stays raw words).

## Step 5 / dry-panel notes

- Hot path: `Camera_Update` + `BgAnim_Update` run every frame (`BG_Init` is
  level-load cold). Profile FIRST (named probes through the overseer), cut
  only ≥~1k cyc/f steady-state or user-visible, log-and-skip below threshold
  with numbers. Both procs are small — a "no cut, here's the per-line
  interrogation" outcome is likely and fine.
- Hot-path second look: the overseer (Fable) reviews camera.emp + bg_anim.emp
  before the merge gate — hold at the gate for it.
- Dry-panel composition: A1 + B1 + C1 + C2 + **C3 ACTIVE** (bg/bg_anim touch
  VDP/DMA/IRQ-mask/Z80-bus). Panel agents are read-only; findings adjudicated
  at the gate.

## Acceptance

Per-file step-1 gate list with named artifacts (byte gates both shapes, region
pins, mixed-build acceptance, negative probes, gate-off CRCs); full paired
strict green from the branch tree at every byte-changing commit; dry = full
3→4→5 circuit empty THEN a clean panel round; step-6 sweep as enumeration with
per-site outcomes; close packet per house format (per-pass step-3 vs step-5
breakdown + neither-bucket headlines); ledger/kill rows land same-commit as
what creates them. Merge is held at the overseer gate — checkpoint the packet,
do NOT merge or push masters yourself; the merge sequence (fetch-first
precondition, --no-ff both, rebuild-from-merged-master, PROVENANCE re-baseline,
strict from main checkouts, push, sweep) runs on the overseer's word.
