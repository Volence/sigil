# CLOSURE-2 packet — the act rigs follow the descriptor's own `use` closure

Branch `fix/rig-closure` off master `72703755`. Sigil-only; no aeon tracked file,
no `native.rs`, no `pins.rs` / `repin.toml` / `provenance.toml` / `golden/` touched.

## The problem

`tranche4_negative_probes.rs` (`act_wrong_base_map_places_the_section_at_a_different_address`,
`swapped_sec_fields_produce_different_bytes`) and `act_descriptor_port.rs`
(`compile_real_file`) lowered `act_descriptor.emp` as ONE synthetic file: a
hand-listed ambient set (`engine.structs`, `engine.constants`, the two generated
const modules, `effects_scenes.emp`, a synthesised `SCENE_ACT_SPAN_Y` shim)
concatenated ahead of the descriptor's items. That list was a second, silent copy
of the descriptor's `use` closure. Aurora's first authored scene gave the generated
`effects_scenes.emp` a real body — `scene(...)` literals, `scene_budget_enforce`,
`fold_caps`, `lower5`, an `ensure` over `Game.SCANLINE_CAPS` — and the rig failed
on names the real build resolves without a word.

Red on master (`target/logs/red-before.log`, stamped `HEAD=72703755`, live aeon
`04147b43`), both files, identical text:

```
lower errors: [unknown function `scene_budget_enforce`, unknown function `fold_caps`,
               unknown name `Game.SCANLINE_CAPS`, unknown function `lower5`, ...]
```

## Which route, and why

Three were tried, in the order the brief asked.

1. **Plain `resolve::build_program` over a scanned manifest** (what `sigil build`
   in `main.rs` does) with a synthetic entry `use <descriptor> / use
   engine.game_contract / use games.sonic4.game` — the harvest-entry shape in
   `native.rs`. **Not viable on the aeon tree: 6012 errors** (`unknown function
   target_bits / op_bits / fire_ops ...`). Ambient injection is one `use` level
   deep, and the real build only lowers because `build_emp` first runs
   `publicize_helper_comptime` + `normalize_helper_imports` over its
   `COMPTIME_HELPERS` list. Those are private to `native.rs` and the list is gated
   for disjointness by aeon's `tools/emp_helper_closure.py`; copying it into
   `test_support` would be a second drift copy of exactly the kind this parcel
   removes, and `native.rs` is another agent's this hour.
2. **A longer hand list** (scene_dsl + parallax_dsl + a comptime-only slice of
   scene_registry + the contract env) — rejected without landing: it is the same
   rot with a later date, and `scene_registry.emp` emits bytes so it cannot ride
   ambient wholesale (the retired shim's own doc said so).
3. **The public whole-path seam: `native::build_emp(aeon, &profile)`** — the exact
   closure the ROM is built by (manifest scan, helper normalisation, game-contract
   bind, `build_program_open_embed`), then slice the `act_descriptor` section out.
   `scene_registry_port.rs` already gates this way. **Landed.** ~1 s per build.

### What landed (`crates/sigil-harness/src/test_support.rs` §5)

- `native_section(aeon, profile, section, assert_files)` — `build_emp`, exactly one
  section by name, and the program's link asserts filtered to the aeon-relative
  files named (the single-section oracle only links its own AS-side seam; other
  modules' `ensure(extern(...))` guards would be undecidable there). The filter
  reads `EmpProgram.sources.locate(span)`, the same location authority the
  warn-tier diagnostics render through.
- `ACT_DESCRIPTOR_ASSERT_FILES` — the descriptor plus the five files the old rig
  carried ambient, so `assert_guards`' bar is unchanged.
- `shadow_aeon_tree(aeon, overrides)` — the doctoring seam. The struct-swap probe
  compiles the real descriptor against a DOCTORED `engine.structs`, and
  `build_emp` reads a tree, so the probe gets a temp COPY of the tree with the
  doctored files written in place. What is copied is derived, never named: every
  top-level directory holding an `.emp` source, then every top-level directory
  those sources `embed("...")` from by a root-relative path (scanned out of the
  copied text); everything else is one symlink the build never opens. Copies and
  not symlinks because (a) `Manifest::scan` never descends a symlinked directory
  and (b) the lowering sandbox canonicalises every embed path and refuses one that
  resolves outside the root (`[sandbox.path-escape]` — 117 errors when the first
  cut symlinked). Nested checkouts are skipped with the scan's own two signatures.
  An override naming no copied file is an error. ~20 MB, removed on drop.
- The `SCENE_ACT_SPAN_Y` shim (`scene_act_span_y_const_src`) is deleted: its only
  callers were the two rigs, and the registry is in the closure now.

Both rigs derive every path from `aeon_dir()` / `aeon_root()` and the
`sonic4_profile` — no absolute path was added.

### One more cross-seam label, derived

With the closure live, the descriptor's Sec0 binds the generated
`EditorSceneBinding_OJZ_Act1_Sec0` record — a fixup the old rig never saw
(`unresolved symbol ... at offset 60`). It is the END label of the pinned
`SCENE_REGISTRY` region (`pins.rs:210`, `DeformTable_Zero ..
EditorSceneBinding_OJZ_Act1_Sec0`), so `as_seam_equs` supplies it as
`pins::SCENE_REGISTRY.{plain,debug}_base + {plain,debug}_len` — from the pin, not a
literal, and without touching `pins.rs`.

## Red-first evidence

- **Before** (`red-before.log`): the four tests red on the `unknown …` cluster above.
- **After** (`green-after.log`): tranche4 6/6 green; the `act_descriptor_port` pair
  now fails ONLY on `act_descriptor must emit exactly 0x274 bytes: left: 634 (0x27A)
  right: 636 (0x27C)` — the REPIN-END length assertion the brief excludes.
- **Re-break** (`rebreak.log`, temporary third override stripping
  `use engine.level.scene_dsl.*` from the copied `effects_scenes.emp`, restored
  via `git checkout` afterwards):
  ```
  native act_descriptor closure: build_program: 9 error(s);
    [Error] unknown function `scene_budget_enforce` @ SourceId(98) ...
    [Error] unknown function `fold_caps` ...
    [Error] unknown function `scene` ...
    [Error] unknown function `scene_hdr` / `scene_band` ×N @ SourceId(96) ...
  ```
  The failure names the missing helpers and the files that lost them.

## Full suite

Aeon worktree `/home/volence/sonic_hacks/.aeon-closure2` at `0e34408d` (the
provenance tip), regenerated with `tools/regenerate-level.sh`
(`DONOR_PROVENANCE.json` churn discarded, tree clean), built with this branch's
`target/release/sigil` + `emit_sound_blob`. All four canonical shapes reproduced
the provenance before the suite ran: s4 `875d591f`/699223, s4.debug
`a02d36db`/715114, demo `bf2cdb42`/96412, demo.debug `62a0019e`/101120
(`target/logs/aeon-build3.log`).

> Bootstrap note for a FRESH aeon worktree (not a sigil matter, recorded for the
> next lane): `build.sh`'s tool-suite pre-flight (`tools/test_bg_emit.py::
> TestBgAnimSectionCeiling`) requires `s4.lst` AND `s4.debug.lst` to exist and
> fails the build before it can produce them. `FAST=1 ./build.sh` and `FAST=1
> DEBUG=1 ./build.sh` (verification lanes skipped) mint the two listings; the four
> canonical builds then run clean. The `.aeon-landing` tree presumably carries
> listings from an earlier build, which is why the other lane never met this.

`SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-closure2 cargo test
--release --workspace --no-fail-fast` — log `target/logs/suite.log`, stamped with
pwd / HEAD / branch / aeon SHA:

- **3854 passed / 3 failed / 4 ignored** over 340 `test result:` lines = 3861, equal
  to the declared `#[test]` count (3861); **zero `skip:` lines**; wall clock
  22:54:49 → 22:57:18 (2 min 29 s). The three red: the `act_descriptor_port` pair
  (REPIN-END length) and `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma`
  (FIVE-REG). `cargo exit=101` is those three.

`cargo clippy --workspace --all-targets -- -D warnings`: exit 0 (`build-clippy.log`).

## Status changes vs the master bar (3852/5/4, 3861 declared)

- `tranche4_negative_probes::act_wrong_base_map_places_the_section_at_a_different_address` — FAILED → ok
- `tranche4_negative_probes::swapped_sec_fields_produce_different_bytes` — FAILED → ok
- `act_descriptor_port::{act_descriptor_region_matches_reference, act_descriptor_debug_region_matches_reference}` — FAILED → FAILED, on a DIFFERENT assertion (the closure error → the REPIN-END length, as the brief predicted)
- `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma` — FAILED → FAILED (FIVE-REG, untouched)

No ledger row: the whole-path rig landed, so there is no deferred hand-list to kill.

## BLOCKED

Nothing blocked. The aeon worktree was removed after the run.
