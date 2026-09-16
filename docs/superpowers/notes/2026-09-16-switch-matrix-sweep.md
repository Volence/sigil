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

1. **The brief's premise that a full cross product is infeasible is wrong, and
   the whole product was run.** Sonic 1 has seven swept switches with domains
   3,2,2,2,2,2,3 and Sonic 2 six with 3,2,2,2,2,2, which is 288 and 96 corners.
   A leg costs 2.4 seconds measured, so the product is a quarter of an hour.
   **The entire documented build-option space of both corpora is now measured,
   not sampled.**
2. **The derivation reaches 16 arms across the two corpora without naming a
   switch**, against the census's 11 hand-typed flips, and it found two of its
   own holes before a single ROM was built.
3. **Three of those 16 arms cannot be measured one at a time at all.** Sonic 1's
   `BackupSRAM` and `AddressSRAM` are read only inside code `EnableSRAM` opens,
   so flipping them alone moves no byte of the stock image. A one-at-a-time
   sweep reports them as agreement. That is not agreement, it is the only answer
   those legs could have given, and the runner now says `NOT-MEASURED` and then
   rescues them.
4. **Every corner both toolchains can build agrees byte for byte**: 144 of Sonic
   1's 288 and 48 of Sonic 2's 96, plus every measurable one-at-a-time arm. No
   new silent ROM was found anywhere in the option space.
5. **The cross product found a Sonic 1 defect nobody had.** `Revision = 0` +
   `FixBugs = 1` + `AllOptimizations = 0` cannot be assembled by **asl**, 24 of
   the 288 corners: `bra.w DisplaySprite` goes out of range. One at a time
   neither option fails, which is why the census could not have found it.
6. **sigil is silent where asl warns, once, and only once.** asl's
   `warning #180: address is not properly aligned` fires on every leg with
   `gameRevision = 0` and sigil says nothing on any of them. Bytes agree, so it
   is a diagnostic gap and not a ROM fault, and the product is what licenses
   "only once": it is the only asl warning code either corpus raises anywhere in
   its option space.
7. **Sonic 1 `FixBugs = 1` is one line.** Measured, not estimated: the refusal is
   at `_incObj/DebugMode.asm:245`, and with a single `.w` added to that one
   operand sigil's image is **byte-identical** to the stock ROM of the unpatched
   tree, CRC32 `888defef` / 551,288 bytes. There is no second site behind it.
8. **The under-run direction of the driver-size check is not reachable from any
   build option either corpus offers.** 422 legs produced exactly one
   driver-size event and it was the over-run the previous parcel closed. That is
   evidence for, not against, the asymmetry that parcel chose.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | built from base `4f2b0cf6` with `CARGO_TARGET_DIR` in scratch. md5 `ab3637357c1301e910f16610e0379b02`, `--version` `4f2b0cf6`. This parcel adds no file under `crates/`, so the binary is the base binary for every figure here. |
| corpora | `s1disasm f6ece657`, `s2disasm e45ebf332`, each extracted by `git archive` into scratch. |
| references | `s1built.bin` CRC32 `afe05eee` / 524,288 B; `s2built.bin` CRC32 `7b905383` / 1,048,576 B, each from the corpus's own unmodified `build.lua`. Both reproduce the previous parcel's figures exactly, from a different method. |
| runner | `scripts/switch_matrix_sweep.py`, committed. |
| committed evidence | `docs/superpowers/notes/2026-09-16-switch-matrix-sweep/logs/`: `cross-run-green.log` (the 422-leg run, per-leg lines and every reconciliation), `red-A.log`, `red-B.log`, `red-C.log` (each opening with the mutation quoted from disk and a `git diff --stat`). |
| authoritative run | `--cross`, 422 legs launched and 422 reported, exit 0, `logs/cross-FINAL3.log`, plus one `.lua.log` and one `.sigil.err` per leg under the same directory. The 90-second run without `--cross` is 38 of those legs. |

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
through the switches it reads: seven derived switches across the two corpora
(three in Sonic 1, four in Sonic 2),
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
* **Second-order interaction between corpora or with the corpus's own Lua
  settings.** `build.lua`'s `improved_sound_driver_compression` and
  `improved_dac_driver_compression` are settings too, and they are not swept:
  they are Lua locals, not `.asm` declarations, and flipping them changes the
  reference toolchain's own behaviour rather than the source. Booked.
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

These are the 38 one-at-a-time legs, which are a subset of the authoritative
422-leg `--cross` run. 38 launched, 38 reported. 16 agree, 2 refuse and are
acknowledged, 18 are
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

## The full cross product, which was assumed infeasible and is not

The brief and this note's first draft both said a full cross product was
infeasible. The arithmetic refutes it. Sonic 1's seven swept switches have
domains 3,2,2,2,2,2,3 and Sonic 2's six have 3,2,2,2,2,2, which is **288 and 96
corners**, and a leg costs 2.4 seconds measured (38 legs in 91 seconds). The
whole product is about a quarter of an hour, and `--cross` runs it.

It is not a bigger sample. It tests three things one-at-a-time cannot.

**Composition, on the sigil side.** Phase 1 shows which single settings sigil
refuses. The prediction is that a refusal is caused by one setting and composes,
so whether sigil builds a corner is decided by whether the corner contains such
a setting and by nothing else. The causes are read off the SAME run's phase 1,
never from a table, so the prediction cannot be tuned to the answer it is tested
against. **384 corners, 384 held, 0 broke.**

**The stock toolchain's own reach.** See the finding below.

**Agreement.** Every corner both toolchains built must agree byte for byte, and
that is asserted per corner rather than read off a summary.

| | Sonic 1 | Sonic 2 |
|---|---|---|
| corners | 288 | 96 |
| composition prediction held | 288 | 96 |
| both toolchains built | 144 | 48 |
| ... of which agreed byte for byte | **144** | **48** |
| sigil declined (the one open row per corpus) | 120 | 48 |
| **the stock toolchain could not build** | **24** | 0 |
| distinct stock images across the corners | 126 | 96 |

Sonic 2's 96 corners produce 96 distinct stock ROMs, so no Sonic 2 corner is
vacuous. Sonic 1's 288 produce 126, which is the SRAM switches collapsing, and
is the same fact the one-at-a-time vacuity check found, seen from the other side.

### The finding: two of Sonic 1's own build options cannot be combined

24 of Sonic 1's 288 corners fail in **asl**, not in sigil, and they are exactly
`Revision = 0` AND `FixBugs = 1` AND `AllOptimizations = 0`, free across the
other four switches (2 x 2 x 2 x 3 = 24).

```text
87, 88, 89 Ending Sequence Sonic, Emeralds, Logo.asm(270):
error #1370: jump distance too big
  bra.w DisplaySprite    ; display sprite
```

Line 270 sits inside an `if Revision=0` arm and a `bra.w` reaches 32 KB.
`FixBugs` adds enough code between the branch and `DisplaySprite` to put the
target out of range, and `AllOptimizations` brings it back only because
`PaddingOptimization` removes 3,314 bytes. So **two documented Sonic 1 build
options cannot be combined unless a third is also set**, and the corpus does not
say so anywhere.

**One at a time neither fails**, which is exactly why a list of eleven single
flips could not have found it and why the product could.

This is a Sonic 1 defect and not a sigil one: the thing that fails is the stock
toolchain. So those corners are their own category, covered by a rule in
`ACK_STOCK_DECLINE` that states the partial assignment responsible rather than
listing 24 tags. The covered set and the observed set are asserted equal in both
directions, and a rule that covers zero corners is as loud as a corner covered by
no rule, so the entry cannot outlive the defect.

**What is NOT known, and is booked rather than guessed:** sigil stops on the
width suffix before it ever reaches that branch, so whether sigil would ALSO
refuse those 24 corners for the branch-range reason is unmeasured. An assembler
that quietly assembled an out-of-range `bra.w` would be a silent wrong-ROM fault
of exactly the class this campaign has been closing. It becomes measurable the
day the width row lands.

### Diagnostic parity: sigil is silent where asl warns, once

Two toolchains can agree on every byte and disagree about what they told the
person who ran them, so every leg where BOTH toolchains ran has its warnings
compared. Over 422 legs there are exactly two parity keys:

* **`asl#180`.** asl says `address is not properly aligned` about
  `move.w (1).w,d0` at `s2.asm:30438`, which sits inside `if gameRevision=0` and
  which the source itself annotates `causes a crash because of the word
  operation at an odd address`. **sigil is silent.** The bytes agree, so this is
  a diagnostic sigil does not have rather than a ROM fault.

  The counts, stated apart because they measure different things: asl emits it
  on **33** legs, which is every leg whose source has `gameRevision = 0` (32
  cross corners plus the phase-1 arm). Parity is only measurable on the **17**
  of those where sigil also built, the other 16 being the `fixBugs = 1` half
  that sigil refuses, and on all 17 sigil says nothing. No sigil output in the
  whole 422-leg run contains the string `align`.

  It is the **only** asl warning code either corpus raises at any corner of its
  option space, and that clause is what the product licenses and a sample could
  not.
* **`sigil-only:` the `shared` warning**, on all 56 Sonic 2 legs where both
  toolchains ran, which is the standing `-c` residual and is not new.

A near-miss worth recording because it was nearly filed as a finding: sigil
emits `[as.warning] 'Revision = 2' is unnecessary with 'FixBugs' enabled` on 48
Sonic 1 legs, and a grep that looked only for `warning #<n>` said asl did not.
**asl does**, as a source `warning` directive rather than a coded warning
(`sonic.asm(139): warning: ...`). Keying asl's coded warnings apart from source
`warning` directives is what keeps that distinction, and it is why the parity
measurement reports no gap there.

## The controls, all eight, each shown red

No figure above comes from a run whose controls did not pass, and every control
has been shown firing on a mutation that was printed back from disk first. The
runner runs C1 to C6 and C8 before it builds anything and aborts printing no
table if any fails; C7 runs once per corpus inside the sweep.

| control | what it proves | how it was shown red |
|---|---|---|
| C1 no-op edit | an edit that writes back the value already there is refused, because an unapplied mutation and a real pass are the same artifact | in-runner, on a file written for the purpose |
| C2 wrong line | an edit aimed at a line that does not declare the switch is refused | in-runner |
| C3 shadowed declaration | a second column-0 declaration of the same name is refused, because the edit would be overridden | in-runner |
| C4 blind comparer | a comparer that cannot see a planted byte is refused | in-runner, by substituting a comparer that always reports 0 |
| C5 option outside the block | a documented declaration outside `ASSEMBLY OPTIONS` fails the cross-check | in-runner |
| C6 one-element domain | a switch whose prose derives a single value fails rather than executing nothing | in-runner; this is the gate `padToPowerOfTwo` tripped |
| C8 warning normaliser | a source `warning` directive both toolchains fire normalises to ONE string, while a coded asl warning and a sigil-only warning stay separate keys | in-runner, over the two real diagnostic shapes these corpora produce. It needs its own control because the shape that matters most occurs only at corners sigil refuses for the unrelated width reason, so **no leg in a passing run exercises it**, and an untested normaliser would report a parity it never checked |
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
| warning-parity set | renamed the `asl#180` acknowledgement | `found-not-acknowledged=[('s2disasm', 'asl#180')] acknowledged-not-found=[('s2disasm', 'asl#999-NOT-REAL')]` |

Two more are asserted by `--cross` and were exercised by the first cross run
rather than by a deliberate mutation, which is a weaker proof and is said so
here: the **stock-decline rule coverage** (the first cross run had no rule, all
24 corners reported as breaking the prediction, and adding the rule is what
turned them green) and the **corner-count** check (`enumerated N corners, the
domains give M`), which has never been seen red.

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
* **The half of the space behind the two open rows.** 120 Sonic 1 corners and 48
  Sonic 2 corners stop at the width suffix and at the driver-size constant
  respectively. Those corners are **blocked, not absent**: the day the front-end
  width row lands, `--cross` reaches them and nothing says what is in them.
* **`build.lua`'s own Lua settings.** `improved_sound_driver_compression` and
  `improved_dac_driver_compression` change the `-z` algorithm both toolchains
  are given, so they are a real sweep axis and a cheap one, since
  `derive_tool_args` already evaluates them. Not swept. Booked.
* **Fault 3.** Diagnosed to one line and one round, not fixed. Front-end row.
* **Whether sigil refuses the 24 corners asl cannot build.** Unmeasured because
  sigil stops earlier. Booked.
* **Runtime.** No emulator. TAGGED for the controller.

## Reproducing

```text
CARGO_TARGET_DIR=<a path on disk, never /tmp> cargo build --release --bin sigil
python3 scripts/switch_matrix_sweep.py --sigil <that binary>            # 90 s
python3 scripts/switch_matrix_sweep.py --sigil <that binary> --cross    # 17 min
```

Nothing else. The runner extracts the corpora itself with `git archive`, derives
the domain, builds every leg with both toolchains, and exits nonzero unless every
reconciliation holds. `--corpus name=path` points it at a different checkout;
`--only <tag>` runs one leg and says loudly in its own output that the run is a
probe and not a sweep result.

The previous parcel's note ends with a Reproducing block naming
`scripts/mk_gen_trees.sh` and `scripts/flip.sh`. Neither is under `scripts/`:
`mk_gen_trees.sh` exists only inside the census note's own directory, and
`flip.sh` exists nowhere in the repo (the census committed `switchflip.sh` and
`switchflip2.sh`). That block is not runnable as written, and this runner
replaces the workflow it describes.
