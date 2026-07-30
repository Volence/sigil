# 2026-07-30 — seam-2 stage-2b Option Y: PROGRESS (mechanism blessed · head co-linked · emitter wired) → the deletion is the next unit

Status: **EXECUTION Y-sequence steps 1-2 + the emitter half of step 3 DONE, GREEN,
committed. The remaining main.asm/build.sh wire + THE COUPLED `.asm` DELETION (steps
3b-4) is the irreversible unit — flagged for its own careful pass (byte-exact BINCLUDE
placement + the real `./build.sh` CRC confirmation rebuilds the reference).** Sigil
branch `seam2-banked-data`. The overseer BLESSED the width-1 arm (own-run 2888/0/1).

## What landed (three committed green steps)

1. **`e9abde2` — the co-link mechanism + the width-1 Z80 SymRef arm (BLESSED).**
   Empirically settled: both recon-planned routes (use-equ-import; inline bankid/winptr
   in cells) are UNBUILT. The mechanism is cross-module equ SymRef — `dac_sample_tab.emp`
   keeps its body byte-identical, sourcing the 30 `SND_*` from `dac_samples.emp`'s
   co-linked equs (no `-D`, no mirror). PTR/LEN (width-2) resolve as-is; the width-1
   BANK cell needed `fixup_kind (Z80,1,false) => Value8` (the width-1 sibling of the
   ratified t27 Value16Le decision; the linker's u8 range check keeps the guard).
   Probe: `seam2_colink_probe.rs`. Unit: `lower/data.rs`.
2. **`490b7bb` — the head co-link, proven against the reference ROM.**
   `seam2::emit_dac_body_and_head` co-links `dac_samples.emp` + `dac_sample_tab.emp` and
   the head byte-matches `s4.bin @ $585AD` (90 B, crc `a19d1706`) in BOTH shapes
   (shape-invariant, t24). Gate: `seam2_dac_head_colink.rs`. **`dac_sample_tab.emp`
   needed NO source edit** — the co-link resolves `SND_*` cross-module without `-D`.
3. **`9dd8611` — the emitter emits the DAC artifacts.**
   `emit_sound_blob` (the bin the real build runs) now also writes `dac_blip_bank.bin`
   ($48000), `dac_shared_bank.bin` ($50000), `dac_sample_tab.bin` (the 90-B head), all
   byte-verified == reference (blip `b99c9c47`, shared `59258c89`, head `a19d1706`).
   ADDITIVE — nothing BINCLUDEs them yet.

Strict at the tip: **2892 passed / 0 failed / 1 ignored**; provenance UNCHANGED
(assembled ROM e5765873/dab4f06c via `mixed_seam1_rom_matches_reference_{plain,debug}`;
artifacts 22f69f77/414414 · d4e8d043/422466; blob c7534c84/fd2a845d).

## The remaining wire + THE DELETION COMMIT (steps 3b-4 — the irreversible unit)

The verified facts for the next porter (the mechanism is settled; this is placement):

- **`games/sonic4/main.asm:311-320`** — the `gameSoundDataIncludes` macro. Today:
  `ifndef SIGIL_EMP_DAC { include dac_samples.asm } else { org $58000 }`. The `else`
  arm's `org $58000` is a STUB (the .emp banks are supplied in-memory by the test
  harness, not the real build). The wire turns it into the REAL BINCLUDE:
  `align $8000` → `Dac_Temp_Blip: BINCLUDE "...generated/dac_blip_bank.bin"` →
  `align $8000` → `Dac_SharedBank_Start:` (`Dac_Kick:`) `BINCLUDE ".../dac_shared_bank.bin"`
  → `align $8000` (to $58000). Reproduce `dac_samples.asm`'s exact align/label/BINCLUDE
  order (per `2026-07-30-seam2-stage1-rebaseline.md` §"STAGE 2b READINESS").
- **`engine/sound/sound_bank.inc:32-38`** — the `soundBankHead` macro includes 5 head
  tables at `phase 08000h`; `dac_sample_tab.asm` is #5 (UNCONDITIONAL, at VMA `$85AD`).
  The wire gates it: `ifndef SIGIL_EMP_DAC { include dac_sample_tab.asm } else { BINCLUDE
  ".../generated/dac_sample_tab.bin" }` — the emitted head lands at `$85AD` after
  `seq_opcode_tab` exactly as the `.asm` did. (`DacSampleTable:` label must precede the
  BINCLUDE — the resident driver's `-D DacSampleTable=$85AD` and `Snd_DacLookup` pointer
  math depend on it.)
- **The `SND_*` coupling:** with `SIGIL_EMP_DAC` on, `dac_samples.asm` is skipped so AS
  no longer defines `SND_*`. Today the mixed build's AS `dac_sample_tab.asm` gets `SND_*`
  cross-seam from `dac_samples.emp`'s equs (via the joint link — `mixed_dac_rom`). Once
  the head is the BINCLUDE'd `.bin`, NO AS consumer of `SND_*` remains — the coupling
  dissolves (the whole point of Option Y). Confirm no OTHER `SND_*` reader survives
  (grep: `dac_sample_tab.asm` was the sole one).
- **`crates/sigil-harness/tests/mixed_dac_rom.rs`** — the whole-ROM proof composes the
  AS main.asm (gates on) + the `.emp` modules. It must now compose the `.emp` HEAD
  (`dac_sample_tab.emp`, co-linked) in place of the AS `dac_sample_tab.asm` when
  `SIGIL_EMP_DAC` is on, and re-prove the full ROM byte-identical BOTH shapes. This is
  the primary bar for the deletion.
- **THE DELETION:** `games/sonic4/data/sound/dac_samples.asm` + `engine/sound/dac_sample_tab.asm`
  together (body + head as a unit), rows 5-dac + 57 closed SAME-COMMIT, plain-spoken
  message. Update `dac_samples.emp`'s header (the "CONSUMERS (unchanged): dac_sample_tab.asm
  ... still AS" line is then stale — the consumer is native). Post-deletion proof:
  strict suite green (full ROM unchanged) + the emitted `.bin`s == reference (already
  gated) + optionally `./build.sh` both shapes reproduces 22f69f77/d4e8d043 (this
  REBUILDS the reference s4.bin — save the CRCs first; the build is deterministic).
- **OQ-4:** the per-sample start labels (`Dac_Kick` etc.) are ALREADY present in
  `dac_samples.emp` and consumed by the `SND_*` equs — nothing to un-suppress, no
  fixture pins their absence (the chosen mechanism references the equs, not the labels).

## The cascade (steps 5-7, unchanged from the colink-probe note)

- **2c/2d (`mt_bank`/`sfx_bank`):** bank BODIES (`bank: $8000`, m68000), the
  `emit_dac_banks` pattern — unaffected by the width-1 arm. Re-baseline the two
  negative-probe oracles (`mt_negative_probes.rs`/`sfx_negative_probes.rs`) per the
  stage-1 ledger as each lands.
- **Stage 3 `sfx_blob_win_tab`:** `dc.w winptr(Sfx_NN)` INLINE (not an equ ref) — the
  unbuilt in-cell-builtin path. **Coordinator's ruling: PREFER THE EQU LAYER** (an
  `SFX_WIN_*` equ layer at the producer, the table referencing it — the DAC head's
  mechanism, zero new machinery). The in-cell-builtin spelling is a LANGUAGE-ASK LEDGER
  ROW for the post-flip ask round. RE-PROBE stage 3; if the equ layer distorts bytes
  anywhere, STOP and report.
- **`seq_opcode_tab`** (Value16Le, proven) + **`sound_tables_z80`** (generator-emits-.emp)
  → dual proofs → deletions → loop + C3-heavy dry panel → checkpoint (b).

## Why the valve stands here

Steps 1-2 + the emitter (3a) are a complete, proven, committed green boundary. The
remaining wire (3b) + the COUPLED body+head DELETION (4) is the irreversible unit: it
edits three build files, re-composes the whole-ROM harness, deletes two `.asm`, and its
belt-and-suspenders confirmation rebuilds the reference ROM. Doing it well wants a fresh
pass with the byte-exact placement discipline, not a rushed tail. The mechanism is
settled and blessed; the placement plan above is exact. No push; the merge is the
overseer's.
