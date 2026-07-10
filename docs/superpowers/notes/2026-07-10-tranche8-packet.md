# Tranche-8 checkpoint packet — rings.asm (2026-07-10)

The full loop ran to dry on `engine/objects/rings.asm` → `rings.emp`.
Branches: sigil `port-tranche8`, aeon `sigil-emp-tranche8` (worktree, editor
data seeded) — both unmerged, awaiting your gate.

## The headline: byte-exact on run 2, and the hot loop got faster

**Transcribe went green almost immediately** — both reference byte gates
passed on the second harness run; the only fixes between run 1 and run 2 were
a parser gap (local labels in displacement position) and a probe's own
zero-length branch. The zero-disp collapse probe promised at row 13 **passed
UNCHANGED** — the F1 splice already rode the general `(0,An)` collapse.

**Step 5 found a real hot-loop win, live-verified.** RingCollision recomputed
`&Ring_Buffer[index × 6]` from scratch EVERY iteration (~36 cycles of index
math per ring test) while DrawRings ten lines up already walked with a
rolling pointer. Now a3 (unused, already in the clobber set) rolls down the
buffer at one `subq.w #6, a3` (8c) per iteration — **~28 c/ring-test/player/
frame** at the loop that runs over the whole buffer for both players every
frame (~3.6KC/frame at 64 buffered rings, 2P). Correctness across
swap-with-last removal is structural: the removed slot is rewritten from an
already-visited HIGHER index, so entries below the cursor never move; a3
survives the collect path (all five callees' clobber contracts verified).
**Oracle-verified live**: draw, collect, Ring_Counter, high-water, and
swap-with-last twice — including a MID-BUFFER collect with live entries below
the cursor. Plus a peephole: RingBuffer_Remove's two `lea (aN,dN.w)` →
`adda.w` (−4 B, −4 c each, matches Add's existing idiom).

## Demanded feature shipped: `dc.b`/`dc.w`/`dc.l` in proc bodies (H8)

Rings is the FIRST ported file with `__DEBUG__`-conditional CODE, and its
`assert.b` expands through debugger.asm's macro tower into mid-proc DATA (the
RaiseError format string sits between the `jsr` and the resume label — the
handler reads it at the return address). No construct could express that, so
per the demanded-features law `dc` shipped at step 1: comptime-only elements
(ints range-checked loud, strings raw-ASCII per D2.16), lowering through the
§6.2 `CodeItem::Inline` path that had been waiting unreachable since T4.
Reserved at mnemonic position on both CPUs (tenet 3, jbra/jbsr footing),
CPU-neutral by construction (scalars take the section CPU's byte order).
Link-expr cells deliberately excluded (ledgered, consumer-gated). Seven
negative probes.

The assert block itself is a TRANSLITERATION (row-9 precedent): instruction
skeleton as real asm (the two `MDDBG__*` targets resolve through the link
seam), FSTRING data as verbatim `dc.b`. The debug byte gate is the drift
guard. Kill row 16; the `.emp assert` construct is ledgered as demand 1/2 —
one call site doesn't justify a comptime format-string compiler, the
debugger.asm port era does.

## Firsts this port proved

- **Shape-dependent-LENGTH region** (plain 0x1B4 / debug 0x210) — `Shape`
  carries per-shape `len`; the mixed gates run different region sizes per
  shape for the first time.
- **Local-label displacement operands** — `pea .raise(pc)` (the AS `pea
  *(pc)` self-address idiom's translation; `*` itself goes in the
  port-translation bucket, ledgered).
- **Reused proc-local label as a template argument** — both aabb splices take
  the same `.no_hit`; the .inc twin needed `utag`, hygiene makes reuse free.
- **SND combo matrix** (game_loop pattern) vs a fresh AS-twin oracle;
  documented gap: the (DEBUG=1, SND=0) combo has no pin (the twin can't
  expand `assert.b` without the whole debugger macro tower; the DEBUG
  dimension is covered by the debug reference gate).

## Kill list

- **Row 13 CLOSED — by consolidation, not deletion.** The written condition
  ("rings ports → delete the .inc") was unexecutable: after this port the
  .inc's only consumers are the GATE-OFF AS TWINS, which live until Spec 5.
  Re-homed under row 5; LOCKSTEP comments updated both files. Process lesson
  recorded: kill conditions written before their port get re-verified against
  the gate-off shape's needs.
- **New rows:** 16 (assert transliteration — dies at debugger.asm's port or
  the assert construct), 17 (MAX_VDP_SPRITES/VDP_SPRITE_* — truth is
  sprites.asm, NOT constants.asm, so row 1's flip doesn't cover them),
  18 (the four game-owned ring mirrors — kill = imm-link width deferral or
  the game config port; drift probe included).

## Loop summary

- **Step 0**: design note `notes/2026-07-10-tranche8-rings-design.md`
  (committed to master pre-branch). All hazards called in advance; H8 (`dc`)
  discovered at step-1 entry and appended.
- **Step 1**: byte gates green both shapes; 58 drift guards (30 SST + 24 twin
  + 4 rings-local); outbound consumer; gate-off neutrality (plain hash
  unchanged pre-wave); mixed tranche-8 both shapes; strict 2038/0.
- **Step 2**: born-modern except five `bsr.w` → `jbsr` flips — byte-identical,
  so NO re-pin. Conditional widths stay pinned (jbcc deferred). The
  transliteration block stays as-expanded (row 16 owns its demolition).
- **Steps 3+4**: ledger +6 (assert construct 1/2, dc link-expr cells, `*`
  port-translation, non-SST packed-record view 1/2 — the ring entry's
  0/2/4/5 literals + ×6 chains want a `record`-over-RAM view; second consumer
  likely entity_window — hardcoded-guard-counts CLOSED, culling literals).
  Back-prop: **every guard-count literal in the suite now DERIVES from the
  shared twin list** (`twin_guards()`) — the tranche-7 shared-list move
  finished; future twin growth is a one-list edit. Stale row-13 references
  swept.
- **Step 5**: above. Re-pin wave: rings −4 (Remove peephole; the rolling
  pointer is net-ZERO bytes — the chain moved out of the loop), every
  downstream engine-block pin re-derived FROM LISTINGS (resume orgs
  $33A4/$36BA; collision_lookup $4C1A/$543E + base $4BF6/$541A; sound_api
  $5F4A/$7408 + base $5D66/$7224; Tile_Cache_GetCollision $430E/$4A7A;
  Sound_* labels; a tranche-5 byte-pin array carrying a cross-region bsr.w
  displacement). MDDBG__* verified UNMOVED (org $10000 absorbs the slide).
  PROVENANCE re-baselined; new pins plain `c973091d…` / debug `6a0f9c3f…`.
  **Final: strict 2048 passed / 0 failed, clippy clean.**
- **Dry check**: the wave's retrospect surfaced the GENERALIZED re-pin rule
  (a size change re-derives every harness pin in the sliding window — orgs,
  map bases, label VMAs, byte arrays, probe constants; sweep grep + let the
  strict suite name survivors) and the deliberately-not-taken cold-path items
  (Add's stack ×6, Remove's remaining chains — spawn/collect-time only).
  Both recorded. Nothing else on the re-read. DRY.

## Process notes

- The step-1 suite run caught the guard-count breakage across SIX targets —
  the derived-count conversion was forced immediately rather than jotted;
  counted as the step-4 back-prop.
- Fresh-worktree editor-data seeding worked exactly per the handoff note
  (baseline hashes matched the tranche-7b pins before any edit).
- Oracle session note: the test level spawns 7 rings in a row at y=$60 —
  convenient collision fixtures at $80/$90/$A0/… (teleport the player's
  x/y int words at Player_1+2/+6).

## Asks

1. **Merge gate ×2**: sigil `port-tranche8` (dc feature + parser + harness +
   docs), aeon `sigil-emp-tranche8` (rings.emp + gate + constants twin +
   step-5 wave — the RingCollision change is behavior-identical and
   live-verified, but it IS an engine hot-loop rewrite; your call).
2. **Empyrean amendment stack** grows: `dc.b/w/l` statement surface,
   local-label displacement operands, the `pea *(pc)` port-translation rule,
   the row-13 consolidation lesson. Still your cadence.
3. **Tranche 9 candidate** (suggestion, not decided): `animate.asm` — kill
   rows 2/3's flip gets closer, the AnimId/FrameId typed surface continues
   the construct-walk thread, and it's another engine-block region (the
   re-pin machinery is warm). Alternative: `entity_window.asm` — bigger, but
   it's the ring system's other half AND the likely second consumer that
   ratifies the packed-record view (ledger demand 2/2).
