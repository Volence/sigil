# SECTION-ROW packet — the `"section:<name>"` `order` row (2026-08-26)

Branch `feat/section-row` (sigil). Design: `2026-08-26-derived-layout-design.md` §3.5.
Consumer: aeon, `games/sonic4/map.toml` (the `ojz_effects_editor_act1` row). Cross-repo
interface: the row syntax and the diagnostic ids + wording below — aeon fixtures may
assert on the text, so the wording changes only with a packet.

## 1. Syntax

An `order` row is a string. Two spellings:

| row | keys the section by |
|---|---|
| `"GameLoop"` | its HEAD LABEL (lowest-offset label) — unchanged |
| `"section:ojz_effects_editor_act1"` | its SECTION NAME — the `<name>` of `module … in <name>` |

No `map_placement.rs` struct change: `order: Vec<String>` stays; `section_row(row)` /
`section_row_key(name)` (`map_placement.rs`) are the one parser and the one printer.

The prefix is unambiguous: no label can contain `:`. The `.emp` lexer's identifier is
`[A-Za-z_][A-Za-z0-9_]*` (`lexer.rs:129-130`, `:` is `Tok::Colon`); the AS lexer's
identifier set is alphanumerics plus `_ . '` (`sigil-frontend-as/src/lexer.rs:235-238`).
Witnessed by `map_placement::tests::no_label_can_spell_the_section_prefix`.

## 2. Rules (each has a gate)

1. **Byte-emitting section, `section:` row** — ranks exactly where the row sits, in
   both the packer (`order_rank_of`, feeding `packed_true_bases`'s `own_rank`) and the
   drive-confirmation (`validate_placement`). Its real head label satisfies
   `[map.order-undeclared]`. Gates: `section_row_drives_the_packer_rank` (same bases as
   the label spelling), `section_row_satisfies_completeness`,
   `section_row_ranks_where_it_sits_in_validation` (a divergence names the row by its
   `section:` spelling).
2. **Zero-byte section, `section:` row** — accepted and inert: a reserved slot that is
   authorable before the content exists. Trips nothing (`order-undeclared`,
   `order-diverged`, completeness), wherever the row sits. Gate:
   `zero_byte_section_row_is_inert` (three positions, one contradicting the layout).
3. **Unknown section** — `[map.order-unknown-section]`. Gate: `unknown_section_row_fires`
   + the live-corpus `misspelled_section_row_fails_the_build_loudly`.
4. **Declared both ways** — `[map.order-double-declared]`. Gate:
   `double_declared_section_fires` + the live-corpus
   `section_row_and_label_for_one_section_fail_the_build_loudly`.
5. **Prefix vs label** — see §1; the AS side's `.`-bearing labels and the `.emp` side's
   identifiers are both colon-free, so a row containing `section:` is a section row and
   nothing else.

Scope note on rule 3: the check is PER BUILD (the shape being validated), because
`validate_placement` sees one shape's resolved sections. The design note's weaker
"absent from EVERY shape" form is not implementable at that site and is not needed by
the live corpus (config_a/config_b/s4/s4.debug share sonic4's map and its section set;
demo has its own map). A shape-conditional section would need a `when`-gated row —
ledgered, not built.

The `[map.order-undeclared]` scoping is unchanged: it still keys on `image_bytes().len()
> 0` and still names the HEAD LABEL (a label-less byte-emitting blob stays exempt).

## 3. Diagnostics (ids + exact wording; cross-repo interface)

```
[map.order-unknown-section] `order` row `section:<row>` names no ROM section in this build — the name is the `module … in <name>` target; fix the spelling or drop the row
[map.order-double-declared] section `<name>` is declared twice in `order` — by its head label `<label>` and by `section:<name>`; keep exactly one row
[map.order-section-row-empty] `order` row `section:` names no section — spell it `section:<name>`
```

(`<row>` / `<name>` / `<label>` are the substituted values, each in backticks.) The
third fires at map LOAD (`load_placement_map`), the first two in `validate_placement`
before the completeness walk — so a typo'd row is reported as itself, never as an
`[map.order-undeclared]` for the section it meant. Unchanged and still exact:

```
[map.order-undeclared] byte-emitting section `<head-label>` is not in the declared `order` — the map DRIVES placement now, so every emitter must be declared; add it in its layout position
[map.order-diverged] the resolved layout places `<row>` after `<row>`, but the declared `order` has `<row>` before it — the packer did not honour the driving order (packer bug)
```

(`order-diverged` now prints the row AS DECLARED — the `section:` spelling for a
section-row section, the head label otherwise.)

## 4. Migration for aeon (`games/sonic4/map.toml`)

Line 124, replace the literal with the section row, same position:

```toml
  "section:ojz_effects_editor_act1",
```

and rewrite the :110-:123 comment to the present-tense fact (no change-history
narration): the block's head label is content-derived (whatever `effects_gen.py`
emits first), so the row keys the SECTION by name; a zero-byte block is inert under
this row, and the day it emits its bytes land here.

Byte identity of that migration is already proven from the sigil side, and the gates
are DIRECTION-AGNOSTIC: `both_spellings_of_the_section_row_build_the_same_rom` derives
the head label from the live build's own section table (never a string constant), reads
whichever spelling the live map currently carries, builds sonic4 plain + debug from a
COPY of the aeon tree with the OTHER spelling, and asserts live == other == provenance
tip (s4 875d591f/699223, s4.debug a02d36db/715114). The invariant "the two spellings
are the same ROM" holds forever, so aeon's landing needs NO action on sigil's suite;
the only real unmeasurable (neither spelling in the live map, or both) fails loud
naming the section.

The `repin.toml` `scene_registry` region end (`end = "EditorSceneBinding_OJZ_Act1_Sec0"`,
its own comment says "when sigil's `section:` order row lands this end should follow
it") is a SEPARATE mechanism and is untouched here. A `section:` spelling there IS
wanted — the same content-derived label rots the same way — and is REPIN-END's
(sibling parcel): the region reader would resolve `section:<name>` to the section's
LMA, the exact analogue of what this parcel does for `order`.

## 5. Draft paragraph for `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §7.1 (not landed)

> **Declaring a section in the map's `order` (parcel SECTION-ROW, 2026-08-26).** The
> per-game placement map's `order` array names the byte-emitting sections in their
> ROM sequence. A row is either a section's HEAD LABEL (`"GameLoop"`) or its SECTION
> NAME with the `section:` prefix (`"section:ojz_effects_editor_act1"`, the `<name>` of
> `module … in <name>`). The second spelling exists for sections whose head label is
> generated from content and so cannot be authored; it ranks the section exactly where
> the row sits, a zero-byte section it names is an inert reserved slot, a name absent
> from the build is `[map.order-unknown-section]`, and one section declared both ways
> is `[map.order-double-declared]`. The prefix is unambiguous because no identifier in
> either front-end admits `:`.

## 6. Evidence

- Red-first: `section_row`/`section_row_key` sabotaged (two lines) — 7 unit gates + 3
  fixture gates red (`target/red-first-unit.log`, `target/red-first-fixture.log`,
  stamped pwd/HEAD/branch/aeon); restored — 18/18 unit, 3/3 fixture green.
  `zero_byte_section_row_is_inert` and the lexer witness are negative-space tests and
  pass under the sabotage by construction (they assert an absence).
- The fixture gates keep working when `effects_gen.py` mints a different head label
  (derived from the build) and after aeon migrates the row (direction-agnostic).
- Live corpus unchanged: no `section:` row exists in either shipped map, so the parcel
  is a no-op on every shipped shape; the seven CRC gates prove it (see the suite log).
- aeon landing tree moved 415e0b6a → 0e34408d (tracked files clean before and after);
  the four ROMs rebuilt with this branch's `sigil`/`emit_sound_blob` after
  `tools/regenerate-level.sh` (DONOR_PROVENANCE churn discarded): 875d591f/699223,
  a02d36db/715114, bf2cdb42/96412, 62a0019e/101120 — all four equal the tip.

## 7. Open

- `when`-gated `section:` rows (a shape-conditional section) — ledgered.
- REPIN-END: the `section:` spelling for `repin.toml` region ends — sibling parcel.
- Per-row alignment (`"section:<name>@align=2"` / a `[[section]]` table) — derived-layout
  Option B/C step 3, not this parcel.
