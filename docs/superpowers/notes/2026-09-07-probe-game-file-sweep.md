# The probe/live-game-file sweep (2026-09-07)

Follow-through on `sig-probe-live-game-file`, closed at merge `87e8fe11`. That finding's last
sentence booked this work: *"Implies a sweep of every read_aeon site naming a game file rather than
an engine vocabulary file."* This note is that sweep.

Reference tree for every measurement here: `/home/volence/sonic_hacks/.aeon-ls12-fix`, aeon at
`ec640bcf`. Churn figures are `git log --since=2026-03-01` commit counts in that tree.

---

## THE CLASSIFICATION, AND WHY THE OBVIOUS ONE IS WRONG

The parcel was briefed with a three-way split: byte-identity port gates (correct), semantic/negative
probes (defective), and engine vocabulary files (a third class, assumed safer because "they move far
less"). **Both halves of that third class are wrong, and the axis is wrong.**

### `engine/` does not move less than `games/`. It moves more.

| aeon path | commits since 2026-03-01 |
|---|---|
| `engine/system/constants.emp` | **79** |
| `games/sonic4/config/constants.emp` | 37 |
| `engine/objects/collision.emp` | 24 |
| `games/sonic4/player/player_sensors.emp` | 18 |
| `engine/objects/sst.emp` | 16 |
| `engine/coords.emp` + `engine/objects/aabb.emp` | 11 |
| `engine/system/types.emp` | 10 |
| `games/demo/game_root.asm` | 4 |
| all of `engine/{system/types,objects/sst,system/constants}.emp` | 95 |
| all of `games/sonic4/objects/` | 64 |

The single most-churned file any sigil probe reads is an **engine** file. A directory name is not a
volatility measurement, and "engine vocabulary is stable" is a name-presence-behaviour claim.

### `engine/` is not one class either.

`engine/system/types.emp` and `engine/objects/sst.emp` are **vocabulary** — declarations, and they
are what probes prepend AS the ambient, so their growth is self-consistent. `engine/objects/
collision.emp`, `engine/objects/aabb.emp` and `engine/coords.emp` are **engine implementation** —
procs and templates, as live and as growable as any game object. A probe that compiles
`collision.emp` has exactly the defect the closed finding names; it is only in a different
directory.

### The axis that actually separates the correct from the defective

Not engine-vs-game. It is:

> **Does the assertion hold for ANY valid content of the file, or does it pin a SNAPSHOT of the
> content as it stands today?**

- **Content-invariant** — "every region's declared end contract holds against whatever the layout
  is", "the emitted stream round-trips", "sigil's bytes equal AS's bytes". Reading live game content
  is *the point*; the gate is correct and stays.
- **Content-snapshot** — "there are exactly 6 dispatch sites", "this literal line occurs once",
  "compiling this specific live file yields no diagnostic". These red when the other repo edits
  something the assertion was never about.

A byte-identity port gate is the extreme content-invariant case, which is why it was safe to name it
as its own class; it is not a separate principle.

---

## THE INSTRUMENT, AND THE FALSE ZERO IT PRODUCED FIRST

The population is every test binary under `crates/*/tests/*.rs` whose text calls any of
`aeon_dir(`, `reference_tree(`, `reference_tree_for_profile(`, `listing_path(`, `AEON_DIR`, or
reaches a `games/` path through a harness helper. That is **144 files** of the 399 test binaries in
the workspace; every one was read.

**The first run of that instrument returned zero, and it was wrong.** `git grep … -- 'crates/*/tests/'`
— a directory pathspec with a trailing slash — matches nothing and exits 0. The corrected pathspec
is `'crates/*/tests/*.rs'`. Recorded because this parcel's whole deliverable is an enumeration, and
a false zero here IS the deliverable being wrong. The canary that caught it: `git ls-files
'crates/*/tests/*.rs'` returned 399 with the same glob shape, so the glob was not the problem and
the trailing slash was.

Paths reach the aeon tree three ways, and a literal-string sweep finds only the first:

1. a literal `"games/…"` or `"engine/…"` joined onto `aeon_dir()`;
2. `reference_tree_for_profile(&profile)`, which resolves to `profile.game_root_rel`
   (`games/<g>/game_root.asm`, `native.rs:667`/`:717`) plus its sibling `map.toml` — **so every
   caller of it reads live game content**, and the subsequent build pulls the whole game tree;
3. a harness helper whose own body names the path — `seam1::check_banked_carrier_drift`,
   `seam2::banked_head_vmas`, `test_support::ACT_DESCRIPTOR_ASSERT_FILES`,
   `native::shape_defines`, `read_dac_declarations`.

## THE ENUMERATION

### CLASS 1 — byte-identity port gates over live game content. CORRECT, UNCHANGED.

Their job is to prove sigil reproduces what the engine's real content assembles to; the coupling to
live game files is the point, and a game-side change moves ROM bytes and is meant to be caught by
the refreeze ritual.

`test_p2_player_states_port` · `test_p4_player_sensors_port` · `ojz_run_a_port` · `objdef_port` ·
`mt_bank_port` · `collision_data_port` · `dac_bank_port` · `sfx_bank_port` · `header_port` ·
`test_mappings_port` · `particle_anims_port` · `scene_registry_port` · `m1c_vector_table` ·
`native_declared_chain` · `seam2_dac_emit` · `seam2_pitchtable` · `test_objects_port` ·
`test_g1/g2/g3/g4_objects_port` · `test_p1_player_port` (its four region tests) ·
`act_descriptor_port` · `ojz_run_b_port` · `soundbankhead_port` (its two gates) · `mt_port` ·
`sfx_port` · `sonic_anims_port` (its region tests).

`seam2_dac_emit` is the pattern the rest should copy: its expected lengths are **read back from the
declaring module** (`read_dac_declarations`), not typed, so regenerating the samples cannot red it.

### CLASS 2 — content-invariant gates that read live game content. CORRECT, UNCHANGED.

These assert a property true of ANY valid content, so aeon can edit freely underneath them. Reading
`games/` is not a defect here and this note exists partly so the next sweep does not re-litigate it.

`region_end_contracts` (every region's declared end contract holds against whatever the layout is) ·
`listing_phase_marker` · `error_handler_island_membership` · `hole_interior_reserved` ·
`section_alignment_declared` · `keystone_flip_relocation` · `derived_layout` ·
`measure_at_packed_base` · `native_object_bank_budget` (its cap is read from the live map, not
typed) · `corpus_builds` (the "every shape builds" half) · `dead_save_corpus` ·
`movem_restore_guard_corpus` · `game_debug_port` · `game_contract_env_coverage`.

**`game_contract_env_coverage` is the exemplar of the whole class** and worth copying: its
expectation is derived by parsing the interface, and it deliberately does not name the member it
removes — *"the member removed is not named here either: it is the FIRST one the contract declares"*.
No count, no name, nothing to rot.

**Correction to my own brief-reading:** `game_debug_port` was ranked fix-first by a reading pass on
the grounds that it compiles a live game file and asserts no bytes. It asserts that
`games/sonic4/debug/game_debug.emp` compiles, that its own link asserts pass, and that it emits
something — all true of any valid content. It is content-invariant and stays.

### CLASS 3 — content-SNAPSHOT assertions over live content. THE DEFECT CLASS.

Each pins a fact about the content as it stands today, so an aeon edit that changes no compiler
behaviour reds sigil's lane.

| site | what it pins | live path | disposition |
|---|---|---|---|
| `tranche7_negative_probes::broken_falls_into_stub_chain_fires_fallthrough` | that one literal line exists in the file, and that the whole file lowers clean under a hard-coded 5-file ambient | `engine/objects/collision.emp` | **FIXED HERE** |
| `cfg_blind_spots::corpus_computed_dispatch_census_is_six_sites_five_procs` | `assert_eq!(sites, 6)`, `assert_eq!(procs.len(), 5)`, and membership of `Player_SensorSurface` / `Player_SensorWallDir` — over a walk of `engine/` **and** `games/` | `games/sonic4/player/player_sensors.emp` (18 commits/6mo) | deliberate tripwire; ledgered |
| `out_verify_corpus` | `assert_eq!(slots.len(), 34)`, `SURVIVES_CLAIM_SITES` name list | walk spans `games/` | ledgered |
| `contract_closure_corpus` | `assert_eq!(r.context_claim_sites.len(), 11)`, named edge pairs | walk spans `games/` | ledgered |
| `parcel_8b_stage_gen_touchers` | a fixed 3-name toucher list | walk spans `games/` | ledgered |
| `preserves_corpus` | fixed proc/reg/status triples (all engine symbols today) | walk spans `games/` | ledgered |
| `warn_tier_corpus` | per-shape warn firing counts at named live game files | `games/sonic4/data/effects/ojz_effects.emp`, `games/sonic4/test/ojz_scroll_test.emp` | **intentional** — this gate's entire purpose is to notice that drift |
| `test_p1_player_port::p1_drift_guards_all_pass` | `assert_eq!(guards, 1)` over live `player_common.emp`, no byte oracle | `games/sonic4/player/player_common.emp` | deliberate ("counting it here is the point"); ledgered |
| `sonic_anims_port::rep_helper_compiles_and_repeats` | `src.contains("comptime fn rep(")` | `games/sonic4/data/animations/sonic_anims.emp` | subject genuinely is aeon's helper; ledgered |
| `dac_port` | `assert_eq!(blip.len(), 1)` over live DAC declarations | `games/sonic4/data/sound/dac_samples.emp` | ledgered |
| `seam2_colink_probe` | typed literal sample lengths `1406`, `2880` | `games/sonic4/data/sound/dac_samples.emp` | ledgered |
| `seam2_layout_derivation` | ten literal LMAs + literal map `order` text | `games/sonic4/map.toml` | intentional frozen-chain detector (pin ritual) |
| `corpus_builds`, `section_row_fixture` | the live map declares `ojz_effects_editor_act1` exactly once | `games/sonic4/map.toml` | ledgered |
| `listing_defines` | `__Aeon_AS_Carrier: equ 0` occurs exactly once | `games/demo/game_root.asm` (4 commits/6mo) | left alone — see below |
| `tranche4_negative_probes` | literal source needles (`"pub const AF_DELETE    = $FB"`) in live game data | `games/sonic4/data/animations/*.emp`, `act_descriptor.emp` | ledgered |
| `game_config_defines` | "the shipped maps declare no game defines today" | `games/{sonic4,demo}/map.toml` | intentional, remedy in the message |
| guard-count asserts inside byte gates: `mt_port:270/297` (7), `sfx_port:261/285` (1), `sonic_anims_port:234` (25), `test_g1:343-344`, `test_g2:411`, `test_g3:352`, `test_g4:484-486`, `test_objects:375`, `act_descriptor_port:402` | the number of `ensure`s the live game file carries | various | **low exposure, left alone** — see below |

**Why the guard counts are left alone.** They ride a byte-identity gate on the same file: a
game-side change that moves bytes reds the byte gate too and is handled by the same refreeze. The
residual exposure is narrow — a **zero-byte** `ensure` added game-side reds the count while the byte
gate stays green. Real but small, and paying to remove it would delete a genuine non-vacuity control
that proves the drift guards were captured by the lowering.

**Why `listing_defines` is left alone.** Its anchor is `__Aeon_AS_Carrier: equ 0`, deliberate
scaffolding whose own comment in aeon says it stays; the file has 4 commits in six months; and the
failure message says exactly what happened and names the file. It is the same shape as the fixed
defect but at a fraction of the exposure, with a legible red.

### CLASS 4 — the "engine vocabulary" reads. SAFE, BUT NOT FOR THE REASON GIVEN.

`core_negative_probes`, `dplc_negative_probes`, `hblank_negative_probes`, `tranche2/3/5`,
`tranche24_spelling_probes`, `structs_module`, `z80_clobbers_incomplete`, `p5_constants_flip`,
`diag_assert_vector`, `act_fixture_drift`, `seam1_native_link` and `tranche6`'s surviving ambient
all read `engine/` files.

They are safe **not because those files move less** — they move more — but because a probe that
prepends `types.emp` + `sst.emp` **as its ambient** grows with them: the ambient IS the subject's
vocabulary, so growth is self-consistent. The failure mode appears only when the ambient stops
closing over the subject, which is exactly what happened when the subject was a separate live file.

`engine/system/constants.emp` (79 commits) is the one with a real standing coupling, and it is a
**managed** one: `test_support::engine_constant_equs` is the single documented sync point with a
twin-growth procedure, and a drift there fails naming the constant.

### CLASS 5 — no aeon read at all.

`tranche22_spelling_probes`, `tranche23_spelling_probes`, `cpu_undeclared`, `here_relaxation_fix`,
`dac_bank_acceptance`, `reference_dependence_is_named`, `shipped_shapes`, `suite_paths_precedence`,
`skip_marker_lint`, `reference_env_read_is_routed`, `reference_tree_named_write`,
`reference_tree_write_guard`, `scripts_name_their_tree`, `shared_target_defaults`,
`golden_write_gate`, `landing_verdict`, `offcanon_assembled_bar`, `repin_gate_message`,
`rev_reachability`, `check_only_census`, `link_assert_reporting`, `source_gate_classification`,
`game_contract_build`, `resolve_manifest`, `encode_base_8bit`, `capstone_diff`.

Several of these carry `games/…` strings that a literal sweep counts as consumers and that are
**data**: `dac_bank_acceptance`'s header prose describing what its local fixture mirrors,
`resolve_manifest`'s tempdir writes, `repin_gate_message`'s expected message text. The tell is
position, and a name-string enumeration errs toward over-counting here.

---

## WHAT WAS FIXED, AND THE RED-FIRST EVIDENCE

`crates/sigil-cli/tests/tranche7_negative_probes.rs`, commit `898a97b1`.

**The defect, reproduced.** A copy of the reference tree with `Touch_Enemy` renamed to
`Touch_Badnik` throughout `engine/objects/collision.emp` — a refactor with no compiler-behaviour
meaning — red the probe on `assertion left != right failed: the doctor must have found its target`,
printing the whole 58 KB file twice into the failure. Mutation shown on disk before the run:

    227:proc Touch_None () clobbers() falls_into Touch_Badnik {}
    228:proc Touch_Badnik () clobbers() falls_into Touch_Boss {}

After the fix the same mutated tree passes 4/4.

**The reworked probe is not vacuous, both arms proven against a mutated compiler.** Baseline
committed first, mutation applied to `crates/sigil-frontend-emp/src/lower/proc.rs:1086` and quoted
back from disk each time, then restored with `git checkout --` from the committed baseline:

| mutation on disk | expected | observed |
|---|---|---|
| `if false && !ends_in_terminator(buf, cpu) {` (lint never fires) | doctored arm reds | FAILED: *"a stub that lost its falls_into must fire [proc.undeclared-fallthrough] naming it: []"* |
| `if true \|\| !ends_in_terminator(buf, cpu) {` (lint always fires) | control arm reds | FAILED: *"control must have no undeclared-fallthrough diagnostic: [… `Touch_Enemy` …]"* |
| restored | green | 4 passed |

The second mutation is the one that matters for this parcel: without it, a probe whose control had
quietly stopped discriminating would still print `ok`.

**Coverage moved, not lost.** The deleted control asserted aeon's real chain lowers with no
`[proc.undeclared-fallthrough]`. `warn_tier_corpus` pins that lint id's firings per shipped shape
over the whole corpus (`CORPUS_LINTS` includes `proc.undeclared-fallthrough`, and the walk covers
`engine/` and `games/`), so it was duplicated here.

**Second change in the same commit.** `sources()` read six live engine files and returned `None` if
any was absent, so the aabb probe — which uses one of them — skipped when a file it never touches
was missing, behind an `eprintln!` naming no path. Replaced with
`reference_tree(&["engine/objects/aabb.emp"])`: an absent file now skips NAMING the path and fails
under `SIGIL_STRICT_GATE=1`. That probe keeps reading the live template on purpose — its subject is
the `ensure(stmp != cdim)` the template carries.

---

## NO NEW GATE, AND WHY

The obvious structural net is "a reference-dependent test must name its paths through
`reference_tree(...)`". Measured before proposing it: **95** test binaries call `aeon_dir(`, **43**
call `reference_tree(`, and **87** call `aeon_dir(` without `reference_tree`. A gate on that
property fires on 87 correct files on day one. An always-red check is a delayed failure — it trains
people to weaken it — so it was not written.

The property that WOULD be worth gating, "a probe must not pin a snapshot of another repo's
content", is not decidable from source text: `assert_eq!(sites, 6)` and `assert_eq!(walked_debug_shapes, 3)`
are the same syntax, and only one is over foreign content. Left as prose here rather than as a lint
that would have to guess.

---

## FIXTURE ASKS FOR AEON — none this parcel

Every defect fixed here was fixable in sigil alone, by synthesising the input. Nothing in the
enumeration needs a frozen fixture aeon must carve: the remaining Class-3 rows are either deliberate
tripwires whose owner wants the red, or low-exposure couplings whose removal would cost a real
control.

Two aeon-owned fixture *sets* already exist and are working as intended — the objroutine probe
fixture (`games/sonic4/test/fixtures/sigil_objroutine_probe.emp`) and the poison fixtures under
`games/sonic4/test/poison/` that `build_check` and `extra_entry` drive. `extra_entry` even carries
its own `every_aeon_fixture_this_file_names_still_resolves` drift guard. If a future parcel does need
a fixture, that header is the template: it states who reads it, what the probes require of it derived
from their source, and that it must not grow.

(sections below filled in as each group's verification lands)

