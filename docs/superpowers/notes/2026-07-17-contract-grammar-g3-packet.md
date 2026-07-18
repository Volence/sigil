# Contract-grammar v2 — G3 PACKET (verified preserves + dead-save + THE FLIP)

**2026-07-17, Opus. Merge checkpoint for Volence's gate (→ Fable).** G3 builds §5
verified `preserves` (symbolic stack tracking), `[proc.dead-save]` (D1d), wires
both into `check_preserves`/the closure, retrofits the 5 residue procs, and — at
zero residue — flips the closure firing check WARN→ERROR. This is the closing act
of the diagnostics arc.

Branches (isolated, byte-neutral): sigil `feat/contract-grammar-g3` (6 commits),
aeon `feat/contract-grammar-g3` (1 commit).

## What becomes true at this merge

- **An undeclared register effect in `.emp` is now a BUILD ERROR.** The transitive
  clobber closure's residue is zero and the strict-gated pin flipped to
  expect-EMPTY: any register a callee leaks into a proc that the proc neither
  declares nor verified-preserves fails `SIGIL_STRICT_GATE`.
- **D2.32 is subsumed.** The syntactic movem-pair slice is now the trivial fast
  path of the §5 dataflow. `[proc.preserves-missing-pair]`/`-mismatch`/`-word-pair`
  retire into the single `[proc.preserves-unverifiable]`.
- **Row 1030 closes** after 5 days open — the individual-push preservation class
  (inexpressible under D2.32) is now verifiable; the 3 census FPs die honestly via
  `preserves(a0)`, never a false `clobbers(a0)`.
- **The pass-3 dead-save worklist ships** as a committed artifact: 16 rows (all 3
  review customers + 1 partial-narrowing class + 8 beyond), TSV +
  `analyze_corpus.dead_saves`.

## Gates (artifacts, not adjectives)

- **Paired strict, both tips, seeded worktree** (`SIGIL_STRICT_GATE=1
  AEON_DIR=<seeded aeon g3>`): **2365 / 0 / 1 ignored**. Failures-first: 0.
- **Byte gates BOTH shapes** (aeon ROMs rebuilt from the retrofitted+seeded
  worktree): plain **8984e510 / 453533**, debug **c80465dc / 461554** — canonical,
  EXACT. The retrofit is contract text; a relative build (retrofit vs base, same
  env) is byte-identical.
- **Residue-0 hard gate**: closure over the retrofitted corpus → firings `[]`
  (was the exact 6-row handoff). Verified the flip both directions — PASS on the
  retrofitted corpus, FAIL on the pre-retrofit corpus (the 6 rows surface as build
  errors). The precondition was zero, not "explainably small."
- **frontend-emp** 80 suites green, clippy clean workspace-wide.
- **TDD**: §5 preserves 19 unit + 1 corpus; dead-save 6 unit + 1 corpus; 6 D2.32
  lower_proc tests repurposed to §5 verdicts; 2 corpus subtract tests. Every §5
  refinement watched fail first.

## Per-pass findings (step-3 modernize vs step-5 optimize vs neither)

**NEITHER-BUCKET (the finding of the phase) — the dbcc shared-CFG gap.** The FIRST
real-corpus preserves run reported Collected_Park/UnparkSlot NotPreserved. Root
cause was NOT the residue procs: the shared CFG's `branch_target` read
`ops.first()`, missing the `dbcc dN, label` two-operand form (label is SECOND), so
the park/unpark `dbf` copy loops resolved as an external `Defer` edge. Fixed —
scan for the LAST `Sym` operand (correct for bcc/bra/jbsr/dbcc alike); G2
flag_check + both corpus pins unaffected (no `dbf` between the 3 flag sites and
their consumers). Invisible to G2's tests, my synthetic §5 vectors, AND the entire
flag_check corpus run — surfaced only by the first preserves pass over real code.
The cleanest demonstration yet of why the checkpoint runs against real code before
a retrofit touches anything. Pinned by `dbcc_target_is_second_operand`.

**Neither-bucket — two DEBUG-only assert/raise_error false negatives, caught by the
paired-strict DEBUG byte-identity gate (not plain).** (1) Sound_PlaySFX's
`raise_error` path clobbers d1/a0 then `jmp`s to a noreturn error handler — the
`Defer` edge was wrongly a preservation counterexample (G3.3 fix: Defer is not an
rts path — ignore it). (2) Collected_ParkSlot's DEBUG `assert` lowers to
`subq #2,sp` (computed sp) on the same noreturn raise path — the GLOBAL bailout
poisoned a0's proof on the RETURNING paths (G3.5 fix: bailouts are PATH-LOCAL;
only a bail reaching a return unverifies). Both are correct-code / analysis-gap;
both invisible to the plain build. Lesson: the debug shape's `assert`/`raise_error`
machinery is a distinct §5 stressor — the paired debug byte gate is load-bearing.

**Step-3 (language/modernize):** §5 makes `preserves` a real, dataflow-verified
contract (declared ⊊ saved now legal — the CheckRing shape), not a movem-shape
heuristic. `[proc.dead-save]` gives pass-3 a machine-generated worklist instead of
a hand census. `Reg` gained `PartialOrd/Ord` (fieldless enum, harmless).

**Step-5 (engine optimize):** none taken here (byte-neutral phase); the dead-save
worklist IS the step-5 backlog handed to pass-3. Notable: the movem-partial class
(CheckRing/Killed save `movem.l d0-d1` but only d1 is needed — Collected_FindSlot
preserves d0) is a narrowing, not a deletion — a finding class the review did not
have.

## Checkpoint results (recorded; see the checkpoint note + TSV)

- **6-row verify table** — 5 verify locally (AllocDynamic/Park/Unpark a0,
  CheckRing/Killed d1), Load_Object a0 clears transitively. Zero soundness
  bailouts. Matched prediction exactly.
- **Dead-save worklist** — 16 firings; all 3 review customers (dplc ~575,
  load_object ~76, children 44-116) + 8 beyond + the movem-partial. Verdict rides
  the closure's VERIFIED effective set (never raw declared text — pass-3 cuts code
  on this).

## Merge runbook

1. `--no-ff` both repos. No cross-era hazard (both fork from the current post-G2
   masters). aeon is contract text in 2 files; sigil is frontend-emp + the flip
   pin. File-disjoint from any in-flight parcel.
2. Post-merge byte gate BOTH shapes: reproduce plain 8984e510/453533 · debug
   c80465dc/461554 EXACTLY (seed the merge checkout if fresh —
   `tools/seed-worktree.sh`; an unseeded tree falls back to air-baseline collision
   and diverges ~130KB).
3. Paired strict from merged tips: expect **2365 / 0**. The flip pin
   `corpus_closure_residue_is_empty_the_error_gate` is now the permanent error
   gate — it must be GREEN (residue empty). Any firing = an undeclared register
   effect = stop.
4. Push together, worktree-before-branch-delete, report.

## Next (planning turn)

Pass-3 (object/render contract surgery) is UNBLOCKED — G1+G2+G3 landed. It feeds
directly from the dead-save worklist. G4 (`[call.input-undefined]` /
`[call.live-clobbered]` — D1c hardens pass-3 mid-flight) and G5 (typed slots) per
the spec §10 order (G3→G4 swappable, done). Remaining-bugs tiers per the campaign
log.
