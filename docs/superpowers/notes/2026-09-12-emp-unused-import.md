# A `use` naming nothing that exists: refused on every path (EMP-UNUSED-IMPORT-UNCHECKED), 2026-09-12

**Provenance.** Sigil base `3bc0d81a` on branch `parcel/emp-unused-import`; fix commits `b7ee95fc`,
`26e4bec9`, `5e841fc7` and `46e8ab20` (the last one fixes the three suite failures the parcel caused;
it changes no verdict and no emitted byte: a read-ledger routing, a source-gate row, a test fixture). Base binary: built clean at `3bc0d81a` by `scripts/provision-aeon-ref.sh`
(`sigil --version`: `3bc0d81a`, clean at capture), frozen as `sigil-base`. Final binary: built
clean at `5e841fc7` (`--version`: `5e841fc7`, clean at capture), frozen as `sigil-final`; the
`26e4bec9` pair it replaced is kept as `sigil-final-26e4` and is what "final" meant in any row
dated before the cache commit.
Reference tree: a private worktree of aeon at `ec640bcf`, provisioned by the script; positive
witness `repin --check` printed `pins.rs unchanged`, and both rebuild controls printed
`MATCHES THE GOLDEN` (`s4.bin` `b09ccd65/820229`, `s4.debug.bin` `1b7fe316/846529`). Aeon
`origin/master` read at `35f54923` (fetched this session). Every canary ran on a `.git`-less
rsync copy of the reference tree, never on the tree itself; every edited file was restored
from the reference tree and the restore proven with `cmp`. Routed finding:
`2026-09-11-aeon-lensz3-portcheck.md`, "Canaries on the prepared parcel" (canary A).

## 1. Reproduction and mechanism

### Minimal

| probe | binary | result |
|---|---|---|
| `sigil emp b.emp --root .` where `b.emp` is `use m.h.{NOPE}` and nothing reads `NOPE` | base | refused, `b.emp:3:1: error: module \`m.h\` has no \`pub\` name \`NOPE\``, rc 1 |
| `sigil emp solo.emp` (single file, no `--root`), `use nowhere.{NOPE}`, unread | base | **accepted**, `built: 2 bytes`, rc 0 |

So the language rule works where the resolve pass sees the line (`sigil emp --root`), and the
single-file path does not read a `use` at all.

### Real tree (canary A, on a copy of the `ec640bcf` reference tree)

| run | binary | result |
|---|---|---|
| control, copy untouched | base | rc 0, `b09ccd65/820229`, md5 equal to the provisioned `s4.bin`, 165 warnings |
| `animate.emp:40` `use engine.objects.frames.{refresh_piece_count_CANARY}` | base | **rc 0**, 0 `CANARY` mentions, same 165 warnings, ROM `cmp`-identical to the control |
| the same, plus `animate.emp:41` `use engine.objects.mapping_dsl.{NOPE_NONHELPER_CANARY}` | base | rc 1, the ONLY error is `animate.emp:41:1: [Error] module \`engine.objects.mapping_dsl\` has no \`pub\` name \`NOPE_NONHELPER_CANARY\``; line 40 still silent |
| line 40 alone | final | rc 1, `animate.emp:40:1: [Error] module \`engine.objects.frames\` has no \`pub\` name \`refresh_piece_count_CANARY\`` |

The third row is the mechanism probe: one binary, one build, one file, two unread imports that
differ only in whether the imported module is a comptime helper. The non-helper one is refused
by the resolve pass; the helper one never reaches it.

### Mechanism, from the code

`native::build_emp` (the whole-program build every `sigil build` shape runs) calls
`normalize_helper_imports(&mut manifest, COMPTIME_HELPERS, &[])` BEFORE
`build_program_open_embed_counted`. For every module it `retain`s away each top-level `use`
whose base is one of the fourteen `COMPTIME_HELPERS` (`engine.objects.frames` is one) and
prepends one glob `use <helper>.*` per helper instead. The author's `use` line is gone before
`ResolveEnv::build` runs, so `resolve_use`, which owns "module `X` has no `pub` name `n`", never
sees it. The glob binds every `pub` name of the helper, so a READ of a real helper name still
resolves, and a read of a missing one fails at the reader (`unknown function`, canary B). An
UNREAD bad name has nothing left to fail at. `publicize_helper_comptime` runs first and makes
every private comptime item of a helper `pub`, so even a `use` the rewrite kept would have
accepted a private helper name.

The note's code read was right about `resolve_use` and wrong only in which lines reach it: its
two hypotheses ("`is_exported` answered true" or "something after `:880` drops the Error") were
both false. The line was deleted before the pass, not dropped after it.

## 2. Every path a `use` travels

| path | what it does with a `use` | before | after |
|---|---|---|---|
| map build, non-helper module (`build_emp` to `build_program_*`) | resolve pass: `resolve_use` per reachable module, BFS refuses a missing module | checked | checked (unchanged) |
| map build, `use` of a `COMPTIME_HELPERS` module | `normalize_helper_imports` deletes the line, `publicize_helper_comptime` widens exports | **unchecked** (missing name and private name both accepted) | checked against the tree as written, before either rewrite, reported for the modules the build lowers |
| map build, helper `use` nested in a `section {}` body | the rewrite removes top-level lines only, so it reaches the resolve pass, against the publicized index | missing name checked, private helper name accepted | both checked (deduplicated against the resolve pass's own report). From the code; no probe planted a section-nested `use` |
| RAM harvest (`harvest_engine_ram_addresses`, `build_program_open_embed` after `publicize_helper_comptime(RAM_HELPERS)`) | resolve pass against a publicized index | missing checked; a private RAM-helper name accepted | unchanged here; the same modules are in `build_emp`'s closure in the same `sigil build`, which now refuses it (see section 6) |
| constants harvests (`harvest_engine_constants`, `harvest_game_constants`, struct offsets) | `eval_all_pub_consts` on one file; `use` lines are not read | unchecked | unchanged; each harvested module is also a map-build module, checked there |
| seam 1, resident Z80 modules (`lower_one`, bare `lower_module`) | `use_import_stubs` derives `extern proc` stubs for listed `pub proc`s and skips every other name; no resolve pass | **unchecked** (missing name, private name, missing module, any spelling) | checked by `resident_import_verdict` (five files resolved among themselves, the tree scanned for any other module), located errors, `Err` from the emitter |
| seam 2, banked tables (`lower_emp_file` and the three direct `lower_module` sites) | bare `lower_module`; `use` never read | **unchecked** | checked per file by `import_verdict`, located errors |
| `sigil emp --root` (closed `build_program`) | resolve pass | checked | checked (unchanged) |
| `sigil emp <file>` single file (`compile_emp`) | bare `lower_module`, no module universe | **unchecked** (`use nowhere.{X}` built) | a `use` of any other module refused; a `use` of the file's own module held to its `pub` names |
| port-test harnesses that lower one file (`animate_port` and kin) | bare `lower_module`, imports never consulted | unchecked | unchanged, and deliberately so: see section 7 |

## 3. Variant table

Every row is a real build of a `.git`-less copy of the `ec640bcf` reference tree: map rows are
`sigil build --game sonic4`, seam rows are `emit_sound_blob` (which runs seam 1 then every seam-2
emitter). "unread" means nothing reads the imported name. The grammar's `use` spellings are
whole (`use m`), blank (`use m._`), glob (`use m.*`) and list (`use m.{a, b}`, one or many lines,
trailing comma allowed); there is no alias spelling (`parser.rs`, `use_decl`: every listed name
is an `expect_ident`).

Base = `sigil-base` / `emit-base` (`3bc0d81a`). Final = `sigil-final` / `emit-final`: the seam
rows, canary A and row 18 were run at `26e4bec9` and again at the tip `5e841fc7` with identical
results (every refusal rc 1 with 0 panics, both valid rows rc 0, all three edited files restored);
rows 1 to 11 were run with the first commit's binary, whose map-build path the later commits do
not touch. The seam rows were also run on the first commit's binaries, which refused with the
same text but exited 101 (section 5).

| # | path | planted line (unread unless marked) | base | final |
|---|---|---|---|---|
| 1 | map, helper, list of one | `animate.emp:40` `use engine.objects.frames.{refresh_piece_count_CANARY}` | rc 0, silent | rc 1, `animate.emp:40:1` `module \`engine.objects.frames\` has no \`pub\` name \`refresh_piece_count_CANARY\`` |
| 2 | map, helper, grouped list | `{refresh_piece_count, NOPE_CANARY}` | rc 0, silent | rc 1, `:40:1` `... \`NOPE_CANARY\`` |
| 3 | map, helper, multi-line list, trailing comma | `{` / `refresh_piece_count,` / `NOPE_CANARY,` / `}` | rc 0, silent | rc 1, `:40:1` `... \`NOPE_CANARY\`` |
| 4 | map, helper, PRIVATE name | `use engine.vdp.{target_bits}` (a non-`pub` `comptime fn`) | rc 0, silent | rc 1, `:41:1` `module \`engine.vdp\` has no \`pub\` name \`target_bits\`` |
| 5 | map, helper, glob | `use engine.objects.frames.*` | rc 0 | rc 0 |
| 6 | map, helper, whole | `use engine.objects.frames` | rc 0 | rc 0 |
| 7 | map, helper, blank | `use engine.objects.frames._` | rc 0 | rc 0 |
| 8 | map, non-helper, missing name | `use engine.objects.mapping_dsl.{NOPE_CANARY}` | rc 1, `:41:1` `... has no \`pub\` name \`NOPE_CANARY\`` | identical |
| 9 | map, non-helper, private name | `use engine.objects.dplc.{perform_dplc}` | rc 1, `:41:1` `module \`engine.objects.dplc\` has no \`pub\` name \`perform_dplc\`` | identical |
| 10 | map, missing module: list, glob, blank | `use engine.objects.no_such_mod_canary.{X}` / `.*` / `._` | rc 1 each, `:41:1` `no module \`engine.objects.no_such_mod_canary\` found under the scan root` | identical |
| 11 | map, helper, missing name, READ | row 1 plus the call at `:299` renamed to match | rc 1, `animate.emp:299:9` `unknown function \`refresh_piece_count_CANARY\`` | identical: the reader fails first |
| 12 | seam 2 via `sigil build` | `dac_sample_tab.emp:70` `use engine.sound_constants.{NOPE_CANARY}` | rc 0, silent | rc 1, `emit_sound_blob (blob): engine/sound/dac_sample_tab.emp import errors:` `:70:1` |
| 13 | seam 1, list of one | driver `:102` `use engine.sound_fm.{NOPE_CANARY}` | rc 0, silent | rc 1, `z80_sound_driver.emp:102:1` `module \`engine.sound_fm\` has no \`pub\` name \`NOPE_CANARY\`` |
| 14 | seam 1, grouped list | `{Fm_NoteOn, NOPE_CANARY}` | rc 0, silent | rc 1, same line, `NOPE_CANARY` only |
| 15 | seam 1, PRIVATE name | `use engine.sound_fm.{Fm_ScratchPart}` | rc 0, silent | rc 1, `... has no \`pub\` name \`Fm_ScratchPart\`` |
| 16 | seam 1, missing module: list, glob, blank | `use engine.no_such_mod_canary.{X}` / `.*` / `._` | rc 0 each, silent | rc 1 each, `no module \`engine.no_such_mod_canary\` found under the scan root` |
| 17 | seam 1, valid import from outside the five | `use engine.sound_constants.{DAC_SAMPLE_COUNT}` | rc 0 | rc 0 |
| 18 | seam 1 via `sigil build` | row 13 | (n/a) | rc 1, `error: native build (sonic4 plain): emit_sound_blob (blob): resident sound modules: import check failed:`, 0 panics |
| 19 | seam 2, missing name / missing module | `dac_sample_tab.emp:70` `{NOPE_CANARY}` / `use engine.no_such_mod_canary._` | rc 0 each, silent | rc 1 each, `:70:1` |
| 20 | seam 2, valid | `use engine.types.{SongId}` | rc 0 | rc 0 |
| 21 | single file | `use nowhere.{NOPE}` / `use nowhere._` / `use solo.{NOPE}` | rc 0 each (the first by a base-binary probe; all three under mutation D1, which is the base code path) | rc 1 each (the `single_file_imports` gates) |

Every refusal is at the planted line, column 1 (the `use` declaration's own span). Rows 5 to 7
and 17, 20 are the accept controls. The whole-module form of a helper (row 6) binds nothing, so
the rule has nothing to check; note that the rewrite also deletes it before the resolve pass, so
its usual `[import.no-names]` warning never fires for a helper base, on either binary (section 7).
Reads (row 11) were refused before the fix and are refused identically after it. After every row
the edited file was restored from the reference tree, and all three were proven equal with `cmp`
at the end of both runs.

## 4. The fix

No language surface: the rule and its wording already existed in `resolve_use`. The fix makes
every path reach it.

- `resolve/imports.rs`: the rule factored as `use_decl_errors(u, index)` (a listed name that is
  not a `pub` item of its module; a glob over a module that exports nothing), with
  `no_pub_name_error` / `glob_matches_nothing_error` the ONE wording `resolve_use` also uses, and
  `unknown_module_error` the one wording the BFS in `reachable_modules` also uses. `use_decls`
  walks a file's `use` lines (section bodies included). `ExportIndex::has_module` tells a module
  that exports nothing from one that does not exist.
- `resolve/mod.rs`: `BuiltProgram::lowered`, the ids of the modules the build lowered, so a
  caller that checks something outside the pass scopes it to the build's own modules.
- `native.rs`: `rewrite_helper_imports` = `helper_import_errors` (the rule over every `use` of a
  helper, against the tree AS WRITTEN) then `publicize_helper_comptime` then
  `normalize_helper_imports`. `build_emp` reports those errors for the modules it lowered
  (`helper_import_errors_in`, deduplicated against what the resolve pass already said). Kill
  condition, in the doc comment: retire with `normalize_helper_imports`; once the author's lines
  reach the resolve pass unrewritten, the pass applies the rule itself.
- `import_check.rs` (new): `standalone_import_errors(root, files)`: a `use` resolves first among
  the given files (as parsed, so an in-memory override is what gets checked), else in the tree
  scanned from `root`, scanned only when some `use` names a module outside the files.
- `seam1.rs`: `resident_import_verdict` over the five resident files, located per file; run first
  in `emit_sound_blob` and `native_blob_checked` (an `Err`), and as a backstop panic in
  `place_resident_sections` for its callers with no error channel.
- `seam2.rs`: `import_verdict` per lowered file, at all four `lower_module` sites.
- `main.rs`: `single_file_import_errors`, the one-module program's version of the rule.

Errors, never warnings, each at the `use` declaration's own span.

## 5. Tests and red-first evidence

New tests (20). Each was run red against a mutation shown on disk (`git diff --stat` and
`git diff -U0` printed in the session before each red run), then restored with
`git restore --source=<the committed tip>` of exactly the mutated files and
`git status --porcelain` = 0 lines. Rounds A, B1+B4+C2 and B2+C1+E1 ran on `b7ee95fc`; the
emitter gate's test-first run on `b7ee95fc` plus the new test; the last two rounds on `26e4bec9`.

| test | file | runner |
|---|---|---|
| `the_import_rule_refuses_each_listed_name_the_module_does_not_export` | `crates/sigil-frontend-emp/tests/resolve_imports.rs` | `cargo test -p sigil-frontend-emp --test resolve_imports` |
| `the_import_rule_and_the_resolve_pass_refuse_identically` | same | same |
| `the_import_rule_refuses_only_a_glob_that_brings_nothing` | same | same |
| `has_module_tells_an_empty_module_from_a_missing_one` | same | same |
| `native::helper_import_tests::an_unread_helper_import_of_a_missing_name_is_refused` | `crates/sigil-harness/src/native.rs` | `cargo test -p sigil-harness --lib` |
| `native::helper_import_tests::an_unread_helper_import_of_a_private_name_is_refused` | same | same |
| `native::helper_import_tests::an_unread_helper_import_of_a_pub_name_is_accepted` | same | same |
| `native::helper_import_tests::a_stale_helper_import_outside_the_build_is_not_reported` | same | same |
| `import_check::tests::a_use_outside_the_files_is_checked_against_the_tree` | `crates/sigil-harness/src/import_check.rs` | same |
| `import_check::tests::a_given_file_outranks_the_trees_copy_of_its_module` | same | same |
| `import_check::tests::a_tree_rewritten_in_place_is_scanned_again` | same | same |
| `an_unread_import_of_another_module_is_refused_at_the_use` | `crates/sigil-cli/tests/single_file_imports.rs` | `cargo test -p sigil-cli --test single_file_imports` |
| `a_blank_import_of_another_module_is_refused` | same | same |
| `an_own_module_import_of_a_missing_name_is_refused` | same | same |
| `an_import_free_file_and_a_valid_own_import_both_build` | same | same |
| `an_unread_resident_import_of_a_missing_name_is_refused` | `crates/sigil-harness/tests/seam1_import_check.rs` | `SIGIL_STRICT_GATE=1 AEON_DIR=<ref> cargo test -p sigil-harness --test seam1_import_check` |
| `an_unread_resident_import_of_a_private_name_is_refused` | same | same |
| `an_unread_resident_import_of_a_missing_module_is_refused` | same | same |
| `an_unread_resident_import_reaches_the_emitter_as_an_error` | same | same |
| `an_unread_resident_import_of_a_real_pub_name_is_accepted` | same | same |

Every refusal test pins the rule's own text (`has no \`pub\` name`, `no module \`X\` found under
the scan root`, `no module \`X\` in this build: a single-file`), which no other diagnostic uses.
The seam-1 refusals and two of the three single-file refusals also pin `path:line:col` of the
planted line; the helper-rewrite tests pin the exact list of Error messages.

Red-first rounds (expectations derived from the mutated code, then observed):

| round | mutation (on disk) | red (as predicted) | green (as predicted) |
|---|---|---|---|
| A | `imports.rs`: `has_module` returns `true \|\| ..`; `use_decl_errors` list filter `false && ..`; glob guard `false && ..` | all 4 frontend tests, each at its own assertion | the 12 existing `resolve_imports` tests |
| B1+B4+C2 | `native.rs`: `helper_import_errors` moved AFTER `publicize_helper_comptime`; lowered filter `true \|\| ..`. `import_check.rs`: scan disabled | private-name (B1: `left: []`), scope first assertion (B4), outside-the-files (C2), seam-1 accept arm (C2: `no module \`engine.sound_constants\` found under the scan root`) | missing-name, accept, seam-1 missing-name / private / missing-module |
| B2+C1+E1 | `native.rs`: `helper_import_errors` output filtered to nothing. `import_check.rs`: the tree's copy of a given module let into the index. `seam1.rs`: both `resident_import_verdict` calls under `if false` | helper missing / private / scope second assertion (B2); seam-1 missing-name, private, missing-module (E1: the blob lowered and linked) | helper accept, seam-1 accept. **The outrank test stayed GREEN under C1**: see below |
| emitter, test-first | none: the new gate run against the committed `seam1.rs` of `b7ee95fc` | `the refusal escaped as a panic, not as the emitter's Err` (panic at `seam1.rs:650`) | the other 4 seam-1 gates |
| C1+B3+D2 (at `26e4bec9`) | C1 again; `native.rs`: every removed helper import reported as `MUTANT`; `main.rs`: own-module branch `false && ..` | repaired outrank test (`left: []`), helper accept arm (`MUTANT`), single-file own-missing and accept arm | outside-the-files; the two other-module single-file tests |
| D1+F (at `26e4bec9`) | `main.rs`: the `single_file_import_errors` call removed; `native.rs`: `build_emp`'s report line removed | the 3 single-file refusals (`built: 2 bytes`); canary A with the mutated binary (`--version` `26e4bec9-dirty`, 2 modified): rc 0, 0 `CANARY`, `b09ccd65/820229` | single-file accept arm |
| cache (at `5e841fc7`) | `import_check.rs:34`: every file's CRC filtered to `None`, so the fingerprint sees only the file list | `a_tree_rewritten_in_place_is_scanned_again` (`left: []`: the stale scan was reused) | the other two `import_check` tests |

**Tightened proof partway, and the claim re-established (rule 7e).** The outrank gate
(`a_given_file_outranks_the_trees_copy_of_its_module`) was green with its target mutated: every
`use` in it resolved among the given files, so the tree was never scanned and the precedence it
names was never exercised. It now also imports a module only the tree has, which forces the
scan; re-run under the same C1 mutation at `26e4bec9` it is red. Its earlier green in the
`b7ee95fc` run certified nothing and is withdrawn. Second: the seam-1 private-name gate first
derived its name from a column-0 non-`pub` `proc` of `sound_fm.emp`, found none, and failed
loudly (as written: "re-point this gate"); it now derives a column-0 non-`pub` `const`
(`Fm_ScratchPart`). That was before the first commit and changed no claim.

**Found by timing, fixed in `5e841fc7`.** After the first two commits `sigil build --game sonic4`
took about 0.2 s longer on the `ec640bcf` copy (base 2.19 to 2.29 s, fixed 2.41 to 2.54 s, three
alternating runs, load average about 9), and `emit_sound_blob` more than doubled (0.19 to 0.20 s
against 0.45 to 0.48 s). A temporary timer around the tree scan (never committed; restored from
`26e4bec9`) attributed all of it: six full `Manifest::scan` calls per emit and per build, 202
modules parsed each time at about 45 ms, 227 ms and 278 ms in total. They come from the two
seam-2 files whose imports name modules outside themselves (`dac_sample_tab.emp`, `mt_bank.emp`),
lowered several times per build. `scan_cached` keeps one scan per thread and root, reused only
while the tree's fingerprint (each `.emp` path and a CRC32 of its contents, over
`manifest::emp_files`, the scan's own walk) is unchanged. It is a content fingerprint, not
length and mtime, because a same-length rewrite inside one coarse mtime tick would slip past
those. The read ledger is process-wide and never reset, so the first scan's reads stand for the
reuses. Gate: `import_check::tests::a_tree_rewritten_in_place_is_scanned_again`.
Re-timed at `5e841fc7` (load average about 18, so noisier): `sigil build` base 2.64 to 2.66 s,
final 2.72 to 2.89 s; `emit_sound_blob` base 0.21 to 0.27 s, final 0.35 to 0.37 s. A second
temporary probe (also never committed, restored from `5e841fc7`) attributes the remaining
~0.14 s per emit: one scan, 39 ms; six fingerprints, 59 ms (reading and hashing 202 files each
call; the CRC is already table-driven); ten `resident_import_verdict` calls, 42 ms (each
re-parses the five resident files, which placement already parses twice more per call); the
rest of the 21 seam-2 checks, about 6 ms. The residual cost is left in place and reported
here (section 7), not engineered away: the remaining halves would need a second cache, and a
content-keyed memo for resident verdicts, each with its own staleness gate, to buy back about
0.1 s.

**Found by the variant table, fixed in `26e4bec9`.** With the first commit, a resident refusal
had the right located text but `emit_sound_blob` exited 101: `emit_sound_blob` reaches placement
through `check_banked_carrier_drift` before `native_blob_checked`, so the placement backstop's
panic fired first. Inside `sigil build` that is a crash. The emitter now runs the verdict first.

Scoped totals at `26e4bec9`: `resolve_imports` 16 passed / 0 failed; `sigil-harness --lib`
filtered to the new modules 6 / 0; `single_file_imports` 4 / 0; `seam1_import_check` 5 / 0
(strict, `AEON_DIR` = the reference tree). clippy (`-p sigil-frontend-emp -p sigil-harness -p
sigil-cli --all-targets --release`): rc 0, no warning outside the vendored `*-sys` crates.

**Full strict workspace run at `5e841fc7`** (`SIGIL_STRICT_GATE=1 AEON_DIR=<ref> cargo test
--release --workspace --no-fail-fast`, the worktree's own `target/`): 473 test binaries launched,
473 reported; **5275 passed, 6 failed, 2 ignored**. The six, every one read in full:

| test | target | cause | verdict |
|---|---|---|---|
| `no_build_reachable_read_bypasses_the_recorder` | `sigil-span` `read_set_gate` | the scan cache's fingerprint read with raw `std::fs::read` (`import_check.rs:34`) | **parcel**; fixed in `46e8ab20` (`read_set::read`) |
| `every_selected_test_file_is_classified` | `sigil-harness` `source_gate_classification` | `seam1_import_check` reads the tree but was not in the source-gate run list | **parcel**; fixed in `46e8ab20` (`scripts/nightly_source_gates.sh`) |
| `the_derived_accessor_set_is_the_declared_guard_set` | same | same | **parcel**; same fix |
| `moved_dac_anchor_moves_the_derivation` | `sigil-cli` `seam2_layout_derivation` | its fixture SYMLINKED `engine/`; the module scan does not follow a symlinked directory (`manifest::collect_emp`, by design), so the new seam-2 check found no `engine.sound_constants` | **parcel** (the new rule meeting a fixture that relied on seam 2 never reading a `use`); fixed in `46e8ab20` by copying `engine/` into the fixture. The check is unchanged |
| `oracle_loadfromaslisting_resolves_emit_listing` | `sigil-harness` `m1b_gate` | no `ORACLE_DIR` named; the run declined the derived `oracle-old` checkout | **environmental**: 5/5 with `ORACLE_DIR` named, at the unedited `5e841fc7` |
| `refreeze_tells_its_children_which_build_directory_to_use` | `sigil-harness` `shared_target_defaults` | strict mode refuses a binary built into the checkout's default `target/` | **environmental**: 13/13 from a non-default build directory, at the unedited `5e841fc7` |

After `46e8ab20`, the three failing targets, `lst_source_digest` (the runtime witness that runs
real builds under an `open(2)` interposer, re-run because the read routing changed), and the
parcel's own gates: `read_set_gate` 3/0, `source_gate_classification` 3/0,
`seam2_layout_derivation` 3/0, `lst_source_digest` 2/0, `sigil-harness --lib` (new modules) 7/0,
`seam1_import_check` 5/0.

**Full strict workspace run at the tip `46e8ab20`** (the same command, built into the worktree's
non-default `.target-ref` so `shared_target_defaults` can run): 473 test binaries launched, 473
reported; **5280 passed, 1 failed, 2 ignored**. The one failure is
`m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`, the environmental one above (no
`ORACLE_DIR` named); re-run at `46e8ab20` with `ORACLE_DIR` named it passes, 5/5. Totals reconcile
with the `5e841fc7` run (5283 tests both times). The whole-shape gates in this run all passed at the
tip: `every_shipped_shape_builds_from_source`, `native_rom_plain`, `native_rom_debug`,
`demo_plain_anchor_matches_golden`, `demo_debug_anchor_matches_golden`, both
`demo_*_game_modules_match_golden`, `config_a_anchor_matches_golden`,
`config_b_anchor_matches_golden`, `contract_baselines_hold_for_every_shipped_shape`.

The closing run at `5e841fc7` also re-ran `repin --check` on the reference tree afterwards:
`pins.rs unchanged`.

## 6. Aeon impact, by compiler

Enumerated by compiler, never by grep: every shipped shape built with the base and the fixed
binary on two `.git`-less copies of aeon, `ec640bcf` (the reference tree) and `origin/master` =
`35f54923` (the reference tree's gitignored artifacts plus `git archive 35f54923` laid over it;
the delta is 72 added and 159 modified files plus one rename, `docs/BUGS.md` to
`docs/2026-09-09-BUGS-archived.md`, whose old name lingers in the copy as an inert doc; no `.emp`
was added, deleted or renamed, and 67 were modified). Nothing in aeon
was read or built in its main working tree.

**`sigil build`, all seven `native::shipped_shapes()`, `SIGIL_WARNINGS=full`:**

| shape | `ec640bcf` base = fixed | `35f54923` base = fixed |
|---|---|---|
| sonic4 plain | rc 0, `b09ccd65/820229` | rc 0, `52828985/821123` |
| sonic4 debug | rc 0, `1b7fe316/846529` | rc 0, `ddf22eca/847389` |
| demo plain | rc 0, `0ad17404/96863` | rc 0, `c0898f05/97075` |
| demo debug | rc 0, `2565ece2/103185` | rc 0, `80bbeb9b/103359` |
| config_a | rc 0, `9a63c8c1/846881` | rc 0, `9df18359/847741` |
| config_b | rc 0, `ed60e162/620727` | rc 0, `23c9e8bc/623295` |
| lean | rc 0, `0b5d25f5/773136` | rc 0, `c8746644/773406` |

At both revisions each shape's full-warning log is BYTE-IDENTICAL between base and fixed (14
`diff -q` comparisons, 168 to 204 lines each), and `emit_sound_blob` writes the same 19 files,
each `cmp`-identical. Every `ec640bcf` CRC above equals the last `provenance.toml` entry's golden
for that target (`aeon_rev = ec640bcf`), which is the control that the copies build the real thing.
(The fixed binary here is the first commit's; the second commit changed only the emitter's error
channel for a refused resident import, which no clean tree reaches. The final binary's shapes are
below.)

**The four aeon shapes through aeon's own `build.sh`, one shape per invocation, `FAST=1`
(the artifact steps only; section 8, item 4):**

| shape | `ec640bcf` base | `ec640bcf` final | `35f54923` base | `35f54923` final |
|---|---|---|---|---|
| `./build.sh` | `s4.bin` `b09ccd65/820229` | `b09ccd65/820229` | `52828985/821123` | `52828985/821123` |
| `DEBUG=1 ./build.sh` | `s4.debug.bin` `1b7fe316/846529` | `1b7fe316/846529` | `ddf22eca/847389` | `ddf22eca/847389` |
| `./build.sh demo` | `demo.bin` `0ad17404/96863` | `0ad17404/96863` | `c0898f05/97075` | `c0898f05/97075` |
| `DEBUG=1 ./build.sh demo` | `demo.debug.bin` `2565ece2/103185` | `2565ece2/103185` | `80bbeb9b/103359` | `80bbeb9b/103359` |

All sixteen runs rc 0; the `ec640bcf` base column equals the goldens. The table was run with
the `26e4bec9` final binaries and again, all sixteen runs, with the tip's `5e841fc7` binaries in
the closing run: every CRC and size identical.

**aeon's poison lane (`tools/emp_expect_fail.py`, about 20 `sigil build --extra-entry <poison>`
builds, each poison's `[Error]` count pinned), final binary:** `ec640bcf`: rc 0, "OK, 54/54
cases (52 comptime + 2 link)"; `35f54923`: rc 0, "OK, 54/54 cases (52 comptime + 2 link)". Every
poison still fails with exactly its pinned `[Error]` count, so no poison carries an import the
fix now refuses. (The base binary was not run: the script runs it only on a tree where final
fails.)

**Newly refused aeon sites: none, at either revision.** Nothing is BLOCKED on aeon; no aeon edit
is needed.

## 7. Left open

- **Port-test harnesses (`animate_port` and kin): unchanged, by call, not BLOCKED.** They are
  oracles: each lowers one real file standalone, prepends what it needs by hand, and never
  consults a `use` by design. The files they lower ship through the checked paths: `build_emp`
  for the map build, and seams 1 and 2, which run inside every sound-on `sigil build`
  (`emit_generated`). The suite's `corpus_builds` builds all seven shipped shapes from source, so
  a stale import in aeon's map-build modules turns the suite red there. Adding the check
  to each harness would repeat the build's gate with no new coverage. If wanted, it is one call to
  `import_check::standalone_import_errors` per harness.
- **RAM harvest.** `harvest_engine_ram_addresses` builds after `publicize_helper_comptime(RAM_HELPERS)`
  without the check, so a private RAM-helper name is accepted by the harvest itself. Its modules
  (`engine.ram`, the game RAM module) are in `build_emp`'s closure in the same `sigil build`, which
  refuses it. Unchecked only if something ran the harvest alone.
- **`[import.no-names]` for a whole-module helper `use`.** The rewrite deletes `use engine.types`
  before the resolve pass, so the warning that tells an author a bare `use` binds nothing never
  fires for a helper base. A warning, unchanged by this parcel (the fix checks names; a whole `use`
  has none). Same kill condition as the check: retiring the rewrite restores it.
- **Modules outside the build.** A stale import in a module no shape reaches is not reported,
  exactly as the resolve pass judges only what it lowers. `[module.unreachable]` already names
  guard-carrying unreachable modules.
- **Seam 1 and a `use` of a `pub const`.** Still the T1 gap (`2026-09-11-emp-extern-unknown-name.md`,
  section 7): seam 1 derives stubs for `pub proc` only. The check now at least says whether the
  name exists; binding it is option A there, unchanged.
- **`--extra-entry` help text.** "its own imports are not checked" describes
  `refuse_artifact_contribution` (it inspects the named module's items, not what its imports
  emit), not the name rule. After this fix an extra entry's `use` lines ARE checked (it is lowered
  by `build_emp`). The sentence can read as the opposite; help text is CLI surface, so it is
  flagged here rather than changed.
- **Runtime.** No emulator. Nothing here needs one: the fix emits no byte (every shape's CRC is
  unchanged, section 6).

## 8. What the brief got wrong

1. **The mechanism was upstream of the pass the brief pointed at.** Both hypotheses carried from
   the routed note ("`is_exported` answered true" or "something after `resolve/mod.rs:880` drops
   the Error") are false. `is_exported` answers correctly and nothing drops the Error: the line
   never reaches `resolve_use`, because `native::build_emp`'s `normalize_helper_imports` deletes
   every top-level `use` of a comptime helper first. It is not animate-specific: it covers every
   import of all fourteen `COMPTIME_HELPERS` in every module, including private helper names.
2. **"Renaming the CALL instead was caught" is true, and is why the hole was invisible.** The glob
   the rewrite inserts binds every real helper name, so any READ of a bad name still fails at the
   reader. Only an unread bad name had nothing left to fail at.
3. **Seam 1 was not the only other path.** Seam 2 (all four banked-table lowerings) and
   single-file `sigil emp` never read a `use` either; both are fixed here.
4. **"Aeon's four shapes... the aeon build is `./build.sh`".** The full `build.sh` cannot run in
   a copy with no `.git`: its tool-suite tier fails before any ROM step, with the BASE binary
   (so not a compiler effect): pytest reported 31 failed, the `FAILED` lines naming
   `tools/test_bg_emit.py` (20), `test_emp_helper_closure.py` (4),
   `test_no_baked_home_paths.py` (3), `test_editor_inputs.py` (1) and `test_suite_paths.py` (1). The shape runs use `FAST=1 ./build.sh`,
   one shape per invocation, which runs exactly the ROM-producing steps; the control is that the
   base FAST ROMs at `ec640bcf` equal the provenance goldens for all four shapes (and all seven
   through `sigil build`). The one verification tier that can see a new refusal in aeon's own
   corpus, `tools/emp_expect_fail.py`, was run separately with the final binary.
5. **"every `use` spelling the grammar accepts (grouped `{a, b}`, aliases, globs, if they exist)".**
   There is no alias spelling. The forms are whole, blank, glob, and list (one line or many,
   trailing comma allowed); every one is in the variant table.
6. **Line numbers.** `resolve_use`'s `is_exported` test is at `imports.rs:384` at base (the
   note's number), and `ResolveEnv::build` at `resolve/mod.rs:879`; both held.
