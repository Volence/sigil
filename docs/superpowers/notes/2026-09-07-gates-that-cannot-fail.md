# Gates that cannot fail: five findings landed (2026-09-07)

Parcel `parcel/gates-that-cannot-fail`, branched from master `99aa2645`. Source: the
22-seat sweep packet `2026-09-06-sigil-lens-sweep.md`, section "Gates that cannot
fail". All five items are test, script and guard changes; byte-neutrality of the ROM
outputs is the controller's landing gate to prove. Every reproduction below was my own;
where it disagreed with the packet's count, the reproduction is what landed.

Commits, one per item, in branch order:

| item | id | commit |
|---|---|---|
| 1 | `sig-landing-bar-skips` (HIGH) | `87380d24`, follow-up `bc788fc4` |
| 2 | `sig-skip-lint-wrap` (MEDIUM) | `04e2e63e` |
| 3 | `sig-plain-arm-proofs` (MEDIUM) | `3159fe8d` |
| 4 | `sig-ab-witness-nonempty` (MEDIUM) | `e4e7c416` |
| 5 | `sig-winptr-dead-copy` (LOW) | `73334d80` |

Nothing is BLOCKED. Two things the brief stated turned out wrong (section at the end).

## Reference tree used for measurement

The live sibling `/home/volence/sonic_hacks/aeon` is at `f7c1c393`, past the provenance
tip, and reds `test_static` at offset 0 in both shapes; it was not used. The tip entry
(203, `relayout-at-aeon-master`) names `/home/volence/sonic_hacks/.aeon-ref-relayout-master`
(clean detached worktree at `483b3e12`, four shapes built). Every reference-dependent run
below names that tree with `AEON_DIR`, read-only. No tree was provisioned or built.

Every cargo command ran with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-gates`.

## Item 1: the landing bar did not fail on a skip line

**Reproduction.** `scripts/landing-run.sh`'s exit decision read `CARGO_RC`, `FAILED`,
`CLIPPY_RC` and `RECONCILED`; `SKIPS` was printed as a WARNING beside `RESULT GREEN`, exit
0. To exercise the verdict path without a suite run I added `--verdict-only <log>` FIRST,
with no change to the rule: the run half became `run_landing()` (body left at its column),
the verdict inputs are read from the log's stamp and its `CARGO_EXIT=`/`CLIPPY_EXIT=`
lines, and one dispatch takes either branch into the same verdict code. The `CLIPPY_SITES`
parse moved beside the other log readers because it reads only the log (the verdict-only
path has no clippy process). A fixture log (three green tests, clippy clean, one `skip:`
line inside the test span):

```
RESULT          GREEN          exit 0     (rule unchanged)
RESULT          FAILED, 1 skip line(s) survived SIGIL_STRICT_GATE=1
                               exit 1     (after)
```

The clean twin of the fixture is GREEN, exit 0, before and after (control).

**Fix.** `SKIPS > 0` joins the FAILED condition, exit code 1 (the EXIT CODES table's
"suite FAILED" code; table and header prose updated, a "skip line is a red run" paragraph
added beside the clippy one). A skip-only failure is named separately, as the lint-only
one is. The warning text now says it fails the run.

**Runner.** `crates/sigil-harness/tests/landing_verdict.rs` shells the real script over
fixture logs written to on-disk scratch:

- `a_skip_line_under_strict_fails_the_landing_verdict` (exit 1 + FAILED; control GREEN)
- `the_second_spelling_fails_the_landing_verdict_too` (`skipping` -> exit 1)
- `a_skip_word_quoted_by_clippy_is_not_a_skipped_gate` (counter is span-scoped, GREEN)
- `a_log_without_an_exit_line_is_refused_not_judged` (no `CARGO_EXIT=` -> exit 2)

**Mutation, shown applied.** Line 781 on disk read
`if (( CARGO_RC != 0 || FAILED > 0 || CLIPPY_RC != 0 )); then` (SKIPS dropped). Runner:
`a_skip_line_under_strict_fails_the_landing_verdict ... FAILED` and
`the_second_spelling... FAILED` ("must exit 1 ..., got 0"), 2 passed / 2 failed. Restored
with `git checkout` from `87380d24`; runner 4/0.

**The four documents.** Script header (made true, mechanism named); README (sentence
added: the wrapper makes a surviving `skip:`/`skipping` line `FAILED`, exit 1);
`docs/OVERSEER-REFERENCE.md` full-suite-bar paragraph (enforcement named, runner named);
the nightly script's comments describe its own check (exit 2 on the marker) and were
already true; `skip_marker_lint.rs`'s module doc claimed both enforcers fail on a skip line
and is now true of both. Two stale `landing-run.sh:369` line pointers in `test_support.rs`
now name the `SKIPS` count.

**Follow-up `bc788fc4`.** The full harness tally counted one skip line in a green run: the
NAME of my own test, `the_skipping_spelling_...`, which `cargo test` prints into the very
log the counter reads. Renamed. No other `fn` name in the tree spells either form.

## Item 2: the skip-marker lint could not see a wrapped literal

**Reproduction.** `print_literal(line)` read only the text after `println!(` on the same
line. Five announcements in `suite_paths_precedence.rs` said `NOT MEASURED` (the lint's own
vocabulary) with the literal on the next line and were green.

**Fix.** The detector takes the source and the line's byte offset and reads the first
string-literal argument from there across line breaks; a `\`-continued literal is joined
as rustc joins it. The per-file scan is `scan_source(src, path)` so fixtures go through the
identical detector. No rustfmt rule: the lint is correct under any wrapping.

**Red-first on the live tree, detector widened, nothing else changed: SEVEN named, not
five.** The five in `suite_paths_precedence.rs` (419, 547, 562, 569, 679), plus two
lexical-only hits the packet did not count: `reference_dependence_is_named.rs:101` (the
partial-run notice, "to turn these skips into named failures") and
`strict_census_lint.rs:86` (a green-run census line naming a population
"missing-reference path(s)").

**The lint's own poison, shown applied.** A planted file
`crates/sigil-harness/tests/zz_planted_wrapped_violator.rs` with
`println!(\n "NOT MEASURED: ..." \n); return;` was the ONLY site named once the seven
were fixed (`... zz_planted_wrapped_violator.rs:5 [early-return + skip vocabulary]`); removed
(never committed); lint green without it. Four in-file fixture tests pin the shapes
(next-line literal, continued literal, wrapped literal carrying the marker, single-line and
value-argument unchanged).

**The seven sites, each made to say what it does:**

- 419, 679 (a bed could not be built), 547, 562 (no suite markers above the crate / git did
  not answer): honest skips, now `skip: ...`. Under a strict landing these red the verdict,
  which is what a check that measured nothing should do; none fires in a landing from the
  suite layout.
- 569 (compiled in a plain checkout) was NOT a skip: the deployed derivation can be asserted
  in either shape (it must reach the walked suite root); only the worktree DISTINCTION is
  the bed row's. Early return gone, assertion runs in both shapes, printed line names the
  shape. From this linked worktree: `ambient anchor: compiled inside a linked worktree; ...`.
- `reference_dependence_is_named.rs:101`: about skipping, so it carries the marker. It
  prints only under `SIGIL_ALLOW_PARTIAL` (the row asserts `!strict` first).
- `strict_census_lint.rs:86`: a green-run report; prefixing it with the marker would red
  every landing on a passing census. The population is renamed by what its sites do
  (`reference-refusal path(s) excluded`); same count, same field.

Runners: `-p sigil-harness --test skip_marker_lint` (6/0), `--test suite_paths_precedence`
(3/0), `--test strict_census_lint` (5/0); `SIGIL_ALLOW_PARTIAL=1 -p sigil-cli --test
reference_dependence_is_named` (1/0).

## Item 3: plain arms that could not fail

**Reproduction.** g1, g2, g3 object ports branch on `<region>_len == 0` (the plain pin)
and their comments claimed the arm "proves the region is EMPTY" / "carries ZERO plain
bytes". In g1 the arm asserted `animated_ref.is_empty()` on `rom[base..base+0]`: empty by
construction, the hardened `assert_region_matches` bypassed.

**Fixes.**

- g1: the plain arm asserts the module's own plain-shape image: non-empty and the size the
  DEBUG pin records (`DEBUG.animated_len`, the one region length the plain pins do not
  carry), within the byte gate's < 16 fill tolerance.
- g2: the code already cross-checked the two sibling pins and re-lowered at the DEBUG
  bases; the comment says that is what it checks, and the messages name which pin disagrees.
- g3: the comment says the only check is that the module still lowers and links at the
  DEBUG base; the "zero plain bytes" claim is deleted.

**Measured.** Baseline 12/12 green against the tip reference; with the new arms 12/12.

**Red-first per arm, mutating the SUBJECT (region data in `pins.rs`), shown on disk
(`grep -n` of lines 184 and 190), restored from the commit:**

- `TEST_ANIMATED debug_len 0x60 -> 0x80`:
  `g1_objects_regions_match_reference ... FAILED` ("plain-shape image is 0x60 bytes; the
  debug region pin records 0x80"); `g1_objects_debug_regions_match_reference ... FAILED`
  ("length mismatch, candidate 96 bytes, expected 128 bytes").
- `TEST_STRESS_EMITTER plain_len 0x0 -> 0x10`: `g2_objects_regions_match_reference ...
  FAILED` ("the emitter pin says plain-empty, the stress pin disagrees").
- g3's plain arm asserts only that lowering succeeds; its subject is the `.emp` source in
  the reference tree, which this parcel does not mutate. No red-first for that arm; its
  comment claims nothing more.

Observed: `place_sections` does not refuse a section whose bytes exceed its map region's
`size` (the plain map gives `test_animated` size 0; the 0x60-byte section places without a
diagnostic). Not this parcel's item; for the gap ledger.

## Item 4: the A/B guard accepted an entry name as evidence

**Reproduction.** Rule (2) accepted any non-empty, non-sentinel `ab`. Entries 130-134
carry the NAME of the preceding entry, chaining to the bare word `master`.

**Constraints held.** `golden/provenance.toml` untouched, no re-baseline, the five stand.

**Boundary, derived.** The rule binds entries carrying `aeon_rev`: first present at entry
167 and on every entry since; `render_entry` writes it unconditionally and `--freeze`
refuses without a vetted SHA, so every entry written from now on is bound. The five (and
the six bare short-SHA entries 159-164) predate the field and are exempt by the same
derivation rule (3) uses for aeon-rev-monotonic.

Candidates rejected: `[entry.strict]` present (first at 173) exempts the TIP until
`--attest`, i.e. exactly the entry being written; a new `ab_kind` field binds nothing in
the chain and adds a field to forget; an entry number is forbidden and is the merge race
`append_gate` documents.

**Predicate** `ab_witness_fault(ab, chain)`: refused when `ab` is the name of any chain
entry, or a bare token (no whitespace, `/`, `.` or `@`). Every in-regime `ab` in the
committed chain (167-203) passes; `tests/provenance_chain.rs` runs `check` on the real
chain (10/0). Applied in `freeze_into` before the write too, so a future
`--ab parcel-r1-ram` is refused by name. One error per moved target, no double-report.

**Red-first on a synthetic chain fixture** (root + mover, no blobs), six tests in
`provenance::tests`: entry-name `ab` flagged; `master` flagged; pre-regime `master` clean;
four real evidence shapes clean; byte-neutral mover clean; empty `ab` reports the floor rule
once. **Mutation, shown applied:** line 590 on disk read
`if !ab.is_empty() { return None; } // MUTATION: every non-empty ab is a witness`;
`anchor_move_with_a_chain_entry_name_as_ab_is_flagged_in_the_aeon_rev_regime ... FAILED`,
`anchor_move_with_a_bare_branch_name_as_ab_is_flagged_in_the_aeon_rev_regime ... FAILED`
(51 passed / 2 failed in the module). Restored from `e4e7c416`.

## Item 5: the winptr mask written twice, one copy dead

**Reproduction.** `lower::data::sym_target` masked behind `Cell::SymRef { windowed: true }`;
`windowed: true` is constructed nowhere (eleven constructors, all `false`). The live path is
`Evaluator::eval_winptr` (`Value::LinkExpr` -> `Cell::Expr`).

**Fix.** `SFX_WIN_MASK`/`SFX_WIN_BASE` now exist once as `pub const` beside `eval_winptr`,
which uses them. Deleted: the `windowed` field, `sym_target`, the `BankPtr16*` selection
arms, the windowed rows of the D-P4.5 table (the doc now says where `winptr` lowers).
`FixupKind::BankPtr16Be/Le` stay in sigil-ir. `buf_carries_words` is byte-neutral by
construction (the flag was always false).

**Measured.** `lower_sections::winptr_data_cell_byte_identical_via_linkexpr` pins the
pre-change bytes ($D69A both endians) and passes; whole crate green. No gate was added, so
no mutation.

## Totals (full crate runs, tip reference named, `--no-fail-fast`, exit 0 each)

| crate | suites | passed | failed | ignored | failing names |
|---|---|---|---|---|---|
| sigil-harness | 46 | 414 | 0 | 1 | none |
| sigil-frontend-emp | 129 | 2641 | 0 | 0 | none |
| sigil-cli | 155 | 697 | 0 | 1 | none |

Skip lines in those logs after `bc788fc4`: 0. Lint bar
`cargo clippy --release -p sigil-harness -p sigil-frontend-emp -p sigil-cli --all-targets -- -D warnings`:
exit 0. The workspace-wide landing gate is the controller's.

## What the brief got wrong

1. "Share the named constant the sibling builtin already uses": there was no named constant
   in Rust; `SFX_WIN_MASK`/`SFX_WIN_BASE` were AS names living in comments. Introduced.
2. "Five live sites" for the lint: the widened detector names seven. The two extra are of
   different kinds (a notice that IS about skipping, and a census line that is not), and
   were handled differently for that reason.

## Open

- `place_sections` region-size non-enforcement (item 3 observation), for the gap ledger.
- The `.emp` sources for the g3 plain arm are not mutated here, so that arm's only check
  (lowering succeeds) has no red-first in this parcel.
- Under a strict landing the four honest `skip:` sites in `suite_paths_precedence.rs`
  (bed could not be built; no markers; git failed) would now red the verdict. They never
  fire in a landing from the suite layout; if one does, that is the correct outcome.
