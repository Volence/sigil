# Spec draft — §4.9 `table` + decision D2.36 (for the empyrean SIGIL_SPEC2_LANGUAGE.md)

Written 2026-07-11 alongside the `table` implementation (Plan 7 T2-d). This is the
**spec-section draft** for the ratified design
(`2026-07-11-counted-sparse-collection-design.md`) — the empyrean spec edit is Volence's
cadence (the offset-table §4.7 precedent), so this file holds the §4.9 + D2.36 text ready to
paste, mirroring the offset-table handoff. Implementation SHIPPED on the `table-construct`
branches (sigil + aeon), byte-neutral; both acceptance vectors green.

---

## §4.9 `table` — counted / sentinel / sparse collections

A **contextual item opener** (S2-D1 headroom, same policy as `offsets`/`align`), valid at top
level or inside a `section {}`. Sibling of `offsets` (§4.7) — it SHARES the lowering machinery
(labeled inline bodies as real link symbols, pointer cells + integer holes, derived comptime
facts, the D2.29 align pad) but has a **disjoint cell byte contract** (typed cells: pointer /
record / scalar in section byte order, vs `offsets`' self-relative `dc.w target-base`).

### Grammar

```
table_item := "table" Name [":" "[" RowType "]"] ["(" attr ("," attr)* ")"] "{" row ("," row)* [","] "}"

attr  := "cell"       ":" PtrType            // index-table mode: emit a cell per key
       | "key"        ":" Lo "..=" Hi        // inclusive integer key domain (v1)
       | "hole"       ":" IntLiteral         // sparse: fill for absent keys (else exhaustive)
       | "header"     ":" Type "(" Expr ")"  // count header; Expr over the reserved `count`
       | "sentinel"   ":" Value              // trailing terminator row/value
       | "item_align" ":" N                  // self-adjusting pad after every emitted part
       | "body"       ":" "before" | "after" // payload placement vs the cell table (default after)

row       := [Key ":"] row_body
row_body  := part ("," part)*                // blob mode: labeled data parts
           | RecordLiteral                   // typed mode: a [RowType] record (§4.5 rules)
part      := Label "=" DataExpr              // the exact `data`-item shape (offsets D2.31 precedent)
```

A parts-row's `,` continues the row only when a `Label =` part follows; otherwise it separates
rows (keeps keyed multi-part rows unambiguous). Key-range bounds parse above the `..` operator
so `$33..=$B9` is read as an inclusive range, not a half-open `Expr::Range`.

### Two emission shapes

- **Record-list** (no `cell:`): `[header?] rows [sentinel?]` contiguous. `Name` labels the
  first byte — the header when present (AS-macro `__LABEL__` parity).
- **Index** (`cell: PtrType`): TWO streams — a **payload stream** (each row's labeled parts,
  declaration order, each part followed by the `item_align` pad) and a **cell table**
  (`[header?] one cell per key in the domain [sentinel?]`; a declared key → a pointer fixup to
  the row's FIRST label; an absent key → the `hole` literal). `body:` places the payload
  `before`/`after` the cell table (default `after`). **`Name` anchors the FIRST CELL** (after
  the header), so `Table[key − min_key]` indexing is correct — the header word(s) emit before
  `Name`'s anchor.

### Semantics

- `key:` + `hole:` = **sparse** (absent keys → hole). `key:` without `hole:` = **exhaustive**
  (every key in the domain must have a row; the error lists the missing keys). Duplicate keys,
  out-of-range keys, and non-ascending keys are hard errors. Keyed rows MUST be declared in
  ascending key order.
- `header: Type(Expr)` — a count word, `Expr` over the reserved name `count` (the derived row
  count): `header: u16(count - 1)` for the −1 macros, `header: u16(count)` for the raw ones.
- `item_align: N` reuses D2.29's align machinery verbatim ($00 fill, lowering-baseline
  computation, link-time congruence assert) — a self-adjusting pad after every emitted part.
  Its congruence asserts are STRUCTURAL (`[layout.align]`-tagged), excluded from the harness's
  drift-guard count like `[layout.odd-item]`; an EXPLICIT `align N` item stays a counted guard.
- **Derived comptime facts** (reserved member names): `Name.count` (row count), and in keyed
  mode `Name.len` (key-span = domain max−min+1, the cell-table element count), `Name.min_key`,
  `Name.max_key`. Plain comptime ints.
- **Typed rows** (`table Name: [RecType]`) follow §4.5 struct-literal rules exactly; part
  labels are ordinary module-scoped data labels (`pub`-able, real link symbols).

### Diagnostics

Missing-keys list (exhaustive); duplicate / out-of-range / non-ascending key; `hole:` value
that doesn't fit the cell; `header:` not comptime; index mode without `key:`; `hole:`/`body:`
without the required companion; a record row in index mode (auto-labeled keyed rows are
deferred — explicit labels only in v1).

### Machinery

**Front-end lowering only — NO `sigil-ir` / `sigil-link` change** (the design's core claim).
Reuses `embed()`/`Value::Data`/`.len`, labeled inline bodies (offsets D2.31), `[*u8; N]`
pointer cells + integer holes (D2.27), struct-literal rows (§4.5), and the D2.29 align pad
(`emit_align_pad` is shared with `align`). The index-mode cell table lowers through the exact
`[PtrType; N]` array path the hand-written `data SfxTable: [*u8; N] = [...]` used, so the
linker sees nothing new.

---

## Decision D2.36 (or next free row)

**`table` — counted / sentinel / sparse ROM-data collection.** Contextual item opener
(non-breaking under A-Spec2.3 post-freeze). Keyword `table`, `header: u16(count - 1)` spelling,
`body:` default `after`, ascending-key hard error, Name-anchors-first-cell in index mode — all
ratified 2026-07-11 (§7 of the design). Sibling of `offsets` (D2.31): shared lowering, disjoint
cell contract. Acceptance: byte-neutral `sfx_bank.emp` retrofit (both build shapes, cross-seam
`Sfx_NN` win-tab reads resolve) + a record-list PLC vector byte-diffed against the AS
`plrlistheader`/`plreq` macros.

**Scoping boundaries (out, each with its home):** packed pointer-composite cells
(`dur<<24|art` — a cell/fixup-level item); byte-script streams (the bytecode construct);
runtime collection kinds (T2-h); the Z80 win-tab (stays `.asm`, R3). **Deferred within
`table`:** auto-labeled keyed record rows (decision 6); interior header-end labels `NamePlc`
(decision 7, ledgered).
