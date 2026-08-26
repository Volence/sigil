# BGROOM-3 — measure at the packed base (packet)

Branch `fix/measure-at-packed-base` (from master a0fbee24). Aeon source: `.aeon-landing`
at 058ad606 (ROM CRCs verified before any work: s4 875d591f/699223, s4.debug
a02d36db/715114, demo bf2cdb42/96412, demo.debug 62a0019e/101120 — the provenance tip).

## 1. Reproduction (step 1) — CONFIRMED, mechanism RE-DIAGNOSED

Shadow COPY of `.aeon-landing` (rsync, no `.git`, no top-level ROMs), seven `nop`s
injected at the top of `RingCollision` (`engine/objects/rings.emp`), then the same
entry `sigil build` uses (`sigil build --aeon <copy> --native --game sonic4`, i.e.
`native::build_rom_chained_with_listing` on the sonic4 profile):

- clean copy: `crc=875d591f len=699223` (identity), wall 1.20 s
- grown copy, exact failure:

```
error: native build (sonic4 plain): packed layout overlaps at its real bases — a run grew
into a declared anchor; the anchor is hardware-fixed, so this content does not fit and
needs a hand ruling (the map's re-layout parcel, or less content). span pass:
resolve_layout: 1 diag(s); first Some(Diagnostic { level: Error, message: "sections
`section\036` [0x5D3C, 0x6168) and `player_sensors\050` [0x5860, 0x5D54) overlap in the
image (colliding pins)", ... })
```

Numbers (from a temporary trace of `packed_true_bases`, not committed):

| quantity | value |
|---|---|
| `player_sensors` frozen provisional base | 0x5840 |
| packed base after +14 B upstream | 0x5860 (+0x20: growth rounded to its align-16) |
| length every MEASURING round reported (`img`, `img2`, rounds 0 and 1) | **0x4DC** |
| length at the packed base (the checked resolve's extent 0x5860..0x5D54) | **0x4F4** |
| difference | 0x18 = 24 B = 12 sites × 2 B |
| differing sites | the 12 `lea` in `games/sonic4/player/player_sensors.emp` (lines 202 `SolidityTable`, 208 `AngleTable`, 214 `{ptable}` = HeightMaps/HeightMapsRot, in `probe_core`, instantiated ×4) — abs.w (4 B) in the measurement, abs.l (6 B) at the real base |

Both rounds' measurements were `distorted` (the collision fallback fired: round 0 on
`rings` [0x37F4,0x39BA) vs `entity_window` [0x39AC,...) — the injected growth itself;
round 1 on the innocent pair), and the walk exited on "lengths stable + still
colliding", which it reads as an anchor overrun.

**Mechanism — aeon's story refuted in detail, the symptom confirmed.** The tables live
at 0x296F0..0x2A7F0 (`collision_data`), above $8000 at ANY real base, so an unsized
`lea Table, a1` is abs.l at the provisional pin too; and an UNRESOLVED `RelaxAbsSym`
target is a hard error (`relax.rs` `unresolved_abs_target_diag`), never a short
encoding. What actually happens: when the pinned measuring resolve collides,
`image_lens_pinned(.., scratch_data = true)` parks every pure-data section at a scratch
slot `0x70_0000 + k·0x10_0000`. `collision_data` is slot k = 41 → **0x300_0000**, and
`asl_width_rule` masks the address to 24 bits → **0x0**; `SolidityTable` (offset 0x2100)
therefore looks like `$2100`, abs.w-reachable, and all 12 sites measure 4 B. (The
native.rs comment even claimed the opposite direction — "code measures LONGER while the
data is at scratch".) The spread fallback inherits the same scratch pass. So the
measurement lied SHORT, the pack round placed `section` 24 B into `player_sensors`, and
the "stable-but-colliding" exit mis-labelled it an anchor overrun.

## 2. Design — (A), measurement that cannot lie

Measure every round AT ITS OWN BASES, exactly: labeled sections pinned at the round's
bases (provisional in round 0, packed after), and let the measuring resolve TOLERATE
overlap — a section's length at a pin depends only on label addresses (pin + offset),
never on whether a neighbour's extent intersects. Then measure → pack → re-measure to a
fixed point (≤ 8 rounds, unchanged budget); at the fixed point ONE checked resolve
proves the image sound, and a collision THERE is the real anchor-overrun message. The
scratch/spread fallbacks (the only places a substitute base could steer a width) are
deleted; the residual scratch for label-less blobs / phase banks stays inside the
abs.l window `[0x80_0000, 0xFF_8000)` of the 24-bit space with a loud refusal on
exhaustion. (C) exists as the non-convergence fallback: the diagnostic names each
section whose length still moves and the `file:line` of every width-flipping site with
both encodings' lengths.

**Why part of the fix lands in `sigil-link` (the brief named `native.rs`):**
`resolve_layout` refuses an overlapping layout after its fixpoint (check c2) and that
refusal is exactly what forced `native.rs` to measure at substitute bases. The
measuring device needs the relaxer's exact widths at colliding pins, so the seam is a
`resolve_layout_measuring` variant in `sigil-link/src/relax.rs` that runs the same
fixpoint and skips only the image-soundness checks (overlap, bank straddle) —
`resolve_layout` itself is unchanged (a wrapper over the shared body). All planning
logic stays in `native.rs`.

(B) (pack pure data first) was rejected: it still measures code at a base that is not
its final one, so it only narrows the window; (A) closes it.

## 2b. Design revision (found while proving identity)

The first cut of (A) — every section, frozen-labeled or not, measured at the walk's
own bases — built both copies but MOVED CLEAN BYTES (clean crc a35f7c1a, then
e7928778/699207): the sections OUTSIDE the frozen table (`replay`, `raster`,
`page_cache`, …) have always measured at the far scratch, and the far slot is what
reproduces asl's conservative widths for references that touch them (asl encodes a
forward reference abs.l; at the real sub-$8000 base sigil's relaxer settles abs.w and
the chained layout packs 2 bytes tighter per site — the file-top "tighter fixpoint
than asl" note, re-met empirically at `Input_Tick` −6 / `HBlank_Install` +0x10). The
far scratch for never-pinned sections is therefore load-bearing equilibrium, not an
accident, and it stays. What shipped:

- `measure_pinned`: measuring rounds go through `resolve_layout_measuring` — exact at
  the round's own pins, overlap tolerated. The scratch-retry and +0x400/rank spread
  fallbacks are DELETED (with them, the only substitute bases that could steer a
  width, and the ~0x400 single-section growth cap: 2500 injected nops — 5000 B — now
  builds where the old rig could not even measure it).
- Frozen-labeled sections measure at prov (round 0) then at their packed bases (every
  later round); never-pinned sections keep the LEGACY `0x70_0000 + k·0x10_0000`
  scratch (asl-width emulation, byte-identity constraint); phase banks at scratch
  (vma-addressed, org pads sized for the original position).
- `img2 == img` is now a true fixed point; ONE checked resolve at the converged bases
  is the anchor witness, so "grew into a declared anchor" can only name a real
  anchor overrun. Non-convergence names the width-flipping sites (`file:line`, both
  encodings) via the per-relaxable lens (`width_flip_report`).

## 3. Identity + behaviour under growth

- clean shadow copy (unmodified `.aeon-landing` sources): `crc=875d591f len=699223` —
  the provenance-tip identity, reproduced 3×.
- grown copy (seven `nop`s): builds, `crc=c9d2a96a len=699223`; vs the reference
  layout exactly 7 symbols move +14 (inside `rings` after the injection) and 932 move
  +16 (the aligned downstream pack) — `player_sensors` lands at a 16-aligned base
  where its measured length is its encoded length; no overlap. The +0x10 drift is
  BELOW `GROWTH_DRIFT_TOLERANCE` (0x1000), so per the §4 design no drift line is
  emitted for this size of growth; the 5000 B variant emits
  `[layout.provisional-drift]` for every moved section (`entity_window` `+0x1388`
  first), each naming the section, head label, both bases, delta, and "refreeze at
  landing".

## 4. Gates

- `crates/sigil-cli/tests/measure_at_packed_base.rs` (in SOURCE_GATES; zero artifact
  words by the audit's own regex):
  - `fourteen_bytes_of_upstream_code_growth_still_builds` — undoctored copy builds
    (copy mechanism is not the signal), doctored copy builds, images differ.
  - `parcel_scale_growth_packs_downstream_and_reports_the_drift` — 2500 nops build +
    drift warnings present.
  - RED-FIRST: with `native.rs` reverted to the pre-fix version (HEAD~2 at the time),
    both fail; the 14 B test dies with the exact innocent-overlap message
    ("packed layout overlaps at its real bases — a run grew into a declared anchor…"),
    the 5000 B test with the old spread's `span pass (spread round, post-growth)`
    refusal. Restored → both green.
- `native.rs` `derived_layout_tests` additions (synthetic, derived expectations):
  `base_dependent_length_reproduces_provisional_bases_at_frozen_sizes`,
  `growth_across_the_boundary_places_the_successor_from_the_long_form` (a
  RelaxAbsSym section straddling $8000: successor placed from the 6 B form),
  `an_unresolvable_operand_refuses_loud`, `width_flip_report_names_the_relaxing_site`.

## 5. Suite / clippy / timing

- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- Build-time cost: clean `sigil build --game sonic4` ≈ 1.20 s before the fix,
  ≈ 1.39 s after (3× each, same machine) — the measuring rounds now lower every
  relaxable through the tolerant resolve; grown ≈ 1.38 s.
- Full suite (record run, stamped pwd/HEAD/branch/aeon SHA in
  `target/suite-measure-at-packed-base.log`; HEAD 928514cd, aeon 058ad606, no commits
  between build and run): exit 0, wall 141 s, **341 `test result:` lines, 3878
  passed / 0 failed / 4 ignored, zero skips** — 3882 declared `#[test]`s. Bar on
  master was 3872/0/4 (3876 declared); the +6 are exactly this parcel's tests
  (`fourteen_bytes_of_upstream_code_growth_still_builds`,
  `parcel_scale_growth_packs_downstream_and_reports_the_drift`, and the four
  `derived_layout_tests` additions). No pre-existing test changed status. A first
  (discarded) run tripped `version_reports_the_head_of_the_tree_it_was_built_from`
  because a docs-only commit landed mid-run — the suite polices the no-commits
  invariant itself; the record run is clean.
- The seven full-image CRC gates, all green against `.aeon-landing` 058ad606:
  `native_full_sonic4_plain`, `native_full_sonic4_debug`, `config_a_full_file`,
  `config_b_full_file`, `demo_plain_full_file`, `demo_debug_full_file`,
  `lean_full_file`. CLI cross-check on the clean shadow copy: s4 875d591f/699223,
  s4.debug a02d36db/715114, demo bf2cdb42/96412, demo.debug 62a0019e/101120 — the
  branch's provenance tip exactly.

## Reply to EMP_PITFALLS §11 (what aeon can now un-pin)

§11's mechanism story needs one correction and its rule can be relaxed:

- CORRECTION: the abs.w round was NOT "the provisional address is still unknown" (an
  unresolved operand is a hard error in sigil's relaxer, and the tables are above
  $8000 at the provisional pin too). It was the collision-fallback scratch slot
  wrapping the 24-bit bus (`collision_data` at `0x300_0000` ≡ `0x0`), which only
  fired once upstream growth made the provisional pins collide — hence "+2/+6 built;
  +14 did not".
- UN-PIN: the spelling rule "always write `lea (Table).l, aN` for ROM tables" is no
  longer needed for build correctness. Unsized `lea` to any label in a FROZEN-labeled
  section now measures at the section's real base every round; labels in un-frozen
  sections measure at far scratch, which encodes abs.l — the same width asl picks.
  Explicit `.l` remains a fine style choice (it documents the 6-byte/cycle cost), but
  sigil will neither mis-measure nor mis-place an unsized spelling, and the
  `measure_at_packed_base` gate holds that line red-first.
- When the placer ever fails to converge, the refusal now names the RELAXING SITE
  (`file:line`, both encodings) instead of an innocent section pair.

## Deferred (ledger rows, same-commit)

- The legacy far-scratch cursor for never-pinned sections still strides past the
  24-bit wrap (slot k ≥ 9 aliases; an unlucky alias could under-measure a reference
  into an un-frozen section the same way §1's fallback did). It survives because the
  frozen equilibrium — and byte identity — ride on it. Kill condition: pin the
  un-frozen sections (map rows or a refreeze that names them), then measure them at
  real bases and delete the cursor.
- No constructive red test for the walk's 8-round non-convergence exit (a genuine
  abs.w/abs.l oscillator needs a target that moves DOWN when an encoding grows, which
  the grow-only pack cannot produce); the diagnostic payload is unit-tested directly.

## Situational note (2026-08-26, from the coordinator)

While this parcel was in flight the aeon lane pushed a refreeze to sigil master
(79e4e242; new tip s4 6e41952f / s4.debug 7d52827c). This branch deliberately stays
on its own consistent pair — provenance tip 875d591f/a02d36db + `.aeon-landing`
058ad606 — and the landing owner re-proves on the merged tree.
