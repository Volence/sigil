# PARCEL BRIEF — item-13 wave-1 domain types (implementation)

**Dispatch: overseer-cut 2026-07-23, Volence-ordered (runs in parallel with the
boundary-crossing parcel — file-disjoint; sprites-hardening parcel MUST wait
behind this one — shared sprites.emp).**
**Canonical sources (read before cutting):** construct-walk #4 ratification
`7b9afa6` (the type rulings — authoritative), the item-13 first-wave ledger seed
rows (`941c5f4`), G5 spec close packet step-2 feed-forward (`5f242ff` — the
typed-slot spelling is the house format).

**Scope class: BYTE-NEUTRAL.** Master canonical plain `00f609a5`/421089 · debug
`80d14183`/429134 must be UNCHANGED by every commit (fresh dual-build confirm at
close — no re-baseline). **Branch:** `item13-wave1` both repos, seeded worktrees
(byte-neutral fast path: copy reference ROMs), AEON_DIR pinned to branch tree.

## The three ratified families (rulings are FROZEN — implement, don't re-litigate)

1. **SongId / SfxId** (both `u8`, DISTINCT) — const-born: type the `SONG_*` /
   `SFX_*` constant definitions so every table and call site checks for free.
   68k-side only (Z80 blob untouched). APIs keep action names; types name values.
2. **AnimId / AnimFrame / MappingFrame** (three types over the cheaper two-type
   cut) — script-frame vs mapping-frame is the highest-risk swap. Accepted cost:
   ~1–2 `as` re-blesses at the animate advance path — spell them at true
   construction sites per the G5 as-bless idiom.
3. **VramTile / VramAddr — COMPTIME LAYER FIRST**: `VRAM_*` layout constants
   typed VramTile; `vram_bytes()` takes VramTile → returns VramAddr;
   `vram_art()` takes VramTile. Zero register ceremony — register-slot typing
   (DMA-queue params etc.) is a LATER wave; do not creep into it.

## Required negative pins (from the ratification — non-optional)

- SfxId-into-SongId-slot pin (doctored misuse fails naming the site).
- AnimFrame/MappingFrame swap pin.
- Plus per-family unit/corpus tests per the G5 pattern.

## Watch-items

- **FlatIDXY preserves-verifier reopen condition** (G5 ledger row): if any typed
  value ends up carried across a multiply, SURFACE it at the gate (the row's
  reopen trigger) — do not absorb.
- **Optional-param Option A rider (SIZE-CAPPED, gate-adjudicated):** §D ruled
  Option A's design ratified with implementation "likely rides item-13 wave-1"
  since animate is being touched anyway. Take it ONLY if it stays a small
  bounded addition on the files already open; if it grows, log-and-split to its
  own parcel — state the decision either way in the close packet.
- Wave 2 (Tile/Block/Chunk, Coord/Velocity, S2-D8) stays A4-i-GATED — out of
  scope, no exceptions.

## Acceptance

Byte gates green with canonical UNCHANGED both shapes (fresh dual builds); full
paired strict green; negative pins demonstrated RED-first; zero new clippy;
ledger seed rows marked implemented; close packet per house format (per-pass
breakdown + neither-bucket headlines); merge via the sequential queue
(coordinate with the boundary-crossing parcel — byte-neutral ⇒ canonical-
unchanged CONFIRM at merge, not a re-baseline).
