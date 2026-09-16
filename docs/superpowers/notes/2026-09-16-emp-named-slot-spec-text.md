# The named slot: spec text for `SIGIL_SPEC2_LANGUAGE.md`

2026-09-16. Decision `d-33`, answered `named-slot` by the owner, which is the agreement the
propose-discuss-land ruling requires before `.emp` language surface lands.

**THIS FILE IS A DRAFT HELD IN SIGIL AND IT IS NOT THE SPEC.** The `.emp` language spec is
`docs/SIGIL_SPEC2_LANGUAGE.md` in **empyrean**, which is not this lane's to commit to. The
text below is written to be moved there through the hub, under §3 (declarations) and §10
(the one-page inventory). Until it is, the construct is specified here and in the code's
own doc comments, and empyrean's inventory is wrong in the way it was already wrong: see
*What this compounds* at the foot.

---

## §3.x `context` parameters and the `with` argument list

### The surface

A `context` declaration may take parameters. A `with` bracket may fill them.

```emp
pub context z80_stopped(interleave: Code = asm {}) {
    acquire = asm { move.w #$0100, Z80_BUS_REQUEST } ++ interleave ++ asm { .wait_z80:
                    btst   #0, Z80_BUS_REQUEST
                    bne    .wait_z80 }
    release = asm { move.w #$0000, Z80_BUS_REQUEST }
}
```

```emp
with z80_stopped(interleave: asm { move.w d7, (a2) }) {
    ...
}
```

and, unchanged and meaning exactly what it meant before the parameter existed:

```emp
with z80_stopped {
    ...
}
```

### The grammar

```
context-decl   ::= [ "pub" ] "context" NAME [ param-list ] "{" context-body "}"
param-list     ::= "(" [ param { "," param } [ "," ] ] ")"
param          ::= NAME ":" type [ "=" expr ]
with-stmt      ::= "with" NAME [ arg-list ] [ "if" comptime-expr ] "{" asm-stmt* "}"
arg-list       ::= "(" [ arg { "," arg } [ "," ] ] ")"
arg            ::= [ NAME ":" ] expr
```

`param-list` is the SAME production a `comptime fn` declaration uses, and `arg-list` is the
SAME production a call uses. Nothing here is a second spelling of an idea the language
already has: positional arguments come first and then named ones, a parameter with a
default is optional, and a parameter without one is required.

**NO NEW RESERVED WORD, and none was available.** §10's statement-leading reserved set is
closed; `with` is already a contextual opener (it fires only on `with` followed by an
identifier) and `context` is already an item-position opener. `context NAME (` can only be
a parameter list, because the only other thing that may follow a context's name is its `{`
body. `with NAME (` can only be an argument list, because the only other things that may
follow a bracket's context name are `if` and the body's `{`. The argument list parses
BEFORE the `if` gate: the arguments belong to the context, the gate belongs to the bracket.

### What it means

A parameter is in scope while the context's own `acquire` and `release` expressions
evaluate, **and nowhere else**. In particular it is NOT in scope in the bracket's body: a
context author adding a parameter cannot change what a consumer's body already meant.

The halves already evaluate at the USE SITE in the consumer's scope; a parameter scope is
pushed around each half and popped after it.

A `Code`-typed parameter defaulting to `asm {}` is the SLOT form. The three pieces it is
built from are older than this feature and are in daily use in the engine: `Code` as a
value, `asm {}` as the empty code literal, and `++` as the `Code` monoid's append.
Concatenating the empty literal appends nothing, so **a bracket that passes no argument
emits exactly the byte stream it emitted before the parameter existed.**

**THE CONTEXT AUTHOR DECIDES WHERE THE SLOT IS.** A consumer cannot inject code anywhere
except where the acquire or release expression splices the parameter. There is no
"anywhere" form and no way to ask for one.

### What it does NOT change

All three bracket proofs are unchanged, and the reason is that none of them reads the
source or a name: they range over the compiler's own marks, `(enter, exit)` for
`[context.escape]` and `[context.entry-skip]`, `(enter, acquire_end)` for the branch form
of `[context.reacquire]`. A slot's statements land strictly inside `(enter, exit)`, so:

* an `rts`, a fall-out, a branch out of the region or a tail call written in a slot is a
  path that skips the release, and fires `[context.escape]`;
* an exported label in a slot is an entry point that takes the hold's later half without
  taking the hold, and fires `[context.entry-skip]`;
* the nesting form of `[context.reacquire]` reads marks only and is untouched.

**The slot is spliced INSIDE the acquire range**, not after it, because the acquire is one
expression and the `AcquireEnd` mark is planted after the whole of it. That is the stronger
of the two available placements: `Region::in_acquire` is bounded by `acquire_end`, so the
branch form of `[context.reacquire]` covers the slot on this placement and would not on the
other.

### Authorship: whose code is it

A context's spliced `acquire`/`release` lines are authored to the CONTEXT: the context's SR
round trip is proven at its declaration, and `[proc.sr-undeclared]` therefore exempts them
at the consumer. **Slot code is not that.** It is the consumer's, and it carries the
authorship it would have carried written in the bracket's body:

* `[proc.sr-undeclared]`, `[proc.clobber-undeclared]` and the preserves model see it and
  charge it to the proc that wrote it;
* the two DEFINITION-SITE checks that scan the spliced acquire,
  `[context.rte-acquire-pushes]` and the context's own SR round trip, do NOT see it, so a
  consumer's push or SR write is never reported at the context's declaration span in
  another file;
* a value that came from a parameter's DEFAULT is the context author's code, written in
  the declaration, and is authored to the context like any other line of the acquire.

### A false comptime gate takes the slot with the acquire

`with ctx(slot: …) if COND { … }` with `COND` false lowers the body verbatim and splices
NEITHER half: there is no acquire, no release and no region, because the context is
genuinely not held in that shape. **The slot lives inside the acquire, so it goes with
it**, and the arguments are not even evaluated.

That is coherent once said (the gate's whole purpose is "this bracket does not exist in
that build shape") and it is the one place where a consumer's own statement disappears on
a condition the consumer wrote, so it is stated here rather than left to be met in an OFF
build. THREE of the corpus's 22 brackets carry such a gate today, measured here at aeon
`ec640bcf` (`section.emp:266`, `vblank.emp:137`, `vblank.emp:353`). The measurement note
says four, and its own parenthesis is what corrects it: the fourth site it counts,
`controllers.emp:39`, is annotated there as explicitly UNCONDITIONAL, so it is a bracket
with no gate.

### Diagnostics

| id | when |
|---|---|
| `[context.no-parameters]` | a bracket passes arguments to a context that declares none |
| `[context.slot-not-code]` | a `Code` parameter is handed a value that is not code |
| `[context.slot-dropped]` | the caller's argument carried instructions and the context's halves never splice that parameter, so the code would assemble into nothing |

`[context.slot-dropped]` is the one worth explaining. A context author who declares a
parameter and then never uses it silently swallows whatever a consumer passes. At a bus
hold that is the difference between the sound chip's reset line being released and not, so
it is an error. An argument that carries no instruction (`asm {}`, the default) is not
reported: nothing was dropped.

### Two limits, both pre-existing and both met immediately by anyone writing a slot

1. **A context field's expression cannot continue onto the next line.** `acquire = <expr>`
   is terminated by the line end, so a `++` chain must keep its top-level operators on one
   line, with newlines appearing only inside `asm { }` braces. This is general to every
   context field and is not caused by parameters.
2. **A label written inside an `asm { }` VALUE resolves within that value.** A bracket's
   body cannot branch to a label the slot defined, and a slot cannot branch to a label the
   body defined; the same is already true of the acquire's own `.wait_z80`. It is not
   accepted quietly: such a branch reads as a transfer out of the region and
   `[context.escape]` fires as an error.

### Why the slot is a `Code` VALUE and not a counted number of statements

The consumer that motivated `d-33` needs exactly one straight-line statement, and the card
says so. The narrow reading would be a slot that admits one statement and refuses two.
**That reading was considered and not taken, and the reason is that it is not the smaller
option.** The slot is a `Code` value, and `Code` is an existing value type with no arity
anywhere else in the language; capping it at one item would be a new restriction needing
new code, a new diagnostic and a rule with no principle under it. The card's own
construction ("code as a value, an empty code block, joining two of them") settles the
arity, and the restriction would be the addition rather than the omission.

What IS narrow, deliberately: there is no post-body slot as a distinct feature, no way for
a consumer to place code anywhere the context author did not, and no widening justified by
sites that might exist. There is one hand-spelled site in the engine and it was measured.

---

## §10 inventory line

§10's declaration inventory lists every declaration form the language has. It does not list
`context` at all, which predates this feature. The line it needs, with the parameter list
folded in:

> `context NAME [(params)] { acquire = …  release = … | released_by_rte | granted }`, a
> declared machine-state context, entered by the `with NAME [(args)] [if …] { … }` bracket.

---

## What this compounds, and what it does not create

`context` and `with` are **absent from empyrean's `docs/SIGIL_SPEC2_LANGUAGE.md`
altogether**, while 22 shipping sites in aeon use the bracket. The measurement note found
this at empyrean `origin/main` `cabaa0d8`; **re-verified here at `77092db4`**, which is a
LATER revision, so the gap is current and not a stale reading. The positive control is the
same: 21 lines of that file contain the substring "context" and every one of them is
"contextual opener", "contextual bareword" or "data context", never the item or the
bracket, and §10's declaration inventory at `:940` lists every declaration form the
language has without listing this one. Their specification lives in sigil, at
`docs/superpowers/specs/2026-08-03-contract-unification-spec.md` §3.1-3.2 and
`docs/superpowers/specs/2026-08-04-contract-delta-spec.md` §2.

So the construct was already undocumented at the suite level and this parcel does not
create that gap. It does make it wider by one form, which is why this draft carries the
whole construct's §3.x and §10 text rather than only the parameter list: whoever moves it
to empyrean should move the bracket too.
