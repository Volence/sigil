# Overlay-write syntax — mini design note (one-pager)

**Batch:** sst-usability-batch, item 1. Byte-neutral, TDD.
**Ledger:** row 1023 (overlay syntax owed) — amend to SHIPPED on landing.

## The problem

A deliberate multi-field write via a typed field's address is rejected by
the totality check. load_object.emp:70 overlays four adjacent SST fields
(prev_anim/anim_frame/anim_timer/mapping_frame, $20-$23) with one long:

```
move.l  #$FF000000, offsetof(Sst, prev_anim)(a1)   // $20-$23 in one write
```

It has to use `offsetof(Sst, prev_anim)(a1)` — the plain $20 displacement —
because `Sst.prev_anim(a1)` with a `.l` instruction trips
`[operand.field-overrun] .l access reads 4 bytes but field prev_anim is 1
byte` (eval/asm.rs:1560, the check that rightly catches an *accidental*
over-wide field write). The offsetof escape works but reads as an escape:
the field is un-named at the write site and the 4-line comment exists only
to explain why the check is dodged.

## The spelling — `Sst.prev_anim:l(a1)`

A `:size` suffix on the field segment (`:b`/`:w`/`:l`) DECLARES the
intended access width, opting the site past the overrun check while
keeping the field NAME:

```
move.l  #$FF000000, Sst.prev_anim:l(a1)             // overlay $20-$23, intent explicit
```

- **`:` not `.`** — `.` is access / path-segment (and `.w`/`.l` already
  decode as size suffixes in operand position, `split_size_suffix`); `:`
  is unambiguous and binds the sized-override tightly to the field.
- **Semantics:** the override REPLACES the field's own size in
  `check_field_overrun` — the check becomes `access_width <= override`
  (was `access_width <= field_size`). The DISPLACEMENT is unchanged (the
  field's offset). So `move.l … Sst.prev_anim:l` → disp $20, check `4 <=
  4` OK. Totality is preserved: `move.l … Sst.prev_anim:w` still errors
  (`4 > 2`) — the override authorizes an overlay of a STATED width, it
  does not blanket-disable the check.
- **Reads as intent:** "prev_anim, written as a long" — the deliberate
  multi-field write is legible at the site, no offsetof escape, no
  compensating comment.

### Alternatives considered (rejected)
- `overlay(Sst, prev_anim, 4)(a1)` — a call form (row 1023's other
  candidate). More verbose; reads as machinery, not an operand; doesn't
  compose with the existing `Qual.field(aN)` operand grammar.
- `Sst.prev_anim.l(a1)` — collides with the `.w`/`.l` size-suffix
  decoding already live in operand position; ambiguous.
- Keep offsetof — the status quo; loses the field name + needs the
  comment. The whole point is to name the intent.

## Where it hooks (implementation sketch — TDD will drive)

- **Parse:** after the field segment in a `Qual.field(aN)` operand,
  accept an optional `:` + size token. The `Qual.field` operand path is
  `two_segment_field` (eval/asm.rs:1018) and the bare `single_segment_field`
  (eval/asm.rs:1000); both call `resolve_qualified_field` / `resolve_field_disp`
  then `check_field_overrun(field, size, width, span)`.
- **Thread the override:** carry the parsed `:size` to `check_field_overrun`
  so it compares `width` against the override instead of the field's
  `size`. Displacement resolution is untouched (still the field offset).
- **Both operand forms** (bare `field(aN)` on a typed reg, qualified
  `Qual.field(aN)` on any reg) accept the override.

## Tests (TDD, before code)
1. `Sst.prev_anim:l(a1)` with `move.l` → same bytes as
   `offsetof(Sst, prev_anim)(a1)` (disp $20, no diagnostic).
2. `Sst.prev_anim:w(a1)` with `move.l` → STILL `[operand.field-overrun]`
   (`4 > 2`) — the override is a stated width, not a mute switch.
3. `Sst.prev_anim:l(a1)` with `move.b` → clean (`1 <= 4`).
4. A `:size` on a field wide enough already (`Sst.mappings:l`, a u32
   field) → clean, no behavior change (documents intent harmlessly).
5. Byte gate: load_object.emp:70 converted → load_object_port GREEN both
   shapes (identical bytes; the offsetof escape + its comment dropped).

## First consumer + close-out
- load_object.emp:70 → `move.l #$FF000000, Sst.prev_anim:l(a1)`; drop the
  offsetof escape and the "offsetof gives the plain $20 displacement"
  comment (replace with a one-line overlay note).
- Amend gap-ledger row 1023 → SHIPPED (`Sst.field:size(aN)` overlay-write).
