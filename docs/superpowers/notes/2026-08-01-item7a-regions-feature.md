# 2026-08-01 — ITEM #7a: the `vars` region-form feature (implementation)

Status: **Checkpoint for the overseer's countersign.** Branch `item7a-regions`
(sigil only; aeon untouched — the feature is UNUSED until #7b). Not merged.

Builds the ratified spec `docs/superpowers/specs/2026-08-01-item7-ram-regions-design.md`
(§2 surface, §2.2–§2.4 lowering, §5 diagnostics, §6 the #7a parcel). The region
form stops being inert: `region` items + region-form `vars` blocks now allocate
real, deterministic RAM addresses and emit reserve-only sections.

## §0 — HEADLINE

The `vars` region form is LIVE. `region name @ base .. limit` (with
`after(<region>)` chaining) + region-form `vars` blocks (`pad`/`mark`/`alias`/
`@align`/conditional groups) resolve to exact addresses via the existing comptime
layout machinery, emit reserve-only Core sections (VMA-placed, zero image bytes),
and run all eight §5 checks. **All six aeon targets remain byte-identical to
chain-9** (feature unused → zero placement effect). Strict suite **2864 / 0 / 4**
(2846 baseline + 18 new tests).

## §1 — WHAT SHIPPED, PER SPEC SECTION

### §2.1 the `region` item (grammar + resolution)
- `pub region name @ base .. limit [, w_addressable]`, plus the
  `@ after(<region>) ..` chained-base form. New AST `Item::Region(RegionDecl)` /
  `RegionBase::{Addr, After}`; parser `region_decl` (base parsed at bp 5 so the
  `..` separator is never swallowed as a range expr).
- Registry with `[region.duplicate]` (declared more than once) and
  `[region.unknown]` (a `vars`/`after(..)` naming an undeclared region).

### §2.2 region-form `vars` field kinds (grammar + lowering)
- Region-form fields now live in `VarsDecl.region_body: Vec<RegionField>`
  (ordered — allocation order is load-bearing); the OVERLAY form keeps `fields`
  untouched (its tests stay green). `RegionField` = `Typed | Pad | Mark | Alias |
  Group`.
- Typed fields (`name: T [@align(N)]`) — primitive / `[T; N]` (any comptime N) /
  struct / `[Sst; N]`, sized via `size_of_type`/`layout_of_struct`; define a
  link-visible label at the running address.
- `pad(N)` — anonymous reserve advance. `mark Name` — zero-size label.
- `name: alias(Other)` — a pure equate to another field's comptime-known address
  (lowered as an `EquSym { name, Expr::Int(target_addr) }`, matching AS `Name =
  Other`). Forward refs allowed (resolved against the completed address map).
- `@align(N)` — RESERVE advance to the next N-boundary of the region-ABSOLUTE
  address (no fill; RAM). Distinct from the ROM `align N` item.
- Conditional groups `if <cond> [@shape_divergent] { .. } [else { .. }]` — the
  comptime condition selects an arm; both arms measured for the shape check.

### §2.3 layout rules
- No auto-alignment. `[layout.odd-field]` lint (WARNING) on a word-or-wider field
  (u16/u32/i16/i32/u16le/ptr/wide-fixed, or an array/struct/tuple containing one)
  placed at an odd region-absolute address, via the new
  `Evaluator::ty_needs_even` (mirrors `size_of_ty`'s recursion + cycle guards).
- `[region.overflow]` — running end past `limit`, message names the over-by byte
  count AND the field that crossed.
- `[vars.shape-divergent]` — a size-varying conditional group without
  `@shape_divergent`; size-EQUAL arms are proven invariant (compared by measured
  size) and need no annotation.
- `[region.not-w-addressable]` — `w_addressable` window whose bytes are not all
  reachable by sign-extended `.w` (low word bit 15 clear, or the window crosses a
  64K page). Subsumes the per-symbol `Object_RAM & $FFFF < $8000` guard.

### §2.4 emission
- Each region → one reserve-only section named for the region, opened at
  `vma_base = lma = base` with `Reserve` fragments (zero image bytes → skipped by
  `sigil-link::flatten`) + labels/marks; aliases as `EquSym`s on the carrier.
- `pub vars` region field/mark/alias names export like ordinary labels
  (`collect_exported`/`collect_defined` walk `region_body`, recursing into
  groups) — §3.3 cross-seam by bare name.

### §3 semantics
- Determinism: layout is a pure function of (region decls, source order, comptime
  defines) — no link-order input (tested).
- Chaining: `after(<region>)` resolved in after-DAG topological order (memoized,
  recursive), `[region.chain-cycle]` on a cycle.
- `[region.multiple-owners]` (§2.3): a whole-program check
  (`lower::check_single_owner`) wired into `build_program_with`, no-op when no
  region vars exist.

## §2 — SPEC-VS-REALITY (recorded; overseer countersigns)

**None are silent deviations. Two are decision points worth a ruling; the rest
are boring-option records.**

1. **The conditional-group condition spelling (`if __DEBUG__`).** The spec §2.2
   writes `if __DEBUG__ @shape_divergent`. I implemented the condition as an
   ARBITRARY comptime expression evaluated in the seeded `-D` define environment
   (`eval_expr` → nonzero = the `then` arm). This is a strict SUPERSET of the
   spec spelling, but the literal `if __DEBUG__` only works if `__DEBUG__` is a
   SEEDED define — an *undefined* bareword condition is a "not a comptime integer"
   error, NOT silently false (deliberate: matches the shipped emp convention and
   refuses to mask a typo). **The shipped engine convention is `-D DEBUG=0|1`
   always-seeded + `if DEBUG == 1`** (every existing `.emp` conditional:
   `engine/objects/rings.emp:84`, `engine/sound/sound_api.emp`, …), NOT the
   AS `ifdef __DEBUG__` presence test. **Recommendation:** #7b/#7c port the two
   `ifdef __DEBUG__` blocks as `if DEBUG == 1 @shape_divergent` (the shipped
   convention), and the spec §2.2 example is updated to `if DEBUG == 1`. If the
   overseer instead wants AS-faithful presence semantics (`if defined(NAME)`),
   that is a small follow-up (a `defined(..)` builtin or undefined-as-false in
   the group condition only) — flagged for a ruling, not taken unilaterally.

2. **Cross-module `after()` is a #7c hook (not built in #7a).** #7a resolves
   regions per lowered FILE — every region a `vars`/`after` references must be
   declared in the same file. This fully serves **#7b** (engine: `lower_ram` +
   `upper_ram`, both FIXED bases, no cross-module `after`) and the intra-file
   `after` DAG is fully realized + tested. **#7c** introduces the cross-module
   chain (`game_ram @ after(upper_ram)` — game module chaining after the engine's
   region), which needs the parent region's end visible across modules. This is
   squarely §6's #7c scope ("the game ports"); I flag it as the remaining
   cross-module hook rather than half-building it here. `[region.multiple-owners]`
   IS built whole-program and tested directly.

3. **Placement integration deferred to #7b.** A reserve-only RAM section would
   fail `resolve::place_sections` (which matches section names to ROM map
   regions). Since #7a's feature is UNUSED by aeon, no RAM section reaches
   placement (the six CRCs prove it). #7b must teach `place_sections`/
   `place_sequential` to SKIP RAM sections (`vma_base >= $F00000`). I left
   placement UNTOUCHED — minimal blast radius, zero CRC risk.

4. **`[layout.odd-field]` is a WARNING.** Spec §2.3/§5 call it a "lint"; I made it
   `Level::Warning` (parity with the shipped `[layout.odd-item]` word-data lint).
   The overflow/w-addressable/shape-divergent/duplicate/unknown/cycle/
   multiple-owners diagnostics are ERRORs.

5. **`region_body` split.** Region-form fields moved to a new
   `VarsDecl.region_body` (the overlay form's `fields` is untouched); the one
   existing parser test's region-form assertions were updated to the new shape
   (overlay assertions unchanged).

## §3 — NEW-TEST INVENTORY (18)

`tests/parser_decls.rs` (+3, plus the updated `vars_region_and_overlay_forms`):
`region_item_forms`, `region_vars_all_field_kinds`, `region_vars_conditional_else`.

`tests/region_lower.rs` (15):
- §4 rows: `scalar_and_array_widths`, `struct_and_struct_array_sizes`,
  `mark_alias_pad_align`, `conditional_group_shape_divergent`,
  `conditional_group_shape_invariant_needs_no_annotation`.
- Chained fixture: `chained_engine_game_fixture` (two fixed regions + a chained
  third modeled on `engine/ram.asm`; marks, alias, pads, `@align(256)`, a
  `@shape_divergent` debug group; release + debug shapes, every address
  hand-computed and asserted exactly).
- `determinism_same_input_same_addresses`.
- Every §5 diagnostic (positive + negative): `region_duplicate`,
  `region_unknown`, `region_chain_cycle`, `region_overflow` (asserts the crossing
  field is named), `region_not_w_addressable`, `vars_shape_divergent`
  (positive + annotated-negative + size-equal-negative), `layout_odd_field`
  (positive + pad-fix-negative + byte-array-negative), `multiple_owners_check`.

## §4 — THE SIX-CRC IDENTITY PROOF (feature unused = byte-identical)

Built with the worktree `sigil`/`emit_sound_blob`, one shape per invocation:

| target | built | chain-9 tip | match |
|---|---|---|---|
| s4 | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ |
| s4.debug | `16615e46` / 421958 | `16615e46` / 421958 | ✓ |
| demo | `9bb8c993` / 90506 | `9bb8c993` / 90506 | ✓ |
| demo.debug | `bc7678d0` / 93006 | `bc7678d0` / 93006 | ✓ |
| config_a | `78df5e6a` / 422297 | `78df5e6a` / 422297 | ✓ (strict gate) |
| config_b | `f38f609b` / 303501 | `f38f609b` / 303501 | ✓ (strict gate) |

`lower_regions` early-returns for any file with no `region` items / region-form
`vars` (every aeon module today), so region-free lowering is byte-identical by
construction; `check_single_owner` is a pure no-op with no region vars.

## §5 — STEP-3 (retrospect) vs STEP-5 (engine-opt) FINDINGS

- **Step-3 (feeds the language-ask round):** the condition-spelling gap (§2 item
  1) is the one design question — the shipped `DEBUG`-seeded convention vs the
  spec's `if __DEBUG__` presence-test spelling. Recommend the note's resolution
  (`if DEBUG == 1 @shape_divergent`) OR a `defined(..)` ruling. Also: the region
  layout reuses `size_of_type`/`layout_of_struct` verbatim — no new sizing path,
  so struct/newtype/array sizing stays single-sourced.
- **Step-5 (engine opt):** none. No existing lowering changed; the six CRCs are
  unmoved.

## §6 — GAP-LEDGER SWEEP + KILL-LIST

- **Gap-ledger `vars / RAM regions` section (rows @ ~140–176):** the three OPEN
  blockers the 2026-08-01 conv-C finding named are now IMPLEMENTED by this
  parcel — region-allocation lowering + reserve-`@align` + the overflow/bit-15
  checks (the PRIMARY blocker), AND conditional `vars` fields + the layout-
  stability lint (the SECOND blocker, `@shape_divergent` + `[layout.odd-field]`).
  Cross-region base chaining is realized intra-file (cross-module = #7c). I did
  NOT edit the ledger — formal closure of these rows is #7c's explicit scope
  (§6: "gap-ledger 153/157/165 rows close" rides the census close). No new
  nice-to-have was left unimplemented (all §4 rows shipped), so no new jotted row.
- **Kill-list:** no closures. The region-`vars` scaffolding has no twin-mirror
  row (the feature is new, not a port seam); it will acquire one only when #7b
  wires the placement RAM-skip and #7c the cross-module chain.

## §7 — NON-CLEAN / OPEN

- Cross-module `after()` + placement RAM-skip are #7b/#7c hooks (§2 items 2–3),
  intentionally not built here.
- The condition-spelling ruling (§2 item 1) awaits the overseer.
- Everything else is clean: strict 2864/0/4, six CRCs identical, aeon tree
  untouched (only build artifacts), sigil main checkout clean, clippy clean on
  the new code.
