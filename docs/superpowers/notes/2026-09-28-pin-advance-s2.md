# PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS: sigil's reference pin, aeon ec640bcf to 1ee78b88

Branch `parcel/pin-advance-s2`, cut from master `ee4089f8`. No emulator was used.

## The reference tree

`scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-sigil-ref-1ee78b88 1ee78b88`
with `REF_BUILD_DEMO=1` (all four shapes BUILT, not copied), assembler built from this tree
(`sigil 0.1.0 (ee4089f8)`, closure-revision `88eb2e35`), target dir `.target-ref` in the
worktree. `PROVISION_END rc=0`. HEAD `1ee78b88b003da5ffbd8fa061eab7d56505c201a`, `git status`
clean (only ignored build products). zlib CRC-32 + size, recomputed with python `zlib.crc32`:

| file | CRC-32 / size |
|---|---|
| s4.bin | `50401674/845398` |
| s4.debug.bin | `198fd717/865722` |
| demo.bin | `2f9e2d4c/99994` |
| demo.debug.bin | `64717bac/106981` |

The two s4 figures EQUAL aeon's own claim (s4 `50401674/845398`, s4.debug `198fd717/865722`,
built on sigil pair `1173bb31`), although this assembler is `ee4089f8`, not `1173bb31`.

The jump is 2161 aeon commits (`git rev-list --count ec640bcf..1ee78b88`), not the handful the
queue row's heads-ups describe. `c06c786b` (parallax round 2) is an ancestor of the target.

`tools/test_extern_guard_reachability.py` IS PRESENT at `1ee78b88` (absent at `ec640bcf`), so
aeon's pytest lane can write into any tree holding their `tools/`. Reported, not acted on. The
provisioner builds with `NO_LINT=1`, which skips that lane.

## Baseline, before any change

- `refreeze --check`: OK, tip `link-zero-byte-move-placement`, chain len 206 (the two known
  DIVERGENT sigil revisions #181, #201). The chain is healthy, so the advance is a DECISION
  (aeon named the revision, hub steer), not a repair. Log: `refreeze-check-before.log`.
- `repin --check` against the new tree: rc 2, `region test_particle plain_anchor: symbol
  ObjDef_PathSwap not found in the listing symbol table`. aeon deleted the path-swap object
  (`2dc0e9da`). Log: `repin-check-before.log`.
- Full suite, `scripts/landing-run.sh --aeon <new tree>` (strict, `--no-fail-fast`, target
  `.target-ref`), at unmodified `ee4089f8`: 520 of 520 binaries launched and reported,
  5657 passed, 202 failed, 2 ignored, CARGO_EXIT 101, clippy 0, ledger 0. Wall clock 7m47s.
  Failing tests by binary: `baseline-failing.txt`; first panic line of each:
  `baseline-panics.txt` (a few lines there are `should_panic` tests that passed).

## Order of work, and why the freeze came before the test repairs

The brief ordered repairs before the refreeze. The freeze's outputs (goldens, size
tables, pins, provenance) are a function of the aeon tree and the assembler, not of
any test file, so it was run as soon as `repin` could resolve, and the post-freeze
suite became the real work list: it separates pin drift (which the freeze absorbs)
from repairs. No repair copied a value from the regenerated `pins.rs`; each was derived
first and only then compared (below).

## repin.toml at 1ee78b88 (commit `88a8399c`, plus `040fc3b9`, `12be6427`)

- The five debug-only object regions anchored at `ObjDef_PathSwap` now anchor at
  `DeformTable_Zero`, the next placement in both listings (aeon `2dc0e9da` deleted the
  path-swap object). Region `path_swap` and symbol `CrossoverTable` (aeon `19978b00`)
  removed.
- `soundbankhead`: `len = 0x630` became `end = "section:soundbankhead"`, the section's
  own `lma + image_len`, because the listing has no label after the head-tail pad (it
  rides `DacSampleTable`). Three derivations agree on **0x6B0** in both shapes: the five
  generated heads sum to 0x6AB (985 + 264 + 274 + 64 + 120) plus a 5 B pad to the 8-byte
  quantum; `DacSampleTable` VMA $8633 + 120 + 5 = $86B0; the next placement
  `Song_MovingTrucks` is at LMA $B86B0. **aeon's reading (0x6B0 counting the pad) is
  confirmed**, and no literal is needed: a head that grows moves the pin by itself.
- `mt_bank_blob`: `len = 0x34E8 / debug_len = 0x4F38` became `end = "section:mt_bank"`
  (mt_bank.emp is now the ROM-placed module); the pin is 0x4EB8 / 0x6900, equal to the
  generated `mt_bank_body.bin` 20152 B and `mt_bank_body_debug.bin` 26880 B.
- New `[[symbol]] Z80_Sound_Start` (0x3DE / 0x3E4): `seam1::blob_lma` was BootData + 54,
  a typed offset that went stale when the boot data ahead of the blob became 50 B.
- The first `--freeze` stopped in `derive_offcanonical_sizes.sh` on the committed
  `ObjDef_PathSwap` boundary row; the row was removed from the five size tables, the
  tools rebuilt (the tables are compiled in), and the freeze re-run whole. Both captures
  gave the same seven CRCs.

## The freeze (commit `040fc3b9`), entry `pin-advance-s2`, chain len 207

| target | CRC-32 / size (python zlib, recomputed) |
|---|---|
| s4 | `50401674/845398` (equals aeon's figure) |
| s4.debug | `198fd717/865722` (equals aeon's figure) |
| demo | `2f9e2d4c/99994` |
| demo.debug | `64717bac/106981` |
| config_a | `a7cfccbb/866074` |
| config_b | `d888a312/617813` |
| lean | `9f0d4de3/793376` |

After it, and at the branch tip: `repin --check` prints `pins.rs unchanged`;
`refreeze --check` OK, tip `pin-advance-s2`, chain len 207. The provenance entry is
FROZEN, not attested (`refreeze --attest` was not run; the controller decides).

Post-freeze strict suite: 520/520 binaries, 5807 passed, **52 failed**, 2 ignored
(`postfreeze-failing.txt`). Every one of the 52 was repaired below.

## Repairs, each derived from the tree under test

| commit | binaries | how |
|---|---|---|
| `c7f2ef03` | test_g4_final_objects_port, test_p1_player_port | path_swap and CrossoverTable deleted upstream; g4's control and doctored probe move to test_enemy's debug window |
| `9c155909` | tile_cache_port, vblank_port, game_loop_port, load_art_port, test_p2_player_states_port | new cross-seam names read from the reference listing (`extend_from_listing_names`, `listing_labels_if_defined`); structs.emp's `LL_*` imports lifted from constants.emp (`engine_constants_imported_by`); vblank.emp's own engine.constants imports as equs (`engine_constant_equs_imported_by`), retiring the typed VDP_CTRL |
| `ecb3e394` | mt_port, mt_negative_probes, sfx_negative_probes, seam2_mt_tables_optional, mt_bank_port, sound_fold_heads, sfx_bank_port, seam2_layout_derivation, seam2_dac_head_colink, seam2_sfx_head_colink | `seam2::mt_bank_embed_root` made pub and used for every mt_bank lower; the mt guard count counted from source (7 at ec640bcf, 0 at 1ee78b88); `DAC_SAMPLE_TAB_LEN = 127` replaced by `dac_sample_tab_len` (DAC_SAMPLE_COUNT x DacSample_len = 120); layout literals resynced from the measured heads |
| `12be6427` | seam1_native_link, boot_port, boot_data_port | blob corpus lengths 6176/6306 -> 6228/6358 (emitted bins and listing Z80_SOUND_SIZE agree); `blob_lma` from the new pin; config_b boot addresses from its frozen size table |
| `d64c9233` | native_full_rom | Ground_Move_Cap offset 0x330 in both shapes (plain's PlaySFX calls became bsr.w, read from the golden encodings); Z80_Sound_Start at BootData + 0x32 |
| `3a9b2839` | repin_pins | 19 hand-typed literals, each derived from the listings first, then compared: all agree with pins.rs |
| `a523772f` | contract_closure_corpus, build_anchor_overlay, anchor_overlay, m68k_roundtrip_stream, listing_equ_shape_aware, warn_tier_corpus | with-census 21 -> 22 (aeon `62cad7b7`); overlays move every sound-on anchor incl. song_bank_2; `exg` now emitted (aeon `2a727b29`); PAGE_FRAMES_CLAMP use sites counted from source (2); mt_bank.emp's module id pinned as an aeon finding |
| `5467b9e8` | (lint) | the listed-names consts had split two doc comments from their fns |

Three new derived checks were proven red-first with the mutation on disk (mt_port's
ensure count `n + 1`, `dac_sample_tab_len` `+ 7`, the use-site count `+ 1`), each
restored from a saved copy.

One test was RETIRED, not repaired: `mt_negative_probes` probe (b), "all five
co-residency ensures fire at a wrong bank". aeon deleted those ensures; a derived count
is 0 and the probe could only pass vacuously. The file header names what holds the
invariant now (section_align, `mt_bank_room`, `validate_sound_fold`, the no-straddle
property probe (a) exercises).

## The six heads-ups, as measured

| heads-up | measured |
|---|---|
| S2 envelope restatements | CONFIRMED: table 120 B with the pad in soundbankhead.emp; head 0x6B0; pitchtable 0xB83D9; the `$8357` prose removed |
| song id renumbering | no sigil test named an id by number; SONG_COUNT now sizes the tables in sfx_bank_blob.emp, so the SFX block length is shape-dependent (0x664 / 0x674) |
| Player_SensorLand | CONFIRMED, test_p2 only |
| LayerLine / Act size | Act moves absorbed by derived field equs; the `LL_*` constants reached the vblank flips through structs.emp |
| crossover removal | CONFIRMED in repin.toml, pins.rs, test_p1 |
| page-cache renames | tile_cache_port needed eight names, not the list aeon gave: Cache_H_Pfx_Run, PageIn_WaitIdle, PageCache_LiveReset, Page_Demand_Held, PageCache_DemandHoldTick, PageCache_Direct_Map, Page_Live_Masks, Page_Live_RowPtr |
| song-bank-2 probes | mt_port's count is 0, not the 2 the 2026-09-28 note predicted (the SONG_* drift guards moved to sfx_bank_blob.emp) |
| +20 B Z80 blob | the blob grew +52 B over the span (6176 -> 6228), +20 being one commit's share |

Also red and in no heads-up: the path-swap deletion (repin.toml, size tables, test_g4),
the boot data table 54 -> 50 B, song_bank_2's anchor under the overlay probes, `exg`,
the second PAGE_FRAMES_CLAMP site, the with-census, and mt_bank.emp's module id.

## For aeon

- `games/sonic4/data/sound/mt_bank.emp` declares `module games.sonic4.mt_bank_blob`:
  `[module.path-mismatch]` on a hand-written module in all seven shapes. No byte effect;
  sigil pins it at this revision and flags the row when it is renamed.
- `tools/test_extern_guard_reachability.py` is present at 1ee78b88. The refreeze's
  `capture_goldens.sh` builds with the lint lanes ON, so aeon's pytest lane ran inside
  this parcel's exclusive tree during both captures (observed as `pytest tools`
  processes whose cwd was the tree). The tree was `git status` clean afterwards.

## Final gate

`scripts/landing-run.sh --aeon .aeon-sigil-ref-1ee78b88 --baseline 5858` at `9b0a9d80`:
RESULT GREEN, 520 of 520 binaries launched and reported, 5858 passed, 0 failed,
2 ignored, 0 skip lines, clippy 0, ledger 0. The baseline run counted 5859 tests
(5657 + 202); the one fewer is the retired `mt_negative_probes` probe (b).

## For the controller

- Merging this branch moves the provenance tip to aeon 1ee78b88. Every suite run
  against `.aeon-sigil-ref` (ec640bcf) is red after that (`aeon_dir_matches_the_provenance_tip`,
  the goldens), and `scripts/provision-aeon-ref.sh` with no revision argument now
  provisions 1ee78b88. The standing reference has to move with the merge. Docs naming
  ec640bcf as the standing reference: `docs/OVERSEER-REFERENCE.md` (the
  `.aeon-sigil-ref` block, lines about 1231 and 1254) and `docs/QUEUE.md` (lines about
  351 and 1547). Not edited here.
- The tree used here, `/home/volence/sonic_hacks/.aeon-sigil-ref-1ee78b88`, is
  provisioned with all four shapes built and can serve as that reference.
