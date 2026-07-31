# Wave-B B-0 — packed placement (the rows-6/58 partial realization)

**Why now:** Wave B's first parcel (plane_buffer #4, +18 bytes) hit `resolve_layout:
sections plane_buffer and tile_cache overlap (colliding pins)`. The engine block is
packed back-to-back (zero slack), so EVERY growing parcel would cascade a hand-bump
through the downstream pins — the pin-tax the campaign has twice refused (G9 ruling).
Worse: the refreeze machinery had a bootstrap hole — pins/tables regenerate FROM a
successful build, but a grown section can't build against the stale pins. The A3
pathfinder never saw this because it was size-neutral by design. B-0 is therefore a
Wave-B prerequisite, pulled forward from the G9 design step (ruled LEAN COMPUTED).

## The mechanism

`SizeSource::Frozen`'s placement semantics upgrade from "every labeled section at its
frozen address" to **anchors + live packing** (`native.rs::packed_true_bases`):

- The frozen table still supplies ORDER and the org-island ANCHORS (a section whose
  provisional base sits > `ANCHOR_GAP` (0x400) past the running end anchors absolutely
  — the object bank, the sound banks, the residual data islands).
- Every other section's base is **packed**: `align_up(running_end, A)` with `A` = the
  largest power of two ≤ 16 dividing its provisional base — at unchanged sizes this
  reproduces the frozen layout exactly (fold identity), and under growth it re-derives
  the alignment pad the layout implies (the ERROR_HANDLER +2 pad is an inferred align).
- Image lengths are relaxation-dependent, so the walk iterates measure → pack to a
  fixpoint (≤ 8 rounds, loud on divergence). Round 0 measures at provisional pins,
  falling back to a +0x40/section cumulative spread when a growth makes them collide
  (bounded so cross-section CONDITIONAL branches — which have no long form — keep
  reach). Re-measures pin only LABELED sections (label-less pure-data blobs measure at
  scratch; the align-padded pre-bank blob would otherwise collide with the bank org).
- Island classification must be IDENTICAL across rounds — a growth big enough to eat
  an org hole errors out for a hand ruling instead of silently repacking an island.

**Canonical joins the same mechanism**: `sonic4_profile` flips PinnedBaked →
`Frozen(s4.txt / s4_debug.txt)`; the canonical boundary tables (one head label per ROM
section + `EndOfRom`) were bootstrapped once from the pinned resolve
(`derive_offcanon --bootstrap-canonical`, `derive_canonical_bootstrap_table`) and
refresh thereafter through the normal `derive_frozen_table` path like every other
target. `build_native_rom_with_listing` routes canonical through the chained driver —
ONE placement authority for all six targets. `sonic4_pinned_profile` (PinnedBaked)
remains as the bootstrap path only.

Rider: `derive_offcanon` now derives ALL SIX tables, and reads each target's EOR from
its own freshly-derived `EndOfRom` (the profile's baked `assembled_len` goes stale the
moment sizes change; it survives only as a fallback).

## Proof

- **Fold identity:** with unchanged sources, all six targets byte-identical to the
  committed goldens (native_rom, native_declared_chain, native_full_rom,
  native_offcanonical_rom, native_offcanonical_full — all green; full strict green).
- **The live t24:** plane_buffer #4 (+18 bytes) is the first consumer — its build must
  shift TILE_CACHE.. downstream automatically and re-freeze cleanly (next parcel).

## Honest bounds (ledgered)

- A single-parcel growth > ~0x40/section (the round-0 spread) or > ~0x400 (the island
  hysteresis margin) needs a hand ruling — the guards fail loud, nothing silent.
- RAM needed no analog of this parcel: B-0b verified RAM already packs (AS `phase`
  chaining; the `pins.rs` RAM cells are repin-generated test snapshots, never build
  inputs) — see `2026-08-01-waveb-b0b-ram-packing.md`.
- Sound-bank INTERIOR sections now pack contiguously after their anchored bank head
  (previously absolute); byte-identical while the blob pipeline is size-stable — the
  Z80-blob-precedes-engine rule keeps it that way.
- The `load_bearing` symbol-address spot sets in `native_offcanonical_full.rs` are
  hand-pinned structural anchors; a layout-shifting parcel updates them consciously
  (they are the point: placement moves must be SEEN).
