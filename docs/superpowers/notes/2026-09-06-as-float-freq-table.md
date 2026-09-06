# AS-FLOAT-FREQ-TABLE: the count, the place and the mechanism were all wrong

Parcel note. Branch `parcel/as-float-freq-table`.

## Provenance

| | |
|---|---|
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, `Macro Assembler 1.42 Beta [Bld 212] (x86_64-unknown-linux)`, md5 `61e672562465725a8c102288a7da9098` |
| invocation | `asl_run -xx -n -q -A -L -U -i .`, through `docs/superpowers/notes/asl-reference/asl_ref.sh`, run from inside the probe directory with a BARE filename |
| probes | `2026-09-06-as-float-freq-table-probes/` beside this file, with `run.sh` |
| corpora | `s2disasm` at `e45ebf33` and `s1disasm` at `f6ece657`, each in its own detached worktree; the live checkouts are never written |
| sigil | this branch, built into `.target-land` inside this worktree; the master baseline binary into `.target-master`, both on disk, neither under `/tmp` |

`s2disasm` ships its own `asl` (the flamewing fork, md5 `0dee1f98...`). It is refused
by the guard and no value here came from it.

**Which listings may be quoted.** Only `values.asm`, `values2.asm`, `clean2.asm`,
`clean3.asm` and `clean4.asm` exit 0. `names.asm`, `names2.asm`, `names3.asm`,
`types.asm`, `domain.asm` and `bigint.asm` exit 2 BY DESIGN and are read for their
diagnostics only: an error stops asl's pass loop, so a failed run's byte column can
carry an unresolved pass-1 placeholder that looks complete.

## The corrected fault statement

The board row read: *"84 complaints, seven octaves of twelve notes, from a frequency
table computed with floating-point arithmetic we cannot fold."* The dispatching brief
corrected it to 23 at two sites. **Both are wrong, in different ways.**

| | count | sites | mechanism |
|---|---|---|---|
| the row | 84 | "a note frequency table" | float arithmetic cannot be folded |
| the brief | 23 | 2 | something narrower than folding |
| **measured** | **6** | **1** | **`log()` was not implemented** |

* **84 is the table's SIZE, not a diagnostic count.** Seven times twelve. No frequency
  table is involved in any of this.
* **Float arithmetic was never the problem.** `int(2.5*4)` already gave `0A` on master,
  matching asl. The brief had that right.
* **There is no note frequency table anywhere near it.** The one real site is a HUD
  digit counter.

The whole defect, on `s2disasm` `e45ebf33` with the corpus in the state its own build
assembles it:

```
6  s2.asm(87677)   .loop_counter = int(log(number))   ; Total digits minus one.
```

`hud_counter` is invoked six times (`Hud_100000` down to `Hud_1`), and `asl`'s `log`
builtin did not exist in sigil.

## Site 2 is not a float gap at all

`s2.sounddriver.asm(3905)` is

```
	db	id(label.pointer),dpcmLoopCounter(int(label.sample_rate*sample_rate_scale))
```

and it produced 17 diagnostics. The brief said it could not reproduce them from a
reduction and that isolating them was the first piece of work. **They are not a float
fault. They are the downstream shadow of seven missing files.**

`label.sample_rate` is defined nowhere in the tracked corpus. It lives in
`sound/DAC/generated/*.inc`, which `build.lua` writes at build time from the `.wav`
sources:

```
$ cat sound/DAC/generated/Kick.inc
.sample_rate = 8250
.size = 660
	binclude "sound/DAC/generated/Kick.dpcm"
```

A bare checkout has none of them, so the operand is undefined and `int()` cannot
evaluate. Running only the generation half of `build.lua` (its first 186 lines, which
stop before the ROM build) and re-measuring, with NO sigil change whatsoever:

| | bare checkout | generated present |
|---|---|---|
| diagnostics | 5,229 | 5,168 |
| `cannot include` | 39 | 0 |
| `could not evaluate float` | 23 | **6** |

17 is exactly the number of `dac_sample_metadata` call sites (18 occurrences of the
name, minus the macro definition). The same class is independently visible in Sonic 1:
`s1.sounddriver.asm(337)` writes
`timpaniLoopCounter function scale,dpcmLoopCounter(int(zDAC_Timpani.sample_rate*scale))`
and reports 4, three lines below three `cannot include sound/dac/dpcm/generated/*.inc`
diagnostics. `2026-09-03-s1-corpus-baseline.md` recorded the same pairing a
fortnight ago and it went unconnected to this row.

**Why the brief's reduction agreed at `64` and told it nothing.** It substituted an
integer for `label.sample_rate`. The real construct does not fail on the arithmetic; it
fails because the symbol does not exist. A reduction that supplies the missing thing
cannot reproduce a fault caused by its absence.

## What was actually missing, and what it now is

sigil's typed evaluator (`crates/sigil-frontend-as/src/eval.rs`) knew exactly two
function names: `sin` and `int`. This build of asl knows at least the twenty-four below
(the census asked about thirty-five candidate names; it is a lower bound, not asl's
full table). Read only for which names draw `error #1860: unknown function`:

| known | unknown |
|---|---|
| `int` `abs` `sin` `cos` `tan` `asin` `acos` `atan` `sinh` `cosh` `tanh` `asinh` `acosh` `atanh` `sqrt` `exp` `log` `ln` `sgn` `bitcnt` `firstbit` `lastbit` `bitpos` `toupper` | `log10` `arcsin` `arctan` `exp2` `pow` `frac` `trunc` `round` `floor` `ceil` `bogusfn` |

This parcel lands the sixteen single-argument NUMERIC ones. The integer-valued family
(`sgn`, `bitcnt`, `firstbit`, `lastbit`, `bitpos`) is censused and deliberately left; it
is a different parcel and is in the gap ledger.

## Every function's base and rounding, with the evidence

All values from `values2.asm`, `ASL_EXIT=0`, `ASL_DIAG=complete`, listing committed
beside the source. `1e6`-style exponent literals appear in `values.asm` only; `values2`
respells them as `1000000` so the IDENTICAL source text goes to both assemblers.

| written | asl | sigil now | base / rounding |
|---|---|---|---|
| `INT(LOG(1))` .. `INT(LOG(100000))` | `0 1 2 3 4 5` | same | `log` is **base 10** |
| `INT(LN(100))` | `4` | same | `ln` is **natural** |
| `INT(LN(1000))` | `6` | same | natural |
| `INT((LOG(10^k)-k)*1e15)`, k=0..5 | `0` at every k | n/a | `log` is an **exact `log10`** |
| `INT(EXP(2)*1000000)` | `7389056` | same | `exp` is **e^x** |
| `INT(SQRT(2)*1000000)` | `1414213` | same | |
| `INT(SIN(1)*1000000)` | `841470` | same | **radians** |
| `INT(COS(1)*1000000)` | `540302` | same | radians |
| `INT(TAN(1)*1000000)` | `1557407` | same | radians |
| `INT(ATAN(1)*1000000)` | `785398` | same | radians |
| `INT(ASIN(1)*1000000)` | `1570796` | same | radians |
| `INT(ACOS(0)*1000000)` | `1570796` | same | radians |
| `INT(SINH(1)*1000000)` | `1175201` | same | |
| `INT(COSH(1)*1000000)` | `1543080` | same | |
| `INT(TANH(1)*1000000)` | `761594` | same | |
| `INT(ASINH(1)*1000000)` | `881373` | same | |
| `INT(ACOSH(2)*1000000)` | `1316957` | same | |
| `INT(ATANH(0.5)*1000000)` | `549306` | same | |
| `INT(ABS(-3.25)*1000000)` | `3250000` | same | `abs` on a float |
| `dc.l ABS(-3)` | `3`, no error | same | `abs` is **type-preserving** |

Every one of those is `std` `f64`'s answer for the same expression, computed
independently in Python before the asl run and matching digit for digit. That was not
assumed: `1e17+1-1e17` = 0 in an earlier parcel already pinned the float type as IEEE
binary64 rather than x87 80-bit extended.

**Rounding.** `int()` FLOORS toward negative infinity in every case, including through
the new functions. See the negative section below.

**Type.** Every function in the table except `abs` and `int` returns a FLOAT even when
the value is integral: `dc.l LOG(100)` and `dc.l SQRT(16)` are each `error #1133:
expected integer or string, but got floating point number`, and `dc.l LOG(100)&1` is
`#1134` (`types.asm`, exit 2, diagnostics only). `abs` preserves the argument's type,
which is why `dc.l ABS(-3)` assembles.

**Case.** Builtin names match case-insensitively even under `-U`, which makes user
symbols case-sensitive. `INT(log(1000))`, `INT(Log(1000))` and `INT(lOg(1000))` all give
`3` (`clean2.asm`, exit 0). The corpus writes lower case; every probe here writes upper.

**Domain.** asl DIAGNOSES rather than producing a NaN or an infinity (`domain.asm`,
exit 2, diagnostics only): `LOG(0)`, `LOG(-1)`, `SQRT(-1)` and `ASIN(2)` draw `error
#1870: function argument out of definition range`; `ATANH(2)`, `ACOSH(0)` and
`EXP(1000)` draw `#1880: floating point overflow`. All seven are exactly the arguments
where the corresponding `f64` method returns a non-finite value, so one `is_finite`
test reproduces all of them.

## Why each probe discriminates

Float is dense with values that separate nothing. Each row was chosen against a
specific wrong implementation.

| probe | separates | the wrong answer |
|---|---|---|
| `INT(LOG(100))` = 2 | base 10 from natural | `ln 100` = 4.605, floors to **4** |
| `INT(LN(100))` = 4 | shows the natural one EXISTS | without it, "log is base 10" could be an absence rather than a choice |
| `INT(LOG(1000))` = 3 | exact `log10` from `ln(x)/ln(10)` | the latter is **2.9999999999999996**, floors to **2** |
| `INT((LOG(1000)-3)*1e15)` = 0 | the same, read as a residual | the ULP-short form gives **-1** |
| `INT(EXP(2)*1e6)` | e^x from 2^x | `2^2*1e6` = **4000000**, a different digit count |
| `INT(SIN(1)*1e6)` = 841470 | radians from degrees | `sin(1 deg)*1e6` = **17452** |
| `INT(ATAN(1)*1e6)` = 785398 | radians from degrees | `atan(1)` in degrees is **45000000** |
| `INT(-3.2)` = -4 | floor from truncate-toward-zero | truncation gives **-3** |
| `INT(LOG(0.5))` = -1 | the same, through the new function | truncation gives **0** |
| `INT(LOG(0.5)*1e6)` = -301030 | the same, six digits in | truncation gives **-301029** |
| `ABS(-3)` in a `dc.l` | type-preserving from float-returning | a float `abs` makes this `error #1133` |
| `dc.l INT(1e30)-INT(1e30)` | whether INT refuses or clamps | value 0, in range for a `dc.l`, and asl still errors TWICE |

Rows deliberately NOT used, and why: `LOG(10)` = 1 is satisfied by several wrong
readings. `EXP(1)` floors to 2 under both e^x and 2^x. `INT(2.0)` distinguishes no
rounding mode from any other. `ACOS(1)` = 0 and `ASIN(0)` = 0 are zero under radians
and degrees alike.

**The `1e30` probe needed its shape checked, not just its value.** `dc.l INT(1e30)`
alone would leave "was that the INT or the `dc.l` complaining?" open, since the value
does not fit a long either way. `dc.l INT(1e30)-INT(1e30)` is 0, fits, and asl reports
the overflow twice on the one line, which places the refusal inside `INT`.

## The negative-value rounding result

**`int()` is floor, not truncation toward zero, and no positive argument can say so.**

```
      40/      30 : FFFF FFFC           	dc.l	INT(-3.2)
      41/      34 : FFFF FFFF           	dc.l	INT(LOG(0.5))
      42/      38 : FFFB 681A           	dc.l	INT(LOG(0.5)*1000000)
```

-4, -1, -301030. Truncation toward zero gives -3, 0, -301029. The third row is the
strongest of the three: `log10(0.5)` is -0.30102999566398120, so the product is
-301029.99566..., and the two rules differ in the last digit six digits into a value
the new function produced.

This confirms rather than discovers: `2026-09-04-f1-as-float-semantics.md` had already
minted `INT(-3.7)` = -4 and `INT(-3.0)` = -3. What is new is that the rule survives
composition with the functions this parcel adds, which a probe on literals alone could
not have said.

## What the corpus count became, and what measured the bytes

**The per-file corpus sweep compares zero bytes.** The dispatching brief carries that
measurement (0 of 1,828 files across four trees emit any; they are include fragments)
and this parcel did not re-derive it, so it is cited and not claimed. What follows is
therefore DIAGNOSTIC counts; the byte evidence is separate and is named below.

### Diagnostics

Sonic 2, `e45ebf33`, generated includes present, entry `s2.asm`:

| | master `0eb20272` | this branch |
|---|---|---|
| total | 5,168 | **5,162** |
| `could not evaluate float` | 6 | **0** |

A full sorted diff of the two diagnostic sets shows **six lines removed and zero added**.

Sonic 1, `f6ece657`, BARE checkout, entry `sonic.asm`, one column per step of the fix:

| | master | + `log` and the float table | + `abs` in integer contexts | + unary plus |
|---|---|---|---|---|
| total | 57 | 57 | 53 | **49** |
| `unresolved rept count` | 8 | 8 | 4 | **0** |
| `could not evaluate float` | 4 | 4 | 4 | 4 |

Sonic 1 with its generated includes present, which is the state its own build assembles:

| | master `0eb20272` | this branch |
|---|---|---|
| total | 50 | **42** |
| `could not evaluate float` | **0** | **0** |
| `cannot include` | 0 | 0 |

**That zero on master is the site-2 mechanism confirmed on a second corpus, and on the
side of it I did not tune.** This branch adds no `log` capability Sonic 1 uses; the four
float diagnostics in the bare column are all `s1.sounddriver.asm(337)`
(`dpcmLoopCounter(int(zDAC_Timpani.sample_rate*scale))`) and they vanish under the
UNCHANGED master binary the moment `sound/dac/dpcm/generated/timpani.inc` exists. The
seventeen at `s2.sounddriver.asm(3905)` are the same thing at a different scale.

### Bytes

**The landing run's shipped-shape CRC gates are what measured real bytes**, and they are
reproduced below. At unit scale the byte evidence is
`hud_counter_loop_counters_match_asl`, which assembles the corpus macro verbatim and
compares the six `dc.l`s AND the six `moveq` immediates that read `.loop_counter` back
against asl's own listing:

```
      27/      18 : 7C05                	moveq	#Hud_100000.loop_counter,d6
      29/      1C : 7C03                	moveq	#Hud_1000.loop_counter,d6
      32/      22 : 7C00                	moveq	#Hud_1.loop_counter,d6
```

**That `7C03` is the byte that made the `log10` spelling load-bearing.** `.loop_counter`
is "total digits minus one" and it is consumed as an immediate at `s2.asm(87595)`,
`(87746)` and four more sites. A `log` written `x.ln() / 10f64.ln()` answers
2.9999999999999996 for 1000, `int()` floors it to 2, and `Hud_1000` emits `7C02`: a
plausible number, one digit wrong, in a program that assembles clean. A wrong BASE is
loud; a wrong SPELLING of the right base is one byte at one of six sites.

## Red-first evidence

The tests are in `crates/sigil-frontend-as/tests/as_float_int.rs`, an existing file, so
no new source file was introduced.

**The mutation, shown applied on disk.** `git checkout 0eb20272 -- <path>` STAGES, so
`git diff --stat` reports nothing and would have read as "no mutation applied":

```
$ git checkout 0eb20272 -- crates/sigil-frontend-as/src/eval.rs
$ git diff HEAD --stat -- crates/sigil-frontend-as
 crates/sigil-frontend-as/tests/as_float_int.rs | 294 +++++++++++++++++++++++++
 1 file changed, 294 insertions(+)          <-- eval.rs is BACK AT BASELINE
$ grep -c 'FLOAT_BUILTINS\|apply_num_builtin\|floor_to_i64' crates/sigil-frontend-as/src/eval.rs
0                                           <-- and the file on disk proves it
$ grep -c hud_counter_loop_counters_match_asl crates/sigil-frontend-as/tests/as_float_int.rs
2                                           <-- while the tests are untouched
```

**What the run MUST fail, stated before running it.** Exactly the six new tests that
depend on the new table, and NOT the fourteen inherited ones. If everything had failed,
the instrument would have been broken rather than the feature; if nothing had, the
mutation would not have landed.

```
failures:
    an_out_of_domain_or_out_of_range_argument_is_refused
    builtin_names_are_case_insensitive
    float_builtin_surface_matches_asl
    hud_counter_loop_counters_match_asl
    int_floors_a_negative_including_through_log
    log_is_base_ten_and_ln_is_natural

test result: FAILED. 17 passed; 6 failed
```

`hud_counter_loop_counters_match_asl` failed with **six** `int(): could not evaluate
float expression` diagnostics: the corpus's own six, reproduced at unit scale. With the
fix restored, 25 passed, 0 failed.

**One test in the new set passes on the baseline too, and says so in its own doc
comment.** `a_float_builtin_result_is_refused_in_an_integer_slot` refuses on master as
well, for a different reason (`log` was an unknown symbol rather than a float-typed
result). It pins the direction only, and the comment records exactly that rather than
letting a future reader mistake it for coverage of the new table.

## The half-fix that looked like partial progress

Worth recording as a shape, not just as a fact.

Wiring `abs` into the typed evaluator reached it only INSIDE `int(...)`, which is not
where either corpus writes it. `s1disasm/Macros.asm(353)` is
`rept 1+(abs(first-last)/abs(step))`, with no float token on the line, so neither the
float-operand path (which fires on a float LEAF) nor the integer folder (which knows no
function names) could give `abs` a meaning. Generalizing `expand_int_builtin` to rewrite
any builtin call that resolves to an INTEGER took Sonic 1's eight `unresolved rept
count` rows to **four**.

Four reads like partial progress on something hard. It was not. The `range` macro is
invoked as `range $21,$2F,+1`, so `abs(step)` arrives as `abs(+1)`, and the typed
evaluator had an arm for unary MINUS and none for unary PLUS. The four that still failed
were precisely the four ASCENDING sites; the four descending ones passed because they
are spelled `-1`. **A fix measured only on the descending spelling would have read as
complete.** asl gives `0000 000F` for the count in both directions (`clean4.asm`,
exit 0), and the test asserts both spellings against that one number.

## The hardening that came out of it

`f.floor() as i64` SATURATES in Rust: `1e30 as i64` is `i64::MAX`. Master emitted that
silently. asl refuses at the `INT` itself (`error #1320: range overflow`), so
`floor_to_i64` now refuses, with an EXCLUSIVE upper bound because `i64::MAX as f64`
rounds up to 2^63 and an inclusive bound would admit exactly the value that saturates.
Domain violations refuse for the same reason: a NaN floored into an integer is a
plausible number from a program asl declines to assemble.

Refusal is spelled as `None` rather than as a diagnostic ON PURPOSE. `float_rhs` runs
this evaluator SPECULATIVELY on every `equ`/`=`/`set`/`:=` right-hand side just to ask
whether the value is float-typed, so anything that reports from here reports on lines
that are not wrong. That is measured, not theoretical: routing that probe through the
erroring expansion once doubled these very diagnostics from 6 to 12.

## The landing run

```
scripts/landing-run.sh --baseline 4689 --aeon /home/volence/sonic_hacks/.aeon-ref
```

**Failures first: there are none.** A grep for `test result: FAILED`, `^failures:` and
`FAILED` over the whole console returns nothing, so there are no failing test names to
list, aggregate or otherwise.

```
=============================== LANDING RUN VERDICT ===============================
  tree            .../worktrees/agent-a4254efe0a3a395d9 @ 9694f817 (parcel/as-float-freq-table, clean)
  reference       /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (HEAD, clean), all four present
  target dir      .../worktrees/agent-a4254efe0a3a395d9/.target-land
  started/ended   2026-09-06T06:58:13Z -> 2026-09-06T07:03:18Z (UTC)
  CARGO_EXIT      0
  CLIPPY_EXIT     0   (lint bar clean)
  suites          412
  passed          4700
  failed          0
  ignored         2
  skip lines      0
  reconciles      4689 baseline + 11 new = 4700 observed

  RESULT          GREEN
===================================================================================
```

`pwd` and `HEAD` beside the verdict, since a suite log does not otherwise name its tree:
`/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a4254efe0a3a395d9`, HEAD
`9694f817`, branch `parcel/as-float-freq-table`.

**skip lines 0**, so no reference-dependent gate silently downgraded. **11 new** is
exactly the count of tests this parcel adds to `as_float_int.rs` (14 inherited, 25 now).

### THIS IS THE SECOND RUN, and the first one is why

An earlier invocation at tip `0d2a6f3d` also came back GREEN with the same 4,700, but
its own stamp read **DIRTY**: the working tree carried the not-yet-committed dash
cleanup and this note. That verdict is not quoted here. Comment-only edits cannot move a
byte, and I believe they did not, but "I believe the diff was inert" is exactly the
argument a stamped log exists to make unnecessary. The transient probe artifacts
(`*.p`, the exit-2 `*.lst`) and the master baseline target dir were cleared, this note
withheld until afterwards, and the gate re-run against a `clean` stamp.

### The shipped-shape CRC gates, which are what measured real bytes

Seven shapes, each assembled natively and compared against the reference tree at
`.aeon-ref` `483b3e12`:

```
S1.4 plain:  assembled=0xbdc82 full=819131 appendix=0xa339 syms=3153 crc=1c09fbfc
S1.4 debug:  assembled=0xc055e full=840324 appendix=0xcd26 syms=3718 crc=e2144057
S2 config_a: assembled=0xc055e full=840676 appendix=0xce86 syms=3742 crc=213eee40
S2 config_b: assembled=0x8ccc0 full=617819 appendix=0xa09b syms=3078 crc=7ad605fc
S2 demo:     assembled=0x1121a full=96602  appendix=0x6740 syms=2028 crc=11ebd7ab
S2 demo_debug: assembled=0x1121a full=102818 appendix=0x7f88 syms=2364 crc=9b0d2ce7
S2 lean:     assembled=0xbcc00 full=773120 appendix=0x0    syms=3062 crc=3fd246f7
```

**All seven are unchanged from the baseline**, which is the intended result and needs
saying plainly rather than being read as a win: Aeon's `.emp` sources write none of the
functions this parcel adds, so the ROM bytes could not have moved. The CRC gates
therefore prove the change is INERT for the shipped shapes; the bytes it does affect are
`s2disasm`'s, which no ROM gate in this repo builds, and those are pinned by
`hud_counter_loop_counters_match_asl` against asl's own listing instead.

## The dash scan, and its own negative control

`dashscan.sh` beside this file. It scans the lines this branch ADDS
(`git diff <base>` filtered to `^+`), not whole files: this repository's prose uses em
dashes throughout, `eval.rs` alone holding 669 and `campaign-gap-ledger.md` 1,470, all
inherited, so a whole-file scan reports a four-figure number that says nothing about
what was written here.

Three guards, because the failure modes are three:

* **A canary on the PATTERN.** It matches a string carrying both characters before
  anything else runs, and exits 2 if it does not. Two parcels the night before had a
  scan return 0 for every file AND 0 for such a string.
* **A refusal on an EMPTY POPULATION.** A green over nothing is the one thing a
  red-first proof cannot catch. My own first version had exactly that defect and did
  not notice: `grep -cE '^\+'` is a stray escape in an extended regex, it matched
  1 line instead of 1,365, and it printed "clean".
* **A pinned self-exemption.** A dash detector must contain the dashes it detects. The
  two that remain are its own `PAT` and its own `canary`; they are counted, printed and
  their expected number is asserted, rather than filtered out. A third dash anywhere,
  including a third in that file, fails.

Result, and the control that shows it can fail:

```
$ dashscan.sh 0eb20272
canary: 1 line(s) matched (must be >= 1) -- the scan can fire
added lines scanned: 1927
clean: 1927 added lines, 2 dash(es), both this detector's own
exit=0

$ # plant one dash in a new tracked file and re-run
DASHES: 3 added line(s)
593:+a stray - dash          <- the plant (spelled with a real em dash in the run)
exit=1
```

## Gaps opened, not closed

Recorded in `campaign-gap-ledger.md`.

1. **The integer-valued builtin family.** `sgn`, `bitcnt`, `firstbit`, `lastbit`,
   `bitpos`, `toupper` exist in asl and not in sigil. `sgn(` is written by
   `s1disasm/MacroSetup.asm(221)` inside a string interpolation.
2. **A bare float builtin in an integer slot gets the wrong WORDING.** `dc.l LOG(100)`
   is refused, but as sigil's generic `bad long expression` rather than asl's `#1133`,
   because the operand path reaches the typed evaluator only when a float TOKEN is
   present. A refusal, never a wrong byte.
3. **Exponent literals.** `1e3`, `1.5e2`, `.5`, `1.` are still unimplemented in sigil's
   lexer (already recorded by `2026-09-04-f1-as-float-semantics.md`). Neither corpus
   writes any, and `values2.asm` exists so this parcel's probes did not need them.

## Anything in this brief you concluded was wrong

Eight things, and the biggest is the brief's own central framing.

1. **"23 float complaints, from two sites."** There is ONE site and SIX complaints. The
   brief measured a bare checkout and read a missing-file cascade as a float gap. Its
   own instinct was right ("site 2 is not isolated and I am telling you so rather than
   guessing") but the number 23 was carried forward into the bar ("the 23 becoming
   zero") as though it were all one population, and it is not.

2. **The brief's reduction of site 2 was not merely incomplete, it was structurally
   unable to reproduce the fault.** It says the reduction "agreed at `64`, so my
   reduction does not reproduce it" and lists four candidate causes: `:=` inside a
   macro, the `if "sampleRateScale"<>""` guard, `label.sample_rate` being float-valued,
   and the `id()`/`dpcmLoopCounter()` wrappers. **None of the four.** The cause is that
   `label.sample_rate` DOES NOT EXIST in a bare checkout, and a reduction that supplies
   a value for it removes the fault by construction.

3. **`as.int` is documented in the `.emp` front end as asl's "verified
   floor-toward-negative-infinity semantic"** and that is right, but the AS front end's
   `int` and the `.emp` front end's `as.int` are two separate implementations of it that
   share no code. The brief's premise question was "can the AS front end route these
   through the existing float machinery" and the answer is **no, and it should not**:
   `float_ns.rs` reports diagnostics, and the AS evaluator is silent by contract because
   `float_rhs` runs it speculatively. Plan 5's float work is real and is not reusable
   here.

4. **"Rounding is a separate question from the function"** was stated as a caution and
   turned out to be a live defect in the other direction: the function's SPELLING and
   the rounding are coupled. `log10` versus `ln(x)/ln(10)` is invisible until `int()`
   floors it, and then it is a wrong shipped byte. The brief's own instruction to test a
   negative would not have caught that one; the six positive powers of ten did.

5. **My own first `types.asm` reading.** I read `ABS(-3)` = `0000 0003` out of a listing
   whose run exited 2, which is exactly what `asl_ref.sh` says not to do. I re-asked it
   in `clean2.asm` (exit 0) before using it. The value was the same; the process was
   wrong and would have been indistinguishable if it had not been.

6. **"Say which subset cannot be closed."** The brief expected a partial answer and
   asked for it honestly. In the end nothing in the float population is left open: the
   6 went to 0 and the 17 were never sigil's. What IS left open is adjacent and is in
   the gap ledger, not hidden in a percentage.

7. **My own first scanner.** The dash scan used `grep -cE '^\+'`, where `\+` is a stray
   escape in an EXTENDED regex. It counted **1** added line instead of 1,365 and
   reported "clean". The canary fired correctly the whole time, because it proves the
   PATTERN can match, not that the INPUT arrived. A canary on the pattern and a refusal
   on an empty population are two different guards and I had only built the first.

8. **My own restore, twice in one hour.** After the red-first proof I ran
   `git checkout HEAD -- eval.rs` to "restore the fix" while HEAD was still master, so
   the restore silently reinstalled the baseline and I had to re-apply the whole change
   from context. The general shape: **`HEAD` is not "my work" until my work is
   committed**, and a restore command whose correctness depends on where HEAD happens to
   be is the same class as a guard whose expectation is derived from its subject. The
   second commit was made before the second baseline build for exactly this reason.
