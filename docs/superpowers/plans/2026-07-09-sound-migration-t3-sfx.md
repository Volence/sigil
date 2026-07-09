# Sound-migration T3 — the SFX tranche Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the SFX block's DATA (9 blobs + 9 patch banks + `SfxTable` + the two SFX
fatals) to `.emp`, mixed-build byte-identical to the AS reference in BOTH build shapes — the
LAST sound-data tranche.

**Architecture:** ONE define-free `.emp` module (`sfx_bank.emp`) supplies everything from
`Sfx_33` through `SfxTable_End`; the block's CONTENT is identical in both shapes (no DEBUG
members — only its base address shifts because it sits after the shape-dependent song
tables), so the per-shape variation lives entirely in the map region base and the gate's org
pins. `sfx_blob_win_tab.asm` STAYS `.asm` (DSM.7 pre-ruling — it's a `soundBankHead` macro
arg inside the Z80 phase head); its `dw sfx_winptr(Sfx_NN)` exprs become the tranche's only
`.asm`→`.emp` reads and are probed FIRST. `sfx_table` becomes HAND-OWNED (Volence design-check
2026-07-09, ruling R1) — the generator stops regenerating it every build.

**Tech Stack:** Rust (sigil workspace), Motorola 68000 + Z80 asm (aeon), `.emp` (Spec 2).

**Design authority:** `docs/superpowers/specs/2026-07-08-sound-migration-tranche1-design.md`
(DSM.1–9, Volence-approved) + the 2026-07-09 design-check ruling (R1 below). This plan's
survey verified the design against live files — no contradictions; deltas frozen as facts.

**Process:** the T2 pattern verbatim — TDD w/ recorded RED (falsification pass where a step
is legitimately instant-green, T2 Task-6 precedent), commit-per-task, two-stage reviews on
load-bearing tasks (1, 3, 4, 5), whole-branch adversarial review, NO merge without a Volence
checkpoint. Sigil work in a worktree off master (branch `sound-migration-t3`); aeon work on
branch `sigil-emp-sfx` off aeon master.

---

## The verified fact base (survey 2026-07-09, post-T2-merge)

All addresses re-derived from the CURRENT `s4.lst`/`s4.debug.lst` (aeon master `4710ef3` +
the uncommitted bg-restore working tree; reference sha256s unchanged: plain `8ce6dd7e…`,
debug `13c7b063…` — see sigil-harness `golden/PROVENANCE.md`).

**Layout (the whole block SHIFTS between shapes; content is IDENTICAL — `$748` = 1864 bytes
in both):**

| symbol | plain | debug | len |
|---|---|---|---|
| `Sfx_33` (block start = T2's org resume) | `$63AE8` | `$6553A` | — |
| `Sfx_B9_Patches_End` | `$64014` | `$65A66` | blobs+patches `$52C` |
| `SfxTable` | `$64014` | `$65A66` | `$21C` (135×4) |
| `SfxTable_End` (block end = T3's org resume) | **`$64230`** | **`$65C82`** | — |

Byte-sum cross-check: blobs 58+58+286+46+267+30+184+71+98 = 1098, patches 7×32+2×0 = 224,
plus TWO align pads (only the odd blobs: `sfx_3C` 267 B, `sfx_B6` 71 B) = 1324 = `$52C`;
+ `$21C` table = `$748`. Arithmetic matches the .lst pins exactly.

**The 9 SFX (ids hardcoded in `tools/sfx_transcode.py` `_CORE_SFX_IDS`, :1720):**
`$33` RING_RIGHT, `$34` RING_LEFT, `$35` DEATH, `$36` SKID, `$3C` ROLL, `$62` JUMP,
`$AB` SPINDASH, `$B6` DASH, `$B9` RINGLOSS. 18 generated files in
`games/sonic4/data/sound/sfx/` (`sfx_NN.asm` + `sfx_NN_patches.asm`), each with an
`--emit-bin` `.bin` twin ON DISK but GITIGNORED. **Two patch banks are zero-length**
(`sfx_36_patches.bin`, `sfx_62_patches.bin` — PSG-only SFX). Every one of the 18 `.asm`
files ends with `align 2` (patch banks are 32/0 bytes so their pads never fire; the two odd
blobs' pads DO fire — the 2 pad bytes in the sum above).

**The include region** — `games/sonic4/main.asm:228-246` (inside the `gameSoundDataIncludes`
macro, body ends :263, expanded from `engine/engine.inc`): the 18 blob/patch includes in
id order (blob then its patches), then `sfx_table.asm` (:246). The two fatals:
- :252 straddle — `if (Sfx_33>>15) <> ((Sfx_B9_Patches_End-1)>>15)` →
  `fatal "SFX blob set straddles a $8000 bank boundary; SFX_BLOB_BANK invalid (split blobs or add per-blob banking)"`
- :260 co-residency — `if (Sfx_33>>15) <> SND_ENGINE_TABLE_BANK` →
  `fatal "SFX blobs not co-located with the engine-table bank (Sfx_33 bank {Sfx_33>>15} != {SND_ENGINE_TABLE_BANK}) — Sfx_Frame's dispatch/table reads would see the wrong bank"`

**The win_tab** — `games/sonic4/data/sound/sfx_blob_win_tab.asm`, passed to `soundBankHead`
at main.asm:147 (Z80 `phase 08000h` blob). Entries: `dw sfx_winptr(Sfx_NN)` for the 9 SFX +
`rept … dw 0` gap fillers keyed on the `SFXID_*` equs. `sfx_winptr` is a FUNCTION
(`engine/sound/sound_sfx.asm:58`): `(((addr) & SFX_WIN_MASK) | SFX_WIN_BASE)` — so each
entry is a compound `(Sfx_NN & mask) | base` expr in a **Z80 phase (vma≠lma) LE `dw`**.

**Cross-seam consumer surface (exhaustive grep, engine/ + games/ minus data/sound/sfx):**
- The win_tab's 9 `dw sfx_winptr(Sfx_NN)` — `.asm` reading `.emp` labels; T0's dw deferral
  covers compound exprs in principle → Probe P1 (the tranche's PROBE-FIRST item).
- The two fatals (`Sfx_33`, `Sfx_B9_Patches_End`) — port INTO the `.emp` (gated out).
- NOTHING else reads `Sfx_NN`/`Sfx_NN_Patches*` labels. (`FmPatch_len`, `SFX_WIN_MASK`,
  `SFX_WIN_BASE` are engine-`.asm`-defined and consumed only by `.asm` — unaffected.)
- **Z80 imm8 consumers**: `engine/sound/sound_sfx.asm` uses `ld a, SFX_BLOB_BANK` (:256,
  :553, :714), `sub SFX_ID_BASE` (:544, :706), `cp SFX_TABLE_LEN` (:546, :708). imm8 NEVER
  defers (T2 carry-forward #4) — these equs must stay resolvable at AS-time → ruling R2.

**`sfx_table.asm`** (`games/sonic4/data/sound/sfx/sfx_table.asm`, 156 lines) — the ONLY
per-build-regenerated sound file (`prebuild.sh:78` → `sfx_transcode.py generate` →
`generate_all()` :1733 / `emit_sfx_table_asm()` :1644). Content: header banner
(GENERATED/DO NOT EDIT), `SFX_BLOB_BANK = sfx_bankid(Sfx_33)`, `SFX_ID_BASE = $33`,
`SFX_COUNT = 9`, `SFX_TABLE_LEN = 135`, then `SfxTable:` — 135 sparse `dc.l` entries
(9 `Sfx_NN` + 126 `dc.l 0` for unused ids in `$33..$B9`), `SfxTable_End:`, and a trailing
`if (SfxTable_End-SfxTable)/4 <> SFX_TABLE_LEN` self-check. `SFX_BLOB_BANK` lives HERE (not
`config/game.asm`) purely for include-order visibility — `sfx_bankid()`/`Sfx_33` don't
exist yet at config-include time (game.asm:15-24 documents the contract).

**The verifier** — `tools/verify_emit_bin.py` auto-enrolls ALL `sfx/*.asm` EXCEPT
`sfx_table.asm` (excluded by name, :168 — pointer table, no byte-payload twin) and gates
`build.sh` (:97, ~26 ms). So the 18 blob/patch pairs are ALREADY twin-drift-guarded; T3's
`.bin` commits need no verifier change.

**Zero-entry gaps in the table** (for the `.emp` transcription): `$37-$3B` = 5, `$3D-$61` =
37, `$63-$AA` = 72, `$AC-$B5` = 10, `$B7-$B8` = 2 → 9 + 126 = 135. ✓

## Frozen rulings

- **R1 — `sfx_table` is HAND-OWNED (Volence design-check, 2026-07-09).** The generator
  stops writing it: `generate_all()` drops the `emit_sfx_table_asm()` call from the default
  `generate` path (keep the emitter function behind an explicit opt-in flag, e.g.
  `generate --emit-table`, for future bootstrap — exact spelling implementer's call,
  recorded in execution notes). `sfx_table.asm` stays COMMITTED for the ungated asl build,
  header rewritten: GENERATED/DO-NOT-EDIT banner → hand-owned contract note + the
  add-an-SFX checklist (update `_CORE_SFX_IDS`, the main.asm includes, THIS table, and
  `sfx_bank.emp`'s table — all four, or the build fails loud via the table self-check and
  the `.emp` length ensures). Blobs/patches stay per-build generated (unchanged).
- **R2 — the four table equs move to hand-owned homes** (kills the Z80-imm8 cross-seam
  need, the R2-of-T2 pattern): `SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN` → plain ints,
  moved to `games/sonic4/config/sound_ids.asm` beside the `SFXID_*` ladder (their natural
  home; no visibility problem for ints). `SFX_BLOB_BANK` → defined in `main.asm`
  immediately after `SND_ENGINE_TABLE_BANK` (:130 area, OUTSIDE the T3 gate), spelled
  `SFX_BLOB_BANK = SND_ENGINE_TABLE_BANK` — sound because the :260 fatal (and its `.emp`
  ensure successor) asserts exactly that equality, and it dissolves the include-order
  constraint that forced generator emission. All four REMOVED from `sfx_table.asm`.
  Byte-neutral for asl (equates emit nothing) — verified both shapes in Task 2.
- **R3 — the win_tab stays `.asm` this arc** (DSM.7 pre-ruling confirmed: it's inseparable
  from the Z80 phase head). Its `dw (Sfx_NN & SFX_WIN_MASK) | SFX_WIN_BASE` entries are the
  only `.asm`→`.emp` reads — Probe P1 proves the deferral BEFORE any port work. If P1 REDs
  and the fix exceeds ~a day, STOP and surface to Volence (that would mean T3's seam is not
  what T0 proved, a design-level surprise).
- **R4 — one define-free `.emp` module.** `sfx_bank.emp` needs NO `-D` (the block has no
  DEBUG members; both shapes' content is byte-identical — only the map base differs).
  18 embeds in include order, each followed by a self-adjusting conditional pad
  (`if Blob.len % 2 == 1 { byte(0) } else { Data.empty }` — T2's `_align` idiom) mirroring
  the `.asm` files' unconditional trailing `align 2`; only the `sfx_3C`/`sfx_B6` pads fire
  today. Labels match the `.asm`'s (`Sfx_NN`, `Sfx_NN_Patches`). The generated patch files'
  interior Count/len asl asserts are NOT ported — covered by the `.bin` twin verifier +
  ROM byte-identity (T2 precedent for non-load-bearing generated asserts).
- **R5 — guard mapping.** Straddle fatal (:252) → the section's `bank: $8000` no-straddle
  property. Co-residency fatal (:260) → `ensure(bankid("Sfx_33") ==
  bankid("MovingTrucks_Bank_Start"), "<the original fatal message>")` — the bankid-label
  idiom (T2 Deviation 2; bare cross-seam equ reads still don't exist). The table self-check
  → enforced structurally by the type `[*u8; SFX_TABLE_LEN]` with `const SFX_TABLE_LEN =
  135` + `const SFX_COUNT = 9`, both annotated as UNCHECKED MIRRORS of
  `config/sound_ids.asm` (T2's `9302751` mirror-comment discipline).
- **R6 — the gate.** `ifndef SIGIL_EMP_SFX` wraps main.asm:228-262 (the 19 includes + both
  fatals); `else` org-resumes: `ifdef __DEBUG__` → `org $65C82`, else → `org $64230` (hard
  pins w/ PROVENANCE comment — T1/T2 precedent). Independent third gate; harness keeps
  DAC-only and DAC+MT tests intact and adds all-three-on.
- **R7 — map region `sfx_bank`, PER SHAPE** (the first shape-dependent region base):
  plain `lma_base = 0x63AE8, size = 0x4518`; debug `lma_base = 0x6553A, size = 0x2AC6`
  (both to the `$68000` bank top). Tests build the map string per shape (`map_toml(debug)`).
- **R8 — the 18 `.bin` twins get narrow gitignore exceptions + commit** (T2's Step-3
  pattern; pre-check = run `tools/verify_emit_bin.py`, which already enrolls all 18).
  Zero-length files commit fine. `sfx_table.asm` stays OUT of the verifier (already
  excluded by name — nothing to change).

## File structure

- `crates/sigil-frontend-as/tests/` — P1 probe (win-tab-shaped dw deferral in a Z80 phase).
- `crates/sigil-frontend-emp/tests/lower_data.rs` — P2 (zero-byte embed), P3 (null entries
  in a pointer array) + any minimal fix they force.
- `aeon: tools/sfx_transcode.py` — R1 generator change.
- `aeon: games/sonic4/config/sound_ids.asm`, `games/sonic4/main.asm`,
  `games/sonic4/data/sound/sfx/sfx_table.asm` — R2 equ moves + R6 gate + R1 header rewrite.
- `aeon: .gitignore` + the 18 committed `.bin`s.
- `aeon: games/sonic4/data/sound/sfx/sfx_bank.emp` — the port (Task 3).
- `crates/sigil-cli/tests/sfx_port.rs` — region byte gate (mt_port.rs sibling).
- `crates/sigil-harness/src/lib.rs` — `assemble_mixed_sfx_as_side` sibling (+`SIGIL_EMP_SFX`).
- `crates/sigil-harness/tests/mixed_dac_rom.rs` — +2 tests (DAC+MT+SFX, both shapes).
- `crates/sigil-cli/tests/sfx_negative_probes.rs` — Task 6.

---

### Task 1: capability probes (sigil side, no aeon dependency)

The T2 Task-2 pattern: prove every construct the port needs that no existing test exercises.
Write test → RED (or instant-green ⇒ pinned regression, still committed) → minimal fix only
if RED; a gap bigger than ~a day → STOP, surface to the orchestrator.

- [ ] **P1 (LOAD-BEARING, run before anything else): the win-tab dw deferral.** In
  `crates/sigil-frontend-as/tests/` beside the T0 `db_dw_defer` tests (mirror their
  structure): assemble a unit containing a Z80 `phase` region with
  `dw (ExtSym & $7F00) | $8000` where `ExtSym` is NOT defined in-unit — assert the fragment
  carries a 2-byte hole + a `Value16Le` fixup whose target is the full compound expr tree
  (grep the T0 tests for the exact kind/shape names). Then LINK it with `ExtSym` supplied at
  a known address and assert the resolved LE bytes equal `((addr & $7F00) | $8000)` in
  little-endian order. Also the exact production shape: the same expr with the real mask
  values (`& $7FFF | $8000` — read `SFX_WIN_MASK`/`SFX_WIN_BASE` from
  `engine/sound/sound_sfx.asm:55-58` and use their literal values). Negative control: an
  unresolved `dw` in a context T0 deliberately left loud (if any) still errors.
- [ ] **P2: zero-byte embed.** In `crates/sigil-frontend-emp/tests/lower_data.rs`:
  `data X = embed("empty.bin")` where the fixture is a ZERO-BYTE file (add
  `tests/vectors/empty.bin`) — label defined, zero bytes emitted, following item lands at
  the same offset (T2's P1 proved `Data.empty`; this proves the embed-of-empty-FILE path).
- [ ] **P3: null entries in a pointer array.** Same file:
  `data T: [*u8; 5] = ["A", 0, 0, "B", 0]` — sym entries lower as `Abs32Be` SymRef fixups
  at offsets 0/12, the `0` entries as plain zero cells with NO fixup; linked bytes: A's
  addr, 0, 0, B's addr, 0. If literal-int elements in a `*u8` array are REJECTED today,
  that's the gap — close it minimally (an int element folds to an absolute cell) and pin
  both accept and reject (a non-zero int should presumably also work — pin whatever
  behavior you implement; a `0` null is the only production need).
- [ ] **GREEN + clippy** — `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
- [ ] **Commit** — `test(as+emp): T3 capability probes — phase-dw compound deferral (win-tab shape), zero-byte embed, null ptr-array entries`

### Task 2: aeon prep — generator change, equ moves, `.bin`s, the gate

**Files (all in `~/sonic_hacks/aeon`, branch `sigil-emp-sfx` off its master):**
- Modify: `tools/sfx_transcode.py`, `games/sonic4/config/sound_ids.asm`,
  `games/sonic4/data/sound/sfx/sfx_table.asm`, `games/sonic4/main.asm`, `.gitignore`
- Add: the 18 `games/sonic4/data/sound/sfx/*.bin`

⚠ If the bg-restore working tree is STILL uncommitted (check first — Volence may have
boot-checked + committed by now): branch from master carrying it along, touch NONE of the
bg files, commit ONLY the files listed here (T2 Task-4's exact discipline).

- [ ] **Step 1 (R2):** Move `SFX_ID_BASE = $33`, `SFX_COUNT = 9`, `SFX_TABLE_LEN = 135`
  from `sfx_table.asm` into `config/sound_ids.asm` right after the `SFXID_*` ladder (:37-45
  today), with a comment noting the `.emp` mirrors. Add to `main.asm`, on the line after
  `SND_ENGINE_TABLE_BANK = MovingTrucks_Bank_Start >> 15`:

```asm
; The SFX blobs share the engine-table bank (asserted by the SFX co-residency
; guard below / sfx_bank.emp's ensure) — declare the contract directly rather
; than deriving from Sfx_33, whose label is .emp-side under SIGIL_EMP_SFX.
SFX_BLOB_BANK = SND_ENGINE_TABLE_BANK
```

  Delete `SFX_BLOB_BANK = sfx_bankid(Sfx_33)` and the three moved equs from
  `sfx_table.asm`. Update `config/game.asm`'s :15-24 contract comment (it currently points
  at the generated table as the declaration home).
- [ ] **Step 2: byte-verify R2.** `./build.sh sonic4` → `sha256sum s4.bin` ==
  `8ce6dd7e30553b8525ddda19ebe3365cc5d24cc62dccfb9c0e6a227d70bc25ef`;
  `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4` == `13c7b06355b658ee…` (then rebuild
  plain, per PROVENANCE). ⚠ prebuild still regenerates `sfx_table.asm` until Step 3 — do
  Step 1's table edits and Step 3's generator change in whatever order keeps the build from
  clobbering them, verifying after BOTH land (implementer's sequencing call; record it).
- [ ] **Step 3 (R1):** In `tools/sfx_transcode.py`'s `generate_all()` (:1733), stop calling
  `emit_sfx_table_asm()` on the default `generate` path; keep it reachable via an explicit
  opt-in (`generate --emit-table` or a separate subcommand). Rewrite `sfx_table.asm`'s
  header: drop GENERATED/DO-NOT-EDIT, add the hand-owned banner + the add-an-SFX checklist
  (R1's four-place list). Re-run `./build.sh sonic4` and verify `sfx_table.asm` is NOT
  rewritten (mtime/content) and both shapes' hashes still match.
- [ ] **Step 4 (R8):** Run `python3 tools/verify_emit_bin.py` — expect 18/18 SFX pairs pass
  (plus the 6 fixed targets). Add 18 narrow `!games/sonic4/data/sound/sfx/<name>.bin`
  exceptions to `.gitignore` (below T2's six, same comment style) and `git add` the 18
  `.bin`s (two are zero-byte — fine).
- [ ] **Step 5 (R6):** Wrap main.asm:228-262 (the 19 includes + both fatal blocks):

```asm
    ifndef SIGIL_EMP_SFX
        include "games/sonic4/data/sound/sfx/sfx_33.asm"
        …                       ; (existing lines through sfx_table.asm + both fatals)
    else
        ; sigil mixed build: everything from Sfx_33 through SfxTable_End comes
        ; from sfx_bank.emp, pinned by the sigil map (region `sfx_bank`).
        ; Resume placement at the per-shape reference address
        ; (see sigil-harness golden/PROVENANCE.md; re-pin on re-baseline).
      ifdef __DEBUG__
        org     $65C82
      else
        org     $64230
      endif
    endif
```

- [ ] **Step 6: byte-verify the gate-off path** — both shapes' sha256 unchanged (same two
  hashes as Step 2).
- [ ] **Step 7: Commit** — `build: SIGIL_EMP_SFX gate + sfx_table hand-owned (generator regen dropped) + SFX equs to sound_ids/main + 18 SFX .bins committed (inert for asl; byte-verified both shapes)`

### Task 3: `sfx_bank.emp`

**Files:**
- Create: `aeon: games/sonic4/data/sound/sfx/sfx_bank.emp` (same aeon branch)

- [ ] **Step 1: Write the module** — in `mt_bank.emp`'s voice (header citing the `.asm`
  guards each ensure replaces; read the LANDED `mt_bank.emp` for exact syntax — NOT any
  paraphrase). Shape (labels/order are the contract; comments abridged here):

```
// data/sound/sfx/sfx_bank.emp — the SFX block DATA (sound-migration T3).
// Everything from Sfx_33 through SfxTable_End; the win-tab (sfx_blob_win_tab.asm,
// Z80 phase head) STAYS driver-side and reads these labels cross-seam via dw
// deferral. NO shape define: content is identical in both shapes — only the map
// base moves (plain $63AE8 / debug $6553A; see harness PROVENANCE on re-baseline).

module data.sfx_bank

const Sfx33Blob     = embed("sfx_33.bin")           // 58
const Sfx33Patch    = embed("sfx_33_patches.bin")   // 32
const Sfx34Blob     = embed("sfx_34.bin")           // 58
const Sfx34Patch    = embed("sfx_34_patches.bin")   // 32
const Sfx35Blob     = embed("sfx_35.bin")           // 286
const Sfx35Patch    = embed("sfx_35_patches.bin")   // 32
const Sfx36Blob     = embed("sfx_36.bin")           // 46
const Sfx36Patch    = embed("sfx_36_patches.bin")   // 0 (PSG-only)
const Sfx3CBlob     = embed("sfx_3C.bin")           // 267 (odd — pad fires)
const Sfx3CPatch    = embed("sfx_3C_patches.bin")   // 32
const Sfx62Blob     = embed("sfx_62.bin")           // 30
const Sfx62Patch    = embed("sfx_62_patches.bin")   // 0 (PSG-only)
const SfxABBlob     = embed("sfx_AB.bin")           // 184
const SfxABPatch    = embed("sfx_AB_patches.bin")   // 32
const SfxB6Blob     = embed("sfx_B6.bin")           // 71 (odd — pad fires)
const SfxB6Patch    = embed("sfx_B6_patches.bin")   // 32
const SfxB9Blob     = embed("sfx_B9.bin")           // 98
const SfxB9Patch    = embed("sfx_B9_patches.bin")   // 32

// UNCHECKED MIRRORS of config/sound_ids.asm (R2) — re-pin together.
const SFX_ID_BASE   = $33
const SFX_COUNT     = 9
const SFX_TABLE_LEN = 135   // max_id - min_id + 1, sparse over $33..$B9

section sfx_bank (cpu: m68000, bank: $8000) {
    data Sfx_33         = Sfx33Blob
    data _p33  = if Sfx33Blob.len % 2 == 1 { byte(0) } else { Data.empty }
    data Sfx_33_Patches = Sfx33Patch
    data _q33  = if Sfx33Patch.len % 2 == 1 { byte(0) } else { Data.empty }
    …   // same 4-line group for 34, 35, 36, 3C, 62, AB, B6, B9 — include order,
        // every item followed by its self-adjusting align-2 pad (the .asm files
        // each end `align 2`; only Sfx_3C/Sfx_B6 fire today)
    data Sfx_B9_Patches = SfxB9Patch
    data _qB9  = if SfxB9Patch.len % 2 == 1 { byte(0) } else { Data.empty }

    // Sparse id->blob table, [SFX_ID_BASE..$B9], 0 = unused id. Transcribed
    // ONCE from the (now hand-owned) sfx_table.asm — 9 syms + zero-runs of
    // 5 ($37-$3B), 37 ($3D-$61), 72 ($63-$AA), 10 ($AC-$B5), 2 ($B7-$B8).
    data SfxTable: [*u8; SFX_TABLE_LEN] = [
        "Sfx_33", "Sfx_34", "Sfx_35", "Sfx_36",
        0, 0, 0, 0, 0,                       // $37-$3B
        "Sfx_3C",
        0, 0, … /* 37 zeros */ …,            // $3D-$61
        "Sfx_62",
        0, 0, … /* 72 zeros */ …,            // $63-$AA
        "Sfx_AB",
        0, … /* 10 zeros */ …,               // $AC-$B5
        "Sfx_B6",
        0, 0,                                // $B7-$B8
        "Sfx_B9",
    ]
}

// The .asm :260 fatal, verbatim message. bankid-label idiom per T2 Deviation 2
// (bare cross-seam equ reads don't exist; MovingTrucks_Bank_Start is bank-aligned
// so bankid() folds to SND_ENGINE_TABLE_BANK's exact value).
ensure(bankid("Sfx_33") == bankid("MovingTrucks_Bank_Start"),
    "SFX blobs not co-located with the engine-table bank — Sfx_Frame's dispatch/table reads would see the wrong bank")
```

  The literal zero-runs must be written out in the real file (135 elements exactly — the
  type errors loud on a miscount, which is the point). The `.asm` :252 straddle fatal has
  NO ensure twin — the section's `bank: $8000` property subsumes it (R5); say so in the
  header comment. Naming/spelling fine-print (pad item names, `byte(0)`, `Data.empty`,
  string-keyed ptr elements): copy the LANDED `mt_bank.emp`, not this sketch.
- [ ] **Step 2: standalone compile check** — `sigil parse` clean; `sigil emp sfx_bank.emp`
  (no defines) expected to fail ONLY at the cross-seam ensure (both operands external —
  the known misleading-but-loud "internal: … anchor label" diagnostic, T2 carry-forward
  #5). Strip-the-ensure scratch build (NOT committed): expect **built: 1864 bytes** — must
  equal `$748` exactly (the pad arithmetic proof; if it's 1862, the pads didn't fire).
- [ ] **Step 3: Commit** — `feat(port): sfx_bank.emp — 9 SFX blobs + patch banks + hand-owned SfxTable (sound-migration T3)`

### Task 4: `sfx_port.rs` — the region-level byte gate

**Files:**
- Create: `crates/sigil-cli/tests/sfx_port.rs` (mt_port.rs as the template — read it first;
  the ONE structural difference is the per-shape map base)

- [ ] **Step 1:** For EACH shape: lower `aeon/games/sonic4/data/sound/sfx/sfx_bank.emp`
  (include_root = its dir, NO defines) → place with `map_toml(debug)`: region `sfx_bank` @
  `0x63AE8` size `0x4518` (plain) / `0x6553A` size `0x2AC6` (debug), + the `text` carrier
  region (mt_port's) → inject the ONE synthetic cross-seam symbol
  (`MovingTrucks_Bank_Start` @ `0x60000`, the mt_port `phase`-label technique verbatim) →
  link + explicit `check_link_asserts` (pin `link_asserts.len() == 1` — the I1 lesson:
  positive gates must not be vacuous-on-empty) → assert section bytes ==
  `s4.bin[0x63AE8..0x64230]` / `s4.debug.bin[0x6553A..0x65C82]` (1864 bytes each).
  `SIGIL_STRICT_GATE` env-gating exactly as mt_port.
- [ ] **Step 2:** RED expected only if Task 3 mis-transcribed; if instant-green, falsify
  (XOR one expected byte, capture the loud diff, revert — T2 Task-6 pattern). Divergences:
  itemize per DSM.9 or zero.
- [ ] **Step 3: Commit** — `test(port): sfx_port — SFX block .emp region byte-identical to reference, both shapes`

### Task 5: the mixed full-ROM gate (DAC+MT+SFX)

**Files:**
- Modify: `crates/sigil-harness/src/lib.rs` — `assemble_mixed_sfx_as_side` SIBLING (T2's
  helper-sibling precedent): `SOUND_DRIVER_ENABLED` + `SIGIL_EMP_DAC` + `SIGIL_EMP_MT` +
  `SIGIL_EMP_SFX` (+ `__DEBUG__` when debug); doc-comment in the house voice.
- Modify: `crates/sigil-harness/tests/mixed_dac_rom.rs` — +2 tests; extend
  `emp_bank_map_with_mt()` with a per-shape 5th region (`sfx_bank`, R7 — note the map
  string becomes a `fn of debug` where it was a const; keep the MT/DAC regions verbatim),
  `placed_module_sections` reused for the third module (defines-less), ONE
  `resolve_layout`+`link` over AS + all THREE `.emp` modules' sections;
  `check_link_asserts` for BOTH mt (5 asserts) and sfx (1 assert) modules — pin both counts.

- [ ] **Step 1:** `mixed_sfx_rom_matches_assembled_reference` +
  `mixed_sfx_debug_rom_matches_assembled_reference` — `assert_rom_matches` vs the same
  `ASSEMBLED_LEN`/allowlists (content identical ⇒ same pins). T1's 2 and T2's 2 existing
  tests stay untouched.
- [ ] **Step 2:** This is where the win-tab dw deferral proves out end-to-end: the `.asm`
  side's `SfxBlobWinTab` entries assemble with `Sfx_NN` unresolved (P1's deferral) and
  resolve through the joint link. Byte-inspect at least the first entry against the
  reference (T2's movea-imm32 xxd evidence pattern): find `SfxBlobWinTab` in `s4.lst`,
  compute `sfx_winptr($63AE8)` = `($63AE8 & $7FFF) | $8000` = `$BAE8` → LE bytes `E8 BA`,
  and confirm both the reference and the assertion window cover it. Record in notes.
- [ ] **Step 3:** Full nets: `SIGIL_STRICT_GATE=1 AEON_DIR=… cargo test -p sigil-harness`
  (ALL prior gates + the new pair) + `cargo test --workspace` + clippy `-D warnings`.
- [ ] **Step 4: Commit** — `test(harness): mixed DAC+MT+SFX full-ROM byte-identical, both shapes (sound-migration T3 acceptance)`

### Task 6: negative probes

**Files:**
- Create: `crates/sigil-cli/tests/sfx_negative_probes.rs` (mt_negative_probes.rs sibling)

- [ ] (a) **straddle**: doctored map base `0x67C00` (the 1864-byte section then ends at
  `0x68348`, crossing `$68000`) → `resolve_layout` errors naming `sfx_bank` + "straddle".
- [ ] (b) **wrong bank**: synthetic `MovingTrucks_Bank_Start` @ `0x58000` → the ONE
  co-residency ensure fires with its message; pin `len() == 1` and the message substring.
- [ ] (c) **table length mismatch**: doctored inline module (134 elements vs
  `SFX_TABLE_LEN = 135`) → clean "array length mismatch", no panic.
- [ ] (d) **null-entry regression**: pin that a doctored table whose `0` entries were
  replaced by a WRONG sym (`"Sfx_34"` where `0` belongs) still lowers (it's legal!) but
  produces DIFFERENT bytes than the reference — i.e. demonstrate the byte-gate is what
  catches transcription errors, per its falsification, OR skip with a written rationale if
  redundant with Task 4's falsification (T2 probe-(e) precedent — judge and record).
- [ ] Falsify at least (a) and (b) (restore-real-value pattern), GREEN, commit —
  `test: T3 negative probes — straddle/wrong-bank/table-len loud`

### Task 7: docs + review + checkpoint

- [ ] Update this plan's `## Execution notes` as tasks land — not at the end.
- [ ] Two-pronged whole-branch adversarial review: prong 1 = the sigil diff (P1/P3's new
  deferral/element paths — hunt byte-drift on resolved paths + diagnostic regressions);
  prong 2 = the aeon diff + `.emp` fidelity (the 135-entry transcription against the
  reference table entry-by-entry; the generator change's default-path behavior; every
  fatal message preserved; R2's comment updates).
- [ ] Completion handoff note in `docs/superpowers/notes/`, memory update
  (`spec2-progress`), checkpoint summary for Volence. **NO merge without the checkpoint**
  (sigil `sound-migration-t3`, aeon `sigil-emp-sfx`).
- [ ] Checkpoint asks: merge decision; the bg boot-check if STILL pending; note the arc is
  DONE after this merge → next: re-evaluate S2-D14(a)(d)(e) + 9d against what the arc
  demanded, then Plan-7 #10 (compression builtins), then spec FREEZE.

---

## Self-review (spec coverage)

- DSM.8 T3 scope (blob embeds, sfx_table, the SFX straddle/co-residency ensures, the
  win-tab call per DSM.7): Tasks 1-5 ✓; win-tab pre-ruling exercised via R3/P1 ✓.
- DSM.7 (LE u16 cells / win-tab): the win-tab STAYS `.asm` (confirmed inseparable —
  soundBankHead arg), so the LE cell surface is the `.asm`-side dw deferral (P1), not an
  `.emp` u16le cell — consistent with DSM.7's "stays `.asm` this arc" arm ✓.
- DSM.9 bar: region byte-diff (Task 4), full-ROM both shapes (Task 5), workspace fully
  green (Task 5 Step 3), divergences itemized (stop rules) ✓.
- Handoff wrinkles: win-tab probe-first ✓ (P1); bankid idiom for the co-residency ensure ✓
  (R5); per-shape org resume derived from the CURRENT .lst ✓ (fact base — `$64230`/`$65C82`,
  which are `SfxTable_End`, not the handoff's stale blobs-only end addresses);
  `[*u8; N]` table shape ✓ (P3 covers the sparse-null delta from SongTable); empty patch
  banks ✓ (P2 covers embed-of-zero-byte-file, distinct from T2's `Data.empty` proof);
  18 `.bin` gitignore exceptions ✓ (R8).
- The design-check ruling (hand-owned table) is fully absorbed: generator change + header
  rewrite + hand-owned `.emp` twin + mirror-pin ensures (R1/R2/R5, Tasks 2-3) ✓.
- No task references an undefined mechanism: every construct is either proven (T0-T2) or
  probed in Task 1 before use ✓.

## Execution notes

### Task 1 (capability probes) — DONE (commits `c168a61` + `747f515`)

- **P1 — INSTANT-GREEN, the tranche's go/no-go cleared:** T0's dw deferral generalizes to
  the Z80 `phase 08000h` vma≠lma context unchanged. New `phase_dw_winptr_defer.rs` (4
  tests): compound-tree `Value16Le` fixup + 2-byte hole pinned; REAL `resolve_layout`+`link`
  resolution pinned for both the synthetic mask (`$7F00` → `$BA00` → LE `00 BA`) and the
  exact production shape (`(Sfx_33 & 32767) | 32768` @ `$63AE8` → `$BAE8` → LE `E8 BA` =
  SfxBlobWinTab[0]); negative control pins the loud unresolved-fixup link error. All four
  falsified (corrupt-expected → real computed value in the failure → revert). NOTE for
  Task 5: the loud error for an unresolvable COMPOUND fixup names the fixup SITE
  ("unresolved target expression for fixup in section … at offset …"), not the symbol —
  same class as T2 carry-forward #5.
- **P2 — instant-green:** zero-byte `embed()` emits nothing, label defined, next item at
  same offset (`tests/vectors/empty.bin`, 0 bytes, tracked). Falsified via corrupted
  next-offset expectation.
- **P3 — REAL RED, gap closed minimally:** int elements in `[*u8; N]` arrays were rejected
  (`"pointer field needs a symbol reference, got int"`). Fix: new Int arm in `lower_ptr`
  (`eval/emit.rs`) folds to a width-4 BE absolute `Cell::Scalar`, no fixup. **Spec review
  caught silent truncation** (`$100000000` emitted `00 00 00 00` = indistinguishable from a
  null, zero diagnostics — totality violation); fixed in `747f515`: `emit_range_check(n, 0,
  u32::MAX, …)` mirroring `lower_prim`'s convention (best-effort cell emission + loud
  `[emit.out-of-range]`, build still fails on Level::Error). Pinned: accept `$1234` +
  `$FFFFFFFF` boundary (bytes `FF FF FF FF`), reject `2^32` and `-1` (messages name the
  value).
- Workspace 1476/0, clippy clean. Two-stage review done: spec ✅ (after the truncation
  fix), quality ✅ approve. **Deferred cosmetic minors for the whole-branch polish pass:**
  (1) `p2_`/`p3_` test-name prefixes in lower_data.rs now collide with T2's unrelated
  p2/p3 probes — consider `t3_`-prefixing; (2) `lower_ptr`'s function-level doc doesn't
  mention the new Int-arm behavior; (3) the Int arm's guard + re-destructure is redundant
  (`matches!` then `if let`).
