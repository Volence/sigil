# Substrate parcel — corpus type environment + §5 register-arithmetic + a0 (PACKET)

**2026-07-17 → 18, Opus. Merge checkpoint (→ Fable, via Volence).** The
substrate-fix-first ruling, executed to completion: the corpus type environment
closes the silent-drop hazard beneath the contract gates, drops are made loud and
pinned to zero, G1–G3 are re-validated as prediction checks, and the two `a0`
under-declarations the fix surfaced are retrofitted honestly — via a §5 extension
that PROVES the contract rather than lying it.

Branches (isolated, off the post-G3 masters 1af06cd / c75924e):
- sigil `feat/contract-corpus-typeenv` — 4 commits (`f29646a` type env + loud
  drops, `b3ca46d` re-validation note, `cd182c5` §5 delta + strip-params,
  `+docs`).
- aeon `feat/contract-corpus-typeenv` — 1 commit (`02b36d0` the a0 retrofit).

## What becomes true at this merge

- **The contract-analysis substrate is COMPLETE.** `analyze_corpus` evaluates
  each `.emp` against a whole-corpus TYPE ENVIRONMENT (every struct/const/type
  declaration in scope), so a field operand on an imported struct no longer fails
  to resolve and vanish. ~150 previously-dropped instructions across 24 of 34
  files now resolve.
- **Silent under-approximation cannot return.** The `AsmStmt::Instr`→`None` drop
  is counted + diagnosed; the pin `corpus_has_zero_dropped_instructions` asserts
  **0**.
- **§5 verifies pointer-arithmetic round-trips.** A per-address-register linear
  delta rides the existing dataflow: `(rN)+`/`-(rN)`/`lea d(rN),rN`/`adda/suba
  #imm` accumulate a static offset; preserved iff Δ==0 at every `rts`. This turns
  the pointer-walk-and-restore idiom from "presumed fine" to "proven or flagged".
- **Params no longer excuse writes.** The closure's `allowed` set is `clobbers ∪
  out` — a param declares an INPUT, not a licence to destroy it. A proc that
  trashes its own input register now FIRES unless it declares the effect.
- **The a0 residue is closed HONESTLY.** `DeleteObject` declares `preserves(a0)`
  (now VERIFIED, not presumed) + `(a0:*Sst)` input; `AnimateSprite` gets the
  `(a0:*Sst)` input and clears transitively; `TestParticle_Main` was already
  typed. Residue returns to `[]`.

## Gates (artifacts, not adjectives)

- **Paired strict, full workspace, retrofitted worktree** (`SIGIL_STRICT_GATE=1
  AEON_DIR=<seeded typeenv>`): **2375 / 0**. Failures-first: 0. (G3 was 2365; +10
  substrate tests.)
- **Byte gate BOTH shapes** (`m1d_rom` + `m1d_debug_rom`, strict, retrofitted
  worktree): PASS. Rebuilt ROMs reproduce canonical EXACTLY — plain
  **8984e510 / 453533**, debug **c80465dc / 461554**. The a0 retrofit is metadata;
  `preserves(a0)` VERIFIES in the real build (the delta tracker runs on the real
  DeleteObject buffer — a build error otherwise).
- **Dropped-count gate**: 0 across all 34 files.
- **Residue gate**: `corpus_closure_residue_is_empty_the_error_gate` GREEN on the
  retrofitted corpus (was RED on-branch pre-retrofit — the designed adjudication
  signal).
- **TDD**: corpus_typeenv 3 (drop-counted w/o ambient; resolves with; local
  shadows); §5 delta 5 (advance+restore verifies; missing-restore NotPreserved;
  runtime-loop untrackable; adda/suba; fresh-load untrackable) — 22 preserves
  tests total; closure test rewritten to the new param semantic. clippy clean.

## Re-validation (the §3 prediction checks — every delta explained by resolution)

| Check | Result |
|---|---|
| Dropped instructions | ~150 → **0** |
| Closure residue | `[]` → **2 genuine a0 rows** → `[]` after honest retrofit |
| §5 six-row preserves | **all re-verify** (the 5 procs absent from residue) |
| Dead-save worklist | **RE-ISSUED, 16 rows byte-for-byte identical** to G3 — zero verdict changes; old TSV marked SUPERSEDED; pass-3 consumes the re-issue only |
| Flag-check | **0** |

The dead-save identity is the load-bearing result: the substrate fix changed no
dead-save verdict, so no false dead-save was hiding and pass-3's code-deletion
worklist was already safe.

## Per-pass findings (step-3 vs step-5 vs neither)

**NEITHER-BUCKET — the catch of the arc.** A silently under-approximated
substrate sat beneath THREE shipped gates (G1 closure, D1d dead-save, flag), and
the dead-save direction was a live code-deletion hazard for pass-3. The G4
checkpoint caught it before a single line was cut. The instrument-audits-itself
pattern is now **five-for-five**: (1) G3's dbcc shared-CFG gap, (2)+(3) the two
DEBUG-only §5 stressors, (4) G4's movem-save/restore + intervening-out-call, and
(5) this — a residue row forcing the VERIFIER to grow (the linear-delta tracker)
instead of the contract to lie. Three of five surfaced ONLY on the real corpus,
pre-retrofit — the checkpoint discipline earning its keep.

**CENSUS ERRATUM (ordered by Fable).** The diagnostics census's claim that
"single-file lower is firing-equivalent" is TRUE for register parsing but FALSE
for field-operand eval: a field operand on an imported struct silently dropped
under single-file eval, under-approximating the write set. This parcel supersedes
that claim — the type environment is required for sound contract analysis.

**Step-3 (modernize / language):** the corpus type environment makes cross-file
field/const resolution complete; drops-are-loud makes the invariant
self-policing; the §5 linear-delta tracker turns the pointer-walk-and-restore
class (pervasive in this engine) into a proven contract; stripping param-allowed
removes a latent unsoundness. `(a0:*Sst)` inputs are the honest role and the G4
`In:`→param direction arriving early.

**Step-5 (optimize):** none (byte-neutral phase). The re-issued dead-save
worklist (unchanged) remains pass-3's step-5 backlog.

**Ledger:** the §5 tracker's limit (runtime-trip-count round-trip loops stay
unverifiable — none exist today) is recorded; a symbolic-trip-count extension is
owed only if such a proc appears.

## Merge runbook

1. `--no-ff` both repos (sigil then aeon, or the campaign's merge-queue order).
   No cross-era hazard (both fork from post-G3 masters). aeon is 2 lines of
   contract text; sigil is frontend-emp analysis + eval + closure + tests +
   docs. File-disjoint from G4 (`feat/contract-grammar-g4`, which resumes after).
2. Post-merge byte gate BOTH shapes from a SEEDED merge checkout: reproduce plain
   8984e510/453533 · debug c80465dc/461554 EXACTLY (`tools/seed-worktree.sh`).
3. Paired strict from merged tips: expect **2375 / 0**. The residue flip pin and
   the new `corpus_has_zero_dropped_instructions` pin must both be GREEN.
4. Push together, worktrees-before-branch-delete, report.

## Next

**G4 RESUMES on the complete substrate** (`feat/contract-grammar-g4`, commits
f55d9cb/cbf50b7 — the WARN-phase D1b/D1c + the two accepted analysis fixes). The
substrate change means D1b/D1c now run on a full instruction stream, so the
firing lists are trustworthy: re-run → checkpoint the real lists → `In:`→params
retrofit → ERROR flip. Pass-3 remains gated behind G1+G2 (met) and consumes the
re-issued dead-save worklist.
