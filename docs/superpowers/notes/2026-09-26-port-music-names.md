# PORT-TESTS-MUSIC-NAMES-AT-TIP: region music names, repaired against aeon d7103d30

Branch `parcel/port-music-names`. Pin: aeon `ec640bcf` (`.aeon-sigil-ref`, the provenance
`aeon_rev`). Tip: aeon `d7103d30` (aeon `origin/master` at dispatch, the Sonic 2 envelopes
landed there), held fixed for the whole parcel, provisioned at `.aeon-music-tip`. Every run
passed `AEON_DIR` and `SIGIL_STRICT_GATE=1`; `CARGO_TARGET_DIR` under
`.scratch/port-music-names/`, where every log named here lives.

## Verdict

Thirteen tests stopped on a name at the tip, not twelve. Twelve are the visible
`Music_Service` / `Music_Want` lines the previous note listed. The thirteenth,
`tranche5_negative_probes::misspelled_extern_slot_is_loud`, failed with a bare
`control: the undoctored source must resolve against the truth`: its `resolves()` returned a
bool, so the name behind it (`Music_Want`) never reached a message. It failed the same way at
`9caa1368` (the previous parcel's failing set carries it), where the 12-line grep could not see
it. Its control now returns and prints the messages, proven red-first below.

After the repair no test fails on a name at the tip (12 name-shaped lines before, 0 after, and
the hidden one now passes). Against pins regenerated at the tip (scratch copy, committed
`pins.rs` untouched) all 15 tests in the five repaired binaries pass.

## Per-test table (SIGIL_STRICT_GATE=1, AEON_DIR named every run)

`before` = `8c6ed20e`, `after` = `9d38e63d`. `tip-pins` = `9d38e63d` exported by `git archive`
to `.scratch/port-music-names/sigil-tippins`, with `pins.rs` regenerated there against the tip
(`repin --harness-root <copy> --aeon <tip>`: `wrote .../sigil-tippins/.../pins.rs`, 449
added lines in its diff against the committed file), target `target-tippins` (`tippins.log`).

| test binary | before @pin | before @tip (first failure) | after @pin | after @tip | tip-pins |
|---|---|---|---|---|---|
| game_loop_port | 4/4 | 0/4 names (`Music_Service`, unresolved branch) | 4/4 | 0/4 pinned length | 4/4 |
| game_debug_port | 2/2 | 1/2, flip: name (`Music_Service`) | 2/2 | 2/2 (flip link stage now measured: green) | 2/2 |
| parallax_port | 2/2 | 0/2 names (`Music_Want`, then `Music_Current` behind it) | 2/2 | 0/2 pinned length | 2/2 |
| sound_api_port | 2/2 | 0/2 names (`Music_Want`) | 2/2 | 0/2 pinned length | 2/2 |
| tranche5_negative_probes | 5/5 | 1/5: 3 names (`Music_Service` x2, `Music_Want`), 1 bare control hiding `Music_Want` | 5/5 | 5/5 | 5/5 |

`after @tip` failures are pinned-length drift, and the regenerated pins show why: `GAME_LOOP`
0x1C/0x1E -> 0x20/0x24, `PARALLAX` 0xAD8/0xBB8 -> 0xB4C/0xC3E, `SOUND_API` 0x2B0/0x45A ->
0x2D6/0x480, each exactly the candidate length the committed-pins run reported.

## What each gate needed, and the mechanism

- **`Music_Service`** (game_loop's `jbsr` under `SOUND_DRIVER_ENABLED`): `game_loop_port`
  reads it from each shape's listing (`listing_labels_if_defined`, helper
  `sound_service_labels`) in both reference gates and both flips. A pc-relative `bsr.w`, so
  the address is load-bearing: plain `$8B88`, debug `$C22A` at the tip; at the pin the
  listing has no row and nothing is declared.
- **`Music_Want`, `Music_Current`** (engine.ram cells): `parallax_port` and `sound_api_port`
  read both from the shape's listing. `parallax` stopped on `Music_Want` first; supplying it
  exposed `Music_Current` (`clr.b Music_Current` in the crossing), the next name in the queue.
- **`game_debug_port` flip**: this test compares no ROM bytes and every one of its callee
  carriers is harness-private (`$900`, `$920`, ...), so `Music_Service` joins them at `$980`
  rather than reading a listing the test never needed before. Unreferenced at the pin, which
  is inert. Its link stage, unmeasurable at `9caa1368`, is green at `d7103d30`.
- **`tranche5` probes**: `Music_Service` joins the game_loop probes' synthetic scopes and
  `Music_Want`/`Music_Current` the sound_api truths, at harness-private positions (the file's
  convention: presence is the point, bytes are not compared).
- **`drain_define_is_load_bearing`**: behind the name sat a LITERAL, `emitted(1) - emitted(0)
  == 4`. At the tip the delta is 8, because game_loop gates two calls on the define
  (`Sound_DrainSfxRing` and `Music_Service`). A repin would not absorb that, so it is derived:
  `sound_gated_calls` scans `game_loop.emp` for the `jbsr` lines inside
  `if SOUND_DRIVER_ENABLED == 1 { }` blocks and the probe expects 4 bytes per call. It must
  find `Sound_DrainSfxRing` or it refuses as unmeasured. Measured: the literal failed at the
  tip (`left: 8, right: 4`, `six-step2-tip.log`); the derived form passes at both revisions
  (one gated call at the pin, two at the tip).
- **`misspelled_extern_slot_is_loud`**: `resolves()` returns the error messages; the control
  panics with them, and the doctored run must name `SND_REQ_MUSICC` rather than fail for any
  reason.

## Red-first: the bool control hid its name

Runner: `redfirst.sh` (scratch), `cargo test -p sigil-cli --test tranche5_negative_probes
misspelled_extern_slot_is_loud` at the pin. Mutation (`mutate_truth.py`): in
`sound_api_truth_sections` only, the label `Sfx_Ring_Rd:` renamed to
`Sfx_Ring_Rd_MUTATED_AWAY:`, so the undoctored composition carries one real unresolved name.
Expected from source: the old control panics without the name, the new one names
`Sfx_Ring_Rd`.

- Old structure: `git checkout 8c6ed20e -- <file>` (staged, `git diff --cached --stat` shown,
  95 lines), mutation quoted back from disk at line 390. Result: FAILED, message
  `control: the undoctored source must resolve against the truth`, no name
  (`redfirst-old.log`).
- New structure (`9d38e63d`), same mutation at line 435: FAILED, message names
  ``symbol `Sfx_Ring_Rd` not defined in this link`` (`redfirst-new.log`).

Each run restored with `git checkout HEAD -- <file>`; `git status --porcelain` empty after.
At the real tip, the same change (before the `Music_*` carriers were added) reported
``symbol `Music_Want` not defined in this link`` (`six-step1-tip.log`).

## Full suites (`cargo test --release --workspace --no-fail-fast`)

Launched = `Running` + `Doc-tests` headers; reported = `test result:` lines. Every log is
stamped (pwd, HEAD, branch, dirty count, AEON_DIR and its HEAD) and ends in `SUITE-END`.
`ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`.

| run | sigil | AEON_DIR | launched | reported | passed | failed | ignored | log |
|---|---|---|---|---|---|---|---|---|
| before @pin | `8c6ed20e` | pin `ec640bcf` | 504 | 504 | 5732 | 0 | 2 | `suite-before-pin.log` |
| before @tip | `8c6ed20e` | tip `d7103d30` | 504 | 504 | 5541 | 191 | 2 | `suite-before-tip.log` |
| after @pin | `9d38e63d` | pin `ec640bcf` | 504 | 504 | 5732 | 0 | 2 | `suite-after-pin.log` |
| after @tip | `9d38e63d` | tip `d7103d30` | 504 | 504 | 5546 | 186 | 2 | `suite-after-tip.log` |

- Name-shaped lines at the tip (`grep -cE 'unknown name|unresolved symbol|not defined in this
  link|embed.not-found|unresolved branch'`): 12 before, 0 after.
- Failing set: 191 -> 186. Five left it (`game_debug` flip and four `tranche5` probes); the
  eight `game_loop`/`parallax`/`sound_api` tests stayed in it with a new reason (pinned length).
  Every other failing test reports a byte-identical first message before and after.
- 191 at `d7103d30` vs 176 at `9caa1368`: the 15 new failures are all sound (see the
  PIN-ADVANCE measurement below), none name-shaped.
- `repin --check` at the pin after: `pins.rs unchanged` (`repin-check-pin.log`).

## The tip tree

`scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-music-tip d7103d30`, tool built
from this tree (`sigil 0.1.0 (8c6ed20e)`, closure-revision `79c1cd94`), `PROVISION-END rc=0`,
default demo handling (`REF_BUILD_DEMO` unset: the demo pair is the pin's goldens, copied).
No `repin --check` control exists at a non-pinned revision; what shows the tree is right is
that its HEAD is `d7103d30`, `git status` is clean, and the provisioner built both s4 shapes
and both listings from it (the listings carry `Music_Service`, `Music_Want`, `Music_Current`,
which the pin's do not). zlib CRC-32 + size, recomputed independently:

| file | CRC-32 / size | origin |
|---|---|---|
| s4.bin | `a22f69de/822288` | BUILT at `d7103d30` |
| s4.debug.bin | `575f38e4/848945` | BUILT at `d7103d30` |
| demo.bin | `1c7a34d3/96863` | the pin's golden, copied |
| demo.debug.bin | `72e405a5/103185` | the pin's golden, copied |

## PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS, measured at d7103d30 (measure only, nothing fixed)

Each item run with committed pins (`suite-before-tip.log`) and against tip-regenerated pins
(`tippins-s2.log`), so "a repin absorbs it" is measured, not inferred.

1. **`seam2.rs` `DAC_SAMPLE_TAB_LEN = 127` / `seam2_dac_head_colink`**: FAILS,
   `colinked_dac_head_matches_the_reference_rom_slice_both_shapes`: `DacSampleTable is 10 ×
   12 + the head-tail align pad = 127 bytes`, `left: 120, right: 127`. Still red with tip
   pins. The 127 is a sigil-side literal (not `pins.rs`), DERIVABLE from the tree. Behind it,
   the slice compare reads the committed golden at the tip-derived LMA, which is
   golden-frozen until the refreeze. The table shrank by 7 bytes, the DacHeadPad size sigil
   records; the row says the pad that moved is 8 bytes, which this parcel did not verify.
2. **`seam2_layout_derivation` (`pitchtable_lma 0xB8357` and later)**: FAILS,
   `sound_layout_derives_the_frozen_addresses`. Measured deltas: `pitchtable_lma`,
   `sfx_win_tab_lma`, `seq_opcode_tab_lma`, `dac_sample_tab_lma` +0x82; `mt_bank_lma`,
   `sfx_bank_lma_plain`, `sfx_bank_lma_debug` +0x80; `dac_blip`, `dac_shared`,
   `sound_tables_z80` unchanged. PIN-FROZEN by design: the test is a literal countersign of
   the derivation; its literals move by hand at the advance, from the move record.
3. **`pins::SOUNDBANKHEAD` length / `soundbankhead_port`**: FAILS, both shapes,
   `soundbankhead must emit 0x630 bytes ($8000..$8628)`, `left: 1712 (0x6B0), right: 1584
   (0x630)`. The head is 0x6B0, as the row says. A repin does NOT absorb it: `pins.rs` takes
   this length from a LITERAL in `repin.toml` (`[[region]] soundbankhead`, `len = 0x630`),
   and the tip-regenerated `pins.rs` still says 0x630 (still red in `tippins-s2.log`). The
   advance therefore needs a `repin.toml` edit (or re-expressing the row with an end symbol,
   which was not tried). Coordinated with the aeon lane; not touched here. The message's
   `$8628` is stale prose even at the pin (0x630 ends at `$8630`).
4. **"VMA $8357" comment in `seam2_pitchtable.rs`**: a comment cannot fail. The test itself
   FAILS: `pitchtable_matches_the_reference_rom_slice_both_shapes`, `differs from s4.bin @
   byte 0x0: emp 0x00 vs rom 0x3f`, still red with tip pins. Cause: the derived
   `pitchtable_lma` moved +0x82 to `0xB83D9` while the golden is the pin's. The same prose
   sits in `seam2.rs` (`placed at VMA $8357`). Golden-frozen; clears at the refreeze.
5. **`movingtrucks_pitchtable.emp` vma `$8357` -> `$8000`, emitted .bin unchanged**:
   CONFIRMED. The tip-built `engine/sound/generated/movingtrucks_pitchtable.bin` (264 bytes,
   written by the gates from `9d38e63d`) equals the pin tree's (`ec9b7b0e` both) and the
   golden slice at `0xB8357`. The built tip ROMs carry the same 264 bytes at `0xB83D9`.

Also red at the tip for the same S2 change, but not in the row, none name-shaped:
`seam2_seq_colink` and `seam2_soundtables_colink` (golden slices at shifted LMAs / grown
`sound_tables_z80`), `seam1_native_link` (a Z80 operand to a moved head, `$85B1` -> `$8633`),
`mt_port` (tip-derived `mt_bank_lma` against the golden). All four stay red with tip pins:
golden-frozen. `mt_bank_port` and `sfx_bank_port` go green with tip pins (pin drift).

## Open

1. PIN-ADVANCE item 3 needs the `repin.toml` `soundbankhead` literal re-expressed at the
   advance; a repin alone leaves `soundbankhead_port` red.
2. PIN-ADVANCE item 1's `DAC_SAMPLE_TAB_LEN` is a derivable restatement left for that
   parcel (the row asks for it to be worked then).
3. `ojz_run_b_port`'s stale declared shape delta (2026-09-25 note, open item 1) is untouched.
