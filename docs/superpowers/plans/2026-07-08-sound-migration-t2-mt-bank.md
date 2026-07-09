# Sound-migration T2 — the Moving Trucks streaming bank Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the MT streaming bank's DATA (song/pitch/patch streams + SongTable/SongPatchTable + their layout guards) to `.emp`, mixed-build byte-identical to the AS reference in BOTH build shapes (plain + `__DEBUG__`).

**Architecture:** The `.asm` engine-table head (`soundBankHead` Z80 phase blob, `$60000..$60607`) stays; ONE `.emp` module (`mt_bank.emp`) defines everything from `Song_MovingTrucks` (`$60607`) through `SongPatchTable_End`, pinned by a map region. The DEBUG shape is driven by a NEW `-D`-style defines mechanism (value-level `if` only — no item-level conditionals needed). Two small sigil gaps close first: comptime defines, and 32-bit immediate-operand deferral in the AS frontend (`movea.l #SongTable` cross-seam).

**Tech Stack:** Rust (sigil workspace), Motorola 68000 asm (aeon), `.emp` (Spec 2).

**Design authority:** `docs/superpowers/specs/2026-07-08-sound-migration-tranche1-design.md` (DSM.1–9, Volence-approved). This plan's survey verified the design against live files — NO contradictions; the deltas below are frozen as facts.

**Process:** the #7/T0+T1 pattern — TDD w/ recorded RED, commit-per-task, two-stage reviews on load-bearing tasks (2, 4, 5, 7), whole-branch adversarial review, NO merge without a Volence checkpoint.

---

## The verified fact base (survey 2026-07-08, post-bg-restore re-baseline)

All addresses from the CURRENT pins (aeon `e5b256c` + bg-restore working tree; non-debug sha256 `8ce6dd7e…`, debug `13c7b06…` — see sigil-harness `golden/PROVENANCE.md` "Re-baseline: forest-bg restore").

**Layout, BOTH shapes identical until the DEBUG members:**

| symbol | addr | len | source |
|---|---|---|---|
| `MovingTrucks_Bank_Start` (.asm head start) | `$60000` | `$607` | main.asm:137-149 (stays .asm) |
| `Song_MovingTrucks` | `$60607` | `$30B1` (12465) | song_movingtrucks.bin |
| `MovingTrucks_PitchTable_Stream` | `$636B8` | `$108` (264) | movingtrucks_pitchtable_stream.bin |
| `MovingTrucks_Patches` | `$637C0` | `$320` (800) | movingtrucks_patches.bin |
| `MovingTrucks_Patches_End` | `$63AE0` | — | (base + lens; all contiguous, no pads) |

**Plain shape tail:** `SongTable` `$63AE0` (1 entry, 4 B), `SongPatchTable` `$63AE4` (4 B), end/`Sfx_33` resume = **`$63AE8`**.

**Debug shape tail:** `Song_DrumTest` `$63AE0` (`$52`), `Song_HCZ2` `$63B32` (`$1970`), `HCZ2_Patches` `$654A2` (`$80`), `SongTable` `$65522` (3 entries, 12 B), `SongPatchTable` `$6552E` (12 B), end/`Sfx_33` resume = **`$6553A`**.

Both tails are even, so song_table.asm's trailing `align 2` (line 137) is a no-op at these pins — the `.emp` port carries no pad and the org resume addresses above are exact.

**Deltas vs the design prose (frozen facts, not contradictions):**
- Guards are SEVEN, not six: the design's list + the window-top guard (song_table.asm:99) + the MT pitch-contiguity fatal (main.asm:164, `MT_PITCHTAB_OFFSET equ $30B1` defined at song_movingtrucks.asm:798).
- `MovingTrucks_Patches` is 25×32 = 800 B (design prose said "33×26=858" — stale; no design impact).
- The `--emit-bin` `.bin`s exist (T1) but are GITIGNORED, and the song emitters need EXTERNAL inputs (the Zyrinx B&R ROM, skdisasm) so they can NOT run in prebuild — T2 **commits the `.bin`s** (mirrors today's committed generated `.asm`). The generated `.asm` files STAY (the plain asl build still uses them; deletion is cutover-era) — T1's `dac_samples.asm` precedent.

**Cross-seam consumer surface (exhaustive grep, engine/ + games/ minus data/sound):**
- `engine/sound/sound_api.asm:83` `movea.l #SongTable, a0` and `:99` `movea.l #SongPatchTable, a0` — 68k **code** operands → needs Task 3's deferral (T0 only deferred db/dw).
- `games/sonic4/main.asm:164-166` — the pitch-contiguity fatal; ports INTO the `.emp` (gated out with the includes).
- `moveq #SONG_MOVINGTRUCKS/#SONG_DRUMTEST/#SONG_HCZ2` in `games/sonic4/config/game.asm:56`, `games/sonic4/debug/game_debug.asm:29,69,83,93` — moveq bakes the imm into the OPCODE word; deferring that is invasive → **ruling R2**: the id constants MOVE to `config/sound_ids.asm` (hand-written contract file, included at main.asm:12 before all consumers), so no cross-seam need exists.
- `SONG_COUNT` has NO code consumers outside song_table.asm (comment mention only) → stays internal to the `.emp`.
- `.emp` → `.asm` reads: `MovingTrucks_Bank_Start`, `SND_ENGINE_TABLE_BANK` (main.asm:138/143) — T0 Probe B direction, proven.

## Frozen rulings

- **R1 — defines.** `LowerOptions` gains `pub defines: Vec<(String, i128)>` (default empty), injected as comptime int consts into the module global scope BEFORE item eval; a module-declared item with the same name is a hard error `[defines.collision]`. CLI: `sigil emp -D NAME=INT` (repeatable; `$`/`0x` int forms accepted). The MT module reads `DEBUG` (0 or 1). Mirrors AS `-D __DEBUG__` — the seam-level shape input, not a language-surface change.
- **R2 — song ids move to `config/sound_ids.asm`.** `SONG_MOVINGTRUCKS=1`, and under `ifdef __DEBUG__` `SONG_DRUMTEST=2` / `SONG_HCZ2=3`, REMOVED from song_table.asm (else the un-gated asl build defines them twice). Byte-neutral for asl (equates emit nothing) — verified in Task 5.
- **R3 — AS imm32 deferral.** `sigil-frontend-as`: a 32-bit immediate operand (`#expr` where the instruction takes a long immediate — the `movea.l`/`move.l` class) whose expr is unresolved on the CONVERGED pass emits a 4-byte hole + value fixup (the `dc.l` deferral's kind) instead of the `unresolved symbol` error. Resolved operands take the existing eager path — byte-neutral, proven by the m1d gates staying green.
- **R4 — one `.emp` module, value-level conditionals only.** DEBUG members exist in BOTH shapes as data items; in plain shape their value is `Data.empty` (zero bytes — label defined, harmless, and `bankid()` of it still folds to the MT bank so the co-residency ensures stay UNCONDITIONAL). Tables and `SONG_COUNT` are if-expressions.
- **R5 — guard mapping.** Straddle (song_table:91,110,128) + window-top (:99) → the section's `bank: $8000` no-straddle property (region `$60607..end` inside `$60000..$68000`; the head's containment is trivially bank-0-relative). Engine-table-head co-residency → per-member `ensure(bankid("X") == SND_ENGINE_TABLE_BANK, …)` cross-seam. Pitch contiguity (main.asm:164) → contiguous BY CONSTRUCTION (§4.3 no-auto-pad) + `ensure(SongBlob.len == MT_PITCHTAB_OFFSET)` with the pinned const `$30B1` (same manual re-pin on regen as today's generator-emitted equ). Table-length + `< $FF` checks → comptime ensures.
- **R6 — the gate.** `ifndef SIGIL_EMP_MT` wraps main.asm lines 150–208 (the six includes + the pitch fatal); `else` org-resumes for the SFX block: `ifdef __DEBUG__` → `org $6553A`, else `org $63AE8` (hard pins w/ PROVENANCE comment, T1's `org $60000` precedent). Independent of `SIGIL_EMP_DAC`; harness exercises DAC-only (T1 tests, unchanged) and DAC+MT (new). MT-only is untested — noted, not exercised.
- **R7 — map region** `mt_bank` at `lma_base = 0x60607`, `size = 0x79F9` (to `$68000`), placed BY NAME; `bank: $8000` on the section. Each `.emp` module still emits its zero-byte `text` equ carrier — TWO of them now (dac + mt), both at LMA 0; Task 6 probes that the link tolerates the pair (both zero-byte; if the name pair collides, give the harness map two nominal regions / stagger LMA — implementer's call, recorded in execution notes).

## File structure

- `crates/sigil-frontend-emp/src/lower/mod.rs` — `LowerOptions.defines` + injection + collision check.
- `crates/sigil-cli/src/main.rs` — `-D` flag on `sigil emp`.
- `crates/sigil-frontend-as/src/eval.rs` — imm32 operand deferral.
- `aeon: games/sonic4/config/sound_ids.asm` — song ids (R2).
- `aeon: games/sonic4/data/sound/mt_bank.emp` — the port (full content in Task 5).
- `aeon: games/sonic4/main.asm` — the `SIGIL_EMP_MT` gate; `data/sound/song_table.asm` — ids removed.
- `aeon: .gitignore` + the six committed `.bin`s.
- `crates/sigil-cli/tests/mt_port.rs` — region-level byte gate (dac_port.rs sibling).
- `crates/sigil-harness/tests/mixed_dac_rom.rs` — +2 tests (DAC+MT, both shapes).

---

### Task 1: comptime defines (sigil-frontend-emp + CLI)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (LowerOptions at :33)
- Modify: `crates/sigil-cli/src/main.rs` (the `emp` subcommand)
- Test: `crates/sigil-frontend-emp/src/lower/mod.rs` (unit tests in-file, per crate convention — check the existing `#[cfg(test)]` home first and follow it)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn defines_are_visible_as_comptime_consts() {
    let src = r#"
module t
const N = if DEBUG == 1 { 3 } else { 1 }
data Tbl: [u8; N] = if DEBUG == 1 { [1, 2, 3] } else { [7] }
"#;
    let ast = parse_str(src).unwrap();
    let out = lower_module(&ast, &LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        defines: vec![("DEBUG".into(), 1)],
    });
    // no diagnostics; the emitted data section holds 3 bytes [1,2,3]
    assert!(out.diags.iter().all(|d| d.level != Level::Error), "{:?}", out.diags);
    // locate the emitted bytes via the module's sections (mirror how existing
    // lower tests fish out data bytes) and assert [1,2,3]; re-lower with
    // ("DEBUG", 0) and assert [7].
}

#[test]
fn define_colliding_with_module_decl_errors() {
    let src = "module t\nconst DEBUG = 0\n";
    let ast = parse_str(src).unwrap();
    let out = lower_module(&ast, &LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        defines: vec![("DEBUG".into(), 1)],
    });
    assert!(out.diags.iter().any(|d| d.message.contains("defines.collision")));
}
```

Adapt the assertion plumbing to the crate's existing lower-test helpers (grep `lower_module(` in existing tests and copy the section-bytes extraction).

- [ ] **Step 2: Run to verify RED** — `cargo test -p sigil-frontend-emp defines_` — expect compile failure (`defines` field missing). Record the RED in the commit message.

- [ ] **Step 3: Implement.** Add `pub defines: Vec<(String, i128)>` to `LowerOptions` (update every existing construction site — `..Default::default()` or add the field; grep `LowerOptions {`). In `lower_module`, before evaluating items: for each `(name, v)` bind `Value::Int(v)` as a global const; if the module declares an item with the same name, push `Error` `[defines.collision] '{name}' is provided by -D and declared by the module`. Injection point: wherever the evaluator's global scope is seeded (follow how top-level consts register — the lazy-const table; a pre-seeded resolved entry is enough).

- [ ] **Step 4: GREEN** — `cargo test -p sigil-frontend-emp` all pass.

- [ ] **Step 5: CLI.** Add `-D NAME=INT` (repeatable) to `sigil emp`, parsed with the same int-literal forms the lexer accepts (`$`, `0x`, decimal; reuse/extract the CLI's existing int parsing if present, else a small helper). Wire into `LowerOptions.defines`. Add one subprocess test beside the existing `sigil emp` CLI test (`crates/sigil-cli/tests/` — follow the include-root subprocess test pattern): a two-line `.emp` using `DEBUG`, run with `-D DEBUG=1`, assert output bytes.

- [ ] **Step 6: GREEN + clippy** — `cargo test -p sigil-cli && cargo clippy --workspace -- -D warnings`.

- [ ] **Step 7: Commit** — `feat(emp): comptime defines (-D) — LowerOptions.defines + CLI; [defines.collision] guard`

### Task 2: `.emp` capability probes for the MT shapes

Prove (as committed tests, sigil-side, no aeon dependency) every construct `mt_bank.emp` uses that no existing test exercises. Each probe is a small `lower_module` test in `crates/sigil-frontend-emp` (same home as Task 1's tests).

**Probes (write all, RED/GREEN each):**

- [x] **P1:** `data X = if C { embed(...) } else { Data.empty }` — zero-length data item in the else arm: label defined, zero bytes emitted, following item lands at the same offset. (Use a tiny fixture bin via the include_root tempdir pattern from the sandbox tests.) — instant-green, see Execution notes.
- [x] **P2:** `data T: [*u8; 3] = ["A", "B", "C"]` — array-of-Ptr with string elements → three `Cell::SymRef` Abs32 cells (12 B). And the 1-element variant. — instant-green.
- [x] **P3:** `const N = if D == 1 { 3 } else { 1 }` driving `[*u8; N]` with an if-expression RHS of matching length — and the MISMATCHED length (2 elems vs N=3) is a clean error, not a panic. — instant-green.
- [x] **P4:** an `ensure` mixing a comptime `.len` and a pinned const (`ensure(Blob.len == $30B1, "...")`) — fires loud on mismatch. — instant-green.
- [x] **P5 (link-level, in `crates/sigil-cli/tests/` beside dac_port.rs helpers):** TWO separately-lowered modules, each with top-level `equ`s (⇒ two zero-byte `text` carriers), sections placed by one map + linked together — link succeeds, both equ sets resolve. If the paired carrier trips a dup-name/overlap diagnostic, fix per R7 (harness-side naming/stagger, NOT a linker change) and record the delta. — instant-green, no diagnostic fired (R7 contingency moot); see Execution notes.

- [x] **Steps per probe:** write test → RED (or instant green if the capability exists — then it's a pinned regression test, still committed) → minimal fix if RED (expected only for gaps; keep fixes additive and small — if a probe reveals a MISSING mechanism bigger than ~a day, STOP and surface to the orchestrator rather than improvising). — all five instant-green, no fixes needed.
- [x] **GREEN + clippy**, then **Commit** — `test(emp): MT-shape capability probes (cond-embed/ptr-arrays/if-arrays/len-ensure/dual-carrier)`

### Task 3: AS-frontend 32-bit immediate deferral

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs` (the m68k immediate-operand encoding path; find via the `movea`/`move` long-imm encoding, near the abs-operand machinery ~:1841-2100 and the poison bookkeeping at :55-86)
- Test: same crate, beside the T0 db/dw-deferral tests (grep `directive_dw` deferral tests and mirror their structure)

- [x] **Step 1: Failing test** — assemble a unit containing `movea.l #ExternalSym, a0` where `ExternalSym` is NOT defined in-unit; expect (today) the `unresolved symbol in operand` error → the test asserts instead: module assembles, the instruction's fragment carries a 4-byte hole at the immediate's offset + a value fixup targeting `Expr::Sym("ExternalSym")` with the same `FixupKind` the `dc.l` deferral uses (grep `dc.l`/width-4 deferral for the exact kind name). Also a companion test: `move.l #ExternalSym, d0` (same class), and a NEGATIVE control: an unresolved symbol in a **non-deferrable** position (e.g. a `moveq #Sym, d0` imm8 or a branch target) still errors — the deferral is scoped to long immediates only. — see Execution notes for the `Value32Be` vs `Abs32Be` kind deviation (reasoned, not the literal `dc.l` kind).
- [x] **Step 2: RED** — `cargo test -p sigil-frontend-as --test imm32_defer` — 3 positive tests failed with today's hard error, 3 negative controls already passed.
- [x] **Step 3: Implement.** `Asm::try_defer_long_imm` in `lower_m68k_generic`; emits the 4-byte placeholder + `Value32Be` fixup for an unresolved `movea.l #expr,aN` / `move.l #expr,dN`, falls through to the untouched eager path otherwise. See Execution notes for the destination-shape scoping rationale (bare `aN`/`dN` only — the caution's stop-rule).
- [x] **Step 4: GREEN**, then the byte-neutrality net: `SIGIL_STRICT_GATE=1 AEON_DIR=~/sonic_hacks/aeon cargo test -p sigil-harness` — ALL reference gates still green (no resolved-path drift).
- [x] **Step 5: Commit** — `feat(as): defer unresolved 32-bit immediates to link fixups (movea.l/move.l #Sym cross-seam; imm8/branches still loud)`

### Task 4: aeon prep — ids move, `.bin`s committed, the gate

**Files (all in `~/sonic_hacks/aeon` — SEPARATE repo, branch `sigil-emp-mt` off its master):**
- Modify: `games/sonic4/config/sound_ids.asm`, `games/sonic4/data/sound/song_table.asm` (ids move, R2)
- Modify: `.gitignore` (+ 6 un-ignores), add the 6 `.bin`s
- Modify: `games/sonic4/main.asm` (the R6 gate)

⚠ aeon has the UNCOMMITTED bg-restore working tree awaiting Volence's boot-check. Branch from master and carry the working-tree changes along (they're in data/tools files disjoint from the sound files); do NOT commit the bg files on this branch — commit ONLY the sound-arc files listed here. If the boot-check lands first, rebase trivially.

- [x] **Step 1:** Move the three `SONG_*` id equates (song_table.asm:17,24,32 with their comment blocks, keeping the `ifdef __DEBUG__` around ids 2-3) into `config/sound_ids.asm`; delete them from song_table.asm (leave `SONG_COUNT` where it is).
- [x] **Step 2:** Verify byte-neutrality: `./build.sh sonic4` then `sha256sum s4.bin` == `8ce6dd7e30553b8525ddda19ebe3365cc5d24cc62dccfb9c0e6a227d70bc25ef`; `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4` == `13c7b063…` (then rebuild plain to restore, per PROVENANCE).
- [x] **Step 3:** `.gitignore` exceptions + `git add` for: `song_movingtrucks.bin`, `movingtrucks_pitchtable_stream.bin`, `movingtrucks_patches.bin`, `song_drumtest.bin`, `song_hcz2.bin`, `hcz2_patches.bin` (all under `games/sonic4/data/sound/`). First verify each still byte-matches its `dc.b` twin: the T1 verifier tool (`tools/` — the dc.b-equality checker from commit `4782cde`) — run it; if any is stale, regenerate via the emitter's `--emit-bin` and note it.
- [x] **Step 4:** The gate in `games/sonic4/main.asm`: wrap lines 150–208 (from `include …song_movingtrucks.asm` through `include …song_table.asm`, INCLUDING the :164-166 pitch fatal):

```asm
    ifndef SIGIL_EMP_MT
        include "games/sonic4/data/sound/song_movingtrucks.asm"
        …                       ; (existing lines 151–207 unchanged)
        include "games/sonic4/data/sound/song_table.asm"
    else
        ; sigil mixed build: everything from Song_MovingTrucks ($60607) through
        ; SongPatchTable_End comes from mt_bank.emp, pinned by the sigil map.
        ; Resume placement for the SFX block at the per-shape reference address
        ; (see sigil-harness golden/PROVENANCE.md; re-pin on re-baseline).
      ifdef __DEBUG__
        org     $6553A
      else
        org     $63AE8
      endif
    endif
```

- [x] **Step 5:** Byte-neutrality again (gate OFF path): both shapes' sha256 unchanged (same two hashes as Step 2).
- [x] **Step 6:** Commit on `sigil-emp-mt` — `build: SIGIL_EMP_MT gate + song-id equates to sound_ids.asm + MT/HCZ2 stream .bins committed (inert for asl; byte-verified both shapes)`

### Task 5: `mt_bank.emp`

**Files:**
- Create: `aeon: games/sonic4/data/sound/mt_bank.emp` (same aeon branch)

- [x] **Step 1:** Write the module. Full content (header comments abridged here to the load-bearing ones — write them in the dac_samples.emp voice, citing the .asm guards each ensure replaces):

```
// data/sound/mt_bank.emp — the MT streaming-bank DATA (sound-migration T2).
// The .asm engine-table head (soundBankHead, $60000..$60607) STAYS driver-side;
// this module is everything after it in the same 32KB bank. Shape input: -D DEBUG=0|1
// (mirrors AS -D __DEBUG__). Pinned at $60607 by the map region `mt_bank`.
// Re-pin via the harness PROVENANCE flow on any re-baseline.

module data.mt_bank

// --- the stream blobs (generated .bin twins of the committed dc.b .asm; see
// tools' --emit-bin + the dc.b-equality verifier) ---
const SongBlob      = embed("song_movingtrucks.bin")              // $30B1
const PitchBlob     = embed("movingtrucks_pitchtable_stream.bin") // $108
const PatchesBlob   = embed("movingtrucks_patches.bin")           // $320
const DrumTestBlob  = embed("song_drumtest.bin")                  // $52  (DEBUG member)
const Hcz2Blob      = embed("song_hcz2.bin")                      // $1970 (DEBUG member)
const Hcz2PatchBlob = embed("hcz2_patches.bin")                   // $80  (DEBUG member)

// song ids (the .asm twins live in config/sound_ids.asm — R2; table order below
// must keep entry[id-1] = that song, same coupling the .asm table had)
const SONG_MOVINGTRUCKS = 1
const SONG_DRUMTEST     = 2   // DEBUG-only id in .asm; harmless const here
const SONG_HCZ2         = 3
const SONG_COUNT = if DEBUG == 1 { 3 } else { 1 }
ensure(SONG_COUNT < $FF, "SONG_COUNT must be < $FF ($FF is the stop sentinel)")

// the generator-baked header offset (song_movingtrucks header pitchtable_ptr =
// packed song length). The .asm twin: MT_PITCHTAB_OFFSET equ $30B1
// (song_movingtrucks.asm:798). Re-pin BOTH on song regeneration; this ensure is
// the detune guard (main.asm's old :164 fatal) — a mismatch means the header
// disagrees with the layout below.
const MT_PITCHTAB_OFFSET = $30B1
ensure(SongBlob.len == MT_PITCHTAB_OFFSET,
    "MT pitch table not contiguous with the song: song len != header's pitchtable_ptr — a pad byte would detune the whole song")

// ONE bank: $8000 section = the .asm's align-$8000 + all four straddle/window-top
// fatals (song_table.asm:91/99/110/128). Items emit contiguously (§4.3 no-auto-pad)
// = the .asm's contiguity comments made structural.
section mt_bank (cpu: m68000, bank: $8000) {
    data Song_MovingTrucks             = SongBlob
    data MovingTrucks_PitchTable_Stream = PitchBlob
    data MovingTrucks_Patches          = PatchesBlob
    data Song_DrumTest = if DEBUG == 1 { DrumTestBlob } else { Data.empty }
    data Song_HCZ2     = if DEBUG == 1 { Hcz2Blob }     else { Data.empty }
    data HCZ2_Patches  = if DEBUG == 1 { Hcz2PatchBlob } else { Data.empty }

    // song id -> SongHeader ptr, indexed SongTable[id-1]; id 0 reserved = stop.
    data SongTable: [*u8; SONG_COUNT] = if DEBUG == 1 {
        ["Song_MovingTrucks", "Song_DrumTest", "Song_HCZ2"]
    } else {
        ["Song_MovingTrucks"]
    }
    // parallel per-song FM-patch-bank ptr (DrumTest reuses MT's bank)
    data SongPatchTable: [*u8; SONG_COUNT] = if DEBUG == 1 {
        ["MovingTrucks_Patches", "MovingTrucks_Patches", "HCZ2_Patches"]
    } else {
        ["MovingTrucks_Patches"]
    }
}

// The engine-table-head rule (the sequencer reads FM pitch/log-volume/opcode
// tables window-relative from the bank head): every stream must share the head's
// bank. SND_ENGINE_TABLE_BANK is .asm-defined (main.asm:143) — cross-seam link
// asserts, UNCONDITIONAL (in the plain shape the DEBUG labels are zero-length
// but still placed in-bank, so bankid() folds identically).
ensure(bankid("Song_MovingTrucks") == SND_ENGINE_TABLE_BANK,
    "Moving Trucks stream not co-located with the engine-table bank — window-relative table reads would see the wrong bank")
ensure(bankid("MovingTrucks_Patches") == SND_ENGINE_TABLE_BANK,
    "MT patch bank not co-located with the engine-table bank")
ensure(bankid("Song_DrumTest") == SND_ENGINE_TABLE_BANK,
    "Song_DrumTest (DEBUG) left Moving Trucks' bank — its stream/table reads need the engine-table head's bank")
ensure(bankid("Song_HCZ2") == SND_ENGINE_TABLE_BANK,
    "Song_HCZ2 (DEBUG) left Moving Trucks' bank — one SetBank must cover its stream + the engine-table head")
ensure(bankid("HCZ2_Patches") == SND_ENGINE_TABLE_BANK,
    "HCZ2 patch bank (DEBUG) left Moving Trucks' bank")
```

Exact `ensure`-with-link-expr spelling and whether `SND_ENGINE_TABLE_BANK` needs quoting: mirror the T0 Probe-B test (grep `MovingTrucks_Bank_Start` in `crates/sigil-cli/tests/ports.rs`); adjust syntax to match what's proven, not this sketch.

- [x] **Step 2:** Standalone compile check, both shapes: `sigil emp games/sonic4/data/sound/mt_bank.emp -D DEBUG=0` (link-ensures referencing `SND_ENGINE_TABLE_BANK` will fail standalone — if the CLI can't inject external symbols, this step just checks PARSE+comptime via `sigil parse`; the real compile happens in Task 6's harness. Record which.)
- [x] **Step 3:** Commit (aeon branch) — `feat(port): mt_bank.emp — the MT streaming bank + song tables (sound-migration T2)`

### Task 6: `mt_port.rs` — the region-level byte gate

**Files:**
- Create: `crates/sigil-cli/tests/mt_port.rs` (dac_port.rs as the template — read it first and mirror its structure/env-gating exactly)

- [ ] **Step 1: Failing test.** For EACH shape (`DEBUG=0`, `DEBUG=1`): lower `aeon/games/sonic4/data/sound/mt_bank.emp` (include_root = its dir, defines = the shape) → place with a map pinning `mt_bank` @ `0x60607` (+ the text carrier region; copy dac_port's map string, swap regions) → inject the two `.asm`-side symbols the ensures read (`MovingTrucks_Bank_Start=0x60000`, `SND_ENGINE_TABLE_BANK=0xC`) the same way the T0 probe injected cross-seam symbols → link → assert the flattened `mt_bank` section bytes == the reference window of the matching ROM (`aeon/s4.bin[0x60607..0x63AE8]` plain / `aeon/s4.debug.bin[0x60607..0x6553A]` debug). Reference-gated + `SIGIL_STRICT_GATE` skip pattern, same as dac_port.
- [ ] **Step 2: RED** (the .emp/map/harness wiring is new), **then GREEN** — divergences itemized per DSM.9's stop rule (expected: none).
- [ ] **Step 3: Commit** — `test(port): mt_port — MT bank .emp region byte-identical to reference, both shapes`

### Task 7: the mixed full-ROM gate (DAC+MT)

**Files:**
- Modify: `crates/sigil-harness/tests/mixed_dac_rom.rs` (+2 tests)
- Modify: `crates/sigil-harness/src/lib.rs` — `assemble_mixed_dac_as_side` grows a defines parameter (or a sibling `assemble_mixed_mt_as_side`) passing BOTH `SIGIL_EMP_DAC` and `SIGIL_EMP_MT`

- [ ] **Step 1: Failing tests** `mixed_mt_rom_matches_assembled_reference` + `mixed_mt_debug_rom_matches_assembled_reference`: assemble the AS side with both gates, lower BOTH `.emp` modules (dac defines-less; mt with `DEBUG` matching the shape), concat sections, one `resolve_layout` + `link`, `assert_rom_matches` vs `ASSEMBLED_LEN`/`DEBUG_ASSEMBLED_LEN` with the same `CONVSYM_REWRITTEN*` allowlists (content identical ⇒ same pins — the module doc-comment reasoning carries over verbatim).
- [ ] **Step 2: RED → GREEN.** This is where the R3 deferral + cross-seam resolution proves out end-to-end (`movea.l #SongTable` resolves to `$63AE0`/`$65522` per shape). Stop rule: any non-allowlisted diff offset fails loud, itemize per DSM.9.
- [ ] **Step 3:** Full nets: `SIGIL_STRICT_GATE=1 AEON_DIR=… cargo test -p sigil-harness` + `cargo test --workspace` + clippy — ALL green (T1's tests keep passing: DAC-only composition unchanged).
- [ ] **Step 4: Commit** — `test(harness): mixed DAC+MT full-ROM byte-identical, both shapes (sound-migration T2 acceptance)`

### Task 8: negative probes (the T1 pattern, commit `9e2bce4`'s sibling)

- [ ] In `mt_port.rs` (or beside it): (a) shrink the map region so the section would straddle `$68000` → the bank no-straddle diagnostic fires; (b) inject `SND_ENGINE_TABLE_BANK=0xB` (wrong bank) → the co-residency ensure fires with its message; (c) `-D DEBUG=1` with a 1-entry table via a doctored source string → the length mismatch is a clean error; (d) omit the `DEBUG` define → clean unknown-name error naming `DEBUG`.
- [ ] GREEN + commit — `test: T2 negative probes — straddle/wrong-bank/table-len/missing-define all loud`

### Task 9: docs + checkpoint

- [ ] Update this plan's `## Execution notes` (deltas, RED evidence, review findings) as tasks land — not at the end.
- [ ] Whole-branch adversarial review (two-pronged, the #7 pattern): one reviewer on the sigil diff (deferral + defines are the load-bearing surfaces — hunt byte-drift on the RESOLVED paths and diagnostic regressions), one on the aeon diff + `.emp` fidelity (every guard message preserved? every consumer enumerated?).
- [ ] Completion handoff note in `docs/superpowers/notes/`, memory update, checkpoint summary for Volence. **NO merge without the checkpoint** (both repos; sigil branch `sound-migration-t2`, aeon branch `sigil-emp-mt`).

---

## Self-review (spec coverage)

- DSM.6(a) descriptor values exported .emp→.asm: T2 exports NO new equs (the DAC ones were T1; MT's cross-seam surface is the two `movea.l` label reads — R3). ✓ covered by Tasks 3/7.
- DSM.6(c) co-location fatals → cross-source ensures: R5/Task 5. ✓ (window-top + pitch-contiguity additions covered — the design listed six guards, reality is seven; all seven mapped.)
- DSM.8 T2 scope (streams + tables + three song_table fatals): Tasks 4-7. ✓
- DSM.9 verification bar: per-file byte-diff (Task 6), full-ROM both shapes (Task 7), workspace fully green (Task 7 Step 3), divergences itemized (stop rules). ✓
- Handoff wrinkles: two build shapes ✓ (defines + per-shape tests); cross-seam `MovingTrucks_Bank_Start`/`SND_ENGINE_TABLE_BANK` ✓ (Probe-B pattern in Tasks 6/5); engine-default `movingtrucks_pitchtable.asm` NOT ported ✓ (stays in the head; its hand-edit-vs-generator conflict is untouched — flag Volence if any task must touch it); bank base $60000 held after the bg rebuild ✓ (verified in the fact base).

---

## Execution notes

### Task 1 (comptime defines) — DONE (commits `b23a8a0` + proc-collision follow-up)

- RED recorded: both plan tests added to `crates/sigil-frontend-emp/tests/lower_data.rs` before the field existed; `cargo test -p sigil-frontend-emp defines_` failed E0560 (`LowerOptions` has no field `defines`). Implementation as specified: `LowerOptions.defines: Vec<(String, i128)>`, `Evaluator::seed_defines()` populates the evaluator's `defines` map at every `Evaluator::with_file` entry point `lower_module` drives, `eval_path` resolves a define like a const (bare-name fallback after consts/equs, returning the resolved `Value::Int` directly), CLI `-D NAME=INT` (decimal/`$hex`/`0x`) on both `sigil emp` paths, one subprocess test (`subcommands.rs::sigil_emp_dash_d_selects_debug_branch` + `tests/vectors/defines.emp`).
- Plan delta: `[defines.collision]` reports from a new once-per-compile `validate_defines` pass in `lower/mod.rs` (mirroring `validate_offsets`/`validate_dispatch`), NOT from `seed_defines` — a fresh evaluator is built per item, so reporting in the evaluator would duplicate the error once per item, and a module whose only item is the colliding `const` builds no evaluator at all (the plan's Step-1 test caught exactly this shape).
- Spec-review fix (follow-up commit): `validate_defines` initially omitted `Item::Proc`/`Item::Script` arms, so `-D Foo=5` + `proc Foo` compiled silently with the define shadowing the proc label in data initializers. Both arms added (R1: procs/scripts are items); regression test `define_colliding_with_proc_name_errors`. `seed_defines` itself is unchanged: proc/script names live in no evaluator index (`index_items` has no proc/script table), and the hard Error makes evaluator-side seeding unobservable.
- Code-review fixes (second follow-up commit): (a) dropped `seed_defines`'s `const_memo` pre-seed as DEAD code — every `resolve_const` call site is guarded by `consts`/`equs` membership, which a define never has, so the memo entry was unreachable; the `defines`-map + `eval_path` fallback is (and always was) the whole mechanism, and the three doc comments that described the memo seed as load-bearing were rewritten. (b) Seeded `validate_overlay` (the one remaining unseeded entry point on `lower_module`'s item loop), so an overlay field size expression can reference a define. (c) Pinned `parse_define_int`'s accepted/rejected forms with unit tests in `sigil-cli`'s `main.rs` and made `-D =5` (empty name) a loud exit-2 usage error.
- Known limitation (loud, not silent): a few fresh-evaluator entry points NOT on `lower_module`'s item loop do not seed defines — `layout.rs` `size_of_type` / `layout_struct` / `layout_structs_shared` / `eval_data_captures` / `resolve_overlay_window` / `layout_bitfield` / `check_value_fits_ty`, and `lower/script.rs`'s `discover_resume_slot` type probe. A define referenced through one of them fails LOUDLY with `unknown name` — never silent wrong bytes. Seed them if/when a real consumer appears.

### Task 2 (MT-shape capability probes P1-P5) — DONE, ALL FIVE INSTANT-GREEN (no fixes forced)

- **P1** (`crates/sigil-frontend-emp/tests/lower_data.rs`): `if C { embed(...) } else { Data.empty }` at the top of a `bank`-adjacent section, with a following `data Next` item. Both arms probed: `C=1` embeds the real 12-byte fixture (`tests/vectors/embed_fixture.bin`, the same fixture `sandbox_embed.rs`/`sandbox_hermeticity.rs` already use — no new fixture needed) and `Next` lands right after at offset 12; `C=0` makes `X`'s label real but zero-length, and `Next` lands at offset 0 (X's own offset) — i.e. the else arm is truly zero bytes, not a hidden pad. Used the crate's ESTABLISHED `vectors_dir()` fixed-fixture pattern (shared by two existing test files) rather than spinning up an ad-hoc `tempfile::tempdir()` — that would just reconstruct the same fixture at test time for no added coverage; the plan's "tempdir pattern" language is satisfied by `LowerOptions.include_root` pointing at a directory, which is exactly what both existing embed tests already do.
- **P2** (same file): `data T: [*u8; 3] = ["A", "B", "C"]` at TOP LEVEL (module scope, not nested in a struct field) already resolves each string element as a `Cell::SymRef` — confirmed by `label_values.rs`'s existing `nested_array_elements_in_struct_field_resolve_as_labels` (string form) plus the fact that top-level array elements only get the "unknown name" treatment for BAREWORD elements (`top_level_data_array_bare_elements_keep_unknown_name`), never for STRING elements. Pinned: three `Abs32Be` fixups at offsets 0/4/8 targeting `A`/`B`/`C`, and the LINKED bytes resolve to the labels' real addresses (0/1/2, since A/B/C are each 1-byte items placed before the table). 1-element variant (`[*u8; 1]`) pinned identically.
- **P3** (same file): `const N = if D == 1 { 3 } else { 1 }` driving `data T: [*u8; N] = if D == 1 {...} else {...}` — both shapes (`D=1`→3-elem, `D=0`→1-elem) lower with the expected fixup count/targets. The mismatched case (2 literal elements against `N` folding to 3) is a clean "array length mismatch" `Error`, not a panic — the same diagnostic `eval_data.rs`'s array-length-mismatch tests already cover for a plain (non-`if`, non-`define`-driven) length.
- **P4** (same file): `ensure(Blob.len == PINNED, "...")` where `Blob = embed(...)` and `PINNED` is a plain const — `Value::Data.len` (already implemented, `eval/expr.rs:354`) combines with the existing `ensure`/interpolation machinery with zero new code. Equal case passes silently; unequal case (`PINNED=999` vs the real 12) fires the exact interpolated message (`"blob length drifted: want 999, got 12"`) as an `Error`, not a panic.
- **P5** (`crates/sigil-cli/tests/mt_dual_carrier.rs`, new file beside `dac_port.rs`): two independently-lowered `.emp` modules, each with a `bank`-placed data section AND a top-level `equ` (⇒ each gets its OWN zero-byte default `text` carrier section, per `equ_link.rs`'s established contract). R7's contingency ("if the dual zero-byte `text` carrier pair trips a duplicate-name/overlap diagnostic, fix it HARNESS-SIDE") turned out **moot** — no diagnostic fires. `place_sections` (`sigil-frontend-emp/src/resolve/mod.rs`) places sections BY NAME against a map region using a PER-REGION CUMULATIVE cursor, so two sections that are BOTH literally named `text` place cleanly one after the other in the SAME `text` region (both zero bytes — same address, harmless, since nothing reads a carrier's address, only the `equ_syms` it carries). Pinned directly: `two_modules_sharing_the_same_text_carrier_name_place_and_link_cleanly` uses the UNMODIFIED shared `text` name (the real shape T1+T2 will hit) and asserts both banks' bytes plus both equs' folded values (`bankid($8000)=1`, `bankid($28000)=5` — distinct banks, so the fold is proven per-module, not coincidental). A second test, `two_modules_with_harness_renamed_carriers_also_work`, pins the alternative (harness renames each carrier to its own map region before placement) as an equally-valid fallback, recorded for a future task that might want per-module carrier addressability — but it was NOT needed here.
- Every probe assertion above pins BYTES/OFFSETS/ADDRESSES/fixup shapes, not merely "no error" — verified by a deliberate falsification pass (temporarily wrong expected values in three probes, confirmed each fails loudly with the real computed value, then reverted) before treating any probe as trustworthy.
- **Net result: zero implementation fixes were forced by any of the five probes.** All constructs `mt_bank.emp` needs (conditional embed/Data.empty, top-level string-keyed pointer arrays, if-expression-driven const array lengths with a clean length-mismatch error, `.len`-vs-const ensures, and dual per-module equ carriers sharing a placement region) already work correctly end-to-end. Nine tests added to `lower_data.rs`, two to the new `mt_dual_carrier.rs` — all committed as regression pins per the plan's "instant-green is still valuable" instruction.
- `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` both clean after the additions.

### Task 3 (AS-frontend 32-bit immediate deferral) — DONE

- RED recorded: `crates/sigil-frontend-as/tests/imm32_defer.rs` (new, mirrors `db_dw_defer.rs`'s structure) — `cargo test -p sigil-frontend-as --test imm32_defer` before the fix: 3 positive-deferral tests (`movea_l_unresolved_symbol_defers_as_value32be`, `move_l_unresolved_symbol_defers_as_value32be`, `move_l_unresolved_compound_defers_with_tree`) FAILED, each with today's hard error `"unresolved symbol \`ExternalSym\` in operand"`; the 3 negative controls (moveq/move.w/branch) already passed unmodified (they were never going to change).
- **Kind deviation from the task brief (reasoned, not a guess):** the brief says "the same `FixupKind` the `dc.l` deferral uses." Read literally that's `Abs32Be` — but `directive_dc_l`'s actual Poison arm only defers a BARE `Expr::Sym` via `Abs32Be` and hard-errors any compound (`"unresolved long expression"`) by deliberate design (see R-T0.4's asymmetry comment at `eval.rs` ~:2096: `dc.w`/`dc.l` were intentionally NOT migrated to the general `Value*` deferral, unlike `db`/`dw`). Ruling R3 (and the task's own Step-1 compound test) explicitly wants compounds to defer too, mirroring `db`/`dw`'s "ANY unresolved expression (bare symbol OR compound) defers" rule. The kind that actually matches that behavior is `FixupKind::Value32Be` — added in `a8b0f63` alongside `Value8`/`Value16Be`/`Value16Le` for exactly this general-link-expr-VALUE family, and ALREADY production-exercised by the `.emp` frontend for 68k 4-byte value cells (`sigil-frontend-emp/src/lower/data.rs:217`, `value_fixup_kind`), with the linker's `write_value` apply arm wired end-to-end — reusing it here means both frontends' 32-bit value deferrals share one proven kind. (An earlier draft of this note claimed the kind was "unused by any emitter before this change"; that was wrong — a silently-failed grep misread as no-matches — caught in spec review. The prior `.emp` use strengthens the choice; the AS frontend is merely its second emitter.) Used `Value32Be`, not `Abs32Be`. Compounds DO defer (pinned by `move_l_unresolved_compound_defers_with_tree`).
- Implementation: `crates/sigil-frontend-as/src/eval.rs`, new method `Asm::try_defer_long_imm` (~:2453, called from `lower_m68k_generic` right after size resolution, before the PC-relative special-cases and the existing `convert_atoms_m68k`/`lower_inst` eager path). Returns `None` (falls through to the untouched eager path) unless ALL of: size is `L`; operand shape is exactly `[Imm(expr), <bare register>]` (a register destination classifies as either `OperandAtom::RegOrCond` or `OperandAtom::Value(Expr::Sym(_))` depending on whether the word is in the Z80 reg/cond list — both shapes are matched, mirroring how `convert_one_atom_m68k` already handles this duality generally); mnemonic is `Movea` (dest must parse as `aN`) or `Move` (dest must parse as `dN`); and the folded immediate expr is `Fold::Poison`. When all hold, it hand-builds the 2-byte opcode word (computed directly from the `encode_movea`/`encode_move` bit-layout, not routed through `sigil_isa::m68k::encode` since that path has no fixup-hole concept) followed by a 4-byte zero hole, with one `Fixup { kind: Value32Be, offset: 2, target: qualified_expr }`. A resolved value returns `None` unconditionally, so the resolved path is 100% untouched — same code, same bytes, same instruction count as before this change.
- **Destination-shape scoping (the caution's stop-rule):** confirmed via `sigil_isa::m68k::encode_ea`/`encode_move`/`encode_movea` (crates/sigil-isa/src/m68k.rs:1055-1128) that `Operand::Imm` can ONLY ever be a SOURCE operand (`encode_ea` returns `IllegalDest` for a `Field::Dest` immediate) and that `encode_move`/`encode_movea` always emit the SOURCE's extension words before the DEST's (`src_ext` then `dst_ext`) — so in principle the immediate's ext-word offset is always 2 (right after the opcode) regardless of destination shape (`(d16,An)`, `(An)+`, absolute, etc. all just append MORE ext words afterward, never before). Rather than rely on that general ordering guarantee, the deferral is scoped MORE conservatively per the caution: only `aN`/`dN` bare-register destinations — the two shapes that add ZERO extension words of their own — are deferred. Every other destination shape (`(d16,An)`, `(An)+`, `-(An)`, `(d8,An,Xn)`, absolute, `(d16,PC)`) returns `None` from `try_defer_long_imm` and falls through to the pre-existing eager path, which still hard-errors an unresolved immediate there exactly as before (untested by this task's fixtures beyond the general workspace/harness regression net, since R3 and the MT-bank's actual `sound_api.asm` call sites — `movea.l #SongTable, a0` / `movea.l #SongPatchTable, a0` — only need the `aN`/`dN` shapes). No BLOCKED condition was hit; the offset-correctness concern in the caution is resolved by scope reduction, not by trusting the ordering invariant.
- GREEN: `cargo test -p sigil-frontend-as --test imm32_defer` — 8 tests (3 positive-deferral incl. the compound case, 3 negative controls, 2 added resolved-value sanity pins `movea_l_resolved_immediate_is_unaffected`/`move_l_resolved_immediate_is_unaffected`), all pass. `cargo test -p sigil-frontend-as` (whole crate) and `cargo test --workspace`: all green, no regressions. `cargo clippy --workspace --all-targets -- -D warnings`: clean (one `identity_op` lint fixed by hoisting the immediate-source encoding bits into a `const SRC_IMM: u16`).
- Byte-neutrality net: `SIGIL_STRICT_GATE=1 AEON_DIR=~/sonic_hacks/aeon cargo test -p sigil-harness` — ALL gates green, including `full_rom_matches_assembled_reference` and `full_debug_rom_matches_assembled_reference` (byte-exact full-ROM parity, both shapes) and `mixed_dac_rom_matches_assembled_reference`/`mixed_dac_debug_rom_matches_assembled_reference` — proves the resolved-immediate eager path (which the real ROM's `movea.l`/`move.l` sites all take today, since nothing cross-seam has moved yet) is completely undisturbed.
- Files changed: `crates/sigil-frontend-as/src/eval.rs` (new `try_defer_long_imm` method + one call site in `lower_m68k_generic`); `crates/sigil-frontend-as/tests/imm32_defer.rs` (new, 8 tests). No linker, `sigil-frontend-emp`, or aeon files touched.

### Task 4 (aeon prep — ids move, `.bin`s committed, the gate) — DONE (aeon commit `34fcf68` on branch `sigil-emp-mt`)

- Branched `sigil-emp-mt` off aeon `master` (`e5b256c` at branch time) with `git checkout -b sigil-emp-mt master`, keeping the working tree — the uncommitted bg-restore files (entity_data.asm, vram_bases.asm, section_*.json ×3, editor_bg_override.json, tools/forest_bg_gen.py + the two untracked paths) carried over untouched and stayed untouched/unstaged through the whole task; confirmed by `git status --short` before and after the commit (identical dirty/untracked set, none of it staged or committed).
- **R2 (id move):** moved `SONG_MOVINGTRUCKS = 1` and the `ifdef __DEBUG__` block (`SONG_DRUMTEST = 2`, `SONG_HCZ2 = 3`) + their full comment blocks from `games/sonic4/data/sound/song_table.asm` (old lines 12-36) into `games/sonic4/config/sound_ids.asm`, placed right after the file's header comment (which was expanded to note the id constants now live there and that the table ORDER stays in data/sound, soon `mt_bank.emp`). `song_table.asm` keeps only the `ifdef __DEBUG__ / SONG_COUNT = 3 / else / SONG_COUNT = 1 / endif` shape, with a short comment pointing at the new home. `main.asm:12` already included `config/sound_ids.asm` before any consumer (game.asm/game_debug.asm `moveq #SONG_*` sites, song_table.asm itself) — no include-order change needed.
- **Verifier (Step 3 pre-check):** ran `tools/verify_emit_bin.py` against all six target `.asm` files before touching `.gitignore` — **6 passed, 0 failed**: `song_movingtrucks.asm` (12465 B), `movingtrucks_pitchtable_stream.asm` (264 B), `movingtrucks_patches.asm` (800 B), `song_drumtest.asm` (82 B), `song_hcz2.asm` (6511 B), `hcz2_patches.asm` (128 B). All six `.bin`s (from T1 commit `4782cde`, already present on disk, untracked) matched their `dc.b` twins byte-for-byte — no regeneration needed, no deviation.
- **`.gitignore`:** added six targeted `!games/sonic4/data/sound/<name>.bin` un-ignore lines (not a blanket unignore) right after the existing "Allow sprite art and DPLC data" block, with a short comment explaining why (T2 commits the emit-bin twins, mirroring the committed generated `.asm`, since the emitters need external inputs unavailable in prebuild). `git check-ignore -v` on each of the six still prints the LAST matching rule, which is the `!`-negation itself (git's normal reporting behavior for a file a later rule un-ignores) — verified the files are actually stageable via `git add -n`, which listed all six cleanly; not a real ignore.
- **The gate (R6):** wrapped `main.asm` lines 150-208 (post-Step-1 numbering; `include song_movingtrucks.asm` through `include song_table.asm`, inclusive of the :164-166 `MT_PITCHTAB_OFFSET` fatal and the nested `ifdef __DEBUG__` block for DrumTest/HCZ2) in `ifndef SIGIL_EMP_MT … else … endif`, exactly per the plan's snippet — the pre-existing inner `ifdef __DEBUG__/endif` (DrumTest+HCZ2) nests unchanged inside the new outer gate. Else-branch org-resumes: `ifdef __DEBUG__` → `org $6553A`, else → `org $63AE8`, matching the fact-base pins exactly (no drift from the bg-restore re-baseline).
- **Byte-verification, both times required:** (1) after Step 1 (id move only) — plain `./build.sh sonic4` → `sha256sum s4.bin` = `8ce6dd7e30553b8525ddda19ebe3365cc5d24cc62dccfb9c0e6a227d70bc25ef` (match); `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4` → `13c7b06355b658ee299756840a80b566005cdbbd5192755e8eae506a5f4fd22f` (match). (2) after Step 4 (the gate added) — re-ran both builds, same two hashes, both matched again. Finished by rebuilding plain a third time so `s4.bin`/`s4.lst` were left in the non-debug state; `s4.debug.bin` (`13c7b063…`) was never written to by this task (the debug build only produces `s4.bin`) and its hash was reconfirmed unchanged after the final plain rebuild.
- **The six `.bin` sha256s** (recorded in the commit message): `song_movingtrucks.bin` = `7342777ec87f2f9f27fa279f7f356498db2f828673841b98f6702a081c6a1567`; `movingtrucks_pitchtable_stream.bin` = `11563261b743ffd30c8c3ec5b145dbb03578bec535621a53d7adea2e9607144f`; `movingtrucks_patches.bin` = `fd821afc0c7976804c025cf7015c73e65bed2b234c3907cd4d72bd6265962b1b`; `song_drumtest.bin` = `6c0848457b17a07eb94c0a34f1927be128952ddbd3388d0dbef01abb6a5f6416`; `song_hcz2.bin` = `02cb28717be712fd439edf441027f1b159eb5adf9e6b4d02ece4e5f23f64ffaa`; `hcz2_patches.bin` = `65c03df9f9abd2116f9e5909686f827b8487292c2b0865603ddcb6d51670d4a5`.
- **Commit:** ONE commit `34fcf68` on `sigil-emp-mt` (parent `e5b256c`, aeon master at branch time) — `build: SIGIL_EMP_MT gate + song-id equates to sound_ids.asm + MT/HCZ2 stream .bins committed (inert for asl; byte-verified both shapes)`. Files: `.gitignore`, `games/sonic4/config/sound_ids.asm`, `games/sonic4/data/sound/song_table.asm`, `games/sonic4/main.asm`, the six new `.bin`s. `git status --short` post-commit shows only the pre-existing bg-restore dirty/untracked files — nothing from this task left uncommitted, nothing bg-restore-related touched.
- No deviations from the plan's Task 4 steps or R2/R6 spelling; no BLOCKED conditions hit.

### Task 5 (`mt_bank.emp`) — DONE (aeon commit `07b5212` on branch `sigil-emp-mt`)

- **File:** `aeon: games/sonic4/data/sound/mt_bank.emp` (185 lines), module `data.mt_bank`. Item order exactly per the sketch: six `const … = embed(...)` blobs → `SONG_MOVINGTRUCKS`/`SONG_DRUMTEST`/`SONG_HCZ2` id consts → `SONG_COUNT` if-expression + `< $FF` ensure → `MT_PITCHTAB_OFFSET` pinned const + the detune ensure → ONE `section mt_bank (cpu: m68000, bank: $8000)` holding, in order, `Song_MovingTrucks`, `MovingTrucks_PitchTable_Stream`, `MovingTrucks_Patches`, `Song_DrumTest` (+ its pad), `Song_HCZ2` (+ its pad), `HCZ2_Patches`, `SongTable`, `SongPatchTable` → the five cross-seam co-residency ensures after the section closes.
- **Deviation 1 — the two trailing-`align 2` pads (both landed, spelled identically as self-adjusting conditionals):** `data _drumtest_align = if DEBUG == 1 && DrumTestBlob.len % 2 == 1 { byte(0) } else { Data.empty }` and the HCZ2 twin `_hcz2_align`. Verified against the live `.asm`/`.bin` pair: `song_hcz2.bin` is 6511 B (ODD) and `song_hcz2.asm` has `align 2` right after `Song_HCZ2_End` — so the HCZ2 pad FIRES today, emitting the one $00 the reference ROM carries at $654A1 (confirmed independently: a standalone no-cross-seam build of the DEBUG shape totals exactly 20275 bytes, and `$60607 + 20275 = $6553A`, matching the fact base's debug-tail resume address bit-for-bit — that arithmetic only comes out right if the HCZ2 pad byte is present). `song_drumtest.bin` is 82 B (EVEN), so `_drumtest_align` folds to `Data.empty` today (confirmed: the same standalone build's non-DEBUG total is exactly 13537 bytes, and `$60607 + 13537 = $63AE8`, the fact base's plain-tail resume address) — a true no-op, kept only for regen-safety parity with the .asm's unconditional `align 2`. Grepped `song_movingtrucks.asm`/`movingtrucks_pitchtable_stream.asm`/`movingtrucks_patches.asm` for a trailing `align` — none found, confirming the plan's "no pads elsewhere" claim; no other pad added.
- **`byte(0)` spelling:** grepped `crates/sigil-frontend-emp/src/eval/builtins.rs` for the `Data` constructors — `byte(x)`/`bytes([...])` are free comptime fns (NOT `Data.byte`/`Data.bytes` member-style, despite the task brief's phrasing), while `Data.empty` (a bare two-segment path, `crates/sigil-frontend-emp/src/eval/expr.rs:226`) is exactly as the sketch already spelled it. Used `byte(0)` for "one zero byte as Data" (pinned precedent: `eval_data.rs`'s `data D = bytes("HI") ++ byte(0)`, the author-controlled-terminator idiom).
- **Deviation 2 — the ensure spelling could NOT be the sketch's bare `SND_ENGINE_TABLE_BANK` (this is the load-bearing finding of the task).** Read `crates/sigil-cli/tests/ports.rs`'s `probe_b` module (the T0 proof the task pointed at): it proves `ensure(bankid("AsProbeLabel") == $B, …)` — a QUOTED label string on the LHS via `bankid(...)`, compared to a plain int literal — never a bare unquoted cross-seam NAME as an operand. Traced why: `eval_path` (`eval/expr.rs:169-214`) only falls back to a deferred `Value::Label`/link-symbol for an unknown bareword when `label_ctx_active()` is true, and `label_ctx` is only ever turned on by `eval_call_arg` (`eval/call.rs:358-372`) — i.e. ONLY the direct argument of a call (like `bankid`'s own arg) gets that treatment. A bareword used as a plain sub-expression operand of `==` (not itself a call argument) goes through ordinary `eval_expr`, which has no such fallback and hard-errors `"unknown name"` at comptime. **Confirmed empirically**: a throwaway probe `ensure(bankid("Song_MovingTrucks") == SND_ENGINE_TABLE_BANK, …)` failed with `unknown name \`SND_ENGINE_TABLE_BANK\`` at comptime (never even reaching link) — falsifying the sketch's literal spelling before it was carried into the real file. Also confirmed there is no generic "read this external equ's raw value" builtin (`bankid`/`winptr` both apply Genesis-specific mask/shift arithmetic; neither is a bare passthrough). **Fix**: `SND_ENGINE_TABLE_BANK = MovingTrucks_Bank_Start >> 15` (main.asm:143) and `MovingTrucks_Bank_Start` is bank-aligned (`align $8000`, main.asm:129), so `(L & $7F8000) >> 15 == L >> 15` for that L — `bankid("MovingTrucks_Bank_Start")` folds to the IDENTICAL value as the equ, and `MovingTrucks_Bank_Start` (unlike the equ) IS a genuine `.asm` label, so it's a legal `bankid()` argument via the proven `probe_b` idiom. All five co-residency ensures now read `ensure(bankid("X") == bankid("MovingTrucks_Bank_Start"), "…")`. Verified this substitution's failure mode is the CORRECT one (a link-time Error naming the ensure's own message when the label lands elsewhere, not a comptime "unknown name") via a second throwaway probe: `ensure(bankid("Song_MovingTrucks") == $B, …)` against a standalone label — failed with exactly the guard's own message ("engine-table co-residency"), not an internal error.
- **A related wrinkle worth flagging, not a blocker:** when BOTH sides of the `==` are unresolved link-exprs referencing symbols absent from the standalone compile's symbol table (the real shape `mt_bank.emp` hits standalone, since `Song_MovingTrucks`/`MovingTrucks_Bank_Start` are both genuinely undefined without the map+harness), `check_link_asserts` (`crates/sigil-link/src/lib.rs:~241`) reports `Fold::Poison` as `"internal: deferred link assertion has an unresolvable condition (an anchor label was never defined) — this is a compiler bug in the \`here()\`-relaxation fix, not a source error"` — worded as if it should never happen, but it's exactly the expected standalone failure shape for a cross-seam ensure with BOTH operands external. Still a genuine `Level::Error` (build fails loudly, no silent pass), so it satisfies this task's "fails at the cross-seam" bar, but the message is misleading for this legitimate case; flagged for Task 6/9 (not fixed here — out of this task's scope, and the plan didn't ask for a link-diagnostics change).
- **Compile observations, both shapes, standalone (`sigil emp mt_bank.emp -D DEBUG=0|1`, no `--root`/`--map`):** `sigil parse` — clean, `module data.mt_bank, 19 items`. `sigil emp -D DEBUG=0` and `-D DEBUG=1` — BOTH fail at exactly the five cross-seam ensures (lines 176/178/180/182/184, the `bankid(...) == bankid("MovingTrucks_Bank_Start")` block), each with the "internal: …anchor label…" wording above (both operands are cross-seam-unresolved in a no-map single-file compile) — confirming everything BEFORE that point (both blob embeds, the `SONG_COUNT`/`< $FF` ensure, the `MT_PITCHTAB_OFFSET` ensure, both alignment pads, both tables) lowers cleanly in BOTH shapes. Positively confirmed by a second run with the five cross-seam ensures stripped (a scratch copy, not committed): both shapes then build FULLY standalone — `DEBUG=0` → `built: 13537 bytes`, `DEBUG=1` → `built: 20275 bytes`. Independently cross-checked against the fact base: `$60607 + 13537 = $63AE8` (plain-shape resume) and `$60607 + 20275 = $6553A` (debug-shape resume) — both match the fact base's pins EXACTLY, byte-for-byte, which is strong evidence the item order, blob lengths, table sizes, and (critically) the HCZ2/DrumTest pad logic are all correct, not merely "no diagnostic fired."
- **Length-guard note (per the task's correction #2):** only `SongBlob.len` is `ensure`d against a pinned const (`MT_PITCHTAB_OFFSET = $30B1`, load-bearing — the detune guard). The other five blob lengths (264/800/82/6511/128) are NOT individually pinned with `ensure`s — ARM'd against regen churn per the task instruction — but are cited in comments and cross-validated by the byte-count arithmetic above.
- No BLOCKED conditions; no other deviations from the corrected brief (the six embeds, the id consts, `SONG_COUNT`, the table if-expressions, and the section/bank property all match the sketch verbatim).
- **Commit:** aeon `07b5212` (parent `34fcf68`) — `feat(port): mt_bank.emp — the MT streaming bank + song tables (sound-migration T2)`. Only file: `games/sonic4/data/sound/mt_bank.emp` (185 insertions). `git status --short` before/after showed the pre-existing bg-restore dirty/untracked set untouched and unstaged.
