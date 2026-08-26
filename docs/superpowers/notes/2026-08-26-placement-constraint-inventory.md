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

**R7 — the alignment number the old table still supplies: NAMED, NOT LOCATED.** BGROOM-2
books it and this pass did not find it. Not carried forward as "probably minor" — it is
unlocated, and an unlocated constraint is the whole subject of this document.

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

## What this changes about the sequencing

The tables cease to be authority at **one flip** (R5: the shipped profiles leaving
`SizeSource::Frozen`), not gradually. So R1's ruling and R2's decision are both gates on
that flip, and R6's 79 sites want converting before it rather than after.

## Open questions for aeon

1. Which of the frozen absolute addresses are **requirements** rather than frozen
   consequences? (R1 — sigil cannot derive this.)
2. Does the re-layout want sigil to keep emulating asl's conservative widths for
   never-pinned sections after the certification is archived? (R2.)
