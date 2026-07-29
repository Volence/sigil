# 2026-07-29 — t37 brief: Z80 rung-3b — the sound_sfx port (scale-1)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target: `engine/sound/sound_sfx.asm` (1627 L, 23 routines per the t32 §5.1 census) →
`sound_sfx.emp` — the interpreter's struct-prefix mirror, reusing the sequencer's Mod*
core. The FOURTH Z80 code port; after it only the driver top (rung 4) remains on the
Z80 code front. Sigil master = THIS brief's commit; aeon master **`1cbd4fd`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2827/0 (1 ignored)**. EXPECTED BYTE MOVEMENT ZERO, STOP-not-absorb
  (Z80-blob-precedes-engine).
- Branches `port-tranche37` BOTH repos, worktrees `.worktrees/port-tranche37`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first, rebuild-worktree-ROMs-after-rebase). The
  other four resident Z80 files READ-ONLY (incl. sound_sequencer.asm/.emp). Parallel
  porter on `hoist-ambient` (68k-side, different files); merge order ruled at the
  gates.
- Checkpoints (a)/(b)/(c); loop text + t24 doctored controls; valve standing. The
  port-loop doc + the t36 close packet are the law + the precedent.

## 1. Charter (the t36 template, fourth instance)

Scale (1) windowed oracle BOTH shapes, windows derived from the listings (sfx follows
the sequencer in the blob — expect base ≈ $0CD7 plain / $0D55 debug, i.e. a
SHAPE-VARIANT base like psg's t32 case since the sequencer's 16 internal `__DEBUG__`
blocks precede it; VERIFY, and check sfx for its OWN internal `__DEBUG__` blocks —
the `if DEBUG == 1` precedent is now in-corpus). Kill row same-commit (the row-78
shape, seam-sub-tranche kill). Demand table (indicative): ~28 Clobbers + 11 Preserves
+ **11 Out headers — the most Out-heavy file yet**; expect heavy `out(carry:)` /
`out(reg)` traffic and report the consumer coverage per the t36 pattern. Full contract
set, honest-contract derivation, invariant(ix) only WHERE it proves (the t36
walker-clobber precedent: if Sfx_Frame is a channel walker, expect the same
per-proc-preserves resolution — adjudicate, don't force). Header-accuracy scoreboard
inherits (36+~46 procs / 6 over-claims; sequencer was first-cut clean — is sfx?).

## 2. Known inputs (verify at step 0; tree wins)

- **Sequencer symbols are now DEFINED in-corpus** (`sound_sequencer.emp`): Mod_ReArm /
  Mod_Advance / Snd_ChanClass (fm) / the VolEnv resolvers (psg). Every `extern proc`
  decl sfx adds for them MUST be verified against the co-resident def's contract
  BY HAND at C3 (the machine cross-check does NOT exist — the t36-ledgered
  extern-decl-vs-def silent-drift hazard; your decls are new instances of the hazard
  class, say so in the packet and keep them exact-or-conservative-subsets).
- **Sequencer_Frame tail-jps INTO Sfx_Frame** (sound_sequencer.asm:98 region) — the
  cross-file entry seam. Sfx_Frame reads `(ix+sc_flags)` + `add ix,de` (the t36
  consequence analysis) — if Sfx_Frame is the sfx-side walker, the invariant(ix)
  adjudication mirrors t36's.
- The `ex (sp),hl` trampoline machinery is BUILT (Z80IndSp + bail + verdict
  tightening) — if sfx carries its own dispatch trampoline, it rides the existing
  feature with the same forced-spelling site comment (credit basis clause included);
  if sfx's dispatch differs in kind, that's a demanded-feature TDD or STOP.
- `(ix+(field+k))` parenthesized spelling stands; typed params at design-named sites
  only; conservative `clobbers(af)` direction; hardware/bank prose = C3-verified
  header prose, NOT contracts (§9-A/B).

## 3. Panel

**A1 + B1 + C2 + C3 ACTIVE** (C3: the bank asserts + SFX_BLOB_BANK claims, priority/
channel-steal prose, any chip-write ordering claims — verified read-only against the
resident tree; PLUS the by-hand extern-decl-vs-def verification set). C1 conditional
per the panel-activation rules (in-source cycle annotations; rung 4 owns T-states).
C2 owes at minimum: one Out-heavy proc re-derivation, one extern-decl-vs-def pair,
and the Sfx_Frame invariant adjudication re-derivation. Dry by panel.

## 4. Duties

Kill row + ledger per pass; close packet with the acceptance delta vs t36 (what the
Out-heavy mirror caught that the sequencer didn't; the B1 `ix_field_ptr` candidate's
sfx sites count toward its cross-file build case — report them); census §5.1 row →
DONE; corrections list. After t37: the Z80 code front is DRIVER-ONLY (rung 4,
T-state capability first), and the seam sub-tranche's input set is complete.
