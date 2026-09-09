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
| C-1 | **There is no help** | job 1: `--help` consumed as a filename, the `.emp` track undiscoverable from the binary | F3: the only reachable usage text names 1 of 6 entry points |
| C-2 | **A success line that does not mean the build succeeded** | F1: `built: N bytes`, exit 0, no file written | F14: `--prelude` parsed, consumed, dropped, run reports success |

C-1 is the one the briefs bought. C-2 is the panel's headline and neither seat could have found the
other's half: one arrived by trying to produce a ROM, the other by sweeping flags.

---

## Findings the controller re-derived firsthand

### 1 . An unrecognized processor name silently assembles as 68000 . **firsthand** . UXb F1/F2

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

### 2 . `built: N bytes` says the same thing whether or not a file was written . **firsthand** . UXa F1

In a directory holding nothing but the source, `sigil emp <file>` prints `built: 4 bytes` and exits
0 **and writes nothing**. The run that does write the file prints the byte-identical line. So the
output cannot distinguish "your ROM is on disk" from "your ROM was discarded".

Cost in the stated unit: one wasted cycle at best, and at worst a porter who believes a stale
artifact is fresh, which costs every cycle after it until noticed.

### 3 . `--prelude` is parsed, consumed, and dropped, and the run reports success . **firsthand** . UXb F14

`--prelude <path>` without `--root` accepts a path **that does not exist**, prints `built: 4 bytes`
and exits 0. Its sibling `--map` under the identical precondition refuses by name, states the
precondition, and exits 2: `error: --map requires --root (region placement is a multi-module
concern)`. **The correct behaviour is written thirty lines away in the same handler.**

### 4 . There is no help, and the usage text names one entry point of six . **firsthand** . both seats

`--help`, `-h` and `help` are each read as an input FILENAME:
`error: cannot read --help: No such file or directory`. The only usage text, printed by a bare
invocation, is two lines naming `<input.asm>`, `-o` and `--hex`. It mentions no subcommand, so the
whole `.emp` track is undiscoverable from the binary. Two accepted flags (`--map`, `--deny-todo`)
appear in no usage line at all, and `main.rs`'s own header lists five entry points against the
dispatch's six.

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
- **UXb.** `error: unexpected character` names neither the character nor a column. AS diagnostics
  drop a column the `.emp` front end on the same binary proves is available, and repeat
  byte-identically for two operands on one line.

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
