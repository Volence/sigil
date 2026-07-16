# Item 5 (row 1054) — SectionId/GridCoord newtypes: SKETCH + balloon finding

**Per Fable's standing gate** (sketch the type homes + seam signatures BEFORE
building; STOP + send the sketch if it balloons past the batch shape). It
balloons — not in file count, but in MECHANISM. Sending the sketch.

## Type homes (clean)
`engine/system/types.emp` (the newtype home — `Coord`/`Angle`/`VramArtTile` precedent):
```
pub newtype GridCoord = u8    // a section grid index (sec_x / sec_y), 0..grid_{w,h}
pub newtype SectionId = u16   // flat section id = sec_y * grid_w + sec_x
```
(A `where 0..` refinement on GridCoord is grid-dimension-bounded, i.e. runtime —
can't be a comptime refinement; would need the act-descriptor grid to prove.)

## The seam (2 procs, 4 call sites — bounded)
- `Section_FlatIDXY` (section.emp:180) — `pub proc … () clobbers(d1) out(d0)`.
  In: **d2.b = sec_x, d3.b = sec_y**, a2 = Act ptr. Out: **d0.w = flat id**.
  → wants: in d2/d3 = `GridCoord`, out d0 = `SectionId`.
- `Section_GetSecPtrXY` (section.emp:202) — `pub proc … () clobbers(d1) out(d0, a0)`.
  In: d2/d3 = sec_x/sec_y. Out: a0 = Sec ptr (+ d0). → in d2/d3 = `GridCoord`.
- Callers: **entity_window.emp:747 (GetSecPtrXY), :750/:838/:1635 (FlatIDXY)** —
  the flat id flows out as a byte into Collected_UpdateCenter.
- tile_cache.emp is OUT: its sec_x/sec_y are block-decompose coords (a different
  granularity), and it never crosses the FlatIDXY/GetSecPtrXY seam.

## THE BALLOON (why STOP): there is no mechanism to type register-flow values
Grep-verified against the whole corpus:
- **No proc has a typed register param or typed `out()`/`in()`** — every proc is
  `proc Foo ()` with an EMPTY param list; register inputs are documented in
  COMMENTS (`// In: d2.b = sec_x`), and `out(d0)` names the *register*, not a type.
- **`let rN: Type` (body-position typed register) has ZERO corpus usage** — the
  construct is specced but never exercised.
- Every newtype in the corpus is applied to **DATA fields** (`x_pos: Coord`) or
  **comptime-fn params** (`Angle` → GetSineCosine). None types a live register.

So typing this seam is NOT a mechanical newtype adoption like the batch's Sec/Act
work. It requires ONE OF:
- **(a) a NEW typed-asm-proc-register-signature feature** — `pub proc
  Section_FlatIDXY(d2: GridCoord, d3: GridCoord) out(d0: SectionId)` with
  construction/coercion checks at the ~4 call sites. New grammar + lowering
  (step-4 verb (c) — a language ask, its own design + build).
- **(b) first-at-scale `let rN: GridCoord`** across the ~26 code sec_x/sec_y
  register sites (10 section + 16 entity_window) — the debut of an unproven
  construct, and mostly documentary (a jbsr doesn't enforce register types across
  the call, so the checking value is thin without (a)).

Either is a construct/language decision, not a byte-neutral consolidation. Row
1054's "adopt WITH the Sec/Act pass" assumed a clean co-adoption; the pass
revealed no register-typing precedent exists, so the co-adoption premise doesn't
hold.

## Recommendation
**Defer row 1054 to a dedicated newtype/register-typing effort** and MERGE the
batch without item 5 (items 4,1,2,3.1–3.4,7,6 are all done, byte-neutral, strict
2257/0). The type homes (GridCoord/SectionId) are trivial; the value is in the
seam checking, which needs feature (a) to be real. Building (b) alone would be
naming-without-checking at 26 sites — adoption-over-cleverness says don't.

If you want SOMETHING in this batch: land only the two `pub newtype` defs in
types.emp (zero consumers yet, byte-neutral) as a forward marker — but that's a
dangling type with no adoption, which the campaign usually avoids. My lean:
defer whole, ledger the (a) feature as the real unblock.

## Ledger
Row 1054 stays OPEN, RE-SCOPED: the blocker is not "adopt with Sec/Act" (that pass
shipped) but "no register-flow typing mechanism" → depends on a typed-asm-proc-
register-signature feature (or first `let rN:Type` at scale). Name that in the row.
