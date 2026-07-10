# Tranche 7b — the interact-pointer staleness fix (2026-07-10, Volence-approved in advance)

Follow-up to the tranche-7 packet's ask #2: Volence chose "fix the
staleness." Ran as a mini-loop (implement lockstep → re-pin → live
verify → dry). Branches: sigil `repin-collision-interact`, aeon
`collision-interact`.

## The design

**`SST_interact`** — engine-owned word at `$4E`, the TAIL of the player
custom window (`engine/structs.asm` equ + build guard; `PlayerV` in
player_common.asm gains a collision guard so the overlay can never grow
into it). NOT an `Sst` struct field: object slots keep the full 34-byte
overlay window — the word has player-slot semantics only. The .emp side
derives it (`comptime fn interact_off() -> int { return sizeof(Sst)-2 }`,
drift-ensured against the AS equ; kill-list row 15).

**Lifecycle** (mirrors the proven player-side ST_ON_OBJECT pattern):
cleared at TouchResponse pass start, set by Touch_Solid top-contact to
the claiming solid's SST address. Single source of truth with a real
lifecycle — the disease (object-side bits with no clear) is structurally
gone, and the tranche-7 claim-by-identity block (16 B) became one
`move.w a3, interact_off()(a2)` (4 B).

**The ledge probe** (player_sensors.asm) reads the pointer directly —
the 12-instruction stale-bit slot scan is DELETED (−28 B, worst-case
~2600 cycles off the teeter path). `ST_P1_STANDING`/`ST_P2_STANDING`
died with their only consumers (AS constants.asm carries a tombstone
note: don't bring per-player object-side bits back; compare
`player.SST_interact` against your own SST instead). Constants twin
20→18.

## Live verification (oracle, all three lifecycle legs)

1. Land on solid A → `Player_1+$4E` == `$8A8E` (A's address), ON_OBJECT
   set.
2. Move onto solid B → pointer reads `$8ADE` (B) — the EXACT case the
   old first-match bit scan got wrong (it would have returned stale A).
3. Walk off → pointer `$0000`, bit clear.

## Numbers

Collision region `$16E → $166`; two-stage engine-tail slide (−8 before
player_sensors, −36 after — the scan collapse measured 28 B, not the
estimated 30; listings rule). EndOfRom unchanged both shapes. New pins:
plain `e22a82b3…`, debug `0c9f1952…`. Full sweep in PROVENANCE; gates
2034/0 strict + clippy + corpus, independently re-verified.

## Compiler finds (ledgered, RECORDED)

- Bare const names in displacement position are CLOSED on typed base
  registers — correct totality (a typo'd field must not silently become
  a const); the call-expr spelling `interact_off()(a2)` is the escape.
- Operand splices are template-only (`splices_allowed` gates proc
  bodies) — F1 scope note.

## Design seed (not asked, recorded)

The "player-only field in a universal struct" awkwardness is the first
concrete demand for role-typed SST views (`*Sst<Player>` / `*Sst<Obj>`)
— second demand ratifies, per the construct-walk discipline.
