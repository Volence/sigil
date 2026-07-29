# 2026-07-29 — t35 brief: game-side P2 — the player state machines (ground + air + spindash)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the census's P2 **and** P3 rows MERGED into one tranche (kickoff-ruled: the
keystone is in, the link fix landed, the three files are one consumer cluster of one
struct). Sigil master = THIS brief's commit; aeon master **`84d23c8`**.

## 0. Bars

- Canonical: plain **`37dd2bb2`/421041** · debug **`bbb822f6`/429102**. Strict baseline
  **2803/0 (1 ignored)**.
- Branches `port-tranche35` BOTH repos, worktrees `.worktrees/port-tranche35`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit paths,
  no `git add -u`, failures-first, rebuild-worktree-ROMs-after-rebase). CANONICAL-BYTES
  tranche: step-1 delta ZERO; any step-2/5 byte movement rides the FULL wave discipline —
  the loop doc's **Wave-Ripple Checklist is now canonical text** (act_descriptor
  self-gate orgs, hardcoded windows, pins, $8000 bar — follow it as written).
- The port-loop doc gained the nine ratified rules since t34 — **re-read step 2 items
  1-11 + Standing Patterns before writing a line** (module headers, symbol-resolution
  ladder, honest-contract derivation, bless-on-the-producer, bare-abs-EA A2 w/ 5 named
  exceptions, contract clause-order A1, canonical pin string A3).
- Checkpoints (a)/(b)/(c); loop text; t24 controls; valve standing. Parallel porter on
  `port-tranche36` (Z80 sequencer) — different files; merge order ruled at the gates
  (expect the overseer-executed rebase for the second merger).

## 1. Scope (FIRM: census P2+P3 merged)

- `games/sonic4/player/player_ground.asm` (783 L; `PState_Ground`:34, `PState_Roll`:322)
  → `player_ground.emp`
- `games/sonic4/player/player_air.asm` (470 L; `PState_Air`/`PState_AirBall`/
  `PState_RollJump`/`PState_Jump`/`PState_AirShared`) → `player_air.emp` — NOTE: this
  twin took the t34 wave (−0x12: slide + 4× dead-`ext.l` cut through the macro def);
  port the bytes that are THERE, tree wins.
- `games/sonic4/player/player_spindash.asm` (119 L; `PState_Spindash`:40) →
  `player_spindash.emp`
- NOT in scope: `player_sensors` (P4, separable engine-block region), harness states
  (T1), `player_common`/`sonic` (ported t34 — READ their .emp, import from them).

## 2. Known inputs (verify at step 0; tree wins)

- **Consume the keystone**: PlayerV / `_pl_*` / `PPHYS_*` and the 4 comptime templates
  live in `player_common.emp` — IMPORT, never redefine. `mask_opposing_lr` was built as
  a hygienic splice precisely for these files (expansion sites: ground ×2, air ×1).
- **Guarded-field survival re-derivation**: the 5 GUARDED PlayerV fields exist because
  these files read `_pl_*` from AS. As each file ports, re-derive guard survival PER
  FIELD against the remaining AS readers (sensors + anything else — grep, don't
  assume). Kill rows 72/74/75 are P4-CLOSE kills — do NOT retire guards early; update
  row conditions same-commit where a last-AS-reader dies.
- **Player_States offset table** (player_common.emp:542 region, extern-extern t31
  form): as each `PState_*` label becomes a real .emp definition, the `extern()` term
  converts — report the count. The `offsets` construct adoption stays LEDGERED with
  kill = ADOPT AT P4 CLOSE (t34 ruling) — do not adopt now. The AS twin's table
  comment "PSTATE_SPINDASH (sonic.asm)" is STALE (relocated; the .emp side is already
  correct) — a step-3 comment-accuracy item for the twin if touched, else note it.
- **The combined-link stale-fold fix is LANDED** (sigil `03d29cd`, all operand
  classes; t34's mixed gates un-ignored, 0-diff both shapes). P2 is UNGATED: the
  duplicate-local-label files (.keep/.abs/.draw class) MUST pass full mixed
  byte-identity BOTH shapes from the start — windowed-only acceptance was t34's
  concession, not yours.
- Computed dispatches through the table are typed (`PlayerState`/`PlayerHook`, t34) —
  bless-on-the-producer per the ratified rule; the slot-type gate will fire, blesses
  are demanded.
- The t30 callee-preserves oracle (`preserves(a0)` shape) — report every use; census
  consumer count grows.
- Newtype candidates (ground_speed, PSTATE_*): T2 waits per the 2026-07-23 ruling —
  LOG at step-2 item 6, do not adopt.

## 3. Template + panel

Per-file gates `SIGIL_EMP_PLAYER_GROUND` / `SIGIL_EMP_PLAYER_AIR` /
`SIGIL_EMP_PLAYER_SPINDASH` (main.asm arms at games/sonic4/main.asm:46-48 — plain
ifndef arms, NOT the keystone internal-gate pattern; these files are pure code); one
test file `test_p2_player_states_port.rs`, one mixed fn `mixed_tranche35`; windowed
both shapes + whole-ROM + t24 controls; regions byte-gate-adjudicated. Step-0 recon
note committed (no design STOP unless a census contradiction appears — the t34
stop-on-3-errors precedent is the bar for stopping). Panel **A1 + B1 + C2 + C1
ACTIVE** (census: hot per-frame physics — C1 runs with named sites, not
flagged-conditional; the overseer takes a hot-path second look at the gate); C3
inactive unless VDP claims appear. Dry by panel. Byte-gate-blind behavior changes (if
any step-5 item is authorized) carry the oracle-A/B rider per the ratified rule.

## 4. Duties

Kill rows same-commit; ledger per pass; close packet with per-pass step-3 vs step-5
breakdown + census STATUS AMENDMENT (P2+P3 ported → P4 sensors next, then T1);
gap-ledger sweep; corrections list. After t35: P4 sensors + the 3 objects + T1 on the
game front; rung 3 continues on the Z80 front.
