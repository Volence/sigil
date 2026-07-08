# Handoff — the sound-migration arc: DESIGN CONVERSATION first

Written 2026-07-08 (Fable, post-#7-merge cleanup session) for the next design session. This arc
needs Volence engaged (same shape as the #7 design conversation). Read [[spec2-progress]] and the
#7 design doc (`specs/2026-07-08-spec2-plan7-item7-banks-design.md`) first.

## Where master stands (5032c7c, pushed)

- Plan 7 #1–#9 + #7 ALL MERGED. **The gate is now WORKSPACE FULLY GREEN** — the 4 strlen-drift
  reds are gone (reference ROMs re-baselined to aeon f828406; three masked sigil bugs fixed:
  string `equ`, phase-aware `align`, image-vs-VMA overlap keying). Do not cite the old allowlist.
- Everything the sound data needs is shipped: `bank:` sections, `bankid()`/`winptr()`, link-expr
  value cells, `embed`+`.len`, `zx0`, offset tables, dispatch/script, truthful placement, `--map`
  regions. Exhibit: `examples/game/data/dac_samples.emp` (the honest VMA==LMA shape).

## What this arc IS (scope question #1 for the conversation)

The first tranche of the migration campaign (68k-side first, per the standing "68k first, Z80 DAC
last" order): porting aeon's SOUND DATA subsystem from `.asm` to `.emp`, byte-identical per file —
`games/sonic4/data/sound/` (dac_samples, song data + pitch tables + patch banks, sfx blobs,
song_table/sfx_table, the co-location fatals in main.asm:110–241). It is NOT the Z80 driver itself
(driver code stays `.asm` until the Z80 relaxation ladder + Spec-5 era). The migration is also the
DEMONSTRATED-NEED GATE for four ledgered decisions — the conversation should open each:

1. **S2-D14(a) / L7.1 — the packing linker** (Volence: "don't forget"): does hand-placing the real
   ~30KB shared DAC bank + the MT/DrumTest/HCZ2 co-located block into `--map` regions hurt enough
   to justify auto-fitting floating blobs into bank free space?
2. **S2-D14(e) / L7.5 — VMA/LMA coupling for `bank:`**: the real sound data is VMA==LMA, but decide
   couple-vs-reject now that real files exercise it.
3. **S2-D14(d) / L7.4 — Z80-side consumption idioms**: what the 68k-emitted tables must promise the
   driver (descriptor shapes, window-relative offsets, the engine-table-head co-residency rule).
4. **9d — the byte-command DSL** (D9.3 gate): song/sfx streams are generated blobs today
   (tools/*.py emitters); decide whether .emp represents them as `embed` (status quo, probably) or
   grows the DSL. Also check 9c's gate (wait_frames rule-of-three) against the ported files.

## Inputs to bring

- aeon's real constraints: `dac_samples.asm` (shared bank, 9 samples), `song_table.asm` (the four
  co-location fatals incl. the engine-table-head rule), `main.asm:110–241` (the include order +
  `SND_ENGINE_TABLE_BANK` + SFX co-residency fatals), `engine/sound_constants.asm` DacSample
  struct (9-byte descriptor: bank u8 / rate / codec / ptr u16le / len u16le / loop).
- The Plan-6 port harness pattern (ports.rs byte-diff per file) and the `@as_compat` story (§8.2).
- Open sigil threads that may ride along: backlog **#10 compression builtins** (nemesis/kosinski
  family — NOT needed for sound, s4 sound uses zx0/raw; sequence it independently), the table-emit
  rule-of-three (lower_offsets/dispatch/script — extract if a sound table touches that seam), DX-2.

## Process

Design conversation → APPROVED design doc with D-rows → implementation handoff (the #7 pattern:
worktree, frozen rulings, TDD, subagent-driven, byte-diff nets — now with the FULLY-GREEN gate).
Per-file bar: byte-identical vs the AS reference via the ports harness; `sigil diff` for the full
ROM. Spec integration for anything ratified lands in empyrean's working tree at the checkpoint
(Fable). NOTE: empyrean's tree carries the uncommitted #7 spec integration (D2.25/§7.4/S2-D14) —
Volence commits at his cadence.
