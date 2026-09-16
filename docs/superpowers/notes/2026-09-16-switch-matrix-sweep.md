# The switch matrix, derived rather than listed: what the corpora's own build options do to sigil

2026-09-16, parcel `SWITCH-MATRIX-SWEEP`, branch `parcel/switch-matrix-sweep`,
base `4f2b0cf6` (master's tip, equal to `origin/master`, read from the tree).
The measurement this replaces is `2026-09-16-as-corpus-census.md` Q5, a
hand-typed list of eleven flips, three of which exposed faults and two of those
silently. `2026-09-16-switch-setting-silent-roms.md` closed two of the three.

The remedy this parcel implements is not a longer list. **The switch set is
derived from the corpus source**, so the set exercised cannot drift from the set
the corpora offer, and the runner refuses to print a table unless the derived
set and the executed set reconcile.

Every tree below is a private copy under
`/home/volence/sonic_hacks/.scratch/switch-matrix-sweep/`. The shared `s1disasm`
and `s2disasm` checkouts were only ever read, through `git archive`, and their
`git status` after the run is identical to before it (`s1disasm` carries the two
modified `.nem` files and two untracked paths it already carried; `s2disasm` is
clean).

## Headlines

1. **The derivation reaches 16 arms across the two corpora without naming a
   switch**, against the census's 11 hand-typed flips, and it found two of its
   own holes before a single ROM was built (below).
2. **Three of those 16 arms cannot be measured one at a time at all.** Sonic 1's
   `BackupSRAM` and `AddressSRAM` are read only inside code `EnableSRAM` opens,
   so flipping them alone moves no byte of the stock image. A one-at-a-time
   sweep reports them as agreement. That is not agreement, it is the only answer
   those legs could have given, and the runner now says `NOT-MEASURED` and then
   rescues them.
4. **Every one of the 16 arms that can be measured agrees byte for byte, except
   the two rows already open**, and both of those refuse loudly. No new silent
   ROM was found. The unexplored arms specifically include Sonic 1 `Revision = 2`
   (REVXB) and Sonic 2 `gameRevision = 2` (the theoretical REV02, which also
   turns on `removeJmpTos` and `addsubOptimize` and turns off `relativeLea`),
   neither of which anything had ever built with sigil.
5. **Sonic 1 `FixBugs = 1` is one line.** Measured, not estimated: the refusal is
   at `_incObj/DebugMode.asm:245`, and with a single `.w` added to that one
   operand sigil's image is **byte-identical** to the stock ROM of the unpatched
   tree, CRC32 `888defef` / 551,288 bytes. There is no second site behind it.
6. **The under-run direction of the driver-size check is not reachable from any
   build option either corpus offers.** 16 arms and 18 rescue pairs produced
   exactly one driver-size event and it was the over-run the previous parcel
   closed. That is evidence for, not against, the asymmetry that parcel chose.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | built from base `4f2b0cf6` with `CARGO_TARGET_DIR` in scratch. md5 `ab3637357c1301e910f16610e0379b02`, `--version` `4f2b0cf6`. This parcel adds no file under `crates/`, so the binary is the base binary for every figure here. |
| corpora | `s1disasm f6ece657`, `s2disasm e45ebf332`, each extracted by `git archive` into scratch. |
| references | `s1built.bin` CRC32 `afe05eee` / 524,288 B; `s2built.bin` CRC32 `7b905383` / 1,048,576 B, each from the corpus's own unmodified `build.lua`. Both reproduce the previous parcel's figures exactly, from a different method. |
| runner | `scripts/switch_matrix_sweep.py`, committed. |
| log | `/home/volence/sonic_hacks/.scratch/switch-matrix-sweep/logs/sweep-FINAL.log`, plus one `.lua.log` and one `.sigil.err` per leg. |

CRC32 throughout is IEEE/zlib, computed by `zlib.crc32` in CPython 3, rendered as
eight hex digits, and quoted with the byte size.

## How the domain is derived, and what the derivation cannot see

Three mechanical steps, none of which reads a list of switch names.

**1. Documented-declaration scan**, over every `.asm` file in the corpus: a line
`Name = <rhs>` at column 0 whose NEXT line is a `;...|` prose comment. That shape
is how both corpora document a build option.

**2. Block cross-check.** The `ASSEMBLY OPTIONS` block is located independently,
by its own header comment and by the section rule that ends it, and the two sets
must be equal in both directions.

The result of running both is itself the answer to "is anything declared outside
the block": **the scan selects the whole block and nothing else, in both
corpora.** 11 declarations in `sonic.asm`, 10 in `s2.asm`, no other file in
either tree carries the shape. So the hypothesis that a switch might be declared
outside the header block is refuted for these two corpora, by enumeration rather
than by looking in the likely places.

**3. Domain from prose.** Values come from the two forms the corpora write:
`If <n>, <what that value does>` clauses, and `<n> = <text>` enumerations
(`AddressSRAM`'s `0 = odd+even; 2 = even only; 3 = odd only`). The current value
always joins the set, because neither form is obliged to mention it.

A switch whose right-hand side names another identifier is **DERIVED**
(`SkipChecksumCheck = 0|AllOptimizations`,
`removeJmpTos = 0|(gameRevision>=2)|allOptimizations`). It is not independently
settable and is never flipped directly. It is still counted, and it is exercised
through the switches it reads: eight derived switches across the two corpora,
all of them functions of `AllOptimizations`/`allOptimizations` or
`gameRevision`, all of which are swept.

### What the derivation cannot see, stated plainly

* **A switch whose legal values exist only as English.** `ZoneCount = 6` is
  documented as "Used for the `zonewarning` macro. Do not change, unless more
  zones get added", which states a count, not a domain. The runner classes it
  UNREADABLE and **fails** unless it is in `ACK_UNREADABLE`, whose contents are
  asserted equal to the unreadable set in both directions, so it cannot go stale
  in either direction. It is the only such switch in either corpus.
* **A switch documented only by its active arm.** `padToPowerOfTwo = 1` is
  followed by `If 1, pads the end of the ROM`, and nothing else. Read literally
  that is the one-element domain `{1}` and therefore **no leg at all**, while
  the switch is plainly a flag the census did flip to 0. See "Two findings from
  the derivation" below.
* **Anything undocumented.** The scan sees a documented declaration. Three
  identifiers the block's prose cross-references are declared elsewhere in the
  plain `Name = rhs` form: `s1.sounddriver.asm:2637 FixMusicAndSFXDataBugs = FixBugs`,
  `s2.sounddriver.asm:8 FixDriverBugs = fixBugs`, and
  `s2.asm:68 FixMusicAndSFXDataBugs = fixBugs`. All three are derived from a
  switch that IS swept, so all three are exercised, but the runner did not find
  them, it was told about them by this note's author reading the prose.
* **A value outside what the prose enumerates.** Nothing stops a hacker setting
  `Revision = 7`. The domain swept is the one the corpus documents.
* **Interaction.** A full cross product is infeasible and is not attempted: the
  derived arms alone give 2^9 and 2^7 corners, before the non-binary switches.
  What IS done is a targeted rescue of the arms one-at-a-time cannot reach, and
  its results are below.
* **Runtime.** Whether any of these ROMs plays is not a byte question. No
  emulator was touched. TAGGED for the controller.

### Corpus-agnosticism, and `skdisasm`

The derivation reads no corpus-specific name: not the root `.asm` (which comes
from `build.lua`'s own `build_rom_and_handle_failure` call), not the output
filename, and not the `-p`/`-z` arguments, which are evaluated out of `build.lua`
including its `improved_*_compression` local and the `and/or` expression it feeds.
So pointing it at a third corpus is `--corpus name=path`. **`skdisasm` was not
run**, because it does not assemble under sigil at all yet (120 rows in four
classes, a separate queue row): the shipped-settings baseline leg would fail and
every arm would fail behind it, which measures the known blocker 30 times over.
It becomes worth running the day `skdisasm` at shipped settings agrees. Its
`build*.lua` scripts would need one check first: there are three of them and they
build three different ROMs, so `derive_tool_args` would have to be told which,
and it currently takes the first `build_rom_and_handle_failure` call it finds.

## Two findings from the derivation itself, before any ROM was built

**`padToPowerOfTwo` would have executed nothing.** Its prose names only the value
it is already set to. A switch that derives a one-element domain is now a **hard
failure of the derivation**, and a prose form naming only values in `{0,1}` is
completed to `{0,1}`. The completion is refused where the prose names a 2 or a 3,
so no value is ever invented for `Revision` or `AddressSRAM`. Without the hard
failure the completion rule would be a silent guess; with it, any future prose
form the rule underdetermines is loud instead of empty. The census flipped this
switch by hand, so a list beat a derivation here, exactly once, and the fix is
that the derivation now cannot fail quietly in that direction.

**Three legs measure nothing.** See the next section.

## Vacuity: the answer a leg could not have failed to give

`BackupSRAM = 0`, `AddressSRAM = 0` and `AddressSRAM = 2` each produce a stock
ROM **byte-identical to the shipped one**, because the code that reads them sits
behind `EnableSRAM`, which ships at 0. Their compare reports 0 bytes different
and there was no other answer available.

So each leg's own stock image is compared against the stock image the flip was
supposed to move away from, and if they are equal the leg is classed
`NOT-MEASURED`. **The RESULT column never prints AGREE for such a leg**, and such
a leg is excluded from the outcome bookkeeping entirely, so it can never be
counted as a pass anywhere.

Then each unmeasured arm is retried against one companion arm at a time, in
declaration order, **with the yardstick moved to the stock build of the companion
ALONE**, so the switch under study is still the only thing being tested. This is
a targeted rescue and not a cross product: it runs only for arms phase 1 could
not measure, and stops at the first companion that moves the image. All three are
rescued by `EnableSRAM = 1`, after five companions each that do not move them,
and all three then agree byte for byte.

An arm no single companion can rescue would be `UNMEASURABLE` and would have to
be acknowledged in `ACK_UNMEASURABLE`, which is asserted equal to the run's
findings in both directions. That table is empty because nothing needed it.

## The result table

38 legs launched, 38 reported. 16 agree, 2 refuse and are acknowledged, 18 are
`NOT-MEASURED` (the 3 vacuous arms plus the 15 rescue attempts that did not move
the image), 2 are controls. Whole-image compare, no window, planted-byte control
on every one.

### Sonic 1, `s1disasm f6ece657`, 11 switches: 7 swept (9 arms), 3 derived, 1 unreadable

| leg | result | sigil image |
|---|---|---|
| shipped | AGREE | `afe05eee` / 524,288 |
| `Revision = 0` | AGREE | `f9394e97` / 524,288 |
| `Revision = 2` (REVXB) | AGREE | `6382b2c5` / 524,288 |
| `FixBugs = 1` | **SIGIL-DECLINED** | see fault 3 below |
| `CheatsEnabled = 1` | AGREE | `ac362f3d` / 524,288 |
| `AllOptimizations = 1` | AGREE | `95dd2ddc` / **520,974** |
| `EnableSRAM = 1` | AGREE | `649236ea` / 524,288 |
| `BackupSRAM = 0` | NOT-MEASURED | stock image unmoved |
| `AddressSRAM = 0` | NOT-MEASURED | stock image unmoved |
| `AddressSRAM = 2` | NOT-MEASURED | stock image unmoved |
| `BackupSRAM = 0` + `EnableSRAM = 1` | AGREE | `0b0a4c57` / 524,288 |
| `AddressSRAM = 0` + `EnableSRAM = 1` | AGREE | `4013c599` / 524,288 |
| `AddressSRAM = 2` + `EnableSRAM = 1` | AGREE | `2d299aa6` / 524,288 |

`AllOptimizations = 1` is the size-changing arm: `PaddingOptimization` removes
3,314 bytes and the image is no longer 512 KiB. sigil agrees on the shorter
image, which is the end-of-ROM field at `0x1A4` that the previous parcel added.

### Sonic 2, `s2disasm e45ebf332`, 10 switches: 6 swept (7 arms), 4 derived

| leg | result | sigil image |
|---|---|---|
| shipped | AGREE | `7b905383` / 1,048,576 |
| `gameRevision = 0` | AGREE | `24ab4c3a` / 1,048,576 |
| `gameRevision = 2` (theoretical REV02) | AGREE | `d10a3a93` / 1,048,576 |
| `padToPowerOfTwo = 0` | AGREE | `dd0ebd72` / **1,048,556** |
| `fixBugs = 1` | **SIGIL-DECLINED** | see fault 2 below |
| `allOptimizations = 1` | AGREE | `6c8cbd9b` / 1,048,576 |
| `skipChecksumCheck = 1` | AGREE | `f8425851` / 1,048,576 |
| `useFullWaterTables = 1` | AGREE | `bbaffa38` / 1,048,576 |

`gameRevision = 2` is the arm nothing had measured, and it is the widest: it
switches `removeJmpTos` and `addsubOptimize` on and `relativeLea` off through
their derived expressions, so three code-shape transforms move together. It
agrees.

### The two refusals, adjudicated

**Fault 2, Sonic 2 `fixBugs = 1`, refusing correctly.** Exit 1, no image, one
error naming `s2.sounddriver.asm(248):6`: the Saxman stream is `$F88` and
`Size_of_Snd_driver_guess` declares `$F64`, so every byte computed from that name
is short by `$24`. `build.lua` repairs its own ROM afterwards, in
`amend_sound_driver_size`, by reading the real size out of asl's share file and
patching `movewZ80CompSize + 2` in place. sigil writes no share file and so
cannot do that, and refuses rather than write a ROM whose decompressor is told
the wrong length. Booked and argued by `SWITCH-SETTING-SILENT-ROMS`.

**Fault 3, Sonic 1 `FixBugs = 1`, refusing on a front-end gap, and it is ONE
LINE.** The refusal is at `_incObj/DebugMode.asm(245):3`:

```text
move.w	(v_limitright2),d0			; get current right level boundary
```

an absolute address operand with no `.w`/`.l` suffix. Every other reference to
`v_limitright2` in the corpus writes `.w`; this one site sits inside an
`if FixBugs` block and so is only reachable on that arm.

**Measured rather than assumed:** the probe added `.w` to that one operand and
re-ran sigil until no error remained. It took **one round**. The image sigil
then wrote is **byte-identical to the stock ROM built from the UNPATCHED tree**,
CRC32 `888defef` / 551,288 bytes, planted-byte control passed. So asl selects
`.w` there, nothing else in the `FixBugs` arm diverges, and the whole of this row
is one width suffix worth zero bytes. That is a useful number for whoever takes
the front-end row: the fix has no ROM-shaped risk behind it.

(`FixBugs = 1` also pushes Sonic 1 past 512 KiB, to 551,288 bytes. The stock
`fix_header` writes the end-of-ROM field accordingly and so does sigil.)

## The controls, all six plus one, each shown red

No figure above comes from a run whose controls did not pass, and every control
has been shown firing on a mutation that was printed back from disk first. The
runner runs C1 to C6 before it builds anything and aborts printing no table if
any fails; C7 runs once per corpus inside the sweep.

| control | what it proves | how it was shown red |
|---|---|---|
| C1 no-op edit | an edit that writes back the value already there is refused, because an unapplied mutation and a real pass are the same artifact | in-runner, on a file written for the purpose |
| C2 wrong line | an edit aimed at a line that does not declare the switch is refused | in-runner |
| C3 shadowed declaration | a second column-0 declaration of the same name is refused, because the edit would be overridden | in-runner |
| C4 blind comparer | a comparer that cannot see a planted byte is refused | in-runner, by substituting a comparer that always reports 0 |
| C5 option outside the block | a documented declaration outside `ASSEMBLY OPTIONS` fails the cross-check | in-runner |
| C6 one-element domain | a switch whose prose derives a single value fails rather than executing nothing | in-runner; this is the gate `padToPowerOfTwo` tripped |
| **C7 end-to-end** | the reference build, the candidate build and the compare are three independent things, not one file read twice | see below |

**C7** builds the reference from the shipped tree and then flips the source
underneath sigil, so the two toolchains are handed different source and the sweep
MUST report a difference. The arm it uses is chosen by measurement, the first
phase-1 arm whose own stock image differs from the shipped one, so the control
can never be run on a flip that moves nothing. Sonic 1 reports 410,881 bytes in
18,594 runs, Sonic 2 210,513 bytes in 24,226 runs. **Only `DIFFER` passes**: a
refusal would leave the compare itself unexercised, which is the thing the
control exists to exercise.

A planted byte proves the comparer can see a difference. Only C7 proves the two
builds are independent, and when C7 was neutered (handing sigil the same source
the reference came from) it reported `AGREE` on both corpora and the run failed,
which is what a non-independent pipeline looks like.

### The four reconciliations, each shown red

Run `red-A.log`, `red-B.log`, `red-C.log`. Each mutation was applied to the
committed file, quoted back from disk and confirmed by `git diff --stat` before
the run, and the file was restored with `git checkout --` from the **committed**
baseline `9b3ae4f8` between runs, never over uncommitted work.

| reconciliation | mutation | red output |
|---|---|---|
| unacknowledged outcome | removed the `s1disasm-FixBugs-1` acknowledgement | `UNACKNOWLEDGED s1disasm s1disasm-FixBugs-1: SIGIL-DECLINED` |
| stale acknowledgement | added `s1disasm-NOT-A-REAL-LEG` | `STALE ACKNOWLEDGEMENT ... expected SIGIL-DECLINED, the leg agreed` |
| wrong acknowledged class | changed the `s2disasm-fixBugs-1` class to `DIFFER` | `CLASS MISMATCH ... ran SIGIL-DECLINED, acknowledged DIFFER` |
| unreadable-domain set | renamed the `ZoneCount` acknowledgement | `found-not-acknowledged=[('s1disasm', 'ZoneCount')] acknowledged-not-found=[('s1disasm', 'ZoneCountXX')]` |
| vacuity verdicts | disabled the rescue loop | three arms reported `UNMEASURABLE`, and the unmeasurable-acknowledgement set failed naming all three |
| end-to-end control | handed sigil the reference's own source | `CONTROL C7 s1disasm: FAILED (AGREE)`, both corpora, run failed |
| legs launched vs reported | incremented the launch count without a matching row | `2 legs launched but 1 reported; 1 produced no row at all` |

## Method notes, and one change from the census's method

**The reference is built first, in the leg's own tree.** The census pre-baked a
"gen" tree from the shipped build and ran sigil against it BEFORE running
`build.lua` in the same tree. Both toolchains then consumed generated inputs
produced under settings that were not the leg's. For these two corpora that
turns out not to matter, because Sonic 2's `build.lua` hardcodes
`FixMusicAndSFXDataBugs = 0` in the wrapper it generates for each compressed song
(`build.lua:105`) rather than reading `s2.asm`'s `fixBugs`, so the `.sax` files
are independent of the switch under test. It is not a property anyone should
rely on. This runner runs `build.lua` FIRST in each leg's own tree, removes only
the stock toolchain's `.p`/`.h`/`.lst` intermediates, and then runs sigil on
exactly the generated inputs the reference consumed.

Worth knowing separately: `s2.asm:68` sets `FixMusicAndSFXDataBugs = fixBugs`
while `build.lua:105` sets it to 0, so a Sonic 2 tree at `fixBugs = 1` assembles
its uncompressed songs with the fix and its compressed songs without it. That is
a corpus inconsistency, not a sigil one, and both toolchains see it identically.

**The nine flips the previous parcel measured all reproduce exactly**, from this
different method and a separately built binary: `f9394e97`, `ac362f3d`,
`95dd2ddc`, `649236ea`, `24ab4c3a`, `6c8cbd9b`, `bbaffa38`, `dd0ebd72`, and both
shipped baselines. That is corroboration of the previous parcel from an
independent path, not a restatement of it.

## What this parcel did NOT do

* **`skdisasm`.** Out of scope, and see the corpus-agnosticism section for what
  running it would take.
* **Interaction beyond the rescue.** The cross product is infeasible; the rescue
  covers only arms that cannot otherwise be measured. Whether a pair of switches
  that BOTH move the image can disagree in combination is unmeasured, and it is
  booked in the gap ledger.
* **Fault 3.** Diagnosed to one line and one round, not fixed. Front-end row.
* **Runtime.** No emulator. TAGGED for the controller.

## Reproducing

```text
CARGO_TARGET_DIR=<a path on disk, never /tmp> cargo build --release --bin sigil
python3 scripts/switch_matrix_sweep.py --sigil <that binary>
```

Nothing else. The runner extracts the corpora itself with `git archive`, derives
the domain, builds every leg with both toolchains, and exits nonzero unless every
reconciliation holds. `--corpus name=path` points it at a different checkout;
`--only <tag>` runs one leg and says loudly in its own output that the run is a
probe and not a sweep result.
