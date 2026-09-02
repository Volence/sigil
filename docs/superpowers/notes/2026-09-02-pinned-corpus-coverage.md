# What the LIVE corpus exercises, and what a PINNED corpus would witness

**The gate this discharges** (`2026-09-02-decouple-step1-parcel.md`, "Sequencing and the one
hard gate"): *the pinned corpus must be shown to exercise what the live corpus exercised,
enumerated, before the live goldens stop being the reference. A corpus that is merely smaller
and green is the always-green trap with a new coat on.*

**A hole here is a FINDING, not a failure.** The vendoring half of step 1 cannot start while one
is open, so the holes below are the working output of this pass, not its shortfall.

---

## PROVENANCE OF EVERY NUMBER IN THIS FILE

Stated at the head rather than in an operator's memory, because this lane spent 2026-09-02 on a
proof that ran the wrong program and could not tell.

| what | value |
|---|---|
| this analysis | sigil branch `analysis/pinned-corpus-coverage`, off master `079cec97` |
| sigil `master` at write time | `6a8b3ecd` (moved under this branch mid-analysis; nothing here depends on it) |
| reference aeon tree read | `/home/volence/sonic_hacks/.aeon-ref-196`, HEAD `5944dad504862231568e04105f02614e379e9ff2`, working tree clean |
| aeon revision the goldens are pinned to | `8dd28114d534a5d045dd40a037020bed411a03d6` (`golden/provenance.toml` tail, entry 197) — **newer** than the reference tree above; `5944dad5` is its ancestor by 25 commits |
| the owner's live aeon checkout | **NOT READ.** `/home/volence/sonic_hacks/aeon` was not touched. |
| ROM/listing artifacts read | `.aeon-ref-196/{s4.bin,s4.debug.bin,s4.lst,s4.debug.lst}` and the seven committed goldens under `crates/sigil-harness/golden/` |
| assembler | **none was run.** No `cargo build`, no `cargo test`, no ROM built. Every measurement is a byte scan or a grep over artifacts that already existed. |
| the frozen `target/release/sigil` | not touched, not relinked, not chmod'd |
| emulator | not used |

Tools written for this pass, each of which prints its own identity and Python version when run.
They live in this session's scratchpad and are **not** committed — they are one-shot probes, and
the numbers they produced are transcribed below with the inputs named so any of it can be redone:

| tool | what it does | positive/negative control it carries |
|---|---|---|
| `absw_probe.py` | exact-pattern count of `jsr`/`jmp` abs.w vs abs.l encodings for a named target address | prints size + crc32 + md5 of every ROM it opens |
| `absw_census.py` | census of ALL `4EB8/4EB9/4EF8/4EF9` operands in a ROM, bucketed by target, to locate where the abs.w/abs.l boundary cuts | asserts the max abs.w target is `< 0x8000` (a violation would mean the scan is misreading); states in its own output that a data word can alias an opcode, so the DISTRIBUTION is the evidence, not any single hit |
| `absw_probe2.py` | per-symbol width with the opposite width as a **negative control** — a symbol reached at exactly one width and zero of the other | the zero-count column IS the control |
| `shape_labels.py` | per-shape label census from `golden/offcanonical_sizes/*.txt` | prints each table's own generated header (target, golden crc, assembled_end, label count) |
| `ref_paths.py` | every reference-tree-relative path the suite names, bucketed by whether a source snapshot carries it | prints an `engine/`-prefix hit count (397); zero there would mean the regex is broken |

For the reasoning that used `sigil --version`, the binary consulted was
`/home/volence/sonic_hacks/.sigil-land-197/target/release/sigil` → `sigil 0.1.0 (6a8b3ecd)`,
`closure-revision 6a8b3ecd…`, `tree: clean at capture`. It produced **no** number in this file;
it is recorded because the rule is to record it.

---

## 1. What "the live corpus" actually is, measured

The parcel note's shorthand is *"live aeon ROMs as goldens"*. Measured, it is four distinct
things, and they do not share a fate under step 1, so the table below is keyed on them.

**(L1) A live aeon SOURCE checkout**, resolved by `test_support::{reference_tree,
reference_tree_for_profile, aeon_dir}` (`crates/sigil-harness/src/test_support.rs:1236, :1265,
:993`). **127 of the 339 test binaries** under `crates/*/tests/*.rs` name one of those three
guards — the same derivation `reference_dependence::reference_dependent_binaries`
(`crates/sigil-harness/src/reference_dependence.rs:53`) performs, whose `FLOOR` is 20.
Split by crate:

| crate | corpus-dependent | synthetic |
|---|---|---|
| `sigil-cli` | 113 | 33 |
| `sigil-harness` | 13 | 16 |
| `sigil-frontend-emp` | 1 | 124 |
| everything else | 0 | 39 |
| **total** | **127 (37%)** | **212 (63%)** |

**The language frontend is already almost entirely synthetic.** The corpus dependence is
concentrated in `sigil-cli` — the port gates and the whole-ROM builds.

**(L2) Seven committed golden ROMs** — `crates/sigil-harness/golden/{s4,s4.debug,demo,
demo.debug,config_a,config_b,lean}.bin`. These are **already pinned artifacts** and, since the
Spec-5 flip, they are *sigil's own frozen output*, not asl's (`golden/provenance.toml`, header).
The asl witness survives only in the chain ROOT entry.

**(L3) Two BUILT LISTINGS that are NOT committed** — `s4.lst`, `s4.debug.lst`, produced by
building the reference tree. See hole **H-2**.

**(L4) The aeon repository's GIT STATE** — `provision-aeon-ref.sh` reads the pinned `aeon_rev`
out of the provenance tail and refuses a revision not reachable from `origin/master`
(`scripts/provision-aeon-ref.sh:104-110`); `provenance.toml` records an `aeon_rev` per entry.

**How fast (L1) moves, because this prices everything below.** Over the reference repository,
measured at `.aeon-ref-196`:

| window | aeon commits | `.emp` files ADDED | `.emp` files in tree |
|---|---|---|---|
| last 7 days (since 2026-08-26) | 600 | 11 | — |
| last 30 days (since 2026-08-03) | 1966 | 82 | 192 |

**43% of aeon's `.emp` corpus is younger than 30 days.** `golden/provenance.toml` carries **197
entries**, and the `aeon_rev` advances on essentially every one.

---

## 2. The enumeration parameter, stated so a later pass can vary it

Following the bar the placement inventory set for itself
(`2026-08-26-placement-constraint-inventory.md`, "Enumeration parameter"), this pass enumerated
by **what a test binary READS FROM THE REFERENCE TREE**, then bucketed each read by whether a
*frozen source snapshot* can supply it. It did **not** enumerate by the list of gates, because
the gate list is the artifact whose sufficiency is in question and taking it as the question
begs it.

**Two parameters a later pass should vary, neither run here:**

1. **By what aeon's own build-side gates check** (`tools/bganim_room.py --gate`, `build.sh`).
   The 08-27 re-check already owed this and it is still owed.
2. **By what CHANGES between two aeon revisions** — the parameter that would find the
   *dynamic* class directly rather than by argument. It needs two builds and was out of scope
   for a no-build pass.

---

## 3. THE PREDICTED HOLE — verdict: **REAL, but not where the prediction placed it**, and the row's own coordinate has rotted a second time

### 3.1 The re-derived coordinate

`ABSW-CEILING-INVARIANT` (`docs/OVERSEER-ROW-HISTORY.md:228`) says the coordinate lives in a
command: `grep -nE 'SoundTablesZ80_Head|Sound_PlaySFX' s4.lst` — *"at 0x8000 or above is clear,
below means the encodings have flipped."* Run against `.aeon-ref-196/s4.lst` (aeon `5944dad5`):

```
(0) 1289/7FBC :        Sound_PlaySFX:
(0) 1292/8000 :        $engine.sound_api$Sound_PlaySFX$ps_ret:
(0) 1293/8000 :        SoundTablesZ80_Head:
```

**`Sound_PlaySFX` is at `0x7FBC` — BELOW the ceiling.** By the row's own reading, *the encodings
have flipped*, and the row's stored snapshot (`0x8024`, above) is stale again — three days after
it was rewritten specifically to stop going stale.

Confirmed at the byte level, not inferred from the listing
(`absw_probe.py` on `.aeon-ref-196/s4.bin`, crc32 `ac10ab85`, 719325 B):

```
Sound_PlaySFX@0x7FBC  jsr: abs.w x9   abs.l x0
Sound_PlaySFX@0x7FBC  jmp: abs.w x2   abs.l x0
```

Eleven short-form transfer sites. The source confirms the mechanism: `player_common.emp:1145`
carries an explicit `jsr Sound_PlaySFX` annotated *"explicit jsr (kept-width class)"*; every
other reference is `jbsr`/`jbra`/`bsr.w`, which are branch-relative and never abs at all.

### 3.2 The row's standing operational advice is now INVERTED for the plain shape

Full census of every `jmp`/`jsr` absolute operand in each ROM (`absw_census.py`):

| shape | ROM read | max abs.w target | slack to `0x8000` | min abs.l target in `[0x8000,0x10000)` | straddles the ceiling? |
|---|---|---|---|---|---|
| `s4` | golden, crc `f403f461` | `0x7FC6` | **58 B** | *none* | no — one-sided |
| `lean` | golden, crc `71ae6ed4` | `0x7FC6` | 58 B | *none* | no |
| `config_b` | golden, crc `ce46aaa3` | `0x6592` | 6766 B | *none* | no |
| `demo` | golden, crc `30a31d81` | `0x2D1C` | 21220 B | *none* | no |
| `demo.debug` | golden, crc `51056291` | `0x3B58` | 17576 B | *none* | no |
| **`s4.debug`** | golden, crc `50545389` | `0x7FE6` | **26 B** | **`0x82EE`** | **YES** |
| **`config_a`** | golden, crc `1335897b` | `0x78CE` | 1842 B | **`0x80BE`** | **YES** |
| `s4` (live) | `.aeon-ref-196`, crc `ac10ab85` | `0x7FBC` | 68 B | *none* | no |
| `s4.debug` (live) | `.aeon-ref-196`, crc `fa866f19` | `0x7FDC` | 36 B | `0x82E4` | YES |

**Two corrections to the row, both measured:**

1. **In the plain family (`s4`, `lean`, `config_b`, both demos) there is NO abs.l side at all**
   in ROM range above the ceiling. Nothing there can flip on a shrink. The exposure is
   **growth**: 58 bytes (golden) / 68 bytes (live) of upstream growth pushes `Sound_PlaySFX`
   over `0x8000` and widens eleven sites by 2 bytes each. *"Growth is the safe direction; only
   a shrink is dangerous"* is **backwards for this shape today.**
2. **The binding symbol is not `Sound_PlaySFX` or `SoundTablesZ80_Head`.** In the debug family
   it is `Raster_Install`, and the tightest margin in the whole set is **26 bytes** (`s4.debug`
   golden). `SoundTablesZ80_Head` at `0x8000` is a *Z80 section VMA* (`vma: $8000`), in a
   different address space from the 68000 abs.w rule; its appearance at exactly the ceiling is a
   coincidence of two numbering schemes, and using it as the ceiling witness reads the right
   number for the wrong reason.

### 3.3 The static boundary IS witnessed — here is the witness

The prediction was that a pinned corpus would stop exercising the encoding boundary. **For the
static decision it does not, and the witness is nameable.** In the debug family the boundary
cuts *through live code*, with named symbols on both sides and a clean negative control — every
symbol is reached at exactly one width and **zero** of the other (`absw_probe2.py`,
`.aeon-ref-196/s4.debug.bin`, crc32 `fa866f19`):

| symbol | address | `jsr`/`jmp` abs.w | abs.l |
|---|---|---|---|
| `Parallax_Update` | `0x77F6` | **3** | 0 |
| `Raster_Install` | `0x7FDC` | **2** | 0 |
| `Raster_GetChannelBand` | `0x82E4` | 0 | **1** |
| `Effects_LatchWorldLines` | `0x8316` | 0 | **1** |
| `Effects_SetWorldY` | `0x8330` | 0 | **1** |
| `Effects_ResolveParallax` | `0x87C4` | 0 | **1** |
| `Level_LoadArt` | `0x8886` | 0 | **1** |

A byte-identity gate over `s4.debug.bin` therefore **does** assert that sigil picks `.w` for
`0x7FDC` and `.l` for `0x82E4`, and a pinned snapshot carries that assertion verbatim. There are
also synthetic unit witnesses for the rule itself: `crates/sigil-link/src/relax.rs:1366-1450`
builds `jmp abs.w` / `jmp abs.l` candidates by hand and pins both halves of `asl_width_rule`,
including the RAM-range `$FF8000` upper half.

### 3.4 …and the DYNAMIC case is the hole. Three of them, actually.

**H-1a — the CROSSING is unwitnessed and unwitnessable by any frozen source snapshot.**
What no pinned corpus can exercise is a symbol *moving across* the ceiling and the relaxation
fixpoint re-converging around it: the grow/shrink iteration at the boundary, the cascade into
neighbouring sections, and the documented **non-monotonicity** of `asl_width_rule`
(`crates/sigil-link/src/relax.rs:529-535`: the rule selects abs.w on `[0, 0x7FFF]`, abs.l on
`[0x8000, 0xFF_7FFF]`, then abs.w *again* — so a fixpoint step can settle a site at abs.l whose
final address only needed abs.w). Crossing requires the layout to move; a pinned layout does not
move. **Closing fixture:** synthetic, and it is *possible* against a frozen snapshot — but only
as a **synthetic link fixture**, not as a corpus fixture: two hand-built sections where a
parameterised filler drives a target address across `0x8000` in both directions, asserting the
chosen rung and the re-converged length at each step. That belongs in `sigil-link`, beside the
existing `relax.rs` unit tests, not in the vendored tree.

**H-1b — the STATIC witness is itself a layout coincidence, and pinning does not make it
durable.** It exists in 2 of 7 shapes and in neither of the two canonical *plain* shapes. It
exists because the debug shapes happen to push code past `0x8000` with absolute references on
both sides. **If the snapshot is taken at a revision where that is not true, the witness vanishes
and nothing goes red** — the same failure mode the prediction named, arriving through the choice
of pin rather than through the pinning itself. **Closing fixture:** an explicit assertion, not a
fixture — a gate over the frozen corpus asserting *"at least one shape reaches an absolute target
in `[0x7F00, 0x8000)` AND at least one reaches one in `[0x8000, 0x9000)`"*, so a snapshot that
loses the straddle refuses instead of passing. It is possible against a frozen snapshot and is
cheap: it is a byte scan of the goldens, which is exactly what `absw_census.py` did here.

**H-1c — the free early-warning disappears, and nothing replaces it.** Today an encoding flip
announces itself as *"an unrelated test failure in a region nobody edited"* — the row's own
words — because the live corpus keeps moving through the boundary. That is a real detector, and
it is the *only* thing that has ever caught this class. It is not replaced by the pinned corpus,
and its designated replacement (the nightly drift observer) **does not run** — see **H-3**.

**The margins are the point.** 26 bytes (`s4.debug`), 36 bytes (live debug), 58–68 bytes (plain).
One parcel of ordinary size crosses any of them.

---

## 4. TABLE A — the seven shipped shapes

`native::shipped_shapes` (`crates/sigil-harness/src/native.rs:997`). "Uniquely exercises" is
what is lost if that shape is dropped from the pinned corpus; every row TRANSFERS if the shape
is carried, so the operative question for step 1 is **which shapes the snapshot carries**.

| shape | `debug` | `sound_on` | labels in its frozen table | uniquely exercises | pinned witness | verdict |
|---|---|---|---|---|---|---|
| `sonic4 plain` | no | yes | 68 | the canonical byte gate; `pins.rs` `ASSEMBLED_LEN`; one-sided abs.w ceiling at 58 B | `golden/s4.bin` + `offcanonical_sizes/s4.txt` + rebuild control | TRANSFERS |
| `sonic4 debug` | yes | yes | 80 | the deb2 appendix + error-handler island order; **the tightest abs.w/abs.l straddle (26 B)**; the debug-only `D1C_DEBUG_EXTRA` contract rows | `golden/s4.debug.bin` + `s4_debug.txt` | TRANSFERS |
| `demo plain` | no | no | 40 | sound-OFF engine, the agnostic-engine path, a live `[[hole]]` | `golden/demo.bin` | TRANSFERS |
| `demo debug` | yes | no | 42 | sound-OFF **and** debug together; a live `[[hole]]` | `golden/demo.debug.bin` | TRANSFERS |
| `config_a` | yes | yes | 86 | `SOUND_DEBUG_HOTKEYS` + `SOUND_DBG_MIRROR` arms; the only shape carrying `Debug_MusicToggle`, `Sound_DebugMirror`, `Ani_Particle_End`; the second abs straddle | `golden/config_a.bin` + `config_a.txt` | TRANSFERS |
| `config_b` | no | no | 68 | sonic4 game with the sound modules *dropped* (a subtractive registry, not a different game); a live `[[hole]]` | `golden/config_b.bin` | TRANSFERS |
| `lean` | no | yes | 68 | the only shape carrying `ReleaseFault` | `golden/lean.bin` | TRANSFERS |

**34 labels appear in all seven** (`shape_labels.py`); only 4 labels are unique to a single shape
(3 in `config_a`, 1 in `lean`). The shapes are near-parallel in *placement* terms and differ in
*which arms are lowered* — so shape coverage is a `-D`-set question, not a label question, and
`native::shape_defines` is where it is decided.

**Shape-level hole:** none found. All seven goldens are already committed and all seven size
tables are already committed. **This axis is the strongest part of the pinned corpus.**

---

## 5. TABLE B — the placement constraints R1–R9

From `2026-08-26-placement-constraint-inventory.md` as amended by `2026-08-27-constraint-recheck.md`.

| # | constraint | what exercises it TODAY (file · symbol) | pinned witness | verdict |
|---|---|---|---|---|
| R1a | island set == declared anchor set | `native::validate_placement` → `[map.undeclared-island]` / `[map.anchor-absent]`, on every `build_rom_chained_with_listing`; red-first probes in `native::placement_validation_tests` (`undeclared_island_fires`, `anchor_absent_fires`, `shape_gated_sound_bank_anchor`) | same code path over the snapshot; the probes are **synthetic** and independent of any tree | **TRANSFERS** |
| R1b | a declared anchor holds *that* section | **nothing.** Anchors are keyed `HashMap<u32,&str>` on `a.at`; the name is only printed. Demonstrated absent 2026-08-27 by a probe that was then deleted. | nothing — unchanged | **HOLE TODAY, HOLE AFTER** (not a step-1 regression; `ANCHOR_BINDS_SECTION` is the drafted predicate) |
| R1c | `ObjCodeBase` 64 KB · `dac_banks`/`sound_bank` `$8000` alignment | **now recaptured**: `crates/sigil-harness/src/section_align.rs` (107 rows) + `native::validate_declared_alignment` / `validate_resolved_alignment`, both always-on inside `build_rom_chained_with_listing`; witness `crates/sigil-cli/tests/section_alignment_declared.rs` (red-first: doctors the `Sfx_33` frozen row `+4`) | the declaration is **sigil's own source**, not the corpus's; the checks run on every build of every shape | **TRANSFERS** |
| R1d | `sound_bank` anchor `vma` ↔ `at` window phase | **nothing.** `seam2::bank_anchors_from_str` derives every head VMA from the declared `vma` and never checks phase. `by-reading` in the 08-27 recheck; not raised since. | nothing — unchanged | **HOLE TODAY, HOLE AFTER** (`SOUND_BANK_WINDOW_PHASE` drafted) |
| R2 | the far-scratch `0x70_0000 + k·0x10_0000` measuring base | a mechanism, not a predicate (`native.rs:2118-2126` — the frozen span is what anchors relaxation to asl's widths); ruled by aeon: drop it as its own byte-moving parcel after step 4 | mechanism is sigil's; unchanged by the corpus | **NOT A CONSTRAINT** — but see **H-6** |
| R3 | a `bank:` section fits and never straddles | `sigil_link::relax::bank_diag` (c1/c3); `crates/sigil-link/tests/final_placement.rs` — `pinned_bank_section_straddling_is_a_loud_error_not_moved`, `bank_section_over_bank_size_is_a_loud_error`, `chained_bank_section_bumps_when_it_would_straddle` | **synthetic already** — `sigil-link` tests build their own sections | **TRANSFERS** |
| R4 | do not gate on `resolve_layout_measuring` | a constraint on gate *construction* (`check_image = false` skips c2/c3) | unchanged | **NOT A CONSTRAINT** in this taxonomy |
| R5 | map bases cosmetic under `Frozen` | **RETIRED** — the premise is gone (`map_placement::PlacementMap` has no per-section bases; `order` drives the walk) | n/a | **RETIRED** |
| R6 | a region's `end` is its own extent, not a neighbour's | `crates/sigil-harness/tests/region_end_contracts.rs` — `every_region_end_contract_holds_against_the_live_layout` and `deleting_an_allotment_declaration_refuses_the_live_manifest` (~50 refusals per run, red-first permanently) | the manifest is sigil's; the *layout* comes from a resolve of the reference tree — a pinned tree resolves identically | **TRANSFERS**, with **H-5**: the 80 remaining allotment conversions are paid *one per port-gate touch*, and port-gate touches are driven by aeon churn |
| R7 | section alignment declared, not inferred | `section_align.rs` + the two always-on validators (see R1c); `native::declared_alignment_tests` (7), `section_align::tests` (4) | as R1c | **TRANSFERS**. Its stated residual — *"a NEW section arriving with a real requirement nobody declares"* — is the completeness half refusing the build. Under a pinned corpus no new section ever arrives, so the check never fires **and never needs to**; the risk migrates wholesale to the flip. |
| R8 | error_handler island is the last emitter | `native::check_error_handler_is_last` from `append_deb2_appendix`; `crates/sigil-harness/tests/error_handler_island_order.rs` (6 tests incl. `a_section_emitted_after_the_blob_is_refused_by_name`) + `error_handler_island_membership.rs` (per-shape set-diff over `native::shipped_shapes`) | the order test is **synthetic** (plants its own section); the membership test is per-shape over the snapshot | **TRANSFERS** |
| R9 | a declared `[[hole]]`'s interior stays empty | `native::hole_interior_faults` from `validate_placement`; `crates/sigil-cli/tests/hole_interior_reserved.rs`, live over demo plain / demo debug / config_b | all three shapes are in the snapshot | **TRANSFERS** |

**Count on this axis: 9 rows classified, 6 TRANSFER, 2 are pre-existing holes that step 1
neither opens nor closes, 1 retired, 2 unclassifiable-by-design (R2, R4).**
**Zero step-1 regressions on the placement axis.** That axis is genuinely in good shape, and
that is *because* the R6 and R7 recapture parcels already moved the declarations into sigil's
own source. The exposure is elsewhere.

---

## 6. TABLE C — what the corpus-dependent binaries read, and whether a source snapshot supplies it

### 6.1 By path, mechanically

Derived by `ref_paths.py` over every string literal in `crates/*/tests/*.rs` and
`crates/sigil-harness/src/*.rs` that looks like a reference-tree path (355 files scanned;
positive control: 397 `engine/`-prefixed hits).

| bucket | distinct paths | examples | does a vendored SOURCE snapshot carry it? |
|---|---|---|---|
| **SOURCE** | 129 | `engine/system/constants.emp` (49 readers), `engine/system/types.emp` (38), `engine/objects/sst.emp` (36), `engine/structs.emp` (31), `games/sonic4/map.toml` (9) | **YES** — this is the bulk, and it transfers cleanly |
| **BUILT ARTIFACT** | 81 | `s4.bin` (130), `s4.debug.bin` (89), **`s4.lst` (5)**, **`s4.debug.lst` (3)**, the five off-canonical `.bin`, the seam-2 blobs (`mt_bank_body.bin`, `sfx_bank.bin`, `dac_*.bin`, …) | **NO.** The seven ROMs are already committed as goldens; **the two listings are not** (→ **H-2**); the seam-2 blobs are regenerated (→ **H-4**) |
| **GENERATOR / TOOL** | 3 | `tools/convsym`, `tools/s4budget.py`, `tools/drift_record.py` | **only if the snapshot carries `tools/`, and `convsym` is a BINARY** |
| **BUILD SCRIPT** | 9 | `build.sh`, `capture_goldens.sh`, `atomic_freeze.sh`, `scripts/lib/suite_paths.sh` | source, but they *run the toolchain* |
| **OTHER** | 7 | `scripts/drift_report.py`, `scripts/systemd/sigil-ref-drift.{service,timer}`, `docs/DRIFT_RECORD_SEAM.md` | sigil's own |

### 6.2 By gate family

**Provenance of this sub-table, stated because I did not read all 136 files myself.** The family
classification was produced by a delegated read-only pass over `crates/*/tests/*.rs`. **Four
citations were re-derived here by hand**, chosen because the verdicts rest on them:

| claim | re-derived at | result |
|---|---|---|
| a missing reference path skips green | `crates/sigil-harness/src/test_support.rs:1236-1248` | **holds, and is stronger than reported** — see §6.3 |
| a source-only dir is not an aeon checkout | `scripts/lib/suite_paths.sh:69-92` | holds — `.git` + `build.sh` + `engine` |
| `rings_port` resolves callees out of the listing | `crates/sigil-cli/tests/rings_port.rs:362-364` | holds |
| `provenance_chain` compares aeon's git HEAD | `crates/sigil-harness/tests/provenance_chain.rs:38-45, :104, :145` | holds |

Every **other** `file:line` in the family rows is the delegated pass's and was not re-derived.
**Treat them as leads, not as measurements.**

Population correction: the guard-string derivation yields **127**; adding 9 files that read the
tree without naming a guard (`freeze_step_gap`, `golden_freeze_atomicity`, `golden_write_gate`,
`offcanon_assembled_bar`, `reference_tree_named_write`, `rev_reachability`,
`scripts_name_their_tree`, `shipped_shapes`, `suite_paths_precedence`) gives **136**. Of those,
**91 also name a built ROM, `.lst`, or golden**. Two guard-string matches are **false positives**
that read no tree: `reference_tree_write_guard.rs` (matches via its own
`fn absent_reference_tree`) and `reference_dependence_is_named.rs` (it is the file that *names*
the guards). Both are counted by the derivation that sizes the partial-run banner.

| family | n | what it asserts | verdict against a vendored SOURCE snapshot |
|---|---|---|---|
| **F1 — region byte gates vs the LIVE tree's built ROM** | 61 | lower one real `.emp`, pin the region at `pins::<REGION>`, byte-compare against `<aeon>/s4{,.debug}.bin` | **DEGRADES.** The comparand is byte-equal to `golden/s4.bin` at the pinned rev, so this is a **mechanical redirect** to `golden/` — the pattern 14 sibling files already use. What is lost is the second reading: today these compare a per-region compile against a whole-program build of the *same* source; frozen, both sides move only when sigil moves. |
| **F2 — byte gates vs sigil's own committed `golden/`** | 17 | same, but the comparand is already `crates/sigil-harness/golden/*` | **TRANSFERS** unchanged |
| **F3 — gates reading the build LISTING** | 3 (`rings_port`, `test_p1_player_port`, `m1b_gate`) | resolve every cross-region `extern("NAME")` out of `<aeon>/s4{,.debug}.lst` | **DIES** — see **H-2** |
| **F4 — gates shelling `<aeon>/tools/convsym`** | 3 (`native_full_rom`, `native_offcanonical_full`, `section_row_fixture`) | the deb2 appendix + full-file CRC vs the provenance tip | **DIES** unless `tools/convsym` (a compiled binary) is vendored — see **H-4** |
| **F5 — source-only lower/link/negative probes** | 30 | doctoring a real `.emp` moves bytes; standalone lower is a loud missing-symbol error; `.emp` spelling == its AS twin | **TRANSFERS** — reads source text only |
| **F6 — corpus sweeps (`read_dir` over the whole `.emp` tree)** | 10 (`contract_closure_corpus`, `dead_save_corpus`, `out_verify_corpus`, `preserves_corpus`, `slot_type_corpus`, `warn_tier_corpus`, `movem_restore_guard_corpus`, `parcel_8b_stage_gen_touchers`, `cfg_blind_spots`, `seam2_layout_derivation`) | closure/lint/census residue over every `.emp`, per shape, against a frozen baseline | **DEGRADES → DIES.** See **H-5**. Two are outright tautologies once frozen: `parcel_8b_stage_gen_touchers` (*"exactly three touchers"* — its value is catching a fourth) and `cfg_blind_spots` (*"census is six sites, five procs"*). |
| **F7 — whole-shape build gates over all 7 shapes** | 8 | every shape builds; alignment declared; island membership; every emitted m68k instruction round-trips through capstone and through sigil's own decoder | **DEGRADES.** `corpus_builds.rs` **DIES** as written — its own header calls it *"the brick witness for the nightly source-gate lane"*, and that lane's subject is aeon's live tip. The two instruction-stream gates keep working but their operand population stops growing. |
| **F8 — drift / staleness / currentness detectors** | 7 | a fixture must track a file that changes | **DIES.** This is the family the prediction's shape belongs to. Named in **H-6**. |
| **F9 — resolver / environment / write-guard gates** | 7 | d-17 and d-18: an external, possibly-absent, possibly-wrong checkout | **DIES.** Named in **H-7**. |
| **F10 — freeze-ritual / ledger gates** | 5 | provenance chain, atomic freeze, write gate, step-gap journal | **TRANSFERS** — these read only committed files or synthetic beds |
| **F11 — external sibling repo** | 1 arm of `m1b_gate` | `g++` against `oracle-old`'s `Symbols.cpp` | **unaffected** — not an aeon dependency |

### 6.3 The skip semantics, verified here rather than taken

The delegated pass reported that a source-only snapshot would make ~66 binaries *"skip green"*.
**Read at the source, the mechanism is better than that and the correction matters**
(`crates/sigil-harness/src/test_support.rs:1236-1248`):

```rust
pub fn reference_tree(rels: &[&str]) -> Option<PathBuf> {
    let aeon = aeon_dir();
    if let Some(missing) = rels.iter().find(|rel| !aeon.join(rel).exists()) {
        let path = aeon.join(missing);
        assert!(!strict_gate(), "SIGIL_STRICT_GATE set but reference missing: {}", path.display());
        eprintln!("skip: reference not at {} (set AEON_DIR)", path.display());
        return None;
    }
    Some(aeon)
}
```

A missing path **panics under `SIGIL_STRICT_GATE=1`, naming the absent file**, and that is the
flag the pre-merge run sets. So against a source-only snapshot the F1/F3/F4 families fail
**loudly in the landing run** and skip green only in a casual one. That is the right direction,
and it means H-2/H-4 announce themselves at cutover rather than hiding.

**And the codebase already anticipated a source-only tree.** `reference_tree_for_profile`'s own
doc comment (`:1252-1258`) says so verbatim:

> *"A built ROM is not one of them. These gates assemble aeon SOURCE and compare against sigil's
> own committed goldens, so a source-only checkout — every `.emp` present, nothing built — is a
> tree they run fully against; sentinelling on `s4.bin` reports such a tree missing."*

The profile-driven guard is the migration target. **The work step 1 actually implies is moving
the 61 F1 binaries from `reference_tree(&["s4.bin"])` to `reference_tree_for_profile` +
`golden/`** — 61 mechanical edits with an existing 14-file precedent, not a research problem.

---

## 7. THE HOLES

Ranked by how quietly each one fails.

### H-1 — the abs.w/abs.l **crossing** (predicted). Three sub-holes; see §3.4.
Static decision **WITNESSED** (2 of 7 shapes, named symbols, negative control).
Dynamic crossing **UNWITNESSED and unwitnessable by a frozen source snapshot**.
Straddle-existence **UNASSERTED**, so the static witness can be lost at pin time in silence.
Early-warning **LOST** with no live replacement.
*Closing fixtures:* a synthetic `sigil-link` boundary-crossing fixture (possible), and a
straddle-existence assertion over the goldens (possible, cheap).

### H-2 — **`s4.lst` / `s4.debug.lst` are consumed by port gates, are NOT committed, and their absence FAILS OPEN.** ← not predicted
`test_support::listing_symbol_addr` (`crates/sigil-harness/src/test_support.rs:552`):

```rust
let text = std::fs::read_to_string(listing).ok()?;   // ← a MISSING FILE returns None
…
panic!("listing {} carries no symbol `{name}`", …);  // ← a PRESENT file missing the symbol PANICS
```

**The two failure modes are opposite.** A listing present but lacking the symbol is loud. A
listing **absent** is silent: `None`, no label pushed, and the failure surfaces later and
elsewhere as `unresolved symbol <X> for fixup in section <Y>`. `provision-aeon-ref.sh:172-183`
records that this *"reads exactly like a real regression in whatever parcel you happen to be
holding, and it cost this lane a false attribution and three needless reverts before the cause
was found."*

A vendored **source** snapshot does not carry a listing. So step 1 must either (a) land
`FREEZE-THE-LISTINGS` first — already an open board row
(`docs/OVERSEER-ROW-HISTORY.md:240`, ruled by the engine lane, four canonical shapes, with the
implementation trap already documented: the off-canonical passes write their listings under the
*canonical* names, so the freeze must happen in the canonical-first window — or (b) rebuild the
listings from the snapshot at test time, which reintroduces a build step and makes the listing
sigil's own output rather than a reference.
**This is a hard prerequisite of the vendoring half and I did not find it recorded as one.**
*Closing fixture:* the freeze itself, plus a red-first probe that DELETES a listing and asserts
a refusal naming the listing — because today that deletion is indistinguishable from a clean run.

### H-3 — **the compensating control does not exist yet: `sigil-ref-drift.timer` is NOT INSTALLED.** ← not predicted at this severity
Step 1(b) moves drift detection to a *non-blocking nightly observer*. Measured on this machine
just now:

```
$ systemctl --user list-timers --all
aeon-effects-gates.timer   … last Wed 2026-09-02 04:17:31
sigil-source-gates.timer   … last Wed 2026-09-02 05:17:31
2 timers listed.
$ systemctl --user list-unit-files | grep -iE 'sigil|drift|aeon'
aeon-effects-gates.{service,timer}   sigil-source-gates.{service,timer}
```

**No `sigil-ref-drift` unit is installed.** The board row `DRIFT-TIMER-NOT-INSTALLED`
(`docs/OVERSEER-ROW-HISTORY.md:216`) says so and says why it cannot self-install; the parcel
note names it as a live prerequisite of (b). The measurement confirms both, today.

**And the ledger is harder evidence than the timer list.** `$XDG_STATE_HOME/sigil-ref-drift/`
exists and holds three log files — **and no ledger, record or database of any kind**:

```
$ /usr/bin/ls -a /home/volence/.local/state/sigil-ref-drift
.  ..  build.log  nightly.log  provision.log
$ cat .../nightly.log
2026-08-29T22:39:37 SELFTEST: the notification path works
2026-08-29T22:39:46 COULD NOT RUN: N is not a positive integer (got '0' from SIGIL_DRIFT_N …)
2026-08-29T22:39:46 COULD NOT RUN: N is not a positive integer (got 'abc' …)
2026-08-29T22:39:52 COULD NOT RUN: N is not a positive integer (got 'unset' …)
Preparing worktree (detached HEAD 3ad7ed02)
HEAD is now at 3ad7ed02 …
HEAD is now at 3ad7ed02 …
2026-08-30T06:05:47 SELFTEST: the notification path works
2026-09-02T04:24:39 COULD NOT RUN: the aeon checkout could not be resolved (see stderr)
```

**Nine lines, of which two are selftests and four are `COULD NOT RUN`. Zero observations.**
Positive control for the grep: `SELFTEST` matches 2, so the file is being read; the count of
lines matching `OBSERVED|MATCH|CASE|verdict|entry` is **0**.

**The observer has never observed anything, in its entire lifetime.** That is a stronger and
worse statement than "the timer is not installed", and it changes how H-10 should be read: the
mechanism nominated to absorb this whole parcel's accepted cost has not been demonstrated to work
end-to-end even once. Note also the last line — **today, 2026-09-02T04:24**, something did invoke
it and it died at *checkout resolution*, which is H-9(a)'s mechanism arriving in the observer's
own log.

**Why this is worse than a scheduling detail.** The pinned corpus's accepted cost is *"assembler
regressions surface nightly rather than at the next aeon landing."* If the nightly never runs,
the cost is not deferred detection — it is **no detection**, and the loss lands on a class that
turns over at **11 new `.emp` files a week**. Compounding: the same row records that the ledger
lives under `$XDG_STATE_HOME/sigil-ref-drift/`, *machine-local*, so even once it runs the
evidence step 4 is supposed to weigh does not survive this machine.
*Closing action:* not a fixture — install the timer and prove one observation lands, **before**
the vendoring half, not after.

### H-4 — **the pinned corpus is not hermetic, and part of it is SIGIL'S OWN OUTPUT.** ← not predicted
`provision-aeon-ref.sh` documents what a bare aeon worktree lacks (`:4-8`: *"WITHOUT THEM THE
SUITE REPORTS ~200 FAILURES THAT READ EXACTLY LIKE GOLDEN DIVERGENCE"*). Three ingredients, and
they are three different problems:

| ingredient | produced by | problem for a vendored snapshot |
|---|---|---|
| `tools/bin/salvador` | `make -C tools/salvador` (`:154-158`) | a compiled **binary**; vendoring it pins a C toolchain output into sigil |
| compression vectors | `python3 tools/gen_compression_vectors.py` (`:159`) | needs `tools/` and a Python run |
| `engine/sound/generated/*` | **`cargo build --bin emit_sound_blob`**, i.e. **sigil itself** (`:169-173`) | **circular.** Vendor it → sigil's past output becomes an input to sigil's own test. Regenerate it → the "pinned" corpus moves with sigil, and is not pinned. |

The third is the one that matters. `seam2` reads those blobs (`mt_bank_body.bin`,
`sfx_bank.bin`, `dac_sample_tab.bin`, … — 20+ distinct names in Table C). Neither branch is
obviously right and **the parcel has to pick one deliberately.**
*Closing fixture:* not a fixture — a recorded decision, plus (if vendored) a gate asserting the
vendored blobs equal what today's `emit_sound_blob` produces, so the circularity is a *checked*
one rather than a silent one.

### H-5 — **the frozen-baseline ratchets stop ratcheting.** ← not predicted
`crates/sigil-harness/src/contract_baseline.rs` holds the corpus's *"standing residue at freeze
time"* in five tables (`Z80_OUT_UNVERIFIED_BASELINE`, `OUT_UNVERIFIED_BASELINE`,
`INOUT_UNVERIFIED_BASELINE`, `D1C_BASELINE`, `D1C_DEBUG_EXTRA`), with a **two-directional**
ratchet: a firing not in the baseline fails, **and a baseline row that stops firing also fails**
(*"the analysis narrowing is the destructive direction"*). Its consumers are the corpus gates —
`dead_save_corpus.rs`, `contract_closure_corpus.rs`, `out_verify_corpus.rs`,
`preserves_corpus.rs`, `slot_type_corpus.rs`, `warn_tier_corpus.rs`,
`movem_restore_guard_corpus.rs`, `game_contract_env_coverage.rs`.

Against a **frozen** corpus, a frozen baseline over a fixed input is a fixed expected output.
The gate still catches an **analyzer** regression — real value, and this is DEGRADES not DIES —
but it stops being a detector of *new engine violations*, and the burn-down direction (*"a
genuinely-loose contract: burn it down"*) has no source of new material. The number stops moving
and the gate stays green, which is the exact silhouette of the always-green trap the parcel note
names — arriving on a mechanism nobody would think to look at, because it is *supposed* to be
frozen.
*Closing fixture:* possible and cheap — record the baseline population sizes at pin time and
assert they are unchanged, so "frozen" is asserted rather than assumed; the *detection* value is
aeon's to reclaim in the nightly, not sigil's to keep.

### H-6 — **R2's asl-parity scratch outlives the corpus that justified it.** ← half-predicted by the inventory, not by the prediction
The `0x70_0000 + k·0x10_0000` measuring base exists to reproduce asl's conservative `abs.l`
widths for never-pinned sections (`native.rs:2118-2126`, `2026-08-26-measure-at-packed-base-packet.md`
§2b). Aeon has ruled it should be dropped, *after* step 4 archives the certification. But note
what it interacts with: it is a **deliberate abs-width emulation**, and §3 has just shown the
abs boundary sits within tens of bytes of live code. **Dropping it is a byte-mover that moves
bytes near the ceiling**, and after step 1 the only thing that would catch a bad interaction is a
byte-diff against a corpus that no longer moves. Sequencing consequence: **the R2 drop wants to
land BEFORE the corpus is pinned, or it wants the H-1a synthetic crossing fixture to exist
first.** I found no record of that ordering.

### H-7 — **every aeon-side check in the freeze ritual goes vacuous, and vacuous reads as passing.** ← not predicted
Five refusals exist today whose subject is *an aeon repository that can move*. Against a pinned
snapshot each becomes true by construction:

| check | where | what it refuses today |
|---|---|---|
| revision reachable from `origin/master` | `scripts/provision-aeon-ref.sh:104-110` (`git fetch` + `merge-base --is-ancestor`) | pinning to an unpublished revision |
| `aeon_dir_matches_the_provenance_tip` | `crates/sigil-harness/tests/provenance_chain.rs` (`git -C <aeon> rev-parse HEAD` vs the tip's `aeon_rev`) | freezing against a tree that is not the tip |
| `aeon_head_unmoved` | `crates/sigil-harness/src/bin/refreeze.rs` (~`:196-230`) | aeon HEAD moving *mid-freeze* |
| dirty-tree refusal | `refreeze.rs` (~`:157-172`) | freezing off uncommitted aeon edits |
| `CONTROL=required` rebuild control | `provision-aeon-ref.sh:92-98, :200-217` | a tree that is not the pinned revision |

**The last one is the sharp edge.** The control is *derived from the revision*, deliberately, so
no caller can opt out: at the pinned revision a rebuilt ROM must match the golden; at any other
revision the comparison is `not-applicable` and the CRCs print as data. Pin the corpus at the
same revision the goldens describe and the strongest single check in the provisioning path is
still armed — **but it now asserts a tautology**, because the thing it compares was frozen from
the thing it compares against. That is not a failure; it is the check quietly changing what it
means, which is the harder thing to notice.

Also note `refreeze.rs`'s dirty-tree refusal specifically **cannot** be relocated later: the
freeze's own build writes into the tree. And the pinned snapshot must be *writable and buildable*
(H-2, H-4), which means it is dirty by that check's own standard. **Nobody has stated whether a
built-in snapshot still counts as identifying.** Recorded as unknown #6.

*Closing fixture:* possible — vendor a content hash of the snapshot beside the `aeon_rev` and
assert the tree matches it. That is a **different claim** (this tree is what we vendored) from
the current one (this revision exists in aeon's published history), and the substitution should
be written down rather than made silently.

### H-8 — **the whole DRIFT-DETECTOR family dies, and it is the family the prediction belongs to.** ← not predicted as a family
Seven gates exist for no other reason than that the tree changes. Frozen, each is a tautology
from the day the snapshot is taken:

| gate | its subject, in its own words |
|---|---|
| `crates/sigil-harness/tests/act_fixture_drift.rs` | the `Act` fixture must track live `engine/structs.emp`; its header records the catch — `Act` gained two fields and the fixture was *"six bytes and two fields stale"*, consumed by 12 port gates |
| `crates/sigil-harness/tests/banked_carrier_drift.rs` | *"the next SFX id-range growth moves the derived head, leaves the literal stale, breaks the whole-ROM golden byte gate — and the natural remediation, a refreeze, blesses the WRONG blob"* |
| `crates/sigil-cli/tests/extra_entry.rs` (`every_aeon_fixture_this_file_names_still_resolves`) | *"An aeon parcel has renamed, moved or deleted it"* |
| `crates/sigil-cli/tests/game_contract_env_coverage.rs` | written *because* aeon added a `hook ring_collected`; its job is the next one |
| `crates/sigil-harness/tests/repin_pins.rs` | keeps half its teeth: a **sigil** placer change still moves a pin; an **aeon** source change no longer can |
| `crates/sigil-harness/tests/region_end_contracts.rs` | same split |
| `crates/sigil-cli/tests/seam2_layout_derivation.rs` | the literal-drift detector for aeon's `map.toml` |

**This is exactly the shape the prediction identified — `H-1c` is one member of this family, not
a special case.** The prediction was right about the *mechanism* (a property exercised only
because the corpus moves) and understated the *population*: it is not one encoding boundary, it
is every gate whose subject is change.

A second-order consequence worth pricing: `provision-aeon-ref.sh:245` nominates
`repin --check` → *"pins.rs unchanged"* as **the positive witness that a tree is the pinned
revision**. With a vendored snapshot that witness becomes vacuous, and the provisioning script
would then be telling its reader to trust a check that can no longer fail.

*Closing fixture:* **none of these can be closed synthetically** — a drift detector with a frozen
subject has nothing to detect. The honest move is to say so per gate: either delete it (its job
moved to the nightly), or keep it and mark it as an analyzer-regression gate rather than a drift
gate. Leaving seven gates green with no subject is the always-green trap by seven doors.

### H-9 — **a vendored source tree is REFUSED by every shell caller, and the whole d-17/d-18 resolver layer loses its subject.** ← not predicted
Two halves, verified here:

**(a) The resolver requires a real checkout.** `scripts/lib/suite_paths.sh:69-92`:

```bash
aeon)     printf '%s\n' build.sh engine ;;      # markers
…
[[ -e $dir/.git ]] || { printf 'no .git — that is not a checkout'; return 0; }
```

A directory holding only `engine/` and `games/` is **not** an aeon checkout by this definition,
and a set-but-wrong value is a hard error, not a fallthrough. Every shell caller passes through
here — `landing-run.sh`, both nightlies, `provision-aeon-ref.sh`, `capture_goldens.sh`,
`derive_offcanonical_sizes.sh`, and the Python twin `golden/ab/suite_paths.py`. `landing-run.sh`
separately requires **all four built ROMs present**, refusing unless `--scoped`, which stamps the
verdict PARTIAL. **So the pin very likely has to be a real, writable, buildable aeon checkout
rather than a vendored subdirectory** — which is what `provision-aeon-ref.sh` already produces.
That reframes step 1(a) from *"vendor sources"* to *"pin the revision the existing provisioner
already pins, and stop advancing it."*

**(b) The apparatus that exists to police an external tree stops having one.**
`reference_dependence_is_named.rs`, `bare_run_refuses.rs` (the d-18 refusal ruled 2026-09-02),
`reference_tree_named_write.rs` (d-17), `suite_paths_precedence.rs`, `scripts_name_their_tree.rs`
— five gates, plus the production layer behind them (`test_support.rs:756, :893, :931, :993,
:1236, :1265`) and `SIGIL_STRICT_GATE`, whose only job is turning an absent reference into a
failure. If the corpus is vendored inside sigil, that layer is dead code. **This is not a loss of
coverage — it is a loss of *purpose*, and dead gates that still pass are worse than deleted
ones.** Worth stating loudly because d-18 was ruled *four hours before this parcel was written*
and would be retired by it.

### H-10 — **the replacement observer's own expectation record freezes with the tree, and its dead branches stay green in its selftest.** ← not predicted; compounds H-3
Three coupled facts:

1. `scripts/drift-nightly.conf:41` points `DRIFT_RECORD_READER` at `drift_record.py`
   **inside the provisioned tree**; `:28-30` forbids the job authoring its own expectation
   (correctly — the expected values are aeon's to own). Freeze the tree and the aeon-owned record
   can only ever answer for one `aeon_rev`: **a one-row table**.
2. `scripts/drift_report.py` still counts chains in `(aeon_rev, sigil_closure_rev)` pairs and
   `drift-nightly.conf:14-19` still defines N as *"one entry per paired landing"*. With one aeon
   rev, **the unit of evidence stops describing what it counts.**
3. `drift_report.py:52-53`'s verdicts `V_UNVERIFIED_AEON_MOVED` and `V_UNATTRIBUTABLE` become
   unreachable in production — **while still passing in the selftest doubles** (~`:805-812`).
   The tool cannot report its own narrowing.

This is the same class as H-5 and it lands on the mechanism that is supposed to *replace* what
H-1c loses. Combined with H-3 (the timer is not installed at all), the compensating control for
step 1 is currently: not running, and — if it were — pointed at a record that can hold one row.

*Closing fixture:* possible and it is the natural pairing — the observer must provision at the
**live tip** (which `provision-aeon-ref.sh:92-98` already supports, `CONTROL=not-applicable`)
while the landing suite follows the pin. That is the design; it is not written down as a
requirement anywhere I found, and H-11 is what happens if it is left implicit.

### H-11 — **the nightly lane and the in-suite gate would silently measure different trees.** ← not predicted
`scripts/nightly_source_gates.sh` hardcodes `AEON_REF=master`. `crates/sigil-cli/tests/corpus_builds.rs`
takes its tree from `aeon_dir()`. `docs/OVERSEER.md:1251-1253` argues the brick family is
*"already covered without the coupling"* **because both build every shipped shape from live aeon
tip.** After step 1 the in-suite gate follows the pin and the lane keeps the tip — so the premise
of that argument silently stops holding, and two things that read as one control become two
controls measuring different corpora with no diagnostic saying so.
*Closing fixture:* not a fixture — a one-line statement at cutover of which tree each side gets,
and a correction to the `OVERSEER.md` paragraph whose argument depends on them being the same.

---

## 8. WHAT I COULD NOT CLASSIFY — explicit unknowns

1. **What "vendor a pinned aeon source snapshot" means concretely.** *Narrowed, not closed.*
   H-9(a) shows a bare source subdirectory is **refused** by `suite_paths.sh` (no `.git`, no
   `build.sh`) and by `landing-run.sh` (no built ROMs). So the pin is very likely a real detached
   aeon checkout whose revision stops advancing — i.e. what `provision-aeon-ref.sh` already
   builds. **That is an inference from what the tooling accepts, not a statement anyone has
   made**, and it changes almost every verdict above, so it must be stated by the vendoring half
   rather than inferred by its reader.
2. **Whether `tools/` comes with it.** Three reads need it, one of them (`convsym`) a compiled
   binary, and `tools/salvador` + `tools/gen_compression_vectors.py` are required to reconstruct
   `engine/debug/generated/compression_vectors.emp` at all. Not stated anywhere I found.
3. **Whether the goldens are re-frozen at the pin, and whether anyone has noticed what that does
   to the rebuild control.** See H-7. **Unknown whether it has been noticed** — I found no
   document raising it.
4. **What the `.emp` construct coverage differential actually is.** §1 shows the language
   frontend is 124/125 synthetic, which is strong evidence that construct coverage does not
   depend on the corpus — but *"a construct with a synthetic test"* and *"a construct whose
   interaction with real placement is only ever seen in the corpus"* are different claims and I
   did not separate them. **Stated as an unknown rather than resolved by the encouraging ratio.**
5. **Whether `[layout.provisional-drift]`, `check_object_bank_budget` against a real cursor, and
   `validate_sound_fold`'s firing are exercised by any run.** The 08-27 recheck listed all three
   as never exercised and built no ROM either. **Still unknown here** — this pass built nothing.
6. **Whether a built-in snapshot still counts as identifying.** `drift_report.py` treats only a
   `clean` tree state as identifying, and `refreeze.rs` refuses a dirty aeon tree — but the pin
   must be writable and buildable (H-2, H-4, H-9a), so it is dirty by that standard the moment it
   is used. **No document states the rule for the aeon side under a pin.**
7. **Per-row skip counts.** The population is 136 binaries / 91 artifact-naming, but I did not
   measure how many `#[test]` **rows** that is, and a binary-level count over-states or
   under-states the coverage loss depending on rows-per-binary. **Not measured; do not quote a
   row count from this file.**

**Two adjacent findings, recorded because they were in the sweep's path and are not corpus
coverage.** `scripts/systemd/sigil-ref-drift.service` and `sigil-source-gates.service` carry
literal `/home/volence` paths — the exact thing every script they fire forbids; and the
04:17/05:17/07:17 stagger documented in `scripts/systemd/README.md` exists because the lanes
contend for `git worktree` locks in the aeon repository, a rationale that evaporates under a pin
while the units keep firing on the old schedule.

---

## 9. THE COUNT

| | n |
|---|---|
| **Shapes enumerated** | 7 |
| — witnessed by the pinned corpus | 7 |
| — holed | 0 |
| **Placement constraints enumerated (R1–R9)** | 9 |
| — TRANSFERS | 6 |
| — pre-existing hole, unchanged by step 1 (R1b, R1d) | 2 |
| — retired (R5) | 1 |
| — unclassifiable by design (R2, R4) | 2 *(R2 also raises H-6)* |
| **Reference-tree read buckets enumerated (by path)** | 5 — 129 SOURCE / 81 BUILT / 3 TOOL / 9 SCRIPT / 7 OTHER distinct paths |
| — carried by a source snapshot | 1 bucket (SOURCE), 129 paths |
| — NOT carried | 4 buckets, ~100 paths |
| **Gate families enumerated (by what they assert)** | 11 (F1–F11) over the 136 binaries |
| — TRANSFERS | 3 families: F2, F5, F10 |
| — DEGRADES | 3 families: F1, F6, F7 |
| — DIES | 4 families: F3, F4, F8, F9 |
| — unaffected | 1 family: F11 |
| *(the per-family binary counts in §6.2 sum to 152, not 136: several binaries sit in two families — `m1b_gate` in F3 and F11, `provenance_chain` in F8 and F10, `extra_entry` in F7 and F8, `game_contract_env_coverage` in F5 and F8, `repin_pins`/`region_end_contracts` in F8. **The families are a classification, not a partition, and no binary total should be quoted from them.**)* | |
| **HOLES** | **11** (H-1 with three sub-holes, H-2 … H-11) |
| — predicted | **1** (H-1 — and it landed differently: the static decision is WITNESSED, §3.3; the dynamic crossing is the hole) |
| — **not predicted** | **10** (H-2 … H-11) |
| **Explicit unknowns** | 7 |
| **Corpus-dependent test binaries** | 127 by guard-string / 136 including the 9 that read the tree without one, of 339 (40%) |
| **of those, naming a BUILT artifact** | 91 |
| aeon `.emp` files in tree / added in last 30 days | 192 / **82 (43%)** |
| `provenance.toml` entries (refreezes) | 197 |

---

## 10. WHAT THIS SAYS ABOUT THE SEQUENCING

Not a ruling — this parcel changes nothing and decides nothing. Five orderings fall out of the
measurements, each with a named blocker:

1. **`FREEZE-THE-LISTINGS` (H-2) is a hard prerequisite**, not a nice-to-have: a missing listing
   fails open at `listing_symbol_addr` and surfaces as unrelated divergence.
2. **`DRIFT-TIMER-NOT-INSTALLED` (H-3) must be discharged BEFORE the cutover, and H-10 discharged
   with it — and the bar is a real observation in the ledger, not an installed unit.** The ledger
   holds nine lines, two selftests, four `COULD NOT RUN`, and **zero observations, ever**. A
   non-blocking observer that does not run converts a deferred cost into an absent control; one
   that runs against a frozen record converts it into a one-row table. Together they are the
   *whole* compensating control for a corpus that turns over 11 `.emp` files a week, and neither
   half has been shown to work once.
3. **The generated-artifact question (H-4) needs a stated answer before anything is vendored** —
   vendor-vs-regenerate is the difference between a self-referential expectation and a corpus
   that is not actually pinned.
4. **The shape of the pin (unknown #1 / H-9a) must be stated first**, because it decides whether
   items 1–3 are even the right questions. The tooling's own answer appears to be *"a detached
   aeon checkout that stops advancing"*, which is a much smaller change than "vendoring" implies
   — and which makes the real work the **61 F1 redirects** (§6.3), not a new corpus.
5. **R2's asl-parity scratch (H-6) wants to land before the pin, or after H-1a's fixture exists**
   — it is a deliberate abs-width emulation and §3 shows the abs boundary sits 26 bytes from live
   code.

And one that is smaller and sharper than all of them: **the `ABSW-CEILING-INVARIANT` row should
be corrected now** (§3.2). Its coordinate has rotted twice; its stated safe direction is
currently **backwards** for the plain shape; and it names the wrong binding symbol. The invariant
is real — the row's operational advice, followed literally today, points a parcel author at the
dangerous direction and calls it safe. That correction is one row edit, and it is not this
branch's to make.

---

## 11. THE ONE-LINE ANSWER TO THE GATE

**The pinned corpus is NOT shown to exercise what the live corpus exercised.** It transfers the
placement-constraint axis intact (9/9), the shape axis intact (7/7) and roughly half the gate
families verbatim — but **four families die and three degrade**, and the two mechanisms meant to
absorb that loss (the nightly observer, the drift record) are respectively **not installed — and
its ledger records zero observations in its lifetime** — and **structurally frozen by the same
change**. Eleven holes are open. **The vendoring half should not start.**
