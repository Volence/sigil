# D-25: `0` as a Label, and a default that is finally class-checked

Parcel `EMP-LABEL-ZERO-UNIMPLEMENTED`, branch `parcel/emp-label-zero`, based on
master `a25c08bb`. The owner answered `d-25` "accept" on 2026-09-04
(`142709bc`) and nothing was ever built. This is that build.

The answer card carries an `answer_note` recording that the owner deferred to
this lane rather than judging the argument, so the consistency reasoning under
the ruling is ours. That made deriving the acceptance predicate the real work,
not the edit.

## The predicate: zero, not "any integer"

The ruling's argument is that `0` is already the blessed empty-slot spelling at
the comparison surface, so refusing it at the argument surface was the
inconsistent half. The obvious way to act on that is to copy the comparison
surface. That would have been wrong, and the two rules are genuinely different.

`eq_compatible` (`eval/expr.rs`) has this arm:

```rust
(Value::Label(_), Value::Int(_)) | (Value::Int(_), Value::Label(_)) => Ok(())
```

It permits a label beside ANY int, which reads like a blanket blessing. Its own
doc comment says why it is written that way:

> **label vs int**: `0` is how `.emp` spells an absent symbol in a pointer slot
> (`variants: [Variant_Water_Deep, 0]`), so `slot == 0` is the ordinary
> emptiness test and must answer `false` for a real label rather than refuse.

That arm answers "is this comparison MEANINGFUL", and the surrounding contract
(D-EQ.1) is explicit that a pair is comparable when some pair of values of those
kinds could come out equal. `slot == 5` is meaningful: it answers `false`, a
real answer about a real value. The arm is coarse because a mismatch there still
has somewhere to land.

At the argument surface there is nowhere to land. The value does not get
compared, it BECOMES the argument, so it has to inhabit the type. `0` is the one
integer the language gives a label meaning; every other integer names nothing.
Accepting all of them would have deleted the class check and bought no spelling
anyone wants.

So "accept `0`" and "match the comparison surface" are different rules and only
the first is the ruling. Implemented as `matches!(v, Value::Int(0))`.

**Accepted:** the literal `0`, a named `const` equal to 0, and any constant
expression folding to 0. The test is on the folded VALUE. A rule reading only
the bare token would refuse the readable spelling of the very sentinel it exists
to bless (aeon spells its sentinels `PATCH_ANCHOR_NONE`, `ANCHOR_MOTION_NONE`,
`RASTER_PROGRAM_NONE`) and would put the two surfaces back out of step, which is
the defect being closed.

**Refused:** every non-zero and negative integer, `Float` `0.0`, and a newtype
wrapping 0. `Typed` beside `Label` is already refused at the comparison surface,
so refusing it here keeps the two surfaces agreeing rather than trading one
inconsistency for another.

The refusal's diagnostic now names the rule when, and only when, the value is an
integer, since that is the near miss the reader can act on.

## The half nothing could reach: a parameter's default

`check_arg_class` had exactly two call sites, both inside the argument loop. The
default-fill branch bound its value with no class check of any kind. That is the
whole defect the card describes: `hand: Label = 0` read as legal, was written
down in ten aeon sites, and none of them had ever been run.

Two changes, and the second is not optional:

1. The bound default is passed to `check_arg_class`, at the PARAMETER's own
   span. The default is written in the declaration, so that is where the reader
   has to go to fix it.
2. The default is evaluated through `eval_call_arg` rather than `eval_expr`, so
   it means what the identical text would mean written at the call.

Without (2), a bareword outside `label_ctx` is not a label value, so `0` would
have become the ONLY writable `Label` default: `Label = 0` legal,
`Label = Some_Real_Proc` an error. Half 2 on its own would have introduced a
fresh instance of exactly the inconsistency this ruling closes. Together the two
are strictly better than the old path in both directions, because a bareword
default that becomes a label in a non-`Label` slot is now caught by the check
that did not previously run.

**What the old path did**, per the standing rule that a change which newly
refuses must state the old behaviour: it evaluated the default in a fresh
global-only env and pushed the value into the slot. No class check, no label
context. Nothing on that path could refuse anything.

## The aeon ripple, enumerated rather than counted

Enumeration parameter: every parameter declaration in every `comptime fn`
signature in every `.emp` file in the aeon reference tree (`ec640bcf`), filtered
on declared type. 202 files, 364 signatures, 623 parameters.

`Label`-typed parameters carrying a default, the complete set (13):

| site | parameter | default |
|---|---|---|
| `engine/effects/preset.emp:141` | `parallax: Label` | `0` |
| `engine/effects/preset.emp:141` | `raster: Label` | `0` |
| `engine/effects/preset.emp:141` | `patched: Label` | `0` |
| `engine/effects/preset.emp:141` | `cycle: Label` | `0` |
| `engine/effects/preset.emp:143` | `variants: [Label; 2]` | `[0, 0]` |
| `games/sonic4/data/generated/ojz/act1/effects_scenes.emp:341` | `hand: Label` | `0` |
| `.../effects_scenes.emp:396` | `hand: Label` | `0` |
| `.../effects_scenes.emp:442` | `hand: Label` | `0` |
| `.../effects_scenes.emp:471` | `hand: Label` | `0` |
| `.../effects_scenes.emp:477` | `hand: Label` | `0` |
| `games/sonic4/test/scene_equiv_proof.emp:195` | `deform_fg: Label` | `0` |
| `games/sonic4/test/scene_equiv_proof.emp:195` | `deform_bg: Label` | `0` |
| `games/sonic4/test/scene_equiv_proof.emp:195` | `v_deform_bg: Label` | `0` |

Every one defaults to `0` or `[0, 0]`. The five `effects_scenes.emp` rows are
generated, and `tools/effects_gen.py` hard-codes `hand: Label = 0` in all five
templates, so the generator cannot emit another shape either.

The newly-refusing population is not just the `Label` rows, because the check
fires on both classes in both directions. Widening to all 83 defaulted
parameters of any type: none evaluates to a `Label` or a `Reg`. No `Reg`
parameter has a default anywhere in the tree. The three named-constant defaults
were checked individually and are all `pub const` ints
(`PARALLAX_ANCHOR_NONE = $FF`, `ANCHOR_MOTION_NONE = 0`,
`PATCH_ANCHOR_NONE = $7FFF`).

So nothing in aeon newly refuses, which is what the ruling predicted.

## Out of scope, deliberately, and named

The card's `recommend` mentions "a related hole on our side where the check is
silently skipped for lists of labels". That is array ELEMENT typing, and it is
not an oversight to sweep up here: `docs/EMP_PITFALLS_EQUALITY.md` section 13
already states the scope as settled, "Array LENGTH only ... A signature
annotation still says nothing about element TYPES". Closing it is a separate
refusal with its own aeon census (`preset()`'s `variants: [Label; 2] = [0, 0]`
sits directly in its path) and belongs in its own parcel with its own decision.

`param_type_is_label` matches a single-segment `Named` path spelled exactly
`Label`, so `[Label; 2]` reaches no class check in either direction today. That
is unchanged by this parcel and is why the array default above is inert rather
than blessed.

## Verification

14 tests in `crates/sigil-frontend-emp/tests/label_values.rs`, all named in the
landing log. The positive and negative directions are asserted separately,
because accepting every integer would satisfy every positive row on its own.

Four mutations, each shown applied on disk before its red run and each restored
from the committed baseline:

| mutation | red |
|---|---|
| `Value::Int(0)` becomes `Value::Int(_)` (accept all ints) | 4 tests |
| the default's `check_arg_class` call deleted (half 2 removed) | 4 tests |
| the `if !is_null_label(v)` guard dropped (half 1 removed) | 6 tests |
| the default's `eval_call_arg` back to `eval_expr` | 2 tests |

## A correction to the parcel brief

The brief asserted that no test anywhere in the workspace guards the
`expected a label` refusal, and concluded that loosening it to accept all
integers would pass the entire existing suite. That is wrong.
`tests/label_values.rs:556 int_into_label_param_is_type_error` passes
`routine 5` and asserts on the resulting error, so the accept-all-integers
half-fix WOULD have gone red. The first mutation above confirms it directly:
that pre-existing test is one of the four it reds.

The brief's grep was for the diagnostic's TEXT; the test asserts on the type
name `Label`. A search for a message misses a test that guards the message's
behaviour, which is the same shape as the standing rule that a property proven
at the producer is not a property of the consumers.
