# 2026-07-29 — t41 STOP 1: the kill-row-35 reconciliation survey + T1 recon

Porter: t41 (Opus subagent, direct-dispatch). Overseer: Fable. This note is the
**STOP-1 deliverable** — a SURVEY, not a port. No engine code changed; no `.emp`
written. The fix-vs-carry decision on row 35 is the overseer's (possibly Volence's).

Branches `port-tranche41` in BOTH repos off the masters (sigil `fb52654` / aeon
`597ce06`), worktrees `.worktrees/port-tranche41`. Editor data rsync'd into the aeon
worktree before building.

## 0. Baseline verified (setup gate)

- **Plain** `./build.sh` → `s4.bin` **4b66cace / 421041** — EXACT canonical.
- **Debug** `DEBUG=1 ./build.sh` → `s4.debug.bin` **1c256b3b / 429102** — EXACT canonical.
- **Strict** (`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon worktree> cargo test --workspace
  --release --no-fail-fast`): **2888 passed / 0 failed / 1 ignored**, exit 0 — matches
  the brief's baseline exactly.

---

## 1. STOP 1 — the row-35 reconciliation survey

Row 35 = the OJZ harness per-frame mode-register force-write, `GameState_OJZScroll_Update`
**:234–273** in `games/sonic4/test/ojz_scroll_test.asm`. Kill condition: **B2
sub-decision (ii)** — the parallax engine owns a per-frame / on-active-change mode
write from `Parallax_Active_Config`, making the harness force-write redundant.

### 1.1 (survey part 1) Does the claim still hold against the CURRENT tree? — YES, structurally.

**Every writer of the mode-3 shadow / VDP reg $0B in the whole tree** (grepped
`engine` + `games`, `.asm` + `.emp`):
1. `ojz_scroll_test.asm:11` — init one-shot, `setVDPReg VDP_Shadow_vdp_mode3, #$02` (shadow only).
2. `parallax.asm:158` / `parallax.emp:229` — `Parallax_StartTransition`'s `.update_mode`,
   `setVDPReg VDP_Shadow_vdp_mode3, d0` (shadow only).
3. `ojz_scroll_test.asm:262` + `:268–273` — the row-35 harness force-write (shadow **and**
   a DIRECT hardware reg-$0B write).

There is **NO per-frame or on-active-change mode write anywhere in the parallax engine.**
`Parallax_StartTransition` is the sole engine mode writer and it is **edge-triggered**:
it runs ONLY from `Parallax_CheckBoundary` on a section-boundary crossing (parallax.asm
:61–95), and even then it short-circuits without writing the mode on three paths —
`a0 == NULL` (:118–119), `a0 == Current_Config` → `.recross_current` with nothing staged
(:120–121, :163–175), `a0 == Target_Config` (:122–123). So the same-config short-circuit
row 35 names is **present and unchanged.** Parallax was ported at t18 (`parallax.emp`, kill
row 5); the ported `.emp` carries the identical mode-write logic (`.update_mode`, :214–229)
— porting did not close the gap.

`setVDPReg` (macros.asm:251) writes ONLY the RAM shadow + a dirty-mask bit; the shadow→reg
flush is a separate VBlank step. The harness's DIRECT reg-$0B write (:270–273, with a
`VDP_CTRL` read to reset the command state machine) bypasses that flush — a SECOND concern
(VDP-command-state safety against a half-finished 32-bit address command from
`Section_UpdateColumns`), distinct from the staleness the shadow re-write addresses.

**Structurally the gap EXISTS.** `Parallax_Active_Config` (the intended fix hook, parallax
.asm:193 / .emp:269) already exists and is already consumed by `buffers.emp:249`
(HScroll build) — but NOT for a mode write.

### 1.2 The load-bearing question — is the force-write EXERCISED by shipped content? — NO.

This is the nuance the fix-vs-carry decision turns on:

- **Only ONE config is ever active in the shipped ROM.** The act descriptor
  (`games/sonic4/data/levels/ojz/act1/act_descriptor.asm`) sets **every** section's
  `sec_parallax_config` = `0` (= act default), with in-line comments recording that the
  windy / sky-haze / caves / LockedClouds per-section fixtures were **all superseded by the
  Deep Forest BG** (:81/98/115/132). The act default is `ParallaxConfig_OJZ_Default` (:54).
- Therefore `Parallax_CheckBoundary` never resolves a config DIFFERENT from Current on any
  crossing → `Parallax_StartTransition` always hits the `a0 == Current_Config` short-circuit
  → **a mode-changing transition NEVER fires in the shipped ROM.** The mode-3 shadow is set
  once and never legitimately changes.
- So the harness force-write at :234–273 re-asserts an **unchanging** value every frame in
  the shipped ROM. It is **effectively dead-insurance for shipped content**, load-bearing
  ONLY in the general multi-config case (which `ojz_windy.asm` — an "F3 fixture: single-band
  BG H-deformation", mode bits differing from OJZ_Default's per-line — was built to exercise
  but is no longer wired to any section).
- Corroborating: `ojz_default.asm:20–24` carries an explicit **"MISDIAGNOSIS"** note — a
  prior "$0B shadow→register propagation" explanation for the force-write was retracted
  ("VDP reg $0B reads $02 correctly in per-cell — propagation is fine"). The surviving
  rationale is purely the edge-triggered-staleness case in the header at :234–239, which the
  content above shows is currently un-exercised.

### 1.3 (survey part 2) The engine-fix shape — scoped, NOT built. Byte-CHANGING, wave + oracle-A/B class.

Sub-decision (ii): give the parallax engine a per-frame (or on-active-change) mode-3 write
derived from `Parallax_Active_Config`, then delete the harness force-write.

- **Site:** most naturally a few instructions in the per-frame parallax entry
  (`Parallax_Update`, called every frame from ojz Update:276) or a new small proc it calls —
  reading `Parallax_Active_Config` (already returns the active config in d0/Z) and computing
  the same mode bits `.update_mode` computes, then `setVDPReg VDP_Shadow_vdp_mode3`. Whether
  the engine also needs the direct reg-$0B write (the command-state-machine concern) or can
  rely on the normal shadow flush is an OPEN sub-question the fix must settle — the harness
  distrusts the flush here; an engine-owned write that only touches the shadow is a
  behavioral change from the harness's direct hardware write.
- **Cost class:** BYTE-CHANGING engine edit on `parallax.emp` + its lockstep twin
  `parallax.asm` (kill row 5) + re-pin (the PARALLAX region, kill row 6). It sits on the
  **per-frame hot path** (`Parallax_Update` runs every frame) → step-5 C1 (cycle) ACTIVE and
  C3 (VDP/timing) ACTIVE, plus the **oracle A/B** rider (a behavior change a byte gate cannot
  see — the two twins would agree). It also RE-touches an already-merged engine file, so it
  is a wave with the full ripple checklist, and it is arguably a **standalone hardening
  parcel on an already-ported file** (the parcel-scope amendment class) rather than t41's own
  file — i.e. it may not belong inside t41 at all.
- **The oracle A/B must confirm two things:** (a) the engine now writes the same mode the
  harness did (no render regression on the ojz boot scene), and (b) the extra per-frame
  engine write does not perturb the **deterministic cache-fill soak** ojz drives under
  `Debug_Scene_Freeze` (§4 below) — that soak is a live oracle instrument.

### 1.4 (survey part 3) The carry-as-is alternative — port the compensation faithfully.

Port `ojz_scroll_test.asm` → `.emp` including the :234–273 force-write verbatim (bytes
proven at step 1), present-tense-commented (no history narration per the comment-rule), and
**row 35 STAYS OPEN** against the parallax fix. This is behavior-IDENTICAL to today (the
harness keeps its dead-insurance write), costs no engine wave, and keeps t41 a pure
game-side conversion tranche. The `.emp` comment states the contract as a fact ("re-asserts
mode-3 from the active config every frame; redundant while all sections share one config,
correct when they do not"), and row 35's kill condition is unchanged.

### 1.5 Survey summary for adjudication (facts, not a ruling)

| axis | engine-fix (sub-decision ii) | carry-as-is |
|---|---|---|
| gap closed | yes (structural) | no (row 35 stays open) |
| bytes | changes parallax.emp+.asm, re-pin, PARALLAX region | zero engine change; ojz ported faithfully |
| verification | full wave + C1/C3 + **oracle A/B** (2 checks) | ojz step-1 byte gate both shapes |
| touches merged file | YES (parallax — hardening-parcel class) | no |
| shipped-content effect | none today (transition never fires) | none (identical to today) |
| fits inside t41 | arguably a standalone parallax parcel, not a T1 file | yes — clean game-side tranche |

**Porter's read (advisory only):** the gap is real but **un-exercised by shipped content**,
and the fix is a byte-changing edit to an already-merged engine file — i.e. it smells like a
standalone parallax-hardening parcel (parcel-scope-amendment class, deferrable to the
post-conversion optimization sweep), not a T1 harness-state file. Carrying the compensation
as-is keeps t41 a clean final game-side conversion and leaves row 35 open with its kill
condition intact. But this is the overseer's call — the gap's structural presence is a valid
reason to close it now while parallax is in view. **STOP for adjudication.**

---

## 2. T1 recon — per-shape byte deltas (from the fresh listings)

Both files are **shape-DEPENDENT** (distinct plain/debug region lengths — confirms the
census's DEBUG-divergent class; each needs SEPARATE plain/debug pins + windows). Addresses
from `s4.lst` / `s4.debug.lst` (built this session, freshness-cross-checked against the
canonical CRCs above).

| file | plain region | plain len | debug region | debug len | debug Δ |
|---|---|---|---|---|---|
| `object_test_state.asm` | $5C230–$5C7EC | **$5BC (1468)** | $5DC82–$5E2DA | **$658 (1624)** | **+$9C (+156)** |
| `ojz_scroll_test.asm` | $5C7EC–$5CAAE | **$2C2 (706)** | $5E2DA–$5E5A8 | **$2CE (718)** | **+$C (+12)** |

Region-end boundaries confirmed: object_test_state ends where `GameState_OJZScroll_Init`
begins ($5C7EC / $5E2DA); ojz ends after `PlayerMarkerTile` (128 B) at $5CAAE / $5E5A8,
where the level-1 (main.asm) include resumes. Order in `gameStatesIncludes`:
object_test_state THEN ojz_scroll_test (contiguous).

- **object_test_state's +$9C (156 B) debug growth is ENTIRELY the one `ifdef __DEBUG__`
  profiling block** (:91–136): three profile brackets around RunObjects / TouchResponse /
  Render_Sprites, each reading `VDP_HV_COUNTER` as a cycle-timing proxy. Notable that this is
  a LARGE debug delta from a single gate — the shape gate must window it precisely.
- **ojz's +$C (12 B) debug growth is the two `Debug_Scene_Freeze` skip-blocks** (:158–165
  camera, :177–184 entity-scan) — `tst.b (Debug_Scene_Freeze).w / bne.s .skip` each.

## 3. Census PORTER-VERIFY — corrections

The census (2026-07-29-game-side-census.md §3c) is now the record; verified against the tree:

1. **CENSUS MISCOUNT:** §3c and §3d call ojz "**5 gates incl `__DEBUG__`**". Actual: **4
   `ifdef __DEBUG__` / `endif` directives forming 2 logical skip-blocks**, and NO other
   conditional-assembly gates (no `ifeq`/`ifne`/etc.). The line 253 `moveq #%11` is a plain
   instruction, not a gate. → the census's "0-for-5" gate-count streak continues; correct
   figure is **2 `__DEBUG__` skip-blocks**.
2. Line counts CONFIRMED: object_test_state 365, ojz 310 (census right).
3. object_test_state "1 `__DEBUG__`" CONFIRMED (the profiling block).
4. Row-35 premise CONFIRMED but REFINED: the gap is structural-yes / exercised-no (§1.2) —
   the census's "entangled with OPEN kill row 35" is right; the refinement is that shipped
   content never fires the transition, which the census did not state.

## 4. C3 (VDP/DMA/timing) claim surface — ACTIVE on BOTH files

The census flagged lens C3 for both; confirmed — this is the first game-side C3-heavy
tranche since the engine block. Claim surface a C3 panel must audit:

**ojz_scroll_test** (VDP/DMA-facing, the game ENTRY state):
- `setVDPReg VDP_Shadow_vdp_mode3/mode2` (:11/:137/:262), `Level_LoadArt` (:37),
  `QueueDMA_Critical` (:44), palette copies to `Palette_Buffer` CRAM lines (:20–33, :231).
- The **VBlank-masked marker-tile VRAM copy** (:78–95): `move.w #$2700,sr` + `stopZ80` + raw
  `VDP_CTRL`/`VDP_DATA` writes — the comment claims a mid-copy VBlank retargets the VDP
  address and tail longs land wrong (observed 88/128 B), so ints are masked. HW-timing claim.
- The **row-35 mode-3 block** (:234–273): the VDP-command-state-machine-reset claim
  (:263–267) and the mode-bit derivation (:249–262). HW claim, audited under §1.
- `stopZ80`/`startZ80` bus wraps around the synchronous `Section_UpdateColumns` (:110–112).

**object_test_state** (VDP-facing via profiling):
- `setVDPReg VDP_Shadow_vdp_mode2` (:67/:249), `QueueDMA_Critical` (:26/:198), palette copies.
- The `VDP_HV_COUNTER` profiling reads (:92–136) — a HW-timing claim (the V/H counter as a
  per-frame cycle proxy), DEBUG-only. C3 audits the header claim "captures per-frame
  profiling via VDP V counter" (:4) against what the counter actually measures.

Panel per the brief: **A1 + B1 + C2 + C3 ACTIVE**; C1 conditional named-basis (both are
harness states — ojz is per-frame but the port is faithful/byte-neutral, so C1 is a
flagged-basis call at the gate, not a silent skip).

## 5. object_test_state as the oracle-A/B soak scene — behavior-preservation notes

Both harness files are LIVE oracle-A/B instruments; behavior must be preserved EXACTLY so
A/B stays a valid measuring stick (any byte-gate-blind concern names the oracle-A/B rider):

- **ojz_scroll_test IS `Game_Entry`** (`config/game.asm:46` `Game_Entry =
  GameState_OJZScroll_Init`) — the boot state. Its two `Debug_Scene_Freeze` skip-blocks
  (camera + entity-scan) are the **deterministic-cache-fill / frame-lock hooks** the oracle
  A/B uses (the `Debug_Scene_Freeze` 0xFF8A10 + Camera-poke technique for OLD/NEW
  byte-identity cache fills, and the Frame_Counter-anchored A/B). The port must keep these
  skips bit-exact (they gate on `Debug_Scene_Freeze`, DEBUG-shape only).
- **object_test_state is the object-CHURN soak vehicle**, entered at runtime by poking
  `GameState_ObjectTestChurn_Init` into `Game_State` ("the OJZ scroll test owns Game_Entry",
  :178). `GameState_ObjectTest` (:90) + `GameState_ObjectTestChurn` (:184, the A2 dynamic-pool
  soak) are the deterministic soak loops old/new ROMs run in lockstep for A/B. Determinism
  comes from the fixed churn pattern, not a freeze hook. Its DEBUG profiling block is itself
  the A/B measurement instrument — preserve it exactly.

Implication for §1: the engine-fix path (sub-decision ii) adds a per-frame VDP write inside
`Parallax_Update`, which runs in ojz's boot scene — so its oracle A/B must prove the extra
write does not perturb the `Debug_Scene_Freeze` cache-fill soak. The carry-as-is path has no
such risk (ojz behavior is byte-identical to today).

---

## 6. Post-STOP scope (for reference; blocked on adjudication)

After STOP 1 clears: port `object_test_state.asm` then `ojz_scroll_test.asm` → `.emp`,
shape-dependent gates per file (separate plain/debug pins/windows), the row-35 outcome
applied as adjudicated. Kill rows same-commit (2 gate/AS-twin rows + the row-35 disposition).
Close packet carries the census amendment: **the 68k GAME SIDE becomes code-complete** — only
main.asm (manifest) + the 4-file config cluster remain, and those are the Spec-5 flip itself.

**STOP for the overseer's adjudication of row 35 (fix vs carry). No port code written.**
