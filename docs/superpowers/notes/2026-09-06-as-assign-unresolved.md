# An assignment whose right-hand side never resolves

Parcel `AS-ASSIGN-UNRESOLVED-BINDS-ZERO`, branch `parcel/as-assign-unresolved`, off
master `e6e942e5`.

## The corrected statement of the fault

The row's title says sigil binds zero. It does not, and the row already carries that
refutation. The row then says the divergence is between the linker path and a
single-file route. The dispatch says that is refuted too, and that the discriminator is
whether the assigned name is USED.

Measured, **both of those are half right and neither is the fault**. There were TWO
independent causes, and they have different populations:

1. **`set` / `:=` / `eval` recorded nothing anywhere.** `directive_set`'s
   `if let Some(v) = self.eval_all(rest, span)` had no `else` arm, so a right-hand side
   that would not fold left no trace at all: no value in `env`, no diagnostic, and no
   obligation carried to the link. This was silent on EVERY route, the aeon ROM build
   included. Use was not the discriminator here; there was nothing to discriminate.

2. **`equ` / `=` recorded an obligation the single-file route never read.**
   `directive_equate`'s failing branch already carried its right-hand side out as a
   deferred symbolic `equ_sym`, and `resolve_layout`'s `fold_equ_syms` already refuses
   an `equ_sym` that will not fold, WITHOUT asking whether anything reads it. So the
   ROM route and the `.emp` route already refused an unread bad `equ`. The
   `sigil <file.asm>` route was the one FINAL link in the binary that stopped at
   `link()`, and `link()` also serves PARTIAL links, so its Pass-1b steps over a
   `Fold::Poison` `equ_sym` and leaves the name undefined. Here the discriminator was
   the ROUTE, exactly as the row said.

The dispatch's "both routes behave identically" was measured at the CLI, where only one
of the two CLI routes takes a `.asm` file. At the library seam the two arms disagreed
outright; see the two-route table below.

So the true statement is: **the refusal an assignment is owed was either not recorded at
all (`set` family) or recorded and never read (`equ` family), and USE was what covered
for both.**

## What asl does, and that it was sought

Reference build `asl_ref.sh` → `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
md5 `61e672562465725a8c102288a7da9098` (the varying `s2disasm` build
`0dee1f98e6480a4783d27ffd8b90896f` is refused by the guard). Every run below via
`asl_run -xx -n -q -A -L -U -i .`, so `ASL_EXIT` and `ASL_DIAG` are in the transcript.

Each probe: `Missing` is never defined, and `Val` is **never read** (the body is `nop`).

| source line 2 | asl | exit | `ASL_DIAG` | passes |
|---|---|---|---|---|
| `Val	equ	Missing` | `p_equ.asm(2):9: error #1010: symbol undefined` | 2 | complete | 2 |
| `Val	=	Missing` | `p_eq.asm(2):7: error #1010: symbol undefined` | 2 | complete | 2 |
| `Val	set	Missing` | `p_set.asm(2):9: error #1010: symbol undefined` | 2 | complete | 2 |
| `Val	:=	Missing` | `p_coloneq.asm(2):8: error #1010: symbol undefined` | 2 | complete | 2 |
| `Val	eval	Missing` | `p_eval.asm(2):10: error #1010: symbol undefined` | 2 | complete | 2 |

`ASL_DIAG=complete` and `2 passes` on all five: the pass loop ran to the pass that
judges undefined symbols, so the refusal was **sought and found**, not left unlooked-for
behind an earlier error. That distinction is the one `swallowed_undef.asm` exists for.

The USED arm, `dc.l Val` instead of `nop`, gets **two** errors, one at the assignment
(line 2) and one at the read (line 3). asl refuses the assignment either way; the read
adds a second diagnostic, it does not create the first.

## Both routes, with the refusal text extracted

Two binaries, both built in this worktree from committed revisions:

| binary | md5 | source |
|---|---|---|
| `sigil-old` | `eddcc759d8de373ed707e3cf17b21bfb` | both changed files checked out from `e6e942e5` (master) |
| `sigil-new` | `20cde28becc410feb891ae234276539b` | `4a152fd5` (this branch's tip) |

`sigil-old` is the baseline binary: the only files this branch changes that reach the
executable are `crates/sigil-cli/src/main.rs` and `crates/sigil-frontend-as/src/eval.rs`,
and both were restored from `e6e942e5` for that build (the third changed file is a test
target and is not linked into `sigil`). The two md5s differ, so the two arms below are
two programs and not one measured twice.

Probe scripts and transcripts: `/home/volence/sonic_hacks/.parcel-as-assign-probe/`
(scratch, deliberately outside the worktree so the landing run's tree stamp reads clean;
not tracked, so it will not survive indefinitely).

### The library seam: `link()` alone vs `resolve_layout` → `link`

Measured on `Val <kw> Missing` + `Stub: dc.w 0`, value never read, at master `e6e942e5`:

| spelling | front end recorded | ROUTE A `link()` only | ROUTE B `resolve_layout` → `link` |
|---|---|---|---|
| `equ` | `Val=Sym("Missing")` | **OK, no diagnostic** | REFUSED (text below) |
| `=` | `Val=Sym("Missing")` | **OK, no diagnostic** | same refusal |
| `set` | **nothing** | **OK, no diagnostic** | **OK, no diagnostic** |
| `:=` | **nothing** | **OK, no diagnostic** | **OK, no diagnostic** |

The refusal text, verbatim, on every row that produces one:

```
<file>(2): error: unresolvable equ `Val`: its first unresolved dependency `Missing`
is not defined (an undefined symbol, or an equ cycle)
```

That table is the whole fault in one place: the `equ` rows differ by ROUTE, the `set`
rows are silent on BOTH, and the dispatch's claim that the two routes behave identically
holds only for the `set` family.

At this branch's tip every one of the four rows records `Val=Sym("Missing")` and Route B
produces the refusal.

### The shipped command, `sigil <file.asm>`

`p_*.asm` = the five spellings, unread. `p_equ_used.asm` = `equ` with `dc.l Val`.
`f_*.asm` = the forward-reference arm.

| file | `sigil-old` | `sigil-new` |
|---|---|---|
| `p_equ.asm` | `4E 71`, **exit 0** | REFUSED at `(2)`, exit 1 |
| `p_eq.asm` | `4E 71`, **exit 0** | same refusal at `(2)`, exit 1 |
| `p_set.asm` | `4E 71`, **exit 0** | same refusal at `(2)`, exit 1 |
| `p_coloneq.asm` | `4E 71`, **exit 0** | same refusal at `(2)`, exit 1 |
| `p_eval.asm` | `4E 71`, **exit 0** | same refusal at `(2)`, exit 1 |
| `p_equ_used.asm` | refused at `(3)`, `unresolved symbol Val for fixup in section sec0 at offset 0`, exit 1 | same refusal at `(2)`, exit 1 |

The last row is the point about USE: old sigil refused it, but at line 3 and for the
read. New sigil refuses at line 2, for the assignment, which is where asl refuses it.

## The forward-reference non-fire proof

This is the over-fire direction and it is the one that breaks working programs. The AS
front end iterates passes to a fixpoint, so a forward reference folds to `Expr::Int` on
a later pass and never reaches the deferral at all. Two things are asserted, not one:
that no symbolic obligation survives the front end, and that the bytes are asl's.

All five shapes on the reference build: `ASL_EXIT=0`, `ASL_DIAG=complete`, `2 passes`,
`0 errors`.

| shape | asl listing bytes | `sigil-new` | `sigil-old` |
|---|---|---|---|
| forward `equ`, read (`Val equ Later` / `dc.l Val` / `Later:` / `dc.w 1`) | `0000 0004` `0001` | `00 00 00 04 00 01` | identical |
| forward `equ`, **never read** | `4E71` `0001` | `4E 71 00 01` | identical |
| forward LABEL (`bra.w Later`) | `6000 0004` `4E71` `0001` | `60 00 00 04 4E 71 00 01` | identical |
| forward `set` | `0000 0004` `0001` | `00 00 00 04 00 01` | identical |
| `set` reassigned over a settled value (`Val set 1` then `Val set Later`) | `0000 0004` `0001` | `00 00 00 04 00 01` | identical |

asl's listing also shows the equate's own value column resolving: `=$4` for the read
case, `=$2` for the unread one. The same equate takes a different value in the two
files because the label sits at a different address, which is what a fixpoint pass loop
is for.

`a_forward_reference_is_not_an_unresolved_symbol` (inline in `eval.rs`) pins all five,
and it asserts the front end left NO symbolic `equ_sym` before it compares bytes; a
byte comparison alone would pass while an obligation quietly rode along to the link.

I could separate "not yet resolved" from "never resolves", so this is not a BLOCKED
item. The separation is not a heuristic: it is the pass loop. Anything the loop can
resolve is resolved before the module is handed to the link, and the deferral is only
ever reached by a name no pass resolved.

**Where the separation genuinely cannot be made, and why the refusal is at the link.**
This unit is one half of a mixed AS + `.emp` program. A name absent from the AS unit's
tables is indistinguishable, IN THE FRONT END, from a `.emp` label the link is about to
supply. `Game_Entry = GameState_OJZScroll_Init` is that shape and it is correct source.
So the front end records an obligation and the LINK, where the whole program is visible,
takes the refusal. Refusing in the front end would have been the asl-shaped fix and it
would have broken the cross-seam equates.

## What the corpus sweep measured

**It measured diagnostics and exit codes. It compared ZERO bytes.** Both arms of it.

Trees, all four read-only, `s2disasm` from a separate clean checkout at `e45ebf33`
(`/home/volence/sonic_hacks/s2disasm-mompass-clean`, `git status --porcelain` empty; the
owner's `/home/volence/sonic_hacks/s2disasm` was not written to):

| tree | revision | `git status --porcelain` lines |
|---|---|---|
| `s1disasm` | `f6ece65` | 4 |
| `s2disasm-mompass-clean` | `e45ebf3` "Avoid total build failure when 'deltas.bin' is missing." | 0 |
| `skdisasm` | `2fcd861` | 2 |
| `sonic_hack` | `858af72` | 17 |

Only `s2disasm` was required to be a clean detached tree at `e45ebf33`, and it is. The
other three are the workspace's own checkouts, read but never written; their dirty
counts are recorded because a sweep run against a tree in an unstated state is not
reproducible, and because both arms read the SAME bytes either way (the comparison is
old-sigil vs new-sigil over one tree, not tree vs tree).

### Arm 1: per file, every `.asm` assembled standalone

Population **1,828** files. What each arm PRODUCED:

```
OLD arm PRODUCED: accepted=0 (of which emitted bytes: 0)  refused=1828
NEW arm PRODUCED: accepted=0 (of which emitted bytes: 0)  refused=1828
DIFFER: exit-code=0  bytes=0  first-diagnostic-text=0
newly REFUSED by NEW: 0
newly ACCEPTED by NEW: 0
```

Neither arm accepted a single file. These trees are include FRAGMENTS, so a file
assembled standalone stops early: the first diagnostics are `no processor declared` and
`malformed number (hex needs a trailing 'h')`, i.e. the file never reaches a state where
this parcel's code could act. So `DIFFER=0` here means: over 1,828 files, the change
moved no exit code and no first diagnostic. **It says nothing about bytes, because no
byte was produced by either arm.**

This is worse than the dispatch's prior measurement (it said 2 of 1,753 were accepted
with zero bytes; here 0 of 1,828 are accepted). I did not chase the difference (the
tree list and the accept rule are not necessarily the same ones), and I am not treating
either number as a byte measurement.

### Arm 2: the four trees' ROOT sources, which are whole programs

Population 5: `s1disasm/sonic.asm`, `s2disasm-mompass-clean/s2.asm`,
`skdisasm/sonic3k.asm`, `skdisasm/s3.asm`, `sonic_hack/S4.asm`.

Every one, on BOTH arms, exits 1 with an identical first diagnostic
(`<tree>.macrosetup.asm(N): error: \`listing\` is not a recognized 68000 mnemonic`) and
an empty image (`d41d8cd98f00b204e9800998ecf8427e`). Verdict `identical` on all five.
**Also zero bytes**: sigil does not assemble these trees today, so the root arm is a
diagnostic comparison as well.

### The instrument was canaried

The sweep script `cd`s into each source file's own directory so an `include` resolves.
Pointed at a directory of files the two binaries are KNOWN to disagree on, the FIRST
version of it reported `DIFFER: exit-code=0` for every file, because a relative binary
path stops naming the binary after the first `cd`, and `timeout: failed to run command`
became the "diagnostic" for both arms, identically. The canary caught that; the script
now absolutizes both binaries and refuses one that is not executable. Re-canaried:

```
population: 13 .asm files
OLD arm PRODUCED: accepted=11 (bytes: 11)  refused=2
NEW arm PRODUCED: accepted=5  (bytes: 5)   refused=8
DIFFER: exit-code=6  bytes=0  first-diagnostic-text=2
newly REFUSED by NEW: 6
```

It also refuses to run over a population below a floor, so a sweep that examined nothing
cannot print the reassuring `DIFFER=0`.

**The real byte measurement is the aeon ROM**, via the landing suite's byte gates
against `/home/volence/sonic_hacks/.aeon-ref` (`aeon_rev` `483b3e12`). See the landing
section.

## Red-first, with the mutation shown applied on disk

Two mutations, each a whole-file `git checkout` from a COMMITTED baseline, each isolating
one of the two causes. `git checkout <rev> -- <path>` STAGES, so `git diff --stat`
reports nothing; every proof below uses `git diff HEAD --stat` plus a content grep.

### Mutation A: baseline `main.rs`, fixed `eval.rs`

```
$ git checkout e6e942e5 -- crates/sigil-cli/src/main.rs
$ git diff HEAD --stat
 crates/sigil-cli/src/main.rs | 28 +---------------------------
 1 file changed, 1 insertion(+), 27 deletions(-)
$ grep -n "sigil_link::link(&module.sections" crates/sigil-cli/src/main.rs
124:    let linked = match sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()) {
```

The run MUST FAIL, and specifically on the `equ` arms with the OLD message shape.

```
test an_unresolved_assignment_is_refused_whether_or_not_it_is_read ... FAILED
  read.asm: the refusal names the binder and its dependency.
  stderr: /tmp/.tmp2FH50T/read.asm(3): error: unresolved symbol `Val` for fixup in section sec0 at offset 0
test result: FAILED. 4 passed; 1 failed
```

The reported stderr is the baseline's own behaviour, at line 3 and for the read.

### Mutation B: baseline `eval.rs`, fixed `main.rs`

```
$ git checkout HEAD -- crates/sigil-cli/src/main.rs
$ git checkout e6e942e5 -- crates/sigil-frontend-as/src/eval.rs
$ git diff HEAD --stat
 crates/sigil-frontend-as/src/eval.rs | 235 ++++-------------------------------
$ grep -c defer_unresolved_assign crates/sigil-frontend-as/src/eval.rs   ->  0
$ grep -c "resolve_layout(&module.sections" crates/sigil-cli/src/main.rs ->  1
```

The run MUST FAIL, and specifically on the `set` arm and NOT on the `equ` arms: the
route fix alone catches those, and a failure on `read.asm` here would mean the two
mutations were not isolating different causes.

```
test an_unresolved_assignment_is_refused_whether_or_not_it_is_read ... FAILED
  unread_set.asm: an assignment naming an undefined symbol was ACCEPTED.
  stdout:
test result: FAILED. 4 passed; 1 failed
```

Both `equ` arms passed under B, which is the isolation the proof needed.

### Restore

```
$ git checkout HEAD -- crates/sigil-cli/src/main.rs crates/sigil-frontend-as/src/eval.rs
$ git status --porcelain   (empty)
$ grep -c defer_unresolved_assign crates/sigil-frontend-as/src/eval.rs   ->  6
$ grep -c "resolve_layout(&module.sections" crates/sigil-cli/src/main.rs ->  1
```

### A correction to this branch's own commit message

`4a152fd5`'s message says "Five tests". There are **six**: four inline in `eval.rs` and
two appended to `crates/sigil-cli/tests/cli_diagnostic_location.rs`. The landing run's
reconciliation is what caught it (`4683 baseline + 6 new = 4689 observed`), which is the
reconciliation line doing exactly the job it exists for. The message is left as written
rather than amended, and this paragraph is the correction.

### What the tests cannot red-first, stated rather than glossed

The four inline `eval.rs` tests live in the file Mutation B reverts, so under that
mutation they are ABSENT, not red. An absence is not a proof. Their red-first is carried
by the CLI-level test, which lives in `cli_diagnostic_location.rs` (untouched by either
mutation) and which fails under each mutation for the corresponding cause.

## The guard that refuses an empty population

`every_assignment_spelling_records_an_unresolved_obligation` is the population control
for `every_assignment_spelling_is_refused_unread`. A refusal assertion is green just as
easily when the front end recorded NOTHING and the link failed for some other reason,
and "recorded nothing" is precisely cause 1, so without the control the refusal test
would have been green over the bug it was written for. The control asserts exactly one
`equ_sym` named `Val`, and that its expression is still `Sym("Missing")` rather than an
`Int` (an `Int` would mean it folded and there is nothing left to refuse).

The corpus sweep carries the same requirement as a floor on its file count.

## The landing run

```
scripts/landing-run.sh --baseline 4683 --aeon /home/volence/sonic_hacks/.aeon-ref \
  --expect-test an_unresolved_assignment_is_refused_whether_or_not_it_is_read \
  --expect-test every_assignment_spelling_is_refused_unread
```

**FAILURES FIRST: 0 failed.** There are no names to list.

```
=============================== LANDING RUN VERDICT ===============================
  log             .../agent-adac9243c6d15a126/.target-land/landing-20260906T060724Z.log
  tree            /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-adac9243c6d15a126
                  @ 1b251fcb (parcel/as-assign-unresolved, clean)
  reference       /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (HEAD, clean), all four present
  target dir      .../agent-adac9243c6d15a126/.target-land
  started/ended   2026-09-06T06:07:24Z -> 2026-09-06T06:13:00Z (UTC)
  CARGO_EXIT      0
  CLIPPY_EXIT     0   (lint bar clean)
  suites          412
  passed          4689
  failed          0
  ignored         2
  skip lines      0
  reconciles      4683 baseline + 6 new = 4689 observed

  RESULT          GREEN
===================================================================================
```

`skip lines 0`, so no reference-dependent gate downgraded itself to a no-op. The run
carries an end marker, so it was not killed. `pwd` and `HEAD` are in the stamp above and
they are this worktree at this branch's tip.

**All six new tests are in the log by name**, which is the check that the green is about
this code and not about other code:

```
234:  test the_single_file_route_still_accepts_a_forward_reference ... ok
235:  test an_unresolved_assignment_is_refused_whether_or_not_it_is_read ... ok
2065: test eval::tests::a_reassigned_binder_owes_one_obligation_not_two ... ok
2082: test eval::tests::a_forward_reference_is_not_an_unresolved_symbol ... ok
2144: test eval::tests::every_assignment_spelling_records_an_unresolved_obligation ... ok
2146: test eval::tests::every_assignment_spelling_is_refused_unread ... ok
```

### Why this is the SECOND landing run, and the first one is not the record

The first run came out GREEN with the identical counts (4689 / 0 failed / 412 suites,
`landing-20260906T060027Z.log`), stamped `@ 4a152fd5 ... clean`. It is not the record,
because I edited a test assertion's string literal in `eval.rs` WHILE it was running, to
settle the dash bar. The stamp is taken at the start, so a log can truthfully say "clean"
about a tree that changed under it, and the tree that log describes is not the tree at
the tip. The edit could not change behaviour (it is text printed only on a failure), and
that is not the point: a green whose provenance has a hole in it is worth less than five
minutes of machine time. Re-run at `1b251fcb`, which is the run quoted above.

### The byte measurement the corpus sweep could not make

This is where bytes were actually compared, and it is the answer to whether the `set`
deferral changed anything real. The `set` obligation becomes a link `equ_sym`, and an
`equ_sym` that FOLDS defines a link symbol, which would land in the deb2 symbol appendix
and move the ROM CRC. Every shipped shape came out unchanged:

```
S1.4 plain:     assembled=0xbdc82  full=819131  appendix=0xa339  syms=3153  crc=1c09fbfc
S1.4 debug:     assembled=0xc055e  full=840324  appendix=0xcd26  syms=3718  crc=e2144057
S2 demo:        assembled=0x1121a  full=96602   appendix=0x6740  syms=2028  crc=11ebd7ab
S2 demo_debug:  assembled=0x1121a  full=102818  appendix=0x7f88  syms=2364  crc=9b0d2ce7
S2 config_a:    assembled=0xc055e  full=840676  appendix=0xce86  syms=3742  crc=213eee40
S2 config_b:    assembled=0x8ccc0  full=617819  appendix=0xa09b  syms=3078  crc=7ad605fc
S2 lean:        assembled=0xbcc00  full=773120  appendix=0x0     syms=3062  crc=3fd246f7
```

Seven shapes, each byte-compared against `golden/` (`native_full_sonic4_plain`,
`native_full_sonic4_debug`, `native_offcanonical_full`'s five). The symbol counts are
unchanged too, which is the specific reading that matters here: aeon has NO `set`/`:=`
whose right-hand side fails to fold in the AS front end, so the new obligation rows are
an empty population in the shipped build.

**And that is a snapshot, not a property.** An aeon `.emp` seam that later writes
`x := SomeEmpLabel` WOULD newly export `x` and move the appendix. Nothing schedules that,
and nothing warns if it happens except these CRC gates, which is where it would surface.

## Anything in this brief you concluded was wrong

1. **"The row says the divergence is between the LINKER PATH and a SINGLE-FILE route.
   Measured, that is also wrong. Both routes behave identically."** This is the brief's
   central claim and it is wrong for the `equ`/`=` family, which is the family the brief
   reproduced with. At master, `resolve_layout` → `link` REFUSED an unread
   `Val equ Missing` with a fully-formed diagnostic naming both symbols, while `link()`
   alone accepted it silently. The row's route framing was right there. What made the
   brief's CLI measurement come out "identical" is that only one CLI route takes a
   `.asm` file at all, so both of its observations were of the same route.

2. **"The discriminator is whether the assigned name is USED."** True at the CLI, and
   true for `equ` only because the route that would have refused it was not reachable
   from there. For `set`/`:=`/`eval` there was no discriminator at all: nothing was
   recorded on any route, so use could not have separated anything.

3. **"...it wants a different fix: evaluate the right-hand side where it is written."**
   That fix cannot be taken. Evaluating at the assignment and refusing there is what asl
   does, and it would refuse the cross-seam equates the mixed AS + `.emp` build depends
   on (`Game_Entry = GameState_OJZScroll_Init`), because the front end cannot tell a
   never-defined name from a `.emp` label the link is about to supply. The refusal has
   to be taken where the whole program is visible. What the fix DOES buy is the thing
   the brief was after: the refusal is reported at the assignment's own line, and it does
   not ask whether the value is read.

4. **"the four-corpus per-file sweep compares ALMOST NO BYTES ... both arms accepted 2 of
   1,753 files."** Directionally right and understated: measured here, both arms accepted
   **0 of 1,828**, and the five ROOT sources are byte-vacuous too (both arms exit 1 on
   `listing` with an empty image). The sweep is not "almost no bytes", it is no bytes.

5. **"`grep` is a SHELL FUNCTION here and its `-r` silently skips gitignored files."**
   Not disputed and I obeyed it, but `ls` is the one that actually bit: bare `ls <dir>`
   in this shell is an alias that rejects a trailing-slash argument as an `--icons`
   value and exits 2. `/usr/bin/ls` by absolute path throughout.

6. **"check your scanner against a canary that must return non-zero: a sibling parcel's
   dash scan returned 0 for every file AND 0 for a string containing a literal em
   dash."** Confirmed by reproduction, and the mechanism is worth writing down because I
   walked straight into it. A `grep` whose pattern is a `$'...'` string holding U+2014
   and U+2013 with a `\|` alternation returns 0 in this shell against a subject that
   plainly contains both characters. The canary caught it on the first try
   and the scan was redone in Python, where the same canary returns 2. Anyone reaching
   for a shell one-liner to check this bar will get a clean answer from a broken
   instrument.

7. **A brief-adjacent one, offered because it cost time.** The first version of my sweep
   script was clean-by-instrument in exactly the shape bar 6 warns about, and the canary
   caught it, but the mechanism was not a broken pattern. It was `cd`-per-file plus a
   RELATIVE binary path, which turned every comparison into two identical
   `timeout: failed to run command` lines. A canary that only checks "can the scanner
   ever return non-zero" on a fixed string would have missed it; the canary has to run
   the real comparison over inputs that must differ.
