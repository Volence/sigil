# Sound-migration T3 (SFX tranche) — COMPLETE, awaiting Volence checkpoint

Written 2026-07-09 at the end of the T3 implementation session. **Both branches are UNMERGED
— NO merge without Volence's checkpoint.** This note is the checkpoint packet + next-arc
orientation. **T3 is the LAST sound-data tranche: on merge, the sound-data migration arc is
DONE.**

## What shipped

Plan (authoritative, incl. every deviation/review outcome in `## Execution notes`):
`docs/superpowers/plans/2026-07-09-sound-migration-t3-sfx.md` (rulings R1–R8; the R1
sfx_table-hand-owned ruling is Volence's 2026-07-09 design-check answer).

**sigil branch `sound-migration-t3`** (worktree `.worktrees/sound-migration-t3`, off master
`29bcef5`, 15 commits `c168a61..960b6ff`):
- **Capability probes** — P1 phase-dw compound deferral (win-tab shape, vma≠lma) instant-
  green; P2 zero-byte embed; P3 int elements in `[*u8; N]` pointer arrays (REAL gap, closed
  + range-checked 0..=u32::MAX after spec review caught silent truncation).
- **`partial_fold`** (the tranche's one engine change, found by Task 5's REAL RED): a
  deferred fixup target now bakes every AS-env-resolvable subterm to `Expr::Int`, leaving
  only genuinely-external leaves symbolic — the win-tab's `dw sfx_winptr(Sfx_NN)` trees
  carried `SFX_WIN_MASK`/`SFX_WIN_BASE` equs the linker can't see (P1 missed it by testing
  literal masks). Applied to all three deferral arms (dw/db/imm32); pinned by 6 dedicated
  tests falsified via a temporarily-neutered fold; survived 8 differential adversarial
  byte-identity probes in review.
- **Gates:** `sfx_port.rs` (region byte-gate, per-shape map bases — zero divergences both
  shapes incl. resolved pointer cells); mixed_dac_rom.rs +2 (DAC+MT+SFX full-ROM, both
  shapes byte-identical, win-tab byte pinned at rom[0x6045F] = `E8 BA`); 
  `sfx_negative_probes.rs` (5 probes incl. the NEW grown-mt overlap probe — abutment-benign
  control falsified); `phase_dw_winptr_defer.rs` + `partial_fold_defer.rs`.

**aeon branch `sigil-emp-sfx`** (off master `b0e5a66`, 5 commits `5a01237..6f5efb5`):
- **`sfx_bank.emp`** (195 lines): 18 embeds (two zero-byte PSG patch banks) + 18
  self-adjusting align pads (only sfx_3C/sfx_B6 fire) + hand-owned 135-entry sparse
  `SfxTable: [*u8; 135]` (int-0 nulls) + the co-residency ensure (bankid-label idiom);
  define-FREE — the block is shape-invariant, only the map base moves.
- **R1 executed:** `sfx_transcode.py generate` no longer writes `sfx_table.asm` (hand-owned,
  header rewritten with the four-place add-an-SFX checklist); `--emit-table` bootstrap
  escape hatch kept, now with a loud clobber warning (whole-branch-review ruling).
- **R2:** `SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN` → `config/sound_ids.asm`;
  `SFX_BLOB_BANK = SND_ENGINE_TABLE_BANK` → main.asm (the co-residency guard justifies the
  identity; kills the Z80-imm8 cross-seam need).
- **R6/R8:** `SIGIL_EMP_SFX` gate (org resumes `$64230` plain / `$65C82` debug); 18 `.bin`
  twins committed under narrow gitignore exceptions (verify_emit_bin 24/24).
- asl build byte-verified UNCHANGED repeatedly, incl. a from-scratch rebuild in review
  (plain `8ce6dd7e…`, debug `13c7b063…`).

## Verification state

- Mixed `.asm`+`.emp` ROM, ALL THREE gates on: **byte-identical both shapes** (same convsym
  allowlists as T1/T2). The win-tab's nine cross-seam `dw` entries resolve end-to-end
  through `partial_fold` + the joint link (byte-inspected).
- Region gate: `sfx_bank` 1864 B byte-identical both shapes. Negative space: straddle,
  wrong-bank, table-length, wrong-sym-for-null, grown-mt overlap — all loud, all falsified.
- Workspace: **1491 tests / 0 failed**, strict-gate, clippy `-D warnings` clean — re-run
  FIVE times sequentially post-review, all clean (see flake disposition).
- Two-pronged whole-branch adversarial review: **zero Critical; both prongs
  checkpoint-ready.** All fix-first items landed (`960b6ff`, `6f5efb5`).

## The one honest caveat (F1 flake — disposition)

During review, ONE non-reproducible full-workspace failure of the PRE-EXISTING T1 test
`mixed_dac_rom_matches_assembled_reference` was observed (~1 in 40 runs, wholesale layout
divergence, under heavy parallel load coincident with concurrent worktree/clippy builds).
It passes on every commit of the range individually; the pipeline was audited deterministic
(no global mutable state; vec-order iteration; branch tests read-only); five sequential
clean workspace runs followed. Judged environmental, RECORDED as a watch item: if it ever
recurs, dump the resolved section LMAs to pin assembler-vs-environment. The T2-era claim
"workspace fully green" holds, but this note is the honest asterisk.

## Recorded-not-fixed (carry forward)

1. **F1 flake watch item** (above).
2. **Misleading "internal: … anchor label" diagnostic** when both sides of a link ensure
   are unresolved standalone — T2 carry-forward #5, hit again by sfx_bank.emp; still open
   (diagnostics-quality).
3. **Bare cross-seam equ reads in `.emp`** still don't exist (bankid-label idiom is the
   workaround; third tranche to hit it) — S2-D14-adjacent spec-integration candidate.
4. **Fable's empyrean spec-integration debt grows:** T2's pass (defines, imm32, the
   ensure-spelling gap) + now `partial_fold` semantics (deferred-target partial folding is
   observable behavior of the AS seam) + P3's int-elements-in-pointer-arrays typing rule.
5. `--emit-table` emits the LEGACY pre-R2 table shape (now with a loud warning); option-2
   upgrade (emit the post-R2 hand-owned shape) if bootstrap ever sees real use.
6. Cosmetic minors consciously left: map_path/load_map ×3 in the mixed harness helpers;
   `AEON_DIR` default ×4 in mt_port/sfx_port (fold only if touched anyway — keep-copies
   convention).

## Checkpoint asks for Volence

1. **Merge decision:** sigil `sound-migration-t3` → master (`--no-ff`, house pattern) and
   aeon `sigil-emp-sfx` → master. Both branches clean, all green; aeon branch is off
   `b0e5a66` (already contains the bg-restore commit — no ordering dance this time).
2. Accept the **F1 flake disposition** (environmental, watch item) — or ask for deeper
   root-causing before merge.
3. Note **the sound-data migration arc is DONE** on merge (DSM.8 T0–T3 all shipped).

## Next after merge (from the plan + DSM.8)

Re-evaluate **S2-D14(a)(d)(e) + 9d** against what the whole arc actually demanded (the
recurring hits: bare cross-seam equ reads, the anchor-label diagnostic, partial-fold
semantics), then the remaining **Plan-7 #10 (compression builtins)** → **spec FREEZE** →
the wider 68k migration campaign. Fable also owes the accumulated empyrean spec-integration
pass (carry-forward #4).
