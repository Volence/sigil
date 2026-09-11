# S1 driver space, stage 1: a second address space is refused at its org, not reported as an overlap

2026-09-11, parcel `parcel/s1-driver-space`, base `66d2ed86`. Stage 1 of
`2026-09-10-s1-phased-overlap.md`. **Nothing here places a second address space
into the ROM, and no flag, file or map field for doing so is named.** Where the
placement, the transform and the budget get declared is stage 2, an open owner
decision.

## What Sonic 1 prints now

```
sound/z80.asm(9):3: error: section `sec0#2` [0x0, 0x1BC6) is assembled for the Z80 at origin 0x0, in a second address space this org opens outside the ROM image; the assembler cannot yet place a second address space into the ROM
```

Line 9 of `sound/z80.asm` is the driver's own `!org 0`. Before this parcel the
same corpus printed one line, at a `dc.l` in the vector table that has nothing to
do with the driver:

```
sonic.asm(81):3: error: sections `sec0` [0x0, 0x2CA) and `sec0#2` [0x0, 0x1BC6) overlap in the image (colliding pins)
```

## The host-space rule, and why

In one sentence: **the image is the address space of the CPU of the program's
first section with content, and a section leaves it only when an `org` has set
the counter to a number and the first section with content after that is
assembled for a different CPU with no `phase` open.**

As `assign_address_spaces` (`crates/sigil-frontend-as/src/eval.rs`) runs it:

- **The image's CPU** is the CPU of the first section that has any fragment. A
  label before the `cpu` line opens an empty section under the provisional
  processor, and must not make a 68000 program a Z80 image.
- **Only an `org` changes the space**, because only an `org` sets the physical
  counter to a number instead of letting it run on. Two front-end paths do that:
  an `org` that leaves the open section, and an in-section seek that is still
  pointing back when its section closes (see below). `cpu`, `phase` and `dephase`
  break sections but continue the counter, and a section opened on a continued
  counter is in the space the counter was already in.
- **After an `org`, the first section with content decides**, in this order:
  1. the image's CPU: in the image (how a driver is left: `restore`, then `org`
     back to the cartridge);
  2. the CPU of the second space the counter is already in: still in that space
     (an `org 38h` inside a Z80 driver lays the driver out; it does not leave it);
  3. another CPU with a `phase` open: in the image (the `org` placed the bytes and
     the `phase` gave them their run address, which is the load/run split the
     image already models);
  4. another CPU with no `phase`: a new second space of that CPU, entered at that
     `org`.
- **Sections without content never decide.** A label between the `org` and the
  `cpu` line opens one under the old CPU.

Each part is forced by corpus text, not assumed:

| part of the rule | the corpus text that forces it |
|---|---|
| decide at the first section AFTER the org, by that section's CPU | S1 `sound/z80.asm` 9-10 and S2 `s2.sounddriver.asm` 248-250: `!org 0` comes BEFORE `CPU Z80`, so the CPU at the org line is still the 68000 |
| the image's CPU returns to the image | all three leave the driver with `restore` then `!org`: S1 `z80.asm` 234-236, S2 `s2.asm` 90861-90862, S3K `Sound/Z80 Sound Driver.asm` 4433-4435 and 5313-5315. The CPU at every return org is the 68000 |
| same second CPU stays in its space | S2 `s2.sounddriver.asm` 392, `org 38h` inside the driver |
| each entry from the image is its own space | S3K has two blobs, `!org 0` (226) and `!org 1300h` (4444), with a return between; its `buildSK.lua` passes two `-z` flags |
| a continued counter keeps its space | S1 `sonic.asm` 322-353, `SetupValues_Z80`: `save` / `CPU Z80` / `phase 0` with no `org`, inline image bytes |
| a phase open means image | protective, no corpus instance: `org X` / `cpu z80` / `phase Y` is the load/run split and must not be refused |

Both directions of "host" are tested. A Z80 program at `org 0` is a Z80 image
(`a_z80_program_at_org_0_is_the_image`, `a_z80_program_at_org_0_still_assembles`),
and a 68000 program with a Z80 driver at `org 0` is a 68000 image with a second
Z80 space (`a_driver_org_d_to_z80_zero_is_a_second_space_entered_at_its_org`).

## What changed

- **sigil-ir.** `Section` gains `space: AddressSpace`, `Image` or
  `Foreign { cpu, entered_at }`. Two `Foreign` values are one space exactly when
  both fields match. The builder closes every section as `Image`, and the AS front
  end re-tags. That is 120 constructor sites in 13 files. Every one was checked to
  be a struct expression and none a pattern, because a `space: ..Image` inside a
  pattern would silently narrow a match. The one production copy site,
  `resolve_layout`'s rebuild in `relax.rs`, carries `sec.space` through.
- **sigil-link.** `overlap_diag` scans per space. A collision inside a second
  space reads `overlap in a second Z80 address space`; the image wording is
  unchanged. A new check, `(c2b) foreign_space_diags`, runs after the overlap
  scan and refuses every second space that holds image bytes: one error per space,
  at the `org` that entered it. It runs only when `check_image` is set, so
  `resolve_layout_measuring` still measures. `flatten` is untouched.
- **sigil-frontend-as.** `Asm` records a `SectionOpen { rebased_at, phased }` for
  every opened section. `directive_org` records its re-base, `close_section`
  records a seek it closes behind, `phase`/`dephase` track `phase_open`, and
  `assign_address_spaces` runs after `dedup_section_names`.
- **sigil-cli.** `main.rs` is not touched. New tests only.

## A pre-existing silent wrong ROM this closes

The brief's probe shape (a 68000 program containing `save` / `!org 0` /
`cpu z80` / ... / `restore`) takes a path the investigation note did not describe
when the section open at `!org 0` itself begins at 0. That `org` is an in-section
**seek**, not a re-base: it pushes an `Org` fragment and moves the cursor to 0.
The `cpu z80` line then closes the section behind it, `phys_base += cursor` leaves
the counter at 0, and the Z80 section opens `Chained`. The placer packs it after
the section's extent. On base `66d2ed86` (sigil md5 `7e16f0561ecdb709ec1204938b2d88fd`):

```
$ sigil --hex p_naive.asm
00 00 00 00 00 00 00 00 F3 3E 01 00 00 00 00 00 00 00 00 00 00 00 00 00 4E 71
exit=0
```

The driver `F3 3E 01` sits at ROM `$8`, straight after the vectors, with no
diagnostic. Now the seek its section closes behind is recorded as the re-base, a
section with image bytes that lands in a second space is `Pinned` at its Z80
origin, and the program is refused at its `!org 0` line. The pin applies only to
sections the linker then refuses, so no accepted image moves.

The same seek-then-close under the **host** CPU is left alone, and it deserves its
own row: the front end opens the next section at `base + cursor` (its labels are
there) while the placer packs its bytes at `base + extent`. Fixing that would move
the bytes of any program that does it today, which this parcel may not do. It is
unmeasured beyond reading the code.

## Half-fix matrix and red-first evidence

**Method.** `mutate.sh`, a script in the parcel's scratch directory (not
committed), applies ONE literal substitution per site to a committed file and
refuses unless exactly one site matched. It quotes the mutated line back from disk
by a `MUTATION <id>` marker and runs the three targets that pin this behaviour:
`sigil-frontend-as --test as_address_space`, `sigil-link --lib`, and
`sigil-cli --test as_second_address_space`. It then restores the file with
`git show HEAD:<path> > <path>` and prints `git status --porcelain`. It refuses to
start on a dirty tree, so a failed restore stops the batch instead of stacking
mutations. The baseline is `ab7e8fe1`. Every row below reported `APPLY rc=0`, ran
all three targets, and printed an empty status after the restore.

The "red at" entries name the assertion that fired, read from its `panicked at`
location. That matters most for the rows whose danger is SILENCE. A red at CLI line
121 (`assert!(!out.status.success(), ...)`) means the command exited 0 with the
driver in the image. CLI line 65 is the same check in the helper, and line 49 is
"must assemble".

| id | half-fix modelled | mutated line, quoted from disk | red tests (red at) |
|---|---|---|---|
| M1 | **discriminator added, overlaps still scanned globally** | `relax.rs:412: if false && a_space != b_space { // MUTATION M1` | CLI `the_driver_is_refused_at_its_own_org_line_and_never_as_an_overlap` (123, `!stderr.contains("overlap")`: the old overlap is back); CLI `a_collision_inside_a_second_space_is_refused_as_an_overlap` (224); relax `a_second_space_section_is_refused_for_its_placement_not_as_an_overlap` (3959), `two_second_spaces_are_refused_separately_and_never_collide` (4010) |
| M2 | **per-space scan, no new diagnostic** | `relax.rs:1212: if false && !foreign.is_empty() { // MUTATION M2` | CLI driver test (**121: exit 0, the driver in the image, silently**); CLI `a_second_space_on_empty_image_ground_is_refused_too` (65: exit 0); relax `..._for_its_placement_not_as_an_overlap`, `..._even_where_the_image_is_empty`, `two_second_spaces_...` (`expect_err` got `Ok`) |
| M3 | **same-space overlap no longer refused** | `relax.rs:412: if true \|\| a_space != b_space { // MUTATION M3` | CLI `a_collision_in_the_image_is_still_refused_as_an_overlap`, `a_collision_inside_a_second_space_is_refused_as_an_overlap`; relax `a_collision_inside_one_space_is_still_an_overlap` |
| M4 | every section opening counts as a re-base (continuation rule lost) | `eval.rs:6218: rebased_at: self.rebased_at.take().or(Some(Span { .. })), // MUTATION M4` | FE `an_inline_z80_block_with_no_org_and_no_phase_is_in_the_image`, `a_label_between_the_org_and_the_cpu_line_does_not_decide`. Every CLI test stays green under it, which is why the inline test was added (`ab7e8fe1`) before this matrix ran |
| M5 | the image is always the 68000 | `eval.rs:1127: let host = Cpu::M68000; // MUTATION M5` | FE `a_z80_program_at_org_0_is_the_image`; CLI `a_collision_in_the_image_is_still_refused_as_an_overlap` (its Z80 half) |
| M6 | the image CPU is the first section, empty or not | `eval.rs:1127: let host = sections[0].cpu; // MUTATION M6` | FE `the_image_cpu_is_the_first_section_with_content` |
| M7 | the refusal fires only where the space collides with the image | `relax.rs:476: // MUTATION M7: refuse a second space only where it collides with the image` (+7 lines) | CLI `a_second_space_on_empty_image_ground_is_refused_too`; relax `..._even_where_the_image_is_empty` |
| M8 | a `phase` open is ignored | `eval.rs:1162: // MUTATION M8: a phase open is ignored` (the arm deleted) | FE `an_org_with_a_phase_open_is_an_image_placement`; CLI `an_org_with_a_phase_open_still_places_z80_code_in_the_image` |
| M9 | sections with no content decide | `eval.rs:1134: if true { // MUTATION M9` | FE `a_label_between_the_org_and_the_cpu_line_does_not_decide` |
| M10 | every org under a second CPU opens a new space | `eval.rs:1161: // MUTATION M10` (the same-space arm deleted) | FE `an_org_inside_a_driver_stays_in_the_driver_s_space`; CLI `a_collision_inside_a_second_space_is_refused_as_an_overlap` (split into two spaces, the collision vanished) |
| M11 | **a seek its section closes behind is not a re-base** | `eval.rs:6250: let _ = seek; // MUTATION M11` | CLI driver test (**121: exit 0**, the seek shape, which is base's own silent wrong ROM); FE `a_seek_back_that_its_section_closes_behind_enters_the_space_too` (52: the driver is in the image) |
| M12 | a second-space section is left `Chained` | `eval.rs:1142: // MUTATION M12` (the pin deleted) | CLI driver test (136: no longer `[0x0, 0x3) ... origin 0x0`, because the placer moved it after the vectors); FE `a_seek_back_...` (109: `Chained`) |
| M13 | the relax rebuild drops the space | `relax.rs:1304: space: sigil_ir::AddressSpace::Image, // MUTATION M13` | relax `the_measuring_entry_point_does_not_refuse_a_second_space` |
| M14 | **the CPU-only rule: any section under a non-image CPU is a second space** (two sites) | `eval.rs:6218: rebased_at: self.rebased_at.take().or(Some(Span { .. })), // MUTATION M14` and `eval.rs:1162: // MUTATION M14 (second site): a phase open is ignored` | CLI `sonic_1_s_phased_setup_block_assembles_to_the_reference_bytes` (**49: refused**), `an_org_with_a_phase_open_still_places_z80_code_in_the_image` (49); FE `a_phased_z80_block_with_no_org_is_in_the_image` (137), `an_inline_z80_block_...` (162), `an_org_with_a_phase_open_is_an_image_placement` (188), `a_label_between_...` (235) |
| M15 | **the image is the 68000, and the first section decides too** | `eval.rs:1127: let host = Cpu::M68000; // MUTATION M15` (and `.skip(first)` in the same substitution) | CLI `a_z80_program_at_org_0_still_assembles` (**49: refused**), `a_collision_in_the_image_is_still_refused_as_an_overlap` (197); FE `a_z80_program_at_org_0_is_the_image` (206) |
| M16 | one second space per CPU, whatever the entry | `eval.rs:1163: _ => ..Foreign { cpu, entered_at: Span { .. } }, // MUTATION M16` | FE `two_entries_from_the_image_are_two_spaces` (289), `a_driver_org_d_...` (79), `a_seek_back_...` (108), `an_org_inside_a_driver_...` (260), `a_label_between_...` (235); CLI driver test (127: not located at the `!org 0`), `a_second_space_on_empty_image_ground_is_refused_too` (158) |
| M17 | an org that leaves the section records no re-base | `eval.rs:6682: self.phys_base = phys_target; // MUTATION M17` (the record deleted) | FE `a_driver_org_d_to_z80_zero_is_a_second_space_entered_at_its_org` (52: the driver is in the image), `two_entries_...`, `an_org_inside_a_driver_...`, `a_label_between_...` (52), `a_seek_back_...` (111); CLI driver test (136: the unrecorded return org swallowed the following 68000 code into the driver's space, so the refusal named two sections, `[0x0, 0x1A)`), `a_second_space_on_empty_image_ground_is_refused_too` (65: exit 0), `a_collision_inside_a_second_space_is_refused_as_an_overlap` (224) |

**Coverage.** Every one of the 22 tests this parcel adds went red under at least
one row: the 10 front-end tests, the 7 CLI tests and the 5 relax tests. M1-M13 left
five tests that nothing had turned red: both phased-block tests, the CLI Z80
program, `two_entries_...` and `a_driver_org_d_...`. That gap was found by auditing
the matrix against the tests rather than against the list of half-fixes. M14-M17
are the half-fixes that turn those five red, run on the same method and baseline.

The brief's five required rows, each proven red:

| brief's row | test | red under |
|---|---|---|
| discriminator + global scan leaves the wrong-location overlap | CLI `the_driver_is_refused_...` (123, "overlap") | M1 |
| per-space scan + no diagnostic assembles silently, exit 0 | CLI `the_driver_is_refused_...` (121, exit 0) | M2, M11 |
| same-space overlap, 68000 with 68000 and Z80 with Z80 | CLI `a_collision_in_the_image_...` (both programs), `a_collision_inside_a_second_space_...` | M3, M5, M15 |
| the phased block assembles, same bytes | CLI `sonic_1_s_phased_setup_block_assembles_to_the_reference_bytes` | M14 |
| a pure Z80 program at org 0 assembles | CLI `a_z80_program_at_org_0_still_assembles` | M15 |

## Three-corpus diagnostic sets, before and after

Each corpus is a `cp -a` into the parcel's scratch directory, restored to its
recorded HEAD in the copy (`git checkout -- . && git clean -fd`, then
`git status --porcelain` = 0 paths): s1disasm `f6ece657`, s2disasm `e45ebf33`,
skdisasm `2fcd861c`. The originals were only read. Base binary md5 `7e16f056`;
new binary md5 `6b67f306`, built before the overlap-wording edit, which only
touches collisions inside a second space, and none of these corpora reaches one.
stderr was sorted, then `diff`ed whole.

| corpus | root | base lines | new lines | exact-line multiset diff |
|---|---|---|---|---|
| s1disasm | `sonic.asm` | 1 | 1 | the one line below |
| s2disasm | `s2.asm` | 149 | 149 | none (diff exit 0) |
| skdisasm | `sonic3k.asm`, plain | 189 | 189 | none (diff exit 0) |
| skdisasm | a wrapper root, `Sonic3_Complete = 0` + `include "sonic3k.asm"` (buildSK.lua's `-D`; the asm route takes no `-D`) | 172 | 172 | none (diff exit 0) |

```
< sonic.asm(81):3: error: sections `sec0` [0x0, 0x2CA) and `sec0#2` [0x0, 0x1BC6) overlap in the image (colliding pins)
---
> sound/z80.asm(9):3: error: section `sec0#2` [0x0, 0x1BC6) is assembled for the Z80 at origin 0x0, in a second address space this org opens outside the ROM image; the assembler cannot yet place a second address space into the ROM
```

S2 and S3K fail in the front end (their own open rows) and never reach the linker,
so this parcel cannot change their sets, and it did not. When they do reach it,
the rule says S2 gets one refusal (at `s2.sounddriver.asm(248)`) and S3K two (at
`Sound/Z80 Sound Driver.asm` 226 and 4444). **That is a prediction, not a
measurement.**

S1 cannot show the phased block's bytes while it is refused, so byte identity is
shown on the block by itself. S1's `SetupValues_Z80` (`sonic.asm` 320-353,
verbatim, md5 `d5337780`), placed between a longword and a word, gives the same 44
bytes on base sigil, on this branch, and on the md5-pinned `asl`
(`61e672562465725a8c102288a7da9098`, through `asl_ref.sh`'s `asl_run`: exit 0,
listing complete, 0 errors, 0 warnings) plus that corpus's `p2bin`
(`4f2fff99c3347bafb93b12d5be1db754`). This is pinned by
`sonic_1_s_phased_setup_block_assembles_to_the_reference_bytes`.

## Scoped suites and clippy

`SIGIL_ALLOW_PARTIAL=1`, with `AEON_DIR`, `EMPYREAN_SUITE_ROOT` and
`SIGIL_STRICT_GATE` unset and `CARGO_TARGET_DIR` in the parcel's scratch
directory. One crate at a time. Each log is stamped with pwd/HEAD/branch and was
checked for this parcel's own test names.

| crate | passed | failed | ignored | at |
|---|---|---|---|---|
| sigil-ir | 52 | 0 | 0 | `d4a957f4` |
| sigil-link | 143 | 0 | 0 | `d4a957f4` |
| sigil-frontend-as | 770 / **771** | 0 | 0 | `d4a957f4` / `ab7e8fe1` (the added continuation test) |
| sigil-frontend-emp | 2668 | 0 | 0 | `d4a957f4` |
| sigil-harness | 470 | 0 | 1 | `d4a957f4` |
| sigil-cli | 773 | 0 | 1 | `d4a957f4` |

The two ignored rows are pre-existing: `sigil_diff_reports_byte_identity` (reads
the aeon tree) and `secondary_pin_classes_match_the_hand_typed_baseline`
(retired). **Unmeasured:** `reference_dependence_is_named` reports the reference
tree absent and **130 test binaries reference-dependent**, with every row in them
skipped. Those rows include every engine byte gate, which the controller's strict
landing run measures.

Clippy (`cargo clippy --release -p <crate> --all-targets -- -D warnings`) exits 0
for all six crates. The first run refused a `type_complexity` in
`foreign_space_diags`. Every other crate failed on that same line, only because
clippy lints the workspace path dependencies it builds. Fixed in `d4a957f4`.

## Open, and corrections to the brief

**Nothing BLOCKED.** Stage 1 needed no ruling.

Left open, each with its reason:

1. **aeon is unmeasured**, by instruction (there is no aeon tree). The change
   cannot move an accepted byte: `flatten` is untouched, and the only placement
   change is on sections the linker then refuses. It CAN refuse a program it used
   to accept, if aeon's AS side ever orgs unphased code for a CPU other than its
   image's. The 130 skipped binaries are where that would show, and it would show
   loudly.
2. **A behaviour change that is not a byte change.** Z80 code org'd into a 68000
   program with no `phase` open used to be accepted when it happened not to
   collide: its bytes landed at ROM == Z80 address, which is what the reference
   post-processor does without its flag. It is refused now
   (`a_second_space_on_empty_image_ground_is_refused_too`). This is the rule
   working as stated, but it narrows what assembles, and the brief's no-byte-change
   bar does not cover it, so it is named here.
3. **The host-CPU seek-then-close defect** (above) needs its own row. It is
   unmeasured beyond reading the code.
4. **S2 and S3K are predictions.** Both fail in the front end first.

Corrections to the brief:

1. **"A Sonic-1-shaped minimal probe ... must exit nonzero" describes base's
   behaviour in only one of the two shapes that fit it.** On base, the shape where
   the open section at `!org 0` begins at 0 already exits **0, silently, with the
   driver at ROM `$8`**. The brief framed that silence as a future half-fix's risk;
   for that shape it is the present. It is now refused, and M11 proves the test
   sees it.
2. **The section's "own source line" is the `org`, not the section's first byte.**
   The refusal is located at the `org` that entered the space
   (`sound/z80.asm(9)`), which is also where the investigation note's sketch put
   it. The section's first byte is the driver's first instruction, which says
   nothing about why the bytes have no place.
3. **"Re-based by `org`" is two front-end mechanisms, not one**: an org that
   leaves the section, and a seek the section closes behind. The brief's
   hypothesis pointed at `switch_section_lma` and `vma_base`; the deciding state is
   really the physical counter's history, recorded at `org` and at
   `close_section`.
4. **The overlap check has moved, a little.** `overlap_diag` is now at `relax.rs`
   ~395 (the note cites ~378), and its call is the `(c2)` block of
   `resolve_layout_impl`. It is otherwise where the note says it is.
5. **`directive_org`'s doc comment** said a backward re-base's collision is named
   by `overlap_diag`. For a second space that is no longer true, and the comment
   now says which check names which case.
