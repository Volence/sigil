# MOMPASS: a pass number a fixpoint cannot report, and what it can report instead

2026-09-05 · branch `parcel/as-mompass` · sigil master base `3c3f625c`

`MOMPASS` is an AS builtin sigil did not implement. Until this morning that was
invisible, because an undefined symbol in an `if` was read as false and the
block it guarded vanished without a word. That silent-false fault was fixed and
landed (`d9f00a3e`), so `MOMPASS` began refusing by name at 7 of the 11
positions the new refusal reports in s2disasm. This parcel gives it a value.

Reference asl throughout: `/home/volence/sonic_hacks/sonic_hack/tools/as/asl`,
md5 `61e672562465725a8c102288a7da9098`, invoked `-xx -n -A -L -U -E -i .`, exit
status checked and quoted at every number below. s2disasm's own asl (md5
`0dee1f98e6480a4783d27ffd8b90896f`) was run ONCE, for the sole purpose of
generating the tree's `sound/*/generated/*.inc` inputs, and no value in this note
comes from it. Probes and run logs are archived in
`2026-09-05-as-mompass-probes/` beside this file.

---

## The semantics, and the evidence that chose them

**`MOMPASS` reports 1 on the first iteration of sigil's fixpoint and 2 on every
later one.**

### Step 1: what asl's MOMPASS is

It is asl's 1-based pass counter, readable as an ordinary value, and it is not
bounded by 2. Three probes, all exit 0, all 0 errors:

| probe | source | asl | `dc.b MOMPASS` |
|---|---|---|---|
| `pA` | no forward reference | `1 pass` | `01` |
| `pB` | one forward reference (`dc.w Later-*`) | `2 passes` | `02` |
| `pI` | `move.w d0,Sym` where `Sym equ $123456` grows the operand to `.l` | `3 passes` | `0003` |

`pI` is the probe that matters, and it was chosen for a property the brief asked
for: a wrong answer looks different from a right one. Pass numbers are small
integers, so `1` and `2` are confoundable with almost anything. `3` is not: it
is a value no "first versus later" rule, no clamp, and no constant could produce.
It rules out four hypotheses at once.

### Step 2: sigil's iteration count is not asl's pass count

Measured, not reasoned from the architecture. sigil instrumented with a
one-line trace in `run_impl`'s pass loop (removed before the implementation
commit; the exact patch is at the end of this section), asl read from its own
`N passes` line:

| probe | asl passes | sigil iterations | agree |
|---|---|---|---|
| `pA` no forward reference | 1 | 2 | no |
| `pB` one forward reference | 2 | 2 | yes |
| `pE` forward `:=` | 2 | 3 | no |
| `pG` chained forward `:=` | 2 | 3 | no |
| `pH` absolute address that fits `.w` | 2 | 2 | yes |
| `pI` absolute address that grows to `.l` | 3 | 3 | yes |
| **s2disasm `s2.asm`** | **2** | **4** | **no** |

Three of six probes disagree, in BOTH directions, and the corpus root disagrees
by two. So "report the current iteration" is not "report asl's pass number"; it
is a different quantity that happens to share a name.

The trace used:

```rust
    for pass in 0..PASS_CAP {
        if std::env::var("SIGIL_PASS_TRACE").is_ok() { eprintln!("SIGIL-ITER {}", pass + 1); }
```

and the asl side of the s2 row is `2 passes` on the complete tree (after
`build.lua` generated the `sound/*/generated/*.inc` inputs), corroborated
independently by s2disasm's own asl exiting 0 on the same tree and printing the
`if MOMPASS=2` message from `s2.asm(91270)`, which only fires when the final
pass is pass 2.

The blessed asl's own s2 run carried **137 errors** (all `error #1010: symbol
undefined`, all from the `jsrto` macro: upstream asl Bld 212 does not implement
what s2disasm's flamewing fork does), so under bar 1 it is **not** a source of
values. Its `2 passes` line is reported here as corroboration of the pass count
only, and the number it corroborates was independently established on a clean
run: see step 4.

### Step 3: the decision, and the measurement that made it

A running count and a saturation disagree at exactly one shape, and it is the
corpus's shape. `m_flag2.asm` mirrors `s2.asm(91270)`: an `if MOMPASS=2` whose
body emits NO bytes (it sets a `:=` a later `dc.b` reads) inside a file whose
iteration count is driven by a forward `:=` chain rather than by MOMPASS itself,
so the guard cannot move the layout. That is the corpus situation, where sigil's
iteration count exceeds asl's pass count for reasons unrelated to MOMPASS.

```
asl   (exit 0, 2 passes, 0 errors)   AA 11 03 EE
saturating at 2                      AA 11 03 EE
running count (`pass as i64 + 1`)    00 11 03 EE
```

The running count is a **byte divergence from asl** at the corpus's own `=2`
shape. That is the reason for the saturation, and it is a byte comparison, not
an argument. Reproduced by `countertest.sh`, which builds both.

### Step 4: a clean corpus-source run, for the value

Bar 1 requires expected values from an exit-0, error-free run. `songprobe.asm`
is `build.lua`'s own standalone song wrapper: real corpus source
(`sound/_smps2asm_inc.asm`, which carries five of the twelve MOMPASS sites, plus
`sound/music/93 - Boss.asm`), self-contained.

```
2 passes    0 errors    0 warnings    exit 0
   15/     2D2 : 02                  dc.b MOMPASS
```

So on real corpus source the blessed asl takes 2 passes and `MOMPASS` is 2 on
the pass whose bytes are kept.

### What is portable

The corpus does not use the number, it uses a distinction, and it writes the
distinction down at `s2.constants.asm(972)`:

> `if MOMPASS > 1 ; Avoid undefined symbol errors by checking only after the first pass.`

Reporting 1 then 2 makes that distinction exact. Across s2disasm, s1disasm and
skdisasm no `MOMPASS` comparison names a pass number above 2. All 33
occurrences are comparisons and the whole census is `=1` x27, `==1` x3, `>1`
x1, `=2` x1, `==2` x1 (the `==2` is s1disasm's). aeon names `MOMPASS` in no assembly source at all.

### What it costs elsewhere, stated rather than hidden

Two divergences, both asserted as tests rather than left to be discovered, both
with **zero corpus population**:

1. **A file asl assembles in ONE pass.** asl emits `01`, sigil emits `02`. No
   definition could avoid this: convergence in `run_impl` requires `pass > 0`,
   so the iteration that emits bytes is never the first, and no value MOMPASS
   could take would make it report 1 on the pass that emits. Test
   `one_pass_asl_file_is_a_known_divergence`.
2. **`if MOMPASS=<n>` guarding a body that EMITS.** Under asl this is self
   destabilising: emitting moves the layout, forcing a further pass on which
   `MOMPASS` is no longer `<n>`, so asl settles with the body OUT (probe
   `m_eq2`: `4 passes`, exit 0, bytes `11 00 02`). sigil saturates, so the
   condition stays true and the fixpoint is stable with the body IN
   (`AA 11 00 02`). A running count matches asl here; the saturation is chosen
   anyway because the shape it gets right instead is the one the corpus has.
   Test `mompass_eq_two_guarding_an_emission_diverges_from_asl`.

A third divergence is NOT specific to MOMPASS and is booked separately: see
"What this parcel newly exposes but did not cause".

---

## The per-site agreement table

All twelve s2disasm `MOMPASS` sites at `e45ebf33`. "asl verdict" is the verdict
on asl's FINAL pass, which is pass 2 for this tree. "Emits?" is what the guarded
body would put in the ROM.

| # | site | form | body | emits? | asl verdict | sigil verdict | agree |
|---|---|---|---|---|---|---|---|
| 1 | `s2.macrosetup.asm(81)` | `if MOMPASS=1` | `message` (the `trace` macro) | no | FALSE | FALSE | yes |
| 2 | `s2.macros.asm(119)` | `elseif (DebugSoundbanks<>0)&&(MOMPASS=1)` | `message` | no | FALSE | FALSE | yes |
| 3 | `s2.macros.asm(224)` | `if (.cur_zone_id<>no_of_zones)&&(MOMPASS=1)` | `message` | no | FALSE | FALSE | yes |
| 4 | `s2.asm(88574)` | `if (cur_zone_id<>no_of_zones)&&(MOMPASS=1)` | `message` | no | FALSE | FALSE | yes |
| 5 | `s2.asm(91270)` | `if MOMPASS=2` | `message` (ROM size) | no | TRUE | TRUE | yes |
| 6 | `s2.constants.asm(972)` | `if MOMPASS > 1` | two `fatal` bounds checks | no | TRUE | TRUE | yes |
| 7 | `s2.sounddriver.asm(297)` | `if MOMPASS=1` | `endpad := $` then `warning` | no | FALSE | FALSE | yes |
| 8 | `sound/_smps2asm_inc.asm(238)` | `if (MOMPASS=1)&&(DEFINED(loc))` | `fatal` | no | FALSE | FALSE | yes |
| 9 | `sound/_smps2asm_inc.asm(258)` | `if MOMPASS=1` | `message` | no | FALSE | FALSE | yes |
| 10 | `sound/_smps2asm_inc.asm(282)` | `if (MOMPASS=1)&&(DEFINED(loc))` | `fatal` | no | FALSE | FALSE | yes |
| 11 | `sound/_smps2asm_inc.asm(349)` | `if (MOMPASS==1)&&...` | `message` | no | FALSE | FALSE | yes |
| 12 | `sound/_smps2asm_inc.asm(947)` | `if MOMPASS=1` | `message` | no | FALSE | FALSE | yes |

**Twelve of twelve agree.** The column that carries the weight is "emits?":
every single site guards a `message`, a `warning`, a `fatal`, or a `:=` that
feeds one. **No MOMPASS site in this corpus emits a byte.** The two divergences
above are therefore unreachable from it as it stands, and the design question
the brief flagged as byte-affecting turns out, for this corpus, not to be.

Site 5, `if MOMPASS=2`, is the one the brief expected to be hardest, and it was:
it is the only site a running count would have decided wrongly, and the only
reason the saturation was chosen over the more literal design.

Site 6 is worth one extra sentence: `if MOMPASS > 1` was previously refused, so
its two `fatal` bounds checks never ran. They now run, and they pass, which the
corpus decomposition confirms by adding no rows.

---

## Red-first, with the mutation shown applied on disk

`redfirst.sh`, log archived. Each round applies the mutation to the COMMITTED
source, prints it back by content grep AND by `git diff HEAD --stat` (plain
`git diff` reports nothing after `git checkout <rev> -- <path>`, because that
STAGES), states what the run MUST fail at, runs it, and restores from the
committed baseline `876ae0c8`. A mutation that failed to apply would void its
round rather than print ok.

| mutation | applied? | result | reds |
|---|---|---|---|
| `MOMPASS` arm deleted from `builtin_num` | `git diff HEAD --stat` 1 file, 1 deletion | **FAILED. 0 passed; 9 failed** | all ten, as refusals instead of bytes |
| `FIRST_PASS`/`LATER_PASS` swapped to 2/1 | grep shows `= 2` / `= 1` | **FAILED. 2 passed; 7 failed** | `left: [170, 17, 0, 2] right: [17, 0, 2]` and six more |
| saturation replaced by `pass as i64 + 1` | grep shows `pass as i64 + 1` | **FAILED. 8 passed; 1 failed** | only `mompass_eq_two_...`, `left: [17, 0, 2] right: [170, 17, 0, 2]` |
| (restored) | 0 dirty paths | **ok. 10 passed; 0 failed** | |

The third round is the informative one and it is why the second commit on this
branch is a retraction. It reds exactly one test, and the value it produced,
`11 00 02`, is **asl's answer** for that probe. The running count is better than
the saturation at that shape, not worse. That refuted a claim I had already put
in the first commit message.

The swap mutation was chosen deliberately for bar 3: swapping one small integer
for another small integer is exactly the confoundable shape, so the round is
only meaningful if it moves the `=1` row and the `>1` row in OPPOSITE
directions, which the quoted `left`/`right` pairs show it does.

Runner: `cargo test -p sigil-frontend-as --test as_mompass_builtin`, ten tests,
all with expectations derived from named asl probes quoted in each test's doc
comment.

### The `fatal` rounds

`redfirst_fatal.sh`, log archived, same bars, baseline `a77ef3b0`. Runner
`cargo test -p sigil-frontend-as --test as_fatal_survives_its_pass`, five tests.

| mutation | result | reds |
|---|---|---|
| the carry removed (`let _ = terminal_fatal;`) | 3 passed, 2 failed | the shape returns to bytes and exit 0 |
| the raise-time label ignored (`if true {`) | 4 passed, **1 failed** | ONLY the include test, quoting `inc/b.asm(1)` |
| the dedupe removed | 4 passed, **1 failed** | the same fatal reported twice |
| (restored) | ok, 5 passed and 10 passed | |

The second round is the discriminating one, and it was designed to be: a
single-file span renders correctly with or without the label capture, so a
mutation that red everything would not have shown the capture was load-bearing.
It reds exactly one test and prints the misattribution in the failure message.

---

## The corpus decomposition

`corpus.sh`, log archived. Plain `sigil s2.asm` in a clean copy of s2disasm at
`e45ebf33` (0 dirty paths), before and after binaries both named by md5 in the
run log, and each answering the parcel probe first as a freshness witness
(before: exit 1, the `MOMPASS` refusal; after: exit 0, `11 00 02`).

```
before  exit=1  5254 diagnostic lines
after   exit=1  5247 diagnostic lines        -7
located: 5254/5254 before, 5247/5247 after   (every row names file(line))
```

The after-count is **measured, not predicted by subtraction**. Closing MOMPASS
lets the assembler reach code it previously abandoned, so rows could have been
added; the point of the run was to find out whether any were.

| level | before | after | delta | class |
|---|---|---|---|---|
| error | 8 | 2 | -6 | `unresolved if condition: \`X\` has no value, ...` |
| error | 1 | 0 | -1 | `unresolved elseif condition: \`X\` has no value, ...` **GONE** |
| error | 5245 | 5245 | +0 | 58 other classes, every one unchanged |
| | 5254 | 5247 | -7 | TOTAL |

**No class ROSE. No class APPEARED.** Diagnostic lines present after and absent
before: **0**. Unresolved-symbol name sets, both directions: before-only
**{`MOMPASS`}**, after-only **empty**, in both **8** (`.loop_counter`,
`Snd_Sega.size`, `ixl`, `ixu`, `iyl`, `iyu`, `zAbsVar.1upPlaying`,
`zVar.1upPlaying`).

The seven lines that went away, one per position, are exactly the seven the
previous parcel booked:

```
s2.asm(88574)                  s2.constants.asm(972)     s2.sounddriver.asm(297)
s2.asm(91270)                  s2.macros.asm(119)        sound/_smps2asm_inc.asm(258)
                               s2.macros.asm(224)
```

Twelve sites, seven positions: the other five sit in macro or `if` arms this
root never reaches (the `SonicDriverVer=1` paths in `_smps2asm_inc.asm`, and the
`trace` macro, which is never invoked).

**The corpus did not assemble before and does not assemble now** (exit 1 both
ways, 5245 unrelated diagnostics). The parcel removed seven refusals from a run
that already failed, and each one was a condition sigil could not evaluate and
had been choosing an arm for anyway.

---

## The aeon assertion

`aeon_assert.sh`, log archived. Asserted, not assumed, because this crate is on
aeon's shipping build path.

aeon at `2be6020a`. `git grep -l MOMPASS -- '*.asm' '*.inc' '*.emp'` returns
**nothing**: the only aeon files naming MOMPASS are four markdown documents.
Both binaries were run over all three AS roots, from each root's own directory:

| root | before | after | MOMPASS firings | stdout+stderr identical |
|---|---|---|---|---|
| `engine/debug/debugger.asm` | exit 1, 23 diagnostics | exit 1, 23 diagnostics | 0 / 0 | **yes** |
| `games/demo/game_root.asm` | exit 0, 0 diagnostics | exit 0, 0 diagnostics | 0 / 0 | **yes** |
| `games/sonic4/game_root.asm` | exit 0, 0 diagnostics | exit 0, 0 diagnostics | 0 / 0 | **yes** |

A standalone run defines fewer symbols than the real build, so it can only
OVER-report; a clean identical result is therefore meaningful rather than an
artifact of too weak a run. No aeon tree was built and `AEON_DIR` was never set.
aeon's dirty-path count was 2 before the run and 2 after (the owner's live
edits, untouched).

---

## The `fatal` that was dropped, and the correction to my own control

The overseer held the branch on this shape, and was right to:

```
    if MOMPASS=1 / fatal "first-iteration problem" / endif / dc.b $11

    asl                            exit 3, assembly terminated
    sigil master                   exit 1, refuses the unresolved condition
    sigil with MOMPASS, unfixed    exit 0, NO OUTPUT AT ALL
```

Quieter than master and quieter than the reference, on the strongest refusal the
language has.

### My first control was confounded, and the framing it supported was wrong

The first version of this note booked the discard as "pre-existing and general",
on `ctrl_fatal.asm`: a `fatal` under `if V = 0` with `V := W` and `W` forward,
no `MOMPASS` in the file, identical before and after (exit 0, `11 EE`).

**That probe never reached the discard.** `V` folds to Poison on iteration 1, so
the arm is SKIPPED and the `fatal` does not execute at all. The identical
before/after was real and meant nothing: there was nothing to drop. The probe
could not have distinguished my explanation from the truth, which is the same
defect class the brief warned about and the second time in this parcel that I
banked reasoning I had not run. Probe `ctrl_vprobe` isolates it: `dc.b V` emits
`02` under BOTH asl and sigil, so the two agree about `V` and the divergence in
`ctrl_fatal` is asl evaluating an undefined symbol as 0 on its first pass where
sigil poisons it, which is a different, already documented difference.

### Re-derived: MOMPASS is the only route I could construct

The discard needs a condition with a VALUE on iteration 1 that is TRUE there and
FALSE later. Three further controls all came back LOUD on master:

| control | mechanism | master | asl |
|---|---|---|---|
| `ctrl2_ifndef` | `ifndef` before the definition | exit 1, reported | exit 3 |
| `ctrl3_relax` | a label moved by operand relaxation | exit 1, reported | exit 3 |
| `co_fatal_plain` | unguarded | exit 1, reported | exit 3 |

One structural reason covers all three: **a `fatal` aborts its pass, which
truncates the env, so whatever would flip the condition on a later pass never
runs and the `fatal` re-fires on every pass.** For the flip to survive it has to
come from something BEFORE the `fatal`, and anything before it has the same
value on every iteration. `MOMPASS` is the exception, because it flips for a
reason internal to the assembler rather than a source fact.

So this parcel did not widen a general fault. It created the only population
there is, which makes the hold right for a sharper reason than either of us had.

### The census turned out not to be needed

I said the fix needed a static census of every corpus `fatal` first. It does
not, and what replaces it is a measurement rather than a count: a three-way run
over every root there is.

| root | MASTER | MOMPASS | +FATALFIX | MOMPASS vs FATALFIX |
|---|---|---|---|---|
| s2disasm `s2.asm` | exit 1, 5254 | exit 1, 5247 | exit 1, 5247 | byte-identical |
| s1disasm `sonic.asm` | exit 1, 68 | exit 1, 50 | exit 1, 50 | byte-identical |
| skdisasm `sonic3k.asm` | exit 1, 2135 | exit 1, 2132 | exit 1, 2132 | byte-identical |
| aeon `debugger.asm` | exit 1, 23 | exit 1, 23 | exit 1, 23 | byte-identical |
| aeon `demo/game_root.asm` | exit 0, 0 | exit 0, 0 | exit 0, 0 | byte-identical |
| aeon `sonic4/game_root.asm` | exit 0, 0 | exit 0, 0 | exit 0, 0 | byte-identical |

stdout AND stderr identical in all six. The two roots that raise a `fatal` at
all raise it on every pass and already reported it: s1disasm's is `sound/z80.asm(229)`,
skdisasm's is `Sound/Z80 Sound Driver.asm(345)`. **No root has a dropped
`fatal`**, so the fix adds nothing anywhere except the shape it was written for.

### Terminating was tried first, and measured

asl literally terminates, so a hard stop is the faithful reading, and it was
implemented first. It is wrong here, and the reason is a number rather than an
argument: it cut s1disasm from **50 located diagnostics to 1** and skdisasm from
**2132 to 1**, and in both cases the single survivor was a line the run already
printed. So the fix CARRIES instead: the run converges exactly as before and any
`fatal` raised on any pass is added to the returned diagnostics, deduped. That
is strictly additive, so it can make a run louder and never quieter.

### A carried span misattributes, shown rather than supposed

The source map is rebuilt every pass and ids are handed out in splice order, so
a file spliced only on the raising pass shifts every later id. Probe `co_map2`
puts the `fatal` in `inc/c.asm`, included only under `if MOMPASS=1`, with four
more includes after it:

```
    carrying the bare span     inc/b.asm(1): error: fatal from inside an included file
    capturing the label        error: inc/c.asm(1): fatal from inside an included file
```

A real file, a real line, and the wrong one. The `file(line)` is therefore
captured against the raising pass's own map at raise time, and used whenever the
returning map disagrees. Probe `co_map`, with fewer includes, shows the other
failure mode of the bare span: the id falls off the end of the map and the
diagnostic renders with no location at all.

### Keyed on `fatal`, not on `aborted`

`aborted` is set by five things: `fatal`, `end`, include-nesting overflow, the
`while` budget, and the undeclared-processor refusal. `end` sits at the bottom
of every well-formed file in all three corpora, so a fix keyed on `aborted`
would fire on all of them. `fatal` needs its own signal, and has one.

### Scope held: `warning` is still dropped

A `warning` under the same guard still vanishes (asl prints it on its first
pass, sigil prints nothing, exit 0 either way). Pinned as a test so it reads as
a decision rather than an oversight, and booked. `warning` is not separable on
the argument that carries `fatal`: asl treats it as a diagnostic and keeps
assembling, and prints one once per pass rather than once per run, so a later
pass genuinely does supersede it. Widening to it needs the census of
pass-dependent diagnostics that `fatal` turned out not to need.

---

## What did not execute

* **The byte-identity golden gates did not run.** The suite was run
  `SIGIL_ALLOW_PARTIAL=1 cargo test --workspace --no-fail-fast`, and the
  harness's own banner says what that costs: *"PARTIAL RUN
  (SIGIL_ALLOW_PARTIAL is set). No reference tree is named, so **127 test
  binaries are reference-dependent** and every row in them is left UNMEASURED.
  A green result from this run does NOT mean those rows passed, it means they
  were not run."* The banner fired in 116 test processes. **The byte gates did
  not execute. This note does not claim they passed.**
* **Totals** after the `fatal` fix: **4613 passed, 0 failed, 2 ignored**, across
  405 result lines, exit 0. (Before it, identical across two runs, captured and
  `--nocapture`: 4608 passed, 0 failed, 2 ignored, 404 result lines.) The two
  ignored are `sigil_diff_reports_byte_identity` ("reads the aeon source tree")
  and `secondary_pin_classes_match_the_hand_typed_baseline` (retired by Wave-B
  B-0). No failing names, because there are none.
* **No emulator was touched.** Nothing here wanted runtime confirmation.
* **No aeon build.** The three roots were read; `AEON_DIR` was never set. aeon
  moved under this parcel while it ran, from `2be6020a` to `747ed40e` (the owner
  commits live), so the three-root assertion is quoted at the SHA each run saw
  and both runs were clean.
* **`warning` and `message` on a non-final pass are still dropped.** Scoped out
  deliberately, pinned as a test, booked.
* **The full s2disasm ROM was never assembled by the blessed asl without
  errors**, and cannot be: upstream asl Bld 212 refuses s2disasm's `jsrto` macro
  137 times. Every asl VALUE in this note comes instead from a clean exit-0 run,
  either a synthetic probe or `songprobe.asm` over real corpus source.

---

## Anything in this brief you concluded was wrong

Five things. The first two are mine rather than the brief's, and the first is
the largest thing in the parcel: it was found by the overseer's hold, not by me.

**1. The brief's central worry does not materialise, and the reason is in the
corpus rather than in the semantics.** The brief framed this as byte-affecting
("a construct that decides what code exists") and named the semantics question
as a real BLOCKED candidate. It is not blocked, and the thing that unblocks it
is not a clever definition: it is that **all twelve corpus sites guard a
`message`, `warning`, `fatal` or a `:=` feeding one, and not one of them emits a
byte**. The whole population is diagnostic gating. The design still had to be
got right, because the `=2` site's verdict does differ between the two candidate
designs, but the risk of emitting different code from the same source was zero
for this corpus before the first line was written. Reading the twelve sites
should be step one of a parcel like this, ahead of any theory.

**0. My "pre-existing and general" framing of the dropped `fatal` was wrong, and
the control that supported it was confounded.** This is the largest correction
in the parcel and it is written up in full above. `ctrl_fatal.asm` never reached
the discard: its `fatal` sits behind a Poison condition and does not execute, so
its identical before/after meant nothing. Three replacement controls all come
back loud on master, for the structural reason that a `fatal` truncates its own
pass and therefore re-fires on every one. `MOMPASS` is the only route to the
discard I could construct, so the parcel did not widen a general fault, it
created the only population there is. I also said the fix needed a static census
of every corpus `fatal`; it did not, and a three-way run over all six roots
bounds it exactly, at zero. And the fix I would have written from the brief's
own logic, a hard stop matching asl, is measurably wrong: it cuts s1disasm from
50 diagnostics to 1 and skdisasm from 2132 to 1.

**2. I put a false mechanism in my own first commit message, and my own
red-first run refuted it an hour later.** I wrote that a running count "would
oscillate with period two and never satisfy `env == prev`, so the run would
exhaust `PASS_CAP`". Built, it converges cleanly and emits `11 00 02`, which is
asl's answer, so the running count is BETTER than the saturation at that shape.
I had reasoned about a fixpoint I had not run, and the conclusion I reached
(saturate) was right for a reason I had not measured. Commit `876ae0c8` retracts
it in its subject line and re-grounds the design on `m_flag2`, a byte
comparison. The lesson is the one the brief's bar 3 states and I still walked
past: a convenient result suppresses the check, and this one was convenient
because it confirmed a decision I had already made.

**3. "7 of the 11 sites the new refusal reports in s2disasm are MOMPASS" is
right, but "the corpus population is 11" would not have been.** The brief's own
census lists twelve `MOMPASS` lines and the refusal reports seven positions. The
two numbers measure different things (source lines versus reached positions),
and five sites are in arms this root never enters. Nothing in the brief conflates
them, but the gap is easy to trip over and the note now says which is which.

**4. Bar 5's premise held, with one correction to its phrasing.** MOMPASS
appears in **0 aeon assembly sources**, which is what matters, but it does
appear in four aeon markdown documents, so a bare `grep -rn MOMPASS aeon` does
not return zero and a future reader checking the claim that way would think it
had rotted. The assertion is unchanged: all three roots byte-identical, zero
firings.

One thing the brief got exactly right and I want to record as such: its warning
that a wrong answer looks like a right one when the values are small integers.
`pI`, the three-pass probe, is the only reason I can say asl's MOMPASS is a
counter rather than a flag, and finding it took five failed attempts (`pC` and
`pD` errored, `pE`, `pF`, `pG` and `pH` all stopped at two passes) because
forcing asl to a third pass needs an operand that GROWS, not merely a forward
reference.
