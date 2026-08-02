# The era lens loop (ruled 2026-08-03, Volence)

The campaign's dry-panel lens discipline carries into the new era as the
STANDING QUALITY BAR for all new work. The point is the lenses — fresh-context
read-only review agents making sure the code is good — not the byte gates
(new content has no twin; identity bars apply only where an OLD exists).

## The rule

Every parcel of new work (a new object, system, level mechanism, engine or
compiler change — anything beyond a trivial data nudge) gets a LENS PANEL
before it merges: 2-4 fresh READ-ONLY subagents, each with ONE lens, reviewing
the diff + its surroundings with no stake in the work. The implementing porter
never reviews itself; the overseer adjudicates the findings (fix / row / decline
with reason) and countersigns.

## The lenses

- **A · ceremony/style** — house style (brace-indent, present-tense comments,
  no change-history narration), naming, dead scaffolding, comment truth,
  needless mirrors; is anything here ceremony that a construct/idiom already
  retires?
- **B · corpus-pattern** — does the code use the idioms the corpus established
  (contracts, newtypes, ensure walls, span guards, comptime fns, the map) or
  reinvent them? Is there an existing helper/module this duplicates? Would a
  small language ask make this class of code better (→ step-3 ledger row —
  this lens is how L2/L7's demand moment gets noticed honestly)?
- **C · perf + hazard** — frame cost, DMA-window budget pressure (the M-1
  model), VRAM/pool/slot pressure, leak classes (children-despawn is the
  canonical example), invariant safety: does every new table/layout carry its
  ensure/span/drift guards, and does every new gate have a negative probe?

## The era baseline (what the lenses diff against)

The campaign-close state, verified clean and pushed (local == origin):

- **aeon `e03aad86e93d91051113a2cac07ace93dcb3c43d`** (a3-span merge — the
  campaign's final byte-relevant commit; goldens ×6 = chain 22)
- **sigil `d1115227`** (this ruling; last byte-relevant: `ea686380` a3-span)
- strict 2990/0/4 · chain 22 tip `a2-mtsyms` · AS survivors 3

Every new-era parcel branches from (a descendant of) this baseline; its lens
panel reviews the PARCEL DIFF (`git diff master...<branch>`) plus surroundings,
and the first content parcel's diff-from-`e03aad8` is the era's opening move.

## Calibration

- Engine/compiler/seam work: full campaign bars UNCHANGED (byte or behavioral
  identity + strict + chain discipline + panel).
- New content: behavioral bars (deterministic oracle A/B, state_hash, budget
  lenses) replace byte bars; panel at parcel boundaries, not per-edit.
- Panel verdicts are adjudicated, never self-declared — the amended dry-panel
  rule's standard, now era-wide.
