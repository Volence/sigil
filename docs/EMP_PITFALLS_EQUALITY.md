# Hand-off for aeon `docs/EMP_PITFALLS.md` — comptime equality and signature annotations

**This file is a DRAFT FOR AEON TO LAND, not sigil documentation.** The pitfalls doc lives
in the aeon tree and this lane cannot commit there. The text below is written to drop in as
a new numbered section, sited next to §3 (whose always-*green* trap is the mirror image of
what §12 describes) and after §10's inversion rule, which it leans on.

It documents behaviour that CHANGED in sigil `parcel/comptime-compare-refuses`
(`eval_equality`, `walk_sig_arity`). Land it in the same window the sigil branch lands, or
the doc will describe a compiler nobody is running yet.

The two DEFINED cross-kind comparisons in §12 are the part that most needs to be written
down: the ruling that authorised this parcel barred "a comparison that is always one value"
but allowed a *defined* comparison provided the semantics are documented where `.emp`
pitfalls are. `label` beside `0` is always false, deliberately, and this is where that is
recorded.

---

## 12. Comparing two things that can never be equal — the always-RED guard

**Trap (measured 2026-09-02, item-5 comptime probe, Q2-e / Q2-D4):** comptime `==`/`!=`
used to be **total** — any two values of different kinds were "simply not equal", with no
diagnostic. That reads as permissive. It is the opposite: it silently converts a mistake
into a **constant**, and a guard built on a constant is not a guard.

Two shapes were hit live, and they are the same defect with opposite signs:

```emp
pub data Variant_Water_Deep: pal_variant = variant(shift_r: 1, shift_g: 1)

// ALWAYS RED. The bareword resolves to a LABEL inside an array literal, and
// label != struct was always true, so this reported "index 0" for the twin that
// AGREES. No value of either side could have made it pass.
ensure(first_mismatch([Variant_Water_Deep], [variant(shift_r: 1, shift_g: 1)]) == -1, "...")

// ALWAYS GREEN-ISH. Two different struct types compared FALSE instead of
// refusing, so a typo'd constructor read as an ordinary mismatch, not a type error.
ensure(variant(shift_r: 1) == cycle_channel(line: 2, first: 8, count: 4, period: 8), "...")
```

The probe discriminated the first three ways, and that triple is what proves "always red"
rather than merely "wrong": the equal twin with `== -1` fired; the unequal twin with `== -1`
fired; the equal twin with `== 0` **passed**.

**What sigil does now.** Equality is defined WITHIN a comparison class and REFUSES across
classes, with `[eq.cross-type]` naming both types and saying which constant the comparison
was stuck at:

```
[Error] [eq.cross-type] `!=` not defined for label `Variant_Water_Deep` and struct `pal_variant`
        — no value of one can equal a value of the other, so this comparison is always true;
        compare same-typed values (or their fields)
```

Aggregates recurse, so the refusal lands whether the two values meet directly (`[a] == [b]`)
or one element at a time inside `first_mismatch`.

**Nothing meaningful was taken away.** These all still ANSWER, and answering is correct:

| Comparison | Answer | Why it is not a mistake |
|---|---|---|
| two `pal_variant` values | `true`/`false` | same class; the CONTENTS decide |
| `[1,2] != [1,2,3]` | `true` | different lengths are a real difference |
| two variants of one enum | `false` | the enum has both variants |
| **`label` vs `0`** | `false` | **DEFINED — see below** |
| **`Angle(5) == 5`** | `true` | **DEFINED — a newtype erases to its stored int (§8.3)** |

### The two cross-kind comparisons that stay DEFINED

Everything else across classes refuses. These two do not, on purpose, and both are always
false — so they are the exception the "never always-one-value" rule makes room for, and
they are recorded here because that is the condition of making it:

1. **`label` beside `0`.** `0` is how `.emp` spells an absent symbol in a pointer slot —
   `preset(variants: [Variant_Water_Deep, 0])` is the ordinary spelling of "one variant, and
   the second slot empty". A real label is never `0`, so `slot == 0` is always false, and it
   is still the emptiness test you want. Refusing it would fire on correct code all over the
   effects tables.
2. **A newtype or `fixed<>` value beside a bare int.** `Angle(5) == 5` compares stored ints,
   which is §8.3's erasure and predates this change. Two DIFFERENT newtypes do NOT compare —
   `Angle(10) == Pos(10)` is the cross-type mistake and now refuses, naming both.

### The shape to reach for instead

Unchanged from the probe's Q2-f, and it is now the ONLY shape that works rather than merely
the tidiest: hold the value in a module-level `const` and feed BOTH the emitted twin and the
guard from it.

```emp
const WATER_DEEP = variant(shift_r: 1, shift_g: 1)
pub data Variant_Water_Deep: pal_variant = WATER_DEEP
ensure(WATER_DEEP == variant(shift_r: 1, shift_g: 1), "water-deep twin drifted")
```

`WATER_DEEP` is a struct VALUE on both sides, so the comparison is in-class and the guard
can genuinely fail. Naming the `pub data` symbol was never comparing values — it was
comparing an address to a struct.

**Rule:** if a guard compares two things and you cannot name a change to either side that
would flip the answer, it is not a guard. Since 2026-09-02 sigil says so itself; before
that date, and in any tree still on an older sigil, §10's inversion is the only thing that
would have caught it — and it is still the check worth running, because the compiler can
only refuse comparisons that are constant by KIND, never ones that are constant because you
compared a value to itself.

---

## 13. A `comptime fn` signature annotation is a length contract (and used to be decoration)

**Trap (measured 2026-09-02, item-5 comptime probe, Q1-L):** a `[T; N]` annotation in a
`comptime fn` signature checked NOTHING. A `[Label; 2]` parameter accepted a three-element
argument and reported `v.len == 3`; the return annotation was read by nothing in the
compiler at all. The wrong length surfaced only later, when a record built from the value
was emitted — with the error blamed on the **consumer's** `pub data` line:

```
[Error] array length mismatch: expected 2 element(s), got 3
        @ the whole `pub data OJZ_Preset_Sec3: EffectsPreset = preset(...)` line
```

That line is innocent. Its author supplied none of the three elements.

**What sigil does now.** Array lengths in a comptime fn signature are checked at the
signature: a parameter at the CALL, naming the fn and the slot; a return at the fn that
returned it.

```
[Error] array length mismatch: expected 2 element(s), got 3 — parameter `hand` of
        `probe_variants_pair` is declared with a fixed length
```

**Scope, deliberately narrow.** Array LENGTH only, exactly like the `const` half of the
same contract (sigil's `const_arity`). A signature annotation still says nothing about
element TYPES, and a parameter still binds loosely when the argument is not an array at all
— so `hand: array` remains the way to say "any array", and `[Label; 2]` now means what it
looks like.

**Rule:** spell the length when you mean it. `-> [Label; 2]` is now worth writing, because
it fails at the fn that broke it instead of at whoever eventually emitted the result.
