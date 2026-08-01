# The language round — agenda + overseer recommendations

**Basis:** notes/2026-08-02-language-round-ledger.md (the evidence per ask lives
there; this document is the DECISION SHEET). Volence rules each item; a BUILD
graduates to the item-7 process (spec → parcel(s), bars per class). Taste items
are flagged as his call, with a lean where I have one.

## Tier 1 — the headliner (its own spec session)

- **L1 · game-contract-hook — RECOMMEND BUILD, as its own ratified spec.**
  The one construct keeping game.asm ×2 alive; retires kill-rows 9/45's
  lockstep matrices; the K spec names it the survivors' liberator. Design
  direction to explore in the spec (FP taste): the ENGINE declares the hook
  SIGNATURES as a typed interface (boot hook, debug tick); a GAME provides the
  implementations as ordinary .emp procs bound by declaration — the game
  implements a declared contract, no macro seam, no text extraction. The
  spec session decides: hook binding mechanism (link-name convention vs an
  explicit `game { }` manifest block), empty-hook defaults, and the -D
  interface values' relationship to it. Scale M-L.

## Tier 2 — the type-layer cluster (one arc, ride together)

- **L5 · typed-const-refs-typed-const — RECOMMEND BUILD (completion-class).**
  A typed const initializer should resolve another typed const by name; the
  VRAM_TEST_MARKER literal is a wart. Arguably a bug-shaped gap, not a feature.
  Scale S.
- **L8 · sound-id newtypes — RECOMMEND BUILD.** The T1 newtype roadmap
  (item-13: MusicId/SfxId first) already ruled the tier; the F/F2 deferrals are
  its arrived demand moment. Needs L5. Scale S-M.
- **L9 · offsets cross-module Ref — RECOMMEND BUILD in the same arc** (ledger
  1767; retires player_common's extern-difference tables — the Parcel-D
  deferral). Scale S-M.

## Tier 3 — small, earned, one-sitting rulings

- **L3 · relaxation-aware align / per-data-item alignment — RECOMMEND BUILD**
  (a wall hit in flight at conv-i8; every data-after-relaxable-code port
  re-hits it). Scale S-M.
- **L4 · struct-scope `@allow("layout.odd-field")` — RECOMMEND BUILD** (E1's
  deliberately-unaligned Z80 records deserve a declarable intent). Scale S.
- **L6 · `[u8; _]` sugar — RECOMMEND DECLINE.** The omit-annotation form is
  already the blessed idiom (embed spec §1 contingency, shipped); `_` would be
  a second spelling for the same thing. One way to do it.
- **L11 · same-module data-label extern() — RECOMMEND DOCUMENT AS IDIOM** (the
  bare-name = cross-module link label rule is coherent; revisit only if it
  keeps biting).
- **L12 · `use` braced multi-line form — RECOMMEND BUILD as a ride-along** the
  next time a parcel touches the parser (cosmetic; not worth a standalone).
- **L13 · parallax_combine sugar — RECOMMEND DECLINE** (dropped as unused at
  Parcel G; no consumer).
- **L10/L14 · comptime-Data-relocations + cross-module sizeof ergonomics —
  RECOMMEND FOLD-OPPORTUNISTIC** (land inside whichever future parcel hits
  them; no standalone ceremony).

## Tier 4 — the human-authoring cluster (TIMING IS VOLENCE'S CALL)

- **L2 · objdef/objentry authoring DSL + L7 · shared mapping-DSL.** The
  campaign proved generators don't need these (typed literals suffice for
  machines); they exist for HUMANS authoring new game content. Two honest
  options: (a) DEFER to the first real new-content moment — design lands with
  a live consumer and Volence's authoring taste in the loop; (b) BUILD NOW as
  the opening of the features era ("as much as I'd like to start features" —
  if content authoring is imminent, the DSL is the on-ramp). **No lean — this
  is a timing-and-taste ruling.** Scale M each.

## Tooling (not language; scheduled separately)

- **T2 · parametric memory_hash(addr,len)** — CONFIRM the gap with the oracle
  tree, then BUILD there (the §17 identity bars' "single highest-value tool");
  the shipped whole-state state_hash covers part of it.
- **T4 · phase-aware repin — BUILD (S).** T1 RAM report — BUILD (S, nice).
  T5/T6/T7 culls — RIDE the modernization+lens sweep (already queued after
  this round).
- **T3 · A/B runner consolidation — RIDE the sweep.**

## Architecture (post-round arcs, sequenced after the rulings)

- **A1 · seam-2 registry unification (P1) + A2 · mt_syms emit split — its own
  arc** when ruled; retires the last non-native sound residue. A3 folds in.

## The proposed round order (if the recommendations stand)

1. L1 spec session (the headliner — Volence + Fable design it properly).
2. The type-layer arc (L5 → L8 → L9), porters under the standard bars.
3. The one-sitting batch (L3, L4, L12-ride, the L6/L13 declines recorded,
   L11 documented).
4. The L2/L7 timing ruling (defer-to-content vs build-now).
5. Tooling batch; then the modernization + dry-panel lens sweep over the
   converted corpus (already queued); then A1/A2 as its own arc.
