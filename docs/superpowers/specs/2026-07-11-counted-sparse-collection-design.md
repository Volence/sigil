# Design — Counted / sentinel / sparse collection (`table`) · Spec 2 · Plan 7 T2-d

Written 2026-07-11 (Fable). **Surface ratified with Volence same day** — keyword `table`,
`header: u16(count - 1)` spelling, and the born-general acceptance bar confirmed directly;
`body:` default + mirror-const handling ruled autonomously under the standing arrangement
(§7 records each). **DESIGN ONLY** — no implementation code, no `.emp`/`.asm` bytes
changed. Worked against `sfx_bank.emp` but designed against the **full T2-d demand set** per the
research doc's R1/R2 coherence requirement (a construct built against one example
under-generalizes). The eventual acceptance test is the **byte-neutral retrofit of
`aeon/games/sonic4/data/sound/sfx/sfx_bank.emp`** through the core_port/sound harness byte gates.

References: research T2-d + R1/R2 + priority order
(`2026-07-06-sigil-spec2-p7-language-completion-research.md`); spec §4.7/§4.8/§4.5
(`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`, D2.15/D2.25/D2.27/D2.29/D2.31); the `offsets` design
(`2026-07-06-offset-table-design.md`); `aeon/.../sfx_bank.emp` + `sfx_table.asm`;
`skdisasm/sonic3k.macros.asm` + `Sonic-Clean-Engine-S.C.E.-/Engine/Macros.asm`.

## 1. Problem — one pattern, ten instances

The demand set is a single abstract shape — **`[count header] rows [terminator]`, optionally
fronted by a keyed index table** — hand-built ten different ways:

| Instance | Header | Rows | Sentinel | Index table |
|---|---|---|---|---|
| aeon `SfxTable` + blobs | — | 9 labeled blob+patch pairs, per-part `align 2` | — | 135 × `dc.l`, keys `$33..$B9`, **122 hand-typed `0` holes** |
| `plrlistheader`/`plreq` | `dc.w (End−Plc)/6 − 1` | 6-byte records (`dc.l art, dc.w vram`) | — | — |
| `watertransheader` | `dc.w (End−L−2)/2 − 1` | words | — | — |
| `zoneanimstart`/`decl`/`end` | `dc.w n−1` (back-patch var) | 8-byte records (packed `dur<<24\|art`) | — | — |
| `dbglistheader` (SCE) / `dbglistinclude` (skd) | `dc.w n` (**raw**, no −1) | 10/12-byte records (packed `frame<<24\|obj`) | — | — |
| `HScroll_Header` | `dc.w n−1` | records | — | — |
| `titlecardresultsheader` | `dc.w n−1` | records (`dc.l` code ptr + words) | — | — |
| `ObjectLayoutBoundary` | — | layout records | `dc.w -1,0,0,0` | — |
| S2 zone-ordered tables (568 entries, 28 decls) | — | **N slots per zone key**, dense, `!org` math + hand count warning | — | rows ARE the table |
| `offsets` (shipped, §4.7) | — | `dc.w target−base` | — | dense ordinal |

Two load-bearing observations:

1. **Every stride-division header collapses to row count.** `(End−Start)/6 − 1`, `/$A`, `/2` —
   the division exists only because AS can't count records; once rows are typed items, the header
   is just `count` or `count − 1`. The six macro *pairs* (open + back-patch close, or
   forward-referenced `set` variables) exist solely to compute that number. The whole back-patch
   dance deletes.
2. **The count/sentinel list and the sparse keyed index are the same construct with orthogonal
   knobs**, not two constructs: shared row model, shared derived facts, shared labeled-inline-body
   machinery (proven by `offsets` D2.31); they differ only in framing (header/sentinel) and in
   whether a key-addressed cell table is emitted over the rows.

What it is **not** (scoping boundaries, per R1/R2 — details in §6):
packed `flags<<24|label` *cells* (a cell-level gap, own item); `$FF`-terminated byte-*script*
streams (the T1-c/R2 bytecode-coroutine construct's turf); runtime collection kinds
(T2-h linked lists/pools — this is ROM emission only); the Z80 win-tab (stays `.asm` per the
sound-migration R3 ruling).

## 2. Prior art — sibling of `offsets`, deliberately not `offsets`

`offsets` (§4.7) already proves every hard mechanism this construct needs: the block name as a
real base label; **labeled member emission with inline bodies** (D2.31 — members are genuine link
symbols, readable cross-seam); derived comptime facts (`.count`, ordinals); reserved member
names; totality-checked cell emission.

It is a **sibling, not an extension**, because the two byte contracts are disjoint:

| | `offsets` | `table` (this design) |
|---|---|---|
| Cell | `dc.w target − base` (self-relative, signed-word-checked) | typed cells — pointer (`Abs32Be`), record, scalar; section byte order |
| Keying | dense 0-based ordinal, implicit | explicit sparse integer key / enum key, or unkeyed |
| Holes | impossible | `hole:`-filled (the 122 `dc.l 0`s) |
| Framing | none | optional count header (raw / −1) + optional sentinel |
| Per-item pad | none | optional `item_align:` |

Bolting five knobs onto `offsets` would kitchen-sink a shipped construct whose Sonic-idiom byte
contract (`dc.w t−base`) is load-bearing across 14k call sites. Tenet 1 is satisfied by **sharing
the lowering machinery**, not the keyword. (The research's `list<T>` name is dropped: nothing
here is a runtime list; it's a table in the ROM-data sense. Bikeshed in §7.)

## 3. Surface

`table` — a contextual item opener at item position (S2-D1 headroom policy, same rule as
`offsets`/`align`), valid at top level or inside a `section {}`.

```
table_item := "table" Name [":" "[" RowType "]"] ["(" attr ("," attr)* ")"] "{" row ("," row)* [","] "}"

attr  := "cell"       ":" PtrType            // index-table mode: emit a cell per key
       | "key"        ":" KeyDomain          // lo..=hi range, or an enum / offsets name
       | "hole"       ":" IntLiteral         // sparse: fill value for absent keys (else exhaustive)
       | "header"     ":" Type "(" Expr ")"  // count header; Expr over the reserved `count`
       | "sentinel"   ":" Value              // trailing terminator row/value
       | "item_align" ":" N                  // self-adjusting pad after every emitted part
       | "body"       ":" "before" | "after" // payload stream placement vs the cell table

row       := [Key ":"] row_body
row_body  := part ("," part)*                // blob mode: labeled data parts
           | RecordLiteral                   // typed mode: a [RowType] record (§4.5 rules)
part      := Label "=" DataExpr              // exact `data`-item shape (offsets D2.31 precedent)
```

Semantics:

- **Modes compose from the knobs; there are only two emission shapes.**
  - *Record-list mode* (no `cell:`): emits `[header?] rows [sentinel?]` contiguously; `Name` is
    the label at the first byte (the header, when present) — matching every AS macro, whose
    `__LABEL__` sits on the count word.
  - *Index mode* (`cell: PtrType`): emits **two streams** — the payload stream (each row's
    labeled parts, declaration order, each part followed by the `item_align` pad when configured)
    and the cell table (`[header?] one cell per key in the key domain [sentinel?]`; declared keys
    → pointer fixup to the row's **first label**; absent keys → the `hole` literal, D2.27-style
    int-in-pointer-cell). `body:` places the payload stream `before` or `after` the table.
    **`Name` anchors to the FIRST CELL, not the header word** (reviewer catch, 2026-07-11): a
    headered index table with `Name` on the header would silently off-by-header every
    `Table[key − min_key]` consumer — key indexing off the base label is the construct's whole
    point, so the header word(s) emit *before* `Name`'s anchor. (Record-list mode is the
    opposite and stays so: `Name` sits on the count word, matching every AS macro's
    `__LABEL__`.) A label on the index-mode header itself, if a consumer ever wants one, joins
    the ledgered `plc_label:`/interior-label knob class (§7 item 7). sfx_bank is header-less
    and unaffected; this ruling exists so the first headered-index instance can't misdesign it.
- **`key:` + `hole:` = sparse** — absent keys emit the hole. **`key:` without `hole:` =
  exhaustive** — every key in the domain must have a row; the error lists the missing keys (the
  S2 zone-table count-warning, promoted to totality). Duplicate keys are a compile error.
- **Keyed rows must be declared in ascending key order** (loud error). This makes the payload
  stream order deterministic and review-obvious, and the sfx retrofit already complies (blob
  include order == id order in the `.asm` stream).
- **`header:`** — the count word is an expression over the reserved name `count` (the derived
  row count, a plain comptime int): `header: u16(count - 1)` for the five −1 macros,
  `header: u16(count)` for the raw dbglist pair. Emitted at `Name`, before rows/cells. The
  expression form means no `bias:`/`adjust:` micro-knobs, and anything weirder stays writable.
- **`sentinel:`** — one terminator value/record emitted after the rows (record-list mode) or
  after the cells (index mode). A whole-row terminator (`[-1 as i16, 0, 0, 0]`), **not** an
  in-stream opcode — byte-script terminators belong to the bytecode construct.
- **`item_align: N`** — the S2-D16(c) `item_align:` idea landing scoped to the construct: a
  self-adjusting pad after **every emitted part**, reusing D2.29's machinery verbatim ($00 fill,
  lowering-baseline computation + link-time congruence assert). Declared once, byte-visible as
  one attribute — §4.3's no-*implicit*-alignment rule holds because it *is* declared.
- **Derived comptime facts** (reserved member names, like `offsets.count`): `Name.count` (row
  count), and in keyed mode `Name.len` (key-span = max−min+1 — the cell-table element count),
  `Name.min_key`, `Name.max_key`. All plain comptime ints (D2.15's no-coercing-enum rationale
  carries over).
- **Typed rows** (`table Name: [RecType] (...)`) follow §4.5 struct-literal rules exactly —
  named-every-field, pointer fields lower to the right fixup kind, section CPU byte order. No
  new cell semantics: a row is a struct literal; the construct only adds counting/framing/keys.
- Labels declared in parts are **ordinary module-scoped data labels** — `pub`-able, visible to
  the AS seam as real link symbols (the hard constraint: the Z80 win-tab's
  `dw sfx_winptr(Sfx_NN)` keeps resolving).

### Worked example — sfx_bank re-expressed

```emp
module data.sfx_bank

section sfx_bank (cpu: m68000, bank: $8000) {
    // 9 SFX: one row per id. Payloads emit first (blob, pad, patches, pad — the
    // .asm include stream), then the sparse id->blob table (135 cells, holes 0).
    table SfxTable (cell: *u8, key: $33..=$B9, hole: 0, item_align: 2, body: before) {
        $33: Sfx_33 = embed("sfx_33.bin"), Sfx_33_Patches = embed("sfx_33_patches.bin"),
        $34: Sfx_34 = embed("sfx_34.bin"), Sfx_34_Patches = embed("sfx_34_patches.bin"),
        $35: Sfx_35 = embed("sfx_35.bin"), Sfx_35_Patches = embed("sfx_35_patches.bin"),
        $36: Sfx_36 = embed("sfx_36.bin"), Sfx_36_Patches = embed("sfx_36_patches.bin"),
        $3C: Sfx_3C = embed("sfx_3C.bin"), Sfx_3C_Patches = embed("sfx_3C_patches.bin"),
        $62: Sfx_62 = embed("sfx_62.bin"), Sfx_62_Patches = embed("sfx_62_patches.bin"),
        $AB: Sfx_AB = embed("sfx_AB.bin"), Sfx_AB_Patches = embed("sfx_AB_patches.bin"),
        $B6: Sfx_B6 = embed("sfx_B6.bin"), Sfx_B6_Patches = embed("sfx_B6_patches.bin"),
        $B9: Sfx_B9 = embed("sfx_B9.bin"), Sfx_B9_Patches = embed("sfx_B9_patches.bin"),
    }
}

// Section-level facts UNCHANGED — the construct is an ordinary section item:
// bank: $8000 (the main.asm:252 straddle fatal) stays a section property; the
// :260 co-residency fatal stays this ensure, spelled bankid-vs-label as today.
ensure(bankid("Sfx_33") == bankid("MovingTrucks_Bank_Start"),
    "SFX blobs not co-located with the engine-table bank — Sfx_Frame's dispatch/table reads would see the wrong bank")
```

~196 lines → ~20. **Adding an SFX = one row** — the pointer cell, both labels, both pads, the
hole re-flow, and the derived `len`/`count` all follow. `sfx_table.asm`'s four-place checklist
loses its two `.emp` places (the SfxTable transcription + the mirror-const bumps) — what remains
is the `.asm` side (transcode tool, main.asm includes, sound_ids.asm), outside `.emp`'s reach
until the sound engine itself migrates.

The three mirror consts become derived (or stay hand-written and merely *checkable* —
open question 5): `SFX_ID_BASE == SfxTable.min_key`, `SFX_COUNT == SfxTable.count`,
`SFX_TABLE_LEN == SfxTable.len`. Nothing ties them to `sound_ids.asm` at comptime either way
(bare cross-seam equ reads still don't exist — the bankid-label idiom note in sfx_bank's header)
— but they are now *derived from the rows* instead of hand-counted alongside them, so `.emp`-side
drift is impossible by construction.

## 4. Byte-neutral mapping — construct output vs current sfx_bank.emp, line by line

| Current `.emp` item | Construct output | Bytes |
|---|---|---|
| `const SfxNNBlob/Patch = embed(...)` ×18 | inlined into rows (the consts existed to make `.len` readable for the hand pads; `item_align` obsoletes that) | none (consts emit nothing) |
| `data Sfx_33 = Sfx33Blob` | row `$33` part 1: label `Sfx_33` + 58 blob bytes | identical |
| `data _p33 = if len%2==1 {byte(0)} else {Data.empty}` | `item_align: 2` pad after part 1 → 58 even → **0 bytes** | identical |
| `data Sfx_33_Patches = Sfx33Patch` | part 2: label + 32 bytes | identical |
| `data _q33` (…and every other even pad) | pad → 0 bytes | identical |
| `data _p3C` after `Sfx_3C` (267 B, **odd**) | pad → **one `$00`** | identical |
| `data _pB6` after `Sfx_B6` (71 B, **odd**) | pad → **one `$00`** | identical |
| rows `$34…$B9` in id order | payload stream in declaration order == ascending key order == the `.asm` include stream (blob, pad, patches, pad; ids ascending, contiguous, §4.3 no auto-pad between rows) | identical |
| `data SfxTable: [*u8; SFX_TABLE_LEN] = [...]` (9 syms + 122 hand `0`s) | cell table after the payload stream (`body: before`): 135 × 4-byte cells; declared keys → `Abs32Be` fixup to `Sfx_NN`; the 122 holes → `hole: 0` literal | identical — lowers to **the same `[*u8; N]` cell stream** the hand table lowers to today (D2.27); the linker sees nothing new |
| `const SFX_TABLE_LEN = 135` consumed by the array type | `SfxTable.len = $B9−$33+1 = 135`, structurally correct (a wrong row count can't *mis-size* the table — it changes `count`, not `len`; a key outside `$33..=$B9` is a range error) | n/a (comptime) |
| section `bank: $8000`, the `ensure` | untouched — outside the construct | identical |

Residual differences, all non-ROM: the `_pNN`/`_qNN` items today are *named* data items and
register symbols; the construct's pads are anonymous. **VERIFIED acceptance-neutral, not
assumed (reviewer check, 2026-07-11):** the debug-ROM hazard (convsym `-a` appends the symbol
table into the debug tail) was grepped against `s4.debug.lst` — **zero `_pNN` symbols exist in
the reference**, because the reference build is gate-off `.asm` (bare `align 2`, no named pad
items). The pad symbols are `.emp`-only scaffolding absent from the reference; anonymizing them
moves the symbol table TOWARD the reference, and the ROM-region bytes are identical either way.

One implementation subtlety inherited from D2.29, not new: the section base is map-pinned per
shape (plain `$63AE8` / debug `$6553A`, both even, delta even), so every `item_align: 2` pad
computes identically in both shapes and the recorded congruence asserts hold. A future shape
whose base went odd would fail loudly at link — the right behavior.

## 5. Machinery: reused vs new

**Reused (the heavy lifting already exists):**
- `embed()` / `Value::Data` / `.len` — payload bodies are ordinary data expressions.
- **Labeled inline bodies as real link symbols** — `offsets` D2.31 shipped exactly this
  (member label registration, in-block self-reference, cross-module visibility).
- **Pointer cells + integer holes** — `[*u8; N]` with int literals is D2.27, and is *literally
  what the retrofit target lowers to today*; `Abs32Be` fixups and the linker fold are untouched.
- **Typed record rows** — §4.5 struct literals, field→fixup lowering, section byte order.
- **Pads** — D2.29 `align` machinery (zero fill, baseline computation, link congruence asserts).
- **Derived comptime facts** — the `offsets` reverse-direction constant registration
  (`.count` et al. as plain comptime ints, reserved-name rule).
- **Section facts** — `bank:` (D2.25), `ensure`/`bankid` (D2.20/D2.25): orthogonal, unchanged.

**New (the actual build):**
- Grammar: the `table` contextual opener, attribute set, keyed rows, multi-part row bodies.
- Comptime checking: key domain/range/uniqueness/ascending-order, exhaustive-vs-sparse rule,
  header-expression evaluation over `count`, sentinel typing.
- Lowering orchestration: synthesizing the two streams (payload + cell table) in `body:` order,
  hole synthesis over the key span, header/sentinel emission, `item_align` pad insertion points.
- Diagnostics: missing-keys list, duplicate/out-of-range/unordered key, `hole:` value that
  doesn't fit the cell, `header:` expression not comptime, reserved member names.

Nothing new in `sigil-ir` or `sigil-link` — by design. The construct is a **front-end lowering**
onto existing fragment/fixup/align machinery; that is the strongest available argument that the
byte-neutral claim will hold (the retrofit emits through the identical back-end path the current
hand-written file uses).

## 6. Coverage check — the full demand set

| Demand | Mode + knobs | Verdict |
|---|---|---|
| **sfx_bank** (acceptance target) | index: `cell: *u8, key: $33..=$B9, hole: 0, item_align: 2, body: before` | ✅ §3–4 |
| **`plrlistheader`/`plreq`** (PLC lists) | record list: `header: u16(count - 1)`, `[PlcReq]` with `struct PlcReq { art: *u8, vram: u16 }` (6-byte stride ✓) | ✅ — one caveat: the AS macro also defines the interior label `NamePlc` *after* the header; the count math needed it, the construct doesn't. **Retrofit check:** if any code references `NamePlc`-style labels, that's a `header_end`-label knob (open question 7); grep before the first PLC port. |
| **`watertransheader`** | record list: `header: u16(count - 1)`, rows `[u16]` (row = one word, so the macro's `/2` byte-math == count) | ✅ |
| **`zoneanimstart`/`decl`/`end`** | record list: `header: u16(count - 1)`, 8-byte records | ⚠️ fits **modulo the packed cell**: `dc.l (duration&$FF)<<24\|artaddr` is a link-time `imm<<24 \| label` composite — a *cell-level* gap (see boundary below), not a collection gap. The open/close macro pair + the `zoneanimcount` back-patch variable delete regardless. |
| **`dbglistheader`** (SCE) / **`dbglistinclude`** (skd) | record list: `header: u16(count)` — the **raw** count proves the header-as-expression knob earns its generality | ⚠️ same packed-cell caveat (`frame<<24\|obj`); skd's `include path` body becomes inline rows (`.emp` has no textual include — the module system is the answer, same as everywhere) |
| **`HScroll_Header`** | record list: `header: u16(count - 1)` | ✅ |
| **`titlecardresultsheader`** | record list: `header: u16(count - 1)`, records mixing `*code` pointers + words | ✅ (code-pointer cells = `Abs32Be`, same as data) |
| **Boundary sentinels** (`ObjectLayoutBoundary = dc.w -1,0,0,0`) | `sentinel: [-1 as i16, 0, 0, 0]` on the layout list | ✅ for authored tables; aeon object layouts are binary `embed()`s today, so this fires on classic-Sonic ports, not the campaign |
| **Enum-indexed, N slots per key** (S2 zone-ordered: 568 entries / 28 decls / `!org` math) | keyed dense: `key: ZoneId` (an enum or `offsets` ordinal set), **no `hole:`** → exhaustive, row type `[u16; 2]` (acts per zone) | ✅ — missing-zone rows become a compile error listing the zones (the hand count-mismatch warning, promoted); insertion re-flows by key, killing the `!org` math |

**Scoping boundaries (explicitly out, each with its home):**

1. **Packed pointer-composite cells** (`dur<<24|artaddr`, `frame<<24|obj`, `levartptrs`'
   `plc<<24|art`) — a *cell/fixup-level* feature: a link-time expression fixup
   (`imm<<24 | Abs`), sibling of the win-tab's proven `(Sym & mask) | base` → `Value16Le` and
   t10's imm-link-with-pinned-abs. Its own small design item; **feeds this construct but is not
   part of it.** Until it lands, zoneanim/dbglist rows can't be fully typed — the collection
   framing still applies, the record interior waits. (Gap-ledger row material.)
2. **Byte-script streams** (`$FF/$FE/$FD/$FC`-terminated animation/palette scripts, SMPS) — the
   T1-c/R2 bytecode-coroutine construct. `sentinel:` here is a whole-*row* terminator on a
   homogeneous record list; an in-stream *opcode* terminator with per-command validity is a
   different, larger machine. Do not stretch `table` there.
3. **Runtime collection kinds** (T2-h — linked lists, pools, priority bands) — RAM-side
   engine-architecture features. `table` is ROM data emission only.
4. **The Z80 win-tab** (`sfx_blob_win_tab.asm`) — stays driver-side `.asm` per the
   sound-migration R3 ruling; it keeps reading the row labels cross-seam, which is precisely why
   parts must be real link symbols.
5. **`offsets`** — remains its own construct (§2); no subsumption, shared lowering only.

Cell byte order rides §4.5 (section CPU order; `BankPtr16Le`-class kinds exist), so unlike
`offsets` there is **no 68k-only restriction** — but no Z80 instance is on the campaign path
(the win-tab is out per R3), so v1 test coverage is 68k and Z80 comes free-or-later.

## 7. Decisions (ratified 2026-07-11) + remaining ledger items

1. **Keyword: `table` — RATIFIED (Volence, 2026-07-11).** ROM-data sense, reads right next to
   `offsets`/`dispatch`; the research's `list` implied runtime semantics it doesn't have.
   Contextual opener, zero breakage (S2-D1 headroom).
2. **Header spelling: `header: u16(count - 1)` — RATIFIED (Volence, 2026-07-11).**
   Type-application form, matches `byte(0)`.
3. **`body:` default: `after` — RULED (Fable, autonomous per the standing arrangement;
   Volence neutral).** One rule shared with §4.7 inline-offsets (bodies emit after the table)
   beats two sibling constructs with opposite defaults; also the header-first shape of most AS
   lists. sfx_bank writes `body: before` explicitly — deliberately so: the one byte-layout fact
   its correctness hangs on stays visible on the page.
4. **Ascending-key requirement: hard error — RULED (Fable).** Deterministic payload order, zero
   ambiguity; relax later only if a real port needs interleaved payload order (none in the
   demand set does).
5. **Mirror consts in the retrofit: derived reads — RULED (Fable, autonomous; Volence
   neutral).** `const SFX_COUNT = SfxTable.count` etc. — zero bytes, keeps the names
   documenting the id contract with `sound_ids.asm` (as the current header goes out of its way
   to do), and the hand-sync hazard is gone since they derive from the rows.
6. **Auto-labeled keyed record rows** (index mode over typed records without explicit part
   labels — cell points at an auto `Name.$33` row label): **DEFERRED** — no demand instance
   needs it; explicit labels only in v1.
7. **Interior header-end labels** (`NamePlc`): **LEDGERED, not built** — only if a
   classic-Sonic port finds real references to them; then a small `plc_label:`-style knob.
8. **Spec placement** (for the implementation tranche): new §4.9 alongside §4.7/§4.8, decision
   row D2.36-or-next, per A-Spec2.3 post-freeze amendment (contextual opener ⇒ non-breaking).

## 8. Acceptance

This document is **design only**. The construct ships when:
1. Re-expressing `sfx_bank.emp` per §3's worked example produces a **byte-identical ROM** through
   the existing core_port/sound harness gates (both build shapes), with the cross-seam
   `Sfx_NN` reads from `sfx_blob_win_tab.asm` still resolving; and
2. at least one **record-list-mode** vector (a `header: u16(count - 1)` PLC-shaped table
   byte-diffed against the AS macro output) proves the second emission shape — so the construct
   is born generalized, not sfx-shaped (the R1/R2 requirement this design exists to satisfy;
   **explicitly ratified by Volence 2026-07-11**: general over sfx-shaped).
