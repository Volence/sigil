# HOLE-INTERIOR-RESERVED: the predicate, and the build path it now gates

Closes row **R9** of `2026-08-27-constraint-recheck.md` — *"a declared `[[hole]]`'s interior
stays empty"*, the second of the three `negative` constraints and the one nothing asserted.

## What the gap was

`native::validate_placement` — on the shipped ROM build path
(`build_rom_chained_with_listing`) — handled a `[[hole]]` with a *presence* check on its
`after` label and nothing else. `at` was read only to print. `filled_by` was read by nothing
at all. A packed layout that put any other emitter in the span the hole reserves passed
green.

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

## It is wired into `validate_placement`

`validate_placement` carries the shape's `registry` and runs both halves of the hole arm:
the `after` label's presence, then the interior. `build_rom_chained_with_listing` calls it,
and `sigil build` calls that, so **a fault is a build error** — `sigil build --native` exits
non-zero and writes no ROM.

The two halves stay in that order. A layout whose `after` label resolves nowhere is refused
by the presence arm as `[map.hole-anchor-missing]` (the spelling aeon fixtures assert on),
so `[map.hole-anchor-unresolved]` is the one tag the build path cannot reach; it survives
for callers that drive the predicate directly. The other four are reachable and pinned
there.

Every occupant of one hole is reported together rather than one at a time: a stale `at` that
swallows three sections is one declaration to correct, and a caller shown only the first
would correct it three times.

## What the wiring cost the shipped shapes: nothing

Four of the seven shipped shapes (`sonic4 plain`, `sonic4 debug`, `config_a`, `lean`) gate
their hole out with `when = "sound_off"` and declare no live hole at all, so the gate says
nothing about them. It is live over exactly three: **demo plain, demo debug, config_b**.

All three pass. The four canonical ROMs, deleted first and rebuilt through the wired path at
aeon `03ed1f1c` (the provenance tip's `aeon_rev`), reproduce the tip byte for byte:

| shape | crc32 | bytes |
|---|---|---|
| `s4.bin` | `711cc4ff` | 719205 |
| `s4.debug.bin` | `5785610a` | 735818 |
| `demo.bin` | `3415e3ef` | 96372 |
| `demo.debug.bin` | `7599953e` | 101113 |

## The declaration that used to fail it

The wiring waited on aeon, and this is the record of why. Measured against
`AEON_DIR=/home/volence/sonic_hacks/.aeon-freeze-slope` (`9bba8700`), demo plain, demo debug
and config_b each returned one fault, identical in all three:

```
[map.hole-interior-occupied] the hole declared after `Z80_IdleProgram` — interior
[0x3D0,0x3FE), reserved for `engine.z80_init` — is occupied at [0x3F8,0x3FE) by
byte-emitting section `boot_tail` (head `BootData_PostBlob`).
```

The cause was the declaration, not the layout. Both `games/sonic4/map.toml` and
`games/demo/map.toml` declared `after = "Z80_IdleProgram" / at = 0x3FE`, written at
`a7375682` (2026-08-01, K1) and untouched since, while the layout had moved twice — the
plain boot region grew (`EntryPoint`'s `lea (SYSTEM_STACK).w, sp`) and the idle body grew
38 → 40 bytes (`ld sp, hl` → `ld sp, nn`). The idle occupies `[0x3D0,0x3F8)` and
`boot_tail`'s first byte — the `$9F` PSG-silence byte the map's own comment calls "post-hole
AS data" — sits at `0x3F8`. `boot_data_port.rs::config_b_boot_data_hole_filled` already
asserted `0x3D0` / `0x3F8` against the golden, so the ROM was right and the map was wrong.

`map.toml` is aeon's file and the correction was theirs to make. **They made it**: at
`03ed1f1c` both maps declare `at = 0x3F8`.

## The positive control, through the wired build

Restoring the stale `0x3FE` on a throwaway `cp -a` of the aeon tree — never in the aeon
tree itself — makes the shipped build refuse, and restoring `0x3F8` makes it build again:

* `games/demo/map.toml` → `at = 0x3FE`, then `./build.sh demo`: exits non-zero with
  `error: native build (demo plain): [map.hole-interior-occupied] … occupied at
  [0x3F8,0x3FE) by byte-emitting section boot_tail (head BootData_PostBlob)`, and no
  `demo.bin` is written. Back at `0x3F8` the same tree builds `3415e3ef` / 96372.
* `games/sonic4/map.toml` → `at = 0x3FE`, then `sigil build --native --config-b`: the same
  fault under `error: native build (config_b)`, no output file. Back at `0x3F8` it builds.

## Coverage, and what it is not

`crates/sigil-cli/tests/hole_interior_reserved.rs` drives the shipped shapes: green over
each shape's real `map.toml` and real resolve, and red once one hole's declared right edge
is moved *in memory* past the post-hole data. Both the doctored edge and the expected
occupied span are read off the resolve, so neither is transcribed, and the aeon tree is
never written to.

Its population guard, `some_shipped_shape_declares_a_live_hole`, names the shapes with a
live hole and fails if there are none. Without it the file would go on reporting green the
day the last live hole is gated away — which is the failure mode `Ok(vec![])`-as-coverage
describes.

`native::placement_validation_tests` holds the predicate's own fixtures and, since the
wiring, six more that drive `validate_placement`: the occupied fault with its control, the
every-occupant report, and the four unmeasurable refusals the build path can reach.

## Reproduction

* `CARGO_TARGET_DIR` on disk, never `/tmp` (tmpfs), never shared with another worktree.
* `AEON_DIR` = a detached aeon worktree at the provenance tip's `aeon_rev`, **with all four
  canonical shapes built in it** — a source-only checkout has no reference ROMs and the
  strict suite refuses on every gate that reads one.
* `cargo test --release -p sigil-harness --lib placement_validation` → 29 passed / 0 failed.
* `SIGIL_STRICT_GATE=1 cargo test --release -p sigil-cli --test hole_interior_reserved`
  → 3 passed / 0 failed, printing `live-hole shapes: ["demo plain", "demo debug",
  "config_b"]`.
