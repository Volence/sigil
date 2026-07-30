# 2026-07-30 — seam-2 stage-2b: OPTION Y RULED + co-link recon (kickoff)

Status: **The overseer countersigned Stage 1 and RULED the 2b fork = OPTION Y
(pull the `dac_sample_tab` co-link forward; rows 5-dac + 57 close TOGETHER, body
and head as a unit). This note records the ruling + the read-only co-link
reconnaissance so the next porter executes the 4-step Y-sequence warm.** The
repo is CLEAN at the green Stage-1 boundary (sigil `424aceb` / aeon `6b8eac2`,
strict 2884/0/1); no Option-Y edits are staged.

## The ruling (verbatim intent)

Option Y over X because Y IS the endorsed design's §2d shape ("descriptor cells
fold directly from placement — no `-D`, no 30-value mirror"); X's syms bridge is
a new interim artifact built only to die at stage 3 (the pending-mechanism
churn the rule forbids). Rows 5-dac + 57 close in ONE deletion commit.

## The Y-sequence (overseer, small committed steps)

1. **OQ-4 label un-suppression** (byte-neutral; verify no fixture pins the
   absence — a pinning fixture updates with its reason stated).
2. **`dac_sample_tab.emp` goes native via the co-link** — the emitter extends to
   co-link body + head; `SND_*` folds from placement; the phased-head machinery
   from `seam2_phased_head.rs` (OQ-5) is the substrate. Proven in the DUAL state
   (both paths byte-identical, `mixed_dac_rom` + assembled-ROM unchanged, t24
   control).
3. **The wire** (build.sh + the BINCLUDE arm, per the recorded facts in
   `2026-07-30-seam2-stage1-rebaseline.md` §"STAGE 2b READINESS"). Proven dual.
4. **THE DELETION COMMIT** — `dac_samples.asm` + `dac_sample_tab.asm` TOGETHER,
   rows 5-dac + 57 same-commit, plain-spoken message, post-deletion proof =
   assembled-ROM unchanged + artifact drift explained.

## Co-link reconnaissance (read-only — the semantic crux for step 2)

**Current mechanism (to eliminate):** `dac_sample_tab.emp` receives the 30
`SND_*` as `-D` comptime defines (`dac_sample_tab_port.rs::compile_emp`'s
`snd_seam`). Option Y replaces the `-D` with real cross-module resolution against
`dac_samples.emp`'s PLACED symbols in a joint link.

**What `dac_samples.emp` already provides:** the 10 start labels (`data
Dac_Temp_Blip = BlipBlob`, `data Dac_Kick = KickBlob`, …) AND the 30 `SND_*`
equs, which ALREADY fold from placement SAME-module (`SND_KICK_BANK =
bankid(Dac_Kick)`, `_PTR = winptr(Dac_Kick)`, `_LEN = KickBlob.len`). The header's
"deliberately not ported" set is the `Dac_*_End` labels + `Dac_SharedBank_Start`
— NOT the start labels (those are present).

**The three fold cells, per cross-module support (verified against the corpus):**
- `bankid` / `winptr` on a **cross-module** label: PROVEN via the string form
  `bankid("MovingTrucks_Bank_Start")` (mt_bank.emp — a CALL ARGUMENT turns an
  unresolved bareword into a deferred link symbol; ports.rs `probe_b`). So
  `bankid("Dac_Kick")` / `winptr("Dac_Kick")` in `dac_sample_tab.emp` resolves
  in the joint link. ✓
- `.len` on a **cross-module data label** (`Dac_Kick.len`): UNPROVEN in the
  corpus — every `.len` seen is same-module on a `const = embed(...)` binding
  (`RingArt.len`, `BlipBlob.len`). This is the length-cell CRUX. Do NOT assume
  `Dac_Kick.len` works cross-module without a probe.

**Recommended route (lowest risk, satisfies the ruling):** have
`dac_sample_tab.emp` do `use data.dac_samples.{the 30 SND_*}` and KEEP the
`SND_*` equ block in `dac_samples.emp`. This gives: NO `-D` (co-linked import);
NO 30-value mirror (single definition, in dac_samples.emp); FOLDS FROM PLACEMENT
(dac_samples.emp's equs do, same-module so `.len` is fine); body+head leave as a
unit. The `SND_*` names live ONCE, at the producer — the honest single-source.
`module data.dac_samples` is the import path. **OPEN QUESTION the porter must
settle first:** does `use module.{EQU_NAME}` import EQUs? The corpus shows `use`
for procs (`use engine.sound_sequencer.{Mod_ReArm}`) and types (`use
engine.types.{SongId}`); equ import is UNVERIFIED. If unsupported, fall back to
inline `bankid("Dac_Kick")`/`winptr("Dac_Kick")` for bank/ptr and import ONLY the
`SND_*_LEN` equs (10 of the 30) — still no mirror, still single-source lengths,
and it sidesteps the unproven cross-module `.len`.

**The right way to settle both open questions:** a focused co-link probe in
`sigil-harness::seam2` (extend `emit_dac_banks` toward an `emit_dac_body_and_head`
that lowers dac_samples.emp + dac_sample_tab.emp, joint-links them, and asserts
the head bytes equal the reference `dac_sample_tab` slice). That IS step-2's
dual-proof substrate and empirically answers "does the import/fold resolve"
before any `.asm` deletion — TDD, not a guess. `seam2_phased_head.rs` shows the
phased-head link at LMA in the `$58000` bank with VMA `$8000`; the head's
reference slice is at ROM `SndDefaultPitchTable`-region — re-derive the exact
`dac_sample_tab` head offset from `s4.lst` (`DacSampleTable` phase VMA → head bank
`$58000`).

**t24 control reminder:** the head is `-D`-shape-invariant CONTENT (the SND_*
fold is the same plain/debug — the DAC banks don't move with `__DEBUG__`), but
its LMA sits in the head bank whose position is shape-stable at `$58000` both
shapes (MovingTrucks_Bank_Start @ `$58000` in BOTH `s4.lst` and `s4.debug.lst` —
verified in Stage 1). So one head blob serves both shapes; still dual-prove.

## Why I stopped here (honest valve stop)

Stage 1 is complete/green/committed both branches. Option Y is a correctness-
critical multi-round stage (unproven cross-module `.len`/equ-import to settle
empirically; the emitter co-link extension; TWO dual-proof rounds across both
shapes; the coupled body+head deletion). Its constants are ADDRESS-DERIVED — a
rushed co-link is the precise false-green trap the campaign guards against. The
read-only recon above de-risks the porter's first two steps; executing them wants
fresh budget and the probe-first (TDD) discipline, not a rushed pass. No pushes;
the merge is the overseer's.
