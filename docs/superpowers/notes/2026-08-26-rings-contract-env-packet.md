# RINGS-ENV — the isolated port oracles bind aeon's own `Game` contract

Branch `fix/rings-contract-env`, off master `c3159b46`.

## 1. The failure, reproduced firsthand

Measured on master before any change, full workspace suite under
`SIGIL_STRICT_GATE=1`, `AEON_DIR=/home/volence/sonic_hacks/.aeon-rings` (a detached
worktree at aeon `6286377e`, four shapes built there and CRC-matched to the
`provenance.toml` tip before the run):

```
crates/sigil-cli/tests/rings_port.rs:294:5:
rings.emp lower errors: [Diagnostic { level: Error,
  message: "[contract.unknown-member] `Game` has no hook member `ring_collected`
            (or the interface is unimplemented in this build)",
  primary: Span { source: SourceId(0), start: 14175, end: 14202 } }]
```

Both `rings_region_matches_reference` and `rings_debug_region_matches_reference`
panicked with it.

**How `rings_port.rs` obtained its `Game` env: it did not.** `compile_real_file_with`
called `sigil_frontend_emp::lower::lower_module`, and `lower_module`
(`crates/sigil-frontend-emp/src/lower/mod.rs:142`) forwards
`&crate::contract::InterfaceEnv::empty()`. So the single
`invoke Game.ring_collected` that aeon's ring-sparkle parcel added at
`engine/objects/rings.emp:336` had no interface to resolve against. The engine
half is `engine/system/game_contract.emp` (`hook ring_collected (a3: *u8)
clobbers(d0-d2/a1) = empty`), bound by `games/sonic4/config/game.emp`
(`hook ring_collected = RingSparkle_Spawn`).

The sibling oracles do pass an env, but each hand-writes its own `.emp` interface
as a Rust string literal — a second, silent copy of the contract, which is the
CLOSURE-2 class one layer up.

## 2. The derived helper

`crates/sigil-harness/src/test_support.rs` §1c:

- `GAME_CONTRACT_IFACE_REL = "engine/system/game_contract.emp"` — the engine half
  is not per-game, so it has nothing to derive from a profile.
- `game_manifest_path(aeon, profile)` — `GameProfile::game_root_rel`'s directory +
  `config/game.emp`. Exactly the derivation `GameProfile::map_path` and
  `reference_tree_for_profile` (`test_support.rs:381`) use for the placement map,
  one file over. Nothing hard-coded.
- `game_contract_env_from_aeon(aeon, profile, defines)` — parses both files, binds
  them with `bind_with_ambient`, and returns the env.
  - The manifest's own `use` edges are followed to their files and supplied as the
    bind ambient, so a binding value naming an imported const
    (`const ENTRY_ID = GS_OJZ_SCROLL_TEST`) folds; the game's own `config/`
    directory is swept as well, because `games.sonic4.constants` lives at
    `games/sonic4/config/constants.emp` and does not sit at its dotted path.
  - `defines` is the profile's own `emp_defines`, so the manifest's
    comptime-`if` binding group (`SOUND_DEBUG_HOTKEYS == 1 && …`) resolves the way
    the real build resolves it.
- LOUDNESS. Panics naming the path on: a missing interface file, a missing
  manifest, a parse error in either, any bind error, a manifest whose module id is
  not the profile's declared `manifest_module`, no `Game` interface in the env, and
  an env carrying fewer members than the interface declares.
- SCOPE, stated in the source: this binds MEMBERS, not bodies. The bound procs live
  in modules outside the two-file bind set, so the §4 subcontract check silently
  passes here; that check is the whole-program build's job and
  `contract_closure_corpus.rs` gates it.

Two supporting derivations, both used by `rings_port.rs`:

- `game_contract_bound_symbols(env)` — every bound hook/proc target name, read off
  the env. A newly bound hook joins the pin list by construction.
- `listing_symbol_addr(listing, name)` — one symbol's address out of the reference
  ROM's own sibling listing (same build, so the operand this oracle encodes and the
  one in the reference cannot disagree). `None` when the listing is absent; a
  present listing missing the name is a hard error naming it.

`rings_port.rs` now lowers with `lower_module_with_contracts` and that env, and
pins the contract's bound targets from the listing. The outbound-consumer lookup
moved from LMA arithmetic (`1 + labels.len()` groups) to "the highest
harness-private LMA", since the contract's pins now vary that count.

Both rings gates are byte-green against the reference — which is the strongest
statement available that the derived binding equals the one the ROM was built
with: the `jsr RingSparkle_Spawn` operand in the re-lowered region matches the
reference ROM byte for byte.

## 3. The six call sites, classified

`game_contract_env` (the hand-written-source helper) had five callers plus one
in-harness wrapper. The brief also named `contract_closure_corpus.rs:1400`; that
site does not call `game_contract_env` — it is a synthetic `interface H` with a
`hook tick`, listed below for completeness.

| # | Site | Class | Reason | Action |
|---|------|-------|--------|--------|
| 1 | `rings_port.rs:294` (no env at all) | MEANS THE REAL CONTRACT | byte-gates `rings.emp` against sonic4's ROM; the ROM's `invoke` was lowered under sonic4's binding | MIGRATED |
| 2 | `game_loop_port.rs:76` | MEANS THE REAL CONTRACT | reproduced "hotkeys off → `debug_tick` unbound", which is sonic4's canonical shape; byte-gated against that ROM | MIGRATED (canonical profile) |
| 3 | `boot_port.rs:304` | MEANS THE REAL CONTRACT | hand-copied `ENTRY_ID = 3` and `entry = GameState_OJZScroll_Init` out of sonic4's manifest — a copied expectation that would gate a stale value the day sonic4 re-points its entry | MIGRATED (per-shape profile) |
| 4 | `game_debug_port.rs:251` | MEANS THE REAL CONTRACT | restated the Config-A binding `debug_tick = Debug_MusicToggle`, which config_a's own defines produce from the real manifest | MIGRATED (`config_a_profile`) |
| 5 | `test_support.rs::scanline_caps_contract_env` | MEANS THE REAL CONTRACT | one-member stub for four byte-gated oracles (raster / parallax / buffers / raster probes); already read the mask from `game.emp`, but a second member any of them started reading would abort the lower | MIGRATED (whole contract, canonical) |
| 6 | `camera_port.rs:52` | BOTH — split | the reference gates passed a hardcoded `jump_lock = 1` (a copied expectation); `jump_lock_off_compiles_without_game_symbols` passes `0`, the `false` arm NO shipped game declares | SPLIT: gates bind sonic4's contract, the probe keeps its synthetic source |
| 7 | `tranche5_negative_probes.rs:67` | SYNTHETIC BY INTENT | sweeps a combo matrix of `SOUND_DEBUG_HOTKEYS`/`SOUND_DRIVER_ENABLED` to prove the BINDER's comptime-group behaviour; it is testing the bind pass, not the corpus, and the real manifest spans only the shapes the games ship | KEEP |
| 8 | `contract_closure_corpus.rs:1400` | SYNTHETIC BY INTENT | `a_synthetic_invoke_charges_the_bound_hooks_clobbers` declares its own `interface H` with a deliberately over-tight invoker so the transitive-clobber diagnostic must fire; a real contract cannot express the violation being probed | KEEP |

Nothing was weakened to reach green: the one case where a straight migration would
have cost coverage (site 6) was split rather than migrated, and both arms survive.

`sonic4_scanline_caps` retired with its only caller — the mask now arrives by
binding sonic4's `implement Game` instead of re-spelling one line out of it.

## 4. The invariant-8 gate, red-first

`crates/sigil-cli/tests/game_contract_env_coverage.rs`, three tests:

- `derived_env_covers_every_declared_member` — for sonic4 and demo in both shapes,
  the derived env carries every member `interface Game` declares. The expectation
  is `game_contract_declared_members(aeon)`, parsed out of `game_contract.emp`;
  no count and no member name is written in Rust.
- `a_member_filtered_out_of_the_env_is_reported_by_name` — a STANDING non-vacuity
  probe: for each declared member in turn, filter it out of the env and require the
  coverage predicate to report exactly that name. Without it, a predicate that
  always answered "nothing missing" would read like the gate passing.
- `every_shipped_manifest_is_where_the_profile_says` — each profile's manifest is
  at the path its own `game_root_rel` derives, and declares the module its
  `manifest_module` names.

RED-FIRST EVIDENCE. Sabotage: one line in `game_contract_env_from_aeon` dropping
`ring_collected` from the bound members after the bind.

```
---- derived_env_covers_every_declared_member stdout ----
the derived env is missing 1 of the 7 members `interface Game` declares in
.../engine/system/game_contract.emp: ["ring_collected"] — an env that under-covers
the contract lets an engine module's `Game.MEMBER` fail at lower instead of here
```

Both the coverage gate and the non-vacuity probe went red naming the member; the
"7" is the parsed member count, not a literal. Sabotage reverted; all three green.

WIRING. The file is source-only (it reads `game_contract.emp` and each game's
`game.emp`; nothing built, nothing compared to a committed artifact), so it joins
`SOURCE_GATES` in `scripts/nightly_source_gates.sh`. The script's own audit rule
classifies a test file as artifact-lane when its text names `s4.bin` / `.lst` /
`golden`; a local replay of that audit reports 38 source gates, 82 artifact-lane
files, ZERO unclassified. `rings_port.rs` stays artifact-lane by that rule (it
names both the ROM and, now, the listing) — the script's rule was followed, not
forced.

## 5. Deferred

| Item | Why deferred |
|---|---|
| `tranche5_negative_probes.rs` keeps a hand-written `Game` interface | Deliberate: it probes the binder over a define matrix the shipped manifests do not span. If the engine ever adds a member `game_loop.emp` reads, this file's own stub goes stale and its lower fails loud — the same loud failure `rings_port` gave, not a silent under-cover. No ledger action needed unless that stub grows a second member. |
| `camera_contract_env` keeps its synthetic source | Same: the `false` arm is a probe input. Kill condition — if a shipped game ever declares `CAMERA_JUMP_LOCK = false`, the probe should bind that game's contract instead of synthesising one. |
| The derived env does not run the §4 subcontract check | Structural: the bound bodies are outside the two-file bind set. The whole-program build and `contract_closure_corpus.rs` own that check; documented in the helper's own header so nobody reads the derived env as a full contract verification. |
