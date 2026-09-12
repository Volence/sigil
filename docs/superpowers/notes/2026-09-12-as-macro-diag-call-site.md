# AS: a diagnostic inside a macro names the call, then the chain

2026-09-12, queue row `AS-MACRO-DIAG-CALL-SITE`, project SIGIL-AS-REPLACEMENT. Branch
`parcel/as-macro-diag-call-site`, base `ff580724`. Evidence (every probe, asl's full
transcript, both sigil transcripts, the corpus streams, the scripts, the mutation logs, the
census) is in `2026-09-12-as-macro-diag-call-site/` beside this note.

| commit | what |
|---|---|
| `81a24af6` | sigil-span: a source map can name an expansion, and `label` renders asl's call-site trail |
| `d1ec0fa9` | as: each macro run and loop iteration executes under its own expansion id; dedup keys go physical |
| (this note) | the note, the evidence directory, five gap-ledger lines |

## Headlines

1. **asl names the CALL, then the chain; the two earlier notes disagreed and the
   2026-09-03 one was wrong.** Measured on the pinned build: `p2_nested.asm(9) outer(2)
   inner(1):9`, and for a body written in another file, `p3_incbody.asm(3) mymac(1):9`.
   The body's file is never named. The 2026-09-03 source-map note said "AS reports the
   same way" (at the body); the 2026-09-11 note's `l6_cnop_undef.asm(16) cnop(1) org(1)`
   was right.
2. **sigil now prints asl's location and chain, loops included.** Of 36 probes, 34 match
   asl row for row with the column stripped; the other 2 are one pre-existing `irpc`
   operand divergence, a different diagnostic, not a location. `l6_cnop_undef.asm`, the
   probe the 2026-09-11 note quoted, now reads `l6_cnop_undef.asm(16) cnop(1) org(1)`,
   asl's own head, where it read `l6_cnop_undef.asm(5)`.
3. **Sonic 2: only locations moved.** 79 rows before and after, the (level, message)
   multiset identical, the row order identical. The 34 rows inside `dac_sample_metadata`
   went from one location, `s2.sounddriver.asm(3905)`, to their 17 distinct calls.
4. **No byte moved** on 1,110 images (every committed `.asm` that builds), and **the
   `.emp` tier's output is byte-identical** on 76 runs. Sonic 2 builds no image in either
   configuration I could make, so it gives no byte evidence (below).
5. **Every half-fix tried went red**: 10 mutations plus one against a pre-existing test.

## The asl measurements

Instrument: `s1disasm/build_tools/Linux-x86_64/asl`, md5
**`61e672562465725a8c102288a7da9098`**, through `asl_run` (`asl_ref.sh`), invocation
`asl_run -xx -n -q -A -L -U -i . <probe>.asm` from the probe's directory
(`scripts/run_asl.sh`). Every probe is deliberately failing, so the transcript is read for
diagnostic text only; no byte value is quoted from any of these runs. Full transcript:
`logs/asl-transcript.txt` (37 probes; every run prints the md5 once at the top).

The location lines, verbatim (the `> > > ` prefix is asl's):

```text
p1_simple.asm(7) mymac(2):9: error #1200: unknown instruction
p1_simple.asm(8) mymac(2):9: error #1200: unknown instruction
p2_nested.asm(9) outer(2) inner(1):9: error #1200: unknown instruction
p3_incbody.asm(3) mymac(1):9: error #1200: unknown instruction          (body in p3_mac.inc)
p4_rept.asm(8) mymac(3) REPT 1(1):9: error #1200: unknown instruction
p4_rept.asm(8) mymac(3) REPT 2(1):9: error #1200: unknown instruction
p5_while.asm(10) mymac(5) WHILE 1/1:9: error #1200: unknown instruction
p6_warn.asm(6) mymac(1): warning: warn from body
p7_undef.asm(6) mymac(1):14: error #1010: symbol undefined               (a later-pass error)
p8_toprept.asm(5) REPT 1(1):2: error #1200: unknown instruction          (file-level rept)
p9_topirp.asm(5) IRP:2(1):9: error #1200: unknown instruction
p10_macirp.asm(8) mymac(3) IRP:2(1):9: error #1200: unknown instruction
p11_call.inc(3) mymac(1):9: error #1200: unknown instruction             (call in an included file)
p12_nest_rept.asm(10) outer(3) REPT 1(1)inner(1):9: error #1200: unknown instruction
p13_usererr.asm(6) mymac(1): error: user error from body
p14_top_control.asm(3):2: error #1200: unknown instruction               (no expansion)
p15_body.inc(2):2: error #1200: unknown instruction                      (include inside a macro)
q1_irp3.asm(5) IRP:bb(2):9   IRP:cc(2):9   IRP:(2):9
q3_topwhile.asm(7) WHILE 1/2:2   WHILE 2/2:2
q5_reptrept.asm(6) REPT 1(3)REPT 1(1):2   ...   REPT 2(3)REPT 2(1):2
q6_three.asm(14) outer(1) mid(3) inner(2):9: error #1200: unknown instruction
q7_nest_rept_space.asm(11) outer(4) REPT 1(2)inner(1):9: error #1200: unknown instruction
q8_rept_in_inner.asm(11) outer(2) inner(3) REPT 1(1):9: error #1200: unknown instruction
q10_mac_in_toprept.asm(7) REPT 1(1)mymac(1):9: error #1200: unknown instruction
r3_irp_space.asm(4) IRP:bb cc(1):9   IRP:dd(1):9   IRP:(1):9             (irp x,aa,  bb cc  ,dd)
r6_irpc_abc.asm(4) IRPC:'b'(1):9   IRPC:'c'(1):9   IRPC:'(1):9
r8_irpc_num.asm(4) IRPC:'5'(1):9   IRPC:'(1):9                           (irpc x,65)
```

The rule these fix:

- The head is the OUTERMOST call's `file(line)`: the file the call is written in
  (`p11`), never the file holding the body (`p3`).
- Then one frame per expansion, outermost first. A frame's number is a line of THAT
  expansion's body, from 1: the line the next frame was entered from, or for the innermost
  frame the line the diagnostic is on (`q6`: `outer(1) mid(3) inner(2)`).
- A loop is entered from its CLOSING line: asl collects the block before it runs it, so
  the enclosing frame names the `endm` (`p4`: `mymac(3)` is the `rept`'s `endm`; `p8`: the
  file-level head is line 5, the `endm`, not line 4, the bad line).
- Spellings: `name(n)`, `REPT i(n)`, `WHILE i/n` (a slash), `IRP:next(n)` where `next` is
  the item AFTER this iteration's, empty on the last, and `IRPC:'c'(n)` with the next
  character, a lone `IRPC:'` on the last.
- A macro frame is followed by a space and a loop frame is not (`REPT 1(2)inner(1)`,
  `REPT 1(3)REPT 1(1)`, but `outer(2) inner(3) REPT 1(1)`).
- An `include` inside a macro body splices a FILE, and its lines report that file alone,
  with no trail (`p15`).
- Warnings carry the trail as errors do (`p6`, `p18`), and so does a diagnostic asl only
  judges on a later pass (`p7`).

## Design

**Chosen: an expansion is a source.** `SourceMap` (sigil-span) holds EXPANSIONS beside
files. Each run of a macro body, and each iteration of a `rept`, `while`, `irp` or `irpc`
body, is `add_expansion(body_source, body_start, call_span, Frame)` and gets a `SourceId`
of its own in a separate range (bit 31 set, so every file id keeps the number it had and
`SourceId(u32::MAX)` stays the no-source sentinel). The frontend restamps the run's lines
with that id (`enter_expansion`); text and offsets do not change. `label()` walks the
chain from a span to its outermost call and renders asl's spelling; `text`, `name` and
`location` resolve an expansion to the file its body was written in (`physical`), so they
answer exactly what they answered before.

**Rejected: a trail field on `Diagnostic`** (route (a)). The deciding fact is the linker:
it raises diagnostics after every expansion is gone, from a fixup's span alone (`p7`'s
undefined symbol is a link-time row in sigil). Two calls of one macro produce the same
body span, so no trail attached at raise time can reach a link-time diagnostic, and no
side table keyed on the span can tell the calls apart. The span itself has to identify the
run, and once it does, a field on `Diagnostic` adds nothing. `Diagnostic` is unchanged:
zero constructions touched.

**Consumers, by what touches them** (enumerated at the base revision):

| what | where | outcome |
|---|---|---|
| makes a `SourceId` for AS lines | `split_src_lines` (root, `include`), `SrcLine::new`, the per-line `source` field | unchanged for files; restamped per run by `enter_expansion` |
| builds spans from a line | `lex_line(..., line.source, line.base)` and the `Span { source: line.source, .. }` literals | carry the run's id; offsets unchanged |
| compares two tokens' sources | `expand.rs:351`, `expr.rs:352`, `call_args`'s `head.source == *source` | unaffected: every token of one line carries the line's one id |
| keys "already reported here" | `arg_faults_seen` (2 sites), `reg_faults_seen`, `register_reported_at`, `cond_faults_seen`, `expand_faults_seen`, the in-pass `author_warnings` list | re-keyed PHYSICAL (`site_key`), so a body line is still one position |
| carries a diagnostic across passes | `terminal_fatal`, `author_warnings`, `carried_*`, `merge_carried_*` | carry the physical span beside the span (`Carried`), because run ids are handed out per pass |
| renders a location | CLI `render_as_diags`, `render_located_diags`, `render_as_warnings`; harness `BuildWarning.location`; eval's `fatal`/`warning` labels and the circular-layout "at" text | `label()`: now the trail |
| reads text, name, line | `text`, `name`, `location` (eval, harness `diag_render`, the `.emp` CLI paths) | resolve through `physical`: identical answers |
| counts sources | `len()` / `is_empty()`; the harness `diag_render` range check | files only, as documented. **The range check was a defect**, not "fine": it read every expansion id as no file. Found in review, fixed by `SourceMap::contains`; see "Review finding" at the end |
| linker | sigil-link never reads a `SourceMap`; its diagnostics carry the fixup span | rendered by the CLI through `label`: the trail (test `a_link_time_error_inside_a_macro_names_the_call`) |

`SourceMap`'s three parallel per-file vectors became one `Vec<File>`: a third vector would
have pushed the frontend's `Failure` past clippy's `result_large_err` threshold. The map is
now two vectors wide, smaller than before.

## Deviations from asl, each flagged

1. **The column inside an expansion** (pre-existing, unchanged). asl counts a tab as eight
   columns there (`p1_simple.asm(7) mymac(2):9`); sigil counts characters (`:2`). asl is not
   consistent about it either: a file-level `rept` body gives `:2` (`p8`) and a file-level
   `irp` body `:9` (`p9`). Ledgered as `AS-TRAIL-COLUMN-TAB8`.
2. **A `warning` or `error` directive's line** (pre-existing). asl prints no column there
   (`p6_warn.asm(6) mymac(1): warning:`); sigil prints one, as everywhere. Ledgered.
3. **Once per position** (pre-existing sigil policy, kept). The four per-pass dedup sets
   report a body line once per pass, at the first call that reached it; asl reports every
   call. The trail changes where the one report points, never how many there are. Kept on
   purpose (the corpus's 81-expansion `if MOMPASS=1` is why those sets exist). Ledgered.
4. **Not a deviation, and considered: naming the body's file.** A reader of a 300-file
   corpus would gain from seeing where `mymac` is written, and asl does not print it. It
   is NOT added: the standing ruling is that a compatibility surface prints what the thing
   it is compatible with prints, and adding it inline would change asl's head, or adding a
   `note:` line would change every count. Ledgered as `AS-TRAIL-BODY-FILE` for a ruling.
5. **Not a location:** `irpc x,abc` with a bare word (`q2`, `r4`): asl iterates, sigil
   refuses the operand. Pre-existing; the two probe DIFFs in the matrix. Ledgered.

## The existing test that pinned the body's file

`cli_diagnostic_location.rs::an_error_in_a_macro_body_names_the_file_the_body_was_written_in`
required `mac.inc(3):`, the body. asl on the same two files prints `mroot.asm(3)
mymac(1):9` (probe `p3` is the same shape) and never names the body's file. Decided on
that evidence: the test's property is not asl's, so it is renamed
`an_error_in_a_macro_body_names_the_call_then_the_macro_line`, requires `mroot.asm(3)
mymac(1):`, and refuses any line of the report headed by the body's file. The fixture is
kept, body in a second file, so the two answers cannot be confused.

## Pins: every test that pinned a location inside an expansion

Two nets. **At the base revision**, before any test file was edited, `git grep -lE 'macro'
-- 'crates/*/tests/*.rs' 'crates/sigil-frontend-as/src/*.rs' 'crates/sigil-span/src/*.rs' |
xargs grep -lE '\.(asm|inc)\([0-9]+\)|\(\{?[0-9]+\}?\):|label\('` (29 files) for tests that
read locations in files mentioning macros, and a grep for negative assertions keyed on a
location (`!…contains(`, `!…starts_with(`, 7 hits:
`cli_diagnostic_location.rs:94` (include, no expansion), `as_fatal_survives_its_pass.rs:240,
347` (`inc/b.asm`, no expansion), `as_register_in_value_position.rs:256` (no expansion), the
rest not locations). **Then a census**, because a sweep can hollow a negative assertion
without reddening it: a scaffold (`scripts/census_scaffold.py`, never committed) made
`label()` append every trail it rendered, with the test thread's name, to a file; the
frontend, span and CLI suites ran with it on the committed tree `d1ec0fa9`, all three exit
0, and the file was restored from HEAD with the tree clean after
(`logs/census-run.log`). Every test that computed a trail (`logs/census.txt`):

- my own: the 13 trail tests, 5 sigil-span unit tests, the 2 CLI tests (`mroot.asm`,
  `p7_undef.asm`);
- `failed_run_reports_everything.rs`, five tests over the `levartptrs` fixture. Its
  helper read the LAST `(n)` of a label as the line. Two went red (the rows are lines 7
  and 8 of the macro body, now `probe.asm(13) levartptrs(1)` and `(2)`), and **one green
  negative was hollowed**: "no diagnostic at line 9" could no longer fail, because line 9
  now reads `levartptrs(3)`. Fixed: the helper compares the whole location head; the
  negative is keyed `probe.asm(13) levartptrs(3)` and preceded by a check that the same
  call site does appear for body lines 1 and 2 (m1 below reddens that check, so it can
  fail). The other two tests assert top-level rows only and moved to string keys with the
  helper.
- `as_unresolved_if_condition.rs::one_condition_reached_many_times_is_reported_once`: it
  asserts a COUNT (one verdict for four expansions), not a location, so nothing to change;
  it is what the physical key protects, and m8b shows it goes red without it.

Not measured here: `sigil-harness` tests and `warn_tier_corpus.rs` need the aeon tree. A
grep of `warn_tier_corpus.rs` and `repin_pins.rs` finds no AS-file location and no
`[as.*]` id registered in either.

## Sonic 2, before and after

Corpus: `git -C s2disasm archive HEAD` (`e45ebf33`) into the scratch directory, run from
its own directory, `sigil s2.asm -o <scratch>` (`scripts/run_s2.sh`). Before: base binary
`ff580724` (md5 `40ee4e28`); after: `d1ec0fa9` (md5 `b08524d1`, `--version` clean tree).
`scripts/setcmp.py`, full output `logs/s2-setcmp.txt`:

| | before | after |
|---|---:|---:|
| rows | 79 | 79 |
| (level, message) multiset | | IDENTICAL, 0 gone, 0 arrived |
| row order and messages, position by position | | identical |
| rows whose location changed | | 34 |
| rows carrying a trail | 0 | 34 |
| distinct locations | 53 | 79 |
| stdout | | identical |

Per class (both sides): 39 `cannot include …generated…` (31 songs, 7 DAC samples, 1 PCM
sample: the archive lacks `build.lua`'s generated sound inputs), 17 `division by zero`, 17 `int(): could not evaluate float
expression`, 3 `unresolved symbol … in operand`, 2 `unresolved if condition`, 1 `shared`
warning. The 34 that moved are the two `dac_sample_metadata` rows per call:

```text
before  s2.sounddriver.asm(3905):45: error: int(): could not evaluate float expression   (x17, 3905 each)
after   s2.sounddriver.asm(3908) dac_sample_metadata(5):45: error: int(): ...
        ...
        s2.sounddriver.asm(3924) dac_sample_metadata(5):46: error: int(): ...
```

17 calls, lines 3908 to 3924, one per `dac_sample_metadata` line; `(5)` is the macro's
`db` line. No row outside a macro moved.

**A second configuration**, the archive plus `build.lua`'s own PCM/DPCM conversion and
every song's `.inc` in uncompressed form (`scripts/s2gen.lua`, no asl and no saxman run):
2 rows before and after, multiset identical, and the `finishBank` fatal inside its macro
moved to `s2.asm(91259) finishBank(2):3` (`logs/s2gen-setcmp.txt`). Uncompressed songs
overflow the sound bank, so this configuration builds no image either.

## Byte neutrality

The change touches no emission path: lines keep their text and offsets and change only
the id they carry. Measured, not argued:

- **Every committed `.asm` in this repository** (`scripts/byte_corpus.sh`, 1,945 files,
  each assembled by both binaries from its own directory): **1,110 images, all 1,110
  byte-identical** (size and crc32 of each in `logs/byte-corpus-report.txt`), no exit
  status changed, no (level, message) multiset changed; 1,894 stderr streams identical
  and 51 differing in locations only.
- **Sonic 2**: no image in either configuration, before or after, so no crc32 exists to
  compare. Stated as the limit it is.
- **The limit**: none of this is the aeon shapes. aeon's shipping build routes three
  `.asm` files through this frontend, and the leased tree was not used; the controller's
  landing gate is the aeon proof.

## The `.emp` tier did not change

`scripts/emp_compare.sh`: every committed `.emp` (19) and a copy of each with an invalid
line appended, through `sigil parse` and `sigil emp`, on the base and after binaries:
**76 runs, 76 identical** in exit status, stdout and stderr; 51 of them carry a
`path:line:col:` location, so the comparison had locations to differ on; 8 images, all
identical (`logs/emp-compare.txt`). By construction `.emp` adds no expansion, and `label`
and `location` on a file id are unchanged. (A first run of the script showed 8 stdout
differences; each was the output path echoed by `sigil emp`, `…/base/x.bin` against
`…/new/x.bin`. The script now gives both sides one path.)

## Tests, red first

New: `crates/sigil-frontend-as/tests/as_macro_call_site_trail.rs` (13 tests, fixtures
are the probes byte for byte, expectations are asl's lines with sigil's column);
`cli_diagnostic_location.rs::a_link_time_error_inside_a_macro_names_the_call`; five
sigil-span unit tests on the renderer.

Each mutation was applied to the committed tree `d1ec0fa9` (clean), quoted back from disk
by `scripts/mutate.py` (which refuses unless the old text occurs once), run, and the file
restored with `git show HEAD:<path>`; the tracked tree was clean before and after every
one (`logs/mutations/red-*.log`).

| # | half-fix | mutated line, read back from disk | red |
|---|---|---|---|
| m1 | the feature absent: no run gets an id | `eval.rs:5670 let Some(first) = body.first().filter(\|_\| false) else {` | 15, incl. both CLI tests, all 3 re-keyed `levartptrs` tests, 10 trail tests |
| m2 | `rept` not framed | `eval.rs:5467 let _ = (entry, n);` | the 3 loop tests |
| m3 | a loop entered from its OPENING line | `eval.rs:5426 let entry = loop_entry(&lines[start]);` | the 3 loop tests |
| m4 | a space after every frame | `lib.rs:160 true` | `a_space_follows…`, `a_file_level_loop…`, unit `a_nested_trail…` |
| m5 | only the innermost call named | `lib.rs:300 break;` | 5, incl. `nested_macros…` and the unit nesting test |
| m6 | IRP named by the CURRENT item | `eval.rs:5562 let next = items.get(at);` | `a_loop_in_a_macro…`, `a_file_level_loop…` |
| m7 | `WHILE` with parentheses | `lib.rs:151 Frame::While(n) => write!(out, "WHILE {n}({line})"),` | 2 trail tests, unit `loop_frames_spell…` |
| m8 | "already reported" keyed on the run id | `eval.rs:5685 let p = span;` | `a_body_line_reported_once_per_pass_stays_reported_once` |
| m8b | the same, against the pre-existing test | same line | `one_condition_reached_many_times_is_reported_once` |
| m9 | a carried warning matched by raw span | `if diags.iter().any(\|d\| d.primary == *span) {` | `a_carried_warning_is_matched_by_position_when_run_ids_shift_between_passes` |
| m10 | frame line counted from the file | `trail.push((&e.frame, line + 0 * first));` | 14 |

None stayed green.

## Suites and clippy, on `d1ec0fa9`, tracked tree clean at start

| crate | result lines | passed | failed | ignored |
|---|---:|---:|---:|---:|
| sigil-span | 3 | 20 | 0 | 0 |
| sigil-frontend-as | 79 | 876 | 0 | 0 |
| sigil-cli (`SIGIL_ALLOW_PARTIAL=1`, no `AEON_DIR`) | 172 | 822 | 0 | 1 |

The ignored test is `sigil_diff_reports_byte_identity` ("reads the aeon source tree; run
with --ignored"). `cargo clippy --release -p sigil-span -p sigil-frontend-as -p sigil-cli
--all-targets -- -D warnings`: exit 0. The machine was loaded by another lane's run
throughout; no failure appeared at any point, loaded or not. No line added by either code
commit contains U+2013 or U+2014.

## Open

- **The column inside an expansion** (asl's tab-as-eight), **the column on a directive's
  line**, **naming the body's file**, and **the bare-word `irpc`**: ledgered, each above.
- **aeon**: unmeasured here by instruction; the landing gate. If aeon's AS units raise any
  diagnostic inside a macro or loop, its location text changes by design.
- **Cost**: not measured. One expansion entry per run (a name `Arc` and four words), and a
  file-level loop body is copied once per loop execution so it can be restamped.

## Things in the brief that turned out wrong

1. **"The 30 `music_metadata` rows: before, one location; after, 30 distinct call
   sites."** Those rows are gone at this master: the 2026-09-11 parcel's feature 1 (`()` is
   the value 0) removed them, and the base binary reports no row at `sounddriver.asm(3817)`.
   The same shape is present in this corpus as the 34 `dac_sample_metadata` rows (17
   calls), and those went from one location to 17.
2. **The two notes' disagreement** resolves against the 2026-09-03 note: asl does not report
   at the body.
3. **"Measure rept/while bodies inside a macro"** understates asl: loops are frames at FILE
   level too, and asl names a file-level loop by its closing line (`p8_toprept.asm(5) REPT
   1(1)` for a bad line 4). Implementing the macro half alone would have been a partial
   asl; this parcel frames loops everywhere, so file-level loop diagnostics moved too
   (none in Sonic 2).
4. **"~129 `Diagnostic {` lines across 7 crates"** did not need re-deriving: the chosen
   route constructs no `Diagnostic` differently, and the linker fact rules out the
   field route before its cost matters.

## Review finding: a locator that range-checked the file count

The coordinator's review, from this note's own "counts sources" row: the harness renderer
`SourceTexts::locate` (`sigil-harness/src/diag_render.rs`) refused any span whose source id
was `>= SourceMap::len()`. `len()` counts files only, and an expansion id has bit 31 set, so
the check read every VALID expansion span of its own map as "no file" and printed the raw
byte span. The consumer table above listed the check and called it fine; that was wrong.

**Who can reach it, enumerated by what constructs and passes the map:**

| locator | constructed at | map holds | spans it is handed |
|---|---|---|---|
| `SourceTexts` | `seam1.rs` x2 (resident sound modules), `seam2.rs` x7 (dac_samples, sfx_bank, seq_opcode_tab, sound_tables_z80, movingtrucks_pitchtable, mt_bank, via `lower_emp_file` and direct `parse_file`), `tests/check_only_census.rs` x1 | `.emp` files only (`add` is its only way in; every registration is a `.emp` path through the `.emp` parser) | `.emp` parse, import and `check_link_asserts` diagnostics; the AS frontend constructs no `LinkAssert` (0 hits) |
| `SourceIndex` (emp manifest) | `native.rs` `build_emp` error render, `BuildWarning::new`, `collect_warnings`, `resolve_chained` (`true_bases_by_index`, `render_declared_chain`, `declared_chain_drift_verdict`); CLI `render_program_diags` and the contract-baseline `report_added`; `test_support.rs` link-assert filter | `.emp` manifest files only (built by `SourceMap::add`) | `.emp` diagnostics, EXCEPT `resolve_chained`: its `resolve_layout` errors and `true_bases_by_index` flips range over a section list holding the AS side's sections too |
| AS `SourceMap` | the AS frontend; rendered by `label()` in the CLI and in `BuildWarning::from_as` | files and expansions | AS diagnostics |

So **no path hands `SourceTexts` an AS map or an AS span**, and its map cannot hold an
expansion: the check could not misfire today. It is fixed anyway, because the next map
that holds one would lose every location without a word: `SourceMap::contains(id)` (a file
of this map or an expansion of this map) replaces the range check, and a span inside an
expansion locates in the file its body was written in. Physical rather than the asl trail,
because this renderer speaks the `.emp` dialect, `path:line:col`, which has no place for a
trail, and `label` is where the AS surface renders it.

**A second finding, pre-existing and not fixed here.** `resolve_chained` locates spans from
BOTH front ends through the `.emp` index. Span ids are not namespaced by front end, so an AS
file id `k` there named the `.emp` manifest's `k`-th file, at a line computed against the
wrong text (the harness's own `BuildWarning::from_as` comment names this hazard). An AS
expansion id now finds no path and prints its raw span instead: less wrong, still not right.
Fixing it needs the span to say which map it belongs to, or the mixed list to carry both
maps. Ledgered as `MIXED-MAP-LOCATE`.

**Every other range check on a `SourceId`, workspace-wide** (`git grep` for `.source.0`,
`SourceId(...)`, `as usize >= ….len()`):

- `sigil-frontend-emp/src/resolve/manifest.rs:163`, `SourceIndex::locate`,
  `self.paths.get(span.source.0 as usize)`: the same shape, left as it is. Its map is built
  only by `add` over manifest files and holds no expansion, so an expansion id reaching it
  is necessarily foreign and `None` is the right answer. Changing it would touch the `.emp`
  surface for no reachable difference.
- Everything else is an identity or ordering key, not a range check: `eval.rs` `site_key`,
  `corpus_contracts.rs:1254` (sort key), `preserves.rs:1448` (dedup), `lower/code.rs:132`
  (net name), `native.rs:2130` (`generated` identity) and `:2954` (fallback text), and the
  synthetic `SourceId(manifest.modules.len())` ids in `native.rs` and two tests, which are
  past the manifest on purpose.
- `sigil-span` itself: `text` and `location` index files after `backing`; `name` and `label`
  use `get`.

**Red-first** (`logs/mutations/red-r*.log`, each on the committed tree `11e3dca2`, quoted
back from disk, restored with `git show HEAD:<path>`, tree clean before and after):

| # | half-fix | mutated line, read back from disk | red |
|---|---|---|---|
| r1 | the file-count range check back in the locator | `diag_render.rs:57 if span.source.0 as usize >= self.map.len() {` | `a_span_inside_an_expansion_of_the_map_is_located_in_its_body_file` |
| r2 | `contains` that knows files only | `lib.rs:214 (id.0 as usize) < self.files.len()` | that test and `contains_answers_for_files_and_this_maps_own_expansions` |
| r3 | `contains` that answers yes for anything | `lib.rs:214 let _ = id; true` | `contains_answers_for_files_and_this_maps_own_expansions` |

None stayed green. The locator test renders through `render_diag_lines`, so its red is the
line a reader would have seen: the raw span, not `probe/m.asm:3:2`.

**Suites and clippy on `11e3dca2`**, tracked tree clean at the start of every run:

| crate | result lines | passed | failed | ignored |
|---|---:|---:|---:|---:|
| sigil-span | 3 | 21 | 0 | 0 |
| sigil-frontend-as | 79 | 876 | 0 | 0 |
| sigil-cli (`SIGIL_ALLOW_PARTIAL=1`, no `AEON_DIR`) | 172 | 822 | 0 | 1 |
| sigil-harness (`SIGIL_ALLOW_PARTIAL=1`, no `AEON_DIR`) | 52 | 486 | 0 | 1 |

The ignored two are `sigil_diff_reports_byte_identity` (reads the aeon tree) and
`secondary_pin_classes_match_the_hand_typed_baseline` (retired by Wave-B B-0). The
harness's aeon-dependent tests do not run in a partial run; the landing gate runs them.
`cargo clippy --release -p sigil-span -p sigil-frontend-as -p sigil-cli -p sigil-harness
--all-targets -- -D warnings`: exit 0.
