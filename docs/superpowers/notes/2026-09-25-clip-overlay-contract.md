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
2. In build.sh's S2CLIP branch, when that file exists, pass `--anchor-overlay <path>` to BOTH
   `sigil build` (`NATIVE_FLAGS`) and the `emit_sound_blob` call. `sigil build` re-emits the
   sound artifacts itself before assembling, from its own switch, so its ROM does not depend
   on the earlier emit; passing the switch to the emit keeps `engine/sound/generated`
   consistent for anything that reads it between the two steps.
3. `tools/bganim_room.py` reads the anchors by name (`anchor_addr`, around line 338) and needs
   the same overlay, or its room and pair checks measure against the canonical anchors.
4. The overlay file is read through sigil's build read set, so it appears as a row in the
   listing's Source Digest with its CRC and size; `tools/artifact_provenance.py` can see which
   positions a listing was built with from that row.

The end-to-end witness is `[sound.bank-id-vs-placement]`: every baked bank id is read out of
the linked ROM and compared with the bank as placed, so a build whose emit and placement saw
different positions fails there.
