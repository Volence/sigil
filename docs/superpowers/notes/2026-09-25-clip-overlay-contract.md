# 2026-09-25: the clip anchor overlay, contract for aeon

The build side of owner decision `d-35-revised` (answered `clip-overlay-file`): a clip build
gets its own sound-bank positions from a small file in the clip's folder, handed to sigil only
when building that clip. `games/sonic4/map.toml` does not change and the layout language gains
no word. Sigil pins no overlay value anywhere; its tests use fixture files.

## The switch

`--anchor-overlay <path>`, on both binaries:

    sigil build --aeon . --game sonic4 [--debug] --anchor-overlay <path> ...
    emit_sound_blob --aeon . --out-dir engine/sound/generated --anchor-overlay <path>

The name is the one aeon's own pricing (aeon `3f208336`, M3) already proposed and plans to
reuse on `tools/bganim_room.py`, so the three tools spell it the same way. It says what the
file may do: replace anchors, nothing else.

**Path resolution.** A relative path is resolved against the process working directory, the
same as `-o` and `--emit-lst`; an absolute path is used as is. build.sh runs both binaries from
the aeon root, so `games/sonic4/data/clips/<id>/anchors.toml` works as written.

**No switch, no change.** Without the switch nothing the switch adds runs: the map is read as
today, the frozen island rows are used as today, and the four canonical shapes stay byte
identical to their goldens.

## The file

TOML holding only `[[anchor]]` rows, each written exactly as `map.toml` writes that anchor:

    # games/sonic4/data/clips/<id>/anchors.toml
    [[anchor]]
    name = "dac_banks"
    at = 0xB8000
    when = "sound_on"

    [[anchor]]
    name = "sound_bank"
    at = 0xC8000
    vma = 0x8000
    when = "sound_on"

Keys: `name` and `at` (required), `vma` and `when` (optional, but they must match the
`map.toml` row, see refusals). The values above are an example, not a pin: aeon derives them
from the bank placement rule over both clip shapes, and moves them in this one file when the
clip grows.

## What a row does

Each row REPLACES the `at` of the `map.toml` anchor of the same name, for that invocation
only. All rows apply in one pass against the original map, so an overlay address that equals
another anchor's original address never moves a section twice (the clip's `dac_banks`
0xB8000 is canonical's `sound_bank` 0xB8000; `Dac_Temp_Blip` lands at 0xB8000 and
`SoundTablesZ80_Head` at 0xC8000).

The island section each anchor holds (the one section sigil's frozen table records at the
anchor's `map.toml` address) is held at the overlay address instead, `validate_placement`
checks the overlaid anchor set, and the sound emit derives every bank LMA and every baked bank
id from the overlaid anchors. Sections after the banks (`Song_MovingTrucks` onward) pack as
usual; their `[layout.provisional-drift]` warnings in the clip shape are warnings, never fatal.

## Refusals

Each is a build error naming the file:

| id | when |
|---|---|
| `[map.overlay-read]` | the file is missing or unreadable |
| `[map.overlay-parse]` | the file is not TOML, has any key other than `anchor` at top level, or an anchor row has a key other than `name`/`at`/`vma`/`when` or lacks `name`/`at` |
| `[map.overlay-empty]` | the file has no `[[anchor]]` row |
| `[map.overlay-duplicate]` | two rows name the same anchor |
| `[map.overlay-unknown-anchor]` | a row names no `map.toml` anchor |
| `[map.overlay-ambiguous-base]` | `map.toml` declares that name more than once |
| `[map.overlay-row-differs]` | a row's `vma` or `when` differs from the `map.toml` row it replaces (the overlay moves positions, nothing else) |
| `[map.when-unknown]` | a `when` other than `sound_on`/`sound_off` (this also applies to `map.toml` itself now, see below) |
| `[map.overlay-off-grid]` | an `at` not on the 0x8000 bank grid |
| `[map.overlay-anchor-collision]` | after the overlay, two anchors of one shape sit at one address |
| `[map.overlay-island-ambiguous]` | sigil's frozen table does not hold exactly one section at the replaced anchor's `map.toml` address, so which section is the island cannot be said |
| usage error | the switch given twice, or combined with `--config-a`, `--config-b`, `--lean`, `--stress-evict`, `--stress-art` or `--report` |

It is accepted with `--game sonic4|demo`, `--debug`, `--check`, `--extra-entry`, `-o` and
`--emit-lst`.

Hardening that ships with this (queue row MAP-WHEN-UNKNOWN-VALUE-SILENT), for `map.toml` and
the overlay alike: an unknown `when` value is refused instead of applying to every shape, and
an unknown key on an `[[anchor]]` or `[[hole]]` row is refused instead of dropped. Both aeon
maps at `ec640bcf` and at today's `origin/master` use only `sound_on`/`sound_off` and only the
keys `name`/`at`/`vma`/`when` and `after`/`at`/`filled_by`/`when`, so neither changes.

## What aeon does

1. Add `games/sonic4/data/clips/<id>/anchors.toml` for a clip that needs its own positions,
   values from the rule taken over both of that clip's shapes (the DEBUG shape binds). A clip
   without the file builds exactly as today.
2. In build.sh's S2CLIP branch, when that file exists, pass `--anchor-overlay <path>` to
   `sigil build` (`NATIVE_FLAGS`) and to nothing else in build.sh. In particular do NOT pass it
   to the preflight `emit_sound_blob` call (see "Why not the preflight emit" below). `sigil build`
   re-emits every sound artifact into `engine/sound/generated` itself, from its own switch,
   before it assembles, so its ROM never depends on the preflight emit.
3. Check the wiring: after the S2CLIP build, require the listing's Source Digest to carry a
   `DIGEST-READ ... path=games/sonic4/data/clips/<id>/anchors.toml` row whenever that file
   exists. This is the only thing that notices the switch was dropped (see "The two one-binary
   mistakes" below); `tools/artifact_provenance.py` already reads that digest.
4. `tools/bganim_room.py` reads the anchors by name (`anchor_addr`, around line 338) and needs
   the same overlay, or its room and pair checks measure against the canonical anchors.
5. After a clip build, `engine/sound/generated` holds the clip's overlay-emitted artifacts until
   the next build's preflight emit rewrites them. Anything that reads that directory between
   builds sees the last build's positions, which is how it behaves for every shape today.

`emit_sound_blob --anchor-overlay` stays available for a standalone emit of a clip's artifacts
into a directory of the caller's choosing; build.sh has no step that needs it.

## Why not the preflight emit (measured 2026-09-25, aeon `c53dde84`, clip `s2_ehz_cpz`)

build.sh runs `emit_sound_blob` into `engine/sound/generated`, then the pre-build tool-suite
lane, then the clip re-bake, then `sigil build`. Inside the tool-suite lane,
`tools/test_extern_guard_reachability.py` runs `sigil build --check` over the canonical shapes
(sonic4 plain and debug, demo plain and debug, `--config-a`) and
`test_check_does_not_perturb_generated_sound_artifacts` digests `engine/sound/generated` on
both sides of those checks. A `--check` on a sound-on shape runs sigil's own emit into that
directory, from `map.toml` as written, because those shapes are canonical. So:

- preflight emit WITH the overlay, then that test file: 1 failed, 4 passed. The canonical checks
  rewrote `dac_sample_tab.bin`, `mt_songtable{,_debug}.bin`, `mt_songpatchtable{,_debug}.bin`
  and `z80_sound_blob{,_debug}.bin`, every artifact that bakes a bank id or a bank address;
- preflight emit WITHOUT the overlay, then the same file: 5 passed.

This is the one failure the killed full build hit (1 failed, 3343 passed). It is not a sigil
defect: a canonical `--check` emitting canonical artifacts is correct, and the test is right
that the lane must not change what it rides in. It is the wiring: an overlay on the preflight
emit puts non-canonical artifacts in front of a canonical lane. With the overlay on `sigil build`
only, the preflight emit and the canonical checks agree, and the clip build re-emits its own.

## The two one-binary mistakes (measured, same tree, `FAST=1`, plain)

- Overlay on `sigil build` only: the ROM is byte-identical to the one built with the overlay on
  both binaries, and `engine/sound/generated` ends up holding the overlay artifacts. This is the
  wiring step 2 asks for.
- Overlay on `emit_sound_blob` only: `sigil build` re-emits from `map.toml`, so the overlay is
  silently dropped. The ROM is byte-identical to a build with no overlay at all, and
  `[sound.bank-id-vs-placement]` has nothing to catch, because the build's own emit and its own
  placement agree. The listing's Source Digest has no row for the overlay file; step 3 turns
  that absence into a refusal.

`[sound.bank-id-vs-placement]` is the witness for a disagreement INSIDE one build: every baked
bank id is read out of the linked ROM and compared with the bank as placed. With the build's own
emit forced to ignore the overlay while its placement used it (a mutation of sigil, measured
against the reference tree at aeon `ec640bcf`), the overlay build fails there with 17 mismatched
ids. No build.sh wiring can produce that disagreement, since the build
never links the preflight emit's files without re-emitting them first.
