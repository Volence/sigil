# 2026-07-29 — seam-1 EXECUTION brief: the resident-blob native link (THE FIRST TWIN DELETION)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
**VOLENCE-RATIFIED GO** (2026-07-29): the deletion arc is authorized — this parcel
executes the endorsed seam-1 design (`2026-07-29-seam1-design.md`, merged to master)
and, on success, **DELETES the five resident sound `.asm` twins** (kill rows
70/71/78/83/87 close). Sigil master = THIS brief's commit; aeon master **`4e04f8e`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2894/0 (1 ignored)**. **THE IDENTITY BAR IS ABSOLUTE**: the natively-linked blob
  must reproduce the EXACT current bytes both shapes ($3DE..$1BFA plain, +$7E debug —
  the sequencer growth and the driver's 9 cross-seam operand bytes falling out of the
  REAL link, not pins) AND the whole ROM must stay canonical. Any divergence = STOP.
- Branches `seam1-native-link` BOTH repos, worktrees `.worktrees/seam1-native-link`;
  full standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first --no-fail-fast, keep commits small,
  rebuild-worktree-ROMs-after-rebase).
- Checkpoints: **(a) = the native link standing GREEN with the twins still present**
  (the dual-proof state: both build paths produce identical ROMs); **(b) = after the
  deletions + the diagnostic + the loop/panel**; (c) mine. The design's OQ-1 (the
  five-file single-module stand-up — only single-file precedent) is the first hard
  test: if the module machinery demands a feature, demanded-feature TDD or STOP.

## 1. The execution sequence (from the design; tree wins, report contradictions)

1. **Stand up the native link** (design §1): the five .emp lowered as ONE Z80 module,
   VMA $0000 / LMA $3DE, order driver→sequencer→sfx→fm→psg; the 68k arm =
   `ifndef SIGIL_EMP_Z80_SOUND … else org` at boot_data.asm:49 per the
   z80_init/sound_api precedents; the combined-link fix classes are the load-bearing
   substrate — cite them in the proof.
2. **The identity proof** (design §2): whole-blob byte gate both shapes vs the
   reference ROM extraction + gate-off/gate-on dual-build ROM identity + the
   downstream-unchanged whole-ROM CRCs. THEN checkpoint (a) — overseer countersign
   while both paths coexist.
3. **After (a) countersign — the retirement** (design §3): delete the five `.asm`
   twins; the include at boot_data.asm:49 becomes the gate arm permanently; the 5
   windowed oracles' AS-reassembly halves die, their byte gates transform to
   reference-slice gates; the **47 intra-blob extern proc decls become imports**
   (psg 3 / fm 1 / sequencer 26 / sfx 13 / driver 4); every touched kill row updated
   same-commit (70/71/78/83/87 CLOSE; sound_api rows 10/24/36/43 = the coupled 68k
   flip if the design's scaffolding makes it free, else explicitly deferred to the
   Spec-5 flip with the reason).
4. **The headline diagnostic** (design §4): `[call.clobbers-incomplete]` — the
   transitive clobbers-completeness fixpoint over the single linked module set;
   failing-first (the t37 Sfx_Frame iy case is the RED fixture — it must fire on a
   REVERTED header; the honest corpus passes); the extern-decl-vs-def ledger row
   retires as moot-for-imports; the bsr-classifier face stays ledgered.
5. The loop (2→3-4-5) on anything the seam touched + the dry panel (A1+B1+C2+C3;
   C1 named-basis) → (b) with the full evidence block.

## 2. Duties

Kill/ledger rows same-commit at every step; close packet with the census amendment
(**THE RESIDENT SOUND BLOB IS NATIVE; five twins DELETED**), the corrections list,
and the seam-2 handoff (what the banked/data side inherits, the generator's seat);
after seam-1: seam-2 + generator, then the Spec-5 flip.
