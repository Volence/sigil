# Sonic 2 residual census: what the 149 rows are, and the wall behind them

2026-09-11, measurement parcel, branch `parcel/s2-residual-census`, base `66d2ed86`.
**No assembler source was changed.** Every stub below is measurement scaffold applied to
private copies of the disassembly, never to sigil. Evidence, raw row multisets, probes,
scripts and every tool digest are in `2026-09-11-s2-residual-census/` beside this note.

## Headlines

1. **The 149 rows reproduce exactly, and 78 of them are not source work.** A clean
   checkout of `s2disasm` lacks the files `build.lua` generates BEFORE it assembles
   (Saxman-compressed songs, PCM/DPCM sample conversions and their `.inc` files). With those
   inputs present the run has **71 rows in 9 classes**: 53 in the Z80 sound driver, 16 in
   68000 code, 2 from the end-of-ROM padding macro.
2. **Behind the 71 is the Sonic 1 wall.** With every class rewritten into spellings asl
   assembles to identical bytes (proven: asl+p2bin on that tree give the reference md5),
   sigil stops on one row, the driver's address-0 section overlapping the vector table.
3. **Behind the wall, sigil builds Sonic 2, and it is silently wrong in 2,176 bytes.** With the
   driver moved out of the image, sigil exits 0 with zero diagnostics. Against the reference
   ROM, outside one asserted window (the compressed-driver hole), 2,176 bytes differ in 231
   runs, all of them text. Cause, proven by removing it: **sigil processes no string escape
   sequences** (`\x3B`, `\2`, `\H`, `\A`), so the 35 escape-bearing `charset` lines map the
   wrong characters. Rewrite only those 35 lines and the difference is **0 bytes**, and the
   uncompressed driver sigil assembled equals asl's Z80 record in all 4,872 bytes.
4. **The pinned reference asl cannot assemble Sonic 2.** It truncates macro argument text at
   255 characters and fails with 274 errors. The reference ROM here comes from `s2disasm`'s
   own asl, used with the controls described under Provenance.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | built from `66d2ed86` with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/s2-residual-census/target`, md5 **`f0b79d574dd24f6489ad9a4d0fcdb9c1`**. The shared `target/` was not used. |
| corpus | `s2disasm` `e45ebf332f39987424ca3102e50c717628f71269`, `cp -a` copies in the scratch directory. The shared checkout was read only: `git status --porcelain --ignored` is 0 lines before and after. |
| reference asl (pinned) | `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`**, through `asl_run`. Used for every per-construct probe (all exit 0, footer complete). **Cannot build Sonic 2** (Q0). |
| s2 toolchain asl | `s2disasm/build_tools/Linux-x86_64/asl`, md5 **`0dee1f98e6480a4783d27ffd8b90896f`** (the build OVERSEER-REFERENCE refuses). Used only for the whole-corpus reference build, with controls. |
| p2bin | md5 `4f2fff99c3347bafb93b12d5be1db754` (the same file in s1disasm and s2disasm) |
| saxman | `s2disasm/build_tools/Linux-x86_64/saxman`, md5 `704e5c8c361b9959f45bac3e803f0033` |
| message files | s2: `as.msg afe3933e...`, `cmdarg.msg 25bcbdf8...`, `ioerrs.msg 90829aac...`; s1: `b06b8f30...`, `48315042...`, `8abd3465...` (full digests in `logs/tool-md5s.*.txt`) |
| lua | `/usr/bin/lua`, Lua 5.5.1 |
| **reference ROM** | `build.lua` run unmodified in a copy: **md5 `9feeb724052c39982d432a7851c98d3e`, CRC32 `7b905383`, 1,048,576 bytes.** Reproduced by hand (asl, p2bin, both post steps) byte for byte. |
| sigil images (stubbed) | stub B md5 `04048be02369bad522f5a9b4a9f0eeea`, stub D md5 `29d1b81e332245f1bd06cb3c4a0df773`, each 3,150,600 bytes. Not committed. |

The CRC32 `7b905383` is, from memory, the listed CRC of the retail Sonic 2 REV01 cartridge.
**It was not checked against any database in this parcel: do not cite it as verified.**

Wall-clock times were not recorded (`run_sigil.sh` called `bc`, which is not installed here).

## Q0. Which asl, and why the pinned one cannot answer for this corpus

`build.lua` run with the reference asl swapped into the copy's `build_tools` exits 1; a hand
run through `asl_run` exits **2** with **274 × `error #1010: symbol undefined`**, every one of
them a `JmpTo…` name (`rows/asl-s1build-on-s2.rows`). The two lines that cause it:

```text
s2.asm(62701) jmpTos(1) jmpTosInternal(13) jmpTosInternal2(5) IRP:(2):0: error #1010: symbol undefined
    jmp     (extractJmpToName("JmpTo23_Objec")).l
s2.asm(84172) jmpTos0(1) ... error #1010: symbol undefined
    jmp     (extractJmpToName("JmpTo2_Mar")).l
```

Measured (`scripts/trunc.py`): line 62701's argument string is 260 characters and the
truncated token `JmpTo23_Objec` (of `JmpTo23_ObjectMove`) **ends at character 255**; line
84172's is 532 characters and `JmpTo2_Mar` (of `JmpTo2_MarkObjGone_P1`) **ends at character
255**. The pinned build cuts macro argument text at 255 characters, so every `JmpTo` label
after the cut is never defined and every `jsrto`/`jmpto` that names one is undefined.

So the pinned build is not a usable whole-corpus oracle for Sonic 2. The reference ROM comes
from `s2disasm`'s own asl, with three controls:

* **exit status and pass loop**: three hand runs, each `ASL_EXIT=0`, listing footer
  `2 passes` with no "Additional necessary passes" line, no `s2.log`;
* **run-to-run stability under ASLR** (`randomize_va_space = 2`): the three object files are
  identical (md5 `ecf36eb473c8c8371b945fe1a98fe366`). The refused build's defect is an
  uninitialized read that varies per run, so identical outputs are the check that no declined
  operand reached a byte;
* the **generated inputs** (77 files) are identical whether the pre-steps ran with the pinned
  asl or the s2 asl (`logs/generated-inputs.md5`).

Every value quoted per construct in Q2 comes from the pinned build, not this one.

## Q1. What remains, by class

`rows/sigil-raw.rows` is the 149-row multiset on the clean tree; `rows/sigil-gen.rows` is the
71-row multiset with `build.lua`'s pre-step outputs present. Class census in
`rows-by-class.txt` (`scripts/classify_rows.py`).

### The 78 rows that are missing inputs, not sigil work

| rows | message | cause |
|---:|---|---|
| 31 | `cannot include sound/music/generated/…` | songs are compressed by `build.lua` before assembly |
| 7 | `cannot include sound/DAC/generated/…` | WAV→DPCM conversion output |
| 1 | `cannot include sound/PCM/generated/SEGA.inc` | WAV→PCM conversion output |
| 17 + 17 | `int(): could not evaluate float expression` / `division by zero: 3579545 / 0` | `sounddriver.asm:3905`, `label.sample_rate` from the missing DAC `.inc`, one pair per `dac_sample_metadata` call (17 calls) |
| 2 | `unresolved if condition: Snd_Sega.size …` | `s2.asm:91000`, `:91003`, from the missing PCM `.inc` |
| 1 | `unresolved symbol Snd_Sega.size` | `sounddriver.asm:1620`, same |
| 2 | `unresolved symbol .loop_counter` | `sounddriver.asm:1604` defines it as `pcmLoopCounter(Snd_Sega.sample_rate)`, same |

149 minus 71 is 78, and the rows above add up to 78. Nothing else moved between the two runs.

### The 71 rows, by class

| # | class | rows | area | trigger (real lines) | what asl does (pinned-build probe) | kind | size |
|---|---|---:|---|---|---|---|---|
| 1 | empty `()` from an omitted macro argument | 30 | Z80 driver | `db (withinSameZ80Bank(DATA.pointer, MusicPoint2)<<7)\|((~~DATA.is_compressed)<<5)\|(FLAGS)\|(getZ80BankOffset(DATA.pointer)/2)` (`sounddriver.asm:3817`), expanded by the 30 of 31 `music_metadata` calls that omit FLAGS, e.g. `zMusIDPtr_EHZ: music_metadata Mus_EHZ` (3823); the 31st passes `MusFlag_SlowerOnPAL` and works | `()` is 0: `dc.b $10\|()\|$02` → `12`, `dc.b ()` → `00`, same under Z80 (`p_emptyparen*.asm`) | missing feature (expression parser) | S |
| 2 | digit-led struct member `1upPlaying` | 11 | Z80 driver | `1upPlaying: ds.b 1` (159, 2 rows: lexer + struct), `ld a,(zAbsVar.1upPlaying)` (1678), `ld a,(ix+zVar.1upPlaying)` (2118); 9 references | accepted AS A MEMBER: offsets 1, 2, `len` 3 (`p_digitmember.asm`); refused as a plain label, `error #1020: invalid symbol name`, exit 2 (`p_digitlabel.asm`) | missing feature (lexer, member position and dotted names) | S |
| 3 | IX/IY half registers | 12 | Z80 driver | `ld a,iyl` (1863), `ld e,ixl` (2258), `add a,ixl` (3687) | `CPU Z80UNDOC` forms: `FD 7D`, `FD 8C`, `DD 5D`, `DD 54`, `DD 6F`, `DD 60` (`p_ixhalf.asm`) | missing feature (undocumented Z80; `z80undoc` maps to plain `Cpu::Z80`, `lib.rs:111`) | M |
| 4 | memory-form shift with no size | 8 | 68000 | `asl y_vel(a0)` (36098), `asr y_vel(a0)` (37592), `asr y_vel(a1)` (46924) | word: `E1E8 001A`, identical to `asl.w` (`p_memshift.asm`) | missing feature (`m68k_default_size`, `eval.rs:11232`, has no entry) | S |
| 5 | `pushv` / `popv` | 2 | 68000 | `pushv ,SonicDplcVer` (69387), `popv ,SonicDplcVer` (69774) | saves and restores a `set` symbol: `02 04 02` (`p_pushv.asm`) | missing feature (directive) | S |
| 6 | macro argument text with a digit-led word | 3 | 68000 | `Pal_SS1_2p:palette Special Stage 1 2p.bin` (3935 to 3937) | substitutes the text verbatim: `dc.b "Special Stage 1 2p.bin"` (`p_macroarg2p.asm`) | **sigil defect**: arguments are lexed, AS arguments are text | S to M |
| 7 | `charset '\H',…` | 2 | 68000 | `charset '\H',"\x39\x37\x38"` (14480, 14606) | `\H` is AS's apostrophe escape: `'\H'` → `0027`, `"\H"` → `27` (`p_charsetH.asm`) | the loud face of class 10 | with 10 |
| 8 | `lastbit` | 2 | 68000 (reported at `s2.macrosetup.asm(20)`/`(22)`) | call site `cnop -1,2<<lastbit(*-StartOfRom-1)` (`s2.asm:91263`) | index of the highest set bit: `lastbit(5)` → 2, `2<<lastbit($FFFEB)` → `$100000` (`p_lastbit.asm`) | missing builtin (not in the front end at all); also a **location defect**: the rows name the macro body, not the call | S |
| 9 | `shared` | 1 | 68000 | `shared movewZ80CompSize` (91275) | writes the symbol to the `-c` share file; without `-c`, `warning #30: no sharefile created, SHARED ignored` (`p_shared.asm`) | missing directive; its only consumer is the post-p2bin patch (Q3) | S |

30 + 11 + 12 + 8 + 2 + 3 + 2 + 2 + 1 = 71. Driver 53 (classes 1 to 3), 68000 18.

**And one class with no rows at all, found by the image compare (Q4):**

| # | class | rows | area | trigger | what asl does | kind | size |
|---|---|---:|---|---|---|---|---|
| 10 | string escape sequences | **0 (silent)** | 68000 text | `charset 'B',"\4\8\xC\4\x10…"` (10273), `charset '@',"\27\30\31…"` (11624), `charset 'H',"\xB\4\x11…"` (70904): 35 lines | `\xHH` hex, `\NNN` decimal, `\H`/`\h` apostrophe, `\A` bell: `"\x41\x42"` → `41 42`, `"\2\8\12"` → `02 08 0C` (`p_escx`, `p_escnum`, `p_escA`, `p_esch`) | missing feature, **exit 0 and wrong bytes** | S to M |

**Line 2674 is not a missed row.** `1upPlaying` appears 11 times in the driver and rows name 10;
the eleventh, `ld a,(zAbsVar.1upPlaying)` at 2674, sits inside `if OptimiseDriver` (line
2657, `OptimiseDriver = 0`) and is never assembled. The ten live sites are the positive control
that the instrument does fire on this name.

### The unresolved-symbol name set, both directions against 2026-09-09

| name | 09-09 | raw today | with pre-step outputs |
|---|---:|---:|---:|
| `zAbsVar.1upPlaying` | 6 | 6 | 6 |
| `ixl` | 4 | 4 | 4 |
| `ixu` | 4 | 4 | 4 |
| `zVar.1upPlaying` | 3 | 3 | 3 |
| `.loop_counter` | 2 | 2 | **0** |
| `iyl` | 2 | 2 | 2 |
| `iyu` | 2 | 2 | 2 |
| `Snd_Sega.size` | 1 | 1 | **0** |

Raw today equals 09-09 in both directions. With the pre-step outputs, two names leave (both
missing-input cascades) and none arrive. The 09-09 note recorded only names and counts, so a
row-level set diff against it is impossible; today's raw multiset is committed so the next one
can be.

## Q3. The sound-driver placement

**Same wall.** With classes 1 to 10 stubbed (stub A, byte-neutral, below), sigil's only row is

```text
s2.asm(86):2: error: sections `sec0` [0x0, 0x2CA) and `sec0#2` [0x0, 0x1308) overlap in the image (colliding pins)
this error list may be incomplete: sigil stopped at layout, so link and the image checks did not run
```

`s2.asm:86` is `dc.l System_Stack`, the first vector: the same wrong-located shape as Sonic 1.
The Z80 blob in asl's object file is `cpu 81 [0x0, 0x1308)`, 4,872 bytes in 2 records, sitting
between the 68000 run ending at `0xEC0E8` (= `Snd_Driver`) and the next at `0xED100`
(`scripts/pfile.py`). The shipped scan reports its first pair only; how many sections the
driver actually crosses was not measured here (it needs the `crates/` scaffold the Sonic 1
note used, which this parcel may not add).

**`<compression>` is `saxman-bugged`**: `improved_sound_driver_compression = false`
(`build.lua:10`), so `build.lua:185` picks `"saxman-bugged"` over `"saxman-optimised"`.

**What p2bin does with it, measured:**

* The stored stream at `Snd_Driver` is `$F64` = 3,940 bytes (`comp_z80_size` in `s2.h`). A
  Python port of the game's own `DecompressSoundDriver` (`s2.asm:90763-90849`, including its
  exit rule: the byte that brings the counter to zero is read and never used) decodes those
  3,940 bytes to 4,872 bytes **equal to asl's Z80 record** (`scripts/saxdec.py`).
* The 180 bytes `[0xED04C, 0xED100)` after it are fill `00`; the DAC samples begin at
  `0xED100` by `cnop -Size_of_DAC_samples, $8000`.
* The encoder: upstream p2bin (`Clownacy/p2bin` `main.c`, fetched from the default branch; **not
  verified to be the revision the binary was built from**) implements `saxman-bugged` as
  `Encode(LZSS_ReadByte, NULL, output_file)` followed by one garbage byte, `0x4E` if the
  encoded length is odd, else `0x00`. The measurement agrees: `saxman -a` (the same accurate
  `lz_comp2` encoder) produces 3,939 bytes that are a byte-exact prefix of the ROM's stream,
  3,939 is odd, and the ROM's 3,940th byte is `0x4E`.
* **An optimal encoder does not reproduce it.** `saxman` without `-a` (clownlzss) gives 3,914
  bytes, differing at byte 277. Sigil's vendored clownlzss Saxman was not run here; it is the
  optimal family, so expect the same result, but that is unmeasured.

**The post-p2bin step, exactly** (`build.lua:138-167`, `amend_sound_driver_size`):

1. asl runs with `-c`, so `shared movewZ80CompSize` (`s2.asm:91275`) writes `s2.h`:
   `/* s2.asm-Include File for C Program */`, `#define movewZ80CompSize 0xEC04E`,
   `/* Ende Include File for C Program */`.
2. p2bin is given `s2.h` as a fourth argument and writes `comp_z80_size 0x%lX ` **over the
   first bytes of that file, in place, without truncating it**. Measured, the first line becomes
   `comp_z80_size 0xF64 le for C Program */`.
3. `build.lua` reads `s2.h` line by line: on a line containing `comp_z80_size` it takes the
   first `0x%x+` after the match (`0xF64`), on a line containing `movewZ80CompSize` likewise
   (`0xEC04E`).
4. If both were found it opens `s2built.bin` `r+b`, seeks to **`movewZ80CompSize + 2` =
   `0xEC050`** and writes `comp_z80_size` as a **big-endian 16-bit word**. That is the
   immediate of `move.w #Snd_Driver_End-Snd_Driver,d7` (`s2.asm:90766`, bytes `3E3C 0F64`),
   which the decompressor uses as its byte count.
5. It deletes `s2.h`. Then `common.fix_header` writes the ROM size minus 1 as a big-endian
   32-bit value at `0x1A4`, and the 16-bit sum of the big-endian words from `0x200` at `0x18E`.

**At this revision both edits change zero bytes.** The assembled immediate is already `$F64`,
because `!org (Snd_Driver+Size_of_Snd_driver_guess)` (`s2.asm:90862`) pins `Snd_Driver_End`,
and the compressed size is also `$F64`. The source header already carries `D951` and
`000FFFFF`. The p2bin output, the amended image and the final image are byte-identical
(`logs/ref2_build.log`). **The patch becomes load-bearing the moment the compressed size
differs from the guess**: any driver edit, or `improved_sound_driver_compression`. What
p2bin does when the stream is LARGER than the reservation was not measured.

What sigil would have to reproduce: place the compressed blob at `Snd_Driver`; compress it with
the accurate encoder plus the garbage-byte rule; check it against the `$F64` budget; patch the
`move.w` immediate at `movewZ80CompSize+2` with the stream length. Sigil holds the symbol
itself, so it needs no side file. Sonic 1's `build.lua` has no such patch (only `fix_header`),
so this step is specific to Sonic 2.

**The fill byte is not a Sonic 2 item.** Sonic 2 passes `-p=0` and sigil's AS route flattens
with `0x00`; every gap in the image compared equal (Q4). The Sonic 1 note's fill-byte item
(`-p=FF`) does not arise here.

## Q4. Silent wrong bytes

**Stubs.** `scripts/stub.py` rewrites each refused construct into a spelling the pinned build's
probes show gives the same bytes, asserting the original text at every site, and keeping the
file's line count so a byte maps back to the original line (`logs/stub-*.edits`):

| stub | adds | checked by |
|---|---|---|
| A | classes 1 to 9 (palette file copied under a name with no digit-led word; `charset $27`; `.w`; an explicit save/restore through `:=`; `OneUpPlaying`; `,0`; the half-register lines as `db` of asl's own listed bytes; `cnop -1,$100000`; `shared` commented out) | asl + p2bin on stub A give **`9feeb724…`**: byte-neutral |
| B | A + driver out of the image: `!org 0` → `!org $300000`, `phase 0`, `dephase` at the end of the driver | sigil only (asl's phase range rule forbids this; not attempted) |
| C | A + the 35 escape-bearing `charset` lines expanded into numeric one-character `charset $XX,$YY` lines | asl + p2bin on stub C give **`9feeb724…`**: byte-neutral |
| D | C + the stub B placement | sigil only |

Stub A's `shared` stub also disables the post-p2bin patch; it is byte-neutral only because that
patch is a no-op at this revision.

**The compare** (`scripts/compare.py`). One window, derived and asserted, never assumed:

```text
WINDOW driver hole [0xEC0E8, 0xED04C) len 0xF64  (Snd_Driver from the p-file, guess from s2.constants.asm)
WINDOW asserted: exactly one window, extent 0xF64 bytes = 0.376% of the reference
CONTROL planted outside at 0x200,0x7FFF0,0xFFFFE and inside at 0xEC100 -> reported [('0x200', 1), ('0x7fff0', 1), ('0xffffe', 1)]
CONTROL passed: the instrument reports planted differences and only the window hides one
```

**Stub B** (`logs/compare-stubB.txt`): **2,176 bytes in 231 runs** outside the window, over 141
source lines, every one a `dc.b` string under an escape-bearing `charset`. For example:

```text
[0x00874B,0x008752)  sigil 5c 35 5c 5c 31 32 32   ref 22 2a 22 2f 1e 29 21   s2.asm 11632  dc.b "EMERALD HILL"
[0x035CC8,0x035CCD)  sigil 54 57 37 44 78         ref 02 10 04 07 01         s2.asm 71577  dc.b "TWICE"
```

`5C` is `\`, and the digits beside it are the digits of an escape: sigil maps each character
to a character of the UNPROCESSED mapping string. The minimal form is `p_charsetx.asm`:
`charset 'A',"\x10\x11\x12"` then `dc.b "ABC"` gives asl `10 11 12` and sigil `5C 78 31`
(`\`, `x`, `1`), exit 0.

**Stub D** (`logs/compare-stubD.txt`), which differs from B only in those 35 lines:

```text
DIFF outside the window: 0 bytes in 0 runs
DRIVER at 0x300000, 4872 bytes vs asl Z80 record: 0 bytes differ
```

So the escape defect is the whole of B's difference. The zero has two positive controls: the
planted bytes the same instrument reports, and stub B's 2,176 bytes it found.

**What this compare can see, and what it cannot.** It covers the 68000 image and the
uncompressed driver. It cannot see the Saxman stream (sigil produces none) or the post-p2bin
patch (a no-op here). Stubbed lines carry asl's bytes by construction, so how sigil would
ENCODE the half-register forms, `lastbit` and the rest is the probes' finding, not this one's.

**Beyond the corpus, the probes show the defect is general**, with exit 0 each time:
`dc.b "\x41\x42"` → sigil `5C 78 34 31 5C 78 34 32`, asl `41 42`; `dc.b "\A"` → sigil
`5C 41`, asl `07`; `dc.b "\h"` → sigil `5C 68`, asl `27`. The only emitting strings in the
Sonic 2 sources that carry escapes are the 35 `charset` lines; the other backslash strings are
`\{…}` interpolation in `message`/`fatal` text and name composition (`scripts/bslines.py`).

## Recommended order of work

Shared with Sonic 1 marked **[S1]**, with Sonic 3 & Knuckles **[S3K]** (source census,
`crossgrep.txt`, with a control pattern that matches in every tree).

1. **String escapes** (class 10, absorbs class 7). First because it is silent and it is the
   only thing between a stubbed build and a byte-identical one. S to M. Neither S1 nor S3K
   writes an escape-bearing string (0 each).
2. **The small AS features**, each S: empty `()` (30 rows), memory-shift default size (8;
   **[S3K]** 76 sites), `lastbit` (2, and give its rows the call site; **[S3K]** 1 site),
   `pushv`/`popv` (2), `shared` (1, accepted, and the input to item 5's patch), digit-led struct
   members (11), macro arguments as text (3).
3. **Z80UNDOC half registers** (12 rows), M: every 8-bit form with `ixh/ixl/iyh/iyl` (AS spells
   them `ixu/ixl/iyu/iyl`), front end and Z80 backend.
4. **Placement stage 1** **[S1] [S3K]**: an address space per section, overlaps per space, a
   right-located diagnostic (already dispatched as `parcel/s1-driver-space`).
5. **Placement stage 2**, per the owner card `d-30` **[S1] [S3K]** for the mechanism, plus the
   Sonic 2 specifics: the accurate Saxman encoder with the garbage-byte rule (a port of
   `lz_comp2`'s `Encode`, not clownlzss), the `$F64` budget check, and the
   `movewZ80CompSize+2` patch. M to L. The acceptance gate the Sonic 1 note proposes
   (decompress round trip plus budget, plus this windowed compare) applies unchanged, and
   `scripts/saxdec.py` is the round-trip half for Saxman.
6. **`build.lua`'s pre-steps**, unsized, a toolchain question for the owner: the songs are
   assembled by asl in a `phase $1380` wrapper and compressed with `saxman -a`, and the WAVs
   converted by Lua. Sigil has no route for either; today they must be run by `build.lua`
   before sigil sees the tree.

## Things in the brief that turned out wrong

1. **"The run reached link and reported `ROM size is $F9198 bytes`."** The line is the source's
   own `message` directive (`s2.asm:91272`), printed by the front end; the same run's stdout
   says `sigil stopped at the front end, so layout, link and the image checks did not run`.
   This was measured on today's binary, not the 09-09 one. But the evidence cited then was this
   same front-end line, so it could not have shown a link either way. The `d-29` card's "gets
   all the way through to producing a cartridge image for the first time" rests on it too.
2. **"Suggests the residual is Z80 sound-driver work."** Half right. 53 of the 71 are in the
   driver, but they are three AS-compatibility features, not placement. Of the eight names,
   `.loop_counter` and `Snd_Sega.size` were never driver work: they are missing generated
   inputs. And 78 of the 149 rows were missing inputs.
3. **"Several parcels have landed since, so 149 may no longer be the number."** It is still
   149, member for member in the name set.
4. **"Use `s1disasm`'s asl."** It cannot assemble Sonic 2 (Q0). Recorded as BLOCKED for the
   whole-corpus reference; the s2 build was used with controls, the pinned build for every
   per-construct value.
5. **"The post-step is a second out-of-band edit of the ROM, and sigil would need to reproduce
   it."** True as a requirement, but at this revision it changes zero bytes (and so does
   `fix_header`). Only a driver whose compressed size differs from the guess exposes it.
6. **"Windowed around the driver hole, the fill byte."** One window is enough for Sonic 2: its
   fill byte is `0x00`, the same as sigil's.

## Open, and why

* **Whether the driver plays** is a runtime question. TAGGED for the controller; no emulator
  was touched.
* **The overlap population** for Sonic 2 is unmeasured (needs a `crates/` scaffold).
* **p2bin's source revision** against the binary (`4f2fff99…`) is unverified; the behaviour it
  describes is confirmed by the measurement.
* **Sigil's own clownlzss Saxman** was not run against the driver.
* **The retail CRC match** is from memory (Provenance).
* **What p2bin does with a stream larger than `Size_of_Snd_driver_guess`**, not measured.

## Reproducing

```text
CARGO_TARGET_DIR=<on disk, not the shared target/> cargo build --release --bin sigil
cp -a s2disasm ref; cp -a s2disasm luaref            # never build in s2disasm itself
(cd luaref && lua build.lua)                          # stock tools: the reference ROM
bash scripts/ref2_build.sh      # three hand asl runs, p2bin, both post steps, cmp with build.lua
bash scripts/ref_build.sh       # the pinned asl in a copy: exits 2, 274 undefined
bash scripts/mk_gen_corpus.sh   # a clean copy plus the pre-step outputs
bash scripts/run_sigil.sh <copy> <out>                # raw: 149 rows; with outputs: 71
bash scripts/probe.sh p_*.asm   # each construct: pinned asl via asl_run, then sigil --hex
bash scripts/stub_run.sh        # stubs A/B, sigil on both, asl on A (byte-neutrality)
bash scripts/stub_run2.sh       # stubs C/D, asl on C, sigil on D, windowed compare
python3 scripts/saxdec.py <rom> 0xEC0E8 0xF64 <z80 record>   # the stream round-trips
```

The scripts carry their scratch paths; the probe sources are in `probes/`.
