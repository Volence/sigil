# MODULE-REGISTRY-HARDCODES-AEON-FILES: enumeration and what was done

Parcel branch `parcel/module-registry-derive`. The build's module roots (the synthetic
entry's `use` lines) now derive from `games/<g>/map.toml`'s `order` over the scanned tree
(`crates/sigil-harness/src/module_roots.rs`). Deleting a game module is the file plus its
`order` row; adding one is the file plus its row.

## How a row names its module

* label row: the in-scope module (`engine.*` + `games.<g>.*`) defining a `data` / `proc` /
  `offsets` / `dispatch` / `script` / `table` of that name, `pub` or not. Private heads are
  real: `Collected_Init`, `Climb_WallDist` and the `Ani_*` offsets tables are private items
  the pinned map names. A pub-only index orphans 7 pinned rows.
* `section:<name>` row: the module declaring the section (`module … in <name>` or a nested
  `section <name> { … }`).
* compiler-minted `__align$<module>$N`: the module between the first two `$`.
* A label row nothing defines: `[map.order-orphan]`, named. A `section:` row nothing
  declares is left to the existing `[map.order-unknown-section]` placement refusal, so one
  mistake gets one diagnostic. Two definitions: the one the other roots' `use` closure
  reaches, else `[map.order-ambiguous]`.

## Enumeration: hard-coded aeon module / section / anchor / file lists

| Site | What it hard-codes | Class | What I did |
|---|---|---|---|
| `native.rs` `registry()` (~98 `m!` rows) | every placed sonic4 + engine module and its section | BUILD PATH | **Deleted.** Derived from `map.toml` `order` (`module_roots::derive_module_roots`). |
| `native.rs` `demo_registry()` | the `engine.*` filter, minus `engine.sound_api`, plus `z80_init` and 4 `games.demo.*` modules | BUILD PATH | **Deleted.** The demo map's own `order` supplies them; game scoping keeps sonic4's twins out. |
| `config_b_profile` filter (5 sound modules) + `z80_init` push | sound-off module set | BUILD PATH | **Deleted.** Now the `SoundOn` / `SoundOff` shape gates. |
| `config_a_profile` pushes (`game_debug`, `sound_debug`) | Config-A dev modules | BUILD PATH | **Deleted.** Now the `SoundDebugHotkeys` / `SoundDbgMirror` gates (read from the profile's `emp_defines`). |
| `registry()`'s `if debug` / `if debug \|\| crash_report` / `else` arms | which modules only debug / crash-report / lean shapes place | BUILD PATH | **RESIDUE, kept in sigil, named:** `module_roots::SHAPE_GATES` (21 rows). The map's `order` is the union over shapes and cannot say which rows a shape drops; these modules emit unconditionally, so their source cannot either. A gate row is a filter: deleting a gated module needs no sigil change; adding or renaming one does. **Proposed home:** a per-row `when` on `map.toml` `order` (the vocabulary `[[anchor]]`/`[[hole]]` already use), owned by aeon. |
| `engine.epilogue` (the `EndOfRom` terminus) | rooted by `registry()` in every shape | BUILD PATH | **RESIDUE, kept, named:** `module_roots::ENGINE_TERMINUS`. The sonic4 map has an `EndOfRom` row; the demo map does not (its header excludes zero-byte markers), and nothing `use`s the module. Rooted as an engine seed beside `engine.game_contract` / `engine.ram`. **Proposed home:** an `EndOfRom` row in the demo map, which would retire the constant. |
| `games.sonic4.sigil_objroutine_probe` | (new) sigil's own negative-probe fixture | BUILD PATH | **Excluded by id** (`SIGIL_PROBE_FIXTURES`). It defines `TestSolid_Init`, the live `test_solid` head, by construction, and neither copy is `use`d (ObjDef_Solid names it by string), so the use graph cannot pick. This names sigil's fixture, not a game module list. |
| `native.rs` `synthetic_entry_src` seeds `engine.game_contract`, `engine.ram` + profile `game_ram_module`, `manifest_module` | the entry's zero-byte contract/RAM modules | BUILD PATH | Kept: engine/game contract seeds, one per game, not a per-module list. |
| `native.rs` `build_emp` `COMPTIME_HELPERS` (14 engine helper ids) | pure-comptime engine helper modules the import rewrite normalizes | BUILD PATH | Kept: engine vocabulary, not game content. Not measured what a deleted helper does to the build. |
| `section_align.rs` `DECLARED` (incl. `d("ObjDef_PathSwap", 2, WORD)`) | per-head-label alignment requirement | BUILD PATH | **Kept, does not block deletion:** rows are looked up by the live section's head label only, so a row for a deleted section is inert (measured: the path_swap-deleted tree builds with the row present). It DOES block an ADDITION: a new section with no row is refused `[layout.undeclared-alignment]` by design (R7: "an undeclared section is refused, never given 1, 2, 16"). Deriving it conflicts with that ruling; **proposed home** for the per-section requirement: the module source (`.emp` already has item-level `align:`) or a map row, with WORD stated once as the 68000 default. Needs a ruling, not a parcel call. |
| `golden/offcanonical_sizes/*.txt` (frozen provisional-base tables, incl. an `ObjDef_PathSwap` row) | per-label provisional bases, read at build time | BUILD PATH, pinned-corpus facts | Not touched (golden). Measured inert for a deleted label: the path_swap-deleted tree builds with the rows present. Tracked separately as INSTALLED-BINARY-READS-BUILD-TREE. |
| `native.rs` profile paths (`game_root_rel`, `game_constants_rel`, `game_sound_ids_rel`, `game_sfx_bank_rel`) and game-config `emp_defines` values | per-game config files and values | BUILD PATH | Kept: per-game wiring, not a per-module list. |
| `seam2.rs` (`games/sonic4/data/sound/...`, `SOUND_PLACEMENT_MAP_REL`, `SOUND_IDS_REL`), `sound_bank_ids.rs` labels (`Z80_Sound_Start`, `DacSampleTable`, `Dac_Temp_Blip`, ...), `ERROR_HANDLER_BLOB_LABEL` | the sound seam's and MDDBG's contract labels and files | BUILD PATH | Kept: labels the build validates as a contract; deleting one is a contract change, not a module deletion. |
| `native.rs` `relocate_fixture_pool` `GROWABLE` | 3 OJZ pool head labels | BUILD PATH, stress-art fixture profile only | Kept: fixture-only waiver, never a shipped shape. |
| `pins.rs` `PATH_SWAP`, `repin.toml` path_swap region + 5 `plain_anchor = "ObjDef_PathSwap"` rows | the pinned tree's placement | PINNED-TEST | Not touched (owned by the landing lane); move with PIN-ADVANCE. |
| `tests/test_g4_final_objects_port.rs` (33 path_swap references), `ojz_run_a_port.rs` (`pins::PATH_SWAP`) | the pinned tree's path_swap bytes | PINNED-TEST | Not touched; move with PIN-ADVANCE. |
| `test_support.rs` act-descriptor file list, comment-only mentions in `diag.rs`/`ast.rs`/`diag_desugar.rs`/`diag_assert_vector.rs`/`scene_registry_port.rs` | test fixtures, prose | TEST / prose | Not touched. |

## Plumbing

`GameProfile.registry` is gone. `build_emp` derives from the manifest it already scans and
returns the list (`EmpProgram` / `ChainedResolve` / `RomBuild` `.registry`);
`validate_placement` (hole filler) and `declares_error_handler_island` read it;
`native::module_registry(aeon, profile)` serves callers that do not lower (cycle_fraction,
tests). `ModuleSpec` fields are owned `String`s.

## Measured

* Root ORDER moves no byte: map-order roots rebuild the pinned tree at the provenance tip.
* Derived list vs the deleted `registry()` at aeon `ec640bcf`, all seven shipped shapes
  (old list recomputed from master `7107ab15`'s source): nothing only in the old list; the
  derived list adds exactly three rows in every sonic4-rooted shape
  (`games.sonic4.ojz_effects_editor_act1`, `games.sonic4.player_instashield`,
  `games.sonic4.ring_sparkle`), modules the old list already reached through `use`; demo
  plain/debug are equal (46/47 specs). Already lowered, so no byte moves.
