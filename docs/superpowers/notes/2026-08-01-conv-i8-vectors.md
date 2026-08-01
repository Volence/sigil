# conv-i8 — the #8 compression-vectors generator port

**Parcel:** I8 (Opus porter; overseer countersigns/merges — NOT merged/pushed).
**Repos:** aeon `conv-i8-vectors` · sigil `conv-i8-vectors` (worktree `.worktrees/opt-sweep`).
**Date:** 2026-08-01.

Headline: **the DEBUG-only golden compression-vector data is now `.emp`-native.**
The generator emits a const-only `.emp` constants module and the five golden
blobs are `embed()`d directly into the consuming module — the "embed site in an
existing module" shape the Parcel-I root finding named as one of the two legal
consumer forms. **Six-target byte-identical AND book-keeping-neutral** (CRCs
unchanged, off-canonical size tables re-derive identical) — no golden re-freeze.

---

## The census-drift check (done first)

The Parcel-I #8 analysis (notes/2026-08-01-conv-i-generators.md) held on
inspection — no drift. Confirmed live:
- `engine/debug/generated/vectors.asm` gitignored, DEBUG-only, `include`d at
  `engine/engine.inc:339` under `ifdef __DEBUG__` at `org $72BE`.
- Its 3 `CSELF_*` value equates + 5 `CSelf_*` `BINCLUDE` labels consumed
  cross-seam by `engine/debug/compression_selftest.emp`.
- `embed(...)` shipped (conv-h2); the Parcel-I "one clean stand-alone native-port
  candidate" premise correct.
One refinement vs the Parcel-I sketch ("emit `.emp` from the generator, OR fold
into residual-split"): the **least-machinery** shape is neither a residual-split
capstone nor a new placed module — it is **folding the embed sites into the
existing consumer** (the mddbg #7 precedent, generalized to ROM data). See below.

## The consumption-shape decision (+ rationale)

Two options the task framed: (A) `embed` of the generated binary, (B) the
generator emits a `.emp` module directly. The Parcel-I root finding constrains
both: the AS frontend cannot `include` a `.emp` file, so the consumer side must
be a native module OR **an embed site in an existing one**. The chosen shape
splits along the two kinds of generated content:

- **The 5 golden BLOBS → `embed()` sites folded into `compression_selftest.emp`.**
  The consuming module already owns the region that immediately precedes the
  data (its region ran `[CompressionSelfTest, CSelf_S4LZ_Plain)` = the code; the
  data started exactly at the region end, contiguous). Folding the `embed()`s in
  as the module's OWN `pub data CSelf_*` extends that one region to
  `[CompressionSelfTest, Sound_PostByte)` — **no new region / gate / pin /
  frozen-anchor / registry entry / port-test module.** This is strictly less
  machinery than a standalone placed `vectors.emp` module (option B-full), which
  the Parcel-I note itself costed at "test_mappings scale."

- **The 3 generated VALUES → a const-only generated `vectors.emp`, `use`d.**
  `CSELF_PAYLOAD_SUM` is a data-dependent checksum the generator alone can
  compute (embed bytes are not comptime-readable, embed spec §2), so the campaign
  principle "the generator owns the values, the consumer never hardcodes" is
  honored literally: the generator emits `module engine.compression_vectors` with
  the 3 `pub const`s (zero ROM bytes, opens no section — the `engine.constants`
  shape), and `compression_selftest.emp` `use`s them. The whole-tree manifest
  scan discovers it automatically; a wrong value would move the resident bytes
  (the six-CRC net) and halt the DEBUG boot self-test assert.

Rejected: hardcoding the checksum in the consumer with a generator build-time
validator (self-resolving like error_handler #7, one fewer file) — it violates
the "consumer never hardcodes" letter even with the validator, and the `use` of
a scanned const module is proven, zero-new-machinery infrastructure.

## What moved

**aeon:**
- `tools/gen_compression_vectors.py` — emits `engine/debug/generated/vectors.emp`
  (const-only, 3 `pub const CSELF_*`) instead of `vectors.asm`; still writes the
  5 `.bin` blobs; removes any stale on-disk `vectors.asm`.
- `engine/debug/compression_selftest.emp` — `use
  engine.compression_vectors.{CSELF_PAYLOAD_SIZE, CSELF_PAYLOAD_SUM,
  CSELF_DICT_LEN}`; appends the 5 golden vectors as `pub data CSelf_* =
  <embed const>`, bound to `const _Vec_*` first so each `.len` feeds a
  per-blob even-length `ensure` (the alignment guard — see step-3).
- `engine/engine.inc` — drops the `org $72BE` + `include vectors.asm`; comment
  updated (the data is `.emp`-native now, region resumes at `Sound_PostByte`).

**sigil (book-keeping — byte-neutral):**
- `crates/sigil-harness/src/pins.rs` — `COMPRESSION_SELFTEST.debug_len`
  `0x218 → 0xC8A` (code 0x218 + data 0xA72), doc `.. CSelf_S4LZ_Plain` `→ ..
  Sound_PostByte`; the 5 `C_SELF_*` symbol pins removed (module self-resolves).
- `crates/sigil-harness/repin.toml` — the `compression_selftest` region `end`
  `CSelf_S4LZ_Plain → Sound_PostByte`; the 5 `[[symbol]] CSelf_*` rows removed.
- `crates/sigil-cli/tests/compression_selftest_port.rs` — rebuilt through the
  real manifest (`Manifest::scan` + a synthetic `use engine.compression_selftest`
  entry + `build_program_open_embed`, `embed_base = aeon`) so the `use` +
  `embed` resolve for real; the CSELF_*/CSelf_* synthetic injection retired
  (the module owns them). Negative probes preserved: the doctored-checksum probe
  now REPLACES the scanned `engine.compression_vectors` module in-memory with a
  `$1234`-sum variant; the abs.w fit-lock probe places the module at `0x7800`
  (pushing `CSelf_Expected` past `$8000`) to fire the `ensure`.

## Identity (six-target FULL-CRC, chain-11 anchors)

Every target BYTE-IDENTICAL — full-file (deb2 appendix included), not
appendix-only. The `CSelf_*` labels appear identically in the symbol table, now
`.emp`-sourced; the `CSELF_*` values fold at comptime (never were appendix
symbols):

| target | CRC32 / size | proof |
|---|---|---|
| s4.bin | ff9037f2 / 412127 | direct `./build.sh` |
| s4.debug.bin | 06680f0b / 421958 | direct `DEBUG=1 ./build.sh` |
| demo.bin | 4e446a64 / 90524 | direct `./build.sh demo` |
| demo.debug.bin | 949e9215 / 93022 | direct `DEBUG=1 ./build.sh demo` (carries the vectors) |
| config_a | 2485eab3 / 422297 | `sigil build --native --config-a` |
| config_b | d6d23298 / 303501 | `sigil build --native --config-b` |

**Book-keeping-neutral too:** `repin --check` → `pins.rs unchanged` (the hand
edits match sigil's own resolve against the updated repin.toml); `derive_offcanon`
re-derives all six off-canonical size tables IDENTICAL to committed (the boundary
symbol addresses did not move). **No golden re-freeze** — the provenance chain
is a fixpoint (CRCs unchanged). The re-freeze machinery need not run.

## Gates (failures-first)

- **Strict:** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon> cargo test --workspace` →
  **2877 passed / 0 failed / 4 ignored** (= baseline; `compression_selftest_port`
  stays 4 tests, all green). Zero failures.
- **`compression_selftest_port` (rewritten):** 4/4 — the debug region byte-match
  (0xC8A window, code+data), the plain-empty shape fact, the doctored-checksum
  divergence, the abs.w fit-lock fire.
- **`repin --check`:** `pins.rs unchanged`. **`derive_offcanon`:** all 6 tables
  diff-clean.
- **Clippy:** `cargo clippy -p sigil-cli --test compression_selftest_port` — no
  warnings in the rewritten test; pre-existing workspace warnings (other crates)
  untouched.

## step-3 (retrospect) / step-5 (engine)

- **step-3 (language gap surfaced):** the AS twin's inter-blob `align 2` does NOT
  port as a top-level `.emp` `align` — an `align` after a section whose proc
  holds size-relaxable branches fires `[align.provisional]` (the pad position is
  unknowable until relaxation converges). Since every golden blob is even-length,
  the aligns were no-ops; they were dropped and replaced with five comptime
  `ensure(_Vec_X.len % 2 == 0, …)` guards (BETTER than the AS silent pad — an odd
  blob now fails LOUDLY at compile, naming the fix). Ledgered: a relaxation-aware
  `align` or a per-`data`-item alignment attribute. The `use`-a-const-module +
  `embed`-into-existing-module path used ONLY proven infrastructure — no new
  language surface invented.
- **step-5 (engine):** none — zero engine bytes changed (pure ownership move:
  AS-residual → `.emp`, same addresses). The parcel did NOT invent a standalone
  placed vectors module or a residual-split island under scope pressure — folding
  into the existing consumer is the correct minimal call.

## Retirements / re-homes

- **Retired:** `engine/debug/generated/vectors.asm` (the generator no longer
  emits it; a stale copy is unlinked). The 5 `C_SELF_*` pins + their repin.toml
  `[[symbol]]` rows (module self-resolves; no injected carriers).
- **Re-homes:** the 3 `CSELF_*` values → generated `engine.compression_vectors`
  (`vectors.emp`, gitignored, generator-authored). The 5 golden blobs →
  `compression_selftest.emp`'s own `pub data CSelf_*` embed sites.
- **Census #8** annotated DONE (notes/2026-07-31-conversion-tail-census.md).
- **Kill-list** row 102; **gap-ledger** conv-i8 entry.

## Bookkeeping for the overseer

- Branches `conv-i8-vectors` on both repos, all committed, unmerged/unpushed.
- Main sigil checkout swept clean (no stray writes); worktree touches only the 3
  intended sigil files + the 4 doc files.
- No re-freeze required (fixpoint). If the overseer runs `refreeze --check` it
  should report the chain intact against the committed blobs.
