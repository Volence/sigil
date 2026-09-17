# Struct size always checked (d-32): reach and cost, measured

The owner's ruling d-32 (`always-check`): if a file is compiled at all, every record in it that
declares a size gets checked; files never compiled stay untouched. Implemented as a pass in
`lower_module_inner` (`crates/sigil-frontend-emp/src/lower/mod.rs`) calling
`layout::validate_declared_struct_sizes`, which forces the layout of every struct the module
itself declares with `(size: N)`.

## Binaries

- OLD: `git archive` of sigil master `91993841`, built into a scratch target dir.
- NEW: branch `parcel/struct-size-always`, built from the worktree into a scratch target dir.

## Probe arms (`scripts/probe_struct_size_closure.sh`)

Count of "declared size 99" diagnostics per arm; blank means silent, exit 0.

| arm | shape | OLD | NEW |
|---|---|---|---|
| A | module outside the use closure | | |
| B | control: name-list import + field deref | 1 | 1 |
| C | blank `use lib.types._`, struct unnamed | | 1 |
| D | module in the closure for a different name | | 1 |
| E | struct imported by name, never used | | 1 |
| F | only `sizeof(Sst)` | 1 | 1 |
| G | nested field type of a used struct | 1 | 1 |
| H | `pub vars` overlay on it | 1 | 1 |
| I | built entry does not reach the module | | |
| J | an intermediary imports it | | 1 |
| K | deref in a proc nothing calls | 1 | 1 |
| L | the entry module declares it, unreferenced | | 1 |
| M | control: entry declares and derefs | 1 | 1 |
| N, O, P, Q | hand-written `ensure(sizeof(Sst) == 99)` | 1 each | 1 each |
| R | struct literal in a `data` item | 1 | 1 |

A and I stay silent: in both the struct's module is outside the built entry's closure.

## Reference tree, aeon `ec640bcf` (`.aeon-sigil-ref`)

`sigil build --aeon <tree> --game <g> [--debug]`, `SIGIL_WARNINGS=full`. CRC32 is zlib.

| shape | OLD exit | NEW exit | OLD crc/size | NEW crc/size | provenance tip | stderr OLD vs NEW |
|---|---|---|---|---|---|---|
| sonic4 | 0 | 0 | 91c46c94 / 820209 | 91c46c94 / 820209 | 91c46c94 / 820209 | identical |
| sonic4 debug | 0 | 0 | 8a378de6 / 846509 | 8a378de6 / 846509 | 8a378de6 / 846509 | identical |
| demo | 0 | 0 | 1c7a34d3 / 96863 | 1c7a34d3 / 96863 | 1c7a34d3 / 96863 | identical |
| demo debug | 0 | 0 | 72e405a5 / 103185 | 72e405a5 / 103185 | 72e405a5 / 103185 | identical |

Zero new refusals, zero new diagnostics of any level, byte-identical ROMs.

### Mutation census on a copy of that tree (plain builds)

Each sized declaration mutated alone, the line quoted from disk before the build, restored from
the reference tree after. Figures are the count of stderr lines naming "declared size".

| declaration | mutated to | sonic4 OLD | sonic4 NEW | demo OLD | demo NEW |
|---|---|---|---|---|---|
| `children.emp:82` SpawnDesc | `(size: 41)` | exit 0, 0 | **exit 1, 1** | exit 0, 0 | **exit 1, 1** |
| `preset.emp:57` EffectsPreset | `(size: 461)` | exit 1, 2 | exit 1, 2 | exit 1, 2 | exit 1, 2 |
| `entity_window.emp:57` EntityScanState | `(size: $1B)` | exit 1, 1 | exit 1, 1 | exit 1, 1 | exit 1, 1 |
| `parallax.emp:201` band_ext | `(size: 101)` | exit 1, 14 | exit 1, 14 | exit 1, 14 | exit 1, 14 |
| `parallax.emp:245` band_curve | `(size: 101)` | exit 1, 14 | exit 1, 14 | exit 1, 14 | exit 1, 14 |
| `parallax.emp:342` band_drift | `(size: 41)` | exit 1, 14 | exit 1, 14 | exit 1, 14 | exit 1, 14 |
| `parallax.emp:418` band_remap | `(size: 81)` | exit 1, 14 | exit 1, 14 | exit 1, 14 | exit 1, 14 |
| `parallax.emp:460` band_record | expression `+ 1` | exit 1, 11 | exit 1, 11 | exit 1, 11 | exit 1, 11 |
| `sst.emp:26` Sst | `(size: $51)` | exit 1, 1 | exit 1, 1 | exit 1, 1 | exit 1, 1 |
| `sst.emp:128` ObjDef | `(size: 261)` | exit 1, 1 | exit 1, 1 | exit 1, 1 | exit 1, 1 |

The positive control: SpawnDesc at 41 builds clean on OLD and is refused on NEW with
`engine/objects/children.emp:82:29: [Error] struct SpawnDesc: declared size 41 but fields total 4`.
Every row that already fired fires with the same line count, so the pass adds no duplicate.

## aeon tip `83ec56d2` (origin/master, 1045 commits past `ec640bcf`)

Detached worktree, removed afterwards. The debug shapes need gitignored artifacts a bare worktree
lacks (`engine.compression_vectors`, `engine/debug/generated/*.bin`); they were produced in the
worktree with aeon's own generator (`make -C tools/salvador`, then
`python3 tools/gen_compression_vectors.py`), after which all four shapes built.

| shape | OLD exit | NEW exit | OLD crc/size | NEW crc/size | stderr OLD vs NEW |
|---|---|---|---|---|---|
| sonic4 | 0 | 0 | f9156c30 / 821499 | f9156c30 / 821499 | identical |
| sonic4 debug | 0 | 0 | 8d3d92a0 / 848095 | 8d3d92a0 / 848095 | identical |
| demo | 0 | 0 | 8c9109cb / 98188 | 8c9109cb / 98188 | identical |
| demo debug | 0 | 0 | d82a0f9e / 104725 | d82a0f9e / 104725 | identical |

Zero refusals at aeon tip. The tip carries 12 sized declarations (the 10 above plus
`games/sonic4/objects/test_solid.emp:602` SpringDir and `engine/structs.emp:132` Region). Mutation
census at the tip, same method: SpawnDesc (now `children.emp:84`) is again the only one OLD misses
(sonic4 and demo both exit 0 on OLD, exit 1 with one diagnostic on NEW). SpringDir fires on both
binaries for sonic4 and on neither for demo, whose closure does not contain the sonic4 game module.
Region, EffectsPreset, EntityScanState, the five band structs, Sst and ObjDef fire identically on
both.

## Per-build cost

`sigil build --aeon .aeon-sigil-ref --game sonic4`, OLD and NEW interleaved, 10 runs each.

| binary | runs | median | min | max |
|---|---|---|---|---|
| OLD | 10 | 2.479 s | 2.339 s | 4.557 s |
| NEW | 10 | 2.418 s | 2.347 s | 3.122 s |

`uptime` load average ranged 7.50 to 11.40 across the runs (16 cores). The difference is inside
run-to-run noise; no cost is measurable at this resolution. A module with no sized struct returns
before building an evaluator.

## Structs that cannot be checked standalone

None found on the engine. The one class identified is a CLONE of an imported struct inside an
importer's synthetic file, whose field types need not resolve in the importer's scope. The pass
checks a struct only in its home module (span source equal to the module's), which is also where
its field types are in scope. Removing that filter makes a correct `Outer (size: 4)` imported
alone fail with `unknown type: Inner` plus a bogus `declared size 4 but fields total 2`; the test
`declared_size_is_checked_in_the_home_module_not_in_an_importer` pins it.

## The other checks that ride along, and the two in-repo fixtures they caught

Forcing a layout runs every declaration check `layout_of_struct` owns (size, `@offset`,
`(align:)`, pad checks, the `[layout.odd-field]` warning), so a sized struct nothing uses now gets
all of them. Engine reach of that: zero new diagnostics of any level at `ec640bcf` and at
`83ec56d2` (stderr diffed whole). In this repo's own suite it caught two test fixtures:

- `tests/script.rs` `comptime_call_inside_script_expands` declared `struct S (size: $24)` over 34
  bytes of fields, a silent wrong size of exactly the class d-32 closes. Corrected to `$22`.
- `tests/overlay.rs` `bare_window_scan_does_not_validate_unrelated_structs` used a sized decoy
  whose odd-offset word now draws the odd-field warning. The decoy lost its `(size: 3)` so the test
  still isolates the bare-window scan, and a counterpart pins that the sized form is checked once.

## Full suite

`scripts/landing-run.sh` with `.aeon-sigil-ref` @ `ec640bcf`, all four ROMs present.

| tree | suites | passed | failed | ignored | skips | clippy | ledger | result |
|---|---|---|---|---|---|---|---|---|
| master `91993841` (detached worktree) | 490 | 5563 | 0 | 2 | 0 | 0 | 0 | GREEN |
| branch `18c76c98` | 490 | 5574 | 0 | 2 | 0 | 0 | 0 | GREEN, 5563 + 11 new |

Passing-test name sets differ by exactly the 11 added tests; none removed.
