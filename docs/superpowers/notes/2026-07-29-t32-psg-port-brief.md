# 2026-07-29 — t32 brief: Z80 rung-2 item 9a — the sound_psg port (scale-1)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the FIRST resident-blob Z80 code port: `engine/sound/sound_psg.asm` (526 L) →
`sound_psg.emp`. The rung-2 contract system (merged, sigil `1c08ea7`) names psg as its
acceptance corpus; T1 + rung-2 make this a PURE PORT tranche. fm is NOT in scope (item 9b).

## 0. Bars (overseer-verified at dispatch)

- Masters: aeon **`b381a83`** / sigil **`72baa7b`**, origin==local, clean.
- Canonical: plain **`c51342d0`/421041** · debug **`992d9e7d`/429102**. Strict baseline
  **2757/0 (1 ignored)**.
- Branches `port-tranche32` BOTH repos, worktrees `.worktrees/port-tranche32`; the standard
  session rules (editor rsync, one shape per invocation, cd-every-call, explicit paths,
  no `git add -u`). Checkpoints (a)/(b)/(c); loop text + positive-control rule; valve.
- **EXPECTED BYTE MOVEMENT: ZERO, STOP-not-absorb** — the blob precedes the engine; one
  Z80 byte slides the whole corpus. The `.asm` twin stays canonical in the build.
- **MERGE QUEUE: t31 merges FIRST** (parallel porter on port-tranche31; different files;
  conflicts = STOP). The FIVE resident files other than psg are READ-ONLY.

## 1. Proof scale (PRE-RULED — the t27 precedent)

**Scale (1) WINDOWED ORACLE is the tranche outcome.** psg is the 4th of 5 includes inside
the resident `cpu z80 / phase 0` blob (z80_sound_driver.asm include chain) — mid-blob
whole-ROM gating is the SAME placement-machinery family the t27 seam STOP deferred (the
banked-head/resident-blob seam sub-tranche). Do NOT attempt whole-ROM placement. The
proof: compile `sound_psg.emp` at its phase-0 window (derive the vma from the listing)
and byte-compare against the AS twin assembled via the AS front-end, cross-seam symbols
as equ carriers (the z80_init_port / t27-lane-B/C template), BOTH shapes if the blob is
shape-variant (verify — the blob is expected shape-INVARIANT; state the evidence). t24
doctored controls mandatory. Kill row for the twin with the seam-sub-tranche kill
condition (the t27 rows 56/57 shape).

## 2. What this port proves (the rung-2 acceptance — the tranche's real cargo)

- First live per-routine Z80 contracts: the `Clobbers:`/`Preserves:` headers (psg:60 ff.)
  become real `clobbers(...)`/`preserves(...)` — the sibling proof verifies them.
  **Every header claim is now CHECKED — expect discrepancies; each one is a step-3
  finding** (an inaccurate 15-year-old header caught by the checker is a headline).
- `module (cpu: z80) invariant(ix)` — the FIRST live module invariant (the design's
  psg-header-line-60 class); inheritance proven across every proc.
- `out(carry: found)` on `PsgVolEnv_Resolve` (:120) + a caller `jr c` — the flag-result
  machinery's first corpus instance (the cross-proc must-consume check goes live).
- The `falls_into` chains (Psg_ApplyMod→Psg_EmitDivisor→Psg_EmitDivisorTo :294-334) —
  separate contracted procs + falls_into per ruling 5; byte-critical, no inserted jumps.
- Explicit `jr`/`jp` per ruling 1 (NO unsized idiom — Z80 house style; the ladder stays
  latent). The de-clobber/caller-re-establish facts (§9-A) are HEADER PROSE to carry
  accurately, not contracts (de is clobbered honestly).
- The step-2 house checklist applies where 68k idioms have Z80 analogues; brace-indent;
  comments describe function (the psg header's contention/latch prose carries as
  present-tense contract facts — C3 will verify them).

## 3. Panel ruling

**A1 + B1 + C2 + C3 ACTIVE** (the PSG-write/no-YM-touch/no-busy-poll hardware claims and
the de/Timer-A caller contract — verify against the resident tree READ-ONLY). **C1
CONDITIONAL**: psg is off the DAC hot loop (recon: "free jr use... writers"), but if any
routine is cycle-annotated in-source, C1 activates for those sites — flagged call with
named sites either way. Lenses synchronous; dry by panel.

## 4. Duties

Kill rows same-commit; ledger per pass (any operand form T1/rung-2 didn't wire = the
demanded-feature TDD class or STOP; expected NONE for psg per the demand tables — a miss
is a design-note corrections row). Close packet ends with the rung-2 acceptance verdict:
which capabilities fired, which header claims were corrected, the contract-discrepancy
list. After t32: fm (item 9b, invariant-heavy) then the interpreters (rung 3).
