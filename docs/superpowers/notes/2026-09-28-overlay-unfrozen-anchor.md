# 2026-09-28: an anchor overlay may move a walk-held anchor

Parcel SONG-BANK-2-OVERLAY-UNFROZEN-ANCHOR, asked by the aeon lane.

## For aeon, in plain words

- Every S2CLIP build of your song-bank-2 tree stopped at
  `[map.overlay-island-ambiguous] ... moves song_bank_2 from 0xc0000 to 0x..., but the
  frozen table holds 0 section(s) at 0xc0000`. That refusal is gone for this one case.
- An overlay row for an anchor that no frozen-table row sits under (today only
  `song_bank_2`) now just moves the anchor. Nothing in the frozen table moves with it,
  because nothing there held it: the packing walk places `SongBank2_Head` by its declared
  0x8000 alignment after bank 1, whatever the anchor says.
- So the overlay's `song_bank_2` address is a CLAIM, and the build checks it. If it is
  wrong, the build is refused by name:
  `[map.undeclared-island] ROM section at 0xC8000 is an ANCHOR_GAP-inferred island but no
  [[anchor]] at = 0xC8000 is declared` (measured: the s2_mtz_cpz overlay with
  `song_bank_2` changed from 0xC8000 to 0xD0000). If the head lands within 0x400 of
  bank 1's end, there is no inferred island there at all and the refusal is
  `[map.anchor-absent] declared anchor song_bank_2 at 0x... is not an inferred island`.
- Your committed clip overlays (`s2_ehz_cpz`, `s2_mtz_cpz`, `s2_woven`, each already
  carrying a `song_bank_2` row at its `sound_bank + 0x8000`) build as they are. Nothing
  was re-derived.
- It is tolerant in both directions: a tree whose map and overlays have no `song_bank_2`
  row never reaches the changed branch, so it builds exactly as before; your change can
  land alone after this one.
- No map or overlay format word changed.

## What still refuses

`[map.overlay-island-ambiguous]` still fires when the frozen table holds MORE than one
section at the moved anchor's `map.toml` address, and when a section the overlay did not
move already sits at the moved anchor's new address. Both are unit-tested beside the new
case in `crates/sigil-harness/src/map_placement.rs`.

## Why a mistaken address stays loud

The walk does not read the overlaid anchor to place a walk-held section. In
`packed_true_bases` (`crates/sigil-harness/src/native.rs`), a section with no frozen row
takes the label-less arm: its provisional base is its baked lma (0 for a `.emp` section),
so the `p > r + ANCHOR_GAP && is_anchor_gap(p)` island test never matches and it packs
through `packed_chained_base` at its declared alignment. The overlaid anchor list reaches
the walk (native.rs `resolve_chained`: `overlaid_placement` replaces `pmap` with the
overlaid map, and `anchor_addrs` is collected from it) but only a section whose
provisional base equals an anchor is held by it.

The overlaid list is also what `validate_placement` checks after the resolve:
`build_rom_chained_with_listing` takes `pmap` from `resolve_chained` and calls
`validate_placement(&resolved, &pmap, ...)` with it. It infers the
islands from the layout (run head, phase bank, or a gap over 0x400) and requires inferred
and declared to agree both ways. A wrong `song_bank_2` address therefore either leaves the
real island undeclared (`[map.undeclared-island]`, which the section loop reaches first) or
names an address nothing landed on (`[map.anchor-absent]`). The unit test
`native::placement_validation_tests::walk_held_anchor_overlay_is_checked_end_to_end` runs
overlay, island rows, the walk and the lint on a synthetic layout and asserts both.

A correction to the ask's framing: the overlaid anchor address is not what the walk
honours for bank 2. It is what the lint checks the walk's result against. The fix is safe
for that reason, not because the walk follows the anchor.

## Probe

Probe copy of aeon at `8afd1de5` (`parcel/song-bank-2-step3`), donors copied in from the
main aeon tree (gitignored input the clip bake needs). All builds `FAST=1 ./build.sh`
with `SIGIL_BUILD`/`SIGIL_EMIT` set. CRC is zlib CRC-32 of the whole ROM, then its size.
"installed" is `sigil/target/release` at the time of the run; "branch" is this parcel.

| shape | installed rc | branch rc | branch CRC/size | SongBank2_Head |
|---|---|---|---|---|
| canonical plain | 0 (66d0cc2f/844245) | 0 | 66d0cc2f/844245 | 0xC0000 |
| canonical debug | 0 (12068f2e/864545) | 0 | 12068f2e/864545 | 0xC0000 |
| S2CLIP=s2_mtz_cpz plain | 1, overlay-island-ambiguous | 0 | 70884f0b/877310 | 0xC8000 |
| S2CLIP=s2_mtz_cpz debug | 1, same | 0 | c4953133/897433 | 0xC8000 |
| S2CLIP=s2_ehz_cpz plain | 1, same | 0 | d92a9ada/943848 | 0xD8000 |
| S2CLIP=s2_ehz_cpz debug | 1, same | 0 | e9ecb28f/963966 | 0xD8000 |
| S2CLIP=s2_woven plain | 1, same | 0 | 3d082582/1174169 | 0x110000 |
| S2CLIP=s2_woven debug | 1, same | 0 | d665a5f1/1194288 | 0x110000 |
| s2_mtz_cpz plain, `song_bank_2` mistyped 0xD0000 | not run | 1, `[map.undeclared-island]` at 0xC8000 | none | |

Each clip build reported its `anchors.toml` FRESH for the shape.
