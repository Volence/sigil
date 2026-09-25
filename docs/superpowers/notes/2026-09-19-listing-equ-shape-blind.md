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

**⚠ TWO LINE NUMBERS, BOTH CORRECT, AND THE DISAGREEMENT IS WORTH MORE THAN EITHER.** Aeon's packet
says the first change is at **1607**; this note said **1604**. Resolved from git objects at the named
revisions rather than by either side conceding: `@@ -1604,6 +1604,7 @@` is where the HUNK opens,
context included, and the INSERTED line (`+            sources: a.sources,`) lands at **1607**, the
fourth line of that hunk, confirmed present at 1607 in the tip. **Both measurements are accurate and
they measure adjacent quantities; both were reported as "the first change".** The equivalence
conclusion is untouched either way, since both are above every cited line.

Filed rather than quietly fixed because it is a NAMING collision, not a transcription error, and that
is the harder kind: **both parties check their own number, both find it correct, and the conflict
lives only in the shared phrase.** A reader meeting both records would otherwise have had to decide
which of us was sloppy, and the answer is neither.

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

## LANDED: option 1, the engine-constants harvest is shape-aware (branch `parcel/listing-equ-shape-aware`, fix commit `df055bd1`)

**What changed.** `harvest_engine_constants(aeon, profile)` now folds `engine/system/constants.emp`
under `shape_defines(profile, aeon)`, the define set the `.emp` build lowers the same module under,
instead of the hard-coded seed `[("STRESS_EVICT", 0)]`. `harvest_game_constants(aeon, rel, profile)`
takes the profile too, because its engine seed was the same shape-blind fold (its `DEBUG` seed still
reads `profile.debug`). The comment that said the harvested value "feeds the AS -D side only" is
replaced by one naming all three readers. Option 2 was not touched: the listing's row set and
published shape are unchanged.

**Every consumer of the harvest's return value, enumerated by grep, not only the one in the report:**

| consumer | reached through | shape-aware after the fix |
|---|---|---|
| residual AS comptime reads | `assemble_as_side` → `AsOptions.guarded_defines` → `eval.rs` seed env | yes |
| guarded-name collision refusal | same → `eval.rs` guarded-define name set | names only, value-independent |
| link `EquSym`s | `attach_guarded_equ_exports` (main and bonus module) → link symbol table, `blob.rs` `declared_size` | yes |
| listing `EQU` rows | `resolved_equates` → `listing_from_resolved` → `emit_listing` | yes, the defect's row |
| game-constant folds | `harvest_game_constants`' engine seed | yes |
| `p5_constants_flip.rs`, `m1c_vector_table.rs` | direct calls | pass the plain sonic4 profile |

**Byte impact, measured at aeon `ec640bcf`** (CRC32 zlib + size, before = master `6a4480d0`, after =
`df055bd1`, every shape built to one output path so the digest's ROM-path line cannot differ):

| shape | ROM before = after | listing |
|---|---|---|
| `s4` | `91c46c94/820209` | identical except the `DIGEST-ASSEMBLER` revision line |
| `s4.debug` | `8a378de6/846509` | identical except the `DIGEST-ASSEMBLER` revision line |
| `demo` | `1c7a34d3/96863` | identical except the `DIGEST-ASSEMBLER` revision line |
| `demo.debug` | `72e405a5/103185` | identical except the `DIGEST-ASSEMBLER` revision line |
| `stress_evict` | `6e3739dc/846509` | that line, plus `EQU PAGE_FRAMES_CLAMP = $0000000C` → `$00000009` |

The `DIGEST-ASSEMBLER` line names the assembler's own revision, so it moves with any commit and is
not a content change. `repin --check` prints `pins.rs unchanged`; nothing under `golden/`, `pins.rs`,
`repin.toml` or `tests/repin_pins.rs` moved.

**The gate.** `crates/sigil-harness/tests/listing_equ_shape_aware.rs` (runner:
`cargo test --release -p sigil-harness --test listing_equ_shape_aware`, the workspace suite, and the
nightly lane, where it is a `SOURCE_GATES` member in `scripts/nightly_source_gates.sh`)
builds `stress_evict`, reads the one `cmpi.w #imm, d6` inside `Level_LoadArt` out of the image, folds
`constants.emp` under the profile's `shape_defines`, and requires the published `EQU` row, the ROM
immediate and the fold to agree, then checks every other harvested constant with an `EQU` row the
same way. Non-vacuity: the stress fold must differ from the canonical debug fold, or the gate fails
saying the fixture went inert. A build failure or a use site it cannot identify uniquely is a failure,
not a skip. Red against the unfixed code and again against the mutation below (re-pinning the seed
on disk, restored with `git checkout` from the committed fix):

```
-    let defines = shape_defines(profile, aeon)?;
+    let _ = profile;
+    let defines = vec![("STRESS_EVICT".to_string(), 0i128)];
```

```
LISTING-EQU-SHAPE-BLIND: the stress listing publishes `EQU PAGE_FRAMES_CLAMP = $0000000C` but the
ROM from the same build encodes `cmpi.w #$0009, d6` at $00902C.
```

**Aeon side.** `tools/evict_witness.py`'s "go with the ROM" handling stays correct and now sees the
two agree. The aeon row can close once a sigil containing this fix is the installed assembler; that
is aeon's call to make and is not changed here.
