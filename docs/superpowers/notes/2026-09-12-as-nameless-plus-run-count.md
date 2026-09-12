# AS-NAMELESS-PLUS-RUN-COUNT: a `+` run leaves asl's forward counter standing

Queue row: sigil advanced the nameless forward counter by m for a `+` x m
definition, asl does not, so a reference spanning a run resolves differently
with no diagnostic. Opened by
`2026-09-12-as-macro-label-leak.md` ("asl's nameless counters are not what
sigil models, and not only in macros"), which recorded probe `n21` as
divergent because of it.

Branch `parcel/as-nameless-plus-run-count`, base `c6651e45`.

## Provenance

* Oracle: `s1disasm/build_tools/Linux-x86_64/asl`, md5
  `61e672562465725a8c102288a7da9098`, only through
  `asl-reference/asl_ref.sh`'s `asl_run`, flags `-xx -n -q -A -L -U -i .`.
  Bytes come from the `p2bin` beside it, and only from runs that exited 0.
* sigil: built in this worktree's own `target/`. Base binary: this worktree
  at `c6651e45` before any change (`sigil 0.1.0 (c6651e45)`, "clean-sources
  at capture"). Fix binary: this branch at `ee1c8807`, its last source
  change (`sigil 0.1.0 (ee1c8807)`, "clean-sources at capture"). Mutation
  binaries are named where they are used.
* Every probe is `org $100` behind a `$1111` filler word, and each
  definition carries its own data word (`$2222`, `$3333`, `$5555`, `$6666`,
  `$7777`), so a reference's value names the line it landed on and a zero
  cannot pass for an answer. One shape per file, at most one suspect line
  (`; REF`; a `rept` may repeat it).

Beside this note in `2026-09-12-as-nameless-plus-run-count/`: `gen.py` (the
65 probes, `probes/`), `symrun.sh` (asl's exit, first errors, its nameless
symbol rows `__forwN` / `__backN`, and its image from `$100`), and
`asl-symbols.txt` (that runner over all 65: 41 exit 0). The both-assembler
matrix is the macro-label-leak note's `matrix.sh` + `classify.py`, run on
scratch copies.

asl's symbol table shows file-level names only: a name filed in a macro
expansion or a loop iteration is local to it and absent from the table, so
for body and loop shapes the evidence is the bytes.

## The rule, as asl answers it

asl's names are zero-based: `__forwN` is sigil's forward slot `N + 1`. Below,
c is asl's forward counter (the number of single `+` and `/` definitions so
far).

| definition | asl names | counter |
|---|---|---|
| `+` | `__forw(c)` | c += 1 |
| `++` | `__forw(c+1)` | unchanged |
| `+++` | `__forw(c+2)` | unchanged |
| `++++`, `+++++` | `error #1020: invalid symbol name` (`a07`, `f03`) | |
| `-` | `__back(b)` | b += 1 |
| `/` | `__forw(c)` only | c += 1, b unchanged |

The note's hypothesis survives: a run of length m >= 2 names
`__forw(c + m - 1)` and does not advance c.

```text
a08  +, +, ++, +       __forw0 = 102   __forw1 = 104   __forw3 = 106   __forw2 = 108
a05  +++, ++           __forw2 = 102   __forw1 = 104
a06  ++, +++           __forw1 = 102   __forw2 = 104
a11  /, ++, +          __forw0 = 102   __forw2 = 104   __forw1 = 106
```

**The alternative "the counter moves to the next undefined slot" is refuted**
by the collisions. Under it `+`, `++`, `+`, `+` would name `__forw0, 2, 1, 3`;
asl instead names the fourth `__forw2` again and refuses it:

```text
a01  +, ++, +, +        error #1000: symbol double defined   (line 9, the fourth)
a02  ++, ++             error #1000: symbol double defined
a03  +++, +, +, +       error #1000: symbol double defined   (the fourth)
a04  ++, +, +           error #1000: symbol double defined   (the third)
a12  +, ++, +, +, ++    error #1000 at the fourth; the fifth, `++`, is __forw4 = 10A,
                        so the colliding `+` still advanced the counter
```

The collision is per namespace. It fires inside one macro expansion (`g01`),
in each iteration of a `rept` (`g03`, reported for both), where the second
definition is a `/` (`g02`: `++`, `/`, `+`), and where a body's `+` moves the
shared counter onto a file-level run's slot (`g04`: file `++`, body `+`, file
`+`). A body's slot and a file-level slot with the same number do not collide
(`c06`, accepted, `__forw2` both in the body at `$104` and at file level at
`$10A`).

### References spanning a run

A reference `+` x k names `__forw(c + k - 1)`, as before. With the counter
standing still, a reference can land where the old model could not:

```text
b02  dc.w ++ ; +, ++, +     -> $0108   the LAST `+` (old model: slot 2, undefined)
b03  dc.w +++ ; +, ++, +    -> $0106   the `++`
b06  ++ ; dc.w ++ ; +       -> $0102   BEHIND itself, on the `++`
b08  ++ dc.w ++ ; +         -> $0102   the `++`'s own line
b17  ++ ; bra.s ++ ; +      -> 60FC    backward branch
b13  dc.w + ; +++, +, +     -> $0106   the first single `+`, past the `+++`
b18  dc.w ++ ; /, ++, +     -> $0108   a `/` counts as a single `+` here
```

The same holds in a macro body (`c01`: the body's `dc.w ++` lands on its own
`++`, `$0102`; `c02`) and in a loop iteration (`d02`: each iteration's `++`,
`$0102` then `$0106`; `d04`). A run in a body or an iteration does not move
the counter for the file level either: `c03`, `c04`, `d01` reach past it to a
file-level `+` (`__forw0` is that `+` in all three), and `d03`'s `+++` from
before a `rept 2 { + }` lands on the file-level `+` after the loop (`$010A`).
`c05` names the body's `++` from after the call: `#1010 symbol undefined`.

`n21` (the macro-label-leak note's probe) names `+++` before a body `++` and a
file-level `+`. Slot 3 (`__forw2`) is the body's, so asl refuses it
(`#1010 symbol undefined`, its line 9). The old counter put the file-level `+`
on slot 3, and sigil built it to `$0106`.

### A nameless name is one to three characters, reference too

```text
f01  dc.w ++++  over four `+`         error #1110: wrong number of operands
f04  bra.s ++++ over four `+`         error #1110
f02  dc.w ----  under four `-`        error #1110
f08  dbf d0,---- under four `-`       error #1110
f06  dc.w -----1                      error #1110
b04  dc.w ++++  (no fourth slot)      error #1110
f05  dc.w ----1 under three `-`       $0101   (---) - 1
f07  dc.w ++++1 over three `+`        $0109   (+++) + 1
```

asl reads a longer run as operators, even where a fourth definition exists
for it to name. In front of an operand the last sign is the operator, so a
run of four there is a three-character name.

### `/` and the backward counter (the x7 measurement)

x7 recorded that a `/` in a macro body advances asl's forward counter and not
its backward one. It is not a macro-body effect: a `/` never takes a `__back`
name and never advances the backward counter, at file level as well.

```text
a09  /, -         __forw0 = 102 (the `/`)   __back0 = 104 (the `-`)
a10  -, /, -      __back0 = 102   __forw0 = 104   __back1 = 106
e07  body `/` called twice; file `-`     __back0 = 106
e08  rept 1 { / }; file `-`              __back0 = 104
e09  body `/`; file `+`, `-`             __forw1 = 104   __back0 = 106
```

Yet a backward reference reaches a `/`: `e04` (body `/`, `-`, then `dc.w --`)
and `e05` (body `-`, `/`, then `dc.w --`) both land on `$0102`, the second
nearest of the two, and the 2026-09-09 note's `ord2` does the same at file
level. So asl does not resolve `-` x k by its `__back` counter alone. sigil
keeps ONE backward sequence counting `-` and `/`, which names different
symbols from asl's table and resolves every measured backward reference to
asl's address (`e04`, `e05`, `e07`, `e08`, `e09`, `e12`, `b15`, `a09`, `a10`
build identically; `e06`, `e10`, `e11` are refused by both).

**Not changed, and booked here rather than as a row:** no shape found gives a
different address, so there is nothing to fix. Mutation M5 below shows why the
literal copy would be wrong.

## The fix

All in `crates/sigil-frontend-as`:

* `eval.rs`, `bind_nameless_def`: `+` x m defines forward slot `fwd + m`;
  only m = 1 moves `fwd` onto it.
* `eval.rs`, `define_nameless_slot`: a key this pass has already defined is
  refused with asl's wording, `symbol double defined`. The key is the FILED
  one (` exp#N. nameless+#k` in an instance), so the check is per namespace,
  which is where asl draws it. `defined_this_pass` is the witness: `Asm` is
  built fresh by `one_pass_with_defer` for every pass, so the set is per pass.
  The slot is still bound after the report, as `define_label` does for a PC
  label; the diagnostic fails the build.
* `nameless.rs`: `MAX_RUN = 3`. `classify_def` returns `Invalid` for a `+`
  run longer than three, so `++++` gets the same refusal as `--` and `//`.
* `expr.rs`, `parse_atom`: a nameless reference longer than `MAX_RUN` returns
  `None`, the caller's own refusal (`bad word expression`, `bad operand
  expression`). The run is still counted with a `take_while`, so a run of any
  length is refused in constant stack.
* `nameless.rs`'s module doc now describes the run rule, the collision, the
  length limit, and the `/` naming difference; its "What this module
  deliberately does NOT model" section described this row and is gone.

Also changed, because they described the old model: `as_nameless_labels.rs`'s
rules table and its `++` test (renamed
`a_multi_plus_definition_names_the_slot_that_many_ahead`; its `q8` bytes are
asl's under both models and unchanged), and `expr.rs`'s unit test
`deep_unary_chains_do_not_abort`, which now asserts that the 60,000-deep `-`
run is refused (asl refuses `-----1`, `f06`) and still terminates on a
4 MiB stack.

## Before and after

Base `sigil 0.1.0 (c6651e45)`; fix `sigil 0.1.0 (ee1c8807)`, this branch's
last source change, "clean-sources at capture". Every run completed: each
`SUMMARY.tsv` ends in `SHAPES_RUN=<n>` and the end marker (67, 84, 54, 8, 4).
Per-shape tables beside this note: `compact-base-new.txt` /
`compact-fix-new.txt` (this note's 67), `compact-base-old1..4.txt` /
`compact-fix-old1..4.txt` (the macro-label-leak note's `probes`, `probes2`,
`probes3`, `probes4`).

### The macro-label-leak note's 150 shapes

| verdict | base `c6651e45` | fix `ee1c8807` |
|---|---|---|
| MATCH | 146 | 147 |
| LEAK | {a12, n21} | {a12} |
| OVER-REFUSE | {a11, gs8} | {a11, gs8} |
| VALUE-DIFF | none | none |

By batch, base then fix: `probes` 82/1/1 and 82/1/1, `probes2` 53/1/0 and
54/0/0, `probes3` 7/0/1 and 7/0/1, `probes4` 4/0/0 and 4/0/0.

A per-shape diff of the verdict column, 150 rows a side, reports one line:
`n21_plusplus_in_body LEAK` became `MATCH`. **n21 is the only shape whose
verdict moved.** The diff is not blind: the same `diff` between two streams
known to differ (this note's base table against those 150) reports 217
lines. n21 now: asl `#1010 symbol undefined` at its line 9, sigil `unresolved
symbol ` nameless+#3`` at link, at line 9.

### This note's 67 shapes

| verdict | base `c6651e45` | fix `ee1c8807` |
|---|---|---|
| MATCH | 31 | 66 |
| LEAK | 19: a01 a02 a03 a04 a07 a12 b04 f01 f02 f03 f04 f06 f08 g01 g02 g03 g04 h01 h02 | none |
| OVER-REFUSE | 17: b02 b06 b08 b09 b12 b13 b16 b17 b18 c01 c02 c03 c04 d01 d02 d03 d04 | 1: b16 |
| VALUE-DIFF | none | none |

35 shapes moved, every one to MATCH, and the 31 that matched at base still
match. By mechanism: the counter rule moves the 16 OVER-REFUSEs other than
b16, and makes the 11 collision LEAKs (a01..a04, a12, g01..g04, h01, h02)
refusals; the definition limit moves a07 and f03; the reference limit moves
f01 f02 f04 f06 f08 (b04 is refused by either the counter rule or the
limit). b16 is not a nameless shape; see "Left open".

## Tests and their red-first evidence

`crates/sigil-frontend-as/tests/as_nameless_plus_run_count.rs`, 6 tests.
Every fixture is a committed probe read at test time (n21 from the
macro-label-leak note's `probes2/`), every expected byte string that probe's
asl `p2bin` image from an exit-0 run (`asl-symbols.txt`), every expected
refusal a probe asl refused.

| test | carries |
|---|---|
| `a_reference_spanning_a_plus_run_lands_where_asl_puts_it` | b01..b03, b05..b14, b17..b19; a05 a06 a08 a11 |
| `a_plus_run_in_a_macro_body_or_a_loop_iteration_leaves_the_counter_too` | c01..c04, c06, d01..d04; c05 and n21 refused |
| `a_single_plus_that_reaches_a_runs_slot_is_symbol_double_defined` | a01..a04, a12, g01..g04, h01, h02 |
| `a_nameless_definition_is_at_most_three_plus_signs` | a07 f03 e01 e02 e03 refused; a05 b14 accepted |
| `a_nameless_reference_is_at_most_three_signs` | f01 f02 f04 f06 f08 b04 refused; f05 f07 accepted |
| `a_backward_reference_reaches_a_slash_where_asl_resolves_it` | e04 e05 e07 e08 e09 e12 b15 a09 a10; e06 e10 e11 refused |

Each mutation below was applied to a committed baseline (`31ee7423` for M1
and M2, `5b7b2311` for M3 to M5) with its anchor asserted to match exactly
once, the mutated line quoted back from disk with `grep -n`, and `git diff
--stat` naming the one file. Then the named runner ran (`cargo test --release
-p sigil-frontend-as --test as_nameless_plus_run_count --test
as_nameless_labels`, plus `--lib deep_unary_chains_do_not_abort` for M4), and
the file was restored with `git checkout --`, with `git status` showing no
tracked modification after each.

| mutation | removes | red | first failing assertion |
|---|---|---|---|
| M1 | `if m == 1` becomes `if m >= 1`: the counter advances by m again | 3 | `b02 ... sigil refused: unresolved symbol ` nameless+#2``; `c01 ... unresolved symbol ` nameless+#4``; `a01: asl refuses it, sigil built [11, 11, 22, 22, 33, 33, 55, 55, 66, 66, 44, 44]` |
| M2 | the front end's collision check (`if false && ...`) | 1, **on the fragment only** | `a01: refused, but no message mentions symbol double defined: ["symbol ` nameless+#3` redefined by section `sec256` ..."]` |
| M3 | the definition limit (`Punct::Plus if n <= MAX_RUN` becomes `Punct::Plus`) | 1 | `a07_pppp: asl refuses it, sigil built [11, 11, 22, 22, 44, 44]` |
| M4 | the reference limit (`if false && ref_len > nameless::MAX_RUN`) | 2 | `f01 ... sigil built [11, 11, 01, 0a, ...]`; `deep_unary_chains_do_not_abort` |
| M5 | a `/` defines its forward slot only and leaves `bwd`: asl's naming, copied literally | 3 | `e04 ... unresolved symbol ` nameless-#0``; `ordinals_count_forward_and_backward_and_slash_counts_for_both`; `the_four_corpus_shapes_match_the_reference` |

**M2 is a finding, stated rather than hidden.** With the check removed, an M2
binary (`31ee7423-dirty`) still refuses all eleven collision shapes, every
one by the IR builder's duplicate-label check, reported at line 1 and naming
the internal slot. h01 and h02 were written to find a hole there (both
definitions at one address, where a check keyed on value might let a
duplicate through); there is none. So the check is outcome-redundant with the
builder on the verdict. It carries asl's stage (the front end), asl's line
(the colliding one) and asl's words, and the test's fragment `symbol double
defined` is what sees that. `31ee7423`'s message claimed the opposite; see the
last section.

**M1 leaves the older nameless test file green** (11 of 11): none of its
shapes has a single `+` after a run, which is how the gap stood unmeasured.

## Verification

At `31ee7423` and again at `ee1c8807`, in this worktree, tracked tree clean:
`cargo test --release -p sigil-frontend-as --no-fail-fast` gave 854 passed,
0 failed, 0 ignored, with 77 `test result:` lines against 77 `Running` and
`Doc-tests` headers, exit 0. `cargo clippy --release -p sigil-frontend-as
--all-targets -- -D warnings` exited 0 with zero diagnostics both times. The
workspace suite and the aeon byte gate were not run here (no aeon tree
leased).

## Corpus exposure

Tracked sources only, `git -C <repo> grep` (never the shell's `grep -r`,
which is a ugrep function here that skips gitignored files), s1disasm
`f6ece65`, s2disasm `e45ebf3`, skdisasm `2fcd861`.

| question | pattern | s1 | s2 | sk | the same pattern on this note's probes |
|---|---|---|---|---|---|
| a column-1 `++` or `+++` definition | `-E '^\+\+'` | 0 | 0 | 0 | 47 lines |
| a reference of four or more signs, outside a `;` comment | `-InP '^[^;]*[\s,(](\+{4,}\|-{4,})'` | 0 | 0 | 0 | 8 lines |
| a column-1 run of four or more `+` | `-nP '^\+{4,}'` | 0 | 0 | 0 | 2 lines |
| control: a column-1 single `+` definition | `-E '^\+([[:space:]]\|$)'` | 1 | 2014 | 90 | |
| control: a three-sign reference | `-InP '^[^;]*[\s,(](\+{3}\|-{3})(?![+\-])'` | 0 | hits (`s2.asm:875 blt.s +++`, `:2917`, `:2919`, `:2996`, `:5622`) | 0 | |

A `\|` in the pattern column is Markdown's escape for the table; the pattern
run was a plain `|` (for example `'^[^;]*[\s,(](\+{4,}|-{4,})'`).

The first three rows are the zero this parcel's exposure rests on, and each
pattern fires on this note's committed probes, which contain the construct.
The `-I` matters: without it the three-sign control's first lines were all
`Binary file build_tools/*/as.msg matches`, which reads like a hit and is not
one.

**The corpus does not exercise this fix at all.** No source in the three
defines a `++` or `+++`, and the counter change and the collision check both
need one: on a source with no run definition, advance-by-m and the measured
rule are the same function, since a single `+` advances by one under both.
No source writes a nameless name longer than three signs, so neither cap is
reached. Every corpus reference, including s2's `+++` references, resolves as
before. The brief asks for a census before and after only if a corpus shape
is affected; none is, so none was run.

aeon: not measured. No aeon tree was leased to this parcel; the landing
gate's byte identity is the check there, and whether aeon's AS residual
defines a `++` I did not look at.

## Left open

* **b16, a zero-distance `bra.s`.** asl emits `4E71` with `warning #60:
  distance of 0 not allowed for short jump (NOP created instead)`; sigil
  refuses at link (`sigil-link/src/lib.rs:724`, "a 0x00 byte displacement is
  the 68000 word-form escape, not a branch to the next instruction"). Loud,
  and sigil's wording shows the refusal is deliberate. Not nameless-label
  behaviour (b19 is the same shape with a `nop` in between and matches); its
  own row if AS compatibility should follow asl there.
* **The `/` naming difference** (x7), booked above: no shape found moves an
  address, so no row unless one is found.
* **a11, a12, gs8**: unchanged by this parcel, the macro-label-leak note's
  other rows.
* **The collision check is outcome-redundant with the builder's duplicate
  label check** on every collision shape measured (see M2 below). It is kept
  because it is asl's refusal at asl's stage and line.
* Not probed: runs in `irp` / `irpc` / `while` iterations (only `rept`); a run
  on a line whose remainder opens a block (`++\trept 2`), which reaches the
  same `bind_nameless_def` through `bind_head_label`; runs in a
  `{GLOBALSYMBOLS}` body; runs across an `include`.

## Things in the brief, and in this parcel's own record, that turned out wrong

1. **x7 is not a macro-body effect.** "A `/` written in a macro body advances
   asl's forward counter and not its backward one" is true, and it is true at
   file level as well: a `/` never takes a `__back` name (`a09`, `a10`). Nor
   does it move an address on any measured shape; see "`/` and the backward
   counter".
2. **The rule has two more parts than the counter.** The note and the brief
   describe the counter; asl's answers add the collision refusal (`#1000`,
   which the counter makes reachable) and the three-character limit (`#1020`
   in column 1, `#1110` in an operand). The limit was eight silent LEAKs at
   base (`a07 f03 f01 f02 f04 f06 f08 b04`), not a curiosity.
3. **Commit `31ee7423`'s message is wrong twice.** "Without the check the
   slot is silently rebound": M2 refutes it, the builder refuses every
   collision shape without the check. "asl names slot c+m" mixes numberings:
   asl names `__forw(c+m-1)`, which is sigil's slot `fwd + m`. The commit is
   not rewritten; `5b7b2311` corrects the test's doc, and this is the record.
4. **An instrument trap, not a brief error.** `matrix.sh` changes into the
   probe directory before invoking sigil, so a RELATIVE sigil path is exit
   127 for every shape, and `classify.py` reads 127 as a refusal: every shape
   asl refuses then reads MATCH. This parcel's first matrix run did exactly
   that (14 MATCH that meant nothing) and was caught because every sigil
   column read 127. Pass the binary as an absolute path.
