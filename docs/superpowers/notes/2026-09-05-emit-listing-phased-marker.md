# The listing now says which of its addresses are VMA and which are stored elsewhere

Branch `parcel/emit-listing-phased-marker`, off master `83d293e7`.
Marker commit `c13e64c3`.

A `.lst` row gave every symbol an address and never said whether that was the
address the code RUNS at (VMA) or the address it is STORED at (LMA). This adds a
fourth section that says so. It ADDS information: no existing row is
reinterpreted, renumbered, or moved.

---

## 1. The derived phased population, and how it was derived

### The method

`crates/sigil-harness/src/native.rs::listing_from_resolved` walks the RESOLVED
sections, which is the only place the fact exists. A symbol is phased when its
defining section satisfies both of:

    sec.vma_origin() != sec.lma        // it runs at one address, is stored at another
    sec.image_len() > 0                // and it actually stores bytes

and its `lma` is then `sec.lma + label.offset`, so a row names the byte and not
just the base. Everything else carries `lma: None`.

The second clause is a judgement and is worth stating plainly. Aeon's RAM blocks
are `vma: $FFFFxxxx` over an LMA that anchors at the physical counter and place
ZERO image bytes. They satisfy the first clause, so a one-clause rule reports
every RAM label as phased, with a "storage address" for bytes that were never
stored and that a consumer could follow into unrelated ROM. RAM symbols are
already recognised by every consumer here from their own `$FFFFxxxx` value; what
the marker exists to disambiguate is a phased address that looks ORDINARY. The
exclusion has its own unit test with a non-vacuity arm
(`a_reserve_only_ram_block_is_not_reported_as_phased`).

### The listings measured, and the command that produced them

    .target-land/release/sigil build --aeon /home/volence/sonic_hacks/.aeon-ref \
        --native --game sonic4 --debug -o <out>.bin --emit-lst <out>.lst

built from this worktree with
`CARGO_TARGET_DIR=<worktree>/.target-land cargo build --release -p sigil-cli`,
against the shared reference tree at `aeon_rev 483b3e12` (read only; nothing was
written into it). Every shipped shape was measured the same way, exit 0 on all
seven.

| shape | symbols | PHASE COUNT |
|---|---|---|
| sonic4 plain | 2406 | 6 |
| sonic4 debug | 2970 | 6 |
| demo plain | 1463 | 2 |
| demo debug | 1654 | 2 |
| config_a | 2989 | 6 |
| config_b | 2351 | 2 |
| lean | 2395 | 6 |
| stress_evict | 2970 | 6 |

NO SHIPPED SHAPE IS UNPHASED. That is why the count-0 case had to be
constructed rather than found (section 4).

The sonic4 set, verbatim from `s4.debug.lst`:

    PHASE COUNT 6
    PHASE SoundTablesZ80_Head      VMA $00008000 LMA $000B8000
    PHASE MovingTrucks_PitchTable  VMA $00008357 LMA $000B8357
    PHASE SndDefaultPitchTable     VMA $00008357 LMA $000B8357
    PHASE SfxBlobWinTab            VMA $0000845F LMA $000B845F
    PHASE SeqOpcodeTable           VMA $00008571 LMA $000B8571
    PHASE DacSampleTable           VMA $000085B1 LMA $000B85B1

All six come from one section,
`games/sonic4/data/sound/soundbankhead.emp`'s `section soundbankhead (cpu:
m68000, vma: $8000)`, placed at LMA `$B8000`. The demo set is a different class:

    PHASE COUNT 2
    PHASE Z80_IdleProgram                              VMA $00000000 LMA $000003CC
    PHASE $engine.z80_init$Z80_IdleProgram$code_end    VMA $00000028 LMA $000003F4

### Cross-checked against the only other derivation that exists

`aeon/tools/scene_spans.py::vma_phased_symbol_names()` derives the same set from
SOURCE: it scans every `.emp` file for `section ... (vma: ...)` and collects the
top-level `proc`/`data` names inside. Run against the reference tree it returns
36 names. Compared with what the listings actually carry:

* the 6 sonic4 names are all in its 36, so for that shape it over-approximates
  soundly;
* **29 of its 36 reach NEITHER shipped listing.** They are the `cpu: z80`
  driver's own names, which compile to a separate seam-2 blob that never enters
  the 68000 listing;
* it **misses** `$engine.z80_init$Z80_IdleProgram$code_end`, which the demo
  listing does carry as a body row at `(0) 3/28` and a symbol row at ` ... : 28
  C |`. Its `TOPLEVEL_DECL_RE` only sees top-level declarations, so a phased
  section's interior labels are invisible to it.

So the source derivation is wrong in BOTH directions. Its consumer
(`lst_proc_sizes`) is insulated from the second one by an unrelated property (its
`LST_HEAD_RE` name class rejects a leading `$`), which is the shape of a defect
that survives review: the derivation is wrong and the caller is right, for a
reason that has nothing to do with the derivation.

### The evidence this parcel rests on, stated where it can be checked

`vma_phased_symbol_names`'s own docstring, in the reference tree, dated
2026-09-03:

> Measured against a real build (2026-09-03): `SoundTablesZ80_Head` at listing
> address $8000 truncated `Parallax_Step5_Vscroll` to 64 bytes [...];
> `SfxBlobWinTab` at listing address $845F, 21 bytes after `Raster_HInt`'s head,
> truncated it to 21 bytes for the same reason.
>
> THE LISTING CARRIES NO MARKER OF ITS OWN for this [...] this is a SOURCE
> derivation, not something recoverable from the listing alone.

And the second, more recent attempt: identifying phased symbols BY ADDRESS RANGE
returned `Flush_VDP_Shadow`, `Process_DMA_Important`, `QueueDMA_Important`,
`Set_VDP_Reg`, `VDP_Shadow_Init`, `BootData_End`. Those are 68000 engine routines
at low ROM addresses. None of them is phased. The re-derivation failed in exactly
the way the marker exists to prevent, in the hands of someone who had just been
warned about the hazard.

**Two premises are NOT carried here and should not be picked up from anywhere
else.** "Five engine tools re-derive this and only one gets it right" is
withdrawn and unsubstantiated. The phased-population figures "45 confident floor
/ 533 upper bound" are withdrawn; the 45 is the address-range list above.

---

## 2. The shape

`ListingSymbol` gains `lma: Option<u32>`. `None` means unphased, which is also
the honest answer for an equate (a value has no storage) and for a reserve-only
block. `Some(l)` means phased, and `l` is where the bytes are.

    Phase Table (every address above is a VMA):
    -------------------------------------------

    PHASE COUNT 6
    PHASE SoundTablesZ80_Head VMA $00008000 LMA $000B8000

Two decisions inside the ruled design:

**Unconditional, carrying a count.** The ambiguity closed is one bit per
LISTING, not one per symbol. Absent section = an older sigil that never looked;
`PHASE COUNT 0` = this sigil looked and found nothing. Omitting the section when
empty spells those two the same way, and that is the reading nothing recovers
from. The cost is that an unphased listing is no longer byte-identical to the
pre-phase format: ruled, accepted, and the byte identity that matters is the
ROM's, which is unchanged (sonic4 debug `crc=e2144057 len=840324` before and
after).

**A phased row carries the LMA, not just the name.** This goes one step past the
brief and it is deliberate. A marker that says only WHICH symbols are phased
leaves a consumer knowing a symbol's printed address is not its storage address
and still having to re-derive where the bytes are, which is the re-derivation the
parcel exists to end. The row states both.

Vocabulary is `VMA`/`LMA`. Not taste: 22 files in the aeon tree already say LMA,
11 say VMA, zero say run or store, and inventing a second name for a thing that
has one, inside the marker whose purpose is to stop tools re-deriving one fact,
would be the same mistake in a new place.

---

## 3. The consumer-grammar proof

Ran before and after on real listings, both arms, with EXIT CODES and, per the
widened bar, an extracted VALUE from each arm. Two arms that agree while having
measured nothing are the same family as a control that bypasses its subject, so
agreement alone is not reported anywhere below.

Script: `grammar_proof.py`, exit **0**. Arms:

* REAL pair: `before/s4.debug.lst` (master's emitter, no Phase Table) vs
  `after/s4.debug.lst` (`PHASE COUNT 6`).
* CONSTRUCTED pair: section 4.

| # | consumer | BEFORE | AFTER |
|---|---|---|---|
| 1 | `s4budget.py` CLI + `parse_listing` | exit **0**, `ROM: 820.6 KB/4.0 MB (20.0%) ... RAM: 59.3 KB/64.0 KB (92.6%)`, declared=**2970**, `Flush_VDP_Shadow`=**0x1c8c**, `Camera_X`=**0xffffa728** | identical, exit **0** |
| 2 | `scene_spans.lst_proc_sizes` (`LST_HEAD_RE`) | **627** procs, `Flush_VDP_Shadow`=**30 B**, `Raster_HInt`=**338 B** | identical |
| 3 | `effects_gates` dense-stream probe | `OJZ_GradientStream`=**0x14264**, 1 hit, the `(0) ` probe accepts **2970** lines | identical |
| 4 | `convsym -input as_lst` | exit **0**, **1053** rows, `1C8C: Flush_VDP_Shadow`, zero phase lines absorbed | identical |
| 5 | `preset_lab_witness.lst_symbol` + `ramp_authored_witness.lst_symbols` | **0x1c8c**, **2970** names, `Camera_X`=**0xffffa728** | identical |

s4budget's cross-check is the load-bearing one: it requires the two address
views to be the same `(name, value)` sequence, of the same length, equal to the
`N symbols` trailer, so a partial parse cannot masquerade as a small program.
Exit 0 with `declared=2970` on both arms is that invariant holding.

**The brief named four grammars. There are more, and one of them is the only one
whose failure would have moved ROM BYTES.** `convsym -input as_lst` is the
deb2-appendix parser, and the appendix is built by running `convsym` over
`emit_listing`'s own output (`native.rs::append_deb2_appendix`). If convsym had
absorbed a `PHASE ...` line as a symbol, the appendix would have grown and the
ROM CRC would have changed. It did not: 1053 rows both arms, and the built ROM is
`crc=e2144057 len=840324` before and after. The other two additions
(`preset_lab_witness`, `ramp_authored_witness`) are further real listing readers
found by grepping the reference tree rather than working from the given list.

Oracle's `LoadFromAsListing` is covered by `m1b_gate::
oracle_loadfromaslisting_resolves_emit_listing`, which compiles a probe against
the real `Symbols.cpp` and runs it over an emitted listing; it passes with
`SIGIL_STRICT_GATE=1`, so it ran rather than skipped.

### Which other answer could this proof have given

Three deliberate mis-spellings of the section, each in one consumer's grammar,
appended to a real listing in place of the shipped shape:

| mutation | grammar_proof exit | caught by |
|---|---|---|
| `PHASE ... : B8000 C \|` (an s4budget symbol row) | **1** | s4budget refused: "declares 2970 symbols and 2971 rows parsed"; convsym also went 1053 -> 1054 |
| `(0) 2971/B8000 : ...:` (an address head) | **1** | `lst_proc_sizes` 627 -> 628, the effects probe 2970 -> 2971, the witness readers 2970 -> 2971 |
| `   6 symbols` (an s4budget trailer) | **1** | s4budget refused: "declares 6 symbols and 2970 rows parsed" |

The proof is not one that cannot fail, and the shipped spelling clears all five
consumers.

---

## 4. The count-0 case

**No shipped aeon shape is unphased** (the table in section 1: 6/6/2/2/6/2/6/6),
so a real-scale count-0 listing does not exist in that tree and had to be
constructed. Both routes are recorded because they cover different things.

**(a) Emitter-produced, small, against the real tool.**
`m1b_gate::s4budget_parses_emit_listing` feeds the real `s4budget.py` a listing
`emit_listing` produced from two unphased symbols. With the section
unconditional, that listing carries `PHASE COUNT 0` by construction. The test now
asserts the line explicitly rather than only exercising it, and its verdict is
that s4budget EXITED 0 and printed a real `ROM:` line, not that two runs agreed.
Nothing is hand-written here; this is the emitter's own output.

**(b) Constructed, real scale, against all five consumers.**
`construct_unphased.py` removes the six phased symbols from BOTH address views of
`after/s4.debug.lst`, renumbers the body, restates both trailers, and writes
`PHASE COUNT 0`, plus an "older sigil" twin with the Phase Table stripped
entirely. That is the before/after pair a build with no phased section would
produce. The construction is not self-certified: s4budget cross-checks the two
views against each other AND the trailer, so its acceptance is what proves the
removal stayed consistent. Result: 2964 symbols in both views, exit 0 on both
arms of all five consumers, with `Flush_VDP_Shadow`=0x1c8c and
`Camera_X`=0xffffa728 extracted on every arm.

The unit-test half is `listing::tests::
unphased_listing_still_carries_a_count_zero_phase_table`, which asserts the
header, the zero count, the absence of rows, and that no existing row moved.

**Aeon's partial discharge is corroboration, not this.** They appended a section
to a listing that HAS phased content, and separately tested a count-0 trailer on
`demo.debug.lst` built with the sound driver off (214,269 -> 214,279 bytes, exit
0 both arms, 1654 rows both, `Camera_X` FFFFA72C, `VDP_Shadow_Init` 402). They
bounded it themselves: they did not show that listing is genuinely unphased, and
they tested the trailer line, not the header shape, which they could not know.
Both limits are honoured; neither result is counted toward the condition above.

---

## 5. Red-first, with the mutation shown applied on disk

`git checkout <rev> -- <path>` STAGES, so plain `git diff --stat` reports nothing
about a mutation that is really there. Every state check below is
`git diff HEAD --stat` plus a content grep of the code site.

**M1: delete the Phase Table from the emitter.** Applied: `crates/sigil-link/src/
listing.rs | 14 --------------`, the count-line grep 1 -> 0.

    listing::tests::unphased_listing_still_carries_a_count_zero_phase_table   FAILED
    listing::tests::phased_symbols_get_vma_and_lma_rows                       FAILED
    listing::tests::phase_lines_never_parse_as_an_address_row                 FAILED
    listing::tests::phase_table_does_not_disturb_the_two_view_cross_check     FAILED
    listing::tests::an_equate_never_reaches_the_phase_table                   FAILED
    every_shipped_shape_declares_which_of_its_addresses_are_phased            FAILED
    s4budget_parses_emit_listing                                              FAILED

**M2: neuter the producer so nothing is ever marked phased.** Applied:
`crates/sigil-harness/src/native.rs | 2 +-`, the `phased.then` grep 1 -> 0.

    native::phase_marker_tests::a_phased_section_yields_vma_and_lma_...       FAILED
    native::phase_marker_tests::a_reserve_only_ram_block_is_not_reported...   FAILED
    every_shipped_shape_declares_which_of_its_addresses_are_phased            FAILED

M2 is the interesting one. Every per-shape consistency check still passes under
it, because `PHASE COUNT 0` on all seven shapes is a perfectly legal listing.
What fails is the non-vacuity clause, and its message prints what it measured:

    Measured: sonic4 plain: PHASE COUNT 0 []
              sonic4 debug: PHASE COUNT 0 []
              ... all seven ...

Restored from the committed baseline both times, verified with
`git diff HEAD --stat` (empty) and the grep back at 1, and green again after:
13 / 2 / 1 / 5 passed, 0 failed.

### The applied-check was itself wrong, twice, and that is the finding

My first applied-check grepped for `Phase Table (every address`, which also
appears in the emitter's DOC COMMENT. It reported "MUTATION DID NOT APPLY" while
`git diff HEAD --stat` showed the 14 deletions sitting on disk. My fix for that
had broken shell escaping and returned 0 at the clean baseline too, i.e. it was
an always-red check that would have refused every run.

Two consecutive instrument defects, in the check whose entire job is to detect
instrument defects, and neither was caught by its own output. What caught both
was printing TWO independent witnesses of the same state side by side
(`git diff HEAD --stat` and the grep) and reading them against each other. A
single-witness applied-check is a coin flip you cannot audit.

---

## 6. The landing run

Invocation copied from the brief, not retyped:

    scripts/landing-run.sh --baseline 4650 --aeon /home/volence/sonic_hacks/.aeon-ref

with `CARGO_TARGET_DIR` set to `.target-land` inside this worktree.

### Run 1, at `4f4f83b5`: RED, 2 failures

    FAILING TESTS (2), all of them:
      every_selected_test_file_is_classified
      the_derived_accessor_set_is_the_declared_guard_set
    CARGO_EXIT 101   CLIPPY_EXIT 0   suites 411
    passed 4656   failed 2   ignored 2   skip lines 0
    reconciles  4650 baseline + 8 new = 4658 returned a verdict (4656 passed + 2 failed)

Both are one cause: the new test file `listing_phase_marker` landed in none of
the nightly source-gate lane's three buckets, and an unclassified file makes that
whole lane REFUSE and produce no coverage. The gate is the landing-time copy of
that lane going dark, so it found the defect one landing before the 05:17 timer
would have, and against this tree rather than against master.

    SOURCE_GATES=46 scanned=140 source=45 artifact=87 no-reference=7 unclassified=1
    unclassified: listing_phase_marker

Fixed in `84228134` by classifying it as a SOURCE gate, which is what its inputs
make it: it compiles the corpus and reads the assembler's own answer about the
sections it produced, opening no built ROM, no listing FILE, and no golden.

A finding came out of the fix. Its near-twin `listing_defines` builds every
shipped shape's listing the same way and sits in the ARTIFACT bucket, and the
difference is not one of inputs: the classifier greps the whole file for the
string `.lst`, `listing_defines` mentions a listing's file extension once in
prose, and this file does not. The script already knows it does this and says so
about six other files that match the artifact pattern "ONLY in prose", keeping it
that way deliberately because tightening it would push those six into the
refusal. So part of that bucket boundary is a property of how a file is WORDED.
Recorded in the run list beside the new entry so a later reader does not conclude
the two tests were judged to have different inputs.

    after: SOURCE_GATES=47 scanned=140 source=46 artifact=87 no-reference=7 unclassified=0

### Run 2, at `84228134`: GREEN

    log        .target-land/landing-20260906T011859Z.log
    tree       .../worktrees/agent-a0abba1b9a6abd93f @ 84228134
               (parcel/emit-listing-phased-marker, clean)
    reference  /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (HEAD, clean), all four present
    started    2026-09-06T01:18:59Z -> 2026-09-06T01:24:27Z (UTC)
    CARGO_EXIT 0    CLIPPY_EXIT 0 (lint bar clean)
    suites     411
    passed     4658   failed 0   ignored 2   skip lines 0
    reconciles 4650 baseline + 8 new = 4658 observed
    RESULT     GREEN

`pwd` and `HEAD` are the log's own stamp, quoted above, not asserted here. The
delta is exactly the parcel's own 8 tests and no others, and all eight are named
in the green log rather than inferred from the total:

    listing::tests::unphased_listing_still_carries_a_count_zero_phase_table ... ok
    listing::tests::phased_symbols_get_vma_and_lma_rows ... ok
    listing::tests::phase_lines_never_parse_as_an_address_row ... ok
    listing::tests::phase_table_does_not_disturb_the_two_view_cross_check ... ok
    listing::tests::an_equate_never_reaches_the_phase_table ... ok
    native::phase_marker_tests::a_phased_section_yields_vma_and_lma_... ... ok
    native::phase_marker_tests::a_reserve_only_ram_block_is_not_reported_... ... ok
    every_shipped_shape_declares_which_of_its_addresses_are_phased ... ok

plus `s4budget_parses_emit_listing ... ok`, which is the count-0 assertion added
to an existing test rather than a new one.

---

## 7. Anything in this brief you concluded was wrong

**1. The retained aeon claim about WHAT is phased is wrong, and aeon's own tool
says so.** The brief kept, "held to what they checked", that the phasing in that
tree is the Z80 side, `phase 0` for the inline driver blob
(`engine/sound/z80_sound_driver.emp`, `sound_sfx.emp`) and `phase 08000h` for the
SFX bank window. Measured: the `$8000` window in the sonic4 listings is
`games/sonic4/data/sound/soundbankhead.emp`, declared `cpu: m68000`, and all six
of its phased symbols are 68000 data. `vma_phased_symbol_names`'s docstring
states the same conclusion in the reference tree already:

> note `cpu: m68000`, not `z80`: the collision is NOT a "Z80 symbol" class, it is
> a PHASED-SECTION class, and `cpu: z80` is neither necessary (this section
> proves it) nor sufficient (several `cpu: z80` sections in this tree -
> `z80_sound_driver.emp`, `sound_fm.emp`, `sound_psg.emp`,
> `sound_sequencer.emp`, `sound_sfx.emp` - declare NO `vma:` ...)

and the two files the brief names, `z80_sound_driver.emp` and `sound_sfx.emp`,
are in the list that declares no `vma:` at all. The `phase 0` half is right in
kind but attributed to the wrong file: the demo listing's two phased symbols come
from `engine/z80_init`, `Z80_IdleProgram` at VMA `$0` / LMA `$3CC`. So the one
claim the brief kept as still-standing from that lane is refuted by that lane's
own measured file, and the class it names ("the Z80 side") is precisely the
mis-classification that file exists to warn about.

**2. "The four grammars" undercounts, and it omits the only one that could have
moved ROM bytes.** `convsym -input as_lst` reads `emit_listing`'s output to build
the deb2 appendix that goes INTO the ROM. Had it absorbed a `PHASE` line, the
CRC would have moved and no listing-level check would have said why. Two more
real listing readers (`preset_lab_witness.lst_symbol`,
`ramp_authored_witness.lst_symbols`) also exist. The four named are the four that
were known, not the population; a grep of the reference tree is what turns that
list into a population, and this is the same shape as gating a producer instead
of enumerating the consumers.

**3. A design addition, flagged rather than smuggled.** The brief's ruled design
is a section carrying a count with rows for phased symbols. It does not say what
a row contains. I made rows carry VMA AND LMA, because a row that says only
"this one is phased" leaves the consumer re-deriving where the bytes are, which
is the failure the parcel is aimed at. If the hub wants rows reduced to names,
that is a one-line change to the emitter and a two-line change to the tests.

**4. A rule of my own that is not in the brief and should be reviewed:
reserve-only sections are excluded.** Aeon's RAM blocks have VMA != LMA and store
nothing. Including them would be defensible on the letter of "phased" and is
wrong on the purpose, and it would swamp six meaningful rows with every RAM label
in the build. I ruled it out, tested it both directions, and am flagging it
because it is a judgement about what the marker MEANS, not a mechanical
consequence of the ruling.

**5. The `grep -r` warning is correct, verified rather than inherited.** A
canary written to a gitignored path: the shell function's `grep -r` returned
nothing and exit **0**; `/usr/bin/grep -r` found it. A zero from the former would
have read as a finding.

**6. A smaller one: the count-0 case was already being exercised before I
asserted it.** The brief says the count-0 line "is the one nothing else
exercises". The moment the section became unconditional,
`m1b_gate::s4budget_parses_emit_listing` was feeding the real s4budget a
`PHASE COUNT 0` listing, because its synthetic symbol set is unphased. What was
missing was not the coverage but the ASSERTION naming it, so that a future change
that dropped the header would fail on the header rather than on some downstream
symptom. That is still worth adding, but the brief's claim about the state of the
world was not quite right.
