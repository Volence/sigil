# The stability sweep, the five notes that cannot be found, and the 22 that can

`STABILITY-RUNNER-MISSING-WHERE-CLAIMED` asks for a sweep. Its premise is *"five notes from the same
week state their ground truth as the UNRELIABLE copy cited by version banner, with no committed
probe directory to re-run"*. **The five cannot be recovered, and the row should stop saying five.**
What can be recovered is a larger, mechanically derived population that contains the class the row
is about, and a committed script that returns it. Tree at `946e83c9`.

## The five are unreproducible, and the previous sweeper was right to refuse to invent them

The 2026-09-08 staleness sweep judged this row `UNGROUNDED-IN-PRACTICE` and said so plainly:
*"The row says 'five notes' and names none of them ... I could not construct an instrument that
returns the five, and I am not going to guess a population and then declare it swept, because a
sweep whose population was invented by the sweeper is the failure mode the lane already books."*

**That judgement is upheld here and nothing below replaces it.** No instrument tried today returns
five. The count is the row's own unverified figure, in the same family as the three premise-less
rows found this morning: **a number with no artifact reads exactly like a measured one.** Sizing
work off it would have meant sweeping until five turned up, which is the shape the earlier sweeper
declined and which would have been the third instance of that defect in one day.

## What IS derivable: the citation form, and why the banner is the defect

All four `asl` binaries in this workspace print `Macro Assembler 1.42 Beta [Bld 212]` verbatim
(`docs/superpowers/notes/asl-reference/README.md`). **So a note citing its oracle by banner has
named nothing.** The rows it states cannot be attributed to a binary, and two binaries that disagree
are indistinguishable in the record. The identifying citation in this lane is an md5. The repaired
instance is `2026-09-05-disp-or-call-probes/README.md`, which reconstructs a note whose probes were
never committed, and which names both binaries by md5 precisely because the banner cannot separate
them.

**Population, from `scripts/sweep_asl_citation_form.sh`: at `946e83c9`, 51 notes cited the banner, 79
cited an identifying md5, and 22 cited the banner and never an md5.** The list is printed by the
script rather than transcribed here, so it cannot rot away from the instrument that produces it.

**THE FIGURES ARE A FUNCTION OF THE NOTES DISCUSSING THE DEFECT, INCLUDING THIS ONE, SO THE SCRIPT
IS THE AUTHORITY AND THESE NUMBERS ARE A SNAPSHOT.** Committing this note moved the count from 22 to
23, because the paragraph above quotes the banner while the first draft cited no md5: **the note
documenting the defect became an instance of it, inside the commit that documented it.** The md5s
are now cited here (`61e672562465725a8c102288a7da9098` reference,
`0dee1f98e6480a4783d27ffd8b90896f` varying, from `asl-reference/README.md`), which takes this file
back out of the population. `engine/z80_bus.emp`'s census header in the aeon tree carries the same
warning from its own parcel, so this is a known class rather than a surprise: a count partly derived
from the document stating it moves whenever that document is edited.

## What this sweep does NOT establish, stated so its silence is not read as coverage

* **It tests ONE spelling**: the literal banner text against two known md5s. A note describing its
  oracle in prose (*"the reference build"*, *"the s2disasm copy"*) with neither string is invisible
  to it. A clean result means *no note cites the banner without also citing an md5*, never *every
  asl claim in the tree is attributable*.
* **It does not test whether a given claim DEPENDS on which binary produced it.** Many of the 22 may
  state build-independent facts, where the citation is untidy rather than unsound. Separating those
  needs reading, not grepping, and is not done here.
* **`git grep` cannot see ignored files**, so the scope line says `tracked .md only`. This is the
  blind spot that became suite contract today at empyrean `e28601a`.
* **It is not a gate and is not wired into one.** Making it one needs a red-first proof and a
  derived expectation, neither of which exists. It reports a population and exits 0.

## The instrument broke on its first run, and only the positive control said so

The first draft ended its control check with `git grep -l ... | grep -qx "$control"` under
`set -uo pipefail`. **`grep -q` exits on its first match and closes the pipe; `git grep` then dies of
SIGPIPE; `pipefail` reports the pipeline as failed. A successful match was reported as a failure**,
so the script announced `POSITIVE CONTROL FAILED` against a perfectly healthy tree.

**The number it printed before the fix was 22, and the number after the fix is 22.** That is the
uncomfortable part and it is the reason the control earns its place: **the figure was right and
unjustified at the same time**, and without the control the run would have looked like an ordinary
success. This inversion is already banked in this lane's session memory, which is the shape of the
day: **the rule was held, and skipped in the direction where this seat was the author of the
instrument rather than the auditor of someone else's.**

## The standing fix, which is what actually closes the class

A stability or provenance claim about the reference assembler **cites a committed script or an md5,
or it is not made**. The row's own remedy applied to the row: the sweep that finds the defect is now
a committed script rather than a sentence asserting that somebody swept.
