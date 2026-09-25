# PORT-TESTS-RED-AT-AEON-TIP: the port scopes, repaired against aeon 8a6f92c4

Branch `parcel/port-tests-aeon-tip`. Pin: aeon `ec640bcf` (provenance tail). Tip: aeon
`8a6f92c4` (aeon `origin/master` at dispatch, held fixed for the whole parcel). Trees
provisioned with `scripts/provision-aeon-ref.sh` at `.aeon-porttip-pin` (witness:
`repin --check` printed `pins.rs unchanged`) and `.aeon-porttip-tip` (no control exists
at a non-pinned revision; the provisioner built both s4 shapes, `950f7d28/822103` and
`a0e248e7/848768`). Tool built from this tree; `CARGO_TARGET_DIR` under
`.scratch/porttip/`. Logs: `/home/volence/sonic_hacks/.scratch/porttip/`.

## Verdict

All 26 measured failures plus the predicted `ojz_run_b_port` failure were scope-shaped,
with one exception: `test_objects_port` was a real harness bug. After the repair none of
the 14 port binaries fails on a name at the tip. Against pins REGENERATED at the tip (a
scratch measurement tree; the committed `pins.rs` untouched) 38 of the 40 tests are
byte-identical. The remaining two (`ojz_run_b_port`) fail on a stale per-shape delta
declaration, not on a name (open item 1).

Fixing the first name in a gate usually exposed more behind it, because the lower and
the link stop at the first error class. The 26 first messages were the front of each
queue, not the whole of it.

## Per-test table (SIGIL_STRICT_GATE=1, AEON_DIR named every run)

`tip-pins` = this branch (c170c14d) with `pins.rs` regenerated against the tip in the
scratch tree `.scratch/porttip/sigil-tippins`, the only run that says the derived scope
is RIGHT at the tip rather than merely past the name error.

| test binary | before @pin | before @tip (first failure) | after @pin | after @tip | tip-pins |
|---|---|---|---|---|---|
| act_descriptor_port | 2/2 | 0/2 names (`OJZ_Preset_Night`) | 2/2 | 0/2 pinned length | 2/2 |
| bg_port | 2/2 | 0/2 names (`BG_VSCROLL_*`) | 2/2 | 0/2 pinned length | 2/2 |
| bg_anim_port | 4/4 | 0/4 names (`BG_Bands_Hold`) | 4/4 | 0/4 pinned length | 4/4 |
| parallax_port | 2/2 | 0/2 names (`GAME_SCANLINE_CAPS`) | 2/2 | 0/2 pinned length | 2/2 |
| s4lz_port | 2/2 | 1 name (`CRASH_REPORT`), 1 bytes | 2/2 | 0/2 pinned length | 2/2 |
| load_art_port | 4/4 | 1 name (`CRASH_REPORT`), 3 bytes | 4/4 | 0/4 pinned length | 4/4 |
| tile_cache_port | 4/4 | 1 name (`CRASH_REPORT`), 3 bytes | 4/4 | 0/4 pinned length | 4/4 |
| dma_queue_port | 2/2 | 0/2 names (`DMA_Queue_End`, `MDDBG__ErrorHandler`) | 2/2 | 0/2 pinned length | 2/2 |
| dplc_port | 4/4 | 2 names (flips), 2 bytes | 4/4 | 0/4 pinned base | 4/4 |
| plane_buffer_port | 4/4 | 3 names (`Plane_Buffer_Peak`, `Region_Resolve`), 1 bytes | 4/4 | 0/4 pinned length | 4/4 |
| section_port | 2/2 | 0/2 names (`Region_Resolve`) | 2/2 | 0/2 pinned length | 2/2 |
| entity_window_port | 4/4 | 2 names (flips), 2 bytes | 4/4 | 0/4 pinned length | 4/4 |
| test_objects_port | 2/2 | 0/2 `embed.not-found`, doubled path | 2/2 | 0/2 colliding pins | 2/2 |
| ojz_run_b_port | 2/2 | 0/2 bytes (`sec_block_blobs`), the name was MASKED | 2/2 | 0/2 bytes | 0/2 declared shape delta |

Name failures before: 26, matching the impact note exactly, plus the two masked
`ojz_run_b_port` failures. After: 0.

The `after @tip` column was measured at 78f5c7ca, before the per-shape MDDBG and
act_descriptor length commit (c170c14d); the full suites and the `tip-pins` run below
are at c170c14d.

## What each gate needed, and the mechanism

Every supplied value is read from the tree under test.

- **Build defines** (`s4lz`, `load_art`, `tile_cache`, `parallax`, `bg`, and the section
  flips): `test_support::sonic4_shape_defines` lowers under `native::shape_defines`, the
  profile rows merged with the tree's own `games/sonic4/map.toml` `[defines]`. That is the
  env the whole-program build reads. `CRASH_REPORT` is a profile row the ports never
  bound. `GAME_SCANLINE_CAPS` is a map define the pin's map does not have.
- **Declaring modules as zero-byte ambients**: `bg` and `section` import consts from
  `engine.parallax` and `engine.bg`, so those modules ride along through `zero_byte_module`
  (`test_support::section_const_modules`). `test_objects` gets `games.sonic4.sound_ids`.
  An operand naming an unknown const lowers to a LINK fixup, not a lower error, so
  `section`'s `BG_STREAM_LEAD_ROWS` surfaced only at link time.
- **Listing-backed labels, only where the build defines them**:
  `test_support::listing_vma_if_defined` / `listing_labels_if_defined` (the BG streamer's
  cells and calls, `Region_Resolve`, `BG_Bands_Hold`, `Player_StartSpringFlip`). A
  missing listing stays a hard error. `section_streamer_labels_if_defined` spells
  section.emp's seam once for its three callers.
- **Family sweeps**: `Parallax_*` and `Region_*` RAM (parallax), `Plane_Buffer*` RAM,
  `OJZ_Preset_` (widened from `OJZ_Preset_Sec`), and `OJZ_Act1_BG_` from the current
  shape's listing (its debug test backgrounds exist in the debug ROM only). `parallax`
  also failed in the OTHER direction: aeon retired `Parallax_Prev_Sec_X/Y`, and the
  transcribed list named a symbol the tip listing no longer has.
- **`DMA_Queue_End`** via `listing_vma` (present at both revisions).
- **MDDBG entry points per shape**: `test_support::mddbg_entry_labels` reads
  `engine/debug/error_handler.emp`'s `pub equ ... = extern("ErrorHandlerBlob") [+ $DC6]`
  and resolves the blob in the shape's listing. The shared `pins::MDDBG_*` pair is one
  address, which holds only while the handler exists in the debug ROM alone. With
  `CRASH_REPORT` the plain ROM ships its own handler, and a plain operand from the shared
  pin encoded `jsr $000C0316` where the plain ROM has `$000BCAB8`. This commit's own
  first version made that mistake; the `tip-pins` run caught it. At the pin the helper
  reproduces the pinned pair exactly (C059E, C1364).
- **`act_descriptor` length per shape**: the gate took the plain length for both shapes.
  At the tip the descriptor is shape-dependent (plain 0x1F8, debug 0x246).
- **`ojz_run_b` OJZ_CLIP_ACT**: a prelude whose right-hand side is copied from the
  tree's `clip_act.emp` by `emp_const_rhs`, declaring nothing on a tree with no clip module.

## test_objects_port: a real bug, proven red-first

The port lowered `test_solid.emp` with `include_root = games/sonic4/objects` and no
`embed_base`, so an embed joined the MODULE directory. The build joins every embed to
the aeon ROOT (`native::build_emp`'s `embed_base_for`), and aeon spells embeds
root-relative. At the pin `test_solid.emp` embeds nothing, so the bug was latent until
the spring's art and palettes arrived. Proof, committed baseline e8952080: with the two
`LowerOptions` lines reverted on disk
(`include_root: Some(aeon.join("games/sonic4/objects"))`, `embed_base: None`) the tip
run reports
`[embed.not-found] cannot read .../.aeon-porttip-tip/games/sonic4/objects/games/sonic4/data/generated/spring/art_spring.bin`
(`.scratch/porttip/mutation-test_objects-tip.log`). Restored with
`git checkout -- crates/sigil-cli/tests/test_objects_port.rs`, `git status` clean after.

## The masking finding (ojz_run_b)

`ojz_run_b_port` compiled each of its four sections only when the comparison loop reached
it. At the tip the first section (`sec_block_blobs`) fails on bytes, so `act_assets.emp`
was never lowered and the predicted `unknown name OJZ_CLIP_ACT` could not appear. The
gate now lowers and links every section before comparing any, and the tip run then
reports the name (DEBUG shape only). A scratch probe of `ojz_run_a_port` in the same way
(not committed) found no hidden name there. Other multi-section gates among the 141 byte
failures were NOT probed for this masking.

## Full suites (`cargo test --release --workspace --no-fail-fast`, SIGIL_STRICT_GATE=1)

Launched = `Running` + `Doc-tests` headers. Reported = `test result` lines minus the two
nested results `eval_match` prints. Every log carries a `SUITE-END` marker and a stamp
line (pwd, HEAD, branch, aeon HEAD).

| run | sigil | AEON_DIR | launched | reported | passed | failed | ignored | log |
|---|---|---|---|---|---|---|---|---|
| branch @ pin | c170c14d | `.aeon-porttip-pin` (ec640bcf) | 504 | 504 | 5727 | 1 | 2 | `suite-branch-pin.log` |
| branch @ tip | c170c14d | `.aeon-porttip-tip` (8a6f92c4) | 504 | 504 | 5556 | 172 | 2 | `suite-branch-tip.log` |
| master @ tip | 107b4387 | `.aeon-porttip-tip` (8a6f92c4) | 504 | 504 | 5556 | 172 | 2 | `suite-master-tip.log` |

- The pin failure is `m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`
  (`NO REFERENCE TREE IS NAMED`, no oracle-old reference named): environmental, the same
  failure the 481ac02e note's master control recorded.
- The failing SETS at the tip are identical (172 names, empty diff both directions). The
  set cannot shrink before a repin: each repaired test now reaches its byte comparison
  and fails there against the pin's values. What changed is the reason:
  `grep -cE 'unknown name|unresolved symbol|not defined in this link|embed.not-found|unresolved branch'`
  counts 26 lines in master@tip and 0 in branch@tip.
- Every one of the 172 tip failures is byte, length, golden, pin or provenance drift, or
  the environmental m1b row, by first message (`fails-branch-tip.txt`,
  `msgs-branch-tip.txt`). The non-byte-looking ones are drift too: `keystone_flip_relocation`
  and `native_declared_chain` index past a ROM of a different length,
  `sfx_bank_port`'s `PoisonError` follows an earlier panic in the same binary,
  `provenance_chain` reports the tree is past the frozen tip, `contract_closure_corpus`'s
  `with` census moved, and `listing_equ_shape_aware` finds two `cmpi.w #imm, d6` in a
  `Level_LoadArt` aeon grew.

## Open

1. `ojz_run_b_port` against tip-regenerated pins: `ojz_act_assets plain/debug spans differ
   by 0x5ec2, more than the 0xf this section declares`. At the tip `act_assets.emp`
   ships three DEBUG-only test backgrounds behind `OJZ_DEBUG_TEST_BGS`: a tall layout
   (96 * 64 * 2 = 12288), a showcase layout (`BG_LAYOUT_SIZE` = 8192) and showcase tiles
   (2 + 118 * 32 = 3778), 24258 = 0x5EC2 bytes in all, exactly the measured delta. The
   gate's message forbids raising the bound to fit. The fix is to derive the delta from
   the module (for example, lower it in both shapes and take the length difference, or
   read the gated sizes, the `bg_anim_view_bytes` precedent). That is a decision about
   what the gate asserts, and it was left for whoever advances the pin.
2. Masking in the other multi-section gates among the 141 byte failures is unprobed.
