# ROM-as-sentinel retirement — packet

Branch `fix/rom-sentinel-port-tests`, off master `8884e255`.

The defect: a port test decides "is there an aeon tree here at all" by testing whether
`$AEON_DIR/s4.bin` EXISTS. Neither question is the other. A **source-only** aeon tree —
every `.emp` present, nothing built — answers "yes" to the question the test needs and
"no" to the one it asks, so the gate panics `aeon tree missing` at a tree that is
entirely there. The precedent fix is `58632984`
(`native_object_bank_budget`), which re-sentinelled on `map.toml` and proved it both ways.

---

## 1. Enumeration

Three passes, each enumerating over a **different parameter**. Stating the parameter is
the point: two passes that share one agree with each other and are wrong together.

### Pass 1 — parameter: the ARTIFACT'S NAME, as literal text

```
git grep -n -E 's4\.bin|demo\.bin|s4\.lst|demo\.lst|\.p\b' -- '*.rs'
```

243 hit lines; 82 test files under `crates/*/tests` name a built ROM at all.

**This pass has a hole, and it fired.** `s4\.bin` does not match `s4.debug.bin`, so the
pass silently missed `compression_selftest_port.rs:291`. It also produces heavy false
positives a name grep structurally cannot resolve: `boot_data_port.rs:103`
`golden("s4.bin")` reads **sigil's own** `crates/sigil-harness/golden/s4.bin`, not the
aeon tree — same eight characters, different repository.

### Pass 2 — parameter: the GUARD'S IDENTIFIER, whatever file it names

```
git grep -n 'reference_tree(' -- '*.rs'
git grep -n -E 'fn (have_aeon|aeon_present|have_ref|reference_gate|read_ref|golden|ref_window|gate)\b' -- 'crates/*/tests/*.rs'
```

Starts at `crates/sigil-harness/src/test_support.rs` (`aeon_dir`, `reference_tree`) and
walks every caller. Catches a test that goes through a shared helper and names no file.

**This pass has the opposite hole.** Five of the eleven targets spell the check inline
with no named guard at all (`demo_native_port`, `error_handler_port`, `vectors_port`,
`soundbankhead_port`, `repin_pins`), so a helper-name grep does not see them. Neither
pass is a superset of the other; they were run and reconciled.

### Pass 3 — parameter: the PRESENCE-TEST OPERATION, independent of name and helper

```
git grep -n -E '\.exists\(\)|fs::metadata' -- 'crates/*/tests/*.rs' 'crates/*/src/test_support.rs'
git grep -n -E 'try_exists|File::open|is_ok\(\)' -- 'crates/*/tests/*.rs'   # other idioms: none
```

This is the pass that closes the enumeration, and it is the one that caught the eleventh
file. It also **proves the classification below is complete**: the defect can only live at
a site that tests for existence, and this pass enumerates every such site in the test tree
regardless of what it names or who wrote it. Of them, exactly twelve named a built ROM —
eleven remaining plus the one master already fixed.

---

## 2. Classification

| Class | What it is | Count | Verdict |
|---|---|---|---|
| **A** | The ROM is the SUBJECT — the test reads its bytes and compares. `s4.bin` is not a sentinel there. | **71 files** (82 naming a ROM, minus the 11 below) | Correct as they are. Untouched. |
| **B** | ROM-as-sentinel — needs aeon SOURCE only, uses the ROM's existence as a proxy for "tree present". | **11 files / 35 tests** | Fixed. Table below. |
| **C** | Fits neither. | **2 findings** | Reported, not fixed. Below. |

Class A's count is not a residue of subtraction: pass 3 enumerated every presence check in
the test tree, and no file outside the eleven contains one that names a ROM. So in all 71,
every ROM mention is a READ.

### Class B — the eleven, with the sentinel each now carries

Every replacement is a file the test genuinely reads, and for the profile-driven gates it
is **derived from the profile the test itself builds** (`GameProfile::game_root_rel` and
its `map.toml` sibling via `GameProfile::map_path`) rather than hardcoded — so the guard
cannot drift from the inputs the gate uses.

| File | Tests | Was | Now | Why the ROM was never a prerequisite |
|---|---|---|---|---|
| `crates/sigil-cli/tests/boot_data_port.rs` | 2 | `aeon/s4.bin` exists | profile sources | Builds the shape from source; compares against **sigil's own** `golden/s4.bin` |
| `crates/sigil-cli/tests/keystone_flip_relocation.rs` | 3 | `aeon/s4.bin` exists | profile sources | Compares against sigil's `golden/config_a.bin` |
| `crates/sigil-cli/tests/native_offcanonical_full.rs` | 9 | `aeon/s4.bin` exists | profile sources | Compares against `golden/provenance.toml` + committed blobs |
| `crates/sigil-cli/tests/native_offcanonical_placement.rs` | 9 | `aeon/s4.bin` exists | profile sources | Placement/derivation only; committed `offcanonical_sizes/*.txt` |
| `crates/sigil-cli/tests/native_offcanonical_rom.rs` | 5 | `aeon/s4.bin` exists | profile sources | Its own `golden()` comment already said the live artifact is NOT read |
| `crates/sigil-cli/tests/demo_native_port.rs` | 2 | `aeon/s4.bin` exists | demo profile sources | Builds demo, compares against `golden/demo*.bin` — and sentinelled on the **sonic4** ROM, a tree it does not even build |
| `crates/sigil-cli/tests/error_handler_port.rs` | 1 | `aeon/s4.bin` exists | `reference_tree(["engine/debug/error_handler.emp"])` | Lowers one `.emp` and checks label resolution |
| `crates/sigil-cli/tests/vectors_port.rs` | 1 | `aeon/s4.bin` exists | `reference_tree(["engine/system/vectors.emp", "engine/debug/error_handler.emp"])` | Same — two `.emp`s compiled together |
| `crates/sigil-cli/tests/soundbankhead_port.rs` | 1 | `aeon/s4.bin` exists | sonic4 profile sources | `resolve_pinned_sections` is a source resolve |
| `crates/sigil-harness/tests/repin_pins.rs` | 1 | `aeon/s4.bin` exists | sonic4 profile sources | Post-P4c the listing is sigil's own resolve; the asl `.lst` parse is gone |
| `crates/sigil-cli/tests/compression_selftest_port.rs` | 1 | `aeon/s4.debug.bin` exists | `reference_tree(["engine/debug/compression_selftest.emp", "engine/debug/generated/compression_vectors.emp"])` | Both arms are placement/link-only |

Six of the eleven are per-file (every test in the file was affected); five are one test
inside a file whose OTHER tests genuinely read a ROM and are correctly class A. The unit of
this defect is the TEST, not the file.

The shared derivation lives once, in
`crates/sigil-harness/src/test_support.rs::reference_tree_for_profile`.

### Class C — the two things that fit neither bucket

**C-1. Ten files probe the aeon tree ROOT, not any file in it.**

```
git grep -lE 'if !aeon\.exists\(\)' -- 'crates/*/tests/*.rs'   # 10 files, 12 sites
```

`contract_closure_corpus` (×2), `dead_save_corpus`, `extra_entry`,
`movem_restore_guard_corpus`, `out_verify_corpus`, `parcel_8b_stage_gen_touchers`,
`preserves_corpus`, `slot_type_corpus`, `warn_tier_corpus`, `cfg_blind_spots` (×2).

Not the ROM defect — these run correctly source-only, and all ten are already in
`SOURCE_GATES`. But `reference_tree`'s own doc comment names this shape as the thing it
exists to replace: *"`rels` are the aeon-relative paths the caller actually reads. Naming
them — rather than probing the tree root — is what makes the guard honest against an
`AEON_DIR` pointed at an incomplete tree."* A root probe passes against an empty directory
that happens to exist. Ledgered, not fixed here.

**C-2. The nightly lane's classifier has only two buckets, and these gates need a third.**

Section 4.

---

## 3. Verification — three directions, per changed test

No direction is generalised from another test. Every row below was executed.

**Direction 1 (the bug is fixed) — a source-only tree, `SIGIL_STRICT_GATE=1`.**

Run against `/home/volence/sonic_hacks/.aeon-rom-sentinel-src`: a `cp -a` copy of
`.aeon-sigil-gates` (aeon `1ee8f8e6`, no `*.bin`, no `*.lst`, `engine/*/generated`
prepared). A **private** copy because `ensure_generated` WRITES
`$AEON_DIR/engine/sound/generated` on every profile build — the shared gates tree was
never written to by this work. That write is pre-existing sanctioned lane behaviour (the
lane deletes and regenerates that directory each run, and `native_object_bank_budget`,
already in `SOURCE_GATES`, does the same), so it is not a blocker — but it is a write, and
the brief's read-only rule was honoured literally rather than argued around.

No `SIGIL_EMIT` was set for any of these runs; the sound-on resolve emits its generated
BINCLUDEs in-process.

| Test binary | Tests | Result | Time |
|---|---|---|---|
| `compression_selftest_port` (`retired_fit_lock_…`) | 1 | 1 passed / 0 failed | 0.14s |
| `error_handler_port` (`vector_labels_resolve_to_emp_ownership`) | 1 | 1 passed / 0 failed | 0.03s |
| `vectors_port` (`vector_labels_resolve_to_error_handler_emp`) | 1 | 1 passed / 0 failed | 0.03s |
| `boot_data_port` | 2 | 2 passed / 0 failed | 4.86s |
| `demo_native_port` | 2 | 2 passed / 0 failed | 0.65s |
| `keystone_flip_relocation` | 3 | 3 passed / 0 failed | 5.49s |
| `native_offcanonical_full` | 9 | 9 passed / 0 failed | 18.17s |
| `native_offcanonical_placement` | 9 | 9 passed / 0 failed | 8.24s |
| `native_offcanonical_rom` | 5 | 5 passed / 0 failed | 6.53s |
| `soundbankhead_port` (`soundbankhead_pinned_bootstrap_…`) | 1 | 1 passed / 0 failed | 1.81s |
| `repin_pins` (`pins_rs_is_current`) | 1 | 1 passed / 0 failed | 3.72s |
| **Total** | **35** | **35 passed / 0 failed** | |

Each did its real work — `native_offcanonical_full` spent 18s building five ROM shapes
with no ROM anywhere in the tree. On master every one of these 35 panics
`aeon tree missing` there.

**Direction 2 (the guard still guards) — `AEON_DIR=/home/volence/sonic_hacks/.aeon-absent-xyz`, a path that does not exist.**

Under `SIGIL_STRICT_GATE=1`, all 35 FAIL. Exit 101. The message names the missing path, so
a real absence cannot be mistaken for the fix:

```
SIGIL_STRICT_GATE set but reference missing: /home/volence/sonic_hacks/.aeon-absent-xyz/games/sonic4/game_root.asm
SIGIL_STRICT_GATE set but reference missing: /home/volence/sonic_hacks/.aeon-absent-xyz/games/demo/game_root.asm
SIGIL_STRICT_GATE set but reference missing: /home/volence/sonic_hacks/.aeon-absent-xyz/engine/debug/error_handler.emp
SIGIL_STRICT_GATE set but reference missing: /home/volence/sonic_hacks/.aeon-absent-xyz/engine/system/vectors.emp
SIGIL_STRICT_GATE set but reference missing: /home/volence/sonic_hacks/.aeon-absent-xyz/engine/debug/compression_selftest.emp
```

| Test binary | Strict result |
|---|---|
| `boot_data_port` | FAILED. 0 passed; **2 failed** |
| `demo_native_port` | FAILED. 0 passed; **2 failed** |
| `keystone_flip_relocation` | FAILED. 0 passed; **3 failed** |
| `native_offcanonical_full` | FAILED. 0 passed; **9 failed** |
| `native_offcanonical_placement` | FAILED. 0 passed; **9 failed** |
| `native_offcanonical_rom` | FAILED. 0 passed; **5 failed** |
| `compression_selftest_port` (1 test) | FAILED. 0 passed; **1 failed** |
| `error_handler_port` (1 test) | FAILED. 0 passed; **1 failed** |
| `vectors_port` (1 test) | FAILED. 0 passed; **1 failed** |
| `soundbankhead_port` (1 test) | FAILED. 0 passed; **1 failed** |
| `repin_pins` (1 test) | FAILED. 0 passed; **1 failed** |

The non-strict arm was proven separately, because the nightly lane greps stdout for
`skip:` and a guard that went silent would read as coverage. Same absent tree, no
`SIGIL_STRICT_GATE`:

```
skip: reference not at /home/volence/sonic_hacks/.aeon-absent-xyz/games/sonic4/game_root.asm (set AEON_DIR)
skip: reference not at /home/volence/sonic_hacks/.aeon-absent-xyz/games/demo/game_root.asm (set AEON_DIR)
```

Two things worth stating. First, this is the failure mode that looks identical to a fix —
a test that stopped panicking because it stopped checking — and it is ruled out here by
the message naming a specific path that the direction-1 tree DOES have. Second, the
strict-arm improvement is not only in *what* is checked: `error_handler_port` and
`vectors_port` previously wrote `if !strict_gate() && !aeon.join("s4.bin").exists()`, which
under strict skipped the guard entirely and let the test fall through to a raw
`read_error_handler.emp: No such file` panic. They now fail through the shared guard with
the standard message.

**Direction 3 (no regression) — the full built tree, `AEON_DIR=/home/volence/sonic_hacks/.aeon-landing` (aeon `1ee8f8e6`, all shapes present).**

Covered by the full-suite run in section 5; the per-test `grep -c` counts are there.

---

## 4. The nightly source-gate lane

Audit replayed against this branch by reading the `SOURCE_GATES` array out of
`scripts/nightly_source_gates.sh` (not out of prose) and re-running the lane's own
classification loop:

| Tree | Result |
|---|---|
| this branch | **`gates=35 unclassified=0`** |
| `.sigil-source-gates` (master as the lane last checked it out) | `gates=34 unclassified=0` |

The 34→35 delta is `native_object_bank_budget`, which `58632984` added; it is not
something this branch changed. **M = 0.** No file changed classification: every one of the
eleven still names a built ROM or a golden somewhere in its own text, either because its
other tests genuinely read one, or because it compares against `crates/sigil-harness/golden`.

### `SOURCE_GATES` decisions — none added, and the reason is the oracle, not the sentinel

The tempting reading is "these are source-only now, so add them". Measured, cost is not the
obstacle it was thought to be: all eleven together run in **~50s** on a source-only tree,
not the "order of magnitude longer" the lane's comment estimates. The real obstacle is what
each gate is measured AGAINST.

Every one of the eleven is oracle'd on a **committed sigil artifact** — a frozen
`golden/*.bin` blob, `golden/provenance.toml`, or `src/pins.rs`. Their *inputs* would run
in this lane; their *expectations* would not. Between an aeon parcel that legitimately
moves bytes and sigil's refreeze of the artifact they compare against, they are red **by
design**. A nightly clock would report that window every time, which is the cry-wolf the
lane's EXCLUDED note exists to prevent — and it is the refreeze ritual's window, already
owned. Fixing the sentinel changes how a gate finds the tree; it does not change what the
gate is measured against.

`repin_pins` is the case worth naming explicitly, because it argues hardest for inclusion:
it is now wholly source-only in its aeon-reading part, it costs 3.72s, and pins.rs
staleness is exactly source-derived drift. It is still excluded, on the same ground —
`pins.rs` is refreshed by `repin` as part of the byte-movement ripple, so a nightly would
go red for the length of that ripple.

One line-level change to the lane, comment only: the EXCLUDED block now names this third
shape (source-only inputs, artifact oracle) so the next reader does not have to
re-derive it, and records that a file in this shape which stops naming its artifact becomes
UNCLASSIFIED and takes the lane to exit 2 — loud, and the safe direction. No gate list
changed; `unclassified` is still 0.

**The classifier's real weakness, ledgered as C-2:** it decides "artifact-dependent" with
`grep -qE 's4\.bin|…|golden'` over the file's text, which cannot tell a USE from a MENTION.
`repin_pins` now classifies partly on `.lst` appearing in explanatory comments about where
values came from. That is a mention. Its stale header comment — which claimed the test
needs `s4.lst`/`s4.debug.lst`, untrue since the P4c re-point — was corrected here rather
than left standing as load-bearing grep bait.

---

## 5. Full suite

Bar: `cargo test --release --workspace --no-fail-fast`, `AEON_DIR=.aeon-landing`,
`SIGIL_STRICT_GATE=1`. The log is stamped BEFORE cargo writes to it, because cargo prints
no cwd, no branch and no HEAD, and a run launched from the wrong tree produces a green
plausible log about somebody else's branch.

```
### pwd=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a19f801c9ae459301
### head=71d10a7a6a0caabd761a040f82f15f2f8c7d8b58
### branch=fix/rom-sentinel-port-tests
### AEON_DIR=/home/volence/sonic_hacks/.aeon-landing
### aeon_head=1ee8f8e68d826b18023639ab32a8f7c82f238e62
### SIGIL_STRICT_GATE=1
### started=2026-08-22T20:12:00-04:00
### exit=0
### finished=2026-08-22T20:14:00-04:00
```

`71d10a7a` is the last code commit; only docs and one comment in the lane script land after
it, so the run covers the whole code change.

| | Branch | Master's bar |
|---|---|---|
| passed | **3821** | 3821 |
| failed | **0** | 0 |
| ignored | **4** | 4 |
| `skip:` lines | **0** | 0 |
| test binaries | 336 | — |
| exit | 0 | — |

Failures-first check on the log: `grep -nE '^failures:|FAILED|panicked'` returns nothing.

**Reconciliation against the tree, not against the remembered number:**

```
git grep -c '#\[test\]' HEAD -- '*.rs' | awk -F: '{s+=$NF} END {print s}'   →  3825
```

`passed + ignored` = 3821 + 4 = **3825**, and the declared count is **3825**. Exact. This
branch adds and removes no tests (the working tree also counts 3825), so the declared total
is unchanged from master, and master's bar reconciles identically.

**Per-test presence in the stamped log.** All 35 changed tests grepped for as
`^test <name> \.\.\. ok$`, each required ≥ 1:

```
names=35 missing_or_not_ok=0
```

Each of the 35 returned exactly 1:

`config_b_boot_data_hole_filled` · `s4_boot_data_blob_present` ·
`flipped_config_a_anchor_matches_golden` · `doctored_golden_at_deform_pointer_is_caught` ·
`deform_pointer_equals_placed_label_vma` · `config_a_full_file` · `config_b_full_file` ·
`demo_plain_full_file` · `demo_debug_full_file` · `lean_full_file` ·
`config_a_doctored_control` · `config_b_doctored_control` · `demo_doctored_control` ·
`lean_doctored_control` · `config_b_frozen_placement_exact` ·
`demo_size_table_rederives_native` · `demo_debug_size_table_rederives_native` ·
`config_a_size_table_rederives_native` · `config_b_size_table_rederives_native` ·
`lean_size_table_rederives_native` · `ram_packing_invariants_plain` ·
`ram_packing_invariants_debug` · `config_b_doctored_size_table_breaks_the_build` ·
`config_b_anchor_matches_golden` · `config_a_anchor_matches_golden` ·
`demo_plain_anchor_matches_golden` · `demo_debug_anchor_matches_golden` ·
`lean_anchor_matches_golden` · `demo_plain_game_modules_match_golden` ·
`demo_debug_game_modules_match_golden` · `vector_labels_resolve_to_emp_ownership` ·
`vector_labels_resolve_to_error_handler_emp` ·
`soundbankhead_pinned_bootstrap_lands_at_lma_not_vma` · `pins_rs_is_current` ·
`retired_fit_lock_stays_silent_and_operands_width_select_past_abs_w`

That is Direction 3 for all 35: unchanged against the full built tree.

**One process note worth keeping.** The first pass of this run produced a green
3821/0/4 log stamped `branch=worktree-agent-a19f801c9ae459301`, `head=8884e255` — the
worktree's generated branch name, and master's HEAD, because the work was not yet
committed. The numbers were correct and the log was about the wrong thing. The stamp is
what made that visible; the suite was re-run after the branch rename and the three code
commits.

---

## 6. Was the booked count of ten right?

**No. The real number is eleven files / 35 tests.**

The miss is `crates/sigil-cli/tests/compression_selftest_port.rs`, and the reason is
mechanical rather than a matter of judgement: it sentinels on **`s4.debug.bin`**, and a
regex written as `s4\.bin` does not match `s4.debug.bin`. Any enumeration that started
from the canonical filename — which is the obvious way to start, and is how the number
ten was almost certainly produced — returns ten and looks complete. It took a pass that
enumerated over the presence-check OPERATION (`.exists()`), with no filename in the
pattern at all, to surface the eleventh.

The booked number was not stale. It was produced by one correct-looking pass whose
parameter had a blind spot, and re-running that same pass would have confirmed it forever.
That is the whole argument for naming the parameter you enumerate over.

The count of ten was also right about something the fix had to preserve: eleven **files**
is not eleven **tests**. Five of the eleven carry the defect in one test inside a file
whose other tests are correctly artifact-dependent, so a file-level fix would have broken
real gates. The unit is the test.
