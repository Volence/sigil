# The `falls_into` closure edge, and the baseline movement it caused

Parcel note, branch `parcel/falls-into-edge` off master `3b454827`, closing
`CLOSURE-MISSES-FALLS-INTO-EDGE`. The change is three lines of mechanism and one
frozen-baseline movement; the baseline movement is the part that needed evidence,
because hand-editing a frozen pin looks exactly like hiding a defect.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | this worktree, `CARGO_TARGET_DIR=.target-parcel`, release |
| reference tree | `/home/volence/sonic_hacks/.aeon-sigil-ref`, aeon `ec640bcf`, READ-ONLY, no build run in it |
| before/after measurement | a throwaway probe example dumping every gated firing family per shipped shape; its numbers are a text diff of two runs, not a reading |
| suite | `SIGIL_STRICT_GATE=1 AEON_DIR=… cargo test --release --workspace --no-fail-fast` |

## The mechanism

`effective(P)` came from `localWrites(P)`, the CALL and TAIL mnemonics found in
P's evaluated body, and its indirect bounds. A `falls_into SUCC` declaration
reaches control into SUCC with **no transfer instruction**, so it contributed
nothing: the successor's writes were absent from the falling proc's effective set,
and absent from every caller's.

`ProcNode::falls_into` now carries the declaration and the fixpoint charges it
like a tail transfer, with the same hole treatment for an unresolvable successor.
The Z80 seam (`seam1.rs`) had modelled the same edge since it was written, by
pushing the successor into `direct_callees`; it now uses the field, so one edge has
one spelling.

**The asymmetry, in the corpus's own words.** `Player_SensorFloor` ends
`jbra Player_SensorSurface`. `Player_SensorCeiling` declares
`falls_into Player_SensorSurface`. One transfer spelled two ways, and the closure
read the first as clobbering the shared body's ten registers and the second as
clobbering `d6/d7`. That pair is the corpus gate's named witness.

## What the row predicted, and what was actually there

The queue row's case rested on two halves. One of them has no instance today, and
saying so is the point of measuring before fixing.

- **The census half (tidying) landed in full.** Over-declaring procs per shape:
  sonic4 plain 90 to 66, sonic4 debug 71 to 47, demo plain 101 to 77, demo debug
  82 to 58, config_a 71 to 47, config_b 91 to 67, lean 90 to 66. The
  `falls_into` artifact class goes 16 to 0 in every shape (the row's figure of 11
  is the same population under `clobber_payoff`'s bucket ORDER, which counts a
  proc whose effective set was EMPTY in the comptime-empty bucket first; 5 of the
  16 were empty-bodied by that reading). The drop is 24 procs rather than 16
  because a caller of a falling proc gains its successor's registers too: the
  residue after both artifact classes goes 60 to 52 on sonic4 plain.
- **The destructive half is HYPOTHETICAL in this corpus.**
  `[proc.clobber-undeclared]` closure firings are **0 before and 0 after, on all
  seven shapes**. No proc in the corpus falls into a successor that writes more
  than the head declares: every falling head already declares at least its
  successor's set. The suppressed-firing mechanism is real and is now proved by a
  test, but it has no instance, and the row's framing of it as the urgent half does
  not survive the measurement. What the edge does buy is that the firing can no
  longer be suppressed when someone writes that pair next.

## The baseline movement, row by row

`[call.live-clobbered]` (D1c) moved, **identically on all seven shapes** (so every
row belongs in the shape-invariant `D1C_BASELINE`, not in a family extra): one row
relocated, six appeared. There is NO generator for `contract_baseline.rs`; the
rows were taken from the machine before/after diff and the file was **edited by
hand**.

All seven are one class. `Player_SensorFloor`'s prose header states the family's
convention, `Out: d0.w dist, d1.b angle, d2.b attr` against
`clobbers(d0-d7/a1-a2)`, and `Player_SensorCeiling` says "same contract". The
result registers are **not declared `out(...)`**, so D1c reads the post-call read
of `d0`/`d1` as a destroyed held value rather than as the callee's product. Three
rows of exactly this class were already frozen (`Ground_Move_Cap @
Player_SensorWallDir :: d0`, `PState_Spindash @ Player_SensorFloor :: d0/d1`,
`TestPlayer_Main @ Player_SensorFloor :: d0/d2`). KILL CONDITION: declaring the
sensors' real `out(...)` surface dissolves all of them at once, and that is an
aeon contract change.

### GONE: `Air_Collide @ Air_WallProbeRight :: d1`

The row the ratchet exists to interrogate. It did not vanish, it **moved to the
later call on the one path that reaches the read**, and the (proc, register)
multiset count is unchanged: `Air_Collide :: d1` is one row before and one row
after.

Derivation, by enumeration of `Air_Collide`'s body:

- d1 is defined at `move.w d3, d1` (the `|x_vel|` compare input, before any call);
- the only read of d1 after any `Air_WallProbeRight` call is `move.b d1, d3` in
  `.mostly_up` (the ceiling angle). The other two call sites tail out of the body
  (`jbra Air_FloorLandFlat` / `Air_FloorLandBanded`) or pass through
  `Air_CeilingBump`, which clobbers d1 and kills the path;
- the sole path to that read runs through `jbsr Player_SensorCeiling`. With the
  edge modelled, that callee carries d1 (it falls into `Player_SensorSurface`,
  which writes d0-d5), so D1c's forward walk stops at it: the value the read
  observes is the sensor's angle, produced two instructions earlier.

So the row moved from a call that destroys a value nobody reads to the call that
destroys and then reproduces the value that IS read. The analysis got sharper at
that site, not weaker.

### NEW, six rows

| row | why it fires now |
|---|---|
| `Air_Collide @ Player_SensorCeiling :: d0` | `tst.w d0` right after the call reads the produced DISTANCE (the call site's own comment says "d0 dist, d1 angle"). `Player_SensorCeiling`'s body is two `moveq`s and a fall-through, so its effective set read `{d6,d7}` until the edge was charged |
| `Air_Collide @ Player_SensorCeiling :: d1` | `move.b d1, d3 // ceiling angle` reads the produced ANGLE. Same suppressed edge; this is where the GONE row above now sits |
| `Air_WallProbeLeft @ Player_SensorWallAt :: d0` | the probe X goes in via `move.w x_pos(a0), d0` (an undeclared INPUT) and `tst.w d0` after the call reads the produced DISTANCE, which the proc's header documents. `Player_SensorWallAt` is three instructions and a fall-through; its effective set read `{d2}` |
| `Air_WallProbeRight @ Player_SensorWallAt :: d0` | the mirror of the row above, same instruction shape |
| `PState_Ground @ Player_SensorCeiling :: d0` | the jump gate reads `cmpi.w #PHYS_JUMP_HEADROOM, d0` immediately after the call: the produced CLEARANCE |
| `PState_Roll @ Player_SensorCeiling :: d0` | the same gate, same instruction, in the rolling state |

Every one of the six is the falling twin of a site that already fired at the
TAILING twin. That is the parcel in one sentence: the gate was blind to half the
corpus's sensor calls because of how the transfer was spelled.

## The dead-save direction, checked in the direction the baseline's comment names

The controller's hypothesis was that modelling the edge GROWS an effective set,
which justifies MORE caller saves and so can only produce FEWER dead-save reports,
never more; a dead-save report APPEARING would be the load-bearing-save case and a
stop condition. Tested rather than adopted:

**`[proc.dead-save]` is unchanged, by count AND by row, on every shape** (1 on the
plain shapes, `TestChurnObj_Main :: A0 around AllocDynamic`; 10 on the debug
shapes, that row plus `Canopy_Persist`'s nine). Nothing appeared and nothing was
lost. The hypothesis is correct in its direction and its magnitude here is zero:
the procs whose effective sets grew are not the procs that bracket a call with a
save. Monotonicity is the reason it can only go that way: the edge is one more term
in a union, and `find_dead_saves` reports a save as dead only when every bracketed
callee PRESERVES the register.

Every other gated family is unmoved: closure firings 0, word-facet 0, out-verify
0, inout 0, survives 0, input-undefined 0, flag 0, unresolved callees 0, dropped
instructions 0, comptime-unresolved 0.

## The pins, and what each one would fail on

Two targets, both in the standing runner
(`SIGIL_STRICT_GATE=1 AEON_DIR=… cargo test --release --workspace`):

- `sigil-frontend-emp/tests/contract_closure.rs` (pure fixpoint, five tests): the
  union, the TAIL-TRANSFER TWIN equality (two spellings of one transfer must
  agree), transitive propagation to a caller, a ⊤ successor staying ⊤, and an
  unresolvable successor surfacing as a hole;
- `sigil-cli/tests/contract_closure_corpus.rs ::
  a_declared_fall_through_is_a_closure_edge_on_every_shape`: over every 68k pair in
  the real corpus, `effective(P) ⊇ effective(SUCC)`, plus the named
  Floor/Ceiling twin equality, with a population floor and a non-vacuity bar on the
  equality (agreeing at a SMALL set would mean both lost the successor).

Red-first, with the mutation applied to the SUBJECT and never to a checker, both
restored by `git checkout HEAD -- <path>` from commit `ce19bb46`:

1. the fixpoint resolves the successor and then DISCARDS it (the pre-parcel
   reading): 4 of the 5 unit tests red, plus both corpus tests, naming
   `EntityWindow_Init` missing `a4` from its successor;
2. the hole walk stops chaining `falls_into` (the first mutation leaves this one
   green, which is why it gets its own): `absent_falls_into_successor_is_unresolved`
   red, the other four green.

## What in the brief turned out to be wrong

1. **The two halves are not separable in code.** One edge produces both, so the
   controller's sequencing ("land the second, leave the rows") had nothing to act
   on; and the baseline file's own instruction is to adjudicate rows in the SAME
   commit as the change that moved them, which forbids the split that would have
   left the suite red between two commits.
2. **The destructive direction the row was booked for did not materialise.** The
   missed firing was predicted in `[proc.clobber-undeclared]`, which stayed at 0.
   The firings that DID appear are in `[call.live-clobbered]`, a different family,
   and they are false positives of a known class rather than caught bugs. The
   parcel is worth its cost for the asymmetry it removes and the test that now
   holds it, not for a bug it found.
3. **The controller's dead-save hypothesis holds**, and a `GONE` row did appear
   anyway, which is worth separating: it is not the narrowing the baseline's comment
   warns about but a kill-path shift inside D1c's own forward walk, and it is
   visible as such only because the same (proc, register) still carries one row.
