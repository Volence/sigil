# UX seat A: the task walk

Pin: sigil `b8594cf9d25210a50596a4daaaa7580862806c1b`
Branch: `ux-seat-a-task-walk`
Seat: A (task walk). Subject: the product surface (diagnostics, `--help`, README, command-line shape).
Method constraint: attempt every job using only the README and `--help`. No source reading, no test reading.

**Population walked:** someone porting an existing disassembly onto this assembler.
**Cost unit:** a build cycle (one more run of the assembler).

**Finding test applied** (as amended mid-walk by the controller): does the person lose information
they need to act, OR to know they are being misled?

## Provenance of the binary under test

Built with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/sigil/.target-uxa cargo build --release --bin sigil`,
finished in 19.57s. The binary was built from this branch, whose single commit touches only
`docs/superpowers/notes/`, so no compiled code differs from the pin. Proven with the tool's own
drift instrument rather than asserted:

```
$ git show --stat --oneline HEAD | tail -3
9df4fc80 ux seat A: the five day-one jobs, written before the repo was opened
 .../notes/2026-09-09-ux-seat-a-task-walk.md        | 34 ++++++++++++++++++++++
 1 file changed, 34 insertions(+)

$ git log -1 --format=%H HEAD -- <the closure paths sigil --version printed>
d724d9c717cf19d9a4ccff9df9bdba8edd72cb13
$ git log -1 --format=%H b8594cf9d25210a50596a4daaaa7580862806c1b -- Cargo.lock Cargo.toml crates
d724d9c717cf19d9a4ccff9df9bdba8edd72cb13
```

The binary's self-reported `closure-revision` is `d724d9c7`, equal to both. The compiled code under
test is the pin's.

## The five jobs, written down before anything in the repo was opened

Written and committed (`9df4fc80`) before the README, `--help`, or any file in the tree was read. I
am a programmer who knows assembly and command-line tools and has never used this assembler.

1. **Find out how to drive it at all.** What is the invocation shape? Is there a subcommand? What is
   required, what is optional, where does output go, what is the default? I want to answer this
   without guessing and without reading source.

2. **Assemble one minimal 68000 source file into a binary and confirm the bytes.** The smallest
   possible end to end run: a file with two or three instructions in, a binary out, and me able to
   check that the bytes are the opcodes I expected.

3. **Break the file on purpose and see whether the error is actionable.** An undefined label, a
   misspelled mnemonic, an operand that does not fit. I want the file, the line, ideally the column,
   the offending text, and enough of a statement of the rule that I can fix it without a manual.

4. **Assemble an existing AS-dialect source file unchanged.** I have a codebase written for the old
   assembler. Can I point this tool at it as is? How do I select the compatibility dialect, and what
   does it do when it meets something it does not support?

5. **Write a small file in the modern `.emp` dialect from documentation alone, and get a symbol map
   or listing out of the build.** Two halves of the same day-one need: learn the new syntax without
   reading the compiler, and get back something I can debug the output with.

---

# The walk, job by job

Every command below was actually run and every output block is what was actually printed.

## Job 1: find out how to drive it. FINISHED, with the first finding on the first keystroke.

The first thing I typed was the universal one.

```
$ sigil --help
error: cannot read --help: No such file or directory (os error 2)
exit=1
```

`--help` is consumed as an input filename. So are the other two conventional forms:

```
$ sigil -h
error: cannot read -h: No such file or directory (os error 2)
exit=1
$ sigil help
error: cannot read help: No such file or directory (os error 2)
exit=1
```

A bare invocation is the only help surface that exists:

```
$ sigil
usage: sigil <input.asm> [-o <output.bin>] [--hex]
       sigil --version
exit=2
```

Two lines. `--version`, by contrast, prints roughly fifty lines of provenance prose (closure paths,
drift-check commands, a paragraph on zsh word-splitting). The tool has a great deal to say about
which revision it is and almost nothing to say about how to run it.

Then the README, which documents a command line the tool never mentions:

```
sigil <input.asm> [-o <out.bin>] [--hex]
sigil parse <input.emp>
sigil emp   <input.emp> [--root <dir>] [-o <out.bin>] [--hex]
sigil test  <input.emp> [--root <dir>]
sigil build --aeon <dir> [--game sonic4|demo] [--debug] [-o <out.bin>]
```

Four subcommands exist. The usage line names none of them. Each has good usage text of its own once
you know to type it:

```
$ sigil emp
usage: sigil emp <input.emp> [--root <dir>] [--prelude <module.id>] [-o <output.bin>] [--hex] [-D NAME=INT]...
$ sigil build
error: --aeon <dir> is required
usage: sigil build --aeon <dir> [-o <out.bin>] [--emit-lst <lst>] [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean] [--report ram|contracts|indirect-cost] [--extra-entry <module|path.emp>]... [--check]
note:  --extra-entry evaluates the NAMED module's comptime guards; the named module must emit nothing (its own imports are not checked)
note:  --check decides every ensure and LinkAssert against final post-relaxation placement and writes no ROM; a green check proves nothing about region budget or overlap, image bounds, the checksum or the closure gate, and is not a statement that the game builds
env:   SIGIL_WARNINGS=off|summary|full  (warn-tier detail; default summary)
```

That `--check` note is one of the best things I read all walk: it states what a green result does
not prove. Note that `--prelude`, `-D`, `--emit-lst`, `--config-a|--config-b|--lean`,
`--extra-entry`, `--check` and `SIGIL_WARNINGS` appear here and nowhere in the README. The two
documentation surfaces are each a strict subset of the union.

Job 1 finished: **3 build cycles**, all three spent on `--help`, `-h`, `help`.

## Job 2: assemble a minimal 68000 file. FINISHED in 4 build cycles.

```
$ cat hello.asm
    move.w  #$1234,d0
    nop
    rts
$ sigil hello.asm
hello.asm(2): error: no processor declared: this assembly unit never says which processor it is for, and sigil will not choose one for it. Declare it on its own line at the top of the root source, before any code, `cpu 68000` for a 68000 program, `cpu z80` for a Z80 one. An `include`d file needs no line of its own: the declaration is the unit's, and the root's covers it. A caller driving this front-end directly declares it by setting `Options::initial_cpu` instead.
hello.asm(1): error: unknown directive or mnemonic `move.w`
exit=1
```

This diagnostic is excellent and I want that on the record before the findings start. It names the
problem, names the fix, gives both spellings, and pre-empts the follow-up question about include
files. It is the single best message in the product. Two things about it are still wrong: it is
printed **second in the file but first in the output**, out of line order, and the last sentence
tells a command-line user to set `Options::initial_cpu`, which is not a thing they can do.

Added the `cpu` line and ran again:

```
$ sigil hello.asm
exit=0
```

Nothing printed. Exit 0. No file anywhere. I checked carefully:

```
$ find . -type f | sort > before.list
$ sigil hello.asm > stdout.txt 2> stderr.txt
exit=0
--- stdout bytes: 0 ---
--- stderr bytes: 0 ---
--- new files anywhere under scratch ---
./before.list
./stderr.txt
./stdout.txt
```

The only new files are my own capture files. The run succeeded and produced nothing. It had in fact
assembled correctly, which the next two cycles showed:

```
$ sigil hello.asm --hex
30 3C 12 34 4E 71 4E 75
$ sigil hello.asm -o out.bin
$ xxd out.bin
00000000: 303c 1234 4e71 4e75                      0<.4NqNu
```

`303C 1234` is `move.w #$1234,d0`, `4E71` is `nop`, `4E75` is `rts`. The encodings are right. Note
that the `-o` run also prints nothing, so the output of a run that wrote my file and the output of
a run that discarded it are byte-identical: both empty, both exit 0.

Job 2 finished. **4 build cycles**, of which 2 were spent discovering that output is not written by
default.

## Job 3: break the file on purpose. FINISHED, mixed result.

Four single-error cases, all recognised, all with file and line:

```
$ sigil e1.asm -o /dev/null
e1.asm(4): error: unresolved symbol `Nowhere` for fixup in section sec0 at offset 5
$ sigil e2.asm -o /dev/null
e2.asm(3): error: `mvoe` is not a recognized 68000 mnemonic
$ sigil e3.asm -o /dev/null
e3.asm(2): error: unsupported form: moveq data 4660 does not fit in a signed byte
$ sigil e4.asm -o /dev/null
e4.asm(2): error: illegal destination EA: #imm
```

All four are actionable. e3 in particular states the rule, which is what I asked for.

Then an error inside a macro body, invoked twice:

```
$ cat m1.asm
    cpu 68000
myop macro src,dst
    moveq   #src,\dst
    endm

Start:
    myop    $12,d0
    myop    $1234,d1
    rts
$ sigil m1.asm -o /dev/null
m1.asm(3): error: unexpected character
m1.asm(3): error: unexpected character
```

Two identical lines. Nothing says these are two expansions from lines 7 and 8, nothing names the
character, no column, no echo of the text. I isolated the message to check whether it ever names
what it found:

```
$ # operand = [\]
    moveq \,d0
u.asm(3): error: unexpected character
$ # operand = [?]
    moveq ?,d0
u.asm(3): error: unexpected character
$ # operand = [@@@]
    moveq @@@,d0
u.asm(3): error: unexpected character
$ # operand = [{]
    moveq {,d0
u.asm(3): error: unterminated `{…}` macro attribute
$ # operand = [#$1234 ; ok]
    moveq #$1234 ; ok,d0
u.asm(3): error: operand count: Moveq expects 2 operands, got 1
```

Three different offending characters, one message, none of them named. The `{` case shows the tool
can be specific when it wants to be, which makes `unexpected character` a catch-all that names
nothing rather than a limitation of the machinery.

Errors in an included file carry the right file and line:

```
$ sigil i1.asm -o /dev/null
subdir/inc.asm(1): error: unsupported form: moveq data 4660 does not fit in a signed byte
```

Correct file, correct line, no include stack. Enough to act on.

Multi-error ordering, seven deliberate errors on lines 2 through 8:

```
$ sigil many.asm -o /dev/null
many.asm(2): error: `mvoe` is not a recognized 68000 mnemonic
many.asm(3): error: unsupported form: moveq data 4660 does not fit in a signed byte
many.asm(4): error: `badop` is not a recognized 68000 mnemonic
many.asm(6): error: unsupported form: moveq data 22136 does not fit in a signed byte
many.asm(8): error: `zzz` is not a recognized 68000 mnemonic
```

Five errors, in line order. The two `bra.s Nowhere1` / `Nowhere2` unresolved symbols on lines 5 and
7 are absent. I fixed exactly the five that were reported and ran again:

```
$ sigil many2.asm -o /dev/null
many2.asm(5): error: unresolved symbol `Nowhere1` for fixup in section sec0 at offset 9
many2.asm(7): error: unresolved symbol `Nowhere2` for fixup in section sec0 at offset 13
```

The two withheld errors appear. Nothing in the first run said the list was partial.

Cascade behaviour, ten instruction lines and no `cpu`:

```
$ sigil casc.asm -o /dev/null
casc.asm(1): error: no processor declared: ...
casc.asm(1): error: unknown directive or mnemonic `move.w`
casc.asm(2): error: unknown directive or mnemonic `move.w`
... (one per line, through line 10)
```

N+1 messages, no cap, no count, no dedup, no summary line anywhere.

Job 3 finished: **11 build cycles**. Four of six error classes were fully actionable.

## Job 4: assemble an existing AS-dialect disassembly unchanged. NOT FINISHED. Feature gap.

I copied `s2disasm` into my own scratch (91M) rather than touching the shared checkout, and pointed
the tool at it as a porting user would.

```
$ sigil s2.asm -o s2.bin
exit=1
```

5227 diagnostic lines. The error classes, whole population, not a head sample:

```
   2624 bad operand expression
   2309 expected mnemonic, directive, or label
     89 `X` is not a recognized N mnemonic
     49 bad word expression
     30 bad byte expression
     24 unresolved symbol `X` in operand
     17 int(): could not evaluate float expression
     17 division by zero: N / N
      8 instruction needs an explicit size suffix (.b/.w/.l)
      6 case needs a string literal
      4 malformed number (hex needs a trailing `X`)
      3 bad displacement expression in `X`
      2 unresolved if condition ...
      2 switch needs a string expression
      1 struct `X` has a member line this cannot read; its size and every member after it would be wrong
     10 cannot include sound/.../generated/*.inc: No such file or directory
```

4933 of 5227, or 94.4%, are the top two messages. I sampled the head **and the tail** of each rather
than the head alone, and printed the source line behind each site:

```
bad operand expression, first three sites:
  s2.asm:402  [	dbf	d6,-]
  s2.asm:489  [	beq.s	-]
  s2.asm:494  [	beq.s	+			; if not, branch]
bad operand expression, last three sites:
  s2.asm:90823 [	dbf	d3,-]
  s2.asm:90834 [	dbf	d3,-]
  s2.asm:90846 [	bne.s	+]

expected mnemonic, directive, or label, first two sites:
  s2.asm:401  [-	move.l	d7,(a6)+]
  s2.asm:487  [-	move.w	(VDP_control_port).l,d0]
```

Head and tail agree: both dominant classes are one AS feature, nameless temporary symbols (`+` and
`-` as a label and as a branch target). Neither message names the construct, says it is
unsupported, or distinguishes a gap in the compatibility front end from a defect in my source. The
s2disasm README itself flags this feature as one you must understand before reading the code, so it
is not an exotic corner.

I stopped here rather than reaching for the source, per the seat rule. From the README and `--help`
alone there is no statement anywhere of which AS features the compatibility front end supports. The
README's fidelity claim is scoped to "the exact bytes AS produced for the Aeon sources", which is a
narrower claim than the usage line's `<input.asm>` invites, but nothing tells a porting reader where
the boundary is.

Job 4 not finished. **2 build cycles** to reach a wall I could not see past from the documented
surface.

## Job 5: write `.emp` from the docs, and get a listing. BOTH HALVES BLOCKED.

Authoring half. The README mentions `.emp` eleven times and contains zero lines of `.emp` code:

```
$ grep -n -E '^\s*(fn|module|import|let|const|section)\b' README.md
48:module.
$ grep -c '```' README.md      # canary: the instrument does find fenced blocks
8
$ grep -c '\.emp' README.md
11
```

The only match is the prose word "module." in a sentence. The language reference the README points
at, `SIGIL_SPEC2_LANGUAGE.md`, is in a sibling repository (`/home/volence/sonic_hacks/empyrean`),
reachable on this machine but not part of a sigil checkout, and the README gives a
`git -C ../empyrean show` recipe rather than a URL.

The repo does ship eleven `.emp` files under `examples/`, and the README never mentions the
directory:

```
$ grep -n -i 'example' README.md
(no output)
$ grep -c -i 'sigil' README.md      # canary: the instrument works on this file
48
```

So I did what a newcomer does and guessed:

```
$ cat g1.emp
fn main() {
    moveq #1, d0
    rts
}
$ sigil emp g1.emp --hex
g1.emp:1:1: file must start with a `module` declaration
g1.emp:1:1: expected a declaration, found Ident("fn")
```

The first message is genuinely useful. It is also the end of the road: I now know I need a `module`
declaration and there is nothing in the product that tells me what one looks like. Declaring the
authoring half impossible from README and `--help`, and stopping.

Listing half. `--emit-lst` exists only on `build`:

```
$ sigil emp examples/main.emp --root examples --emit-lst x.lst
error: unexpected argument '--emit-lst'
$ sigil hello.asm --emit-lst y.lst
error: unexpected argument '--emit-lst'
```

Clean refusals, but no listing or symbol output exists on either single-file path. A listing
requires `build --aeon <dir>`, that is, an Aeon game tree. A person assembling their own source
cannot get one.

While here I ran the shipped examples, which is where the strongest finding of the walk came from.

```
$ sigil emp examples/guards.emp --hex
00 06 00 07 00 08 00 01 02 10 20 30 40
built: 13 bytes
exit=0
```

Then in a clean directory containing nothing but the source:

```
$ find . -type f | sort
./guards.emp
$ sigil emp guards.emp
built: 13 bytes
exit=0
$ find . -type f | sort
./guards.emp
```

`built: 13 bytes`, exit 0, no file. And with `-o`:

```
$ sigil emp examples/guards.emp -o g.bin
built: 13 bytes
exit=0
-rw-r--r-- 1 volence volence 13 g.bin
```

Identical output. The run that wrote the file and the run that did not print the same line.

The example set as a whole does not build:

```
$ sigil emp examples/main.emp --root examples --hex
examples/dispatch.emp:33:1: warning: [module.path-mismatch] module `m` ends in `m` but its file is `dispatch.emp`, ...
examples/guards.emp:20:1: error: module `m` declared twice (also at module #1)
examples/offset_table.emp:39:1: error: module `m` declared twice (also at module #6)
exit=1
```

That may not be the intended invocation. Nothing documents an intended invocation, which is the
point.

Job 5 not finished, both halves. **8 build cycles**.

---

# Findings, most severe first

## F1. `sigil emp <file>` with no `-o` prints `built: N bytes`, exits 0, and writes nothing. VERIFIED.

The message is byte-identical to the one printed by the run that does write the file. "built" plus
a byte count is an unambiguous assertion that an artifact exists.

Evidence: three runs above, including one in a directory containing only the source, where `find`
before and after shows no new file. Confirmed the string is the tool's, not the source's:
`grep -n 'built' examples/guards.emp` returns nothing while the canary `grep -c 'module'` returns 2.

Cost: a build rule that omits `-o` reports success forever and the downstream step consumes a stale
or absent ROM. Unbounded build cycles until someone thinks to check a file timestamp, because
nothing in the output ever changes.

This is the mirror case the brief named, and it is caught only by the amended half of the test.

## F2. `sigil <file>.asm` with no `-o` assembles successfully, prints nothing, exits 0, and writes nothing. VERIFIED.

The AS-path twin of F1, and worse in one respect: here silence is the entire output, and silence is
also exactly what a successful `-o` run prints. The same observation means "your file is written"
and "your assembly was discarded". The usage line's `[-o <output.bin>]` is in square brackets, which
in every command-line tool means optional-with-a-default; there is no default.

Evidence: the `find`-before-and-after run under Job 2, zero bytes on both streams, and the `--hex`
run on the same file proving the assembly did succeed and the result was thrown away.

Cost: 2 of my 4 job-2 cycles. For the port population, the same trap as F1 with no message at all.

## F3. There is no `--help`, and the only help omits every subcommand. VERIFIED.

`--help`, `-h` and `help` are each consumed as input filenames and produce a
`No such file or directory` error, which points the person at a missing file rather than at an
unrecognised option. The bare-invocation usage covers `<input.asm>`, `-o`, `--hex` and `--version`
and names none of `emp`, `parse`, `test`, `build`. Without the README, the entire `.emp` language
track, the whole reason the tool exists beyond replacing `asl`, is undiscoverable from the binary.

Evidence: the four transcripts in Job 1.

Cost: 3 cycles to establish that no help exists, then a hard dependency on having the README to
learn that four subcommands do.

## F4. 94.4% of a real port's errors are one unsupported feature, reported by two messages that name nothing. VERIFIED.

`bad operand expression` (2624) and `expected mnemonic, directive, or label` (2309) are, at head and
at tail, AS nameless temporary symbols. Neither message names `+` or `-`, says the construct is
unsupported, quotes the source, or gives a column. A porting reader cannot tell "this assembler does
not implement this yet" from "your expression is malformed", and their first inference from
"bad operand expression" is that their own valid source is wrong. No supported-subset statement
exists in the README or in any `--help` output.

Evidence: the full 5227-line class histogram and the head-and-tail source samples under Job 4.

Cost: not build cycles. This is the finding that costs hours, because the person spends them
looking for a defect in source that is correct.

## F5. The error list is silently partial across phase boundaries. VERIFIED.

Seven errors in one file, five reported, two withheld until all five were fixed, with no count, no
"further checks not run" line, and no indication the list was a subset. The output is well ordered
and looks complete, which is what makes it misleading: it invites exactly the fix-everything-then-
rebuild loop that it then defeats.

Evidence: `many.asm` and `many2.asm` transcripts under Job 3.

Cost: one guaranteed extra build cycle per phase boundary, and on a real port the whole
unresolved-symbol class is invisible until every parse error is gone.

## F6. `unexpected character` never names the character. VERIFIED.

Identical message for `\`, `?` and `@@@`. No column, no caret, no echo. The adjacent
`unterminated {…} macro attribute` message shows the machinery can be specific.

Evidence: the five-case isolation table under Job 3.

Cost: bisecting one line by hand, realistically 3 to 6 build cycles per occurrence.

## F7. Macro-body errors carry no expansion site. VERIFIED.

One bad macro invoked twice prints two byte-identical lines pointing at the body. Nothing names
lines 7 and 8 as the call sites and nothing distinguishes the two messages, so they read as one
message printed twice, which reads as a defect in the tool.

Evidence: the `m1.asm` transcript.

Cost: for a macro-heavy disassembly, which is all of them, the person cannot tell how many call
sites are affected without editing and re-running.

## F8. On a failed run, source `message` output goes to stdout while every diagnostic goes to stderr. VERIFIED, with an important correction to my own first reading.

```
$ sigil s2.asm -o s2.bin > s2.out 2> s2.err
exit=1
$ cat s2.out
ROM size is $F9198 bytes (996.3984375 KiB). About $B543 bytes are padding.
$ wc -l s2.out s2.err
1 s2.out
5227 s2.err
$ ls -l s2.bin
ls: cannot access 's2.bin': No such file or directory
```

`sigil s2.asm -o rom.bin > build.log` yields a build log whose entire content is a plausible ROM
size, for a run that emitted 5227 errors and wrote no file.

**The correction, which I am recording because I nearly filed this as the headline.** That sentence
is not sigil's summary line. It is `s2.asm:91272`, a `message` directive in the disassembly itself:

```
$ grep -rn "ROM size is" --include='*.asm' .
./s2.asm:91272:		message "ROM size is $\{EndOfRom-StartOfRom} bytes ..."
```

I checked because the result was convenient. So the finding is narrower than it first looked: sigil
is not fabricating a summary, it is running the source's own `message` output and routing it to
stdout on a run that has already failed. Emitting messages during a pass is defensible AS-compatible
behaviour and the stream split is defensible on its own terms. What survives is the interaction: the
one stream a person captures by habit contains only the reassuring line.

Cost: one to several cycles, and only for someone who reads their log rather than their exit code.

## F9. Severity prefixes are inconsistent inside the `.emp` front end. VERIFIED.

Parse-stage diagnostics carry no severity word:

```
g1.emp:1:1: file must start with a `module` declaration
g3.emp:2:1: expected a declaration, found Ident("fn")
```

Later-stage ones do, and so do warnings, and they interleave:

```
examples/guards.emp:20:1: warning: [module.path-mismatch] ...
examples/guards.emp:20:1: error: module `m` declared twice (also at module #1)
```

In mixed output an unprefixed line cannot be classified as error or warning by reading it.

Cost: low on its own. It matters when someone greps a log for `error:` and a parse failure does not
match.

## F10. No error count, no cap, and one out-of-order case. VERIFIED.

No run at any size prints a total. One missing `cpu` line produces N+1 messages with no dedup or
suppression; on `s2.asm` the output is 5227 lines. Separately, the no-`cpu` case reports line 2
before line 1, reproducibly, when the first line needing a CPU to disambiguate is not line 1.

Cost: one cycle plus paging. Modest, but it compounds F5, since with no total there is no signal at
all that a list is truncated by phase.

## F11. Two dialects, two location formats. VERIFIED.

AS diagnostics are `file(line)`. `.emp` diagnostics are `file:line:col`. Only the `.emp` side has
columns, and only the `.emp` format is the one editors and `wc`-style tooling parse by default.

Cost: low for a human, real for anyone wiring the AS path into an editor's error list.

## F12. `SIGIL_WARNINGS` is silently ignored outside `build`. VERIFIED.

```
$ SIGIL_WARNINGS=off sigil emp examples/main.emp --root examples --hex
examples/dispatch.emp:33:1: warning: [module.path-mismatch] ...
```

The variable is documented in `build`'s usage block and reads as a global knob. Setting it elsewhere
has no effect and says nothing.

Cost: one cycle. Low, and arguably correct scoping. Filed because a silently-ignored setting is
indistinguishable from a broken one.

## F13. README and `--help` are each a strict subset of the union. VERIFIED.

The README omits `--prelude`, `-D NAME=INT`, `--emit-lst`, `--config-a|--config-b|--lean`,
`--extra-entry`, `--check` and `SIGIL_WARNINGS`. The top-level usage omits all four subcommands.
Neither surface is complete, so a person must read both and still cannot know they have everything.

## F14. The `.emp` language has no reference inside the product. VERIFIED.

Eleven mentions of `.emp` in the README and zero lines of `.emp` code. The spec is in a sibling
repository, cited by a `git -C ../empyrean show` recipe rather than a URL. Eleven `.emp` example
files ship in `examples/` and the README never mentions the directory, with the instrument
canary-checked in both directions. The one invocation I could guess for the example set fails with
`module `m` declared twice`.

Cost: this is what made half of job 5 impossible rather than merely hard.

---

# Look and taste (NOT findings)

- `Options::initial_cpu` in the `no processor declared` message is a Rust API path shown to someone
  holding a command line. Everything before it in that message is excellent.
- `expected a declaration, found Ident("fn")` shows Rust `Debug` formatting of an internal token.
- `for fixup in section sec0 at offset 5` uses linker vocabulary for a symbol the user wrote. The
  odd-valued `offset 5` can send a 68000 programmer hunting for a misaligned instruction.
- `moveq data 4660 does not fit in a signed byte` reports in decimal what the source wrote as
  `$1234`.
- `unsupported form:` is an odd prefix for what is plainly a range error.
- No "did you mean" on near-miss mnemonics. `mvoe` is one transposition from `move`.
- The asymmetry between a fifty-line `--version` and a two-line usage is the most striking thing
  about the surface, and I note it here rather than as a finding because the `--version` content is
  genuinely good; it is the imbalance that reads oddly.
- `996.3984375 KiB` and a trailing space in the s2 message are the disassembly's own text, not
  sigil's.

# Rig conditions (NOT findings)

- Ten `cannot include sound/.../generated/*.inc` errors in the s2 run are mine: I did not run
  s2disasm's `build.lua` generator step before pointing the assembler at the tree. The diagnostic for
  them was clear and named the exact missing path. Excluded from the F4 analysis.
- **My own instrument was wrong once and I nearly filed a false finding from it.** My first
  exit-code probe was `sigil <sub> 2>&1 | head -6; echo $?`, which measures `head`, not sigil, and
  reported exit 0 for usage errors. Re-run without the pipe, every usage path exits 2 consistently
  and `--help` exits 1. There is no exit-code finding.
- `grep -r` with an unquoted `--include=*.asm` fails under zsh globbing with `no matches found` and
  the surrounding `echo $?` still printed 0. Quoted the pattern and re-ran.
- This worktree's sandbox refuses commands with runtime-computed command names or compound
  constructs, so every invocation in this document uses a literal absolute path to the binary. No
  product bearing.
- The release build emits C++ `-Wmaybe-uninitialized` warnings from the vendored `clownlzss`
  sources. Pre-existing, not a product surface, not investigated.
- I copied `s2disasm` (91M) into my own scratch rather than assembling the shared checkout in place.
  No shared tree was written to. `s1disasm` was never touched.

# The clean walks: what worked

Reported because a panel that lists only failures cannot be told from one that only looked for them.

- **Bare `sigil` gives usage in one step**, and the usage it gives is accurate for what it covers.
- **Every subcommand has good usage text**, better than the top-level line: `build`'s block carries
  two explanatory notes and an env var, and the `--check` note states what a green result does not
  prove. That is a level of honesty most tools never reach.
- **The minimal 68000 job genuinely works** and the bytes are right: `303C 1234 4E71 4E75`.
- **`--hex` is a good affordance.** It is the fastest way to see what came out and it made the
  silent-discard behaviour diagnosable in one cycle.
- **Four of six deliberate error classes were fully actionable** on first read: unresolved symbol,
  unrecognised mnemonic, immediate out of range, illegal destination EA. The out-of-range message
  states the rule rather than just the violation.
- **The `no processor declared` message is the best diagnostic in the product.** It names the fix,
  gives both spellings, and answers the follow-up question about includes before it is asked.
- **Include provenance is correct**: an error in an included file reports that file and its own line
  number, not the includer's.
- **Multi-error output is in line order** in the ordinary case, 5 errors on lines 2, 3, 4, 6, 8.
- **`.emp` diagnostics carry line and column**, and `file must start with a `module` declaration` is
  precise and correct.
- **Warnings carry stable codes** (`[module.path-mismatch]`) and explain the rule and both fixes.
- **`--version` is a genuinely unusual and good artifact.** It states its own limits, names what its
  evidence cannot prove, prints a runnable drift check rather than a recipe, and explains why it
  prints the command whole. I used it to prove this binary's compiled closure equals the pin's, and
  it worked exactly as advertised.

# On the finding test

Applied as amended. **I would not have invented a different test, and the amendment changed my
binning of my own top finding**, which is the useful part of the answer. Under the original wording,
F1 and F2 both fall out: the person acts, the action appears to succeed, and nothing they needed for
the next step is withheld. I would have written them into the look-and-taste section as "no success
line, I would have phrased this differently". Under the amended wording F1 is my headline. So the
amendment is doing work on this walk, on the finding that matters most.

**One clause I would add, and my walk is the case for it.** F2 sits awkwardly in both halves.
Nothing states anything false, because nothing is stated at all, and nothing needed for the next
action is missing, because the person's next action is unaffected. What they lose is the ability to
tell two materially different outcomes apart, and they lose it to an absence rather than to an
assertion. I would extend the test with: **or to know what the run actually did**, that is, a run
whose output does not distinguish two materially different outcomes is a finding even when no
individual statement is false. That clause is what F1 and F2 both actually turn on, and stating it
directly would stop the next seat having to reason F2 in through the misleading half, where it only
half fits.

I agree with both rejected alternatives being rejected. "Would a new user be confused" would have
let me file the `--version` verbosity, which is not a defect. "Is it inconsistent with the rest of
the tool" would have promoted F11 to the top of my list, where it does not belong, and would have
demoted F2, since silence-on-success is uniform across the AS path and therefore perfectly
consistent.

# What I could not attempt, and things in the brief I judged wrong

- **Job 4 stopped at a feature wall** and I did not read source to characterise it further, per the
  seat rule. The 94.4% figure is from output, not from source.
- **Job 5's authoring half is impossible from README and `--help`**, and I stopped rather than
  reading `examples/*.emp` as a substitute grammar. Job 5's listing half is unreachable without an
  Aeon tree, verified by two refusals.
- **`build --aeon` was never exercised.** It needs an Aeon tree and the brief forbids touching one.
  Everything the README says about the flagship path is untested by this seat, including
  `--emit-lst`, `--report`, and `--check`. That is the largest gap in my coverage.
- **The brief was right that `--help` would be the highest-value target, and slightly wrong in
  assuming it exists.** Naming it explicitly was still what made F3 the third finding of the walk
  rather than an unexamined assumption.
- **The most valuable instruction I used was not in the brief.** "A convenient result is a trigger"
  is what made me check whether the `ROM size is ...` line was sigil's own before filing it as the
  headline. It was not sigil's, it was `s2.asm:91272`, and without that check F8 would have gone out
  as a fabricated-summary finding that a controller would have had to retract. I would put that
  sentence in the next version of this brief, next to the transcript requirement, because the
  transcript requirement alone would not have caught it: I had a real command and real output, and
  the wrong conclusion.
