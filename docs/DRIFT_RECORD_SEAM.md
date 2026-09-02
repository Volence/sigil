# The drift record seam

`scripts/nightly_ref_drift.sh` measures a CRC. It holds **no expectation of its own**, and
the harness is built so that it cannot acquire one: every expectation enters through the
one command named by `DRIFT_RECORD_READER`, and when that command is absent the job
reports NOTHING MEASURED rather than a pass. A drift job whose expectations it generated
itself measures nothing, and that is the failure this whole project exists to retire.

The record is the **aeon lane's** artifact (their plan's §2 item 1: *"the aeon-committed
expected CRCs step 1's nightly job reads. Not merely a file: which shapes, at what
cadence, and what a mismatch means when the assembler has legitimately moved"*). This
document defines only the **protocol** between the job and that record — the format and
storage stay entirely on their side of the seam.

## The append cadence — a REQUIREMENT, and this document did not have one

**One entry per landing, appended by the aeon lane: the four shapes with size, CRC and the
aeon revision.** Accepted by that lane 2026-09-02 as a requirement rather than a
recommendation, via their `tools/drift_record.py measure` verb, which emits a candidate
entry for review instead of writing one.

**Why it is stated here at all.** Everything above defines how to ASK the record a
question; nothing said how answers get into it. So there was no cadence, and with both
coordinates moving constantly a record with no cadence can only fall behind. Measured the
day this was written: the record held **two** entries, both hand-measured, at a single
sigil revision already well behind. The job was working perfectly and correctly reporting
that it could not attribute anything — which is the honest answer and a useless one. A
seam that specifies only the read side produces exactly that.

**The cadence is nearly free because a landing already does the work.** A freeze measures
all four shapes at a known `(aeon_rev, sigil_rev)` pair; that *is* an entry. The record
then holds the coordinates the job looks up first, and a night on which nothing moved can
finally come back QUIET rather than "cannot say".

**And the caveat travels with the requirement, because the entry count will look like
coverage and is not.** A record built only from freezes contains **only agreements** — a
freeze's pair is by construction one where the bytes matched. That is fine for the lookup
the job performs, which asks what a given pair should produce. It means the record can
never hold a known-bad expectation, so **the number of entries is never evidence of
coverage breadth**, and nobody should later read a long record as a well-exercised one.

## The reader protocol

`DRIFT_RECORD_READER` is a command. The job appends a verb and its arguments, and reads
stdout. Four verbs:

| invocation | stdout | exit |
|---|---|---|
| `<reader> shapes` | one shape name per line | 0 |
| `<reader> lookup <aeon_rev> <sigil_rev>` | `<shape> <crc8> <size>` per line | 0 = hit, 3 = no entry for this pair |
| `<reader> lookup-aeon <aeon_rev>` | `<sigil_rev> <shape> <crc8> <size>` per line | 0 = hit, 3 = no entry at this aeon revision |
| `<reader> has-sigil <sigil_rev>` | — | 0 = the record has at least one entry at this sigil revision, 3 = none |

Exit **2** from any verb means *the reader could not answer* — a broken record, an IO
error, a version it does not understand. It is never treated as "no drift": the job
records NOTHING MEASURED and says which verb failed.

`crc8` is eight lowercase hex digits of CRC32; `size` is the byte length. Provenance
identity in this suite is CRC32 + size, never SHA1.

Both revisions are full 40-character SHAs.

## Why the key is the PAIR, and how a pair key still catches assembler drift

The record is keyed on `(aeon_rev, sigil_rev) -> crc` per shape so that the *cause* of a
difference is a property of the key rather than something either side computes. The four
cases the job discriminates fall straight out of the two lookup verbs:

1. **Same pair, different CRC** (`lookup` hits, CRC differs) — the only unambiguous
   defect. Identical inputs on both sides produced different bytes: nondeterminism or an
   environment leak. **A red.**
2. **`lookup` misses, `lookup-aeon` hits** — the engine source is one the record knows and
   the assembler is not. The record's CRC at that `aeon_rev` is a real expectation for
   this build, so a mismatch is the assembler moving bytes under identical engine source:
   **a red, and the population step 4 actually needs.** A match is quiet *and*
   evidence-bearing.
3. **`lookup-aeon` misses, `has-sigil` hits** — aeon moved only. No expectation exists for
   this engine source, and a CRC change here is the ordinary consequence of an engine
   edit. **Not a red and not quiet: UNVERIFIED.** It does not advance N.
4. **Both miss** — both moved. The difference is unattributable and the job **says so**
   rather than picking a cause. It does not advance N either.

**Case 2 is the load-bearing one and it is a constraint on the record, not on the job.**
If the record only ever gains an entry when *aeon* moves, then every sigil-only move is a
`lookup` miss with a `lookup-aeon` hit and case 2 works exactly as written. If instead the
record is regenerated on sigil's revision as well — an entry minted for the new pair from
the very build being judged — case 2 collapses into case 1 with a self-authored
expectation, and the job's ability to see assembler drift disappears silently. **The
record must not mint an entry for a pair from the build that pair is about to be judged
against.** That is the one property of aeon's format this job cannot check for itself, and
it is worth stating in their generator rather than here.

## What `sigil_rev` must mean

Not sigil's `HEAD`. The build input is the **assembler binary that ran**, and this repo
does not pin it: `SIGIL_BUILD`/`SIGIL_EMIT` come from the environment, and `SIGIL_EMIT`
*writes* `engine/sound/generated/`. A clean tracked tree at a fixed `aeon_rev` can build a
different ROM with no cause visible in the tree at all.

`sigil --version` answers this directly, and the job reads it rather than asking git:

* `revision` — the revision the executable was **linked at**. `build.rs` names git `HEAD`,
  `refs`, `packed-refs` and every manifest in the closure as cargo rerun triggers, so the
  stamp is re-captured when the linked code moves.
* `closure-revision` — the last commit that touched the paths cargo actually compiles this
  binary from, walked from cargo's own dependency graph. **This is the better key
  component**, because `revision` moves on every commit in the repository including ones no
  compilation can see: a docs-only commit makes two byte-identical assemblers look like
  two different ones, and keying on it manufactures case-2 misses that carry no evidence.
* `tree:` — the working-tree state at capture. **`dirty` makes the key non-identifying**: the
  bytes correspond to no committed revision. The job records the state word and never
  advances N on a dirty build.

The job records all three. Which one aeon keys on is theirs to choose; the recommendation
is `closure-revision`, with `revision` and the tree state carried as recorded provenance.

## Until the record exists

`DRIFT_RECORD_READER` is empty in `scripts/drift-nightly.conf`. Every run therefore reports

```
STATUS: NOTHING MEASURED — no drift record is configured
```

with a non-zero exit, no chain credited, and N unmoved. That state is deliberately
indistinguishable in the report from a broken reader, because both mean the same thing:
nothing was measured. It is not a pass, it is not a zero, and it is not green.
