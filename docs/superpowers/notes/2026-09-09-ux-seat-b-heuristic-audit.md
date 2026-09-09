# UX seat B: heuristic audit of the sigil product surface

Pin: `b8594cf9d25210a50596a4daaaa7580862806c1b` (`git rev-parse HEAD` in this worktree).
Binary under test: `/home/volence/sonic_hacks/sigil/.target-uxb/release/sigil`, whose own
`--version` line reads `sigil 0.1.0 (b8594cf9)` and
`revision: b8594cf9d25210a50596a4daaaa7580862806c1b`, so the tool exercised is the tool at
the pin and not a stale install.

**Population swept:** someone porting an existing disassembly to this assembler.
**Cost unit:** build cycles.
**Test applied (amended):** does the person lose information they need to act, or to know
they are being misled?

Every finding below carries the command run and the output read. Findings reached only by
reading source are labelled **unexercised**; there are two, and they are marked.

---

## 1. Enumeration: how the surface was derived

The surface was derived three ways, and each way is stated with what it misses.

### 1a. Entry points (6)

Read the top-level dispatch `match` in `crates/sigil-cli/src/main.rs`, then confirmed each
one behaviourally by invoking it. The six are: the bare AS form (`sigil <input.asm>`),
`parse`, `emp`, `test`, `build`, and `--version` / `-V`.

The module doc comment at the head of `main.rs` lists five of these and omits `test`. The
dispatch is therefore the authority, not the doc comment, which is the first sign that a
written inventory here under-reports the live one.

### 1b. Flags (22)

For each entry point, read its argument loop, then cross-checked the loop against the usage
line that same entry point prints. The union is:

`-o`, `--hex`, `--root`, `--prelude`, `--map`, `--deny-todo`, `-D`, `--aeon`, `--emit-lst`,
`--game`, `--native`, `--debug`, `--config-a`, `--config-b`, `--lean`, `--stress-evict`,
`--stress-art`, `--extra-entry`, `--check`, `--report`, `--version`, `-V`.

The cross-check is what produced finding F7: two accepted flags appear in no usage line.
Had I taken either the usage lines or the README as the inventory, I would have had 20 and
missed both.

### 1c. Diagnostic messages (807 call sites, 474 distinct literals, 130 source files)

A scanner (`enum_msgs.py`, kept in scratch) walks every `.rs` file under `crates/` outside
`tests/` and `benches/`, matches a call to a named diagnostic-emitting helper, and takes the
first string literal in a three-line window. The helper names are passed in on the command
line rather than read off a sample of the population, and the scanner's own doc states the
shape it looks for without quoting an instance of it, so it cannot count itself.

Input count asserted independently:
`find crates -name '*.rs' -not -path '*/tests/*' -not -path '*/benches/*'` returns `130`,
matching the scanner's own `# files scanned: 130`.

Three canaries, each a message class I had separately confirmed the binary emits:

| canary | hits |
|---|---|
| `cannot read` (seen from `sigil --help`) | 8 |
| `usage: sigil` (seen from bare `sigil`) | 3 |
| `tree-tracked` (seen from `sigil --version`) | 1 |

**The canaries earned their keep.** The scanner's first version returned `0` for the
`usage: sigil` canary while returning `8` for the other, because its call pattern required
`name(` and so never matched a macro-form helper (`name!(`). That zero was invisible in the
result: 480 sites and 238 literals looked like a plausible census. With the pattern
corrected the same run returns 807 and 474, so the first instrument was silently discarding
40 percent of the population. No emptiness in this report rests on an instrument that was
not first shown to return non-empty for its class.

### 1d. What this method would have missed

Stated as a floor, not a census:

1. **The scanner keys on helper names I supplied.** A diagnostic built as a bare
   `Diagnostic { .. }` struct literal, or through a helper whose name I did not pass, is
   invisible to it. At least one such shape exists in the tree (a local `err(diags, span,
   message)` in the emp lowering module, and an `unlocated_error` constructor), so 474 is a
   lower bound on the distinct message population and the true figure is higher by an
   amount this method cannot report.
2. **It takes the first literal in a window.** A message assembled from variables reads as
   `<non-literal>`; a site whose first literal is not the message is misattributed.
3. **58 invocations is the whole behavioural sweep.** The great majority of those 474
   literals were never triggered. This report claims nothing about any of them.
4. **The largest blind spot is `sigil build`.** The brief forbids touching an Aeon tree, so
   the build subcommand was exercised only through its argument validation. Its entire
   run-time diagnostic surface (the contract closure gate, region budget and overlap, image
   bounds, the checksum, the `--report ram|contracts|indirect-cost` outputs, and every
   `SIGIL_WARNINGS` tier) is untested here. That is the surface a porter reaches last and
   then spends the most build cycles inside, so the untested fraction is weighted toward
   where the cost is.
5. Two parcels landing on master during this run add diagnostic text not present at
   `b8594cf9`. Nothing here is a claim about text outside this pin.

---

## 2. Findings, most severe first

### F1. An unrecognized `cpu:` name is silently taken as 68000 (exercised)

Fails checklist 1 (what happened), 3 (what to do), 4 (is it true), 6 (adjacent outcomes),
7 (reachable).

A section header declares its processor. Any spelling other than `z80` resolves to the
68000, with no diagnostic of any kind.

```
$ sigil emp cpu_absurd.emp --hex        # section code (cpu: banana, ...) holding data
exit=0
01 02
built: 2 bytes

$ sigil emp cpu_code.emp --hex          # section code (cpu: banana, ...) holding 68k code
exit=0
70 01 4E 75
built: 4 bytes

$ sigil emp cpu_ok.emp --hex            # the identical file with cpu: m68000
exit=0
70 01 4E 75
built: 4 bytes
```

Byte-identical output, exit 0, silence. The person is told the build succeeded; they are not
told the processor they named was not understood.

The second half is worse, because there the tool does speak, and what it says points away
from the cause:

```
$ sigil emp z80_ok.emp --hex            # section snd (cpu: z80, ...) holding `ld a,5 / ret`
exit=0
3E 05 C9
built: 3 bytes

$ sigil emp z80_typo.emp --hex          # the identical file with cpu: z80typo
exit=1
z80_typo.emp:5:9: `ld` is not a recognized 68000 mnemonic
z80_typo.emp:6:9: `ret` is not a recognized 68000 mnemonic
z80_typo.emp:4:5: [proc.undeclared-fallthrough] `Start` can reach its closing `}` without
    an unconditional terminator and does not declare `falls_into`, it will run into
    whatever follows it
```

The source says this section is not a 68000 section. The diagnostic asserts it is one, names
correct Z80 instructions as the defect, and offers a third diagnostic that is pure
consequence of the wrong backend. The implied repair, rewrite these instructions, is the
exact wrong action.

Mechanism confirmed rather than assumed: `attr_cpu` in the emp lowering module resolves
`z80` case-insensitively and returns 68000 for everything else. There is no unknown-cpu
diagnostic anywhere in the tree, so this is an **absent** message, not an unreachable one.
The two are different defects and this is the first kind.

Cost: for a data-only section, unbounded, because nothing ever surfaces. For a Z80 section,
many cycles, since no thread in the output leads back to the section header.

### F2. `--prelude` is silently discarded without `--root`, while its sibling `--map` is refused (exercised)

Fails checklist 4, 5, 7.

```
$ sigil emp ok3.emp
exit=0
built: 4 bytes

$ sigil emp ok3.emp --prelude no.such.module
exit=0
built: 4 bytes

$ sigil emp ok3.emp --map m.toml
exit=2
error: --map requires --root (region placement is a multi-module concern)
```

Same precondition, opposite handling. `--map` is refused with a message that names the
precondition and gives the reason. `--prelude` is parsed, consumes its value, and is dropped;
the run then reports success. A prelude naming a module that does not exist is accepted just
as quietly.

Mechanism: on the single-file path the compile call takes the source, the include root and
the defines, and no prelude; `prelude` reaches only the `--root` program path. So the flag is
not merely ineffective, it is unread.

The existence of the `--map` message is what makes this a finding rather than a preference:
the shape of the correct response is already written thirty lines away in the same argument
handler.

Cost: unbounded in the misled direction. A porter who supplies a prelude and is told
`built: 4 bytes` has no reason to check whether it was applied.

### F3. `--help`, `-h` and `help` all report a missing file (exercised)

Fails checklist 3, 6, 7.

```
$ sigil --help
exit=1
error: cannot read --help: No such file or directory (os error 2)

$ sigil -h
exit=1
error: cannot read -h: No such file or directory (os error 2)

$ sigil help
exit=1
error: cannot read help: No such file or directory (os error 2)
```

There is no help handler; the token falls through to the positional-argument slot and is
opened as a source file. Three consequences.

First, the message is indistinguishable from a genuine missing input, which is checklist 6:
`sigil --help` and `sigil typo.asm` print the same sentence in the same shape.

Second, the only usage text a person can reach without already knowing a subcommand name is
the bare-invocation one:

```
$ sigil
exit=2
usage: sigil <input.asm> [-o <output.bin>] [--hex]
       sigil --version
```

That block names neither `parse`, nor `emp`, nor `test`, nor `build`. Five of the six entry
points, including the one that builds a ROM, are undiscoverable from the tool itself.

Third, the same fall-through makes every mistyped flag on the bare form a filename:

```
$ sigil --nonsense
exit=1
error: cannot read --nonsense: No such file or directory (os error 2)
```

Cost: at least one cycle to discover the subcommands exist, and they can only be discovered
outside the tool.

Worth saying plainly, because it bears on the fix: the `build` subcommand's usage block is
the best surface in the product, and it is what the others should look like. It lists every
flag, adds two `note:` lines, and documents an environment variable. Its `--check` note goes
out of its way to enumerate what a green check does *not* prove. That is a message written by
someone who had already thought about checklist item 4, and it is reachable only by someone
who already knew to type `build`.

### F4. `error: unexpected character` names no character and no column (exercised)

Fails checklist 1, 2, 3.

```
$ sigil c08_char.asm
exit=1
c08_char.asm(5): error: unexpected character
```

The line is 91 characters wide in the real corpus this stands in for. The message does not
say which character, does not give a column, and does not quote the line. The user is told a
line contains something unacceptable and given nothing to narrow it with.

This is the weakest message found. Against a multi-file disassembly of the size the README
describes, locating the offender is manual bisection of the line, which is cycles rather than
one cycle.

### F5. AS-tier diagnostics carry no column, and repeat identically on one line (exercised)

Fails checklist 2, 6.

```
$ sigil c07_multi.asm       # dc.b $01, $02, $1FF, $04, $2FF, $06
exit=1
c07_multi.asm(4): error: operand 511 out of range -128..=255
c07_multi.asm(4): error: operand 767 out of range -128..=255

$ sigil c09_same.asm        # dc.b $1FF, $01, $1FF
exit=1
c09_same.asm(4): error: operand 511 out of range -128..=255
c09_same.asm(4): error: operand 511 out of range -128..=255
```

In the first case the differing values let a reader work back to the operand, but only after
converting 511 and 767 to `$1FF` and `$2FF` by hand. In the second the two messages are byte
for byte identical and the line has two candidates. Nothing distinguishes them.

The `file(line)` shape is a deliberate AS-compatibility choice and the reasoning is written
into the source. That reasoning is sound as far as it goes, and it is not what makes this a
finding. What makes it a finding is that the emp front end on the same binary emits
`file:line:col`, so the column is available and is being dropped on one side only. The
compatibility argument explains the format; it does not require discarding a coordinate the
tool already computes.

Cost: one cycle per ambiguous line, and a repeat-value line can cost more than one.

### F6. Feeding a file to the wrong front end produces no pointer to the right one (exercised)

Fails checklist 3.

The most likely single slip in this population is running a `.emp` through the bare form or
an `.asm` through `emp`.

```
$ sigil ok3.emp
exit=1
ok3.emp(1): error: no processor declared: this assembly unit never says which processor it
    is for, [...]
ok3.emp(1): error: unknown directive or mnemonic `demo.ok3`
ok3.emp(3): error: unterminated `{…}` macro attribute
ok3.emp(4): error: unexpected character
ok3.emp(5): error: unexpected character

$ sigil emp ok1.asm
exit=1
ok1.asm:1:3: file must start with a `module` declaration
ok1.asm:1:3: expected a declaration, found Ident("cpu")
```

Five diagnostics in the first case, none of which is the true one. `unterminated {…} macro
attribute` for a section body brace and `unknown directive or mnemonic demo.ok3` for a module
declaration are confident, specific and wrong. The tool has the file extension, has an `emp`
subcommand, and mentions neither.

Cost: at least one cycle in each direction, and the first case can cost more, because the
diagnostics are plausible enough to be chased.

### F7. `--map` and `--deny-todo` are accepted and honoured but appear in no usage line (exercised)

Fails checklist 2, and is the inverse of checklist 7: not a flag nothing honours, but a
honoured flag nothing names.

```
$ sigil emp
exit=2
usage: sigil emp <input.emp> [--root <dir>] [--prelude <module.id>] [-o <output.bin>]
       [--hex] [-D NAME=INT]...

$ sigil emp ok3.emp --deny-todo
exit=0
built: 4 bytes

$ sigil emp ok3.emp --map m.toml
exit=2
error: --map requires --root (region placement is a multi-module concern)
```

Both are accepted. `--map` has an error message of its own, so the surface will talk about a
flag it never advertises. The README's `emp` line lists the same five as the usage line and
so does not close the gap.

`--deny-todo` is the one that costs. A flag whose purpose is to make an unfinished marker fail
the build is exactly what a porter mid-migration wants, and the only place it is written down
is the argument loop.

### F8. A wrong `--aeon` path and a wrong `--aeon` tree print the same thing (exercised)

Fails checklist 6, and partly 3.

```
$ sigil build --aeon /home/volence/sonic_hacks/.scratch/ux-seat-b/w      # exists, not an aeon tree
exit=1
error: shape `sonic4`: no game config at .../w/games/sonic4/map.toml, a game homes its
[defines] rows there, so a missing or renamed map is a MISSING config, not an empty one,
and no shape may walk with the built-in rows alone. [...]

$ sigil build --aeon /home/volence/sonic_hacks/.scratch/ux-seat-b/nope   # does not exist at all
exit=1
error: shape `sonic4`: no game config at .../nope/games/sonic4/map.toml, [...]
```

Identical text for two different causes. The full path is printed, so a careful reader can
spot a typo, but the body of the message spends its length arguing that a missing map is a
missing config rather than an empty one, which steers the reader toward hunting for
`map.toml` and away from the possibility that `--aeon` itself is wrong. A `--aeon` path that
does not exist is cheap to detect and is the likelier of the two mistakes.

### F9. `sigil parse` writes its diagnostics to stdout; every sibling writes to stderr (exercised)

Fails checklist 5, with an act-cost consequence.

```
$ sigil parse ok2.emp
exit=1
--- stdout ---
ok2.emp:5:1: expected a declaration, found Ident("in")
--- stderr ---

$ sigil emp ok2.emp
exit=1
--- stdout ---
--- stderr ---
ok2.emp:5:1: expected a declaration, found Ident("in")

$ sigil test ok2.emp
exit=1
--- stdout ---
test result: FAILED. 0 passed; 0 failed; 1 module(s) failed to parse
--- stderr ---
ok2.emp:5:1: expected a declaration, found Ident("in")
```

The same diagnostic, from the same file, on three sibling subcommands, on two different
streams. Anyone who redirects `2>` to capture errors from `parse` captures an empty file and
a bare exit 1.

### F10. The best message in the tool ends with advice for a different audience (exercised)

Fails checklist 3.

```
$ sigil empty.asm
exit=1
empty.asm(1): error: no processor declared: this assembly unit never says which processor
it is for, and sigil will not choose one for it. Declare it on its own line at the top of
the root source, before any code, `cpu 68000` for a 68000 program, `cpu z80` for a Z80 one.
An `include`d file needs no line of its own: the declaration is the unit's, and the root's
covers it. A caller driving this front-end directly declares it by setting
`Options::initial_cpu` instead.
```

The first three sentences are the model of what this checklist asks for: what happened, what
to write, where to write it, and a pre-empted follow-up question about includes. The last
sentence is Rust API guidance addressed to someone embedding the library, printed to every
person running the command. A porter reads `Options::initial_cpu` as a thing they might be
able to set and goes looking for a flag that does not exist.

Cost: under a cycle, but it is the one sentence in an otherwise exemplary message that sends
the reader outward.

### F11. Internal token spellings reach the user (exercised)

Fails checklist 1.

```
$ sigil emp ok1.asm
ok1.asm:1:3: expected a declaration, found Ident("cpu")

$ sigil parse ok2.emp
ok2.emp:5:1: expected a declaration, found Ident("in")
```

`Ident("cpu")` is the compiler's internal debug spelling of a token, not a description of
what is in the file. The reader has to know that `Ident(...)` means identifier before the
message says anything. The second instance is the worse one: `in` is a real keyword in this
language, and being told the parser `found Ident("in")` where a declaration was expected
gives no hint that the problem is that a section body must be attached to its section header.

### F12. Two spellings of one idea, and hex source rendered as decimal (exercised)

Fails checklist 5.

```
$ sigil c04_range.asm         # moveq #$1234,d0  /  dc.b $1FF
exit=1
c04_range.asm(4): error: unsupported form: moveq data 4660 does not fit in a signed byte
c04_range.asm(5): error: operand 511 out of range -128..=255
```

Two adjacent lines, one concept (a value too large for its slot), two vocabularies: `does not
fit in a signed byte` against `out of range -128..=255`, under a leading `unsupported form:`
on one of them that suggests an encoding problem rather than a magnitude problem. Neither
message names the directive or instruction the operand belongs to.

Separately, both render in decimal what the source wrote in hex. In ROM work every constant
in sight is hex, and `4660` requires conversion before it can be matched to `$1234` on the
line the message points at.

### F13. The `env:` line names one of the two environment variables that change a build (unexercised)

Fails checklist 2. Labelled unexercised: the `SIGIL_CONTRACTS` paths need a real Aeon tree,
which this seat may not touch, so the messages below were read in source and never triggered.

The `build` usage block ends `env: SIGIL_WARNINGS=off|summary|full`. `SIGIL_CONTRACTS=0` also
changes what a build does: it skips the contract closure gate. It is not on the `env:` line.

The mitigation is real and worth recording: the gate's own failure message names the variable
at the moment a person would need it, and the skip path prints `warning: contract closure
gate SKIPPED (SIGIL_CONTRACTS=0)`, which is the honest shape. So the cost is low. It is listed
because a line labelled `env:` reads as the environment section, and a reader who takes it as
complete has an incomplete picture of what can alter their build.

### F14. `SIGIL_WARNINGS` silently accepts a misspelled value (unexercised)

Fails checklist 4 and 7, same class as F1. Labelled unexercised for the same reason as F13:
reaching the warning tiers requires a build against an Aeon tree.

The source states that an unset or misspelled value reads as the default. A person who writes
`SIGIL_WARNINGS=ful` and sees a summary rather than a list has been told nothing, and will
read the short output as "there was little to report".

I am reporting this at low confidence and low severity precisely because I could not trigger
it. It is here because it is the third instance of one pattern (F1, F2, F14: a named knob
whose unrecognized value is silently replaced by a default), and the pattern is the finding
even where an individual instance is unconfirmed.

---

## 3. Look and taste, not findings

Kept separate because none of these costs anyone information.

- `built: N bytes` is printed even when no `-o` was given and nothing was written. I checked
  this against checklist item 4 deliberately and cleared it: the word is `built`, not
  `wrote`, and the byte count was correct in every case I could verify by counting `--hex`
  output (6 of 6). A reader could take it for a file having appeared. I would prefer
  `assembled` with no `-o`, but this is preference and I am not booking it.
- `sigil parse ok3.emp` reports `1 items`.
- The bare AS form prints nothing at all on success, while `emp` prints `built: N bytes`.
  Silence on success is a defensible convention; the inconsistency between two sibling forms
  is the part I noticed.
- Error-tier AS diagnostics carry no bracketed code, while AS warnings do (`[as.warning]`)
  and emp diagnostics do (`[proc.undeclared-fallthrough]`). Codes are what makes a class
  greppable and suppressible. I nearly booked this as a checklist 5 finding and did not,
  because no information is lost from any single message; it is a capability the error tier
  does not have rather than a message that misleads.
- The `--version` output runs to about fifty lines of prose. Its content is unusually
  careful, it is the only surface in the tool that states what its own evidence does not
  prove, and I would not cut it. It is a lot of screen for a version string.
- The README documents the workspace and the discipline well and is the wrong shape to serve
  as the tool's help, which is the job F3 leaves it holding.

---

## 4. Rig conditions, not findings

- The Bash tool in this worktree-isolated agent refuses compound shell commands. Every probe
  was run from a script file under `/home/volence/sonic_hacks/.scratch/ux-seat-b/`. No effect
  on any result.
- Built with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/sigil/.target-uxb`, never the shared
  default, so no other lane's pinned binary was relinked.
- `git grep` and `/usr/bin/grep` by absolute path throughout; the shell's `grep -r` function
  and the `eza` alias on `ls` were avoided.
- My first scanner under-reported the message population by 40 percent (238 distinct literals
  against the true 474) because it did not match macro-form helper calls. Caught by a canary
  before any count was published. This is a defect in my instrument, not in the product, and
  the corrected figures are the ones used above.
- `sigil build` could not be exercised against a real Aeon tree, by the brief's own
  prohibition. This is scope, not a product problem, and it is the dominant blind spot named
  in section 1d.

---

## 5. Clean sweeps: what passed, with counts

Reported so this cannot be mistaken for a report that only looked for failures. 58 invocations
across all 6 entry points.

| Check | Result |
|---|---|
| Usage line printed on a bare or argument-less invocation | 5 of 5 entry points that take arguments |
| Exit-code discipline: 2 for usage misuse, 1 for source or IO failure, 0 for success | held in 58 of 58 invocations; no case of exit 0 over an error |
| Source diagnostics located to a file and line | 11 of 11 AS-tier, 7 of 7 emp-tier |
| emp diagnostics carry a column | 7 of 7 |
| Multiple errors collected rather than stopping at the first | 3 of 3 files tested (2, 2 and 5 diagnostics respectively) |
| IO failures name the path and the operating system's reason | 5 of 5 (directory as input, write into a missing directory, write onto a directory, missing include, missing file) |
| A value-taking flag refuses a missing value with exit 2 | 2 of 2 |
| `-D` malformed input gives an exact and actionable message | 2 of 2 |
| `build` argument misuse refused with exit 2 and a specific message | 5 of 5 (no `--aeon`, unknown flag, unknown `--game`, unknown `--report`, mutually exclusive pair) |
| A source-author `warning` directive reaches the user, leaves exit 0, and still writes the output file | 2 of 2, and the file was confirmed on disk |
| `built: N bytes` matched the actual byte count | 6 of 6 verifiable by counting `--hex` output |
| No success summary asserted anything the run did not do | 0 defects found in this class among summaries reachable without an Aeon tree |
| `--version` reports the pin under test | matches `git rev-parse HEAD` exactly |

The last two rows are the ones I most expected to break and did not. The reachable success
summaries are honest. The truthfulness defects that exist here (F1, F2, and probably F14) are
not in the summaries; they are in silent acceptance upstream of them, where a knob is
misunderstood and the summary that follows is then perfectly accurate about a program the
person did not write.

---

## 6. On the test itself

The amended test is the right one, and the amendment is what makes it work.

Applied to my own set: under the original wording I would have had to throw away F1 and F2,
the two most severe findings here. Neither costs the person anything at the moment of acting.
Both runs exit 0 and produce a ROM image. What is lost in both is only the knowledge that the
tool did not do what the source asked, which is precisely the half the base test cannot see.
F14 would have gone too. So the amendment moved three of fourteen findings, including the top
two, from discarded to booked.

Would I have invented a different test? Before the amendment, yes. I would have reached for
something like "would the person's next action be wrong?", which is close to the amended
version and catches F1 and F2 for the same reason, but it is worse in one respect: it makes
severity depend on predicting a person's next move, which reintroduces the imagined-user
problem the brief rightly rejects. The amended test asks a question about the message
(is information withheld, is false information supplied) rather than about a hypothetical
reader, and that is a better place to put the burden. I would not now propose an alternative.

One observation offered as data rather than as a challenge. The amended test bins by what is
lost, and my strongest findings cluster in a shape it captures but does not name: **a named
knob whose unrecognized value is silently replaced by a default.** F1 (`cpu:`), F2
(`--prelude`), and F14 (`SIGIL_WARNINGS`) are one defect appearing three times, and the
population that produces it is enumerable in a way individual findings are not: every place
this tool parses a name the user chose. The test correctly books each instance; what it does
not do is say that finding the third instance should send you looking for the fourth. That is
not a flaw in the test, which is a per-message rule and should stay one. It is an argument
that a heuristic seat should report classes alongside instances, which is why section 2 states
the class explicitly under F14 rather than leaving three findings to look unrelated.

Nothing in the brief was wrong. Two things were load-bearing in ways worth confirming back:
naming `--help` in bounds produced F3, which is the finding a task-walking seat would most
likely have routed around by reading the README instead; and the instruction to derive the
population rather than accept a list produced F7 and the `test`-subcommand omission, both of
which are exactly the failure the charter was guarding against, reproduced inside this tool's
own documentation.
