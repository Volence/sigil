# The AS corpus at 2026-09-16: both disassemblies build whole, and the headline rests on two no-ops

2026-09-16, measurement parcel `AS-CORPUS-CENSUS`, branch `measure/as-corpus-census`,
base `72aca2e2` (master, read at start). **No sigil source was changed.** Every tree
below is a private copy in `/home/volence/sonic_hacks/.scratch/as-corpus-census/`;
the shared `s1disasm`, `s2disasm` and `skdisasm` checkouts were only read
(`logs/corpus-readonly-proof.txt`). Evidence, raw row multisets, set diffs, scripts,
logs and every tool digest are in `2026-09-16-as-corpus-census/` beside this note.

## Headlines

1. **Both corpora assemble whole and byte-identical, from a PRISTINE checkout, with no
   stub tree.** Sonic 1 exits 0 with zero diagnostics; Sonic 2 exits 0 with one
   warning. The 2026-09-11 census's stub C scaffold is no longer load-bearing for the
   image.
2. **The whole 71-row, 9-class source residual the census measured is gone, and so is
   the address-0 overlap wall and the silent string-escape class.** Every class is
   pinned to a landed commit below.
3. **What remains is not assembler work.** From a pristine checkout with no pre-steps,
   Sonic 1 reports 14 rows and Sonic 2 reports 78 rows plus 1 warning, and **every one
   of them is a build input `build.lua` generates before it assembles**. There is no
   class (b) row, no class (c) wall and no class (d) byte at the shipped settings.
4. **And that last clause is the catch. The byte-identity result is load-bearing on two
   build-script steps that happen to change zero bytes at the shipped assembly-option
   settings.** Flip one switch the corpus itself offers and sigil's image stops being
   the toolchain's, at exit 0, with no diagnostic: the ROM header checksum on both
   corpora, and Sonic 2's compressed-driver size immediate. A third flip makes sigil
   refuse a construct asl assembles. Eleven flips, three distinct faults.
5. **S3K is down to 120 rows in four named classes** (from 172), which is where the next
   piece of loud work actually is.

## Provenance

Every instrument by md5. Full digests in `logs/tool-md5s.txt`.

| Instrument | Identity |
|---|---|
| sigil | built from `72aca2e2` with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/as-corpus-census/target`, md5 **`878ed5704aac873f47290b7b1a9524c9`**. `sigil --version` reports `72aca2e2abfb455d401fa679e089d679bf329b79`. The shared `target/` was not used and no cargo command ran in a shared checkout. |
| corpora | `s1disasm` `f6ece657c1cf253404312137dfcb8ec15fa42318`, `s2disasm` `e45ebf332f39987424ca3102e50c717628f71269`, `skdisasm` `2fcd861c208f342b6d14df694c6422c74f20a4be`. Each extracted by `git archive` into the scratch directory, so the copies carry the tracked blobs and nothing else. |
| reference asl (pinned) | `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`**. Used for every Sonic 1 and S3K reference build. |
| s2 toolchain asl | `s2disasm/build_tools/Linux-x86_64/asl`, md5 **`0dee1f98e6480a4783d27ffd8b90896f`** (the build `OVERSEER-REFERENCE` refuses). Used only for the Sonic 2 whole-corpus reference, with the controls below, exactly as the census did. |
| p2bin | md5 `4f2fff99c3347bafb93b12d5be1db754`, the same file in all three disassemblies |
| saxman | `s2disasm/build_tools/Linux-x86_64/saxman`, md5 `704e5c8c361b9959f45bac3e803f0033` (run by `build.lua` only) |
| message files | s1: `as.msg b06b8f30…`, `cmdarg.msg 48315042…`, `ioerrs.msg 8abd3465…`; s2: `afe3933e…`, `25bcbdf8…`, `90829aac…` |
| lua | `/usr/bin/lua` md5 `6c07a63a91719d9b42a76b2c342efedc`, Lua 5.5.1 |
| python | 3.14.7 |
| **Sonic 1 reference ROM** | `build.lua` run unmodified in a copy: md5 `09dadb5071eb35050067a32462e39c5f`, **CRC32 `afe05eee`, 524,288 bytes** |
| **Sonic 2 reference ROM** | `build.lua` run unmodified in a copy: md5 `9feeb724052c39982d432a7851c98d3e`, **CRC32 `7b905383`, 1,048,576 bytes** |
| **S3K reference ROM** | `buildSK.lua` run unmodified in a copy: md5 `4ea493ea4e9f6c9ebfccbdb15110367e`, **CRC32 `0658f691`, 2,097,152 bytes** |

**The controls on the references.** Each reference build was run three times in the
same copy: identical md5 every run (`logs/reference-stability.txt`). That is the check
the refused s2 asl build needs, because its defect is an uninitialized read that varies
per run, so identical outputs are the evidence no declined operand reached a byte.
Neither reference build left a `.log` file, and `common.lua` runs asl with `-E`, so asl
emitted no diagnostics on either corpus.

**Master moved while this ran, and it does not touch the figures.** The parcel started at
`72aca2e2`, read from the tree rather than taken from the brief. By the end master was
`9c431394`; the two commits between them (`7a271bfc`, `9c431394`) change only
`docs/OVERSEER.md` and `docs/OVERSEER-REFERENCE.md`, so no assembler behaviour moved and
every number below still describes the current assembler. Checked with `git diff --stat`,
not assumed from the subject lines.

**Wall clock.** Every sigil run is under 2.5 s (`runs/*/exit` carries the elapsed
seconds and the machine uptime at the time of the run; at the start of the parcel the
box was up 6 h 19 m at load 3.30, at the end 6 h 35 m at load 3.01). The release build
of sigil took 15.50 s.

## Method, and the trees it needed

Three tree shapes per corpus, all in scratch:

* **pristine**: `git archive <rev>` of the shared checkout. This matters. The shared
  `s1disasm` carries 17 gitignored and modified paths and `skdisasm` 110, and a `cp -a`
  copy of either inherits them, including the generated DAC files a build leaves behind.
  The 2026-09-11 stage-2 note found this the hard way and said so; `git archive` removes
  the question.
* **gen**: pristine plus the pre-step outputs `build.lua` generates before it assembles,
  copied from a third tree where the stock `build.lua` ran (`scripts/mk_gen_trees.sh`).
* **luaref**: the stock `build.lua` run unmodified, which is where the reference ROM
  comes from.

Runs are `scripts/run_sigil.sh`, which records the sigil md5, the tree, the argument
list, the machine uptime, the exit status, wall time, both streams, and the image's md5,
CRC32 and size. Compares are `scripts/compare.py`: whole image, no window, **and a
positive control** that plants three bytes into a copy of the candidate and requires the
comparer to report exactly those three. Every compare in this note carries its control.

## Q1. What stops each corpus, per class, at current master

### Sonic 1 (`s1disasm f6ece657`)

| run | exit | rows | outcome |
|---|---|---:|---|
| pristine, `sigil sonic.asm -o image.bin` | 1 | 14 | all class (a) |
| gen, same | 1 | 1 | the second-address-space refusal, dissolved by `-z` |
| gen, `-p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after` | **0** | **0** | **byte-identical** |

**Class (a), missing build-generated inputs: 14 rows, the whole residual.**

| rows | message | named example |
|---:|---|---|
| 3 | `cannot include sound/dac/dpcm/generated/…` | `sound/z80.asm(223):12: error: cannot include sound/dac/dpcm/generated/kick.inc` |
| 1 | `cannot include sound/dac/pcm/generated/sega.inc` | `s1.sounddriver.asm(2858):10` |
| 4 | `int(): could not evaluate float expression` | `s1.sounddriver.asm(337):51` |
| 4 | `division by zero: 3579545 / 0` | `s1.sounddriver.asm(340):8` |
| 2 | `unresolved if condition: Snd_Sega.size` (here `SegaPCM.size`) | `s1.sounddriver.asm(2861):3` |

The float and division rows are cascades of the missing PCM/DPCM `.inc`: the sample rate
comes from the generated file, so `int()` gets nothing and the rate divides into zero.

**Classes (b), (c) and (d): empty.** (c) needs one word of care: on the `gen` tree with
no `-z`, sigil still refuses

```text
sound/z80.asm(9):3: error: section `sec0#2` [0x0, 0x1BC6) is assembled for the Z80 at origin 0x0, in a second address space this org opens outside the ROM image, and no -z instruction places it into the ROM
```

That is not a residual. It is the correct refusal of a program that does not say where
its driver goes, and `build.lua` passes exactly the `-z` that answers it. The row is
byte-identical in text to the one the 2026-09-11 stage-2 note recorded.

### Sonic 2 (`s2disasm e45ebf33`)

| run | exit | rows | outcome |
|---|---|---:|---|
| pristine, `sigil s2.asm -o image.bin` | 1 | 78 + 1 warning | all class (a) |
| gen, same | 1 | 1 + 1 warning | the second-address-space refusal, dissolved by `-z` |
| gen, `-p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after` | **0** | **0 + 1 warning** | **byte-identical** |
| the census's stub C tree, same line | **0** | **0** | **byte-identical** |

**Class (a): 78 rows, the whole error residual.**

| rows | message | named example |
|---:|---|---|
| 31 | `cannot include sound/music/generated/…` | `s2.asm(91048):10: error: cannot include sound/music/generated/82 - EHZ.inc` |
| 7 | `cannot include sound/DAC/generated/…` | `s2.asm(90887):14: … Kick.inc` |
| 1 | `cannot include sound/PCM/generated/SEGA.inc` | `s2.asm(90998):11` |
| 17 | `int(): could not evaluate float expression` | `s2.sounddriver.asm(3908) dac_sample_metadata(5):45` |
| 17 | `division by zero: 3579545 / 0` | `s2.sounddriver.asm(3908) dac_sample_metadata(5):29` |
| 2 | `unresolved if condition: Snd_Sega.size` | `s2.asm(91000):2` |
| 1 | `unresolved symbol Snd_Sega.size` | `s2.sounddriver.asm(1620):2` |
| 2 | `unresolved symbol .loop_counter` | `s2.sounddriver.asm(1632):2` |

The one warning is

```text
s2.asm(91275):2: warning: `shared` is ignored: sigil writes no share file (asl without `-c` says "no sharefile created, SHARED ignored")
```

which is the intended behaviour landed at `4d52fb99`, and which stub C comments out (the
only diagnostic difference between the stub C run and the pristine gen run).

**Classes (b), (c) and (d): empty at the shipped settings.** Q4 is about what that
sentence is and is not worth.

## Q2. The set delta against 2026-09-11, in both directions

Totals are reported only because the sets are. Every comparison below is a member-level
`comm` over the sorted row multisets, committed in `rows/`.

### Sonic 2, pristine tree, plain run: 149 to 79

`rows/setdiff-s2-pristine-vs-2026-09-11.txt`. **105 rows left, 35 entered, 44 common.**

**What left (105).**

| rows | class | closed by |
|---:|---|---|
| 30 | empty `()` from an omitted macro argument (`bad byte expression`) | `1b8cb9ae` |
| 21 | `unresolved symbol` references to `zAbsVar.1upPlaying`, `ixl`, `ixu`, `iyl`, `iyu` | `428342b6` and `f3b63c1f` |
| 8 | memory-form shift with no size (`instruction needs an explicit size suffix`) | `d0cea72b` |
| 3 | `pushv` / `popv` (`is not a recognized 68000 mnemonic`) | `89a33bba` |
| 3 + 1 | macro argument text with a digit-led word (`malformed number`) | `b006f40c` |
| 2 | `charset '\H'` (`charset operand out of range`) | `99445b2c` |
| 1 | digit-led struct member (`struct has a member line this cannot read`) | `428342b6` |
| 2 | `lastbit` (`unresolved if/elseif condition` at `s2.macrosetup.asm`) | `97413ee9` |
| 34 | the missing-input float and division cascades, at their OLD location | `81a24af6` + `d1ec0fa9` |

Each of those SHAs is an ancestor of master (checked with `git merge-base --is-ancestor`).
The census's class 9 (`shared`, 1 row) also left the ERROR set: it is the warning now.

**What entered (35).** 34 of them are the same missing-input cascade rows, moved from the
macro body `s2.sounddriver.asm(3905)` to their 17 distinct call sites with a macro frame,
`s2.sounddriver.asm(3908) dac_sample_metadata(5)`, by `AS-MACRO-DIAG-CALL-SITE`
(`81a24af6`, `d1ec0fa9`); that parcel's own note already records Sonic 2 as 79 rows
before and after with an identical (level, message) multiset. The 35th is the `shared`
warning. **Nothing genuinely new entered the set in either direction.**

The census's class 10, the silent string escapes, had **no rows**, so it cannot appear in
this diff at all. It is closed by `99445b2c`, and Q3's byte compare is the only thing
that can say so.

### Sonic 2, generated-inputs tree: 71 to 1 error + 1 warning

The 71 source-work rows are the 71 named above minus the 34 relocations. All gone. The
one error is the second-address-space refusal, which `build.lua`'s own `-z` dissolves.

### Sonic 1: unchanged, member for member

The 2026-09-11 stage-2 note recorded 1 row plain and 0 with the flags, on a tree that had
generated inputs. Today, on the `gen` tree: 1 row plain, byte-identical text to
`2026-09-11-s1-driver-stage2/diag/s1-plain-new.rows`, and 0 with the flags. **The
pristine-checkout figure, 14 rows, is new**: no earlier note measured a Sonic 1 tree that
had never been built, because the copies all inherited gitignored generated files.

### S3K: 172 to 120 (wrapper root), 189 to 137 (plain root)

`rows/setdiff-sk-wrapper-vs-2026-09-11.txt`. **69 left, 17 entered, 103 common.** The 69
are 52 unsized shifts (`d0cea72b`) and the 17 `codepage` rows that used to be reported at
the macro body `sonic3k.macros.asm`. The 17 that entered are those same 17 rows now
reported at the call site, `sonic3k.asm(10553) levselstr(2)`. Again, only relocations
entered.

## Q3. The two byte-identity claims, re-run

Both hold. Both re-derived here, not carried: the references were rebuilt from the
sources, and each compare has its planted-byte control (`logs/compare.txt`).

| claim | 2026-09-11 | 2026-09-16 |
|---|---|---|
| **S1 whole**, `sigil sonic.asm -o s1built.bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after` | md5 `09dadb50…`, CRC32 `afe05eee`, 524,288 B | **identical**: md5 `09dadb5071eb35050067a32462e39c5f`, CRC32 `afe05eee`, 524,288 B, **0 bytes in 0 runs**, no window |
| **S2 on stub C**, `-p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after` | md5 `9feeb724…`, CRC32 `7b905383`, 1,048,576 B | **identical**: md5 `9feeb724052c39982d432a7851c98d3e`, CRC32 `7b905383`, 1,048,576 B, **0 bytes in 0 runs** |

Stub C was rebuilt by the census's own `stub.py` in mode C on the `gen` tree. It reported
the same **108 edits** the stage-2 note records (`logs/stub-C.edits`), and it is still
byte-neutral under the stock toolchain: `build.lua` on the stub C tree writes
`9feeb724052c39982d432a7851c98d3e`.

**And the claim is now weaker than the tree needs.** The pristine `gen` tree, with no
stub at all, gives the same image. Stub C's only remaining effect on the run is
commenting out the `shared` line, which removes the warning. Whoever maintains
`as_sonic2_whole_rom.rs` can drop the scaffold.

**The post-p2bin steps still change zero bytes** (`logs/postcheck.log`). Hand-running the
exact asl and p2bin lines `common.lua`'s `assemble_file()` runs, and keeping p2bin's raw
output, gives `09dadb50…` for Sonic 1 and `9feeb724…` for Sonic 2: the same ROMs
`build.lua` writes after `amend_sound_driver_size` and `fix_header`. The share file p2bin
wrote reads `comp_z80_size 0xF64 le for C Program */` with
`#define movewZ80CompSize 0xEC04E`, exactly the in-place overwrite the census measured.

## Q4. Class (d), accepted but wrong: how it was looked for, and what the method cannot see

**The method.** Whole-image byte comparison against the ROM the stock toolchain builds
from the same tree, no window, with a positive control on every compare. It is the
strongest instrument available for this class, and at current master the "accepted" half
is total: sigil accepts every line of both corpora, so the image compare is the only
thing left that can disagree. It reports 0 bytes on both.

**What that method cannot see, measured rather than asserted where possible.**

1. **Source no build assembles.** `logs/blindspot.log`. A line no assembler could accept,
   planted at `s2.sounddriver.asm:2674` inside the `if OptimiseDriver` arm
   (`OptimiseDriver = 0`, arm spans 2657 to 2682), leaves **both** toolchains at exit 0
   and both still writing `9feeb724…`. The corrected control plants the same marker on
   the live line `s2.sounddriver.asm:1678`: sigil exits 1 with
   `unexpected character '%'`, `build.lua` exits 1 with asl's `error #1200: unknown
   instruction`. Scale: 557 `if`/`elseif` sites in the Sonic 1 `.asm` closure, 698 in
   Sonic 2's. **This is the largest blind region, and Q5 shows it is not empty.**
   *(My first control line, 2678, was itself inside the excluded arm and proved nothing.
   It is recorded rather than deleted: guessing which line is live is unreliable, the arm
   bounds have to be read.)*
2. **Outputs other than the image.** sigil has no `-c`, so it writes no share file; no
   `-L`, so no listing; no `-D`, so no command-line symbol. None of these is a byte in
   the ROM, and the compare is blind to all of them. The missing `-c` is not cosmetic:
   see Q5 item 2.
3. **Over-acceptance.** Byte identity on programs asl accepts says nothing about programs
   asl refuses. Nothing in this parcel measures that direction;
   `crates/sigil-frontend-as/tests/over_acceptance/` is the lane that does.
4. **The generated inputs.** Both sides read the SAME bytes from `build.lua`'s pre-steps,
   so a defect in them is invisible to the compare and identical in both images. sigil
   has no route to produce them.
5. **Runtime.** Whether either ROM plays is not a byte question. **TAGGED for the
   controller; no emulator was touched.**

## Q5. What the blind region actually contains

Eleven flips of switches the corpora themselves offer, each edit shown applied on disk,
each tree built by **both** sigil and the stock `build.lua`, each pair compared
whole-image with a control (`logs/switchflip.log`, `logs/switchflip2.log`).

| flip | sigil | agree | what differs |
|---|---|---|---|
| s2 `gameRevision=0` | exit 0 | no | 2 B, header checksum |
| s2 `fixBugs=1` | exit 0 | no | 3 B, header checksum and the driver-size immediate |
| s2 `allOptimizations=1` | exit 0 | no | 2 B, header checksum |
| s2 `useFullWaterTables=1` | exit 0 | no | 2 B, header checksum |
| s2 `padToPowerOfTwo=0` | exit 0 | **yes** | 0 B |
| s1 `AllOptimizations=1` | exit 0 | no | 2 B, header checksum |
| s1 `CheatsEnabled=1` | exit 0 | no | 2 B, header checksum |
| s1 `Revision=0` | exit 0 | **yes** | 0 B |
| s1 `EnableSRAM=1` | exit 0 | **yes** | 0 B |
| s1 `FixBugs=1` | **exit 1** | n/a | sigil refuses what asl assembles |

**1. The header checksum at `0x18E`, both corpora, silent.** The source hardcodes it:
`s2.asm:169` is `dc.w $D951   ; Checksum (patched later if incorrect)`, and
`sonic.asm:175-180` is an `if Revision=0` / `dc.w $264A` / `else` / `dc.w $AFC7`.
`build.lua`'s `fix_header` rewrites it with the 16-bit sum of the big-endian words from
`0x200`. At the shipped settings the hardcoded literal already **is** that sum, which is
the only reason the byte-identity headline holds. `logs/checksum.txt` derives both
numbers for six images: sigil is stale on s1 `CheatsEnabled=1` (`BF37` required, `AFC7`
written) and on s2 `fixBugs=1`. **The three flips that still agree are exactly the three
that change no byte at or after `0x200`** (a header version string, an SRAM field,
trailing padding), which is the mechanism predicting its own exceptions rather than an
excuse for them.

**2. Sonic 2's compressed-driver size immediate, silent.** With `fixBugs=1` the
compressed driver is `0xF88` bytes against `Size_of_Snd_driver_guess = $F64`
(`s2.constants.asm:9`), so `amend_sound_driver_size` patches the `move.w #…,d7` immediate
at `movewZ80CompSize+2`, which in that tree is `0xED04E`: the reference holds
`3E 3C 0F 88` and sigil's image `3E 3C 0F 64`. The game's own decompressor would read
`0x24` bytes too few. The 2026-09-11 census predicted exactly this ("the patch becomes
load-bearing the moment the compressed size differs from the guess"); it is now measured.
sigil has no `-c`, writes no share file, and warns only that `shared` is ignored, so a
`build.lua` driving sigil finds no `movewZ80CompSize` and skips the patch in silence.
A cross-check that the two faults are the same arithmetic: sigil's `fixBugs=1` image sums
to `CA85` and the reference to `CAA9`, and `CAA9 - CA85 = 0x24 = 0xF88 - 0xF64`.

**3. A class (b) row no shipped-settings census can see, loud.** With `FixBugs=1`
Sonic 1 stops at

```text
_incObj/DebugMode.asm(245):3: error: absolute address operand `(expr)` needs an explicit `.w`/`.l` width suffix (width-selecting bare `(expr)` is out of scope)
```

on `move.w (v_limitright2),d0`. asl assembles that tree; sigil refuses it.

**An unapplied mutation that read as a pass.** `switchflip.sh`'s first s1 `Revision=0`
run asserted "exactly one site" for the text `Revision = 1`; `sonic.asm:139` mentions it
inside a warning string, the assertion fired, the edit was never made, **and the run then
reported 0 bytes different against the unflipped reference.** Only the assertion
separates that from a real pass. `switchflip2.sh` re-runs it anchored to a line number
and prints the before and after text of every edit it makes.

## Q6. S3K, the next loud corpus

Not this brief's corpus. Measured read-only because the recommendation needs sizing.
skdisasm `2fcd861c`, `buildSK.lua`'s pre-steps run in a copy, `buildSK`'s own `-D`
wrapper root and its two `before` `-z` instructions: **120 rows in four classes**.

| rows | class | named example |
|---:|---|---|
| 93 | `$$name` labels read as a hex prefix | ``sonic3k.asm(302):1: error: `$` with no hex digits``, source line `$$compareChars:` |
| 19 | the `codepage` directive | `sonic3k.macros.asm(140):2` and `sonic3k.asm(9951):3`, `codepage LEVELSELECT` |
| 6 | the `abcd` (3) and `subx` (3) mnemonics | `sonic3k.asm(62886):3`, `sonic3k.asm(100959):3` |
| 2 | `(d8,PC,Xn)` indexed PC-relative addressing | `sonic3k.asm(174875):3`, `movem.w word_82872(pc,d0.w),d2-d3` |

Without the wrapper the plain root adds 17 rows (16 `unresolved if condition` and 1
`unresolved symbol`, all `Sonic3_Complete`): sigil has no `-D`, so a wrapper root is
required today. The `buildSK` reference reproduces here at md5
`4ea493ea4e9f6c9ebfccbdb15110367e`, CRC32 `0658f691`, 2,097,152 bytes, the stage-2 note's
value.

## Recommendation, sized, ranked by what each item costs rather than by its row count

**The count is now actively misleading, and this is the shape the `OVERSEER-REFERENCE`
count block warns about.** 92 of the 93 remaining Sonic 1 and Sonic 2 rows are missing
generated inputs. Closing them would delete more rows than all the other work combined
and would buy the project nothing about correctness, because the classes they hide behind
are already closed. Meanwhile the three faults Q5 found produce **zero rows**.

1. **The build-script gap: `fix_header`, `amend_sound_driver_size`, and `-c`.**
   **Silent, consequential, small to medium.** Two of the three faults in Q5 are this
   one item, and both make a ROM that is wrong in a way nothing announces. Three
   defensible shapes, an owner call: sigil performs the header fix and the size patch
   itself (it holds `movewZ80CompSize` already and needs no side file); or sigil grows
   `-c` so `build.lua`'s own steps can run against it; or sigil refuses out loud when it
   assembles a source whose hardcoded checksum does not match the image it just built.
   The third is the cheapest and closes the silence even if the other two never land.
   Note the asymmetry: at the shipped settings this item is unfalsifiable by any corpus
   test that exists, which is exactly why it has survived two byte-identity headlines.
2. **Sweep the excluded arms.** **Silent, cheap, high yield.** Q5 is eleven flips and a
   few minutes of machine time, and it found three faults, one of them a missing front-end
   feature. A runner that builds each corpus under a matrix of its own switch settings and
   compares both toolchains needs no new assembler feature to be useful, and it converts
   557 + 698 conditional sites from unmeasured to measured. Fold Q5's three findings in
   as its first expected results. Small, and it is the only item here that can find the
   NEXT unknown rather than the known ones.
3. **S3K's four classes.** **Loud, and the only place left where the row count and the
   risk point the same way.** Rank inside it by failure mode, not by rows:
   `codepage` (19 rows) first, because it is the character-mapping sibling of the escape
   class that cost Sonic 2 2,176 silent bytes, and an accepted-and-ignored `codepage` is
   that defect again; then `$$` labels (93 rows, a lexer and scoping feature, loud, M);
   then `abcd` and `subx` (6 rows, two instructions, S, byte-checkable against asl); then
   `(d8,PC,Xn)` (2 rows, an addressing mode the lowerer declines by name, S). A `-D`
   option is a few lines and removes the wrapper-root scaffold.
4. **`build.lua`'s pre-steps.** **Loud, cheap, lowest value, do it last.** 92 rows, and
   not one of them is an assembler defect. If it is done for the row count, say so in the
   commit message.

## Things in the brief that turned out wrong

1. **"Five days of assembler work have landed since."** Almost none of the delta is from
   the five days. The census's evidence directory is stamped 15:09 on 2026-09-11, and the
   string-escape merge `571c4a76` landed at 15:59 that same day, the placement merge
   `a5b31a20` at 17:08, the small-features merge `f8052111` at 17:18 and the half-register
   merge `0811e659` at 19:29. **Nine of the ten census classes and the overlap wall closed
   within hours of the census being written**, which is what a good census is for. Only
   `lastbit`'s sibling work and the macro relocation are later. The figures were stale on
   the day, not after five days.
2. **"The address-0 overlap wall."** There is no wall to report. It is not merely absent:
   it was the subject of a landed parcel (`a5b31a20`, owner decision `d-30`) whose `-z`
   option is the answer to it, and the refusal that remains without `-z` is correct.
3. **"Which classes are (a) … (b) … (c) … (d)."** The framing invites a four-way split
   and the honest answer is that (b), (c) and (d) are all empty for both corpora at the
   shipped settings, so the interesting question moved to what "at the shipped settings"
   is worth. Q4 and Q5 are that question, and they are the parcel's actual finding.
4. **"The string-escape silent-wrong-answer finding"** as a thing to re-measure: it has
   no rows and never did, so it is not in any row set. The only instrument that can speak
   to it is Q3's byte compare, which now reports 0 bytes with no window.
5. **"The S1 pinned build cannot assemble Sonic 2."** Not re-tested here, and not needed:
   the Sonic 2 reference came from `s2disasm`'s own asl with the stability control, as the
   census did. Recorded so nobody reads this note as a second witness for that claim.
6. **"A pristine checkout copy."** `cp -a` of the shared `s1disasm` is not pristine: it
   carries 17 gitignored and modified paths, including generated DAC files that make the
   build work. `git archive` is what the word needs, and the 14-row Sonic 1 pristine
   figure exists only because of it.

## Open, and why

* **Whether either ROM plays.** TAGGED for the controller. No emulator was touched.
* **Over-acceptance** is unmeasured by this parcel in either corpus (Q4 item 3).
* **The switch matrix is a sample, not a sweep.** Eleven flips out of 557 + 698
  conditional sites. Three faults in eleven flips is a rate, not a bound: assume more.
* **Interactions between switches** were not tried at all; every flip was one switch from
  the shipped tree.
* **S3K's byte identity** is unmeasured: the front end stops at 120 rows, so no image
  exists to compare. The `probes/p_s3k.asm` shape in the stage-2 note is the current
  evidence for the two-blob `before` placement.
* **`p2bin`'s source revision** against the binary `4f2fff99…` is still unverified.
* **What p2bin does with a stream larger than the reservation** was not re-measured here;
  the stage-2 note has it (refuses, exit 1, deletes its output).

## Reproducing

```text
CARGO_TARGET_DIR=<on disk, never a scratchpad> cargo build --release --bin sigil
git -C <corpus> archive --format=tar <rev> | tar -x -C <scratch>/trees/<corpus>-pristine
cp -a <scratch>/trees/<corpus>-pristine <scratch>/trees/<corpus>-luaref
(cd <scratch>/trees/<corpus>-luaref && lua build.lua)     # the reference ROM, three times
bash scripts/mk_gen_trees.sh                              # pristine + the pre-step outputs
bash scripts/run_sigil.sh <tree> <root.asm> <tag> [args]  # one row multiset per run
python3 scripts/compare.py <reference> <candidate> --control
bash scripts/postcheck.sh                                 # asl + p2bin by hand, no post steps
bash scripts/blindspot.sh                                 # the excluded-arm control
bash scripts/switchflip.sh ; bash scripts/switchflip2.sh  # the eleven flips
python3 <census>/scripts/stub.py <gen tree> <stub C tree> C
```
