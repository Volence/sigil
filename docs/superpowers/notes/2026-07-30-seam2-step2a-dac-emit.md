# 2026-07-30 — seam-2 step-2a: the DAC bank-body EMIT (additive, GREEN)

Status: **EXECUTION step 2 (bank-body first, OQ-3 ordering) — stage 2a: the DAC
bank emit + reference-slice proof. ADDITIVE, no build/deletion, provenance
UNCHANGED.** Sigil branch `seam2-banked-data`. Follows OQ-5 (step 1, committed).

## What landed

- `sigil_harness::seam2::emit_dac_banks(aeon) -> DacBanks { blip, shared }` — lowers
  the REAL `dac_samples.emp`, places its two `bank:` sections at the current-baseline
  pins, links, returns the two bank payloads. Byte-deterministic (`.emp` + embedded
  `.pcm`/`.bin` + toolchain). The Option-A artifact a later stage BINCLUDEs.
- `crates/sigil-cli/tests/seam2_dac_emit.rs` (2 tests, GREEN): the emitted banks are
  BYTE-IDENTICAL to the reference ROM slices (`s4.bin` @ `$48000` blip 2880 B /
  `$50000` shared 30908 B), the surrounding align gaps are zero pad, and the emit is
  deterministic across runs.

This is the "twins present, both paths byte-identical" dual proof for the DAC bank —
green BEFORE any deletion. `mixed_dac_rom` (the whole-ROM mixed gate) already proves
the same bytes compose into the full ROM; this stage adds the STANDALONE emit that
the real build will consume.

## CRITICAL FINDING — the bank-body oracles carry a STALE layout (re-baseline owed)

The authoritative current-baseline DAC layout (`s4.lst`, and `mixed_dac_rom`'s CODE):
  * `dac_blip_bank`   @ **`$48000`** — bank id **`$9`** (`$48000 >> 15`).
  * `dac_shared_bank` @ **`$50000`** — bank id **`$A`** (`$50000 >> 15`).

But `dac_samples.emp`'s header comment AND `dac_port.rs` pin the STALE
**"aeon-f828406 layout"** `$50000`/`$58000` (bank `$A`/`$B`). `dac_port.rs` is a
self-consistent WINDOWED oracle at that older baseline — it proves the
`bankid()`/`winptr()`/`.len` FOLD machinery, NOT the current ROM's addresses; its
hardcoded `SND_BLIP_BANK=$A` is f828406's value (the current ROM has `$9`). It passes
because it compares `.emp`-at-`$50000` to hand-computed-`$A` — both at `$50000`.

`mixed_dac_rom.rs`'s CODE is correctly current-baselined (`$48000`/`$50000`), but its
TOP DOC-COMMENTS (lines 18, 173) still cite the stale "aeon-f828406 pins / $50000 /
$58000" — a comment-claim-audit finding.

**This trap is exactly the false-green the coordinator warned of:** building the DAC
emit at the `.emp`-header/`dac_port` addresses ($50000/$58000) would have produced a
wrong artifact that "passes" against a wrong reference slice. This stage's emit + test
use the CURRENT baseline, verified against `s4.lst` and the real `s4.bin`.

**Owed to the finisher (byte-neutral, at canonicalization time):**
1. Re-baseline `dac_port.rs` (map + the 30 `SND_*` pins) OR retire it in favor of the
   current-baselined proof — its f828406 pins are stale drift-guards.
2. Fix the stale header comment in `dac_samples.emp` ("$50000/$58000, aeon-f828406")
   and `mixed_dac_rom.rs`'s top doc-comments to the current `$48000`/`$50000`.
3. The `mt_bank`/`sfx_bank` oracles need the SAME re-baseline audit before their emits
   (mt_bank header cites `$60607`/`$60000`-based addresses; the current main.asm
   SIGIL_EMP_MT resume is `$5BAE8`/`$5D53A` — reconcile against `s4.lst` first).

## Next stages (for the finisher — the valve stopped here at a clean green boundary)

- **2b (DAC wire+delete):** add the DAC emit to `emit_sound_blob`'s output; main.asm
  gameSoundDataIncludes BINCLUDEs `generated/dac_blip_bank.bin` (after `align $8000`)
  + `generated/dac_shared_bank.bin`, replacing the `include dac_samples.asm`; DELETE
  `dac_samples.asm`. Gate: `mixed_dac_rom` + assembled-ROM CRC unchanged. Kill row 5
  (dac entry) moves from Spec-5 to here (design §4, OQ-1). Standalone deletion commit.
- **2c/2d:** `mt_bank` then `sfx_bank`, same shape (re-baseline the pins FIRST per the
  finding above; mt_bank is `-D DEBUG` per-shape; sfx_bank is shape-invariant content).
- **step 3:** the phased head (`seq_opcode_tab` + `dac_sample_tab` — the SND_* fold
  from `bankid/winptr/.len` per design §2d) + `sound_tables_z80` generator-emits-`.emp`.
  OQ-5 (committed) proved the phased-head placement machinery.
- **step 4/5:** remaining retirements (rows 56/57 + sfx_blob_win_tab + sound_tables_z80)
  + the loop + C3-heavy dry panel (three owed C2 re-derivations) → checkpoint (b).

## Provenance

Additive (new harness module + tests; no `emit_sound_blob` binary change, no build.sh,
no main.asm, no deletion). Baseline UNCHANGED: plain `22f69f77/414414`, debug
`d4e8d043/422466`, blob `c7534c84/fd2a845d`, syms `87b87b1b`. Strict 2880 + 2 (OQ-5) +
2 (DAC emit) = **2884/0/1** expected. Reference DAC slices: blip crc `b99c9c47`,
shared crc `59258c89`.
