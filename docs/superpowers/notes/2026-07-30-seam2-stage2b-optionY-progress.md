# 2026-07-30 — seam-2 stage-2b Option Y: PROGRESS (mechanism blessed · head co-linked · emitter wired) → the deletion is the next unit

> ## ✅ UPDATE 3 — STAGE 2d (sfx) + STAGE 3 (seq_opcode_tab, sound_tables_z80) ALL LANDED → checkpoint (b). The banked sound side is FULLY sigil-native.
>
> The whole banked sound HEAD + SFX block are now `.emp`-native, BINCLUDE'd; the
> only sound `.asm` left is the deferred 68k caller `sound_api.asm` (rides the flip)
> + the still-AS head `movingtrucks_pitchtable.asm` (SndDefaultPitchTable, NOT in
> this pass's scope — a remaining head file). Strict **2903 passed / 0 / 1**.
>
> **Commits (aeon `seam2-banked-data` / sigil `seam2-banked-data`):**
>   * Stage 2d additive: aeon `23eabe8` (SFX_WIN_* equ layer + sfx_blob_win_tab.emp) · sigil `a1286dd` (emit + head co-link gate).
>   * Stage 2d deletion: aeon `4b89ae2` (THE 20 SFX `.asm` deleted) · sigil `176a335` (whole-ROM gate + DSM BODY_STUB + rows).
>   * Stage 3 seq_opcode_tab: sigil `50cda1a` (emit + gate) · aeon `de06d44` (deletion).
>   * Stage 3 sound_tables_z80: sigil `52b6280` (emit + gate) · aeon `c0a81c5` (deletion, generator-emits-.emp).
>
> **What was new / the four deletions' mechanisms:**
>   * **SFX (2d) — the coupled body+head, BOTH shape-dependent.** sfx_bank.emp gained
>     an `SFX_WIN_NN = winptr(Sfx_NN)` equ layer; the NEW sfx_blob_win_tab.emp head
>     references those cross-module (the DAC SND_*_PTR mechanism). Both halves are
>     shape-dependent (the SFX block sits after the shape-dependent songs — the body's
>     SfxTable `*u8` cells hold per-shape absolute Sfx_NN addrs; every win-tab cell is a
>     winptr shifting with the base). Emit `sfx_bank{,_debug}.bin` + `sfx_blob_win_tab{,_debug}.bin`.
>     **NO syms** (CORRECTION vs the 2d scoping: no surviving AS reader of `SfxTable` —
>     sound_sfx.emp's `SfxBlobWinTab` reads are native, address is the seam-1 banked
>     carrier `$845F`). `sfx_transcode.py` moved from prebuild-auto to MANUAL (the
>     `.asm` it regenerated are deleted; the committed `.bin` embed sources stay).
>   * **seq_opcode_tab (3) — co-linked to the RESIDENT handlers.** `emit_seq_opcode_tab`
>     resolves the 32 `dc.w Seq_Op_*` cells against `native_sound_blob().symbols` (the
>     same seam-1 blob link the driver ships from). SHAPE-DEPENDENT (handlers re-base after
>     sound_sequencer's `if DEBUG==1`). `seq_opcode_tab_port` windowed oracle RETIRED with
>     the twin; `SeqOpcodeTable`/`_End` + the 32*2 span guard move into the BINCLUDE bracket.
>   * **sound_tables_z80 (3) — the generator-emits-.emp step (rows 1615/1619/1620 SETTLED).**
>     `gen_sound_tables.py` now emits `sound_tables_z80.emp` (its `emit_emp_z80()`, single
>     source of the 4 pure-math LUTs + PSG/FM vol-env tables). SELF-CONTAINED (the
>     PsgVolEnv_Ptrs/FmVolEnv_Ptrs cells are intra-module `dc.w` folding to `$8000`-window
>     addrs — the last Value16Le demand sites) + SHAPE-INVARIANT.
>   * **The BODY_STUB pattern generalized to SFX** (`SIGIL_EMP_SFX_BODY_STUB`, kill row 91);
>     the win-tab + seq + sound_tables HEADS are BINCLUDE'd UNCONDITIONALLY in soundBankHead
>     (can't be stubbed — the phase PC must advance). `SIGIL_EMP_SFX` is now vestigial too.
>
> **Proof (each deletion, both shapes):** region gate (`seam2_sfx_head_colink` /
> `seam2_seq_colink` / `seam2_soundtables_colink`, == reference slice) + whole-ROM gate
> (`seam2_sfx_rom` incl. doctored win-tab + doctored seq controls) + the DSM `mixed_dac_rom`
> 52 tranches + `./build.sh` BOTH shapes through real asl. **Assembled region (0..ASSEMBLED_LEN)
> byte-for-byte UNMOVED** except the 2 convsym header fields (`$18E-$18F`, `$1A6-$1A7`); the
> full file shrinks per deletion (deb2 — the deleted labels leave the convsym appendix).
>
> **Per-deletion artifact-CRC drift ledger (full-file s4.bin / s4.debug.bin):**
>   * baseline (post-MT): `414258` (`0x1087aac8`) / `422313` (`0xf6311a78`)
>   * after SFX deletion: `413886` (`0x119c19fc`, −372) / `421880` (`0xcf24dd29`, −433)
>   * after seq deletion: `413886` / `421880` (UNCHANGED — SeqOpcodeTable/_End labels preserved in the bracket)
>   * after sound_tables deletion: `413555` (`0x67ee0011`, −331) / `421559` (`0x53d2b731`, −321)
>   * assembled-region CRC UNMOVED throughout (the primary bar; the whole-ROM gates encode it).
>
> **Emitted artifacts (== reference slices):** `sfx_bank.bin` 1864B `0x1160dc56` / `_debug` `0x6de42f99`;
> `sfx_blob_win_tab.bin` 270B `0x96724056` / `_debug` `0x43efa523`; `seq_opcode_tab.bin` 64B `0x140f1fcc` /
> `_debug` `0x231576c2`; `sound_tables_z80.bin` 855B `0xfa0fe7c8`.

> ## ✅ UPDATE (later 2026-07-30) — THE WIRE + THE DAC DELETION LANDED (the irreversible unit is DONE, green, committed)
>
> Four commits close the unit. **aeon** `seam2-banked-data`: `cd31928` (the wire) →
> `7c4769c` (the deletion). **sigil** `seam2-banked-data`: `6de01ac` (the whole-ROM
> gate + harness) → `22d0225` (retire the row-57 oracle + close kill rows). Strict at
> the tip: **2891 passed / 0 failed / 1 ignored** (−4 vs 2895: the retired oracle;
> +3 vs the 2892 baseline: the new seam2_dac_rom gate). The assembled ROM is UNMOVED
> (674166e9 plain / 16ee80b9 debug over 0..ASSEMBLED_LEN; the seam2/m1d/seam1 gates
> are green). `./build.sh` BOTH shapes through the REAL asl reproduces the baseline
> assembled region byte-for-byte except the 2 convsym header fields ($18E-$18F,
> $1A6-$1A7) — the deb2 appendix shrinks 156 B (plain), the deleted labels leaving
> the table (Option B, out of scope); the reference s4.bin stays the FIXED gitignored
> baseline (never rebuilt/committed — the seam-1 precedent; it is gitignored, not
> tracked).
>
> **THE MECHANISM THAT WAS SETTLED (the tension the plan flagged, resolved):** the
> DSM `mixed_dac_rom` harness (~20 tranches) composes the DAC banks IN-MEMORY (the
> `org $58000` stub + `.emp` sections) — that path is INCOMPATIBLE with turning the
> body else-arm into a BINCLUDE. Resolution:
>   * **The BODY keeps a stub sub-arm** gated by a NEW harness-only define
>     `SIGIL_EMP_DAC_BODY_STUB`. The 12 sound DSM helpers (dac/mt/sfx/hblank/tranche2-9)
>     set it → they keep composing the `.emp` banks in-memory (ZERO tranche/map churn —
>     one define added). The real build + the seam-2 whole-ROM gate do NOT set it → the
>     real BINCLUDE arm.
>   * **The HEAD is ALWAYS AS-BINCLUDE'd (no stub).** It CANNOT be stubbed: for T1
>     (MT AS-included) the AS PC must advance the 90 head bytes past $585AD so the MT
>     song lands at $58607 — a hole would shift it. So the head byte-gate is: AS
>     BINCLUDEs the co-linked `dac_sample_tab.bin` (proven == the reference 90-B slice
>     by seam2_dac_head_colink). This is a NEW code path — **BINCLUDE inside a
>     `phase 08000h` block** — and it works (the whole-ROM gate is green both shapes).
>   * **THE DELETION collapsed both gates** to their native path (the seam-1
>     unconditional-BINCLUDE pattern): main.asm body = `ifdef SIGIL_EMP_DAC_BODY_STUB
>     org else BINCLUDE`; sound_bank.inc head = unconditional BINCLUDE. `m1d`/`seam1`/
>     tranche20+ (which set no DAC define) now take the BINCLUDE arm — identical ROM,
>     they need the `.bin`s present (same kind of dependency as the seam-1 blob `.bin`).
>
> **THE NEW GATE:** `crates/sigil-cli/tests/seam2_dac_rom.rs` — the seam-1
> `mixed_seam1_rom` pattern applied to the DAC: emit → assemble main.asm with
> `SIGIL_EMP_DAC` on (no stub) → whole ROM == reference BOTH shapes
> (`assert_rom_matches_convsym`) + a t24 doctored-head control. `assemble_seam2_dac_rom_as_side`
> is its AS side. This IS the primary deletion bar (the design-note "whole-ROM byte gate").
>
> **CORRECTIONS / findings for the close:**
>   * `SIGIL_EMP_DAC` is now VESTIGIAL — after the gate collapse nothing in aeon reads
>     it (main.asm uses `SIGIL_EMP_DAC_BODY_STUB`, sound_bank.inc is unconditional). The
>     12 DSM helpers + `assemble_seam2_dac_rom_as_side` still SET it (harmless no-op).
>     Cleanup candidate (drop the define; keep only BODY_STUB in the DSM helpers) —
>     deferred to avoid churn on the deletion; a dry-panel / next-pass item.
>   * `SIGIL_EMP_DAC_BODY_STUB` is TWIN SCAFFOLDING (the DSM in-memory composition
>     entry). Kill condition: when the DSM `mixed_dac_rom` DAC-composition tranches
>     retire (or convert to BINCLUDE), the stub arm + the define die and the body
>     collapses to a bare BINCLUDE. Needs a kill-list row (not yet added).
>   * The oracle-retirement ripple was REAL and caught by the byte gate:
>     `dac_sample_tab_port.rs` (t27 lane C) read the deleted `.asm` and failed 4/4 →
>     deleted same-unit (row 57's identity bar moves to the whole-ROM gate). No sibling
>     BODY oracle broke (`dac_bank_acceptance` proves the `.emp` banks vs the reference
>     SLICE, not vs a `.asm` twin — it stays).
>   * The plan's "compose the .emp HEAD in place of the AS head" resolved to
>     **AS-BINCLUDE the .emp-derived head .bin** (not a composed .emp section) — the
>     only shape that advances the AS PC correctly for T1. Both paths byte-identical.
>
> **CASCADE READINESS (2c/2d/3):** the head-BINCLUDE-in-`phase` pattern is PROVEN
> (reusable for `seq_opcode_tab` / `sfx_blob_win_tab` / `sound_tables_z80` heads). The
> `SIGIL_EMP_*_BODY_STUB` idiom is the template for any bank BODY the DSM harness must
> keep composing. mt_bank/sfx_bank are bank BODIES (`bank: $8000`, m68000) placed after
> the DAC — the `org`-resume addresses in their gates are unchanged by this unit.
>
> ## ✅ UPDATE 2 — STAGE 2c (mt_bank) LANDED (cascade authorized; DAC unit countersigned)
>
> The overseer countersigned the DAC unit and authorized the full cascade (no per-unit
> stop). Stage 2c (Moving-Trucks bank) is DONE, green, committed, the SAME wire→dual-
> proof→standalone-deletion shape as the DAC. **aeon:** `cfe2abd` (MT wire) → `19fce50`
> (MT deletion, 7 `.asm` + the verify_emit_bin fix). **sigil:** `10eb2ce` (emit + gate)
> → `f6938b9` (probe re-baseline + row closes). Strict **2894/0/1**; `./build.sh` BOTH
> shapes through real `asl` reproduces the baseline assembled region (only the 2 convsym
> header fields differ; deb2 shrinks further).
>
> **What was new vs the DAC (the reusable MT pattern):**
>   * **Shape-dependent bank** — `emit_mt_bank(aeon, debug)` emits `mt_bank{,_debug}.bin`
>     (plain 13,537 B / debug 20,275 B; the debug build adds DrumTest + HCZ2). Supplies
>     the same 3 cross-seam carriers `mt_port.rs` does (`MovingTrucks_Bank_Start`@$58000
>     + `SONG_MOVINGTRUCKS` + `SONG_COUNT`) and checks the 7 link asserts.
>   * **A SYMS file** — unlike the DAC head (consumed by `-D`), the MT bank is consumed by
>     AS-assembled 68k code: `sound_api.asm`'s `movea.l #SongTable`/`#SongPatchTable`.
>     So `emit_mt_artifacts` ALSO emits `mt_syms{,_debug}.asm` (equs at the emitted
>     addresses, extracted from the placed section labels — the seam-1 syms pattern), and
>     the MT BINCLUDE arm `include`s it. THIS is the template for any bank BODY with
>     AS-side label consumers (find them by grepping the bank's exported labels).
>   * **`SIGIL_EMP_MT_BODY_STUB`** — same idiom as DAC (kill row 91, generalized).
>   * **Ripple caught by `./build.sh`, NOT the strict suite:** `tools/verify_emit_bin.py`'s
>     `_FIXED_TARGETS` hardcoded the 6 MT `.asm` → MISSING_ASM at the build's twin-verify
>     preflight. Emptied the list (the `.bin` embed source is covered by the byte gates).
>     **LESSON for 2d/stage-3: run `./build.sh` (both shapes) per deletion — the strict
>     suite does NOT exercise build.sh's preflights (verify_emit_bin, lint).**
>   * **Probe re-baseline** — `mt_negative_probes` probe (b) was at the stale f828406
>     $60607/bank-$C; re-baselined to $58607/bank-$B with the wrong label moved to
>     $60000/bank-$C so the 5 co-residency ensures still fire (verified 4/4).
>
> ## 🔜 STAGE 2d (sfx_bank) — SCOPED for the next pass (VALVED here; the delicate half)
>
> 2d has TWO coupled parts; the second is a genuinely fresh design unit (why this pass
> valved after 2c rather than rush it):
>   1. **sfx_bank BODY** — the 18 `sfx/sfx_NN{,_patches}.asm` + `sfx_table.asm` →
>      `sfx_bank.emp` (already exists at `games/sonic4/data/sound/sfx/sfx_bank.emp`),
>      the MT pattern EXACTLY: emit `sfx_bank{,_debug}.bin` @ the sfx region ($5BAE8
>      plain size $4518 / $5D53A debug size $2AC6, from `emp_bank_map_with_mt`) +
>      `sfx_syms.asm` for the ONE AS-side consumer **`SfxTable`** (sound_sfx.asm reads
>      it; grep confirmed SfxTable is the only cross-seam body label). Shape-dependent.
>      Add `SIGIL_EMP_SFX_BODY_STUB`. NOTE: sfx_bank.emp's Sfx_NN are its OWN table-part
>      labels; its co-residency ensures read `MovingTrucks_Bank_Start` (same carrier).
>   2. **`sfx_blob_win_tab` HEAD table** (the delicate half — coordinator-ruled) — a
>      `soundBankHead` table of `dw sfx_winptr(Sfx_NN)` (with `rept` gaps for unused ids)
>      that references the Sfx_NN blob labels. When sfx_bank is BINCLUDE'd, Sfx_NN are
>      gone. **RULING: the SFX_WIN_* equ layer** — sfx_bank.emp gains a `pub` SFX_WIN_NN
>      equ layer (`winptr(Sfx_NN)` per blob), and sfx_blob_win_tab references SFX_WIN_NN
>      (co-linked, the DAC-head mechanism). The in-cell-builtin spelling (`dw winptr(Sfx_NN)`
>      inline) is a LEDGERED language-ask — DO NOT build it. `sfx_blob_win_tab.emp` does
>      NOT yet exist — it must be created (the `rept`-gap unused-id pattern → some .emp
>      construct) OR sfx_blob_win_tab.asm stays AS consuming SFX_WIN_NN from an emitted
>      syms file. This is a head-table port (like `dac_sample_tab`/`seq_opcode_tab`) — use
>      the BINCLUDE-in-`phase` pattern (proven). RE-PROBE: if the equ layer distorts bytes
>      anywhere, STOP and report. Re-baseline `sfx_negative_probes` as it re-proves.
>      SFX_WIN_MASK/BASE + sfx_winptr live in `boot_data.asm` today (seam-1 handoff §7).
>
> **STAGE 3 (after 2d):** `seq_opcode_tab` head (Value16Le proven + BINCLUDE-in-phase) +
> `sound_tables_z80` (generator-emits-.emp, rows 56/1619/1620). Then the loop (2→3-4-5) +
> the C3-HEAVY dry panel (C2 owes 4 re-derivations) → checkpoint (b).
>
> --- (original note below) ---

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
