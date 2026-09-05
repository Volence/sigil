# AS-ANDAND-PRECEDENCE: sigil's expression ladder was asl's in three tiers out of nine

Branch `parcel/as-andand-precedence`, off master `78c6134d`.

The row was opened on `&&` and `||` sitting at the wrong precedence relative to
the comparison operators. They were. So were the shifts, the bitwise operators
and `!`. The ladder is now measured end to end rather than patched at the point
that was noticed.

## 1. The derived asl order, and the probe that establishes each relation

Tightest first:

```
  <<  >>
  &
  |
  !                        (bitwise xor)
  *  /  #
  +  -
  &&
  ||
  !!                       (logical xor; sigil does not lex it, see section 9)
  =  <>  <  >  <=  >=
```

Every relation below is a `dc.b` from an `asl -L` listing that exited 0 with
`0 errors` and `0 warnings`, from asl 1.42 Beta [Bld 212],
`s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`. `s2disasm`'s build (md5
`0dee1f98e6480a4783d27ffd8b90896f`) was refused by the runner's guard and never
answered anything here.

**Why each probe can distinguish the answers.** Every probe was emitted as three
`dc.b` lines in the same listing: the bare `a op1 b op2 c`, and BOTH candidate
parenthesisations `(a op1 b) op2 c` and `a op1 (b op2 c)`. The listing therefore
shows, for each probe, the two answers the question could have had. A probe
whose two candidates print the same byte cannot distinguish the parses at all,
and `digest.py` prints `CONFOUNDED` for it rather than a verdict. That is the
mechanical form of "choose values where a wrong answer looks different from a
right one", and it is checked per probe rather than argued once.

| relation established | bare probe | `(a op1 b) op2 c` | `a op1 (b op2 c)` | asl | reading |
|---|---|---|---|---|---|
| `<<` tighter than `/` | `12/2<<1` | `0C` | `03` | `03` | `<<` |
| `<<` tighter than `#` | `12#5<<1` | `04` | `02` | `02` | `<<` |
| `<<` tighter than `+` | `1+1<<3` | `10` | `09` | `09` | `<<` |
| `>>` tighter than `+` | `1+8>>2` | `02` | `03` | `03` | `>>` |
| `<<` tighter than `-` | `8-1<<2` | `1C` | `04` | `04` | `<<` |
| `<<` and `>>` one tier, left-assoc | `8>>1<<2` | `10` | `00` | `10` | left |
| `<<` tighter than `&` | `1&3<<1` | `02` | `00` | `00` | `<<` |
| `>>` tighter than `&` | `1&3>>1` | `00` | `01` | `01` | `>>` |
| `<<` tighter than `|` | `1|3<<1` | `06` | `07` | `07` | `<<` |
| `<<` tighter than `!` | `1!3<<1` | `04` | `07` | `07` | `<<` |
| `&` tighter than `|` | `1|2&2` | `02` | `03` | `03` | `&` |
| `&` tighter than `|` (reverse) | `1&6|3` | `03` | `01` | `03` | `&` |
| `&` tighter than `!` | `1!3&2` | `02` | `03` | `03` | `&` |
| `|` tighter than `!` | `3!1|2` | `02` | `00` | `00` | `|` |
| `|` tighter than `!` (reverse) | `3|1!3` | `00` | `03` | `00` | `|` |
| `&` tighter than `*` | `3*2&5` | `04` | `00` | `00` | `&` |
| `|` tighter than `*` | `3*2|5` | `07` | `15` | `15` | `|` |
| `!` tighter than `*` | `3*2!5` | `03` | `15` | `15` | `!` |
| `!` tighter than `*` (reverse) | `3!2*2` | `02` | `07` | `02` | `!` |
| `&` tighter than `+` | `1+3&2` | `00` | `03` | `03` | `&` |
| `|` tighter than `+` | `1+3|4` | `04` | `08` | `08` | `|` |
| `!` tighter than `+` | `1+3!2` | `06` | `02` | `02` | `!` |
| `*` tighter than `+` | `6+2*3` | `18` | `0C` | `0C` | `*` |
| `*` `/` `#` one tier, left-assoc | `12/2*3` | `12` | `02` | `12` | left |
| `*` `/` one tier (reverse) | `12*2/3` | `08` | `00` | `08` | left |
| `#` `/` one tier | `12#5/2` | `01` | `00` | `01` | left |
| `#` `*` one tier | `7#5*2` | `04` | `07` | `04` | left |
| `+` tighter than `&&` | `1&&2+3` | `04` | `01` | `01` | `+` |
| `*` tighter than `&&` | `2&&3*4` | `04` | `01` | `01` | `*` |
| `&` tighter than `&&` | `1&&12&3` | `01` | `00` | `00` | `&` |
| `|` tighter than `&&` | `0&&8\|4` | `04` | `00` | `00` | `|` |
| `!` tighter than `&&` | `0&&8!4` | `04` | `00` | `00` | `!` |
| `<<` tighter than `&&` | `1&&1<<3` | `08` | `01` | `01` | `<<` |
| `&&` tighter than `=` | `1&&2=2` | `00` | `01` | `00` | `&&` |
| `&&` tighter than `=` (reverse) | `2=2&&1` | `01` | `00` | `00` | `&&` |
| `&&` tighter than `<>` | `7<>3&&0` | `00` | `01` | `01` | `&&` |
| `&&` tighter than `>` | `5>1&&0` | `00` | `01` | `01` | `&&` |
| `&&` tighter than `<=` | `0<=1&&0` | `00` | `01` | `01` | `&&` |
| `&&` tighter than `>=` | `0>=1&&0` | `00` | `01` | `01` | `&&` |
| `&&` tighter than `||` | `1\|\|0&&0` | `00` | `01` | `01` | `&&` |
| `&` tighter than `||` | `0\|\|12&3` | `01` | `00` | `00` | `&` |
| `|` tighter than `||` | `0\|\|8\|4` | `05` | `01` | `01` | `|` |
| `!` tighter than `||` | `0\|\|8!4` | `05` | `01` | `01` | `!` |
| `<<` tighter than `||` | `1\|\|1<<3` | `08` | `01` | `01` | `<<` |
| `+` tighter than `||` | `1\|\|2+3` | `04` | `01` | `01` | `+` |
| `||` tighter than `=` | `1\|\|2=2` | `00` | `01` | `00` | `||` |
| `||` tighter than `=` (reverse) | `2=2\|\|0` | `01` | `00` | `00` | `||` |
| `+` tighter than `=` | `4=1+1` | `01` | `00` | `00` | `+` |
| `&` tighter than `=` | `6&2=2` | `01` | `00` | `01` | `&` |
| `<<` tighter than `=` | `6<<1=12` | `01` | `06` | `01` | `<<` |
| `!` tighter than `=` | `3!1=2` | `01` | `03` | `01` | `!` |
| `!` tighter than `=` (reverse) | `1=3!2` | `02` | `01` | `01` | `!` |
| comparisons one tier, left-assoc | `1<2=1` | `01` | `00` | `01` | left |
| comparisons one tier (reverse) | `2=1<2` | `01` | `00` | `01` | left |

**Values, as distinct from tiers.** `&&` and `||` are NORMALISING logical
operators, not bitwise ones: `6&&3` = `01` (not `02`), `4&&2` = `01` (not `00`),
`4||2` = `01` (not `06`). This matters because a bitwise reading of `&&` explains
three of the four divergent rows the parcel was opened on just as well as the
logical reading does, so those rows are confounded on SEMANTICS even where they
discriminate on PRECEDENCE. `6&&3` and `4&&2` are the probes that separate them,
and they are in the golden listing for that reason.

**The confounded probes, excluded from the derivation.** `3*1&&0`, `1<5&&0` and
`0<1&&0` fold to `00` under both parses. `1&&0&&1` and `0||1||0` cannot show
grouping because logical and/or are associative. `A>4&&B<5`, one of the rows in
the brief, folds to `1` under asl's ladder AND under the C ladder sigil shipped:
it agreed with both and was evidence for neither. It is kept in the test as a
labelled control.

## 2. Tier by tier against ours

| tier (asl, tightest first) | asl | sigil before | verdict |
|---|---|---|---|
| 1 | `<<` `>>` | `*` `/` `#` | MOVED |
| 2 | `&` | `+` `-` | MOVED |
| 3 | `|` | `<<` `>>` | MOVED |
| 4 | `!` | `&` | MOVED |
| 5 | `*` `/` `#` | `|` `!` | MOVED, and `!` was fused to `|` |
| 6 | `+` `-` | `=` `<>` `<` `>` `<=` `>=` | MOVED |
| 7 | `&&` | `&&` | MOVED |
| 8 | `||` | `||` | MOVED |
| 9 | `=` `<>` `<` `>` `<=` `>=` | (loosest was `||`) | MOVED |

Stated as changes rather than as a table: the shifts and the three bitwise
operators bind TIGHTER than `*` and `+` in asl and bound looser in sigil; `!` is
its own tier looser than `|` and shared `|`'s tier in sigil; and the comparisons
are the LOOSEST tier and were a middle one in sigil, above `&&` and `||`.

Only the relative order WITHIN `{&, |, !}` and within `{*, /, #}` and within the
comparisons survived unchanged, and the `!` split means even the first of those
is only half-survived.

Consequences on concrete expressions, all from the golden listing:
`1+1<<3` folded to `$10` and asl folds `09`; `3!1|2` folded to `02` and asl folds
`00`; `3*2|5` folded to `07` and asl folds `15`; `8-1<<2` folded to `$1C` and asl
folds `04`.

## 3. The doc comment: what it actually claimed, and the verdict

The comment above `infix_bp` read:

> `||` is loosest, `&&` binds tighter than `||` but looser than comparisons,
> mirroring AS's real operator surface (empirically confirmed against `asl`:
> both fold to a neutral `1`/`0`, same as the comparison tier).

**Verdict: the narrow reading is the right one, and the comment was still
wrong.** The parenthetical attaches to what follows the colon, and what follows
the colon is a claim about VALUES: that `&&` and `||` fold to a neutral `1`/`0`.
That claim is TRUE and I re-measured it (`6&&3`=`01`, `4&&2`=`01`, `4||2`=`01`).
The precedence claim in the sentence before it, that `&&` is looser than
comparisons, is FALSE and was never what the parenthetical certified.

So this is not a case of a measurement having been done wrong. It is a case of a
verified claim and an unverified one sharing a sentence, with the word
"empirically confirmed" sitting close enough to lend its authority to both. The
reader who wrote it knew which half was measured. Every reader since could not
tell, and the sentence gave them no reason to suspect there was a division.

Corrected to state the ladder as a table, to say that every tier boundary in it
is measured with a pointer to the probes and the test, and to put the neutral
`1`/`0` claim in its own paragraph explicitly labelled as a claim about values
and independent of the tiers. The perishable part now has a named falsifier
(`tests/as_operator_precedence.rs`) rather than an adjective.

## 4. Affected population, and the engagement witness

**Zero, and it is inertness rather than safety.**

Measured by instrument rather than by reading. A temporary build parsed every
expression with BOTH ladders and reported any whose trees differed. Numbers:

| root | expressions parsed | trees that differ |
|---|---|---|
| `s2disasm` `s2.asm` at `e45ebf33` | 932,452 | 0 |
| `aeon/games/sonic4/game_root.asm` | 4 | 0 |
| `aeon/games/demo/game_root.asm` | 4 | 0 |
| `aeon/engine/debug/debugger.asm` | 140 | 0 |

**Engagement witness.** A zero from an instrument that never ran looks exactly
like a zero from an instrument that ran and found nothing, so the same
instrumented binary was pointed at a file built to diverge
(`dc.b 1&&2=2`, `1+1<<3`, `3!1|2`): 8 expressions parsed, 6 divergences reported.
It fires.

**Second witness, on the shipped binaries rather than the instrument.** The
before and after release binaries assemble that same file to `01 10 02` and
`00 09 00` respectively, and asl assembles it to `00 09 00`. The two binaries are
demonstrably different programs on this parcel's own subject, so the corpus
identity in section 6 is a statement about the corpus and not about a build that
did not happen.

**The static reading agrees.** Over tracked corpus source, 37 lines use `&&` or
`||` outside a comment (the raw grep count is dominated by `; ||||| SUBROUTINE
|||||` banners in `s2.sounddriver.asm`, which is why a count of matches is not a
count of uses). Every one of the 37 fully parenthesises both operands of the
logical operator, except `s2.asm:91262`
(`if padToPowerOfTwo && (*-StartOfRom)&(*-StartOfRom-1)`), whose right operand is
a bare `&` expression, and `&` binds tighter than `&&` under BOTH ladders, so its
grouping does not move.

**Instrument hygiene.** `grep` on this machine is a shell function whose `-r`
mode skips gitignored files and returns a clean empty result, so an emptiness
from it is not a finding. The divergence counts above were taken with
`/usr/bin/grep` by absolute path (the `.err` files are generated artifacts), and
cross-checked against the shell `grep`, which agreed. Before believing the zero
I planted a `LADDER-DIVERGE` line in a copy of the 5,247-line corpus `.err` and
confirmed both greps returned 1 on it. The corpus source counts were taken with
`git grep` over tracked files, with a positive control (`cpu` in `*.asm` returns
6, not 0) proving the pathspec matched files at all. My first corpus count, before
this check, used `grep -rn --include='*.asm'` and printed
`ugrep: warning: --include=*.asm: No such file or directory`: that number (408)
is withdrawn and is not used anywhere above.

## 5. The typed evaluator

`eval.rs::parse_num_bp` calls `crate::expr::infix_bp` directly rather than
copying it, so the change lands there by construction. That is what the old
comment asserted, and asserting it is not checking it.

Checked: six `int(...)` rows are in the golden test, and all six were WRONG
before the change and are right after it, with no edit to `eval.rs`.

| row | asl | sigil before | sigil after |
|---|---|---|---|
| `int(1&&2=2)` | `00` | `01` | `00` |
| `int(1+1<<3)` | `09` | `10` | `09` |
| `int(3!1\|2)` | `00` | `02` | `00` |
| `int(2=2&&1)` | `00` | `01` | `00` |
| `int(1\|\|2=2)` | `00` | `01` | `00` |
| `int(1.0+1<<3)` | `09` | REFUSED | `09` |

The last row is the interesting one and was not predicted. Under the old ladder
`1.0+1<<3` parses as `(1.0+1)<<3`, which shifts a FLOAT, and sigil declined the
whole program with `int(): could not evaluate float expression`. asl parses it as
`1.0+(1<<3)` and assembles it. So the wrong ladder was not only folding integers
wrongly, it was refusing at least one shape of program that asl accepts. A
precedence defect reaching the diagnostic surface is not something I would have
looked for.

## 6. Corpus and aeon

Corpus: `s2disasm` detached worktree at `e45ebf33`, run-unique path
`/home/volence/sonic_hacks/s2disasm-andand-a3c8`, 0 dirty paths. The owner's live
checkout was not written to. Runner: `corpus.sh` beside this note.

```
before md5 c0c16133e41296869b316334010e2412
after  md5 c931a3efe1f560ea917c3f4471547d95

before exit=1  stdout lines=0  stderr lines=5247
after  exit=1  stdout lines=0  stderr lines=5247
stdout IDENTICAL
located vs locationless, both runs: total=5247 with file(line)=5247 without=0
```

Neither run writes an output binary: the corpus does not assemble to completion
today, which is the pre-existing baseline, so the diagnostics are the gate.

Class decomposition, top rows (full table in the runner's output):

```
  level    before   after   delta  class
  error      2624    2624      +0  bad operand expression
  error      2309    2309      +0  expected mnemonic, directive, or label
  error        89      89      +0  `X` is not a recognized N mnemonic
  error        49      49      +0  bad word expression
  error        30      30      +0  bad byte expression
  ...
             5247    5247      +0  TOTAL
```

* Classes that ROSE: none. Classes that APPEARED: none.
* Diagnostic lines present in AFTER and absent from BEFORE: 0.
* Diagnostic lines present in BEFORE and absent from AFTER: 0.
* Unresolved-symbol name sets: before-only 0, after-only 0, in both 8.

Aeon, the three AS-routed roots, run and compared rather than assumed:

```
  before sonic4-game_root: exit=0 stdout=0 stderr=0 bytes=0
  after  sonic4-game_root: exit=0 stdout=0 stderr=0 bytes=0   .bin/.out/.err IDENTICAL
  before demo-game_root:   exit=0 stdout=0 stderr=0 bytes=0
  after  demo-game_root:   exit=0 stdout=0 stderr=0 bytes=0   .bin/.out/.err IDENTICAL
  before debug-debugger:   exit=1 stdout=0 stderr=23 bytes=none
  after  debug-debugger:   exit=1 stdout=0 stderr=23 bytes=none  .out/.err IDENTICAL
```

`debugger.asm` invoked as a root exits 1 with 23 diagnostics in BOTH runs; it is a
macro library and this is its pre-existing standalone behaviour, not something
this parcel moved. No aeon build was run and `AEON_DIR` was not set.

One correction to the runner, made rather than annotated: its first version
compared the three files with `cmp -s` and printed `DIFFERS` for
`debug-debugger.bin`, which neither run produces. `cmp -s` exits 2 on a MISSING
file, and the script read any non-zero as "they differ". That is a check that
fires on correct code, which trains its reader to discount it. It now
distinguishes absent-in-both from present-in-one from genuinely different.

## 7. Red-first

The mutation is `git checkout 78c6134d -- crates/sigil-frontend-as/src/expr.rs`,
restoring the ladder from a COMMITTED baseline.

Proof it landed on disk, since that form STAGES and so `git diff --stat` reports
nothing:

```
git diff --stat            (empty, as expected, and this is why it is not the check)
git diff HEAD --stat       crates/sigil-frontend-as/src/expr.rs | 97 ++---
                           1 file changed, 30 insertions(+), 67 deletions(-)
content grep on disk:      Shl => (6, ...)   Bang => (4, ...)   AndAnd => (2, ...)
md5 on disk                8eca5ac4ebb795d24741d3196811bc96
md5 of 78c6134d's copy     8eca5ac4ebb795d24741d3196811bc96
```

With it applied: `FAILED. 0 passed; 1 failed`, reporting
`36 of 89 expressions fold differently from asl`. Restored from `HEAD`, re-run:
`ok. 1 passed; 0 failed`.

What the test MUST fail on: the C-style ladder that shipped; any ladder leaving
the shifts below `+`; any giving `!` the same tier as `|`; any putting the
comparisons above `&&` or `||`. The 36 red rows span all four.

## 8. What did not execute

* **The byte-identity golden gates DID NOT RUN.** They were skipped, not passed.
  The run was `SIGIL_ALLOW_PARTIAL=1 cargo test --workspace --no-fail-fast` with
  no reference tree named. The harness's own count, quoted verbatim from
  `reference_dependence_is_named`: **"127 test binaries are reference-dependent
  and every row in them will SKIP. A green result from this run does NOT mean
  those rows passed, it means they were not run."** That message also says the run
  is not a landing and that `SIGIL_STRICT_GATE=1` turns the skips into named
  failures. Nothing in this note should be read as evidence that the Aeon ROM
  still builds byte-identically. **That is the single largest thing this parcel
  did not measure, and given that the change moves five tiers of the expression
  ladder it is the measurement I would most want taken before merge.**
* Suite totals actually taken: **406 test binaries, 4614 passed, 0 failed,
  2 ignored, exit 0.** No failing names, because there were none.
* No emulator was touched, per the standing invariant. Nothing here needs runtime
  confirmation, since every claim is about assembly-time folding.
* No aeon build was run.

## 9. Gap-ledger items found and not closed

* **`!!`, AS's logical XOR, is not lexed by sigil.** asl has it (`1!!0`=`01`,
  `5!!3`=`00`) and it sits between `||` and the comparisons: `2=2!!0` folds to
  `00`, so `!!` binds tighter than `=`; `1!!0&&0` folds to `01`, so `&&` binds
  tighter than `!!`; `1!!1||1` folds to `00`, so `||` binds tighter than `!!`.
  Sigil's lexer has no `BangBang`, so `a!!b` would lex as two `Bang` tokens and
  fail to parse. It appears nowhere in the corpus outside comments (checked with
  `git grep` over tracked source), so this is a latent hole and not a live defect.
  Left open deliberately: adding an operator is a surface change, and the ladder
  slot it would take is now measured and recorded here for whoever adds it.
* `~~` (prefix logical not) IS supported and unaffected; it appears in the corpus
  (`s2.macrosetup.asm:245`, `s2.sounddriver.asm` passim) and its atom-tier binding
  was already asl-verified in 2026-09-03.

## 10. Anything in this brief you concluded was wrong

1. **"Same silent-wrong-answer class this lane has closed four times today" was
   an understatement of the row, not a description of it.** The brief scoped the
   defect to `&&`/`||` against comparisons. That was one of five misplaced tiers.
   The shifts and the bitwise operators were also on the wrong side of `*` and
   `+`, and `!` was fused to `|`'s tier. A patch to the pair named in the row
   title would have left `1+1<<3` folding to 16 against asl's 9, in a frontend
   where `x<<n-1` is an ordinary thing to write. The brief's instruction to derive
   the full order is what caught this, so the instruction was right and the row
   title was wrong.

2. **The brief's framing of the two divergent rows as "pointing opposite ways"
   and therefore "not a simple shift one tier story" was right about the
   conclusion and wrong about the reasoning.** The two rows differ in direction
   because their operand values differ, not because the tier displacement differs.
   `A=6&&C<>3` and `(K*2)=6&&(J<>3)` are both explained by the SAME single fact,
   `&&` binding tighter than the comparisons, applied to different numbers. Had I
   taken "opposite directions implies a complex displacement" as a premise I would
   have gone looking for a second mechanism that does not exist. The correct
   reason not to assume a simple story is that no probe had yet ruled out the
   alternatives, which is a different reason and the one that generalises.

3. **Three of the four rows in the brief's table cannot distinguish logical `&&`
   from BITWISE `&&`.** `A=6&&C<>3`, `A=6||C=3` and `(K*2)=6&&(J<>3)` give the
   same bytes under a bitwise reading of the logical operators as under the
   normalising one. The brief presented them as establishing a precedence fact,
   which they do, but a reader could take them as establishing the operator's
   meaning as well, which they do not. `6&&3` and `4&&2` were added for that.

4. **`A>4&&B<5` is not an "agree" row, it is a confound.** The brief's table marks
   it `agree`, in the same column and typeface as `A&B=2` and `A<<1=12`, which are
   genuine agreements: those two discriminate between the ladders and sigil got
   them right. `A>4&&B<5` folds to `1` under both ladders and could not have come
   out any other way. Filed under "agree" it reads as one more piece of evidence
   that the non-logical part of the ladder was sound; it is zero evidence about
   anything. It is now a labelled control in the test.

5. **"Both fold to a neutral 1/0, same as the comparison tier" is doing two jobs
   in the brief as well as in the code comment.** The brief quotes it while
   raising the question of what the parenthetical certifies, which is the right
   question. What it does not flag is that the phrase "same as the comparison
   tier" is itself ambiguous between "the same VALUES a comparison yields" (true)
   and "the same TIER comparisons occupy" (false, and the very error under
   investigation). The sentence contains the wrong answer twice, once per reading.

6. **The brief's claim that `.emp` does not share this parser is correct, and I
   verified it, but the reason given understates the check.** `sigil-frontend-emp`
   has no dependency on `sigil-frontend-as` and carries its own evaluator, so the
   change cannot move the game's language. Confirmed. What the brief did not say
   is that the sharing that DOES exist, `eval.rs::parse_num_bp` reaching into
   `expr::infix_bp`, is not merely a second consumer of the same numbers: it has a
   different value domain, and the ladder change moved one of its rows from a
   wrong ANSWER to a wrong REFUSAL and back (section 5). "The change lands in both
   by construction" was true and was not the whole effect.

7. **The population instruction pointed at the wrong population, though it
   pointed at it for the right reason.** The brief asked for "an unparenthesised
   comparison adjacent to a logical operator". Once four more tiers turned out to
   be wrong, that predicate stopped covering the defect: an unparenthesised `+`
   adjacent to a `<<` is equally affected and matches no part of that description.
   I measured tree-shape divergence under both ladders instead, which is
   value-independent and covers every tier at once. The brief's underlying
   instruction, measure the affected subset rather than quoting 398, is what made
   me build the instrument.

8. **The 398 figure and my own 408 both need retiring, for different reasons.**
   The brief's 398 came from `git grep` over tracked source and is a sound count
   of LINES CONTAINING the tokens; it is just not a count of uses, because
   `s2.sounddriver.asm`'s ASCII subroutine banners are made of `|` characters. My
   first recount returned 408 from an instrument that printed a pathspec warning
   and is withdrawn entirely. The number that answers the question is 37.
