# 2026-07-30 — seam-2 CLOSE PACKET: THE BANKED SOUND SIDE IS SIGIL-NATIVE

Status: **CHECKPOINT (b) COUNTERSIGNED. Seam-2 execution COMPLETE.** The merge,
provenance re-baseline, roadmap update, and any corpus-wide comment sweep are the
overseer's. No push, no merge, no rebase from the porter seat.

Tips (branch `seam2-banked-data` both repos): **aeon `b6c95cf`** · **sigil `554ded5`**.
Strict **2904 passed / 0 failed / 1 ignored**.

---

## THE HEADLINE

After seam-1 made the RESIDENT Z80 code blob native, seam-2 makes the WHOLE
`$8000`-window BANKED data side native: the DAC sample banks, the Moving-Trucks
streaming song bank, the SFX block, and the engine-table HEAD (the FM/PSG LUTs +
vol-env tables, the sequencer opcode table, the SFX window-pointer table, the DAC
descriptor table). **Everything that assembles a sound byte is now sigil-native,
BINCLUDE'd from a `.emp` source. Only two sound `.asm` remain — `sound_api.asm`
(the 68k caller) and `movingtrucks_pitchtable.asm` (the SndDefaultPitchTable head)
— and BOTH are flip inputs, not seam-2 work.**

The Volence convert-vs-embed test, exercised across ALL FIVE classes:

| class | file(s) | disposition |
|---|---|---|
| **SEMANTIC shell + `embed()` payload** | `dac_samples.asm` | the bank layout is `.emp`, the PCM is `embed()`ed — the two-class exemplar (stage-2b). |
| **GENERATED → `embed()` the tracked `.bin`** | `song_*.asm`, `movingtrucks_*.asm`, `hcz2_*.asm` (7); `sfx/sfx_NN{,_patches}.asm` (18) | opaque packed byte streams; `mt_bank.emp` / `sfx_bank.emp` embed the committed `.bin`s. |
| **SEMANTIC → `.emp` data** | `song_table.asm` (→ `mt_bank.emp` pointer arrays); `sfx/sfx_table.asm` (→ `sfx_bank.emp` `table`) | the pointer/id tables are hand-authored orchestration → typed `.emp`. |
| **SEMANTIC → typed `.emp` (BANK-HEAD)** | `dac_sample_tab.asm`, `seq_opcode_tab.asm`, `sfx_blob_win_tab.asm` | the phased `$8000`-window head tables; cells resolve as link symbols (co-linked). |
| **GENERATED → the generator emits `.emp`** | `sound_tables_z80.asm` | **THE FIRST generator-emits-`.emp` realization** (`gen_sound_tables.py`'s `emit_emp_z80()` single-sources the pure-math LUTs) — the class the whole generator-conversion arc turned on (rows 1615/1619/1620). |

---

## THE SEAM STORY (arc + rulings)

**1. Design gate (STOP 1).** `2026-07-29-seam2-design.md` answered the brief's five
deliverables: the data census + convert-vs-embed classification; the generator
conversion design; the link/emit mechanics at scale-2 (the `db`-can't-carry-a-link
class + the load-bearing bank asserts); the retirement set; the sequencing to the
flip. Rulings endorsed at the gate: **OQ-1** seam-2 = the WHOLE banked side (not
head-only — row-5's dac/mt/sfx move from "Spec 5" to seam-2); **OQ-2** extend the
ONE `emit_sound_blob` binary to emit the banked artifacts (co-linking the resident
blob so `Seq_Op_*`/DAC labels are in-scope); **OQ-3** bank-body first, then head;
**OQ-4** un-suppress the DAC per-sample labels (byte-neutral).

**2. OQ-5 — the phased-head placement mode.** `2026-07-30-seam2-step1-oq5-phased-head.md`
stood up the first machinery: a head at `phase $8000` (VMA ≠ LMA, physically in
the `$58000` bank) — proven the emitter places the head bytes at the correct
window addresses. The reusable BINCLUDE-in-`phase` pattern.

**3. The audit + THE FORK (Option Y).** The stage-2b probe empirically settled that
BOTH recon-planned routes (use-equ-import; inline `bankid`/`winptr` in cells) were
UNBUILT, and settled the mechanism: **cross-module equ SymRef** (the head sources
its constants from the producer's co-linked equs; no `-D`, no mirror). The DSM
in-memory-composition tension forced THE FORK, ratified per-stage and generalized:
- **The BANK BODY keeps a stub arm** gated by `SIGIL_EMP_{DAC,MT,SFX}_BODY_STUB`
  (the DSM `mixed_dac_rom` tranches compose the `.emp` banks in-memory; the real
  build + the whole-ROM gate take the BINCLUDE arm). Kill row 91.
- **The HEAD is ALWAYS BINCLUDE'd** (no stub) — it can't be stubbed, the phase-block
  PC must advance past it so the following content lands correctly. Like the DAC head.

**4. The co-link mechanism.** The head cells resolve as CROSS-MODULE link symbols
against a producer-side equ layer that folds from placement: DAC's `SND_*_PTR =
winptr(Dac_*)`; SFX's `SFX_WIN_NN = winptr(Sfx_NN)`; seq's `dc.w Seq_Op_*` against
the resident `native_sound_blob().symbols`; sound_tables' intra-module
`PsgVolEnv_Ptrs`/`FmVolEnv_Ptrs` folding to `$8000`-window addresses. The in-cell
builtin spelling (`dc.w winptr(X)` inline) stays a LEDGERED language-ask; the equ
layer is the shipped stand-in.

**5. The four deletion units** (each: twins-present region gate → whole-ROM gate →
standalone deletion → `./build.sh` both shapes → kill/ledger rows same-commit):
- **2b DAC** (prior porter) — `dac_samples.asm` + `dac_sample_tab.asm`; the co-link
  dissolved the `SND_*` comptime-source circularity (blocker of row 1620/1623).
- **2c MT** (prior porter) — 7 Moving-Trucks `.asm`; the SYMS-file template
  (`mt_syms.asm` for `sound_api.asm`'s `movea.l #SongTable`); the `verify_emit_bin`
  preflight lesson.
- **2d SFX** (this porter) — the coupled body+head, BOTH shape-dependent: `sfx_bank.emp`
  gains the `SFX_WIN_*` equ layer, the new `sfx_blob_win_tab.emp` head references it.
  **CORRECTION: NO syms** — no surviving AS reader of `SfxTable` (sound_sfx is native
  `.emp`; its `SfxBlobWinTab` reads resolve via the seam-1 banked carrier `$845F`).
  `sfx_transcode.py` moved prebuild-auto → MANUAL.
- **3 seq_opcode_tab + sound_tables_z80** (this porter) — seq co-linked to the resident
  handlers (shape-dependent, handlers re-base on `__DEBUG__`); sound_tables the
  generator-emits-`.emp` realization (self-contained, shape-invariant).

---

## CORRECTIONS LIST

- **The stale-`sfx_syms` catch** — the 2d scoping predicted a `sfx_syms.asm` for
  `SfxTable`; grep showed NO surviving AS reader (sound_sfx is native), so NO syms
  file is emitted. (Countersigned correct.)
- **Stale-pin audit** — every LMA pin ($5BAE8/$5D53A/$5845F/$5856D/$58000) verified
  against the CURRENT `s4.lst`, not a stale layout (contrast the DAC f828406 trap).
- **The fork story** — BODY_STUB (DSM in-memory) vs unconditional HEAD (phase-PC must
  advance); `SIGIL_EMP_{DAC,SFX}` are now vestigial (the arms key on `_BODY_STUB`).
- **The two-unbuilt-routes probe** — the in-cell `dc.w winptr(X)` inline spelling is a
  ledgered language-ask, NOT built; the `SFX_WIN_*` equ layer shipped instead.
- **The MT preflight lesson applied** — `./build.sh` both shapes per deletion caught
  the `verify_emit_bin`/regeneration traps early (the `sfx_transcode` MANUAL move
  prevented deleted-`.asm` regeneration).
- **The lens A comment sweep** (coordinator-ruled, at-next-touch) — `sfx_bank.emp`'s
  pre-existing parcel tags + change-history narration rewritten present-tense,
  byte-neutral, confined to the one file.

---

## PER-DELETION ARTIFACT-CRC DRIFT LEDGER (full-file s4.bin / s4.debug.bin)

| point | s4.bin | s4.debug.bin |
|---|---|---|
| baseline (post-MT) | 414258 / 0x1087aac8 | 422313 / 0xf6311a78 |
| after SFX deletion | 413886 / 0x119c19fc (−372) | 421880 / 0xcf24dd29 (−433) |
| after seq deletion | 413886 (unchanged: SeqOpcodeTable/_End kept in bracket) | 421880 (unchanged) |
| after sound_tables deletion | 413555 / 0x67ee0011 (−331) | 421559 / 0x53d2b731 (−321) |
| after dry-panel F2 span guard | **413577 / 0xeff2396f (+22)** | **421579 / 0x1e9097bc (+20)** |

The **assembled region (`0..ASSEMBLED_LEN`) is byte-for-byte UNMOVED** across every
deletion — the only sub-`ASSEMBLED_LEN` diffs are the 4 allowlisted convsym header
fields (`$18E-$18F`, `$1A6-$1A7`). Full-file drift = deleted labels leaving the
convsym appendix (deb2, by design); F2's +22/+20 = 2 new guard labels joining it.

Emitted artifacts (== reference slices): `sfx_bank.bin` 1864B `0x1160dc56` / `_debug`
`0x6de42f99`; `sfx_blob_win_tab.bin` 270B `0x96724056` / `_debug` `0x43efa523`;
`seq_opcode_tab.bin` 64B `0x140f1fcc` / `_debug` `0x231576c2`; `sound_tables_z80.bin`
855B `0xfa0fe7c8`.

---

## THE FLIP'S FINAL INPUT SET (as corrected at the design gate + this pass)

1. **main.asm / config** — collapse every `SIGIL_EMP_*` gate to the `.emp` path,
   delete the else-`org` resume arms + the `soundBankHead` / `phase 08000h` bracket.
2. **The 68k gate-off BODY twins** (kill row 5) — deleted as each gate collapses.
3. **`sound_api.asm`** (rows 10/24/36/43) — the 68k sound caller, its own parcel.
4. **`config/sound_ids.asm` + `constants.asm`** (row 54) — the SONG_*/SFXID_*/BUTTON_* mirrors.
5. **The level/OJZ generator reproducibility + `ojz_entity_gen`** — its own session.
6. **asl itself** — retired once no `.asm`/`.inc`/macro survives.

**Three NEWLY-SURFACED flip inputs from this pass:**
- **`movingtrucks_pitchtable.asm`** — the SndDefaultPitchTable, the LAST surviving AS
  head in `soundBankHead` (out of this pass's scope; the next head to port or a flip input).
- **Kill row 92 — the vestigial `boot_data.asm` sound scaffolding**: the dead
  `z80_sound_syms.asm` include (its `Seq_Op_*` equs have no AS consumer post-seq) +
  the `SFX_WIN_*`/`sfx_winptr`/`sfx_bankid` helpers (no AS consumer post-sfx). Drop
  the include + stop `seam1` emitting it (the syms contract shrinks, design §3c) +
  drop the 4 helpers — a byte-neutral seam-1 cleanup, deferred here.
- **The cross-repo count harness-assert** — tie `seam1.rs`'s `PSGVOLENV_COUNT`/
  `FMVOLENV_COUNT` (0x0B/3) to the emitted `PsgVolEnv_Ids`/`FmVolEnv_Ids` section-label
  lengths so the resident scan count can't drift from the generator's list (gap-ledger,
  the id/ptr half is already generator-structural).

---

## KILL / LEDGER ROW STATE

**Kill-list (twin-scaffolding):**
- Row 5 — sfx block **RETIRED** (the 20 SFX `.asm` deleted, joining dac/mt).
- Row 56 — `seq_opcode_tab` **CLOSED** (the banked-head seam landed; the windowed
  oracle retired with the twin).
- Row 91 — `SIGIL_EMP_*_BODY_STUB` **+ `SIGIL_EMP_SFX_BODY_STUB`** (the 10 SFX-and-
  downstream DSM helpers); kill = the DSM composition tranches convert to BINCLUDE.
- Row 92 — **NEW**: the vestigial `boot_data.asm` sound scaffolding (see flip inputs).

**Gap-ledger:**
- Rows **1615 / 1619 / 1620 SETTLED** — the Value16Le census discharged (the last two
  demand sites `PsgVolEnv_Ptrs`/`FmVolEnv_Ptrs` native); the generator-emits-`.emp`
  step landed; the generated-file toolchain blocker done.
- **NEW**: the in-cell-`winptr()` language-ask (equ-layer is the shipped stand-in;
  demand 2 — DAC + SFX heads); the vol-env count-linkage hazard (generator-structural
  for id==ptr; cross-repo count-hardening deferred).
- Row 1805 (RHS-only data-proc length guards) — the win-tab + vol-env span guards
  inherit it (no emitted-length introspection); unchanged.

---

## THE COMMITS

**aeon `seam2-banked-data`:** `4b89ae2` (SFX deletion) · `23eabe8` (SFX .emp twins) ·
`de06d44` (seq deletion) · `c0a81c5` (sound_tables deletion) · `a972996` (dry-panel A) ·
`9ecb4ad` (dry-panel C) · `b6c95cf` (sfx_bank comment sweep).

**sigil `seam2-banked-data`:** `a1286dd` (SFX emit+gate) · `176a335` (SFX whole-ROM
gate + DSM BODY_STUB + rows) · `50cda1a` (seq emit+gate) · `52b6280` (sound_tables
emit+gate) · `3692024` (dry-panel A/B) · `554ded5` (dry-panel C ledger/kill-list).

The gates: `seam2_sfx_head_colink` / `seam2_seq_colink` / `seam2_soundtables_colink`
(region, both shapes) + `seam2_sfx_rom` (whole-ROM, 5 tests incl. 3 doctored controls)
+ the DSM `mixed_dac_rom` 52 tranches. The four C2 re-derivations
(structural-vs-relational asserts · SFX_WIN_* fold · embed byte-equivalence ·
stale-pin audit) all verified. `./build.sh` both shapes green through real asl.
