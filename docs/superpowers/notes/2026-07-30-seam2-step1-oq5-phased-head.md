# 2026-07-30 — seam-2 step-1: OQ-5 phased-head placement STOOD UP (the first hard test)

Status: **EXECUTION step 1 of the endorsed sequence — the OQ-5 phased-head
placement machinery, TDD, GREEN.** Sigil branch `seam2-banked-data`
(`.worktrees/seam2-banked-data`). Follows the countersigned design gate
(`2026-07-29-seam2-design.md`) and the coordinator's OQ-5 ruling: "the first hard
test, stood up before anything depends on it, demanded-feature-TDD-or-STOP."

## Result: NOT STOP — the placement mode BUILDS on existing machinery

Row-1620 blocker 3 flagged the phased-head placement (`vma: $8000` window ≠ `lma`
in the `$58000` bank) as an UNPROBED mode neither `z80_init.emp` (`vma:$0`, LMA 0)
nor `dac_samples.emp` (`bank:$8000`, VMA==LMA) matched. It turns out to be the SAME
`vma_base`/`lma` decoupling seam-1 already uses for the resident blob (VMA 0 / LMA
`$3DE`), applied at (VMA `$8000` / LMA `$58000`) — no new linker feature needed.

**The mechanism (verified in `sigil-link/src/lib.rs`):** `link()` Pass 1 defines every
symbol at `vma_origin() + offset` (the WINDOW address); Pass 2 places the bytes at
`sec.lma` (the PHYSICAL address). So a section's labels resolve to the window while
its bytes sit in the high bank, and a cross-section `Value16Le` cell resolves to its
target's VMA regardless of either section's LMA. The `.emp` SECTION owns the window
VMA via its attribute (`section seq_opcode_tab (cpu: z80, vma: $8000)`); the
map/emitter owns only the physical LMA (a map `vma_base` is overridden by the section
attr — the source is the single source of truth for the window).

## The stand-up test — `crates/sigil-cli/tests/seam2_phased_head.rs` (2 tests, GREEN both shapes)

Faithful, not a toy: it lowers the REAL `seq_opcode_tab.emp` and co-links it against
the REAL resident-blob handler VMAs (seam-1's `native_sound_blob(...).symbols` — the
exact contract seam-1 exports), at the phased head (LMA `$5856D` in the song bank).

- `phased_head_cells_resolve_to_resident_vmas_bytes_at_bank_lma` — proves (1) the
  first cell (`$E0`=MEV_VOL → `Seq_Op_Vol`) equals `Seq_Op_Vol`'s REAL resident VMA
  little-endian (a phase-0 address < `$8000`, NOT dragged into the head's own bank);
  (2) the linked section's bytes land at LMA `$5856D`; (3) `SeqOpcodeTable` resolves
  to the `$8000` window VMA, not the bank LMA. Both shapes.
- `phased_head_emits_identical_bytes_to_the_windowed_oracle` — cell resolution is
  PLACEMENT-INVARIANT: the phased head (LMA in the `$58000` bank) emits byte-for-byte
  the same 64-byte table as the windowed oracle (VMA==LMA==`$8000`), with distinct
  LMAs and identical window VMAs. So the scale-1 byte gate (`seq_opcode_tab_port`)
  CARRIES to the scale-2 placement — the head port adds no new byte-drift surface.

## What this de-risks for the rest of seam-2

- **Row-1620 blocker 3 (phased placement) — DISSOLVED with a passing test.** The head
  tables (`seq_opcode_tab`, `dac_sample_tab`, and the generator-emitted
  `sound_tables_z80` LUTs) place at their window VMA with LMA in the bank via the
  existing map/link path.
- **Blocker 2 (Seq_Op_* resident export) — confirmed a non-issue under the emit route:**
  the head resolves its cells against the resident handler VMAs supplied as the
  co-link's symbol table (here from seam-1's contract); in the whole-ROM emit the SAME
  binary links resident + banked, so those labels are in-scope with no cross-frontend
  AS export.
- **Blocker 1 (SND_* comptime source)** was already dissolved by the design (§2d:
  dac_sample_tab folds from `bankid/winptr/.len` once co-linked with dac_samples).
- The remaining execution is now well-grounded: extend `emit_sound_blob` to emit the
  bank-body (dac_samples/mt/sfx, `bank:$8000`) + the phased head, per shape; dual-prove
  each; delete the twins (rows 5/56/57 + sfx_blob_win_tab + sound_tables_z80 via the
  generator-emits-`.emp` step); loop + dry panel (C3 heavy) → checkpoint (b).

## Provenance

Additive test only — no emitter/engine/`.emp` byte change. Baseline UNCHANGED: plain
`22f69f77/414414`, debug `d4e8d043/422466`, blob `c7534c84/fd2a845d`, syms `87b87b1b`.
Strict baseline 2880/0/1 + 2 new phased-head tests = **2882/0/1** expected. The
emit-tool extension + the twin retirements are the byte-moving work ahead.
