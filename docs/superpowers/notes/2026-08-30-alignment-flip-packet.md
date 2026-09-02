# The R7 alignment flip — parcel packet (2026-08-30)

Branch `parcel/alignment-flip` (code, proven on its branch, NOT landed). Trial freeze on
`trial/alignment-flip-freeze` (disposable; never cherry-pick). Lands only through the paired
aeon+sigil freeze the aeon lane schedules.

Every number below was measured by a command listed in §8; nothing is computed from the
frozen tables.

## 0. Headline

* The packing walk now aligns every chained section to `section_align::DECLARED` through ONE
  function, `native::packed_chained_base(running, head_label)`, which `seam2::sound_layout`
  also calls with the same labels. `packed_align_of` (the largest of 16/8/4/2 dividing the
  frozen provisional base) and `seam2::frozen_prov` are deleted. The provisional bases keep
  their anchor / round-0 / drift-report / `derive_frozen_table` roles (scope point 3: untouched).
* Measured effect, full file CRC32/size, pre → post (post built against the trial aeon rev,
  see §3):

  | shape | pre (tip) | post | Δ size |
  |---|---|---|---|
  | s4 | 5a25a0d4/719355 | 0f0153cb/719293 | −62 |
  | s4_debug | 1131b2bf/736357 | 512b42a4/736311 | −46 |
  | demo | aca8c043/96476 | 30a31d81/96458 | −18 |
  | demo_debug | 932f496f/101359 | 51056291/101323 | −36 |
  | config_a | a7ed4e81/736725 | ebbe8e04/736663 | −62 |
  | config_b | dc997981/611439 | 4c52b46a/611301 | −138 |
  | lean | 454a546c/674830 | c347e317/674816 | −14 |

* **Two findings the brief did not predict, both load-bearing for the landing:**
  1. **The sound-off shapes do not build against aeon's shipped maps post-flip.** Both
     `[[hole]] after = "Z80_IdleProgram" at = 0x3F8` rows transcribe a MEASURED boundary
     that the idle's inferred quantum (16, from its 0x3D0 pin) produced; declared WORD the
     idle packs flush at 0x3C8 (plain) / 0x3CC (demo debug), `boot_tail` lands at
     0x3F0 / 0x3F4, and `validate_placement` refuses `[map.hole-interior-occupied]` on
     `demo`, `demo debug`, `config_b`. The landing needs `at = 0x3F0` in BOTH
     `games/demo/map.toml` and `games/sonic4/map.toml` — and the row can no longer name one
     resume address for both sound-off shapes (0x3F0 plain vs 0x3F4 debug); 0x3F0 is the
     lower bound. The aeon lane owns that row's semantics. See §3.
  2. **The `abs.w` ceiling falsifier fired in `s4` (and `lean`).** `Sound_PlaySFX` moved
     from 0x8054 to **0x7FBC** — below 0x8000 — so absolute references to it (and to
     `ps_checkfull`/`ps_drop`) re-encode as `abs.w`. The relaxation fixpoint handled it
     (the ROM builds, all gates green), but the −62 is not pure pad removal. See §4.

## 1. What the flip is (as landed on the branch)

`crates/sigil-harness/src/native.rs`:
* `packed_chained_base(running: u32, head_label: &str) -> Result<u32, String>` — rounds
  `running` up to `required_for(head_label).required`; `None` is a
  `[layout.undeclared-alignment]` refusal naming the label. The ONLY alignment input.
* `packed_true_bases`, `Some(r) =>` arm (pinned, chained): `packed_chained_base(r, head)`.
  The zero-byte-marker cap-at-2 is deleted (design point 2, below).
* `packed_true_bases`, label-less/unpinned arm: `Some(r) =>` contiguity is now rounded to the
  declaration when the section HAS a head label (the 21 sections no frozen table names);
  a label-less blob still packs flush. Byte-neutral on every shipped shape by construction —
  those bases were already even, or `validate_resolved_alignment` would have refused them
  before this parcel — and it removes the one path where a head-labelled section escaped
  its declaration. (Unit witness `an_unpinned_section_packs_to_its_declaration_too`.)
* `validate_declared_alignment(sections)` — pre-walk, scope = every ROM section with a head
  label, pinned or not: refuses undeclared by name, in one report. No longer reads `prov`.
* `validate_resolved_alignment` — unchanged logic; doc rewritten.
* `validate_sound_fold` — unchanged logic, always-on; its message now quotes the DECLARED
  quantum instead of a frozen-table residue.
* `seam2::sound_layout` — `mt_bank_lma = packed_chained_base(.., "Song_MovingTrucks")?`,
  `sfx_bank_lma_{plain,debug} = packed_chained_base(.., "Sfx_33")?`; `frozen_prov` deleted;
  seam2 no longer reads any frozen table.
* Doc comments (native.rs packer doc + K5 comment, section_align.rs module doc + row
  comments, seam2.rs, both test files) rewritten to present-tense function.

Commits: `7919e3ee` (the flip + tests), `ff18afa6` (fixture needles follow the label
constants), plus this packet's commit.

## 2. Design decisions (brief points 1–4)

1. **Undeclared = refusal in the walk itself.** `packed_chained_base` errs; the walk `?`s it.
   Also refused pre-walk by `validate_declared_alignment` so a build names EVERY undeclared
   section at once instead of the first reached. Never 1/2/16/pass. Witnesses:
   `an_undeclared_head_label_is_refused_by_the_walk_itself` (walk),
   `a_section_with_no_declaration_is_refused_by_name_before_the_walk` (pre-walk),
   `packed_chained_base_rounds_to_the_declared_alignment_only` (function).
2. **Zero-byte-marker cap-at-2: deleted.** The cap existed so `EndOfRom` would not inherit a
   wide INFERRED quantum. Its declaration says 2 for exactly that reason (row comment in
   `section_align.rs`); keeping a second rule would let the cap silently override a
   declaration. Witness: `the_zero_byte_terminus_packs_to_the_image_end_by_declaration`
   (0x12-byte head → terminus at 0x1012, not the 16-aligned pin 0x1020). On every shape
   `EndOfRom` post-flip equals the last section's end (see §5 tables).
3. **Scope: provisional bases' other roles untouched.** The walk still reads `prov` for the
   run head, `is_anchor_gap(p)` islands, phase-bank orgs, round-0 `measure_pinned` pins and
   `[layout.provisional-drift]`; `derive_frozen_table` untouched. The only thing that
   stopped reading `prov` is the alignment. Not BLOCKED.
4. **Pre-walk half post-flip** asserts DECLARATION COMPLETENESS (every head-labelled ROM
   section has a row), and no longer `required | prov` — post-flip that would be a check
   on a stale cache against a requirement the walk already satisfies by construction, and
   it would refuse the first build after any declaration is tightened (chicken-and-egg
   with the refreeze). Existing witnesses, disposition:
   * `a_pinned_section_with_no_declaration_is_refused_by_name` → kept as
     `a_section_with_no_declaration_is_refused_by_name_before_the_walk` (+ asserts no
     "inferred" quantum is quoted).
   * `a_pin_that_violates_the_declaration_is_refused_with_the_residue` → REPLACED: the
     same section/residue is now proven to PASS the pre-walk half and be refused by the
     resolved half (`a_resolved_base_that_violates_the_declaration_is_refused`, which also
     took over the source/residue-naming assertions).
   * `the_bank_window_requirement_is_checked_beyond_the_mod_16_cap` → REPLACED by
     `the_bank_window_requirement_is_measured_on_the_resolved_anchor` (the $8000 rows are
     enforced on anchors only by the resolved half; there is no cap any more).
   * `an_unpinned_section_is_skipped_before_the_walk_and_measured_after_it` → INVERTED:
     `an_unpinned_section_needs_a_declaration_and_is_measured_after_the_walk`.
   * `every_faulting_section_is_named_in_one_report` → kept, over two undeclared sections.
   * NEW `a_label_less_blob_is_out_of_both_halves_scope`.
   * Integration: `a_repin_that_breaks_a_declared_alignment_is_refused_by_name` → INVERTED
     into `a_doctored_pin_residue_does_not_move_a_packed_section` (§6).
   * `the_requirements_above_the_cap_exceed_what_the_inference_can_express` → the
     `packed_align_of` assertion is gone; kept as
     `the_requirements_above_16_are_declared_for_the_anchored_sections`.
   * Walk fixtures (`derived_layout_tests`, the two packer-rank probes) use REAL `DECLARED`
     labels via `L_*` constants (the walk refuses a synthetic one); the boundary test's
     derived expectation moved from `align4(code+6)` to `align2(code+6)` = 0x8016 — it still
     discriminates the abs.w arithmetic (0x8014).

## 3. The aeon-side map finding (REQUIRED for the landing)

Post-flip, with aeon at the pinned `25731dfa`, the seven-shape gate reports:

```
shape `demo plain`:  [map.hole-interior-occupied] the hole declared after `Z80_IdleProgram` — interior [0x3C8,0x3F8), reserved for `engine.z80_init` — is occupied at [0x3F0,0x3F8) by byte-emitting section `boot_tail` (head `BootData_PostBlob`) …
shape `demo debug`:  … interior [0x3CC,0x3F8) … occupied at [0x3F4,0x3F8) by `boot_tail` …
shape `config_b`:    … interior [0x3C8,0x3F8) … occupied at [0x3F0,0x3F8) by `boot_tail` …
```

Cause, from the pre-flip listings and frozen tables: `boot_head` ends at 0x3C8 (0x3CC in
demo debug); `Z80_IdleProgram`'s pin 0x3D0 inferred 16, so the idle sat at 0x3D0..0x3F8
(40 B) and `boot_tail` at 0x3F8 — the number both maps' `at` rows transcribe ("MEASURED
0x3D0..0x3F8"). Declared WORD, the idle packs at 0x3C8/0x3CC, ends at 0x3F0/0x3F4, and
`boot_tail` follows. Nothing in aeon consumes 0x3F8 as an absolute (boot_data.emp: "the
post-hole content resumes at the right address purely through contiguous packing — no
absolute org"); the row is layout data, and the flip re-measures it.

For the measurement and the trial freeze the ref tree carries a throwaway local aeon commit
(`75044465`, branch `trial/alignment-flip-hole` inside `/home/volence/sonic_hacks/.aeon-ref-r7`,
NEVER pushed) with exactly this diff:

```
games/demo/map.toml:    at = 0x3F8  ->  at = 0x3F0  (comment: 0x3F0 plain / 0x3F4 debug; lower bound)
games/sonic4/map.toml:  at = 0x3F8  ->  at = 0x3F0  (and the MEASURED comment 0x3C8..0x3F0)
```

plus a second trial commit (`5823ea77`) re-stamping the two shape-keyed fixture cuts
`build.sh` gates on build-fatally — `tools/fixtures/sprite_tilt_cut.json` (six address
fields; `sprite_tilt_gate.py --emit-fixture`, precedent e1f412ed / chain 181) and
`tools/fixtures/instashield_cut.json` (`instashield_gate.py --write-fixture`; the plain cut is
64 → 62 bytes because of §4's `abs.w` re-encoding, not a routine change). Without both,
`build.sh sonic4` exits 1 (the ROM is already written; the gate is after it) and
`capture_goldens.sh` aborts under `set -e`. After both: `build.sh` exit 0 on plain and
DEBUG=1 with CRCs identical to the measurement (fixtures carry no ROM bytes).

**Open for the aeon lane:** the hole's `at` was implicitly shape-invariant only because the
inferred quantum made it so. With WORD packing the resume address differs between demo plain
and demo debug (0x3F0 vs 0x3F4). `hole_interior_faults` only uses `at` as the interior's
right edge, so the lower bound is correct for the check; whether the row should become
per-shape, or the idle should carry a wider declared alignment WITH A SOURCE (there is none in
`z80_init.emp` — the 68k copies it byte-wise), is aeon's call. Do not "fix" it by declaring
16 in `section_align.rs`: that is the packing accident this table forbids.

## 4. The abs.w ceiling falsifier

`grep -nE 'SoundTablesZ80_Head|Sound_PlaySFX' <shape>.lst`, pre → post:

| shape | SoundTablesZ80_Head | Sound_PlaySFX | note |
|---|---|---|---|
| s4 | 8000 → 8000 (VMA; LMA anchor 0xA0000) | **8054 → 7FBC** | crossed below 0x8000; `ps_ret` at exactly 0x8000 |
| s4_debug | 8000 → 8000 | AF8A → AF0A | above |
| config_a | 8000 → 8000 | B09A → AFE2 | above |
| lean | 8000 → 8000 | **8054 → 7FBC** | crossed |
| demo / demo_debug / config_b | absent (sound off) | absent | — |

The brief's hypothesis was that `SoundTablesZ80_Head` "sits at 0x8000 exactly with zero
margin": the 8000 in the listing is its **VMA** (phase bank `vma: $8000`); its LMA is the
`sound_bank` anchor 0xA0000 and cannot move. The symbol with zero margin was `Sound_PlaySFX`,
84 bytes above the ceiling in `s4`/`lean`, and the flip's −0x98 shift of everything upstream
of it took it under. Consequence: every `jsr Sound_PlaySFX` / `lea`/`pea` of those three
labels in `s4`/`lean` is now a 4-byte `abs.w` form where it was 6-byte `abs.l`; the walk's
fixpoint converged (no `island classification changed` / width-flip refusal), and the ROM
passes every in-build gate, but the s4 size delta is pad removal PLUS those encodings.
Byte proof (`measure-pre/s4.bin` vs `measure-post/s4.bin`): the insta-shield's tail
`jmp Sound_PlaySFX` is `4EF9 0000 8054` pre and `4EF8 7FBC` post; the whole ROM holds 9
`jsr (Sound_PlaySFX).l` sites pre (`4EB9 0000 8054`) and 9 `jsr (Sound_PlaySFX).w` post
(`4EB8 7FBC`); the routine's own `bne` displacements shrank by 2 to match. Aeon's
`instashield_gate` saw it as its cut going 64 → 62 bytes with OPCODE bytes differing (§7).
Runtime confirmation is TAGGED for the controller (no emulator here): a sound-on boot of the
post-flip `s4.bin` that plays an SFX exercises the re-encoded calls.

## 5. Per-section base tables, pre → post

Summary (moved = head label's address changed, which includes downstream ripple; the
histogram is over moved sections: declared quantum vs the quantum a residue-of-pin reading
gave that section in that shape's frozen table):

| shape | moved / declared-in-listing | largest single move | own quantum 16→2 | 8→2 | 4→2 | unchanged quantum (2→2) | 16→8 (Sfx_33) |
|---|---|---|---|---|---|---|---|
| s4 | 81 / 89 | −152 (BgAnim_Init) | 40 | 4 | 6 | 7 | 1 |
| s4_debug | 91 / 100 | −136 (HeightMaps) | 46 | 9 | 9 | 3 | 1 |
| demo | 37 / 47 | −132 (BgAnim_Init) | 21 | 3 | 3 | 2 | — |
| demo_debug | 38 / 48 | −126 (BgAnim_Init) | 18 | 5 | 6 | 1 | — |
| config_a | 94 / 102 | −198 (Sound_DebugMirror) | 53 | 7 | 7 | 3 | 1 |
| config_b | 77 / 85 | −124 (replay_fixture align) | 41 | 5 | 3 | 5 | 1 |
| lean | 81 / 89 | −152 (BgAnim_Init) | 39 | 4 | 6 | 8 | 1 |

(+23 unpinned head-labelled sections per sonic4 shape, 8 per demo shape, moved by ripple.)
`EndOfRom`: s4 0xA5C90→0xA5C82, s4_debug 0xA7F40→0xA7F34, demo 0x1121C→0x1121A (both),
config_a 0xA7F40→0xA7F34, config_b 0x8B9B0→0x8B934, lean 0xA4C0E→0xA4C00. The bank anchors
(0x90000/0xA0000) and `ObjCodeBase` 0x10000 did not move in any shape. The sound blobs:
`Song_MovingTrucks` 0xA0630 unmoved (the cursor was already 8-aligned), `Sfx_33` 0xA3B20→
0xA3B18 (plain) and 0xA5570→0xA5568 (debug) — the one 16→8 row, and `validate_sound_fold`
agreed with the walk on all four sound-on shapes (the seven-shape gate builds through it).

The full per-shape tables follow in §9 (generated by `compare-layouts.py`, §8).

## 6. Red-first evidence

* **Integration `a_doctored_pin_residue_does_not_move_a_packed_section`** — run against the
  PRE-flip harness (sources checked out from `db0a28d8`, test file from the branch):
  `redfirst-integration-preflip.log`: `FAILED … a pin's residue is not an alignment input;
  the build must go through, got: [layout.undeclared-alignment] 1 section(s): … sfx_bank_blob
  … frozen provisional base is 0xa3b24 … (base % 8 = 4)`. Post-flip: `ok`, `Sfx_33` at the
  control base and `built.rom == control.rom`.
* **Sabotage A** (`packed_chained_base` ignores the declaration, rounds to 16, undeclared →
  `Ok(running)`): 7 red — `packed_chained_base_rounds_to_the_declared_alignment_only`,
  `an_undeclared_head_label_is_refused_by_the_walk_itself`,
  `an_unpinned_section_packs_to_its_declaration_too`,
  `the_walk_packs_to_the_declared_alignment_not_the_pin_residue`,
  `the_zero_byte_terminus_packs_to_the_image_end_by_declaration`,
  `base_dependent_length_reproduces_provisional_bases_at_frozen_sizes`,
  `growth_across_the_boundary_places_the_successor_from_the_long_form`. Restored;
  `git status` clean.
* **Sabotage B** (`validate_declared_alignment` returns `Ok(())`): exactly 3 red —
  `a_section_with_no_declaration_is_refused_by_name_before_the_walk`,
  `every_faulting_section_is_named_in_one_report`,
  `an_unpinned_section_needs_a_declaration_and_is_measured_after_the_walk`. Restored.
* Post-flip green: `cargo test -p sigil-harness --lib` 188/0/0;
  `section_alignment_declared` 3/0 against the trial aeon rev (2/1 against the pinned rev,
  the red being §3's `[map.hole-interior-occupied]`, not an alignment fault).
* Runners: all of the above live in `cargo test -p sigil-harness --lib` and
  `cargo test -p sigil-cli --test section_alignment_declared`, both inside
  `scripts/landing-run.sh`'s `--workspace` run.
* Not red-first-proven here: `validate_sound_fold`'s FIRING (its logic is untouched; only its
  message changed). No witness makes it fire today — ledgered (§10).

## 7. Landing runs, classification, trial freeze

All four through `scripts/landing-run.sh --baseline 4176 --aeon /home/volence/sonic_hacks/.aeon-ref-r7
--target /home/volence/sonic_hacks/.sigil-r7-target` (SIGIL_STRICT_GATE=1, full workspace,
`--nocapture`), each log stamped with pwd / sigil HEAD+branch / aeon HEAD+branch / target dir,
and `--expect-test` naming this parcel's own tests (all executed in every run). Wall clock
per run: 3 min 25 s – 3 min 40 s.

| # | tree | passed | failed | ignored | skip | exit | note |
|---|---|---|---|---|---|---|---|
| 1 | `parcel/alignment-flip` @ 4bcbea9d | 4014 | 168 | 2 | 0 | 1 | 158 unique names, ALL bucket A |
| 2 | `trial/alignment-flip-freeze` @ c601a7fc (freeze) | 4178 | 4 | 2 | 0 | 1 | 4 hand sites the freeze cannot write |
| 3 | trial @ b59a0699 (rebased on 223612b2 + 3 literal sites) | 4181 | 1 | 2 | 0 | 1 | the hand baseline's other 14 literals |
| 4 | **trial @ 396df3a0** | **4182** | **0** | **2** | **0** | **0** | **GREEN** |

Reference tree in every run: `.aeon-ref-r7 @ 5823ea77 (trial/alignment-flip-hole, clean)`,
all four ROMs present (post-flip, built by `build.sh` exit 0 after the fixture re-stamps).

**Run 1 classification (the parcel branch, pre-freeze).** 168 FAILED lines, 158 unique names
(some run in more than one binary). Every one is bucket **A** — its subject is a frozen
artifact: 113 `*_region_matches_reference` / `*_matches_reference` /
`*_undoctored_compile_equals_the_reference_window` / `*_regions_match_reference` (reference
windows read at pinned addresses); and the 45 read one by one:
`*_anchor_matches_golden` ×7, `*_full_file` ×7, `*_size_table_rederives_native` ×7
(committed tables), `native_full_sonic4_*` ×2 / `native_rom_*` ×2 / `declared_chain_*` ×2 /
`flipped_config_a_anchor_matches_golden` / `doctored_golden_at_deform_pointer_is_caught` /
`deform_pointer_equals_placed_label_vma` / `deb2_appendix_negative_controls` /
`a_passing_extra_entry_moves_no_bytes` / `both_spellings_of_the_section_row_build_the_same_rom`
/ `doctored_af_delete_produces_different_bytes` / `a_doctored_indexed_mode_changes_the_bytes`
(assembled length or a control window against the golden), `pins_rs_is_current` (267 changed
pins), `aeon_dir_matches_the_provenance_tip` (aeon 5823ea77 vs the tip's 25731dfa),
`config_b_frozen_placement_exact` / `config_b_boot_data_hole_filled` /
`config_b_doctored_size_table_breaks_the_build` (frozen table / golden window / t24 — see
below), `s4_boot_data_blob_present` / `demo_*_game_modules_match_golden` /
`objdefs_match_reference_*` / `colinked_sfx_head_matches_the_reference_rom_slice_both_shapes` /
`two_module_*_flip_*` ×4 (golden regions), `sound_layout_derives_the_frozen_addresses` (hand
literals), `every_region_end_contract_holds_against_the_live_layout` (repin.toml allotments
that carry no pad post-flip). **Bucket B: 0.**

**Trial freeze** (`refreeze --freeze alignment-flip-trial --ab "<packet §5>" --note "TRIAL …"`,
AEON_DIR = the ref tree at 5823ea77, journalled, exit 0): appended provenance entry #194
`alignment-flip-trial`, aeon_rev 5823ea77; its seven `full_crc/full_size` EQUAL the §0 table
(config_a ebbe8e04/736663, config_b 4c52b46a/611301, demo 30a31d81/96458, demo_debug
51056291/101323, lean c347e317/674816, s4 0f0153cb/719293, s4_debug 512b42a4/736311);
`refreeze --check`: OK, chain len 194. The freeze regenerated the 7 goldens, the 7 size
tables, and `pins.rs` (267 pins moved — that diff IS the per-section byte accounting, §9 is
the same data from the listings).

**What the freeze does NOT regenerate, all hand-edited on the trial branch (the landing's
5-site ripple, enumerated):**
1. `crates/sigil-harness/repin.toml` — four regions declared `end_measures = "allotment"`
   and carry no pad in any shape post-flip (`every_region_end_contract_holds…` ratchet):
   `entity_window`, `children`, `dust_spindash` → `end = "section:<name>"`; `objdefs` →
   `len = 0x34` (its section is named `text`, which the resolved layout holds twice, so
   `section:text` is refused as ambiguous; the content is two 26-byte ObjDef records in
   every shape). `pins.rs` re-derived after: only `DUST_SPINDASH.len` 0x220 → 0x102 (its
   own extent; the old window swept `dust_puff` (has its own region) and `ring_sparkle`
   (no region — pre-existing, ledgered). Nothing consumes that length.
2. `crates/sigil-cli/tests/seam2_layout_derivation.rs` — `sfx_bank_lma_plain` 0xA3B20 →
   0xA3B18, `sfx_bank_lma_debug` 0xA5570 → 0xA5568.
3. `crates/sigil-cli/tests/boot_data_port.rs` — the config_b hole literals `$3d0..$3f8` →
   `$3c8..$3f0`, window end `$406` → `$3fe`.
4. `crates/sigil-harness/tests/repin_pins.rs` — the hand-typed baseline: BOOT_DATA.plain
   0x3A0 → 0x398, ANIMATE/RINGS/… bases, ASSEMBLED_LEN 0xA5C90 → 0xA5C82,
   DEBUG_ASSEMBLED_LEN 0xA7F40 → 0xA7F34 (14 literals, each tagged with the one reason).
5. `crates/sigil-cli/tests/native_full_rom.rs` — `Ground_Move_Cap` plain offset
   `P_STATE_GROUND.plain + 0x2F8` → `+ 0x2F4`: the two `jsr Sound_PlaySFX` sites ahead of it
   in `player_ground` encode `abs.w` in the plain shape (§4); debug unchanged (its
   Sound_PlaySFX is above $8000). Verified: exactly 2 `4EB9 0000 8054` sites in
   `[PState_Ground, Ground_Move_Cap)` pre-flip.

**One test changed for SEMANTICS, on the parcel branch (commit 223612b2), not as a pin:**
`config_b_doctored_size_table_breaks_the_build` arm (a) demanded that doctoring `ObjCodeBase`'s
frozen row +2 MOVE the ROM ("the island anchor authority is load-bearing"). Post-flip it
cannot: a doctored row is no map anchor, so the object bank packs as a chained section to
its DECLARED 0x10000 alignment and lands at 0x10000 regardless — byte-identical (run 2
proved it: "left the ROM byte-identical"). The arm now asserts byte-identity and the test is
`config_b_doctored_size_table_moves_no_bytes`; the anchor's own authority is witnessed by
`growth_into_a_declared_anchor_still_fails_loud` and
`stale_provisional_gap_is_an_island_only_when_declared`. This is the frozen table losing one
more placement role, which is the direction the decouple is going; it is recorded here so
the aeon lane's step-2/3 sequencing knows the object bank's island row is now redundant with
the map anchor + the declaration.

**Reconciliation.** `git grep -o -e '#\[test\]' HEAD -- '*.rs' | wc -l` = 4183 on the branch
(4178 at master; the +5 are this parcel's net new tests: −1 replaced, +2 in
`declared_alignment_tests`, +4 in `derived_layout_tests`). Every run executed
passed+failed+ignored = **4184**, run 1 through run 4 identically. The attr census is an
approximate model of the executed population, not an exact one — measured on this log:
`tests/freeze_step_gap.rs` carries 12 attrs and executes 21 tests (macro-instantiated),
`tests/strict_census_lint.rs` 7 attrs / 5 executed (cfg-gated), the harness lib 204 attrs /
188 executed. The tip's attested run (4176+2 = 4178) ran at sigil 06936bff, whose attr count is
4177 — the same +1 executed-over-attrs, before this parcel; master (db0a28d8) is 4178
attrs after one more test landed. So: +6 executed and +6 attrs from the tip to here, +5 of
them this parcel's. The harness lib's executed count pre-flip vs
post-flip sources by `cargo test -- --list` is 183 → 188 (+5, exactly the attrs added).

## 8. Exact commands (re-derivable by the aeon lane)

```
# target dir + binaries (control = master db0a28d8)
export CARGO_TARGET_DIR=/home/volence/sonic_hacks/.sigil-r7-target
cargo build --release --bin sigil --bin emit_sound_blob

# exclusive reference tree at the provenance tip's aeon_rev (25731dfa…)
AEON_REPO=/home/volence/sonic_hacks/aeon SIGIL_BIN=$CARGO_TARGET_DIR/release/sigil \
  REF_TARGET=$CARGO_TARGET_DIR scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-ref-r7
#   -> REBUILD CONTROL s4.bin 5a25a0d4/719355 MATCHES THE GOLDEN; s4.debug.bin 1131b2bf/736357 MATCHES
AEON_DIR=/home/volence/sonic_hacks/.aeon-ref-r7 SIGIL_EMIT=$CARGO_TARGET_DIR/release/emit_sound_blob \
  cargo run --release -p sigil-harness --bin repin -- --check      # -> "pins.rs unchanged"

# seven shapes, pre and post (script text in §8a): rm -f ROM+lst, build, stale-guard, copy out
AEON_DIR=… SIGIL_BUILD=$CARGO_TARGET_DIR/release/sigil SIGIL_EMIT=$CARGO_TARGET_DIR/release/emit_sound_blob \
  measure-shapes.sh $CARGO_TARGET_DIR/measure-pre     # at db0a28d8, aeon 25731dfa
cargo build --release --bin sigil --bin emit_sound_blob   # at ff18afa6
  measure-shapes.sh $CARGO_TARGET_DIR/measure-post    # aeon trial 75044465 (§3)
python3 compare-layouts.py measure-pre measure-post .   # §5/§9

# aeon-side, in the ref tree, both required by build.sh post-flip:
#   games/{demo,sonic4}/map.toml  [[hole]] at = 0x3F0
#   python3 tools/sprite_tilt_gate.py --lst s4.lst --rom s4.bin --emit-fixture tools/fixtures/sprite_tilt_cut.json
#   python3 tools/sprite_tilt_gate.py --lst s4.debug.lst --rom s4.debug.bin --emit-fixture tools/fixtures/sprite_tilt_cut.json
```

### 8a. measure-shapes.sh (as run)

Canonical shapes: `rm -f $ROM $LST; env [DEBUG=1] NO_LINT=1 SIGIL_BUILD=… SIGIL_EMIT=… ./build.sh <sonic4|demo>`;
off-canonical: `rm -f …; sigil build --aeon . --native --config-a -o s4.debug.bin --emit-lst s4.debug.lst`
(`--config-b` / `--lean` → `s4.bin`/`s4.lst`), each ROM+listing copied out BEFORE the next
build (config_b and lean both write `s4.lst`); a `mktemp` marker proves each artifact is
newer than the build's start; then `rm -f` the four canonical artifacts and rebuild
`sonic4` plain + `DEBUG=1`. CRC32+size via `zlib.crc32`.

## 9. Per-section base tables (all seven shapes)

Generated by `compare-layouts.py` (§8) from the pre/post listings, the DECLARED rows, and each
shape's pre-flip frozen table. "moved" includes downstream ripple; the last column is the
quantum a residue-of-pin reading gave that section in that shape's frozen table.


### s4: pre 5a25a0d4/719355  post 0f0153cb/719293  size delta -62
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x3a0 | 0x398 | -8 **moved** | 2 | 16 |
| BootData_PostBlob | 0x1bf0 | 0x1be2 | -14 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x1c00 | 0x1bf0 | -16 **moved** | 2 | 16 |
| Init_DMA_Queue | 0x1c3a | 0x1c2a | -16 **moved** | 2 | 2 |
| Init_SpriteTable | 0x1f70 | 0x1f56 | -26 **moved** | 2 | 16 |
| VBlank_Handler | 0x2250 | 0x222e | -34 **moved** | 2 | 16 |
| HBlank_Install | 0x2430 | 0x240e | -34 **moved** | 2 | 16 |
| Read_Controllers | 0x2460 | 0x243e | -34 **moved** | 2 | 16 |
| GameLoop | 0x256e | 0x254c | -34 **moved** | 2 | 2 |
| Input_Tick | 0x2590 | 0x256e | -34 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0x26e0 | 0x26b6 | -42 **moved** | 2 | 16 |
| ZX0R_Decompress | 0x27d8 | 0x27ae | -42 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x2850 | 0x2826 | -42 **moved** | 2 | 16 |
| Perform_DPLC | 0x2c48 | 0x2c1c | -44 **moved** | 2 | 8 |
| InitObjectRAM | 0x2cf0 | 0x2cc0 | -48 **moved** | 2 | 16 |
| InitSpriteSystem | 0x2ff0 | 0x2fb8 | -56 **moved** | 2 | 16 |
| AnimateSprite | 0x3410 | 0x33d2 | -62 **moved** | 2 | 16 |
| TouchResponse | 0x35a4 | 0x3566 | -62 **moved** | 2 | 4 |
| RingBuffer_Add | 0x37a4 | 0x3766 | -62 **moved** | 2 | 4 |
| Collected_Init | 0x3964 | 0x3924 | -64 **moved** | 2 | 4 |
| PopulateSpawnedPieceCount | 0x4260 | 0x4212 | -78 **moved** | 2 | 16 |
| Load_Object | 0x4550 | 0x44fe | -82 **moved** | 2 | 16 |
| Plane_Buffer_Reset | 0x45d8 | 0x4586 | -82 **moved** | 2 | 8 |
| Tile_Cache_GetTile | 0x4900 | 0x48a0 | -96 **moved** | 2 | 16 |
| Collision_GetType | 0x5790 | 0x572e | -98 **moved** | 2 | 16 |
| Collision_ProbeDown | 0x5800 | 0x5796 | -106 **moved** | 2 | 16 |
| Section_Init | 0x5cf4 | 0x5c8a | -106 **moved** | 2 | 4 |
| Camera_Init | 0x6160 | 0x60ea | -118 **moved** | 2 | 16 |
| Parallax_Init | 0x6330 | 0x62b2 | -126 **moved** | 2 | 16 |
| Raster_Install | 0x6c6e | 0x6bf0 | -126 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x6fd2 | 0x6f54 | -126 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x7456 | 0x73d8 | -126 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x7520 | 0x749a | -134 **moved** | 2 | 16 |
| PageIn_Process | 0x75d8 | 0x7552 | -134 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x78c4 | 0x783e | -134 **moved** | 2 | (unpinned) |
| BG_Init | 0x7db0 | 0x7d24 | -140 **moved** | 2 | 16 |
| BgAnim_Init | 0x7e90 | 0x7df8 | -152 **moved** | 2 | 16 |
| Sound_PostByte | 0x7f2e | 0x7e96 | -152 **moved** | 2 | 2 |
| SoundTablesZ80_Head | 0x8000 | 0x8000 | +0 | 32768 | 16 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| Player_Init | 0x10002 | 0x10002 | +0 | 2 | 2 |
| PState_Ground | 0x10690 | 0x10686 | -10 **moved** | 2 | 16 |
| PState_Air | 0x10b20 | 0x10b10 | -16 **moved** | 2 | 16 |
| PState_Spindash | 0x10e70 | 0x10e5a | -22 **moved** | 2 | 16 |
| PState_Fly | 0x10f10 | 0x10ef6 | -26 **moved** | 2 | (unpinned) |
| PState_Glide | 0x11044 | 0x11028 | -28 **moved** | 2 | (unpinned) |
| Climb_WallDist | 0x1131a | 0x112fa | -32 **moved** | 2 | (unpinned) |
| Ability_InstaShield | 0x11620 | 0x115fe | -34 **moved** | 2 | (unpinned) |
| CharDef_Sonic | 0x11e80 | 0x11e50 | -48 **moved** | 2 | 16 |
| CharDef_Tails | 0x11ec0 | 0x11e86 | -58 **moved** | 2 | 16 |
| CharDef_Knuckles | 0x11ef6 | 0x11ebc | -58 **moved** | 2 | (unpinned) |
| CharacterDefs | 0x11f30 | 0x11ef2 | -62 **moved** | 2 | 16 |
| TailsAppendage_Refresh | 0x11f7a | 0x11f3c | -62 **moved** | 2 | 2 |
| DustPuff_Spawn | 0x12096 | 0x12058 | -62 **moved** | 2 | (unpinned) |
| Dust_Tick | 0x120dc | 0x1209e | -62 **moved** | 2 | (unpinned) |
| RingSparkle_Spawn | 0x121e0 | 0x121a2 | -62 **moved** | 2 | (unpinned) |
| TestStatic_Main | 0x12300 | 0x122be | -66 **moved** | 2 | 16 |
| TestSolid_Init | 0x12310 | 0x122c2 | -78 **moved** | 2 | 16 |
| ObjDef_PathSwap | 0x12322 | 0x122d4 | -78 **moved** | 2 | 2 |
| DeformTable_Zero | 0x123b4 | 0x12366 | -78 **moved** | 2 | 4 |
| EditorSceneBinding_OJZ_Act1_Sec0 | 0x13148 | 0x130fa | -78 **moved** | 2 | (unpinned) |
| OJZ_TestRaster | 0x1329a | 0x1324c | -78 **moved** | 2 | (unpinned) |
| ObjDef_Static | 0x137f0 | 0x1379a | -86 **moved** | 2 | 16 |
| OJZ_Sec0_TypeTable | 0x13828 | 0x137ce | -90 **moved** | 2 | 8 |
| OJZ_Act_Pool_Page0 | 0x13998 | 0x1393e | -90 **moved** | 2 | 8 |
| OJZ_Act1_Descriptor | 0x168a4 | 0x1684a | -90 **moved** | 2 | 4 |
| OJZ_Sec0_Blocks | 0x16b20 | 0x16ac4 | -92 **moved** | 2 | 16 |
| OJZ_Sec0_LocalMap | 0x22130 | 0x220ce | -98 **moved** | 2 | 16 |
| OJZ_Palette | 0x22e00 | 0x22d92 | -110 **moved** | 2 | 16 |
| BgAnim_Table | 0x27682 | 0x27614 | -110 **moved** | 2 | 2 |
| Map_TestObj | 0x296b0 | 0x29642 | -110 **moved** | 2 | 16 |
| Map_DustSpindash | 0x296e0 | 0x29672 | -110 **moved** | 2 | (unpinned) |
| Ani_Sonic | 0x2a2c0 | 0x2a24c | -116 **moved** | 2 | 16 |
| Ani_Tails | 0x2a3ca | 0x2a356 | -116 **moved** | 2 | 2 |
| Ani_Knuckles | 0x2a586 | 0x2a512 | -116 **moved** | 2 | (unpinned) |
| Ani_DustSpindash | 0x2a6f2 | 0x2a67e | -116 **moved** | 2 | (unpinned) |
| Map_Tails | 0x2a710 | 0x2a692 | -126 **moved** | 2 | 16 |
| Map_Knuckles | 0x4b66e | 0x4b5f0 | -126 **moved** | 2 | (unpinned) |
| HeightMaps | 0x6dd40 | 0x6dcb8 | -136 **moved** | 2 | 16 |
| Dac_Temp_Blip | 0x90000 | 0x90000 | +0 | 32768 | 16 |
| Song_MovingTrucks | 0xa0630 | 0xa0630 | +0 | 8 | 16 |
| Sfx_33 | 0xa3b20 | 0xa3b18 | -8 **moved** | 8 | 16 |
| GameState_OJZScroll_Init | 0xa4410 | 0xa4404 | -12 **moved** | 2 | 16 |
| __align$games.sonic4.replay_fixture$0 | 0xa4980 | 0xa4972 | -14 **moved** | 2 | (unpinned) |
| BusError | 0xa4be0 | 0xa4bd2 | -14 **moved** | 2 | 16 |
| EndOfRom | 0xa5c90 | 0xa5c82 | -14 **moved** | 2 | 16 |

sections moved: 81 of 89; largest single move: -152 (BgAnim_Init)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 23
  declared  2 vs residue          2: 7
  declared  2 vs residue          4: 6
  declared  2 vs residue          8: 4
  declared  2 vs residue         16: 40
  declared  8 vs residue         16: 1
  SoundTablesZ80_Head: pre 0x8000 post 0x8000
  Sound_PlaySFX: pre 0x8054 post 0x7fbc
  EndOfRom: pre 0xa5c90 post 0xa5c82

### s4_debug: pre 1131b2bf/736357  post 512b42a4/736311  size delta -46
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x3a0 | 0x3a0 | +0 | 2 | 16 |
| BootData_PostBlob | 0x1c70 | 0x1c6c | -4 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x1c7e | 0x1c7a | -4 **moved** | 2 | 2 |
| Init_DMA_Queue | 0x1d10 | 0x1d0a | -6 **moved** | 2 | 16 |
| Init_SpriteTable | 0x2048 | 0x2042 | -6 **moved** | 2 | 8 |
| VBlank_Handler | 0x2320 | 0x231a | -6 **moved** | 2 | 16 |
| HBlank_Install | 0x2510 | 0x2506 | -10 **moved** | 2 | 16 |
| Read_Controllers | 0x2540 | 0x2536 | -10 **moved** | 2 | 16 |
| GameLoop | 0x2650 | 0x2644 | -12 **moved** | 2 | 16 |
| Input_Tick | 0x2674 | 0x2668 | -12 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0x2870 | 0x2860 | -16 **moved** | 2 | 16 |
| ZX0R_Decompress | 0x2a70 | 0x2a60 | -16 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x2af0 | 0x2ad8 | -24 **moved** | 2 | 16 |
| Perform_DPLC | 0x2ee8 | 0x2ece | -26 **moved** | 2 | 8 |
| InitObjectRAM | 0x2f90 | 0x2f72 | -30 **moved** | 2 | 16 |
| InitSpriteSystem | 0x36e0 | 0x36b2 | -46 **moved** | 2 | 16 |
| AnimateSprite | 0x3c14 | 0x3be6 | -46 **moved** | 2 | 4 |
| TouchResponse | 0x3ecc | 0x3e9e | -46 **moved** | 2 | 4 |
| RingBuffer_Add | 0x40d4 | 0x40a6 | -46 **moved** | 2 | 4 |
| Collected_Init | 0x42f8 | 0x42ca | -46 **moved** | 2 | 8 |
| PopulateSpawnedPieceCount | 0x5060 | 0x5026 | -58 **moved** | 2 | 16 |
| Load_Object | 0x5400 | 0x53c2 | -62 **moved** | 2 | 16 |
| Plane_Buffer_Reset | 0x5488 | 0x544a | -62 **moved** | 2 | 8 |
| Tile_Cache_GetTile | 0x58b0 | 0x586e | -66 **moved** | 2 | 16 |
| Collision_GetType | 0x69b0 | 0x6964 | -76 **moved** | 2 | 16 |
| Collision_ProbeDown | 0x6a20 | 0x69cc | -84 **moved** | 2 | 16 |
| Section_Init | 0x6f14 | 0x6ec0 | -84 **moved** | 2 | 4 |
| Camera_Init | 0x7490 | 0x7438 | -88 **moved** | 2 | 16 |
| Parallax_Init | 0x7670 | 0x760a | -102 **moved** | 2 | 16 |
| SoundTablesZ80_Head | 0x8000 | 0x8000 | +0 | 32768 | 16 |
| Raster_Install | 0x8042 | 0x7fdc | -102 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x83a6 | 0x8340 | -102 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x882a | 0x87c4 | -102 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x88f0 | 0x8886 | -106 **moved** | 2 | 16 |
| PageIn_Process | 0x89a8 | 0x893e | -106 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x8e04 | 0x8d9a | -106 **moved** | 2 | (unpinned) |
| BG_Init | 0x9c80 | 0x9c12 | -110 **moved** | 2 | 16 |
| BgAnim_Init | 0x9dc0 | 0x9d46 | -122 **moved** | 2 | 16 |
| CompressionSelfTest | 0x9f18 | 0x9e9e | -122 **moved** | 2 | 8 |
| Sound_PostByte | 0xad00 | 0xac80 | -128 **moved** | 2 | 16 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| Player_Init | 0x10002 | 0x10002 | +0 | 2 | 2 |
| PState_Ground | 0x107a0 | 0x1079e | -2 **moved** | 2 | 16 |
| PState_Air | 0x10c30 | 0x10c28 | -8 **moved** | 2 | 16 |
| PState_Spindash | 0x10f80 | 0x10f72 | -14 **moved** | 2 | 16 |
| PState_Fly | 0x1101c | 0x1100e | -14 **moved** | 2 | (unpinned) |
| PState_Glide | 0x11150 | 0x11142 | -14 **moved** | 2 | (unpinned) |
| Climb_WallDist | 0x11426 | 0x11418 | -14 **moved** | 2 | (unpinned) |
| Ability_InstaShield | 0x1172c | 0x1171e | -14 **moved** | 2 | (unpinned) |
| CharDef_Sonic | 0x11f80 | 0x11f72 | -14 **moved** | 2 | 16 |
| CharDef_Tails | 0x11fc0 | 0x11fa8 | -24 **moved** | 2 | 16 |
| CharDef_Knuckles | 0x11ff6 | 0x11fde | -24 **moved** | 2 | (unpinned) |
| CharacterDefs | 0x12030 | 0x12014 | -28 **moved** | 2 | 16 |
| TailsAppendage_Refresh | 0x120e0 | 0x120c4 | -28 **moved** | 2 | 16 |
| DustPuff_Spawn | 0x12254 | 0x12238 | -28 **moved** | 2 | (unpinned) |
| Dust_Tick | 0x1229a | 0x1227e | -28 **moved** | 2 | (unpinned) |
| RingSparkle_Spawn | 0x1239e | 0x12382 | -28 **moved** | 2 | (unpinned) |
| TestStatic_Main | 0x124c0 | 0x1249e | -34 **moved** | 2 | 16 |
| TestAnimated | 0x124d0 | 0x124a2 | -46 **moved** | 2 | 16 |
| TestPlayer | 0x12530 | 0x12502 | -46 **moved** | 2 | 16 |
| TestEnemy_Init | 0x127c4 | 0x12796 | -46 **moved** | 2 | 4 |
| TestSolid_Init | 0x1280c | 0x127de | -46 **moved** | 2 | 4 |
| TestParticle | 0x12820 | 0x127f0 | -48 **moved** | 2 | 16 |
| TestEmitter | 0x12878 | 0x12848 | -48 **moved** | 2 | 8 |
| TestChildPart | 0x128d6 | 0x128a6 | -48 **moved** | 2 | 2 |
| TestStressEmitter | 0x12a10 | 0x129dc | -52 **moved** | 2 | 16 |
| TestChurnObj | 0x12a70 | 0x12a3a | -54 **moved** | 2 | 16 |
| ObjDef_PathSwap | 0x12aec | 0x12ab6 | -54 **moved** | 2 | 4 |
| DeformTable_Zero | 0x12be8 | 0x12bb0 | -56 **moved** | 2 | 8 |
| EditorSceneBinding_OJZ_Act1_Sec0 | 0x1397c | 0x13944 | -56 **moved** | 2 | (unpinned) |
| OJZ_TestRaster | 0x13ace | 0x13a96 | -56 **moved** | 2 | (unpinned) |
| ObjDef_Static | 0x14090 | 0x14052 | -62 **moved** | 2 | 16 |
| OJZ_Sec0_TypeTable | 0x140d0 | 0x14086 | -74 **moved** | 2 | 16 |
| OJZ_Act_Pool_Page0 | 0x14240 | 0x141f6 | -74 **moved** | 2 | 16 |
| OJZ_Act1_Descriptor | 0x17150 | 0x17102 | -78 **moved** | 2 | 16 |
| OJZ_Sec0_Blocks | 0x173d0 | 0x1737c | -84 **moved** | 2 | 16 |
| OJZ_Sec0_LocalMap | 0x229dc | 0x22986 | -86 **moved** | 2 | 4 |
| OJZ_Palette | 0x236a0 | 0x2364a | -86 **moved** | 2 | 16 |
| BgAnim_Table | 0x27f30 | 0x27ecc | -100 **moved** | 2 | 16 |
| Map_TestObj | 0x29f5e | 0x29efa | -100 **moved** | 2 | 2 |
| Map_DustSpindash | 0x29f8e | 0x29f2a | -100 **moved** | 2 | (unpinned) |
| Ani_Sonic | 0x2ab68 | 0x2ab04 | -100 **moved** | 2 | 8 |
| Ani_Tails | 0x2ac80 | 0x2ac0e | -114 **moved** | 2 | 16 |
| Ani_Knuckles | 0x2ae3c | 0x2adca | -114 **moved** | 2 | (unpinned) |
| Ani_Particle | 0x2afa8 | 0x2af36 | -114 **moved** | 2 | 8 |
| Ani_DustSpindash | 0x2afb0 | 0x2af3e | -114 **moved** | 2 | (unpinned) |
| Map_Tails | 0x2afd0 | 0x2af52 | -126 **moved** | 2 | 16 |
| Map_Knuckles | 0x4bf2e | 0x4beb0 | -126 **moved** | 2 | (unpinned) |
| HeightMaps | 0x6e600 | 0x6e578 | -136 **moved** | 2 | 16 |
| Dac_Temp_Blip | 0x90000 | 0x90000 | +0 | 32768 | 16 |
| Song_MovingTrucks | 0xa0630 | 0xa0630 | +0 | 8 | 16 |
| Sfx_33 | 0xa5570 | 0xa5568 | -8 **moved** | 8 | 16 |
| GameState_ObjectTest_Init | 0xa5e60 | 0xa5e54 | -12 **moved** | 2 | 16 |
| GameState_OJZScroll_Init | 0xa61e4 | 0xa61d8 | -12 **moved** | 2 | 4 |
| __align$games.sonic4.replay_fixture$0 | 0xa6c30 | 0xa6c24 | -12 **moved** | 2 | (unpinned) |
| BusError | 0xa6e90 | 0xa6e84 | -12 **moved** | 2 | 16 |
| EndOfRom | 0xa7f40 | 0xa7f34 | -12 **moved** | 2 | 16 |

sections moved: 91 of 100; largest single move: -136 (HeightMaps)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 23
  declared  2 vs residue          2: 3
  declared  2 vs residue          4: 9
  declared  2 vs residue          8: 9
  declared  2 vs residue         16: 46
  declared  8 vs residue         16: 1
  SoundTablesZ80_Head: pre 0x8000 post 0x8000
  Sound_PlaySFX: pre 0xaf8a post 0xaf0a
  EndOfRom: pre 0xa7f40 post 0xa7f34

### demo: pre aca8c043/96476  post 30a31d81/96458  size delta -18
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Z80_IdleProgram | 0x0 | 0x0 | +0 | 2 | 16 |
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x392 | 0x392 | +0 | 2 | 2 |
| BootData_PostBlob | 0x3f8 | 0x3f0 | -8 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x408 | 0x3fe | -10 **moved** | 2 | 8 |
| Init_DMA_Queue | 0x442 | 0x438 | -10 **moved** | 2 | 2 |
| Init_SpriteTable | 0x770 | 0x764 | -12 **moved** | 2 | 16 |
| VBlank_Handler | 0xa50 | 0xa3c | -20 **moved** | 2 | 16 |
| HBlank_Install | 0xbe0 | 0xbca | -22 **moved** | 2 | 16 |
| Read_Controllers | 0xc10 | 0xbfa | -22 **moved** | 2 | 16 |
| GameLoop | 0xd20 | 0xd08 | -24 **moved** | 2 | 16 |
| Input_Tick | 0xd3e | 0xd26 | -24 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0xe90 | 0xe6e | -34 **moved** | 2 | 16 |
| ZX0R_Decompress | 0xf88 | 0xf66 | -34 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x1000 | 0xfde | -34 **moved** | 2 | 16 |
| Perform_DPLC | 0x1400 | 0x13d4 | -44 **moved** | 2 | 16 |
| InitObjectRAM | 0x14a4 | 0x1478 | -44 **moved** | 2 | 4 |
| InitSpriteSystem | 0x17a0 | 0x1770 | -48 **moved** | 2 | 16 |
| AnimateSprite | 0x1bba | 0x1b8a | -48 **moved** | 2 | 2 |
| TouchResponse | 0x1d50 | 0x1d1a | -54 **moved** | 2 | 16 |
| RingBuffer_Add | 0x1f50 | 0x1f1a | -54 **moved** | 2 | 16 |
| Collected_Init | 0x2110 | 0x20ce | -66 **moved** | 2 | 16 |
| PopulateSpawnedPieceCount | 0x2a00 | 0x29bc | -68 **moved** | 2 | 16 |
| Load_Object | 0x2cf0 | 0x2ca8 | -72 **moved** | 2 | 16 |
| Plane_Buffer_Reset | 0x2d78 | 0x2d30 | -72 **moved** | 2 | 8 |
| Tile_Cache_GetTile | 0x30a0 | 0x304a | -86 **moved** | 2 | 16 |
| Collision_GetType | 0x3f30 | 0x3ed0 | -96 **moved** | 2 | 16 |
| Section_Init | 0x3f98 | 0x3f38 | -96 **moved** | 2 | 8 |
| Camera_Init | 0x43d0 | 0x436e | -98 **moved** | 2 | 16 |
| Parallax_Init | 0x4580 | 0x4518 | -104 **moved** | 2 | 16 |
| Raster_Install | 0x495a | 0x48f2 | -104 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x4c94 | 0x4c2c | -104 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x5118 | 0x50b0 | -104 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x51e0 | 0x5172 | -110 **moved** | 2 | 16 |
| PageIn_Process | 0x5298 | 0x522a | -110 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x557c | 0x550e | -110 **moved** | 2 | (unpinned) |
| BG_Init | 0x5a70 | 0x59f8 | -120 **moved** | 2 | 16 |
| BgAnim_Init | 0x5b50 | 0x5acc | -132 **moved** | 2 | 16 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| DemoBox_Main | 0x10002 | 0x10002 | +0 | 2 | 2 |
| ObjDef_DemoBox | 0x10006 | 0x10006 | +0 | 2 | 2 |
| BgAnim_Table | 0x100fe | 0x100fe | +0 | 2 | (unpinned) |
| GameState_Demo_Init | 0x10100 | 0x10100 | +0 | 2 | 16 |
| BusError | 0x1016c | 0x1016a | -2 **moved** | 2 | 4 |
| EndOfRom | 0x1121c | 0x1121a | -2 **moved** | 2 | 4 |

sections moved: 37 of 47; largest single move: -132 (BgAnim_Init)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 8
  declared  2 vs residue          2: 2
  declared  2 vs residue          4: 3
  declared  2 vs residue          8: 3
  declared  2 vs residue         16: 21
  SoundTablesZ80_Head: pre 0x0 post 0x0
  Sound_PlaySFX: pre 0x0 post 0x0
  EndOfRom: pre 0x1121c post 0x1121a

### demo_debug: pre 932f496f/101359  post 51056291/101323  size delta -36
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Z80_IdleProgram | 0x0 | 0x0 | +0 | 2 | 16 |
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x396 | 0x396 | +0 | 2 | 2 |
| BootData_PostBlob | 0x3f8 | 0x3f4 | -4 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x410 | 0x402 | -14 **moved** | 2 | 16 |
| Init_DMA_Queue | 0x4a0 | 0x492 | -14 **moved** | 2 | 16 |
| Init_SpriteTable | 0x7e0 | 0x7ca | -22 **moved** | 2 | 16 |
| VBlank_Handler | 0xac0 | 0xaa2 | -30 **moved** | 2 | 16 |
| HBlank_Install | 0xc60 | 0xc3c | -36 **moved** | 2 | 16 |
| Read_Controllers | 0xc90 | 0xc6c | -36 **moved** | 2 | 16 |
| GameLoop | 0xda0 | 0xd7a | -38 **moved** | 2 | 16 |
| Input_Tick | 0xdbe | 0xd98 | -38 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0xfc0 | 0xf90 | -48 **moved** | 2 | 16 |
| ZX0R_Decompress | 0x11c0 | 0x1190 | -48 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x1240 | 0x1208 | -56 **moved** | 2 | 16 |
| Perform_DPLC | 0x1638 | 0x15fe | -58 **moved** | 2 | 8 |
| InitObjectRAM | 0x16dc | 0x16a2 | -58 **moved** | 2 | 4 |
| InitSpriteSystem | 0x1e20 | 0x1de2 | -62 **moved** | 2 | 16 |
| AnimateSprite | 0x2354 | 0x2316 | -62 **moved** | 2 | 4 |
| TouchResponse | 0x2608 | 0x25ca | -62 **moved** | 2 | 8 |
| RingBuffer_Add | 0x2810 | 0x27d2 | -62 **moved** | 2 | 16 |
| Collected_Init | 0x2a2a | 0x29ec | -62 **moved** | 2 | 2 |
| PopulateSpawnedPieceCount | 0x3790 | 0x3748 | -72 **moved** | 2 | 16 |
| Load_Object | 0x3b2c | 0x3ae4 | -72 **moved** | 2 | 4 |
| Plane_Buffer_Reset | 0x3bb4 | 0x3b6c | -72 **moved** | 2 | 4 |
| Tile_Cache_GetTile | 0x3fe0 | 0x3f90 | -80 **moved** | 2 | 16 |
| Collision_GetType | 0x50d0 | 0x507c | -84 **moved** | 2 | 16 |
| Section_Init | 0x5138 | 0x50e4 | -84 **moved** | 2 | 8 |
| Camera_Init | 0x5690 | 0x5632 | -94 **moved** | 2 | 16 |
| Parallax_Init | 0x5848 | 0x57e6 | -98 **moved** | 2 | 8 |
| Raster_Install | 0x5cb6 | 0x5c54 | -98 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x5ff0 | 0x5f8e | -98 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x6474 | 0x6412 | -98 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x6540 | 0x64d4 | -108 **moved** | 2 | 16 |
| PageIn_Process | 0x65f8 | 0x658c | -108 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x6a4e | 0x69e2 | -108 **moved** | 2 | (unpinned) |
| BG_Init | 0x78d0 | 0x785e | -114 **moved** | 2 | 16 |
| BgAnim_Init | 0x7a10 | 0x7992 | -126 **moved** | 2 | 16 |
| CompressionSelfTest | 0x7b68 | 0x7aea | -126 **moved** | 2 | 8 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| DemoBox_Main | 0x10002 | 0x10002 | +0 | 2 | 2 |
| ObjDef_DemoBox | 0x10006 | 0x10006 | +0 | 2 | 2 |
| BgAnim_Table | 0x100fe | 0x100fe | +0 | 2 | (unpinned) |
| GameState_Demo_Init | 0x10100 | 0x10100 | +0 | 2 | 16 |
| BusError | 0x1016c | 0x1016a | -2 **moved** | 2 | 4 |
| EndOfRom | 0x1121c | 0x1121a | -2 **moved** | 2 | 4 |

sections moved: 38 of 48; largest single move: -126 (BgAnim_Init)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 8
  declared  2 vs residue          2: 1
  declared  2 vs residue          4: 6
  declared  2 vs residue          8: 5
  declared  2 vs residue         16: 18
  SoundTablesZ80_Head: pre 0x0 post 0x0
  Sound_PlaySFX: pre 0x0 post 0x0
  EndOfRom: pre 0x1121c post 0x1121a

### config_a: pre a7ed4e81/736725  post ebbe8e04/736663  size delta -62
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x3b0 | 0x3a6 | -10 **moved** | 2 | 16 |
| BootData_PostBlob | 0x1c80 | 0x1c72 | -14 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x1c90 | 0x1c80 | -16 **moved** | 2 | 16 |
| Init_DMA_Queue | 0x1d20 | 0x1d10 | -16 **moved** | 2 | 16 |
| Init_SpriteTable | 0x2060 | 0x2048 | -24 **moved** | 2 | 16 |
| VBlank_Handler | 0x2340 | 0x2320 | -32 **moved** | 2 | 16 |
| HBlank_Install | 0x2540 | 0x2512 | -46 **moved** | 2 | 16 |
| Read_Controllers | 0x2570 | 0x2542 | -46 **moved** | 2 | 16 |
| GameLoop | 0x267e | 0x2650 | -46 **moved** | 2 | 2 |
| Input_Tick | 0x26a8 | 0x267a | -46 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0x28a0 | 0x2872 | -46 **moved** | 2 | 16 |
| ZX0R_Decompress | 0x2aa0 | 0x2a72 | -46 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x2b20 | 0x2aea | -54 **moved** | 2 | 16 |
| Perform_DPLC | 0x2f18 | 0x2ee0 | -56 **moved** | 2 | 8 |
| InitObjectRAM | 0x2fc0 | 0x2f84 | -60 **moved** | 2 | 16 |
| InitSpriteSystem | 0x3710 | 0x36c4 | -76 **moved** | 2 | 16 |
| AnimateSprite | 0x3c50 | 0x3bf8 | -88 **moved** | 2 | 16 |
| TouchResponse | 0x3f08 | 0x3eb0 | -88 **moved** | 2 | 8 |
| RingBuffer_Add | 0x4110 | 0x40b8 | -88 **moved** | 2 | 16 |
| Collected_Init | 0x4340 | 0x42dc | -100 **moved** | 2 | 16 |
| PopulateSpawnedPieceCount | 0x50a0 | 0x5038 | -104 **moved** | 2 | 16 |
| Load_Object | 0x543c | 0x53d4 | -104 **moved** | 2 | 4 |
| Plane_Buffer_Reset | 0x54c4 | 0x545c | -104 **moved** | 2 | 4 |
| Tile_Cache_GetTile | 0x58f0 | 0x5880 | -112 **moved** | 2 | 16 |
| Collision_GetType | 0x69f0 | 0x6976 | -122 **moved** | 2 | 16 |
| Collision_ProbeDown | 0x6a58 | 0x69de | -122 **moved** | 2 | 8 |
| Debug_MusicToggle | 0x6f50 | 0x6ed2 | -126 **moved** | 2 | 16 |
| Section_Init | 0x7020 | 0x6f98 | -136 **moved** | 2 | 16 |
| Camera_Init | 0x75a0 | 0x7510 | -144 **moved** | 2 | 16 |
| Parallax_Init | 0x7780 | 0x76e2 | -158 **moved** | 2 | 16 |
| SoundTablesZ80_Head | 0x8000 | 0x8000 | +0 | 32768 | 16 |
| Raster_Install | 0x8152 | 0x80b4 | -158 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x84b6 | 0x8418 | -158 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x893a | 0x889c | -158 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x8a00 | 0x895e | -162 **moved** | 2 | 16 |
| PageIn_Process | 0x8ab8 | 0x8a16 | -162 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x8f14 | 0x8e72 | -162 **moved** | 2 | (unpinned) |
| BG_Init | 0x9d90 | 0x9cea | -166 **moved** | 2 | 16 |
| BgAnim_Init | 0x9ed0 | 0x9e1e | -178 **moved** | 2 | 16 |
| CompressionSelfTest | 0xa028 | 0x9f76 | -178 **moved** | 2 | 8 |
| Sound_PostByte | 0xae10 | 0xad58 | -184 **moved** | 2 | 16 |
| Sound_DebugMirror | 0xb270 | 0xb1aa | -198 **moved** | 2 | 16 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| Player_Init | 0x10002 | 0x10002 | +0 | 2 | 2 |
| PState_Ground | 0x107a0 | 0x1079e | -2 **moved** | 2 | 16 |
| PState_Air | 0x10c30 | 0x10c28 | -8 **moved** | 2 | 16 |
| PState_Spindash | 0x10f80 | 0x10f72 | -14 **moved** | 2 | 16 |
| PState_Fly | 0x1101c | 0x1100e | -14 **moved** | 2 | (unpinned) |
| PState_Glide | 0x11150 | 0x11142 | -14 **moved** | 2 | (unpinned) |
| Climb_WallDist | 0x11426 | 0x11418 | -14 **moved** | 2 | (unpinned) |
| Ability_InstaShield | 0x1172c | 0x1171e | -14 **moved** | 2 | (unpinned) |
| CharDef_Sonic | 0x11f80 | 0x11f72 | -14 **moved** | 2 | 16 |
| CharDef_Tails | 0x11fc0 | 0x11fa8 | -24 **moved** | 2 | 16 |
| CharDef_Knuckles | 0x11ff6 | 0x11fde | -24 **moved** | 2 | (unpinned) |
| CharacterDefs | 0x12030 | 0x12014 | -28 **moved** | 2 | 16 |
| TailsAppendage_Refresh | 0x120e0 | 0x120c4 | -28 **moved** | 2 | 16 |
| DustPuff_Spawn | 0x12254 | 0x12238 | -28 **moved** | 2 | (unpinned) |
| Dust_Tick | 0x1229a | 0x1227e | -28 **moved** | 2 | (unpinned) |
| RingSparkle_Spawn | 0x1239e | 0x12382 | -28 **moved** | 2 | (unpinned) |
| TestStatic_Main | 0x124c0 | 0x1249e | -34 **moved** | 2 | 16 |
| TestAnimated | 0x124c4 | 0x124a2 | -34 **moved** | 2 | 4 |
| TestPlayer | 0x12524 | 0x12502 | -34 **moved** | 2 | 4 |
| TestEnemy_Init | 0x127b8 | 0x12796 | -34 **moved** | 2 | 8 |
| TestSolid_Init | 0x12800 | 0x127de | -34 **moved** | 2 | 16 |
| TestParticle | 0x12820 | 0x127f0 | -48 **moved** | 2 | 16 |
| TestEmitter | 0x12878 | 0x12848 | -48 **moved** | 2 | 8 |
| TestChildPart | 0x128d6 | 0x128a6 | -48 **moved** | 2 | 2 |
| TestStressEmitter | 0x12a10 | 0x129dc | -52 **moved** | 2 | 16 |
| TestChurnObj | 0x12a70 | 0x12a3a | -54 **moved** | 2 | 16 |
| ObjDef_PathSwap | 0x12aec | 0x12ab6 | -54 **moved** | 2 | 4 |
| DeformTable_Zero | 0x12bf0 | 0x12bb0 | -64 **moved** | 2 | 16 |
| EditorSceneBinding_OJZ_Act1_Sec0 | 0x13984 | 0x13944 | -64 **moved** | 2 | (unpinned) |
| OJZ_TestRaster | 0x13ad6 | 0x13a96 | -64 **moved** | 2 | (unpinned) |
| ObjDef_Static | 0x140a0 | 0x14052 | -78 **moved** | 2 | 16 |
| OJZ_Sec0_TypeTable | 0x140e0 | 0x14086 | -90 **moved** | 2 | 16 |
| OJZ_Act_Pool_Page0 | 0x14250 | 0x141f6 | -90 **moved** | 2 | 16 |
| OJZ_Act1_Descriptor | 0x17160 | 0x17102 | -94 **moved** | 2 | 16 |
| OJZ_Sec0_Blocks | 0x173e0 | 0x1737c | -100 **moved** | 2 | 16 |
| OJZ_Sec0_LocalMap | 0x229ec | 0x22986 | -102 **moved** | 2 | 4 |
| OJZ_Palette | 0x236b0 | 0x2364a | -102 **moved** | 2 | 16 |
| BgAnim_Table | 0x27f40 | 0x27ecc | -116 **moved** | 2 | 16 |
| Map_TestObj | 0x29f6e | 0x29efa | -116 **moved** | 2 | 2 |
| Map_DustSpindash | 0x29f9e | 0x29f2a | -116 **moved** | 2 | (unpinned) |
| Ani_Sonic | 0x2ab78 | 0x2ab04 | -116 **moved** | 2 | 8 |
| Ani_Tails | 0x2ac90 | 0x2ac0e | -130 **moved** | 2 | 16 |
| Ani_Knuckles | 0x2ae4c | 0x2adca | -130 **moved** | 2 | (unpinned) |
| Ani_Particle | 0x2afc0 | 0x2af36 | -138 **moved** | 2 | 16 |
| Ani_DustSpindash | 0x2afc8 | 0x2af3e | -138 **moved** | 2 | (unpinned) |
| Map_Tails | 0x2afe0 | 0x2af52 | -142 **moved** | 2 | 16 |
| Map_Knuckles | 0x4bf3e | 0x4beb0 | -142 **moved** | 2 | (unpinned) |
| HeightMaps | 0x6e610 | 0x6e578 | -152 **moved** | 2 | 16 |
| Dac_Temp_Blip | 0x90000 | 0x90000 | +0 | 32768 | 16 |
| Song_MovingTrucks | 0xa0630 | 0xa0630 | +0 | 8 | 16 |
| Sfx_33 | 0xa5570 | 0xa5568 | -8 **moved** | 8 | 16 |
| GameState_ObjectTest_Init | 0xa5e60 | 0xa5e54 | -12 **moved** | 2 | 16 |
| GameState_OJZScroll_Init | 0xa61e4 | 0xa61d8 | -12 **moved** | 2 | 4 |
| __align$games.sonic4.replay_fixture$0 | 0xa6c30 | 0xa6c24 | -12 **moved** | 2 | (unpinned) |
| BusError | 0xa6e90 | 0xa6e84 | -12 **moved** | 2 | 16 |
| EndOfRom | 0xa7f40 | 0xa7f34 | -12 **moved** | 2 | 16 |

sections moved: 94 of 102; largest single move: -198 (Sound_DebugMirror)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 23
  declared  2 vs residue          2: 3
  declared  2 vs residue          4: 7
  declared  2 vs residue          8: 7
  declared  2 vs residue         16: 53
  declared  8 vs residue         16: 1
  SoundTablesZ80_Head: pre 0x8000 post 0x8000
  Sound_PlaySFX: pre 0xb09a post 0xafe2
  EndOfRom: pre 0xa7f40 post 0xa7f34

### config_b: pre dc997981/611439  post 4c52b46a/611301  size delta -138
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Z80_IdleProgram | 0x0 | 0x0 | +0 | 2 | 16 |
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x392 | 0x392 | +0 | 2 | 2 |
| BootData_PostBlob | 0x3f8 | 0x3f0 | -8 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x408 | 0x3fe | -10 **moved** | 2 | 8 |
| Init_DMA_Queue | 0x442 | 0x438 | -10 **moved** | 2 | 2 |
| Init_SpriteTable | 0x770 | 0x764 | -12 **moved** | 2 | 16 |
| VBlank_Handler | 0xa50 | 0xa3c | -20 **moved** | 2 | 16 |
| HBlank_Install | 0xbe0 | 0xbc8 | -24 **moved** | 2 | 16 |
| Read_Controllers | 0xc10 | 0xbf8 | -24 **moved** | 2 | 16 |
| GameLoop | 0xd20 | 0xd06 | -26 **moved** | 2 | 16 |
| Input_Tick | 0xd3e | 0xd24 | -26 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0xe90 | 0xe6c | -36 **moved** | 2 | 16 |
| ZX0R_Decompress | 0xf88 | 0xf64 | -36 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x1000 | 0xfdc | -36 **moved** | 2 | 16 |
| Perform_DPLC | 0x1400 | 0x13d2 | -46 **moved** | 2 | 16 |
| InitObjectRAM | 0x14a4 | 0x1476 | -46 **moved** | 2 | 4 |
| InitSpriteSystem | 0x17a0 | 0x176e | -50 **moved** | 2 | 16 |
| AnimateSprite | 0x1bba | 0x1b88 | -50 **moved** | 2 | 2 |
| TouchResponse | 0x1d50 | 0x1d18 | -56 **moved** | 2 | 16 |
| RingBuffer_Add | 0x1f50 | 0x1f18 | -56 **moved** | 2 | 16 |
| Collected_Init | 0x2110 | 0x20d2 | -62 **moved** | 2 | 16 |
| PopulateSpawnedPieceCount | 0x2a00 | 0x29c0 | -64 **moved** | 2 | 16 |
| Load_Object | 0x2cf0 | 0x2cac | -68 **moved** | 2 | 16 |
| Plane_Buffer_Reset | 0x2d78 | 0x2d34 | -68 **moved** | 2 | 8 |
| Tile_Cache_GetTile | 0x30a0 | 0x304e | -82 **moved** | 2 | 16 |
| Collision_GetType | 0x3f30 | 0x3edc | -84 **moved** | 2 | 16 |
| Collision_ProbeDown | 0x3f98 | 0x3f44 | -84 **moved** | 2 | 8 |
| Section_Init | 0x4490 | 0x4438 | -88 **moved** | 2 | 16 |
| Camera_Init | 0x48c8 | 0x486e | -90 **moved** | 2 | 8 |
| Parallax_Init | 0x4a90 | 0x4a36 | -90 **moved** | 2 | 16 |
| Raster_Install | 0x53ce | 0x5374 | -90 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x5732 | 0x56d8 | -90 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x5bb6 | 0x5b5c | -90 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x5c80 | 0x5c1e | -98 **moved** | 2 | 16 |
| PageIn_Process | 0x5d38 | 0x5cd6 | -98 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x6024 | 0x5fc2 | -98 **moved** | 2 | (unpinned) |
| BG_Init | 0x6510 | 0x64a8 | -104 **moved** | 2 | 16 |
| BgAnim_Init | 0x65f0 | 0x657c | -116 **moved** | 2 | 16 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| Player_Init | 0x10002 | 0x10002 | +0 | 2 | 2 |
| PState_Ground | 0x10680 | 0x10680 | +0 | 2 | 16 |
| PState_Air | 0x10b00 | 0x10af6 | -10 **moved** | 2 | 16 |
| PState_Spindash | 0x10e50 | 0x10e40 | -16 **moved** | 2 | 16 |
| PState_Fly | 0x10ed4 | 0x10ec4 | -16 **moved** | 2 | (unpinned) |
| PState_Glide | 0x10fe0 | 0x10fd0 | -16 **moved** | 2 | (unpinned) |
| Climb_WallDist | 0x11298 | 0x11288 | -16 **moved** | 2 | (unpinned) |
| Ability_InstaShield | 0x11594 | 0x11584 | -16 **moved** | 2 | (unpinned) |
| CharDef_Sonic | 0x11de0 | 0x11dce | -18 **moved** | 2 | 16 |
| CharDef_Tails | 0x11e20 | 0x11e04 | -28 **moved** | 2 | 16 |
| CharDef_Knuckles | 0x11e56 | 0x11e3a | -28 **moved** | 2 | (unpinned) |
| CharacterDefs | 0x11e90 | 0x11e70 | -32 **moved** | 2 | 16 |
| TailsAppendage_Refresh | 0x11eda | 0x11eba | -32 **moved** | 2 | 2 |
| DustPuff_Spawn | 0x11ff6 | 0x11fd6 | -32 **moved** | 2 | (unpinned) |
| Dust_Tick | 0x1203c | 0x1201c | -32 **moved** | 2 | (unpinned) |
| RingSparkle_Spawn | 0x12140 | 0x12120 | -32 **moved** | 2 | (unpinned) |
| TestStatic_Main | 0x12260 | 0x1223c | -36 **moved** | 2 | 16 |
| TestSolid_Init | 0x12264 | 0x12240 | -36 **moved** | 2 | 4 |
| ObjDef_PathSwap | 0x12276 | 0x12252 | -36 **moved** | 2 | 2 |
| DeformTable_Zero | 0x12310 | 0x122e4 | -44 **moved** | 2 | 16 |
| EditorSceneBinding_OJZ_Act1_Sec0 | 0x130a4 | 0x13078 | -44 **moved** | 2 | (unpinned) |
| OJZ_TestRaster | 0x131f6 | 0x131ca | -44 **moved** | 2 | (unpinned) |
| ObjDef_Static | 0x13750 | 0x13718 | -56 **moved** | 2 | 16 |
| OJZ_Sec0_TypeTable | 0x13790 | 0x1374c | -68 **moved** | 2 | 16 |
| OJZ_Act_Pool_Page0 | 0x13900 | 0x138bc | -68 **moved** | 2 | 16 |
| OJZ_Act1_Descriptor | 0x16810 | 0x167c8 | -72 **moved** | 2 | 16 |
| OJZ_Sec0_Blocks | 0x16a90 | 0x16a42 | -78 **moved** | 2 | 16 |
| OJZ_Sec0_LocalMap | 0x2209c | 0x2204c | -80 **moved** | 2 | 4 |
| OJZ_Palette | 0x22d60 | 0x22d10 | -80 **moved** | 2 | 16 |
| BgAnim_Table | 0x275f0 | 0x27592 | -94 **moved** | 2 | 16 |
| Map_TestObj | 0x2961e | 0x295c0 | -94 **moved** | 2 | 2 |
| Map_DustSpindash | 0x2964e | 0x295f0 | -94 **moved** | 2 | (unpinned) |
| Ani_Sonic | 0x2a228 | 0x2a1ca | -94 **moved** | 2 | 8 |
| Ani_Tails | 0x2a340 | 0x2a2d4 | -108 **moved** | 2 | 16 |
| Ani_Knuckles | 0x2a4fc | 0x2a490 | -108 **moved** | 2 | (unpinned) |
| Ani_DustSpindash | 0x2a668 | 0x2a5fc | -108 **moved** | 2 | (unpinned) |
| Map_Tails | 0x2a680 | 0x2a610 | -112 **moved** | 2 | 16 |
| Map_Knuckles | 0x4b5de | 0x4b56e | -112 **moved** | 2 | (unpinned) |
| HeightMaps | 0x6dcb0 | 0x6dc36 | -122 **moved** | 2 | 16 |
| GameState_OJZScroll_Init | 0x8a130 | 0x8a0b6 | -122 **moved** | 2 | 16 |
| __align$games.sonic4.replay_fixture$0 | 0x8a6a0 | 0x8a624 | -124 **moved** | 2 | (unpinned) |
| BusError | 0x8a900 | 0x8a884 | -124 **moved** | 2 | 16 |
| EndOfRom | 0x8b9b0 | 0x8b934 | -124 **moved** | 2 | 16 |

sections moved: 77 of 85; largest single move: -124 (__align$games.sonic4.replay_fixture$0)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 23
  declared  2 vs residue          2: 5
  declared  2 vs residue          4: 3
  declared  2 vs residue          8: 5
  declared  2 vs residue         16: 41
  SoundTablesZ80_Head: pre 0x0 post 0x0
  Sound_PlaySFX: pre 0x0 post 0x0
  EndOfRom: pre 0x8b9b0 post 0x8b934

### lean: pre 454a546c/674830  post c347e317/674816  size delta -14
| head label | pre | post | delta | declared | pin-residue quantum |
|---|---|---|---|---|---|
| Vectors | 0x0 | 0x0 | +0 | 2 | 16 |
| GameHeader | 0x100 | 0x100 | +0 | 2 | (unpinned) |
| EntryPoint | 0x200 | 0x200 | +0 | 2 | 16 |
| BootData | 0x3a0 | 0x398 | -8 **moved** | 2 | 16 |
| BootData_PostBlob | 0x1bf0 | 0x1be2 | -14 **moved** | 2 | (unpinned) |
| VDP_Shadow_Init | 0x1c00 | 0x1bf0 | -16 **moved** | 2 | 16 |
| Init_DMA_Queue | 0x1c3a | 0x1c2a | -16 **moved** | 2 | 2 |
| Init_SpriteTable | 0x1f70 | 0x1f56 | -26 **moved** | 2 | 16 |
| VBlank_Handler | 0x2250 | 0x222e | -34 **moved** | 2 | 16 |
| HBlank_Install | 0x2430 | 0x240e | -34 **moved** | 2 | 16 |
| Read_Controllers | 0x2460 | 0x243e | -34 **moved** | 2 | 16 |
| GameLoop | 0x256e | 0x254c | -34 **moved** | 2 | 2 |
| Input_Tick | 0x2590 | 0x256e | -34 **moved** | 2 | (unpinned) |
| S4LZ_DecompressDict | 0x26e0 | 0x26b6 | -42 **moved** | 2 | 16 |
| ZX0R_Decompress | 0x27d8 | 0x27ae | -42 **moved** | 2 | (unpinned) |
| GetSineCosine | 0x2850 | 0x2826 | -42 **moved** | 2 | 16 |
| Perform_DPLC | 0x2c48 | 0x2c1c | -44 **moved** | 2 | 8 |
| InitObjectRAM | 0x2cf0 | 0x2cc0 | -48 **moved** | 2 | 16 |
| InitSpriteSystem | 0x2ff0 | 0x2fb8 | -56 **moved** | 2 | 16 |
| AnimateSprite | 0x3410 | 0x33d2 | -62 **moved** | 2 | 16 |
| TouchResponse | 0x35a4 | 0x3566 | -62 **moved** | 2 | 4 |
| RingBuffer_Add | 0x37a4 | 0x3766 | -62 **moved** | 2 | 4 |
| Collected_Init | 0x3964 | 0x3924 | -64 **moved** | 2 | 4 |
| PopulateSpawnedPieceCount | 0x4260 | 0x4212 | -78 **moved** | 2 | 16 |
| Load_Object | 0x4550 | 0x44fe | -82 **moved** | 2 | 16 |
| Plane_Buffer_Reset | 0x45d8 | 0x4586 | -82 **moved** | 2 | 8 |
| Tile_Cache_GetTile | 0x4900 | 0x48a0 | -96 **moved** | 2 | 16 |
| Collision_GetType | 0x5790 | 0x572e | -98 **moved** | 2 | 16 |
| Collision_ProbeDown | 0x5800 | 0x5796 | -106 **moved** | 2 | 16 |
| Section_Init | 0x5cf4 | 0x5c8a | -106 **moved** | 2 | 4 |
| Camera_Init | 0x6160 | 0x60ea | -118 **moved** | 2 | 16 |
| Parallax_Init | 0x6330 | 0x62b2 | -126 **moved** | 2 | 16 |
| Raster_Install | 0x6c6e | 0x6bf0 | -126 **moved** | 2 | (unpinned) |
| Palette_LoadPal | 0x6fd2 | 0x6f54 | -126 **moved** | 2 | (unpinned) |
| Effects_ResolveParallax | 0x7456 | 0x73d8 | -126 **moved** | 2 | (unpinned) |
| Level_LoadArt | 0x7520 | 0x749a | -134 **moved** | 2 | 16 |
| PageIn_Process | 0x75d8 | 0x7552 | -134 **moved** | 2 | (unpinned) |
| PageCache_Init | 0x78c4 | 0x783e | -134 **moved** | 2 | (unpinned) |
| BG_Init | 0x7db0 | 0x7d24 | -140 **moved** | 2 | 16 |
| BgAnim_Init | 0x7e90 | 0x7df8 | -152 **moved** | 2 | 16 |
| Sound_PostByte | 0x7f2e | 0x7e96 | -152 **moved** | 2 | 2 |
| SoundTablesZ80_Head | 0x8000 | 0x8000 | +0 | 32768 | 16 |
| ObjCodeBase | 0x10000 | 0x10000 | +0 | 65536 | 16 |
| Player_Init | 0x10002 | 0x10002 | +0 | 2 | 2 |
| PState_Ground | 0x10690 | 0x10686 | -10 **moved** | 2 | 16 |
| PState_Air | 0x10b20 | 0x10b10 | -16 **moved** | 2 | 16 |
| PState_Spindash | 0x10e70 | 0x10e5a | -22 **moved** | 2 | 16 |
| PState_Fly | 0x10f10 | 0x10ef6 | -26 **moved** | 2 | (unpinned) |
| PState_Glide | 0x11044 | 0x11028 | -28 **moved** | 2 | (unpinned) |
| Climb_WallDist | 0x1131a | 0x112fa | -32 **moved** | 2 | (unpinned) |
| Ability_InstaShield | 0x11620 | 0x115fe | -34 **moved** | 2 | (unpinned) |
| CharDef_Sonic | 0x11e80 | 0x11e50 | -48 **moved** | 2 | 16 |
| CharDef_Tails | 0x11ec0 | 0x11e86 | -58 **moved** | 2 | 16 |
| CharDef_Knuckles | 0x11ef6 | 0x11ebc | -58 **moved** | 2 | (unpinned) |
| CharacterDefs | 0x11f30 | 0x11ef2 | -62 **moved** | 2 | 16 |
| TailsAppendage_Refresh | 0x11f7a | 0x11f3c | -62 **moved** | 2 | 2 |
| DustPuff_Spawn | 0x12096 | 0x12058 | -62 **moved** | 2 | (unpinned) |
| Dust_Tick | 0x120dc | 0x1209e | -62 **moved** | 2 | (unpinned) |
| RingSparkle_Spawn | 0x121e0 | 0x121a2 | -62 **moved** | 2 | (unpinned) |
| TestStatic_Main | 0x12300 | 0x122be | -66 **moved** | 2 | 16 |
| TestSolid_Init | 0x12310 | 0x122c2 | -78 **moved** | 2 | 16 |
| ObjDef_PathSwap | 0x12322 | 0x122d4 | -78 **moved** | 2 | 2 |
| DeformTable_Zero | 0x123b4 | 0x12366 | -78 **moved** | 2 | 4 |
| EditorSceneBinding_OJZ_Act1_Sec0 | 0x13148 | 0x130fa | -78 **moved** | 2 | (unpinned) |
| OJZ_TestRaster | 0x1329a | 0x1324c | -78 **moved** | 2 | (unpinned) |
| ObjDef_Static | 0x137f0 | 0x1379a | -86 **moved** | 2 | 16 |
| OJZ_Sec0_TypeTable | 0x13828 | 0x137ce | -90 **moved** | 2 | 8 |
| OJZ_Act_Pool_Page0 | 0x13998 | 0x1393e | -90 **moved** | 2 | 8 |
| OJZ_Act1_Descriptor | 0x168a4 | 0x1684a | -90 **moved** | 2 | 4 |
| OJZ_Sec0_Blocks | 0x16b20 | 0x16ac4 | -92 **moved** | 2 | 16 |
| OJZ_Sec0_LocalMap | 0x22130 | 0x220ce | -98 **moved** | 2 | 16 |
| OJZ_Palette | 0x22e00 | 0x22d92 | -110 **moved** | 2 | 16 |
| BgAnim_Table | 0x27682 | 0x27614 | -110 **moved** | 2 | 2 |
| Map_TestObj | 0x296b0 | 0x29642 | -110 **moved** | 2 | 16 |
| Map_DustSpindash | 0x296e0 | 0x29672 | -110 **moved** | 2 | (unpinned) |
| Ani_Sonic | 0x2a2c0 | 0x2a24c | -116 **moved** | 2 | 16 |
| Ani_Tails | 0x2a3ca | 0x2a356 | -116 **moved** | 2 | 2 |
| Ani_Knuckles | 0x2a586 | 0x2a512 | -116 **moved** | 2 | (unpinned) |
| Ani_DustSpindash | 0x2a6f2 | 0x2a67e | -116 **moved** | 2 | (unpinned) |
| Map_Tails | 0x2a710 | 0x2a692 | -126 **moved** | 2 | 16 |
| Map_Knuckles | 0x4b66e | 0x4b5f0 | -126 **moved** | 2 | (unpinned) |
| HeightMaps | 0x6dd40 | 0x6dcb8 | -136 **moved** | 2 | 16 |
| Dac_Temp_Blip | 0x90000 | 0x90000 | +0 | 32768 | 16 |
| Song_MovingTrucks | 0xa0630 | 0xa0630 | +0 | 8 | 16 |
| Sfx_33 | 0xa3b20 | 0xa3b18 | -8 **moved** | 8 | 16 |
| GameState_OJZScroll_Init | 0xa4410 | 0xa4404 | -12 **moved** | 2 | 16 |
| __align$games.sonic4.replay_fixture$0 | 0xa4980 | 0xa4972 | -14 **moved** | 2 | (unpinned) |
| ReleaseFault | 0xa4be0 | 0xa4bd2 | -14 **moved** | 2 | 16 |
| EndOfRom | 0xa4c0e | 0xa4c00 | -14 **moved** | 2 | 2 |

sections moved: 81 of 89; largest single move: -152 (BgAnim_Init)
histogram over moved sections (declared quantum, pin-residue quantum) -> count:
  declared  2 vs residue (unpinned): 23
  declared  2 vs residue          2: 8
  declared  2 vs residue          4: 6
  declared  2 vs residue          8: 4
  declared  2 vs residue         16: 39
  declared  8 vs residue         16: 1
  SoundTablesZ80_Head: pre 0x8000 post 0x8000
  Sound_PlaySFX: pre 0x8054 post 0x7fbc
  EndOfRom: pre 0xa4c0e post 0xa4c00

## 10. Open, and the ledger

* **Aeon-side, REQUIRED at landing** (in the trial aeon commits 75044465 + 5823ea77, on a
  local branch of `.aeon-ref-r7` that is NOT pushed): both maps' `[[hole]] at = 0x3F0`, and
  the two fixture re-stamps. The hole row's per-shape semantics (0x3F0 plain vs 0x3F4 debug)
  is the aeon lane's call (§3).
* `validate_sound_fold` has no red-first witness that makes it FIRE (its logic is untouched
  here; message only). Ledgered.
* `ring_sparkle` has no `repin.toml` region of its own; the `dust_spindash` allotment used to
  sweep it. Ledgered.
* `section:<name>` region ends cannot address a section whose NAME the layout holds twice
  (`text`); a head-label spelling would. Ledgered.
* TAGGED for the controller (no emulator here): a sound-on boot of post-flip `s4.bin`
  playing an SFX exercises the nine `abs.w`-re-encoded `Sound_PlaySFX` calls (§4).
* The trial aeon revision is unreachable from aeon `origin/master` by construction;
  `refreeze --check` reports reachability without gating on it, and nothing here was attested.

## 11. What in this brief was wrong

1. **"`SoundTablesZ80_Head` sits at 0x8000 exactly today with zero margin."** The 8000 in the
   listing is its VMA (`vma: $8000` phase bank); its LMA is the declared `sound_bank` anchor
   0xA0000 and cannot move. The zero-margin symbol was `Sound_PlaySFX` at LMA 0x8054, and it
   DID cross: 0x7FBC post-flip in `s4` and `lean` (§4). The brief's hedge ("your listing
   outranks this sentence") was right to be there.
2. **"the section BEFORE it shrinks."** The sound head did not move and nothing before it
   "shrinks" as a section; 40–53 sections upstream each lost their 16-vs-2 pad and the
   cumulative shift (−0x98 at `Sound_PlaySFX` in s4) is what crossed the ceiling.
3. **`scripts/provision-aeon-ref.sh` "run FROM YOUR WORKTREE" with its defaults overridden
   for `SIGIL_BIN`/`REF_TARGET`** — a third default also breaks from a linked worktree:
   `AEON_REPO` defaults to `$SIGIL_ROOT/../aeon`, which under `.claude/worktrees/` does not
   exist (`cd: …/worktrees/../aeon: No such file or directory`). `AEON_REPO=` must be set too.
4. **"the flip changes placement and nothing else" as the trial-run expectation** — true of
   sigil's placement, but the flip also changes aeon's BUILD: three sound-off shapes refuse
   to build against the shipped maps (§3), and both sonic4 shapes exit `build.sh` 1 on two
   address-keyed fixture gates until re-stamped. A trial freeze cannot even start
   (`capture_goldens.sh` under `set -e`) without an aeon-side commit; the brief's sequencing
   ("the real freeze is the aeon lane's to run at landing") is right, but the landing is
   not sigil-only.
5. **"passed + ignored == git grep -c '#\[test\]' summed"** — `-c` counts lines and the attr
   count is not the executed count even with `-o` (macro-instantiated and cfg-gated tests,
   §7). The run-to-run invariant (4184 in all four runs) and the per-file deltas are what
   reconcile; the exact equality did not hold at the tip's own attested revision either
   (4177 attrs, 4178 executed).
6. **"`packed_chained_base` becomes a function of the declaration, not the pin"** — and it
   also had to become fallible: the only honest signature returns `Result`, because the
   walk's refusal of an undeclared section IS the function's `None` arm.
7. Nothing else in the brief was contradicted by command output. The scope boundary (point
   3) held without needing the BLOCKED exit.

## 12. Chain-195 re-baseline and handover for chain 196

Sections 0–11 are the chain-193 account (sigil master 5a25a0d4-era, aeon 25731dfa). Chain
194 (red) and 195 (`d34-ceiling-band`, aeon `027ec1620dd977bf7b8ee47cbafe2b2197059092`,
sigil master `036800fd`) landed underneath it, so the parcel was re-done at 195 on
`parcel/alignment-flip-195` / `trial/alignment-flip-freeze-195` (worktree
`.claude/worktrees/agent-ae9148ef5f647aadf`; ref tree `.aeon-ref-r7b`, never `.aeon-ref-r7`).
The code half (`e2517405`, `62733604`) is the same flip as §1; everything below is what 195
changed in the numbers, the hand sites, and the aeon-side edits. No emulator was used; the
runtime item stays TAGGED (§12.9).

### 12.1 Seven shapes, pre → post (pre == the chain-195 goldens)

`pre` is `provenance.toml` entry `d34-ceiling-band` verbatim (its `full_crc`/`full_size`
lines: config_a 10746–47, config_b 10753–54, demo 10760–61, demo_debug 10767–68, lean
10774–75, s4 10781–82, s4_debug 10788–89), reproduced by `measure-shapes.sh` from the
pre-flip binary at the ref tree's `027ec162`. `post` is the same script from the flip binary
at `cb4b3f5a` (`sigil --version`: revision cb4b3f5a, clean) against the ref tree at
`fbe89918` (§12.5), run twice — the inherited `measure-post-195/` and a fresh
`measure-post-195b/` (log header `sigil_head=cb4b3f5a aeon_head=fbe89918
at=2026-09-02T03:27:56Z`), identical CRC32/size on all seven (`zlib.crc32`).

| shape | pre (chain 195) | post | Δ size | EndOfRom pre → post |
|---|---|---|---|---|
| s4 | fdd1cf81/719387 | ac10ab85/719325 | −62 | 0xA5C90 → 0xA5C82 (−14) |
| s4_debug | 0f6b1359/736391 | fa866f19/736345 | −46 | 0xA7F40 → 0xA7F34 (−12) |
| demo | aca8c043/96476 | 30a31d81/96458 | −18 | 0x1121C → 0x1121A (−2) |
| demo_debug | 932f496f/101359 | 51056291/101323 | −36 | 0x1121C → 0x1121A (−2) |
| config_a | 80f9c672/736759 | 819634c2/736697 | −62 | 0xA7F40 → 0xA7F34 (−12) |
| config_b | 61512d30/614991 | 46e2f38b/614839 | −152 | 0x8C770 → 0x8C6E6 (−138) |
| lean | 4d3f718f/674830 | 0373dec8/674816 | −14 | 0xA4C0E → 0xA4C00 (−14) |

The demo shapes' post CRCs are UNCHANGED from §0 (the demo game did not move between 193
and 195); the five sonic4-derived shapes moved with their baselines. The file Δ and the
EndOfRom Δ differ per shape (s4 −62 vs −14, config_b −152 vs −138): the remainder sits in
the deb2 appendix past `EndOfRom`, whose length follows the symbol addresses it encodes.
Observed arithmetic only; the appendix encoding was not re-derived here (ledgered, §12.10).
Per-shape declared-label tables (every head label, pre/post/delta/declared/quantum) are in
`.sigil-r7-target/compare-195.md` (regenerable: §8a with the `-195` dirs); the s4 and
s4_debug slides read exactly as §9 (Sound_PostByte −0x98 plain / −0x80 debug, engine bank
org-anchor absorbed at $10000).

### 12.2 The abs.w ceiling falsifier at 195, both sonic4 shapes

Byte-scanned in the seven `measure-*-195` ROMs (`4EB9 0000 xxxx` → `4EB8 xxxx`, `4EF9` →
`4EF8`, plus `lea`/`pea` forms — none of those), listings for the addresses:

| shape | Sound_PlaySFX pre → post | crossed | sites re-encoded abs.l → abs.w |
|---|---|---|---|
| s4 | 0x8054 → **0x7FBC** | yes | 9 `jsr` + 2 `jmp` = **11** (`ps_checkfull` 0x807E→0x7FE6, `ps_drop` 0x8094→0x7FFC, `ps_ret` 0x8098→**0x8000**: no absolute references to those three) |
| lean | 0x8054 → **0x7FBC** | yes | 9 `jsr` + 2 `jmp` = 11 (same code) |
| s4_debug | 0xAF8A → 0xAF0A | no | Sound_PlaySFX: 0. **But `Raster_Install` 0x8042 → 0x7FDC crosses**, and `Debug_BandDemoHotkey`'s two `jbra Raster_Install` (ojz_scroll_test.emp:1698/1709, DEBUG-only) re-encode `jmp` abs.l → abs.w: **2 sites, −4 B** |
| config_a | 0xB09A → 0xAFE2 | no | `Raster_Install` 0x8152 → 0x80B4 stays above: 0 |

`SoundTablesZ80_Head` is 0x8000 (VMA) in every sound-on shape, pre and post. §4 counted 9
`jsr` and named the insta-shield's `jmp`; the definitive count is 11 (the second `jmp` is a
tail call in the same bank) — and §4's debug row "above" was true of Sound_PlaySFX and
FALSE of the shape: the debug crossing is a different label. The falsifier as specified
(one label) cannot see that; the general check is "every head label with pre ≥ 0x8000 and
post < 0x8000, per shape, with its absolute-reference count" (ledgered, §12.10). The two
`abs.w` `jsr` sites in `[PState_Ground, Ground_Move_Cap)` put `Ground_Move_Cap` at
`P_STATE_GROUND.plain + 0x2F4` (was +0x2F8) and leave debug at +0x2F4 (site 5, §12.6).

### 12.3 §3 cross-check at 195 (the sound-off hole)

| shape | BootData | idle (40 B) | BootData_PostBlob pre → post | BootData_End pre → post |
|---|---|---|---|---|
| demo | 0x392 | 0x3C8..0x3F0 | 0x3F8 → 0x3F0 | 0x406 → 0x3FE |
| demo_debug | 0x396 | 0x3CC..0x3F4 | 0x3F8 → 0x3F4 | 0x406 → 0x402 |
| config_b | 0x392 | 0x3C8..0x3F0 | 0x3F8 → 0x3F0 | 0x406 → 0x3FE |

§3's per-shape resume addresses (0x3F0 plain / 0x3F4 demo debug) hold at 195 unchanged; the
`at = 0x3F0` lower-bound row is still what both maps need. The idle's `code_end` is 0x28 in
every listing. `boot_data_port.rs`'s comment/window start is **0x392** (the chain-193 site
3 text and the inherited draft said "$39a"; the symbol table, the compare tables and
0x392 + 0x36 = 0x3C8 all say 0x392 — corrected in `16720845`).

### 12.4 Landing runs

`scripts/landing-run.sh --baseline 4177 --aeon /home/volence/sonic_hacks/.aeon-ref-r7b
--target /home/volence/sonic_hacks/.sigil-r7-target --expect-test <7 names>` with
`SIGIL_BUILD`/`SIGIL_EMIT` exported (SIGIL_STRICT_GATE=1, full workspace, `--no-fail-fast`,
`--nocapture`); expect-tests: `declared_chain_plain`, `declared_chain_debug`,
`both_spellings_of_the_section_row_build_the_same_rom`, `config_b_boot_data_hole_filled`,
`config_b_doctored_size_table_moves_no_bytes`,
`every_region_end_contract_holds_against_the_live_layout`,
`generated_pins_match_the_hand_typed_baseline` — every one present by name (`test <name>
... ok`) in every run. Reference in every run: `.aeon-ref-r7b @ fbe89918
(trial/alignment-flip-hole-195, clean)`, all four ROMs present. Logs
`.sigil-r7-target/landing-195-{1-parcel,2-trial,3-trial}.log` (+ `.verdict`).

| # | tree | passed | failed | ignored | skip | exit | wall clock (UTC) | note |
|---|---|---|---|---|---|---|---|---|
| 1 | `parcel/alignment-flip-195` content @ cb4b3f5a (pre-freeze) | 4016 | 166 | 2 | 0 | 1 | 03:40:14 → 03:43:53 | 156 unique names, ALL bucket A |
| 2 | `trial/alignment-flip-freeze-195` @ 8106bbe1† (freeze + 5 hand sites) | 4182 | 0 | 2 | 0 | 0 | 04:08:09 → 04:11:34 | **GREEN** |
| 3 | **trial @ 8c45f07d†** (comment-only re-tag of site 4) | **4182** | **0** | **2** | **0** | **0** | 04:13:33 → 04:16:52 | **GREEN**, code-final tip |

† The log stamps are the pre-rebase SHAs: after run 3 the three trial-only commits were
rebased onto the commit that adds this section (docs only, no `.rs`/golden change — `git diff
--stat` between the two trial tips is these two notes), so the run-3 tree IS the pushed trial
tip's tree minus this text. Cite the trial commits by subject, not SHA.

Runs 2–4 of §7 collapse to one here because sites 1, 2, 3, 5 were already on the parcel
branch (`33bfcfb8`, `16720845`) before the freeze; only site 4 followed it. The three
`panicked` lines in the green logs are `#[should_panic]` tests
(`override_of_unknown_constant_panics`, `ensure_generated_refuses_before_it_touches_an_absent_tree`,
`compress_panics_on_error`).

**Run 1 classification.** 166 FAILED lines, 156 unique names; every one bucket **A** (its
subject is a frozen artifact): 111 `*_region_matches_reference` / `*_matches_reference` /
`*_undoctored_compile_equals_the_reference_window` / `*_regions_match_reference`; 16 =
`{config_a,config_b,demo_debug,demo_plain,lean}_{anchor_matches_golden,full_file}` +
`{config_a,config_b,demo_debug,demo,lean}_size_table_rederives_native` +
`flipped_config_a_anchor_matches_golden` (§7 wrote "×7" for these — the s4 pair is named
`native_full_sonic4_*` / `native_rom_*`, counted next); 29 read one by one:
`a_doctored_indexed_mode_changes_the_bytes`, `aeon_dir_matches_the_provenance_tip`
(fbe89918 vs the tip's 027ec162), `a_passing_extra_entry_moves_no_bytes`,
`both_spellings_of_the_section_row_build_the_same_rom`,
`colinked_sfx_head_matches_the_reference_rom_slice_both_shapes`,
`config_b_boot_data_hole_filled`, `config_b_doctored_size_table_moves_no_bytes`,
`config_b_frozen_placement_exact`, `deb2_appendix_negative_controls`,
`declared_chain_{debug,plain}`, `deform_pointer_equals_placed_label_vma`,
`demo_{debug,plain}_game_modules_match_golden`, `doctored_af_delete_produces_different_bytes`,
`doctored_golden_at_deform_pointer_is_caught`, `native_full_sonic4_{debug,plain}`,
`native_rom_{debug,plain}`, `objdefs_match_reference_{debug,plain}`, `pins_rs_is_current`
(268 changed pins), `s4_boot_data_blob_present`, `two_module_ownership_flip_{debug,plain}`,
`two_module_tail_call_flip_{debug,plain}`, `vector_table_matches_reference_rom_first_256_bytes`.
**Bucket B: 0.** Set difference vs §7 run 1: only-193 =
`config_b_doctored_size_table_breaks_the_build` (renamed),
`every_region_end_contract_holds_against_the_live_layout`,
`sound_layout_derives_the_frozen_addresses` (both pass pre-freeze now — repin.toml and the
seam2 literals are on the parcel branch); only-195 = the renamed
`config_b_doctored_size_table_moves_no_bytes`.

**Reconciliation.** `git grep -o -e '#\[test\]' HEAD -- '*.rs' | wc -l` = 4183 on the trial
tip, 4178 at master `036800fd` (= the attested `cffdb56c`); executed 4182 + 2 = 4184 vs the
chain-195 attest's 4177 + 2 = 4179: **+5 both**, all in `crates/sigil-harness/src/native.rs`
(+6 walk tests, −1 `a_pin_that_violates_the_declaration_is_refused_with_the_residue`). The
constant +1 between attr count and executed count is the pre-existing macro-instantiated
test §11.5 records. `strict_bodies` is not re-attested here (the attest is chain 196's).

### 12.5 The trial freeze and the aeon-side edits

`refreeze --freeze alignment-flip-trial-195 --ab "…" --note "TRIAL — NOT FOR MERGE: …"`,
AEON_DIR = `.aeon-ref-r7b @ fbe89918`, 03:45:05 → 04:00:37 UTC, committed as `TRIAL freeze: alignment-flip-trial-195 (aeon
fbe89918) - NOT FOR MERGE, never cherry-pick`:
provenance entry #196 `alignment-flip-trial-195`, aeon_rev
`fbe89918347c580aa1598687b3491940ee1b0ab6`, seven `full_crc/full_size` EQUAL §12.1's post
column (config_a 819634c2/736697, config_b 46e2f38b/614839, demo 30a31d81/96458,
demo_debug 51056291/101323, lean 0373dec8/674816, s4 ac10ab85/719325, s4_debug
fa866f19/736345); `refreeze --check`: OK, chain len 196; DIVERGENT rows: #181 (sigil
bfbedc11, pre-existing) and #196 (aeon fbe89918, by construction — unreachable from aeon
`origin/master`). `pins.rs` regenerated: 268 pins changed (§7 counted 267 before its repin.toml re-spelling;
here `33bfcfb8` precedes the freeze, so `DUST_SPINDASH.len` is inside the 268).

Aeon side, ref tree `.aeon-ref-r7b` branch `trial/alignment-flip-hole-195` =
`027ec162` + two TRIAL commits (never pushed; the aeon lane re-does them on its own branch):

```
3d943138  games/demo/map.toml:    [[hole]] after="Z80_IdleProgram"  at = 0x3F8 -> at = 0x3F0
          games/sonic4/map.toml:  same row, at = 0x3F8 -> 0x3F0 (+ MEASURED comment 0x3C8..0x3F0)
fbe89918  tools/fixtures/sprite_tilt_cut.json   <- python3 tools/sprite_tilt_gate.py --emit-fixture
          tools/fixtures/instashield_cut.json   <- python3 tools/instashield_gate.py --write-fixture
```

both fixture files re-stamped by aeon's OWN gates against the post-flip ROMs (build logs
`.sigil-r7-target/fixtures-195*.log`: the gates' "wrote" lines, then "byte-identical" ×12 and
`build.sh` exit 0 on plain and DEBUG=1). instashield_cut.json: plain cut 64 → 62 B (ends
`4ef87fbc4e75`), stubs 007FBC Sound_PlaySFX / 0103E6 Player_SetState / 01163C
InstaShield_Spawn, window 71166..71228; debug 71454..71516, stubs 00AF0A / 0104AA / 01175C.
sprite_tilt_cut.json: Ani_Knuckles 173376/175608, Ani_Sonic 172666/174898, Ani_Tails
172932/175164, refresh 13630/15990 (plain/debug). Sequence for the lane: aeon commit(s) →
`SIGIL_BUILD=… SIGIL_EMIT=… AEON_DIR=<that tree> refreeze --freeze <chain-196 name>` on
sigil → the five hand sites (§12.6, values already on the branches) → landing run → attest.

### 12.6 The five hand sites, 195 values

1. `crates/sigil-harness/repin.toml` — as §7 (`entity_window`/`children`/`dust_spindash`
   `end = "section:<name>"`, `objdefs` `len = 0x34`); `33bfcfb8`, parcel branch.
2. `crates/sigil-cli/tests/seam2_layout_derivation.rs` — `sfx_bank_lma_plain` 0xA3B20 →
   **0xA3B18**, `sfx_bank_lma_debug` 0xA5570 → **0xA5568** (Sfx_33 −8 both); `16720845`.
3. `crates/sigil-cli/tests/boot_data_port.rs` — window `0x39a..0x406` → **`0x392..0x3fe`**,
   idle `0x3c8..0x3f0`, tail byte at `0x3f0`; `16720845`.
4. `crates/sigil-harness/tests/repin_pins.rs` — **13 assert lines / 15 pin fields** in the
   live `generated_pins_match_the_hand_typed_baseline` (§7 said 14), each tagged
   `alignment-flip` with its one reason; every old literal equalled the 036800fd `pins.rs`,
   so no STALE-CATCHUP term exists: BOOT_DATA.plain 0x3A0→0x398; ANIMATE/RINGS bases −0x3E
   plain / −0x2E debug; CORE base −0x30 / −0x1E and **CORE.debug_len 0x742→0x740** (the
   `jbsr Draw_Sprite` in `RunObjects_Frozen` relaxes bsr.w→bsr.s once the 0xE core→sprites
   pad is gone — verified in the s4_debug listings); DPLC base −0x2C / −0x1A; DELETE_OBJECT
   −0x30 / −0x1E; ASSEMBLED_LEN 0xA5C90→0xA5C82; DEBUG_ASSEMBLED_LEN 0xA7F40→0xA7F34 (its
   −0xC = Sfx_33 −8, game −4, the two `jmp Raster_Install` −4, replay_fixture's own align
   +4). Trial branch only (the two `trial: repin_pins …` commits): the values are the freeze's. The RETIRED
   `#[ignore]` `secondary_pin_classes_match_the_hand_typed_baseline` was left alone — its 8
   literals (SOUND_API 0x7A9E/0xA330, MDDBG_* 0x5_E8F2/0x5_F6B8, PLAYER_1, DYNAMIC_SLOTS,
   RINGCOL_OFF.debug) were already behind master's `pins.rs` before this parcel (ledgered).
5. `crates/sigil-cli/tests/native_full_rom.rs` — `Ground_Move_Cap` plain `+0x2F8` → **`+0x2F4`**,
   debug `+0x2F4` unchanged; `16720845`.

### 12.7 Branches and the split point

`parcel/alignment-flip-195` = code + tests + packet, ends at the commit that adds this
section; `trial/alignment-flip-freeze-195` = that tip + `TRIAL freeze` (goldens, size tables,
pins.rs, provenance #196) + the site-4 baseline. The split point is the parcel tip: the first
trial-only commit is the freeze, because a freeze regenerates artifacts the aeon lane's own
freeze will regenerate against ITS aeon revision (the goldens would be byte-identical, the
`aeon_rev` would not), and site 4 asserts the freeze's `pins.rs`. Nothing on the trial
branch is cherry-pickable into a landing; the parcel branch is.

### 12.8 What was wrong in THIS brief

1. **"baseline 4177 passed + 2 ignored" as the landing-run baseline** — `landing-run.sh`
   reconciles `PASSED − BASELINE` (ignored excluded; §7 used 4176 for a 4176 + 2 attest), so
   the baseline is **4177**, not 4179. Passing 4179 would have reported a −3 delta on a green
   run.
2. **"Sound_PlaySFX LMA for BOTH sonic4 shapes" as the falsifier** — the debug shape's
   ceiling crossing is `Raster_Install` (0x8042 → 0x7FDC, 2 `jmp` sites), which a
   Sound_PlaySFX-only probe reports as "no crossing". Also the number is a VMA (§4); in the
   engine bank VMA == LMA, so the figure stands, but the wording does not generalise.
3. **§7 site 4 "14 literals"** — 13 assert lines (15 pin fields) in the live test; the
   RETIRED test's stale literals are a separate, pre-existing fact.
4. **§7 "`*_anchor_matches_golden` ×7, `*_full_file` ×7, `*_size_table_rederives_native`
   ×7"** — 5 + 5 + 5 + `flipped_config_a_anchor_matches_golden`; the s4 pair is spelled
   `native_full_sonic4_*` / `native_rom_*`.
5. **The inherited draft's `boot_data_port.rs` window "$39a"** — BootData is at 0x392 in
   config_b (§12.3); the draft's window start hid 8 bytes of the head from the compare.
6. **`measure-shapes.sh` as a one-call step** — seven builds through `build.sh` (each
   ≈ 1 min 50 s with aeon's in-build tool tests) plus two canonical restores exceed a 10-minute
   tool timeout; the restore of `s4.debug.bin` had to be finished as a separate call.
7. Master moved during the work, docs-only: `036800fd` → `80dc87d5` (three notes/ledger
   commits, no `.rs`/golden change). The chain-195 baseline is intact; the ledger append in
   this branch will need a trivial tail merge.
8. Nothing else in the brief was contradicted by command output; no BLOCKED exit was
   needed.

### 12.9 Open / TAGGED

* TAGGED for the controller (no emulator here): a sound-on boot of post-flip `s4.bin`
  (ac10ab85/719325) that plays an SFX exercises the 11 `abs.w`-re-encoded `Sound_PlaySFX`
  sites; a DEBUG boot that fires `Debug_BandDemoHotkey` exercises the two re-encoded
  `jmp Raster_Install`.
* The hole row's per-shape semantics (0x3F0 vs 0x3F4) — still the aeon lane's call (§3).
* The chain-196 attest (`strict_bodies` ratchet, 29 at 195) is the lane's; not run here.

### 12.10 Ledgered this round

`campaign-gap-ledger.md`, `[alignment-flip parcel, 2026-09-02]`: the one-label ceiling
falsifier; the RETIRED baseline's stale literals; the deb2-appendix share of the file delta;
`rewrite-baseline.py` handling only the `.field` assert form.
