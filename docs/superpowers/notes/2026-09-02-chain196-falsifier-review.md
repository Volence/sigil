# Chain 196: reviewing the aeon lane's runtime evidence by re-derivation

The aeon lane reported the chain-196 runtime evidence for the alignment flip
(their `parcel/alignment-flip-hole-196` @ `fe565724`,
`docs/superpowers/notes/2026-09-02-chain196-runtime-evidence.md`): both TAGGED
runtime items pass, the abs.w falsifier widened from a label list to an encoding
scan, and a build control isolating what the aeon-side edits move on their own.

This note records what was re-derived here rather than accepted, on the reference
tree this lane already had provisioned: `/home/volence/sonic_hacks/.aeon-ref-resolver`,
a detached aeon worktree at `027ec1620dd977bf7b8ee47cbafe2b2197059092` (the chain-195
provenance tip, i.e. the PRE-flip side), whose rebuild control matched the golden at
2026-09-02T04:43Z. Their measurements are post-flip; these are pre-flip, so the two
sides pair rather than repeat.

## Method

A word-aligned scan of each shape's ROM image for the 68000 absolute-addressing
encodings:

- `4EB9` / `4EF9` — `jsr`/`jmp` (xxx).L, 32-bit operand;
- `4EB8` / `4EF8` — `jsr`/`jmp` (xxx).W, 16-bit operand, sign-extended at execution,
  so any operand >= `0x8000` resolves into RAM rather than ROM. That is the hazard
  the flip has to not create.

Byte-stepped and word-aligned scans return identical counts on all four shapes, so
the population carries no odd-offset noise. That was a doubt worth removing, not a
finding.

## Corroborated firsthand

**The eleven sites, from the other side.** Pre-flip `s4.bin` contains exactly 11
abs.l references to `Sound_PlaySFX` at `0x00008054`, split **9 `jsr` + 2 `jmp`**.
The aeon lane counted 9 `jsr ($7FBC).w` + 2 `jmp ($7FBC).w` post-flip with zero
abs.l survivors. Same count, same encoding-class split, both sides of the flip,
measured independently. The conversion is complete and nothing is half-converted.

**The two debug sites and their shift.** Pre-flip `s4.debug.bin` has exactly 2 abs.l
`jmp` to `Raster_Install` at `0x00008042`, at `0xA687C` and `0xA688E`. Post-flip the
aeon lane reports them at `0xA6870` and `0xA6880` — deltas of -12 and -14. The extra
2 bytes on the second is the first site's own abs.l -> abs.w shrink, so the pair is
internally consistent.

**The build control's identity.** Their control builds the same 196 tree with the
pre-flip binary and lands on `fdd1cf81/719387`, which they identify as the chain-195
golden. This lane's own provisioning of that tree rebuilt `s4.bin` to
`fdd1cf81/719387` and asserted it against the golden at 04:43Z, before their control
ran — so the identity is corroborated by a second, earlier, independent build, and
their conclusion that the aeon-side edits move nothing on their own stands.

## Two findings the evidence does not yet cover

**1. The control is one flag deep; the second debug flag is new, not pre-existing.**

Pre-flip flagged encodings (operand >= `0x8000`), word-aligned, whole image:

| shape | abs.w encodings | flagged | where |
|---|---|---|---|
| s4 | 99 | 1 | `0xA4F4C` `4EF8 FFFE` |
| s4_debug | 155 | 1 | `0xA71FC` `4EF8 FFFE` |
| demo | 10 | 1 | `0x104D8` `4EF8 FFFE` |
| demo_debug | 10 | 1 | `0x104D8` `4EF8 FFFE` |

The aeon lane's post-flip figures are s4 109/1, s4_debug 158/**2**, demo 10/1,
demo_debug 10/1. Their control demonstrates that the `FFFE` flag is pre-existing:
`0xA71FC` pre-flip vs `0xA71F0` post-flip, twelve bytes apart, the shift the flip
causes. That control covers ONE of the two debug flags. The other, which they place
at `0xA970A` and attribute to `EndOfRom+0x17D6` (deb2 appendix, data), has **no
pre-flip counterpart**: pre-flip `s4.debug.bin` contains no `4EB8`/`4EF8` encoding
anywhere in `0xA9600`-`0xA9900`, at any operand.

Their attribution is right in kind. The bytes there are plainly a data table of
ascending 16-bit values in pairs — pre-flip `0xA9700` reads
`9218 4E6A 92BE 4E78 9346 4E86 936C 4E99 93EC 4EA7 9458 4EB6 94FE 4EC4 ...`, with the
ascending series passing straight through the `4EB8`/`4EF8` range. Post-flip the table
sits at a different offset and one entry pair happens to read as `jsr ($9542).w` (the
aeon lane's post-flip measurement; this lane only had the pre-flip side and predicted
the encoding class wrongly as a `jmp`). It is a coincidence in data, not an
instruction, and it is harmless.

But the sentence "Pre-existing, not created" holds for the `FFFE` flag and not for this
one, and the difference matters because the falsifier's whole claim is a statement about
what the flip did or did not create.

**2. The count deltas do not reconcile, and the population is contaminated by data.**

Conversion alone predicts pre-flip + 11 for s4 and + 2 for s4_debug:

| shape | pre-flip | predicted | aeon post-flip | unexplained |
|---|---|---|---|---|
| s4 | 99 | 110 | 109 | **-1** |
| s4_debug | 155 | 157 | 158 | **+1** |
| demo | 10 | 10 | 10 | 0 |
| demo_debug | 10 | 10 | 10 | 0 |

Both are off by one, in opposite directions. The explanation is almost certainly the
same one as finding 1 — incidental `4EB8`/`4EF8` byte pairs in data appearing and
disappearing as a 62-byte shrink re-aligns the tables. That is benign, but it is
unstated, and it is the same mechanism that produced the new flag. Since the claim
being defended is "zero abs.w encodings in engine/game code, any shape, that would
sign-extend into RAM", and the scanned population is the whole ROM image rather than
code, each unreconciled delta is a site that has not been shown to be data.

The cheap fix is to bound the scan by the code region (or attribute each encoding to
its section through the listing) instead of scanning the image, which turns the claim
from "the flags I found resolve outside the subject" into "the subject contains none".
Either that, or reconcile the two deltas by name.

Neither finding contradicts the flip. They bear on what the evidence proves, not on
whether the flip is sound, and the first is a wording repair plus one attribution.

## Both findings closed by the aeon lane (their `5944dad5`)

They checked rather than took both, and closed gap 2 the better way — by bounding the
scan rather than by excusing names. Word-aligned, three bands: emitted code
`[0x200, ErrorHandlerBlob)`, the vendored MD Debugger blob up to `EndOfRom`, and the
deb2 appendix past it, with this lane's chain-195 goldens as the pre side.

| band | s4 pre -> post | s4_debug pre -> post |
|---|---|---|
| code region | 97 -> 108 (+11) | 154 -> 156 (+2) |
| blob + appendix | 2 -> 1 | 1 -> 2 |
| whole image | 99 -> 109 | 155 -> 158 |

The band decomposition reproduces this lane's independent whole-image pre-flip totals
(97+2 = 99, 154+1 = 155) and their own earlier post-flip totals (108+1 = 109,
156+2 = 158) exactly, so the -1/+1 of finding 2 is fully accounted: it is entirely the
blob-and-appendix band, and `0xA970A` is the +1. Flagged operands >= `0x8000` in the
code region: **zero, pre AND post, both shapes**. Because it holds on the pre side too,
that is a property of the region rather than a lucky post-flip reading — a stronger
claim than the one this note asked for. Their separate observation that the vendored
blob is `0xF56` bytes in both shapes pre and post is its own evidence the island's
content is untouched.

Gap 1 was worse than one mis-attributed flag: the "pre-existing, not created" sentence
was true of `$FFFE` and written as though it covered both, and the prose counted five
flags then resolved four — one cause, which was never looking at the appendix flag.
Corrected in their note's section 4 and kept as a record rather than quietly patched.

The general lesson, theirs and worth carrying: **a whole-image scan defending a claim
about code will always carry flags that need individual excuses, and every excuse is a
place to be wrong.** Bound the population to the subject and the excuses disappear.

## The landing run's test-count reconciliation, checked against this lane's own branch

The aeon lane's landing run reports 4182 passed / 0 failed / 2 ignored across 363
binaries, reconciling as 4177 + 5 new, and they summed `test result:` across all 363
independently rather than taking the wrapper's figure. The "+5" is a claim about THIS
lane's branch, so it is this lane's to check rather than accept.

It reconciles, but not as a bare +5: `parcel/alignment-flip-195` adds **six** `#[test]`
and removes **one**, for a net of five. A net that contains a removal is exactly the
shape in which a reconciliation can hide a lost guard, so the removal was chased.

The removed test is `a_pin_that_violates_the_declaration_is_refused_with_the_residue`,
which asserted that a pin four bytes off its declared alignment is REFUSED with
`[layout.undeclared-alignment] … base % 8 = 4`. It is not a lost guard for two
independent reasons:

- `validate_declared_alignment`'s signature changed from three arguments
  (sections, pins, flags) to one (sections), because the parcel's whole subject is
  that the packing walk reads `required_for(head label)` and no longer reads the
  pin residue. The old test exercised an API that no longer exists.
- The assertion was deliberately INVERTED, not dropped. Its replacement,
  `a_doctored_pin_residue_does_not_move_a_packed_section`, doctors `Sfx_33`'s frozen
  pin by +4 and requires the build to go through, the section not to move, and the ROM
  to be byte-identical. And it names the removed test's own behaviour as its falsifier
  in its doc comment: "run against [the pre-flip packer], the doctored build is refused
  with `[layout.undeclared-alignment] … base % 8 = 4` and the `Ok` arm below is never
  reached."

So the retired assertion survives as the new test's documented falsifier, which is the
right shape for a guard whose meaning a parcel reverses. The `[layout.undeclared-alignment]`
diagnostic is still asserted in three places on the branch, the same as on master.

## A ledger row this raises on the sigil side

The aeon lane observed that the pre-flip binary did not REFUSE the `at = 0x3F0` hole
row — it silently built the old layout. The consumer of that row is this repo's
parser, so the row is ours: a declaration that an older toolchain ignores rather than
rejects is a silent-divergence class, and it means a hole declaration is not
self-checking against the toolchain that reads it. Worth an unknown-key or
version-floor refusal. Booked; not scheduled.
