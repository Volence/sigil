# 2026-07-29 — t38 brief: game-side P4 — player_sensors (the player cluster CLOSES)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the census's P4 row: `games/sonic4/player/player_sensors.asm` (493 L) →
`player_sensors.emp`. P4 is the player cluster's CLOSE — a large retirement checklist
rides it (§3). Sigil master = THIS brief's commit; aeon master **`0037db4`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2827/0 (1 ignored)**. CANONICAL-BYTES tranche: step-1 delta ZERO; any step-2/5
  movement rides the FULL wave discipline (the canonical Wave-Ripple Checklist).
- Branches `port-tranche38` BOTH repos, worktrees `.worktrees/port-tranche38`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first, rebuild-worktree-ROMs-after-rebase).
- Parallel porter on `port-tranche37` (Z80 sfx — different files); merge order ruled
  at the gates. Checkpoints (a)/(b)/(c); loop text; t24 controls; valve standing.

## 1. Scope + region facts (verify at step 0; tree wins)

- player_sensors lives in the ENGINE BLOCK (`games/sonic4/main.asm:21`,
  gameEngineBlockIncludes) — a DIFFERENT region than the object code bank; its own
  pin; census says no code_addr entry points (a called primitive). Derive the region
  + shape behavior from the listings; per-file gate `SIGIL_EMP_PLAYER_SENSORS` with
  whatever arm shape the region demands (plain ifndef expected — verify).
- The engine.constants ambients are LIVE (the hoist parcel merged) — consume
  `use engine.constants.{...}`; file-local mirrors only for truth that is genuinely
  local or game-side (the PPHYS layering correction is the precedent).
- Hot-math file (angle finding, distance probes): panel **C1 ACTIVE named-sites +
  C2**; C3 inactive unless VDP claims appear; A1+B1 standard.

## 2. Contract duties (the callee side of the standing externs)

The 4 sensor procs become real defs. Their machine contracts must SATISFY what the
callers declared at t35 (Floor/Ceiling `d0-d7/a1-a2`, WallAt/WallDir `d0-d6/a1`):
the def may refine (clobber less) but never exceed. Any mismatch = adjudicated
finding with evidence (the 68k flavor of the t36 drift class). Same for
`Player_AtLedgeEdge` vs t34's boundary decl. Honest-contract derivation throughout;
oracle `preserves(a0)` uses reported.

## 3. THE P4-CLOSE RETIREMENT CHECKLIST (each item = same-commit row updates)

1. **Row 80 CLOSES**: the 7 `extern proc` sensor decls in player_{ground,air,
   spindash}.emp DELETE — the calls become module-to-module (editing those three
   .emp files for the closure is IN scope). The corpus heads back toward the
   zero-extern-proc headline.
2. **t34's Player_AtLedgeEdge boundary decl dies** (its kill row said dies-at-P4).
3. **The 5 GUARDED PlayerV fields**: sensors is expected to be the LAST AS reader of
   `_pl_*` — verify PER FIELD (grep the whole AS tree, not just player files); retire
   each guard whose last reader dies; rows 72/74/75's P4-close clauses execute where
   their conditions are now true (templates going .emp-only etc. — verify each
   condition, execute what holds, report what doesn't).
4. **The offsets-construct adoption** (t34 ruling: ADOPT AT P4 CLOSE): the
   Player_States table's terms are all real .emp defs since t35 — adopt the
   `offsets` construct spelling in player_common.emp (byte-neutral; the gates prove).
5. **Row 81's P4 arm**: execute per its reduced text where the condition fires.
- NOT in scope: the G9 d7 fix (stays ledgered for the post-conversion pass with
  oracle A/B — do not touch), the harness states (T1), any Z80 file.

## 4. Duties

Kill rows same-commit; ledger per pass; close packet with per-pass step-3 vs step-5
breakdown + the retirement checklist outcomes (each item: executed / condition-not-
met-with-reason) + census STATUS AMENDMENT (**THE PLAYER CLUSTER: COMPLETE**);
corrections list. After t38: the 3 objects + T1 close the game side; rung-4 driver +
seams close the Z80 side.
