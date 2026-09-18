# `P2BIN-OPTIMISED-COMPRESSORS`, 2026-09-18

p2bin offers Kosinski and Saxman twice over: an "authentic" compressor, which is
Sega's own greedy one, and an "optimised" one. Each corpus's build.lua picks
between them (`improved_dac_driver_compression` in Sonic 1,
`improved_sound_driver_compression` in Sonic 2). sigil implemented only the
authentic side, refused the other two by name, and so compared no byte at half
of each corpus's swept corners. This closes that.

## What the optimised pair turned out to be

clownlzss's optimal parser, which `sigil-clownlzss-sys` already vendors at
`8055bd2e`. `kosinski-optimised` is `compress_kosinski` and `saxman-optimised`
is `compress_saxman` without a header, so the work was identification, not
implementation.

Identified by measurement, never by reading the vendored source.
`gen_opt_vectors.py` hands the fourteen generated blobs of the stage-2 vector
set to the p2bin binary (md5 `4f2fff99c3347bafb93b12d5be1db754`) once per
compressed format; `gen_opt_vectors.out` is what it printed. Both formats match
on all fourteen. The run also reprinted the `kosinski`, `saxman` and
`saxman-bugged` columns and all forty-two matched what `accurate_vectors.rs`
already held: the same binary through a second, independently written script.

## The three things the byte pins do not say, each measured separately

* The header-framed Saxman is exactly two bytes longer than the stored stream on
  every vector and never equal to it. `-z` writes the stored size into the
  header FILE p2bin is given, so the stream carries no size of its own. This is
  the positive control: without it the byte comparison has never been shown to
  be able to fail.
* An optimised stream decompresses back to its input through the AUTHENTIC
  decoder. A pair is one container; only the match selection differs. The
  placement's round-trip check rests on this.
* Sega's Saxman emits a match reaching back past the start of the output on five
  of these fourteen inputs, which the game reads as zeros; clownlzss's parser
  never looks back past the start, so none of the fourteen straddles. This is
  why the round trip above holds on fourteen rather than on nine, and why the
  straddle probe that sigil refuses under `saxman` builds under
  `saxman-optimised`.

## Plain `saxman` came free, `kosinskiplus` did not

`compress_saxman_authentic` already existed and was already pinned against
p2bin's `saxman` format, so only the enum arm was missing. `kosinskiplus` stays
unimplemented and is now the only p2bin format sigil refuses by name; no corpus
here selects it.

## What the sweep then found

`sweep-full.log`: 40 legs launched, 40 reported, `SWEEP PASSED`, 0 unacknowledged
and 0 stale acknowledgements.

| Leg | Before | After |
|---|---|---|
| `s1disasm-build.lua:improved_dac_driver_compression-1` | `SIGIL-DECLINED` | **AGREE**, crc32 `faa36f4d`, 524,288 bytes |
| `s2disasm-build.lua:improved_sound_driver_compression-1` | `SIGIL-DECLINED` | **DIFFER**, 2 bytes at `0x18F` and `0xEC051` |

Sonic 1 agrees across a whole ROM at exactly the image the gap ledger predicted
the flip would move to.

Sonic 2's two bytes are not the compressor. `Size_of_Snd_driver_guess = $F64`
and the optimal parser stores `$F4A`, `$1A` fewer; Sonic 2 loads that constant
into the `move.w` its Saxman decompressor reads as a byte count, and build.lua
patches the immediate afterwards from the real size asl's share file reports.
sigil writes no share file, so the patch finds nothing and skips. `0xEC051` is
the low byte of that immediate and `0x18F` is the header checksum that follows
from it. It is `SWITCH-SETTING-SILENT-ROMS` fault 2 in the direction sigil
deliberately does not refuse, and it is booked as Open 5 of the `SWEEP-NIGHTLY`
row. `sweep-two-legs-before-ack.log` is the `--only` probe that first saw it,
before the acknowledgement existed.

Acknowledging a DIFFER needed the acknowledgement to be able to say more than
"the images are not the same". `leg_report` is now the one place that decides
what a leg reported in the terms of its own class, read by both the summary
table and `ack_mismatch`, so an acknowledgement's text is pinned to what a
reader of the table sees. Self-test C12b covers it.

## Checked and NOT part of this

s2disasm's build.lua hands the same local to a STANDALONE `saxman` tool for its
music blobs (the `-a` flag at line 129), which is a different code path from
p2bin's `-z`. Nothing is needed of sigil: `generate_music_data` runs inside
build.lua before either toolchain assembles, and the sweep leaves the generated
inputs in the tree so sigil consumes the same `.sax` files the reference
consumed. One thing that holds only by luck is booked as Open 6.

## Red-first

`mutations.log`, from `mutate_opt.py`: nine mutations, each applied on disk from
the committed baseline with the mutated line quoted back beside `git diff
--stat`, each run red, each restored. A tenth, on the new acknowledgement's
pinned offsets, is in the commit that added it.
