# 2026-07-29 — t28 brief: parser debt + the DEMAND-3 off-canonical machinery + the sound_debug mini-tranche

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target ruled by the overseer after t27 closed: the sound_debug mini-tranche was deferred at
t25 "until the parser-hang deep fix + the extern-in-lea parse gap close" — t28 closes those
(plus t27's sibling parser catch) and builds the whole-ROM off-canonical machinery whose
demand count reached THREE at t27 (game_debug hotkeys / sound_debug mirror / z80_init
no-sound). Kill row 42 is this tranche's kill target.

## 0. Bars (overseer-verified at dispatch)

- Masters: aeon **`20f1136`** / sigil **`ad60753`**, origin==local, clean.
- Canonical: plain **`c51342d0`/421041** (`s4.bin`) · debug **`992d9e7d`/429102**
  (`s4.debug.bin`); EndOfRom `0x5DB60`/`0x5F65A`.
- Strict baseline: **2675/0 (2 ignored)** paired (AEON_DIR at the matching tree),
  failures-first, explicit counts. NOTE: the two t25 parser-hang repros are `#[ignore]`d and
  uncounted — lane P1 un-ignores them, so the counted total will grow by those too.
- **EXPECTED CANONICAL BYTE MOVEMENT: ZERO at every commit** (STOP-not-absorb). Off-canonical
  shapes get their own oracles; they are not canonical bars.
- Branches `port-tranche28` BOTH repos, worktrees `.worktrees/port-tranche28`, editor-dir
  rsync before first build, one shape per invocation, cd-every-call, no `git add -u`,
  explicit paths, never chain two repos' git in one compound.
- Checkpoints: (a) after lane P + step-0 designs for lanes M/S — STOP for countersign;
  (b) after lanes M/S + loop + panel; merge gate (c) overseer-opened.
- Canonical loop text `notes/campaign-port-loop.md` (house spellings incl. t27's additions;
  positive-control rule; brace-indent; comments describe function). Context valve: standing.

## 1. Scope (three lanes, ordered)

### Lane P — parser debt (sigil-only, TDD each, separate commits)
- **P1 — the parser infinite-loop deep fix** (t25; minimized ~26-line context-sensitive repro
  committed `#[ignore]`d in `parser_recovery_hang.rs`; an `asm_body` zero-progress guard
  already shipped as defense; the loop lives deeper in operand/expr parse). Fix the root
  cause, un-ignore the repros, keep the zero-progress guard. If the root cause balloons past
  a bounded fix, STOP the item with findings (the guard already prevents the hang class from
  hard-locking builds — say precisely what remains).
- **P2 — the extern-in-lea-displacement parse gap** (t25, demand-1):
  `lea (extern("X")-const)(a0)` fails to parse. TDD from the ledgered repro; corpus-scoped
  (the sound_debug transliteration is the demand site — the acceptance test).
- **P3 — the leftmost-local-label immediate fix** (t27, gap-ledgered): a leading-`.` local
  label at operand start of an IMMEDIATE expression stays raw/unmangled (`.x+1` → dangling
  `Sym(".x")`; `1+.x` mangles). Fix the parser asymmetry so both spellings mangle; positive
  control = `.x+1` == `1+.x` byte-identity; negative control = the dangling-raw shape now
  impossible (or loud). Census says ZERO live corpus instances, so this is byte-neutral by
  construction; the t27 commuted house spelling then becomes optional (note it in the
  checklist entry, don't remove the spelling).
- All of lane P: 68k paths frozen where not intentionally extended; full paired strict +
  dual rebuild after each landing.

### Lane M — the DEMAND-3 whole-ROM off-canonical mixed-build machinery (step-0 design FIRST)
The harness capability to build the WHOLE ROM mixed (asl + sigil-gated .emp) at an
OFF-CANONICAL config and byte-compare against a pure-asl build at the SAME config. Three
consumers, two distinct configs:
- hotkeys+mirror shape (`SOUND_DEBUG_HOTKEYS` / `SOUND_DBG_MIRROR` — both debug-side flags;
  step 0 rules whether ONE combined build serves game_debug AND sound_debug, the t26 "one
  build serves both" intent),
- no-sound shape (`SOUND_DRIVER_ENABLED` off — z80_init).
Design in a step-0 note section BEFORE building: config plumbing through the build
invocations, gate flags per consumer, where the comparison harness lives (the
`assert_rom_matches_convsym` family is the t25 precedent), and which existing
window-scale oracles get superseded vs kept. LEAN bar: the machinery is done when the three
consumers' whole-ROM gates run green by name in the strict suite — no speculative configs.

### Lane S — `games/../engine sound_debug.asm` → `sound_debug.emp` (98 L; the deferred mini-tranche)
Triple-gated (`__DEBUG__` + `SOUND_DRIVER_ENABLED` + `SOUND_DBG_MIRROR`, third off by
default) → ZERO bytes both canonical shapes; sole caller `vblank.asm:28`. Port after P1/P2
unblock the transliteration (t25's STOP reasons). Proof = canonical-emptiness both shapes +
the mirror-shape parity (extend `vblank_port::vblank_mirror_shape_twin_parity` per kill row
42) + the lane-M whole-ROM gate at the mirror shape. **KILL ROW 42 KILLED** here; this port
also verifies the Sound_DebugMirror closure (the corpus's last extern — t26's census).
Byte-neutral canonical; the t25 close packet + `2026-07-28-t25-debug-trio-brief.md` carry
the lane's history — read both.

## 2. Overseer recon notes (porter re-verifies at step 0; tree wins, flag discrepancies)

- The t25 STOP reasons for sound_debug were exactly (a) the P2 parse gap and (b) the P1 hang
  on the full file; the removed `.emp` draft's history is at aeon `0618dd4` (do not resurrect
  blindly — re-derive against current capabilities).
- t26 deviation row: game_debug shipped with NO main.asm gate arm / mixed placement — lane M
  closes that deferred row (bump/close the ledger rows ~1512(c) and the paired-machinery row).
- t27's z80_init joins as the no-sound consumer; its boot_data.asm gate else-arm comment
  names DEMAND-3 — update that comment's claim in lockstep when the machinery lands.
- The `#[ignore]`d repros are uncounted in 2675 — report counted-total math explicitly.

## 3. Panel ruling

**A1 + B1 + C2.** C1 inactive-recorded (debug/human-timescale paths). C3 CONDITIONAL:
activates if lane S's port makes claims about the Z80 mirror seam / vblank timing (the
sound_debug mirror copies Z80 state during vblank — if the header claims bus/timing
contracts, C3 verifies them read-only). Lenses synchronous. Dry adjudicated by panel.

## 4. Duties

Kill rows: 42 killed (lane S); new rows for any gate arms/mirrors born (lane M gate flags,
lane S vblank caller gate), same-commit. Ledger: close/bump the parser-debt rows (P1 demand-1,
P2 demand-1, P3), the DEMAND-3 row, the t26 paired-machinery row; sweep per pass. Close
packet with per-pass findings + corrections list (this brief's errors, if any — tree wins).
Step-2 checklist feed-forward for anything that becomes a house spelling. Type layer: LOG-only
unless the slot-type gate fires (t26 precedent — then it's demanded, take it).

## 5. After t28

Z80 rung 2 (psg → fm) — its contract-vocabulary design note is drafting in parallel
(module-scope `invariant`, Z80 register contracts, shadow-set/di-ei vocabulary, the jr→jp
relax ladder, the rung-2 Z80Reg-splice test obligation); the streams meet at the rung-2
dispatch. Then: the banked-head seam sub-tranche (generated-file/DSL row, demand-3-convergent)
and the game-side ~20 (needs a fresh recorded census — the t26 census list was never
committed; a future recon owes the durable file).
