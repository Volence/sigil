# PIN-ADVANCE-STEP0: sigil's reference pin, aeon 1ee78b88 to 936439fa (step 0 Task 6 / A9)

Branch `parcel/step0-soundness` (Tasks 1-5 already on it). No emulator was used. The precedent
followed step for step is `2026-09-28-pin-advance-s2.md`.

## The revision

The hub named aeon `936439faee52e89aacb0daef141b2e48ff613fc2` (aeon origin/master), 63 commits past
`1ee78b88` (`git rev-list --count 1ee78b88..936439fa`). Aeon's local master is three commits further
(`90a45adb`, `6db86e0d`, `c0ced27e`) plus the A2 pair, docs and one inert map key only; the hub measured
`git diff --stat 936439fa c0ced27e -- engine games tools build.sh` empty. `scripts/provision-aeon-ref.sh`
refuses a revision not on aeon origin/master, so 936439fa is the pin.

## The reference trees

- `.aeon-sigil-ref-936439fa` (the provisioned tree): HEAD `936439fa`, `git status` clean (ignored build
  products only). Python `zlib.crc32`: s4.bin `193507d9/850296`, s4.debug.bin `ce326232/870560`, demo.bin
  `2f9e2d4c/99994`, demo.debug.bin `64717bac/106981`.
- `.aeon-sigil-ref-936439fa-t6`, made for this task: `git worktree add --detach` at the same SHA, then its
  gitignored artifacts (`engine/debug/generated/`, `engine/sound/generated/`, the four ROMs and listings,
  `tools/bin/`, salvador) rsynced from the provisioned tree. Same HEAD, clean, same four CRCs. Reason: the
  freeze's capture deletes and rebuilds every ROM in `AEON_DIR`, and Tasks 7-9 may be reading the
  shared tree at the same time. Every measurement below used this copy. Remove it after landing.

## Baseline, before any change (branch tip `503e4689`)

- `refreeze --check`: OK, tip `pin-advance-s2`, chain len 207 (the two known DIVERGENT revisions #181, #201).
- `repin --check` against 936439fa: **rc 1**, `pins.rs is STALE`, 26 pin values moved. It RESOLVED: no
  symbol or row was missing, unlike the s2 advance (rc 2 on a deleted symbol). Warnings: the standing
  `player_climb` declared-allotment pair only.
- SS, `scripts/landing-run.sh --aeon <t6>`: 525/525 binaries launched and reported, **5,850 passed / 67
  failed / 3 ignored**, 0 skip lines, clippy 0, ledger 0, CI-shaped leg 0 (525 suites, 5,917 passed).
  The 67 names equal `.scratch-t1/failed_base.txt` exactly. Wall 06:27:49 -> 06:37:41 UTC (9m52s).

## The strict-attestation precondition (not in the brief)

The first `refreeze --freeze` ran capture, sizes and repin, then refused the append: the tip
`pin-advance-s2` (entry #207) carries no strict run and the rule is armed (entry #173). The s2 note left
`--attest` to the controller and it was never run. The run's outputs were saved aside and discarded
(`git restore --source=HEAD`, journal removed; recovery option 2 the tool printed).

`refreeze --attest` with `AEON_DIR=.aeon-sigil-ref-1ee78b88` (HEAD 1ee78b88, clean, ROMs `50401674` /
`198fd717`: the tip's own revision and goldens) and `ORACLE_DIR=oracle-old` (`1eb09a9`), at `503e4689`:
**passed**, 525 suites, 5,917 passed / 0 failed / 3 ignored, 0 skip lines, strict bodies 29 -> 29,
declared-count reconciliation 5,920 / 5,920, census 29 sites from 31 tests. 7m30s. Commit `a8973000`.

So the branch (Tasks 1-5) is green against the OLD pin's own tree and goldens; the 67 red names were pure
drift to 936439fa. The record names sigil `503e4689`, which is not on sigil master yet (AHEAD OF REMOTE):
it becomes reachable when the branch merges. A rebase or squash of the branch would orphan it.

## repin.toml at 936439fa

No row changed: `repin --check` resolved at baseline. (Three comments named the retired module id
`games.sonic4.mt_bank_blob`; the `repin.toml` one was corrected in the repairs commit. `pins.rs` is
unaffected.)

## The freeze (commit `fa042108`), entry `pin-advance-step0`, chain len 208

```
CARGO_TARGET_DIR=.target-land SIGIL_EMIT=.target-land/release/emit_sound_blob \
SIGIL_BUILD=.target-land/release/sigil AEON_DIR=<t6> \
  cargo run --release -p sigil-harness --bin refreeze -- --freeze pin-advance-step0 \
    --ab "sigil-side corpus pin advance 1ee78b88 to 936439fa, the quiet revision aeon named; s4 and s4.debug reproduce aeon's own figures" \
    --note "advances the reference pin 63 aeon commits past 1ee78b88; song bank 2 WFZ/OOZ grew the Z80 sound tables head 0x80 and the sfx bank 0x10"
```

Assembler `sigil 0.1.0 (503e4689)`, closure revision `eab19132`. 07:45:38 -> 08:48:38 UTC (63 min).
After it: `refreeze --check` OK, tip `pin-advance-step0`, chain len 208 (+1); `repin --check` prints
`pins.rs unchanged`. The entry is FROZEN, not attested.

| target | committed golden (python zlib) | aeon's figure at 936439fa | previous entry |
|---|---|---|---|
| s4 | `193507d9/850296` | `193507d9/850296` EQUAL | `50401674/845398` |
| s4.debug | `ce326232/870560` | `ce326232/870560` EQUAL | `198fd717/865722` |
| demo | `2f9e2d4c/99994` | (provisioned `2f9e2d4c`) | unchanged |
| demo.debug | `64717bac/106981` | (provisioned `64717bac`) | unchanged |
| config_a | `6be62db3/870972` | n/a (sigil-only shape) | `a7cfccbb/866074` |
| config_b | `d888a312/617813` | n/a | unchanged |
| lean | `60a3552b/798202` | n/a | `9f0d4de3/793376` |

aeon's figures are the provisioned tree's canonical ROMs (built by aeon's own `build.sh`) and the hub's
measurement of the same SHA; they agree. The refused first capture and this one are byte-identical in all
seven blobs and in `pins.rs`.

**The relationship, stated plainly.** No shipping byte changed. The seven goldens moved to match what the
branch binary ALREADY produced against 936439fa: they are exactly the seven `SHAPE` rows of Task 0's and
Task 5's BN runs at 936439fa. Measured again after this task: `step0_shapes_cmp.sh` base `94cf811d` vs the
branch binary over 936439fa, `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`. A BN-style comparison of the
binary's output against the OLD goldens (1ee78b88) is NOT 7/7 and is not expected to be: s4, s4.debug,
config_a and lean moved because aeon moved.

### What moved, derived from aeon

All from aeon `70027733` (song bank 2: WFZ and OOZ):

- `sound_tables_z80` head 985 -> 1,116 B (+0x83; the S2 PSG envelope `fTone_0C`). The emitted
  `sound_tables_z80.bin` (1,116 B) and the listing (PitchTable VMA `$845C`) agree. soundbankhead
  0x6B0 -> 0x730 after the 8-byte pad, so `mt_bank` and `sfx_bank` slide +0x80.
- SONG_COUNT +2 (plain 6, debug 8): SongTable and SongPatchTable each +8 B, the sfx bank +0x10.
- Song bank 2 at `$C0000` grows +0x12DA: its head copy +0x80 (`Song_S2_MTZ` `$C06B0` -> `$C0730`),
  `Song_S2_WFZ` + patches +0x65E (`$C132C..$C198A`), `Song_S2_OOZ` + patches +0xBFC (`$C198A..$C2586`).
  Everything after it (GameState_OJZScroll_Init, the error-handler island, EndOfRom, the epilogue) slides
  +0x12DA in both shapes.

## Repairs

Post-freeze SS at `fa042108`: 525/525, **5,914 passed / 3 failed / 3 ignored**, CI-shaped leg 101 (1
failed). The freeze absorbed 64 of the 67 baseline names; one new name went red (it held the old literals).

| test | derivation | vs pins.rs |
|---|---|---|
| `seam2_layout_derivation::sound_layout_derives_the_frozen_addresses` | heads from 0xB8000: +1116 = pitchtable 0xB845C, +264 = SfxBlobWinTab 0xB8564, +274 = SeqOpcodeTable 0xB8676, +64 = DacSampleTable 0xB86B6, +120 = 0xB872E, to 8 = mt_bank 0xB8730; + 0x4EB8 / 0x6900 (emitted bodies) = sfx_bank 0xBD5E8 / 0xBF030 | equal (MT_BANK_BLOB, SFX_BANK_BLOB bases) and equal the listing PHASE lines |
| `repin_pins::generated_pins_match_the_hand_typed_baseline` | ASSEMBLED_LEN 0xC2BA2 -> 0xC3E7C, DEBUG_ASSEMBLED_LEN 0xC49B0 -> 0xC5C8A: EndOfRom read from both listings, each +0x12DA, the song bank 2 sum above | equal |
| `warn_tier_corpus::warn_tier_firing_files_match_the_pinned_sites` | the `module.path-mismatch` row's one file, `mt_bank.emp`, fires in no shape: aeon `97d139ca` renamed its module to `games.sonic4.mt_bank`. Row retired to `files: &[]` (its comment predicted this); 19 firings per shape remain, all under the unpinned prefixes | n/a |

Repairs commit `7dc3c1cf`. No new gate, so RF does not apply; the guards
are the existing `provenance_chain_holds` and `refreeze --check`.

`mt_bank_debug_matches_reference` (a named carry) went green with the freeze alone: it reads
`pins::MT_BANK_BLOB`, whose base moved +0x80.

## Not done here, booked

- `module_roots.rs` `SHAPE_GATES` still gates both `games.sonic4.mt_bank_blob` and `games.sonic4.mt_bank`.
  QUEUE MT-BANK-RENAME-PAIR says to retire the old row at this advance. Kept: the old id is dead at
  936439fa (byte-neutral either way), but retiring it changes a sigil source in the shipping closure,
  and the attest above showed the branch is green against the 1ee78b88 tree, which still has the old id
  and would root the bank in sound-off shapes without that row. Retire it once no lane builds a pre-97d139ca
  tree.

## For the controller

- Merging moves the provenance tip to aeon 936439fa. Every suite run against `.aeon-sigil-ref-1ee78b88`
  is red after that, and `scripts/provision-aeon-ref.sh` with no revision argument provisions 936439fa.
  Docs naming 1ee78b88 as the standing reference (`docs/OVERSEER-REFERENCE.md`, `docs/QUEUE.md`) are not
  edited here.
- The attest record on entry #207 names sigil `503e4689`: merge, do not rebase or squash, or the record is
  orphaned (DIVERGENT, like #181 and #201).
- `.aeon-sigil-ref-936439fa-t6` is a disposable copy; `git -C aeon worktree remove` it after landing.

## Final gate

`scripts/landing-run.sh --aeon <t6>` at `2ec622da` (tree clean): **RESULT GREEN**, 525 of 525 binaries
launched and reported (census 525: 512 runnable + 13 doctest), **5,917 passed, 0 failed, 3 ignored**,
0 skip lines, clippy 0, ledger 0, CI-shaped leg 0 (525 suites, 5,917 passed, 0 failed);
`scripts/check_landing_log.py` exit 0. 09:04:03 -> 09:12:47 UTC (8m44s). The baseline counted 5,917
tests (5,850 + 67); the final run passes the same 5,917. BN at `2ec622da`: `SHAPES 7/7 IDENTICAL`,
`STEP0_CMP_END rc=0`. `refreeze --check` OK, tip `pin-advance-step0`, chain len 208.
