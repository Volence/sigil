# SECOND-SPACE-PIN-UNWITNESSED: the second-space pin in `assign_address_spaces` is dead, and is removed

Queue row SECOND-SPACE-PIN-UNWITNESSED. Parcel branch `parcel/second-space-pin`,
base master `dd6e9f1b`. Tests commit `9061e56d`, removal commit `2f60b782`.

**Verdict: DEAD.** No shape can observe the `Pinned` that `assign_address_spaces`
stamped on a second-space section with image content: not its placed address,
not an image byte, not a label, not a diagnostic. The one reader of the flag is
the linker's placement pass, and in every shape where the stamp was new
information the `Chained` section it would otherwise be is placed at the same
address. The stamp is removed, and the function's doc comment now says what it
does: it sets `space`, and placement is the builder's.

It was not unreachable. It fired in thirteen probe sections. And it did guard
something, a counterfactual: with the builder's own pin at a re-base removed, the
stamp is what kept a second space's code at its origin. That made it a second
guard over the builder's pin, for second spaces only (the image never had one),
and it hid the builder pin's absence from the second-space tests. Two new tests
now guard the builder's pin for this shape directly.

## The question

`assign_address_spaces` (`crates/sigil-frontend-as/src/eval.rs`) tags every
section with its address space. After an `org`, the first section with content
decides the space, and when that section landed in a second space and held image
bytes, the function also stamped it `Pinned`. The doc comment gave the reason: the
builder pins the section opened at a re-base already, and "this pin also covers
the first section WITH CONTENT when an empty one opened first".

The backward-seek fix (`2026-09-12-as-backward-seek-split-fix.md`, row M4) removed
that stamp and turned nothing red across four test targets. It left open whether
the stamp was dead or merely untested. This parcel ran the whole workspace suite
and built the shapes the doc comment names.

## Provenance

| | |
|---|---|
| base | `dd6e9f1b`, `cargo build --release --bin sigil`; md5 `f08b4d99075dcf7437593d5e9e5f540a`, `--version` `sigil 0.1.0 (dd6e9f1b)`, tree clean at capture |
| M4 binary | the same tree with M4 applied; md5 `6aa1179fd32caee7529ea94115245faf`, `--version` `sigil 0.1.0 (dd6e9f1b-dirty)`, 1 modified |
| asl | `61e672562465725a8c102288a7da9098` (s1disasm `build_tools/Linux-x86_64`), only through `asl_ref.sh`'s `asl_run`, flags `-xx -n -q -A -L -U -i .`; every cited run `ASL_EXIT=0`, 0 errors |
| p2bin | `4f2fff99c3347bafb93b12d5be1db754`, beside that asl, with each probe's own `-z` |
| aeon | `.aeon-sigil-ref` at `ec640bcf`, all four shapes built and verified by the controller; read, never built here |
| cargo target | `/home/volence/sonic_hacks/.scratch/second-space-pin/target` for every cargo command |
| emulator | none; nothing here needs runtime confirmation |

The probe sources, the instruments and the measured results are in
`2026-09-12-second-space-pin/`: `probes/`, `instruments/` (run from this parcel's
scratch directory, whose paths they name), `results/`.

## Step 1: M4 reproduced

`mutate.py` refuses a dirty tracked tree, applies one literal substitution by an
anchor that must match exactly once, and quotes the mutated line back from disk.
On `dd6e9f1b`:

```
MUTATED crates/sigil-frontend-as/src/eval.rs:1199: let _ = &sec.placement; // MUTATION M4 second-space pin removed
```

The four targets the backward-seek note names, with the runner's log showing
`Compiling sigil-frontend-as` from the mutated tree:

| target | passed | failed |
|---|---|---|
| `sigil-frontend-as --test as_address_space` | 11 | 0 |
| `sigil-link --lib` | 142 | 0 |
| `sigil-cli --test as_backward_seek_close` | 4 | 0 |
| `sigil-cli --test as_second_address_space` | 7 | 0 |

## Step 2: the whole suite under M4

`scripts/landing-run.sh --baseline 5317 --aeon /home/volence/sonic_hacks/.aeon-sigil-ref`,
with M4 on disk. The baseline is the observed total of the last green landing log
on an ancestor (`19dd5a4d`, 5317 passed).

```
  tree            .../agent-ab5e0036e93441809 @ dd6e9f1b (parcel/second-space-pin, DIRTY)
  reference       /home/volence/sonic_hacks/.aeon-sigil-ref @ ec640bcf (HEAD, clean), all four present
  started/ended   2026-09-12T09:07:31Z -> 2026-09-12T09:14:32Z (UTC)
  CARGO_EXIT      0
  CLIPPY_EXIT     0   (lint bar clean)
  LEDGER_EXIT     0   (ledger gate clean)
  suites          478
  passed          5342
  failed          0
  ignored         2
  skip lines      0
  binaries        478 launched, 478 reported
  reconciles      5317 baseline + 25 new = 5342 observed
  RESULT          GREEN
```

`DIRTY` is the M4 line and nothing else. `ratchet:` lines: 0. No test in the
workspace needs the stamp, including the strict aeon four-shape byte gates.

## Why no shape can observe it

A "can never matter" claim is a claim about every reader and every route, so
here are all of them.

**1. Who reads `placement`.** Across every crate's source, `place_pass`
(`relax.rs` 282 and 286) and the bank bump beside it (311), which acts only on a
section with `bank` set; the AS front end never sets one (no `set_section_bank`
call in it). `resolve_layout`'s rebuild copies the flag through. Nothing prints
it: no tracked non-Rust file carries a `placement:` line (the control, the same
pattern over `.rs` files, finds 13). The overlap check, the out-of-image refusal,
the run-overrun check and `flatten_placing` all read `lma` and `space`, never
`placement`.

**2. When the stamp was new information.** Three sites re-base the counter:
`directive_org`'s arm for an `org` with no section open, its arm for an `org`
that leaves the open section, and `close_section` for a seek back its section
closes behind. Each sets `rebased_at` and calls `pin_next_section` together.
`open_section_if_needed` is the front end's only caller of `switch_section_lma`,
and one call takes `rebased_at` and consumes the pin. So the section opened at a
re-base, call it R, is always `Pinned` by the builder. The stamp could only change
the flag of a LATER section C, which happens exactly when R held no fragments,
because a section with fragments would have decided the space itself.

**3. Everything from R to C opened at the same counter.** Every section after R
and before C has no fragments and no `rebased_at` (one with a `rebased_at` is a
new R, and the chain restarts there). `phys_base` changes only at those three
re-base sites and in `close_section`, by `current_offset()`. Every cursor move in
`IrBuilder` pushes a fragment first (`seek` 231, `emit_data` 316, `emit_fill`
323, `reserve` 330, `emit_fragment` 338), so a section with no fragments closes at
offset 0 with `max_offset` 0. So C opens where R opened: `C.lma == R.lma`.

**4. `Chained` lands C there.** All AS sections are in the anonymous group.
`place_pass` puts R at `R.lma` (pinned), then advances by
`max(reserved_span, final_size)`, which is 0 for every fragment-less section, so
C's `Chained` base is `R.lma`, which is `C.lma`. R never moves, so this holds on
every pass of the fixpoint.

**5. Every route that assembles AS.**

| route | what it does with the sections |
|---|---|
| `sigil <file.asm>` (`main.rs` 428) | hands `module.sections`, in order, to `resolve_layout_placing`: the argument above |
| the aeon harness (`native.rs` 1603) | assembles only aeon's AS unit, which is `games/*/game_root.asm` and the one file they include, `engine/debug/debugger.asm`. They hold 0 `org` lines; the same pattern shape finds the `include` lines (the 2 directives, and a comment that mentions one), so it can match. `rebased_at` is never set there, and the stamp never fired. The harness also reorders by its map and stamps its own placement |
| `examples/s1_phase_profile.rs` | the CLI route, timed, writes nothing |
| tests | call the front end and the link functions directly |

## The probes

Sixteen sources, each run through the base and the M4 binary. `results/STREAMS.tsv`
has each run's exit code, error count, image md5 and diagnostic. `diff -r` over the
two output trees: exit 0, identical in exit code, stdout, stderr and image, every probe.

| probe | shape | stamp fired | outcome, both binaries |
|---|---|---|---|
| p01 | one label between `!org 0` and `cpu z80` | yes | refused: `[0x0, 0x3)` Z80 at origin 0x0, no `-z` |
| p02 | p01 with `-z=0` | yes | assembles, 274 bytes, = reference |
| p03 | two labels (the second under the new CPU) | yes | refused, as p01 |
| p04 | inside the driver: `org 40h`, a label, `phase 1000h`, a byte (rule 2 with a phase) | yes | refused: `sec0#1`, `sec4096` `[0x0, 0x41)` |
| p05 | a reservation leading the code | yes | refused: `[0x0, 0x5)` |
| p06 | a reservation-only section decides, code after it | no (control) | refused: `sec4` at origin 0x4 |
| p07 | a later seek back, closed behind, next bytes on the tail | yes | refused at the seek, overlap in the second space |
| p08 | a collision inside the second space | yes | refused, colliding pins in the second space |
| p09 | a Z80 image entering a 68000 space | yes | refused: 68000 at origin 0x100 |
| p10 | the same empty-first shape inside the image | no (control) | assembles |
| p11 | two second spaces, each behind a label, both `-z` | yes, both | assembles, 290 bytes, = reference |
| p12 | p03 with `-z=0` | yes | assembles, 274 bytes, = reference |
| p13 | p05 with `-z=4` | yes | refused: origin 0x0, no `-z` |
| p14 | a `phase` open at the `org` (rule 3, image) | no (control) | assembles |
| p15 | p05 with `-z=0` | yes | refused by `-z`: no code starts at 0x0 (starts at 0x4) |
| p16 | the `org` finds no section open | yes | assembles with `-z=0`, 274 bytes, = reference |

Reference toolchain, same sources, same `-z`: p02, p11, p12 and p16 are byte for
byte sigil's image (`cmp`). Without `-z` (p01, p03, p04, p09), p2bin writes the
second space's bytes over the image at their own addresses, exit 0, which is the
silent wrong ROM sigil refuses. The listings bind every label where the tests
say: `DriverStart` and `Inner` at 0, `L40` at $40 with the phased `db 5` at run
address $1000, `Mark` at $100, `D2` at $1300.

p05, p06, p13 and p15 spell the reservation `ds.b 4`, because sigil refuses `ds 4`
under `cpu z80`. asl does the reverse (see "Left open"), so those four are sigil-only
probes of the reservation fragment and cite no asl behaviour.

## The positive control

An identical-output result says nothing unless the stamp ran. A probe
instrument (`instruments/zz_pin_probe_dump.rs.txt`, put into
`sigil-frontend-as/tests/` for the run and removed after) dumps every section:
CPU, space, placement and `lma` from the front end, then its `lma` and label
addresses after the placement pass (`resolve_layout_measuring`), and the checked
verdict (`resolve_layout_placing` with the probe's `-z` origins).

- `dd6e9f1b` against M4, the first 15 probes: the dumps differ only in the
  placement word, on 13 sections (`results/dump-dd6e9f1b-vs-m4.diff`).
- `9061e56d` (stamp present) against `2f60b782` (stamp removed), all 16: 14
  sections change, the thirteen plus p16's, and every changed pair is identical
  once the word `Pinned`/`Chained` is masked. The paired diff exits 0; the same
  diff without the mask exits 1, so the check can see a difference
  (`results/dump-9061e56d-vs-2f60b782.diff`).

The stamp fired thirteen times, and not one placed `lma`, label or verdict moved.
It never fired in the three controls.

## What the stamp did guard

M5 removes the builder's pin in `directive_org`'s arm for an `org` that leaves
the open section. M6 removes the pin in its arm for an `org` with no section open.

- **M5 on `9061e56d`, stamp present.** `a_second_space_behind_sections_with_no_content_is_placed_at_its_org`
  stays GREEN. The stamp pins the driver at its origin even though R is no longer
  pinned. Five CLI tests go red (three in `as_second_address_space`, two in
  `as_backward_seek_close`), all through image code that moves.
- **M5 on `2f60b782`, stamp removed.** The same test is RED, and so are three
  existing second-space CLI tests that stayed green on `9061e56d`:
  `the_driver_is_refused_at_its_own_org_line_and_never_as_an_overlap`,
  `a_second_space_on_empty_image_ground_is_refused_too` and
  `a_collision_inside_a_second_space_is_refused_as_an_overlap`.

So the stamp was redundant with the builder's pin while that pin held, and it
masked that pin's absence where it did not. The image never had such a second
guard, and its empty-first shape (p10) has always relied on the same `Chained`
placement. With the stamp gone, both spaces rest on the builder's pin, and the
suite catches its removal in more places than before.

## What changed

- `crates/sigil-frontend-as/src/eval.rs`: the stamp is removed. The doc comment
  now reads: this function sets only `space`; the builder pins the section it
  opens at a re-base and chains the rest; a fragment-less section spans nothing,
  so a `Chained` section behind one is placed at its `lma`, in either space; a
  section outside the image with bytes is refused at link unless `-z` places it.
- `crates/sigil-frontend-as/tests/as_address_space.rs`:
  `a_second_space_behind_sections_with_no_content_is_placed_at_its_org`. Five
  shapes (p01, p03, p16, p04, p09). Each asserts the shape it depends on (the
  label opens a fragment-less section ahead of the code, and the code is a second
  space entered at the named `org`), runs the placement pass, and asserts the
  code is placed where asl binds the label.
- `crates/sigil-cli/tests/as_second_address_space.rs`:
  `a_driver_behind_sections_with_no_content_is_placed_where_p2bin_places_it`.
  p12, p16 and p11 through the shipped command with `-z`, against the reference
  toolchain's images. `assemble_ok_with` passes the `-z`.

## Red-first evidence

Baseline: the committed `2f60b782`. Each row: applied with the line quoted from
disk, run through `new.sh` (`as_address_space`, `as_second_address_space`,
`as_backward_seek_close`, `sigil-link --lib`), restored with
`git show HEAD:<path> > <path>`, tracked status empty after.

| id | mutated line, quoted from disk | the frontend test | the CLI test |
|---|---|---|---|
| M5 | `eval.rs:7206: let _ = 0; // MUTATION M5 org-leave pin removed` | RED at 246: `one label between the org and the cpu line: ... sec0#1 [0x0] was placed at 0xA` (left 10, right 0) | RED at 59, `must assemble`: `section sec0#1 [0xA, 0xD) is assembled for the Z80 at origin 0xA ... no -z instruction places it` |
| M6 | `eval.rs:7182: let _ = 0; // MUTATION M6 no-section org pin removed` | RED at 246, on the case aimed at this arm: `the org finds no section open: ... sec0#1 [0x0] was placed at 0x102` (left 258, right 0) | RED at 59, `must assemble`: `sections sec0 [0x0, 0x8) and sec272 [0x3, 0x5) overlap in the image` |

After the M6 restore, `new.sh` on `2f60b782`: `as_address_space` 12, `as_second_address_space` 8,
`as_backward_seek_close` 4, `sigil-link --lib` 142, all passed. Both new tests
were also green on `9061e56d` with the stamp present, before the removal.

The CLI test's red under M6, and under M5 on `9061e56d`, comes through the image:
the `!org $110` back to the image takes the arm the mutation broke, so the image
code moves too. It is red for a real placement defect, but not specifically the
driver's. The frontend test is the one that isolates the driver.

## The landing gate at `2f60b782`

`scripts/landing-run.sh --baseline 5342 --aeon /home/volence/sonic_hacks/.aeon-sigil-ref`
with `--expect-test` for both new tests. The baseline is the M4 run's observed
total, which M4 did not change; commit `9061e56d` adds two tests.

```
  tree            .../agent-ab5e0036e93441809 @ 2f60b782 (parcel/second-space-pin, clean)
  reference       /home/volence/sonic_hacks/.aeon-sigil-ref @ ec640bcf (HEAD, clean), all four present
  started/ended   2026-09-12T09:29:31Z -> 2026-09-12T09:36:22Z (UTC)
  CARGO_EXIT      0
  CLIPPY_EXIT     0   (lint bar clean)
  LEDGER_EXIT     0   (ledger gate clean)
  suites          478
  passed          5344
  failed          0
  ignored         2
  skip lines      0
  binaries        478 launched, 478 reported
  test targets    478 expected launches (465 runnable + 13 doctest, 0 excluded for required-features; cargo metadata --no-deps), all launched
  reconciles      5342 baseline + 2 new = 5344 observed
  RESULT          GREEN
```

`ratchet:` lines: 0. Each new test appears once in the log, `ok`. The strict aeon
four-shape byte gates are inside this run, so the removal moves no engine byte,
which the reachability argument already says it cannot: aeon's AS unit has no
`org`.

## Left open

- **sigil and asl disagree on Z80 reservations, both ways.** Under `cpu z80`, asl
  accepts `ds 4` (its listing puts the next byte at 4) and refuses `ds.b 4` and
  `defs 4` with `#1200 unknown instruction`. sigil refuses `ds 4` and `defs 4`
  (`unknown directive or mnemonic`) and accepts `ds.b 4`. `ds` is under-accepted,
  and `ds.b` is over-accepted with a value asl never gives.
- **`org 1300h` under `cpu 68000`**: sigil assembles it (a 4866-byte image); asl
  refuses it with `#1020 invalid symbol name`, because the `h` suffix is Intel
  syntax and is read only under the Z80. Found by this parcel's own first p11,
  which had the `org` before the `cpu z80` line.
- **A reservation-led second space cannot be placed by any `-z`.** The link-time
  check exempts a space whose origin, meaning its first section's `lma`, is named
  by a `-z`. `flatten_placing` matches a `-z` against the first byte's run. For
  a driver that opens with a reservation, those are 0 and 4. `-z=4` fails the first
  (p13) and `-z=0` fails the second (p15). Loud either way; asl's behaviour here is
  unmeasured, since `ds` is the spelling asl takes and sigil refuses it.
- Where a front end section's `lma` leaves the placer unchanged is now a property
  the tests pin for five shapes, not one the linker checks. A CLI-route check
  that no AS section with a baked `vma_base` moves in placement would make every
  label-from-bytes split loud, whichever pin went missing. Booked in the gap
  ledger, not built.

## Corrections to the brief

1. **"either dead or untested"**: dead in the sense that matters (no observable
   difference in any shape, for the reason above), but not inert. It fired
   thirteen times and was a second guard over the builder's pin that M5 shows
   masking. The decision stands because the guard was redundant while the
   builder's pin holds, one-sided (second spaces only), and hid a real regression
   from existing tests.
2. **H1, "it may matter to what the linker is allowed to move, relax or refuse"**:
   it does not. Relaxation never reads `placement`. Every refusal reads `lma`
   and `space`. The bank bump reads `placement` only for a section with `bank`,
   which the AS front end never sets.
3. **H2, "the phased/unphased rules in `space_after_org`, or a link-time refusal
   of an out-of-image section with bytes"**: neither reads `placement`. Rule 3
   (a `phase` open) makes the section image, where the stamp never applied (p14).
   Rule 2 with a phase (p04) fired it, with no difference.
4. The brief's "about four minutes" for the gate: the M4 run took seven minutes
   on this machine.
5. The M4 row quotes `eval.rs:1166` for the stamp. At `dd6e9f1b` the line is 1199.
