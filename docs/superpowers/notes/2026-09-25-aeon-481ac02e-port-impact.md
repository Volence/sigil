# Aeon 481ac02e (S2 clip act row 7): impact on sigil's aeon-reading tests

Measurement only. No sigil source changed. Sigil at `65f5337d` (branch
`measure/aeon-481ac02e`), tool built from this tree (`sigil 0.1.0 (65f5337d)`, closure
`e5b6ee06`), `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/aeon-481ac02e/target`.

## Verdict

**481ac02e changes no sigil test outcome.** The full workspace suite against aeon
`481ac02e` and against its first parent `b2820dc3` fails the identical 168 tests with
identical first messages (text diff empty apart from truncation). No log line in either
run mentions `clip_act` or `ojz_clip`. The hypothesis (a standalone `*_port` test missing
the new `games.sonic4.ojz_clip_act_act1` module) is refuted: there is no class (a) failure
attributable to this commit, so there is no fix to make.

Why: the only tests that lower `act_descriptor` (`act_descriptor_port`,
`tranche4_negative_probes`) do it through `test_support::native_section`, which runs a
whole-program `build_emp` and slices the section out, so a new `use` is resolved like any
whole-program build. The committed `clip_act.emp` is the neutral module
(`OJZ_CLIP_ACT = 0`, `ojz_clip_act_regions(hand)` returns `hand`), emitting no bytes or labels.

## ROM CRCs (IEEE CRC32 via zlib, built at 481ac02e with the tool above)

| shape | built | aeon reported |
|---|---|---|
| s4 | 6d1af7a3/821479 | 6d1af7a3/821479 |
| s4.debug | 62238a15/848075 | 62238a15/848075 |
| demo.debug | ce922bf7/104707 | ce922bf7/104707 |
| demo (plain, extra) | 2d7e5c18/98168 | not reported |

The parent `b2820dc3` builds the same s4 and s4.debug CRCs, consistent with aeon's
byte-identical claim. The provisioner builds only the two s4 shapes and copies the frozen
golden demo ROMs (`1c7a34d3/96863`, `72e405a5/103185`); the demo shapes were built
separately, measured, and the golden copies restored so the suite saw the provisioner's
default state. The suite runs therefore used the golden demo references in all three trees.

`repin --check` at 481ac02e: `pins.rs is STALE`, 442 pin values moved (expected: the tree
is past the pinned `ec640bcf`).

## Suite totals (`cargo test --release --workspace --no-fail-fast`)

Launched = `Running` + `Doc-tests` headers; reported = `test result` lines minus the two
nested results `eval_match` prints. All three runs wrote their end marker.

| AEON_DIR | aeon HEAD | launched | reported | passed | failed | ignored | failing binaries |
|---|---|---|---|---|---|---|---|
| `.aeon-481ac02e` | 481ac02e | 496 | 496 | 5472 | 168 | 2 | 68 |
| `.aeon-481ac02e-parent` | b2820dc3 | 496 | 496 | 5472 | 168 | 2 | 68 |
| `.aeon-sigil-ref` (control) | ec640bcf | 496 | 496 | 5639 | 1 | 2 | 1 |

The control's single failure, `m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`
(`NO REFERENCE TREE IS NAMED ...`), is environmental (no oracle-old reference named) and
fails in all three runs.

## Set difference against the pinned control (481ac02e minus ec640bcf): 167 tests

None is caused by 481ac02e (all 167 also fail at b2820dc3). They are drift accumulated
across aeon `ec640bcf..b2820dc3`:

- **141, class (b)**: byte or length moved because the tree is past the pin
  (`first diff at offset`, `length mismatch`, `must emit exactly`, `assembled length`,
  size tables that no longer re-derive, `pins.rs is STALE`, BootData divergence, the
  undoctored-control arms of the negative-probe files, `keystone_flip_relocation` index
  past a shorter ROM, and `sfx_bank_port` `PoisonError` secondary to an earlier panic in
  the same binary). Also `contract_closure_corpus` (the `with` bracket census moved),
  which is the same drift expressed as corpus adoption rather than bytes.
- **26, scope-shaped (class (a) in shape, but from EARLIER aeon commits, not 481ac02e)**:
  standalone ports whose hand-supplied scope lacks a name aeon added between the pin and
  b2820dc3:
  - `act_descriptor_port` (2): `unresolved symbol OJZ_Preset_Night` (and `_NightSnap`)
  - `bg_port` (2): `unknown name BG_VSCROLL_MAX_STEP` / `BG_VSCROLL_ROW_PX`
  - `bg_anim_port` (4): `BG_Bands_Hold` not defined in this link
  - `parallax_port` (2): `unknown name GAME_SCANLINE_CAPS`
  - `s4lz_port` (1), `load_art_port` (1), `tile_cache_port` (1): `unknown name CRASH_REPORT`
  - `dma_queue_port` (2), `dplc_port` (2): `MDDBG__ErrorHandler` / `extern("DMA_Queue_End")`
  - `plane_buffer_port` (3): `Plane_Buffer_Peak`, `Region_Resolve`
  - `section_port` (2), `entity_window_port` (2): `Region_Resolve`
  - `test_objects_port` (2): `[embed.not-found] .../games/sonic4/objects/games/sonic4/data/generated/spring/art_spring.bin`
    (a doubled path prefix, an embed resolved relative to the wrong directory)

  These belong to whoever next advances the pin; they are out of scope for this
  question and none was investigated further.

## Brief corrections

1. ec640bcf is not a control for 481ac02e: it is 442 pins behind, so its difference
   mixes every aeon commit since the pin. The first parent `b2820dc3` is the control that
   isolates the commit; it was provisioned at `/home/volence/sonic_hacks/.aeon-481ac02e-parent`.
2. The act_descriptor ports do not lower against a hand-picked module list; they slice a
   whole-program build, so a new `use` cannot fail there as hypothesised.
3. The provisioner does not build demo shapes by default (it copies golden demo ROMs),
   so the demo.debug CRC needed a separate build to measure.

Logs: `/home/volence/sonic_hacks/.scratch/aeon-481ac02e/` (`suite-{new,par,ctl}.log`,
`fails-*.txt`, `provision*.log`, `repin.log`).
