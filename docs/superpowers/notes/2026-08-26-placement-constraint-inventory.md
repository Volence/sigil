# What the frozen placement tables quietly enforce — inventory, first pass

**Why this exists.** Step 2 of the decouple project (aeon `docs/DEFERRED_WORK.md` at
`822c382a`, owner ruled 2026-08-26) carries its own precondition: *"Every constraint the
frozen tables encode today must be recaptured as an explicit rule BEFORE the tables stop
being authority, or it silently stops being enforced."* The tables are sigil's, so the
inventory is sigil's. Aeon's `ROM-RELAYOUT` parcel (in flight as this is written) **keeps
the tables as authority and refreezes them**, so this feeds the later rule-ification half
rather than blocking that landing — confirmed by the aeon lane directly.

**Enumeration parameter, stated so a later pass can vary it** (bar 19). This pass
enumerated **by what READS OR WRITES a placement decision in sigil's own crates** — the
frozen-table loader and its consumers, the placer's checks, and the fixtures that hardcode
an address. It did **not** enumerate by what the tables list, deliberately: the tables are
the artifact whose authority is being retired, so taking their row set as the question
begs it, and a constraint that lives in a fallback rather than in a row is exactly the kind
that disappears silently. **A second pass should enumerate by a different parameter** —
aeon's own `map.toml` and build-side gates are the obvious one, since a constraint may be
jointly encoded and visible from neither side alone.

**This is a first pass and is not complete.** Rows 1-6 are evidenced below; row 7 is named
and NOT yet located. Nothing here should be read as the closed set.

## The surfaces that carry placement authority today

- `crates/sigil-harness/golden/offcanonical_sizes/*.txt` — seven shapes, ~53-99 rows each
  (68 labels for `s4`), each a label→ROM LMA. Loaded by `native::load_frozen_table` into
  `SizeSource::Frozen`.
- `crates/sigil-harness/repin.toml` + the generated `pins.rs` — region start/end anchors.
- `crates/sigil-harness/src/native.rs` — the per-shape profiles and the chainer.
- `crates/sigil-link/src/relax.rs` — the placer proper: the fixpoint, its soundness checks,
  and the measuring variant that skips them.
- `engine.inc`, `mixed_dac_rom.rs`, `repin_pins.rs` and the literal-address test assertions
  — hand-edited address fixtures, four of the five ripple sites.

## The rows

**R1 — Absolute anchors and their consequences are not distinguished anywhere.** The frozen
tables carry `EntryPoint 0x200`, `Checksum 0x18e`, `Dac_Temp_Blip 0x48000`,
`BusError 0xa0120`, `EndOfRom 0xa11d0` in one flat namespace with 63 other labels. Some are
**requirements** (the sound-bank latch at `0x48000`; the vector/header addresses the
hardware fixes) and the rest are **consequences** of packing that were frozen and have been
read back as authority ever since. A fresh-placement rule can only assert the requirements,
and nothing in the current form says which those are. **Aeon must rule which anchors are
real**; sigil cannot derive it, because the table's shape destroyed the distinction.

**R2 — The far-scratch measuring base is load-bearing, and the project retires its REASON
while leaving the mechanism wired in.** Never-pinned sections (`replay`, `raster`,
`page_cache`, …) measure at `0x70_0000 + k·0x10_0000`, and that is deliberate: asl encodes
a forward reference `abs.l`, while at the real sub-`$8000` base sigil's relaxer settles
`abs.w` and the chained layout packs **2 bytes tighter per site**. The scratch reproduces
asl's conservative widths — it is **asl-parity emulation, not a game requirement**
(`2026-08-26-measure-at-packed-base-packet.md` §2b, where measuring every section at its
own base moved clean bytes and was rejected for exactly this).
**The consequence for the project, and it wants a ruling rather than an inheritance:**
step 1 pins the corpus and step 4 archives the byte-identical certification, so the reason
this equilibrium exists is being retired by the same project that will re-place everything
fresh. Keeping it means the assembler goes on emulating a toolchain nobody runs; dropping
it moves bytes. Either is defensible; drifting into one of them because nobody enumerated
it is not.

**R3 — The bank rules are ALREADY rules, and need no recapture.** `relax::bank_diag` runs
post-fixpoint against the final converged placement and errors on two conditions: content
larger than its bank (`final > n`, "over by K bytes"), and a `bank:` section whose
`[first, last]` straddles an N-boundary. Chained sections get a constructive bump in
`place_pass`, so the straddle case exists to catch **pins**, which are never moved. This is
the one part of the current authority that is already expressed as a rule over a declared
property, and aeon's `map.toml` anchors can lean on it directly.

**R4 — Any rule-side gate must not be built on the measuring path.**
`resolve_layout_measuring` runs the same fixpoint but **skips the image-soundness checks**
— overlap (c2) and bank straddle (c3) — by design, because the measuring device needs the
relaxer's exact widths at colliding pins. A gate written against that path would pass over
layouts the shipped path refuses.

**R5 — Under `SizeSource::Frozen` the map's placement bases are COSMETIC, and this is the
trap step 2 walks into.** `native.rs` says so in four places (`:698` "IGNORED under
`SizeSource::Frozen`", `:724` "cosmetic pin under Frozen", `:880` "their placement base/len
are IGNORED", `:1967` "the placement map bases are COSMETIC — the chainer recomputes").
So aeon can declare an anchor in `map.toml` today, sigil will accept it, and **nothing will
read it** while the shipped profiles remain Frozen. An anchor that is declared and inert is
worse than an absent one: it reads as enforced. Sequencing follows — the anchors become
authority only when the profiles stop being `Frozen`, and that flip is the actual moment
the tables cease to be authority.

**R6 — "the gap between two labels is an allotment" stops being true, and 79 sites assume
it.** `repin`'s bare `end` sweeps placer pad; the REPIN-END parcel converted the two worst
and left **79 region/shape pairs** carrying a neighbour's padding by convention, warned and
ledgered. The live instance of the same family cost a landing already: a refreeze put
`OJZ_Sec0_Blocks` two bytes past the descriptor on an alignment boundary and the
**successor's pad entered `ACT_DESCRIPTOR`** (0x27C pinned vs 0x27A real). Under fresh
placement the neighbour is no longer where it was, so every one of the 79 is a silent
mis-measure waiting rather than a warning.

**R7 — LOCATED, and it is the row that most needs recapturing: alignment is INFERRED FROM
THE PIN'S OWN ADDRESS, AND THE INFERENCE IS CAPPED AT 16.** `native::packed_align_of` returns
*the largest power of two **from the set {16, 8, 4, 2, 1}** dividing a section's frozen
provisional base* — it walks `[16, 8, 4, 2]` and returns 1 otherwise, so **an address divisible
by 32, 64 or more still infers exactly 16**, and
`packed_chained_base` aligns the packing cursor to it. So the table does not declare
alignment anywhere — **alignment is a side effect of where a section happened to land**. A
section frozen at a `%16 == 0` address is thereafter treated as requiring 16; the identical
section frozen eight bytes earlier requires 8.

**Its own doc comment states the failure mode, and it is not hypothetical:** *"the quantum
is a property of the frozen PIN, so a repin can change a section's alignment without a line
of alignment code changing."* Commit `2c49f538` moved the SFX pin from `$5BAE8` (`%16 == 8`,
quantum 8) to `$5BB10` (`%16 == 0`, quantum 16), silently doubling it and invalidating the
mod-8 structural pads aeon had built against the old value.

**Why this is the worst row to lose silently.** These are not cosmetic alignments: `seam2`
bakes ABSOLUTE pointers into emitted blobs (SfxTable cells, `SFX_WIN_*` window pointers, MT
song-table pointers) against its own prediction of these bases, and a disagreement means the
pointers are short by the delta and the sound is silent or garbled at runtime **with no other
symptom**. The `[sound.fold-vs-placement]` gate (`validate_sound_fold`) makes exactly that
class loud — but it covers **two labels**, `Song_MovingTrucks` and `Sfx_33`. Every other
section's inferred quantum is unguarded.

**⚠ THE CAP IS LOAD-BEARING AND THIS ROW BURIED IT — corrected 2026-08-29 after it misled a
peer at a live freeze.** The original headline read *"the largest power of two dividing a
section's frozen provisional base"* with `(16, 8, 4, 2, else 1)` in a **parenthetical**. The
aeon lane, reasoning from the headline during chain 181, computed `BgAnim_Table` moving
`0x27D20 → 0x27EC0` as an inferred alignment of **32 → 64** and was about to record *"R7 fired"*
in the freeze prose. It did not fire: `packed_align_of` returns **16 for both**. The real cause
was ordinary quantum-16 padding — the region's base moved `+0x198`, which is not a multiple of
16, so the trailing fill went 6 → 14 and the length grew 8. Both fills reproduce exactly from an
unchanged quantum, and the region's `0x4882` of content was byte-identical.

**The defect in this row was not omission — it was placement.** The cap WAS stated, in a
parenthetical, next to a summary sentence that contradicted it. **A qualifier in a parenthetical
loses to the sentence it qualifies**, which is this file's own *"a qualifier printed beside a
value is part of the value"* lesson turned on its author. A reader who takes the headline gets a
wrong model and never reaches the parenthetical, because the headline already answered them.

**Two consequences for how this row should be priced.** (1) **The ratchet has a ceiling**: the
inferred quantum ranges over {1,2,4,8,16} only, so a silent over-align costs at most 15 bytes per
section — bounded, and materially smaller than an unbounded reading suggests when weighing it
against the `DATA_GROWTH_RESERVE` budget. (2) **The dangerous direction is still the DOWNWARD
one** — a base landing on `%8` where something needed 16, the `2c49f538` SFX precedent below —
and it remains unexercised. Chain 181 is **not** evidence about it in either direction, and must
not be cited as though it were.

**What recapture means here, and it is a question rather than a transcription.** Most of
these quanta are almost certainly accidents of packing that nothing requires; some are
load-bearing (aeon built pads against one). Writing them out as declared rules verbatim would
enshrine every accident as a requirement forever — the opposite of the point. The honest form
is: each section's alignment is declared as what its CONTENT needs, and anything that only
ever held by accident is allowed to stop holding, deliberately and once, rather than being
discovered later by silent audio corruption.

**Live hazard for the re-layout, flagged to aeon:** their parcel moves every island past the
sound bank, so any moved section whose NEW base has a different largest-power-of-two divisor
than its old one has its alignment quantum silently changed — the `2c49f538` class, in
flight, today.

### R7 IS RECAPTURED (2026-08-29, `parcel/declare-section-alignment`) — byte-neutral

**What now exists.** `crates/sigil-harness/src/section_align.rs` declares, per ROM section
keyed by HEAD LABEL, the alignment that section **requires** and the **source** the
requirement comes from. 107 rows, covering every section that carries a frozen provisional
base in any shipped shape (86) plus every section that appears in a shipped resolved layout
without one (21). Two always-on halves in `native.rs` check it inside
`build_rom_chained_with_listing` — the entry `sigil build` reaches — so every build of every
shape runs both:

| half | when | against | loud on absent declaration |
|---|---|---|---|
| `validate_declared_alignment` | before the packing walk (`true_bases_by_index`, Frozen arm) | each pinned section's **frozen provisional base** | yes — `[layout.undeclared-alignment]`, naming the section, its base, and the quantum the inference would have given it |
| `validate_resolved_alignment` | after `resolve_layout`, beside `validate_placement` | each section's **resolved LMA** | yes — `[layout.alignment-violated]` / `[layout.undeclared-alignment]` |

Witness: `crates/sigil-cli/tests/section_alignment_declared.rs` (7 shipped shapes green;
red-first witness doctors the `Sfx_33` frozen row `+4` and asserts the refusal names the
section, the requirement, the source and the residue) plus
`native::declared_alignment_tests` (7 unit tests) and `section_align::tests` (4).

**The row's own headline was still not quite right, and the measurement says so.** This row
called for "declared == what the inference produces". **That scalar does not exist.**
Measured across the seven shipped shapes: **38 of the 86 pinned sections infer a DIFFERENT
quantum in different shapes.** `Ani_Tails` infers 16 in `config_a`/`config_b`/`s4_debug` and
2 in `s4`/`lean`; `Collected_Init` infers all four of 2, 4, 8, 16. A per-(section, shape)
table that did hold the equality would be a mechanical re-encoding of the frozen tables —
an expectation copied off the pin it is checking. So what is declared is the REQUIREMENT,
and the checks are divisibility, not equality.

**That is not a weaker check.** For every `r ∈ {2,4,8,16}` — everything the inference can
express — `r | prov` and `r | packed_align_of(prov)` are **equivalent** (`packed_align_of`
returns the largest element of `{16,8,4,2}` dividing `prov`; a power of two `≥ r` dividing a
multiple of `r`). Checking the provisional base directly also stays meaningful above the cap,
and needs no second copy of the walk's island classification.

**And it answers the cap question with numbers.** *No section requires 32.* Sections whose
frozen base is divisible by 32+ are divisible by wildly different powers in different shapes
(`BG_Init`: 16, 32, 512; `Tile_Cache_GetTile`: 16, 32, 64, 256, 2048) — coincidence, not
requirement. **Three sections require more than 16, and the cap hides every one of them**:
`Dac_Temp_Blip` and `SoundTablesZ80_Head` require `$8000` (one Z80 `SetBank` window;
`packed_align_of($90000)` = 16), and `ObjCodeBase` requires `$10000` (R1's ruling below;
`packed_align_of($10000)` = 16). All three are held at declared `[[anchor]]` addresses today,
so the inference never runs on them — which is exactly why the requirement had to be written
down before the anchors become the only authority.

**Everything else declares 2** — the 68000 word rule — because nothing in aeon's sources asks
for more. The wider quanta those sections receive today are slack the inference hands out. At
the flip, `align_up(running, required)` replaces `align_up(running, packed_align_of(prov))`
and bytes WILL move for them; that is the flip's paired freeze, not this parcel's.

**A consequence worth pricing.** With the requirement written down, the `2c49f538` class is no
longer dangerous where the requirement is 2: a refreeze silently moving such a section's
inferred quantum 16 → 8 is a genuine non-event, and the gate stays silent by design. It is
dangerous only where a real requirement exists — and there the gate fires, on every shape,
by name. What remains uncovered is a NEW section arriving with a real requirement nobody
declares: the completeness half refuses the build until someone writes a row, but it cannot
know the row is right. That is a review obligation, not a mechanism.

#### How the fourteen non-`2` rows were enumerated — LEADS FIRST, THEN ONE SWEEP

**Stated plainly because the number is a floor and the method is what prices it.** The rows
declaring more than 2 were found by FOLLOWING LEADS, not by sweeping what touches a section's
base. The leads were: the brief's own pointer to `Sfx_33`; `packed_align_of`'s doc comment;
**this document's R1 ruling** (which handed over `ObjCodeBase` 64 KB, `dac_banks` / `sound_bank`
`$8000`); `map.toml`'s BANK PLACEMENT RULE comment; one `align` keyword grep over `map.toml`
and the `.emp` corpus; one `ensure(` grep **scoped to `games/sonic4/data/sound/`**; and
`mt_bank.emp`'s `MT_TAIL_PAD` comment. 104 of the 107 rows therefore declare **2 by rule**,
not by measurement.

**The sweep that should have run first, run afterwards: `ensure(` over the WHOLE corpus,
filtered to mask/modulo/align shapes.** Its complete yield of constraints on a ROM SECTION
BASE:

| site | constraint | effect on the table |
|---|---|---|
| `engine/system/epilogue.emp:29` | `(extern("EndOfRom") & 1) == 0` | **confirms** the `EndOfRom` row's 2 from source — it was not a default |
| `games/sonic4/data/sound/sfx_bank_blob.emp:55` | `(winptr(Sfx_33) & 7) == 0` | already held |

Everything else the sweep returned is a constraint on something that is **not** a ROM section
base: Z80 RAM addresses (`SND_RING_BASE`, `SND_SFX_BASE`, both `& $FF`), a work-RAM window
(`Player_1 & $FFFF8000`), struct offsets and blob lengths (`% 2`, `% 4`), or `bankid()` /
`winptr()` **consumers** that compute a value rather than assert one.

**So the sweep raised the count by zero and upgraded one row from rule to source.** What it
CANNOT reach, and what keeps the count a floor: a requirement implicit in an INSTRUCTION with
no comptime wall beside it. Aeon's own sources contain a documented near-miss of exactly that
shape — `engine/sound/z80_sound_driver.emp:1034` records a 256-byte page-aligned optimisation
NOT TAKEN because `ensure((DacSampleTable & $FF) == 0)` fails today. Had it been taken without
that `ensure`, a 256-byte requirement would exist with nothing in the corpus naming it, and
this table would declare 2 and pass green.

#### THE SPAN CLASS — asked about specifically, and the design is NOT blocked on it

The question put to this parcel: the VDP's 68k→VRAM DMA source must not cross a **128 KB**
boundary (the source counter's low 16 words wrap), which is a NON-CROSSING constraint on a
SPAN, not a quantum on a base — and a table of alignments cannot express it. Measured:

1. **Does the constraint exist in this corpus? YES, and it is load-bearing.**
   `engine/objects/dplc.emp:57` — "a landing whose ROM source straddles a `$20000` boundary is
   SPLIT into two entries by QueueDMA"; character art totals ~354 KB, "i.e. ~2.7 x 128 KB, so
   at least two boundary-straddling entries exist by construction."

2. **Are the frozen tables the only thing keeping it true? NO — and that is the finding.**
   It is kept true **structurally at runtime**. `engine/system/dma_queue.emp`'s shared
   `.transfer` core computes the crossing arithmetically from the ACTUAL source word address
   (`moveq #0,d0 / sub.w d3,d0 / sub.w d1,d0 / blo .split`) and splits the transfer into two
   queue entries at the boundary. No placement can make a transfer read wrong data, because no
   placement is consulted — the check is on the address the transfer actually carries. The
   hypothesis that the frozen tables are silently holding this up is **false for this corpus**.

3. **Therefore NOT "BLOCKED on the declaration's expressiveness" — the surfaces are already
   correctly factored.** A span constraint does not belong in `section_align` and does not need
   to be bent into one, because the span surface ALREADY EXISTS and is separate:
   `map.toml`'s `[[budget]] region / ceiling / cursor`, gated by
   `native::check_object_bank_budget` → `map.check_budget`. The object bank's
   `ceiling = 0x20000` **is** a declared, enforced 128 KB span constraint today. Bank
   co-residency — `ensure(bankid(X) == bankid("MovingTrucks_Bank_Start"))` for six labels in
   `mt_bank.emp` / `sfx_bank.emp` — is the other span-class constraint, and it is folded
   against real addresses by the seam-2 emitters (`emit_mt_bank_at` takes `mt_bank_lma` and
   `bank_start` for exactly that fold). Verified by signature and by the module's own comment;
   NOT proven red-first here.

**The one real gap this turned up, and it is a different failure mode than the question
assumed.** `.split_reject` needs **two** free Important slots or it rejects **both halves** of
a split transfer. `DPLC_ENTRY_RESERVE = 2` is sized from total art volume, with aeon's own
note reading "Two is the floor, not a comfortable margin." **How many transfers straddle a
boundary in a given frame is a function of where art lands — placement — and nothing in sigil
checks it.** The consequence is a DROPPED transfer (carry set: a missing or late art update),
not wrong data — a visual glitch, not corruption. That is a queue **slot-budget** gap, not an
alignment gap and not a correctness gap, and it is the item to sequence.

## R1 is RULED by aeon (2026-08-26), and the check they asked for found a second population

Their ruling, read off `games/sonic4/map.toml`'s own anchor comments: vectors + header at
`0x0` are hardware; `ObjCodeBase` requires a **64 KB-aligned** base (`0x10000` itself is a
kept design choice, not a hardware fact); `dac_banks` / `sound_bank` / the `0x60000`
`z80_bank` require **`0x8000` alignment** (Z80 `SetBank` latch granularity) plus a derivable,
co-resident bank id — the specific `0x48000` / `0x50000` / `0x58000` / `0x60000` addresses
are **not** requirements; the error-handler / MD Debugger island requires **ORDER** (last
emission), not an address; `EndOfRom` and every other row are packing outcomes. They flagged
it as read from comments rather than by re-tracing consumers, and named this document's
touch-enumeration as the check.

**Run, and it finds what a requirement-shaped question cannot: sigil's own fixtures are
pinned to the addresses aeon just declared non-requirements.** Twenty-one files match the
four bank literals; after classifying, the genuine consumers are — in `sigil-harness/src`,
the four the 08-10 audit already names (`seam2.rs`, `native.rs`, `repin.rs`,
`map_placement.rs`) plus `harness/tests/repin_pins.rs`; and a **second population the audit
does not reach**, nine files under `crates/sigil-cli/tests` and one under
`crates/sigil-frontend-emp/tests`: `seam2_dac_emit.rs`, `seam2_dac_head_colink.rs`,
`seam2_layout_derivation.rs`, `seam2_colink_probe.rs`, `dac_port.rs`,
`sound_migration_negative_probes.rs`, `sfx_negative_probes.rs`, `mt_negative_probes.rs`,
`ports.rs`, `z80_resident_cell.rs`.
Classified OUT as coincidence, so the raw count is not the finding: `eval_typed.rs`
(16.16 fixed-point arithmetic where `0x60000` is 6.0) and the synthetic
`lma: 0x60000` unit fixtures in `sigil-ir/src/lib.rs`, `sigil-link/src/lib.rs`,
`map_load.rs` and `two_section_ab.rs`, which use the address as an arbitrary LMA and are
independent of the game's layout.

**The hazard inside that second population, and it is bar 2 rather than bookkeeping.**
Several are NEGATIVE probes whose validity is a fact about bank arithmetic at those exact
addresses: `ports.rs` independently computes `(0x58000 & 0x7F8000) >> 15 = 0xB` and asserts
both a right-bank build that must succeed and a wrong-bank build that must fail;
`sfx_negative_probes.rs` and `mt_negative_probes.rs` plant a cross-seam label at `0x58000`
*"instead of the real `0x60000`"*. Move the banks and each of those keeps compiling and
keeps passing while no longer testing what it names — a poison green for the wrong reason.
**They need their bank ids RE-DERIVED, not their addresses retyped**, and the re-derivation
has to come from the moved map, not from the constant in the assertion.

## R2 is RULED by aeon: drop the asl-parity scratch, as its own byte-moving parcel

Agreed there is no game requirement. Sequenced AFTER the re-layout lands and after step 4
archives the certification it serves, and folded into neither — one byte-mover per branch.

## R8 — the debugger-island order gate ALREADY EXISTS, and the real gap is that it has never fired

Aeon asked (2026-08-26) for a sigil-side rule that the error_handler island is the last
emitting section, red-first, hard under strict. **Their own `games/sonic4/map.toml` names
the guard that already does it**, and reading the code rather than the comment shows it is
both real and *stronger* than the ask:
`native::check_error_handler_is_last`, called from `append_deb2_appendix` **before** convsym
is shelled, asserts the appendix starts at **exactly** `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`
— so a byte placed after the blob fails, and so does a blob that ends short, each with its own
explanation. It is derived from the blob label and length, never from a constant, and it names
the mechanism (the two `lea` displacements baked into the vendored blob) and the fix. It is on
the shipped path: `sigil-cli/src/main.rs:1874` reaches it on every `sigil build`.

**The gap is real but it is a different one: nothing has ever proved it fires.**
- **No poison exists.** Nothing in the workspace plants a section after the blob, and nothing
  asserts the violation message. The only non-`native.rs` matches are comments.
- **The two existing negative controls walk PAST it.** `native_full_rom.rs` (~:351, ~:362)
  rejects a collapsed listing and an empty one — but both target the appendix **size band**,
  and because their synthetic listings carry no `ErrorHandlerBlob` label at all, both take
  this guard's *vacuous* arm on the way through. They exercise the code around it without
  ever testing it.
- **It fails OPEN on a missing label**: no `ErrorHandlerBlob` in the listing returns `Ok(())`.
  That is correct for a shape genuinely without the island and **indistinguishable from a
  rename or a harvest miss for a shape that has one**, which is the absence-shaped failure
  our bars name — nothing asserts that the canonical shapes reach the non-vacuous arm.

**So the parcel is smaller and sharper than the ask:** (1) a red-first probe that plants an
emitting section after the blob and asserts the guard's own wording, matched on phrasing
unique to this rule; (2) a per-shape assertion that every shape carrying the island reaches
the NON-vacuous arm, which is what closes the fail-open; (3) only then, if wanted, the strict
tier. **No new rule should be written** — writing one would add a second, weaker statement of
a contract that already has a stronger one, and the two would drift.

**Sequencing:** (1) is a new test file and collides with nothing, so it can land now. (2)
touches `native.rs`, which aeon's in-flight re-layout agent is editing, so it waits for that
pair to land.

## What this changes about the sequencing

The tables cease to be authority at **one flip** (R5: the shipped profiles leaving
`SizeSource::Frozen`), not gradually. So R1's ruling and R2's decision are both gates on
that flip, and R6's 79 sites want converting before it rather than after.

## Open questions for aeon

1. Which of the frozen absolute addresses are **requirements** rather than frozen
   consequences? (R1 — sigil cannot derive this.)
2. Does the re-layout want sigil to keep emulating asl's conservative widths for
   never-pinned sections after the certification is archived? (R2.)
