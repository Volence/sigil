# Shared Sec/Act struct module — MERGE PACKET (row 1051 micro-batch)

**Branch:** `refactor/shared-struct-module` (aeon + sigil). **For Volence's
countersign → `--no-ff` merge both repos (paired-state, pushed together).**
The row-1051 consolidation Fable ordered before t17 step 1. **Byte-neutral
throughout** — the batch changes only `.emp` files + their sigil test harnesses;
the shipped ROMs are the AS twins, untouched.

## Provenance / gates
- **ROMs byte-identical**: plain **453087 / crc32 b335bdc6**, debug **461110 /
  827e18c4** — verified after every item (repin `pins.rs unchanged` each time).
- **Full strict suite**: `SIGIL_STRICT_GATE=1 AEON_DIR=<worktree> cargo test
  --workspace` = **2257 / 0** (2252 baseline + 5 new tests: 3 in
  `struct_field_disp_plus_n.rs` + 2 in `structs_module.rs`). Run with `AEON_DIR`
  → `.worktrees/shared-struct-module` (paired-state gate), not aeon master.
- **Provenance = CRC32 + byte size** (campaign standard).

## Per-item table

| Item | What | aeon | sigil |
|---|---|---|---|
| 4 | `Act.grid_w+1` disp composition probe (WATCH SITE) — resequenced FIRST | — | `61b0e3e` |
| 1+2 | `engine/structs.emp` — type-only Act/Sec twins + per-field `offsetof==extern` drift wall (34) + sizeof guards; `act_sec_field_equs()` + `structs_module.rs` | `e1badfe` | `b7ef117` |
| 3.1 | act_descriptor.emp → `use engine.structs` (emission side; proves the move byte-neutral) | `ba05da9` | `c4e332d` |
| 3.2 | section.emp unwind + shared `Act_grid_w_lo`/`Act_grid_h_lo` (Fable Option-2 ruling) | `96bae91` | `050b05b` |
| 3.3 | tile_cache.emp unwind (the flip-harness file — took the recipe with NO resistance) | `8c84e87` | `949fca2` |
| 3.4 | entity_window.emp unwind (last consumer; shared grid-lo consts) → **mirror class EXTINCT** | `0cb52c6` | `4acaab0` |
| 7 | row-1068 enumeration correction + kill-linkage → RE-JUDGE; loop-doc idiom line | — | `9788738` |
| 6 | hoist `TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE}` → engine.constants twin | `8e1784d` | `3d8e738` |
| 5 | SectionId/GridCoord newtypes — **BALLOON → DEFERRED** (sketch, not diff) | — | `b487f83` + `b59a0f3` |

(Interleaved sigil doc/plan commits: `554736c`/`791d641`/`a368d7d`/`3338c70`/
`534a9dd` plan-anchor, `d83f1ca` loop-doc.)

## The item-4 finding + the 3.2 enumeration correction (Fable-owned miss)
- **Item 4** (Fable resequenced it FIRST, correctly): does `Struct.field + N` compose
  as a `d16(An)` displacement? **RED** — `Act.grid_w + 1(a2)` → `unknown name
  Act.grid_w` (the parser reads the field access as a bare Sym inside disp arithmetic;
  the `.field`-in-disp sugar only handles a BARE `Struct.field(An)`). Byte-neutral
  fallback `offsetof(Act, grid_w) + 1` composes identically (`12 2A 00 05`). Persistent
  3-test gate artifact + row 1068 + language ask deferred.
- **3.2 enumeration correction** — the item-4 file-scoped check UNDERCOUNTED: the
  pattern-enumeration rule (applied to the OWN ruling) found the field+N idiom at
  **10 sites / 3 consumers / 2 fields** (`grid_w+1` AND `grid_h+1`), not 2 in one file.
  Fable owned the miss (a file-scoped check where an enumeration was owed). PARK-AND-
  ENUMERATE surfaced it before any code shipped; Fable's **Option-2 ruling** put the two
  derived consts SHARED in engine.structs (rule-1 amended: a multi-consumer blessed
  sub-field view is layout-adjacent, not consumer-specific).

## Item-5 balloon → deferred (the two rulings)
The SectionId/GridCoord seam is bounded (`FlatIDXY`/`GetSecPtrXY` ← entity_window ×4),
but typing register-flow values has NO corpus mechanism: procs are `proc Foo ()` +
`// In:` comments + untyped `out(dN)`; `let rN: Type` has zero usage. Building it would
need either a new typed-asm-proc-register-signature feature or first-at-scale `let
rN:Type` (documentary, unenforced across a jbsr). **Deferred whole** (Fable ruling).
- **Row 1054 EXTENDED** — premise falsified (seam crosses in registers), re-keyed to
  the feature below, stays OPEN.
- **NEW verb-(c) row** — typed asm-proc register signatures (`proc FlatIDXY(d2:
  GridCoord) out(d0: SectionId)`), the register-CONTRACT system extended from naming to
  typing; row 1054's real unblock; design seed = the item-5 sketch; demand data = the
  seam TODAY + a TO-RUN corpus `// In:`/`// Out:` comment census. Its own effort, Volence's call.

## Batch wins
- **The Sec/Act file-local offset-const mirror class is EXTINCT** — all 4 engine
  consumers (`act_descriptor`, `section`, `tile_cache`, `entity_window`) import
  `engine.structs`; grep-confirmed 0 `const Sec_*`/`const Act_*` remaining.
- **Per-field drift wall** — `offsetof(S,f)==extern("S_f")` for every named field
  (strictly stronger than the old sizeof-only guard; catches a swapped same-size
  neighbour — proven by the `doctored_field_offset_fires_its_guard` negative probe).
- **`sizeof(Sec)`-derived stride guards** — section + tile_cache's `Sec_len==66` ×66
  stride guards now read `sizeof(Sec)==66` — the guard derives from the truth it
  protects, not a mirrored const.
- **`Act_grid_w_lo`/`Act_grid_h_lo` naming upgrade** — the cryptic `Act_grid_*+1`
  (low byte) is now a named, `offsetof`-derived, self-documenting const shared by 3
  consumers.
- **TILE_CACHE_* deferral discharged** — the 4 geometry consts hoisted to the shared
  twin; tile_cache.emp's "hoist is deferred to the same wave" comment REWRITTEN to the
  new state (no stale claim survives).

## R1 trip-check outcome
R1 (the t17-gate rider) predicted section.emp/tile_cache.emp would each plant a THIRD
file-local mirror of `TILE_CACHE_*`, deferred to "exactly this moment." OUTCOME: the
hoist rode the batch as item 6 — both files' 4 mirrors + 4 ensures each killed,
`use engine.constants.{TILE_CACHE_*}`, byte-neutral. The `engine_constant_equs()` ripple
was self-consistent (`twin_guards()` derives from its len, auto-adjusting every
`X+twin_guards()` assertion; only one hardcoded `53→57` count needed a touch).

## Rows closing / opening
- **Row 1051 CLOSED** — the shared Sec/Act struct module shipped; unwind set
  (entity_window + section + tile_cache offset consts + act_descriptor's twins) fully
  discharged.
- **Row 1054 EXTENDED-open** (above). **New typed-register-signature row OPEN.**
- **Row 1068 CORRECTED** (10-site enumeration; kill = re-judge).
- Kill-list: the Sec/Act mirror rows (7/8-class per-file) collapse — the shared module
  is the new home (dies at Spec-5 twin retirement, like sst.emp).

## Riders to master (topology CORRECTION)
- **The t17 step-0 note (`5739388`) is ALREADY on sigil master** — it is the merge-BASE
  (committed directly to master before the branch). It does NOT ride; Fable's gate note
  said it merges with the batch, but the true topology is that it's already there.
- **What DOES ride** with the batch merge: the loop-doc amendments (`d83f1ca` idiom
  line + `9788738` item-7 touch-up), the plan/anchor notes, the ledger rows, the item-5
  sketch — all on the refactor branch above the merge-base.

## Merge mechanics
- aeon `refactor/shared-struct-module` (6 commits) → aeon master; sigil branch (17
  commits) → sigil master. Both `--no-ff`, pushed TOGETHER (paired-state; coupled
  masters, no stale window).
- Post-merge: canonical provenance UNCHANGED (byte-neutral batch) — plain
  453087/b335bdc6, debug 461110/827e18c4 stay the reference.
- THEN: t17 step 1 begins (plane_buffer.emp transcribe) from the merged master, with
  its consumers now on the shared struct module.
