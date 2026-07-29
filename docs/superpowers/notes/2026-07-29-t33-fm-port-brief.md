# 2026-07-29 — t33 brief: Z80 rung-2 item 9b — the sound_fm port (scale-1)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target: `engine/sound/sound_fm.asm` (998 L, 22 routines) → `sound_fm.emp`, the second
resident-blob Z80 code port and the INVARIANT-HEAVY half of rung-2 item 9. Sigil master
= THIS brief's commit; aeon master **`a4008ca`**.

## 0. Bars

- Canonical: plain **`85111814`/421041** · debug **`eb5e94be`/429102** (the post-t31
  canonical). Strict baseline **2783/0 (1 ignored)**.
- Branches `port-tranche33` BOTH repos, worktrees `.worktrees/port-tranche33`; the full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit paths,
  no `git add -u`, failures-first, REBUILD WORKTREE ROMS AFTER ANY REBASE — the t32
  lesson). EXPECTED BYTE MOVEMENT ZERO, STOP-not-absorb; the `.asm` stays canonical.
- The other four resident Z80 files READ-ONLY. Parallel porter on `port-tranche34`
  (game-side P1) — different files; t33/t34 merge order ruled at their gates.
- Checkpoints (a)/(b)/(c); loop text + t24 controls; valve standing.

## 1. Charter (the t32 template, harder corpus)

Scale (1) windowed oracle BOTH shapes (derive fm's windows from the listings — expect
shape-variant bases like psg's, layout-invariant; VERIFY not assume). t24 doctored
controls. Kill row with the seam-sub-tranche condition (the row-70 shape).

The t32 close packet §5.1 census is your demand table: ~22 preserves + 20 clobbers +
6 out headers. fm adds what psg lacked:
- **Deep push/pop LIFO chains** (the census's "invariant-heavy" note) — the sibling
  proof's pair-slot model meets real nesting; any proof failure on a TRUE contract is a
  finding (checker gap or header lie — adjudicate with evidence, the t32 pattern).
- **Snd_ChanClass is DEFINED here** (fm:119, `preserves(bc,de,ix)`) — psg's biggest
  extern trust CONVERTS TO VERIFICATION this tranche; state it explicitly in the report.
- The $2A re-park discipline (Fm_ReparkDac) and the de=$4001-by-construction facts are
  HEADER PROSE to carry present-tense (C3 verifies) — NOT register contracts (§9-A/§9-B
  rulings stand; the port-lint seam stays design-only).
- `invariant(ix)` module-wide per the psg precedent; `out(carry:)` sites per headers;
  falls_into chains per ruling 5; explicit jr/jp per ruling 1.
- Any operand form the wired set lacks = demanded-feature TDD (the t27/t32 class) or
  STOP; the (ix+(field+k)) parenthesized house spelling stands.

## 2. Panel

**A1 + B1 + C2 + C3 ACTIVE** (the YM port-write discipline, the $2A/$2B claims, the
Timer-A/DAC coexistence prose — all verified against the resident tree read-only).
C1 conditional (fm has cycle-sensitive YM-write spacing? — check for in-source cycle
annotations; flagged call with named sites either way). Lenses synchronous; dry by panel.

## 3. Duties

Kill rows same-commit; ledger per pass (the declaration-trust conversion, any new
residue); close packet with the acceptance delta vs psg (what fm's corpus caught that
psg's didn't — expected candidates: LIFO depth, the af' absence re-confirmed, header
accuracy rate); corrections list; census updates. After t33: rung 3 (sequencer+sfx)
needs the ex(sp),hl trampoline + the out(carry) cross-proc credit goes live.
