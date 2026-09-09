# S1-DIRECTIVES-LISTING-AND-DUP-OPERAND

Two unimplemented AS features that were 21 of the 46 diagnostics sigil emitted
on the Sonic 1 disassembly: the `listing` / `page` listing-file controls, and
the `[count]value` duplicate-operand syntax.

## The instruments

**sigil, before.** `sigil 0.1.0 (bd7fce7b)`, md5 `ea2ea282285b89ad84b34801c9a20fcc`
(the controller's measurement, quoted, not re-run here).

**sigil, after.** Built from this branch at `248a5c72`, md5
`3c31cb20b2b368b4f02c72bc9a97ba07`, `CARGO_TARGET_DIR=.target-s1dir` so nothing
relinked the shared `target/release/sigil` another lane may be pinned to.

**asl.** `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`, verified before the first run. Every probe
went through `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run`, so
the digest was re-checked and the exit status reported on each invocation. A
run with a non-zero status was read for its DIAGNOSTIC only; no byte column was
taken from one.

**Corpus.** A private copy at `/home/volence/sonic_hacks/.scratch/s1-directives/s1disasm`
of the prepared tree, rev `f6ece657`, entry `sonic.asm`. Generated-include
readiness 4/4 both runs, so both numbers are baselines rather than an absent
generator's shadow.

## The corpus baselines

`scripts/corpus-baseline.sh --compare` against the controller's
`baseline-bd7fce7b/s1-bd7fce7b.err`:

```text
  level    before   after   delta  class
  error        18       0     -18  unexpected character  <== GONE
  error        11       9      -2  `X` is not a recognized N mnemonic
  error         6       6      +0  bad immediate expression
  error         6       6      +0  case needs a string literal
  error         2       2      +0  switch needs a string expression
  error         2       2      +0  unresolved if condition
               46      25     -21  TOTAL

  error         1       0      -1  unknown directive or mnemonic `X`  <== GONE

  lines only in the NEW run:  0
  lines only in the OLD run:  4
    MacroSetup.asm(7): error: `listing` is not a recognized 68000 mnemonic
    MacroSetup.asm(8): error: `page` is not a recognized 68000 mnemonic
    MacroSetup.asm(98): error: unexpected character
    sound/z80.asm(11): error: unknown directive or mnemonic `listing`
```

**No new diagnostics were exposed, of any class, at any site.** The brief
expected the count to rise where accepting a construct uncovers what sits behind
it, and it did not: the 21 rows came off and nothing came on. Both classes clear
to zero; the third moves 11 to 9 because two of its rows were `listing` and
`page`, and the remaining nine are `charset`, another parcel's.

A falling count is evidence noise was removed, never that anything is correct.
The correctness claim rests on the byte assertions below, not on this table.

## Group A: `listing` and `page`

`MacroSetup.asm:6-9` is `padding off` / `listing purecode` / `page 0` /
`supmode on`. Two of the four were handled and two were not, and the two that
were not produced DIFFERENT diagnostics on the two CPU surfaces
(`is not a recognized 68000 mnemonic` versus `unknown directive or mnemonic`),
which reads as two refusal sites. It is one absence with two fallbacks: under
`cpu 68000` an unrecognised head reaches `lower_m68k`, under `cpu z80` it reaches
`dispatch`'s final arm.

`padding` was the population proxy for the enumeration sites, since it is the
same shape of directive (an operand, no bytes, no block). It appears in exactly
three places, and so `listing`/`page` needed:

| site | needed | why |
|---|---|---|
| `dispatch`'s match (`eval.rs:5069`ff) | ADDED | routes the directive |
| `is_op_keyword` (`eval.rs:9418`ff) | ADDED | a COLUMN-0 spelling is a directive, not a label |
| `scan_plain_labels` (`eval.rs:10309`ff) | already present | the name column |

The second is the one a dispatch-only fix loses, and it is invisible in the
corpus: every corpus site is indented, and an indented unknown head dispatches
anyway. Measured (mutation M2 below): with the dispatch arm alone, a column-0
`listing purecode` binds a symbol named `listing` and hands `purecode` to
instruction lowering, which reports `` `purecode` is not a recognized 68000
mnemonic `` on correct source.

**Implementation: accept, check the arity, ignore the value.** sigil emits no
listing file, so neither directive can change a byte.

- The ARITY is checked because asl checks it: probe `l3` gives
  `error #1110: wrong number of operands`, `expected one argument` for `listing`
  and `expected between 1 and 2 arguments` for `page`.
- The VOCABULARY is not checked, and this is a deliberate divergence. asl does
  check it (probe `l2`: `listing zqp_bogus` is `error #1520: only ON/OFF
  allowed`) but that message understates asl's own accepted set, which takes
  `purecode`. A vocabulary sigil could only guess at would refuse working
  source, and nothing downstream reads the value.
- **No warning.** The controller rules on warning tiers, so this is a
  recommendation and not a fait accompli: a warning here would fire on
  `listing purecode`, which is what every correct Sonic 1 build writes. A check
  that fires on correct code is not the safe direction; it trains readers to
  ignore the stream it lives in. If a listing file ever exists, `listing off`
  acquires a real meaning and gets real semantics then.

## Group B: `[count]value`

**It died in the LEXER, not in `operands.rs`.** `[` and `]` were absent from
`punct`'s alphabet outright, so any line carrying one failed at
`unexpected character` before an operand parser saw it. `dc.ATTRIBUTE` was
already working: probe `d13` runs `MacroSetup.asm`'s exact macro through the
reference assembler and the listing shows `dcb.b 3,$FF` expanding to
`dc.b [3]$FF`, and sigil now assembles the same shape. The brief's hypothesis
was right, and its stated uncertainty about `ATTRIBUTE` resolves in the
convenient direction, which is itself a reason it was checked rather than
assumed.

### The semantics, each with its listing

Full listings and per-probe verdicts are in
`docs/superpowers/notes/2026-09-09-as-dup-operand-probes/README.md`; the probe
sources are committed beside it, one construct per file so that an error in one
cannot poison another's byte column.

| established | probe | asl exit | evidence |
|---|---|---|---|
| repeats the value at each width | `d1` | 0 | `[3]$FF` -> `FF FF FF`; `[2]$1234` -> `1234 1234`; `[2]$AABBCCDD` -> the long twice |
| the count advances `$` | `d1` | 0 | labels land at `$1003`, `$1008`, `$1010` from an origin of `$1000` |
| the group is ONE operand's prefix | `d2` | 0 | `$01,[3]$FF,$02` -> `01 FF FF FF 02` |
| the count is any constant expression | `d3` | 0 | `[1+2]`, `[(2*2)]`, `[n]`, `[n-1]` with `n equ 4` |
| whitespace is immaterial | `d4` | 0 | `[ 3 ]$AA`, `[3] $BB`, `[ 2 ] $CC` |
| `[0]` emits nothing, `$` unmoved | `d5` | 0 | the byte after stays at `$1001` |
| a forward count is refused | `d6` | 2 | `error #1820: expression must be evaluatable in first pass` |
| a string value repeats whole | `d7` | 0 | `[2]"ab"` -> `61 62 61 62` |
| only a LEADING bracket is a count | `d9` | 2 | `[2][3]$FF` is `error #1010` naming the SYMBOL `[3]$FF` |
| a negative count is refused | `d10` | 2 | `error #1920: code overflow` |
| `ds` does not take the group | `d11` | 2 | `error #1820` |
| an instruction operand does not | `d12` | 2 | `error #1010` naming `[2]1` |
| a missing value emits zeros | `d15` | 0 | `dc.b [3]` -> `00 00 00` |

**Two things the brief and the corpus both got wrong, found by probing rather
than assuming.**

`asl has no `dcb` builtin in this build.` Probe `d8`: `dcb.b 3,$FF` is
`error #1200: unknown instruction DCB`. Sonic 1's `dcb` macro therefore shadows
nothing, and sigil needs no `dcb` directive. (`scan_plain_labels` lists `dcb`
among the names that are not labels in the name column, which is harmless here
because the one corpus occurrence is a macro definition.)

`MacroSetup.asm`'s `org0` macro chunks its fill at 1024 with the comment "AS can
only generate 1 kb of code on a single line". Probe `d17`: 1024, 1025, 1596 and
`$62A` all assemble on one line with exit 0. The comment does not bind this
build, which is just as well, since `sonic.asm` writes `dcb.b $62A,$FF` (1578
bytes) five lines at a time.

### The implementation

`[` and `]` join `Punct` and the lexer's one-char table, and NOTHING ELSE gained
a meaning for them. The lexer's alphabet was checked before widening: no
construct in this front end used either character, and the corpus's only other
occurrences are inside comments. A bracket outside a leading `dc`-operand
position is still refused, which is a pinned test rather than a claim.

`split_top_commas` now counts bracket depth alongside paren depth, so a count
containing a comma (`[f(1,2)]$FF`) is one operand rather than two.

`Asm::dup_expanded_groups` peels the group per operand ahead of the existing
per-operand loop in `directive_db` / `directive_dc_w` / `directive_dc_l`,
returning `None` (and so allocating nothing extra) for the overwhelming majority
of lines that carry no bracket at all. Expanding at the operand level rather
than restructuring the emit loops is what makes `[0]`, string values and
mixed comma lists fall out for free instead of each needing its own arm.

### Four deliberate divergences from asl

Each is pinned as a test, so reversing one has to be deliberate.

1. `dc.b [3]` with no value: asl emits `00 00 00` (`d15`), sigil refuses by
   name. An empty operand meaning zero is a rule `dc` does not implement here,
   and inheriting it through the bracket path would let a truncated line
   assemble.
2. A forward-referenced count: asl refuses (`d6`), sigil resolves it, because
   sigil assembles to convergence rather than in one pass. Pinned as BYTES, so
   what is guarded is the value and not merely the acceptance.
3. The `listing` argument vocabulary, above.
4. `db` and `dc.b` are one directive in sigil on both CPUs, so sigil takes the
   group on either spelling. asl gates the whole SPELLING by CPU: `dc.*` does
   not exist under `cpu z80` (`d14`) and `db`/`dw` do not exist under
   `cpu 68000` (`d16`), which is why `db [3]v` is refused there. The alias
   predates this work; a bracket-only gate would imitate half a rule sigil does
   not implement, so the alias is left total and the divergence recorded.

## The defect this work introduced and then caught

The first draft answered an unfoldable count by dropping the operand. `eval_all`
returns `None` for a poisoned expression WITHOUT a diagnostic of its own, so
`dc.b [nosuchsym]$AA` assembled to zero bytes and exited 0: an assembly silently
short by however many bytes the count was worth, with nothing said. Named now,
the way `directive_ds` names its own count. Mutation M3 below is that defect
put back, and it measures exactly that shape.

## Verification

### Red-first mutations

Each was applied to the committed tree at `248a5c72`, shown on disk, run, and
restored with `git checkout HEAD -- <path>` from that commit. The prediction was
written before each run.

| # | mutation | predicted | measured |
|---|---|---|---|
| M1 | delete the lexer's `[`/`]` arms | every bracket row red, the listing rows green | 11 failed, 5 passed, exactly the split predicted |
| M2 | remove `listing`/`page` from `is_op_keyword` ONLY, keeping the dispatch arm | ONLY the column-0 row red | 1 failed, 15 passed, and the failure is `` `purecode` is not a recognized 68000 mnemonic `` |
| M3 | drop the `unresolved duplicate count` diagnostic | only the unresolvable row red, and red by ASSEMBLING rather than by a different message | 1 failed: `assembled to 0 byte(s) [] instead of refusing` |
| M4 | peel the bracket but ignore the count, emitting the value once | the byte rows red, the refusal and listing rows green | 8 failed, 8 passed, exactly the split predicted |

M2 and M4 are the two that could have come out green and meant something bad.
A green M2 would have made the `is_op_keyword` edit dead code and the column-0
row worthless; a green M4 would have shown the byte rows proving only that the
bracket LEXES, not that the count is honoured. Neither did.

M2 also corrected a prediction. The commit message for `248a5c72` says a
dispatch-only fix binds a label "emitting nothing, with no diagnostic". Measured,
it binds the label AND reports the ARGUMENT as an unrecognised mnemonic, so the
defect is a wrong diagnostic on correct source rather than a silent one. The
silent shape needs a column-0 `listing` with no argument, which no corpus writes.
The test's own comment carries the measured wording.

### Suites

`CARGO_TARGET_DIR=.target-s1dir cargo test --release --workspace --no-fail-fast`

- Strict (no `AEON_DIR`): 4518 passed, 382 failed, 2 ignored. **All 382 are the
  harness refusing to measure a reference-dependent row because no aeon tree is
  named**, which this parcel's brief forbids naming (379 carry the
  `NO REFERENCE TREE IS NAMED` refusal verbatim; the other 3 are `PoisonError`
  collateral from a sibling in the same test binary panicking on it). None is a
  golden divergence.
- With `SIGIL_ALLOW_PARTIAL=1`, the sanctioned declaration: **4900 passed, 0
  failed, 2 ignored.** Read that honestly: 382 of those 4900 rows print `ok`
  with `0 measured` because they were declared unmeasured, not because anything
  compared. The measured result is 4518 green, 0 red.

`crates/sigil-frontend-as/tests/as_dup_operand_and_listing.rs`, 16 tests, all
passing, wired into the normal `cargo test` workspace run.

## Still open

- **The aeon byte gate is owed and was not run here.** `sigil-frontend-as` is on
  aeon's shipping build path, and this parcel changes the lexer's alphabet and
  three `dc` directives. The brief assigns that proof to the controller at
  landing on the merged tree. Nothing in this parcel measured it.
- **s2disasm was not re-measured.** The other corpus lives in a shared, dirty
  checkout this parcel was told not to touch, and no prepared private copy was
  available. The change is additive at the lexer and no-op for any line without
  a bracket, but that is an argument and not a measurement.
- **A hostile count allocates.** The count is bounded by the 32-bit address
  space, which is the bound the emit needs anyway, but the expansion materialises
  the value's tokens per repetition, so a deliberately enormous count costs a
  constant factor more memory than the bytes it asks for. The corpus maximum is
  1596. Left as it stands rather than invented a tighter ceiling that no
  measurement supports.
- **The `charset` rows (9 of the remaining 25) are another parcel's.**
