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

**C1 — a colliding type name is STRICT for its owner and STINGY for its callers.**
The first version of this rule (one width, `max`) was unsound, and the reason is
structural: the width map has TWO consumers with OPPOSITE fail-safe directions.
The proc's own obligation wants the WIDEST reading (over-firing is safe); a
caller's credit wants the NARROWEST (over-crediting is not). **No single width is
safe for both**, so `max` made the wrong answer deterministic rather than removing
it. Executed at the previous tip: `proc P () out(d0) { jbsr C }` fired alone and
VERIFIED once an unrelated module declared the same newtype name — and through an
`extern`, where §3 seeds VERIFIED by axiom with no body to re-prove the inflated
width, the credit had nothing behind it at all.

The claim is now a pair, `OutClaim { strict, credit }`, merged with `max` and
`min` respectively. The safety condition is `credit <= strict`: an out is credited
only once VERIFIED, and verification is charged at `strict`, so every credit is
backed by a proof — except across an extern, which is exactly why the extern case
is one of the three the gate exercises. Module-scoping the type table remains the
real repair and stays ledgered.

**C2 — a duplicated proc name cannot relax a bare out.** Unchanged from round 1
(the same merge, plus every declared out register carried explicitly at its bare
width) and now merging `OutClaim`s rather than widths.

**Stated exactly, because round 1 stated it too broadly:** no migration from the
TYPE facet — with no type written anywhere, every bare `out(rN)` means what it
meant. That is pinned by gates. It is NOT "no bare verdict moves": the
partial-coverage fix moves two, `out(d0) { ext.l d0 }` and
`out(d0) { bclr.l #1, d0 }`, which verified on master and now fire. Both move in
the false-negative-closing direction and neither has a corpus site.

**C3 — REVISED after measuring: refuse only a type that states an IMPOSSIBLE
width, not every type.** My round-1 blanket refusal was too broad. Measured
against the corpus: address PARAMS are typed at ten-plus sites, and
`ZX0_Decompress (a0: *u8, a1: *u8) … out(a0, a1)` and `Art_Decompress` type the
very registers they then declare bare as outs — so a blanket refusal makes the
output-direction dual of an in-use facet unsayable, while `collect_typed_slots`
already accepts `a0`-`a7`. Thirty address-register outs are declared today, none
typed.

So: `out(a0: u8)` is refused (an address write covers all 32 bits, so the claim
cannot be violated and would cap callers below what the hardware produces);
`out(a0: *u8)` is PERMITTED and carries its domain to `[call.slot-type-mismatch]`.
**And the soundness does not rest on that diagnostic:** an address-register out is
pinned to a full long inside `collect_out_widths`, which is the one function every
declaration form flows through. Cost accepted: `out(a0: u8)` is refused rather
than silently pinned, so an author who means something by it is told, and a
per-file lint that is silenced still cannot make the credit unsound.

**The C3 refusal now covers all three declaration forms.** It had lived in the
per-proc contract check, which `extern proc` and `type X = proc (…)` never reach;
executed, both produced ZERO diagnostics, and the extern's harm was live — its
caller fired with `a0 is produced only .b wide` and no body existed to re-prove
it. The rule moved to one `validate_out_types` pass over `Item::Proc` /
`Item::ExternProc` / `Item::ContractType`. A rule enforced on one of three forms
is a rule an author meets by accident.

## Named mutants — 32, every one RED

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
| **`ext` credited at operand size** | `ext_makes_no_production_of_its_own` | RED |
| **`ext` credited at operand size (launder)** | `an_ext_after_a_call_cannot_launder_a_capped_credit` | RED |
| **partial-bit writers credited** | `single_bit_writers_produce_nothing_but_scc_produces_its_byte` | RED |
| **`Scc` swept into the partial-bit family** | `single_bit_writers_produce_nothing_but_scc_produces_its_byte` | RED |
| **diagnostic arguments swapped** | `the_width_diagnostic_names_produced_then_claimed` | RED |
| **the `ExternProc` arm deleted** | `an_externs_typed_out_caps_its_callers` | RED |
| **`Section` recursion dropped (widths)** | `a_typed_out_inside_a_section_is_collected` | RED |
| **`Section` recursion dropped (newtypes)** | `a_newtype_declared_inside_a_section_resolves` | RED |
| **newtype collision: first wins** | `a_colliding_newtype_name_is_strict_for_its_owner_and_stingy_for_its_callers` | RED |
| **bare out regs dropped from the row** | `a_duplicated_proc_name_cannot_relax_a_bare_out` | RED |
| ‡ collision resolved by first-reading | `a_colliding_newtype_name_is_strict_for_its_owner_and_stingy_for_its_callers` | RED |
| ‡ **`OutClaim::merge` takes `max` on BOTH sides** (the rule F1 rejected) | same | RED |
| ‡ `delivered()` reads the STRICT side | same | RED |
| ‡ `validate_out_types` walks `Item::Proc` only | `a_narrow_type_on_an_address_result_is_refused_on_every_declaration_form` | RED |
| ‡ the address-out width pin dropped | `an_address_out_credits_a_full_long_whatever_its_type_says` | RED |
| § **`flag_check`'s `.map(\|c\| c.credit())` -> `.strict()`** | `the_conditional_out_edge_credit_draws_the_credit_side` | RED |
| § `unresolvable_leaf`'s unknown-`Named` arm -> `None` | `an_out_type_the_corpus_cannot_resolve_is_reported` | RED |
| § an unresolvable type credits `B` instead of `L` | `an_unresolvable_out_type_credits_exactly_what_a_bare_out_would` | RED |

Bold rows are fixup round 1; ‡ rows are fixup round 2; § rows are round 3.

**Which spelling was mutated, where a phrase has two.** The ‡ "collision
resolved by first-reading" row mutates the NEWTYPE-side resolution
(`out_claim_of`'s `.reduce(OutClaim::merge)` -> `.first()`). The other reading of
that phrase — the proc-row `merge` keeping the first entry — is a different mutant
caught by a different gate, `a_duplicated_proc_name_cannot_relax_a_bare_out`.

**Exact mutated strings matter.** The `Scc` mutant is `mnem == "seq"`, NOT
`mnem == "scc"` — no CodeItem mnemonic is ever the string `"scc"` (the forms are
`seq`/`sne`/`shi`/…), so the obvious spelling leaves the whole suite GREEN and
reads as a vacuous gate. `"bchg"` in `writes_partial_bits` matches nothing today
for the same class of reason: `Bchg` is not in the ISA `Mnemonic` enum. Both are
now said in the code. Five of them (`diagnostic arguments swapped`, `the
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

## Round 3: the fifth charge site, and a type that resolves to nothing

**The conditional-out edge credit was correct and UNGATED.** It is the FIFTH
place a width is charged and the only one that does not route through
`credit_target_outs` — it is a per-EDGE transfer, so it reads `.credit()` itself.
Measured before gating: flipping that one read to `.strict()` re-opens the
collision blessing verbatim (both collision orders drop to zero firings) while the
**entire frontend suite stayed green — 2344 passed / 0 failed, measured on the tip that carried the ungated read**. Now gated, with
an extern carrying the claim because an extern's outs seed VERIFIED by axiom.

The spec's charge-site list said "four places, one helper". Both halves were
wrong, and the corrected count is derived from the tree rather than remembered:
`credit_target_outs` has THREE call sites (jbsr / `falls_into` / `TailOut`),
`required()` is read at ONE (the obligation, not through the helper), and
`flag_check`'s edge credit is the fourth credit site. So **five places — one
obligation and four credits, three of the four through the helper — and ALL FOUR
credits read the credit side.** The site the old list omitted was the one nothing
checked. It is now labelled a soundness artifact rather than a summary: anything
missing from it is something nothing is checking.

**An unresolvable out type: reported, not re-widthed.** `out(d0: NotAType)`
answers `L` on both sides. That is not "conservative" — on the credit side `L` is
the weakest — but it IS exact DEGRADATION: measured, an `out(d0: u8x)` extern and
an `out(d0)` extern credit their callers identically, so an unresolvable type
behaves as the bare declaration it collapses to and is no less sound than one.

The loss is silence: the author asked for something narrow and got 32 bits with no
signal, and a width cannot be guessed from a name that means nothing. So the
repair is a REPORT, not a number — `ContractReport::unresolvable_out_types`,
assert-empty over the corpus with a non-vacuity guard, covering `proc` /
`extern proc` / contract types. The corpus is clean today. Both texts that called
`L` conservative are corrected.

**Scoped, because the universal is false.** "An unresolvable type credits exactly
what a bare out would" holds for a simple typo — the case it was measured on — and
NOT under a name collision: with `Dup = u8` in one module and `Dup = NotAType` in
another, `credit = min(L, B) = B`, so the unresolvable reading loses the `min` and
the caller is credited NARROWER than bare and fires where the bare control does
not. Fail-safe in direction, so no hole, but the reported set and the
degrades-to-bare set are not the same set — and it was the degradation that
justified reporting rather than re-widthing. The test carrying that claim is
renamed to the case it proves, and the counterexample is now a gate of its own.

**The corpus gate's non-vacuity guard was itself vacuous, and is replaced.** It
filtered `verified_uncond_out.keys()` — the PRODUCTION half's output, which
carries a key for a proc whether or not any of its outs verified, or whether it
declares a type at all. Attack executed: stripping the types off all three
exemplars in the aeon corpus left the guard GREEN while moving the residue 16 to
20. The guard now measures the SCAN's own subject, `typed_out_slots` (17, pinned
exactly, both directions), and the same attack turns it RED naming the slot that
vanished. The rule was already written down 40 lines above it in the same file, for
`survives_claim_sites`; this is its second instance.

**`out(a0: *T)` does not reach `[call.slot-type-mismatch]`** — measured
(`out(a0: Sst)` types the slot, `out(a0: *Sst)` leaves it Untyped). The decision to
permit it stands, on the corrected grounds that it is TRUE rather than that it is
checked; the diagnostic's remedy text now steers at the newtype spelling and marks
the pointer one as documentation.

**`OutClaim`'s invariant is carried by FIELD PRIVACY.** The fields are private
behind `strict()` / `credit()` accessors, so a direct
`OutClaim { strict: B, credit: L }` cannot be written outside the module and both
constructors that exist preserve `credit <= strict`. That is the real enforcement.
The `debug_assert!`s beside them are documentation with a runtime check attached
and should be read as no more: the workspace declares no `[profile.release]`, so
`debug_assertions` is off under `--release` and they never execute in the strict
gate — and by construction nothing in-tree can make them fire anyway.

## The engine finding (F3) — investigated, not fixed

`Section_RedrawPlanes` clamps its LEFT tracker and assigns its RIGHT one:

```
move.w  Cache_Left_Col, d0
cmp.w   d0, d5
bge     .track_left_ok
move.w  d0, d5              // d5 = max(start_world_col, Cache_Left_Col)
.track_left_ok:
move.w  Cache_Head_Col, d7  // unconditional — no min against start_world_col + 63
```

Both sit under ONE comment reading "Clamp to cache range". Measured against the
engine's own idiom, the asymmetry is real: `section.emp:591-593` and
`plane_buffer.emp:262-264` both spell the right edge `min(x, Cache_Head_Col)` and
comment it as a clamp. The redraw paints at most 64 columns and skips any outside
`[Cache_Left_Col, Cache_Head_Col]`, so the painted set is
`[max(start, Cache_Left_Col), min(start+63, Cache_Head_Col)]` — and the symmetric
tracker is the `min`. **The cache is wider than the plane** (`TILE_CACHE_COLS =
80` vs 64 plane columns), so `Cache_Head_Col` exceeds `start_world_col + 63` by up
to 16 in the ordinary case, and `Section_UpdateColumns` reads that tracker to
decide what still needs streaming.

**What stops me calling it a bug:** the plane is a 64-cell ring, so a column past
`start+63` aliases a cell inside the painted span and may already be correct by
wrap. That is an oracle question, not a reading one. Three sources disagree —
header (`start_world_col + 63`), sibling idiom (`min`) and code (unconditional
assignment) — which is itself the finding. Ledgered with the experiment that
settles it. Not fixed here, as instructed.

## Files

**sigil** — `out_verify.rs` (the width lattice, `OutWidth`/`OutWidths`, the three
capped transfer-out charges, `writes_partial_bits` + `ext_promotion`),
`flag_check.rs` (edge credits carry widths), `calls.rs` (must-def reads the keys),
`corpus_contracts.rs` (`collect_out_widths` / `out_claim_of` /
`collect_newtype_underlying`, the per-side merge, `unresolvable_out_types` +
`typed_out_slots`), `parser.rs` (`out(dN: T if cc)`), `lower/mod.rs`
(`validate_out_types` over all three declaration forms), `lower/proc.rs`,
`sigil-isa/src/m68k.rs` (the partial-coverage cross-reference),
`contract_baseline.rs` (16 rows + the corrected header), `tests/out_width.rs`
(new, 30 gates), `tests/out_verify.rs` (arity + the retired-limitation note),
`tests/contract_closure_corpus.rs` (the retired-witness note),
`sigil-cli/tests/out_verify_corpus.rs` (the unresolvable-type gate + its
non-vacuity guard), the design note, the gap-ledger rows.

Every count in this packet is derived from the tree at this tip, not remembered:
30 `#[test]` in `out_width.rs`, 3 in `out_verify_corpus.rs`, 32 mutant rows, 17
typed out slots in the corpus.

**aeon** — `collision_lookup.emp`, `tile_cache.emp`, `math.emp` (+ the stale
comment), `section.emp` (d5 and d7), `entity_window.emp`, `player_sensors.emp`
(4 procs + the `SensorProbe` contract type). `sprites.emp` and `rings.emp` are
UNCHANGED — their adoptions were made and reverted on the two-sided test.
