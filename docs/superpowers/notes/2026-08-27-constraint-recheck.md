# Re-check: which of the eight inventoried constraints the ROM build path already asserts

**Why.** `2026-08-26-placement-constraint-inventory.md` enumerated eight constraints (R1–R8)
the frozen tables enforce without declaring, to hand to the aeon lane for rule-ification
under the `SIGIL-DECOUPLE` ruling (*"every constraint the frozen tables encode today must be
recaptured as an explicit rule BEFORE the tables stop being authority"*). That inventory was
written from prose claiming `map.toml` was cosmetic under `SizeSource::Frozen`. It is not:
`native::validate_placement` runs on the shipped ROM build path, and the map's `order` DRIVES
the packing walk. **Some of the eight are already enforced, and handing those over would have
aeon build a second, weaker gate for a rule that already fails loud.** This note re-checks each.

**The bar used.** `already-asserted` means *the build goes RED if the constraint is violated* —
not that a check with a promising name runs on that data. Where the firing was reproduced
(a run, or an existing negative probe) the row says **demonstrated**; where the reading is
unambiguous but no run was made it says **by-reading**, with what would raise it.

**Address-keyed, not name-keyed.** `validate_placement` keys anchors by `a.at`. Every anchor
conclusion below was checked against the address path, never the declared name. See the
FALSE-FRIEND finding at the end — the "nothing matches anchors by name" premise the code
comment rests on is **no longer true**.

---

## The table

| # | Constraint (as inventoried) | Tag(s) | The check, named | Verdict |
|---|---|---|---|---|
| **R1a** | The set of org islands in the layout is exactly the set of declared anchors | `anchor` | `native::validate_placement` — `[map.undeclared-island]` and `[map.anchor-absent]` | **already-asserted — demonstrated** |
| **R1b** | A declared anchor holds *that particular section* | `anchor` | — none | **NOT asserted — demonstrated absent** |
| **R1c** | `ObjCodeBase` 64 KB-aligned; `dac_banks` / `sound_bank` `$8000`-aligned | `anchor` (`aligned_to`) | — none | **NOT asserted — demonstrated absent** |
| **R1d** | The `sound_bank` anchor's declared `vma` is in window phase with its `at` | `anchor` | — none | **NOT asserted — by-reading** |
| **R2** | The far-scratch (`0x70_0000 + k·0x10_0000`) measuring base for never-pinned sections | *unclassifiable* | n/a — a mechanism, not a placement predicate | **not a constraint in this taxonomy** |
| **R3** | A `bank:` section fits its bank and never straddles an N-boundary | `anchor` (`within`) | `sigil_link::relax::bank_diag` (c1/c3 in `resolve_layout_impl`) | **already-asserted — demonstrated**, with a scope caveat |
| **R4** | A rule-side gate must not be built on `resolve_layout_measuring` | *unclassifiable* | n/a — a constraint on gate construction | **still true — by-reading** |
| **R5** | Under `SizeSource::Frozen` the map's placement bases are cosmetic | — | n/a | **RETIRED — the premise is gone** |
| **R6** | "the gap between two labels is an allotment" (79 `repin` region/shape pairs) | `room`, `negative` | `repin`'s bare-`end` **warning** — and `repin` is not on the ROM build path | **NOT asserted — by-reading** |
| **R7** | A section's alignment quantum (`native::packed_align_of`) | `anchor` (`aligned_to`) | `native::validate_sound_fold` — covers **two labels** | **partially asserted — scope demonstrated, firing by-reading** |
| **R8** | The error_handler island is the final byte-emitting section | `order`, **`negative`** | `native::check_error_handler_is_last`, called from `append_deb2_appendix` | **already-asserted — demonstrated** |
| **R9 (new)** | A declared `[[hole]]`'s interior stays empty | **`negative`** | `native::hole_interior_faults`, called from `validate_placement` — `[map.hole-interior-occupied]` | **ASSERTED on the build path over demo plain / demo debug / config_b; see `2026-08-27-hole-interior-reserved.md`** |

---

## ⚠ The `negative` constraints, called out

The aeon lane asked specifically for the *"nothing may be placed between X and Y"* family and
suspected it was under-represented. It was. There are **three** in the set, and they do not
share a fate:

- **R8 — "nothing after the error_handler blob."** **ASSERTED AND PROVEN TO FIRE.** Do not
  build a second rule for this; the inventory's own recommendation ("no new rule should be
  written") stands and its three asks have since been closed in-tree.
- **R9 — "nothing inside the declared hole."** **ASSERTED — closed by this re-check.** The
  inventory had no row for it. When it was found, `validate_placement`'s entire hole handling
  was a *presence* check on the anchor label and a packed layout that filled the hole passed
  green; `hole_interior_faults` now runs from that same function, so it is a build error.
  Section R9 below records the state at the time of the finding.
- **R6 — "the region between these two labels is the first one's, not a shared allotment."**
  NOT asserted; warned in a tool that never runs during a ROM build.

One of the three negatives is still unguarded (R6). R9 was unguarded when this re-check
found it and has since been wired. **This is the finding the handover most needs.**

---

## Row detail

### R1a — the island set equals the anchor set. `already-asserted`, demonstrated.

`native::validate_placement` infers islands from the resolved layout (run head, phase-bank
head, or a gap `> ANCHOR_GAP` past the previous section's end) and set-diffs them against
`pmap.anchors_for(sound_on)` **by address**, in both directions:

```
"[map.undeclared-island] ROM section at {lma:#X} is an ANCHOR_GAP-inferred island but no
 `[[anchor]] at = {lma:#X}` is declared — add it to the placement map"

"[map.anchor-absent] declared anchor `{}` at {:#X} is not an inferred island in this build —
 the layout no longer anchors it (stale map or shape gate)"
```

It is reached from `native::build_rom_chained_with_listing` — the shipped `Frozen` ROM path —
immediately after `resolve_layout`, and its own call-site comment states the contract:

> *"the map DROVE the order above; this post-resolve pass CONFIRMS the drive — every
> byte-emitting section is declared (completeness) and the resolved layout honours the declared
> sequence + island anchors + hole (a bug in the drive, or a section the map omits, fails loud)."*

**Demonstrated:** `cargo test -p sigil-harness --lib placement_validation` → 13 passed, 0
failed. `undeclared_island_fires` and `anchor_absent_fires` are red-first probes over doctored
maps; `shape_gated_sound_bank_anchor` covers the `when`-gated arm.

**A coupling worth stating, because a re-layout can break it.** Under `Frozen`, a section is
held absolute only when its *frozen provisional base* is in the declared anchor set
(`packed_true_bases`' `is_anchor_gap`). So if the map's `[[anchor]] at` and that section's
frozen provisional base ever disagree, the section stops being an island, packs contiguously,
and `[map.anchor-absent]` fires. The map anchor and the frozen table are therefore already
pinned to each other — loudly. (by-reading; the mechanism is the same one
`anchor_absent_fires` exercises.)

### R1b — an anchor does not bind a section. **NOT asserted; demonstrated absent.**

Anchor matching is `HashMap<u32, &str>` keyed on `a.at`; the name is carried only into the
`[map.anchor-absent]` text. Nothing compares the section at an anchor to the anchor's name.

Probe (run, then deleted — it asserts the *current* absence and would become a poison-green
the day the predicate lands): a layout whose only island past the head is a section labelled
`SomethingElse` at `0x10000`, against a map declaring `[[anchor]] name="object_bank"
at=0x10000` — `validate_placement` returns `Ok`.

### R1c — no alignment predicate exists. **NOT asserted; demonstrated absent.**

Probe: a map declaring `[[anchor]] name="dac_banks" at=0x90001` against a layout with an
island at `0x90001` returns `Ok`. `sigil_ir::map::MemoryMap` has exactly one validator,
`validate_section` (region overflow); it does not look at alignment.

*Positive control for the absence* (bar 5): grepping `is_multiple_of|% 0x8000|& 0x7FFF` across
`sigil-harness/src`, `sigil-link/src`, `sigil-ir/src` matches — so the query is not broken —
and matches exactly **one** site, `native::packed_align_of`, which *derives* a quantum and
asserts nothing. There is no `(x & 0x7FFF) == 0` assertion in any of the three crates.

### R1d — the sound bank's LMA↔VMA window phase. **NOT asserted; by-reading.**

`seam2::bank_anchors_from_str` requires the `sound_bank` anchor to declare `vma`, then derives
every head's VMA as `sound_bank_vma + (lma - sound_tables_z80_lma)`. It never checks that
`vma` and `at` agree modulo `$8000`. A `sound_bank` anchor at a non-`$8000`-aligned `at` with
`vma = 0x8000` yields window pointers the Z80 latch cannot reach, silently.

*Raise to demonstrated by:* a probe feeding `bank_anchors_from_str` a map with
`at = 0xA0004, vma = 0x8000` and asserting it is refused (it currently is not).

### R2 — the far-scratch measuring base. **Unclassifiable in this taxonomy, and that is the finding.**

The `0x70_0000 + k·0x10_0000` scratch is not a constraint on where anything lands in the ROM;
it is a *measurement device* that reproduces asl's conservative `abs.l` widths for never-pinned
sections. It has no subject, no relation and no value in the predicate shape — nothing can
"violate" it, only remove it. It is `order`/`anchor`/`negative`/`room`-shaped in none of the
four senses.

*What would settle it:* aeon has already ruled it (drop it, as its own byte-moving parcel,
after step 4 archives the certification). The classification question is whether the decouple
project wants a *declared* statement of "sigil measures at the packed base" so the drop is a
diff rather than a silent equilibrium change. If yes, it is a predicate about the assembler's
measurement policy, not about placement, and belongs in a different list from the other seven.

### R3 — bank fit and no-straddle. `already-asserted`, demonstrated — **with a scope caveat**.

`relax::bank_diag`, over the converged placement:

```
"section `{}` ({:#X} bytes) cannot fit a {:#X} bank — over by {} bytes"
"section `{}` [{start:#X}, {end:#X}) straddles the {boundary:#X} bank boundary ({n:#X}-byte bank)"
```

**Demonstrated:** `cargo test -p sigil-link --test final_placement` → 9 passed, 0 failed,
including `pinned_bank_section_straddling_is_a_loud_error_not_moved`,
`bank_section_over_bank_size_is_a_loud_error`, `chained_bank_section_bumps_when_it_would_straddle`.
Port-shaped probes exist too (`sound_migration_negative_probes.rs`, `mt_negative_probes.rs`,
`sfx_negative_probes.rs`).

**The caveat, which the inventory's "the bank rules are ALREADY rules" does not carry.** The
check applies **only to sections declaring `bank:`**, and the `.emp` lowering makes `bank:` and
`vma:` mutually exclusive (`[section.bank-vma]`). In the freeze tree
(`/home/volence/sonic_hacks/.aeon-freeze-slope`, `9bba8700`) that splits the sound sections
three ways:

- `mt_bank`, `sfx_bank` — `bank: $8000` → **covered**;
- `soundbankhead` (the `sound_bank` anchor, head label `SoundTablesZ80_Head`) and
  `sound_tables_z80` — `vma: $8000`, therefore **no `bank:`, therefore no no-straddle check**;
- `dac_banks` — **no `section` declaration at all** (only `module games.sonic4.dac_banks in
  dac_banks`), so default attributes: no `bank:`, no check. It deliberately spans two `$8000`
  windows via an intra-section `align $8000`, and its correctness rests entirely on its head
  being `$8000`-aligned — which is exactly R1c, which nothing asserts.

*Positive control for that grep:* the same pattern over the freeze tree's `*.emp` does match
section declarations (`soundbankhead`, `sound_tables_z80`, `mt_bank`, `sfx_bank`), so the
empty result for `dac_banks` is an absence, not a broken query.

So "aeon's `map.toml` anchors can lean on `bank_diag` directly" is true for the two blob
sections and **false for both bank heads the anchors actually name**.

### R4 — do not gate on the measuring path. **Still true; by-reading.**

`relax::resolve_layout_measuring` calls `resolve_layout_impl(sections, stubs, dash_a, false)`
— `check_image = false` — which skips the overlap (c2) and bank-straddle (c3) checks. The
shipped path uses `resolve_layout(&all, &stubs, true)`, and `validate_placement` is fed *that*
resolve's output in `build_rom_chained_with_listing`. The advice remains correct and the
current gate does not violate it.

### R5 — "the map's placement bases are COSMETIC". **RETIRED. The premise no longer exists.**

Three separate things have changed since the inventory:

1. `map_placement::PlacementMap` has **no per-section placement bases at all** — its fields are
   `anchors`, `holes`, `budgets`, `order`. There is nothing left to be cosmetic.
2. The declared anchors are **read**, twice: into `anchor_addrs` (island classification in
   `packed_true_bases`) and by `validate_placement` (the set-diff above), and by
   `seam2::bank_anchors` for the whole sound-bank derivation.
3. `order` **drives** the packing walk (K5) — `packed_true_bases` sorts by map rank, and the
   frozen table is demoted to provisional bases + alignment quanta + round-0 measurement pins.
   `native.rs`'s own doc was corrected to say so (`cd4a0693`, *"the map is the placement
   authority, not the table"*).

The four `native.rs` line-numbered quotes the inventory cited ( `:698 :724 :880 :1967` ) do not
resolve to those comments today; the line numbers had rotted. **The sequencing conclusion the
inventory drew from R5 — "the tables cease to be authority at one flip" — should not be
carried forward unchanged.** The authority has already moved for order and anchors; what
remains behind `SizeSource::Frozen` is narrower: provisional bases, alignment quanta,
measurement pins, and the boundary keys `derive_frozen_table` reads back.

### R6 — the gap-is-an-allotment assumption. **NOT asserted; by-reading.**

`repin`'s bare-`end` finding is a **warning**, one line per region/shape
(`repin.rs`: *"a bare-label `end` that measures placer pad past the region's last section
byte"*), currently 79 region/shape pairs per `docs/OVERSEER.md`. More decisively: `repin` is a
maintenance binary (`src/bin/repin.rs`), not a step of `build_rom_chained_with_listing`. No
ROM build consults it. Under fresh placement each of the 79 is a silent mis-measure.

### R7 — alignment inferred from the pin's own address. **Partially asserted.**

`native::packed_align_of` returns the largest power of two in `{16,8,4,2}` dividing a section's
**frozen provisional base**, and `packed_chained_base` rounds the packing cursor to it. No
alignment is declared anywhere; it is an emergent property of the last refreeze. Its own doc
comment states the failure mode and names commit `2c49f538` as the live instance.

The only guard is `native::validate_sound_fold`, which is on the shipped path, always-on
(deliberately not behind `SIGIL_STRICT_GATE`), and compares seam-2's *predicted* base against
the *placed* base:

```
"[sound.fold-vs-placement] seam-2 folded absolute pointers against `{label}` = {predicted:#x}
 but the chainer placed it at {actual:#x} (delta {:+}). Every pointer cell in that blob is off
 by the same amount, so the sound would be silent or garbled at runtime with no other symptom."
```

**Its scope is demonstrable from the code shape and it is two labels** — the loop iterates a
literal two-element array:

```rust
for (label, predicted) in
    [("Song_MovingTrucks", layout.mt_bank_lma), ("Sfx_33", sfx_predicted)]
```

Every other section's inferred quantum is unguarded. `seam2.rs::frozen_prov` already ledgers
the remedy: *"A better end-state is to declare the quantum in map.toml and have both sides read
the declaration — then it is a reviewed fact in a diff instead of an emergent property of the
last refreeze. Ledgered, not done here."*

*Raise the firing to demonstrated by:* a probe that perturbs `seam2::sound_layout`'s prediction
by one quantum and asserts the `[sound.fold-vs-placement]` wording. Not done here — it needs a
built engine tree and a seam-2 emit.

### R8 — the error_handler island is last. `already-asserted`, demonstrated. **Do not re-rule.**

`native::check_error_handler_is_last`, called from `append_deb2_appendix` **before** convsym is
shelled, asserts the appendix begins at exactly `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`,
with a separate diagnosis for each direction of the drift:

```
"MDDBG blob-end contract VIOLATED: the deb2 appendix starts at EndOfRom {appendix_start:#x},
 but `{ERROR_HANDLER_BLOB_LABEL}` ({:#x}) + blob length {ERROR_HANDLER_BLOB_LEN:#x} =
 {expect:#x} — a drift of {drift:+} byte(s)."
```

**The three gaps the inventory named as the remaining parcel have all been closed since.**
The fail-open on a missing label is gone — `expect_island` is now a parameter, and both
directions of the membership mismatch are refusals:

```
"MDDBG island MEMBERSHIP violated: this shape DECLARES the error_handler island, but its
 listing defines no `{ERROR_HANDLER_BLOB_LABEL}` among {} symbol(s), so the blob-end contract
 has no subject and would have passed by having nothing to check."
```

**Demonstrated:** `cargo test -p sigil-harness --test error_handler_island_order` → 6 passed,
0 failed, including `a_section_emitted_after_the_blob_is_refused_by_name`,
`a_declared_island_with_no_blob_label_is_refused`, and the control
`the_exact_blob_end_placement_passes_the_guard_and_proceeds`.
`error_handler_island_membership.rs` adds the per-shape set-diff over `native::shipped_shapes`.
Landing commits: `1a03c75c`, `83b6610e`, `2247b0f2`, merge `9d9e164d`.

### R9 (new) — a declared hole's interior. **Unasserted when found; wired since.**

*The state below is the one this re-check measured. `validate_placement` now calls
`hole_interior_faults` after the presence arm — see the closing paragraph of this
section and `2026-08-27-hole-interior-reserved.md`.*

`validate_placement`'s hole handling at the time of the finding was:

```rust
for h in pmap.holes_for(sound_on) {
    let present = resolved.iter().any(|s| s.labels.iter().any(|l| l.name == h.after));
    if !present {
        return Err(format!(
            "[map.hole-anchor-missing] declared hole after `{}` (at {:#X}) — its `after` label
             is not in the resolved layout", h.after, h.at));
    }
}
```

`h.at` was read only to *print*. Nothing checked that `[end of the `after` section, h.at)`
was free. The comment defers to K2 (*"Holes (data; K2 enforces)"*) — but K2's enforcement is the
`filled_by` module overlaying the hole, i.e. it makes the hole *filled by the right thing*
under the current layout, not *reserved against a different one*.

**Demonstrated absent, with a positive control.** Two probes over the same synthetic layout:

- an `Intruder` section occupying the whole hole interior → `validate_placement` returns `Ok`;
- the same map with the `after` label absent from the layout → `[map.hole-anchor-missing]`
  fires.

So the green in the first probe is the check declining to look, not a broken query. Both probe
files were deleted after the run — an assertion that today's absence is correct would go
poison-green the moment the predicate lands.

`[map.hole-anchor-missing]` is pinned by `placement_validation_tests::hole_anchor_missing_fires`.

**The predicate exists and gates the build.** `native::hole_interior_faults` implements
this row and `validate_placement` calls it, so on `build_rom_chained_with_listing` — and
therefore on `sigil build` — an occupied interior is a build error. It is live over the
three shapes with a live hole (demo plain, demo debug, config_b) and says nothing about the
four that gate theirs out with `when = "sound_off"`.

The declaration those three used to fail on was aeon's, not the layout's: both maps declared
`at = 0x3FE` while `boot_tail` resumes at `0x3F8`. Aeon corrected it, and the maps declare
`at = 0x3F8` from `03ed1f1c`. Full measurement, both directions of the control, and the
shapes the gate is silent about: `2026-08-27-hole-interior-reserved.md`.

---

## Predicates for the aeon lane

Only for rows **not** already asserted. `value` is the derivation; a literal in a `value` is a
pin that goes stale silently.

### `ANCHOR_BINDS_SECTION` (R1b)
- **subject** — each `[[anchor]]`'s declared `name`, against the head label of the section
  resolved at that anchor
- **relation** — `at`
- **value** — the head label (or `section:<name>` row) the map's `order` places at that
  anchor's position
- **because** — anchor identity is address-only today, so a re-layout that lands a different
  section on an anchor address satisfies every existing check while the section the anchor
  exists to protect has moved.

### `OBJ_BANK_ALIGN` (R1c)
- **subject** — `ObjCodeBase`
- **relation** — `aligned_to`
- **value** — the `object_bank` region's declared granularity (aeon's own ruling: a 64 KB-aligned
  base is the requirement; the specific base is a kept design choice, not a hardware fact)
- **because** — the object-bank budget cursor and every bank-relative reference assume the bank
  starts on its own window; a misaligned base makes the `ceiling` arithmetic measure the wrong
  region.

### `SOUND_BANK_ALIGN` (R1c)
- **subject** — the `dac_banks` and `sound_bank` anchors' sections (`Dac_Temp_Blip`,
  `SoundTablesZ80_Head`)
- **relation** — `aligned_to`
- **value** — a multiple of the Z80 `SetBank` latch granularity, i.e. the same quantum
  `bankid()` masks with (`sigil-frontend-emp` `eval/builtins.rs`) and the `align` the
  `dac_banks` module uses between its two banks — **one derivation, three readers**
- **because** — `bankid()`/`winptr()` fold silently at any address, so a misaligned bank head
  produces a latch value and window pointer that are internally consistent and point at the
  wrong bytes; the DAC section carries no `bank:` attribute, so `bank_diag` never looks.

### `SOUND_BANK_WINDOW_PHASE` (R1d)
- **subject** — the `sound_bank` anchor's `vma` against its `at`
- **relation** — `aligned_to`
- **value** — congruent modulo the `SetBank` window size (equivalently: `vma` must equal
  `winptr(at)` as `sigil-frontend-emp` folds it)
- **because** — `seam2::sound_layout` derives every head's VMA as an offset from the declared
  `vma`, so a `vma` out of phase with `at` makes every folded window pointer wrong by the phase
  difference with no build symptom.

### `HOLE_INTERIOR_RESERVED` (R9) — **negative**
- **subject** — the span from the end of the section named by a `[[hole]]`'s `after` to the
  hole's `at`
- **relation** — `within`
- **value** — occupied by nothing but the module named in that hole's `filled_by`
- **because** — the presence guard alone checks only that the `after` label exists; without
  the interior half a packed layout that places any other emitter in the hole passes green,
  and the boot-region data that resumes at `at` is then displaced with no diagnostic.

### `REGION_END_IS_OWN_SECTION` (R6) — **negative / room**
- **subject** — each `repin.toml` region's `end` spec
- **relation** — `before`
- **value** — that region's own last section byte (the `section:<name>` spelling REPIN-END
  added), never a successor's head label
- **because** — a bare label sitting on the successor's head sweeps the placer pad between them
  into the pin, so the region measures an allotment rather than its content; under fresh
  placement the neighbour is no longer where it was and all 79 warned pairs become silent
  mis-measures (the `OJZ_Sec0_Blocks` / `ACT_DESCRIPTOR` incident is the live instance).

### `SECTION_ALIGN_DECLARED` (R7)
- **subject** — every section the packing walk aligns (today: every chained, non-island,
  byte-emitting section)
- **relation** — `aligned_to`
- **value** — a quantum **declared for that section by what its content requires**, read by both
  `native::packed_align_of`'s caller and `seam2::sound_layout` from the one declaration —
  explicitly *not* transcribed from the largest power of two dividing its current frozen
  provisional base
- **because** — alignment is currently an emergent property of the last refreeze, so a repin
  changes it with no alignment code changing (`2c49f538` doubled the SFX quantum silently and
  invalidated aeon's mod-8 pads); transcribing today's inferred quanta would enshrine every
  packing accident as a permanent requirement, which is the opposite of recapture.

---

## ⚠ FALSE-FRIEND: the "nothing keys anchors by name" premise is stale

`validate_placement`'s ledger comment (2026-08-03) says the `boot_head` collision — the
`[[anchor]] name = "boot_head" at = 0x0` meaning *the ROM's first island*, versus the actual
emitted section called `boot_head` (`engine.boot_data`, head label `BootData`) — is

> *"Harmless today precisely because nothing matches anchors by name — but any future change
> that starts keying anchors by name would silently bind the 0x0 anchor to the boot_data
> section."*

**A name-keying consumer now exists.** `seam2::bank_anchors_from_str`:

```rust
let anchor = |name: &str| -> Result<&crate::map_placement::Anchor, String> {
    map.anchors_for(true).find(|a| a.name == name) ...
};
let dac = anchor("dac_banks")?;
let snd = anchor("sound_bank")?;
```

It is on the shipped path (every sound-on build reaches `seam2::sound_layout`). It reads only
`dac_banks` and `sound_bank`, neither of which is the false friend, so **nothing is broken
today** — but the invariant the comment rests on has already been given up, and the next
name-keyed reader is one parcel away. `ANCHOR_BINDS_SECTION` above would close this by making
the name mean something checkable; failing that, renaming the `0x0` anchor is a one-line fix
that costs nothing (it is matched by address).

---

## What this did NOT cover

Stated rather than implied, and none of it should be read as clean.

- **Only the eight inventoried rows (plus R9, found in passing) were checked.** This is not a
  claim that R1–R9 is the closed set of constraints the frozen tables encode. The inventory
  itself says its enumeration parameter was "what reads or writes a placement decision in
  sigil's own crates"; this re-check inherits that parameter and adds nothing to it. A pass
  enumerating from aeon's build-side gates (`tools/bganim_room.py --gate`, `build.sh`) is still
  owed and would very likely find more.
- **No ROM was built.** Every demonstration is a unit/integration test or a synthetic probe.
  `validate_sound_fold`'s firing, the `[layout.provisional-drift]` warning path, and
  `check_object_bank_budget` against a real cursor were not exercised.
- **Only `games/sonic4/map.toml` was read**, from the freeze tree
  `/home/volence/sonic_hacks/.aeon-freeze-slope` at `9bba8700`. `games/demo/map.toml` was not
  examined, and the owner's live aeon tree was deliberately not read for map content — so any
  map change made today is not reflected here.
- **The `config_a` / `config_b` / `lean` off-canonical shapes** were not checked shape-by-shape;
  the anchor/order reasoning above is over the sonic4 union.
- **The stale prose inside `dac_banks.emp`** (its header still describes the banks at `$48000` /
  `$50000`, superseded by the 2026-08-26 re-layout to `0x90000` / `0x98000`) was noticed and
  **not** fixed — it is aeon's file and this lane changes no behaviour. Flagged because it is
  the same defect class the map's own struck-through comment block warns about.
- **`check_object_bank_budget`** was read but not classified as one of the eight; it is a
  `room`-shaped check (`cursor` vs `ceiling`) that is genuinely on the build path and has its
  own test (`native_object_bank_budget.rs`), and it may deserve a row in a later pass.

## Reproduction

- `CARGO_TARGET_DIR` used: `<worktree>/.audit-target` (on disk, not tmpfs, not the shared
  `target/`).
- `cargo test -p sigil-harness --lib placement_validation` → 13 passed / 0 failed
- `cargo test -p sigil-harness --test error_handler_island_order` → 6 passed / 0 failed
- `cargo test -p sigil-link --test final_placement` → 9 passed / 0 failed
- Four throwaway probes over `validate_placement` (hole interior, hole positive control,
  unaligned anchor address, anchor-does-not-bind-a-section) — written, run, all four green as
  predicted, then **deleted**, because each asserts that a missing predicate is missing.
