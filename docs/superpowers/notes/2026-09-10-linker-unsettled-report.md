# The linker's layout fixpoint stops naming a pass count

Owner ruling `d-23-answered` (`docs/decisions.jsonl`), option `split`, reached the
assembler front end (`crates/sigil-frontend-as/src/eval.rs`, the `SETTLE_GUARD` doc
comment) and never reached the linker. `crates/sigil-link/src/relax.rs` still ended its
layout fixpoint with:

```
relaxation width selection did not converge within {cap} passes
```

This note records what the surface turned out to be, which is not what the parcel brief
predicted, and what landed.

---

## 1. The diagnostic is unreachable, and the reason is the design

The parcel asked whether the message is reachable at all, and told me not to answer from
reading. Both halves of the answer follow.

**The argument.** Three facts, each read off the code rather than off a comment:

1. **Rung selection is strictly grow-only.** The two-rung fragments
   (`JmpJsrSym`, `RelaxAbsSym`) advance only under `rung == 0` and only to `1`. The
   `RelaxLadder` arm advances by `rungs[si][fi].max(want)`. Nothing anywhere lowers a
   rung. Each fragment therefore advances at most `rung_count - 1` times, ever.
2. **`grew` implies an advance.** Every site that sets `grew` sits inside the branch
   that raised the rung, so the number of passes reporting `grew` is bounded by
   `Sigma(rung_count - 1)` over all fragments, which is exactly the `total_flips` the
   guard is computed from.
3. **`place_pass` is idempotent given fixed rungs.** A `Pinned` section takes
   `base = sec.lma` and writes it straight back, so a pin never moves. A `Chained`
   section takes its group cursor, and the cursor map is rebuilt from scratch on every
   call from the pins and the current rungs. The bank bump is a pure function of `base`
   and `final_size`. So with rungs unchanged, a second call produces the same lmas and
   reports no move.

Together: a pass that does not grow cannot move an lma either (an lma moves only when a
byte length changed, which requires a grow on the pass before), so a non-growing pass
converges, or at worst is followed by one settling pass that does. At most
`total_flips` growing passes, plus one settling pass, plus one converging pass. The
guard is `(total_flips + 2).max(PASS_GUARD_FLOOR)`. It is **derived from the input's own
flip budget**, so an input that needs more passes brings a larger budget with it. There
is no input that outruns it.

**The control, because the argument alone is not evidence.** Cutting the guard below the
budget (`.min(50)` appended to the guard expression, applied and quoted off disk) makes
the report fire on the cascade test below, with the new message text. Restoring the
guard makes it green again. So the path is live, it is reachable **only** by guard
exhaustion, and nothing else in the fixpoint routes to it. That is as close to
"constructed an input that produces it" as the surface permits: the only way to produce
it is to break the invariant that makes it unreachable.

**Consequence for the brief's test ask.** A test pinning the string via the real code
path is not available, and a test that pins an unreachable string would be worthless.
See section 4 for what was pinned instead.

## 2. The assembler's three-way split does not reproduce here

The ruling splits on: provable circularity, refused by name; otherwise settle with no
fixed count; report oscillation honestly. In the linker, **the first two of those three
classes have no members**, and the reason is the same monotonicity.

- **(a) Provable circularity: not expressible.** In the assembler, a `rept` count reads a
  symbol its own emission moves, and the value can move in either direction, so a
  self-referential expression can chase its own tail. Here, width selection is monotone:
  a fragment that grows can only ever push a later target **higher**, which can only ever
  cause another fragment to grow, which never comes back. There is no cycle to exhibit
  because there is no path by which a growth undoes itself. Implementing a circularity
  refusal in the linker would be a detector with a nonzero false-positive rate and a
  provably empty true-positive set, which is precisely the risk the owner named as worse
  than the status quo.
- **(b) Oscillation: not expressible.** The assembler detects oscillation by
  `env == prev_prev`, an environment revisiting an earlier state. Here the state is the
  rung vector, which is non-decreasing in every component. A repeated state therefore
  means an identical state, which is convergence. The relaxation state function cannot
  revisit an earlier state without having already terminated.
- **(c) The subject the message names.** Not symbols, and not a rung set as an
  abstraction. Two concrete things: the **fragment** (named by its section and pointed at
  by its span) with the two **encodings** it moved between, and the **section** with the
  two **load addresses** it moved between. Those are the linker's analogue of "which
  symbols were still moving and what they moved between".

The grow-only choice is deliberate and predates this parcel. The doc comment above
`resolve_layout` records why: asl 1.42's *bidirectional* relaxer is itself
non-terminating on the `0xFF_8000` sign-extension wrap, and sigil buys guaranteed
termination by giving that up. The ruling's refuse-half is aimed at exactly the failure
mode the linker already designed out.

**So the honest-report half is the whole of what was available, and it is what landed.**

## 3. What landed

`crates/sigil-link/src/relax.rs`:

- `place_pass` returns the sections that moved and the two addresses each moved between,
  instead of a bare `bool`.
- Each of the three rung-growth sites records the fragment's section, span, and the two
  encodings it moved between. The two-rung fragments name `abs.w` and `abs.l`; a ladder
  rung is named by the reach its fixup encodes and what it costs in bytes.
- Both records reset at the head of every pass, so the report describes the last attempt
  rather than the whole history.
- `unsettled_diag` builds the report. No count, and the message says plainly that
  reaching it means the grow-only invariant is broken inside the linker rather than that
  the author wrote something wrong, because section 1 is what makes that true.
- The `64` is hoisted to a named `PASS_GUARD_FLOOR` carrying the ruling citation, so the
  test derives its expectation from the constant rather than copying a literal.

The guard itself is kept, as the ruling directs. Only the number left the message.

## 4. What was pinned, given the string is unreachable

Two tests in `crates/sigil-link/src/relax.rs`, runner `cargo test -p sigil-link`.

- `unsettled_report_names_what_moved_and_never_an_attempt_count` drives the report
  builder directly, which is the one way to exercise the contract without depending on an
  unreachable path. It asserts the guard's own number is absent (derived from
  `PASS_GUARD_FLOOR`, not typed), that the words `passes` and `within` are gone, and that
  every moving subject and both of its values are present.
- `a_worst_case_growth_cascade_settles_inside_the_derived_guard` is the property that
  actually protects the user. A hundred `abs.w` operands whose labels descend two bytes
  apiece, so operand `i` crosses the `abs.w` boundary only after the `i` operands before
  it have each grown by two. The fixpoint can retire exactly one per pass and needs a
  hundred passes, settling in 101 of its 102 allowed. This is the guard-boundary test:
  it is what goes red if anyone makes width selection bidirectional, or replaces the
  derived guard with a fixed constant.

Both were proven red first, with the mutation quoted off disk before each red run, and
restored from the committed baseline.

## 5. Neighbouring surfaces (report only, the call is the owner's)

### The two `while` surfaces are a different class

`crates/sigil-frontend-as/src/eval.rs:5198` and `:5205`.

The discriminator that separates them from the ruling's class is **whose loop it is**.

The settle loop and the relax loop are the *tool's own* fixpoint search. The author wrote
no loop and no termination condition; convergence is entirely our business. So a count
there measures our effort and displaces the only thing the author could act on, which is
exactly the ruling's complaint.

A `while` is the *author's* loop, with the author's own termination condition. The honest
content of `:5198` is "your condition never became zero", and the iteration count is a
measurement of their construct, not of our attempts. It tells them the loop is
non-convergent rather than merely slow. `:5205` (`GLOBAL_WHILE_CAP`, 1,000,000 body
executions per pass) is even further from the ruling: it is a stated resource budget, and
the figure is the budget itself.

**Recommendation: different class, keep both numbers.** With one qualification I would
rather state than leave implied. `WHILE_CAP` is 10,000, and its doc comment justifies it
as "generous relative to any real `while`-driven table-fill idiom" without a corpus
measurement behind it, unlike `GLOBAL_WHILE_CAP`/`GLOBAL_REPT_CAP`/`GLOBAL_MACRO_CAP`,
which were all sized against measured s1/s2/skdisasm draws. So `:5198` is the one surface
where a legitimate program could plausibly sit above the cap and receive a message that
reads exactly like the AS complaint. If the owner wants the ruling's spirit extended
without losing the information, the move is additive rather than subtractive: keep the
count and also name the condition and the values its symbols held, so the author sees
what was not converging. That is a separate parcel and is not proposed here.

### The sweep, and a correction to its population

The brief's sweep used pathspec `crates/*/src` and reported zero files, then a filtered
form that found three. Both were checked:

| pathspec | hits |
|---|---|
| `crates/*/src` (the brief's first attempt) | 0 |
| `crates/**/src/**` | 18 |
| `crates` | 45 |

Widening the pattern past the brief's word list (to any `CAP`/`LIMIT`/`MAX`/`BUDGET`
constant interpolated into a message) found **a fourth surface the brief's sweep missed**:

- `crates/sigil-frontend-emp/src/parser.rs:4102`, `"{what} nesting too deep (max {MAX_EXPR_DEPTH})"`.
  Class B by the same discriminator: the nesting is the author's, and the depth is a
  property of their expression. Recommend no change.

Two more that the word list catches but that are not this class, recorded so the
population is closed rather than sampled:

- `crates/sigil-harness/src/rev_reachability.rs:316`, a timeout in seconds inside a
  `COULD NOT MEASURE` report. The number is the timeout, and the report is already loud
  on unmeasurable.
- `crates/sigil-frontend-emp/src/z80_cycles.rs:1113`, an internal assertion, not a
  user-facing diagnostic.

And one corroboration worth recording: the linker's *other* tool-owned bounded fixpoint,
`MAX_EQU_PASSES` in the same file, already reports by naming the subject
("unresolvable equ `X`: its first unresolved dependency `Y` is not defined") with no
count. So `relax.rs:1116` was the only non-compliant surface in the linker, and the
shape this parcel installs was already the file's habit elsewhere.

## 6. Left open

- **Two dangling `PASS_CAP` references.** `crates/sigil-frontend-as/src/eval.rs:5172` and
  `:8855` cite `PASS_CAP` in doc comments. The constant no longer exists: the assembler
  parcel that implemented this ruling removed it. Both are prose, not diagnostics, so no
  user sees them and this parcel did not touch the file. They are decay left by the
  ruling's own first implementation and want a one-line fix in an assembler-side parcel.
- **Runtime confirmation not attempted.** Nothing here changes emitted bytes (the
  diagnostic is unreachable and the guard's value is unchanged for every input), so no
  emulator check was warranted. Tagged rather than attempted, per standing invariant.
