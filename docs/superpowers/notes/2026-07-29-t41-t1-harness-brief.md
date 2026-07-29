# 2026-07-29 — t41 brief: T1 — the harness states (object_test_state + ojz_scroll_test)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the census's T1 rows, the LAST game-side code tranche before main/config.
The census ranked these HIGHEST-RISK game files: both are `__DEBUG__`
shape-DEPENDENT, VDP/DMA-facing, and ojz_scroll_test carries OPEN kill row 35.
Sigil master = THIS brief's commit; aeon master **`597ce06`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2888/0 (1 ignored)**. CANONICAL-BYTES class where bytes exist; both files are
  shape-DEPENDENT (distinct plain/debug byte lengths — the vblank/core gate class;
  PORTER-VERIFY the per-shape deltas from the listings).
- Branches `port-tranche41` BOTH repos, worktrees `.worktrees/port-tranche41`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first --no-fail-fast, keep commits small,
  rebuild-worktree-ROMs-after-rebase). Checkpoints: **STOP 1 = the row-35 survey**
  (below), then (a)/(b)/(c); loop text; t24 controls; valve standing.

## 1. STOP 1 — the kill-row-35 reconciliation survey (BEFORE any port code)

Row 35: ojz_scroll_test's `GameState_OJZScroll_Update` :234-273 force-writes the VDP
mode-set-3 shadow + reg $0B EVERY FRAME — a harness compensation for
`Parallax_StartTransition` writing the mode only at frame 0 (its same-config
short-circuit leaves the register stale across a transition). NOT a twin mirror — an
ENGINE GAP compensation. Porting the harness file would FREEZE the compensation into
.emp. Step-0 duties: (1) verify the row's claim against the CURRENT tree (parallax
has been touched since — does the gap still exist?); (2) survey the fix shape (does
Parallax_StartTransition need the same-config short-circuit removed / a mode
re-write? byte-CHANGING engine fix → full wave + oracle A/B class); (3) survey the
alternative (port the compensation as-is, present-tense-commented, row 35 stays open
against the parallax fix). Commit the survey, STOP for the overseer's adjudication —
the fix-vs-carry decision is his (and possibly Volence's). Do NOT fix the engine
unilaterally.

## 2. Scope after STOP 1 clears

- `games/sonic4/test/object_test_state.asm` → `.emp` (the ObjectTest soak scene the
  oracle A/B work uses — behavior must be preserved EXACTLY; any byte-gate-blind
  concern names the oracle-A/B rider).
- `games/sonic4/test/ojz_scroll_test.asm` → `.emp` (the game ENTRY state; VDP/DMA-
  facing; the row-35 outcome applied as adjudicated).
- Shape-dependent gates per file (separate plain/debug pins/windows per the census);
  panel **A1 + B1 + C2 + C3 ACTIVE** (VDP/DMA claims — the first game-side C3-heavy
  tranche since the engine block); C1 conditional named-basis.

## 3. Duties

Kill rows same-commit (the 2 gate rows; the row-35 disposition); ledger per pass;
close packet with the census amendment (**THE GAME SIDE IS CODE-COMPLETE** — only
main/config remains, and that is the Spec-5 flip itself); corrections list. After
t41: the seams + the generator + the flip ARE the remaining campaign.
