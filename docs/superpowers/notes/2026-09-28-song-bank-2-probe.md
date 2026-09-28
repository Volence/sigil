# SONG-BANK-2-STUB-PROBE: what the stub bank does to the build

Sigil branch `parcel/song-bank-2`, assembler at `8d2466e5` (release `sigil` and
`emit_sound_blob` rebuilt from that tip before the runs). Probe tree: a copy of aeon at
`53931a88`, with a stub bank-2 module and one `order` row added. No emulator was used.

## For aeon, in plain words

- **Without an `[[anchor]]` at 0xC0000**, both sound-on shapes that build.sh ships (plain
  and debug) FAIL, with the same one gate:
  `[map.undeclared-island] ROM section at 0xC0000 is an ANCHOR_GAP-inferred island but no
  `[[anchor]] at = 0xC0000` is declared, add it to the placement map`.
  The demo shapes build and are byte-identical to the control (the demo game never places
  the sonic4 module; its listing only shows the file in the source scan).
- **With your anchor** (`name = "song_bank_2"`, `at = 0xC0000`, `when = "sound_on"`, no
  `vma`), all four shapes build, rc 0. `SongBank2_Head` lands at exactly 0xC0000 in plain
  and debug, and the test game states follow it at 0xC0008. No new gate fires; the
  `layout.provisional-drift` warning count is the same as the control's (5 plain, 13 debug).
  Built directly with `sigil build`, config_a, lean and config_b also build rc 0 with the
  anchor; config_a and lean place the head at 0xC0000; config_b (sound off) does not place
  the module at all.
- **A refreeze will NOT pin `SongBank2_Head`.** The refreeze regenerates the frozen tables
  over the label set the committed tables already hold, and never adds a label. The head
  stays walk-placed (rounded up to 0x8000), and your anchor stays a lint. Citations below.
- **The gap, measured** (bank 1 ends at `Sfx_33` + the 1,603 B SFX blob):

  | shape | `Sfx_33` | bank-1 end (last byte) | gap to 0xC0000 | bank-1 growth that keeps the island |
  |---|---|---|---|---|
  | plain (and lean) | 0xBD580 | 0xBDBC3 | 9,277 B | 8,252 B |
  | debug (and config_a) | 0xBEFD8 | 0xBF61B | 2,533 B | 1,508 B |

  The gate infers an island only when `lma > prev_end + 0x400`, strictly. So in debug,
  1,508 B more of bank-1 content is fine and the 1,509th byte makes 0xC0000 no longer an
  inferred island: `[map.anchor-absent]` fires for your `song_bank_2` anchor. (Your note
  said 1,509 B left; it is 1,508, off by one at the strict `>`.) Growth that comes from the
  MT bank moves `Sfx_33` in 8-byte steps (its base is kept 8-aligned), so MT-only growth
  has 1,504 B in debug and 8,248 B in plain. Past 0xC0000 the head rounds to 0xC8000; by
  the source that fires `[map.undeclared-island]` at 0xC8000 first (the loop checks each
  section before the anchors). That second direction is READ, not run.

## Probe results

CRC is zlib CRC-32 over the whole ROM file, then the size in bytes.

| variant | shape | rc | CRC32/size | gate fired |
|---|---|---|---|---|
| control (stub removed) | plain | 0 | d2c5842a/829987 | none |
| control | debug | 0 | 1000eded/856982 | none |
| control | demo | 0 | 3bd1f8e2/99719 | none |
| control | demo_debug | 0 | 6e95b260/106701 | none |
| stub, no anchor | plain | 1 | no ROM | `[map.undeclared-island]` at 0xC0000 (text above) |
| stub, no anchor | debug | 1 | no ROM | `[map.undeclared-island]` at 0xC0000, same text |
| stub, no anchor | demo | 0 | 3bd1f8e2/99719 | none (= control) |
| stub, no anchor | demo_debug | 0 | 6e95b260/106701 | none (= control) |
| stub + anchor | plain | 0 | 21fad9c7/839291 | none |
| stub + anchor | debug | 0 | e5dd3b61/859536 | none |
| stub + anchor | demo | 0 | 3bd1f8e2/99719 | none (= control) |
| stub + anchor | demo_debug | 0 | 6e95b260/106701 | none (= control) |

Extra shapes, stub + anchor, built with `sigil build --aeon <probe> --<shape>` (not
build.sh; CRC as sigil printed it): config_a rc 0 `04164d1d`/859890, lean rc 0
`50b2dcea`/788604, config_b rc 0 `cff2b684`/616662. `Sfx_33` sits at the debug address in
config_a and the plain address in lean, matching the committed frozen tables, where
config_a equals s4_debug and lean equals s4 on `Sfx_33`.

The failing builds stop inside `sigil build`, after aeon's pytest lanes pass; the message is
the post-resolve placement lint in `native.rs` `validate_placement`.

Growth cost of the anchored stub: plain grows by 9,304 B (the 9,277 B gap, the 8 B stub,
and the tail's own re-alignment), debug by 2,554 B.

## Step 4: why a refreeze does not pin the head

`refreeze --freeze` runs three regeneration steps (`crates/sigil-harness/src/bin/refreeze.rs:1525-1535`):
golden capture, then `derive_offcanonical_sizes.sh` (the frozen tables), then `repin`
(`pins.rs`).

1. The frozen tables are the only placement input that can hold a section: a section is
   `labeled` (held at its provisional base) only if a label of it is a key of the
   profile's frozen table (`native.rs:2383-2401`); otherwise its `prov` is the baked lma
   (`native.rs:2401`) and it packs through `packed_chained_base` (`native.rs:2758-2761`).
2. The size-table step runs `derive_offcanon` (`bin/derive_offcanon.rs:60-61`) which calls
   `native::derive_frozen_table`. That function takes its label set from the COMMITTED table:
   `let want = profile.frozen_sizes.keys()` (`native.rs:4117-4118`) and writes a row only
   `if want.contains(&l.name)` (`native.rs:4126`). A label that is not already frozen is
   never added. `SongBank2_Head` is in no committed table (grep of
   `crates/sigil-harness/golden/offcanonical_sizes/` finds nothing).
3. `repin` writes `pins.rs` from the `[[region]]` rows of `repin.toml` only
   (`bin/repin.rs` header); no region names `SongBank2_Head`, and the packing walk does not
   read `pins.rs` (the only `crate::pins` read in placement code is `seam1.rs:75`,
   `BOOT_HEAD`).

So a refreeze leaves `SongBank2_Head` walk-placed. What WOULD pin it is a hand edit that adds
the label to a frozen table. That is caught: `section_alignment_declared.rs`
`the_requirements_above_16_are_declared_held_or_rounded` lists `SongBank2_Head` as ROUNDED
and fails if any shipped shape's frozen table carries a row for it.

## Reproduce

`scratch/probe.sh <variant>` in the sigil worktree (not committed) builds the four build.sh
shapes against the probe tree with `SIGIL_BUILD`/`SIGIL_EMIT` set to the branch binaries.
The stub module:

```
module games.sonic4.song_bank2

section song_bank2 (cpu: m68000, bank: $8000) {
    pub data SongBank2_Head: [u8; 8] = [$53, $42, $32, $48, $00, $00, $00, $00]
}
```

with `"SongBank2_Head"` after `"Sfx_33"` in `games/sonic4/map.toml` `order`, and for the
anchor variant the `[[anchor]]` above added after the `sound_bank` anchor. The probe tree was
left as handed over: the stub and its order row, without the anchor.
