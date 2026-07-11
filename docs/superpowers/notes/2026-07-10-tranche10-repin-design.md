# Step-0 design — the `repin` tool (tranche 10; ratified plan from the t10 handoff)

Volence's ask (2026-07-10): "20 minutes for a simple fix smells funny." Root
cause: ~115 hand-typed layout literals across ~16 test files (survey below);
re-pin waves are string substitution, and each substitution error costs a
suite run to find. This note settles the design BEFORE code, per the loop.

## What exists today (survey, 2026-07-10 @ sigil eccd038 / aeon 1e94102)

- **Per-shape region pins** in 13 `sigil-cli` port tests: base + len (+
  per-test cross-seam `(symbol, VMA)` label lists, region-relative offsets
  like `CC_DELETE_OFF`). rings/collision/animate/test_objects/sound_api/
  game_loop carry `Shape` structs; the older ports inline base/len pairs.
- **Harness pins**: `m1d_rom.rs` `ASSEMBLED_LEN = 0x658B4`,
  `m1d_debug_rom.rs` `DEBUG_ASSEMBLED_LEN = 0x673A2` (both == the listing
  `END` address), `m1c_vector_table.rs` 14 exception-stub VMAs,
  `lib.rs:582/589` `CONVSYM_REWRITTEN{,_DEBUG} = [0x18E,0x18F,0x1A6,0x1A7]`.
- **No test parses a listing today.** All pins are hand-typed.
- The listings (`aeon/s4.lst`, `aeon/s4.debug.lst`) carry a proper
  `Symbol Table (* = unused):` section — two `NAME : VALUE TYPE |` columns
  per line, `*` = unused, local labels appear as `Parent.local`, RAM
  symbols as sign-extended 64-bit hex (`FFFFFFFFFFFF89EE`), long entries
  span a full line. This is the parse source — NOT the source lines.

## Decisions

**D-T10.1 — Home: a `repin` binary in `sigil-harness`** (`src/bin/repin.rs`,
logic in `sigil_harness::repin` so tests can call it). NOT a `sigil` CLI
subcommand: the assembler CLI is public-release surface (adoption tenet);
aeon-specific dev tooling doesn't belong in it. The harness is already the
aeon-aware crate (golden/, PROVENANCE, AEON_DIR conventions). Run as:
`cargo run -p sigil-harness --bin repin -- [--aeon DIR] [--check] [--verbose]`.

**D-T10.2 — One declarative manifest**, `crates/sigil-harness/repin.toml`:
- `[[region]]` — `name`, `start` (symbol), `end` (symbol), `tests` (the
  test binaries that consume it, printed as the rerun hint).
- `[[symbol]]` — bare cross-seam names (RAM cells, call targets, equ-like
  VMAs), each with optional `tests`.
- `[[offset]]` — `name`, `sym` (dotted local ok), `region` — emitted as
  `sym - region.start`, asserted shape-INVARIANT unless `per_shape = true`.
- `[rom]` — `end_symbol = "__END__"` sentinel: the listing `END` line's
  address (the assembled-length pins).
Unknown symbol in either listing = HARD ERROR naming it (never a silent 0).

**D-T10.3 — Output: generated, checked-in
`crates/sigil-harness/src/pins.rs`** with a DO-NOT-EDIT header + provenance
(listing paths, their `Source File ... Page` date stamps, region count).
Shapes are explicit — no chaining:

```rust
pub struct Pin { pub plain: u32, pub debug: u32 }
pub struct Region { pub plain_base: u32, pub debug_base: u32,
                    pub plain_len: usize, pub debug_len: usize }
pub const ANIMATE: Region = Region { plain_base: 0x2D78, .. };
pub const DELETE_OBJECT: Pin = Pin { plain: 0x281C, debug: 0x29AE };
pub const CC_DELETE_OFF: usize = 0x104; // shape-invariant, asserted
```

Slice ranges in tests become `base..base + len` COMPUTED from these — the
slice-end bug class is unwritable. Lens are computed `end - start` at
generation, PER SHAPE (core's debug len ≠ plain len: the assert
transliterations — first shape-dependent region since rings).

**D-T10.4 — Review stays in the loop.** The tool diffs the freshly
generated table against the existing `pins.rs` and prints every changed
pin as `name: old → new (Δ)` plus the union of the changed pins' `tests`
lists as the rerun hint. `--check` regenerates and exits nonzero on drift
without writing (CI/staleness mode). Only the typing is automated — the
strict suite still independently verifies bytes.

**D-T10.5 — Staleness guard as a test.** A reference-dependent
`#[test] pins_rs_is_current()` in sigil-harness regenerates in-memory and
compares against the committed file; skips green when aeon/listings are
absent, HARD FAILS under `SIGIL_STRICT_GATE=1`. A stale pins.rs can no
longer hide.

**D-T10.6 — Convsym allowlist: derived but CONFINED.** Replace the pinned
`CONVSYM_REWRITTEN{,_DEBUG}` arrays with a computed diff of
assembled-vs-final header bytes, asserted ⊆ the SEMANTIC field set
`{0x18E,0x18F}` (checksum word) ∪ `{0x1A4..0x1A7}` (ROM-end long). Those
bounds are Genesis header facts, not layout pins — they stay as named
consts. The exact rewritten subset shifts whenever the deb2 append
changes size; deriving it kills that re-pin row entirely.

**D-T10.7 — engine.inc org values stay hand-written** (they are asl build
INPUTS), but `repin` PRINTS the ready-to-paste per-gate block:

```
    ifdef __DEBUG__
        org     $31C4
    else
        org     $2F0A
    endif
```

for every `[[region]]` with `gate = "SIGIL_EMP_*"` in the manifest.

**D-T10.8 — Acceptance = the live baseline.** The tool's first green is:
generated pins from the CURRENT listings byte-match every CURRENT
hand-typed value (the survey table is the cross-check). Parser TDD runs
against vendored listing EXCERPTS (hermetic) + the real listings
(reference-dependent).

**D-T10.9 — Migration is per-binary, affected-first.** Convert one test
file at a time to `sigil_harness::pins`, run THAT binary, full workspace +
strict pass once at the end — the ratified process note.

## What the tool does NOT do

- Never edits engine.inc (prints the block only).
- Never touches the .emp/.asm sources or maps.
- Never re-pins a NEW region into existence — adding a region is a
  manifest edit + regenerate (deliberate, reviewed).
- Doesn't replace test-private geometry that is genuinely local (e.g.
  synthetic harness LMAs like `0x0100_0000` — not layout-derived).

## Target

A core-sized re-pin wave = edit nothing by hand except engine.inc paste +
`cargo run -p sigil-harness --bin repin` + affected binaries + one full
run ≈ the 5-minute floor (two aeon builds + one suite pass).

## Tranche-10 geometry this unlocks (from the current listings)

| region | plain | debug | len |
|---|---|---|---|
| dplc | $26FC..$2794 | $288E..$2926 | 0x98 / 0x98 (invariant) |
| core | $2794..$2958 | $2926..$2C12 | 0x1C4 / 0x2EC (SHAPE-DEPENDENT) |

core's debug surplus = the three `assert` transliterations + two
`ifdebug bsr.w Debug_AssertObjLoop` sites + the `Debug_AssertObjLoop`
proc itself (rings.emp precedent for the byte-locked expansion).
