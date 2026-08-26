# Derived layout — §4 first-usable-step packet (2026-08-26)

Branch `feat/derived-layout` (from master `6030e4e6`; master moved to `8f8b03ee` during the
parcel, docs only). Implements §4 of
`docs/superpowers/notes/2026-08-26-derived-layout-design.md` (branch `design/derived-layout`
@ `0292d8e8`) under decision d-7: placement is derived from `map.toml` order and real section
sizes; the frozen tables are an after-the-fact check that never blocks `sigil build`.
No emulator. Nothing under `golden/`, `pins.rs`, `repin.toml`, `provenance.toml`, or aeon's
tracked files was touched. §3.5 (`section:<name>` row) NOT implemented — follow-on.

## What changed (all `crates/sigil-harness/src/native.rs` unless noted)

| Step | Symbol | Change |
|---|---|---|
| 1 | `measure_or_spread` | Fallback ladder on a pinned-resolve collision: (1) pure-data sections at disjoint scratch (`image_lens_pinned(.., scratch_data=true)`), (2) the `0x400`/rank spread. Returns `Option<String>` (the collision) instead of `bool`; either fallback is DISTORTED and never the fixpoint witness. |
| 1 | `image_lens_pinned` | `fixture: bool` → `scratch_data: bool`. Scratch is a fallback, not the default (see "deviation" below). |
| 1 | `declared_spans_by_index` | measures at true bases with no scratch (`false`). |
| 2 | `packed_true_bases`, `true_bases_by_index` | new `warnings: &mut Vec<BuildWarning>` parameter. The `GROWTH_DRIFT_TOLERANCE` hard `Err` is now `provisional_drift_warning` pushed per round, and only the CONVERGED round's reports reach the sink (one line per drifted section). |
| 2 | `provisional_drift_warning` (new) | `[layout.provisional-drift] section \`X\` (\`Head\`) packed at 0x.., frozen provisional 0x.. (delta +0x..); the frozen placement tables are stale against this content — refreeze at landing`. `Level::Warning`, `location: None`. |
| 2 | `build_rom_chained_with_listing` | passes `&mut warnings` → `report_warnings` in `main.rs` prints it on `sigil build`. |
| 3 | `packed_true_bases` labeled arm | `Some(_) if is_anchor_gap(p)` — a labeled section at a declared anchor is HELD absolute (no `!fixture ||`, no gap test). |
| 3 | `packed_true_bases` unlabeled arm | `Some(r) if p > r + ANCHOR_GAP && is_anchor_gap(p)` — `!fixture ||` dropped; the gap test STAYS (see hazard 2). |
| 3 | `packed_true_bases` convergence | lengths stable but pins still colliding ⇒ `Err("packed layout overlaps at its real bases — a run grew into a declared anchor …")` naming the overlap, instead of 8 rounds then "relaxation oscillation". |
| 3 | `resolve_frozen_sections` | passes the REAL anchor set (`placement_map` replaces `placement_map_order`) + a throwaway sink; repin/derive now walk with the same islands as the build. |
| — | `resolve_frozen_layout` (new pub), `ERROR_HANDLER_BLOB_LABEL/LEN` (now pub) | for the gate below. |
| tests | `derived_layout_tests` (6 unit tests, end of `native.rs`) | expectations derived from synthetic sizes + anchor sets. |
| tests | `crates/sigil-cli/tests/derived_layout.rs` (2) | error_handler island is the last emission on both sonic4 shapes, read off the resolve. |

### Two deviations from the note as written (both caught by the byte gate, both recorded in the ledger)

1. **§4 step 1 unconditional scratch is NOT byte-neutral.** A pure-data section's own
   length is position-independent, but code that references its labels is not: a boot-region
   label fits `abs.w`; at scratch it needs `abs.l`. First AFTER build: `span pass (declared):
   … player_sensors [0x5850, 0x5D44) overlap` (+0x18). Landed as the collision fallback
   flagged distorted; the pack rounds re-pin at real bases and converge there.
2. **"Islands = declared anchors" on the unlabeled arm needs the gap guard.** A label-less
   section's `prov` is its baked lma — `0` for every `.emp` section the frozen table does
   not name — and `0x0` IS the `boot_head` anchor. Without the guard `replay`, `raster`,
   `page_cache`, `ojz_effects_editor_act1`… pinned at 0 and the build failed at
   `dma_queue`/`s4lz`. Bisect: old island rule + new measuring = byte-identical (V1);
   probe printed zero non-anchor gap islands on the shipped walk, confirming the note's
   "inferred == declared" claim for LABELED sections.

## Reference tree

`/home/volence/sonic_hacks/.aeon-landing` = aeon `415e0b6ad62ca355102e4010811bce6aa1afd8f1`
(== `origin/master`, detached worktree, created this parcel; left in place). Hazards lived,
per the aeon lane: `tools/regenerate-level.sh` run first (only
`games/sonic4/data/generated/ojz/act1/DONOR_PROVENANCE.json` churned — discarded, not
committed); ALL FOUR ROM outputs `rm -f`'d before EACH build; `AEON_SKDISASM_DIR` exported.
No `sigil`/`skdisasm` symlinks exist in the main aeon tree; `tools/emp_helper_closure.py`'s
locator falls through to `<parent>/sigil` (the main checkout) for a non-`.worktrees` path —
no paired sigil worktree needed, none created.
**Bootstrap defect (aeon's):** a fresh checkout cannot pass `build.sh`'s pytest lane until
BOTH `s4.lst` and `s4.debug.lst` exist (ledgered). Bootstrapped with `./build.sh sonic4
--no-lint` and `DEBUG=1 ./build.sh sonic4 --no-lint`, listings kept, ROMs removed; every
number below is from a CANONICAL (all lanes) build after that.

## Byte identity — four shapes, CRC32/size (aeon 415e0b6a)

| Shape | provenance.toml tip | BEFORE (sigil 6030e4e6) | AFTER (this branch) |
|---|---|---|---|
| s4.bin | c7b9d10d / 699106 | c7b9d10d / 699106 | c7b9d10d / 699106 |
| s4.debug.bin | f0175028 / 715010 | f0175028 / 715010 | f0175028 / 715010 |
| demo.bin | c708b114 / 96336 | c708b114 / 96336 | c708b114 / 96336 |
| demo.debug.bin | dec88cc1 / 101044 | dec88cc1 / 101044 | dec88cc1 / 101044 |

The brief's expected s4 `060401e4` / s4.debug `0dbaa80f` were STALE (older provenance
entry); the coordinator corrected this mid-parcel. The tip was verified in
`crates/sigil-harness/golden/provenance.toml` directly.
Wall-clock per shape ≈ 52–73 s (`build.sh`, all lanes). Direct `sigil build` checks during
the bisect: V1 (old islands) c7b9d10d; V3 (final) c7b9d10d plain, f0175028 debug.

## Suite (`SIGIL_STRICT_GATE=1 AEON_DIR=.aeon-landing cargo test --release --workspace --no-fail-fast`)

Logs stamped with pwd/HEAD/branch/AEON_DIR SHA at the top:
`target/suite-BEFORE.log` (master worktree `8f8b03ee`, code == `6030e4e6`) and
`target/suite-AFTER.log` (this branch).

| Run | passed | failed | ignored | `test result:` lines | declared `#[test]` | `skip:` lines | wall clock |
|---|---|---|---|---|---|---|---|
| BEFORE (master `8f8b03ee`) | 3844 | 5 | 4 | 337 | 3853 | 0 | 21:35:44 → 21:37:26 (1 m 42 s) |
| AFTER (`4f303b0d`) | 3852 | 5 | 4 | 338 | 3861 | 0 | 21:37:26 → 21:39:45 (2 m 19 s) |

Reconciliation: declared − (passed + failed + ignored) = 3853 − 3853 = 0 before, 3861 − 3861 = 0
after; the +8 are exactly this parcel's 6 unit + 2 integration tests, all green. The five
failures are THE SAME FIVE by name before and after — `act_descriptor_region_matches_reference`,
`act_descriptor_debug_region_matches_reference`, `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma`,
`act_wrong_base_map_places_the_section_at_a_different_address`, `swapped_sec_fields_produce_different_bytes`
(the `CLOSURE-FIX` / `REPIN-END` / `FIVE-REG` set, out of scope). No test changed status in
either direction. (A first BEFORE attempt read 168 failures: the suite needs the four
reference ROMs present in `AEON_DIR`, and the freshness `rm -f` had left only the last-built
shape — rebuilt all four, each removed before its own build, then both suites ran.)

## Clippy

`cargo clippy --workspace --all-targets -- -D warnings` → exit 0 on the branch (one
`doc_lazy_continuation` hit during development, fixed before commit).

## Red-first evidence (invariant 8)

Unit tests: with three deliberate breaks applied at once — (a) the drift site restored to
`return Err(...)`, (b) the labeled island arm restored to `p > r + ANCHOR_GAP && (!fixture
|| is_anchor_gap(p))`, (c) the warning text reduced to `section {} drifted…` (no backticked
name, no `delta`) — `cargo test -p sigil-harness derived_layout_tests`:

```
test derived_layout_tests::drift_warning_names_section_and_delta ... FAILED
    missing "`sec_Tail`" in "[layout.provisional-drift] section sec_Tail drifted (`Tail`) 12320 4128 8192"
test derived_layout_tests::stale_provisional_gap_is_an_island_only_when_declared ... FAILED
    undeclared gap packs contiguously: left: Some(8192) right: Some(4112)
test derived_layout_tests::grown_pure_data_packs_downstream_and_warns_with_the_delta ... FAILED
    the walk must not refuse pure-data growth: packed base 0x3020 for section `sec_Tail`
    overruns its provisional 0x1020 by more than the drift tolerance — hand ruling needed
test derived_layout_tests::growth_into_a_declared_anchor_still_fails_loud ... FAILED
    overrunning a declared anchor must not build: [Some(4096), Some(4112), Some(8208)]
test result: FAILED. 2 passed; 4 failed
```
(`unchanged_sizes…` and `growth_within_tolerance…` stay green under the breaks, as they
should — they assert the fold identity the old code also had.) Restored with `git checkout`.

Integration gate (`cargo test --release -p sigil-cli --test derived_layout`,
`SIGIL_STRICT_GATE=1`, no `skip:` lines): green = 2 passed in 2.05 s. With the blob-end
arithmetic broken by `+ 2`:

```
test s4_error_handler_is_the_last_emission ... FAILED
    blob end 0xa11c2 != EndOfRom 0xa11c0
test s4_debug_error_handler_is_the_last_emission ... FAILED
    blob end 0xa3292 != EndOfRom 0xa3290
test result: FAILED. 0 passed; 2 failed
```
(the addresses are the resolve's own `EndOfRom`, which the provenance tip's `anchor_end`
0xa11c0 / 0xa3290 independently agrees with). Restored with `git checkout`.

## Open

- `SECTION-ROW` (§3.5) — follow-on S parcel; sigil specs, aeon lands the map row.
- `FIVE-REG` — untouched; the five known-red tests keep their status (see suite table).
- Option B step 3 (declared alignment quantum) + `seam2::frozen_prov` — the last
  `load_frozen_table` readers outside the walk.
- Unnamed-section `prov = 0` sentinel (ledgered; `None` is the right carrier).
- Aeon lane: first real-band build + refreeze under `--ab`; `bganim_room.py` retirement;
  the clean-checkout bootstrap defect in `build.sh`'s pytest lane.
- The stress fixture (`stress_art_profile`) now measures scratch-only on collision and
  treats it as distorted; it converges through the pack rounds like every other profile.
  Not rebuilt here (unfrozen, off-canonical); its next soak is the witness.

## Worktrees created

- `/home/volence/sonic_hacks/.aeon-landing` (aeon 415e0b6a, detached) — left in place.
- `/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-aaf5cff6181520afa-before` (sigil
  master `8f8b03ee`, detached) — used only for the BEFORE suite; safe to remove.
