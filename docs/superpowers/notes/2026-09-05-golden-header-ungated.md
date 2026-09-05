# GOLDEN-HEADER-UNGATED

Branch `parcel/golden-header-ungated`, off master `0587f2af`.

Three committed artifacts carried a stale copy of a generated header:

| artifact | em dashes before | after |
| --- | --- | --- |
| `crates/sigil-isa/tests/z80_golden_vectors.txt` | 6 | 0 |
| `crates/sigil-isa/tests/m68k_golden_vectors.txt` | 6 | 0 |
| `crates/sigil-frontend-as/tests/snippets_golden.txt` | 6 | 0 |

**The dashes were the symptom. The defect was that nothing compared the two.**

The cause was the dash sweep `f6618ec9`. It correctly swept string literals and
correctly left comments alone, but `PREAMBLE` and `push_banner` in
`crates/sigil-isa/src/asl_provenance.rs` are string literals that RENDER HEADER
PROSE into generated files, so the generator's output moved while the committed
artifacts stood still. The identical cause hit a fourth artifact,
`crates/sigil-harness/src/pins.rs`, which HAD a currency check and went red within
hours (`d6ed3bec`). One cause, four artifacts; the only difference in how it
surfaced was whether somebody had written a comparison.

## The risk that decided the parcel

Regenerating re-mints the whole file, not just the header, and the rows below the
header are real digest measurements produced by `asl`. A moved row would have been
a BLOCKED report, not a tidy-up. **The bar was to prove only header prose moved.**

## Per-file diff shape, proving only header prose moved

Measured by diffing the pre-regeneration snapshot against the regenerated file:

| file | lines | changed | changed lines outside the `#` header | body rows | body md5 before | body md5 after |
| --- | --- | --- | --- | --- | --- | --- |
| `z80_golden_vectors.txt` | 144 | 6 | **0** | 121 | `aac464097e7df0fa4f60a91a46981476` | `aac464097e7df0fa4f60a91a46981476` |
| `m68k_golden_vectors.txt` | 200 | 6 | **0** | 177 | `90c0c7af2d0b7b342695935342b4c84c` | `90c0c7af2d0b7b342695935342b4c84c` |
| `snippets_golden.txt` | 2187 | 6 | **0** | 2164 | `410a95d3f37c1ddc41a3c24b9d0ce9a3` | `410a95d3f37c1ddc41a3c24b9d0ce9a3` |

Three further checks, because "the bodies matched" is a convenient result and a
convenient result is a trigger:

* **The body split excludes nothing but the header.** Each file's first non-`#`
  line is line 24, and there are **0** `#`-leading lines after line 23. So
  `grep -v '^#'` is the body, exactly.
* **The changed header lines are punctuation only.** Strip every non-alphanumeric
  character from each side of the diff and the two agree byte for byte, per file.
  The letters and digits did not move; only em dashes became commas.
* **The prover is not blind.** Perturbing one body row in the snapshot
  (`nop => 00` to `nop => 01`) makes it print `BODY: DIFFERS  ***BLOCKED***` and
  name the row. Restoring returns the green above. The check could have given the
  other answer, and was made to.

## asl provenance of the mint

The reference build, selected through `docs/superpowers/notes/asl-reference/asl_ref.sh`
with `|| exit $?` propagated:

* `asl` **md5 `61e672562465725a8c102288a7da9098`**,
  `s1disasm/build_tools/Linux-x86_64/asl`.
* `p2bin` **md5 `4f2fff99c3347bafb93b12d5be1db754`**, beside it.
* `s2disasm`'s copy, md5 `0dee1f98e6480a4783d27ffd8b90896f`, was never invoked. It
  substitutes an uninitialized word for operands it declines and answers
  differently every run.

**Exit status, not only the digest:**

| generator | exit | vectors written |
| --- | --- | --- |
| `gen-z80-vectors` | **0** | 120 |
| `gen-m68k-vectors` | **0** | 176 |
| `gen_snippet_vectors` | **0** | 227 |

Each generator asserts `asl`'s and `p2bin`'s own `status.success()` at every
invocation (`gen_z80_vectors.rs:125`, `gen_m68k_vectors.rs:112`,
`gen_snippet_vectors.rs:207`) and panics otherwise, so a zero from the generator
carries all 523 asl runs rather than only the last one. That matters here for the
stated reason: a run carrying any error is not a source of values for the lines
that DID assemble, because one bad line can corrupt an unrelated correct line
while the listing still looks complete.

The three regenerated files' own headers still record `asl-md5
61e672562465725a8c102288a7da9098`, unchanged, which is the same build that minted
them before. That is why no measurement could move, and it is a fact about the
instrument rather than a substitute for the row comparison above.

## The comparison

`crates/sigil-harness/tests/golden_header_currency.rs`. It reads the provenance
VALUES out of each committed file, rebuilds a `Provenance`, renders it with the
SAME `header()` the generators call, and requires byte equality with the committed
header.

**Derived from the renderer, never duplicated into a fixture.** No copy of
`PREAMBLE` or of `push_banner`'s prose exists in the test, so there is nothing to
keep in step and no way for it to go green because somebody maintained it. The
inverse that reads the values back is structural rather than textual: `push_field`
writes a space-free label then two spaces, and `push_banner` emits an unlabelled
continuation line ONLY for an absent banner, so the parse tells the two banner
shapes apart without quoting either one's words.

**No asl is run and no digest is pinned.** The values come from the file, so this
runs in CI where asl is out of repo (P4d / OQ-A), and the gate stays honest about
its scope: it says the header is CURRENT, never that the build named in it was the
right one.

### Red first, with the mutation shown applied on disk

`git checkout <rev> -- <path>` **stages**, so plain `git diff --stat` prints
nothing on an applied mutation. Every mutation below was proven landed with
`git diff HEAD` or a content grep, and restored from a committed baseline after.

| # | mutation | on-disk proof | verdict |
| --- | --- | --- | --- |
| A | artifact reverted to its pre-regeneration text (`git checkout 0587f2af -- z80_golden_vectors.txt`) | em dash count on disk 0 to 6; `git diff HEAD --stat` shows 6/6 while `git diff --stat` shows nothing | **RED**, 1 of 3, first difference at header line 1, names `gen-z80-vectors` |
| B | **`PREAMBLE` perturbed, artifacts untouched** | `sed -n 124p` shows the changed literal; `git diff HEAD` on the source | **RED**, 3 of 3, all at header line 1 |
| C | `push_banner`'s no-banner prose perturbed | `git diff HEAD` on the source | **RED**, 3 of 3, at header line 22 |
| control | `asl-md5` in a committed artifact set to 32 zeroes | `git diff HEAD` on the artifact | **GREEN**, deliberately, see the scope limit below |

**B is the case that matters.** It is the actual 2026-09-05 failure mode: the
generator moving underneath static files nobody touched. A check that only fired
when the artifact was edited would have slept through the whole of it. C shows the
gate covers the entire header, including the second literal the sweep moved, not
just the preamble.

The message names the drift and points at regeneration:

```
STALE GENERATED HEADER: crates/sigil-isa/tests/z80_golden_vectors.txt
...
  first difference at header line 1:
    committed: "# PROVENANCE, generated. This records WHICH BUILD ANSWERED."
    generator: "# PROVENANCE, generated. This records exactly WHICH BUILD ANSWERED."

FIX BY REGENERATING, never by hand editing the header:
    ASL_BIN=<asl> cargo run -p sigil-isa --bin gen-z80-vectors
```

Green after every restore: 4 passed, 0 failed.

### What the gate does NOT cover, measured rather than assumed

* **A hand-edited provenance value.** The control row above: set `asl-md5` to 32
  zeroes and the gate stays green, because the values are read from the file and
  rendered back, so a lie about which build answered round trips undisturbed.
  Catching that needs the binary the header names, which is out of repo. Written
  into the module doc so no future reader over-trusts it.
* **The vector rows.** They have their own gates
  (`sigil-isa/tests/{z80,m68k}_golden.rs`, `sigil-frontend-as/tests/asl_snippets.rs`).
* **A `header()` restructure** that changes the label set or drops the
  continuation convention. That needs the inverse to move with it, and
  `the_inverse_round_trips_both_banner_shapes` is what says so.

Three supporting tests carry the gate's own preconditions rather than leaving them
assumed: the inverse round trips all three banner shapes, a missing or truncated
header is refused instead of read as "nothing to compare", and the refusal message
is rendered by the same function the gate uses rather than by a lookalike.

## Strict run

```
SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-ref \
  cargo test --release --workspace --no-fail-fast
```

**FAILURES FIRST: zero.** No `FAILED` line, no `failures:` block, no `error` line,
no panic, over 408 test-result blocks.

| | |
| --- | --- |
| passed | **4631** |
| failed | **0** |
| ignored | 2 |
| `cargo test` exit | **0** |
| test-result blocks | 408 |

Master's bar is 4,627 passed / 0 failed; this branch adds exactly the four tests in
`golden_header_currency.rs`, so 4,631 is the bar plus this parcel and nothing else.

**The log names its own tree**, because a green from the wrong worktree reads
exactly like a green from the right one:

```
PWD    .../sigil/.claude/worktrees/agent-a2b29ac2caccac04e
HEAD   95a29bf36b3f7ea05e57c1285d958c2c8640c9e3
BRANCH parcel/golden-header-ungated
DIRTY  0 path(s)
AEON   /home/volence/sonic_hacks/.aeon-ref
TARGET .../agent-a2b29ac2caccac04e/.target-land
CARGO_TEST_EXIT=0
END_MARKER_STRICT_RUN
```

and it was grepped for this parcel's own test name, which is present and `ok`:
`test committed_golden_headers_are_current ... ok` at log line 6302. **The end
marker was written by the run itself**, so this is a completed run rather than a
killed one that happened to stop somewhere clean. `.aeon-ref` was read only; no
ROM in it was rebuilt.

## Reported, not fixed

**`cargo clippy --workspace --all-targets -D warnings` is RED on master today**,
and not because of this parcel. Six `doc_lazy_continuation` errors at
`crates/sigil-frontend-as/src/eval.rs:9470-9475`, byte-identical on master
`0587f2af` and untouched here (verified with `git show master:...`). With that one
lint allowed, `clippy -p sigil-harness --all-targets -D warnings` exits 0, so the
new file is clean. The fix is six lines of doc indentation; it is out of this
parcel's scope and is left to whoever owns that diagnostic.

**`gen_snippet_vectors.rs` documents a command that does not work.** Its module doc
says `cargo run -p sigil-frontend-as --bin gen-snippet-vectors`, and cargo rejects
that: the bin target is `gen_snippet_vectors` with underscores (the other two
generators DO have hyphenated `[[bin]]` names in `sigil-isa/Cargo.toml`; this crate
declares none, so the target takes the file name). The gate's own regeneration
advice uses the working spelling. Not corrected in the generator here, because
touching that file changes what three artifacts hash to, and this parcel's whole
claim is that only header prose moved.

## Anything in this brief you concluded was wrong

**One correction, one sharpening, and one place the brief was righter than it knew.**

1. **WRONG, and it is the brief's central factual claim about the risk: the
   regeneration could not have moved a vector row, and the reason is stronger than
   "it happened not to".** The brief frames the row comparison as guarding against
   a live possibility, and the deliverable demands a per-file proof, which I ran
   and which came back clean. But the generators are deterministic functions of
   (corpus, asl, p2bin). `crates/sigil-isa/tests/corpus/mod.rs` last changed at
   `11640d31`, which predates the last mint `edc51c98`, so the corpus has not moved
   since; and the asl and p2bin digests the regenerated headers record are
   byte-for-byte the ones the previous mint recorded. So the only inputs that could
   have moved a row did not move. The measurement was still worth running, because "the same binary" is a
   claim about the digest and not about the file system state, and because a
   generator can be non-deterministic in ways its inputs do not explain. But the
   brief's framing that a moved row was "a real BLOCKED candidate, not a
   hypothetical" was not true of THIS run, and I want that on the record rather
   than banked as a near miss. The candidate becomes real the day someone
   regenerates under a different `ASL_BIN`, and the header is what would say so.

2. **SHARPENED: bar 1 asks me to check exit status at the call site, but the call
   sites are inside the generators and already do it.** The brief says "the exit
   check is yours at the call site", which reads as though I had to add one. All
   three generators already assert `status.success()` on every `asl` and `p2bin`
   invocation and panic otherwise. What I could actually check was the
   GENERATOR's exit status, and the value of that check comes entirely from the
   assertions already being there: without them a zero from the generator would
   mean nothing. So the bar was satisfied by reading the code and confirming the
   property, not by adding a check, and I have said which line numbers carry it
   rather than asserting the property.

3. **The brief's "no maintenance burden" bar was harder to satisfy than it sounds,
   and the first draft of my own red-first test failed it.** That test perturbed
   the rendered header by replacing the literal string `"PROVENANCE,"`, which is a
   copy of `PREAMBLE`'s first words living in a test, exactly the shape bar 4
   forbids. It would have gone red on a legitimate preamble edit and trained
   whoever hit it to weaken it. It is now positional: it appends a character to the
   first rendered line, whatever that line says. Worth recording because the bar
   caught its own author.

Nothing else. The two rulings (regenerate never hand-edit; "next time someone runs
the generators" is not a resting place) were both correct and both load-bearing,
and the diagnosis of the cause was exactly right down to the commit.
