# `LISTING-EQU-SHAPE-BLIND`: one table answers "what is this name worth in this ROM" in two senses, unmarked

Reported by the aeon lane, diagnosed there, routed rather than patched; **nothing of sigil's was
touched by them.** Every claim below was re-verified at this seat against this tree rather than
taken from the report.

## The defect

`s4.stress.lst` publishes `EQU PAGE_FRAMES_CLAMP = $0000000C` while `s4.stress.bin`, built in the
same invocation, emits `cmpi.w #$0009,d6`. **A listing and the binary it describes disagree about a
constant**, and nothing marks which sense the reader is getting.

## Verified here, claim by claim

Their mechanism cites `d7e6aa15`, the revision the running binary stamps into `DIGEST-ASSEMBLER`,
not this lane's tip. **Checked rather than assumed:** `d7e6aa15` is an ancestor of master, and the
first diff hunk in `native.rs` between it and `1bfce22f` opens at line **1604** — above every line
they cite, so the tip carries them identically.

| claim | verified in this tree |
|---|---|
| `harvest_engine_constants(aeon: &Path)` takes no profile | yes, `native.rs:1262` |
| it folds with the hard-coded seed `[("STRESS_EVICT", 0)]` | yes, `native.rs:1279` |
| `harvest_game_constants(aeon, rel, debug: bool)` IS shape-aware | yes, `native.rs:1308` |
| the harvest feeds `attach_guarded_equ_exports` | yes, `native.rs:1542` → `sigil-frontend-as/src/eval.rs:927` |
| a guarded export is published as an equate row | yes, `sigil-link/src/listing.rs:182`, `EQU {name} = ${:08X}` |

So one `pub const` is evaluated twice — once shape-blind for the harvest, once shape-aware through
`stress_evict_profile()`'s `emp_defines` (`native.rs:937`) — and **the listing publishes the pinned
one** while the ROM carries the other.

**Their control is the part worth copying.** They did not read the code and conclude; they perturbed
the formula so the two folds would land on values that are *neither* of the observed ones and are
unequal, registered the prediction, then built: `EQU = 11`, emitted `#$0007`. That also killed the
benign reading, since a capacity-versus-index offset would be fixed and this gap moved.

## Scope, as measured by them and consistent with this tree's own behaviour

Canonical shapes agree (both publish 12 and emit `#$000C`), and at `STRESS_EVICT=0` the two paths
evaluate the same expression and structurally cannot differ. **Severity is bounded by sigil's own
refusal:** a `pub const` folding an unseeded define aborts at exit 101 and writes no ROM, so the
silently-wrong case needs a define the harvest seeds, and it seeds exactly one at exactly one value.
**No shipping ROM is affected**; they read `0c46 000c` out of `s4.debug.bin` at `$93B6` directly
rather than trusting its listing.

## ⚑ The class, and it is this lane's own banked rule failing on this lane's own comment

The comment at `native.rs:1272-1279` already anticipated the stale value and called it *"an unused AS
define"*, saying the harvest *"feeds the AS `-D` side only"*. **The verifiable half is true** — no
`.asm` in aeon references the name. **But the same guarded define is also exported as a link
`EquSym` and published in the listing**, where `tools/evict_witness.py` reads it. A correct sentence
whose SCOPE was the AS side, about a value whose REACH is wider.

This lane already holds that rule, in memory as *gate every consumer of a value* and in
`docs/OVERSEER-REFERENCE.md` as *a property verified at the producer is not one of the consumers*.
**It did not fire, because the comment was not being written as a verification — it was being written
as an aside.** Same shape as the process-check rule that failed the same night: a rule that asks the
reader to NOTICE does not fire at the moment of writing; only one that names a thing to look for and
fail to find does. **The question that would have caught it is mechanical: this value is computed
here, so what reads it — and did I check each one, or only the one I had in mind?**

## The choice, NOT made here

Two options, aeon's framing, and they are right that what cannot stand is one table answering in two
senses with no marking:

1. **Make the engine-constants harvest shape-aware**, as the game harvest already is.
2. **Stop publishing shape-blind harvested values as equate rows** — omit them, or mark them.

**Option 2 is probably not this lane's call alone.** The listing's equate table is consumed by the
engine lane's tools, and the debug-info/build-manifest sigil emits is an Aether contract artifact
whose schema is coordinated in `contract/` (empyrean `dd9e34e1`). Changing what the table publishes,
or adding a marking to it, is therefore contract-adjacent and goes to the hub as a CR rather than
landing as a lane decision. **Option 1 changes no published shape and is internal**, which is one
real argument for it beyond taste.

Not started tonight: this lane is past its clear line and at a boundary, and the standing rule is to
reach a boundary without dispatching the next wave.

Aeon-side handling that stays regardless: `evict_witness.py` prints both and goes with the ROM.
Their full packet is at aeon `512548ce`, `docs/DEFERRED_WORK.md`.
