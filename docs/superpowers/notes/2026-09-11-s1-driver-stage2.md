# S1 driver space, stage 2: the AS route takes p2bin's `-z` and `-p` as a build script writes them

2026-09-11, parcel `parcel/s1-driver-stage2`, base `704966f3` (contains stage 1's
merge `40365ac2`), with `origin/master` `ac639ebe` merged in by the controller's
authorization (a fast-forward: the CLI's option table). Owner decision `d-30`,
option `flag`.

## Headlines

1. **Sonic 1 is byte-identical, whole, no window.** `sigil sonic.asm -o s1built.bin
   -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after`, on s1disasm `f6ece657`, writes
   md5 `09dadb5071eb35050067a32462e39c5f`, CRC32 `afe05eee`, 524,288 bytes: the ROM
   the pinned asl plus p2bin write from the same tree and the same line, 0 bytes
   different. `tests/as_driver_placement_corpus.rs` builds the reference live and
   holds this.
2. **Sonic 2 is byte-identical too, whole, no window**, on the census's stub C tree
   with `build.lua`'s own instruction `-p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after`:
   md5 `9feeb724052c39982d432a7851c98d3e`, CRC32 `7b905383`, 1,048,576 bytes, equal to
   what `build.lua` itself builds.
3. **S3K's two-blob `before` shape**, in a probe built from its own text, gives the
   image p2bin gives under both `kosinski` (buildSK / buildS3Complete) and
   `uncompressed` (buildS3).
4. **The size constant's value is never read.** p2bin only prints its name in the
   overflow message. The budget is the layout's, and the brief's premise that the
   constant is "a symbol in asl's output" that p2bin reads is wrong (below).
5. **A Sega-Saxman stream can be wrong for the game, and sigil now says so.** The
   authentic compressor can emit a match that starts in its zero-filled ring buffer;
   Sonic 2's decompressor reads it as zeros throughout. p2bin writes such a stream;
   sigil's round-trip check refuses it. Sonic 2's real driver does not trigger it.

## Provenance

| Instrument | Identity |
|---|---|
| asl (reference) | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098` (the skdisasm copy is the same file). Every run checked for exit 0 and a complete pass footer. `asl_ref.sh` could not be sourced under this agent's worktree isolation guard, so its three checks (md5 literal, exit status, footer) ran inline, in `ptools.py`. |
| asl (Sonic 2 toolchain) | `s2disasm/build_tools/Linux-x86_64/asl`, md5 `0dee1f98e6480a4783d27ffd8b90896f`, the refused build, used only by `build.lua` for the Sonic 2 reference exactly as the census did (the pinned build cannot assemble Sonic 2). |
| p2bin (the oracle) | md5 `4f2fff99c3347bafb93b12d5be1db754`, the same file in all three disassemblies |
| saxman | `s2disasm/build_tools/Linux-x86_64/saxman`, md5 `704e5c8c361b9959f45bac3e803f0033` (run by `build.lua` only) |
| accurate-kosinski | `programs/accurate-kosinski/build/kosinski-compress`, md5 `044a1bf2e26005d6880ae9d3b36016af`, revision `45abe26a`; a second witness for the Kosinski port, and the only one past p2bin's 0x2000 limit |
| lua | `/usr/bin/lua`, Lua 5.5.1 |
| sigil, base | built from `704966f3`, md5 `69f6a8d1cd998dd8410e5bf8d0d4e995` |
| sigil, new | built from this branch's tree, md5 `ea1531bfd8dfbd4de6df6a5bdc71823b` for every corpus number below |
| corpora | `cp -a` copies in `/home/volence/sonic_hacks/.scratch/s1-driver-stage2/`, restored in the copy (`git checkout -- .`, `git clean -fd`, 0 status paths): s1disasm `f6ece657`, s2disasm `e45ebf33`, skdisasm `2fcd861c`. The three checkouts were only read. |
| cargo | `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/s1-driver-stage2/target`; the shared `target/` was not used. No aeon tree. |

## p2bin's rules, as measured

Probes and the scripts that ran them are in `2026-09-11-s1-driver-stage2/`
(`probes/*.asm`, `probe_rules.py`, `probe_rules2.py`, `cmp_probes.py`, and their
`.out` files). Each probe was assembled by the pinned asl; every statement below is
an output of the p2bin binary on one.

**Records.** asl's object file is a list of records, each a CPU (1 = 68000,
`0x51` = Z80), a load address and bytes, in assembly order. asl splits records far
more often than at `org`s: Sonic 1 has 442 records in 7 contiguous runs, 333 of the
splits inside a run.

**The blob.** The first Z80 record whose address equals `-z`'s address, then every
following record that is ALSO Z80 and starts where the blob so far ends. A 68000
record contiguous with it does not join (`p_cont68k`: the reported size is the
driver's 10 bytes, not 12); a Z80 record at a different address ends it
(`p_seg`, `p_segback`, `p_phase_after`).

**One `-z` places every blob that starts at its address** (`p_sameorigin`: two
drivers at Z80 0, one `-z=0`, both compressed and each placed after the record
before it). I first coded "the first match" from reading, and the measurement
refuted it.

**`after`:** the stream is stored where the record before the blob ENDS, which is
the last byte written, not the driver's label (`p_gap`: label at $20, stream at 8).
The reservation is the gap from there to the start of the record after the blob;
with no record after it, the reservation is unbounded (`p_last`). With no record
before the blob the previous record is an empty one at 0 (`p_first`).

**`before`:** the stream is stored where the record before the blob STARTS, over
that record's own bytes, and the reservation is that record's length. S3K makes the
record exactly the reservation: its `!org z80_SoundDriverStart` rewind starts a
record at `Z80_SoundDriver`, and the `org` macro pads it with `$FF`. Measured on the
real S3K object file (asl run with `-D Sonic3_Complete=0`, md5 `847e6fdf`): the
record before blob 1 is `[0xF6960, 0xF7760)`, `0xE00` long, the record before blob 2
`[0xF7760, 0xF7DF0)`, `0x690` long, and each Kosinski stream is EXACTLY that long
(the guesses are tuned to the byte), equal to accurate-kosinski's output. The
S3K reference `skbuilt.bin` (p2bin, before `fix_header`) is md5 `4ea493ea`, CRC32
`0658f691`, 2,097,152 bytes.

**The constant.** Its value is never read. `-z=0,uncompressed,Small,after`
(`Small = 4`) and `-z=0,uncompressed,Nope,after` (undefined) give the same image as
`Guess`; the name appears only in the overflow message. On Sonic 1 an undefined name
builds the same md5. Sonic 2's reservation is `0x1018` (`0xEC0E8` to the DAC samples
at `0xED100`), while its constant is `0xF64`: a budget equal to the constant would
refuse builds p2bin accepts, and Sonic 2's post-p2bin size patch exists exactly to
support a stream larger than the guess.

**Overflow.** A stored stream larger than its reservation: `Error: Space reserved
for the compressed Z80 segments is too small. Set '<constant>' to at least $<size>.`,
exit 1, and the output file is deleted (a pre-existing one included).

**Size limit.** A blob over 0x2000 uncompressed bytes, whatever the format:
`Error: Compressed Z80 segment is too large.`, exit 1 (0x2000 accepted, 0x2001
refused, including zeros that compress to almost nothing).

**Two `-z`.** Independent, order irrelevant (`p_two` both orders). A driver no
`-z` names is written at its Z80 address, over the image (the punt).
A `-z` that names nothing is silently ignored.

**Gaps.** Every address no record writes holds the pad byte: after the stream in
an `after` gap, between sections, and inside one where `ds.b` leaves a hole
(`p_pad`). Default pad 00.

**Grammar.** `-z=<address>,<format>,<constant>,<type>` split at the first three
commas (so `after,x` is an unknown type); address as `sscanf("%lX")`: hex, leading
white space, sign and `0x` accepted, trailing junk ignored (`10j` is 0x10), `-0` is
0, `0x` alone and an empty address are errors; the constant may be empty; format
and type are exact and case-sensitive (`Uncompressed`, `After`, ` uncompressed` are
errors). `-p=<pad>` as `sscanf("%X")`: `FF`, `ff`, `0xFF`, `+7`, `FFx` accepted;
`$FF`, empty, `0x`, `g`, `-pFF` are parse errors; `-1`, `100`, `1FF` are "too high".
The last `-p` wins. Options may follow the file names. **A grammar error prints and
p2bin goes on, exit 0**, ignoring that `-z` (so the driver lands on the vectors) or
using the default pad; sigil stops with exit 2 instead.

**Formats.** `uncompressed`, `kosinski` (authentic), `kosinski-optimised`, `saxman`
(authentic), `saxman-bugged`, `saxman-optimised`, `kosinskiplus`. sigil implements
the three the build scripts select at their shipped settings (`kosinski`,
`saxman-bugged`, `uncompressed`) and refuses the other four by name with that list.

**The header file** (third positional argument): p2bin writes `comp_z80_size 0x%lX `
over its first bytes in place. sigil has no such argument; Sonic 2's
`amend_sound_driver_size` step that reads it is a separate build-script step, out of
scope, and changes zero bytes at this revision (census).

## The compressors

- **Kosinski (authentic)** is Clownacy's accurate-kosinski, including all six of
  Sega's mistakes and the zero padding of the whole stream to a multiple of 16. The
  port is `sigil-clownlzss-sys::accurate::compress_kosinski_authentic`. It equals the
  stream in the Sonic 1 ROM (5,984 bytes, exactly the `0x1760` gap) and both S3K
  streams. clownlzss's optimal Kosinski does not.
- **Saxman (authentic)** is Okumura's 1989 LZSS as modified for Saxman (s2disasm's
  `lz_comp2`), ported as `compress_saxman_authentic`. **`saxman-bugged`** appends one
  byte: `0x4E` after an odd-length stream, `0x00` after an even one, always. The
  census's "0x4E if odd" was half the rule.
- **Vectors.** 14 generated inputs, placed by `binclude` and compressed by p2bin
  itself (lengths read from p2bin's header file): all three formats match on all 14
  (`tests/accurate_vectors.rs`, generator `gen_vectors.py`). The 0xA000 dummy match
  cannot be reached through p2bin (its 0x2000 limit), so it is pinned against the
  accurate-kosinski binary on a 0xA100-byte input.
- **The straddle.** Okumura's decoder reproduces all 14 inputs from p2bin's streams;
  Sonic 2's decompressor (ported from `s2.asm` `DecompressSoundDriver` in the
  census), and clownlzss's which has its semantics, differ on exactly the 5 vectors
  that hold one match starting in the ring buffer's zero prefix. This is a property
  of the original toolchain, not of the port.

## The checks the owner's option carries

1. **Budget:** an overflow is refused by name with both sizes and where the
   reservation is: ``-z=0,uncompressed,Guess,after`: the blob [0x0, 0x14) is 0x14
   bytes stored uncompressed, and only 0x10 are reserved for it, at the gap [0x8,
   0x18) up to the code after it; set `Guess` to at least $14``. Never truncated,
   never spilled.
2. **Round trip:** the stored stream must decompress, through sigil's own
   decompressor (clownlzss), to exactly the assembled blob, in the build (it costs a
   decompression of at most 0x2000 bytes) and, on the real corpus, in
   `as_driver_placement_corpus.rs` against asl's own Z80 records.
3. **Byte compare** against images p2bin wrote: 17 behaviours in
   `as_driver_placement.rs`, Sonic 1 whole in the corpus test.

Also refused, where p2bin is silent: a `-z` that names nothing; a second space no
`-z` names (stage 1's refusal, at its `org`, now worded "and no -z instruction
places it into the ROM"); a space whose code does not continue the blob; a stream
another record would overwrite; two `-z` naming one address; a grammar error.

## What changed, by file

- `sigil-clownlzss-sys`: new `accurate` module (the two authentic compressors) and
  `tests/accurate_vectors.rs`.
- `sigil-link`: new `blob` module (`flatten_placing`, `parse_pad`, `parse_blob`,
  `BlobCodec`, `BlobFormat`, `BlobInsert`, `BlobInstruction`, `MAX_BLOB`,
  `P2BIN_FORMATS`); `relax.rs` gains `resolve_layout_placing` (a new entry point;
  `resolve_layout` and `resolve_layout_measuring` pass no origins and are unchanged
  in behaviour), and `foreign_space_diags` exempts a Z80 space whose origin a `-z`
  names; the refusal's wording. **No new dependency**: `crate_graph_is_one_way` pins
  the linker to `sigil-ir` + `sigil-span`, which the first version of this parcel
  broke (caught by the scoped `sigil-cli` suite, fixed in `ea992d37`), so the
  compressors come in through the `BlobCodec` trait.
- `sigil-cli/src/p2bin_codec.rs` (new): the AS route's codec, and the
  `sigil-clownlzss-sys` dependency it needs.
- `sigil-cli/src/main.rs`, every hunk: `mod p2bin_codec;` beside `mod tree_class;`;
  `Opt` gains `attached` and an `attached()`
  constructor; `unlisted_option` matches `-p=...` as `-p`; the AS row's `usage` (two
  lines plus a note naming `-p` and `-z` as words, because the usage gate's extractor
  ignores a piece containing `=`) and `options` (`attached("-p")`, `attached("-z")`);
  `run_asm`'s doc comment, argument loop (`-p` last wins, `-z` repeatable),
  `resolve_layout_placing` call, and tail (`flatten_placing` when `-p` or `-z` is
  given, with `&p2bin_codec::Codec`; the plain zero-filled `flatten` otherwise).
- `sigil-frontend-as/src/eval.rs`: the `restore` arm closes the section when the
  processor changes (below).
- Tests: `sigil-cli/tests/as_driver_placement.rs` (17),
  `as_driver_placement_corpus.rs` (1), `as_second_address_space.rs` (wording).

## A front-end defect the probe matrix found, fixed

`restore` restored the processor without ending the section, as `cpu` does. After
a driver's `restore`, 68000 bytes joined the Z80 section whenever the next `org`
landed on the driver's own counter (`p_cont68k`: `!org 10` after a ten-byte
driver). asl gives them their own record, so p2bin refuses the program (a two-byte
gap); sigil with `-z` accepted it and compressed the 68000 bytes into the blob, the
one probe where sigil built a ROM p2bin refuses. Fixed in `b2d25fea`; both now
refuse with the same sizes. No corpus reaches the shape. **aeon is unmeasured**
here: a `restore` after a Z80 block followed by 68000 bytes with no `org` now opens a
new, Chained section instead of continuing the Z80-cpu one. Its bytes land where the
continuation put them, and the controller's engine byte gates are the measurement.
TAGGED.

## Sonic 1

Reference re-derived, not copied: pinned asl, exit 0, `2 passes`, 0 errors, no log,
`sonic.p` md5 `4f54eb0a983bee02aefe8c3348695a67`; p2bin with `build.lua`'s line gives
`09dadb5071eb35050067a32462e39c5f`, CRC32 `afe05eee`, 524,288 bytes.

**A correction to how every earlier reference was built.** A copy "restored" to
`f6ece657` with `git checkout -- . && git clean -fd` still carries gitignored
files, and `build.lua`'s pre-step writes the DAC samples to gitignored
`sound/dac/{pcm,dpcm}/generated/`. So the phased-overlap note's reference, and this
parcel's first one, built only because the checkout had been built before. From a
bare `git archive` asl stops (`error #10001` at `z80.asm(223)`, `kick.inc`).
Running `build.lua`'s two conversion calls on the archive regenerates every `.pcm`,
`.dpcm` and `.inc` byte for byte (only a `hashes.lua` cache differs, and nothing
includes it), and the reference is unchanged. The corpus test runs that pre-step.

sigil, same tree, same line: md5 `09dadb5071eb35050067a32462e39c5f`, CRC32
`afe05eee`, 524,288 bytes, **0 bytes different**, in 1.8 s. The stored driver at
`[0x72E7C, 0x745DC)` (the window read from asl's records) decompresses through
sigil's decompressor to asl's `0x1BC6`-byte Z80 records.

## Sonic 2

`build.lua`, run unmodified in a copy, gives `9feeb724052c39982d432a7851c98d3e`,
CRC32 `7b905383`, 1,048,576 bytes, the census's value, re-derived. A clean tree
plus that run's generated inputs, rewritten by the census's `stub.py` mode C (108
edits, every residual class except the driver, byte-neutral under asl + p2bin per
the census), assembled by sigil with `-p=0
-z=0,saxman-bugged,Size_of_Snd_driver_guess,after`: md5 `9feeb724`, CRC32
`7b905383`, 1,048,576 bytes, **0 bytes different, no window**. The census's single
excluded window, the compressed-driver hole, is gone. The move.w size patch and
`fix_header` are out of scope and change zero bytes here (census), and they were not
run on sigil's image.

## S3K

The front end stops first (189 rows plain, 172 through the `-D` wrapper root), so the
shape is exercised by `probes/p_s3k.asm`, built from skdisasm's own text: `notZ80`
and the `org` macro verbatim, the driver file's RAM phase blocks and
`!org z80_SoundDriverStart` rewind, both driver entries and both exits verbatim,
small Z80 bodies. With both `-z` (`kosinski`, and `uncompressed` as buildS3), sigil's
image equals p2bin's (268 bytes, CRC32 `eacc2d60` and `c3956dd1`). With only the
first `-z`, sigil refuses the second driver at its `!org 1300h`.

## Probe matrix: sigil against p2bin

`cmp_probes.py`, final binary `ea1531bf`, 39 cases: 20 same image, 11 both refuse,
8 sigil refuses where p2bin does not, 0 sigil accepts where p2bin refuses. The 8:

| case | p2bin | sigil | why |
|---|---|---|---|
| `p_two`, only `-z=0` | punts driver 2 onto the image | refused at its org | required by the brief |
| `p_two`, only `-z=1300` | punts driver 1 | refused at its org | same |
| `p_s3k`, only `-z=0` | punts driver 2 | refused at its org | same |
| `p_after`, `-z=40` | ignores it, punts the driver | refused (stage 1, uncovered space) | a `-z` that names nothing |
| `p_hex`, `-z=16` | ignores it, punts the driver | refused (stage 1) | same |
| `p_first` | places the driver | "names no second address space" | stage 1 reads an `!org 0` at counter 0 as no re-base, so this driver is image bytes to sigil; OPEN |
| `p_phase_after` | places both | refused: the phased image block is in the driver's space | stage 1's rule order: "same second CPU stays" precedes "a phase open means image"; OPEN |
| `p_straddle`, `saxman-bugged` | writes the stream | round trip refused | the game would read it back wrong |

## Diagnostic multisets, before and after

Base `69f6a8d1` against new `ea1531bf`, stderr sorted, compared as sets.

| corpus | run | base | new | difference |
|---|---|---|---|---|
| s1disasm | plain | 1 | 1 | the wording, below |
| s1disasm | with `build.lua`'s instruction | 1 | 0 | the refusal is gone; the ROM is the reference |
| s2disasm | raw clean tree | 149 | 149 | none |
| s2disasm | with `build.lua`'s generated inputs | 71 | 71 | none |
| s2disasm | stub C, with the instruction | 1 | 0 | `s2.sounddriver.asm(248):6: error: section `sec0#2` [0x0, 0x1308) ...`, stage 1's predicted refusal, now measured and gone |
| skdisasm | `sonic3k.asm` | 189 | 189 | none |
| skdisasm | wrapper root, with buildSK's instruction | 172 | 172 | none |

```
< sound/z80.asm(9):3: error: section `sec0#2` [0x0, 0x1BC6) is assembled for the Z80 at origin 0x0, in a second address space this org opens outside the ROM image; the assembler cannot yet place a second address space into the ROM
> sound/z80.asm(9):3: error: section `sec0#2` [0x0, 0x1BC6) is assembled for the Z80 at origin 0x0, in a second address space this org opens outside the ROM image, and no -z instruction places it into the ROM
```

## Half-fix matrix and red-first evidence

**Method.** `mutate.py` (in the evidence directory) applies each row's literal
substitutions to committed files, refusing unless each matches exactly once and
refusing to start on a dirty tree. It quotes every marked line back from disk,
runs `sigil-cli --test as_driver_placement --test as_driver_placement_corpus
--test as_second_address_space` (and `sigil-clownlzss-sys --test accurate_vectors`
where the row touches a compressor), runs the rebuilt binary on a witness probe,
restores each file with `git show HEAD:<path>`, and requires an empty `git status
--porcelain`. It ran in full at `8ebbeb9e` (`matrix.log`), and again in full at
`ea992d37` (`matrix2.log`), because the codec refactor moved the lines M1, M2 and
M6 mutate: every earlier claim is re-established on the code that lands, not
carried over. The table quotes the second run. Every row reported `APPLY rc=0` and
an empty status after the restore.

| id | half-fix | mutated line, quoted from disk | red | witness |
|---|---|---|---|---|
| M1 | **`-z` parsed, blob stored uncompressed** | `p2bin_codec.rs:15: BlobFormat::Kosinski => data.to_vec(), // MUTATION M1` | `kosinski_and_saxman_bugged_store_the_bytes_p2bin_stores`, `both_blobs_of_a_two_driver_program_are_placed_in_either_order`, `a_blob_larger_than_its_reservation_is_refused_with_both_sizes`, `a_stream_the_game_would_read_back_wrong_is_refused`, corpus `sonic_1_builds_to_the_reference_rom_byte_for_byte` | refused by the round trip: `decompressor returns 0xD bytes against 0xA` |
| M2 | **wrong Kosinski variant (clownlzss optimal)** | `p2bin_codec.rs:15: BlobFormat::Kosinski => sigil_clownlzss_sys::compress_kosinski(data).expect("kosinski"), // MUTATION M2` | the same five, **corpus included**: the round trip passes (both variants decompress), only the byte compare sees it | exit 0, `... FF 0B 10 .. 19 00 F0 00 FF AA BB` (p2bin: `... 00 F0 00 00 AA BB`) |
| M2b | wrong Kosinski variant (Sega's 0xFD match cap corrected) | `accurate.rs:28: const KOS_MAX_MATCH: usize = 0x100; // MUTATION M2b` | vectors `kosinski_authentic_matches_p2bin_on_every_vector`, `..._past_the_0xa000_boundary`. **The CLI and corpus tests stay green**: neither the probes nor Sonic 1's driver has a match the cap changes, so only the vectors see this variant | none |
| M3 | **`after` read as `before`** | `blob.rs:200: "after" => BlobInsert::Before, // MUTATION M3` | 11 CLI tests and the corpus test | refused: `only 0x8 are reserved ... the code before it` |
| M4 | **stored at its own Z80 address, not the gap** | `blob.rs:428: (z.address, reserved, where_) /* MUTATION M4 */` | 9 CLI tests and the corpus test | refused: `the stored stream at [0x0, 0xA) overlaps section sec0` |
| M5 | **overflow accepted** | `blob.rs:437: ... reserved.filter(\|&r\| false && (stored.len() as i64) > r) /* MUTATION M5 */` | `a_blob_larger_than_its_reservation_is_refused_with_both_sizes`, `code_written_after_a_restore_is_not_part_of_the_blob` | still refused, by the overlap check, without the sizes the test requires |
| M6 | **`-p` ignored** | `main.rs:490: match sigil_link::flatten_placing(&resolved, &linked, &blobs, 0x00, &p2bin_codec::Codec) /* MUTATION M6 */ {` | 8 CLI tests and the corpus test | exit 0, pad `00` where p2bin writes `FF` |
| M7 | **only the first of two `-z` honoured** | `main.rs:376: Ok(blob) => if blobs.is_empty() { blobs.push(blob) }, // MUTATION M7` | `both_blobs_of_a_two_driver_program_are_placed_in_either_order` | refused at `p_two.asm(20)` |
| M8 | **an uncovered second space placed anyway** (both layers) | `relax.rs:488: .filter(\|space\| !(space.cpu == sigil_ir::Cpu::Z80 && !placed_origins.is_empty())) /* MUTATION M8 */` and `blob.rs:513: if false && matches!(r.space, AddressSpace::Foreign { .. }) && !claimed[idx] { // MUTATION M8` | `a_second_space_no_instruction_names_is_refused_at_its_org` | **exit 0**, driver 2 silently dropped |
| M8a | the layout layer alone | the `relax.rs:488` line, `/* MUTATION M8a */` | the same test, through its S3K assertion (`origin 0x1300` is stage 1's wording) | still refused, by the placement layer: `the Z80 code at [0x1300, 0x1306) is outside the ROM image and no -z instruction places it` |
| M8b | the placement layer alone | the `blob.rs:513` line, `// MUTATION M8b` | **none, by design**: stage 1's layout refusal still fires | refused at `p_two.asm(20)` by stage 1 |
| M9 | round-trip check removed | `blob.rs:458: Ok(back) if back == blob \|\| true => {} // MUTATION M9` | `a_stream_the_game_would_read_back_wrong_is_refused` | exit 0, the straddling stream written |
| M10 | `restore` does not end the section | `eval.rs:6026: if false && self.state.cpu != before { // MUTATION M10` | `code_written_after_a_restore_is_not_part_of_the_blob` | exit 0, `C0 C1` inside the blob |
| M11 | saxman-bugged's junk byte always `00` | `accurate.rs:393: let junk = 0x00u8; // MUTATION M11` | vectors `saxman_bugged_matches_p2bin_on_every_vector`, `the_vectors_cover_both_parities_of_the_saxman_junk_byte`; the CLI tests stay green (their streams are all even-length) | none |
| M12 | `-p=..` / `-z=..` not matched by `main`'s option scan | `main.rs:328: o.name == arg /* MUTATION M12 */` | all 17 CLI tests and the corpus test | exit 2, `unexpected argument '-p=FF'` |

The brief's seven rows: stored uncompressed (M1), the wrong Kosinski variant (M2,
and M2b for Sega's own mistakes), the wrong offset both ways (M3, M4), overflow
accepted (M5), `-p` ignored (M6), only the first `-z` (M7), an uncovered second
space silently placed (M8). Each is red. M8a and M8b show the two layers each hold
the last one alone.

## Scoped suites and clippy

`SIGIL_ALLOW_PARTIAL=1`, `AEON_DIR`, `SIGIL_STRICT_GATE` and `EMPYREAN_SUITE_ROOT`
unset, `CARGO_TARGET_DIR` in the parcel's scratch directory, `cargo test --release
-p <crate> --no-fail-fast -- --nocapture`, one crate at a time (`suites.py`; each
log stamped with pwd/HEAD/branch and checked for this parcel's own test names).

| crate | at | passed | failed | ignored | skip lines |
|---|---|---|---|---|---|
| sigil-clownlzss-sys | `8ebbeb9e` (unchanged since) | 41 | 0 | 0 | 0 |
| sigil-frontend-as | `8ebbeb9e` (unchanged since) | 774 | 0 | 0 | 0 |
| sigil-link | `ea992d37` | 146 | 0 | 0 | 0 |
| sigil-cli | `ea992d37` | 806 | 0 | 1 | 386 |

At `8ebbeb9e` the `sigil-cli` suite had one red, `crate_graph_is_one_way`:
`sigil-link library must depend only on sigil-ir + sigil-span`, left
`["sigil-clownlzss-sys", "sigil-ir", "sigil-span"]`. That was this parcel's own
defect (the first version linked the compressors into the linker), fixed in
`ea992d37` by the `BlobCodec` trait, after which the suite is green. The ignored
row is `sigil_diff_reports_byte_identity`, which reads the aeon tree and was
ignored before this parcel. The 386 skip lines are the partial run's
reference-dependent rows, left unmeasured by instruction (no aeon tree): every
engine byte gate among them is the controller's strict landing run's to measure.

Clippy (`cargo clippy --release -p <crate> --all-targets -- -D warnings`) exits 0 for
all four crates at `ea992d37`: `sigil-clownlzss-sys`, `sigil-link`,
`sigil-frontend-as`, `sigil-cli`.

## Open, and why

1. **Whether the drivers play** is a runtime question; no emulator was touched.
   TAGGED for the controller.
2. **aeon**, for the `restore` fix, above. TAGGED.
3. **`p_first`**: a driver that is a program's first content, entered by an
   `!org 0` at counter 0, is image bytes to stage 1; without `-z` it is placed at ROM
   0 silently, which is also p2bin's punt. Not in any corpus.
4. **`p_phase_after`**: a phased Z80 image block right after a driver's return
   `org` is put in the driver's space and refused (loud). Not in any corpus.
5. **`before` and asl's record splits.** sigil's runs end only at seeks,
   reservations and section changes; asl also splits contiguous records elsewhere.
   `after` never depends on that; `before` does (the stream goes at the previous
   record's START), and in S3K the previous record starts at an `org`, where sigil's
   run starts too. A `before` reservation spanning one of asl's other splits would
   place differently; no corpus has one.
6. **The four unimplemented formats** are refused by name, not guessed.
7. **Sonic 2's `amend_sound_driver_size`** and `fix_header`: out of scope.
8. **`origin/master` moved after the authorized merge** (`ac639ebe` to `2c5b3608`:
   the string-escape fix and the listing digest). Its new commits touch three files
   this branch changes (`main.rs`, `eval.rs`, `sigil-link/src/lib.rs`). A trial
   merge (`git merge-tree --write-tree HEAD origin/master`, no ref moved) is clean,
   result tree `e93584dc`. The merged tree is unmeasured here; the landing gate is
   its measurement. Not merged a second time.

## Things in the brief that turned out wrong

1. "How the size constant is read (it is a symbol in asl's output)": p2bin never
   reads it. The name is only printed.
2. The census's "0x4E if the encoded length is odd": the junk byte is always
   appended, `0x00` after an even stream.
3. "What p2bin does when the compressed blob exceeds the reserved size": it refuses,
   exit 1, and deletes its output. The strings in the binary say so and the probes
   confirm it.
4. The reference built "exactly as the S1 note did" depends on gitignored generated
   files that `git clean -fd` does not remove (above).
5. "Implement exactly the set these build scripts name": the scripts also name
   `kosinski-optimised` and `saxman-optimised` behind their `improved_*` switches.
   Taken as the three they select at their shipped settings; the other two are refused
   by name.
