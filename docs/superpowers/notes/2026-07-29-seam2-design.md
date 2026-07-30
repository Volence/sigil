# 2026-07-29 — seam-2 design: the BANKED/DATA side + the generator conversion (DESIGN GATE)

Status: **DESIGN NOTE — answers the seam-2 brief's five deliverables. STOP 1 (the
step-0 design gate). No implementation, no twin deletion, no build change.**
Sigil branch `seam2-banked-data` (worktree `.worktrees/seam2-banked-data`), aeon
branch `seam2-banked-data`. Masters: sigil `a740c21` / aeon `6c311b5`.

**Baseline VERIFIED (both shapes reproduce, strict green):**
- plain `s4.bin` = **22f69f77 / 414414** ✓ · debug `s4.debug.bin` = **d4e8d043 / 422466** ✓
- emitted blob plain **c7534c84** · debug **fd2a845d** · syms **87b87b1b** ✓ (all match)
- PRIMARY assembled-ROM bar e5765873/dab4f06c proven by the strict suite's
  `mixed_seam1_rom_matches_reference_{plain,debug}`.
- Strict **2880 passed / 0 failed / 1 ignored** (full workspace, `--no-fail-fast`,
  `SIGIL_STRICT_GATE=1` + `SIGIL_EMIT` + `AEON_DIR` at these worktrees).
- Aeon worktree seeded via `tools/seed-worktree.sh` (the gitignored generated tree +
  editor data), then dual-built — both shapes byte-identical to the references.

Cites: `.asm`/`.emp`/main.asm are `aeon` at `6c311b5`; `.rs`/ledger/kill-list are `sigil`
at `a740c21`. Row numbers are the campaign gap-ledger / twin-scaffolding kill-list.

---

## §0 SCOPE — seam-2 is the WHOLE banked sound side

Seam-1 made the RESIDENT blob (`phase 0`, five code files) native. Seam-2 owns
everything on the `phase 08000h` / `$8000`-window BANKED side that the resident blob
READS: the DAC sample payload banks, the MT/DrumTest/HCZ2 streaming song bank, the SFX
blob block, and the engine-table HEAD (the sequencer opcode table + the DAC descriptor
table + the generated FM/PSG LUTs + the pitch/win tables). After seam-2, the ONLY sound
`.asm` left is the 68k CALLER `sound_api.asm` (its own deferred flip) — everything else
that assembles sound bytes is sigil-native. Confirmed against the seam-1 design §5 map
and kill-list rows 5/56/57.

The banked side splits into two placement classes, and the design treats them differently:

- **BANK-BODY** (`bank: $8000` payload, VMA == LMA in a high bank): `dac_samples`,
  `mt_bank` (song/pitch/patch), `sfx_bank`. `.emp` files EXIST and are windowed-proven;
  each already carries its `SIGIL_EMP_{DAC,MT,SFX}` org-resume arm in main.asm (present
  but NOT `-D`-defined, so the `.asm` still builds).
- **BANK-HEAD** (`phase 08000h`, VMA = `$8000+offset`, LMA inside the `$58000` bank —
  the row-1620 hard seam): `seq_opcode_tab`, `dac_sample_tab`, and the generated
  `sound_tables_z80` + the game pitch/win tables emitted by `soundBankHead`
  (`sound_bank.inc:32-38`, main.asm:349-354).

---

## §1 THE DATA CENSUS + CONVERT-vs-EMBED CLASSIFICATION

The Volence test: **human-authored SEMANTIC → typed `.emp` data**; **tool-GENERATED →
the generator emits sigil-consumable output**; **opaque BULK → `embed()`**. Applied per
file. Almost the entire generated-bulk column is ALREADY realized: the `.emp` files
`embed()` git-TRACKED `.bin` twins the generators emit (the DSM.4 twin pattern,
`.gitignore:25-40` un-ignores those `.bin`s; `build.sh`'s `verify_emit_bin.py` fails the
build if a `.asm`/`.bin` twin drifts). So the classification below is mostly a
CONFIRMATION of an in-flight split, plus the two genuinely-open files (`sound_tables_z80`,
the head tables).

### 1a. `data/sound/` — 29 files

| file(s) | origin | class | reason / disposition |
|---|---|---|---|
| `dac_samples.asm` | HAND (bank layout) + BULK (PCM) | **SEMANTIC shell + `embed()` payload** | The bank/align/straddle structure is human-authored semantics → `.emp` (`bank: $8000`, `bankid()`/`winptr()`/`.len`); the `.pcm`/`.bin` PCM payloads are opaque BULK → `embed()`. `dac_samples.emp` already does exactly this. |
| `song_movingtrucks.asm` (+`.bin`) | GENERATED `zyrinx_player.py --emit-native-song` | **GENERATED → `embed()` the tracked `.bin`** | Opaque packed byte stream; `mt_bank.emp` embeds `song_movingtrucks.bin`. |
| `song_drumtest.asm` (+`.bin`) | GENERATED `song_packer.py` (DEBUG-only) | GENERATED → `embed()` | DEBUG-shape member of `mt_bank.emp` (folds to `Data.empty` plain). |
| `song_hcz2.asm` (+`.bin`) | GENERATED `song_packer.py` (DEBUG-only) | GENERATED → `embed()` | DEBUG-shape member of `mt_bank.emp`. |
| `movingtrucks_patches.asm` (+`.bin`) | GENERATED `song_packer/smps` | GENERATED → `embed()` | Patch bank stream; `mt_bank.emp` embeds. |
| `hcz2_patches.asm` (+`.bin`) | GENERATED `smps_import.py` (DEBUG-only) | GENERATED → `embed()` | DEBUG-shape member. |
| `movingtrucks_pitchtable.asm` / `_stream.asm` (+`.bin`) | GENERATED `zyrinx_player.py` | GENERATED → `embed()` | The 132-entry fnum table; `mt_bank.emp` embeds `_stream.bin`. The `MT_PITCHTAB_OFFSET` contiguity `fatal` (song_table.asm) → a `.emp` ensure. |
| `song_table.asm` | HAND (song/patch pointer tables + the load-bearing bank asserts) | **SEMANTIC → `.emp` data** | `SongTable`/`SongPatchTable` = `dc.l <song-label>` pointer tables; folded into `mt_bank.emp` as `data` pointer arrays. Its no-straddle/window-top/count asserts → `.emp` ensures (§3). |
| `sfx/sfx_NN.asm` ×9 (+`.bin`) | GENERATED `sfx_transcode.py` | GENERATED → `embed()` | Per-SFX FM/PSG blobs; `sfx_bank.emp`'s `table` embeds each `.bin` as a cell payload. |
| `sfx/sfx_NN_patches.asm` ×9 (+`.bin`) | GENERATED `sfx_transcode.py` | GENERATED → `embed()` | Per-SFX FM patch banks (2 zero-length, PSG-only). |
| `sfx/sfx_table.asm` | GENERATED `sfx_transcode.py` (but header says "HAND-OWNED") | **SEMANTIC → `.emp` `table`** | The sparse id→blob `SfxTable`; folded into `sfx_bank.emp`'s `table` (`cell:*u8, key:$33..=$B9, hole:0`). Regenerated by the tool but the SEMANTICS (id→label mapping) are the readable form. |
| `sfx_blob_win_tab.asm` | HAND (id→`$8000`-window ptr) | **SEMANTIC → `.emp` data (BANK-HEAD)** | `dw sfx_winptr(Sfx_NN)`. Currently `soundBankHead` phase-head, R3-deferred to stay `.asm`. Seam-2 folds it into the head-table emission (§3); it needs the `sfx_winptr`/`SFX_WIN_*` helpers seam-1 parked in boot_data.asm's arm re-homed here. |

### 1b. `engine/sound/` — the 3 still-AS carriers

| file | origin | class | reason / disposition |
|---|---|---|---|
| `seq_opcode_tab.asm` | HAND | **SEMANTIC → typed `.emp` (BANK-HEAD)** | 32 `dc.w <resident-Seq_Op_*>` jump table (`Value16Le`). `seq_opcode_tab.emp` EXISTS + windowed-proven (kill row 56). The cells are RESIDENT handler addresses → resolved as imports from the seam-1 blob (§3). |
| `dac_sample_tab.asm` | HAND | **SEMANTIC → typed `.emp` (BANK-HEAD)** | 10 × 9-byte descriptor. `dac_sample_tab.emp` EXISTS + windowed-proven (kill row 57). The SOLE consumer of the `SND_*` sample constants (§2, §3 — the circularity dissolves). |
| `sound_tables_z80.asm` | GENERATED `gen_sound_tables.py` | **GENERATED → the generator emits `.emp`** (NOT `embed()`) | THE exception (row 1619). 4 pure-math LUTs (FmPitchTableZ / PsgDivisorTableZ / LogVolumeLutZ / CarrierMaskTableZ) + the PsgVolEnv/FmVolEnv id-lists AND `dw <resident-label>` pointer tables (`PsgVolEnv_Ptrs` 11 cells, `FmVolEnv_Ptrs` 3 cells — row 1615). `embed()` is WRONG here: the pointer tables carry LINK labels a flat blob can't hold, and the LUTs are comptime-computable. So the generator emits `.emp` (or the data-table DSL single-sources it). This is the file the whole generator-conversion arc turns on. |

### 1c. Borderline cases argued both ways (for the gate)

- **`sfx_table.asm` — GENERATED-tool-output vs SEMANTIC-table.** The tool regenerates it,
  which argues GENERATED→embed. BUT the id→label mapping is the readable intent, the
  table is tiny, and `sfx_bank.emp` already models it as a typed `table` (not an embedded
  blob) — the `table` is the diffable, label-validated form the Volence test prefers for
  a semantic table. **Ruling: SEMANTIC → `.emp` `table`.** The tool still emits the `.asm`
  twin while asl survives; post-flip the `.emp` `table` is canonical and the tool's
  `sfx_table.asm` emission dies (its data lives in the `table` rows).
- **`song_table.asm` — HAND but derives from song ids the generator assigns.** The
  pointer values are `dc.l <generated-song-label>`, so one could argue GENERATED. BUT the
  TABLE STRUCTURE (which songs exist, their ids, the parallel patch table, the bank
  asserts) is hand-authored orchestration, not tool output. **Ruling: SEMANTIC → `.emp`
  data** (folded into `mt_bank.emp`); the songs it points at are the embed() payloads.
- **`dac_samples.asm` — split file.** Correctly BOTH: the layout shell is SEMANTIC `.emp`,
  the PCM is `embed()`. Not a borderline so much as the canonical two-class exemplar.

**Net:** the classification is already the shape of the existing `.emp` files. The only
file whose class is genuinely UN-realized is `sound_tables_z80.asm` (generator-emits-`.emp`),
and it is the lead blocker (§2) and the natural first execution step.

---

## §2 THE GENERATOR CONVERSION DESIGN + reproducibility settlement + ojz ruling

### 2a. The generators in scope (SOUND only)

Surveyed `build.sh` + `games/sonic4/prebuild.sh` + `tools/`:

| generator | emits | invoked | tracking |
|---|---|---|---|
| `gen_sound_tables.py` | `engine/sound/sound_tables_z80.asm` | MANUAL (not in prebuild) | output git-TRACKED |
| `zyrinx_player.py --emit-native-song` | `song_movingtrucks.asm`+`.bin`, `movingtrucks_pitchtable*.asm`+`.bin` | MANUAL | TRACKED |
| `song_packer.py` | `song_drumtest/hcz2.asm`+`.bin`, patch banks | MANUAL | TRACKED |
| `smps_import.py` | `hcz2_patches.asm`+`.bin` | MANUAL | TRACKED |
| `sfx_transcode.py generate` | `sfx/sfx_NN{,_patches}.asm`+`.bin`, `sfx_table.asm` | **prebuild.sh:78 (EVERY build)** | TRACKED (regen-overwritten) |

### 2b. The reproducibility ledger-row settlement (rows 178 / 529 / 1619)

**Settled: the SOUND generators are already reproducible-by-tracking; the row-178
non-determinism is NOT theirs.** Evidence: (1) every sound generated file is git-TRACKED
(unlike the gitignored OJZ/level tree row 178 is actually about); (2) no `import
random` / `time` / `datetime` in any sound generator; (3) `sfx_transcode.py` regenerates
its tracked outputs on EVERY build and `verify_emit_bin.py` gates them — the seed+dual
rebuild this session left all 24 twins PASS and both ROMs byte-identical, an empirical
determinism proof. So the reproducibility row does not block seam-2's sound generators.

The row-178 non-determinism (a fresh worktree building ~131 KB larger) is the GITIGNORED
level-generator tree (`ojz_strip_gen` → `entity_data.asm` etc.), a separate session that
"owns a session of its own" (row 178) and rides the flip / a dedicated level-toolchain
step, NOT seam-2. `tools/seed-worktree.sh` is the standing workaround this session used.

The convergence (rows 1619 ↔ 1620 blocker-1): the DEEP reproducibility question seam-2
DOES own is `sound_tables_z80.asm` — a generated file included in the bank head. The fix
is **the generator emits `.emp`** (or the data-table DSL single-sources it), so the LUTs
are comptime-computed FROM the tool's formulas and the `dw`-pointer tables carry real link
labels. This same step dissolves the `SND_*` comptime-source blocker (see 2d), so the two
rows converge on one toolchain move.

### 2c. The emit-tool architecture (inherited from seam-1, extended)

Seam-1's `emit_sound_blob` (Option A) is the template: sigil natively links `.emp`
sources → deterministic `.bin` artifact(s) asl BINCLUDEs + a generated `z80_sound_syms.asm`
CONTRACT for surviving AS consumers; `build.sh` fails LOUDLY if the emitter is missing;
the assembled-ROM CRC is the provenance bar. Seam-2 EXTENDS the same binary (not a new
one) to also emit the banked side:

- The emitter's `--out-dir` gains the banked artifacts: the DAC banks, the MT/SFX bank
  bodies, and the bank-HEAD table blob (or one combined banked `.bin` per placement region).
- Because the SAME emitter already links the seam-1 resident blob, the `Seq_Op_*` handler
  VMAs `seq_opcode_tab` needs are IN-SCOPE in the same link (blocker 2 dissolves — no
  cross-frontend AS export needed; the syms contract that seam-1 emits FOR the AS
  seq_opcode_tab.asm becomes an internal import once seq_opcode_tab is native).
- The bank-body `.emp` files place `bank: $8000` (VMA==LMA, sigil's placement pass); the
  bank-head tables place at their phased window LMA — the emitter computes both, so the
  "unprobed phased-head placement mode" (blocker 3) and the "org-advance resume" (blocker
  4) are emitter-internal facts, not mixed-map unknowns.

### 2d. The `SND_*` comptime-source settlement — the blocker DISSOLVES

The decisive scale-2 blocker (row 1620 #1 / row 1623): `dac_sample_tab`'s 30 `SND_*`
cells are build-time constants ADDRESS-DERIVED from where the DAC samples land
(`SND_KICK_PTR = (Dac_Kick & $7FFF) | $8000`, …), and a width-1 Z80 `db` can't carry a
link symbol, and `extern()` returns a `LinkExpr` `dc` rejects — so they MUST be comptime
ints, and `-D` is circular (values unknown until `dac_samples` assembles).

**KEY FINDING: `dac_sample_tab.asm` is the SOLE consumer of the `SND_*` sample constants**
(grep confirmed — no other `.asm`/`.emp` reads `SND_{BLIP,KICK,…}_{BANK,PTR,LEN}`). And
`dac_samples.emp` ALREADY folds the identical arithmetic via `bankid(L)`/`winptr(L)`/`.len`
builtins at LINK. So when the SAME sigil link places `dac_samples.emp` (it knows
`Dac_Kick`'s LMA) and emits `dac_sample_tab`, the descriptor cells fold DIRECTLY from
`bankid(Dac_Kick)`/`winptr(Dac_Kick)`/`Dac_Kick.len` — the `SND_*` names disappear
entirely. The circularity was purely an artifact of the two files living in DIFFERENT
frontends (asl places samples, sigil folds `SND_*` = circular). Co-link them under sigil
and the blocker is gone — NO `-D`, NO 30-value mirrored twin. This is the emit-tool
route's central payoff, and it obsoletes the "-D or mirror" framing in row 1623/1620.

(Execution note: `dac_samples.emp` must EXPORT the per-sample start labels so
`dac_sample_tab.emp` can `bankid(Dac_Kick)` them — the header currently says those labels
are "deliberately not ported, zero external consumers." dac_sample_tab becomes the first
consumer → un-suppress the labels. Cheap; flagged for execution.)

### 2e. `ojz_entity_gen` — rides the FLIP, NOT seam-2

`ojz_entity_gen.py` reads editor JSONs → `data/generated/ojz/act1/entity_data.asm` (GAME
entity/ring placement), invoked by `ojz_strip_gen.py generate` from prebuild. Ruling:
**flip-side (or its own level-toolchain session), not seam-2.** Three reasons: (1) it
feeds GAME level data, a different domain from the sound stack seam-2 scopes; (2) its
output is on the GITIGNORED, row-178-non-reproducible level tree — folding it in balloons
seam-2 into the whole OJZ pipeline; (3) its consumers (`act_descriptor`, `ObjDef_*`) are
game `.emp`/`.asm` tied to the t24-t34 game arc + the flip, not sound. Named explicitly so
the gate can overrule if it wants it bundled — but the recommendation is keep seam-2 pure
sound.

---

## §3 LINK/EMIT MECHANICS AT SCALE-2 (the db-link class + the bank asserts)

### 3a. The `db` can't-carry-a-link-symbol class (row 1623)

Two cell classes exist on the banked head, and only one hit the row-1623 wall:

- **`dac_sample_tab` cells** — routed through the `SND_*` circularity, they LOOK like they
  need link symbols but are CONSTANTS. §2d dissolves them (fold from `bankid`/`winptr`/
  `.len`, all comptime once sigil places the samples). No `db`-link needed.
- **`seq_opcode_tab` / `sfx_blob_win_tab` cells** — these ARE genuine 16-bit Z80
  addresses (`dc.w`), and `Value16Le` already carries a link symbol in a Z80 `dc.w`
  (proven at scale-1, kill row 56; the width-2 path, unlike width-1 `db`). `seq_opcode_tab`
  imports `Seq_Op_*` from the resident blob (same link, §2c); `sfx_blob_win_tab` reads the
  `Sfx_NN` part labels `sfx_bank.emp`'s `table` defines (module-scoped link symbols). The
  `db`-can't-carry-link constraint is a WIDTH-1 rule; the only width-1 banked cells are the
  `dac_sample_tab` ds_bank/ds_codec/ds_rate bytes, which are constants — so the width-1
  link problem never actually arises on the banked side once §2d is applied.

**Settlement: the row-1623 "scale-2 complication" is real for the WIDTH-1 constant cells
but is resolved by co-linking (fold from placement), not by defeating the `db`-link rule
(which stays correct — a 1-byte Z80 pointer IS unrepresentable).**

### 3b. The bank-alignment / no-straddle asserts (LOAD-BEARING — where they live)

These are hardware-correctness walls (a bank straddle = one `SetBank` can't cover the
block = garbage pitch/dispatch mid-frame). Enumerated:

| assert (AS today) | where it lives post-conversion |
|---|---|
| `dac_samples.asm` per-blob straddle `fatal` ×2 | `dac_samples.emp`'s `bank: $8000` property → the emitter's ALWAYS-ON no-straddle placement check (subsumes the hand `if`). |
| `song_table.asm:73` MT no-straddle (`Bank_Start>>15 <> (Patches_End-1)>>15`) | `mt_bank.emp` `bank: $8000` + the emitter check. |
| `song_table.asm:81` window-top guard | `mt_bank.emp` ensure. |
| `song_table.asm` `MT_PITCHTAB_OFFSET` contiguity `fatal` | `mt_bank.emp` ensure (the song↔pitchtable byte-offset the header bakes). |
| `song_table.asm` `SONG_COUNT` length `error` ×2 | `mt_bank.emp` ensure over the `data` pointer-array `.len`. |
| main.asm:459 SFX no-straddle (`Sfx_33>>15 <> (Sfx_B9_Patches_End-1)>>15`) | `sfx_bank.emp` `bank:`/`item_align` + emitter check. |
| main.asm:467 SFX co-residency (`Sfx_33>>15 <> SND_ENGINE_TABLE_BANK`) | `sfx_bank.emp` `ensure(bankid(...) == bankid("MovingTrucks_Bank_Start"))`. |
| co-residency (every stream/patch bank == engine-table bank) | `mt_bank.emp` ×5 `ensure(bankid(...) == bankid("MovingTrucks_Bank_Start"))` (present today). |

**Answer to the brief's question (ensures vs emitter vs both): BOTH, by kind.** The
NO-STRADDLE class is STRUCTURAL — it becomes a `bank: $8000` section property enforced by
the emitter's always-on placement pass (the arithmetic lives ONCE, in the linker, not
re-spelled per file). The CO-RESIDENCY / OFFSET / COUNT class is RELATIONAL (this block
must share a bank with THAT one; this offset must equal that baked value) — it stays as
per-file `ensure(...)` link asserts in the `.emp`, because only the source knows the
relation. The mixed-build subtlety: co-residency asserts reference `MovingTrucks_Bank_Start`
(the AS-side bank-start label in main.asm's surviving `soundBankHead` bracket) — resolved
cross-seam via the proven `bankid("MovingTrucks_Bank_Start")` idiom (ports.rs `probe_b`)
while asl owns that label; once the head is fully native, it becomes an internal reference.

### 3c. The syms contract at scale-2

Seam-1 emits `z80_sound_syms.asm` (handler VMAs FOR the AS seq_opcode_tab). Seam-2's
emitter emits the analogous contract for any BANKED symbol a SURVIVING AS consumer reads.
Post-§2d there are essentially NONE on the sound side (the sole `SND_*` consumer becomes
native; `Seq_Op_*` becomes an internal import). The one live cross-seam edge is the
OTHER direction — the resident blob and 68k `sound_api` read the banked engine tables
(FmPitchTableZ etc.) window-relative — those are already `-D`/equ-carried and unchanged by
seam-2 (they resolve against `sound_tables_z80`, which seam-2 makes native, so its labels
enter the same link). Net: the syms contract SHRINKS across seam-2 as the banked head
stops being an AS island.

---

## §4 THE RETIREMENT SET (per-row dispositions + identity bars)

Dual-proof discipline (seam-1 precedent): every retirement is proven in the TWIN-PRESENT
state (windowed oracle / mixed byte gate) BEFORE deletion; the whole-blob/whole-ROM byte
gate is the post-deletion bar; deletion commits standalone; kill/ledger rows same-commit.

| kill row | `.asm` | seam-2 disposition | identity bar |
|---|---|---|---|
| 5 (dac_samples entry) | `dac_samples.asm` | **DONE (seam-2 stage-2b, aeon 7c4769c)** — DELETED; the `.emp` is canonical via the emitter (BINCLUDE'd at $48000/$50000). | ✔ whole-ROM byte gate both shapes (`seam2_dac_rom`); `.emp` banks == reference (`seam2_dac_emit`). |
| 5 (mt_bank block) | `song_movingtrucks/drumtest/hcz2.asm`, `movingtrucks_patches/pitchtable*.asm`, `hcz2_patches.asm`, `song_table.asm` | **DELETE at seam-2** (the `.asm` include stream; the `.bin` twins STAY as `embed()` source). | whole-ROM byte gate; `mt_bank.emp` == the `$5BAE8/$5D53A` reference windows both shapes; the MT contiguity/co-residency ensures fire on doctored inputs. |
| 5 (sfx block) | `sfx/sfx_NN{,_patches}.asm` ×18, `sfx_table.asm` | **DELETE at seam-2** (`.bin` twins stay). | whole-ROM byte gate; `sfx_bank.emp` `table` == the `$5C230/$5DC82` reference windows. |
| 56 | `seq_opcode_tab.asm` | **DELETE at seam-2** (kill condition = "banked-head seam lands" — this IS it). | windowed 64 B oracle → whole-ROM head-table byte gate; `Seq_Op_*` imports resolve to the same VMAs the syms contract exported. |
| 57 | `dac_sample_tab.asm` | **DONE (seam-2 stage-2b, aeon 7c4769c)** — DELETED; the head is co-linked (SND_* resolve cross-module from `dac_samples.emp`) and BINCLUDE'd at VMA $85AD. | ✔ whole-ROM head-table byte gate (`seam2_dac_head_colink` 90 B both shapes + `seam2_dac_rom` whole-ROM + t24 doctored control); the windowed oracle (`dac_sample_tab_port`) retired with the twin. |
| 1619 | `sound_tables_z80.asm` | **DELETE at seam-2** IF the generator-emits-`.emp` step lands (the LUTs + `dw`-pointer tables become native); else it stays the last AS head file and rides the flip. | new head-table byte gate: the emitted LUT bytes + pointer cells == the current generated `.asm` bytes both shapes. |
| — | `sfx_blob_win_tab.asm` | **DELETE at seam-2** (folded into the native head; re-home the `sfx_winptr`/`SFX_WIN_*` helpers from boot_data.asm's arm — the seam-1 handoff §7 debt). | head-table byte gate: `dw sfx_winptr(Sfx_NN)` == the reference win-tab. |

**RIDES THE FLIP, not seam-2** (unchanged by this seam):
- Row 5's BODY code twins (the 68k gate-off twins) + the resident already-deleted set.
- Row 54 (`game_debug` SONG_*/SFXID_* const mirrors) — game-constants `.emp` home / flip.
- `sound_api.asm` (rows 10/24/36/43) — the 68k caller, its own deferred parcel (seam-1 §8).
- The `soundBankHead` macro + `sound_bank.inc` + main.asm's `save/cpu z80/phase 08000h`
  bracket: the bracket EMPTIES as its includes go native; the macro retires when the last
  include (`sound_tables_z80`) is native; the bracket itself is deleted at the flip with
  the gate arms.

**Open sub-question for the gate:** is the row-5 "Spec 5" disposition for
dac_samples/mt/sfx a DELIBERATE deferral (keep the banked body windowed until the flip,
seam-2 = head-only) or a stale estimate (seam-2 SHOULD flip them)? The brief's deliverable-5
framing ("what remains for Spec-5 after seam-2") reads as the LATTER — seam-2 retires the
whole banked side. This design assumes that (the emit-tool route makes it one coherent
step) but flags the row-5 wording as needing a same-commit update at execution.

---

## §5 SEQUENCING TO THE FLIP — the flip's final input set

Ordering: **seam-1 (resident code) ✓ DONE → seam-2 (banked data + sound generators) →
Spec-5 (main/config flip)**; `sound_api` slots anywhere before Spec-5 as its own 68k parcel.

**After seam-2 lands, the Spec-5 flip's FINAL INPUT SET is:**

1. **main.asm / config** — collapse every `ifndef SIGIL_EMP_*` gate to the `.emp` path,
   delete the else-`org` resume arms, delete the `soundBankHead`/`phase 08000h` bracket.
2. **The 68k gate-off BODY twins** (kill row 5): every ported engine/game `.asm` twin
   (collision/vdp/hblank/math/anims/plane_buffer/parallax/children/load_object/
   entity_window/section/tile_cache/…) deleted as its gate collapses.
3. **`sound_api.asm`** (rows 10/24/36/43) — the 68k sound caller; recommend its own small
   parcel BEFORE the flip (`SIGIL_EMP_SOUND_API` scaffold exists), leaving the flip pure.
4. **`config/sound_ids.asm` + `constants.asm`** (row 54) — the SONG_*/SFXID_*/BUTTON_*
   mirrors collapse into `use`s / a game-constants `.emp`.
5. **The LEVEL/OJZ generator reproducibility** (row 178) + `ojz_entity_gen` — its own
   session (gitignored-tree determinism); NOT sound, but a flip precondition for a clean
   from-scratch build. (Corrects the brief's "expected" list, which omitted it.)
6. **asl itself** — retired once no `.asm`/`.inc`/macro survives the build; the sigil
   frontend + emitter is the whole toolchain.

**Correction to the brief's expected list:** it named "main/config + 68k gate-off twins +
sound_api + asl." Add (a) the row-54 game-constants mirrors and (b) the level-generator
reproducibility/`ojz_entity_gen` session as flip inputs. Otherwise confirmed.

---

## OPEN QUESTIONS (for the gate / execution)

- **OQ-1 (scope ruling)** — confirm seam-2 = the WHOLE banked side (bank-body + head), not
  head-only; i.e. row-5's dac/mt/sfx dispositions move from "Spec 5" to "seam-2". (§4.)
- **OQ-2 (emit-tool extension)** — endorse extending `emit_sound_blob` (one binary) to
  emit the banked artifacts + the head-table blob, co-linking the resident blob so
  `Seq_Op_*` + the DAC sample labels are in-scope (§2c/§2d), vs a mixed-map placement route.
  The emit route is the recommendation (it dissolves blockers 1-4 of row 1620).
- **OQ-3 (generator-emits-`.emp` first)** — is `sound_tables_z80.asm` → generator-emits-`.emp`
  a PREREQUISITE first execution step (it single-sources the LUTs + `dw`-pointer tables and
  is the row-1619↔1620 convergence), or can seam-2 land the bank-body first and take the
  head (incl. sound_tables_z80) as a second sub-step? Recommend: bank-body first (lowest
  risk, `.emp` proven), then head, with sound_tables_z80's generator step gating the head.
- **OQ-4 (dac_samples label export)** — un-suppressing `dac_samples.emp`'s per-sample start
  labels so `dac_sample_tab.emp` can `bankid(Dac_Kick)` them (§2d) is byte-neutral (labels
  emit nothing) — confirm no windowed-oracle fixture pins their absence.
- **OQ-5 (phased-head placement mode, row-1620 #3)** — the emit-tool route makes the phased
  head an emitter-internal layout, but the emitter must still emit the head bytes at the
  correct `$8000+offset` window addresses with LMA in the `$58000` bank; this placement
  mode is unproven at the emitter (seam-1 emitted `phase 0` VMA==0; the head is
  `phase $8000` VMA≠LMA). First machinery the execution parcel stands up.
