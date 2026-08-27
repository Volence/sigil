# HOLE-INTERIOR-RESERVED: the predicate, and the three shipped shapes that fail it

Closes row **R9** of `2026-08-27-constraint-recheck.md` — *"a declared `[[hole]]`'s interior
stays empty"*, the second of the three `negative` constraints and the one nothing asserted.

## What the gap was

`native::validate_placement` — on the shipped ROM build path
(`build_rom_chained_with_listing`) — handles a `[[hole]]` with a *presence* check on its
`after` label and nothing else. `at` is read only to print. `filled_by` is read by nothing at
all. A packed layout that puts any other emitter in the span the hole reserves passes green.

## The predicate

`native::hole_interior_faults(resolved, pmap, sound_on, registry)`. A hole opens at its
`after` label, runs to `at`, and the module named by `filled_by` is the one thing allowed
inside it. Every other byte-emitting ROM section overlapping `[after, at)` is a
`[map.hole-interior-occupied]` fault naming the hole, the intruder, its head label, and the
occupied sub-span.

**The permitted set is derived, never transcribed.** `filled_by` is a MODULE id
(`engine.z80_init`); the section names it may occupy come from the shape's `registry` — the
module list the build is handed, upstream of every section it goes on to place.

**Loud when it cannot measure**, five ways, each its own tag: `hole-anchor-unresolved`,
`hole-anchor-ambiguous`, `hole-bounds-degenerate` (`at` at or before the `after` label — an
empty declared interior that would read as checked while checking nothing),
`hole-filler-unknown`, `hole-filler-absent`. None of them is a clean sheet.

`Ok(vec![])` over a shape whose `when` gates every hole out is a correct empty answer and
**not** coverage — a caller claiming "no hole is violated" must first establish that some
shape declares a live hole.

## ⚠ IT IS NOT WIRED INTO `validate_placement`, AND THAT IS THE FINDING

**Three of the seven shipped shapes fail it today**, and the cause is the declaration, not
the layout.

Measured over `native::shipped_shapes()`, `AEON_DIR=/home/volence/sonic_hacks/.aeon-freeze-slope`
(`9bba8700`), each shape's own `resolve_frozen_layout`:

| shape | live holes | result |
|---|---|---|
| sonic4 plain / sonic4 debug / config_a / lean | 0 (the hole is `when = "sound_off"`) | says nothing |
| demo plain | 1 | **1 fault** |
| demo debug | 1 | **1 fault** |
| config_b | 1 | **1 fault** |

The identical fault in all three:

```
[map.hole-interior-occupied] the hole declared after `Z80_IdleProgram` — interior
[0x3D0,0x3FE), reserved for `engine.z80_init` — is occupied at [0x3F8,0x3FE) by
byte-emitting section `boot_tail` (head `BootData_PostBlob`).
```

**The declared `at` is stale by 6 bytes.** Both `games/sonic4/map.toml` and
`games/demo/map.toml` declare `after = "Z80_IdleProgram" / at = 0x3FE`, written at
`a7375682` (2026-08-01, K1) and untouched since. The layout has moved twice since: the plain
boot region grew (`EntryPoint`'s `lea (SYSTEM_STACK).w, sp`) and the idle body grew 38 → 40
bytes (`ld sp, hl` → `ld sp, nn`). Today the idle occupies `[0x3D0,0x3F8)` and `boot_tail`'s
first byte — the `$9F` PSG-silence byte the map's own comment calls "post-hole AS data" —
sits at `0x3F8`, not `0x3FE`. `boot_data_port.rs::config_b_boot_data_hole_filled` already
asserts `0x3D0` / `0x3F8` against the golden, so the ROM is right and the map is wrong.

**POSITIVE CONTROL, over the same resolved layouts.** Correcting only the declared `at`
(`0x3FE` → `0x3F8`) in the map source and re-measuring returns **CLEAN** for all three
shapes. So the fault is the stale declaration alone: not the layout, not the registry
derivation, and not a probe that reports faults indiscriminately.

**And the shipped path's own answer on the same inputs is `Ok(())`** —
`validate_placement(resolved, pmap, false)` over config_b's real layout returns Ok while
`hole_interior_faults` over the identical inputs returns 1 fault. That is the gap, measured
rather than read.

## What is owed, and by whom

`map.toml` is aeon's file. The fix is one number in two files (`at = 0x3FE` → `at = 0x3F8`,
plus the two stale `# the 38-byte Z80 idle occupies $3d8..$3fe` comments), byte-neutral, and
not this lane's to make — the owner authors live in that tree.

**Once the declaration matches the layout, wiring is one call**, next to the existing hole
arm in `validate_placement`, threading the profile's `registry` through the signature. It is
deliberately not done here: adding it now turns every sound-off build red, including another
lane's, for a defect in a file this lane cannot correct — and weakening the predicate to fit
the stale declaration is the failure mode the predicate exists to remove.

`[map.hole-anchor-missing]` — which had no test anywhere in the workspace and had never been
observed to fire by anything committed — is now pinned by
`placement_validation_tests::hole_anchor_missing_fires`, with a control proving the red is
the absent label and not the doctored map.

## Reproduction

- `CARGO_TARGET_DIR` = `<worktree>/.hole-target` (on disk; never `/tmp`, never the shared
  `target/`).
- `AEON_DIR=/home/volence/sonic_hacks/.aeon-freeze-slope` (detached at `9bba8700`).
- `cargo test --release -p sigil-harness --lib placement_validation` → 23 passed / 0 failed
  (13 before; 10 new).
- The per-shape table above and its control came from a throwaway probe over
  `native::shipped_shapes()` + `resolve_frozen_layout`, deleted after the run: committing it
  would assert that today's violation is present, which goes poison-green the day the map is
  corrected.
