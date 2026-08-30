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

plus (second trial commit, see §7) the sprite-tilt fixture re-stamp `build.sh` demands.

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

_(filled in below as the runs complete)_

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
