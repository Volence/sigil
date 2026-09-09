# Sigil Roster C (UX seat pair), packet

**Panel run 2026-09-09 at pin sigil `b5b0219b`** (the code merge, not the lane-log commit that
carries it: a docs SHA standing in for a code guarantee is the SHA-class bar, and this lane made
exactly that mistake two hours earlier in a message to aeon).

- **Seats:** UXa (task walk, five newcomer jobs written down BEFORE reading anything, then attempted
  on README and `--help` only) and UXb (heuristic audit, the surface enumerated by the seat against
  a fixed checklist). Opposed, not reading each other, neither reading the controller.
- **Population walked:** someone porting an existing disassembly to this assembler.
  **Cost unit:** a build cycle.
- **Test:** *does the person lose information they need to act, OR to know they are being misled?*
  Adopted from the empyrean hub as protocol rather than invented here, deliberately, because two
  panels using different tests produce bins nobody can compare.
- **Surfaces:** diagnostics, `--help`, the README, the shape of the command line. `--help` was named
  IN BOUNDS in both briefs; see the inheritance note at the end for why that one word mattered.
- **Out of scope and therefore UNEXAMINED rather than cleared:** `sigil build` against a real aeon
  tree. Both seats were forbidden an aeon tree, so the contract closure gate, the region budget,
  the image bounds and checksum reporting, the three `--report` outputs and every `SIGIL_WARNINGS`
  tier went untested. **That is the surface a porter reaches LAST and then spends the most cycles
  inside**, so the untested fraction is weighted toward where the cost is.

**28 findings, 26 exercised.** Verification column: **firsthand** means the controller re-derived it
against the pinned tree; **seat** means it is carried on the seat's committed evidence.

---

## CONVERGENCE, the panel's strongest signal

| # | target | UXa met it as | UXb met it as |
|---|---|---|---|
| C-1 | **There is no help** **CLOSED** | job 1: `--help` consumed as a filename, the `.emp` track undiscoverable from the binary | F3: the only reachable usage text names 1 of 6 entry points |
| C-2 | **A success line that does not mean the build succeeded** **CLOSED** | F1: `built: N bytes`, exit 0, no file written | F14: `--prelude` parsed, consumed, dropped, run reports success |

C-1 is the one the briefs bought. C-2 is the panel's headline and neither seat could have found the
other's half: one arrived by trying to produce a ROM, the other by sweeping flags.

---

## Findings the controller re-derived firsthand

### 1 . An unrecognized processor name silently assembles as 68000 . **firsthand** . UXb F1/F2 . **CLOSED**

**Closed on branch `parcel/emp-cpu-name-refusal`.** Refused by name at the value's span against
`lower::CPU_SPELLINGS`, the ruling below now enumerated onto the `.emp` surface. Two things the
finding did not anticipate: the value reaches THREE call sites, not two (the third re-reads an
attribute already refused, so it resolves silently and one mistake earns one diagnostic), and two
further mirrors of the resolver live outside the lowering module, in `corpus_contracts` and in the
`cycle_fraction` harness bin. All five now read the one table.

`section code (cpu: banana, vma: $0)` holding 68k code and the identical file spelling
`cpu: m68000` both emit `70 01 4E 75`, `built: 4 bytes`, **exit 0, no diagnostic anywhere**.
Re-derived here on the seat's own committed probes.

**The second half is worse, and it is why this is first.** On a section the source declares as Z80,
the typo makes the tool *speak*, and what it says points away from the cause: it calls `ld` and
`ret` unrecognized **68000** mnemonics for a section the source says is not 68000. The repair the
message implies is rewriting correct instructions.

**⚠ THIS LANE HAD ALREADY RULED ON THIS EXACT CLASS AND APPLIED THE RULING TO ONE SURFACE ONLY.**
`docs/OVERSEER.md` carries *AS-DEFAULT-CPU is REFUSE BY NAME*: a source with no `cpu` directive is a
hard error naming what was not declared, never a silent default of any processor. That ruling was
made for the AS frontend. **The `.emp` surface has the same defect and was never enumerated**, which
is this lane's own banked bar (*a property verified at the producer is not a property of the
consumers, and the population to enumerate is always the consuming end*) failing on a ruling rather
than on a value. An unrecognized NAME is also a different door from a MISSING declaration, so even
within the AS surface the ruling's population deserves re-deriving.

### 2 . `built: N bytes` says the same thing whether or not a file was written . **firsthand** . UXa F1 . **CLOSED**

**Closed on branch `parcel/emp-success-line-truth`.** The line states the disposition of the image
and the wrote-case names the path:

```
built: 13 bytes, no file written (pass -o <path> to write one)
built: 13 bytes, wrote out.bin
```

`built: N bytes` stays the prefix, because the byte count is the fact both outcomes share and is
what the acceptance gates assert on; the disposition is the suffix that separates them. Building
without `-o` stays a SUCCESS, since `--hex` and a plain syntax check are legitimate uses, and a
control gate holds that open so the honest line cannot later be bought by turning the no-output case
into an error.

The gates take the row's own evidential point as their design: they assert on the FILE (it exists,
and its real length equals the count the line printed) rather than on a golden string, because
stdout is the thing under test and cannot also be the ground truth, and they run two shapes of
different byte length so what is established is the code path's behaviour.

In a directory holding nothing but the source, `sigil emp <file>` prints `built: 4 bytes` and exits
0 **and writes nothing**. The run that does write the file prints the byte-identical line. So the
output cannot distinguish "your ROM is on disk" from "your ROM was discarded".

Cost in the stated unit: one wasted cycle at best, and at worst a porter who believes a stale
artifact is fresh, which costs every cycle after it until noticed.

**The re-derivation strengthened the row rather than confirming it, and UXa spotted why before this
seat did.** The seat measured `built: 13 bytes` on `examples/guards.emp`; the controller measured
`built: 4 bytes` on a different file. **A single input can only support "this file does it"; two
inputs at different byte counts make it the code path's behaviour rather than the example's.** The
generalisation was not available from either measurement alone, and neither party set out to run a
two-input control: it fell out of the controller using its own probe instead of the seat's.

### 3 . `--prelude` is parsed, consumed, and dropped, and the run reports success . **firsthand** . UXb F14 . **CLOSED**

`--prelude <path>` without `--root` accepts a path **that does not exist**, prints `built: 4 bytes`
and exits 0. Its sibling `--map` under the identical precondition refuses by name, states the
precondition, and exits 2: `error: --map requires --root (region placement is a multi-module
concern)`. **The correct behaviour is written thirty lines away in the same handler.**

**Closed on branch `parcel/emp-success-line-truth`, and the diagnosis above is HALF WRONG in a way
worth keeping on the record.** Two candidate defects were carried forward for this row: (a) the flag
silently no-ops without `--root`, and (b) it accepts a path that does not exist. Only (a) is real,
and it is the whole of it.

**(b) does not exist, and its frame is wrong.** `--prelude` takes a MODULE ID, not a path, which the
usage string already said (`--prelude <module.id>`). With `--root`, a bogus id is already refused at
exit 1 by `reachable_modules`: ``no module `nope` found under the scan root``, blamed at the seed
span. So there is no path for a not-found check to check. The transcript that looked like (b) was
(a) wearing a path-shaped argument, and the argument's shape was doing all the work: the same run
with a bare id (`--prelude prelude`) behaved identically.

The fix mirrors `--map` exactly, at the same exit code, with a gate pinning the two together so a
future change to one cannot leave the other behind:

```
error: --prelude requires --root (a prelude is a module id resolved under the scan root,
not a file path); pass --root <dir>, or drop --prelude
```

`--map`'s own message was corrected in the same change. It named the precondition but not the fix,
which is the half of the wording rule this lane keeps closing on itself, and it now ends
`pass --root <dir>, or drop --map`. The sibling that was the model for the refusal turned out to
need the same treatment as the surface it was modelling.

### 4 . There is no help, and the usage text names one entry point of six . **firsthand** . both seats . **CLOSED**

`--help`, `-h` and `help` are each read as an input FILENAME:
`error: cannot read --help: No such file or directory`. The only usage text, printed by a bare
invocation, is two lines naming `<input.asm>`, `-o` and `--hex`. It mentions no subcommand, so the
whole `.emp` track is undiscoverable from the binary. Two accepted flags (`--map`, `--deny-todo`)
appear in no usage line at all, and `main.rs`'s own header lists five entry points against the
dispatch's six.

**Closed on branch `parcel/cli-help`.** All three forms print help on stdout at exit 0, `sigil
<command> --help` and `sigil help <command>` print that command's page, a bare invocation keeps
exit 2 and points at `--help`, and every wrong shape keeps the exit code it had.

**The fix is a table, not a help string, and that is the whole point of the row.** A help text is a
second copy of the interface, so the copy was removed instead of being written: `ENTRIES` holds one
row per entry point (words, label, summary, usage, the function that runs it), `main` dispatches by
walking it, both the top-level list and every per-command page render from it, and each run function
prints its own row's usage on a missing argument. A seventh command cannot be dispatched without
being listed, because the list IS the dispatch. The five per-command usage strings the seats found
were good were kept as the rows' text, checked against the code first: `--map`'s note was written
from the flag's own refusal message and said it WRITES a placement report, and it reads one.

**The `--map` and `--deny-todo` half is closed as a CLASS, not as two instances.**
`usage_names_every_accepted_flag` reads each entry point's own argument-match arms out of the source
and fails if a flag it accepts is missing from that entry point's usage, and fails again if any
argument parser in the file belongs to no entry point. It surfaced three more undocumented flags the
seats had not reached (`--native`, `--stress-evict`, `--stress-art`, all on `build`), now on a `dev:`
line. The seats' count of two was a floor.

**Two gate defects, both found by mutation, both the same shape, and both would have shipped a gate
that could not fail for its own subject.** Dropping a command from the list left the binary-level
gate green, because the page's footer says ``for example `sigil emp --help` `` and the gate asked
whether the page contained the string `emp`. Deleting `emp` from the `sigil emp` usage line left two
gates green, because the line still reads `usage: sigil <input.emp>` and `emp` is inside
`<input.emp>`. **Every command in this CLI is a substring of the argument it takes**, so a substring
search over help text can never answer a question about which command a line is about. Both gates
now compare the token after `sigil ` against the table's own words, and the mutation that produced
each defect is red in both places.

Ten mutations, each shown on disk before its run, restored from a committed baseline: nine red at
the gate named for them, and the tenth (deleting the `--deny-todo` note line) green because it was a
BAD MUTATION rather than a bad gate: the flag was still in the synopsis line above, so the run never
produced the defect. Re-run with the flag removed from both places, it is red.

---

## Carried on the seats' evidence

- **UXa F4.** A real AS disassembly yields 5,227 errors of which **4,933 (94.4 percent)** are one
  unsupported feature, nameless temporary symbols, reported as `bad operand expression` or
  `expected mnemonic, directive, or label`. Neither message names the construct or says it is
  unsupported, and **no supported-subset statement exists anywhere in the product.** This is the
  measurement behind the `AS-NAMELESS-LABELS-RC1` row, met from the user's side rather than the
  implementer's.
- **UXa F5.** The error list is silently partial across phase boundaries: 7 errors present, 5
  reported, 2 withheld until the first 5 were fixed, with no count and no note. Each hidden error
  is a full build cycle.
- **UXa F8.** A source `message` directive goes to stdout while diagnostics go to stderr, so a
  habitual `> build.log` captures the reassuring line and none of the errors.
  **CLOSED on branch `parcel/ux-message-stream-and-diagnostic-columns`, and the seat's remedy was
  the wrong one.** Reproduced exactly. But moving the stream is refused at the CONSUMING end:
  asl writes `message` to stdout (probe `p1b`), and this repo's own
  `scripts/corpus-baseline.sh:160` splits the two streams on purpose and treats stderr as the
  diagnostic POPULATION it counts, classifies, and `comm`-diffs against a stored baseline. Both
  corpora fire `message`, so the move would have inflated that count and made every stored
  baseline incomparable, to fix a log nobody had to keep. The fix keeps the stream and makes
  stdout unable to END a failing run on a reassuring note:
  `assembly failed: 1 error (reported on stderr)`.
  **A claim in the pinning test's header turned out to be false** and is corrected there: it said
  s1disasm and s2disasm read their size lines off this stream, and both invoke the assembler
  through `os.execute`, which captures no output at all.
- **UXb.** `error: unexpected character` names neither the character nor a column. AS diagnostics
  drop a column the `.emp` front end on the same binary proves is available, and repeat
  byte-identically for two operands on one line.
  **CLOSED on the same branch, and the row was THREE defects rather than one.** The seat framed
  the repeat as a consequence of the missing column. It is not: `SourceMap::label` now carries a
  column and `dc.b Big, Big` still printed the same column twice, because every item of a data
  directive was blamed at the DIRECTIVE's span. The item's own span was available and discarded,
  exactly as the column was, and both had to move. The three parts red-separate cleanly under
  mutation, which is the evidence that they were three.
  The two dialects are untouched, and the gate holds the split open in BOTH directions rather than
  only the direction this parcel touched.
  **The finding was more right than it knew, and the controller's first fix was wrong.** asl
  ALREADY prints a column, spelled `file(line):col:` (reference build, `h.asm(2):9: error #1010`,
  with its numbers across five assignment spellings equal to the columns the offending name starts
  at). So sigil's `file(line)` was asl's format with the column DELETED, not merely less than the
  `.emp` surface. The first fix here rendered `file(line,col)` from the Microsoft convention
  without checking what the incumbent does, which would have made sigil a third dialect answering
  a question asl had already answered; the evidence was two files away in
  `sigil-frontend-as/src/eval.rs` the whole time. Corrected before landing.

## Look and taste, captures for the owner and NOT packet findings

Both seats kept these separate. `--version` verbosity, two location formats across the two
dialects (deliberate and ruled, and the seats did not know that), and message casing.

---

## What the test did, measured rather than asserted

**The amendment changed both seats' bins, and it changed the headline.** UXb: three of fourteen
findings moved from discarded to booked, including its top two. UXa: its number one finding falls
out into taste under the unamended wording and is its headline only because of the amended half.
So *or to know they are being misled* is not a refinement, it is what makes this panel's principal
result visible at all.

**UXa proposes a third clause and its argument is good: *or to know what the run actually did*.**
Its case is finding 2 above, which sits awkwardly in both existing halves. Nothing false is stated,
because nothing is stated at all; and nothing needed for the next action is missing. What is lost is
the ability to distinguish two materially different outcomes, and it is lost to an **absence** rather
than to an assertion. Forwarded to the hub as a proposal, not adopted here.

**Neither seat would have invented a different test after seeing the amended one**, and UXb named
what it would have proposed before (*would the person's next action be wrong?*) and rejected it
itself, because it makes severity depend on predicting a person's move and so reintroduces the
imagined-user problem. **Both seats' answers are anchored** by having read the base test first;
this lane has no uncontaminated data point and has told the hub so.

---

## Rig conditions, kept separate from findings

- **The controller walked into the pipe trap while adjudicating, and so did UXa.** `cmd | head`
  reports `head`'s exit status: this seat read `--map` as exiting 0 on an error path, which would
  have been a truthfulness finding, and it exits 2. Re-measured without the pipe. **Two independent
  parties hit the same instrument defect on the same tool in one night**, which is a convergence
  about the instrument rather than the product: any exit-code claim about this tool needs the
  no-pipe form.
- UXb's diagnostic-literal scanner **silently discarded 40 percent of its population** on its first
  version (238 distinct literals against a true 474) by not matching macro-form helper calls. A
  canary caught it before any count was published. Corrected figures: 807 call sites, 474 distinct
  literals, 130 files, and the seat states this is a floor rather than a census.
- UXa's ten `cannot include .../generated/*.inc` errors in its s2 run are its own, from not running
  that corpus's generator, and are excluded from its analysis.

## Clean walks, reported because a report of only failures cannot be told from one that only looked for them

UXb: 58 invocations across all 6 entry points, exit-code discipline held 58 of 58, and **every
success summary reachable without an aeon tree is honest**, with `built: N bytes` matching the real
byte count in 6 of 6 countable cases. That row is the one this lane most expected to break.
**The truthfulness defects here are not in the summaries. They are upstream, where a knob is
misunderstood in silence, and the summary that follows is then perfectly accurate about a program
the person did not write.** UXa: 12 things worked, the minimal 68000 job produces correct bytes, and
`no processor declared` is the best diagnostic in the product.

## What the briefs got right, and the one thing that cost a finding

**Naming `--help` in bounds produced a finding in both seats**, and it is the surface a task-walking
seat would otherwise have routed around by reading the README. That word came from the oracle
pilot's packet; without it this panel would have missed the most-used surface in the product.

**Enumerating nothing for UXb produced two findings its own charter would have hidden**, including
the entry-point omission, which is the pilot's *a list that reads as complete* reproduced inside
this tool's own documentation.

**The transcript requirement did not catch everything it was written for, and UXa said so.** Its
near-miss had a real command and real output and a wrong conclusion: it nearly filed a failed s2
build's stdout as a false success report, and the line turned out to be the disassembly's own
`message` directive. It caught itself because the result was convenient. **The clause it recommends
for the next brief, and this lane already carries it as a memory: a convenient result is a
trigger.**
