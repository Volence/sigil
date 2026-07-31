# 2026-08-01 — CONV-B PARCEL B: the engine residual-constants tail (close packet)

Status: **Checkpoint for the overseer's countersign + merge.** Branch pair
`conv-b-constants` (aeon + sigil). The P5 constants ownership flip is EXTENDED to
the last residual `=` in `engine/constants.asm`; the file is **DELETED**. Every
target byte-identical to the chain-9 goldens. NOT merged — the merge is the
overseer's.

## §0 — THE HEADLINE

`engine/constants.asm` is **DELETED**. Its 129 residual `=` definitions moved to
`engine/system/constants.emp` as `pub const`s — the same harvest→inject mechanism
P5 built for the first 114 (`harvest_engine_constants` → `eval_all_pub_consts` →
guarded `-D` defines + `attach_guarded_equ_exports`; **no sigil harvester change
needed** — the harvest auto-reads every `pub const`). VDP_DATA/VDP_CTRL moved from
`engine.vdp` into `engine.constants` (the hardware-address authority the residual
AS harvests). Every consumer that mirrored a residual constant now `use`s it; the
retired drift walls + their negative probes re-homed to six-target byte-identity;
the three AS `if/error` invariant blocks re-homed to `.emp` `ensure`s. **Byte-
identical to the chain-9 goldens across all six targets.**

## §1 — PREMISE CORRECTIONS (census vs reality)

- **The census said "147 `=`; files: 1; effort M."** Re-count of current source:
  **129** residual `=` (several parcels landed between the 2026-07-31 census and
  now). And **files: 1 is wrong** — the flip is NOT a single-file move. The build's
  `normalize_helper_imports` glob-injects `use engine.constants.*` into every
  module, so moving a constant to `constants.emp` while a consumer keeps its local
  `const X` mirror is a **name collision** — the full consumer flip (delete mirror,
  add explicit `use`, retire wall) is forced, exactly the tranche-15 SECTION_SIZE
  precedent. Real touch: **23 aeon files** (constants.emp + 21 consumers + engine.inc,
  minus the deleted constants.asm) + **7 sigil files** (6 port tests + m1c_root.asm).
- **`extern()` still resolves after the flip.** `attach_guarded_equ_exports`
  (frontend-as eval.rs:128/155) link-exports the guarded defines, so real-code
  `extern("X")` sites (`sound_debug.emp`/`z80_init.emp` `Z80_RAM`,
  `act_descriptor.emp` `BLOCK_INDEX_SIZE`) resolve unchanged and were **left as
  extern** (zero risk), and `engine.vdp`'s six `target_bits/op_bits == extern("…")`
  mapper cross-checks keep working.
- **VDP_DATA/VDP_CTRL had to move.** `boot_data.asm` uses `dc.l VDP_DATA` (residual
  AS), and only `constants.emp` is harvested — so the vdp.emp `pub const` authorship
  moved to `constants.emp`, and the 8 `use engine.vdp.{VDP_DATA, VDP_CTRL}` consumer
  imports repointed to `engine.constants` (VDP_MODE*_OFF stay in engine.vdp).

## §2 — THE FLIP (aeon)

- `engine/system/constants.emp`: **+129 `pub const`s** (hardware addresses, VDP
  access-type bits, VRAM layout, timing, buttons, game-state ids, DMA budgets,
  decompression, the full COLLISION_* / ST_* / AF_* / PHYS_* / SOLID_* families,
  section-tile geometry, parallax, tile-cache COLL_*/BLOCK_* block format, camera,
  the entity-window scan block, slot tags, OEF_* placement flags). **Derived
  constants stay expressions** (`BLOCK_COLL_SIZE = BLOCK_COLL_PLANE_SIZE *
  TILE_CACHE_COLL_PLANES`, `BG_TILE_BASE_SLOT = BG_TILE_BASE_VRAM / 32`,
  `ENTITY_RESCAN_ROW_SIZE = $10000 - ENTITY_RESCAN_COARSE_MASK`, …) — the value is
  never baked, so it can't drift.
- `engine/constants.asm`: **DELETED**; include removed from `engine/engine.inc`.
- `engine/vdp.emp`: `pub const VDP_DATA/VDP_CTRL` + their 2 drift walls DELETED
  (moved to constants.emp); the 6 `target_bits/op_bits` mapper walls KEPT (extern
  still resolves), messages re-pointed `engine/constants.asm`→`engine.constants`.
- **21 consumer .emp files**: local mirror `const X` → `use engine.constants.{X}`;
  tautological drift walls `ensure(extern("X") == X)` deleted. Files: bg, camera,
  parallax, plane_buffer, section, tile_cache, collision_lookup, entity_window,
  boot, buffers, dma_queue, ojz_scroll_test, act_descriptor, game_debug,
  test_objects, player_sensors, s4lz_decompress, object_test_state (+ the vdp
  repoint touched bg/parallax/plane_buffer/section/boot/buffers/dma_queue/ojz).
- **The three AS `if/error` invariants** re-homed to `engine/objects/entity_window.emp`
  as `ensure`s: the SECTION_SIZE power-of-two invariant and the two Y-band
  coarse-row coverage invariants (they're real cross-constant checks, not
  tautologies — kept, re-homed to the consumer that imports the constants).

## §3 — THE BYTE-IDENTITY PROOF (all six vs chain-9)

| target | built | chain-9 golden | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ |
| demo        | `9bb8c993` / 90506  | `9bb8c993` / 90506  | ✓ |
| demo.debug  | `bc7678d0` / 93006  | `bc7678d0` / 93006  | ✓ |
| config_a    | `78df5e6a` / 422297 | `78df5e6a` / 422297 | ✓ (strict-suite byte-identity) |
| config_b    | `f38f609b` / 303501 | `f38f609b` / 303501 | ✓ (strict-suite byte-identity) |

s4/s4.debug/demo/demo.debug verified directly; config_a/config_b via the strict
suite's native-chained byte-identity tests. Ownership move at unchanged bytes —
no re-freeze, no oracle A/B.

## §4 — THE RETIRED-TEST ENUMERATION (strict 2854 → 2849, net −5)

Five `doctored_X_fires_its_guard` negative probes exercised drift walls the flip
DELETED, so they cannot fire. Each re-homes to the six-target byte-identity (a
wrong value moves ROM bytes).

| retired test | file | re-homed to |
|---|---|---|
| `doctored_psg_port_fires_its_guard` | boot_port | byte-identity (PSG displacement) |
| `doctored_cam_max_y_step_fires_its_guard` | camera_port | byte-identity |
| `doctored_parallax_lerp_shift_fires_its_guard` | parallax_port | byte-identity |
| `doctored_plane_buffer_size_fires_its_guard` | plane_buffer_port | byte-identity |
| `doctored_tile_size_fires_its_guard` | s4lz_port | byte-identity |

Net: −5 → **2849 passed / 0 failed / 4 ignored**.

### Non-retirement test edits (count-neutral)
- `act_descriptor_port.rs`: the "drifted mirror" count `assert_eq!(drifted, 1)` →
  `0` — MAX_ACT_SECTIONS flipped, so no drifted-mirror assert survives (the two
  MAX_ACT_SECTIONS *invariant* ensures carry no "drifted" text).
- `boot_port.rs::lower_boot`: prepends `constants.emp` items (boot.emp newly reads
  VDP_DATA/VDP_CTRL/PSG_PORT via `use engine.constants`, so the standalone lower
  needs the twin — the structs-§4 ambient-seed pattern).
- `crates/sigil-harness/m1c_root.asm`: `include "engine/constants.asm"` removed
  (deleted); SYSTEM_STACK now arrives via the harvest, as its comment records.
- The camera/parallax/plane_buffer/s4lz standalone harnesses that already prepend
  `constants.emp` (or resolve via link) needed no change beyond the doctored
  retirement — their region byte-matches passed untouched.

## §5 — STEP-3 (retrospect) vs STEP-5 (engine optimization) FINDINGS

- **Step-3:** the flip removed ~55 drift-wall `ensure`s + ~85 local `const` mirrors
  across 21 `.emp` files and the 386-line `constants.asm` — a large strict
  reduction in twin scaffolding with zero behavior change. `engine.constants` is
  now the single authority for every engine hardware address, geometry constant,
  and enum id; VDP_DATA/VDP_CTRL rejoined it (the hardware-address family is whole
  again, reversing the earlier "vdp.emp is the right owner" placement now that the
  residual AS's harvest requirement decides it). The kill-list closes/advances
  rows 8, 22 (engine-const half), 44, 47.
- **Step-5:** none. Pure ownership move; no lowering changed, no bytes moved. The
  §17 opt-sweep owns byte-changers.

## §6 — NEITHER-BUCKET HEADLINES

- **`VRAM_PLANE_B` ($E000) and `VRAM_WINDOW` ($F000) are DEAD** — declared in
  `constants.asm`, no consumer in `.asm` or `.emp`. Moved faithfully (the flip is
  an ownership move, not a dead-code cull), but they are delete-candidates (§7).
  `VRAM_PLANE_B` also duplicates the value of the live `VRAM_PLANE_B_BYTES`.
- **The full build never needs the explicit `use engine.constants.{X}` lines** —
  `normalize_helper_imports` drops + re-globs them. They exist for the STANDALONE
  port-test lowerings (curated ambient, no glob), matching the tranche-15
  precedent. So each consumer edit is "delete mirror + add explicit `use`", never
  a bare-reference change.
- **`s4lz_port`'s standalone TILE_SIZE resolves via the value-seam link equ**, not
  the new `use engine.constants` path (the harness doesn't prepend constants.emp).
  Region byte-matches, so functionally proven; a fidelity nicety is logged (§7).

## §7 — GAP-LEDGER SWEEP (nice-to-haves NOT implemented → campaign-gap-ledger.md)

1. **`VRAM_PLANE_B` / `VRAM_WINDOW` are dead** (no consumer) and `VRAM_PLANE_B`
   duplicates `VRAM_PLANE_B_BYTES`'s value. Delete-candidates; kept in the flip for
   a faithful ownership move. A separate dead-constant cull (byte-neutral: they
   only removed an unused injected define) should retire them.
2. **Stale `engine/constants.asm` comment references in UNTOUCHED files** (e.g.
   `engine/structs.emp`, several sigil port-test doc comments) still name the
   deleted file as "truth". Fix-at-next-touch per the exhibit-comments rule; the
   files this parcel edited were cleaned in-place.
3. **`s4lz_port` standalone could prepend `constants.emp`** so its lowering
   exercises the real `use engine.constants.{TILE_SIZE}` path instead of the
   value-seam link equ (region byte-match unaffected either way). Left minimal.
4. **`act_sec_field_equs()` / `engine_constant_equs()` supply-only helpers** in
   `test_support` may now carry names no standalone test reads after this flip's
   consumer consolidation — a harness-helper prune candidate (harmless supply).
