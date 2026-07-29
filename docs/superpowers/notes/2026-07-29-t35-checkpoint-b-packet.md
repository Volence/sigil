# 2026-07-29 — t35 checkpoint (b) packet (P2+P3 player state machines)

Porter: Opus subagent. Loop authorized after (a) countersign. Files:
player_ground.emp / player_air.emp / player_spindash.emp (+ AS twins) + main.asm.

## Evidence block

- **Branch tips:** aeon `c72038d` (over 7698f98 · 80ad407 · a72d271), sigil `79319ae`
  (over 77a8eb8 · bacaed1 · 72f163a). Both worktrees clean.
- **Canonical CRCs (NEW — a byte-moving step-2 wave landed):**
  plain **`4b66cace`/421041** · debug **`1c256b3b`/429102** (was 37dd2bb2/bbb822f6;
  sizes pad-absorbed). Gate-off dual rebuild reproduces both exactly.
- **Own strict suite:** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon-wt> cargo test --workspace
  --release` = **2817 passed / 0 failed / 1 ignored** (the 14 t35 tests + baseline;
  no regressions). Same count pre/post-wave (no new tests; the wave moved bytes).

## Step 2 — modernization + the −0x20 control-flow wave

- Control flow → new-style: `.emp` bare Bcc + `jbra`/`jbsr` (named bra/jmp→jbra,
  bsr/jsr→jbsr; no computed jmp/jsr in these files). AS twins hand-set to EXPLICIT
  widths matching asl's optimal selection — **KEY FINDING: sigil's AS frontend pins
  branch width / no relaxation** (unlike asl), so the AS twins MUST carry explicit
  widths (that is why t34 hand-set them; a bare-AS-twin build failed boot_port with
  208 "branch needs explicit size suffix" errors). Widths extracted from the asl
  listing by instruction order (skipping the `maskOpposingLR` macro-injected `bne`).
- 16 shrink sites = −0x20: ground −0xC (3 named-jmp→bra + 3 near .w→.s), air −0x10
  (4 + 4), spindash −0x4 (2 named-jmp→bra.w). One named jmp became bra.s (near:
  `jmp PState_Jump` in Player_Jump).
- Byte-neutral refinements: brace-indent the if-blocks (if@8/body@16, player_common
  style); Air_LandState contract clause-order (68k A1: `clobbers` before `out`).
- **Wave-ripple checklist (canonical instance):**
  1. reference rebuilt both shapes → NEW canonical 4b66cace/1c256b3b.
  2. org arms — main.asm 17 (3 player: $108A2→$10896 / $10B74→$10B58 / $10C14→$10BF4;
     sonic + 13 downstream object gates −0x20) + the **act_descriptor SELF-GATE class**
     (act_descriptor.asm:236/238 −0x20). Confirmed the $6408 engine-block arm and the
     $58000+ sound/error arms are FIXED (no $5xxxx pin slid).
  3. downstream slide: repin regenerated pins.rs (74 pins — object bank + data run
     −0x20; the 3 player regions' own shrinks).
  4. 5-site hand-check: mixed_dac_rom.rs 7 hardcoded reference windows −0x20 ($25xxx
     map / $14xxx OJZ / $10Fxx test objects) + the self-relative objroutine LITERAL
     `#(TestSolid_Main-ObjCodeBase)` $F74→$F54 (both shapes). engine.inc unaffected
     (org $10000 fixed). repin.toml — the 3 t35 regions already present. repin_pins.rs
     regenerates in-memory (pins.rs update suffices).
  5. $8000 bar: N/A — a SHRINK; object-bank plain==debug bases still coincide.
  6. neighbour canaries: all prior-tranche mixed + windowed gates green (tranche6's
     objroutine-literal + test_solid window were the two the wave first exposed —
     fixed).
  7. per-region delta table: ground $45A→$44E, air $2D2→$2C2, spindash $A0→$9C.
  8. no rebase (N/A).

## Step-2 checklist (items 1-11)

1. control flow → new-style ✓ (the wave).  2. no structural width pins in these files
(all converted; no mem-to-mem two-symbolic operands).  3. bare-abs-EA: clean — RAM
symbols bare (Ctrl_1_Held/Player_JumpBuffer/Player_Quadrant/Camera_Hold_Frames), no
explicit `(Sym).w` in code.  4. brace-indent ✓.  5. idioms walked: `Sst.field` bare
accessors (player_common style), contract clause-order fixed, movem-range clobbers
(`d0-d7/a1-a2`); N/A: dc.l-label, bareword bankid/winptr, raise_error.  6. type layer:
Angle adopted (GetSineCosine ×4 `as Angle` blesses, ground:181/439/686/851 region);
ground_speed→Speed + broader angle→Angle are T2-deferred (shift/add chains — the
2026-07-23 ruling), LOGGED as item-13 candidates.  7. module headers: role + gate +
"pure code" stated; no hardware/timing claims (correct — no bus access).  8. resolution
ladder ✓ (import player_common/engine.objects.sst/engine.types; extern proc ×7 for the
P4 sensors).  9. honest-contract ✓ (Ground_DetachState/Player_SlopeRepel widened to the
Player_SetState tail-clobber union `d0-d2/a1-a2`; no unverifiable preserves).
10. as-bless on producers ✓.  11. noticing: none new.

## The 3→4→5 loop — per-pass findings

**Pass 1**
- step-3(a) asks: (i) const+ensure mirror ceremony → the engine.constants hoist,
  ADJUDICATED at step 4 (below); (ii) the abs-value idiom `move;bpl;neg.w;.l:` (~8
  sites) → `abs_w` comptime-fn candidate, LOGGED/DEFERRED (marginal readability vs a
  twin-mirror + hygienic-label + kill-row cost); (iii) the crossing-zero friction
  pattern (Ground_Move + PState_Roll) → structural-clone candidate, LOGGED/DEFERRED
  (the two copies differ — roll adds the fixed brake); (iv) domain types ground_speed/
  angle → T2-deferred (item-13).
- step-3(b): the 6 session-codename comments (bug #4 ×3 / bug #5 ×2 / Task-6 ×1) →
  FIXED in `.emp` (rewrote to the adjacent behavioral reason); the research§/spec§/
  physics-classics§/feel-modern§ refs are DURABLE anchors, kept. The stale "(sonic.asm)"
  refs in player_ground twin → FIXED both twins (wave-touched). The player_common.asm
  Player_States table comment → NOT wave-touched, recorded.
- step-4: **hoist adjudication (per overseer ruling 1)** — the full engine.constants
  hoist has a large test-suite blast radius: every `constants.emp`-ambient test would
  then have to supply the new physics externs (`PHYS_ROLL_START_MIN` etc.) or its drift-
  guard ensures fail to resolve. RULING: DEFER to a dedicated cross-suite parcel AFTER
  t35 (never inside it); file-local mirrors stay (the 1st-consumer rung). Kill-list
  row 81 records it. No new twin mirror created this loop.
- step-5: interrogation run per hot proc (see panel C1). NO byte-changing cuts taken
  in-tranche — oracle A/B is unavailable this tranche (dispatch constraint), so behavior-
  affecting optimizations DEFER to the master step-5 backlog (gap-ledger §17 / the
  emp-port optimization review). Byte-neutral reorderings: none material found.

**Pass 2** — the pass-1 actions (codename/stale-comment fixes) created no new asks;
the remaining items are logged/adjudicated/deferred. Substantively empty → DRY claim,
subject to the panel.

## Dry panel (A1 + B1 + C2 + C1 ACTIVE — hot per-frame physics, C1 non-conditional)

One round, 4 read-only lenses over the 3 files. Adjudication:
- **C2 (correctness — the load-bearing lens): CLEAN. NO SEMANTIC DIVERGENCE.** Verified
  every gate-blind survival/clobber claim against the REAL callee contracts (GetSineCosine
  `clobbers() out(d0,d1)`; Player_SnapToSurface `d0/d2`; Player_SetState hook-preserves-d0;
  Sound_PlaySFX `preserves(d1/a0)`; the 4 sensor externs match player_sensors.asm headers;
  d4/d7/d2/d3 survivals honest) + every CC-chain (incl. the subtle `btst #0,Player_Quadrant`
  sets-only-Z so the move.w's N survives) + every falls_into/branch-target + the aliased
  empty PState_Air + the honest-contract widenings. Nothing to act on — the port is faithful.
- **A1 + B1 (independent) both ranked `abs_w` #1** — B1 refuted the deferral cost objection
  with the mask_opposing_lr/dist_to_fix precedent. → **BUILT** (step-4, byte-neutral, 9 sites;
  kill row 82). This RE-OPENED and was addressed in-loop.
- **C1 (perf): no byte-neutral in-tranche cut exists** (byte-gated port + oracle A/B down —
  every cycle win is byte-changing → master step-5 backlog; 10/12 duplicate §17). One item
  ELEVATED: **G9** — the byte-loaded-`d7`-used-as-word latent hazard, benign under current
  dispatch (d7=0), byte-gate-blind → LEDGERED as a correctness-hardening row (gap-ledger t35).
- **Deferred candidates (all ledgered):** copysign_w (2 clean sites, sub-threshold like the
  friction clone), S3K back-out-accel clone (byte-changing → §17), the d7 direction-decode
  dup (byte-changing → §17), QuadrantId / ButtonMask-vs-Bit newtypes (item-13, need new
  types), BUTTON_*_BIT adoption (waits on the hoist parcel — no new mirror).

**Dry status:** the one actionable byte-neutral finding (abs_w) was built; every other
finding is defer-class with a documented reason (correctness-clean per C2; perf byte-changing
+ oracle-down per C1; types T2/item-13-gated). A SECOND panel round on the abs_w-folded state
is cost-bounded-out (a clean construct adoption C2 already implicitly cleared) — flagged for
the overseer's call at gate (c).

## Duties

- Kill rows 79/80/81 written (renumbered at the overseer rebase: t36 merged first and holds row 78; the abs_w row is 82) (correction commit 77a8eb8 — the same-commit miss at the
  step-1 commit is logged here as a corrections row).
- Census amendment: **P2+P3 PORTED** (all 7 PState_* are real .emp defs; the offsets-
  construct adoption for the Player_States table stays ledgered to P4). NEXT on the game
  front: P4 player_sensors (retires the 7 extern-proc rows + the 5 PlayerV guards + the
  file-local mirrors' P4 arm), then the 3 objects + T1.
- Gap-ledger sweep: no new gaps beyond the logged step-3(a) asks + the hoist (row 81) +
  the step-5 backlog (§17, pre-existing).

## Corrections
- Kill rows were a same-commit duty missed at the step-1 commit (a72d271); written as
  the first post-(a) commit (77a8eb8) per overseer ruling 3.
