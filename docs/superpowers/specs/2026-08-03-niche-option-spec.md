# Niche-sentinel Option — typed sentinels, forced guards, zero bytes

**Status: FULL DRAFT (Fable, 2026-08-03) — porter-grade; awaiting Volence's
gate.** Origin: the 2026-08-03 memory-safety design round. Companion plan:
`2026-08-03-finish-line-plan.md` §3 (Track C). Two slices: **C1** (distinctness
— buildable immediately, independent) and **C2** (guard-dominance — rides the
contract spec's P4 CCR/dataflow machinery).

## §0 — The idea and its non-negotiables

Sonic-lineage code encodes absence in sentinels: `$FF` = no slot, `0` = null
object, `-1` = list end. The bytes are fine; the bug class is *untypedness* —
nothing stops code using a maybe-absent value as if present ("forgot to test
for `$FF` before indexing"). This spec types the EXISTING representation:

- Does NOT reverse D2.14's runtime-tagged-union rejection. D2.14 said no cheap
  representation exists; the sentinel IS the cheap representation (Rust's
  `NonZero` niche, `Maybe` with a known uninhabited value). No tag byte, no
  layout change, no codegen — every check erases (§4.1 discipline).
- Guards stay hand-written instructions (`cmpi.b #…, d0; beq .none`). The
  language never emits a branch (S2-D15 stands). The type layer only decides
  what the value may be USED as, where.

## §1 — Surface (C1)

```
newtype SlotId  = u8 where 0..$7E          // the payload; niche excluded by range
newtype SlotRef = SlotId ? $FF             // SlotId-or-the-$FF-sentinel
```

- `T ? sentinel` is a newtype form: a distinct type whose values are either a
  valid `T` or the sentinel literal. Requires the sentinel to be provably
  outside `T`'s refinement range — overlap is `[option.niche-overlap]` (error;
  if `T` has no `where` range, the full-range overlap makes the error
  immediate: an option NEEDS the niche carved out).
- Auto-derived member: `SlotRef.none` — a typed const of the sentinel value,
  usable in operands (`cmpi.b #SlotRef.none, d0`, `move.b #SlotRef.none, var`).
  The raw literal in `SlotRef`-typed positions lints `[option.raw-sentinel]`
  (warn — write `.none`).
- Directionality: `SlotRef(x)` wraps a `SlotId` (the always-safe direction,
  the existing explicit-newtype spelling). The UNSAFE direction — using a
  `SlotRef` where `SlotId` is expected (indexing, arithmetic, a `SlotId` field
  store) — is `[option.unguarded-use]` (error) unless it flows through §2.
- Zero layout impact: `SlotRef` is `u8`-sized everywhere `SlotId` was;
  annotating an existing field/var/table from `SlotId`→`SlotRef` (or from
  bare `u8`) is byte-neutral by construction, proven ×6 on adoption parcels.

## §2 — The extraction marker

```
    cmpi.b  #SlotRef.none, d0
    beq     .no_slot
    assume_some! d0, SlotId        // d0: SlotRef → SlotId from here on this path
```

`assume_some!` is a statement-position marker in the `todo!`/`unreachable!`
bang family: emits nothing, retypes the named register (or var/field access
path) from the option to its payload for the remainder of the path. In C1 it is
TRUSTED — a greppable, auditable "I checked" (exactly `unsafe`'s social
contract, one instruction wide). Untyped escape hatches (raw `u8` math on a
`SlotRef`) remain cross-type errors under the existing newtype rules.

## §3 — C2: the guard-dominance check (rides contract-spec P4)

When P4's CCR + dataflow machinery lands, `assume_some!` is promoted from
trusted to CHECKED: the marker must be dominated, on every path, by a
sentinel-compare on the SAME location (`cmp`/`cmpi`/`tst` against
`.none`, flags unconsumed in between per `[ccr.stale-flags]` rules) whose
not-equal edge leads to the marker. Failure: `[option.unproven-assume]` —
tier: warn at P4-landing (corpus grace), error one parcel later. An
`assume_some!` that P4 can prove becomes silent; the residue is the audit
list. `--report contracts` gains an assume census (proven / trusted counts).

## §4 — Adoption + parcels

- **C1 parcel (one porter sitting):** the `? sentinel` newtype form,
  `.none` derivation, `[option.niche-overlap]` / `[option.unguarded-use]` /
  `[option.raw-sentinel]`, `assume_some!` (trusted). Tests: overlap positive
  (ranged + unranged), unguarded-use across register/field/table-index sites,
  wrap direction, `.none` operand golden bytes, byte-neutral ×6.
- **C1 corpus step (same parcel):** convert the two flagship sentinels — the
  object-slot "no free slot" return (the `Alloc*` family's documented $FF/Z
  idiom; pairs with their existing `out(a1)` contracts) and one table-index
  case picked by the porter from the SST field census. Small on purpose: the
  goal is the pattern proven, not a sweep. The sweep is its own later
  mechanical parcel once C2 makes the guarantee real.
- **C2 parcel:** inside contract-spec P4 (same branch pair; the dominance
  check is ~a consumer of P4's engine). Tests: proven-assume silent,
  unproven positive (guard on the WRONG register), flags-consumed-between
  positive, eq/ne edge orientation.

## §5 — Explicitly out

Multi-niche options (two sentinels), option-of-option, payload-carrying enums
at runtime (still D2.14-rejected), pointer-typed niches (`*T ? 0` — wants the
memory-region facet story first; ledgered as the natural sequel once the
contract spec's §9 region facets ever wake).
