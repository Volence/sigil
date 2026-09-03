# The per-parcel-term convention lost its feed when the freeze pairing was cut

**Found 2026-09-03, working the chain-201 pin-advance tail. It is the finding under the
last red, and it is larger than the red.**

## The convention

`crates/sigil-harness/tests/repin_pins.rs` asserts literal pin values against a
hand-typed baseline. Its standing rule — visible in every comment block in that file — is
that a moved pin gets **one term per parcel, derived from the schema rather than read off
the repin diff**. That discipline is real and it works: it caught `SCENE_REGISTRY` tonight,
and forced the band-drift term to be derived (67 bands × 4 = 0x10C) before the new value
was written down.

## What fed it, and what happened to the feed

The convention was fed by the **paired aeon↔sigil freeze**. Every aeon byte-mover came
through a freeze, each freeze produced a provenance entry, and each entry's term was
measured at its own landing by whoever ran it. The baseline was maintainable because the
measurements arrived one at a time, attached to the parcel that caused them.

**The owner cut the pairing at chain 199.** From that point aeon's byte-movers no longer
produce a sigil-visible freeze entry, and sigil's corpus advances in jumps — chain 201's
advance crossed **nine** aeon chains at once (`8876459e..4f5ad5a1`, 29 commits).

**Nothing announced that the feed had stopped.** The baseline kept looking maintained,
because no corpus advance happened between the cut and tonight. The first advance after the
cut is the first time the gap is observable, and it arrives as nine chains with no term
behind any of them.

## The state, measured rather than assumed

- `DEBUG_ASSEMBLED_LEN` wants `0xA7F38`; the advanced corpus resolves `0xA81FC`. `+0x2C4`.
- The plain total **HOLDS** at `0xA5C82`.
- Asked aeon for the nine per-chain debug-tail terms. **They do not have them** — checked,
  not assumed: they grepped their ten merge commits for the span (none carries a debug
  figure), their lane-log (four prose deltas, not a series), and the provenance ledger
  (covers only up to 199, by the same cut).
- What they DO have per chain is **file size**, which is not the same quantity.
  `file = assembled + appendix`, and sigil's own suite prints the split:
  `S1.4 debug: assembled=0xa81fc full=737683 appendix=0xbf97` (`0xa81fc + 0xbf97 = 737683`,
  exactly). Their file span across the range is `+0x924` against an assembled `+0x2C4`, so
  **0x660 of it is appendix**. Taking file deltas as terms would have made the baseline
  1632 bytes wrong while reading as nine measured numbers — the chain-198 conflation one
  level out. They declined to hand them over for exactly that reason, correctly.

## What is being done

Aeon offered, and this lane accepted, a rebuild of the nine chains to read `EndOfRom` from
each listing — terms **measured at each chain** rather than back-derived from the total the
baseline exists to check. Roughly an hour of unattended machine time, explicitly not to
preempt their EFFECTS-W1 work. Nothing is behind it: it is the last red in sigil's suite and
no parcel in any lane waits on it.

**The reason that hour is worth spending is that it is ONE-TIME.** Aeon has booked, against
themselves, that their merge commits now carry the four shape sizes, the CRCs, and
`EndOfRom` for a debug byte-mover. Chains 200-212 are the only span that will ever need
archaeology, because they are the span between the owner's cut and that fix.

## The open question, which the rebuild does NOT answer

**Should this baseline exist in its current shape at all?** Its sibling,
`secondary_pin_classes_match_the_hand_typed_baseline`, was already RETIRED with the
reasoning that it *"asserts literal pin VALUES, which now legitimately move on every
layout-shifting parcel — the hand-typed baseline is the pin-tax class the packing walk
exists to kill."* That reasoning applies to `DEBUG_ASSEMBLED_LEN` word for word.

**Do not read that as a licence to retire it tonight.** Retiring a check while it is red,
because it is red, is the protocol's bar-9 failure with the causation hidden: nobody decides
to weaken the gate, the red build simply makes weakening it the path of least resistance,
and the tell is that the conclusion requires work from nobody. The check earned its keep
twice this session. It goes to a ruling with the numbers, not to a convenience.

The honest framing for that ruling: the per-parcel-term convention is a **maintenance model**,
and its cost is now paid in multi-chain jumps rather than single parcels. Either aeon's new
record-keeping restores a per-chain feed good enough to keep it (likely, and cheap), or the
assertion moves to a form that does not need a population maintained by hand — which is
`SIGIL-DECOUPLE` step 3's territory, *"retire `repin`/`pins.rs` from the landing path into an
internal regression tool"*. Read the two together; they are one question.
