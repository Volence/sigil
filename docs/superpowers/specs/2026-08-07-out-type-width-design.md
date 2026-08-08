# `out(dN: T)` — the width a register result claims

The design record for the typed-out width rule: what was ruled, what was
measured, and the two questions the brief left open.

## The shape

A bare `out(rN)` claims all 32 bits of `rN`. A typed `out(dN: T)` claims the low
`sizeof(T)` bytes and nothing above them. Production credit is
**`produced >= required`**: a write at the declared width or wider proves the
claim; a narrower one does not.

**No migration from the TYPE facet**: with no type written anywhere, every bare
`out(rN)` means exactly what it meant. That property is exact and is pinned by
gates. It is NOT a claim that no bare verdict moves at all — the partial-coverage
fix below does move two, `out(d0) { ext.l d0 }` and `out(d0) { bclr.l #1, d0 }`,
which verified before and now fire. Both move in the false-negative-CLOSING
direction, and neither has a corpus site.

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

One authority for a NAME, and the name is resolved unscoped: the type table is
keyed by bare leaf, matching every other G5 consumer. Two modules declaring the
same newtype name share a row, and the collision is resolved per SIDE — widest
for the obligation, narrowest for the credit (see [`OutClaim`] below). There is no
single reading that is safe for both, which is why "answer the widest" is not the
rule. Module-scoping is ledgered; doing it for widths alone would produce the
second authority this paragraph exists to avoid.

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

**An underivable type answers `L` on BOTH sides** — a struct, an enum, an array,
an unknown name. On the STRICT side that is the strongest reading. On the CREDIT
side it is the weakest, and calling it "conservative" would be exactly the
one-number thinking this design rejects everywhere else.

What makes it defensible is not conservatism but DEGRADATION: `L`/`L` is precisely
what the declaration means with no type at all, so an unresolvable type behaves as
the bare `out(rN)` it collapses to and is no less sound than one. Measured —
`extern proc E () out(d0: u8x)` and `extern proc E () out(d0)` credit their
callers identically.

The loss is silence, not soundness: the author asked for something narrow and got
32 bits with no signal. A width cannot be guessed from a name that resolves to
nothing, so the answer is to REPORT it rather than to pick a number —
`ContractReport::unresolvable_out_types`, assert-empty over the corpus. Only the
corpus walk can do this; a per-file pass would fire on every legitimate
cross-module type.

## RMW: measured, not tasted

The brief said to start from "defining writes produce, RMW does not" and extend
only on evidence. Both rules were implemented and the corpus measured under each,
with the adoption in place:

| production rule | residue | |
|---|---|---|
| pure width (a write of the declared width or wider) | **16** | — |
| defining writes only below `.l` | **20** | 4 rows re-open |

The set diff is exactly four rows: `Collision_Probe{Down,Left,Right,Up} :: d0`,
which reach their `.full_back` return through `add.w d3, d0` / `neg.w d0`.

A fifth corpus production is RMW-only — `Emit_ObjectPieces :: d5`'s `addq.b #1,
d5` — but it is NOT in the diff, because its type was refused on the two-sided
test and the row is open under both rules. Stated separately rather than folded
into the four: a set diff reports what MOVED, and a row that never closes cannot
move.

**Ruled: pure width.** The evidence is one third of the target set, but the
argument is what settles it. The module's `.l` rule has ALWAYS credited RMW —
`add.l #1, d0` produces `d0` today and always has, because all 32 bits are written
on this pass. Applying a defining/RMW distinction only BELOW `.l` would make
"produce" mean two different things depending on the declared type, with the
stricter meaning applied precisely where the claim is WEAKER. That is incoherent,
and it is the kind of incoherence a later reader resolves in whichever direction
is convenient.

The brief's guard — "never let RMW alone count as production from nothing" — is
honoured for the RMW case by the transfer function rather than by a mnemonic
list. Width is tracked per register as a lattice and production is **gen-only
across widths**: a write at width `w` raises the recorded width to
`max(recorded, w)` and a later narrower write retracts nothing. So `addq.b #1,
d5` credits one byte and one byte only; it can never manufacture the claim above
it.

**The lattice is not the whole guard, and an earlier draft of this note was wrong
to say it was.** A write also has to COVER the size it is written at, which the
operand size alone does not establish. `ext.w d0` writes bits 8-15 from bits 0-7
and never touches bits 0-7; `tas.b` sets one bit; `bset`/`bclr`/`bchg` set or
clear one. All are `writes_last_operand` forms, so crediting them at operand size
produced a false verification at every width — measured as `out(d0: u8) { ext.w
d0 }` and `out(d0: u8) { tas.b d0 }` both VERIFYING. Worse, one `ext.l` after a
correctly-capped byte of callee credit laundered it back into a long, defeating
the cap this note advertises below as its soundness argument.

So `ext` is modelled as a PROMOTION — it raises an existing production one step
and makes none — and the single-bit forms produce nothing. `Scc` is deliberately
NOT swept in with them: `seq.b d0` writes all eight bits ($00 or $FF) and
produces a byte exactly as `move.b` does. The `.l` forms of this family were
mis-credited on master too (`out(d0) { ext.l d0 }` verified there), so the fix
closes a pre-existing hole as well as the one narrowing widened; it moves no
corpus row in either direction. What it does credit is
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

The corollary rule: **never type a register whose bare claim already verifies AND
whose full width is a RESULT.** Both halves are load-bearing, and
`Section_RedrawPlanes :: d5` is why. Its bare `out(d5)` verifies — `move.l
Camera_X, d5` is a full-width gen — but the `swap` that follows leaves the high
word holding the camera's sub-pixel fraction, so the "machine-checked 32-bit
claim" is over sixteen bits of residue and only the low word is the result. Its
sole caller has always read `move.w`. `d5` is typed `u16`, exactly like the `d7`
beside it.

Stated without the register: a width credit proves which BYTES were written on
this pass, never which ones CARRY the result. Only the caller sweep can tell those
apart, so only the caller sweep decides an adoption.

## An ADDRESS-register result takes a type, but never a WIDTH

Two rules, and they answer different questions.

**The width is pinned, unconditionally.** An `out(aN: …)` is recorded as a full
long on both sides of its claim whatever its type says, in `collect_out_widths`.
Every 68k write to an address register covers all 32 bits — `movea.w`
sign-extends, `(aN)+` advances the whole pointer — so a long is both the only
width such a result can be obligated at and the only width it can honestly credit.
This is where the soundness lives, and deliberately so: it is one function that
every declaration form flows through, so it does not depend on a per-file lint
firing, and no type, newtype or name collision routes around it.

**A type that STATES a narrower width is refused**, with `[proc.out-invalid]`, on
`proc`, `extern proc` and `type X = proc (…)` alike. `out(a0: u8)` claims
something the hardware cannot produce; the claim can never be violated, and a
declaration that cannot be wrong is not a contract. Refusing beats silently
pinning, because the pin already makes the declaration harmless and the author is
otherwise never told their statement means nothing.

**A type whose width IS a long is PERMITTED** — `out(a0: *Sst)`,
`out(a0: SomeNewtype)`. It carries no width news, and that is the point: it states
a DOMAIN. The corpus already types address PARAMS at ten-plus sites, and
`ZX0_Decompress (a0: *u8, a1: *u8) … out(a0, a1)` types the very registers it then
declares bare as outs. Refusing every type would make the output-direction dual of
a facet already in use unsayable — thirty address-register outs are declared
today, none typed, and that asymmetry is a gap, not a design.

**What a permitted address type does NOT do today, measured:** it does not reach
`[call.slot-type-mismatch]` in its POINTER spelling. `newtype_of` matches a
`Type::Named` whose leaf is a declared newtype, so `out(a0: Sst)` records a typed
out slot and `out(a0: *Sst)` records none. So the domain claim is real for a
newtype spelling and inert for a pointer one. The permission is still right — the
alternative is refusing a true statement — but it buys documentation rather than
checking until the slot collector reads through a pointer.

## Where the width is charged

FIVE places: ONE obligation and FOUR credits. Three of the four credits route
through one helper; the fourth does not, and that is exactly why it needs naming
here:

- the proc's OWN returns — `OutWidths::own`, the obligation, read through
  `required()` (the STRICT side);
- a `jsr`/`jbsr`/`bsr` callee's verified out — `credit_target_outs`;
- an `Edge::TailOut` target's — `credit_target_outs`;
- a declared `falls_into` successor's — `credit_target_outs`;
- a conditional callee's `out(rN: T if cc)` on the caller's cc-SUCCESS edge —
  `flag_check::conditional_out_edge_credits`. It is a per-EDGE transfer rather
  than a per-instruction one, so it cannot share the helper, and it reads
  `.credit` itself.

ALL FOUR credit sites read the CREDIT side; the single obligation site reads the
STRICT side. Naming any smaller number drops a site from the enumeration this
paragraph exists to make exhaustive — an earlier draft said "four places, one
helper" and both halves were wrong: it is five places, and the helper carries
three of them. The site that draft omitted was the one with no gate: flipping its single `.credit` to `.strict` re-opened the
collision blessing while the entire frontend suite stayed green at 2344 passed.
**A list of charge sites is a soundness artifact, not a summary** — anything
missing from it is something nothing is checking.

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
- It does not take a contract TYPE's `out` widths into the credit map. `type
  SensorProbe = proc (...) out(d0: i16, ...)` states a width nothing enforces: a
  proc and the function-pointer type describing it can disagree with no
  diagnostic (measured), so reading that declaration as credit would let a wrong
  type widen a claim silently. The width map therefore walks procs and externs
  only. A contract type IS reached by the two checks that need no enforcement to
  be safe — the narrow-address-type refusal and the unresolvable-type report —
  because both only ever say that a written declaration is meaningless.
- It does not carry a POINTER-spelled domain type to `[call.slot-type-mismatch]`.
  `out(a0: *Sst)` is permitted and inert there; `out(a0: Sst)` is checked. See the
  address-register section.
