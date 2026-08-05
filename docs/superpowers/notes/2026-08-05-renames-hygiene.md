# 2026-08-05 — renames-hygiene: the three file renames + the packet-header sweep (close packet)

Status: Merge state lives in the campaign log, not here. Branch pair
`renames-hygiene` — worktrees `sigil/.worktrees/b3` + `aeon/.worktrees/b3`,
built off sigil `22e7274f` (chain 46) / aeon `77f80c6`, REBASED at merge prep
onto sigil `7eab683f` (post-B′-3b) / aeon `f3fc6a8` (docs-only), every bar
re-proven at the new base (§3).

Commits (rebased ids):
- **aeon `e8264fa`** — the three renames + generator + living docs. **A merge-order
  constraint: the sigil test commit below reads the renamed aeon paths, so the
  pair merges together (aeon-first, sigil immediately after; sigil-first breaks
  the corpus gates against an un-renamed aeon).**
- sigil `7599afdc` — test-side path ripple.
- sigil `e86c47dd` — the packet-header sweep + both ledger rows.
- sigil `2f9c5f31` + `64704de6` — this packet, and the sweep's rebase-time
  extension to the B′-3b packet.

## §1 — Part 1: the three file renames

`[module.path-mismatch]`'s contract: the id's last segment must equal the file
stem. The three ledger-ruled FILE renames landed:

| module | was | is |
|---|---|---|
| `engine.s4lz` | `engine/compression/s4lz_decompress.emp` | `engine/compression/s4lz.emp` (`git mv`) |
| `engine.zx0` | `engine/compression/zx0_decompress.emp` | `engine/compression/zx0.emp` (`git mv`) |
| `engine.compression_vectors` | `engine/debug/generated/vectors.emp` | `engine/debug/generated/compression_vectors.emp` (generator-emitted) |

**No map edit was needed, and that is a finding, not an omission:** module files
are discovered by the manifest's directory walk (`resolve/manifest.rs` `read_dir`
scan) and indexed by their `module` headers; `games/*/map.toml` drives PLACEMENT
by section head-label (`S4LZ_DecompressDict`, `ZX0_Decompress` — proc labels,
unchanged by a file rename). Nothing functional in either repo names these module
files by path except the sigil test suite (below).

**The `vectors.emp` generator finding (trap 2):** the generated island is
entirely gitignored, so this rename is not a `git mv` at all — the file's name
is owned by `tools/gen_compression_vectors.py` (`emp_path`, and it IS invoked by
`build.sh` line 126 every build, despite the island's "not regenerated" folklore
— only the MD Debugger blobs are static). The generator now emits
`compression_vectors.emp` and removes a stale `vectors.emp` alongside its
existing stale-`vectors.asm` cleanup, because the manifest scan walks the
directory: a leftover old file would be compiled as a duplicate module. No
generator was left writing a file the walk no longer wants.

Reference sites moved with the renames, all in the same two commits:

- aeon living docs: `docs/ENGINE_ARCHITECTURE.md` (both decompressor paths),
  `docs/LEVEL_EDITOR_SPEC.md` (a dead `engine/s4lz_decompress.asm` pointer →
  the real `engine/compression/s4lz.emp`), `tools/salvador/README.md` (dead
  `engine/zx0_decompress.asm` → `engine/compression/zx0.emp`).
- sigil tests (path joins + witness assertions + comments/assert messages):
  `crates/sigil-cli/tests/{s4lz_port,zx0_port,load_art_port,tile_cache_port,
  compression_selftest_port,slot_type_corpus,contract_closure_corpus}.rs`,
  `crates/sigil-harness/tests/repin_pins.rs` (comment). The
  `slot_type_corpus`/`contract_closure_corpus` generated-module witness
  (`ends_with(...)`) now names `compression_vectors.emp` — without that edit the
  define-free gates fail loudly against the renamed corpus, which is those
  witnesses doing their job.
- Dated notes/packets/plans/reviews keep the old names — they record what was
  true when written.

**Firings, measured per shape** (tally line, before → after): all SEVEN shapes
`module.path-mismatch 12 → 9` (s4 22→19, s4.debug 63→60, demo 22→19,
demo.debug 62→59, config_a 63→60, config_b 22→19, lean 21→18 total warnings —
the −3 is exactly this lint on every shape). `SIGIL_WARNINGS=full` enumerates
the 9 survivors: the eight OJZ act-suffixed modules (**stay OPEN** —
second-act-gated, ledger class (b), untouched here) plus
`games.sonic4.parallax_configs` in `data/parallax/configs.emp` (hand-authored,
unruled — noted in the ledger row). `warn_tier_corpus.rs` needed **no edit**,
verified not assumed: its frozen baseline pins the firing lint-id SET per shape,
and `module.path-mismatch` still fires on all seven.

## §2 — Part 2: the packet-header sweep

The adopted ruling (ledger row `[process, 2026-08-04]`, closed in `f48cb0bb`):
packets stop carrying merge-state claims entirely; the campaign log is the
single authority. Every status-header claim (MERGED / unmerged / NOT merged /
awaiting merge) now reads `Merge state lives in the campaign log, not here.`
Branch provenance ("built off X"), gate numbers, and time-scoped records
("masters at close were X" in the a1a2 arc-close and the campaign
retrospective) were left standing — facts about the work, not claims about
master. 33 files touched, all in `docs/superpowers/notes/`:

The ruled 2026-08-01/02 stale set (21):
`2026-08-01-conv-a-structs-flip.md` · `2026-08-01-conv-b-constants-tail.md` ·
`2026-08-01-conv-c-ram-ports.md` · `2026-08-01-conv-d-gated-twins.md` ·
`2026-08-01-conv-f-game-config.md` · `2026-08-01-conv-f2-sound-ids.md` ·
`2026-08-01-conv-g-parallax.md` · `2026-08-01-conv-h-game-data.md` ·
`2026-08-01-conv-hdemo.md` · `2026-08-01-conv-i-generators.md` ·
`2026-08-01-conv-i8-vectors.md` · `2026-08-01-item7a-regions-feature.md` ·
`2026-08-01-item7b-engine-ram-port.md` · `2026-08-01-item7c-game-ram-ports.md` ·
`2026-08-01-k5-order-drive.md` · `2026-08-01-sound-e1-flip.md` ·
`2026-08-01-sound-e2-mirror.md` · `2026-08-02-l1-p2-conversion.md` ·
`2026-08-02-l5-l8-type-layer.md` · `2026-08-02-l9-offsets-cross-module.md` ·
`2026-08-02-onesit-batch.md`

Later packets carrying claims — swept under the same ruling, an extension
beyond the brief's ~15 the overseer should eyeball (12): the four corrected in
place on 08-04, whose "countersigned and MERGED" lines were TRUE claims that
rot the same way (`2026-08-04-bprime-0-condout.md` ·
`2026-08-04-bprime-0b-survives-verifier.md` ·
`2026-08-04-bprime-0c-closure-soundness.md` · `2026-08-04-bprime-1-contexts.md`
· `2026-08-04-warning-tier.md`), plus `2026-08-04-bprime-2-stack-delta.md` ·
`2026-08-04-bprime-3a-cycle-budgets.md` · `2026-08-04-sr-contracts.md` ·
`2026-08-05-bprime-4-report-and-cc-precision.md` ·
`2026-08-05-define-free-gate-flip.md` · `2026-08-05-edge-return-falloff-split.md`
(several of these "NOT merged / unmerged" lines were ALREADY stale — the
class reproduced itself while the fix was in flight). At merge-prep rebase onto
`7eab683f` the sweep gained a 12th late file the same way:
`2026-08-05-bprime-3b-68k-cycle-table.md` landed on master mid-lane with a
"NOT merged, NOT pushed" header that was stale the moment its merge commit
existed — 33 files total.

Deliberately untouched: session handoffs and audits (narratives, not packet
status headers), `2026-08-02-a1a2-arc-close-packet.md` /
`2026-08-02-campaign-retrospective.md` (time-scoped "at close"/"at writing"
records), historical kill-list/ledger rows naming the old filenames inside
CLOSED entries.

## §3 — Gates

Run twice in full: at the original base (`22e7274f`/`77f80c6`) and again after
the merge-prep rebase (`7eab683f`/`f3fc6a8`, binaries rebuilt at the new base).

- **Byte bar (seven targets, derived from `golden/*.bin`, `capture_goldens.sh`
  order, `cmp`):** s4 · s4.debug · demo · demo.debug · config_a · config_b ·
  lean — **all seven IDENTICAL** to `crates/sigil-harness/golden/`, canonical
  s4.bin/s4.debug.bin rebuilt afterwards and re-`cmp`'d identical (nine
  comparisons), at BOTH bases. Also proven on the untouched baseline before the
  first edit (cheap re-confirm of the previous lane's proof).
- **`refreeze --check`:** OK, tip `objtest-gate`, chain len 46 — both bases.
- **`repin --check`:** pins.rs unchanged — both bases.
- **Strict suite** (`SIGIL_STRICT_GATE=1`, `AEON_DIR` = own worktree, full log,
  failures-first): original base **3261 passed / 0 failed / 4 ignored**, exit 0;
  rebased **3294 passed / 0 failed / 4 ignored**, exit 0 — the +33 is B′-3b's
  own tests arriving with the new master, not this lane's.
- **Test delta: exactly 0 at each base.** `#[test]` totals: 3265 at both
  `22e7274f` and the pre-rebase HEAD; 3298 at both `7eab683f` and the rebased
  HEAD. passed + ignored equals the total in both runs — nothing silently
  skipped.

## §4 — Lens panel (B, corpus-pattern, read-only)

**PASS — no live reference to any old name survives.** The lens swept both
worktrees for `s4lz_decompress` / `zx0_decompress` / `generated/vectors.emp` /
bare `vectors.emp` (excluding `target/`, `.git`): every live code/config/
generator/test surface points at the new names; every old-name hit is a dated
historical note/plan/spec (deliberately kept) or the DIFFERENT
`engine/system/vectors.emp` boot-vector module, which legitimately keeps its
name. Disk state confirmed: no stale old-named files anywhere;
`engine/debug/generated/` holds exactly one `.emp`, `compression_vectors.emp`.
Two borderline `.asm`-lineage provenance comments
(`tile_cache_port.rs:19`, `repin.toml:171`) name the long-deleted
`*_decompress.asm` donors — pre-existing, nothing tooling follows, out of this
parcel's `.emp` scope; left standing.

## §5 — Open

- The 8 OJZ act-suffixed `module.path-mismatch` survivors: **OPEN**, second-act
  gated (ledger class (b)).
- `games.sonic4.parallax_configs` in `data/parallax/configs.emp`: **OPEN**,
  unruled (rename the file or shorten the id segment).
