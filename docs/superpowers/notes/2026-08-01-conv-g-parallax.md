# 2026-08-01 — CONV-G PARCEL G: the parallax config DSL (close packet)

Status: **Checkpoint for the overseer's countersign + merge.** Branch pair
`conv-g-parallax` (aeon + sigil). The §4.6 parallax macro layer + the 10 config
files are `.emp`-native; six targets byte-identical; strict green at the baseline
count. NOT merged — the merge is the overseer's.

## §0 — THE HEADLINE

`engine/parallax_macros.inc` and all ten `data/parallax/*.asm` config files are
DELETED. The §4.6 parallax authoring vocabulary is now `engine/level/parallax_dsl.emp`
(the factor encoding + the deform-table generators — a pure-comptime helper); the
OJZ parallax block (6 deform tables + 20 `parallax_config` records) is
`games/sonic4/data/parallax/configs.emp`, placed natively at the pinned
`PARALLAX_CONFIGS` region. **Byte-identical to the chain goldens across all six
targets** — a pure ownership+placement move at unchanged bytes.

## §1 — THE DESIGN GATE OUTCOME: existing surfaces sufficed (no STOP)

The census phrase "a parallax-config construct" is discharged by **existing
surfaces** — no new grammar. This was pre-ratified: SIGIL_SPEC2_LANGUAGE.md
**Appendix A** is a worked `.emp` design for exactly this family, and every surface
it needs is implemented and glob-visible:

- `as.sin`/`as.int` (bit-compatible with asl 1.42, golden-gated in `float_ns.rs`) +
  `comptime for i in 0..256 {…}` → the deform tables (`float_ns`/`corpus_p5` already
  prove the four shipping sine amplitudes byte-for-byte).
- comptime fns with **default params** (`vdp_comm_reg`'s `clr: bool = true` precedent),
  `Label` params (null = `0`), struct literals with `*u8` fields lowering to SymRef
  relocations (the `act_descriptor.emp` precedent), and nested struct fields
  (`ParallaxCfgN { hdr: parallax_config, bands: [band_entry; N] }`).

The one place the spec's Appendix-A sketch does NOT lower literally: `ParallaxConfig{…}
++ bands.map(band_entry)`. `++` (`eval_concat`) requires both operands already be the
same kind (`Data++Data`/`Array++Array`), and pointer relocations only flow through a
data-item's **declared type** (`lower_to_data`), not a comptime `Data` value. So the
implemented shape is the **nested wrapper struct** (`ParallaxCfgN`) emitted as one
typed `data` item — the header authored once by the shared `parallax_config` struct,
the bands a `[band_entry; N]` field. Byte-identity is the proof.

**Readability judgment (the gate's test — "would the ported configs read as well as
the .inc?"):** yes, arguably better. The effect variants collapse to one-liners
(`pub data ParallaxConfig_Shimmer: ParallaxCfg1 = shimmer_bg(speed: 3)`), the
per-band deform shifts are explicit `band(…, dsa:, dsb:)` args instead of the AS
`BAND_DSA :=` mutable-global dance between `band` calls, and the header defaults live
in one `hdr()` signature instead of the macro's fourteen empty-string checks. No
ceremony tax.

## §2 — THE MACRO → COMPTIME MAPPING

| AS (`parallax_macros.inc`) | `.emp` |
|---|---|
| `packed(s1,s2,op)` function + `FACTOR_*` equates | `parallax_dsl.packed()` + `pub const FACTOR_*` (identical arithmetic) |
| `parallax_section` macro (28-byte header, `:=` accumulators, `org` back-patch) | `configs.hdr() -> parallax_config` (band_count a computed field — no back-patch) |
| `band` macro (10-byte record, factor split, monotonic/count `fatal`s) | `configs.band() -> band_entry` (the `(fa>>4)&15` split, typed) |
| `parallax_section_end` (patches count byte via `org`) | gone — `band_count` is a struct field |
| `deform_table_sine` (`rept 256 / dc.b int(A·sin(2π·i/P))`) | `parallax_dsl.deform_sine() -> [i8;256]` (`as.sin`/`as.int`, `comptime for`) |
| `deform_table_triangle`, `v_column_perspective`, `v_column_floor` | ported to `parallax_dsl` (integer `comptime for`) |
| `shimmer_bg`/`haze_fg`/`rocking`/`perspective` (per-effect file macros) | `configs.*` comptime fns returning `ParallaxCfgN` |
| `parallax_combine` / `parallax_combine_split` sugar | NOT ported (see §5 step-5) — `sky_haze` is authored directly |
| all-zero table (`rept 256 / dc.b 0`) | `parallax_dsl.deform_zero()` |

## §3 — THE BYTE-IDENTITY PROOF (all six vs the chain goldens)

| target | built | golden | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ |
| demo        | `9bb8c993` / 90506  | `9bb8c993`          | ✓ |
| demo.debug  | `bc7678d0` / 93006  | `bc7678d0`          | ✓ |
| config_a    | `78df5e6a` / 422297 | `78df5e6a`          | ✓ |
| config_b    | `f38f609b` / 303501 | `f38f609b`          | ✓ |

Gates: `native_full_rom` (plain+debug), `native_offcanonical_full` (demo/config ×4),
`native_offcanonical_rom` (anchors), `native_offcanonical_placement` (frozen
placement + native rederive). All green. Data-level: `parallax_configs_port.rs`
lowers the real module through `build_emp` and byte-diffs the region window
`[0x11294, 0x11D1E)` (plain) / `[0x1131C, 0x11DA6)` (debug).

## §4 — PER-FILE PORT CENSUS (#36–45)

The block is one contiguous native region (`PARALLAX_CONFIGS`, 0xA8A bytes), emitted
in main.asm include order:

| # | old file | → | new home |
|---|---|---|---|
| 40 | `ojz_default.asm` | → | `DeformTable_Zero` + `ParallaxConfig_OJZ_Default` (configs.emp) |
| 41 | `ojz_windy.asm` | → | `DeformTable_OJZ_Calm` + `ParallaxConfig_OJZ_Windy` |
| 39 | `effects/shimmer.asm` | → | `DeformTable_Shimmer` + `shimmer_bg` + 3 variants |
| 36 | `effects/haze.asm` | → | `DeformTable_Haze` + `haze_fg` + 4 variants |
| 38 | `effects/rocking.asm` | → | `DeformTable_Rocking` + `rocking` + 3 variants |
| 37 | `effects/perspective.asm` | → | `DeformTable_Perspective` (`v_column_floor`) + `perspective` + 3 variants |
| 45 | `scenes/windy_haze.asm` | → | `ParallaxConfig_WindyHaze` (direct hdr/band) |
| 44 | `scenes/sky_haze.asm` | → | `ParallaxConfig_SkyHaze` (direct — was `parallax_combine_split`) |
| 42 | `scenes/caves.asm` | → | `ParallaxConfig_OJZ_Caves` |
| 43 | `scenes/locked_clouds.asm` | → | `ParallaxConfig_OJZ_LockedClouds` |
| (macro layer) | `parallax_macros.inc` | → | `engine/level/parallax_dsl.emp` |

## §5 — RETIREMENTS + RE-HOMES · STEP-3 / STEP-5

**Retirements (11 files deleted):** the 10 `data/parallax/*.asm` + `parallax_macros.inc`.
Their includes drop from `main.asm` (the block is native, resume orgs unchanged) and
from `engine/engine.inc` + the test root `m1c_root.asm`.

**Re-homes (sigil-side placement wiring):**
- `pins.rs`: new `PARALLAX_CONFIGS` region (`repin.toml` region added; `repin` reports
  0 pins changed — the hand-computed bases were exact).
- `native.rs`: registry entry (`games.sonic4.parallax_configs` @ `parallax_configs`),
  gate `SIGIL_EMP_PARALLAX_CONFIGS`, `engine.level.parallax_dsl` added to
  `COMPTIME_HELPERS`.
- `s4.txt`/`s4_debug.txt` (frozen canonical tables): the section head anchor
  `DeformTable_Zero` added (0x11294 / 0x1131C). NOTE: `--bootstrap-canonical` (strict
  resolve) is unavailable here — it trips the *pre-existing* `objdefs`/org-$11D7E
  8-byte overlap the lenient frozen chainer tolerates — so the anchor was added
  directly, consistent with the frozen-loop rederive.

**Test bookkeeping:** NO test retired — the port is byte-neutral and every existing
guard passes (strict 2866→**2868** with the two new `parallax_configs_port` gates).
`band_entry` made `pub` (imported by the config module); its harvest row is unchanged
(still `engine/level/parallax.emp`).

**Step-3 (retrospect):** the per-band `BAND_DSA :=`/`BAND_PHASE :=` mutable-global
idiom (the macro's stateful accumulators) becomes plain per-`band()` args — a strict
reduction in hidden state. The `org` back-patch of the count byte is gone (computed
field). No construct WISH: Appendix A's surfaces sufficed; the only divergence
(wrapper struct vs `Struct ++ Array`) is an implementation reality of how relocations
lower, not a missing feature.

**Step-5 (engine optimization):** dropped the `parallax_combine` / `parallax_combine_split`
sugar — `parallax_combine` had zero consumers; `parallax_combine_split` had one
(`sky_haze`), inlined as a direct 2-band config that reads clearer than the
where-mask macro. Byte-identical. (Gap-ledgered for re-add if a multi-effect-stack
authoring need returns.)

## §6 — NEITHER-BUCKET HEADLINES

- **The wrapper structs `ParallaxCfg1/2/4/5` exist only to carry the header+bands
  byte shape** — the runtime never reads them (it walks bytes off the header pointer).
  One shape per shipping band count (1/2/4/5). A reader expecting a single
  `parallax_config`-typed record will find the header nested as `.hdr`.
- **`deform_triangle` + `v_column_perspective` are ported but unused** (no shipping
  config references them; the AS self-test that exercised them was `ifdef`-gated off).
  Kept as the faithful library port of the `.inc` generators, verified by construction
  (integer math mirrors the AS `dc.b` expressions). Gap-ledgered for a probe test.

## §7 — GAP-LEDGER SWEEP (→ campaign-gap-ledger.md)

1. **`parallax_combine` / `parallax_combine_split` sugar** — dropped (0 / 1 consumer,
   the 1 inlined). Re-add as `configs.emp` comptime fns if a game wants uniform or
   split multi-effect stacks without hand-authoring bands.
2. **`deform_triangle` / `v_column_perspective` probe test** — ported but unused; add
   a `parallax_dsl` unit probe (like `sonic_anims`'s `rep`) if they stay long-term, or
   delete on a "no consumer in N parcels" rule.
3. **The frozen-table `--bootstrap-canonical` strict overlap** (`objdefs` vs the
   org-$11D7E resume, 8 bytes) is pre-existing and load-bearing only for a full
   re-bootstrap; the normal frozen-loop rederive is unaffected. Worth a look when the
   capstone (Parcel K) recomputes resume orgs from placement.

## §8 — KILL-LIST (→ twin-scaffolding-kill-list.md)

The parallax macro-layer twin (`parallax_macros.inc`) and the ten `.asm` config twins
are DELETED — no scaffolding survives this parcel (the `.emp` is the sole source).
No new twin introduced.
