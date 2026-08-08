# `outw` — queue item 2: `out(dN: T)` widths and their adoption

Lane `outw`, sigil branch `outw` (from master `3e0824b1`) + aeon branch `outw`
(from master `d8c93d7`). Contract-verification queue item 2.

## Headline

`[proc.out-unverified]` residue **30 → 16**, not the predicted 15. The extra row
is `Emit_ObjectPieces :: d5`, and it is open **on purpose**: a type would close it
while publishing a byte contract to callers that read a word. That is the parcel's
most useful result and it is a row NOT closed.

Byte bar: **byte-NEUTRAL across all seven targets**, full and anchor, chain 53.

## What the measurement said, step by step

Every number below is a SET DIFF against the previous state, measured on this
branch with `AEON_DIR` pointed at the lane's own aeon worktree, never a count read
off a baseline.

| state | residue | the diff |
|---|---|---|
| both masters, untouched (the seed proof) | 30 | — |
| the width mechanism in, **zero aeon declarations changed** | 29 | `EntityWindow_EntryForSection :: d0` REMOVED |
| aeon adoption, 12 registers across 11 procs | **16** | 13 more removed, none added |

The middle row is the mechanism's own non-regression proof: with no declaration
written, bare `out(rN)` still means 32 bits and nothing moves — except the ONE
proc whose author had already written `out(d0: EntryRef)` and been ignored by the
verifier since the day it was declared.

The 16 that remain are exactly the census's non-width set (8 `probe_core` d1/d2 +
3 S4LZ chain + 4 `DrawRings`/`InsertSpriteMasks`) plus the deliberate
`Emit_ObjectPieces :: d5`. **No row a type could reach is left unreached by
accident.**

## Re-derivations that corrected what I was handed

**The grammar already shipped.** The brief said "add `out(dN: type)`". `ast.rs`
has carried `out_types` since G5 and two corpus procs already declared one. The
work was a CONSUMER, not a surface. Confirmed before building on it.

**But it did not compose.** `out(d0: u16 if eq)` did NOT parse: the typed arm
consumed the type and returned to a loop expecting `,` / `/` / `)`, so the `if`
was a syntax error. That is the parcel's only grammar change.

**The census's `ext.w d0` claim is wrong, and so was my correction of it until I
measured.** The overseer flagged that there is no `ext.w d0` at a partial-height
return. Measured rather than read: with `out(d0)` bare and ONLY the `.full_back`
tail widened (`add.w`/`neg.w` → `.l`), all four `Collision_Probe*::d0` rows
vanish. So `.full_back` is the sole failing return; the primary path's
`moveq #16, d0` + `sub.w d3, d0` was never failing. That measurement doubles as
the proof of a design ruling — if a later `.w` write retracted the `moveq`'s
full-width credit, the primary path would fail too and widening `.full_back` alone
could not have closed the rows.

**A contract type and its implementors can disagree with no diagnostic.**
Measured: reverting `type SensorProbe`'s `out` to bare while the four typed procs
stayed typed produced zero new diagnostics — residue 16, D1c 20, warn tier
`module.path-mismatch 9`, and the build's own contract gate silent. Ledgered.
`SensorProbe` is updated in lockstep by hand because nothing will do it for us.

**A stale comment retired.** `math.emp`'s `GetSineCosine` header said "Register
OUTPUTS can't carry type annotations yet — the out-typing ask is ledgered with
this ruling pre-made". They can, and the ruling is now spent.

## The RMW question, decided by measurement

Both rules were implemented and the corpus measured under each, with adoption in
place:

| production rule | residue | rows closed |
|---|---|---|
| pure width — a write of the declared width or wider | **16** | all a type can reach |
| defining writes only below `.l` | **20** | 5 fewer |

The five rows that need RMW credit, by set diff: `Collision_Probe{Down,Left,
Right,Up} :: d0` (`add.w`/`neg.w`) and `Emit_ObjectPieces :: d5` (`addq.b`).

Ruled **pure width**, and the argument outweighs the count: the `.l` rule has
always credited RMW, so a defining/RMW split below `.l` would make "produce" mean
two different things depending on the declared type, stricter exactly where the
claim is weaker. The brief's guard ("never let RMW alone count as production from
nothing") is kept by the LATTICE, not a mnemonic list — `addq.b` credits one byte
and can never manufacture the claim above it. Full reasoning in
`specs/2026-08-07-out-type-width-design.md`.

## The per-site caller read-width sweep

The only thing making adoption sound — no caller-side out-read-width check exists
in the toolchain. Every call site of every touched proc, with the width it reads.

| site | adopted | body produces | callers, and the width each reads |
|---|---|---|---|
| `Collision_GetType :: d0` | `u8` | `.w` (`lsr.w #3, d0` then `move.b (a0,d1.w), d0`; the `.cgt_air` return is `moveq`, `.l`) | `player_sensors.emp:175` `move.b d0, d2` — **.b** |
| `Tile_Cache_GetTile :: d2` | `u16` | `.w` (`move.w (a0,d1.w), d2`) | **none — zero call sites** (dead export; ledgered) |
| `GetSineCosine :: d0` | `fixed<8,8>` | `.w` (`move.w Sine_Table(pc,d0.w), d0`) | `player_ground.emp:126` `asr.w #3, d0`; `:387` `move.w d0, d1`; `:627` `muls.w d2, d0`; `:811` `muls.w d2, d0`; `test_parent.emp:121` `asr.w #3, d0` — **all .w** |
| `GetSineCosine :: d1` | `fixed<8,8>` | `.w` | `player_ground.emp:626` `muls.w d2, d1`; `:810` `muls.w d2, d1`; `test_parent.emp:122` `neg.w d1` — **all .w** |
| `Section_RedrawPlanes :: d7` | `u16` | `.w` (`move.w Cache_Head_Col, d7`) | `section.emp:469` `move.w d7, Section_Right_Col_Written` — **.w** |
| `EntityWindow_DeriveWindow :: d2` | `u8` | `.w` (`move.w d4, d2`) | `entity_window.emp:724` `move.b d2, Entity_Window_Anchor`; `:891` `cmp.b Entity_Window_Anchor, d2` — **both .b** |
| `EntityWindow_DeriveWindow :: d3` | `u8` | `.w` (`move.w d5, d3`) | `entity_window.emp:725` `move.b d3, …`; `:893` `cmp.b …, d3` — **both .b** |
| `EntityWindow_DeriveWindow :: d4` | `i16` | `.w` (`move.w Camera_X, d4` / `moveq #0, d4` on the clamp arm; the meet is `.w`) | `entity_window.emp:729` `asl.w d0, d4`, `:730` `move.w d4, Entity_Window_OriginX` — **.w**. Site `:890` does not read it. |
| `EntityWindow_DeriveWindow :: d5` | `i16` | `.w` | `entity_window.emp:731` `asl.w d0, d5`, `:732` `move.w d5, Entity_Window_OriginY` — **.w**. Site `:890` reloads d5 from RAM. |
| `Collision_Probe{Down,Up,Left,Right} :: d0` | `i16` | `.w` on the `.full_back` return; `.l` on the other three | `player_sensors.emp:245` `move.w d0, -(sp)`, `:253` `cmp.w d0, d5` (both via the indirect `jsr (a2) as SensorProbe`); `:539` `cmpi.w #LEDGE_NO_GROUND, d0` — **all .w** |
| `EntityWindow_EntryForSection :: d0` | `EntryRef` (already declared; now honoured) | `.w` (`move.w d1, d0`) / `.l` (`moveq #-1, d0`) | `rings.emp:316`, `entity_window.emp:269`, `:1480`, `:1579` — all `tst.w d0` / `bmi`, then `EntityLoaded_Clear (d0: EntryIndex)` reads `lsl.w #5, d0` — **all .w** |

**Two sites where the two-sided test refused an adoption:**

| site | why not |
|---|---|
| `Emit_ObjectPieces :: d5` | body `.b` (`addq.b #1, d5`), **nine caller reads at `.w`** (`sprites.emp:259, 277, 336, 395, 453, 475, 484, 488` + `:755` in `InsertSpriteMasks`). ESCALATED — see below. |
| `Section_RedrawPlanes :: d5` | its bare claim ALREADY verifies at 32 bits (`move.l Camera_X, d5`). Typing it would trade a machine-checked claim for a weaker one and close nothing. **Never type a register whose bare claim already verifies.** |

### ESCALATION: the sprite counter's contract is narrower than its use

`Emit_ObjectPieces` and `InsertSpriteMasks` advance the running VDP sprite total
with `addq.b #1, d5`; nine call-site reads take it at `.w`. The reads are correct
TODAY only because `Sprite_Render`'s `moveq #0, d5` zeroes all 32 bits before the
loop and `MAX_VDP_SPRITES = 80` keeps the byte from wrapping. That is a
CALLER-side invariant. The callee's `out(d5)` cannot state it, `out(d5: u8)` would
contradict it, and nothing in the toolchain checks it.

`out(d5: u8)` was therefore **not adopted** and the row stays in the baseline with
this reason attached. The repair runs the other way: widen both increments to
`addq.w #1, d5` — the same 2-byte encoding and the same 4 cycles on a data
register, so it costs nothing — then adopt `out(d5: u16)` at both procs. It is
byte-CONTENT-changing, so it belongs to a byte-changing parcel, not this one.
Ledgered with that kill condition.

## The newtype-narrowing decision

**Newtypes narrow, transitively.** Set-diff evidence, with no aeon declaration
touched: narrowing ON removes exactly `EntityWindow_EntryForSection :: d0` (30 →
29) and adds nothing.

The forcing argument is stronger than the yield. A typed out slot holds ONE
declaration. With newtypes not narrowing, the only way to give that row a width
would be replacing `EntryRef` with `i16` — deleting the domain type that four call
sites' `assume_some! d0, EntryIndex` depends on and that the niche-option check
polices. A design where stating a width costs you your domain type is a trap.

The risk was measured, not waved off: narrowing can weaken a claim that already
verifies at 32 bits. Exactly one corpus site is exposed — `Section_FlatIDXY :: d0`
(`moveq #0, d0` under a `SectionId` = word claim). It stays verified and the
weaker claim is the honest one.

`EntryRef`'s `? -1` sentinel was treated with the care the earlier niche-overlap
catch earned it: its two arms are `moveq #-1` (a long) and `move.w`, and all four
callers discriminate with `tst.w` / `bmi`, which reads `$FFFF` against `$000X`.
The niche is a word-level niche; the width comes from `EntryIndex`'s `i16`, not
from the sentinel's magnitude.

## Named mutants — all 15 run, all RED

The standing bar, run rather than asserted. Script:
`scratchpad/mutants.py` (applies, runs, requires FAILURE, reverts).

| mutant | test | result |
|---|---|---|
| `required` always answers `L` | `a_word_write_proves_a_word_claim_and_not_a_bare_one` | RED |
| drop the width comparison in `check_return` | `a_byte_write_does_not_reach_a_word_claim` | RED |
| `have < need` → `have != need` | `a_wider_write_satisfies_a_narrower_claim` | RED |
| `join` takes the wider width | `a_merge_charges_the_narrower_incoming_width` | RED |
| `produce` overwrites instead of widening | `a_later_narrower_write_does_not_retract_a_wider_production` | RED |
| callee credit uncapped at `L` | `callee_out_credit_is_capped_at_the_callees_declared_width` | RED |
| the `TailOut` arm alone uncapped | `tail_target_credit_is_capped_at_the_targets_declared_width` | RED |
| the `FallOff` arm alone uncapped | `falls_into_credit_is_capped_at_the_successors_declared_width` | RED |
| a newtype does not narrow | `a_newtype_narrows_to_its_underlying_type_transitively` | RED |
| a newtype does not narrow (niche chain) | `a_niche_option_narrows_to_its_payload_width` | RED |
| `Type::Refined` does not narrow | `an_inline_where_refinement_narrows_to_its_inner_type` | RED |
| `fixed<I,F>` does not narrow | `a_fixed_point_type_narrows_by_its_bit_count` | RED |
| an unknown type defaults to `B` | `an_underivable_out_type_keeps_the_bare_32_bit_claim` | RED |
| the parser drops the `if` clause | `a_typed_out_composes_with_the_conditional_form` | RED |
| the parser drops the cond entry | `a_typed_conditional_out_is_still_unobligated_on_the_not_cc_return` | RED |

### The bar earned its keep — one mutant SURVIVED on the first pass

`refined-does-not-narrow` ran GREEN against
`a_niche_option_narrows_to_its_payload_width`. Cause: `newtype Idx = i16 where
0..3` parks its range in `NewtypeDecl.refine`, NOT as a `Type::Refined`, so
`out_width_of`'s refinement arm was **never reached by any test** and the doc
comment claiming that mutant was **false**. Fixed both ways: the niche test's
mutant claim now names the mutant it actually catches, and a new gate
(`an_inline_where_refinement_narrows_to_its_inner_type`, over an inline `u16 where
0..3`) covers the arm and goes RED under the refinement mutant. Fourth instance of
the family; the first where the false claim was in a comment I had just written.

Every "must verify" assertion asserts membership in the VERIFIED map rather than
the absence of a firing — a non-vacuity guard, since "no firing" is also what a
proc the walk never parsed produces. `assert_subjects` guards the firing side the
same way.

## Gates (all own-run, on this branch, foreground)

- **Byte bar — SEVEN targets, all byte-IDENTICAL to the chain-53 tip**, full and
  anchor: `s4 3192f989/411429 · 0631b582`; `s4.debug 557c98d8/423831 · b1d0d921`;
  `demo 80b9531c/91330 · c24cf9af`; `demo.debug 8f4be1a1/94031 · 52ae6971`;
  `config_a cc6627c9/424209 · f281c69f`; `config_b 6ab35984/301467 · 61ff4b23`;
  `lean 197a2645/379350 · f642d9af`. Contract declarations emit nothing, as
  expected. The seed was PROVEN against these same bars before any edit.
- **Strict** `cargo test --workspace --release` (`SIGIL_STRICT_GATE=1`, `AEON_DIR`
  = the lane's aeon worktree): **3532 passed / 0 failed / 4 ignored**, 313 suites,
  zero non-`ok` results. Closing arithmetic: 3532 + 4 = **3536** = the branch's own
  `#[test]` total. Master was 3517/0/4 = 3521; the +15 is this parcel's new file.
- **`refreeze --check`**: OK (tip `slide-fixture`, chain len 53).
- **clippy** `--workspace --release --all-targets -- -D warnings`: CLEAN.
- **Warn tiers**: the firing lint-id SET is identical across all seven targets
  before and after — `module.path-mismatch 9` ×9, `+ proc.undeclared-fallthrough
  5, proc.out-unwritten 2, proc.clobber-undeclared 1` ×5, and the
  `undeclared-fallthrough 6` variant ×4. No deltas, deliberate or otherwise.
- **D1c** unchanged at 20 (plain family) and `[proc.out-cond-survives-unverifiable]`
  at 0, both measured beside the residue.

## Per-pass findings

### Step 3 — retrospect (what the port surface taught)

- The `out` clause had a consumer-shaped hole for weeks: a declaration the parser
  stored, the type-slice read, and the verifier ignored. **A grammar that parses
  is not a feature; the question is always who READS it.** Two corpus authors had
  already written the annotation and got nothing for it.
- The census's "the width gap is the dominant cause" framing was already corrected
  to "exactly half" by item 1. This parcel adds a second correction: of the half a
  type can reach, one row should NOT be reached by one. The reachable set and the
  set worth closing are different sets.
- The `[proc.out-unverified]` diagnostic now names the produced width and the
  claimed width. That turned every remaining width row into a self-describing
  measurement — the per-site "body produces" column above was READ OFF THE
  DIAGNOSTIC, not derived by hand, and it is what exposed `Collision_GetType`
  producing `.w` where its doc says `.b`.

### Step 5 — optimize / engine-side (recorded, not done here)

- `addq.b #1, d5` → `addq.w #1, d5` at both sprite-counter sites: same size, same
  cycles, and it makes the contract match nine existing reads. Byte-changing, so
  it is a parcel of its own. This is the highest-value item the lane produced and
  it is deliberately not in it.
- `Collision_GetType` leaves the tile column in d0's high byte under a `u8`
  result. Harmless (the sole caller reads `.b`) but it means d0's word is
  semantically junk; a `moveq #0, d0` before the fetch would make the word claim
  true, at 2 bytes.
- `Tile_Cache_GetTile` has zero callers. Its adopted type rests on a doc comment.
  Delete or use.

### Neither bucket

- **Blocks entered only by a local `bsr`/`jbsr .label` are invisible to
  `out_verify`** (independently proven by the `probe2` lane; corroborated here —
  `probe_core`'s `.cell` never contributes a return). The polarity is fail-safe:
  it over-fires and can never bless a false `out`. It bounded nothing in this
  parcel because every row in scope failed at a return the CFG does reach, but it
  is why a future width row might not move.
- **D1b's must-def is width-blind on BOTH sides** — `written_names` credits a
  `.b` write as a definition, and the callee-out credit ignores declared width.
  `conditional_out_edge_credits` now carries widths for the out-verifier while
  must-def reads its keys, and that asymmetry is deliberate: honouring a width at
  the callee credit alone would make a call stricter than the identical inline
  write beside it. Ledgered with the real close (a width-carrying must-def
  compared against declared PARAM widths, using the map this parcel already
  builds).
- **A contract type's `out` list is unenforced in both directions.** Hence
  `collect_out_widths` walks procs and externs only: feeding an unenforced
  declaration into the credit map would let a wrong type widen a claim silently.

## Files

**sigil** — `out_verify.rs` (the width lattice, `OutWidth`/`OutWidths`, the three
capped transfer-out charges), `flag_check.rs` (`conditional_out_edge_credits`
carries widths), `calls.rs` (must-def reads the keys), `corpus_contracts.rs`
(`collect_out_widths` / `out_width_of` / `collect_newtype_underlying`),
`parser.rs` (`out(dN: T if cc)`), `contract_baseline.rs` (30 → 16 rows + the
corrected header), `tests/out_width.rs` (new, 15 gates), `tests/out_verify.rs`
(arity), the design note, four gap-ledger rows.

**aeon** — `collision_lookup.emp`, `tile_cache.emp`, `math.emp` (+ the stale
comment), `section.emp`, `entity_window.emp`, `player_sensors.emp` (4 procs + the
`SensorProbe` contract type). `sprites.emp` is UNCHANGED — the adoption was made
and then reverted on the two-sided test.
