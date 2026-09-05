# TOOLTEXT-DASH-SWEEP: no em dash or en dash in text a tool shows a person

Branch `parcel/tooltext-dash-sweep`, from `master` at `5b5558d8`.

Owner ruling, 2026-09-05, to every lane in the suite: *"get rid of all current
emdashes and update so no more emdashes to all the tool agents"*. The spec is
`design/CHROME_SPEC.md`, "Text in the tools", at empyrean `origin/main`
(`f9fbfc9`), which names sigil's diagnostics, warnings and help among the
surfaces it covers.

Half of the ruling is the sweep, which happened once. The other half is that
nobody writes a new one, which is now a test.

**Every em dash still visible in this note, and in this branch's commit messages,
is a QUOTED SPECIMEN**: the substitution table, the strip set, a mutation diff, or
the before-text of an edit. A note about removing a character cannot document the
work without exhibiting it. There is no em dash in the prose of either. Flagged
here rather than left for a reader to notice, because "the rule binds what I write
too" is the half of a ruling most easily lost in the doing of it.

---

## 1. The population, re-derived, with its positive control

The dispatching brief carried a table and said to re-derive it. It does not
reproduce. Here is what does, and how it was established.

### The positive control comes first, because a zero is not a result

The brief's own first attempt at this population returned a confident **zero**
from a pathspec that matched no files, which reads exactly like a clean tool. So
before either scanner was pointed at the tree, both were pointed at a fixture
with a known right answer: **five** dash-bearing string literals, plus dashes in
a line comment, a doc comment, a nested block comment and a char literal that
must not be counted, plus a string containing `//` and an escaped quote to catch
a lexer that loses string state, plus a `\`-continued multi-line string.

Both scanners returned exactly those five, and touched nothing in the comments.
That fixture now lives inside the regression gate (§4) rather than in a
scratch directory, so the control ships with the check.

### Two independent scanners

| scanner | what it is |
|---|---|
| `rslex.py` | a hand-written Rust lexer tracking line-comment, block-comment (nested), char-literal, string-literal and raw-string state |
| `dashscan` | **rustc's own tokenizer**, by way of `proc-macro2` 1.0.106 with `span-locations`, walking the token tree and skipping `#[doc]` attributes (which is what `///` and `//!` become) |

They are independent in the way that matters: one is my reading of Rust's
lexical grammar, the other is the grammar itself. They agree to the literal.

### The measurement

493 `.rs` files scanned, 0 parse failures.

| crate | src literals | test literals |
|---|---|---|
| sigil-frontend-emp | 258 | 99 |
| sigil-harness | 152 | 87 |
| sigil-cli | 45 | 288 |
| sigil-link | 16 | 0 |
| sigil-isa | 15 | 12 |
| sigil-frontend-as | 6 | 31 |
| sigil-ir | 2 | 0 |
| sigil-clownnemesis-sys | 0 | 1 |
| **total** | **494** | **518** |

Counting dash OCCURRENCES rather than literals: **503 across 64 producer files**
and **528 across 176 test files**, 1,031 in all.

### Where it disagrees with the brief, and where it does not

| | brief | measured |
|---|---|---|
| producer, occurrences | 595 | 503 |
| producer, FILES | 64 | **64** |
| test | 556 | 518 |
| `sigil-frontend-as` producer | 55 | **6** |

The file count agrees exactly, so both measurements are looking at the same 64
files; the disagreement is entirely in how many hits each file yields. I could
not reproduce the brief's per-file counts by any metric I tried (literals,
occurrences, dash-bearing lines, with and without doc comments), and the brief's
tool is not available to me, so I can say what mine measures and not what its
measured.

The `sigil-frontend-as` row is the one worth looking at: 55 against 6, a factor
of nine. That crate's producer side is comment-dense.
`crates/sigil-frontend-as/src/eval.rs` alone carries **663 lines bearing an em
dash and only 5 of them are inside a string literal**. A lexer that loses comment
state in a 14,000-line file over-reports exactly there, and the crates where the
two measurements agree exactly (sigil-isa 15/12, sigil-link 16/0, sigil-ir 2/0,
sigil-clownnemesis-sys 0/1, sigil-frontend-as tests 31) are the small ones.

### One thing the measurement narrowed

**Every one of the 1,031 dash occurrences in Rust string literals is U+2014.**
There was not a single U+2013 en dash in any Rust string literal in this
repository. The gate still refuses both.

---

## 2. What changed, per crate

One deterministic function was applied to every dash-bearing string literal in
the workspace, producer and consumer alike. It is punctuation-only by
construction:

| rule | source pattern | becomes |
|---|---|---|
| A | ` — ` | `, ` |
| B | a dash opening a continuation line | the comma moves up to the previous line |
| C | ` —` ending a literal (a spliced sentence) | `,` |
| D | `— ` opening a literal (a spliced sentence) | `, ` |
| E | ` —` before a `\n` escape or a `\`-continuation | `,` |

A comma is the owner's first-listed replacement and reads correctly in the
register these diagnostics use ("X, the reason"). **Uniformity was a design
choice, not laziness**, and §3 is why: a per-string mix of comma, colon and
period would require every consumer substring to be hand-matched to its producer,
which is the re-baseline risk this parcel exists to avoid.

### Engagement counter: distinct string literals changed

| crate | src | tests | total |
|---|---|---|---|
| sigil-frontend-emp | 258 | 99 | 357 |
| sigil-cli | 45 | 288 | 333 |
| sigil-harness | 152 | 87 | 239 |
| sigil-frontend-as | 6 | 31 | 37 |
| sigil-isa | 15 | 12 | 27 |
| sigil-link | 16 | 0 | 16 |
| sigil-ir | 2 | 0 | 2 |
| sigil-clownnemesis-sys | 0 | 1 | 1 |
| **Rust total** | **494** | **518** | **1,012** |
| Python string tokens, 19 files (§6) | | | 43 |
| shell lines, 11 files (§6) | | | 78 |
| **grand total** | | | **1,133** |

After the sweep both scanners report **0** dash-bearing string literals anywhere
under `crates/`, producer or consumer.

Commits, in the order a person reads the surfaces:

| commit | step |
|---|---|
| `d59fb67f` | 1. sigil-cli producer, the command line surface |
| `93e39c88` | 2. the .emp and AS front-end diagnostics |
| `69d8a694` | 3. the linker, the ISA tables, the IR map |
| `2268fe04` | 4. the harness |
| `ba4348a0` | 5. the 518 consumer assertions, plus the coupled Python |
| `f9a1a854` | the regression gate |
| `a0c58d36` | the gate's control fixture, strengthened after it failed a mutation |
| `2b22cc73` | the gate's doc numbers, re-derived rather than copied |
| `8a26c478` | the shell and Python tools, after a suite failure proved the scope line had a hole |

---

## 3. The punctuation-only gate, and why it is the centre

The danger in this parcel is one specific thing: 518 test assertions quote these
strings, and when they red the tempting repair is to paste in the new actual
output. That is a re-baseline, and a re-baseline does not explain a green, it
manufactures one.

**No actual output was read back and pasted into any assertion at any point.**
The same function moved both ends. That means a test that quotes a producer
string stays green BECAUSE both moved, and a test whose diagnostic stopped firing
altogether still goes red. A hand-tuned wording on one side is precisely what
could have papered that over, which is why no wording was hand-tuned.

That property is mechanically checkable, and the check was run over the whole
diff:

> For every changed line, strip all of `— – , : ; . ( )` and all whitespace from
> the old text and from the new text. The remaining letters and digits must be
> byte-identical.

Two views were computed. The per-line view is the one the brief specified. The
whole-file view is stronger and immune to how a diff happens to pair its hunk
lines, so both are reported.

### Output, over the whole branch against `master` at `5b5558d8`

```
changed files vs master: 271
changed files that are EDITS: 270; ADDED (exempt, listed): 1
    ['crates/sigil-harness/tests/tool_text_dash_lint.rs']
whole-file stripped-identity: 270/270 identical
per-line paired: 1168 identical after stripping, 0 NOT identical
PUNCTUATION-ONLY GATE: PASS
```

The one exemption is named rather than folded in: the gate file is a NEW file, so
it has no "old text" to be identical to. The invariant is about what an EDIT did
to existing text, and every one of the 270 edited files satisfies it whole-file.

Per step, as each was committed:

| step | files | paired lines | not identical |
|---|---|---|---|
| 1. sigil-cli src | 2 | 47 | 0 |
| 2. front ends src | 36 | 270 | 0 |
| 3. link/isa/ir src | 8 | 33 | 0 |
| 4. harness src | 18 | 156 | 0 |
| 5. tests + drift_report.py | 177 | 546 | 0 |
| 6. shell and Python tools | 29 | 116 | 0 |

**Zero deviations.** Every changed line in this branch differs from its
predecessor by punctuation and whitespace alone. Nothing is being flagged for a
ruling under this heading, which is the whole reason a reviewer can accept 1,133
edits without reading 1,133 edits.

---

## 4. The regression gate, and its red-first proof

`crates/sigil-harness/tests/tool_text_dash_lint.rs`, two tests.

**Runner: the workspace test suite.** It is an integration test under
`crates/sigil-harness/tests/`, so `cargo test --workspace` runs it, which is what
`scripts/landing-run.sh` executes (`--workspace` is added at line 434). It needs
no reference tree, no `AEON_DIR` and no `SIGIL_STRICT_GATE`, so unlike the
byte-identity goldens it runs on every ordinary invocation. It reads the sources
at RUNTIME rather than at compile time, which is why a mutation in another crate
reddens it with no rebuild.

**It lexes, it does not grep.** The tree holds 10,887 dash occurrences on comment
lines against 1,012 in string literals, so a grep-based gate would have been 91
percent false positives. A check that fires on correct code is not the safe
direction; it is a delayed failure, because it trains people to weaken it.

**Loud rather than green when it cannot measure**, with every floor derived from
the tree rather than typed in as a wish:

* the crates directory must exist, or `COULD NOT MEASURE`;
* the walk must reach at least 100 `.rs` files (the tree holds 494);
* the lexer must find at least 10,000 string literals (the tree holds 38,717), or
  it has lost its place and a clean verdict is an artefact of the scan;
* an unreadable file panics rather than being skipped.

**It does not exclude itself.** Its own dashes are written as `\u{2014}` /
`\u{2013}` escapes, which are ASCII in the source text, so it is scanned like
every other file. A self-exclusion is a hole.

### Red-first proof

Three mutations. Each is shown on disk before the run, and each run is stated
with what it MUST fail before it is run.

**Mutation A: a dash back into a string literal, in a different crate.**
Applied to `crates/sigil-cli/src/main.rs`. On disk:

```
 crates/sigil-cli/src/main.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
-        println!("  revision:  unknown, {error}");
+        println!("  revision:  unknown — {error}");
```

`grep -n` confirmed the mutated text at line 242 of the file on disk, not merely
in the patch. MUST FAIL, naming that file and line. Result:

```
test no_em_or_en_dash_in_any_rust_string_literal ... FAILED
  1 string literal(s) carry an em dash (U+2014) or an en dash (U+2013). ...
  crates/sigil-cli/src/main.rs:242    revision:  unknown — {error}
test result: FAILED. 1 passed; 1 failed
```

Restored with `git checkout --` on an otherwise clean tree; `git status` confirmed
clean; re-ran green.

**Mutation B: a dash in a COMMENT must NOT fire.** This one did not need
applying, and the reason is better evidence than the mutation would have been:
the tree already contains **10,887 dash occurrences on comment lines**, including
several dozen in the same `crates/sigil-cli/src/main.rs` the gate scans, and the
gate passes green over all of them. The negative direction is established by the
population, not by a fixture.

**Mutation C: blind the lexer.** `if c == '/' && ...` became
`if false && c == '/' && ...`, disabling the line-comment branch. On disk:

```
 crates/sigil-harness/tests/tool_text_dash_lint.rs | 2 +-
-        if c == '/' && i + 1 < n && b[i + 1] == '/' {
+        if false && c == '/' && i + 1 < n && b[i + 1] == '/' {
```

MUST FAIL BOTH tests: the tree scan, because comment text starts landing in the
string bucket, and the control fixture, because a blind lexer cannot return the
fixture's known answer.

**The first run of Mutation C is a finding, and it is why the fixture changed.**
The tree scan went red. **The control fixture stayed GREEN.** Its comments held
no quote character, so a lexer that stopped skipping line comments still opened
no string inside them and still counted exactly four hits. The control was
decoration for the very mutation it existed to catch.

Two comment lines that QUOTE a dashed phrase were added to the fixture
(`a0c58d36`). Under the identical mutation, re-run:

```
test lexer_finds_dashes_in_strings_and_only_in_strings ... FAILED
  the lexer found 6 hit(s): [ "a phrase — with a dash", "another phrase – with a
  dash", "an em dash — in a string", ... ]
test no_em_or_en_dash_in_any_rust_string_literal ... FAILED
test result: FAILED. 0 passed; 2 failed
```

Restored, clean, green.

**Invariant 8(e), the proof method changed partway.** Strengthening the fixture
changed the instrument, so Mutation A was re-run against the strengthened gate
rather than left standing on the earlier one. It reddened identically, naming
`crates/sigil-cli/src/main.rs:242`. Mutation B rests on the tree's own 10,887
comment dashes and is unaffected by a fixture change.

---

## 5. The byte question, answered rather than assumed

`sigil-frontend-as` is not a side-car: aeon's `build.sh` routes residual `.asm`
data through it, so it sits in a shipping build path. String literals *should
not* be able to reach emitted bytes, but *should not* is not a measurement.

**All six changed strings in that crate, enumerated:**

| site | what it is |
|---|---|
| `eval.rs:3502` | `format!()` into a diagnostic, while-budget non-convergence |
| `eval.rs:4606` | diagnostic text, `defines.collision` |
| `eval.rs:5996` | `self.err()` span diagnostic, `branch.missing-size` |
| `eval.rs:11716` | diagnostic text, IF/ELSE/ENDIF fold |
| `expr.rs:199` | an `expect()` panic message on a thread join |
| `lib.rs:53` | the no-processor-declared refusal text |

Diagnostics go to stderr and panics abort. None is written into a section buffer,
a symbol name, or a listing record. **Nothing is BLOCKED under this heading.**

`sigil-link` also emits ROM bytes, and all 33 of its and sigil-isa's and
sigil-ir's changed lines were read individually: every one is a refusal, an
overflow diagnostic, an internal-invariant message or an assert message.

**The changed lines of every crate were then grepped for `writeln!`, `write!`,
`push_str`, `fs::write`, `to_writer` and `as_bytes`** rather than trusting that
diagnostics were all there was. That found three generator couplings, which is
the reason the grep was worth running:

1. **`crates/sigil-harness/src/repin.rs` writes `crates/sigil-harness/src/pins.rs`**,
   which is committed. Two changed strings are doc comments the generator emits,
   so the committed `pins.rs` now carries text `repin` would no longer produce.
   Nothing enforces the match: no test regenerates and compares, and the tests
   that touch it parse it for region names
   (`crates/sigil-cli/tests/probe_base_hygiene.rs`). Those lines are COMMENTS in
   the generated file, which the brief puts out of scope, so `pins.rs` is left
   alone and the next `repin` brings it into line.
2. **`crates/sigil-harness/src/bin/derive_offcanon.rs`** writes a `# GENERATED`
   header. No committed artefact in the tree carries that header, so nothing is
   stale.
3. **`crates/sigil-isa/src/asl_provenance.rs` owns `PREAMBLE`**, the header of the
   generated golden-vector provenance block, and three committed artefacts carry
   a copy of it with its em dashes:
   `crates/sigil-isa/tests/z80_golden_vectors.txt`,
   `crates/sigil-isa/tests/m68k_golden_vectors.txt`,
   `crates/sigil-frontend-as/tests/snippets_golden.txt`.
   `PREAMBLE` has exactly one reader, the writer at line 74; nothing compares the
   committed header against it, and the files are `include_str!`'d and parsed line
   by line, so no test reds either way. **See §8, flagged for a ruling.**

### What did NOT execute, stated plainly

Without `SIGIL_STRICT_GATE=1` and an `AEON_DIR` pointing at a prepared aeon
reference tree, the **byte-identity golden gates early-return and skip green**.
This parcel touched no aeon tree and none was prepared for it, so **the byte
gates did not execute**. They did not pass; they were not run. That limit is
acceptable here because the punctuation-only gate in §3 closes the same risk by a
different route, and the two must not be blurred: §3 proves no letter or digit
moved anywhere in the branch, which is a stronger statement about the source than
a byte comparison would be about one build of it.

---

## 6. The scope extension, which a red test forced

The brief drew scope at "string literals in Rust source". That line does not
close the ruling, and the suite proved it from the inside rather than my
arguing it.

### 6.1 The coupling that reddened

`crates/sigil-harness/tests/shared_target_defaults.rs:459` asserts:

```rust
text.contains("REFUSING, cannot locate the aeon checkout")
```

against the output of **`scripts/lib/suite_paths.sh`**, a shell script that still
printed `REFUSING — cannot locate the aeon checkout`. The consumer moved with the
Rust sweep and its producer did not, so the row went RED. That is the loud
direction, and it is what the whole design of §3 is for. **The fix went to the
producer. Nothing was re-baselined.**

A second coupling was found before it could red, by reading rather than by
running: `crates/sigil-cli/tests/drift_nightly_harness.rs:418` asserts
`text.contains("— nothing")` against `scripts/drift_report.py`. Both moved
together in `ba4348a0`, and `python3 scripts/drift_report.py selftest` reports
`SELFTEST PASSED` afterwards, which also witnesses that the changed strings are
on a path that executes.

### 6.2 Why the whole class rather than the one line

Patching only `suite_paths.sh:59` would have restored green and left the class
open, and **the silent half of that class is worse than the loud half**: a Rust
assertion matching a SUBSTRING that does not span a dash keeps passing while its
producer and its expected text drift apart. So every printed dash in the suite's
shell and Python tools moved under the same function:

| | files | edits |
|---|---|---|
| shell, non-comment lines | 11 | 78 |
| Python, `STRING` tokens | 19 | 43 |

Largest: `scripts/landing-run.sh` 29, `scripts/nightly_source_gates.sh` 16,
`scripts/lib/sigil_tool.sh` 8, `crates/sigil-harness/golden/capture_goldens.sh` 6.

Shell has no lexer here worth writing, so the line rule errs in the safe
direction: a line whose first non-space character is `#` is left alone, which
under-sweeps a heredoc body written with `#` and never over-sweeps a comment.
Python used the `tokenize` module, so a `COMMENT` token could never be taken for a
`STRING` one. Comments stay out of scope in every language.

**Two cross-file contracts were checked by hand**, because a uniform substitution
is only safe when both ends of a pair are inside the swept set:

* `scripts/landing-run.sh` writes `##### CLIPPY SPAN, cargo …` at line 482 and
  matches `/^##### CLIPPY SPAN,/` in awk at line 501; the same pair for TEST SPAN
  at 506 and 553. Both ends moved; verified after the edit.
* `crates/sigil-harness/golden/ab/suite_paths.py` mirrors
  `scripts/lib/suite_paths.sh`'s announce format as a stated cross-language
  contract. Both moved.

**Verified after the extension:** all 11 shell scripts pass `bash -n`; all 18
changed Python files parse under `ast.parse`; `drift_report.py selftest` still
passes.

**An asymmetry, noted rather than hidden.** Four Python DOCSTRINGS were swept
(three in `drift_report.py`, one in `ab_wavec_state.py`). Rust doc comments were
left alone as comments; a Python docstring is a `STRING` token and the tokenizer
cannot tell it from a runtime one without an AST pass.

### 6.3 What is left outside the sweep, enumerated rather than waved at

| where | dashes | note |
|---|---|---|
| Rust comments under `crates/` | 10,887 | out of scope by the ruling's own words |
| `docs/**` | ~28,900 | out of scope; the no-new-dashes half governs new writing |
| shell and Python COMMENT lines | the balance of the 300 counted | out of scope, same rule as Rust comments |
| three committed golden-vector headers | see §5 | flagged, §8 |

The regression gate covers Rust string literals only, and its module doc says so
in as many words. A shell or Python tool can still grow a dash without reddening
anything, which is the residual this parcel leaves behind.

---

## 7. Suite totals

`cargo test --workspace --no-fail-fast`, `CARGO_TARGET_DIR` inside this worktree.

`cargo test --workspace --no-fail-fast`, `SIGIL_ALLOW_PARTIAL=1`, `CARGO_TARGET_DIR`
at the repository's own gitignored `.target-land`. Log header stamps pwd, HEAD and
branch, per the standing rule that a suite log does not otherwise name its tree.

```
PWD:    .../.claude/worktrees/agent-a32eda86bbc251ed8
HEAD:   8a26c478
BRANCH: parcel/tooltext-dash-sweep
SIGIL_ALLOW_PARTIAL=1  AEON_DIR=<unset>  SIGIL_STRICT_GATE=<unset>

PASSED 4560   FAILED 0   IGNORED 2   test binaries 398
SUITE-RUN-COMPLETE exit=0
```

**Failure names: none.** Both new gate tests ran and passed
(`no_em_or_en_dash_in_any_rust_string_literal`,
`lexer_finds_dashes_in_strings_and_only_in_strings`). Zero compile errors.

`cargo clippy --workspace --all-targets -- -D warnings` exits **0**.

### What did NOT measure inside that green

**371 of those 4560 passing rows are reference-dependent and measured nothing.**
That number is derived rather than estimated: the first run of the suite, without
`SIGIL_ALLOW_PARTIAL`, produced **371 refusals at
`crates/sigil-harness/src/test_support.rs:1204`**, the d-18 guard, each saying
"NO REFERENCE TREE IS NAMED, so this run can measure nothing it could attribute,
and STOPS". Those same 371 rows are what `SIGIL_ALLOW_PARTIAL=1` converts from a
refusal into a declared, unmeasured pass. **The byte-identity golden gates are in
that set. They did not pass; they were not run.**

### Four runs, and why it took four

Reported in full because three of them were red and none of the reds was this
branch's doing except one, which was.

| run | environment | result |
|---|---|---|
| 1 | no `AEON_DIR`, no `SIGIL_ALLOW_PARTIAL` | 4183 passed, **377 failed** |
| 2 | `SIGIL_ALLOW_PARTIAL=1`, `CARGO_TARGET_DIR=<root>/target-agent` | 4558 passed, **2 failed** |
| 3 | `SIGIL_ALLOW_PARTIAL=1`, `CARGO_TARGET_DIR=<root>/target` | 4559 passed, **1 failed** |
| 4 | `SIGIL_ALLOW_PARTIAL=1`, `CARGO_TARGET_DIR=<root>/.target-land` | **4560 passed, 0 failed** |

**Run 1's 377.** 371 are the reference-tree refusal above, which is the guard
working. Of the remaining 6: 1 is the real coupling in §6.1 below, 2 are the
target-directory artefact below, and 3 are `PoisonError` cascades from a mutex
another panicking test had poisoned.

**Run 1 to run 2, the one real regression, and it was mine.**
`crates/sigil-harness/tests/shared_target_defaults.rs:459` asserts on the output
of `scripts/lib/suite_paths.sh`, a shell script the Rust-only scope line did not
cover. See §6.1. Fixed at the producer, in `8a26c478`.

**Runs 2 and 3, the target-directory artefact, and a finding about the suite
rather than about this branch.** Two tests place contradictory requirements on
`CARGO_TARGET_DIR` when it is inside the repository root:

* `crates/sigil-harness/tests/scripts_name_their_tree.rs:58` walks the repo for
  `suite_paths.py` and its `find_named` skips only a directory literally named
  `target` (or one starting with `.`). A build directory called anything else,
  holding the test beds the suite itself plants, yields 5 copies and a `COULD NOT
  MEASURE`.
* `crates/sigil-harness/tests/shared_target_defaults.rs:375` asserts the build
  directory is `assert_ne!` to `<root>/target`.

So `target-agent` reds the first and `target` reds the second, and no name inside
the root satisfies both **except a dotted one**. `.target-land` is exactly that,
and it is `scripts/landing-run.sh`'s own documented default ("the default
`$ROOT/.target-land` is gitignored"). Run 4 used it and went green. **No source
change was made for any of this**; the three logs are in `scratch/` and the only
variable across them is where the build directory points.

---

## 8. Flagged for a ruling

**One item.** The three committed golden-vector files carry a stale copy of the
`PREAMBLE` header with its em dashes:

* `crates/sigil-isa/tests/z80_golden_vectors.txt`
* `crates/sigil-isa/tests/m68k_golden_vectors.txt`
* `crates/sigil-frontend-as/tests/snippets_golden.txt`

I did not hand-edit them, and the reason is a standing one: **a regenerated
header that no generator produced is a hand-written artefact wearing a
generator's name.** The digest and banner rows below the header are measurements,
and an edit that reaches into that file to make it look freshly minted is
indistinguishable, at read time, from an edit that adjusted a measurement. The
honest path is that they lose the dashes the next time `gen-z80-vectors`,
`gen-m68k-vectors` and `gen-snippet-vectors` run against `asl`.

The counter-argument, so the ruling has both halves: the header is pure text
derived deterministically from a constant, no test compares the two, and leaving
them means three tool-readable artefacts keep dashes the ruling forbids for an
unbounded time, because nothing schedules those generators.

**Ruling wanted: hand-edit the three headers now, or wait for a regeneration.**

Nothing else is flagged. The punctuation-only gate found zero deviations across
all 1,046 changed lines.

---

## 9. Anything in this brief I concluded was wrong

**1. The population table. All eight producer rows and five of six test rows.**
Two independent scanners, one of them rustc's own tokenizer, agree on 494
producer literals (503 occurrences) and 518 test literals (528 occurrences)
against the brief's 595 and 556. The FILE count agrees exactly at 64, so this is
not a different population, it is a different count over the same files. The
worst row is `sigil-frontend-as` producer at 55 against 6, in the crate whose
`eval.rs` carries 663 dash-bearing lines of which 5 are strings.

**2. "Every re-measure of this population carries a positive control" was right,
and the brief's own numbers did not have one that would have caught this.** A
control that proves a lexer FINDS dashes in strings does not prove it EXCLUDES
them from comments. My own first fixture had the same hole and Mutation C found
it (§4). The control has to be able to fail in the direction the tool is likely
to be wrong, and for a comment-tracking lexer that direction is over-counting,
not under-counting.

**3. "No U+2014 and no U+2013" is half a rule with no work in it.** There are zero
U+2013 en dashes in Rust string literals in this repository. The gate still
refuses both, correctly, but the sweep half of that clause found nothing to do.

**4. The scope line "string literals in Rust source" does not close the ruling,
and the SUITE proved it rather than my arguing it.**
`shared_target_defaults.rs:459` asserts on the output of a SHELL script and went
RED when its consumer moved and its producer did not (§6.1). A second instance,
`drift_nightly_harness.rs:418` against a Python tool, was caught by reading
first. Any scope drawn at a language boundary has a hole wherever a tool in one
language is tested from another, and the silent version of that hole is a
substring assertion that keeps passing while the two ends drift. The sweep was
extended to 121 shell and Python edits to close the class rather than the
instance.

**5. The sequencing is right about audience and wrong about greenness, and it is
worth saying which.** Reading order (`sigil-cli` first, harness last) is a good
order for a reviewer. But steps 1 to 4 leave the branch with producers changed
and 518 consumer assertions still quoting the old text, so no intermediate commit
is individually green. That is fine and was followed as dispatched, but a
reviewer bisecting this branch should know that only `ba4348a0` and later are
expected to pass.

**6. "A before/after diagnostic-stream diff cannot distinguish correct-and-inert
from never-ran" is right, and the engagement counter the brief asked for is the
weaker of the two answers available here.** The counter (§2) says how many
strings moved, not that any of them ran. The stronger witness is that
`drift_report.py selftest` renders the report and asserts on the changed text,
and that the 518 consumer assertions in the suite quote the changed producer
strings and are checked against live output. §7's totals are the engagement
evidence; the counter is the bookkeeping.

**7. My own first attempt at Mutation C was vacuous in the fourth way the
standing note names: the proof ran an instrument that could not fail.** Not
"the patch failed to apply" (it applied, and the tree scan reddened), but "the
control fixture could not detect this class of breakage at all". Stating what a
run MUST FAIL, before running it, is what surfaced it: the run was declared to
require BOTH tests red, and only one was.

**8. The brief's `CARGO_TARGET_DIR` instruction ("a path inside your own
worktree") is under-specified, and two suite tests make it contradictory.** A
build directory inside the repo root named anything other than `target` reds
`scripts_name_their_tree.rs:58`; one named exactly `<root>/target` reds
`shared_target_defaults.rs:375`. Only a DOTTED name inside the root satisfies
both, and the repo already has one: `.target-land`, which is
`scripts/landing-run.sh`'s own default and is gitignored. It cost two suite runs
to find, and it will cost the next parcel the same unless the instruction names
it. §7 has the four runs.

**9. The engagement counter the brief specified is the weaker of the two
witnesses available, and the brief's own framing says why.** A count of strings
changed cannot distinguish "changed and inert" from "never ran" any better than
an empty stream diff can. What does distinguish them here is that 518 consumer
assertions quote the changed producer strings and were checked against LIVE
output in a 4,560-row green run, and that `drift_report.py selftest` renders its
report and asserts on the changed text. The counter is bookkeeping; §7 is the
evidence.
