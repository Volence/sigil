# 2026-07-29 — t30 brief: game-side G2 — the effect/child-lifecycle objects

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the census's G2 tranche (`2026-07-29-game-side-census.md` §G2, as STATUS-AMENDED
at t29 — read the G2 rows AND the t29 close packet's standing-pattern section first).

## 0. Bars (overseer-verified at dispatch)

- Masters: aeon **`c6a89a5`** / sigil **`e17b403`**, origin==local, clean.
- Canonical: plain **`c51342d0`/421041** · debug **`992d9e7d`/429102**; EndOfRom
  `0x5DB60`/`0x5F65A`. Strict baseline **2691/0 (1 ignored)**.
- Branches `port-tranche30` BOTH repos, worktrees `.worktrees/port-tranche30`, editor-dir
  rsync before first build, one shape per invocation, cd-every-call, no `git add -u`,
  explicit paths, never chain two repos' git in one compound.
- CANONICAL-BYTES tranche (the t29 class): step 1 byte-identical (delta ZERO expected —
  census: "Zero byte movement"); any step-2 flip deltas ride the standard wave discipline
  (one wave / one re-pin / row-1257 sweep / 5-site ripple / $8000 bar). Panel after any wave.
- Checkpoints (a) steps 0-2 STOP / (b) loop+panel STOP / (c) overseer-opened.
- Canonical loop text `notes/campaign-port-loop.md`; positive controls; kill rows
  same-commit; comments describe function; brace-indent. Context valve standing.
- **MERGE-QUEUE NOTE:** the `z80-rung2-contracts` branch is in flight (sigil-only) and
  merges BEHIND t30. Do not touch it; do not touch masters.

## 1. Scope (FIRM: the census G2 trio)

| Lane | File | Census facts (re-verify at step 0; PORTER-VERIFY rows are yours) |
|---|---|---|
| A | `games/sonic4/objects/test_emitter.asm` → `.emp` | Effect emitter; drives CreateEffect/AllocDynamic. |
| B | `games/sonic4/objects/test_stress_emitter.asm` → `.emp` | Stress emitter (the t24 step-5 measurement driver). |
| C | `games/sonic4/objects/test_churn.asm` → `.emp` | Alloc/delete churn driver (DeleteObject seam). |

No other files. The value of this tranche is proving the game→children.emp/core.emp
EFFECT SEAM at scale from the `.emp` side.

## 2. The effect seam (step-0 required reading, then verify)

- `engine/objects/children.emp` + `core.emp` are the callee side (t24). Read children.emp's
  module docs and kill rows 55-63's neighbours before porting the callers.
- t24 shipped behaviour fixes ON this seam (the effect `parent_ptr` dead-store deletion;
  COORDMODE inherit) and BANKED a known engine BUG (children of an entity-window-despawned
  parent are never freed) plus the POST-TWIN-RETIREMENT band-inheritance bucket. The G2
  objects are the test harness that exercises exactly these paths: if a transliteration
  choice interacts with a banked item, LEDGER the interaction — do not fix engine behaviour
  in a game-object port tranche (byte-neutral bar).
- Zero extern procs: everything resolves module-to-module or bare-link. A symbol that
  cannot = STOP-finding.

## 3. Template (the t29 STANDING PATTERN — copy it)

Per-file gates (`SIGIL_EMP_TEST_EMITTER` / `_TEST_STRESS_EMITTER` / `_TEST_CHURN`), ONE
shared test file (`test_g2_objects_port.rs`), ONE mixed-build fn (`mixed_tranche30`),
windowed byte gates both shapes + whole-ROM mixed arms + t24 doctored controls, regions
derived per lane with anchor situation stated, pins regenerated. Adopt — never re-roll —
`vram_art`, `pixels_to_coord` (row 49) where the promote idiom appears, and `vram_bytes`
(NOTE: if any G2 file consumes it, that is the SECOND consumer — the row-63 hoist condition
TRIPS and the hoist to the shared VRAM-layout home is IN SCOPE, executed byte-neutrally
with test_animated.emp updated in the same commit).

## 4. Panel ruling

**A1 + B1 + C2.** C1: these are per-frame spawn/churn drivers — the t24 step-5 measurement
regime ran ON them, so if step 5 finds any takeable, C1's activation question is LIVE;
flag your call with the named site either way (t29 pattern). C3 inactive (no hardware).
Lenses synchronous; dry adjudicated by panel.

## 5. Duties

Kill rows same-commit (gate arms/org pins → row-58 class; any const mirrors →
drift-guard + kill-row). Ledger sweep per pass; the item-13 objroutine/$FF-sentinel
demand counts grow if G2 adds sites — update the counts on those rows. Close packet with
per-pass findings + corrections; census STATUS AMENDMENT at close (G2 rows ported).

## 6. After t30

G3 (test_parent — the struct-overlay tranche the t29 overlay census seeds), then the P1
player keystone arc. The rung-2 checker cluster merges behind t30 when its implementer
reports.
