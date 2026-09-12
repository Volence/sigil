# AS-BACKWARD-SEEK-SPLIT fixed: the section after a seek its section closes behind is pinned at its labels, and its overlap is refused at the seek

Queue row AS-BACKWARD-SEEK-SPLIT. Fix parcel, branch
`parcel/as-backward-seek-split`, base master `8f5e03c3` (which holds the
measurement, `2026-09-12-as-backward-seek-split.md`). Fix commit `c5a2564d`.

**Verdict: no probed shape produces a silently wrong image any more.** Every shape
the measurement found splitting labels from bytes is now refused, by name, at
the `org` that left the cursor behind. Every other probed shape, and every corpus
run, is byte-for-byte what it was on `8f5e03c3`: images, stdout and stderr.

## Provenance

| | |
|---|---|
| base | `8f5e03c3`, `cargo build --release --bin sigil` in this worktree's own `target/`; md5 `358abd9335e0952e40647e2ebdab5aed`, `--version` `sigil 0.1.0 (8f5e03c3)` |
| fix | `c5a2564d`, built by `cargo test` at a clean tracked tree; md5 `4eaf1b454ab8b1e924ca3e7bec2e7b65`, `--version` `sigil 0.1.0 (c5a2564d)`, tree `clean-sources` |
| asl | `61e672562465725a8c102288a7da9098`, only through `asl-reference/asl_ref.sh`'s `asl_run`, flags `-xx -n -q -A -L -U -i .`; every probe row `ASL_EXIT=0`, `asl_diag=complete`. `p2bin` `4f2fff99c3347bafb93b12d5be1db754` |
| corpora | s1disasm `f6ece65`, s2disasm `e45ebf3`, skdisasm `2fcd861`, read only through `git archive HEAD` (`census-prepare.sh`) |
| aeon | not read, not built (no tree leased). The measurement found no `org` in its three AS inputs |
| emulator | none; nothing here needs runtime confirmation |

## What changed

Two pieces, one in each crate the measurement named.

**1. Front end, `close_section` (`eval.rs`).** Where it records the seek as the
re-base, it now also calls `self.builder.pin_next_section()`, as the two `org`
arms of `directive_org` already do. The next section opens at `base + cursor`
(unchanged, where its labels bind) and is `Pinned` there, so the placer leaves
its bytes there instead of packing it `Chained` at `base + extent`.

**2. Linker, `overlap_diag` (`relax.rs`).** With piece 1 alone, every split shape
became a refusal, but a badly located one. Measured on c01 with piece 1 only:

```
c01_cpu_same.asm(4):9: error: sections `sec0` [0x0, 0x10) and `sec12` [0xC, 0x12) overlap in the image (colliding pins)
```

Line 4 is `dc.l L_next`, the head table: the first fragment of the section the
seek sits in. In an AS program a section runs from the last `org`, `cpu` or
`phase` line, so in a real source that line can be the vector table, any
distance above the seek. The overlap check now locates one pair differently: a
section and the NEXT section holding bytes, when that next one begins exactly at
the offset where the LAST `org` inside the first left its write cursor when it
ended. Since the two overlap, that cursor is inside bytes the first already
holds. The diagnostic is placed at that `org` and says what happened. The replay
that finds the end cursor and the last `org` is `image_final_size`'s own loop
(`image_replay`), extended to report both, not a new one. Every other overlap
keeps the plain message and location; probes c19 and c20 guard the two
conditions.

What c01 prints now:

```
c01_cpu_same.asm(7):9: error: sections `sec0` [0x0, 0x10) and `sec12` [0xC, 0x12) overlap in the image: this `org` left the write position at 0xC, inside bytes already written up to 0x10, when its section ended, and `sec12` was placed there (asl writes the later bytes over the earlier ones; sigil refuses the overlap)
```

Line 7 is `org Start+2`. The numbers are asl's: its listing puts `L_next` and
its `1234` at `$C`, and the four marker words run to `$10`.

The policy is the one `2026-09-06-as-org-backwards-fix.md` set for `p5_overlap`:
asl accepts, `p2bin` resolves last write wins, sigil refuses by name. The linker
does not learn to overwrite.

## Per shape, before and after

`run.sh` over all 20 probes, base binary then fix binary, `classify.py` over
each. The asl columns of the two `RAW.tsv` files are identical (`diff` exit 0).

| verdict | base `8f5e03c3` | fix `c5a2564d` |
|---|---|---|
| MATCH asl byte for byte | c05 c06 c07 c15 c17 | c05 c06 c07 c15 c17 |
| DISAGREE, exit 0 (silent wrong image) | c01 c02 c03 c04 c08 c09 c11 c13 c14 c18 | none |
| REFUSED, by name at the seek (new) | none | c01 c02 c03 c04 c08 c09 c11 c13 c14 c18 |
| REFUSED, as before | c10 c12 c16 c19 c20 | c10 c12 c16 c19 c20 |

The 15 original probes alone: MATCH {c05 c06 c07 c15} before and after;
DISAGREE {c01 c02 c03 c04 c08 c09 c11 c13 c14} before, none after; REFUSED {c10
c12} before, {c01 c02 c03 c04 c08 c09 c10 c11 c12 c13 c14} after.

Per-shape transcripts (`cmp` of each `sigil.out` and `sigil.err`): exactly the
ten DISAGREE shapes differ; the other ten are byte-identical in both streams. The
committed binary and the pre-commit build of the same source give the same 20
transcripts.

The ten refusals, each at its own seek line (probe listing lines, the probe's
first line being its comment):

| shape | line | the seek | "left the write position at X, inside bytes already written up to Y" |
|---|---|---|---|
| c01 `cpu 68000` | 7 | `org Start+2` | 0xC, 0x10 |
| c02 `phase $8000` | 7 | `org Start+2` | 0xC, 0x10 |
| c03 `dephase` | 8 | `org $8002` | 0xC, 0x10 |
| c04 cpu-changing `restore` | 10 | `org 8001h` | 0xA, 0xC |
| c08 two seeks | 9 | `org Start+4` (the second) | 0xC, 0x10 |
| c09 no bytes after the seek | 7 | `org Start+2` | 0xA, 0x10 |
| c11 `cpu z80` + `phase` | 7 | `org Start+2` | 0xC, 0x10 |
| c13 a Z80 program | 7 | `org Start+1` | 0x6, 0x8 |
| c14 `restore` out of the host | 9 | `org Start+2` | 0xC, 0x10 |
| c18 `ds.b` over the tail | 7 | `org Start+2` | 0xC, 0x10 |

### The five new probes

- **c16**, a trailing `ds.b` and then a seek back to where it began. asl:
  `AAAA BBBB 1234` at `$C`. sigil refuses it before and after with the existing
  layout refusal (`section `sec0` mixes an `org` back-patch with a `ds`/reserve,
  unsupported ...`, at `org Start+4`), so it never reaches this row.
- **c17**, the code after the close writes nothing into the tail: its label at
  the cursor, then an `org` past the old end before its first byte. MATCH, 24
  bytes, before and after.
- **c18**, the code after the close steps over the tail with `ds.b 4`. asl leaves
  `CCCC DDDD` under the reservation and puts `1234` at `$10`; its listing shows
  `C : L_next: ds.b 4` and `10 : 1234`. Base sigil was the same silent split
  (`1234` at `$14`, `L_next` still `$C`). Now it is refused. **This refusal is
  not an overwrite asl performs**: sigil's image model fills a reservation that
  has bytes after it inside its section (`Section::image_bytes`, and
  `a_reservation_inside_a_section_counts_toward_its_overlap_extent` pins the
  overlap check to the same model), so in sigil's model the next section does
  write zeros over the tail, and the overlap is real there. Matching asl would
  change that reservation model for every section, not this row.
- **c19** and **c20**, an author's own `org` lands in the tail, off the cursor
  (c19) or back to exactly the cursor after other code (c20). Refused before and
  after with the plain overlap diagnostic at the first line of `sec0`; the
  located diagnostic must not claim them.

### "A no-overlap seek-then-close now lands its bytes at its labels"

No accepted shape can show that, because none exists in sigil. A seek's target
is at most the section's extent, so everything from the cursor to the extent is
either bytes the section wrote or a reservation. With bytes there, any section
that writes before the old end overlaps them and is refused. With a reservation
there, the section mixes an `org` with a reservation, which layout already
refuses (c16). What is left is a following section that writes nothing before
the old end, which already landed at its labels on base, because it held no bytes
to misplace (c17). So the pin moves no byte of any accepted program: it turns
every program where the bytes would have been misplaced into a refusal. The CLI
test for c17 guards the other direction, that a correct program is not refused.

## Exposure census, before and after

The measurement's own tooling, run twice: `census-prepare.sh` at `8f5e03c3` and
at `c5a2564d`, then `census.sh` over each. `instrument.py` now accepts the
`close_section` block with or without the new pin line and prints which anchor
matched (`SUBSTITUTION_1_ANCHOR=0` at the base, `=1` at the fix), so the same
three warnings go into both builds.

| run | SEEKFEED | SEEKSPLIT | BY-ORG | stderr lines, base / fix |
|---|---|---|---|---|
| probes (20 now; positive control) | 22 / 22 | **17 / 17** | 2 / 2 | 48 / 58 |
| s1disasm `sonic.asm` | 103 / 103 | **0 / 0** | 0 / 0 | 104 / 104 |
| s2disasm `s2.asm` | 620 / 620 | **0 / 0** | 0 / 0 | 622 / 622 |
| skdisasm `sonic3k.asm` | 0 / 0 | 0 / 0 | 0 / 0 | 137 / 137 |
| skdisasm, `Sonic3_Complete = 0` | 0 / 0 | 0 / 0 | 0 / 0 | 120 / 120 |
| skdisasm, `Sonic3_Complete = 1` | 0 / 0 | 0 / 0 | 0 / 0 | 110 / 110 |

- **Every corpus stream is byte-identical.** `diff -rq` over the two output
  directories names exactly two files: `CENSUS.txt` (its md5 line and the probes
  row) and `probes.err`. Every `s1`, `s2`, `sk`, `sk0`, `sk1` `.out` and `.err`
  is the same file. The corpus rows are also identical to the measurement's
  committed `census/CENSUS.txt`.
- **`probes.err` gains ten lines and loses none**: the ten refusals above, each at
  its seek line.
- **Uninstrumented as well.** The base and fix release binaries over the same
  prepared corpora, run the way `census.sh` runs them: 15 stdout / stderr / exit
  pairs, 0 differing (every run exits 1, on the corpora's own open rows).
- The probes' SEEKSPLIT of 17 (12 on the original 15, plus one per new probe) is
  the instrument's positive control. The corpora's zeros are the measurement's,
  reproduced: no s1 or s2 section closes behind its cursor, so the fix reaches
  nothing there, and no corpus byte or diagnostic moved.

**A defect in the census tooling, found and fixed here.** The first run of the
fix revision reused the shared `target/instr` the base build had filled. Its
build log recompiled sigil-frontend-as, sigil-harness and sigil-cli, and not
sigil-link, and the resulting binary printed the plain overlap at line 4 for c01
although its archived `relax.rs` held the located diagnostic: a fix front end
with the base linker. The likely mechanism is cargo naming a workspace crate's
build unit by its path relative to the workspace, so two archive copies share
units, with the base copy's still-present, unchanged files keeping the unit
fresh; the measured part is the missing recompile and the printed message.
`census-prepare.sh` now builds each commit into `target/instr-<sha>`. The table
above comes from that; both logs show sigil-link compiled from their own copy,
and the fix binary prints the located refusal at line 7 for c01. The first run's
corpus streams were identical too, but it proved nothing about the linker piece,
so it is not the evidence here.

## Tests

`pwd` `/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a34c11299808c7cd0`,
HEAD `c5a2564d`, branch `parcel/as-backward-seek-split`, tracked tree clean
(`suites.sh` stamps all four into its log):

| command | targets | passed | failed | ignored |
|---|---|---|---|---|
| `cargo test --release -p sigil-frontend-as -p sigil-link --no-fail-fast` | 80 | 1002 | 0 | 0 |
| `cargo test --release -p sigil-cli --test as_second_address_space --test as_backward_seek_close --no-fail-fast` | 2 | 11 | 0 | 0 |

`cargo clippy --release -p sigil-frontend-as -p sigil-link --all-targets -- -D warnings`
and the same over the two sigil-cli test targets: clean, at HEAD `c5a2564d`.

New tests:

- `sigil-frontend-as` `as_address_space`:
  `a_seek_back_that_an_image_section_closes_behind_pins_the_next_section_at_its_labels`.
  c01's source; `L_next` bound at `$C`, its section's `lma` `$C` and `Pinned`;
  `L_after` at `$12`, `Chained` (asl listing values quoted in the test).
- `sigil-cli` `as_backward_seek_close` (new file, runs the shipped command):
  - `a_seek_its_section_closes_behind_is_refused_at_the_org_when_the_next_bytes_land_on_the_tail`:
    c01, exactly one error, no image written, at `root.asm(6):` (the `org Start+2`
    line; the test source has no comment line), with the ranges, the "left the
    write position" clause and the asl clause.
  - `every_closer_is_refused_at_the_seek_when_the_next_bytes_land_on_the_tail`:
    c02 c03 c04 c08 c09 c11 c13 c14 c18, each at its own seek line with its own
    two addresses, read off asl's listings.
  - `a_seek_its_section_closes_behind_assembles_to_asl_s_image_when_nothing_lands_on_the_tail`:
    c17, asl's 24 bytes.
  - `an_overlap_the_seek_did_not_place_keeps_the_plain_overlap_diagnostic`: c19 and
    c20, the plain message, at the first line of `sec0`.

## Red-first evidence and the half-fix matrix

`mutate.sh` (scratch, not committed) refuses to start on a dirty tracked tree,
applies ONE literal substitution by an anchor that must match exactly once,
quotes the mutated line back from disk by its `MUTATION` marker, runs four
targets (`sigil-frontend-as --test as_address_space`, `sigil-link --lib`,
`sigil-cli --test as_backward_seek_close`, `sigil-cli --test as_second_address_space`),
restores the file with `git show HEAD:<path> > <path>`, and prints the tracked
status, which was empty after every row. Baseline: the committed `c5a2564d`.

| id | mutation (line quoted from disk) | red | green |
|---|---|---|---|
| M1 | piece 1 removed. `eval.rs:6559 self.rebased_at = Some(seek); // MUTATION M1 pin removed` | as_address_space: the new pin test, at line 149, `and the linker may not move them` (placement; its `lma` line still passes, since the front end's `lma` was always `$C` and only the linker moved it). as_backward_seek_close: the c01 test and `every_closer`, both at line 67, `must be refused` (exit 0: the silent wrong image) | c17, c19/c20, sigil-link 142, as_second_address_space 7 |
| M2 | piece 2 disabled. `relax.rs:468 if false && j == i + 1 { // MUTATION M2` | the c01 test at line 114 and `every_closer` at line 236: refused, but not at the seek | the pin test, c17, c19/c20, sigil-link, as_second_address_space |
| M2a | piece 2's adjacency condition dropped. `relax.rs:468 if true \|\| j == i + 1 { // MUTATION M2a` | the guard test, case c20, line 312: the diagnostic moved to the seek and claimed code the author's own `org` placed | everything else |
| M2b | piece 2's exact-start condition dropped. `relax.rs:470 if true \|\| b_lo == a_lo.saturating_add(cursor) { // MUTATION M2b` | the guard test, case c19, line 312: "left the write position at 0xE", which no seek did | everything else |
| M3 | the rejected design, refusing every seek its section closes behind. `eval.rs:6561 self.err(seek, "MUTATION M3 ...");` | all four as_backward_seek_close tests, including c17 at line 51 (`must assemble`); both pin tests in as_address_space; `the_driver_is_refused_at_its_own_org_line_and_never_as_an_overlap` | sigil-link |
| M4 | NOT this parcel's code: the second-space pin in `assign_address_spaces` removed. `eval.rs:1166 let _ = &sec.placement; // MUTATION M4 second-space pin removed` | **nothing** | all four targets |

Every new test is red under at least one mutation of the committed fix: the pin
test under M1; the c01 and `every_closer` tests under M1 and M2; the guard test
under M2a (c20) and M2b (c19); the c17 test under M3. Every piece of the fix
reddens something when removed. M4 applied and green is not a runner that missed
the patch: the same runner reddened on M1 and M3, also edits to `eval.rs`.

**M4 is a finding.** With `close_section` pinning, nothing in these targets needs
the pin `assign_address_spaces` stamps on a second-space section. On base
`close_section` did not pin, so that stamp was the only source of the `Pinned`
which `a_seek_back_that_its_section_closes_behind_enters_the_space_too` asserts
(read from the code; M4 was not run against the base). That section is the next
one the builder opens after the seek, so `close_section`'s pin now reaches it
first. Where an empty section
opens first, the section with content is `Chained` right behind the pinned empty
one, at the same `lma`. Left in place; it is either dead or untested, and which
is for a parcel that can run the workspace suite.

## Whether the design held

Pinning held: it is the fix, it moves no accepted image, and every split shape
becomes a refusal where asl overwrites. It needed one addition. The existing
overlap refusal names the collision, but locates it at the first line of the
section the seek sits in, which is not a line an author would connect to the
cause, so the linker now locates this one pair at the seek (piece 2). No IR type
changed; the linker reads the seek from the `Org` fragment the section already
carries. The rejected alternative, letting the linker overwrite, was not built.

## Left open

- **c18** is refused where asl does not overwrite. sigil's model fills a
  reservation that has bytes after it in its section, so the next section writes
  zeros over the tail in sigil's model and the overlap is real there. Loud, not
  silent. Matching asl would change the reservation model for every section.
- **c16** is refused by the existing `org`-plus-reservation layout rule, where asl
  accepts. Pre-existing and separate.
- **c10** (stage 1's second-space refusal) and **c12** (AS `section` / `public` /
  `endsection` unimplemented) are unchanged, as the measurement found them.
- **M4**, above: the second-space pin in `assign_address_spaces` has no test that
  needs it any more.
- The diagnostic names the sections by sigil's synthetic names (`sec0`, `sec12`)
  and does not name the line that ended the section (`cpu`, `phase`, ...),
  which the linker does not know. The location (the seek) and the two addresses
  carry it.
- Not run here: the workspace suite and the landing gate (aeon four-shape byte
  identity), by instruction.

## Corrections to the brief

1. **"the existing overlap refusal names the collision"**: it does, at the first
   line of the section the seek is in. Measured on c01, line 4 (`dc.l L_next`),
   six lines above the seek; hence piece 2.
2. **"one [test] that asserts a no-overlap seek-then-close now lands its bytes at
   its labels"**: no accepted program's bytes move under this fix (see the section
   above on that sentence). A seek-then-close with nothing written into the tail
   already landed at its labels on base (c17). The c17 test asserts asl's image,
   and it guards against over-refusal (M3), not against the split.
3. **"c09 is close to it"**: c09's next section writes `1234` at `$A`, inside the
   tail (asl overwrites `BBBB` there), so it is refused, not matched. c17 is the
   shape that writes nothing into the tail.
4. **The census tooling as committed** would have handed back a hybrid binary for
   any second revision built into the same worktree; fixed (above).
