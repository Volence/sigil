---
kind: audit-stage
system: sigil-assembler
stage: "5 Checker"
agent: Opus
status: done
description: "44 citations re-opened: 40 hold, 2 mislabelled, 2 wrong. Headline test totals re-aggregate exactly. Both reviewers missed a known open linker gap (region overflow can reach the ROM) and that CI is red on master; 8 gaps sent back."
---
# Sigil assembler: checker

Sigil `7630ba0d`, aeon `936439fa`. Checked 2026-11-16 against stages 0 to 4 in this folder, the v2 [[AUDIT_TEMPLATE]] and the known-defect sources the task names. One bounded round. I wrote nothing outside this file. My probes ran in `scratchpad/audit-builds/sigil-checker/` with stage 0's release `sigil` binary.

> [!verdict] Both reviewers' verdicts hold up on the evidence they cite, but each claims one thing the known-defect list contradicts
> - **The citations are sound.** I re-opened 44 `path:line` citations, weighted toward the claims the verdicts depend on (F1 truncation, F4 quadratic, F5 placement loop, F6 debug info, F7 appendix, F8 hermeticity, Q1 coupling, the compiled-tier evidence). Results: **40 hold, 2 mislabelled, 2 wrong.** Neither wrong claim changes a verdict. Both change a correctness statement.
> - **The headline numbers hold.** Re-aggregating `audit-builds/sigil/test-full.log` gives 521 binaries, **5,864 passed, 3 failed, 3 ignored**, `CARGO_EXIT=101`, exactly as stage 2 reports. The truncation, const and `btst` probes reproduce byte for byte (MEASURED, release Sigil).
> - **Missed known defect 1 (the most important): region overflow can reach the ROM.** The open queue row `SEAM2-TABLE-REGION-UNENFORCED` (state `next`) records three gaps. `place_sections` never reads a region's size. The seam-2 emitters never call `emit_rom`, so all 7 synthetic seam-2 region sizes are unenforced (`sound_tables_z80` is 1,116 B against a `0x400` region). `MemoryMap::validate_section` picks the region by LMA (`sigil-ir/src/map.rs:135-136`), so `emit_rom` accepts a spill that starts at the next region's base, and any overflow of a region nested inside an earlier ROM region. Stage 2's own strict run **passed the four tests that pin these gaps as current behaviour** (`gap_emit_rom_accepts_…`, `gap_seam2_tail_links_…`, `gap_place_sections_is_silent_…` in `sigil-harness/tests/seam2_region_unenforced.rs`). So the gaps are live at `7630ba0d`. MEASURED from the log. Reviewer B states the opposite ("I found no case where an exceeded budget produces a ROM"). Reviewer A's edge note ("the ROM must lie inside a declared region") is incomplete.
> - **Missed known defect 2: CI is red on master.** `gh run list` shows the last 12 runs on 2026-09-28, the day of the final commit, all `failure`. The 33 earlier runs that day succeeded (MEASURED). The failing test is `g4_objects_regions_match_reference` (`sigil-cli/tests/test_g4_final_objects_port.rs:138`). It reads an aeon file directly and panics on the `/nonexistent/SIGIL_ALLOW_PARTIAL-…` placeholder path, so this port gate has no skip guard. This is a recurrence of the class behind lens finding `sig-ci-red` (the lens log records it as fixed). Reviewer A says CI "runs debug and partial, and says so honestly". That is true of the CI configuration, but the badge is red. Stage 0 had recorded CI history as not checked.
> - **The coverage hole that matters:** neither reviewer read the Z80 side (`seam2.rs` beyond its header, `z80_cycles.rs`, `z80_preserves.rs`, `z80_out_verify.rs`, `sigil-backend-z80`, `sigil-isa/src/z80.rs`), and that is the named workload's heavy part. Neither read the three FFI crates, which hold all 14 `unsafe` blocks and run on every build.

## 1. Citation checks

"Holds" means the cited lines say what the stage says they say. MEASURED by opening each range at `7630ba0d` (aeon `936439fa` for aeon paths).

| # | Claim (stage) | Cited | Result |
|---|---|---|---|
| 1 | 4 MB flat window, no mapper (0, 1, A, B) | `sigil-ir/src/map.rs:35` | holds (`CARTRIDGE_SIZE = 0x40_0000`) |
| 2 | Cross-build cache ruled out on purpose (0, 1) | `frontend-emp/src/resolve/mod.rs:80-86` | holds ("WRONG ROM THAT PASSES") |
| 3 | Threads are used only for the eval stack (0, A F4, B 2) | `eval/mod.rs:1916-1940` | holds: a 64 MiB scoped spawn followed by a join |
| 4 | `Diagnostic` = level, message, one span (0, 1, B Q6) | `sigil-span/src/lib.rs:425-431` | holds |
| 5 | `sigil build` writes into the aeon tree (0, A F8, B 4) | `sigil-harness/src/native.rs:839-857` | holds: 7 emitters write `engine/sound/generated` |
| 6 | `.emp` lowering defers the size check to the encoder (A F1, B 1) | `lower/code.rs:1396-1405` | holds |
| 7 | The encoder says the front end checks, and masks (A F1, B 1) | `sigil-isa/src/m68k.rs:13-21`, `:1907-1913` | holds: `(v as u16) & 0x00FF` |
| 8 | The Z80 path checks its immediates (A, B) | `lower/code.rs:2035-2051` | holds |
| 9 | The AS front end checks instruction immediates (A) | `frontend-as/src/eval.rs:2440-2470` | **mislabelled.** Those lines are `check_data_range`, the `dc.w`/`dc.l` check. The instruction-immediate check is `settle_imm`, `eval.rs:3333-3338`, which B cites correctly. The claim itself holds |
| 10 | Typed scalar consts unchecked (A) | `eval/const_arity.rs:105-107` | holds: returns early unless the type has an array |
| 11 | `index_items` per evaluator (A F4, B 2) | `eval/mod.rs:848-849` | holds. It indexes the ambient **and** the file, so per-proc cost also scales with the ambient |
| 12 | Two evaluators per proc (B 2) | `eval/mod.rs:2070`, `:2088` | holds (`probe` built once per proc) |
| 13 | Outer placement loop stops at 8 rounds (A F5) | `native.rs:2836-2844` | holds |
| 14 | Island-reclass refusal (A F5) | `native.rs:2779-2786` | holds |
| 15 | Span heuristic `gap <= im + 0x10` (A F5) | `native.rs:2959` | holds |
| 16 | Inner relaxation: grow-only, proven cap (1, A) | `sigil-link/src/relax.rs:61-75`, `:1053-1078` | holds |
| 17 | Appendix band capped at 64 KB, no 4 MB check (A F7) | `native.rs:4880-4917` | holds: `min_appendix..=0x10000`, no `len <= 0x400000` |
| 18 | Zero-firing families print a count only (A F3) | `sigil-cli/src/main.rs:2236-2248` | holds |
| 19 | `--check` skips the contract gate (A challenge 5) | `main.rs:1885`, `:3018-3020`, `:3080` | holds: the doc comment says so, and only `run_build_native` calls `run_contract_gate`. **Stage 2's 21% link/emit split is therefore wrong** |
| 20 | Duplicate symbol with a null span (A) | `sigil-link/src/lib.rs:86-91` | holds (TODO at `:89`) |
| 21 | One synthetic listing line per symbol (1, A F6, B Q3) | `sigil-link/src/listing.rs:157-162` | holds |
| 22 | Every `DataFragment` carries a span (A F6, B Q3) | `sigil-ir/src/lib.rs:29-36` | holds. A's second cite (`backend-m68k/src/lib.rs:45-46`) is the `lower_inst(…, span)` signature: holds |
| 23 | Helper list: "the later helper silently wins" (A) | `native.rs:1814-1816` | holds, but the text is at `:1819` (list at `:1802`). Line drift, not a wrong claim |
| 24 | Sound driver file list in Rust (B Q1) | `seam1.rs:131-151` | holds |
| 25 | Contract baseline names aeon procs (B Q1) | `contract_baseline.rs:137-146` | holds (`Air_Collide` rows) |
| 26 | Cycle budget cannot cross loops or calls (B Q4) | `cycle_budget.rs:27-31` | holds |
| 27 | Closed attribute list (B) | `parser.rs:307-310` | holds |
| 28 | Attribute arguments validated in the parser (B walk B step 2) | `parser.rs:323-335` | holds |
| 29 | `budget` keyed by name, `cycles:` assumed (B walk B step 3) | `lower/proc.rs:597`, `:681-687` | holds |
| 30 | Side tool counts any `@budget` (B walk B step 11) | `bin/cycle_fraction.rs:492` | holds |
| 31 | `Cfg` reused (B) | `flag_check.rs:708` | holds |
| 32 | Stack dataflow observer (B walk B) | `preserves.rs:548`, `:571` | holds (cited as `:541-571`) |
| 33 | `ItemAuthor` exists (A, B) | `value.rs:337` | holds |
| 34 | Strict-gate skip rule (0) | `test_support.rs:40-44` | holds |
| 35 | Link-time abs.w window (A, B) | `sigil-link/src/lib.rs:735-742` | holds |
| 36 | Overlap refused (A, B) | `lib.rs:1029-1033` | holds |
| 37 | Region containment (A edges note) | `sigil-ir/src/map.rs:135-160` | **wrong as a safety claim.** The code checks only the **first** ROM region containing the LMA. Known gap; see section 4 |
| 38 | `corpus_contracts.rs` holds Aeon-specific contracts (0 §8) | grep of non-comment lines | **wrong** (B challenge 5 holds: 0 hits for `sonic4\|ojz\|aeon\|RunObjects\|engine.`) |
| 39 | `build.sh` published 1.15 s (0, 1) | `aeon/build.sh:37-48` | holds as PUBLISHED. Stale against 2.74 to 2.92 s MEASURED (B challenge 6 holds) |
| 40 | `SIGIL_BUILD` hard error; `SIGIL_EMIT` run (0) | `aeon/build.sh:644-650`, `:792-799` | holds |
| 41 | Sonic 2 whole-ROM against a pinned real asl (A, B challenges) | `sigil-cli/tests/as_sonic2_whole_rom.rs:1-27` | holds (md5-checked tools) |
| 42 | S3K byte-identical (A, B) | `docs/QUEUE.md:826-828` | holds as PUBLISHED. No gated test file names `skdisasm` (B is right to say so) |
| 43 | `@budget` on 1 of 607 procs; `clobbers` on 596 (B Q4) | aeon grep | holds (MEASURED: 607 / 1 / 596) |
| 44 | 49 `const` names declared more than once (A F2) | aeon grep | holds within regex noise (I count 50) |

**Probes re-run** (release Sigil, `sigil emp`, MEASURED):

| Input | Bytes | Exit | Confirms |
|---|---|---|---|
| `move.w #$12345, d0` / `move.b #300, d1` | `303c 2345 123c 002c 4e75` | 0 | stage 2, A F1, B 1 |
| `const X: u8 = 300` / `move.w #X, d0` | `303c 012c 4e75` | 0 | **B challenge 4**: the const is not truncated; the annotation is ignored |
| `btst #40, d0` | `0800 0028 4e75` | 0 | B 6, A F1 |
| AS: `btst #3,Lbl(pc)`; unterminated `if`; `dc.w $12345` | correct `083a 0003 fffc`; error; error | 0; 1; 1 | lens `sig-btst-pcrel`, `sig-unterminated-block`, `sig-dcw-dcl-truncate` are fixed |

## 2. Headline numbers: labels and build shape

| Number | Stage | Label and shape stated? | Check against the raw output | Result |
|---|---|---|---|---|
| 5,864 / 3 / 3, 521 binaries | 2 | MEASURED, release, strict, aeon `1ee78b88` | Re-aggregated `test-full.log`: identical; `CARGO_EXIT=101` | holds. The 3 failures are correctly DID NOT RUN validly, not passes |
| 2.92 s s4 build, 119 MB | 2 | MEASURED, release Sigil, load 10 to 12 | Agrees with A's 2.87 s and B's 2.74 s (each at aeon HEAD) | holds |
| `--check` 2.31 s → "link/emit about 21%" | 2 | ESTIMATED by subtraction | `--check` also skips the 0.43 s contract gate (row 19) | **wrong**. A's ~7% is the defensible figure |
| Quadratic: "4k 1.6 s, 8k 6.9 s, 16k 28 s, 32k 122 s" | 2 conclusion 6 | MEASURED | The series mixes two generators: 4k and 32k are the `jsr` variant, 8k and 16k the no-`jsr` variant. The `jsr` series is 1.56 → 45.6 → 122.1 s (not a clean 4× under load) | **mislabelled**. The 4× per doubling claim holds for the no-`jsr` and no-branch series only |
| "`const W: u16 = $12345` … accepted and truncated" | 2 conclusion 5 | MEASURED | The probe shows the full value flows on | **wrong wording** (B challenge 4 holds): two defects, not one |
| 167 warnings (stage 2) / 170 (A, B) | 2, A, B | MEASURED | Stage 2 does not say which aeon revision its 167 came from; A and B are at HEAD | label gap, minor |
| 5,857 `#[test]` attributes (0) / 5,867 (B) | 0, B | MEASURED | Differing grep scopes | minor; B should cite stage 0's number or state its own regex |
| 0.43 s contract gate (A) | A | MEASURED, median of 5, `SIGIL_CONTRACTS=0` A/B, release | Method sound; not re-run | holds as labelled |
| 1.15 s `sigil build` | 1 | PUBLISHED (`build.sh:41`) | Used as the "today" value in the targets table | stale; B challenge 6 holds |
| "4 MB Thunder Force IV-scale" | task, 1, 2 | — | B notes that the retail cart is 1 MB (from memory). The 4 MB figure is a ceiling test, not TF4's size | framing only |

## 3. Coverage: in-scope files nobody read

From both coverage tables. A file that either reviewer read is not listed.

| Path | A | B | Why it matters |
|---|---|---|---|
| `sigil-harness/src/seam2.rs` (bulk; the 7 synthetic sound regions) | header + function index | not read | Home of the known region gap; the named workload is Z80-sound-heavy |
| `frontend-emp/src/z80_cycles.rs` (1,499), `z80_preserves.rs`, `z80_out_verify.rs` | not read | not read | Z80 contract and cycle analyses for the TF4 workload |
| `sigil-backend-z80`, `sigil-isa/src/z80.rs`, `decode`, `cycles` | not read | not read | Covered in part by lens `sig-z80-codegen` (holds, clean) and the 153 frozen Z80 vectors. No reviewer judgment |
| `sigil-clownlzss-sys`, `-clownnemesis-sys`, `-salvador-sys`, `sigil-s4lz` | not read | not read | All 14 `unsafe` blocks (e.g. raw `user_data` deref at `clownnemesis-sys/src/lib.rs:122`, `:143`); 19 `-Wmaybe-uninitialized` warnings in vendored C++; runs in-process every build |
| `sigil-harness/src/module_roots.rs` (`SHAPE_GATES`, 22 hard-coded aeon module ids, `:129`) | not read | not read | More Aeon-in-the-tool coupling for Q1 / second-game; known rows `ORDER-WHEN-SHAPE-GATES`, `MT-BANK-RENAME-PAIR` |
| `sigil-link/src/relax.rs:80-880` | not read | not read | The fixed critical `sig-shift-offset-org` lived at `:411` |
| `sigil-link/src/blob.rs`, `code_file.rs`, `map_load.rs` | not read | not read | `code_file.rs` is the record format |
| `sigil-cli/src/asl_mode.rs`, `build.rs` (1,067) | not read | not read | Drop-in mode and the `--version` provenance |
| `scripts/` (incl. `landing-run.sh`), `tools/` | not read | listing only | The landing bar is the merge gate |
| `docs/OVERSEER-LOG.md`, `OVERSEER-REFERENCE.md` | not read | not read | Open items below |

## 4. Known-defect list: found, or listed as unexamined?

Sources: `lens-findings.jsonl` (77 rows, 40 ids; latest state per id: 37 fixed, 3 decided/holds), `reviews/2026-08-1{3,4}`, the 2026-09-09 briefing, `DRIFT_RECORD_SEAM.md`, `QUEUE.md` open rows, `lane-status.json` (14 queue items), `OVERSEER-LOG.md` open items.

| Known defect | Source | State now | A | B |
|---|---|---|---|---|
| Region overflow reaches the ROM (`validate_section` by LMA; seam-2 sizes unenforced; `sound_tables_z80` 1,116 B > `0x400`) | QUEUE `SEAM2-TABLE-REGION-UNENFORCED`, lane-status `next` | **Open**; gap tests passed in stage 2's run | **missed**; edges note says the opposite | **missed**; says no exceeded budget produces a ROM |
| CI red (lens `sig-ci-red`, recorded fixed) | lens; `gh run list` | **Red again**: last 12 runs on master | **missed**; "says so honestly" | not examined |
| `sigil-harness` crate split deferred (S8) | closeout | Open, grown | found (driver in harness) | found (Q2) |
| Warn-tier clobber narrowing costs firings (S12; 53 in August) | closeout | Partial by design | relates to F3's 63 warnings; not linked | not examined |
| The contract gate resolves a new firing to `file:line:col` (S14, closed) | closeout | **Possibly regressed**: zero-firing families print a count only (A F3) | found the symptom; did not reconcile with S14 | not examined |
| Listing lacks the CPU per PHASE row | lane-status `PHASE-ROW-CPU` | Open | not examined | not examined |
| `pub struct` size mandatory per spec, unenforced (42 records) | lane-status `PUB-STRUCT-SIZE-MANDATORY` | Open | not examined | not examined |
| Undocumented Z80 register names: silently wrong encoding if a source defines `ixl` | OVERSEER-LOG `AS-Z80-UNDOC-REGISTER-NAMES` | Open | not examined | not examined |
| 21 to 22 shape-gated module ids hand-listed in the tool | lane-status `ORDER-WHEN-SHAPE-GATES`, `EDITOR-BINDING-COUPLING` | Open / blocked | not examined | Q1 covers the class, not these rows |
| 519 of 1,169 warnings unwatched | lane-status `WARN-TIER-UNPINNED-PREFIXES` | Open | 170-warning noise noted | 170 noted |
| `landing-run.sh` defaulted to the live aeon tree | OVERSEER-LOG "STILL OPEN … A HOLE" | **Fixed**: the literal is gone (grep) | n/a | n/a |
| Lens criticals `sig-btst-pcrel`, `sig-unterminated-block`; high `sig-dcw-dcl-truncate` | lens | Fixed; re-probed (section 1) | n/a | n/a |
| Briefing items 2, 4 to 7 | briefing addendum | Discharged | n/a | n/a |
| Briefing item 3 (Z80 `lower_abs16` fixup change, CRC re-pin) | briefing | Effectively covered: 7/7 shapes byte-identical (stage 2) | n/a | n/a |
| Drift record holds only agreements; entry count is not coverage | `DRIFT_RECORD_SEAM.md` | Design caveat | consistent with "Sigil vs Sigil" | consistent (gates table) |
| `DEAD-SAVE-DOC-AND-FLOOR-STALE`, `PARTIAL-ROWS-PRINT-PLAIN-OK` | QUEUE | Open, small | not examined | not examined |

## 5. Corrections to carry into the lead's merge

1. B's "no case where an exceeded budget produces a ROM" and A's region-containment edge note are wrong (section 4, row 1).
2. A's CI statement needs "CI is red on master since 2026-09-28" (section 4, row 2).
3. A's AS-immediate citation should be `eval.rs:3333-3338`, not `:2440-2470`.
4. Stage 2's 21% link/emit split is wrong; about 7% (A).
5. Stage 2 conclusion 6 mixes two series.
6. Stage 2 conclusion 5: the const is not truncated (B).
7. Stage 0 §8: `corpus_contracts.rs` is generic; the Aeon rows are in `contract_baseline.rs` (B).
8. Stage 0 and 1: "no independent whole-ROM oracle" holds for Aeon and `.emp` only (A, B).
9. Stage 1: the 1.15 s figure is stale (B).
10. Stage 1: "a proven pass cap" applies to the inner loop only (A F5).
11. Stage 2 §5.8: "contract enforcement weak" applies to `sigil emp`, not `sigil build` (A, B).
12. Stage 0: CI history was "not checked"; it is now MEASURED red.

## Commands run

- `sed -n` over every cited range; `grep` for the counts in rows 38, 43, 44 and for `SHAPE_GATES`, `unsafe` and `skdisasm`.
- `python3` over `lens-findings.jsonl` (latest state per id) and `lane-status.json`.
- Re-aggregation of `audit-builds/sigil/test-full.log` (`awk` over every `test result:` line); `grep` for the `gap_*` tests.
- `sigil emp` and `sigil <file.asm>` probes in `audit-builds/sigil-checker/`.
- `gh run list -L 60`, `gh run view 36488380435 --log-failed` (read-only).
