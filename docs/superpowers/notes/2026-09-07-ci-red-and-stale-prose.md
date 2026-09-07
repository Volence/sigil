# 2026-09-07: CI red (three causes, not one) and the seam2 stale-address prose

Branch `parcel/ci-red-and-stale-prose` off master `99aa2645`. Commits:
`23d76c8d` (item 1), `6106e2af` (item 2), then this note. Builds went to
`CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-cired`; no emulator, no
aeon tree, no `SIGIL_BUILD`/`SIGIL_EMIT` were used.

## Item 1, `sig-ci-red`

### The brief's derived cause is false, measured

The brief derived that two rows of `crates/sigil-harness/tests/bare_run_refuses.rs`
panic under CI's `SIGIL_ALLOW_PARTIAL=1`. Locally, with no `AEON_DIR` and no
`EMPYREAN_SUITE_ROOT` in the environment:

```
$ cargo test -p sigil-harness --test bare_run_refuses -- --nocapture
running 3 tests
test the_child_opens_a_reference_dependent_gate ... ok
test strict_and_partial_together_are_refused ... ok
test a_bare_run_refuses_and_a_declared_partial_run_says_its_size ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
exit=0

$ SIGIL_ALLOW_PARTIAL=1 cargo test -p sigil-harness --test bare_run_refuses -- --nocapture
running 3 tests
test the_child_opens_a_reference_dependent_gate ... ok
test strict_and_partial_together_are_refused ... ok
test a_bare_run_refuses_and_a_declared_partial_run_says_its_size ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
exit=0
```

Both directions are green because the test builds every child's environment
from scratch (`env_remove` on all four resolver variables), so the workflow's
variable never reaches the child. The refusal at `test_support.rs:1204` that the
brief quoted is the child's EXPECTED refusal in direction 1; it is not what fails.

### What the CI logs actually say (`gh run view <id> --log-failed`, gh authenticated as Volence)

`gh run list --limit 6`: every listed run is `failure`; the run at this parcel's
base `99aa2645` (id 34125114423) was in progress at boot and completed red.

| date | run / sha | failing step and rows, quoted |
|---|---|---|
| 08-26 | 33025062471 / c0fda952 | last SUCCESS. None of the three binaries below existed yet. |
| 08-27 | 33093764297 / 0334be9c | clippy step: `error: this creates an owned instance just for comparison` at `harness_root.rs:163:35`, `error: useless use of format!` at `harness_root.rs:246:17`, `could not compile sigil-harness`. Fixed on master 2026-09-02 (`e4a5d24d`). |
| 09-04, 09-06 | 33928112689 / 3ddaf27a, 34067586210 / 43bf606a | test step: `a_bare_run_refuses_and_a_declared_partial_run_says_its_size` panicked at `bare_run_refuses.rs:142:5` ("the refusal must say it DECLINED a tree it could have used"). CI's child printed: `NO REFERENCE TREE IS NAMED ... Nothing was derived either. The resolver's own answer: ... derivation from this checkout's own location: /home/runner/work/sigil is this repository's parent but holds no aeon/ + empyrean/`. |
| 09-07 | 34125114423 / 99aa2645 | test step: `org_minus_one_is_refused_not_a_four_gib_allocation` and `org_minus_one_with_hex_is_refused_before_rendering` panicked at `image_bounds.rs:43:5`; the child `sigil` process: `thread 'main' panicked at crates/sigil-link/src/relax.rs:193:43: attempt to add with overflow`. |

So "red since 08-27" is three causes in sequence, and `cargo test` is fail-fast
per binary: the 09-07 red (sigil-cli) stops the run before the harness binaries,
which is why the current log no longer shows the 09-04 row at all.

### Cause A, current tip: a debug-profile overflow in the linker

Reproduced locally in the debug profile (the profile CI uses):

```
$ SIGIL_ALLOW_PARTIAL=1 cargo test -p sigil-cli --test image_bounds
thread 'main' (…) panicked at crates/sigil-link/src/relax.rs:193:43:
attempt to add with overflow
assertion `left == right` failed: expected exit 1, got ExitStatus(unix_wait_status(25856))
test result: FAILED. 4 passed; 2 failed
```

and green in release (`cargo test --release ... --test image_bounds`: 6 passed),
because release wraps. The landing lane runs release and never saw it. The gate
itself (`image_bounds.rs`, added 2026-09-07 in `5247f1a6`/`f28aaa33`) is right:
it exists to prove `org -1` is refused BY NAME with exit 1, not aborted.

Fix (`relax.rs`): the four `u32` LMA sums on the placement path saturate:
the group cursor advance (`:193`, the site CI hit), `overlap_diag`'s range end
(`:294`, the site hit next once 193 was fixed), and the two bank-path sums
(`:183` straddle test, `:244` `bank_diag` end). An address past the top of the
space stays there and `check_image_bounds` names it. No valid program reaches
saturation (the cartridge window ends at `0x400000`), so ROM bytes cannot move;
the controller's landing gate proves that. Red-first: the committed baseline is
red in debug (2 failed) and `6 passed` after. `sigil-link` (134 rows) and
`sigil-ir` (49) pass. Note: sites 183 and 244 are on the `bank:` path, which
`image_bounds` does not exercise; they saturate by the same argument but have
no dedicated red-first row.

### Cause B, 09-04 and 09-06: an assertion that encoded the developer's machine

`bare_run_refuses.rs:142` required the literal `DECLINED to use`, which
`bare_run_refusal` prints only when step 3 DERIVED a sibling tree. On a runner
holding this repository alone there is no sibling to decline, so no run
configuration can produce that text. The brief's "fix it in how CI runs them"
has exactly one workflow-side shape: create empty `aeon/` and `empyrean/`
directories beside the checkout so step 3 "derives" an empty directory and the
refusal declines nothing. I judged that worse than correcting the test, and did
the latter; the controller can reverse it, the alternative is two `mkdir`s.

Fix (`bare_run_refuses.rs`): the parent runs the child's own step-3 mechanism
(`derive_suite_root_from(CARGO_MANIFEST_DIR)`, the same anchor `derived_suite_root`
uses) and pins the ONE spelling this environment owes. Beside a suite root the
refusal must contain `DECLINED to use <derived aeon path>` (stricter than before,
which matched the bare phrase). With no suite root it must contain
`Nothing was derived either.` AND `this checkout's own location` (the resolver's
own step-3 reason), so a resolver that broke still cannot read as a runner with
no suite. Nothing about the refusal, the opt-in, or the skip accounting changed.

Red-first in a CI-shaped environment: `git clone` of the branch into
`/home/volence/sonic_hacks/.target-cired/ci-sim/sigil`, whose parent holds no
`aeon/` + `empyrean/`. Baseline there fails at `:142` with CI's exact text
(`... /home/volence/sonic_hacks/.target-cired/ci-sim is this repository's parent
but holds no aeon/ + empyrean/`); with the patch copied in (diff shown applied:
`+    let derived = derive_suite_root_from(...)`) it passes 3/3. In this worktree
(suite root present) it passes 3/3 through the DECLINED arm.

### The workflow diff

```
           set -o pipefail
+          # --no-fail-fast: every test binary runs, so a red run names EVERY red
+          # binary instead of the first one cargo reached. The exit status is
+          # still non-zero when any binary failed. Without it, one red binary
+          # early in the crate order hid a second, independent red for days.
-          cargo test --workspace -- --nocapture 2>&1 | tee test-output.txt
+          cargo test --workspace --no-fail-fast -- --nocapture 2>&1 | tee test-output.txt
```

Why this is the honest shape: the two rows that were red are fixed in the code
they live in, the refusal tests are untouched in what they refuse, and the only
workflow change makes a red run list every red binary (this parcel needed three
log fetches to find what one log with `--no-fail-fast` would have shown).
Nothing else in the workflow was wrong; its comment block is accurate.

### The verbatim CI command, run where CI runs it (as far as this machine allows)

GitHub Actions cannot run here. The test step's command was run verbatim in the
CI-shaped clone at the branch tip (`pwd=.../ci-sim/sigil HEAD=6106e2af
branch=parcel/ci-red-and-stale-prose`), `AEON_DIR` and `EMPYREAN_SUITE_ROOT`
unset, `SIGIL_ALLOW_PARTIAL=1`, `cargo test --workspace --no-fail-fast -- --nocapture`:

```
binaries run: 404      passed total: 4757     exit=101
skip lines: 400        PARTIAL RUN banners: 117 ("128 test binaries are reference-dependent")
FAILED rows (3, in 2 binaries):
  tests/reference_tree_named_write.rs
    a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it
      -> :133 UNMEASURABLE: with AEON_DIR removed the resolver names no default tree
         (... ci-sim is this repository's parent but holds no aeon/ + empyrean/ ...)
  tests/suite_paths_precedence.rs
    the_resolver_follows_the_contract_precedence
      -> :177 expected a resolved answer, got: RESULT err no aeon checkout could be resolved ...
    the_step_3_derivation_is_proven_from_a_linked_worktree
      -> :523 step 3 cannot derive from this crate's own location: ... not a suite root
```

The skip reporter step would pass (400 > 0). **CI will still be red after this
parcel, on exactly those three rows.** They are a third class the d-18 contract
does not name: not reference-tree readers (which the partial run leaves
unmeasured and counts) but SUITE-ROOT readers, proofs of the resolver's step 3
that can only be measured where a sibling checkout exists. Their doctrine is
"UNMEASURABLE is loud", which on a runner with no suite means red forever. The
write guard arrived 2026-08-30 (`92e3d123`), the precedence proofs 2026-09-02
(`2188f19a`); neither has ever run green in CI, and both were hidden behind
fail-fast. **BLOCKED on a design call**: either the partial run also leaves
suite-root-dependent rows unmeasured (and says so, and the derived count learns
the class), or CI provisions a suite root, or these three rows are accepted as
red on GitHub. That is an owner/controller ruling, not a parcel edit.

### The nightly source-gate lane (`~/.local/state/sigil-source-gates/nightly.log`)

The script writes `nightly.log` (one summary line per run) and `gates.log` (the
full cargo output, OVERWRITTEN each run, so only the latest run's text survives).

- 09-04: `SOURCE GATES FAILED at sigil e219bc58 / aeon 7c1a0f78, 2 failed / 183 passed: fixture_values_match_the_live_structs harvest_emits_the_as_field_offsets_and_sizes`
- 09-05: `OK at sigil b02f5d0b / aeon 9e3d2861 (185 passed, 46 gates; ...)`
- 09-06: `SOURCE GATES FAILED at sigil e9a9dfa6 / aeon a7a4f640, 1 failed / 185 passed: misspelled_objroutine_target_dangles_while_control_resolves`
- 09-07: `SOURCE GATES FAILED at sigil 43bf606a / aeon 27ac8cc9, 8 failed / 178 passed: corpus_context_requirements_are_satisfied_the_error_gate corpus_flag_results_are_all_consumed dac_bank_payloads_match_disk_fixtures doctored_shift_count_produces_different_bytes_than_genuine misspelled_objroutine_target_dangles_while_control_resolves snd_equ_values_match_s4lst_baseline standalone_compile_without_cross_seam_labels_is_a_loud_missing_symbol_error wrong_base_map_places_the_section_at_a_different_address`

Row text from the 09-07 `gates.log` (the 09-06 row re-failed on 09-07, so its
text is this):

- `misspelled_objroutine_target_dangles_while_control_resolves` (`tranche6_negative_probes.rs:209`): `unexpected lower errors: unknown name PlayerV.ground_speed ... unknown name PlayerV.move_lock ... unknown name DEBUG ...` (aeon renamed fields the probe's fixture source still names).
- `corpus_context_requirements_are_satisfied_the_error_gate` (`contract_closure_corpus.rs:1170`): `the with bracket census moved, corpus adoption changed. Update deliberately: [...] left: 21 right: 20`.
- `corpus_flag_results_are_all_consumed` (`contract_closure_corpus.rs:850`): `shape sonic4 debug: unexpected flag-result firings (a dropped carry?): [FlagFiring { proc: "Parallax_Update", callee: "Parallax_InstallScratch", flag: "carry", ... kind: Unused }]`.
- `snd_equ_values_match_s4lst_baseline` (`dac_port.rs:259`): `SND_S3K_SNARE_PTR: expected 0x9512, got 0x857E`.
- `dac_bank_payloads_match_disk_fixtures` (`dac_port.rs:194`): `read dac/s3k_snare.pcm: No such file or directory`.
- `doctored_shift_count_produces_different_bytes_than_genuine`, `wrong_base_map_places_the_section_at_a_different_address`, `standalone_compile_without_cross_seam_labels_is_a_loud_missing_symbol_error` (all `dplc_negative_probes.rs:131`): `lower errors: unknown name FRAME_PIECE_COUNT`.

Every one is aeon-source drift (aeon `a7a4f640` on 09-06, `27ac8cc9` on 09-07:
a DAC sample removed, `FRAME_PIECE_COUNT` gone, `PlayerV` fields renamed, a new
`with` bracket, an unconsumed carry). None is this repo's prose and none is a
one-line fix here; nothing was changed. For the controller.

## Item 2, `sig-stale-prose-addrs`

Population, enumerated twice and reconciled. Grep A, the old bank's addresses
(`\$5[0-9A-F]{4}` / `0x5[0-9A-F]{4}`) over `crates/sigil-cli/tests/seam2_*.rs`;
grep B, the five head symbols (`SoundTablesZ80_Head`, `SndDefaultPitchTable` /
`movingtrucks_pitchtable`, `SfxBlobWinTab`, `SeqOpcodeTable`, `DacSampleTable`).
Grep A hit EIGHT seam2 files, not seven; grep B added no file A had missed. The
addresses were doubly stale: the bank moved (`$58000` to `$B8000`) and the
in-bank offsets moved too (`seq_opcode_tab_lma` is `0xB8571`, not `+0x56D`).

| file | was | now |
|---|---|---|
| `seam2_soundtables_colink.rs` | `$58000` in module doc, gate doc, final assert | names `SoundTablesZ80_Head` / `sound_tables_z80_lma`; assert prints `@ {lma:#X} (SoundTablesZ80_Head)` from the value the slice is cut at |
| `seam2_pitchtable.rs` | `$58357`, same three sites | `@ {lma:#X} (SndDefaultPitchTable)` |
| `seam2_seq_colink.rs` | `$5856D`, same three sites | `@ {lma:#X} (SeqOpcodeTable)` |
| `seam2_dac_head_colink.rs` | `$585AD` (3), VMA `$85AD`, "90 bytes" (it is 127), first-difference panic with a stride of 9 (emitter uses 12) | names `dac_sample_tab_lma` / `DAC_SAMPLE_TAB_LEN`; panic prints byte index, ROM address `lo + i`, 8-byte window, no stride; assert `@ {lma:#X} (DacSampleTable)` |
| `seam2_sfx_head_colink.rs` | `$5845F` (3), `$5BAE8`/`$5D53A`, "270 = 135 x 2" (const is 274), a change-history comment, "bank $B ($58000..$5FFFF)" | `@ {win_lma:#X} (SfxBlobWinTab)`; present-tense doc; the +$100 control now asserts the moved base shares `bankid` with the real one |
| `seam2_phased_head.rs` | `$58000` (5) and the synthetic map's literal `lma_base = 0x5856D`, pinned by the assert | map built from `sound_layout().seq_opcode_tab_lma`; assert compares the placed LMA to that derived value |
| `seam2_colink_probe.rs` | `$48000/$50000` and a bank-id history note | names the derivation |
| `seam2_dac_emit.rs` | "`$90000/$98000` since the re-layout, `$48000/$50000` before it" (the "current" pair was itself one relayout stale; the anchor is `0xA8000`) | names `Dac_Temp_Blip` in the frozen size table |
| `crates/sigil-harness/src/seam2.rs` (the doc the tests send a reader to; outside the brief's seven) | `BankAnchors` and `SoundLayout` field docs carried seven old addresses and a "+$48000 since the re-layout" note (the delta is +$60000 today) | name the symbol and `golden/offcanonical_sizes/*.txt` |

Residual after the edit: grep A over `seam2_*.rs` returns nothing. No line this
parcel added carries an em or en dash (checked over `git diff -U0`).

Results: the nine seam2 binaries under `SIGIL_ALLOW_PARTIAL=1`: 28 rows, all
pass, all of them unmeasured skip-as-pass (no tree, expected; the controller's
full gate measures them). `cargo clippy --release -p sigil-cli --all-targets -- -D warnings`:
clean (`Finished release`; the only warnings printed are the vendored clownlzss
C++ build-script notes, not clippy). `cargo clippy --workspace --all-targets -- -D warnings`
(CI's command): clean.

## Open, and why

1. **CI stays red on three suite-root-dependent rows** (above). Design ruling needed.
2. `seam2_phased_head.rs` now builds its map from `sound_layout`; compiled and
   run as a partial row only. Not run under `SIGIL_STRICT_GATE` here (rule 6).
3. Stale old-bank prose outside the brief's population, untouched: in
   `crates/sigil-harness/src/seam2.rs` at lines 17, 22, 24, 41, 409, 413, 423,
   456, 501, 503, 608, 645, 671, 673, 697, 754, 810, 892, 971, 1029, 1083,
   1113, 1124 (docs and comments; 1292/1301 are a unit test's own synthetic
   map, correct as written). Non-seam2 sigil-cli test files carrying old-bank
   addresses, by file: `dac_bank_port.rs`, `dac_port.rs`, `error_handler_port.rs`,
   `header_port.rs`, `keystone_flip_relocation.rs`, `mt_negative_probes.rs`,
   `mt_port.rs`, `native_offcanonical_placement.rs`, `sfx_bank_port.rs`,
   `sfx_port.rs`, `ports.rs`, `sound_migration_negative_probes.rs`,
   `soundbankhead_port.rs`, `section_alignment_declared.rs`,
   `sfx_negative_probes.rs`. Some of these are synthetic-layout oracles whose
   literals are correct by construction; each needs its own read.
4. Pre-existing em dashes remain in the touched files (54 across the nine seam2
   tests, 76 in `seam2.rs`); only lines this parcel wrote are dash-free.
5. `relax.rs` sites 183 and 244 (bank path) saturate without a red-first row.
6. `gates.log` is overwritten per run, so the 09-06 row's own text is gone; the
   text quoted is the 09-07 re-failure of the same row.

## What in the brief was wrong

- The derived cause (refusal tests panic under `SIGIL_ALLOW_PARTIAL=1`): false;
  they scrub the environment. Their CI red was the DECLINED assertion on a
  runner with no suite root; the tip's red is a linker overflow in debug.
- "The fix is in how CI runs them": no run configuration produces the text
  that assertion required; only a fabricated suite root would.
- "Both items are byte-neutral (CI config and test prose)": item 1 changes
  linker code. Byte-neutral by argument, proven only by the landing gate.
- "Seven seam2 files": eight, plus the harness doc they cite.
- "Red since 2026-08-27": true as a date, but three causes in sequence, and
  the clone run shows a fourth cause (three rows) still behind them.
