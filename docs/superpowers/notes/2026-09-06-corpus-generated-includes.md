# CORPUS-GENERATED-INCLUDES: the headline metric was measured over an unfinished tree

Parcel note. Branch `parcel/corpus-generated-includes`.

## What this is about

The SIGIL-AS-REPLACEMENT project steers by one number: the diagnostics `sigil`
emits over a community Sonic disassembly. Every row has been sized off it.

Both corpora have build-time generated include files that their own `build.lua`
writes from `.wav` and `.asm` sources, and that are **gitignored**. A bare `git`
checkout has none of them. A run over a bare checkout counts the assembler's
defects PLUS an absent generator's output, and nothing in the run separates the
two.

`git status` is blind to this. A bare tree and a prepared tree both report
**0 dirty paths**, because everything the generator writes is ignored. The usual
provenance stamp (revision plus dirty count) therefore cannot tell them apart,
which is why this survived so long.

## Provenance

| | |
|---|---|
| sigil | branch tip, which at measurement time equals master `7b6f7dc7`; built into `.target-land` in this worktree, md5 `93b368052dbb77a5d2c199134f9c002d`, 8,094,760 bytes |
| corpora | `s2disasm` at `e45ebf33`, `s1disasm` at `f6ece657`, `skdisasm` at `2fcd861`, each in its OWN detached worktree under `/home/volence/sonic_hacks/.corpus-cgi/`; the shared live checkouts were never written |
| generator | each corpus's own `build.lua`, generator half only, run by `scripts/corpus-prepare.sh` under `Lua 5.5.1` |
| corpus dirty | 0 paths, both trees, both before and after generation (see above: that is the point) |
| oracle | none. No `asl` invocation produced any value in this note. The corpus's own `asl` runs as part of `build.lua`'s music compression, as a build tool, and is never asked a question |

## The restated baseline

**Sonic 2**, `s2disasm` `e45ebf33`, entry `s2.asm`:

| | bare checkout | prepared | delta |
|---|---|---|---|
| **diagnostics** | **5,223** | **5,162** | **-61** |
| `cannot include` | 39 | 0 | -39 |
| `int(): could not evaluate float expression` | 17 | 0 | -17 |
| `unresolved symbol X in operand` | 24 | 21 | -3 |
| `unresolved if condition: X has no value` | 2 | 0 | -2 |
| every other class | unchanged | unchanged | 0 |

Population: 5,223 and 5,162 non-empty stderr lines, every one of them parsed as
`file(line): level:`. Nothing APPEARED and nothing ROSE; the new-lines set is
empty (0) and the lost-lines set is 45 distinct texts.

The prepared class table, in full, over a population of 5,162:

```
error      2624  bad operand expression
error      2309  expected mnemonic, directive, or label
error        89  `X` is not a recognized N mnemonic
error        49  bad word expression
error        30  bad byte expression
error        21  unresolved symbol `X` in operand
error        11  unexpected character
error         8  instruction needs an explicit size suffix (.b/.w/.l)
error         6  case needs a string literal
error         4  malformed number (hex needs a trailing `X`)
error         3  bad displacement expression in `X`
error         2  switch needs a string expression
error         2  trailing tokens in operand
error         1  struct `X` has a member line this cannot read
error         1  unknown directive or mnemonic `X`
error         1  unresolved elseif condition
error         1  unresolved if condition
           5162  TOTAL
```

**Sonic 1**, `s1disasm` `f6ece657`, entry `sonic.asm`:

| | bare checkout | prepared | delta |
|---|---|---|---|
| **diagnostics** | **49** | **42** | **-7** |
| `cannot include` | 3 | 0 | -3 |
| `int(): could not evaluate float expression` | 4 | 0 | -4 |

Populations 49 and 42 lines. Generated files written: 74 for Sonic 2 (7 DAC
`.inc` + 7 `.dpcm`, 31 music `.inc` + 27 `.sax`, 1 PCM `.inc` + 1 `.pcm`), 8 for
Sonic 1.

**Sonic 3 and Knuckles**, `skdisasm` `2fcd861`, entry `sonic3k.asm`. This corpus
was not in the brief. It was checked because the same class had to be either
ruled in or ruled out for it, and it is ruled IN:

| | bare checkout | prepared | delta |
|---|---|---|---|
| **diagnostics** | **2,170** | **2,126** | **-44** |
| `cannot include` | 15 | 0 | -15 |
| `int(): could not evaluate float expression` | 67 | **38** | -29 |

Populations 2,170 and 2,126 lines; 102 generated files written; 50 generated
paths named by the corpus. `sigil` was run plain, without `buildSK.lua`'s
`-D Sonic3_Complete=0`, which is how every previous corpus run has invoked it.

**The float class does not go to zero here.** Sonic 2 and Sonic 1 both do; this
one keeps 38 rows that are a real gap and have never been looked at. A parcel
sizing that gap off the bare figure would size it at 67, which is 76 percent too
big. That is in the gap ledger.

## What the old number was, and what happened to it

The number in circulation is **5,229**, from
`2026-09-06-as-float-freq-table.md`. It is not wrong for what it measured, and
subtracting is not how to get from it to today's figure. Two different things
moved.

| | | bare | prepared |
|---|---|---|---|
| master `0eb20272` | before the float builtins landed | 5,229 | 5,168 |
| master `7b6f7dc7` | after them | **5,223** | **5,162** |

* The **6** between the two rows is the six `int(log(number))` sites at
  `s2.asm(87677)` that the float parcel closed. That is a real assembler
  improvement and it is in both columns.
* The **61** between the two columns is the corpus's own build system, at every
  sigil revision. It is not an assembler property at all.

`5,229` was correctly measured over a bare checkout of a binary that no longer
exists on master. The figure to carry forward is **5,162**, and the offset to
remember is **61**.

The `cannot include 39 -> 0` and `float 23 -> 6` rows of that note reproduce
exactly. Its `23` is `17 + 6`: the 17 that are the missing files, plus the 6 that
were `log`.

## The 61 does not decompose into "noise to subtract"

Two reasons a reader should not treat the old totals as "5,162 plus 61 of noise".

**First, the missing files do not only ADD rows, they CHANGE one.** Sonic 1
bare and prepared both report exactly one `The driver is too big` error, and the
number inside it moves:

```
bare      sound/z80.asm(229): ... It currently takes 72325h bytes.
prepared  sound/z80.asm(229): ... It currently takes 73DFDh bytes.
```

The bincluded DPCM data is part of the driver's size. Sonic 3 and Knuckles does
the same thing in a different message: `Function GetPointerTable is at 0EA188h`
becomes `at 0F0908h`. A count is not the only thing an absent generated file
perturbs, and in both cases the perturbed message is an assertion the corpus
makes about its own layout.

**Second, a partly prepared tree gives a third number.** With one of the 39
generated includes present but unreadable, the same binary over the same
revision reports **5,164**: not the bare 5,223, not the prepared 5,162, and
higher than either endpoint of the range a reader would expect. There is no
monotone "less prepared means more diagnostics" rule to lean on.

## The deltas the old totals were used for survive, and this is measured

The old absolute numbers were inflated. The DELTAS parcels reported off them are
a separate question, and leaving that question open would leave a refuted
mechanism's arithmetic standing.

`2026-09-05-as-z80-instruction-coverage.md` reported `5247 -> 5229`, delta
**-18**, over bare trees. Re-measured over the PREPARED tree, with binaries
built from the parcel's own base `c38b44fd` (md5 `4243a728c3c09dcd43659d167daede5e`)
and tip `6bdf8d86` (md5 `40ee42921b6df71c141a70124bc15171`):

```
error   18 -> 1   -17   unknown directive or mnemonic `X`
error    1 -> 0    -1   unsupported form: Sbc, ops: [Pair(Hl), Pair(Bc)]
      5186 -> 5168  -18  TOTAL
```

Same delta, same two classes, no class ROSE or APPEARED, name sets identical in
both directions. Both endpoints moved by exactly **61**, the same 61 measured at
today's master. So that parcel's finding stands and only its two absolute
figures were inflated. Its `5247` is `5186` and its `5229` is `5168`.

`5,168` there is also, independently, the float parcel's prepared figure for
master `0eb20272`. Nothing between `6bdf8d86` and `0eb20272` moved the corpus.

## The runner

Committed, because the durable deliverable is the thing the next parcel runs,
not this measurement. Every parcel so far has hand-rolled a `corpus.sh`, and a
hand-rolled one measures a bare checkout again.

`scripts/corpus-prepare.sh <corpus-dir>` runs the corpus's own generator half.

`scripts/corpus-baseline.sh --sigil BIN --corpus DIR --entry FILE` measures and
refuses. `--compare FILE` diffs against a previous run's stream, by class, by
unresolved-symbol name set in both directions, and by whole line.

`scripts/lib/corpus_classes.py` is the classifier, taking one stream or two.

### The cut point is derived, not a line number

The float parcel's account said the generation half is `build.lua`'s "first 186
lines, which stop before the ROM build". **Line 186 IS the ROM build.** At
`e45ebf33`, `s2disasm/build.lua` is 195 lines and line 186 is

```lua
common.build_rom_and_handle_failure("s2", "s2built", "", "-p=0 -z=0," .. compression .. ",Size_of_Snd_driver_guess,after", true, repository)
```

so `head -186` runs it. The generated set comes out the same either way, so no
number in that note is affected, but the mechanism as written is off by one and
a future reader following it would assemble a ROM they did not ask for.

`corpus-prepare.sh` finds the first line naming `build_rom_and_handle_failure`
and keeps everything before it, refusing if no such line exists rather than
guessing. It prints the cut line, so the derivation is checkable rather than
trusted, and it counts the lines it actually extracted against the number it
meant to extract. On Sonic 1 the same rule finds line 28 of 34, on Sonic 3 and
Knuckles line 28 of 34 in `buildSK.lua`.

**Not every corpus has a `build.lua`.** skdisasm ships `buildS3.lua`,
`buildSK.lua` and `buildS3Complete.lua`, one per ROM shape, and there is no
correct default among them. The script takes an optional second argument, and
when the default is absent it enumerates the root Lua scripts that name
`build_rom_and_handle_failure` and refuses rather than picking one:

```
FATAL: .../skdisasm has no build.lua.
       Root Lua scripts that build a ROM, any of which may carry the
       generator half; name one as the second argument:
         buildS3Complete.lua
         buildS3.lua
         buildSK.lua
```

### The readiness check derives its expectation

The design constraint was a check that must CLOSE rather than one that must
AGREE, and specifically NOT an enumerated list of expected generated files,
which would go green the day someone stopped maintaining it.

**Check 1, from the corpus's own sources.** Every distinct double-quoted path
naming a `generated/` component in the corpus's tracked `*.asm` and `*.inc`
files must exist on disk. s2disasm names 39 such paths, s1disasm names 4. The
expectation lives in the corpus, is written by the people who add generated
includes, and cannot be forgotten here.

**Check 2, from the run itself.** No diagnostic may name a `generated/` path.
This needs no parse of how the path was spelled.

**They are independent in both directions, and that is measured rather than
argued.**

* Source scan stricter: on a bare `s1disasm` it finds **4** missing while the
  run complains about only **3**. `sound/dac/pcm/generated/sega.inc` is in
  `s1.sounddriver.asm`, which the run never reaches. On a bare `skdisasm` the
  gap is far wider: **50** named, **15** complained about. A run-only check
  would call that tree 70 percent prepared.
* Run gate stricter: with `sound/DAC/generated/Kick.inc` present but mode `000`,
  the source scan reads `present 39, missing 0` and the run gate catches
  `cannot include ...: Permission denied (os error 13)`.

### Where it refuses instead of reporting a zero

* Empty readiness population is `VACUOUS`, printed as "this check has NOT
  established that the tree is prepared", never as a pass.
* `sigil` exit above 1 is `UNMEASURABLE` with the last 5 lines quoted, never a
  low count.
* An empty diagnostic stream is refused by the classifier rather than reported
  as 0, because a clean run and a run that never happened produce the same file.
* A generator that exits 0 having written nothing is a failure, not a no-op.
* Every count prints beside the population it was computed over.

## Red-first evidence

Two gates, two mutations, each shown applied on disk, each restored, each
producing a refusal the baseline run does not.

**Mutation 1: one generated file absent.** Against the prepared second tree:

```
$ ls -la .../s2disasm-b/sound/DAC/generated/Kick.inc
-rw-r--r-- 1 volence volence 74 Sep  6 03:33 .../Kick.inc
$ rm .../s2disasm-b/sound/DAC/generated/Kick.inc
  after rm, on disk: ls: cannot access '.../Kick.inc': No such file or directory
  generated files now: 73 (was 74)
```

The mutation is a file that is gone from the filesystem, so "applied" is not a
staged-versus-unstaged question. `git status` still reports the tree **clean**,
which is the whole hazard. The gate:

```
  population  39 generated path(s) named by the corpus
  present     38
  missing     1
    sound/DAC/generated/Kick.inc
  VERDICT: REFUSED.
CORPUS-BASELINE-END rc=3
```

**Restored from the corpus's committed baseline, not by hand.** Re-running
`corpus-prepare.sh` regenerated exactly the one file from its `.wav`
(`generated files now present: 74 (was 73, wrote 1)`), and the run returned to
`DIAGNOSTICS 5162 ... RESULT BASELINE`.

**Mutation 2: one generated file unreadable.** This one exists to prove the
second gate fires where the first cannot.

```
$ chmod 000 .../s2disasm-b/sound/DAC/generated/Kick.inc
mutation applied, mode now: 0 .../Kick.inc
source-scan view: -e says present
```

```
  present     39
  missing     0                      <-- check 1 sees nothing wrong
  diagnostics naming a generated/ path: 1  (of 5164 scanned)
    s2.asm(90887): error: cannot include ...Kick.inc: Permission denied
  VERDICT: REFUSED.
CORPUS-BASELINE-END rc=3
```

Restored with `chmod 644`, verified by `stat`, and the run returned to 5,162.

**What a run MUST fail, stated before running it.** The bare-tree run must
refuse with `rc=3` naming 39 missing paths, and both mutated runs must refuse
with `rc=3` while the prepared run reports `RESULT BASELINE`. All four came out
that way. Had the mutated runs reported a baseline, that would be a runner
defect and not a pass.

## The generator is deterministic, and this has a control

The Sonic 2 generator half reassembles 27 compressed songs through the corpus's
own `asl` (the flamewing fork, md5 `0dee1f98...`), which this lane refuses as an
ORACLE because it answers differently between runs for operands it declines to
value. It is used here as a build tool, not asked a question, and its runs exit
clean or `build.lua` aborts. That is an argument, so it was checked.

Two independent detached worktrees of `s2disasm` at `e45ebf33` were prepared
separately and every generated file md5'd:

```
77 files each, diff of the two md5 listings: IDENTICAL
```

77 rather than 74 because this control did not filter out the three tracked
placeholder files that sit in the `generated/` directories. The second tree also
reproduced the diagnostic count exactly: **5,162**, `READY (39/39)`.

## Anything in the brief I concluded was wrong

1. **"the first 186 lines, which stop before the ROM build".** Line 186 is the
   ROM build. Lines 1..185 are the generator half and the last generating call
   is line 182. No number is affected; the instruction is.

2. **The bare figure 5,229 is not a corpus constant.** It is `master 0eb20272`
   over a bare tree. Today's bare figure is 5,223 and the six-row difference is
   the float parcel's own fix, present in both the bare and the prepared column.
   Reporting a bare total without the binary is how a figure with two moving
   parts gets read as having one.

3. **"a run over a bare checkout counts defects PLUS an absent generator's
   output" understates it.** It also silently ALTERS a diagnostic that appears in
   both runs (Sonic 1's driver-size figure moves `72325h -> 73DFDh`), and a
   partly prepared tree gives a third number outside the range of the other two
   (5,164 against 5,223 bare and 5,162 prepared). "Subtract the noise" is not
   available even in principle.

4. **The Sonic 1 pairing is right but incomplete.** The brief said 4 float rows
   three lines below three `cannot include`. Confirmed. It does not mention that
   Sonic 1 names a FOURTH generated path the run never complains about, because
   the run dies before `s1.sounddriver.asm` reaches it. That asymmetry is the
   reason the source-derived check exists alongside the run-derived one.

5. **"a check whose failure mode is green because nobody maintained a list" was
   avoidable, and the way out was in the corpus.** The brief allowed "I could not
   find a derived shape, so I shipped nothing" as a legitimate outcome. The
   derivation was there: the corpus's own assembly sources name every generated
   path they need, so the expectation is maintained by the people who add
   generated includes and cannot be forgotten in this repository.

## What is left open

* **`docs/lane-log.jsonl` entries 208 and 216 carry `5,247 / 5,229` and
  `5,229 / 5,168`.** The lane log is an append-only dated record of what was
  believed at the time, and rewriting a historical entry is the shape this lane
  treats as a maintenance-act hazard. It is left for the overseer's next append.
  The replacements are in this note: `5186 / 5168` and `5223 / 5162`, and the
  Z80 delta of -18 is re-derived above and unchanged.
* **`skdisasm` keeps 38 float diagnostics after preparation** and nobody has
  looked at them. In the gap ledger, with the warning not to size the work off
  the bare figure of 67.
* **`skdisasm` was measured with the plain invocation, not `buildSK.lua`'s
  `-D Sonic3_Complete=0`.** That is how every previous corpus run has invoked
  it, so the figures here are comparable to the ledger's, but it is not the
  build the corpus's own script performs and the difference is unmeasured.
* **Aeon's own trees were not checked for this class.** The three roots the
  gap ledger's six-root sweeps use are in this repository's own build and were
  out of scope here.
* **No baseline figure is recorded anywhere machine-readable.** A future parcel
  still has to re-measure the before-side itself, which is correct (a stored
  number goes stale) but means the runner cannot yet tell a parcel it has
  regressed the corpus without being given both binaries.
