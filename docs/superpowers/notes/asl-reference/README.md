# The reference assembler is a BUILD, and the banner cannot name it

2026-09-05.

Four `asl` binaries in this workspace print

```
Macro Assembler 1.42 Beta [Bld 212]
```

**verbatim**, and they are not the same program. A runner, a note or a test
comment that cites the version string has therefore identified nothing. This
directory holds the guard that identifies the instrument by digest instead, and
the selfcheck that proves the guard refuses.

| path | md5 | fork | second banner line |
|---|---|---|---|
| `s1disasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | upstream (Arnold, MPC821 additions) | `(x86_64-unknown-linux)` |
| `skdisasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | same binary | `(x86_64-unknown-linux)` |
| `sonic_hack/tools/as/asl` | `61e672562465725a8c102288a7da9098` | same binary | `(x86_64-unknown-linux)` |
| `s2disasm/build_tools/Linux-x86_64/asl` | `0dee1f98e6480a4783d27ffd8b90896f` | flamewing | `(x86_64-Linux)` |
| `s1disasm/build_tools/Linux-x86/asl` | `a8cd8b80b765686b2e9266c31ffa6987` | upstream, genuine i386 | `(x86_64-unknown-linux)` |
| `skdisasm/build_tools/Linux-x86/asl` | `a8cd8b80b765686b2e9266c31ffa6987` | same binary | — |
| `s2disasm/build_tools/Linux-x86/asl` | `aa6de52f266cef0a7f60a748919ab1d3` | flamewing, **ELF 64-bit in the 32-bit slot** | `(x86_64-Linux)` |

`61e67256…` is the **reference build**. Every other digest is **refused**.

**⚠ THE COUNT IN THIS SECTION WAS WRONG TWICE, AND THE SECOND TIME WAS THE
CORRECTION.** This file said *"four binaries"*; a counting note then corrected that
to *"four PATHS and TWO PROGRAMS"* — and that correction was itself a count of what
someone happened to check. **Measured 2026-09-05 by running every `asl` on the
machine and reading `file` on each: SEVEN paths execute here, under FOUR distinct
digests**, all printing `Macro Assembler 1.42 Beta [Bld 212]` verbatim. The table
above is now the population rather than a sample.

Two things that only the enumeration shows. `s2disasm/build_tools/**Linux-x86**/asl`
is an **ELF 64-bit x86-64 executable sitting in the 32-bit slot** — so a runner that
picked a build by *architecture directory* would get a program neither its path nor
its banner describes. And the reference digest is reached by **three** paths, so
"which repo's copy" was never the question; the digest always was.

*(The lesson this file already teaches, arriving at the file itself: a population
you did not enumerate is a sample, and correcting a count without enumerating
produces another sample wearing a correction's clothes.)*

## THE RULE

> **A probe cites the assembler's MD5, never its version banner.**

That applies to a runner, a note's ground-truth stanza, a test's module doc, a
golden's provenance line, and a commit message. Naming the *path* is not enough
either: a path is an argument on some runners and a checkout on others, and
neither is pinned by being written down. If you are stating where a number came
from, the digest is the sentence that does it — `md5 61e672562465725a8c102288a7da9098`
— and the path is context beside it, not a substitute for it.

Two corollaries, both learned the expensive way:

- **A stable value is not an answer.** Both builds substitute *something* for an
  operand they declined to value. The varying build substitutes uninitialized
  memory, so it is caught by re-running. The reference build substitutes THE
  LAST VALUE IT COMPUTED, so it agrees with itself on a re-run and reads like a
  measurement. See *What the varying build does*, below.
- **The instrument is not identified until it is identified at the point of
  use.** A directory README that names the digest does not identify the binary a
  runner two levels down was handed at runtime. Runners print the md5 of what
  they ran; guarded runners refuse anything else.

## What the varying build does

For any operand it declined to give a value it substitutes an **uninitialized
memory read**, so it answers differently on every run. Measured on
`../2026-09-04-as-end-probes/wrange.asm`, whose four range-refused immediates
come back, on four consecutive runs:

```
303C 5602    303C 55B1    303C 5655    303C 557F
```

against `303C 8000` on every run of the reference build. `wimm.asm` behaves the
same way (`5570` / `5632` / `555B` / `561E` against a steady `0000`).

> **NEITHER OF THOSE TWO REFERENCE-BUILD FIGURES IS AN ANSWER** *(measured
> 2026-09-05; the varying build's four draws above are real and reproduce, the
> contrast this paragraph drew from them does not)*.
>
> `wrange.asm`'s `8000` is **line 5 leaking downward**. Line 5 is
> `move.w #-32768,d0`, which is in range, is ACCEPTED, and legitimately computes
> `$8000`; lines 6-9 are the refused ones, and they echo it. Probe `wcarry.asm`
> beside `wrange.asm` is that file with line 5's accepted value changed to
> `$1234` and the four refused lines untouched — and all four then read
> `303C 1234`:
>
> ```text
>   wrange.asm                        wcarry.asm  (line 5 changed, 6-9 identical)
>    5/  4 : 303C 8000  #-32768        5/  4 : 303C 1234  #$1234
>    6/  8 : 303C 8000  #-32769        6/  8 : 303C 1234  #-32769
>    7/  C : 303C 8000  #65536         7/  C : 303C 1234  #65536
>    8/ 10 : 303C 8000  #-65536        8/ 10 : 303C 1234  #-65536
>    9/ 14 : 303C 8000  #$FFFFF700     9/ 14 : 303C 1234  #$FFFFF700
> ```
>
> `wimm.asm`'s `0000` is the other half of the same mechanism: nothing is
> accepted above its refused lines, so the slot still holds its initial state.
> `wcarry0.asm` is the minimal form — two refused immediates and nothing else —
> and reads `0000` for the same reason.
>
> So the contrast is not *real answer* versus *garbage*. It is
> **stale-and-stable** versus **fresh-and-random**, and the stable one is the
> one that gets frozen into a table. See *And what the REFERENCE build does with
> the same operand*, below.

**The regime is wider than it looks and the loud case is not the dangerous one.**
`move.w #zz,d0` with `zz` undefined is unstable and exits 2 with a diagnostic;
`#f(<register>)` — a `function` call in an immediate whose argument is a
register name — is unstable with **exit 0 and no diagnostic at all**, which is
the shape a tool will accept and freeze. Under `setarch -R` the value collapses
to a constant `$5555`, which is what identifies the mechanism as an
uninitialized read rather than a hash or a counter. `../2026-09-05-asl-nondeterminism-sweep-probes/`
carries the characterisation and the sweeps.

### And what the REFERENCE build does with the same operand

**It substitutes too. It just substitutes the same thing every time.** Measured
2026-09-05 on `../2026-09-05-disp-or-call-probes/d9.asm` — three declined
`#f(<register>)` immediates, each preceded by a successful `#f(5)` call holding
`$0111`, `$0222`, `$0333`:

```text
reference 61e67256              varying 0dee1f98
 303C 0111   #one(a1)            303C 5630
 303C 0222   #two(a1)            303C 5630
 303C 0333   #three(a1)          303C 5630
```

Each declined operand on the reference build **echoes the last value that build
computed**, so what it returns depends on the lines above it. `d8.asm`, whose
declined lines come before any successful call, reads `0000` in all four cells —
that zero is the initial state of the slot, not a policy. Exit 0 and no
diagnostic on both builds.

**Two different refusal paths, one mechanism.** `d9` is the function-call path
(`#f(<register>)`, exit 0, no diagnostic). `../2026-09-04-as-end-probes/wcarry.asm`
is the RANGE path (`move.w #65536,d0`, exit 2, a loud `range overflow`), and it
behaves identically: it is `wrange.asm` with line 5's accepted value changed
from `$8000` to `$1234`, and its four refused lines follow from `303C 8000` to
`303C 1234`. `wcarry0.asm` is the same with nothing accepted above, reading
`0000`. A silent refusal and a loud one leak the same way, so this is a property
of how a declined operand is filled in, not of one diagnostic class.

This is why the rule above says a stable value is not an answer. The varying
build's defect announces itself the second time you run it. The reference
build's does not, which makes it the one that gets frozen into a golden or a
note's table. The digest tells you WHICH build answered; it does not tell you
that the build answered at all. For that, ask whether the shape is one asl
declines — and if it is, the number is an artifact either way.

### Sweeping for it, which the banner sweep cannot do

The version-string sweep below asks *was the instrument identified*. It cannot
find these sites: `asl_ref.sh`, this file, `../2026-09-04-as-end-probes/README.md`
and `crates/sigil-frontend-as/tests/as_word_immediate_range.rs` all cited the
digest correctly and still printed a reference-build artifact as an answer. The
second parameter is:

> **Does this text quote a byte column for a line asl REFUSED?**

and the population is derivable rather than guessable. The probes that come back
UNSTABLE when `../2026-09-05-asl-nondeterminism-sweep-probes/sweep_probes.sh` is
pointed at the varying build ARE the declined-operand set — 22 of them as of
2026-09-05 — so any committed text quoting a value for one of those probes'
refused lines is a candidate. Run the control, take its unstable list, grep the
tree for those probe names and for substitute-shaped words next to a refusal.

## The guard

`asl_ref.sh` is sourced, not run:

```sh
. "$(dirname "$0")/../asl-reference/asl_ref.sh" || exit $?
asl_run -xx -n -q -A -L -U -i . probe.asm
```

The `|| exit $?` is load-bearing — these runners do not set `-e`, so a sourced
guard that merely returns non-zero stops nothing. It exports `$ASLDIR`, `$ASL`
and `AS_MSGPATH`, defines `asl_run`, and refuses (exit 3) any binary whose md5 is
not the literal in the file. `ASLDIR` may be overridden, and pointing it at
another build is how the refusal is demonstrated; there is no escape hatch,
because a runner that can be talked into the varying build has the defect back.

The expected digest is **written out**, never computed from the binary being
checked: a guard whose expectation derives from its subject moves with the
subject and can never come out red.

## The run: `asl_run`

**The md5 says WHICH PROGRAM ran. The exit status says WHETHER ITS ANSWERS MEAN
ANYTHING.** The digest check answers only the first, and always did; its own
header says so. `asl_run` answers the second: it runs the pinned binary, writes
`ASL_EXIT=<n>` to stderr whether or not the caller thought to look, refuses out
loud on a non-zero status, and **returns that status**, so `|| exit $?` works the
same way it does for the digest.

**Why the gap is not academic**, measured on `partial_failure.asm` beside the
guard. That file has one invalid line, `bra.s /` (in AS, `/` is a nameless label
*definition* and not a reference). asl reports the one error, exits 2, and prints
a full byte column for every other line. One of those other lines is wrong
because of it: the macro's `beq.s +` assembles to

| | `beq.s +` |
|---|---|
| with the bad line present | `67FE`, a branch to **itself** |
| with the bad line deleted | `6702`, the correct forward branch |

The listing looks complete. The corrupted value is plausible, in range, and the
right shape, and nothing announces it. A reader who pinned the digest perfectly
and quoted `67FE` would have carried a fabricated number while obeying every rule
then written down. **A run carrying any error is not a source of values for the
lines that did assemble.**

### What `asl_run` still cannot tell you

**A zero exit is not sufficient either.** For an operand the reference build
*declines* to value, it substitutes the last value it computed, exits 0, and
prints no diagnostic: that is the `303C 8000` finding above, where four
range-refused immediates echo an accepted line five rows up. Digest plus exit
status answer *which program ran* and *did the run as a whole fail*. **They do
not answer "did the build answer this line"**, and nothing here does. For a shape
asl declines, the byte column is an artifact on a clean exit too. Do not pin it.

Two further limits, stated because an unstated one reads as covered:

* **It cannot make itself get used.** A script that sources the guard and then
  calls `"$ASL"` directly gets none of this, and nothing reddens. `"$ASL"` still
  works and is the **unblessed** path; see *Why there is no adoption lint* below.
* **A caller who redirects stderr loses the banner.** The return status survives
  that, which is why the status and not the banner is the load-bearing half.

### Why there is no adoption lint

A lint requiring every guard-sourcing script to use `asl_run` was considered and
rejected. Six of the thirty shell callers deliberately run a **second, non-pinned
build** as a cross-check (`run.sh <asl-dir>`), and several treat a non-zero exit
as the *answer* rather than a failure (`z80_byte_sweep.sh` counts lines asl
declines to assemble). A lint would fire on all of those, which is the
always-red shape: it trains people to weaken the check. The alternative, an
allowlist of the historical probe runners, is a hand-maintained population, and
editing it to go green is indistinguishable from hiding a defect. **So adoption
here is documentation and migration, not enforcement, and that is a real cost of
this design rather than an oversight.**

## Proof it fires

`./selfcheck.sh` — exit 0 means every case held.

```
case 0  both builds print the SAME banner, so a version check cannot discriminate
case 1  the reference build is ACCEPTED
case 2  THE VARYING BUILD IS REFUSED                      ← the load-bearing case
case 3  the refusal names both the wanted and the seen digest
case 4  a missing binary is refused, not silently skipped
case 5  with the comparison STUBBED TO ACCEPT, case 2 goes RED
case 6  asl_run ACCEPTS a clean assembly, with no banner
case 7  the fixture is a PARTIAL failure: exit 2, full byte column, 67FE vs 6702
case 8  asl_run REFUSES the partial failure               ← the load-bearing case
case 9  with asl_run's STATUS PROPAGATION stubbed, case 8 goes RED
```

Cases 0 to 5 ask which program ran; cases 6 to 9 ask whether its answers mean
anything.

Case 5 is the selfcheck's own honesty check, and it verifies the stub APPLIED
before drawing a conclusion from it — a stub that fails to apply runs the
original guard and produces the same "refused" answer as a working one. Case 9
is the same check for `asl_run`, and it stubs a **different** line on purpose:
case 5 disables the digest comparison, case 9 disables the exit check, and a
stub of one says nothing about the other.

Case 7 exists because **the easy case is a file that does not assemble** and
nobody quotes a listing that is not there. It measures the dangerous property
directly rather than assuming it: it assembles the same fixture with the one bad
line deleted and requires the `beq.s` value to *move*. If it ever stops moving,
case 7 fails and says the fixture is no longer the shape this covers. Case 6
fences the other side, because a check that fires on correct input is the shape
people weaken.

## Which runners were pinned OFF the varying build

These six had defaulted to `0dee1f98…`; the full current population is the table
further down, under *The runner population*.

| runner | before | now |
|---|---|---|
| `../2026-09-04-as-end-probes/run.sh` | `0dee1f98…` | guard |
| `../2026-09-04-as-warning-exitm-probes/run.sh` | `0dee1f98…` | guard |
| `../2026-09-04-as-warning-exitm-probes/img.sh` | `0dee1f98…` | guard |
| `../2026-09-05-as-interp-radix-probes/run.sh` | `0dee1f98…` | guard |
| `../../probes/2026-09-03-align/gen_org.sh` | `0dee1f98…` | guard |
| `../../probes/2026-09-03-align/run.sh` | `0dee1f98…` by default | guard by default; `[asl-dir]` still reaches a second build and prints its md5 |

**Some runners name a second build and keep it, deliberately** — seven of them,
not the two this section first claimed; they are marked *unguarded by design* in
the population table below, and every one prints the md5 of every binary it
runs. **A guard on those would delete the capability, not protect it** — the
defect was never a second build, it was an unidentified instrument.

## What repinning cost

All 71 probes in the four repinned corpora were assembled under BOTH builds and
their emitted code lines compared. **Exactly two differ** — `wrange.asm` and
`wimm.asm`, the only two that carry operands asl refuses. Every other committed
answer in those four directories is identical under either build, so the tables
carry over without being re-taken.

The one committed table repinning invalidates is the asl listing pasted into
`crates/sigil-frontend-as/tests/as_word_immediate_range.rs`'s module doc, whose
four `303C 55F5` words are one draw of uninitialized memory frozen into a
comment. The supersession is written beside it there, and the listing is kept
rather than swapped so it is visible instead of silent. That file's assertions
are unaffected: they assert acceptance and refusal, and the `range overflow`
diagnostics are identical under both builds.

## The runner population, derived rather than listed

Closed 2026-09-05. The list this section used to carry — "roughly a dozen" — was
**wrong in both directions**, so it is replaced by the enumeration that produced
the current state. Three parameters over tracked files, reconciled: a grep for
`build_tools/Linux-x86_64|tools/as/asl`, a grep for `ASLDIR|ASL_BIN|AS_MSGPATH|$ASL`,
and a walk of all 75 tracked `.sh` classifying how each obtains an assembler.
The second and third find runners the first does not, which is the whole reason
for using more than one.

**Every tracked runner that invokes an `asl`, and its disposition:**

| runner | disposition |
|---|---|
| `../2026-09-03-as-struct-probes/run.sh`, `cmp.sh` | guard |
| `../2026-09-03-irp-irpc-probes/diff_bytes.sh` | guard |
| `../2026-09-04-as-end-probes/run.sh` | guard |
| `../2026-09-04-as-enum-probes/run.sh` | guard |
| `../2026-09-04-as-symbol-class-probes/run.sh` | guard |
| `../2026-09-04-as-warning-exitm-probes/run.sh`, `img.sh` | guard |
| `../2026-09-05-as-include-repeat-probes/run.sh`, `depth.sh`, `siblings.sh` | guard |
| `../2026-09-05-as-interp-radix-probes/run.sh` | guard |
| `../2026-09-05-as-macro-body-label-probes/run.sh` | guard |
| `../../../../scripts/z80_byte_sweep.sh` | guard, pointed at the corpus dir it is handed |
| `../../../../.f1probe/cmp.sh` | guard |
| `../../../../.s1probe/2026-09-04/probe/cmp.sh` | guard |
| `../../probes/2026-09-03-align/gen_org.sh` | guard |
| `../../probes/2026-09-03-align/run.sh` | guard by default; `[asl-dir]` reaches a second build and prints its md5 |
| `../2026-09-03-tilde-tilde-probes/run.sh` | `ref` by default; an explicit path reaches a second build and prints its md5 |
| `../2026-09-03-tilde-tilde-probes/diff_bytes.sh` | guard by default; `ASL_CROSSCHECK_DIR` likewise |
| `../2026-09-03-isa-missing-encodings/asl_probe.sh` | guard by default; an explicit dir likewise, md5 on stderr so it stays out of the TSV |
| `../2026-09-05-disp-or-call-probes/run.sh` | guard by default; `[asl-dir]` likewise |
| `../2026-09-05-disp-or-call-probes/both.sh` | **unguarded by design** — both builds, md5 per block |
| `../2026-09-05-asl-nondeterminism-sweep-probes/aslr.sh` | **unguarded by design** — demonstrates the instability |
| `../2026-09-05-asl-nondeterminism-sweep-probes/characterise.sh` | **unguarded by design** — walks three builds, md5 each |
| `../2026-09-05-asl-nondeterminism-sweep-probes/sweep_probes.sh` | **unguarded by design** — the build is the subject |
| `../2026-09-05-asl-nondeterminism-sweep-probes/sweep_isa_vectors.sh` | as above |
| `../2026-09-05-asl-nondeterminism-sweep-probes/sweep_snippets_golden.sh` | as above |
| `../../probes/2026-09-03-align/gen_org_both.sh` | **unguarded by design** — asks both builds |
| `./selfcheck.sh` | **unguarded by construction** — it must reach the varying build to demonstrate the refusal |

Every runner in the unguarded group prints the md5 of every binary it runs;
that was verified, not inherited. A guard on any of them would delete the
capability rather than protect it — the defect was never a second build, it was
an unidentified instrument.

**Not runners, and CLOSED 2026-09-05** (`../2026-09-05-asl-gen-vector-provenance.md`):
`gen_m68k_vectors`, `gen_z80_vectors` and `gen_snippet_vectors` took their
assembler from `ASL_BIN` with **no default and no digest check**, and the vector
files they mint carried no provenance line. They are deliberately out-of-repo
since the P4d flip, so a digest pin would contradict that decision — the fix is
therefore **stamp, not constrain**: each generator now derives the md5 and banner
of the `asl` and `p2bin` it was handed and writes them into a header on the file,
and **refuses (exit 4, nothing written) any toolchain it cannot identify**.
`ASL_BIN` still names any build; the choice is simply no longer unrecorded.

The demonstration is in the artifact. Minting `z80_golden_vectors.txt` under the
varying build changes **no vector line at all** — all 120 are identical — and
changes only:

```diff
-# asl-md5       61e672562465725a8c102288a7da9098
+# asl-md5       0dee1f98e6480a4783d27ffd8b90896f
 # asl-banner    Macro Assembler 1.42 Beta [Bld 212]     <- unchanged
-# asl-banner    (x86_64-unknown-linux)
+# asl-banner    (x86_64-Linux)
```

Before the header that mint was a git-clean no-op, indistinguishable from a
reference-build mint. The banner's *first* line is identical across the two, so
this file's own rule is what the artifact now records.

## The version-string sweep

Of 114 tracked files that cite an asl version string, **72 carry no digest
anywhere in the file**. Most are harmless: `asl-declock/fixtures/*.lst` are
asl's own listing headers rather than citations, and the 2026-07 plans and specs
point at `aeon/tools/asl`, a binary deleted at the P4d flip — unreproducible,
but not misidentified.

**The sites whose claims are actually at risk** are five notes that state their
ground truth as *AS V1.42 Beta Bld 212, `s2disasm/build_tools/Linux-x86_64/asl`*
— the varying build, cited by banner — and have no committed probe directory, so
nothing reproduces their rows:

| note | quoted byte rows | quoted diagnostics |
|---|---|---|
| `../2026-09-03-as-shift-macro-argument-walk.md` | 19 | 4 |
| `../2026-09-03-as-maclocal-scope.md` | 25 | 5 |
| `../2026-09-03-as-macrosetup-three-sites.md` | 23 | 2 |
| `../2026-09-03-as-intlabel-capture.md` | 23 | 2 |
| `../2026-09-03-as-name-composition-braces.md` | 8 | 0 |

Same defect and same week as `../2026-09-05-disp-or-call.md`, which has since
been re-measured and amended. The exposure is bounded by what the two builds
actually disagree about — only operands asl declined to value — so the first
three, which discuss undefined and refused shapes most, are the ones to re-take
first, and `as-name-composition-braces` is the least exposed. Booked, not
re-measured here.

`crates/sigil-frontend-as/src/operands.rs:519` says `f(a1)` "has no stable
meaning to match (AS V1.42 …)". That is a version-string citation for a claim
`../2026-09-05-disp-or-call-probes/d9.asm` has since sharpened: the meaning is
stable on the reference build and still not a meaning. Worth a touch at the next
edit of that file.

`docs/OVERSEER.md` cites `s2disasm/build_tools/Linux-x86_64/asl` without a
digest and is the overseer's file to edit, not a parcel's.
