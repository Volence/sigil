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

- [ ] **P1:** `data X = if C { embed(...) } else { Data.empty }` — zero-length data item in the else arm: label defined, zero bytes emitted, following item lands at the same offset. (Use a tiny fixture bin via the include_root tempdir pattern from the sandbox tests.)
- [ ] **P2:** `data T: [*u8; 3] = ["A", "B", "C"]` — array-of-Ptr with string elements → three `Cell::SymRef` Abs32 cells (12 B). And the 1-element variant.
- [ ] **P3:** `const N = if D == 1 { 3 } else { 1 }` driving `[*u8; N]` with an if-expression RHS of matching length — and the MISMATCHED length (2 elems vs N=3) is a clean error, not a panic.
- [ ] **P4:** an `ensure` mixing a comptime `.len` and a pinned const (`ensure(Blob.len == $30B1, "...")`) — fires loud on mismatch.
- [ ] **P5 (link-level, in `crates/sigil-cli/tests/` beside dac_port.rs helpers):** TWO separately-lowered modules, each with top-level `equ`s (⇒ two zero-byte `text` carriers), sections placed by one map + linked together — link succeeds, both equ sets resolve. If the paired carrier trips a dup-name/overlap diagnostic, fix per R7 (harness-side naming/stagger, NOT a linker change) and record the delta.

- [ ] **Steps per probe:** write test → RED (or instant green if the capability exists — then it's a pinned regression test, still committed) → minimal fix if RED (expected only for gaps; keep fixes additive and small — if a probe reveals a MISSING mechanism bigger than ~a day, STOP and surface to the orchestrator rather than improvising).
- [ ] **GREEN + clippy**, then **Commit** — `test(emp): MT-shape capability probes (cond-embed/ptr-arrays/if-arrays/len-ensure/dual-carrier)`

### Task 3: AS-frontend 32-bit immediate deferral

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs` (the m68k immediate-operand encoding path; find via the `movea`/`move` long-imm encoding, near the abs-operand machinery ~:1841-2100 and the poison bookkeeping at :55-86)
- Test: same crate, beside the T0 db/dw-deferral tests (grep `directive_dw` deferral tests and mirror their structure)

- [ ] **Step 1: Failing test** — assemble a unit containing `movea.l #ExternalSym, a0` where `ExternalSym` is NOT defined in-unit; expect (today) the `unresolved symbol in operand` error → the test asserts instead: module assembles, the instruction's fragment carries a 4-byte hole at the immediate's offset + a value fixup targeting `Expr::Sym("ExternalSym")` with the same `FixupKind` the `dc.l` deferral uses (grep `dc.l`/width-4 deferral for the exact kind name). Also a companion test: `move.l #ExternalSym, d0` (same class), and a NEGATIVE control: an unresolved symbol in a **non-deferrable** position (e.g. a `moveq #Sym, d0` imm8 or a branch target) still errors — the deferral is scoped to long immediates only.
- [ ] **Step 2: RED** — `cargo test -p sigil-frontend-as imm_deferral` (name accordingly).
- [ ] **Step 3: Implement.** In the long-immediate encoding arm: when the folded operand is Poison-due-to-unresolved AND this is the converged pass path, emit the 4-byte placeholder + fixup and DON'T register the poison ref (mirror `directive_dw`'s arm at ~:2062 — "ANY unresolved expression (bare symbol OR compound) defers"). Keep the eager path untouched for resolved values.
- [ ] **Step 4: GREEN**, then the byte-neutrality net: `SIGIL_STRICT_GATE=1 AEON_DIR=~/sonic_hacks/aeon cargo test -p sigil-harness` — ALL reference gates still green (no resolved-path drift).
- [ ] **Step 5: Commit** — `feat(as): defer unresolved 32-bit immediates to link fixups (movea.l/move.l #Sym cross-seam; imm8/branches still loud)`

### Task 4: aeon prep — ids move, `.bin`s committed, the gate

**Files (all in `~/sonic_hacks/aeon` — SEPARATE repo, branch `sigil-emp-mt` off its master):**
- Modify: `games/sonic4/config/sound_ids.asm`, `games/sonic4/data/sound/song_table.asm` (ids move, R2)
- Modify: `.gitignore` (+ 6 un-ignores), add the 6 `.bin`s
- Modify: `games/sonic4/main.asm` (the R6 gate)

⚠ aeon has the UNCOMMITTED bg-restore working tree awaiting Volence's boot-check. Branch from master and carry the working-tree changes along (they're in data/tools files disjoint from the sound files); do NOT commit the bg files on this branch — commit ONLY the sound-arc files listed here. If the boot-check lands first, rebase trivially.

- [ ] **Step 1:** Move the three `SONG_*` id equates (song_table.asm:17,24,32 with their comment blocks, keeping the `ifdef __DEBUG__` around ids 2-3) into `config/sound_ids.asm`; delete them from song_table.asm (leave `SONG_COUNT` where it is).
- [ ] **Step 2:** Verify byte-neutrality: `./build.sh sonic4` then `sha256sum s4.bin` == `8ce6dd7e30553b8525ddda19ebe3365cc5d24cc62dccfb9c0e6a227d70bc25ef`; `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4` == `13c7b063…` (then rebuild plain to restore, per PROVENANCE).
- [ ] **Step 3:** `.gitignore` exceptions + `git add` for: `song_movingtrucks.bin`, `movingtrucks_pitchtable_stream.bin`, `movingtrucks_patches.bin`, `song_drumtest.bin`, `song_hcz2.bin`, `hcz2_patches.bin` (all under `games/sonic4/data/sound/`). First verify each still byte-matches its `dc.b` twin: the T1 verifier tool (`tools/` — the dc.b-equality checker from commit `4782cde`) — run it; if any is stale, regenerate via the emitter's `--emit-bin` and note it.
- [ ] **Step 4:** The gate in `games/sonic4/main.asm`: wrap lines 150–208 (from `include …song_movingtrucks.asm` through `include …song_table.asm`, INCLUDING the :164-166 pitch fatal):

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

- [ ] **Step 5:** Byte-neutrality again (gate OFF path): both shapes' sha256 unchanged (same two hashes as Step 2).
- [ ] **Step 6:** Commit on `sigil-emp-mt` — `build: SIGIL_EMP_MT gate + song-id equates to sound_ids.asm + MT/HCZ2 stream .bins committed (inert for asl; byte-verified both shapes)`

### Task 5: `mt_bank.emp`

**Files:**
- Create: `aeon: games/sonic4/data/sound/mt_bank.emp` (same aeon branch)

- [ ] **Step 1:** Write the module. Full content (header comments abridged here to the load-bearing ones — write them in the dac_samples.emp voice, citing the .asm guards each ensure replaces):

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

- [ ] **Step 2:** Standalone compile check, both shapes: `sigil emp games/sonic4/data/sound/mt_bank.emp -D DEBUG=0` (link-ensures referencing `SND_ENGINE_TABLE_BANK` will fail standalone — if the CLI can't inject external symbols, this step just checks PARSE+comptime via `sigil parse`; the real compile happens in Task 6's harness. Record which.)
- [ ] **Step 3:** Commit (aeon branch) — `feat(port): mt_bank.emp — the MT streaming bank + song tables (sound-migration T2)`

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
