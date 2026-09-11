# Audit briefing cleanups: items 2, 4, 5, 6 and 7

2026-09-11. Branch `parcel/audit-briefing-cleanups`, base `5e3d389a`
(`5e3d389a024dc388b6af9327af4a499b8431623c`). Discharges five of the eight
cleanups in `docs/2026-09-09-audit-briefing.md`, section 2. Items 1 (stray
build directories), 3 (the Z80 fixup-kind change) and 8 (AEON-REFREEZE-DEBT)
are not this parcel's: 1 is done, 3 is settled by the controller's green strict
landing runs, and 8 is an owner question.

The briefing was two days old, so every item was re-found at the base by its
content before anything was changed. All five were still present. One had a
fact the briefing and two tracked records got wrong (item 5: the witness was
never lost), and the briefing's details were wrong twice (item 6's count, item
7's line).

| item | at base | commit | what changed |
|---|---|---|---|
| 7 | present, `main.rs:52` | `f4db79ab` | `Entry.flags_fn` off the shipping struct; mapping moved into the tests |
| 2 | present, `main.rs:2691` | `a9f0589c` | each row lists its options, `main` refuses the rest, the gate compares two table-derived sets |
| 4 | present, `eval.rs:117` | `81b086c6` | the keyword memo's key covers the `charset` page, by a stamp the page renews |
| 5 | present (untracked witness), and not lost | `0be48af4` | the original witness committed, with a test that reads it |
| 6 | present, no README | `889b58c8` | `.s1probe/README.md` |

Item 7 is committed before item 2 so each commit stands alone; item 2 then
removes the test-side table item 7 introduced, because the structural gate
needs no parser names.

Method, for every gate or test added: the mutation applied to a file equal to
the named commit (checked with `git diff --quiet HEAD` first), the mutated line
quoted back from disk, the focused tests run, and the file restored by copying
the committed version back from a snapshot, proven with `git diff --quiet HEAD`.
Never `git checkout --`. All cargo runs used
`CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/audit-cleanups/target`; no
aeon tree was involved.

## Item 7: `Entry.flags_fn`

**At base: still present.** `crates/sigil-cli/src/main.rs:52` on `5e3d389a`:

```rust
    #[allow(dead_code)]
    flags_fn: &'static str,
```

with six initializers (lines 65, 84, 96, 104, 127, 135). Only
`help_gates::usage_names_every_accepted_flag` read it.

**Done (`f4db79ab`).** The field and its initializers are gone. The mapping
moved into `help_gates` as `FLAG_PARSERS`, keyed by label, and
`every_entry_names_one_flag_parser` held it to `ENTRIES` in both directions,
so the flag gate still read the same six parser functions.

**Red first**, each against `f4db79ab`, mutation quoted from disk, restored by
copying the committed file back and proven with `git diff --quiet HEAD`:

| mutation | on disk | result |
|---|---|---|
| M7a: drop the `parse` row | `FLAG_PARSERS` lists five rows, no `("parse", "run_parse")` | 2 failed: `every_entry_names_one_flag_parser` ("`parse` has 0 rows in FLAG_PARSERS, not one"), `usage_names_every_accepted_flag` ("`parse` has no row in FLAG_PARSERS"); 7 passed |
| M7b: add `("nosuch", "run_asm")` | seventh row present | 1 failed: `every_entry_names_one_flag_parser` ("FLAG_PARSERS maps `nosuch` to `run_asm`, and no entry point is labelled `nosuch`"); 8 passed |

Item 2's commit removes `FLAG_PARSERS` and its test with the text scanner they
served: under the structural gate nothing needs to know a row's parser.

## Item 2: the help gate was a text heuristic

**At base: still present.** `crates/sigil-cli/src/main.rs:2691-2710` on
`5e3d389a`, `help_gates::flag_arms`:

```rust
            let trimmed = line.trim_start();
            if !trimmed.starts_with('"') || !trimmed.contains("=>") {
                continue;
            }
```

and `fn_body` at line 2684: `let end = rest.find("\n}\n").map(|i| i + 3).unwrap_or(rest.len());`.

**The hole, shown first** at `f4db79ab` (old gate still in place). A two-line
arm planted in `run_emp`, quoted from disk:

```
1134-            "--deny-todo" => deny_todo = true,
1135:            "--planted-a"
1136-            | "--planted-b" => hex = true,
```

`help_gates`: 9 passed, 0 failed, so the old gate was silently green. The binary
built from that tree accepted the flag nothing documents:
`sigil emp prog.emp --planted-a` exit 0, `DE AD BE EF`. The base binary
(no plant) refuses the same invocation, exit 2, so the acceptance was the plant's.

**Done (`a9f0589c`): structural.** Each `ENTRIES` row lists its `options`
(`Opt { name, takes_value }`), and `main` calls `unlisted_option` before a row
runs, so an option the row does not list is refused whatever an argument loop
matches. What the command line accepts is the table. The gate
`usage_names_the_listed_options` compares the row's options with the options
its usage text names, both directions, and parses no source text. Three more
tests: `usage_options_reads_the_shapes_usage_lines_use` (the extractor's
control), `main_refuses_every_option_a_row_does_not_list` (every case derived
from the table, cross-row included), and
`the_build_parser_reads_every_listed_option_as_listed` (sigil build's parser
returns rather than exits, so its arms and arity are held to the row directly).

**Why the brief's literal proof shape does not apply after the fix.** The brief
asked for the planted arm to go red against the fixed gate. Under the fix the
planted arm is dead code rather than a defect: `main` refuses `--planted-a`
before `run_emp` sees it. So the proof is in two parts, M2a (the plant alone:
gate green, binary refuses) and M2b (the only way to make the planted flag
accepted, listing it, without a usage line: red).

**Red first**, each against `a9f0589c`, mutation quoted from disk, restored by
copying the committed file back and proven with `git diff --quiet HEAD`:

| mutation | on disk | result |
|---|---|---|
| M2a: the same two-line arm, alone | `1222: "--planted-a"` / `1223: \| "--planted-b" => hex = true,` | `help_gates` 11 passed, 0 failed (the arm is dead); the binary built from that tree: `sigil emp prog.emp --planted-a` exit 2, `error: unexpected argument '--planted-a'` |
| M2b: the plant plus `flag("--planted-a")` in the emp row | `107: flag("--planted-a"),` | 1 failed: `usage_names_the_listed_options`, "`emp` accepts ["--planted-a"] and its usage never names them"; 10 passed |
| M2c: `valued("--map")` dropped from the emp row | emp options list six entries, no `--map` | 1 failed: `usage_names_the_listed_options`, "`emp` usage names ["--map"], which the command refuses"; 10 passed |
| M2d: `unlisted_option` passes everything | `310: None => {}` in place of `None => return Some(arg),` | 1 failed: `main_refuses_every_option_a_row_does_not_list`, "`<input.asm>` let `--not-an-option` through after `--hex`"; 10 passed |
| M2e: `--report` listed as standing alone | `165: flag("--report"),` | 1 failed: `the_build_parser_reads_every_listed_option_as_listed`, "`--report` is listed as standing alone and the parser answered \"--report requires a value argument\""; 10 passed |
| M2f: the extractor stops splitting on `\|` | `2850: ... "[]()<>,;`".contains(c)` | 2 failed: `usage_options_reads_the_shapes_usage_lines_use`, `usage_names_the_listed_options`; 9 passed |

**The limit that remains**, stated in `unlisted_option`'s doc: the scan skips a
listed option's value by the row's `takes_value`, not by what the loop does. A
loop that reads a value for an option listed as standing alone, or the reverse,
reads a different argument as an option than the scan did. The build loop is
held to the row by a test; the emp, test and bare-file loops exit the process
and are not. `dispatch_reads_only_the_entry_table` still slices `main` with
`fn_body`, unchanged, and that is outside this item.

**Behaviour changed, bytes did not.** An unlisted option now fails the same way
on every row: `error: unexpected argument '<arg>'`, that row's usage, exit 2.
Against the base binary: `sigil parse prog.emp --anything` exit 0 (flag
ignored) becomes exit 2; `sigil --version --anything` exit 0 (ignored) becomes
exit 2; `sigil emp --bogus` exit 1 ("cannot read --bogus") becomes exit 2;
`sigil emp prog.emp --hex` is exit 0 with the same output on both;
`sigil build --aeon x --bogus` prints the same first line on both. No test in
`sigil-cli` passes an unlisted option to the binary.

## Item 4: the keyword memo's key omitted `charset`

**At base: still present.** `crates/sigil-frontend-as/src/eval.rs:108-121` on
`5e3d389a` documents `HeadKey` as "every input of `Asm::dispatch_head` that is
not the line itself" and holds three:

```rust
struct HeadKey {
    cpu: Cpu,
    macros_gen: u64,
    frame: u64,
}
```

while `dispatch_head_checked` (line 4302) lexes under a fourth:

```rust
        let (toks, lex_err) = lex_line_recover(text, self.state.cpu, &self.state.charset, line.source, line.base);
```

**Which fix, and why.** The page is now in the key. The briefing's other
option, an enforced invariant that the memoised value does not depend on
`charset`, has nothing to test against: the memoised value is the keyword, an
identifier, and the page reaches only character constants, which lex to
integers. No input makes today's memo wrong, so a test of the invariant passes
whether or not it holds, and it could not fail on the day a token memo arrives,
because that memo would be new code the test never saw. A test can hold the
key's coverage instead, and with the page in the key a memo of anything else the
lex produces is correct without anyone remembering to widen it.

**Done (`81b086c6`).** `CodePage` carries a stamp from the same process-global
counter as the other `HeadKey` stamps (`next_stamp`, now `pub(crate)`). Every
constructor draws one, `set` renews it, `reset` rebuilds the page and so draws
one, and a clone shares its original's until either changes. The map is
private, so there is no other way to mutate it, and two pages with one stamp map
every character alike. `HeadKey` gains `charset: u64`. `PartialEq` is written
out to compare the map alone, so equality means what it did. Cost: one more
`u64` compare per keyword ask, and one memo miss per line after each `charset`
directive; a miss recomputes the same keyword.

**Red first**, each against `81b086c6`, quoted from disk, restored by copy and
proven with `git diff --quiet HEAD`:

| mutation | on disk | result |
|---|---|---|
| M4a: `head_key` leaves the page out | `4428: charset: 0,` | 1 failed: `eval::tests::the_keyword_memo_key_covers_the_code_page`, "a charset change left the keyword memo's key unchanged"; the stamp test passed |
| M4b: `set` stops renewing the stamp | `set` body is `self.map[src as usize] = tgt;` alone (lines 180-182) | 2 failed: `the_stamp_moves_whenever_the_mapping_can` ("set left the stamp where it was") and `the_keyword_memo_key_covers_the_code_page` ("a charset change left the keyword memo's key unchanged") |
| M4c: equality includes the stamp | `120: self.map == other.map && self.stamp == other.stamp` | 1 failed: `the_stamp_moves_whenever_the_mapping_can`, "two identity pages compare unequal, so the stamp is in equality"; the key test passed |

**No output moved.** `sigil-frontend-as`, whole crate: 763 passed, 0 failed,
0 ignored, 64 test binaries, no partial-run banner; clippy exit 0. Corpus runs,
base `5e3d389a` (md5 `2a58bd6d...`) against this tree's sigil (md5
`d5643a0d...`), `sigil <root> -o <scratch>` from each corpus root:
`s1disasm/sonic.asm` (the corpus that uses `charset`), `s2disasm/s2.asm`,
`skdisasm/sonic3k.asm`. All three exit 1 on both binaries, and stdout and
stderr are byte-identical (`cmp`) on each. None of the three reaches an image,
so this compares diagnostics, not image bytes; the crate's suite is where
images are pinned. No file in any corpus was newer than a marker touched
before the runs.

## Item 5: `451cb3e2` cites an untracked witness

**At base: still present, and the witness was never gone.** `451cb3e2`'s
message says the flip is reachable and "this commit carries a witness for it
(`.scratch/repro/control.asm` shape: ...)". No such file is tracked. Two
tracked records already said it was lost: the ledger row
`sig-ux-partial-error-list` (`docs/lens-findings.jsonl`) and
`docs/superpowers/notes/2026-09-10-ux-partial-error-list-repro/README.md`
("untracked and does not exist ... the worktree is gone"). The row closed by
reconstructing a different reproduction (the stage boundaries of UXa F5), which
answers a different question from `451cb3e2`'s flip.

**Found.** `find /home/volence/sonic_hacks/.scratch /home/volence/sonic_hacks/sigil
-name control.asm` returned it at
`sigil/.claude/worktrees/agent-af9ee78f0abdc942f/.scratch/repro/control.asm`,
a worktree whose `HEAD` is `refs/heads/parcel/failing-build-reports-everything`.
Beside it: `gated.asm`, `levartptrs.asm`, and `../msg1.txt`, which differs from
`git show -s --format=%B 451cb3e2` only in git's trailing blank line. Its mtime
is 14:06 on 2026-09-09; the commit is 14:11 the same day. So this is the
authoring witness, not a reconstruction. `/home/volence/sonic_hacks/.scratch/`
holds no copy; the sigil main checkout has no `.scratch/`.

**Done (`0be48af4`).** The three files are committed byte for byte at
`docs/superpowers/notes/2026-09-11-451cb3e2-witness/` with a README (where
found, md5s, what each witnesses, the measurement below).
`failed_run_reports_everything.rs` gains
`the_witness_451cb3e2_cites_is_refused_by_the_front_end`, which reads the
tracked `control.asm` and holds the front end to exactly its two diagnostics,
so the pointer cannot rot silently again.

**Re-run, 2026-09-11.** Two binaries built into this parcel's scratch target:
`27db72a7` (the parent of `451cb3e2`, from `git archive`, md5
`cfaac2bd65eaf1b330a0d9949cfd20b9`) and `5e3d389a` (md5
`2a58bd6dc1311de598bae1cfefe7996c`).

| file | `27db72a7` | `5e3d389a` |
|---|---|---|
| `control.asm` | exit 1, 1 error, from layout: `(5) unresolved jmp/jsr target ... NoSuchTarget not defined in this link` | exit 1, 2 errors, from the front end: `(4) unresolved long expression`, `(5) unresolved symbol NoSuchTarget in operand` |
| `gated.asm` | exit 1, 1 error: `(8)` moveq | exit 1, 3 errors |
| `levartptrs.asm` | exit 1, 1 error: `(23)` moveq | exit 1, 4 errors |

The `control.asm` row is the claim: the front end used to accept the module
and now refuses it, naming both lines. The commit says the refusal moved "from
the linker"; in the `27db72a7` binary it is layout (`resolve_layout`).

`levartptrs.asm` is NOT byte-identical to the test's `LEVARTPTRS_FAILING`
constant, which a comparison shows: the file indents `endm` by four spaces and the
constant, through Rust's `\` line continuation, carries it at column 0. Same
four diagnostics either way.

**Red first**, against `0be48af4`, quoted from disk, restored and proven
by comparing the tree with `HEAD`:

| mutation | on disk | result |
|---|---|---|
| M5a: the gate reverted to its form before `451cb3e2` | `434: if !force_relocate && poison.is_empty() {` | 4 failed: the new test ("expected a failing run, got Ok with 1 sections"), and three existing pins of the same gate (`a_skipped_bonus_pass_still_reports_its_leftover_poison`, `levartptrs_shape_reports_both_unresolved_long_expressions_on_a_failing_run`, `the_new_diagnostic_set_contains_the_old_one`); 5 passed |
| M5b: the tracked `control.asm` deleted | the directory lists `gated.asm`, `levartptrs.asm`, `README.md` | 1 failed: the new test, "cannot read the committed witness .../control.asm: No such file or directory"; 8 passed |

## Item 6: `.s1probe/` has no README

**At base: still present.** No `README.md` directly under `.s1probe/`; the only
one in the tree is `.s1probe/2026-09-04/probe/f4/README.md`.

**Population, enumerated** (`git ls-files .s1probe`): **75 files**, which
matches the briefing. The briefing's "26 added by `9aed994f`" does not: that
commit changes 26 files, and 25 of them are under `.s1probe/`
(`2026-09-09-switch-int/`: 22 probes, `asl-listings.txt`, `capture.sh`,
`run.sh`); the 26th is `crates/sigil-frontend-as/src/eval.rs`. Ten commits add
to the directory: `dd9d201e`, `7491c371`, `b1c5e40e`, `f02c1379`, `d2c32940`,
`ce9d998c`, `163be7f6`, `5a57e2bf`, `9aed994f`, `7ccd4dba`.

**Done (`889b58c8`).** `.s1probe/README.md`: what the directory is (evidence;
nothing in the build or suite reads it), the asl selection rule with a pointer
to the `OVERSEER-REFERENCE.md` block and the asl-reference README, every file
by group with the commit that added it and the instrument that produced it (by
md5 where one was recorded, and saying so where none was), and how to re-run
each runner.

**Measured for it, 2026-09-11:**
- `s1disasm/build_tools/Linux-x86_64/asl` is still md5
  `61e672562465725a8c102288a7da9098`.
- `capture.sh`, run on a copy of `2026-09-09-switch-int/`, reproduces the
  committed `asl-listings.txt` on all 410 lines outside the dated page headers
  (`diff` exit 0, 22 probes, the same exit codes).
- `real_dac_v2.lst`'s producer was cited by banner only (`7ccd4dba`). Its own
  `*ARCHITECTURE` row identifies it: re-assembled with each build on the
  machine, the reference (`61e67256...`) differs from it only in the page header
  and `*DATE`/`*TIME`; the upstream i386 build (`a8cd8b80...`) prints
  `i386-unknown-linux`, and the flamewing build (`0dee1f98...`) `x86_64-Linux`.
- `2026-09-04/probe/cmp.sh` runs its asl half on a copy of `p1.asm` (exit 2,
  three `error #1133`); its sigil half, `run.sh`, `construct_probe.sh` and the
  four `2026-09-04/*.py` scripts hardcode paths that no longer exist, or (for
  `run.sh`) another worktree, and the README says so.

## Hunks in files other lanes are working in

For merge prediction. Line numbers are base (`5e3d389a`) lines, from
`git diff -U0 5e3d389a 889b58c8`.

`crates/sigil-cli/src/main.rs` (items 7 and 2). **No hunk touches `run_asm`'s
body** (base lines 247 to 379), nor any other argument loop:

| base lines | what |
|---|---|
| 34 | `Entry` doc: what the tests hold |
| 46-52 | `flags_fn` field and its doc removed |
| 57 | `options` field; `Opt`, `flag`, `valued` added after the struct |
| 65, 84, 96, 104, 127, 135 | each `ENTRIES` row: `flags_fn` line replaced by its `options` list |
| 240, 243 | `main`: the unlisted-option refusal before `(entry.run)`, and `fn unlisted_option` after `main` |
| 2652, 2663 | `help_gates` module doc and the `SOURCE` doc |
| 2688-2711 | `flag_arms` removed |
| 2792-2836 | `usage_names_every_accepted_flag` replaced by `usage_options`, its control test, `usage_names_the_listed_options`, `main_refuses_every_option_a_row_does_not_list`, `the_build_parser_reads_every_listed_option_as_listed` |

`crates/sigil-frontend-as` (items 4 and 5):

| file, base lines | what |
|---|---|
| `src/eval.rs` 109-115, 118, 127 | `HeadKey` doc, its `charset` field, `next_stamp` made `pub(crate)` |
| `src/eval.rs` 4396 | `line_keyword` doc |
| `src/eval.rs` 4419 | `head_key` builds the key with the page's stamp |
| `src/eval.rs` 11851 | one test, at the top of `mod tests` |
| `src/charset.rs` 109-113, 141, 161, 168 | the stamp, the written-out `PartialEq`/`Eq`, `identity`, `reset` doc, `set`, `stamp()`, and a new `mod tests` at the end |
| `tests/failed_run_reports_everything.rs` 268 | one test appended |

`crates/sigil-link`: no change. The `.emp` processor-name recognition: no change.

## What the brief got wrong, or could not have known

- **Item 5's premise.** The brief allowed that the witness might be gone and
  asked for a reconstruction if so. It was on disk, in the worktree the
  commit was authored in, beside that commit's message draft. Two tracked
  records say otherwise: `docs/lens-findings.jsonl` row
  `sig-ux-partial-error-list`, and
  `docs/superpowers/notes/2026-09-10-ux-partial-error-list-repro/README.md`
  ("untracked and does not exist ... the worktree is gone"). This parcel does
  not edit either; their reconstruction answers a different question and
  stands. Only the statement that the witness is gone is wrong.
- **Item 6's count.** "26 added by `9aed994f`": that commit changes 26 files
  and adds 25 under `.s1probe/`. The briefing's total of 75 is right.
- **Item 7's line.** The briefing cites `main.rs:127` for `Entry.flags_fn`;
  that is the build row's initializer. The field was at line 52. (The brief
  warned that its line numbers would be stale; this one was never the field.)
- **Item 2's proof shape.** "Red against your fixed gate" with the planted arm
  assumes the fix is a better scanner. A structural fix makes the planted arm
  dead code, so the red comes from the only change that would make the planted
  flag accepted (listing it), and the plant alone is shown refused by the
  binary instead.
- **Item 4's second option.** "A test or an assertion that fails the day a
  memoised value starts depending on `charset`" cannot be written: such a test
  can see only today's memo, whose value does not depend on the page, and the
  memo that would break it is code that does not exist yet. That is why the
  page went into the key.
- **The unmeasured count.** The brief asks for it from the partial-run banner.
  `sigil-frontend-as` prints no banner, so it states no unmeasured count; the
  `sigil-cli` figure is its banner's, below.

## Tip suites

At `889b58c8` (item 6, the last code-bearing tip; this note's own commit adds
only documents), with `SIGIL_ALLOW_PARTIAL=1`, `--release --no-fail-fast`, and
the parcel's own `CARGO_TARGET_DIR`. No `AEON_DIR` was set.

| crate | passed | failed | ignored | test binaries | unmeasured |
|---|---|---|---|---|---|
| `sigil-cli` | 774 | 0 | 1 | 165 | 130, per its partial-run banner |
| `sigil-frontend-as` | 764 | 0 | 0 | 64 | none: it prints no partial-run banner |

The one ignored test is `sigil_diff_reports_byte_identity` ("reads the aeon
source tree; run with --ignored"). The `sigil-cli` banner reads: "No reference
tree is named, so 130 test binaries are reference-dependent and every row in
them is left UNMEASURED." Its skip lines point at
`/nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named/...`, so nothing
ran against an aeon tree. `version_reports_the_head_of_the_tree_it_was_built_from`
and `the_published_line_states_this_revision_s_position_against_a_named_remote_ref`
both passed.

`cargo clippy --release -p <crate> --all-targets -- -D warnings`: exit 0 for
`sigil-cli`, exit 0 for `sigil-frontend-as`.

The strict landing gate, with the four engine ROM shapes, is the controller's
at merge and was not run here.
