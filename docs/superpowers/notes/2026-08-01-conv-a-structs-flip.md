# 2026-08-01 — CONV-A PARCEL A: the structs flip (close packet)

Status: **Checkpoint for the overseer's countersign + merge.** Branch pair
`conv-a-structs` (aeon `c745d75` structs flip → sigil `9b7fcf7` harvester →
sigil test-ripple commit). The struct-offset sibling of the P5 constants flip is
DONE, byte-neutral across all six targets, strict green. NOT merged — the merge is
the overseer's.

## §0 — THE HEADLINE

`engine/structs.asm` is DELETED. The `.emp` struct twins are the sole author of
the object / section / DMA-queue / parallax-config / VDP-shadow layouts; the
residual AS reads their field offsets + sizes through a new **struct-offset
harvest** (`harvest_engine_struct_offsets`), the sibling mechanism of
`harvest_engine_constants`. Every per-field / sizeof drift wall and the
`VDP_Shadow_len` bridge RETIRED — they became tautologies once the `.emp` owns the
layout. **Byte-identical to the chain-9 goldens across all six targets.**

## §1 — THE MECHANISM (sigil)

- `sigil_frontend_emp::layout::layout_struct_ambient(file, ambient, name)` — new
  pub fn: lay out a struct with a cross-file TYPE ENVIRONMENT (so `Sst`'s
  `Coord`/`Velocity`/… newtypes from `engine.types` resolve standalone). The seven
  other twins are all-primitive and ignore the ambient.
- `sigil_harness::native::harvest_engine_struct_offsets(aeon)` — reads each of the
  eight `.emp` struct twins (a fixed `(file, struct, AS-prefix)` table) and emits
  the `<Struct>_<field>` = offsetof + `<Struct>_len` = sizeof equs that
  `structs.asm`'s `struct … endstruct` generated, plus the one derived equate
  `SST_interact` (= `sizeof(Sst) - 2`, the object record's tail word). `engine.types`
  is the shared ambient. The AS prefix is verbatim the AS spelling — the `.emp`
  type `Sst` carried the `SST_*` equs, so its prefix is `SST`; the new `VdpShadow`
  struct carries `VDP_Shadow_*`.
- `assemble_as_side` extends `guarded_defines` with the struct harvest right after
  the constant harvest — same harvest→inject ordering, `.emp` → residual AS. The
  build.sh CLI path (`build_rom_chained` → `assemble_as_side`) is on this path.
- `VDP_Shadow_len` stays EXCLUDED from `harvest_engine_constants` (single injector):
  the `VdpShadow` struct owns it now (the reason changed from "structs.asm owns it"
  to "the struct harvest owns it").

## §2 — THE AEON FLIP

- `engine/structs.asm` DELETED; its `include` removed from `engine/engine.inc`.
- `engine/structs.emp`: the per-field + sizeof drift walls (Act/Sec/DMAEntry/
  parallax_config) DELETED; a new `pub struct VdpShadow` (19 `u8` fields) added as
  the VDP-shadow layout author.
- `engine/objects/sst.emp`: the 30 `SST_*` walls DELETED (Sst keeps its `@` field
  offsets + `(size: $50)`, verified in-file).
- `engine/objects/entity_window.emp`: the `EntityScanState_*` walls DELETED
  (keeps `@` + `(size: $1A)`).
- `engine/level/parallax.emp`: the `band_entry_*` walls DELETED.
- `engine/vdp.emp`: the four `VDP_MODE*_OFF == extern("VDP_Shadow_vdp_*")` walls
  DELETED; the convenience offsets stay literals (a wrong value moves the emitted
  `VDP_Shadow_Table + off` displacement → byte-identity is the drift check).
- `engine/system/constants.emp`: the `VDP_Shadow_len` drift guard DELETED; the
  `pub const VDP_Shadow_len = 19` named immediate kept (byte-identity checks it).
- `engine/objects/collision.emp` + `games/sonic4/player/player_sensors.emp`: the
  `ensure(extern("SST_interact") == interact_off())` walls DELETED;
  `interact_off()` (= `sizeof(Sst) - 2`) kept as the displacement-escape fn.

### The VDP_MODE*_OFF / VDP_Shadow_len design decision (recorded)
An earlier cut derived `VDP_MODE*_OFF = offsetof(VdpShadow, …)` and pinned
`VDP_Shadow_len == sizeof(VdpShadow)` in `vdp_init.emp` — the DRYer "single source
of truth" shape. REVERTED: it adds an `engine.structs` dependency to `engine.vdp` /
`vdp_init.emp`, which breaks the standalone port tests that lower those modules with
a curated (incomplete) ambient. Kept the literals + re-homed to byte-identity — the
same re-homing model as the Act/Sec walls (which never had `@` offsets either). The
`VdpShadow` struct is still the sole layout author for the AS-side symbols the
harvest injects.

## §3 — THE BYTE-IDENTITY PROOF (all six vs chain-9)

| target | built | chain-9 golden | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ |
| demo        | `9bb8c993` / 90506  | `9bb8c993`          | ✓ |
| demo.debug  | `bc7678d0` / 93006  | `bc7678d0` / 93006  | ✓ |
| config_a    | `78df5e6a` / 422297 | `78df5e6a`          | ✓ |
| config_b    | `f38f609b` / 303501 | `f38f609b`          | ✓ |

Ownership move at unchanged bytes — no re-freeze, no oracle A/B.

## §4 — THE RETIRED-TEST ENUMERATION (strict 2861 → 2854, net −7)

The struct drift walls the probes exercised are DELETED, so the probes cannot fire.
Each protection re-homes to (a) the in-file `@`/`(size:)` checks where the twin has
them, (b) the six-target byte-identity everywhere (a wrong offset moves ROM bytes),
and (c) the NEW positive harvester test (`structs_module`).

| retired test | file | re-homed to |
|---|---|---|
| `doctored_dmaentry_field_fires_its_guard` | dma_queue_port | byte-identity |
| `doctored_sibling_ptr_fires_its_guard` | children_port | byte-identity |
| `misspelled_sst_extern_dangles_loud_while_control_resolves` | tranche6 | link-checker inherent + constants probes |
| `drifted_sst_twin_fires_its_own_guard_naming_the_field` | tranche6 | sst.emp `@`/size + byte-identity |
| `drifted_sst_twin_fires_its_own_guard_naming_the_field` | tranche7 | sst.emp `@`/size + byte-identity |
| `act_standalone_twin_pins_fail_loud_on_missing_externs` | tranche4 | byte-identity |
| `standalone_drift_guard_fails_loud_on_the_missing_extern` (VDP_Shadow_len) | tranche4 | byte-identity |

`structs_module.rs`'s two old drift-wall tests (`per_field_drift_wall_passes`,
`doctored_field_offset_fires_its_guard`) were REPLACED by two new harvester tests
(`harvest_emits_the_as_field_offsets_and_sizes`,
`sst_interact_is_the_record_tail_word`) — net 0 there, the POSITIVE re-home of the
deleted walls (they pin that the harvester emits the exact AS offsets/sizes).

Net: 7 probes retired − 0 (structs_module even) = **−7** → 2854 passed / 0 failed /
4 ignored.

### The guard-count ripple (byte-neutral, no tests lost)
~14 region-test `assert_drift_guards` sites hardcoded "sst.emp's 30" (the prepended
`SST_*` wall count) plus a module's own guards. With the wall retired the sst
contribution is 0, so each dropped to the module's own remaining guards:
animate/sprites/dplc/collision/test_objects/test_g1-static → 0; core/test_g2/g3 →
`twin_guards() + 1`; rings/test_g4-player → `+ 4`; test_p1_player threshold `> 40` →
`> 20` (27 now); act_descriptor `drifted 63` → `1`. `engine_constant_equs()` emptied
(VDP_Shadow_len's guard retired), so `twin_guards()` is 0 everywhere — the P5 base
list is now zero-length. `sst_field_equs()` kept as a SUPPLY-ONLY blob (its 30
offsets + SST_interact still feed `ojz_scroll_test.emp`'s legitimate `Player_1 +
SST_x_pos` field-address externs). `m1c_vector_table` + `m1c_root.asm` updated to
seed the struct-offset harvest (they assemble the residual AS front-matter, which
lost its `include "engine/structs.asm"`).

## §5 — STEP-3 (retrospect) vs STEP-5 (engine optimization) FINDINGS

- **Step-3:** the flip removed ~158 `extern()`-bearing drift-wall `ensure`s across 8
  `.emp` files and 258 lines of `structs.asm` — a strict reduction in twin
  scaffolding with no behavior change. The `SST` vs `Sst` naming (AS uppercase vs
  `.emp` type) is handled by the explicit AS-prefix in the harvest table, not a
  rename (renaming `Sst` would ripple through every consumer — out of scope).
- **Step-5:** none. This is a pure ownership move; no lowering changed, no bytes
  moved. The §17 opt-sweep owns byte-changers.

## §6 — NEITHER-BUCKET HEADLINES

- **`VdpShadow` now has no `.emp` consumer** — it exists purely as the harvest's
  layout author (the mode offsets + `_len` the residual AS + demo_state's
  `setVDPReg VDP_Shadow_vdp_mode2` read). Correct and byte-neutral, but a reader
  expecting a `use engine.structs.{VdpShadow}` somewhere will not find one.
- **The struct-offset harvest re-parses `structs.emp` five times** (once per struct
  it hosts). Negligible (a comptime lower of a zero-byte module), left simple.

## §7 — GAP-LEDGER SWEEP (nice-to-haves NOT implemented → campaign-gap-ledger.md)

1. **`VDP_MODE*_OFF` / `VDP_Shadow_len` as explicit comptime pins.** Currently
   re-homed to byte-identity (the deriving was reverted to keep `engine.vdp` /
   `vdp_init.emp` free of the `engine.structs` dependency that broke standalone
   port tests). If the standalone port tests learn to carry the full helper ambient
   (the real build already blankets it via `normalize_helper_imports`), derive
   `VDP_MODE*_OFF = offsetof(VdpShadow, …)` and pin `VDP_Shadow_len ==
   sizeof(VdpShadow)` for a single source of truth.
2. **`act_sec_field_equs()` is now fully supply-only-unused** (no `.emp` module reads
   `Act_*`/`Sec_*`/`DMAEntry_*`/`parallax_config_*` externs). Its prepend callers
   (act_descriptor_port, section_port, …) could drop the prepend and the helper could
   retire. Left as a harmless supply to keep the ripple minimal.
3. **`sst_field_equs()` could shrink to the four fields `ojz_scroll_test` actually
   externs** (`SST_x_pos`/`_y_pos`/`_x_vel`/`_y_vel`); kept whole for greppability.
