# 2026-09-04 — the ROM re-layout refreeze, and the ratchet that blocked it

Aeon moved two ROM bank anchors to buy data headroom. **A `[[anchor]]` places nothing on
its own**, so the re-layout could not take effect until sigil's frozen tables moved. They
have. Chain 202 is frozen against aeon `5875e60e`.

Getting there required fixing a gate that refused correct code, and that fix is the more
durable half of this parcel.

## Identity

| | |
|---|---|
| aeon revision frozen against | `5875e60e5c5213b45b9e24059cd337a2ac22f394` (branch `parcel/rom-relayout-more-room`) |
| durability | `git ls-remote origin` at measurement time — tip of `refs/heads/parcel/rom-relayout-more-room`. NOT on `origin/master` |
| sigil base | master `48313276` |
| assembler | `sigil 0.1.0 (27489944)`, built into `/home/volence/sonic_hacks/.sigil-refreeze-target` |
| reference tree | `/home/volence/sonic_hacks/.aeon-relayout-freeze`, clean detached worktree at `5875e60e` |
| attest tree | `/home/volence/sonic_hacks/.aeon-attest-201`, clean detached worktree at the pinned `4f5ad5a1` |
| shared `target/release/sigil` | md5 `6c2378ae8a657e26684d4019a7d976d7`, unchanged throughout |

## The mechanism, confirmed at the build rather than taken on report

`native::load_frozen_table` reads `golden/offcanonical_sizes/<shape>.txt` out of the
sigil checkout **at run time**, `true_bases_by_index` builds every ROM section's
provisional base from those rows, and `map.toml`'s anchors reach the packing walk only as
a `HashSet<u32>` of addresses — matched by address, never by name — that *authorize* a
section to sit where the table already put it.

Building aeon `5875e60e` against the UNMOVED tables:

```
error: native build (sonic4 plain): [map.undeclared-island] ROM section at 0x90000
is an ANCHOR_GAP-inferred island but no `[[anchor]] at = 0x90000` is declared
```

The DAC bank had not moved. After the two rows moved, all four canonical shapes build and
`s4.lst` reads `Dac_Temp_Blip: A8000`.

## What moved by hand, and what was derived

`Dac_Temp_Blip 0x90000 -> 0xA8000` and `SoundTablesZ80_Head 0xA0000 -> 0xB8000`, in the
four SOUND-ON tables (`s4`, `s4_debug`, `config_a`, `lean`) — eight rows. `config_b`,
`demo` and `demo_debug` carry neither row; that was grepped, not assumed. Everything
downstream was re-packed by `derive_offcanonical_sizes.sh`, per the 08-26 precedent.

## THE CONTROL — the relayout's own share is exactly +0x18000

The raw before/after is CONFOUNDED. The committed goldens were frozen at aeon
`4f5ad5a1`, and `5875e60e` is **195 commits past it** (191 on aeon master, 4 on the
branch). A naive diff therefore mixes the re-layout with two days of engine content, and
it moves `config_b` — which the re-layout cannot touch.

The separating measurement: derive the tables TWICE at the SAME aeon revision, once with
the anchors and island rows reverted to `0x90000`/`0xA0000` in a second worktree.
Identical content; layout the only variable.

| shape | per-label delta, control -> new |
|---|---|
| `s4` | `+0x18000` ×8, zero ×60 |
| `s4_debug` | `+0x18000` ×9, zero ×71 |
| `config_a` | `+0x18000` ×9, zero ×77 |
| `lean` | `+0x18000` ×8, zero ×60 |
| `config_b` | zero ×68 — **nothing moved** |
| `demo` / `demo_debug` | zero ×40 / zero ×42 |

**Not one symbol moved by anything other than `+0x18000` or zero.** The `+0x18330` a
naive diff shows on the two DEBUG shapes (`Replay_OJZ_Fixture`, `BusError`, `EndOfRom`)
is `0x330` = 816 B accruing between `GameState_OJZScroll_Init` and `Replay_OJZ_Fixture`,
in the DEBUG-only region; aeon's diff carries a matching `ojz_scroll_test.emp` growth.

## The seven shapes

| shape | before (crc/size) | after (crc/size) | EndOfRom before -> after |
|---|---|---|---|
| `s4` | `14ee2440`/719700 | `6f047af2`/819123 | `0xa5c82` -> `0xbdc82` |
| `s4.debug` | `142294b3`/737683 | `d772f7d8`/840179 | `0xa81fc` -> `0xc052c` |
| `config_a` | `b9574a32`/738015 | `f598841a`/840531 | `0xa81fc` -> `0xc052c` |
| `config_b` | `46dc1eda`/615905 | `07002ea1`/618293 | `0x8c9ac` -> `0x8cea2` |
| `lean` | `6678ba60`/674816 | `7c8ec3e0`/773120 | `0xa4c00` -> `0xbcc00` |
| `demo` | `0c456778`/96474 | `3c5dcde6`/96602 | `0x1121a` unchanged |
| `demo.debug` | `2e603d53`/101339 | `36014485`/102818 | `0x1121a` unchanged |

The capture ran twice, hours apart and across a source change to `refreeze` itself, and
produced bit-identical CRCs for all seven.

**For the headroom rule:** `s4` `EndOfRom` = `0xBDC82` = **777,346**; `s4.debug`
`EndOfRom` = `0xC052C` = **787,756**. Both far under `0x100000` (1,048,576). If the rule
means the whole cartridge image rather than the assembled anchor, the full-file sizes are
819,123 and 840,179 — also under. Note that the hub's predicted 835,987 for the debug
shape matches neither figure; it is nearest the full file size, 4,192 bytes low.

## THE RATCHET — a check that refused correct code

`--freeze` refused the ledger append: tip #201 `corpus-pin-advance` carried no strict run.
So chain 201 was attested first, from a `provision-aeon-ref.sh` tree whose rebuild control
printed `MATCHES THE GOLDEN` for both s4 shapes and whose `repin --check` printed
`pins.rs unchanged` — the named positive witness. **That suite was green.**

`--attest` refused anyway: *"strict_bodies FELL from 30 to 29 since the last recorded
strict run ... Restore the gate, or say why it is gone: `--retired-strict-gates`."*

**No gate had been retired.**

* The baseline was `chain.entry.iter().rev().find_map(|e| e.strict.as_ref())` — the last
  RECORDED run, **with no filter on outcome**, fourteen lines above the code that computes
  `OUTCOME_FAILED` from `run.failed > 0`. The concept was already in the same function.
* That entry is #200 `tails-jump-gate`, `outcome = "failed"`, 12 failing. It is the ONLY
  one of the chain's 28 recorded strict runs to record 30; **#173 through #199 all record
  29**, across both outcomes.
* The declared strict-gate site population is **byte-identical** between #200's own sigil
  rev `64bc7158` and HEAD: 38 `if !strict_gate()` consultations across the same 12 files.
* The population census — the detector this module exists to be, built because "a gate
  going dark showed up as a SMALLER GREEN" — was green.

Because the chain is append-only, that 30 was a permanent floor no honest later run could
clear. The cheapest exit was a permanent ledger field asserting a retirement that did not
happen: **the damage was written into the remediation advice, not the check.** A rule
whose remedy is a false statement is worse than no rule.

**The fix** (`ratchet_baseline`) takes the last run whose `outcome` is `passed`, and stays
dormant while none has — the rule may not arm off a red run at all. Max-over-all-entries
was rejected: it would anchor to the same anomalous 30 permanently, the identical bug with
a monotonic face.

Red-first, with the mutation shown applied on disk (`git diff --stat` naming the file) and
restored from a committed baseline. Mutated back to unfiltered, the two new tests failed on
`left: Some(30), right: Some(29)` — the real anomaly — while
`the_strict_body_ratchet_fails_on_a_shrink_and_has_a_named_exit` still passed, showing the
mutation was targeted rather than a blanket break. The chain-fixture test asserts the
anomaly is STILL PRESENT before asserting the baseline skips it, so it fails loudly instead
of going vacuous if the fixture ever moves; it is bounded to the immutable prefix through
`tails-jump-gate`, so it cannot rot as the chain grows.

On the live chain: `refreeze --attest: strict-body ratchet: 29 -> 29, held.`

## The red attestation that was discarded, and why

The first `--attest` after the ratchet fix came back **RED on exactly one test**:
`harness_root::root_derivation::the_compile_time_manifest_dir_is_only_ever_displayed`
(4376 passed / 1 failed / 2 ignored / 0 `skip:` across 379 suites). The cause was mine —
the new chain-fixture test read `provenance.toml` through `env!("CARGO_MANIFEST_DIR")`,
and that gate forbids `refreeze.rs` and `repin.rs` from carrying any compile-time path,
the macro's absence being the whole proof. The gate was right and caught it.

That record was **uncommitted** and was discarded rather than filed. The reasoning, stated
so it can be disagreed with: entry #201's strict record answers "does the strict suite pass
on the goldens this tip names", and a defect in the *attesting* tree is not an answer to
that question. Filing it would have implied the tip's ROMs were suspect and forced a
spurious `--supersede-tip` abandonment of another lane's entry. The test was fixed to go
through `resolve_harness_root`, and the re-run recorded `passed`. This paragraph is the
record the ledger does not carry.

The distinction being relied on: discarding an *uncommitted* tool output after fixing a
defect in the runner is not the same act as editing a committed entry, which is the
forgery the provenance docs warn against. Nothing committed was altered.

## The hand-typed baselines the freeze moved

Three independent baselines countersign the freeze, each deliberately unpinned or
hand-typed so it cannot be derived from what it checks. All were re-derived with reasons
(commit `56956d26`). Split by cause:

**The relayout's own** — `seam2_layout_derivation`'s ten `SoundLayout` literals, all
`+0x18000` with every in-bank offset unchanged; `ASSEMBLED_LEN` `+0x18000` exactly; and
`DEBUG_ASSEMBLED_LEN` `+0x18330`, the extra `0x330` being the DEBUG-only
`ojz_scroll_test.emp` growth that lands after the banks.

**The content jump's** — five `debug_base` pins at a flat `+0x32`, traced to `dma_queue`
`+0xE` and `vblank` `+0x24` (aeon's DMA straddle counter), a pure downstream slide with no
length change and no plain-shape movement; `SCENE_REGISTRY` `+0x328` in both shapes; and
`Ground_Move_Cap`'s plain offset `0x2F4 -> 0x2F8`, because plain's `Sound_PlaySFX` crossed
`$8000` and its two call sites widened to `jsr abs.l` — corroborated independently by
`SOUND_API.plain_base 0x7A9E -> 0x8082`.

`secondary_pin_classes_match_the_hand_typed_baseline` was left byte-identical. It is
`#[ignore]`d and its own reason says the body "is preserved for archaeology"; four edits
initially landed inside it and were reverted. Its literals have rotted far out of date
(`MDDBG_ERROR_HANDLER` reads `0x5E8F2` against a live `0xBF5D6`), which is the retirement
working as declared rather than a defect to repair.

## A PROPERTY OF ANY LARGE REFERENCE JUMP, not a fact about this parcel

**A freeze that moves the reference forward by many commits inherits that many commits'
worth of port-gate catch-up, by construction.** It is worth stating as a rule, because the
next person to move a reference this far will otherwise read a red suite as their own
defect.

The mechanism is not subtle once named. A port gate lowers ONE module standalone, so every
symbol that module reaches across a seam must be supplied from outside. Ordinary practice
freezes often, so each freeze brings one or two new cross-seam refs and the repair is
invisible. This freeze spanned 195 aeon commits and brought them all at once: the suite
opened at **24 failures**, and only three of them were about the relayout.

They also arrive ONE AT A TIME. The standalone link reports the first unresolved symbol and
stops, so each fix uncovers the next — `Dbg_DMA_Straddle_All`, then `_Frame`, then `_Peak`,
then `DMA_Split_Reject_Count`. Reading that as bad luck is the trap; it is the same finding
repeating, and the answer is to stop resolving symbols and start resolving FAMILIES.

## What closed the 24, in three groups

**Three were the freeze's own hand-typed baselines** — the `SoundLayout` literals,
`ASSEMBLED_LEN`, `DEBUG_ASSEMBLED_LEN`, the `+0x32` debug slide, `SCENE_REGISTRY`, and
`Ground_Move_Cap` — recorded above with their causes.

**Two were real gate defects hiding behind a symbol error**, and neither was a threshold
nudge. `ojz_run_b_port` lowered every section with `defines: vec![]`, so a module gating
emission on `DEBUG` could not see it; unfixed it fails on the name, and had it resolved it
would have built one shape's bytes and diffed them against the other shape's ROM window.
The same gate declared content shape-invariant for every section, which `ojz_bg_anim` is
not by design — `BGANIM_VIEW_EMIT` gates six arrays, so DEBUG carries a camera-motion view
record worth exactly 92 bytes. That assert is now per-section and TIGHTER than it was
(`<= declared` rather than `< 16`), with the number derived from the module.

**The rest were cross-seam symbols**, and the owner ruled they resolve from the LISTING
rather than from new pins. The grounds: `listing_symbol_addr` already served three port
gates, so this extends a proven seam; a listing address is read from the artifact the
reference build just produced, so it cannot rot and cannot disagree with the operand in the
ROM it is diffed against; and the tree is explicitly moving away from literal pins, the
retired baseline calling them "the pin-tax class the packing walk exists to kill". All were
verified present in the listings BEFORE any edit.

Where a family is generated or instrumentation-shaped, it is now SWEPT rather than listed —
`EditorRaster_`/`EditorCycle_`/`EditorSceneBinding_`/`OJZ_Preset_Sec`, `Dbg_DMA_`/
`DMA_Peak_`/`DMA_Split_`, `Canopy_`, `Cache_`, `Effects_`/`Raster_`/`Logic_`. That is what
ends the treadmill: `act_descriptor_port` carried a pinned row per authored scene and its
own comments record each one arriving as a red gate; the next scene now needs no edit at
all.

Two distinctions in the sweep are load-bearing rather than tidy. `extend_from_listing`
skips names a scope already pins, so a sweep beside hand-written rows cannot redefine one.
`extend_from_listing_ram` restricts to work RAM, because a family like `Canopy_*` names both
the RAM cells a neighbour reads and the PROCS `section.emp` defines — `plane_buffer_port`
carries BOTH spellings, needing the procs when it lowers alone and needing them excluded in
the two-module flip. The discriminator is the address, not a name list, because a name list
is the maintenance the sweep exists to remove.

Three symbols are deliberately NOT swept. `Frame_Counter` and the two camera cells already
have pins, so naming them adds no new literal. `VDP_HV_COUNTER` is a VDP HARDWARE address
(`$C00008`) rather than a layout one — fixed by the console, so it cannot drift with the
ROM, which is the very property the listing rule exists to protect.

## Booked, not fixed here

**`scripts/provision-aeon-ref.sh` refuses a legitimately pushed non-master revision.** Its
own comment states the intent as "REACHABLE FROM THE REMOTE, read with ls-remote"; the
implementation is `git merge-base --is-ancestor "$REV" origin/master`. `5875e60e` is
durable on the remote as a branch tip and was refused, so the freeze tree was
hand-provisioned to the same steps. Deliberately not fixed in this parcel — off the
critical path, and it should not be entangled with a freeze.

**Clippy was red in a second file and is now clean.** `48313276` corrected four tabbed
doc-comment lines in `crates/sigil-frontend-as/src/eval.rs`; the identical defect sat 35
times in that parcel's own TEST file, which `--all-targets` lints, so the workspace stayed
red after the lib was fixed. Reported rather than absorbed into this diff, and closed by
its owner in `472a36b8`. The shape is worth keeping: a defect fixed at one site of a
population, where the population was never enumerated.

**A bare `git worktree add --detach` of aeon is not enough** for the DEBUG shapes: the
control tree's first derive died on `no module engine.compression_vectors` plus a run of
`[embed.not-found]`. That is the gitignored-artifact class, not divergence.

## Trees

`/home/volence/sonic_hacks/.aeon-relayout-freeze` (`5875e60e`) and
`/home/volence/sonic_hacks/.aeon-attest-201` (`4f5ad5a1`) are provisioned and kept. The
control tree carried a DOCTORED `map.toml` and was removed rather than left where a later
run could resolve to it.

## The freeze cannot be re-taken against aeon master, and the reason is structural

The re-freeze this branch was asked for — regenerate the goldens against aeon's current
master `88547edacb9ef8a2f94fe321668ac0c62cb65dda` — is not merely inadvisable. It cannot
run. `refreeze --freeze` step 1 is `golden/capture_goldens.sh`, which deletes each target
and rebuilds it with `./build.sh <game>` inside `AEON_DIR`. That build refuses:

```
error: native build (sonic4 plain): [map.undeclared-island] ROM section at 0xB8000 is an
ANCHOR_GAP-inferred island but no `[[anchor]] at = 0xB8000` is declared — add it to the
placement map
```

**The mechanism.** `native::load_frozen_table` (`crates/sigil-harness/src/native.rs:247`)
reads `golden/offcanonical_sizes/<shape>.txt` through a `CARGO_MANIFEST_DIR` path baked in
at compile time, and `sonic4_profile` feeds it to the profile as `frozen_sizes`
(`native.rs:716`). `sigil-cli` depends on `sigil-harness`, so those rows travel INSIDE the
assembler binary. This branch's tables carry `Dac_Temp_Blip 0xa8000` and
`SoundTablesZ80_Head 0xb8000`; aeon master's `games/sonic4/map.toml` still declares
`dac_banks at = 0x90000` and `sound_bank at = 0xA0000`. The placement and the validating
map disagree, and the map lint says so.

This is the paired landing aeon's own map.toml prescribes, quoted from it verbatim: "the
remedy is to move BOTH anchors per the rule and refreeze sigil's frozen tables (a paired
aeon+sigil landing; the frozen tables are the placement authority, these anchors validate
them)."

**The controlled A/B, because a failing build alone does not name its cause.** One aeon
tree at `88547eda`, clean, detached, in the state the first run left it; two assemblers;
consecutive runs:

| assembler | md5 | `./build.sh` (sonic4 plain) |
|---|---|---|
| sigil master `1d33db75` | `ce7f5485513acb0b2604ae096e30e704` | exit 0 |
| this branch `72a7a355` | `a1998d2f2bd813b20d74aae6c1e873e0` | exit 1, the lint above |

Master's binary carries master's frozen tables (`0x90000` / `0xA0000`), which agree with
aeon master's anchors. The binary is the only variable, and the direction is the one the
mechanism predicts.

**`aeon_rev 5875e60e` is not stale and was never on an abandoned line.** Measured in
aeon: `git rev-list --count 88547eda..5875e60e` is **4**, and the four are
`265bf6fa` `446a27d9` `032b4cff` `5875e60e` — `446a27d9` being "relayout(rom): the banks
move to 0xA8000/0xB8000". They sit on `origin/parcel/rom-relayout-more-room`, pushed and
alive. `5875e60e` is aeon master plus the unlanded aeon half of THIS parcel. It is the
only aeon revision in which this branch's goldens can be reproduced at all.

## Which goldens reach `effects_scenes.emp`

Asked because aeon's `games/sonic4/data/generated/ojz/act1/effects_scenes.emp` gained
126/-9 lines across `5875e60e..88547eda`. Measured here, both halves of the question:

**Its symbols ARE enumerated, by name, in the pin manifest.** `repin.toml` curates five
labels generated into that file — `EditorSceneBinding_OJZ_Act1_Sec4`,
`EditorRaster_OJZ_Act1_{authored_probe,ojz_sec5_showcase,ojz_sec3_shimmer}` and
`EditorCycle_OJZ_Act1_ojz_sec3_shimmer` — each emitted into `pins.rs` as a per-shape VMA
and consumed by `act_descriptor_port`. So this is a curated five, not a wholesale
enumeration: the `offcanonical_sizes/*.txt` boundary tables name none of them, and the
file is additionally a named member of `ACT_DESCRIPTOR_ASSERT_FILES`
(`test_support.rs:1732`), whose link asserts the act-descriptor oracles decide.

**Its bytes are pinned transitively.** The file is a build input, so every ROM golden
holds its emitted output. The `pub data EditorReels_*: [i8; …]` that arrives at
`88547eda` therefore is NOT a listing-only change — `pub equ` is the zero-byte half of
that commit, `pub data` is not.

**None of the new symbols is known to this repository.** `git grep` over the whole tree
for `EditorReels`, `EditorReelBindings`, `reel_rates_ok` and `ojz_act1_sec_patched`
returns zero hits.

**The prediction that follows, recorded before it can be checked.** Every previous time a
new cross-seam `Editor*` label was generated into this file, the standalone
`act_descriptor_port` scope needed a new `[[symbol]]` row or its link assert failed with
"references symbol(s) … not defined in this link" — three instances, each documented in
`repin.toml` beside its row. `EditorReelBindings_*` at `88547eda` carries two `extern()`s,
so a freeze against any tree containing it should be expected to need a fourth.

## Verification of the merged branch, against the tree its goldens name

Since the freeze cannot be re-taken, the verification available is whether the merged
branch still holds against `5875e60e` — the revision the tip already names.

**The positive witness first**, because "no errors" is not one. `repin --check` with
`AEON_DIR=/home/volence/sonic_hacks/.aeon-relayout-freeze` (`5875e60e`, clean) reports
**`pins.rs unchanged`**, preceded only by the two standing `player_climb` declared-allotment
warnings. A tree that could not reproduce the pinned placement cannot produce that line, so
it is a positive result rather than an absence. No pin was regenerated and no file was
hand-edited: `repin.toml` was not touched and neither was `pins.rs`.

**The landing run.** `scripts/landing-run.sh --aeon /home/volence/sonic_hacks/.aeon-relayout-freeze`,
own target dir, `2026-09-04T17:02:41Z -> 17:06:52Z`:

```
tree        .../sigil/.worktrees/relayout-refreeze @ 0d40dd8f (parcel/relayout-refreeze-current)
reference   /home/volence/sonic_hacks/.aeon-relayout-freeze @ 5875e60e (HEAD, clean) — all four present
CARGO_EXIT  0
suites 386   passed 4463   failed 0   ignored 2   skip lines 0        RESULT GREEN
```

The wrapper's totals were re-derived independently from the raw `test result:` lines and
agree exactly. The three `panicked at` lines in the log are `should_panic` bodies
(`override_of_unknown_constant_panics`, `compress_panics_on_error`,
`ensure_generated_refuses_before_it_touches_an_absent_tree`), not failures. The verdict
stamps the tree DIRTY; the only untracked paths were this run's own `.runlogs/` and
`.target-master/`, and no tracked file differed from `0d40dd8f`. No `--baseline` was
passed, so the wrapper's reconciliation arm reports NOT CHECKED.

**`aeon_dir_matches_the_provenance_tip`, both directions**, because a gate that has only
been seen to pass has not been seen to work:

| AEON_DIR | aeon rev | result |
|---|---|---|
| `.aeon-relayout-freeze` | `5875e60e` (= the tip's `aeon_rev`) | `ok` — inside the landing run |
| `.aeon-ref-relayout-cur` | `88547eda` | `FAILED`, naming both revisions and the entry |

The refusal reads: "is at aeon 88547eda…, but the goldens were frozen from aeon 5875e60e…
(provenance tip `rom-relayout-more-room`, entry #202)."

**The four shapes did not move, because no freeze was taken.** They stand at the tip's
recorded values, and the tip's own tree reproduces every one:

| shape | golden CRC32/size | built in `.aeon-relayout-freeze` | paired `.lst` |
|---|---|---|---|
| `s4.bin` | `6f047af2`/819123 | identical | `s4.lst`, same second |
| `s4.debug.bin` | `d772f7d8`/840179 | identical | `s4.debug.lst`, same second |
| `demo.bin` | `3c5dcde6`/96602 | identical | `demo.lst`, same second |
| `demo.debug.bin` | `36014485`/102818 | identical | `demo.debug.lst`, same second |

Those four ROMs carry mtimes of `2026-09-04T06:28:59Z`..`06:47:27Z`, which PREDATE this
session — they are the earlier run's artifacts, not this one's, and the table says so
rather than presenting them as a fresh control. What makes them a build rather than a
copy of the goldens is the pairing: each `.bin` sits within milliseconds of a `.lst` of
the same stem, and a copied golden has no listing to pair with and would carry the
golden's mtime.

## What this parcel cannot close, and what closes it

The freeze is BLOCKED on aeon, not on sigil, and there are two independent locks. Either
alone is sufficient to refuse; both are the mechanism working.

**Lock 1 — the build.** `capture_goldens.sh` cannot build at `88547eda` at all, for the
reason traced above.

**Lock 2 — the ledger, which would refuse the append even if the build succeeded.**
`provenance::append_gate` reads the chain and finds it ARMED (entry #173 is the first to
record a strict run) with a tip that carries no strict run at all, so it returns
`Refused`: "A refreeze must not be built on top of goldens whose strict suite never ran …
Run `refreeze --attest` first." `plan_freeze` turns that into `FreezeAction::Refuse` for
any freeze that moves bytes, and `--supersede-tip` is refused too — `SUPERSEDE_WITHOUT_A_RED_RUN`
requires an already-recorded RED run, which the tip does not have. Measured from the
committed ledger, not from the code alone: 202 entries, 29 carrying `[entry.strict]`, the
first at #173, and the tip `rom-relayout-more-room` carrying neither `[entry.strict]` nor
`[entry.superseded]`.

So entry #202 needs `refreeze --attest` before ANY later freeze, byte-moving or not. That
attest must run with `AEON_DIR` at `5875e60e`, which `refreeze` enforces itself.

**The sequence that closes this, in order:**

1. aeon lands `parcel/rom-relayout-more-room` — the four commits `88547eda..5875e60e`,
   principally `446a27d9` — onto aeon master. Nothing sigil does can substitute.
2. `refreeze --attest` on entry #202 against `5875e60e`, which the landing run above
   already shows GREEN at 386/4463/0/2 — so this is expected to record a passing run
   rather than to discover something.
3. `refreeze --freeze` against the new aeon master SHA, which will then contain both the
   anchors and the `EditorReels_*` content, and should be expected to need a fourth
   `[[symbol]]` row for `EditorReelBindings_*`'s two `extern()`s.

Until step 1, `aeon_rev = 5875e60e…` is the correct and only honest value for the tip.

## Trees this run created and removed

`/home/volence/sonic_hacks/.aeon-ref-relayout-cur` (`88547eda`) was provisioned and then
REMOVED. Its `s4` ROMs were built by sigil master's assembler during the A/B control, not
by any freeze, so leaving it would leave a directory that reads as a provisioned reference
while holding artifacts from a different assembler than any golden — the substitution
class the provisioner's own header exists to prevent. The evidence it produced is in this
note and in the run logs; the tree is cheap to re-create once step 1 above lands.

`/home/volence/sonic_hacks/.aeon-relayout-freeze` (`5875e60e`) was READ and not written:
same HEAD, `git status` clean, and all four ROM mtimes unchanged across the landing run.
