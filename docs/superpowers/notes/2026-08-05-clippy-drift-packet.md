# 2026-08-05 — clippy toolchain-drift packet (solo/last lane)

Branch `clippy-drift` at sigil master `25174b55` (refreeze chain 46); fix commit
`d5b07ab9`. SIGIL-ONLY — zero aeon edits (aeon `.worktrees/b4` at `83ce5c2` used
as `AEON_DIR` reference only). No merge performed; no push.

## The measured number, with its scope

**64 unique findings** at master `25174b55` under
`cargo clippy --workspace --all-targets --release`, deduplicated by
(lint id, primary span, message) via `--message-format=json`.

Toolchain: **rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 / clippy 0.1.97**
(Arch Linux `rust 1:1.97.1-1.1`).

Why the historical measurements disagreed — all three were views of this one
surface:

- **~110 (overseer count)**: the raw human-readable stream is 112
  `warning:`-prefixed lines — it double-counts lib-vs-test duplicates and
  includes the per-crate `generated N warnings` summary lines.
- **62 (handoff)**: essentially the deduplicated number (64 here; the tree has
  moved a few merges since that measurement).
- **10 (lens)**: a default-target run — without `--all-targets` the ~54
  test-target and duplicate findings never surface.
- **Vendored C++ noise**: NOT part of any count. The three `-sys` crates' Rust
  shims have **zero** clippy findings. The only vendored noise in the stream is
  the `sigil-clownlzss-sys` build script relaying GCC `-Wmaybe-uninitialized`
  notes from `vendor/compressors/enigma.h` — C++ compiler output, not a
  rustc/clippy lint; it does not fail `-D warnings` and stays untouched per the
  lane's scope rule.

## The clean invocation

```
cargo clippy --workspace --all-targets --release -- -D warnings
```

exits 0 (default features, no exclusions). Caveats: the only remaining stderr is
the vendored-C++ note stream above. Release-profile coverage is total: the
workspace has no `#[cfg(debug_assertions)]`-gated code (only two `cfg!()` macro
uses in `sigil-link/src/relax.rs`, which compile — and lint — in both profiles).

## Fix census by lint id (64 findings, 30 files, +89/−115)

Mechanical (behavior-preserving on its face) — 49:

| lint | n | fix |
|---|---|---|
| `doc_lazy_continuation` | 27 | comment-only. Two root causes: a mid-sentence wrap landing the next line on `+` (or `1)`) which markdown reads as a list marker — rewrapped 6 doc blocks so no line starts with a marker; and a closing paragraph following a real list with no separator — inserted a blank `///`/`//!` line at 5 sites (`native.rs` ×3, `seam2.rs` ×2), which is the intended rendering anyway |
| `ptr_arg` | 7 | `&PathBuf` → `&Path` in private test helpers (`have_aeon`/`read_ref`/`build_chained`/`residue_status`); all call sites deref-coerce |
| `needless_range_loop` | 4 | the two `header_neutral_diffs` twins' zeroing loops → `buf[0x18e..0x190].fill(0)` / `buf[0x1a4..0x1a8].fill(0)` — identical half-open ranges |
| `question_mark` | 3 | `if let …  else { return None }` → `?` (two auto-applied; `symbols.rs:55` by hand). Pure lookups, identical branch order |
| `unused_imports` | 3 | dead `Path`/`Section` imports removed (auto) |
| `unnecessary_to_owned` | 2 | `.to_vec()` dropped where `&[u8]` coerces (auto) |
| `redundant_closure` | 1 | `.filter(\|e\| expr_has_sym(e))` → `.filter(expr_has_sym)` (auto) |
| `manual_contains` | 1 | `iter().any(\|r\| *r == reg)` → `contains(&reg)` (auto) |
| `useless_borrows_in_formatting` | 1 | `&out[0]…` → `out[0]…` in a format arg (auto) |
| `unnecessary_cast` | 1 | `plain_len as usize` on an already-`usize` field (auto) |
| `err_expect` | 1 | `.err().expect(…)` → `.expect_err(…)` (auto). Panic-path message shape differs (appends the Ok Debug); lens verified nothing asserts on that text |

Judged (listed with disposition, none silently chosen) — 15:

| lint | n | disposition |
|---|---|---|
| `type_complexity` | 7 | **type aliases**, pure type-level (term code unchanged): `RegSegs` (seam1 ×2 — the reglist segments), `AuthorityConstCache` (seam1 ×3 — the per-aeon-root memo statics), `pub type ProcBodyEnvResult` (eval/mod.rs — the 5-tuple return), `ResolvedRegions` (regions.rs). Chose aliases over `#[allow]` because they are provably neutral AND name the shapes |
| `dead_code` | 3 | **deleted** — all three provably unreferenced (rustc + lens grep): `SrCover::any` (lower/proc.rs — no caller survives), `asm_touchers_in_file` (parcel_8b test — its subject matter, `.asm` corpus files, no longer exists post-K-capstone), `lower` helper (struct_field test — the tests call `lower_module` directly) |
| `too_many_arguments` | 2 | **scoped `#[allow]`** + present-tense justification: `one_pass_with_defer` (frontend-as — one param per pass-to-pass seed table, by design) and `collect_nodes` (seam1 — one param per recursion-threaded accumulator) |
| `assertions_on_constants` | 1 | **scoped `#[allow]`** on the test fn `children_region_pins_share_both_anchors` + justification: the operands are repin-generated pin constants — a live contract on the generated table, not a tautology. (A statement-level allow on `assert!` is ignored — `unused_attributes` — so it sits on the fn) |

No crate-level blanket allows; `Cargo.toml` lint tables untouched.

## The `symbols.rs:55` origin verdict: CONFIRMED

`sigil-ir/src/symbols.rs:55` carries a real `clippy::question_mark` finding
under clippy 1.97 (the `match scope` in `SymbolTable::resolve`), and `sigil-ir`
sits early in the build graph, so it was the first surfaced finding — the
d-batch row's "masks a further backlog" claim is also confirmed
(`sigil-frontend-as`, `sigil-link`, `sigil-frontend-emp`, plus `sigil-harness`,
`sigil-cli`, and the ir crate itself). Fixed with the `?` rewrite; ledger row
closed.

## Bars

1. **Byte bar**: binaries rebuilt at HEAD post-fix; all SEVEN golden targets
   `cmp` OK in `capture_goldens.sh` order (s4 / s4.debug / demo / demo.debug /
   config_a→`s4.debug.bin` / config_b→`s4.bin` / lean→`s4.bin`), canonical
   s4.bin + s4.debug.bin rebuilt and re-verified afterwards. RC=0. (Baseline
   run at HEAD pre-edit was also green.)
2. **refreeze --check**: OK (tip `objtest-gate`, chain len 46). **repin**:
   `pins.rs unchanged`.
3. **Full strict** (foreground, no detached shell; no cargo running before
   start): `SIGIL_STRICT_GATE=1 SIGIL_EMIT=… AEON_DIR=….worktrees/b4 cargo test
   --workspace --release` — exit 0, **3317 passed / 0 failed / 4 ignored**,
   zero `FAILED`/`panicked` lines (failures-first sweep of the full log).
4. **Test delta**: `#[test]` count 3321 = the `25174b55` base exactly
   (3317+4). Delta 0 — the three dead-code deletions were helpers, not tests.
5. **Lens (C, hazard, read-only)**: verdict **CLEAN** over the whole commit —
   explicitly re-proved the `expect_err` panic-message question, the three `?`
   rewrites' control flow, the three deletions' unreferenced-ness (workspace
   grep), the `&Path` coercions, the aliases' character-identity, and the
   `fill` ranges. No findings.

## Loose ends

- `bytebar.sh` (worktree root) is an untracked seven-target byte-bar helper a
  prior porter left; it matches `capture_goldens.sh` order and was used for
  both byte-bar runs. Left untracked — not part of the parcel.
- The vendored `enigma.h` `-Wmaybe-uninitialized` C++ notes remain (scope rule:
  vendored code is not ours to style). If the noise ever bothers a gate, the
  shim's build script could pass `-Wno-maybe-uninitialized` — flagged only, not
  done.
