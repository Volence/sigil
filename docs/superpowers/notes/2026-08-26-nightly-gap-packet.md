# NIGHTLY-GAP packet — the lane builds the corpus, and counts what it skips (2026-08-26)

Branch `fix/nightly-gap` off master `a6fa4d67`. Sigil-only: `crates/sigil-cli/tests/corpus_builds.rs`
(new), `scripts/nightly_source_gates.sh`, `docs/OVERSEER.md`, this packet. Nothing under
`golden/`, `pins.rs`, `repin.toml`, `provenance.toml`; aeon untouched (the lane's own
`.aeon-sigil-gates` checkout was recreated by the script, which is what it does every night).

## 1. The facts first

**(a) The 2026-08-24 five, by class.** Reconstructed from `docs/OVERSEER.md` ("Full suite
bar", the CORRECTED 2026-08-26 paragraph) and the 2026-08-26 packets (`rig-closure`,
`section-row`, `repin-end`, `five-reg`). None of the five was a `sigil build` brick:

| Tests | Message | Class | Closed by |
|---|---|---|---|
| `tranche4_negative_probes` ×2, `act_descriptor_port` ×2 | `unknown function ojz_act1_act_default / ojz_act1_sec_scene` | **rig gap** — a single-file `lower_module` rig with a hand-listed ambient set; `sigil build` never saw it | CLOSURE-2 (`native_section` over `build_emp`) |
| `act_descriptor_port` pair, unmasked by the row above | `section.bytes.len() == pins::ACT_DESCRIPTOR.plain_len` (0x27A vs 0x27C) | **pin length** — the successor's pad entered the pin | REPIN-END |
| `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma` | `section ojz_effects_editor_act1 has no region in the map` | **rig gap** on the PinnedBaked/registry path, which `sigil build` does not take | FIVE-REG (re-homed) |

The one real BRICK of that window — `[map.order-undeclared]` naming
`EditorSceneBinding_OJZ_Act1_Sec0` when Aurora's first scene save made the block emit
(§3.5) — lived on aeon's **uncommitted** tree and was closed by the interim label row before
aeon committed (`0e34408d`), so the nightly, which checks out aeon *master*, never had it in
front of it. That is the exact shape the lane could not have reported even if it had looked:
the brick existed for hours in a tree the clock never reads. What the lane CAN own is the
committed case, and before this parcel it owned it only by accident (below).

**(b) Would any `SOURCE_GATES` entry have gone red on a corpus that no longer builds?**
Read, not assumed:

- `warn_tier_corpus` — builds all seven shapes via `native::build_emp` (lower + place; the
  `corpus_warnings()` cache). `build_emp` does NOT reach the map-driven link: `validate_placement`
  runs only inside `build_rom_chained_with_listing`, after `build_emp` returns. A
  `[map.order-undeclared]` / `[map.order-unknown-section]` brick is invisible to it. A lower-time
  brick (unknown function in a reached module) would panic it, under a warn-tier test's name.
- `native_object_bank_budget` — `resolve_canonical_sections` → `resolve_frozen_sections`, which
  packs but does not call `validate_placement`; sonic4 plain only.
- **`m68k_roundtrip_stream` and `m68k_capstone_stream`** (both `crates/sigil-harness/tests/`) —
  `native::build_rom_chained(&aeon, &profile)` over `native::shipped_shapes()`, every shape,
  `unwrap_or_else(|e| panic!("shape `{label}`: build failed: {e}"))`. **These two WOULD have gone
  red on a committed map brick** — at the first bricked shape, as a panic inside an instruction
  round-trip test, with no line in the lane's verdict saying a brick is what it was. So the
  premise "no gate asserts the corpus builds" is true as a *stated* assertion and false by
  construct; the gap this parcel closes is that the brick was measured incidentally, one shape
  deep, and reported as a roundtrip failure.

**(c) Found by the audit replay, not by the brief:** `derived_layout`
(`crates/sigil-cli/tests/derived_layout.rs`, master `4f303b0d`) matches the lane's selector
(`reference_tree_for_profile`), is source-only (`resolve_frozen_layout`), is not in
`SOURCE_GATES`, and names no artifact. `unclassified=1` — the lane would have exited 2 (the
whole backstop dark) at 2026-08-26 05:17. Added to the array.

## 2. The gate — `crates/sigil-cli/tests/corpus_builds.rs`

- **Shape enumeration:** `native::shipped_shapes()` — the table documented as "the one table a
  gate meaning all shipped shapes reads", the same one the CRC gates and the two m68k streams
  iterate. No copied list.
- **Build entry:** `native::build_rom_chained_with_listing(aeon, &profile)` — what `sigil build`
  reaches for every target (`main.rs::run_build_native`; the canonical sonic4 driver
  `build_native_rom_with_listing` delegates to it under `SizeSource::Frozen`). The deb2 appendix
  is not appended (an external tool over the finished image, not the compiler accepting the
  corpus) and the closure gate is not re-run (`contract_closure_corpus` is already in the lane).
- **Assertion:** `Ok`, zero `Level::Error` rows in `RomBuild::warnings`, non-empty image. Every
  shape is measured before any is judged; the failure lists every bricked shape.
- **No byte compared to any committed image** — legitimately source-only, green across a
  byte-moving aeon parcel.
- **Loud on unmeasurable:** `reference_tree_for_profile` per shape (strict → panic naming the
  absent path); `shipped_shapes()` empty → fail.
- **Audit classification by construction:** the file names no built ROM image, no listing
  extension, no reference-blob word; its header says why. Replay:
  `SOURCE_GATES=37 scanned=119 unclassified=0`, `corpus_builds → SOURCE_GATES`.
- **Runners:** workspace `cargo test` (auto-discovered under `crates/sigil-cli/tests/`) and
  the nightly's `SOURCE_GATES` (`--test corpus_builds`).

## 3. Red-first (invariant 8)

Doctored COPY (`cp -a` of `.aeon-landing@058ad606` under the worktree's `target/aeon-brick`,
ROMs/listings/generated removed, so it is source-only like the lane's checkout), one brick
injected: the `"section:ojz_effects_editor_act1"` row deleted from `games/sonic4/map.toml`.

**Red** (`target/logs/corpus_builds-RED.log`, 5s):

```
corpus builds: 2 of 7 shipped shapes build from source at …/target/aeon-brick
5 of 7 shipped shapes do NOT build from aeon source (a BRICK — the compiler refuses the corpus; no refreeze clears this):
  shape `sonic4 plain`: [map.order-undeclared] byte-emitting section `EditorSceneBinding_OJZ_Act1_Sec0` is not in the declared `order` — the map DRIVES placement now, so every emitter must be declared; add it in its layout position
  shape `sonic4 debug`: [map.order-undeclared] … `EditorSceneBinding_OJZ_Act1_Sec0` …
  shape `config_a`: …   shape `config_b`: …   shape `lean`: …
test every_shipped_shape_builds_from_source ... FAILED
```

(The two demo shapes stay green — they read their own map. Five red, not one: every shape
measured.) **Restored** (`map.toml` copied back, `diff` clean; `corpus_builds-GREEN.log`):
`corpus builds: 7 of 7 shipped shapes build from source … test … ok`.

The same brick is a permanent self-witness inside the binary:
`a_deleted_map_row_bricks_the_build_and_the_gate_names_it` doctors a `shadow_aeon_tree` copy
(copy, not symlink — `Manifest::scan` skips symlinked dirs; the embed sandbox refuses paths
outside the root), proves the undoctored copy builds, then requires `[map.order-undeclared]`
and the shape's label in the report. Output on the landing tree:
`shape `sonic4 plain`: [map.order-undeclared] byte-emitting section `EditorSceneBinding_OJZ_Act1_Sec0` …`.

## 4. The lane — `scripts/nightly_source_gates.sh`

- `corpus_builds` heads `SOURCE_GATES` as the brick witness; `derived_layout` added (§1c).
- The audit now COUNTS the artifact-lane files it skips (`artifact=()`), and the verdict line
  prints it. Green line, as written to `nightly.log` by the hand-run:

```
2026-08-25T23:35:58-04:00 OK at sigil a886fd2b / aeon a840d68f (162 passed, 37 gates; 82 aeon-reading gates skipped as artifact-lane (CRC/region oracles against committed artifacts, not measured here; build bricks witnessed by corpus_builds))
    open warn-tier findings: 2
      import.no-names · ojz_effects.emp · games.sonic4.scene_registry · owner: sigil language lane (the spelling); aeon adopts it · open 8 days
      import.no-names · ojz_scroll_test.emp · games.sonic4.scene_equiv_proof · owner: sigil language lane (the spelling); aeon adopts it · open 8 days
```

- A red run whose output carries the gate's phrase (`shipped shapes do NOT build from aeon
  source`) is announced as `BUILD BRICK (the corpus does not build from source); SOURCE GATES
  FAILED …`, with the same skipped-count clause.
- **Exit contract unchanged**: 0 / 1 (a gate failed) / 2 (could not run). The consumer
  (`~/.config/systemd/user/sigil-source-gates.service`, `Type=oneshot`, no env, no args;
  `ExecStopPost` notifies on any non-zero `$EXIT_STATUS`) reads only the exit status; the
  script's own outputs stay `$XDG_STATE_HOME/sigil-source-gates/{nightly.log,gates.log,prepare.log}`.
- The EXCLUDED comment no longer claims the lane "would go red on aeon build breakage rather
  than on the thing it watches": it now says it builds every shape on purpose.

**Hand-run, as the timer would** (no args; the ONE env the script documents for this purpose,
`SIGIL_SOURCE_GATES_REF=fix/nightly-gap`, because the timer's `ExecStart` is the main checkout's
script at master and the branch is what is under test): `rc=0`, **wall 72s** including a
from-scratch release build of the 37 gate binaries in `.sigil-source-gates-target`; 37
`test result:` lines, 162 passed, 0 `skip:`. Both lane checkouts (`.sigil-source-gates`,
`.aeon-sigil-gates`) were ABSENT before the run (removed some time after today's 05:17 run;
the timer log shows that run green) and were recreated by the script exactly as the timer
would recreate them; `.aeon-sigil-gates` holds no `*.bin` after the run. Aeon master at run
time was `a840d68f` (ahead of the landing tree's `058ad606`).

## 5. Full suite

`SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-landing cargo test --release
--workspace --no-fail-fast`, `CARGO_TARGET_DIR` under the worktree, log
`target/logs/suite.log` stamped `pwd=<worktree> head=a886fd2b branch=fix/nightly-gap`,
no commit between build and run. Aeon `.aeon-landing@058ad606`, clean, not built or modified
by this parcel (the build's `ensure_generated` writes `engine/sound/generated`, as every
profile build does).

- 340 `test result:` lines aggregated: **3872 passed / 0 failed / 4 ignored**, wall **137s**,
  exit 0, **zero `skip:` lines**.
- Declared: `git grep -c '#\[test\]' HEAD -- '*.rs'` summed = **3876**; 3872 + 4 = 3876. ✔
- Bar on master: 3870/0/4 (3874 declared). Delta: +2, both this parcel's
  (`every_shipped_shape_builds_from_source`,
  `a_deleted_map_row_bricks_the_build_and_the_gate_names_it`); `grep -c corpus_builds
  suite.log` ≥ 1.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.

## 6. Open

- The nightly reads aeon *master*. A brick that lives only in the owner's live tree (the §3.5
  case) is not this lane's to see; aeon's own ritual is.
- `m68k_roundtrip_stream` / `m68k_capstone_stream` still build the corpus themselves — a brick
  now fails three binaries. Fine (they need the bytes), noted so nobody de-duplicates the
  witness away.
- `derived_layout` was one audit replay from taking the lane down. The OVERSEER rule
  ("replay the audit against any branch adding a `crates/*/tests/*.rs`") is restated with
  this as the precedent; a `cargo test` that replays the audit would make it structural
  (ledger candidate).
