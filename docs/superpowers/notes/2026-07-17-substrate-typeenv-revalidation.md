# Substrate parcel — corpus type environment + LOUD drops + G1–G3 re-validation

**2026-07-17, Opus. Fable checkpoint (via Volence).** The substrate-fix-first
ruling, executed. Branch sigil `feat/contract-corpus-typeenv` (1 commit
`f29646a`, off master 1af06cd, byte-neutral). **One adjudication item: 2 genuine
`a0` under-declarations surfaced — ruling requested on the retrofit form before
the flip pin goes green.**

## 1. The fix (§1 of the ruling — a general two-pass type environment)

`analyze_corpus` evaluated each `.emp` in ISOLATION, so a field operand on an
IMPORTED struct (`Sst.mappings(a0)`, `sizeof(Sst)`) failed `resolve_field_disp`
and the whole instruction was **silently dropped** from the analysis buffer.

- **PASS 1** collects every declaration item (struct/const/newtype/…) across all
  34 files into a corpus TYPE ENVIRONMENT. **PASS 2** lowers each proc with that
  env in scope (`with_file_and_ambient` / `eval_proc_body_env`; ambient indexed
  first, file-local last so local always shadows). NOT the resolve pass's
  `use`-driven per-file ambient (whose per-file maintenance proved incomplete).
- `eval_proc_body` delegates with an empty ambient — the real lowering path gets
  imports from the resolve pass and is untouched (byte-neutral).

## 2. Drops are LOUD (§2 of the ruling)

The `AsmStmt::Instr` lower-to-`None` path was a silent skip. Now it **counts and
diagnoses** the dropped instruction. `ContractReport` carries `dropped_instrs` +
per-proc counts; the new corpus pin **`corpus_has_zero_dropped_instructions`**
asserts 0. This class of silent under-approximation cannot return.

Before the env: **~150 instructions across 24 of 34 files** were dropping
(sprites.emp alone: 32 field accesses → 1 survived). After: **0 dropped**.

## 3. Re-validation as prediction checks (§3 of the ruling)

Every delta explained by resolution; nothing adjusted to pass. Full suite
`sigil-frontend-emp` + `sigil-cli` (AEON_DIR set): **1780 passed / 1 failed**,
the single failure being the flip pin below. clippy 0.

- **(a) Closure residue: [] → 2 rows.** `DeleteObject` **direct a0** +
  `AnimateSprite` **transitive a0**. This is the predicted "dropped writes were
  hiding them" outcome — the flip pin correctly fails. Root cause (ONE, not two):
  - `DeleteObject` (core.emp:199, `clobbers(d0-d1, a1)`) writes `a0` at
    `.clear_slot`: `clear_longs(sizeof(Sst)/4)` advances `a0` via `(a0)+`, then
    `lea -sizeof(Sst)(a0), a0` restores it (core.emp:286). Both instructions need
    `sizeof(Sst)`/the `Sst` layout — DROPPED before the env, so the `a0` write was
    invisible and the closure residue was FALSELY clean. `a0` is genuinely
    `DeleteObject`'s object-pointer INPUT (`move.w Sst.code_addr(a0), d0`, …).
  - `AnimateSprite` (animate.emp:81) **tail-calls `jbra DeleteObject`**
    (animate.emp:203), inheriting DeleteObject's now-visible `a0`. `a0` is equally
    AnimateSprite's object input (28 `Sst.*(a0)` accesses).
  - The `a0` round-trip is a NON-STACK pointer advance/restore; §5's symbolic
    STACK tracking cannot verify it, so `preserves(a0)` would be
    `[proc.preserves-unverifiable]` — not a viable contract.
- **(b) §5 six-row preserves — RE-VERIFY: all hold.** The 5 retrofitted procs
  (AllocDynamic/Park/Unpark `a0`, CheckRing/Killed `d1`) are NOT in the new
  residue → their declared `preserves` still verify on resolved buffers (a failed
  verify would leave the register un-subtracted and it would fire). No falsely-
  verified preserves. (Load_Object's unchanged dead-saves confirm AllocDynamic's
  `preserves(a0)` transitively.)
- **(c) Dead-save worklist — RE-ISSUED, IDENTICAL.** 16 rows, byte-for-byte the
  same procs/regs/callees as the G3 TSV (only the sort order differs). **The
  substrate fix changed zero dead-save verdicts** — no false dead-save was
  hiding, so pass-3's code-deletion worklist is safe. (The old TSV will be marked
  SUPERSEDED in place at merge, per the ruling; the diff is: none.)
- **(d) Flag-check: 0 firings holds. Dropped: 0 across all 34.**

## 4. The adjudication item — the honest retrofit for the 2 `a0` rows

Both procs read `a0` as their object-pointer INPUT; the honest, accurate
declaration is a **param `(a0: *Sst)`** on `DeleteObject` and `AnimateSprite`
(not a false `clobbers(a0)` — a0 is an input, not scratch; not `preserves(a0)` —
unverifiable). A param puts `a0` in the closure's `allowed` set (`clobbers ∪
params ∪ out`), clearing both firings (AnimateSprite's transitive a0 too).

**Byte-neutrality VERIFIED:** a param sets `reg_pointee_struct[a0]=Sst`, which
only affects BARE `field(a0)` resolution (D6.A3). Both procs use ONLY qualified
`Sst.field(a0)` — grep confirms zero bare field accesses — so the param is inert
for codegen. The retrofit is contract-text-only.

This is also exactly the G4 `// In:`→param direction, arriving early here because
the same undeclared input is ALSO an undeclared register EFFECT the G1 closure
should have caught but couldn't (the write was dropped).

**Requesting the ruling:** confirm `(a0: *Sst)` param on both procs as the honest
retrofit (my recommendation), or direct otherwise. On confirmation: apply the
retrofit (aeon), flip pin returns to GREEN (residue []), re-issue the dead-save
TSV marking the old SUPERSEDED, byte gates both shapes (expect canonical
8984e510/c80465dc reproduced — retrofit is metadata), paired strict, packet,
merge. Then G4 resumes on the now-complete substrate.

## Per-pass findings

- **NEITHER-BUCKET (the catch of the arc):** a silently under-approximated
  substrate sat beneath three shipped gates (G1 closure, D1d dead-save, flag),
  and the dead-save direction was a live code-deletion hazard for pass-3. The
  checkpoint caught it before a single line was cut. The instrument-audits-itself
  pattern is now five-for-five (dbcc, 2× debug §5, movem/out in G4, and this).
  **Census erratum (for the packet):** the census's "single-file lower is
  firing-equivalent" claim is TRUE for register parsing, FALSE for field-operand
  eval — this parcel supersedes it.
- **Step-3 (modernize):** the corpus type environment makes cross-file field/
  const resolution complete for all contract analysis; drops-are-loud makes the
  invariant self-policing.
- **Step-5 (optimize):** none (byte-neutral). The re-issued dead-save worklist
  (unchanged) remains pass-3's step-5 backlog.
