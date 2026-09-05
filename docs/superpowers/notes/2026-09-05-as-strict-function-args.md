# Strict `function` arguments: the one shape where sigil was the more permissive assembler

2026-09-05 · branch `parcel/as-strict-function-args` · sigil master base `a8d3c5c1`

The fix half of **ASL-SILENT-WRONG-ON-BOTH-BUILDS**, whose measurement half is
`2026-09-05-asl-silent-decline-regime.md`. That note found sigil refusing 31 of
33 shapes and accepting two, **both from one cause**; this closes both. Sigil
now refuses 33 of 33.

Every number below was produced by a command, and each is named at the number.

---

## THE DEFECT, and why it is not the one the row was booked under

`substitute` pastes an argument's TOKENS into the body. An argument bound to a
parameter the body **uses** therefore reaches the evaluator as part of the
operand, and a bad one is already refused there, naming it — which is why
`#fu(zz)` was already loud. An argument bound to a parameter the body
**ignores** was never looked at at all:

```
fi  function p,$3C7          ; the body never mentions p
    move.w  #fi(zz),d0       ; zz defined NOWHERE  -> sigil emitted 30 3C 03 C7, exit 0
    move.w  #fi(a1),d0       ; a register in value position -> the same
```

asl (reference build, md5 `61e672562465725a8c102288a7da9098`,
`-xx -n -q -A -L -U -i .`) refuses the first with `error #1010: symbol
undefined`, exit 2. That row is the serious one: **no register is involved, asl
is LOUD, and sigil was the more permissive of the two.** It is the only such
row in the whole 33-shape regime.

AS's manual states the rule (`doc_EN/as.tex`, `FUNCTION`): *"all parameters are
calculated once and are then inserted into the function's formula"*, naming
integer, float and string as the types that have such a form. A register is not
one of them and an undefined symbol has no value to calculate.

---

## THE THREE GROUNDS, RE-VERIFIED

The brief's grounds for this being a sigil-internal parcel rather than a
byte-mover. All three hold; the aeon one is re-verified two ways.

**1. It is the AS-compatibility surface, not `.emp`.** The expansion is
`crates/sigil-frontend-as/src/eval.rs`; `.emp` has its own evaluator at
`crates/sigil-frontend-emp/src/eval/call.rs`. Separate crates, and nothing in
this parcel touches the second.

**2. It cannot move aeon bytes.** aeon (`ce0ac25b`) routes three `.asm` files
through this frontend — `engine/debug/debugger.asm` and both `game_root.asm`.
Grep for a `function` DEFINITION in all three and in everything they include:
**zero**. The four `function` hits in `debugger.asm` are the English word, in
comments. Then the same claim as a measurement rather than a grep — both aeon
ROM shapes rebuilt with the post-change assembler:

```text
REBUILD CONTROL s4.bin           1c09fbfc/819131 MATCHES THE GOLDEN
REBUILD CONTROL s4.debug.bin     e2144057/840324 MATCHES THE GOLDEN
repin --check                    pins.rs unchanged
```

`scripts/provision-aeon-ref.sh`, then the `repin --check` it prints. The second
line is the positive witness (invariant 8): a tree that could not reproduce the
pinned revision's placement cannot leave the pin file unchanged.

**3. Corpus population is zero.** `corpus.sh`, the committed instrument, run
over four trees rather than the note's two:

| tree | git | `.asm` | `function` defs | **param-ignoring** | `f(reg)` as a CALL | `dc.x reg` | `#name(reg)` |
|---|---|---|---|---|---|---|---|
| `s1disasm` | `f6ece65` | 455 | 25 | **0** | 0 | 0 | 0 |
| `s2disasm` | `e45ebf3` | 332 | 49 | **0** | **0** | 0 | 0 |
| `skdisasm` | `2fcd861` | 909 | 42 | **0** | 0 | 0 | 0 |
| `aeon` | `ce0ac25b` | 360 | 0 | **0** | 0 | 0 | 0 |

116 `function` definitions, not one with a body that ignores a parameter. The
`289` in s2disasm's raw P2 count is `id(aN)`, all in one file, all peeled as
addressing modes — P2b, the discriminator, is 0. No directory came back
UNMEASURABLE.

---

## WHAT CHANGED

`crates/sigil-frontend-as/src/eval.rs`, and nothing else in the assembler.

- **`check_call_args`** walks the operand tokens before expansion and, for each
  argument bound to a parameter the body ignores, calculates it once for its
  diagnostic alone and discards the value. It recurses into arguments AND into
  the substituted body, so a body that spends its parameter on another ignoring
  call is covered (`hu function p,gi(p)` over `gi function q,$100` — that is a
  test, not a hypothetical).
- **`check_ignored_arg`** is the per-argument calculation, and holds the
  register diagnostic.
- **`expand_calls_checked`** is the wrapper every REPORTING expansion site now
  uses (`eval_all`, `expand_operand_builtins`, `directive_db`,
  `directive_equate`, `directive_set`, `float_rhs`,
  `expand_calls_m68k_operands`). `fold_const` and `fold_str_as_expr` keep the
  plain silent `expand_calls`: they are the deliberately-silent counterpart, and
  a `substr` bound that does not fold is a `None` there, not an error.
- **`arg_faults_seen`** dedupes by argument span within a pass. A call reached
  through a `rept` or a macro is expanded more than once from one source
  position, and the verdict on an argument is a property of that position.
- `expand_calls_m68k_operands` checks the SAME slices it expands, so a
  held-back `dsp(a1)` stays an addressing mode. That is the `disp-or-call`
  parcel's peel and it is untouched — its four tests still pass.

**The expansion itself is byte-for-byte untouched.** An argument that calculates
produces exactly the tokens it produced before; the new code only reads.

### Why "arguments bound to an IGNORED parameter" and not "all arguments"

Because a used parameter's argument is already calculated — by substitution, in
the operand, once. Reporting it a second time at the call would say the same
thing twice about one line, which is a defect the measurement note already books
for `#$1234,fu(a1)`. The two halves together are what makes every argument
evaluated exactly once, which is the manual's rule. **The visible consequence is
a message asymmetry, and it is stated in WHAT IS LEFT OPEN rather than hidden.**

---

## THE TWO DIAGNOSTICS, exact text

```text
reg.asm(5): error: `a1` is a register: a function argument must be an integer,
                   floating point number or string
undef.asm(5): error: unresolved symbol `zz` in operand
```

The second is verbatim what `#fu(zz)` (body USES the parameter) already said,
which was the landing condition: the two shapes now answer in the same place
with the same words. Exit 1, one diagnostic each, no duplication.

The first is new. `unresolved symbol \`a1\`` is a true statement about sigil's
symbol table and a false one about the program — a register is not a definition
anyone forgot. asl's own catalogue (`as.msg`, a data file beside the binary)
carries *"expected integer, floating point number or string but got register"*;
this says the same thing, naming the register first because that is the word the
reader needs.

Both render `file(line):` — the AS surface's form. The `.emp` surface's
`path:line:col:` is a standing ruling and nothing here touches it.

**The register set is the MEASURED one, not the plausible one:** `a0`–`a7`,
`d0`–`d7` and `sp`, case-folded, 68000 only. `pc`, `sr`, `ccr`, `usp` and any
`a1.w` are `#1010 symbol undefined` on asl and stay on the unresolved-symbol
path here; z80 register names never reach asl's expression parser, and a z80
program is free to define a symbol called `sp`.

---

## THE 33-SHAPE TABLE, BEFORE AND AFTER

`sigil_today.sh <sigil> 1 | classify.py`, same instrument both times, same
reference asl. The whole diff:

```diff
- imm_fn_reg_body_ignores        SILENT 0  303C A101  accept  30 3C A1 01 30 3C 03 C7 30 3C A2 02
+ imm_fn_reg_body_ignores        SILENT 0  303C A101  refuse  error: `a1` is a register: a function…
- imm_fn_undef_arg_body_ignores  LOUD   2  303C A101  accept  30 3C A1 01 30 3C 03 C7 30 3C A2 02
+ imm_fn_undef_arg_body_ignores  LOUD   2  303C A101  refuse  error: unresolved symbol `zz` in operand
- asl SILENT and sigil ACCEPTS (neither refuses): ['imm_fn_reg_body_ignores']
- asl LOUD and sigil ACCEPTS (sigil the more permissive): ['imm_fn_undef_arg_body_ignores']
+ asl SILENT and sigil ACCEPTS (neither refuses): []
+ asl LOUD and sigil ACCEPTS (sigil the more permissive): []
```

Two rows. **The other 31 shapes and all 7 controls are byte-identical rows** —
that is a `diff` of the two classified tables, not an eyeball. The controls all
still accept, with the same bytes, which is what keeps the table a measurement.
`listings whose pass loop stopped early: 0` on both runs.

---

## THE CORPUS NULL RESULT, AS ACTUALLY RUN

A zero predicted from `param-ignoring: 0` would be a zero nobody ran. So both
binaries were built and put to the same files:

```text
witness: OLD accepts #fi(zz) (exit 0), NEW refuses it (exit 1) — two distinct tools
OLD md5 51bdbaa988d8903dfe4bf19abea4fc50   (a8d3c5c1, via `git archive`)
NEW md5 ea26467e549da337f4949523977b725d

s1disasm  files=455  differ=0
s2disasm  files=332  differ=0
skdisasm  files=909  differ=0
aeon      files=6    differ=0
TOTAL files=1702  identical=1702  DIFFER=0
```

Compared per file: emitted bytes, every diagnostic, and the exit code. A file
neither tool assembles is still compared — what is being measured is that the
two tools AGREE, not that either succeeds.

**The witness line is the part that matters.** 1702 identical pairs is equally
consistent with having run the same binary twice, so the sweep refuses to start
unless the old binary ACCEPTS `#fi(zz)` and the new one REFUSES it. That check
is in the script, ahead of the loop, and it is why this is a null result rather
than an absence.

(1702, not the measurement note's 2056: that count included aeon's 354 agent
worktree copies of the same three files. Both counts are of the same three
files.)

---

## THE RED-FIRST PROOF, WITH THE MUTATION ON DISK

`crates/sigil-frontend-as/tests/as_strict_function_args.rs` was committed ALONE,
in `a760c1d6`, on sources identical to `a8d3c5c1` — `git diff --stat a8d3c5c1`
over the tracked tree was empty at that commit, so the run below is the
committed baseline's behaviour and not a mutation that failed to apply:

```text
test an_argument_that_calculates_is_unaffected_whether_or_not_the_body_uses_it ... ok
test an_undefined_symbol_is_refused_through_a_nested_ignoring_call ... FAILED
test an_undefined_symbol_is_refused_even_where_the_body_ignores_it ... FAILED
test a_register_argument_is_refused_as_a_register_not_as_a_missing_symbol ... FAILED
test a_forward_reference_in_an_ignored_argument_still_assembles ... ok
test result: FAILED. 2 passed; 3 failed
```

All three failures are `expected a refusal, the source assembled`, which is the
defect itself and not a mis-shaped assertion. **The two PASSING rows are the
other direction of the bar** and are as load-bearing as the failing three: an
argument that calculates, and a forward reference, must keep assembling —
strictness that refused those would be a regression, and the corpus is made
entirely of the first shape. After the change: `5 passed; 0 failed`.

The file's module doc states, per test, what each MUST FAIL on.

---

## SUITE AND GATES

```text
cargo test --workspace --release, AEON_DIR=/home/volence/sonic_hacks/.aeon-ref
  397 test binaries   4556 passed   0 failed   2 ignored   exit 0
cargo clippy --workspace --all-targets   0 warnings, 0 errors
```

**Zero rows left unmeasured.** The reference tree was provisioned
(`scripts/provision-aeon-ref.sh`) and witnessed by `repin --check` →
`pins.rs unchanged`, so the reference-dependent rows RAN rather than being
refused by ruling d-18.

The two ignored rows are pre-existing and named rather than counted:
`sigil_diff_reports_byte_identity` (`--ignored`, reads the aeon source tree) and
`secondary_pin_classes_match_the_hand_typed_baseline` (retired by Wave-B B-0).

`cargo fmt --check` is not a gate in this tree and was not made one: the
untouched baseline reports 6586 diff hunks, 107 of them in `eval.rs` alone.
This parcel adds one.

---

## WHAT IS LEFT OPEN

- **The message asymmetry, stated plainly.** `#fi(a1)` (body ignores) now says
  *"`a1` is a register"*; `#fu(a1)` (body uses) still says
  *"unresolved symbol `a1`"* — the same fault, two messages, and the second is
  the misleading one. Closing it means suppressing the downstream operand report
  when the argument-level check has already spoken, which needs `substitute` to
  see the fault; `expand_calls`/`substitute` are `&self` and the `&mut` cascade
  runs through the whole string-builtin subtree (`fold_const`,
  `fold_str_as_expr`, `eval_str`, `eval_substr`, `expand_str_builtins`). Not
  attempted here: the brief scoped the other 31 shapes to unchanged, and doing
  it half-way would have produced two diagnostics for one operand. **This is the
  measurement note's recommendation 2, and it is only half done.**
- **A non-integer ignored argument reports nothing.** `check_ignored_arg`
  returns silently when the argument does not parse as an integer expression (a
  bare float literal, say). asl has its own diagnostics for those shapes and
  they are reached through the substituted body when the parameter is used;
  under an ignored parameter they are simply not reported. Population is zero
  and no shape in the 33 covers it.
- **`#nofn(a1)`** — an undefined function name — still answers `trailing tokens
  in #immediate` where asl says `#1860 unknown function`. Unchanged by this
  parcel, already booked by the measurement note.
- **The duplicated diagnostic** for `#$1234,fu(a1)` and `1+fu(a1)` is untouched.
  A dedup at poison-promotion time would fix it and would also have changed two
  rows of the 33-shape table, so it was left for its own parcel.
- Everything the measurement note left open (the pass-loop suppression sweep,
  the `asl-reference/README.md` corrections) is still open. This parcel touched
  neither file.

## WHAT IN THE BRIEF TURNED OUT WRONG

Nothing material. Three refinements:

- The brief's *"evaluate every argument at the call, whether or not the body
  mentions the parameter"* is the right RULE and the wrong IMPLEMENTATION note:
  evaluating a used parameter's argument at the call as well would emit the
  diagnostic twice, because substitution already routes it to the evaluator. The
  rule is delivered by checking exactly the ignored ones. Stated above rather
  than quietly done.
- *"aeon routes three residual `.asm` files"* — correct, and the `find` that
  returns 360 is counting 354 agent-worktree copies of those same three.
- The brief's *"the same laziness is why we also accept `#fi(a1)`"* is exactly
  right, and it is worth recording that the fix confirmed it: one change closed
  both rows, with no second mechanism.
