# `out(dN: T)` — the width a register result claims

The design record for the typed-out width rule: what was ruled, what was
measured, and the two questions the brief left open.

## The shape

A bare `out(rN)` claims all 32 bits of `rN`. A typed `out(dN: T)` claims the low
`sizeof(T)` bytes and nothing above them. Production credit is
**`produced >= required`**: a write at the declared width or wider proves the
claim; a narrower one does not.

`out(dN: T)` is not new grammar. The parser has carried `out_types: Vec<(String,
Type, Span)>` since the G5 typed-slot work, and two corpus procs were already
declaring one (`Section_FlatIDXY out(d0: SectionId)`, `EntityWindow_EntryForSection
out(d0: EntryRef)`). What was missing was a consumer: `out_verify` never asked
what the type was. The only grammar change in this parcel is composition —
`out(d0: u16 if eq)` now parses, because the typed arm consumed the type and then
left `if` sitting where a separator had to be.

## A TYPE, not a width suffix

The clause takes `u8` / `u16` / `SectionId` / `fixed<8,8>`, never `.b` / `.w`.

The reason is not aesthetic. A typed out slot has room for exactly ONE
declaration, and the domain type is already living in it — `out(d0: EntryRef)` is
what the `[call.slot-type-mismatch]` slice reads to check the caller's
`assume_some!`. A separate width spelling would have to sit BESIDE the domain
type, which means two authorities for how wide an `EntryRef` is, free to disagree.
Making the type carry its own width gives one authority and no second thing to
keep in sync.

### The spelling asymmetry against `preserves` is deliberate

`preserves` takes a FACET (`preserves(d3.w)`, `preserves(sr.mask)`); `out` takes a
TYPE. They look like they should harmonize. They should not, because the two
clauses say different kinds of thing about their register.

`preserves(d3.w)` is a claim about a REGION OF STORAGE surviving a call: the
caller's low word comes back, the high word may not. There is no value and no
domain — the register's content is the caller's, and the claim is only about which
of its bits are still there. A facet is the whole content of that claim, and a type
would be a lie: the callee neither knows nor cares what `d3` means.

`out(d0: EntryRef)` is a claim about a VALUE the callee produced. Its width is a
consequence of what the value IS, not an independent fact — an `EntryRef` is two
bytes because `EntryIndex` is `i16`, and a declaration that could say
`out(d0.l: EntryRef)` would be able to say something false. So `out` names the
value and reads the width off it; `preserves` names the bits because there is no
value to name.

Stated as a rule: **`preserves` describes bits, so it spells bits; `out` describes
a value, so it spells the value.** Do not harmonize them.

## Do newtypes narrow? YES — and it is forced, not merely convenient

`SectionId = u16` narrows to two bytes; `EntryRef = EntryIndex ? -1` →
`EntryIndex = i16 where 0..3` → two bytes, transitively; `Coord = fixed<16,16>`
stays four.

The measured evidence, as a set diff over the corpus residue, with no aeon
declaration changed at all:

| | residue | diff vs the 30-row baseline |
|---|---|---|
| narrowing OFF (before the mechanism) | 30 | — |
| narrowing ON, zero declarations touched | **29** | `EntityWindow_EntryForSection :: d0` REMOVED, nothing added |

One row, and it closes because the author had ALREADY written the type. That is
the attractive half. The forcing half is what happens under the alternative: with
newtypes not narrowing, the ONLY way to give
`EntityWindow_EntryForSection :: d0` a width would be replacing `EntryRef` with
`i16` — deleting the domain type that four call sites' `assume_some! d0,
EntryIndex` depends on, and that the niche-option check exists to police. A design
where stating a width costs you your domain type is not a design; it is a trap.

The risk this creates is real and was measured rather than argued: narrowing can
silently WEAKEN a claim that already verifies at 32 bits. Exactly one corpus site
is exposed — `Section_FlatIDXY :: d0`, whose `moveq #0, d0` produces a long under
a `SectionId` (word) claim. It stays verified, and the weaker claim is the
HONEST one: the value is `sec_y * grid_w + sec_x` in a word.

`EntryRef` was treated with the care its `? -1` sentinel deserves rather than
assumed. Its two arms are `moveq #-1, d0` (a long) and `move.w d1, d0`; all four
callers discriminate with `tst.w d0` / `bmi`, which reads `$FFFF` on the sentinel
arm and `$000X` on the payload arm. The niche is entirely a word-level niche, so
narrowing to the payload's width is exactly right — narrowing to a BYTE would not
have been, and the width comes from `EntryIndex`'s `i16`, not from the sentinel's
magnitude.

**An underivable type answers `L`** — a struct, an enum, an array, an unknown
name. That is the conservative direction: the bare claim is the strongest one, so
a typo in a type name cannot quietly relax a contract.

## RMW: measured, not tasted

The brief said to start from "defining writes produce, RMW does not" and extend
only on evidence. Both rules were implemented and the corpus measured under each,
with the adoption in place:

| production rule | residue | rows that close |
|---|---|---|
| pure width (a write of the declared width or wider) | **16** | all the width-gap rows an adoption reaches |
| defining writes only below `.l` | **20** | 5 fewer |

The five rows that close ONLY through an RMW write, by set diff:
`Collision_Probe{Down,Left,Right,Up} :: d0` (`add.w d3, d0` / `neg.w d0` on the
`.full_back` path) and `Emit_ObjectPieces :: d5` (`addq.b #1, d5`).

**Ruled: pure width.** The evidence is one third of the target set, but the
argument is what settles it. The module's `.l` rule has ALWAYS credited RMW —
`add.l #1, d0` produces `d0` today and always has, because all 32 bits are written
on this pass. Applying a defining/RMW distinction only BELOW `.l` would make
"produce" mean two different things depending on the declared type, with the
stricter meaning applied precisely where the claim is WEAKER. That is incoherent,
and it is the kind of incoherence a later reader resolves in whichever direction
is convenient.

The brief's guard — "never let RMW alone count as production from nothing" — is
honoured, but by the transfer function rather than by a mnemonic list. Width is
tracked per register as a lattice and production is **gen-only across widths**: a
write at width `w` raises the recorded width to `max(recorded, w)` and a later
narrower write retracts nothing. So `addq.b #1, d5` credits one byte and one byte
only; it can never manufacture the claim above it. What it does credit is
d5's low byte holding a value written on this pass, which is exactly the property
`out` states and exactly what an inline `move.b` beside it would credit.

Where an RMW-only production is genuinely not a good enough contract, the answer
is the ADOPTION bar, not the production rule — see `Emit_ObjectPieces` below.

## The two-sided adoption test

A type is adopted at a site only when it is BOTH:

1. **no wider than the body provably produces** — otherwise the declaration is a
   claim the checker would have to bless on faith; and
2. **no narrower than any caller reads** — otherwise the declaration is true and
   useless, and every caller is relying on something the contract does not say.

Where the two disagree, the row stays OPEN with a named reason. **A row left open
with a measurement attached is worth more than a row closed with a type the
callers do not honour.** One site in this parcel failed the test and was left
open on purpose:

`Emit_ObjectPieces :: d5` — body produces `.b` (`addq.b #1, d5`), all three call
sites read `.w`. `out(d5: u8)` would have closed the row and published a byte
contract to callers consuming a word. The reads are correct today only through a
caller-side invariant (`moveq #0, d5` before the loop, capped at
`MAX_VDP_SPRITES = 80`) that the callee cannot state. The repair is widening the
increment, not narrowing the declaration; both are in the gap ledger.

The corollary rule, from `Section_RedrawPlanes :: d5`: **never type a register
whose bare claim already verifies.** `d5` is produced by `move.l Camera_X, d5`, so
`out(d5)` is proven at 32 bits today; adding `: u16` would trade a machine-checked
claim for a weaker one and buy nothing. Its sibling `d7` is typed because `d7` is
in the residue and `move.w Cache_Head_Col, d7` is all the body delivers.

## Where the width is charged

Four places, one helper, so they cannot drift:

- the proc's OWN returns — `OutWidths::own`, the obligation;
- a `jsr`/`jbsr`/`bsr` callee's verified out — credited at the CALLEE's declared
  width;
- an `Edge::TailOut` target's — same;
- a declared `falls_into` successor's — same.

A callee promising `out(d0: u8)` credits its caller one byte. Crediting it as a
long is the dangerous direction: the verified map is consumed as a must-def
definition, so an uncapped credit would let a caller publish a 32-bit claim
resting on a byte. Each of the three transfer-out arms has its own mutant test
proving it routes through the shared helper.

`conditional_out_edge_credits` carries the width beside each register for the same
reason. Its other consumer, D1b must-def, reads the keys and drops the widths —
deliberately, because must-def is width-blind on its local-write side too, and
honouring a width at the callee credit alone would make a call stricter than the
identical inline write beside it. Ledgered.

## What this does not do

- It says nothing about the bits ABOVE a typed out's width. `out(d0: u8)` leaves
  d0's upper 24 bits unclaimed, which is the point of writing the type.
- It does not check CALLERS. No caller-side out-read-width check exists; the
  per-site sweep in this parcel's packet is manual and is the only thing making
  the adoptions sound. Two gap-ledger rows carry the consequences.
- It does not reach a contract TYPE's `out` list. `type SensorProbe = proc (...)
  out(d0: i16, ...)` is documentation today: a proc and the function-pointer type
  describing it can disagree in width with no diagnostic (measured). Reading an
  unenforced declaration into the credit map would let a wrong type widen a claim
  silently, so `collect_out_widths` walks procs and externs only.
