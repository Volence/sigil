# The md5 says WHICH program ran. The exit status says WHETHER ITS ANSWERS MEAN ANYTHING.

2026-09-05. Parcel `ASL-GUARD-EXIT-STATUS`, branch `parcel/asl-guard-exit-status`.

`docs/superpowers/notes/asl-reference/asl_ref.sh` selected the reference `asl` by
md5 and refused any other build. It asked nothing about whether a **run**
succeeded, and its own header already conceded the gap in those words:

> THIS GUARD PINS WHICH BUILD ANSWERED; it cannot tell you the build answered at
> all.

This closes what that sentence names.

## The finding, measured rather than argued

`docs/superpowers/notes/asl-reference/partial_failure.asm` (new, committed beside
the guard) has **one** invalid line, `bra.s /`, where `/` in AS is a nameless
label *definition* and not a reference, so the branch has no target. Everything
else in the file is valid. asl reports the one error, **exits 2**, and prints a
**full byte column for every other line**. One of those other lines is wrong
because of it:

| | `beq.s +` inside the macro |
|---|---|
| with the bad line present | `67FE`, a branch to **itself** |
| with the bad line deleted | `6702`, the correct forward branch over the `nop` |

Both measured on the reference build, md5 `61e672562465725a8c102288a7da9098`.
The listing looks complete. The corrupted value is plausible, in range, and the
right shape, and **nothing announces it**. A session that had pinned the md5
perfectly, read that listing, and quoted `67FE` would have carried a fabricated
number while obeying every rule then written down.

The listing footer does say `Additional necessary passes not started due to
errors, listing possibly incorrect`, which is the same tell
`2026-09-05-asl-silent-decline-regime-probes/run.sh` already prints on. It is at
the very bottom, after four pages of symbol table, and it is not what a reader
scrolling to a byte column sees.

## The design, and what it costs

**Chosen: `asl_run`, a shell function the guard defines after the digest check
passes.** It runs the pinned binary, writes `ASL_EXIT=<n>` to stderr whether or
not the caller thought to look, refuses out loud on a non-zero status naming that
status, and **returns it**, so a caller's `|| exit $?` works exactly as it does
for the digest. It exists only when the digest check passed, so its presence is
itself a statement that the program was identified. The md5 half is untouched;
this is additive.

### What was rejected, and why

**Repointing `$ASL` at a generated wrapper script.** This is the only option with
automatic adoption across all 30 shell callers, and it fails on measurement, not
on taste. `git grep -n 'md5sum "$ASL"'` returns **eight occurrences across seven
callers** besides the guard itself, and two of those **compare the result to the
reference literal** (`2026-09-05-as-jmptos-518-block-probes/run.sh` line 37,
`2026-09-05-s2-top-blocks-decompose-probes/run.sh` line 62): they would digest
the wrapper and refuse the assembler. Several more derive `AS_MSGPATH`
from `$(dirname "$ASL")` and would lose the message files. And it would not even
be complete, because a majority of runners call `"$ASLDIR/asl"` directly and
never touch `$ASL`. Rejected with evidence.

**An adoption lint** requiring every guard-sourcing script to use `asl_run`.
Rejected, and this is the honest cost of the chosen design rather than an
oversight. Six callers carry a **second, non-pinned build** as a cross-check
(`git grep -ln 'CROSS-CHECK BUILD'`: the two `2026-09-03-tilde-tilde-probes`
scripts, `2026-09-03-isa-missing-encodings/asl_probe.sh`,
`2026-09-05-asl-silent-decline-regime-probes/run.sh`,
`2026-09-05-disp-or-call-probes/run.sh`, `probes/2026-09-03-align/run.sh`; and
`README.md`'s own table separately lists a set as "unguarded by design"), and
several treat a non-zero exit as **the answer**
rather than a failure: `scripts/z80_byte_sweep.sh` counts the Z80 lines the
reference build declines to assemble. A lint would fire on all of those. That is
the always-red shape: it fires on correct code, and the damage is written into
the remediation advice people then follow. The alternative, an allowlist of the
thirty historical runners, is a hand-maintained population, and editing it to go
green is indistinguishable from hiding a defect.

**So the cost is stated plainly: adoption here is documentation and migration,
not enforcement.** A future script that sources the guard and calls `"$ASL"`
directly gets none of this, and nothing reddens. That is written into the guard
header, the README, and `docs/OVERSEER.md`.

## Call sites: the count and what was done about them

`git grep -l asl_ref.sh` returns **36 files**: 30 shell scripts, plus
`docs/OVERSEER.md`, `docs/lane-log.jsonl`, two probe `README.md`s, and the guard
and selfcheck themselves. The brief's 36 is correct.

**Three migrated:**

| file | why |
|---|---|
| `docs/superpowers/notes/2026-09-05-as-macro-body-label-probes/run.sh` | printed `ASL_EXIT=2` and then dumped the listing anyway |
| `docs/superpowers/notes/2026-09-05-as-logical-precedence-probes/run.sh` | same shape, a copy of the above |
| `scripts/z80_byte_sweep.sh` | the one live repo tool outside `docs/` |

The first two are the point. **The exit status was already in the transcript at
those sites.** They printed it and then printed the listing with nothing between,
which is exactly how a reader arrives at `67FE`. What was missing was not a
report, it was a **refusal**. After migration `digest.sh` output opens with the
refusal block before any byte column.

**One concrete breakage found and avoided.**
`2026-09-05-as-macro-body-label-probes/digest.sh` line 12 has an awk rule
`/^ASL_EXIT=/ { print; next }` reading run.sh's **stdout**. `asl_run` reports on
stderr, so a clean migration would have killed that rule **silently**, which is
this parcel's own defect class one level down. Both migrated runners therefore
keep `echo "ASL_EXIT=$?"` on stdout, with a comment saying why. Verified by
running `digest.sh` after the change: the rule still fires.

**Not migrated, for a structural reason.** The six cross-check runners enumerated
above take an optional second `asl-dir` argument and, on that path, **do not
source the guard at all** because the whole point is to run a non-pinned build.
`asl_run` does not and must not exist there. Migrating only
their reference branch would mean two different invocations in one file, or a
duplicated fallback wrapper without the guard behind it, in six places.

**The rest are historical probe runners** under dated `docs/superpowers/notes/`
directories, records of measurements already taken. They were left alone, and the
`README.md` now says the raw `"$ASL"` path is unblessed rather than pretending
they were swept.

## The selfcheck case, and its red-first proof

`docs/superpowers/notes/asl-reference/selfcheck.sh` gains four cases. Cases 0 to
5 ask which program ran; 6 to 9 ask whether its answers mean anything.

```
case 6  asl_run ACCEPTS a clean assembly, with no banner
case 7  the fixture is a PARTIAL failure: exit 2, full byte column, 67FE vs 6702
case 8  asl_run REFUSES the partial failure               <- the load-bearing case
case 9  with asl_run's STATUS PROPAGATION stubbed, case 8 goes RED
```

**Case 7 constructs the real failure, not a synthetic one.** A file that fails to
assemble at all is the easy case; nobody quotes a listing that is not there. Case
7 measures the dangerous property directly instead of assuming it: it requires a
non-zero exit, requires `4E75` (the `rts` after the error) to be present so there
IS a byte column past the error, requires `beq.s` to read `67FE`, and assembles
the same fixture with the one bad line deleted and requires the value to **move**
to `6702`. If it ever stops moving, case 7 fails and says the fixture is no
longer the shape this covers.

**Case 6 is the not-always-red side.** A check that fires on correct input trains
people to weaken it, so `asl_run` is required to accept a clean assembly silently.

**Case 9 stubs a different line from case 5 on purpose.** Case 5 disables the
digest comparison; case 9 disables the exit check. A stub of one says nothing
about the other.

### Red-first proof, two mutations, both shown applied on disk

`git checkout <rev> -- <path>` **stages**, so `git diff --stat` reports nothing on
an applied mutation. Both proofs below use `git diff HEAD --stat` plus a content
grep.

**Mutation A: the guard as it stood on master** (`git checkout a0cc6997 --
.../asl_ref.sh`), new selfcheck unchanged.

```
git diff HEAD --stat  ->  asl_ref.sh | 74 ------------------
grep -c asl_run asl_ref.sh  ->  0   (exit 1: the function really is gone)

case 6  FAIL  asl_run returned 127 ... asl_run: command not found
case 7  PASS
case 8  FAIL  asl_run returned 127 but printed no refusal
case 9  FAIL  the stub did not apply
pass=7 fail=3      SELFCHECK_EXIT=1
```

Case 7 passing there is correct and deliberate: it measures the fixture through
the raw `$ASL` path, which the change does not touch.

**Mutation B: `asl_run` present but not propagating** (`return "$asl_rc"` ->
`return 0`), which is the defect direction that matters.

```
git diff HEAD --stat  ->  asl_ref.sh | 2 +-
git diff HEAD         ->  -    return "$asl_rc"
                          +    return 0

case 8  FAIL  ASL_RUN RETURNED 0 ON A FAILED ASSEMBLY: the exit check is decoration
pass=9 fail=1      SELFCHECK_EXIT=1
```

Case 8's primary branch is reachable and fires with the intended message, not
merely via the missing banner.

**And mutation B caught a defect in case 9's own honesty check.** As first
written, case 9 verified its stub applied by asking `grep -q '^    return 0$'`
of the stub's OUTPUT, which is also true of a guard that **already** said
`return 0`. Under mutation B it therefore passed vacuously. It now asks both
halves: the guard carries `return "$asl_rc"` before, and the stub does not carry
it after. Re-run under mutation B, case 9 reports *"the guard has no 'return
$asl_rc' line to stub"* and fails. Restored: `pass=10 fail=0`, exit 0.

## The honest limit

**A zero exit is not sufficient either, and this is not a caveat, it is a
prohibition.** For an operand the reference build **declines** to value, it
substitutes **the last value it computed**, exits 0, and prints no diagnostic at
all. That is the `303C 8000` finding the guard's header already carries: in
`2026-09-04-as-end-probes/wrange.asm` four range-refused immediates echo an
accepted `move.w #-32768,d0` five rows above them, and changing that accepted
line moves all four.

So the digest and the exit status together answer two questions:

* **which program ran** (digest), and
* **did the run as a whole fail** (status).

**They do not answer "did the build answer THIS line", and nothing in
`asl-reference/` does.** For a shape asl declines, the byte column is an artifact
on a clean exit too. Do not read this parcel as "now the numbers are safe"; the
honest claim is narrower than that. Stated in the guard header, in
`asl-reference/README.md` under *What `asl_run` still cannot tell you*, and in
`docs/OVERSEER.md`.

Two further limits, stated because an unstated limit reads as covered:

* **`asl_run` cannot make itself get used** (see *what was rejected*, above).
* **A caller who redirects stderr loses the banner.** `z80_byte_sweep.sh` sends
  it to a per-line temp log; `digest.sh` captures it. The **return status**
  survives redirection, which is why the status and not the banner is the
  load-bearing half.
* **Nothing in the landing suite runs `selfcheck.sh`.** It is operator-invoked,
  the same as `asl-declock/selfcheck.sh` beside it. This parcel adds no Rust
  test, so the landing baseline should be **unchanged at 4650**, and any other
  delta is a finding. Closing that would mean the suite depending on
  `s1disasm`/`s2disasm` trees outside this repo; not done here, and named rather
  than left implied.

## Landing run

Recorded below in the commit that carries it. Failures first, with names.

## Anything in this brief you concluded was wrong

Three things.

**1. "A change that breaks them is not acceptable" was aimed at the wrong
risk.** The brief framed the 36 call sites as a thing a migration might break by
touching them. The breakage I actually found was the opposite shape: it was
caused by touching a runner **correctly**, in the way the brief recommended, and
the damage landed in a *third* file that parses that runner's stdout. Moving
`ASL_EXIT=` from stdout to stderr in
`2026-09-05-as-macro-body-label-probes/run.sh` silently kills an awk rule in
`digest.sh`, which never appears in `git grep -l asl_ref.sh` at all because it
does not name the guard. **The population to check before migrating a runner is
its CONSUMERS, not the guard's callers.** The count that mattered here was 3
(`all.sh`, `digest.sh`, `stability.sh`), and it is not derivable from the 36.

**2. "The natural home is a wrapper the callers invoke instead of `$ASL`
directly" understates how much was already there.** All four recent probe runners
I read already print `ASL_EXIT=$?`. The exit status was in the transcript at the
site of today's incident. What failed was not measurement, it was that the datum
sat one line above a full byte column with nothing marking it as
disqualifying. So the wrapper's value is the **refusal**, not the report, and a
version of this parcel that had only added exit-status *reporting* would have
shipped something those sites already had, and would have read as a fix.

**3. My own first cut of case 9 could not fail in the state it was meant to
detect.** Written as "the stub output contains `return 0`", it passed against a
guard that already said `return 0`, which is exactly the defect it exists to
catch. It survived a green run and was only exposed by running mutation B, and
the brief's own note about `git checkout` staging is why I was looking at disk
state closely enough to notice. Fixed, and the fix is proven by the same
mutation.
