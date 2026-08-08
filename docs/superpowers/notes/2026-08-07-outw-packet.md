# `outw` — queue item 2: `out(dN: T)` widths and their adoption

Lane `outw`, sigil branch `outw` (from master `3e0824b1`) + aeon branch `outw`
(from master `d8c93d7`). Contract-verification queue item 2.

## Headline

`[proc.out-unverified]` residue **30 → 16**, not the predicted 15. The extra row
is `Emit_ObjectPieces :: d5`, and it is open **on purpose**: a type would close it
while publishing a byte contract to callers that read a word.

Byte bar: **byte-NEUTRAL across all seven targets**, full and anchor, chain 53.

## What the measurement said, step by step

Every number below is a SET DIFF against the previous state, measured on this
branch with `AEON_DIR` pointed at the lane's own aeon worktree.

| state | residue | the diff |
|---|---|---|
| both masters, untouched (the seed proof) | 30 | — |
| the width mechanism in, **zero aeon declarations changed** | 29 | `EntityWindow_EntryForSection :: d0` REMOVED |
| aeon adoption, 14 registers across 9 procs | **16** | 13 more removed, none added |

The middle row is the mechanism's own non-regression proof: with no declaration
written, bare `out(rN)` still means 32 bits and nothing moves — except the ONE
proc whose author had already written `out(d0: EntryRef)` and been ignored by the
verifier since the day it was declared.

**The adoption arithmetic, stated so it reconciles.** 14 typed registers across 9
procs are written; 13 of them close a residue row. The 14th row closes with NO
declaration touched (`EntityWindow_EntryForSection :: d0`, via newtype
narrowing), and one written declaration closes no row at all
(`Section_RedrawPlanes :: d5`, which already verified). 13 + 1 = the 14 rows
removed.

The 16 that remain are exactly the non-width set (8 `probe_core` d1/d2 + 3 S4LZ
chain + 4 `DrawRings`/`InsertSpriteMasks`) plus the deliberate
`Emit_ObjectPieces :: d5`. No row a type could reach is left unreached by
accident.

## Re-derivations that corrected what I was handed

**The grammar already shipped.** `ast.rs` has carried `out_types` since G5 and two
corpus procs already declared one. The work was a CONSUMER, not a surface.

**But it did not compose.** `out(d0: u16 if eq)` did NOT parse: the typed arm
consumed the type and returned to a loop expecting `,` / `/` / `)`. That is the
parcel's only grammar change.

**The census's `ext.w d0` claim is wrong.** Measured: with `out(d0)` bare and ONLY
the `.full_back` tail widened (`add.w`/`neg.w` → `.l`), all four
`Collision_Probe*::d0` rows vanish. So `.full_back` is the sole failing return;
the primary path's `moveq #16, d0` + `sub.w d3, d0` was never failing. That
doubles as proof of a design ruling — if a later `.w` write retracted the
`moveq`'s full-width credit, widening `.full_back` alone could not have closed the
rows.

**A contract type and its implementors can disagree with no diagnostic.**
Measured: reverting `type SensorProbe`'s `out` to bare while the four typed procs
stayed typed produced zero new diagnostics — residue, D1c and the warn tier all
unchanged, and the build's own contract gate silent. Ledgered. `SensorProbe` is
updated in lockstep by hand because nothing will do it for us.

**A stale comment retired.** `math.emp`'s `GetSineCosine` header said "Register
OUTPUTS can't carry type annotations yet — the out-typing ask is ledgered with
this ruling pre-made". They can, and the ruling is now spent.

## RMW: measured, not tasted

Both rules implemented, corpus measured under each, against the parcel as it
stands:

| production rule | residue | |
|---|---|---|
| pure width — a write of the declared width or wider | **16** | — |
| defining writes only below `.l` | **20** | 4 rows re-open |

**The set diff is exactly four rows** — `Collision_Probe{Down,Left,Right,Up} ::
d0`, which reach `.full_back` through `add.w d3, d0` / `neg.w d0`.

A fifth corpus production is RMW-only (`Emit_ObjectPieces :: d5`'s `addq.b`) but
is NOT in the diff, because its type was refused and the row is open under both
rules. Stated separately rather than folded in: a set diff reports what MOVED, and
a row that never closes cannot move. (An earlier revision of this packet said
"five, by set diff" and was wrong on both halves of that phrase.)

Ruled **pure width**, and the argument outweighs the count: the `.l` rule has
always credited RMW, so a defining/RMW split below `.l` would make "produce" mean
two things depending on the declared type, stricter exactly where the claim is
weaker.

## SOUNDNESS — what the panel found and what the repair was

**Forms that do not cover their own operand size were credited at that size.**
`produced_regs` read the width off the CodeItem, and `ext`/`tas`/`bset`/`bclr` are
all `writes_last_operand` forms. Executed differential, confirmed independently
before repairing:

| body | before | after |
|---|---|---|
| `out(d0: u8) { ext.w d0 }` | VERIFIED | fires |
| `out(d0: u16) { ext.w d0 }` | VERIFIED | fires |
| `out(d0) { ext.l d0 }` | VERIFIED (**pre-existing on master**) | fires |
| `out(d0: u8) { tas.b d0 }` | VERIFIED | fires |
| `out(d0: u8) { bset.b #1, d0 }` | VERIFIED | fires |
| `out(d0) { jbsr Byte; ext.l d0 }` | VERIFIED | fires |
| `out(d0) { move.b; ext.w; ext.l }` | VERIFIED | VERIFIED (correct) |
| `out(d0: u8) { seq.b d0 }` | VERIFIED | VERIFIED (correct) |

`ext.w` sign-extends bits 0-7 INTO bits 8-15 — it never writes bits 0-7, which are
the entire content of a `u8` claim. The single-bit forms write one bit. **Scope,
stated precisely: the `.l` forms were mis-credited on master too, so this widened
a known family at u8/u16 and the repair closes both halves.**

Two corrections to the report I was handed, re-derived rather than accepted:
`bset`/`bclr` only mis-credit when written with an EXPLICIT size (a bare
`bset #1, d0` carries no size and already produced nothing), and **`Scc` is not
in the family** — `seq.b d0` writes all eight bits ($00 or $FF) and produces a
byte exactly as `move.b` does. A rule that swept the whole "sets bits" family
would have been wrong about it, so `writes_partial_bits` names four mnemonics and
a mutant pins the exclusion.

**The repair.** `ext` is modelled as a PROMOTION — it raises an existing
production one step (`.w` needs a produced byte, `.l` needs a produced word) and
makes none on its own. `tas`/`bset`/`bclr`/`bchg` produce nothing. This is also
what closes the credit-laundering hole: a correctly-capped BYTE of callee credit
followed by `ext.l` is no longer a long.

**The design note's claim that the lattice alone held the "no production from
nothing" guard was FALSE and is corrected there.** The lattice holds it for the
RMW case; covering-your-own-size is a separate obligation and it was missing.

**Corpus impact: none.** Residue before the fix 16, after the fix 16, same set.
Verified myself rather than relying on the "not live today" assessment.

## ADOPTION — the two-sided test, and where it changed a verdict

A type is adopted only when it is BOTH no wider than the body provably produces
AND no narrower than any caller reads. Where the two disagree the row stays OPEN.

**`Section_RedrawPlanes :: d5` — the refusal that was wrong, now adopted.** I had
left it bare under a corollary "never type a register whose bare claim already
verifies". Re-derived at the source: `move.l Camera_X, d5` / `swap d5` /
`lsr.w #3, d5` leaves d5 = `[camera sub-pixel fraction : start_world_col]`, and
the sole caller reads `move.w d5, Section_Left_Col_Written`
(`section.emp:473`). The 32-bit claim verifies on a WIDTH credit over 16 bits of
garbage. The tell was inside my own declaration — `out(d5, d7: u16)`, two
registers documented identically and read identically, of which only the one in
the residue got typed. **Residue membership had decided the adoption instead of
the two-sided test.** Now `out(d5: u16, d7: u16)`, with the header stating that
nothing above the word is a result.

The corollary is narrowed to what it should always have said: **never type a
register whose bare claim already verifies AND whose full width is a result.**

**`Emit_ObjectPieces :: d5` — the refusal that stands**, independently
re-derived. Body produces `.b`; call sites read `.w`. Correct today only via a
caller-side invariant (`moveq #0, d5` before the loop, capped at
`MAX_VDP_SPRITES`) the callee cannot state. Sharpened: the same loop nest compares
the counter at BOTH widths — `sprites.emp:681` `cmpi.b` against `:259`/`:336`
`cmpi.w` — so the mismatch is not merely a contract nicety. The repair runs the
other way (widen the increment; same encoding, same cycles, byte-content-changing
and so out of scope here). Ledgered.

## The per-site caller read-width sweep

Line citations below are machine-derived by grep against the branch, not
transcribed. The only thing making adoption sound: no caller-side out-read-width
check exists.

| site | adopted | body produces | callers, and the width each reads |
|---|---|---|---|
| `Collision_GetType :: d0` | `u8` | `.w` (`lsr.w #3, d0` then `move.b (a0,d1.w), d0`; the `.cgt_air` return is `moveq`) | `player_sensors.emp:175` `move.b d0, d2` — **.b** |
| `Tile_Cache_GetTile :: d2` | `u16` | `.w` (`tile_cache.emp:106` `move.w (a0,d1.w), d2`) | **none — zero call sites** (dead export; ledgered) |
| `GetSineCosine :: d0` | `fixed<8,8>` | `.w` (`move.w Sine_Table(pc,d0.w), d0`) | calls at `player_ground.emp:125, 386, 626, 810` and `test_parent.emp:117`; first reads `:126` `asr.w #3, d0`, `:387` `move.w d0, d1`, `:630`/`:815` `muls.w d2, d0`, `test_parent.emp:123` `asr.w #3, d0` — **all .w** |
| `GetSineCosine :: d1` | `fixed<8,8>` | `.w` | `player_ground.emp:630`, `:815` `muls.w d2, d1`; `test_parent.emp:124` `neg.w d1` — **all .w** |
| `Section_RedrawPlanes :: d5` | `u16` | `.l` by width credit, but only the low word is a result (`swap`+`lsr.w`) | `section.emp:473` `move.w d5, Section_Left_Col_Written` — **.w** |
| `Section_RedrawPlanes :: d7` | `u16` | `.w` (`move.w Cache_Head_Col, d7`) | `section.emp:472` `move.w d7, Section_Right_Col_Written` — **.w** |
| `EntityWindow_DeriveWindow :: d2` | `u8` | `.w` (`move.w d4, d2`) | `entity_window.emp:724` `move.b d2, Entity_Window_Anchor`; `:891` `cmp.b Entity_Window_Anchor, d2` — **both .b** |
| `EntityWindow_DeriveWindow :: d3` | `u8` | `.w` (`move.w d5, d3`) | `entity_window.emp:725` `move.b d3, …`; `:893` `cmp.b …, d3` — **both .b** |
| `EntityWindow_DeriveWindow :: d4` | `i16` | `.w` (meet of `move.w Camera_X, d4` and the clamp's `moveq #0, d4`) | `entity_window.emp:729` `asl.w d0, d4`, `:730` `move.w d4, Entity_Window_OriginX` — **.w** |
| `EntityWindow_DeriveWindow :: d5` | `i16` | `.w` | `entity_window.emp:731` `asl.w d0, d5`, `:732` `move.w d5, Entity_Window_OriginY` — **.w** |
| `Collision_Probe{Down,Up,Left,Right} :: d0` | `i16` | `.w` on the `.full_back` return; `.l` on the other three | see the forwarder note below — **7 direct call sites, every terminal read .w** |
| `EntityWindow_EntryForSection :: d0` | `EntryRef` (already declared; now honoured) | `.w` (`move.w d1, d0`) / `.l` (`moveq #-1, d0`) | `rings.emp:316`, `entity_window.emp:269`, `:1481`, `:1579` — all `tst.w d0` / `bmi`, then `EntityLoaded_Clear (d0: EntryIndex)` reads `lsl.w #5, d0` — **all .w** |

### The probe sweep is a FORWARDER chain, and that is the structural point

Seven direct call sites, not the three an earlier revision of this packet listed:
`player_sensors.emp:243` and `:250` (`jsr (a2) as SensorProbe`, inside
`Player_SensorPair`), `:442`, `:446`, `:450`, `:454` (inside
`Player_SensorWallDir`), and `:539` (`Player_AtLedgeEdge`).

Four of those seven do not READ `d0` at all — `Player_SensorWallDir` contains no
`d0` reference between its header and its `rts`; it FORWARDS the value to its own
caller. **And it declares no `out` whatsoever** (`clobbers(d0-d6/a1)`, no out
clause), as does `Player_SensorPair`. So the typed out dies at the first
forwarder: nothing downstream is checked against `i16`, and the manual sweep is
the entire guarantee.

Terminal reads, followed through: `player_sensors.emp:245` `move.w d0, -(sp)`,
`:254` `cmp.w d0, d5`, `:540` `cmpi.w #LEDGE_NO_GROUND, d0`;
`player_ground.emp:715` `tst.w d0`; `player_air.emp:518` and `:536` `tst.w d0`,
each followed by `dist_to_fix(d0)`. That last one is the only non-obvious read in
the chain and was checked rather than assumed: `dist_to_fix` expands to
`pixels_to_coord` = `swap d0` / `clr.w d0`, which moves the low word up and
CLEARS what the swap brought down — so the surviving value depends only on `d0.w`.
A `.w` read. **`i16` is correct at every site.**

## RULINGS

**C1 — newtype resolution is unscoped and was file-order dependent. Fail-safe
built; scoping ledgered.** The type table is keyed by bare leaf name, matching
every other G5 consumer (`newtype_of` reads `path.segments.last()`), so two
modules declaring one name share a row. Measured before the fix: `analyze([a,b])`
and `analyze([b,a])` disagreed, and the relaxing direction was reachable. **A
colliding name now answers its WIDEST reading** — the only direction that can
over-fire rather than bless — pinned by an order-independent gate that runs the
same corpus in both orders. Full module-scoping is NOT built here: doing it for
widths alone would create a second type authority scoped differently from the one
`[call.slot-type-mismatch]` uses, which is the two-authorities failure the design
exists to prevent. Ledgered with that as the kill.

**C2 — a duplicated proc name can no longer relax a bare out.** Same max-merge,
plus every declared out register is now carried explicitly at its bare width so a
merge can SEE an untyped declaration. Without that, a typed row would stand alone
against an absent one. Duplicate proc names are ill-formed anyway, so this is a
fail-safe — but it was the one construction where writing a type somewhere changed
a bare declaration's verdict somewhere else, and the no-migration property is what
the whole feature rests on. Two order-independent gates.

**C3 — `out(aN: T)` is REFUSED**, with `[proc.out-invalid]` at lowering. Measured
first: `out(a0: u8)` verified vacuously (every 68k address write covers all 32
bits, so the claim cannot be violated) while its bare-claiming caller FIRED — the
declaration's only observable effect was capping callers at a width the hardware
will not produce. A declaration that cannot be wrong and can only over-fire is not
a contract. Refusing beats ignoring because ignoring leaves the author's stated
intent unanswered, and a domain type on an address result has a correct home
already: the pointee, as a param.

## Named mutants — 26, all RED

`scratchpad/mutants.py` applies each, runs its test, requires FAILURE, and
restores by SNAPSHOT (a deletion mutant has no unique text to reverse-patch —
that bit once and stranded the tree mid-run).

| mutant | test | result |
|---|---|---|
| `required` always answers `L` | `a_word_write_proves_a_word_claim_and_not_a_bare_one` | RED |
| drop the width comparison | `a_byte_write_does_not_reach_a_word_claim` | RED |
| `have < need` → `have != need` | `a_wider_write_satisfies_a_narrower_claim` | RED |
| `join` takes the wider width | `a_merge_charges_the_narrower_incoming_width` | RED |
| `produce` overwrites instead of widening | `a_later_narrower_write_does_not_retract_a_wider_production` | RED |
| callee credit uncapped | `callee_out_credit_is_capped_at_the_callees_declared_width` | RED |
| the `TailOut` arm alone uncapped | `tail_target_credit_is_capped_at_the_targets_declared_width` | RED |
| the `FallOff` arm alone uncapped | `falls_into_credit_is_capped_at_the_successors_declared_width` | RED |
| a newtype does not narrow | `a_newtype_narrows_to_its_underlying_type_transitively` | RED |
| a newtype does not narrow (niche chain) | `a_niche_option_narrows_to_its_payload_width` | RED |
| `Type::Refined` does not narrow | `an_inline_where_refinement_narrows_to_its_inner_type` | RED |
| `fixed<I,F>` does not narrow | `a_fixed_point_type_narrows_by_its_bit_count` | RED |
| an unknown type defaults to `B` | `an_underivable_out_type_keeps_the_bare_32_bit_claim` | RED |
| the parser drops the `if` clause | `a_typed_out_composes_with_the_conditional_form` | RED |
| the parser drops the cond entry | `a_typed_conditional_out_is_still_unobligated_on_the_not_cc_return` | RED |
| **`ext` credited at operand size** | `ext_promotes_an_existing_production_and_makes_none` | RED |
| **`ext` credited at operand size (launder)** | `an_ext_after_a_call_cannot_launder_a_capped_credit` | RED |
| **partial-bit writers credited** | `single_bit_writers_produce_nothing_but_scc_produces_its_byte` | RED |
| **`Scc` swept into the partial-bit family** | `single_bit_writers_produce_nothing_but_scc_produces_its_byte` | RED |
| **diagnostic arguments swapped** | `the_width_diagnostic_names_produced_then_claimed` | RED |
| **the `ExternProc` arm deleted** | `an_externs_typed_out_caps_its_callers` | RED |
| **`Section` recursion dropped (widths)** | `a_typed_out_inside_a_section_is_collected` | RED |
| **`Section` recursion dropped (newtypes)** | `a_newtype_declared_inside_a_section_resolves` | RED |
| **newtype collision: first wins** | `a_colliding_newtype_name_resolves_to_the_widest_reading` | RED |
| **bare out regs dropped from the row** | `a_duplicated_proc_name_cannot_relax_a_bare_out` | RED |
| **the address-result refusal deleted** | `a_type_on_an_address_register_result_is_refused` | RED |

Bold rows are the fixup round. Five of them (`diagnostic arguments swapped`, `the
ExternProc arm deleted`, and the three walk-coverage mutants) were GREEN against
the whole strict suite before this round — the code they mutate had no test of any
kind, and the diagnostic one is the sharpest: **the per-site "body produces"
evidence in the sweep above is read off that string**, so a transposition would
have inverted every row of that table while 3532 tests stayed green.

**One mutant SURVIVED in the first round** and produced a real gate.
`refined-does-not-narrow` ran GREEN against the niche-option test, because
`newtype Idx = i16 where 0..3` parks its range in `NewtypeDecl.refine` and NOT as
a `Type::Refined` — so that arm had no test at all and the doc comment claiming
that mutant was false. Both fixed.

Every "must verify" assertion asserts membership in the VERIFIED map rather than
the absence of a firing — "no firing" is also what an unparsed proc produces.

## Gates (all own-run, on this branch, foreground)

- **Byte bar — SEVEN targets, byte-IDENTICAL to the chain-53 tip**, full and
  anchor: `s4 3192f989/411429 · 0631b582`; `s4.debug 557c98d8/423831 · b1d0d921`;
  `demo 80b9531c/91330 · c24cf9af`; `demo.debug 8f4be1a1/94031 · 52ae6971`;
  `config_a cc6627c9/424209 · f281c69f`; `config_b 6ab35984/301467 · 61ff4b23`;
  `lean 197a2645/379350 · f642d9af`. The seed was PROVEN against these same bars
  before any edit.
- **Strict** `cargo test --workspace --release` (`SIGIL_STRICT_GATE=1`,
  `AEON_DIR` = the lane's aeon worktree), with the closing arithmetic.
- **`refreeze --check`** OK (tip `slide-fixture`, chain len 53).
- **clippy** `--workspace --release --all-targets -- -D warnings` CLEAN.
- **Warn tiers**: the firing lint-id SET identical across all seven targets.
- **D1c** unchanged at 20 (plain family); `[proc.out-cond-survives-unverifiable]`
  at 0.

## Residue witnesses

Cleared by RUNNING: `corpus_out_residue_is_the_verified_complement` pins
`Art_Decompress :: a1`, which this parcel does not close. Grepping every closed
row across `crates/*/tests` and `crates/*/src` found no other assertion pinning
one. One correction landed: that test's note on the RETIRED
`Collision_GetType :: out(d0)` witness said it "still passes, and detects
nothing" — it no longer fires at all, so restoring it would be a failing
assertion, and the note says so.

## Per-pass findings

### Step 3 — retrospect

- The `out` clause had a consumer-shaped hole: a declaration the parser stored,
  the type-slice read, and the verifier ignored. **A grammar that parses is not a
  feature; the question is who READS it.**
- The reachable set and the set worth closing are different sets. Of the half of
  the residue a type can reach, one row should not be reached by one.
- **Reading evidence off a diagnostic makes the diagnostic load-bearing.** The
  sweep's "body produces" column came from the new width message, which had no
  test. That is a general trap: a measurement instrument used in an argument needs
  a gate of its own, at the same standard as the thing it measures.
- The width verifier proves which BYTES were written on this pass, never which
  carry the result. `Collision_GetType` returns a `u8` above a stale tile column;
  `Section_RedrawPlanes` returned a "verified" long over a camera fraction. Both
  are honest under the checker and misleading to a reader — which is exactly why
  the two-sided caller test, not the checker, decides an adoption.

### Step 5 — optimize / engine-side (recorded, not done here)

- `addq.b #1, d5` → `addq.w #1, d5` at all FOUR sites (`sprites.emp:583`, `:589`,
  `:760`, `rings.emp:232`): same size, same cycles, and it makes the contract
  match the reads AND removes a genuine two-width comparison in one loop nest.
  Byte-changing, so it is a parcel of its own. Highest-value item the lane
  produced and deliberately not in it.
- `Collision_GetType` could zero d0 before the fetch (2 bytes) and widen to
  `u16`, making the word claim true instead of merely unstated. Ledgered.
- `Tile_Cache_GetTile` has zero callers. Delete or use.

### Neither bucket

- **Blocks entered only by a local `bsr`/`jbsr .label` are invisible to
  `out_verify`.** Fail-safe polarity: it over-fires and can never bless a false
  `out`. It bounded nothing here, but it is why a future width row might not move.
- **D1b's must-def is width-blind on BOTH sides.** `conditional_out_edge_credits`
  now carries widths for the out-verifier while must-def reads its keys, and that
  asymmetry is deliberate: honouring a width at the callee credit alone would make
  a call stricter than the identical inline write beside it. Ledgered.
- **A contract type's `out` list is unenforced in both directions**, and a typed
  out dies at the first forwarder that declares no `out`. Both are why
  `collect_out_widths` walks procs and externs only.

## Files

**sigil** — `out_verify.rs` (the width lattice, `OutWidth`/`OutWidths`, the three
capped transfer-out charges, `writes_partial_bits` + `ext_promotion`),
`flag_check.rs` (edge credits carry widths), `calls.rs` (must-def reads the keys),
`corpus_contracts.rs` (`collect_out_widths` / `out_width_of` /
`collect_newtype_underlying` + the collision merge), `parser.rs`
(`out(dN: T if cc)`), `lower/proc.rs` (the address-result refusal),
`contract_baseline.rs` (16 rows + the corrected header), `tests/out_width.rs`
(new, 25 gates), `tests/out_verify.rs` (arity + the retired-limitation note),
`tests/contract_closure_corpus.rs` (the retired-witness note), the design note,
the gap-ledger rows.

**aeon** — `collision_lookup.emp`, `tile_cache.emp`, `math.emp` (+ the stale
comment), `section.emp` (d5 and d7), `entity_window.emp`, `player_sensors.emp`
(4 procs + the `SensorProbe` contract type). `sprites.emp` and `rings.emp` are
UNCHANGED — their adoptions were made and reverted on the two-sided test.
