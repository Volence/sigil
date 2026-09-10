# AS-REPT-CIRCULAR-SPLIT: what "provably circular" means, and why it cannot misfire

Implements owner ruling `d-23-answered` (`docs/decisions.jsonl`), answered direct in
session: **"Both, split by which it actually is."** Option key `split`.

- A count that **provably** depends on its own block is refused by name, naming the
  label, the repeat, and the loop between them.
- Everything else settles with **no fixed pass count**. `16` stops being a number a
  user is ever shown.
- Only a run that **truly oscillates** is reported, and the report says what was still
  moving and between which values, never a pass count.

The cost the owner explicitly accepted is the whole risk: *calling something circular
when it merely needed one more attempt would refuse a valid program.* A false positive
here is worse than the status quo it replaces. Today's failure is a confusing message on
a program that does not assemble; a false positive is a refusal of a program that works.

---

## 1. The measurement that reshaped the design

Taken against the reference `asl`,
`/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098` (verified at use). Sources in
`crates/sigil-frontend-as/tests/circular_layout.rs` as inline fixtures; the throwaway
probe files are reproduced there.

| probe | shape | asl | sigil BEFORE |
|---|---|---|---|
| `c1` | `rept N` above `N equ End-Start` (self-referential, settles at 0) | **exit 2**, `expression must be evaluatable in first pass` | accepts, emits |
| `c2` | `rept N` above `N equ End-Start+1` (diverges) | **exit 2**, same message | `did not converge within 16 passes` |
| `c3` | `rept Back-Start`, both labels BEHIND the repeat | exit 0 | exit 0 |
| `c4` | `rept N` above `N equ 4` (forward, but a pure constant) | **exit 2**, same message | accepts, emits |
| `c5` | `rept N`; an `org` sits between the repeat and `End`; `N equ End-Start` | **exit 2**, same message | accepts, emits |
| `c6` | `X set X+1` (no repeat at all) | exit 2, `symbol undefined` | already refused, `equ cycle` |
| `c7` | `dc.b F` above `F equ 4` (forward constant, NOT layout) | exit 0 | exit 0 |
| `c8` | `rept 3` with a literal count, `N equ End-Start` below | exit 0 | exit 0 |
| `d1` | `ds.b N` above `N equ End-Start+1` | **exit 2**, same message | `did not converge within 16 passes` |
| `d2` | `if End-Start>0` selecting a 3-byte arm (settles false) | **exit 2**, same message | accepts, emits |
| `d3` | `if End-Start=0` selecting a 3-byte arm (period 2) | **exit 2**, same message | `did not converge within 16 passes` |

**Three things this settles that the row's own name got wrong.**

1. **`rept` is not the only door to the 16-pass message.** `ds` (d1) and `if` (d3) reach
   it too. The row is named for a repeat; the mechanism is a *layout-determining
   expression* that reads a symbol its own emission moves.

2. **asl has no "needs another pass" tolerance here AT ALL.** Every layout-determining
   expression that names a not-yet-defined symbol is refused outright, with one message,
   *expression must be evaluatable in first pass*. That refusal covers the circular cases
   (`c1`, `c2`, `d1`, `d3`) and the plainly non-circular ones alike (`c4`'s forward
   constant, `c5`'s org-separated label, `d2`'s settling condition). asl never does cycle
   analysis; it applies a blunt structural rule and stops.

3. **Sigil is the LOOSER of the two today, not the stricter one.** `c1`, `c4`, `c5` and
   `d2` assemble here and are refused by asl. That is the over-acceptance direction, and
   it is a pre-existing defect, not one this parcel creates. See section 6.

## 2. The property, stated precisely

> A **layout-determining expression** is one whose value decides how many bytes are
> emitted at the point it is evaluated. Here: a `rept` count and a `ds.b`/`ds.w`/`ds.l`
> count.
>
> A layout-determining expression at program point `p` is **provably circular** when
> there is a symbol `S` such that all four of the following are observed **within a
> single pass**:
>
> **(a) `p` names `S`.** `S` appears as a symbol reference in the expression's tokens
> (not as a function call head, not as a builtin).
>
> **(b) `S` is not defined at `p`.** At the moment the expression is folded, this pass
> has not yet executed a definition of `S`. Its value, if any, came from the previous
> pass's seed.
>
> **(c) `S` is defined LATER IN THE SAME PASS, and the definition is reached.** Not
> assumed, not inferred from the previous pass. The refusal fires from inside the
> binding of `S`.
>
> **(d) `S`'s value is location-derived, in the same uninterrupted address flow as
> `p`.** Location-derived: `S` is a label, or an `equ`/`set` whose right-hand side
> names the location counter or a symbol already known to be location-derived. Same
> address flow: no `org`, `phase`, `dephase` or section change ran between `p` and
> `S`'s definition.

**Why that is a proof and not a heuristic.** Conjuncts (b) and (c) place `S`'s
definition strictly after `p` in this pass's execution order. Conjunct (d) says the
address flow from `p` to that definition is uninterrupted, so the location counter at
`S`'s definition is `PC(p) + (bytes emitted between them)`, and the bytes emitted at `p`
are one of the terms of that sum. Conjunct (d) also says `S`'s value is a function of
that location counter. So `S = f(PC(p) + k*E + ...)` where `E` is the expression's own
value and `k > 0` is the per-unit byte cost. Conjunct (a) says `E = g(S)`. The two
together are `E = g(f(... E ...))`: the expression's value is one of its own inputs. The
loop is exhibited term by term, not inferred from a symptom.

**Why it cannot fire on a program that merely needed another attempt.** Two independent
arguments, and the second does not depend on the first being right.

*(i) Containment in asl's refusal set, measured per case.* Every conjunct (a)+(b) pair is
exactly the condition asl calls *not evaluatable in first pass* for these directives. So
the set this refuses is a **subset** of the set asl already refuses. No program that
assembles under asl today can be refused by this. Probes `c1`, `c2`, `c4`, `c5`, `d1`,
`d2`, `d3` all show asl exit 2 on the whole enclosing class; `c3`, `c7`, `c8` show the
accepted shapes falling outside (a)+(b) and staying accepted. This is the containment
argument, and it is what makes the accepted cost bounded rather than open.

*(ii) Positive observation of every conjunct.* A program that "merely needed one more
attempt" fails conjunct (c) or (d). It fails (c) when the forward symbol is never bound
in the pass at all (an `ifndef`-guarded block that ran on pass 0 and skips afterwards is
the real instance of this in the corpus, and it is why the check waits for the binding to
be *reached* rather than predicting it). It fails (d) when the forward symbol is a plain
constant (`c4`, which settles in two passes and stays accepted here) or when an `org`
sits between (`c5`, which also settles and stays accepted). Nothing here is inferred from
"the value moved", which is the shape a heuristic would take and the shape that would
misfire.

**Direction of every approximation.** Where the analysis is imprecise it is imprecise
toward ACCEPTING. Location-derivation ignores the 68k `*` location-counter token (a
multiplication operator in the same position), so `N equ *` is not marked derived and a
repeat counting by it is not refused. The address-flow epoch bumps on `close_section` as
well as on `org`/`phase`/`dephase`, so extra bumps only ever suppress a refusal. A watch
that is never resolved is never reported.

## 3. What replaces the pass cap

`PASS_CAP: usize = 16` is gone. What decides now, in order:

1. **Convergence.** `env == prev`, unchanged. Every real build ends here in two or three
   passes.
2. **Provable circularity**, section 2. Terminal: a proof does not expire on a later
   pass, so the diagnostic is carried out of the pass that raised it rather than dropped
   as superseded (non-converged passes' diagnostics are dropped, which is why `c2`'s
   refusal would otherwise vanish).
3. **Provable oscillation.** The pass function is deterministic and its only varying
   input is the previous pass's symbol table. So if this pass's environment equals an
   environment from **two or more passes back**, then by induction the sequence repeats
   that segment forever and no fixpoint exists. That is a proof of non-convergence with
   no bound in it. `d3` is the instance: period 2. The report names the symbols that
   differ across the cycle and the two values each moves between.
4. **A resource guard**, `SETTLE_GUARD`, stated honestly. Divergence without repetition
   (`c2`'s shape, if its circularity were somehow not proven) grows without bound and
   would otherwise spin forever. The guard terminates it and emits the SAME "still
   moving" report as (3), naming symbols and values. **Its number is never printed, never
   named in any diagnostic, and is not a pass count in any user-facing sense.** It sits
   beside `EXPAND_CAP`, `WHILE_CAP` and `GLOBAL_REPT_CAP`, which are the same kind of
   thing and which the ruling did not touch. It is 200, not 16, so it is not a limit any
   real program approaches; the answer for every shape measured above comes from (2) or
   (3), never from the guard.

## 4. What the refusal says

Raised at the repeat, because that is the line the author changes, and naming the other
site by `file(line)`:

```
circular repeat count: this `rept` counts by `N`, and `N` is not defined until
c2.asm(8), after this repeat. `N`'s value comes from the location counter, which
this repeat's own bytes move, so the count is one of its own inputs. Give the
count a value that is known where the repeat stands, or move `N` above it.
```

`ds` gets the same sentence with "reservation" in place of "repeat count".

## 5. Half-fix, and what proves this one is not it

A detector that never fires would pass every existing test, remove the pass cap, and read
exactly like success: the corpora would still build, the suite would still be green, and
`c2` would report "still moving" instead of "16 passes", which is a visible improvement
on its own. Nothing in the existing suite distinguishes that from a working detector,
because nothing anywhere measures over-refusal or over-acceptance
(`NOTHING-MEASURES-OVER-ACCEPTANCE`).

So the tests are built in both directions and are red-first from a committed baseline:

- **Fires when it should** (`c1`, `c2`, `d1`): each asserts the refusal text, including
  the named symbol and the named definition site. Against the baseline these three read
  "accepts", "16 passes", "16 passes" respectively, so all three are red before the
  change and for three different reasons.
- **Does NOT fire when it should not** (`c3`, `c4`, `c5`, `c7`, `c8`, `d2`): each asserts
  a successful assembly AND the exact bytes. `c4` and `c5` are the two that a
  cycle-detector written without conjunct (d) would refuse, and they are the whole
  false-positive surface.
- **The oscillation path** (`d3`): asserts the moving-symbol report, asserts the symbol
  name and both values appear, and asserts the string `16` and the word `pass` do NOT
  appear.

## 6. The over-acceptance finding, tagged and NOT acted on

`c1`, `c4`, `c5` and `d2` are refused by asl and accepted by sigil. `c1` is closed by
this parcel (it is provably circular). `c4`, `c5` and `d2` are **left accepted**, because
the ruling's own words put them in "everything else settles with no fixed pass count",
and widening to asl's blunt first-pass rule would refuse programs the ruling asked us to
keep settling. They are recorded here as a standing divergence for the owner:

> Sigil accepts three shapes asl refuses with *expression must be evaluatable in first
> pass*: a `rept`/`ds` count naming a forward constant (`c4`), one naming a forward label
> separated by an `org` (`c5`), and an `if` condition naming a forward label that settles
> (`d2`). Each is a program that works here and does not assemble under asl. Closing them
> is a second decision, not this one.

## 7. The corpus gate, and the control that makes its zero mean something

**The result.** Diagnostic streams, baseline `ee6d1941` against this branch, each run from
its own corpus root with the tree read and never written:

| root | diagnostic lines | diff | exit, both |
|---|---|---|---|
| `s1disasm/sonic.asm` | 3 | 0 lines | 1 (a link-stage section overlap, unrelated) |
| `s2disasm/s2.asm` | 151 | 0 lines | 1 |
| `skdisasm/sonic3k.asm` | 190 | 0 lines | 1 |

**Zero new refusals, and no line moved.** Nothing that assembled before is refused now, so
there is no per-site ruling-or-defect classification to make: the population is empty.

**The zero is not vacuous, and proving that took two instruments.**

*Input count asserted.* `SIGIL_CENSUS_LAYOUT` prints four numbers per pass: expressions
folded, watched (conjuncts (a)+(b)), bound later in the same pass (conjunct (c), the near
misses), and refused (conjunct (d) survived).

```text
s1disasm   folded=483   watched=0   bound-later=0   refused=0
s2disasm   folded=667   watched=0   bound-later=0   refused=0
skdisasm   folded=956   watched=0   bound-later=0   refused=0
```

2,106 layout count expressions swept per pass. Every one had a count already known where
it stood, which is what asl's own first-pass rule forces on any disassembly that ships.

**`bound-later=0` everywhere deserves its own line.** Conjunct (d), the part that carries
all of the false-positive risk, was never even reached on 2,106 real sites. The corpus does
not come near the surface where a wrong answer would be possible.

*Positive control at corpus scale, and the first attempt at it was itself vacuous.*
Appending the circular repeat to a COPY of each root changed nothing: `folded` stayed 483
and 667. Both roots end in `END`, so the appended lines never execute, and that control
would have "confirmed" the mechanism while never running it. Injecting the same block
BEFORE the final `END` instead:

```text
s1 copy   folded=484 (+1)   watched=1   bound-later=1   refused=1   named at sonic.asm(5239)
s2 copy   folded=668 (+1)   watched=1   bound-later=1   refused=1   named at s2.asm(91278)
```

The `+1` in the fold count is what says the needle was present, and it is exactly the
number the first attempt failed to produce.

## 8. What the row's own name got wrong, for the record

`AS-REPT-CIRCULAR-SPLIT` names a repeat. Three constructs reach the message the ruling
removed: a `rept` count, a `ds` count, and an `if` condition. Two of them are answered by
the circularity proof and the third by the oscillation proof, and both answers are the
ruling's. No part of the ruling turned out to be unimplementable as stated; the only
correction is to its scope, which was narrower than the defect.

## 9. A 20% regression this parcel shipped, and how it was found

Not by anything going red. The suite was GREEN at 5,055/0, both corpora were
byte-identical, and the AS front end had gone from **2.28s to 2.73s** on s2disasm. No gate
in this repo measures that.

Isolated rather than guessed: stubbing `expr_is_pc_derived` to return `false` gave 2.38s,
so 0.35s of the 0.45s was in that one function and the rest was an extra `SymbolTable`
clone per pass. The cause was `sym_key`, which allocates a `String` for every name handed
to it. `expr_is_pc_derived` ran it on every identifier of every `equ`/`set` right-hand side
in the unit, and for a plain name outside a macro expansion the key IS the bare spelling,
so the allocation bought nothing. The bare-name lookup now runs first, and the key is built
only for a `.`-local or a name a live expansion owns. The oscillation history now takes
`prev` by move rather than cloning the same table twice.

**2.22s / 2.29s against a baseline 2.21s / 2.29s**, two runs each. Within noise.

Re-verified afterwards, because a performance edit to the exact function conjunct (d) rests
on could have changed what it decides: 11/11 and 2/2 green, all six accepting probes
byte-identical to their pre-optimization output, all three corpus diagnostic streams still
0 lines from baseline with the same census, and both injected-circular controls still
firing.

## 10. The half-fix question, answered

**What a half-fix looks like here:** a detector that never fires. It would remove the pass
cap, keep 5,055 tests green, keep all three corpora byte-identical, and turn the diverging
`rept` into a moving-symbol report, which is a real improvement on its own. Nothing in the
suite would say a word.

**What proves this one fires when it should.** Four `refuses_*` tests, red-first against a
committed baseline with the mutation shown on disk, red for three different reasons (two
silent wrong images, two 16-pass messages), plus a corpus-scale control on copies of
s1disasm and s2disasm where the injected repeat is refused by name at the right line and
the census fold count goes 483 to 484 and 667 to 668. That `+1` is the number that
distinguishes a control that ran from one that did not, and the FIRST version of that
control did not produce it: appended after the roots' `END`, it changed nothing and would
have "confirmed" a mechanism it never reached.

**What proves it does NOT fire when it should not.** The six `accepts_*` tests pass on the
baseline too, so on their own they prove nothing. They were therefore given their own
positive control: conjunct (d) was deleted on disk (a cycle detector built from the
dependency graph alone, which is what a reasonable person would write) and exactly two
tests went red, `accepts_forward_constant_count` and `accepts_count_separated_by_an_org`,
which are exactly the two written for it. Every `refuses_*` stayed green under that
mutation, so the two halves are independent. On top of that, the containment argument in
section 2: the refused set is a subset of asl's, measured probe by probe.

## 11. Follow-ups this parcel did NOT take

1. **The over-acceptance divergence**, section 6: three shapes sigil assembles that asl
   refuses. The ruling puts them in "everything else settles", so closing them is a second
   decision.
2. **`sigil-link/src/relax.rs:1116` still prints a pass count**: *"relaxation width
   selection did not converge within {cap} passes"*. It is the same class of message the
   ruling removed from the front end, in a different subsystem with its own convergence
   argument. Named here rather than changed, because changing it needs its own proof and
   its own boundary tests; a front-end ruling is not authority over the linker's.
