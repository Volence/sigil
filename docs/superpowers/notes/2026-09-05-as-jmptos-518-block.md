# AS-JMPTOS-518-BLOCK: the 518-row block was two defects, and neither was the one in the brief

Sonic 2's third largest diagnostic block, 518 rows from the single line
`s2.macrosetup.asm:304`, class `bad absolute address expression`. It is closed.
The corpus falls from **5,761 rows to 5,243**, the class is **gone**, no class
rose, none appeared, and no diagnostic line in the after run is absent from the
before run.

Two commits, each with its own red-first test:

| commit | defect |
|---|---|
| `96e1dafa` | a string builtin nested in `substr`'s `pos`/`len` argument |
| `33ceea3a` | the builtin layer over a 68000 instruction operand |

Branch `parcel/as-jmptos-518-block`, off master `e4b003af`.

## Provenance

| | |
|---|---|
| corpus | `s2disasm` at `e45ebf332f39987424ca3102e50c717628f71269`, detached worktree `/home/volence/sonic_hacks/.s2-jmptos-518`, `git status --porcelain` empty. The owner's live checkout at `/home/volence/sonic_hacks/s2disasm` was read but never written. |
| sigil BEFORE | master `e4b003af`, `git archive`d to a scratch tree and built fresh, md5 `5efd3e9115012cb6e11bf492947798f8` |
| sigil AFTER | branch tip, md5 `44ada6a3e7935ffb363be28711e4ce0f` |
| command | `cd <corpus> && <bin> s2.asm`, no flags |
| reference assembler | `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`, flags `-q -A -L -U`, **exit status 0 checked on every invocation**. The `s2disasm` build (`0dee1f98e6480a4783d27ffd8b90896f`) was never invoked. |
| re-run | `docs/superpowers/notes/2026-09-05-as-jmptos-518-block-probes/run.sh <before> <after> <corpus> <out>` reproduces every number in this note, including the class table and both symbol-set directions |

**Freshness witness for the BEFORE binary.** It reports 5,761 rows over 5,113
distinct sites on this corpus, reproducing the figure
`2026-09-05-s2-top-blocks-decompose.md` measured at `297dcd8f` from a separately
built binary. A number that matches an independent earlier measurement is
evidence the instrument ran; the binary's md5 alone would not have been.

## Why the exit-status check is not decoration

The first capture of `insn_operands.asm` carried one extra line,
`jmp val("Foo")`, which asl rejects as `addressing mode not allowed here`. asl
exited **2**, and its listing still printed a full byte column for the ten lines
that HAD assembled:

```
asl exit=2  md5=61e672562465725a8c102288a7da9098
> > > insn_operands.asm(11): error: addressing mode not allowed here
       4/    1000 : 4EF9 0000 1000      	jmp	(Foo).l
       5/    1006 : 4EF9 0000 1002      	jmp	(strlen("ab")+Foo).l
       ...
```

Nothing in that listing announces itself as unusable. Those bytes were discarded
and the offending line deleted rather than quoted, and the probe re-run to exit
0. Every expected value in both test files comes from an exit-0 listing, and the
listings are committed verbatim beside the probes.

## The bisection, as re-derived

### DEFECT 1: a string builtin inside `substr`'s `pos` or `len`

`substr(s, pos, len)` folds `pos` and `len` through `fold_const`, which ran user
`function` expansion and `parse_expr` and nothing else. A `strstr(...)` /
`strlen(...)` / `val(...)` call in either argument therefore parsed as a bare
symbol followed by a paren group, the fold returned `None`, and the OUTER
builtin reported `could not evaluate string builtin`, naming a call the source
had not got wrong.

Whether the nesting worked depended on what SURROUNDED it, which is what made it
hard to see. The top-level builtin scanner walks its token slice linearly and
descends into an unrecognized head's argument list as a side effect of that
walk, so in

```asm
	dc.b	substr("JmpTo_Foo", strstr("JmpTo_Foo","_")+1, 3)   ; assembles on master
```

the `strstr` is replaced by an `Int` before `substr` is ever evaluated. The same
`substr` consumed by an outer `val` is taken whole by that outer call and never
gets that treatment:

```asm
	dc.l	val(substr("JmpTo_Foo", strstr("JmpTo_Foo","_")+1, 3))
	; master: val(): could not evaluate string builtin
```

**The fix.** `expand_str_builtins` was split into a `&self`, diagnostic-free
`scan_str_builtins` and two wrappers: the existing `&mut self` one that emits the
failures, and a new `expand_str_builtins_opt` that returns `None` instead, so an
inner failure does not print a second message for one cause. `fold_const` calls
the latter.

### DEFECT 2: no builtin layer at all on a 68000 instruction operand

`lower_m68k` called `expand_calls_m68k_operands`, which ran
`expand_calls_checked` and stopped. That is ONE of the four layers
`dc.b`/`dc.w`/`dc.l` have run since the `expand_operand_builtins` parcel: user
`function` calls, then `int(...)`/`sin(...)`, then the string builtins, then
string comparisons. Measured on master, all of these are refusals, and note that
one of them contains nothing string-shaped at all:

```
inertness.asm(18): error: trailing tokens in absolute address   jmp    (strlen(...)+Foo).l
inertness.asm(19): error: trailing tokens in absolute address   jmp    (strlen(...)+Foo).w
inertness.asm(20): error: trailing tokens in absolute address   jsr    (val("Foo")).l
inertness.asm(21): error: trailing tokens in #immediate         move.l #strlen(...),d0
inertness.asm(22): error: trailing tokens in #immediate         move.l #int(3.7),d0
inertness.asm(23): error: trailing tokens in operand            move.l strlen(...)+Foo,d0
inertness.asm(24): error: trailing tokens in absolute address   move.w d0,(val("Foo")).l
inertness.asm(25): error: trailing tokens in operand            bra.w  strlen(...)+Foo
```

asl draws no such distinction. `insn_operands.asm`, exit 0, has `int`, `val` and
`strlen` answering in an immediate, in a long-absolute address and in a bare
absolute.

**The fix.** `expand_calls_m68k_operands` routes the same slices through
`expand_operand_builtins` instead of `expand_calls_checked`. The held-back EA
base group is untouched, which is what keeps `val(a0)` a displacement rather than
a call.

### How the two compose, and why the block reads as one thing

With defect 1 fixed and defect 2 open, the corpus construct reports
`bad absolute address expression`, the 518-row message. With defect 2 fixed and
defect 1 open it reports `val(): could not evaluate string builtin`. Neither fix
alone assembles the line, and the message the block is named after is emitted by
neither defect's own code.

## The tests, and their red-first proofs

Both files run under `cargo test -p sigil-frontend-as` and under
`cargo test --workspace`.

### The red-first proof, with the mutation shown applied on disk

Each file was first run before its fix was written, which is a red-first in
sequence but leaves nothing a later reader can check. So after both commits
landed, the whole thing was re-established as a MUTATION, because an unapplied
patch and a correct restore are the same artefact and both print `ok`:

```
$ git checkout e4b003af -- crates/sigil-frontend-as/src/eval.rs
$ git diff HEAD --stat -- crates/sigil-frontend-as/src/eval.rs
 crates/sigil-frontend-as/src/eval.rs | 86 +++++-------------------------------
 1 file changed, 10 insertions(+), 76 deletions(-)
$ grep -c "scan_str_builtins\|expand_str_builtins_opt" crates/sigil-frontend-as/src/eval.rs
0
```

The diff stat and the absent marker names are the proof the mutation LANDED, and
they are quoted rather than described because that is the step a vacuous proof
skips. `git diff --stat` alone would have shown nothing here: `git checkout <rev>
-- <path>` stages as well as writes, so the unstaged diff is empty and the file
on disk is master's. A run that read only `git diff --stat` would have concluded
the mutation had not applied when it had.

With it applied:

```
test a_builtin_name_used_as_a_displacement_symbol_is_untouched ... ok
test builtins_in_every_instruction_operand_position ... FAILED
test the_sonic_2_jump_table_generator ... FAILED
test result: FAILED. 1 passed; 2 failed

test a_computed_name_that_no_label_defines_is_still_refused ... ok
test strlen_in_the_length_argument_of_a_nested_substr ... FAILED
test strstr_in_the_position_argument_of_a_nested_substr ... FAILED
test a_substr_with_plain_arithmetic_arguments_is_unchanged ... ok
test the_corpus_function_body_through_a_user_function_call ... FAILED
test result: FAILED. 2 passed; 3 failed
```

Restored with `git checkout HEAD -- crates/sigil-frontend-as/src/eval.rs`, a
COMMITTED baseline and not a `git checkout --` over a dirty tree, which would
have deleted uncommitted work: `3 passed` and `5 passed`.

### `crates/sigil-frontend-as/tests/as_str_builtin_nesting.rs`, 5 cases

Red-first run, taken with the `eval.rs` edit not yet written (re-established as a
shown-applied mutation above):

```
test strstr_in_the_position_argument_of_a_nested_substr ... FAILED
test strlen_in_the_length_argument_of_a_nested_substr ... FAILED
test the_corpus_function_body_through_a_user_function_call ... FAILED
test a_substr_with_plain_arithmetic_arguments_is_unchanged ... ok
test a_computed_name_that_no_label_defines_is_still_refused ... ok
test result: FAILED. 2 passed; 3 failed
```

All three failures reported `val(): could not evaluate string builtin`, the
defect's own message, so the red was the subject and not a fixture error. After
the fix: 5 passed.

The two that passed BEFORE are controls, and their passing before is what makes
them controls rather than second copies of the subject: one pins that a `substr`
with ordinary arithmetic arguments is unchanged, the other that a computed name
no label defines is still refused, so an implementation answering `0` for
anything it cannot resolve would fail there.

### `crates/sigil-frontend-as/tests/as_insn_operand_builtins.rs`, 3 cases

Red-first run, same discipline:

```
test a_builtin_name_used_as_a_displacement_symbol_is_untouched ... ok
test builtins_in_every_instruction_operand_position ... FAILED
test the_sonic_2_jump_table_generator ... FAILED
test result: FAILED. 1 passed; 2 failed
```

with the failures reporting seven `trailing tokens in ...` and two
`bad absolute address expression`. After: 3 passed.

### Why the fixture values are discriminating

The lane's standing hazard is a probe whose right and wrong answers look alike.

* `strlen` is taken over a **twelve**-character string. A length of 1 is
  indistinguishable from a "found" boolean and 0 from a failure that folded to
  zero.
* `strstr("JmpTo_Foo","_")` is **5**, not 0 and not 1, and the `+ 1` past it is
  the difference between `"_Foo"` and `"Foo"`. An off-by-one there is a
  different SYMBOL NAME, so it cannot fold to a near-miss number: it either
  refuses or names something else.
* `val` is asked for **both** `"$4142"` and `"4142"`, which asl answers `$4142`
  and `$102E`. `val` is an expression evaluator, not a decimal parser, and this
  pair is the only kind of fixture that can tell those apart. A single-digit
  probe spells the same characters in both radices, which is how a live
  hex-versus-decimal divergence survived months in this repo.
* The corpus fixture has **two** targets, at `$1000` and `$1008`, and the
  function fixture two at `$1000` and `$100C`. An implementation that resolved
  every call to one symbol passes a one-target probe and fails these.
* Every fixture sits at `org $1000` rather than 0, because 0 is a value a broken
  fold produces by accident.
* `move.l strlen("abcdefghijkl")+Foo,d0` assembles to `2038 100C`, which is
  `abs.w`. The width rule is not touched by this parcel and the fixture would
  catch it if it were.

## The shipping-build argument, in place of the byte gates

`sigil-frontend-as` is on aeon's shipping path: `build.sh` routes three residual
`.asm` files through it. **The byte-identity golden gates did not execute in this
parcel.** The run was declared partial and said its own size:

```
PARTIAL RUN (SIGIL_ALLOW_PARTIAL is set). No reference tree is named, so 127
test binaries are reference-dependent and every row in them is left UNMEASURED.
A green result from this run does NOT mean those rows passed, it means they were
not run.
```

That includes the anchor goldens whose names appear as `ok` in the log
(`config_a_anchor_matches_golden`, `demo_plain_anchor_matches_golden`, and the
rest). Those `ok` lines are **not** evidence about aeon bytes.

What stands in for them is a claim with a falsifier, not an assurance:

> Any 68000 instruction operand that spells a builtin head immediately before a
> `(`, outside a held-back EA base group, is a hard refusal on master.

If that holds, no program that assembles today contains such an operand, so no
program that assembles today can change what it assembles to. The mechanism is
that `parse_expr` has no call syntax, so `name(...)` outside a held-back group
leaves trailing tokens and is diagnosed.

`inertness.asm` is that claim in runnable form: eight operand positions, one per
line, and **a line that assembles on the before binary is a counterexample**. All
eight are refused (quoted above). The same file's last two lines are the
exception the claim names, a builtin head used as an ordinary displacement
symbol, and those two assemble on master, assemble after, and assemble to the
same bytes:

```
== 3. the displacement control assembles to the same bytes on both ==
IDENTICAL
```

`2028 0004 2030 8804`, which is also what asl answers (`disp_head.lst`, exit 0).

The corresponding structural fact about defect 1 is narrower and needs no
enumeration: `scan_str_builtins` clones every token through unchanged and records
no failure unless the slice spells `strlen`, `strstr` or `val` immediately before
a `(`, so `fold_const` sees the identity on every other input; and on the inputs
where it does not, master's behaviour is a hard `None`. The change can turn a
refusal into a value. It cannot turn one value into another.

## The corpus effect

Exit 1 both times, **stdout 0 bytes both times: the run does not reach link.**
The front end returns `Err`, `main.rs` renders and exits, and `sigil_link::link`
is never called, before and after alike.

| rows before | sites before | rows after | sites after | row delta | class |
|---|---|---|---|---|---|
| 2624 | 2602 | 2624 | 2602 | +0 | `bad operand expression` |
| 2309 | 2309 | 2309 | 2309 | +0 | `expected mnemonic, directive, or label` |
| 518 | 1 | 0 | 0 | **-518** | `bad absolute address expression` (GONE) |
| 89 | 89 | 89 | 89 | +0 | `` `X` is not a recognized 68000 mnemonic `` |
| 49 | 1 | 49 | 1 | +0 | `bad word expression` |
| 39 | 39 | 39 | 39 | +0 | `cannot include <file>: no such file` |
| 30 | 1 | 30 | 1 | +0 | `bad byte expression` |
| 24 | 24 | 24 | 24 | +0 | ``unresolved symbol `X` in operand`` |
| 23 | 2 | 23 | 2 | +0 | `int(): could not evaluate float expression` |
| 18 | 18 | 18 | 18 | +0 | ``unknown directive or mnemonic `X` `` |
| 11 | 1 | 11 | 1 | +0 | `unexpected character` |
| 8 | 8 | 8 | 8 | +0 | `instruction needs an explicit size suffix (.b/.w/.l)` |
| 6 | 6 | 6 | 6 | +0 | `case needs a string literal` |
| 4 | 4 | 4 | 4 | +0 | `` malformed number (hex needs a trailing `X`) `` |
| 3 | 3 | 3 | 3 | +0 | ``bad displacement expression in `X` `` |
| 2 | 2 | 2 | 2 | +0 | `trailing tokens in operand` |
| 2 | 2 | 2 | 2 | +0 | `switch needs a string expression` |
| 1 | 1 | 1 | 1 | +0 | ``struct `X` has a member line this cannot read`` |
| 1 | 1 | 1 | 1 | +0 | `unsupported form: <insn>` |
| **5761** | **5113** | **5243** | **5112** | **-518** | **TOTAL** |

**No class rose. No class appeared.** Every other class is unchanged in rows AND
in distinct sites. Distinct sites fall by exactly one, the site that is gone.

Stronger than the class table, because a class table can net a rise against a
fall: the whole diagnostic multiset was diffed line by line.

```
lines present AFTER but not before: 0
lines present BEFORE but not after: 518   (all `bad absolute address expression`)
```

The after run's output is a strict subset of the before run's.

### The unresolved-symbol name sets, both directions

Eight distinct names before, eight after, and the two sets are equal:

```
newly unresolved (in after, not before):   (none)
newly resolved (in before, not after):     (none)
```

The eight are `ixl`, `ixu`, `iyl`, `iyu`, `.loop_counter`, `Snd_Sega.size`,
`zAbsVar.1upPlaying`, `zVar.1upPlaying`.

### Did the 518 lines start resolving to the WRONG thing?

The set comparison above is necessary but not sufficient here, and saying so is
the point: a `jmp` whose target is unresolved DEFERS to the linker rather than
diagnosing, and this run never reaches the linker, so a name that silently
resolved to nothing would leave no trace in either set.

Two things were done instead of leaning on it.

1. The construct's bytes were checked against asl directly, on two targets at
   two different addresses (`jmptos.asm` / `jmptos.lst`, exit 0):
   `4EF9 0000 1000` and `4EF9 0000 1008`. That is the test
   `the_sonic_2_jump_table_generator`.
2. Every name the corpus actually feeds it was checked to exist. The 549 distinct
   `JmpTo<n>_<Name>` labels in `s2.asm` strip to **92** distinct target names,
   and all 92 are defined labels in `s2.asm`. **0 missing.**

## What did not execute

* **The byte-identity golden gates.** 127 reference-dependent test binaries were
  left unmeasured, by declaration. No `AEON_DIR`, no aeon tree read or written,
  no aeon build, per the parcel's scope.
* **No emulator.** Nothing here wanted runtime confirmation; no `mcp__oracle__*`
  tool was touched.
* **`sigil_link::link` on the corpus**, before or after. The corpus still refuses
  at the front end, so no ROM bytes were produced from it by either binary.

## Suite

`SIGIL_ALLOW_PARTIAL=1 cargo test --workspace --no-fail-fast`, into
`CARGO_TARGET_DIR=.target-land`:

**4,568 passed, 0 failed, 2 ignored**, process exit 0. The two ignored are
`sigil_diff_reports_byte_identity` ("reads the aeon source tree; run with
--ignored") and `secondary_pin_classes_match_the_hand_typed_baseline` (retired by
Wave-B B-0). `sigil-frontend-as` alone: 485 passed, 0 failed.

## Anything in this brief you concluded was wrong

Three things, and the first two are the brief's own two defect claims.

**1. Defect 1 as stated is refuted.** The brief said `val()` accepts a string
LITERAL but not a COMPUTED string, on the evidence that
`val(substr("JmpTo_Foo", 7, 3))` fails while `val("Foo")` works.

The probe is confounded. `substr("JmpTo_Foo", 7, 3)` is **`"oo"`**, not `"Foo"`:
the underscore is at index 5, so the name starts at 6. `val("oo")` fails because
no symbol `oo` exists, which is correct behaviour and not a defect. With the
offset corrected, `val(substr("JmpTo_Foo", 6, 3))` **assembles on master**, and so
does `val(lowstring(...))` over a defined name. `val` over a computed string was
never the boundary. The real boundary is one level deeper and is about `substr`'s
`pos`/`len` arguments, not about `val`'s argument.

The brief's other two probes in that section are both correct as stated, and its
conclusion that a fix was needed is correct. Only the mechanism was wrong, and a
wrong mechanism would have led to a fix in `eval_str` rather than `fold_const`.

**2. Defect 2 is real but materially wider than stated**, and the brief's own
instruction to establish the boundary myself is what found it. It said the long
absolute address operand cannot hold a STRING BUILTIN "though it holds everything
else". The truth is that **no** 68000 instruction operand ran **any** builtin
layer: `move.l #int(3.7),d0` fails on master with nothing string-shaped in it,
and so do the immediate, the bare absolute and the branch-target positions. The
narrow reading would have produced a patch in `operands.rs` at the absolute-
address arm, which would have fixed the 518 rows and left `int()` in an immediate
broken.

**3. One measurement in the brief is right and worth confirming**, since a
convenient number deserves a check rather than gratitude: the brief said all four
string builtins already exist, so this is not a missing feature. That is correct.

**A caution on the total, not a correction.** The corpus fell by exactly 518,
which is exactly the subtraction the brief prohibited predicting. It is not a
prediction that came true, it is a measurement that happened to land there, and
the reason it could have gone either way is real: closing the block lets sigil
assemble 518 lines it previously abandoned. It landed on the subtraction because
the construct is self-contained (a `jmp` whose target defers to a linker this run
never reaches) rather than because subtraction was ever a sound method. Do not
quote the coincidence as evidence that the method works.
