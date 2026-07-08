# Sound-Migration T0+T1 Implementation Plan (DAC bank port)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the T0 language gaps (LE cells, `equ` link symbols, AS-side value deferral, `--emit-bin`) and port `games/sonic4/data/sound/dac_samples.asm` to `.emp`, byte-identical at the full-ROM level in a mixed `.asm`+`.emp` build.

**Architecture:** Per the APPROVED design `docs/superpowers/specs/2026-07-08-sound-migration-tranche1-design.md` (DSM.1–DSM.9). The DAC data becomes two `bank: $8000` `.emp` sections pinned via `--map` at aeon's real addresses ($50000 blip / $58000 drums); the 30 `SND_*` constants become link-folded `equ` symbols consumed by unmodified `.asm` engine code; aeon's `main.asm` gets an inert `ifndef SIGIL_EMP_DAC` gate around the include. The mixed build concatenates AS + emp `Vec<Section>` and links once (the Plan-6 seam, `crates/sigil-cli/tests/ports.rs:195–243`).

**Tech Stack:** Rust workspace (`sigil-frontend-emp`, `sigil-frontend-as`, `sigil-ir`, `sigil-link`, `sigil-harness`), aeon 68k/Z80 asm + python tools.

**Process:** The #7 pattern — worktree off master, strict TDD with recorded RED, commit-per-task, two-stage reviews on load-bearing tasks (T2, T3, T4, T7, T9), whole-branch adversarial review, NO merge without a Volence checkpoint. Gate at every commit: **workspace FULLY GREEN** (no allowlist — do not cite the old 4-reds baseline), `clippy --workspace -D warnings` clean, `scripts/corpus_bytediff.sh` all-identical.

---

## Frozen rulings (record deltas in `## Execution notes`, don't relitigate)

- **R-T0.1** — LE surface is an explicit type keyword `u16le` (DSM.7: explicit-at-usage-site beats CPU inference when a 68k section emits Z80-consumed bytes). No `u32le`, no `u16be`-on-Z80 — YAGNI until a customer exists.
- **R-T0.2** — The export construct is a new top-level item **`equ NAME = expr`** (an assembler equate: ALWAYS a link-level symbol; that is its entire purpose). `pub equ` additionally makes it module-visible like other `pub` items. Rejected alternative: overloading `pub const` — would silently change the meaning of every existing `pub const`. `equ` is self-describing to ROM hackers (adoption tenet) and greppable.
- **R-T0.3** — Equ values fold at LINK, post-placement, in `resolve_layout`: `Value::LinkExpr` → residual `ir::Expr`; comptime ints → `Expr::Int`. Equ-referencing-equ allowed via iterate-to-fixpoint with a pass cap; a cycle is a loud link error naming the chain. Duplicate symbol = the existing dup-symbol link error.
- **R-T0.4** — AS `db`/`dw` unresolved-expression deferral becomes GENERAL: any unresolved expr (bare sym or compound) defers as `FixupKind::Value8`/`Value16Le` carrying the full expr tree in `Fixup.target` (the linker already folds arbitrary trees — `RelWord16Be` precedent). The old `dw`-bare-sym → `BankPtr16Le` auto-window-mask behavior is REMOVED: masking is written in SOURCE where wanted (aeon's `sfx_winptr()` macro already does), never applied silently — a `dw SND_KICK_LEN` under the old rule would emit `$857E` instead of `$057E`, the silent-wrong-bytes class. Survey existing tests for reliance; any change must be argued byte-by-byte in Execution notes. `dc.w`/`dc.l` (68k, BE) are NOT touched this tranche (no customer; note asymmetry in a comment).
- **R-T0.5** — `winptr()` is re-expressed over link-exprs: returns `Value::LinkExpr((Sym & $7FFF) | $8000)` instead of `Value::Data(Cell::SymRef{windowed})`. This **discharges ledger L7.3**, whose gate ("next quality tranche touching it") this is — an equ value must be an expression, and one general mechanism beats the windowed-SymRef special case (D7.3 logic). BYTE-DIFF-CLEAN is L7.3's own condition: every existing winptr customer (dac exhibit, corpus) must produce identical bytes via the `Cell::Expr` path. If any diverges, STOP and record — do not absorb.
- **R-T1.1** — Region bases are pinned to the CURRENT reference build: blip `$50000`, drums `$58000`, MT continues at `$60000`. These derive from aeon @ f828406 (the harness re-baseline, `s4.lst`); when aeon re-baselines, the pins move with the PROVENANCE.md rebuild. Any overlap after a re-baseline is a LOUD linker error (truthful placement), never silent.
- **R-T1.2** — `Dac_*_End` labels and `Dac_SharedBank_Start` are NOT ported (external consumers reference only `SND_*` — verified: 37 refs, all `SND_*`, in `engine/sound/dac_sample_tab.asm` + `z80_sound_driver.asm`). Lengths come from `.len`; the file comment notes the dropped labels.
- **R-T1.3** — The aeon `main.asm` gate must be INERT for asl: `ifdef SIGIL_EMP_DAC` never true in `./build.sh` builds; verified by byte-diffing aeon's own s4.bin before/after the edit.

## Reference facts (from the survey; all byte expectations derive from these)

| blob | size | addr | SND_*_BANK | SND_*_PTR | SND_*_LEN |
|---|---|---|---|---|---|
| temp_blip.bin | 2880 | $50000 | $A | $8000 | $B40 |
| dac/kick.pcm | 1406 | $58000 | $B | $8000 | $57E |
| dac/snare.pcm | 3748 | $5857E | $B | $857E | $EA4 |
| dac/hat.pcm | 240 | $59422 | $B | $9422 | $F0 |
| dac/s3k_snare.pcm | 3748 | $59512 | $B | $9512 | $EA4 |
| dac/s3k_hitom.pcm | 3724 | $5A3B6 | $B | $A3B6 | $E8C |
| dac/s3k_midtom.pcm | 4656 | $5B242 | $B | $B242 | $1230 |
| dac/s3k_lowtom.pcm | 5558 | $5C472 | $B | $C472 | $15B6 |
| dac/s3k_floortom.pcm | 6422 | $5DA28 | $B | $DA28 | $1916 |
| dac/s3k_kick.pcm | 1406 | $5F33E | $B | $F33E | $57E |

Drum payload = 30,908 B, ends $5F8BC; MT bank at $60000; `SND_ENGINE_TABLE_BANK = $C` (stays `.asm`, defined `main.asm:135`). Consumers: `engine/sound/dac_sample_tab.asm:36–104` (Z80 `db BANK` / `dw PTR` / `dw LEN` per sample, incl. BLIP), `z80_sound_driver.asm:631` (`ld a, SND_ENGINE_TABLE_BANK` — all-AS-side, unaffected). DEBUG builds do not move the DAC banks (include precedes the `__DEBUG__` block).

---

### Task 0: Worktree setup

- [ ] **Step 1:** `git -C /home/volence/sonic_hacks/sigil worktree add .worktrees/sound-migration-t0-t1 -b sound-migration-t0-t1 master` — all sigil work happens there. Aeon edits (Tasks 6, 8) happen in `/home/volence/sonic_hacks/aeon` directly on a branch: `git -C /home/volence/sonic_hacks/aeon checkout -b sigil-emp-dac`.
- [ ] **Step 2:** Baseline gate: `cargo test --workspace` (expect fully green), `cargo clippy --workspace --all-targets -- -D warnings`, `scripts/corpus_bytediff.sh` (all-identical). Record counts in Execution notes.

### Task 1: `u16le` data cells (R-T0.1)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/layout.rs` (~27–71 `Ty::Prim`; ~213–219 `resolve_type`; ~1076 `check_value_fits_ty`)
- Modify: `crates/sigil-frontend-emp/src/value.rs` (~170–233 `Cell`)
- Modify: `crates/sigil-frontend-emp/src/lower/data.rs` (~123–130 `encode_scalar`; ~196–205 `value_fixup_kind`)
- Test: `crates/sigil-frontend-emp/tests/` (follow the existing data-emission test file the exhibit tests live in)

- [ ] **Step 1: Write the failing tests** — three behaviors, exact bytes:

```rust
#[test]
fn u16le_scalar_in_68k_section_emits_little_endian() {
    // data X: u16le = $1234 in (cpu: m68000) → bytes 34 12
    let bytes = compile_data_section("section s (cpu: m68000) { data X: u16le = $1234 }");
    assert_eq!(bytes, vec![0x34, 0x12]);
}
#[test]
fn u16le_equals_u16_on_z80() {
    // Z80 sections already emit LE; u16le must be identical there (no double-swap)
    let le = compile_data_section("section s (cpu: z80) { data X: u16le = $1234 }");
    let be = compile_data_section("section s (cpu: z80) { data X: u16 = $1234 }");
    assert_eq!(le, be);
}
#[test]
fn u16le_linkexpr_cell_uses_value16le_on_68k() {
    // data B: u16le = bankid(L) in a 68k section → FixupKind::Value16Le
    // (assert on the section's fixups before link, or on final bytes after a pinned link)
}
```

- [ ] **Step 2:** Run: `cargo test -p sigil-frontend-emp u16le` — expect FAIL (`u16le` unknown type).
- [ ] **Step 3: Implement.** Add `le: bool` to `Ty::Prim { width, signed }` (compiler errors enumerate every construction site; default `false` everywhere except the new keyword). `resolve_type`: `"u16le"` → `Ty::Prim { width: 2, signed: false, le: true }`. Range rules in `check_value_fits_ty`: identical to `u16` (the `le` flag never affects ranges). Thread the flag into `Cell::Scalar { .., le: bool }` and `Cell::Expr { .., le: bool }`; in `lower/data.rs`, `encode_scalar(value, width, cpu)` gains the override (`le=true` forces the Z80 arm's byte-reverse), `value_fixup_kind(cpu, width)` gains it (`le=true` + width 2 → `FixupKind::Value16Le` regardless of CPU). `Cell::SymRef` is deliberately untouched (winptr becomes LinkExpr in Task 3; plain `Ty::Ptr` LE has no customer).
- [ ] **Step 4:** Run: `cargo test -p sigil-frontend-emp` — expect PASS, zero regressions.
- [ ] **Step 5:** Gate + commit: `git commit -m "feat(emp): u16le data cells — explicit little-endian 16-bit scalars/link-exprs from any section (R-T0.1, DSM.7)"`

### Task 1b: reject `vma:` on `bank:` sections (DSM.2 / L7.5)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs:612–679` (`section_attrs` — both attrs are parsed here)
- Test: alongside the existing `[section.bank-attr]` diagnostic tests

- [ ] **Step 1: Failing test:** `section s (cpu: m68000, bank: $8000, vma: $FF0000) { data X: u8 = 0 }` → diagnostic `[section.bank-vma]`: "a bank: section's labels must follow its placed LMA — an explicit vma: can decouple bankid()/winptr() (VMA) from the no-straddle check (LMA), yielding a wrong latch value on hardware. Drop vma: (labels follow placement) or drop bank:."
- [ ] **Step 2:** Run — expect FAIL (currently accepted).
- [ ] **Step 3:** Implement: after both attrs are read in `section_attrs`, `bank.is_some() && explicit_vma` → error, section still lowered (poison-tolerant, no cascade).
- [ ] **Step 4:** `cargo test -p sigil-frontend-emp` — PASS; the dac exhibit (no `vma:`) unaffected.
- [ ] **Step 5:** Commit: `git commit -m "feat(emp): reject bank: + explicit vma: — the wrong-latch trap is now unconstructible (DSM.2, resolves L7.5)"`

### Task 2: `equ` item — grammar, AST, evaluator binding (R-T0.2)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lexer.rs` (keyword), `src/ast.rs` (item), `src/parser/` (item parse; mirror the `const` item's shape)
- Modify: `crates/sigil-frontend-emp/src/eval/` (name resolution: an `equ`'s value is evaluated lazily like a `const`; a comptime READ of an equ whose value is a LinkExpr gets the existing `[bank.provisional]`/`[here.provisional]` refusal via `reject_if_provisional`, `eval/expr.rs:768–798`)
- Test: parser + eval test files alongside the `const` tests

- [ ] **Step 1: Failing tests:** `equ FOO = 42` parses to an `Item::Equ { name, value, is_pub: false }`; `pub equ` sets `is_pub`; `equ` as an identifier now errors (reserved-word rule — grep `examples/` + `examples/game/` first to prove no corpus collision, record in Execution notes); comptime code reading `equ B = bankid(L)`'s value in an array-size position gets `[bank.provisional]`, not a crash; `equ LEN = KickBlob.len` (comptime int value) IS readable comptime (it's an int — same rule as `const`).
- [ ] **Step 2:** `cargo test -p sigil-frontend-emp equ_` — expect FAIL (parse error).
- [ ] **Step 3:** Implement: keyword + item + lazy binding. The evaluator treats `equ` bodies exactly like `const` bodies (same lazy resolution + cycle detection); the ONLY semantic difference lands in lowering (Task 3).
- [ ] **Step 4:** `cargo test -p sigil-frontend-emp` — PASS.
- [ ] **Step 5:** Gate + commit: `git commit -m "feat(emp): equ item — assembler equates as first-class items (grammar+eval; link emission next) (R-T0.2)"`

### Task 3: equ → IR → link-time fold; `winptr` over link-exprs (R-T0.3, R-T0.5 / discharges L7.3)

**Files:**
- Modify: `crates/sigil-ir/src/lib.rs` or section module — `ir::Section` gains `pub equ_syms: Vec<EquSym>` with `pub struct EquSym { pub name: String, pub expr: Expr, pub span: Span }`
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` — lower `Item::Equ`: fold `Value::Int(n)` → `Expr::Int(n)`, `Value::LinkExpr(e)` → `e` verbatim, anything else → diagnostic `[equ.value]` ("an equ's value must be an integer or a link-time expression"); attach to the module's carrier section (the section that exists even for data-less modules — follow where module-level data items land)
- Modify: `crates/sigil-link/src/lib.rs` — in `resolve_layout`, AFTER the placement⇄relaxation fixpoint converges and labels are defined (see the label-define site at lib.rs:73), fold all sections' `equ_syms` against the symbol table, iterating with a pass cap (`MAX_EQU_PASSES = 8`); define each as `SymbolValue::Int`; unresolvable after cap → error naming the symbol and its unresolved dependency; duplicate name → existing dup-symbol error path
- Modify: `crates/sigil-frontend-emp/src/eval/builtins.rs:431–469` — `eval_winptr` returns `Value::LinkExpr(Or(And(Sym, $7FFF), $8000))` (delete the `Cell::SymRef{windowed:true}` construction; verify `ir::Expr`/linker fold support `And`/`Or`/`Shr` — `bankid` already uses `And`/`Shr`)
- Test: `crates/sigil-cli/tests/` (link-level, mixed-section shaped) + emp unit tests

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn equ_bankid_folds_to_symbol_at_link() {
    // emp: section d (cpu: m68000, bank: $8000) { data L = ... }
    //      equ B = bankid("L")   equ P = winptr("L")   equ N = <comptime 6>
    // pin section at $58000 via map/region; after resolve_layout+link the
    // symbol table must contain B=$B, P=$8000, N=6.
}
#[test]
fn equ_chain_folds_and_cycle_is_loud() {
    // equ A = B + 1; equ B = bankid("L")  → folds (pass 2)
    // equ X = Y; equ Y = X                → link error naming X/Y
}
#[test]
fn winptr_data_cell_byte_identical_via_linkexpr() {
    // The L7.3 condition: a data cell `data P: u16 = winptr("L")` in both a 68k
    // and a z80 section must emit EXACTLY the bytes master emits (build the same
    // program on master first and pin the bytes as constants in the test).
}
```

- [ ] **Step 2:** Run — expect FAIL (`equ_syms` doesn't exist; winptr still Data).
- [ ] **Step 3:** Implement in this order: (a) IR field + link fold + tests green with hand-built sections; (b) lowering attach; (c) winptr switch LAST, then immediately run the exhibit acceptance (`dac_bank_acceptance.rs`) and `scripts/corpus_bytediff.sh` — R-T0.5 says any byte divergence is a STOP.
- [ ] **Step 4:** Full gate: `cargo test --workspace` + corpus_bytediff — PASS, all-identical.
- [ ] **Step 5:** Commit: `git commit -m "feat(ir+link+emp): equ symbols folded post-placement + winptr over link-exprs (discharges L7.3, byte-diff-clean) (R-T0.3, R-T0.5)"`
- [ ] **Step 6:** Two-stage review (spec conformance vs this plan's rulings, then `superpowers:code-reviewer`).

### Task 4: AS-side `db`/`dw` general expr deferral (R-T0.4)

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs:1983–2054` (`directive_db`, `directive_dw`)
- Test: `crates/sigil-frontend-as/tests/` (follow existing directive-test layout)

- [ ] **Step 1: RECORD RED + survey.** First grep the workspace for tests exercising the current `dw`-unresolved → `BankPtr16Le` path (`grep -rn "BankPtr16Le" crates/ --include="*.rs"`). List every hit in Execution notes with a byte-level argument for why the new behavior is right for it (expected: the deferral arm is only reachable cross-seam; in-program assembly always folds by the final pass). Then write failing tests:

```rust
#[test]
fn db_unresolved_symbol_defers_as_value8() {
    // Z80: `db SND_KICK_BANK` with the symbol undefined at assembly →
    // one byte $00 + Fixup{kind: Value8, target: Sym("SND_KICK_BANK")};
    // linking against a table where SND_KICK_BANK=$B yields byte $0B.
}
#[test]
fn dw_unresolved_symbol_defers_verbatim_le() {
    // `dw SND_KICK_LEN` with SND_KICK_LEN=$057E at link → bytes 7E 05.
    // Under the OLD BankPtr16Le rule this emitted 7E 85 — the recorded RED.
}
#[test]
fn dw_unresolved_compound_expr_defers_with_tree() {
    // `dw (SomeLabel & $7FFF) | $8000` with SomeLabel cross-seam → the full
    // tree in Fixup.target, folded at link; window masking ONLY because the
    // source wrote it.
}
```

- [ ] **Step 2:** Run — expect FAIL (db: "bad byte expression"-class error; dw: masked bytes / compound error).
- [ ] **Step 3:** Implement: `directive_db` gains the same `Fold::Poison` arm shape as `directive_dw` but emitting `Value8` with the parsed tree; `directive_dw`'s Poison arm emits `Value16Le` with the tree for ANY expr (bare or compound), deleting the `BankPtr16Le` special case and the "unresolved word expression" error. Check `FixupKind::Value8` exists in `sigil-ir/src/fixup.rs` (Plan 7 #7 shipped `Value8/16Be/16Le/32Be/32Le`); range behavior comes with the kind.
- [ ] **Step 4:** Full gate incl. the s4 harness (`cargo test -p sigil-harness`) — the all-AS ROM must be untouched (this path never fires in-program).
- [ ] **Step 5:** Commit: `git commit -m "feat(as): db/dw defer unresolved exprs as Value8/Value16Le trees — no silent window-masking (R-T0.4)"`
- [ ] **Step 6:** Two-stage review (this task changes cross-seam semantics — load-bearing).

### Task 5: Cross-source probes, both directions (the DSM.8 T0 gate)

**Files:**
- Test: `crates/sigil-cli/tests/ports.rs` (extend the Plan-6 mixed-seam suite, same construction: `emp_sections() + as_sections()`, `place_sequential`/pin, `resolve_layout`, `link`)

- [ ] **Step 1: Probe A (.emp→.asm), failing first if any piece is missing:** emp module defines a pinned bank section + `equ SND_PROBE_BANK/PTR/LEN`; AS Z80 source is a verbatim miniature of `dac_sample_tab.asm`'s shape (`db SND_PROBE_BANK` / `dw SND_PROBE_PTR` / `dw SND_PROBE_LEN`); assert the linked descriptor bytes exactly (e.g. section pinned at $58000, 6-byte blob → `0B 00 80 06 00`-shaped per the table above).
- [ ] **Step 2: Probe B (.asm→.emp):** AS source defines a label at a known address; emp has `ensure(bankid("AsLabel") == $B, "engine-table co-residency")` — assert (a) it PASSES when the label is in bank $B, (b) it FAILS LOUDLY (link assert with the message) when pinned elsewhere. This is the co-location-fatal mechanism T2/T3 will reuse.
- [ ] **Step 3:** Run both — PASS (if either fails structurally, that is the arc's blocker: STOP, report, do not improvise around it).
- [ ] **Step 4:** Commit: `git commit -m "test(seam): cross-source probes — emp equ consumed by as db/dw; as label read by emp bankid ensure (DSM.8 T0 gate)"`

### Task 6: `--emit-bin` for the four aeon emitters (DSM.4)

**Files (aeon repo, branch `sigil-emp-dac`):**
- Modify: `tools/song_packer.py`, `tools/zyrinx_player.py`, `tools/smps_import.py`, `tools/sfx_transcode.py`
- Create: `tools/verify_emit_bin.py`

- [ ] **Step 1:** Add `--emit-bin <path>` to each emitter's CLI: write the exact byte string the tool already builds internally (`pack_song()` et al.) before its `emit_asm()` formatting; `emit_asm` behavior unchanged.
- [ ] **Step 2:** Write `tools/verify_emit_bin.py`: for each generated `.asm` currently in `games/sonic4/data/sound/`, parse its `dc.b`/`db` lines back to bytes and byte-compare against the corresponding `--emit-bin` output; print per-file PASS/FAIL + sizes. (The DSM.4 one-time equality check; keep it — T2/T3 rerun it.)
- [ ] **Step 3:** Run it; expect every file PASS. Record the file list + sizes in Execution notes.
- [ ] **Step 4:** Commit (aeon): `git commit -m "tools: --emit-bin on song/sfx emitters + dc.b-equality verifier (sigil sound migration DSM.4; .asm emission unchanged)"`

### Task 7: `dac_samples.emp` — the real port (T1 begins)

**Files:**
- Create: `/home/volence/sonic_hacks/aeon/games/sonic4/data/sound/dac_samples.emp`

- [ ] **Step 1: Write the file** (complete content; module dir = `games/sonic4/data/sound/`, so embed paths are relative to it):

```
// ============================================================================
// data/sound/dac_samples.emp — ROM-resident DAC sample data.
// Port of dac_samples.asm (byte-identical payload; see the tranche-1 design,
// DSM rows). The hand-written ceremony this replaces:
//   align $8000 + straddle fatals  → (bank: $8000) sections + the always-on
//                                    no-straddle link assert
//   SND_* mask/shift constants     → equ + bankid()/winptr() (derivations live
//                                    ONCE, in the builtins)
//   *_End labels + length fatals   → .len + comptime ensure
// Dropped (no external consumers; R-T1.2): Dac_*_End, Dac_SharedBank_Start.
// Regions pin blip @ $50000 and drums @ $58000 (R-T1.1) — aeon f828406 layout.
// ============================================================================
module data.dac_samples

const BlipBlob     = embed("temp_blip.bin")        // 2880 B, raw 8-bit PCM
const KickBlob     = embed("dac/kick.pcm")         // 1406
const SnareBlob    = embed("dac/snare.pcm")        // 3748
const HatBlob      = embed("dac/hat.pcm")          //  240
const S3kSnareBlob = embed("dac/s3k_snare.pcm")    // 3748
const S3kHiTomBlob = embed("dac/s3k_hitom.pcm")    // 3724
const S3kMidTomBlob = embed("dac/s3k_midtom.pcm")  // 4656
const S3kLowTomBlob = embed("dac/s3k_lowtom.pcm")  // 5558
const S3kFloorTomBlob = embed("dac/s3k_floortom.pcm") // 6422
const S3kKickBlob  = embed("dac/s3k_kick.pcm")     // 1406

ensure(0 < BlipBlob.len && BlipBlob.len < $8000, "DAC sample length must be > 0 and < $8000")
ensure(0 < KickBlob.len && KickBlob.len < $8000, "raw DAC drum length must be > 0 and < $8000")
// ... same one-line ensure per remaining blob (8 more; comptime — .len is a comptime int)

section dac_blip_bank (cpu: m68000, bank: $8000) {
    data Dac_Temp_Blip = BlipBlob
}

section dac_shared_bank (cpu: m68000, bank: $8000) {
    data Dac_Kick         = KickBlob
    data Dac_Snare        = SnareBlob
    data Dac_Hat          = HatBlob
    data Dac_S3K_Snare    = S3kSnareBlob
    data Dac_S3K_HiTom    = S3kHiTomBlob
    data Dac_S3K_MidTom   = S3kMidTomBlob
    data Dac_S3K_LowTom   = S3kLowTomBlob
    data Dac_S3K_FloorTom = S3kFloorTomBlob
    data Dac_S3K_Kick     = S3kKickBlob
}

// --- the 30 driver-facing constants (consumed by engine/sound/dac_sample_tab.asm
// via db/dw — names must match EXACTLY) ---
equ SND_BLIP_BANK  = bankid("Dac_Temp_Blip")
equ SND_BLIP_PTR   = winptr("Dac_Temp_Blip")
equ SND_BLIP_LEN   = BlipBlob.len
equ SND_KICK_BANK  = bankid("Dac_Kick")
equ SND_KICK_PTR   = winptr("Dac_Kick")
equ SND_KICK_LEN   = KickBlob.len
// ... same triple for SNARE, HAT, S3K_SNARE, S3K_HITOM, S3K_MIDTOM,
//     S3K_LOWTOM, S3K_FLOORTOM, S3K_KICK (24 more equs, names per the
//     reference-facts table — write ALL out, no macros)
```

- [ ] **Step 2: Unit-level check in sigil** (new test in `crates/sigil-cli/tests/ports.rs` or a sibling): compile the real file (path into `../aeon`, like `sigil-harness` reads aeon live) with regions `dac_blip_bank @ lma_base 0x50000 size 0x8000` and `dac_shared_bank @ lma_base 0x58000 size 0x8000` (map TOML per `sigil-link/src/map_load.rs` shape); assert (a) payload bytes at $58000..$5F8BC equal the concatenation of the 9 pcm files, (b) all 30 equ symbols fold to the reference-facts table's values exactly.
- [ ] **Step 3:** Run — PASS. Commit (aeon): the .emp file; commit (sigil): the test. `git commit -m "feat(port): dac_samples.emp — the real DAC bank port, values pinned to reference layout (T1)"`
- [ ] **Step 4:** Two-stage review (the exemplar file for the whole campaign — worth the review).

### Task 8: aeon `main.asm` gate (R-T1.3)

**Files:**
- Modify: `/home/volence/sonic_hacks/aeon/games/sonic4/main.asm` (~line 111, inside `gameSoundDataIncludes`)

- [ ] **Step 1:** Replace the bare include:

```asm
    ifndef SIGIL_EMP_DAC
        include "games/sonic4/data/sound/dac_samples.asm"
    else
        ; sigil mixed build: the DAC banks come from dac_samples.emp, pinned by
        ; the sigil map at $50000/$58000. org skips the two-bank hole; the next
        ; align $8000 (MT bank) then lands at $60000 exactly as before. If art
        ; growth ever collides with the pins, the sigil linker errors loudly.
        org     $60000
    endif
```

- [ ] **Step 2: Prove inertness (R-T1.3):** in aeon, `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4` then plain `./build.sh sonic4`; byte-compare both ROMs against pre-edit builds (`cmp`). Expect identical. Record sizes.
- [ ] **Step 3:** Commit (aeon): `git commit -m "build: SIGIL_EMP_DAC gate around dac_samples include (inert for asl; byte-verified)"`

### Task 9: mixed full-ROM harness test — the T1 acceptance

**Files:**
- Create: `crates/sigil-harness/tests/mixed_dac_rom.rs` (mirror `m1d_rom.rs:52–118`'s aeon invocation)

- [ ] **Step 1: Write the test:** (a) assemble aeon via the AS frontend from a generated wrapper source `SIGIL_EMP_DAC = 1` + `include "games/sonic4/main.asm"` (same mechanism m1d uses to reach aeon; two variants — plain and `__DEBUG__`, matching however m1d_rom selects DEBUG); (b) compile `dac_samples.emp` via the emp pipeline with the Task-7 map regions; (c) concatenate `Vec<Section>` (Plan-6 seam), `resolve_layout`, `link`, flatten with the map (fill per map default); (d) byte-diff the full image against the reference ROMs (`s4.bin` / `s4.debug.bin` vectors, PROVENANCE.md provenance) — report first-diff offset + 16-byte context on failure.
- [ ] **Step 2:** Run. Expected first failure mode: inter-section gap fill (the old in-section `align` pad bytes are now flatten fill). If bytes differ ONLY in gaps ($4xxxx-end..$50000, $50B40..$58000, $5F8BC..$60000), set the map `fill` to the reference's pad byte (inspect with `xxd s4.bin`). Every other divergence: STOP, itemize, argue (DSM.9 — expected divergences: none).
- [ ] **Step 3:** Iterate to byte-identical on BOTH build shapes. This is the tranche's acceptance bar.
- [ ] **Step 4:** Commit: `git commit -m "test(harness): mixed .asm+.emp full-ROM build byte-identical to reference (both DEBUG shapes) — T1 acceptance"`
- [ ] **Step 5:** Two-stage review.

### Task 10: negative probes

**Files:** extend Task 7/9 test files.

- [ ] **Step 1:** Straddle: an oversized synthetic blob (or a region mis-pinned to $57000) in a `bank: $8000` section → the always-on no-straddle link error naming section + extent. Length guard: a zero-length embed → the comptime ensure fires. Equ collision: a second `equ SND_KICK_BANK` cross-seam → dup-symbol link error. Overlap: shrink the `org` in a wrapper variant so AS content collides with the pinned region → loud overlap error (truthful placement, not silent corruption).
- [ ] **Step 2:** All four fire with the right diagnostics; commit: `git commit -m "test: negative probes — straddle/length/dup-equ/overlap all loud (T1)"`

### Task 11: whole-branch gate + adversarial review

- [ ] **Step 1:** Full suites: `cargo test --workspace` (fully green), clippy `-D warnings`, `scripts/corpus_bytediff.sh` all-identical, m1d + mixed harness green, aeon asl build still byte-identical (Task 8 check re-run).
- [ ] **Step 2:** Whole-branch adversarial review (fresh subagent, two prongs: seam semantics — equ fold ordering vs placement fixpoint, Value8/16Le range edges, winptr byte-equivalence; and port fidelity — every SND_* value re-derived independently from s4.lst). Fix or itemize every finding.
- [ ] **Step 3:** Write the completion handoff note (`docs/superpowers/notes/2026-07-08-sound-migration-t0-t1-complete.md`): what shipped, deltas vs this plan, T2 pointers (MT bank: the `ensure(bankid(...) == SND_ENGINE_TABLE_BANK)` pattern from Probe B, `--emit-bin` outputs ready). Update the design doc's ledger table if any ruling shifted.
- [ ] **Step 4:** STOP — Volence checkpoint before any merge (both repos: sigil branch AND aeon branch).

---

## Execution notes

(append deltas, review findings, recorded REDs, and byte arguments here as tasks complete)
