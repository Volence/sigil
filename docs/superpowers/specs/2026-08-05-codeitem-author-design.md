# CodeItem authorship — design ruling

**Fable, 2026-08-05.** Answers the sr-contracts pass-2 panel ask (packet §8,
"should a `CodeItem` carry its AUTHOR?") and the ledger row that names the
three point solutions now standing in for one missing fact. Porter-grade;
dispatch as its own sigil-side parcel after the current wave merges.

## §0 — The question, and why three answers exist

A lint over a proc's item stream asks "what does this proc do", but since
constructs began splicing, the stream holds instructions the proc's author
never wrote. Three shapes, three shipped-or-proposed answers:

1. **The `assert` desugar** — compiler-emitted `move.w sr,-(sp)` /
   `move.w (sp)+,sr` brackets charged to the containing proc. Today's answer:
   `[proc.sr-undeclared]`'s DEBUG rows stay at 43/42/43 and a property test
   (`every_surviving_sr_firing_is_the_assert_desugar`) READS THE SOURCE LINE
   behind every firing to prove they are all the desugar. A measurement is
   standing in for a fact the compiler had and threw away.
2. **Context acquire/release splices** (`with ints_off {}`) — the sr lane
   built a range exemption off B′-1's `ContextMark` items, gated on the pair
   round-tripping SR. Correct, but per-lint and hand-rolled; the next lint
   that walks instructions (`[proc.clobber-undeclared]` the moment a context
   clobbers a scratch register) re-answers it from scratch or charges the
   wrong party.
3. **The synthetic entry module** — `build_emp` synthesises it, and the
   warning tier caught sigil warning about its own synthesis
   (`import.no-names` ×88, fixed as a special case).

Three point solutions, one missing fact. The campaign is one lint away from a
fourth.

## §1 — RULING: `CodeItem::Instr` gains an `author` field

```rust
/// Who put this item in the stream. `User` means the containing item's
/// author wrote it; everything else names the construct that spliced or
/// synthesised it. Authorship REDIRECTS a checking obligation to the
/// author's own declared surface — it never waives one (§2).
pub enum ItemAuthor {
    User,
    AssertDesugar,
    Context { name: String, phase: ContextPhase }, // Acquire | Release
    EntrySynth,
    Splice { template: String },   // carried, not yet consumed (§6)
}
```

- Field on `Instr` only. `Label`/`Data`/`ContextMark` are not charged by any
  effect lint; extending later is additive. Default `User`; ~20 non-test
  construction sites migrate mechanically (54 with tests).
- Spans are NOT the answer and stay untouched: the desugar's items carry the
  user's `assert` span (right for diagnostics, wrong for authorship), and a
  same-file template splice is indistinguishable by source. Authorship is a
  semantic fact, not a location fact; diagnostics keep pointing where they
  point today.
- Setting sites are exactly the three constructs above plus the context
  splicer (which already knows its ranges — it plants the `ContextMark`s).
  No new analysis; the field records what the emitting code already knows at
  the moment it emits.

## §2 — The invariant that keeps this sound

**Authorship never exempts an effect from checking; it moves the obligation
to the author's own contract.** An exemption without a receiving contract is
how a soundness hole would dress up as ergonomics. Concretely:

- `Context`-authored SR traffic is exempt from the CONSUMER's
  `[proc.sr-undeclared]` because the round-trip proof moves to the context
  DEFINITION (checked once, where the code lives, not per adopter). The sr
  lane's round-trip gate survives — relocated, not deleted. A context that
  masks and never restores still fires, at its own definition site.
- `AssertDesugar`-authored traffic is exempt because the desugar's balance is
  the COMPILER's obligation: pinned by a unit test at the emission site
  (push/pop pairing by construction). If the desugar ever grows an unbalanced
  path, that test — not a corpus measurement — is what fails.
- `EntrySynth` mirrors the warn-tier fix it replaces: sigil does not lint its
  own synthesis, and the synthesis site owns its invariants.

## §3 — First-wave consumers (all in the dispatching parcel)

1. `[proc.sr-undeclared]`: replace the `ContextMark`-range walk with
   `author == Context{..}`, keeping the round-trip gate at the context
   definition. Behavior-identical on today's corpus (0 plain firings stay 0).
2. **The DEBUG `sr` surface retires**: exempt `AssertDesugar` items; DEBUG
   shapes go 43/42/43 → 0; `DEBUG_ONLY_LINTS` baseline updated deliberately;
   the source-line-reading property test is REPLACED by a typed assertion
   (`every sr-write in a DEBUG shape stream is User-authored or fires`),
   with a non-vacuity guard (`seen > 0` on desugar-authored items).
3. The `never-examined DEBUG sr surface` open item closes with numbers: after
   exemption, any remaining DEBUG firing is a real hand-written undeclared SR
   write and the warn baseline will show the id — the hiding place the sr
   lane's Lens C worried about cannot re-form.

## §4 — The perturbation-set ask: substrate now, surface on demand

The sr lane's ask ("a context should declare its own perturbation set") is
DEFERRED, demand-parked exactly like Z80 `VALID_CCS`: today's only contexts
round-trip SR and clobber nothing, so a `perturbs(...)` clause would be an
annotation with zero honest users. The author field is deliberately the
substrate that makes it cheap later: when a context with a genuinely
clobbering acquire appears, `[proc.clobber-undeclared]` charges
`Context`-authored writes against the context's (then-new) declared
perturbation instead of the consumer — one clause plus one lookup, no new
walk. Ledger the trigger: first context whose acquire writes a register it
does not restore.

## §5 — Deliberately not taken

- **`Splice { template }` is carried but not consumed.** Checking template
  splices against a declared `-> Code` fn contract is ledger row 1551's ask
  (contract annotations on Code-returning fns) and a language-round item; the
  author field just stops the information loss so that parcel starts whole.
- **No new diagnostics.** This parcel moves existing obligations to their
  owners; it introduces no lint. Anything it newly reveals (a DEBUG firing
  that is NOT the desugar) was true before and hidden.

## §6 — Gates

Byte bar ×7 unchanged by construction (a field on an IR enum emits nothing) —
if a target moves, stop. `refreeze --check` at current chain. Full strict with
delta arithmetic. Warn-tier baseline: the DEBUG rows' id removal is the one
deliberate baseline edit; explain it in the packet. Lens panel standard; point
Lens C at §2 — try to construct an authored-but-unchecked effect path.

## §7 — Rows

Closes: the three-answers row (sr packet §8/Lens A19), the DEBUG-sr-surface
open item, the property test's source-reading brittleness. Opens: the
perturbation-set trigger row (§4), the `Splice` consumer row pointing at 1551.
