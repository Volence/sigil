# Shared Sec/Act struct module — micro-batch PLAN (row 1051, pre-t17)

**Why now:** Fable's t17 step-0 gate (2026-07-16, PASS w/ riders) ruled the
row-1051 shared-struct consolidation runs **serially BEFORE t17 step 1** — it edits
section/tile_cache/entity_window.emp, the files the t17 flip test compiles, so it
must land first. Volence-approved. Own branch `refactor/shared-struct-module` (both
repos); aeon worktree `.worktrees/shared-struct-module` (seeding). Fable
gate-reviews before merge. **Byte-neutral throughout** — canonical CRCs unchanged
(plain 453087/`b335bdc6`, debug 461110/`827e18c4`).

## Triggers being discharged
- **Row 1051** — Sec/Act shared-struct module; 3rd consumer confirmed
  (section + entity_window + tile_cache reg-relative-consume Sec/Act;
  act_descriptor.emp has file-local typed twins). Kill: shared module ships.
- **Row 1054** — SectionId/GridCoord cross-seam newtypes, adopt WITH this pass.
- **R1 rider** — TILE_CACHE_COLS/ROWS/STRIDE/NT_SIZE 4-const hoist into the
  `engine.constants` twin (discharges tile_cache.emp:8-12 deferral).

## Seed source (a MOVE, not authoring)
`games/sonic4/data/levels/ojz/act1/act_descriptor.emp:15-56` — complete typed
`struct Act` (14 fields, $22) + `struct Sec` (21 fields, $42), offset-commented,
sizeof-guarded. Field names VERBATIM from `engine/structs.asm` (stutter included:
`Act.sec_grid_ptr`, `Sec.sec_bg_layout`). **Do NOT touch structs.asm** (AS frozen).

## Work items (Fable R2 (1)-(7))
1. **Create `engine/structs.emp`** — twin-named against structs.asm; MOVE the two
   structs out of act_descriptor.emp into it. Kill condition = Spec-5 twin retirement.
2. **Per-field drift wall** — `ensure(offsetof(S, f) == extern("S_f"))` for EVERY
   field of BOTH structs (stronger than act_descriptor's sizeof-only guard, which
   would pass a swapped adjacent same-size pair). Keep the sizeof guards too.
   Consumers carry NO Sec/Act ensures afterward.
3. **Unwind the consumers** — entity_window.emp + section.emp + tile_cache.emp
   Sec/Act offset consts + their ensures DIE; each `use engine.structs`.
   act_descriptor.emp re-points to the shared module (KEEPS its validating
   constructor `act()` + field defaults).
4. **WATCH SITE** — entity_window.emp:845/1642 read `Act_grid_w+1(a2)` (field+N byte
   access into a word field, the corpus's only one). `Act.grid_w + 1` must compose
   as a comptime offset expr — own TEST, or a named fallback if it doesn't compose.
5. **row-1054 newtypes** — SectionId + GridCoord adopt with the pass (bound by the
   row). FLAG to Fable if this balloons the batch rather than stall.
6. **R1 hoist** — TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE} → `engine.constants` twin;
   kill section.emp + tile_cache.emp local mirrors; discharge tile_cache.emp:8-12.
7. **Test/harness unwind** — the port test files' value-equ seams
   (`entity_window_equs`, `tile_cache_value_equs/pairs`, `section_value_pairs`,
   `engine_constant_equs`) re-home the moved consts; add a per-field offset test +
   the watch-site (item 4) test.

## Gate bar (each edit, per sst-usability template)
- All affected port byte gates GREEN both shapes (entity_window_port,
  tile_cache_port, section_port, act_descriptor_port, + new struct-module test).
- Full strict suite (`SIGIL_STRICT_GATE=1 AEON_DIR=<worktree> cargo test --workspace`)
  = 194/2252/0, `AEON_DIR` → the branch worktree (paired-state gate).
- `repin --check` clean (byte-neutral → pins unchanged).
- Gate-off neutrality: worktree ROMs rebuild to canonical CRCs.
- Rows 1051 + 1054 CLOSE; kill-list rows updated same-commit; sst-usability-template packet.

## Sequencing
Build structs.emp + per-field wall (1,2) → unwind act_descriptor first (proves the
move, byte-neutral) → unwind the 3 consumers one at a time, gate each → R1 hoist (6)
→ newtypes (5) → final strict + packet. Probe A (t17 R4) is independent (measures
shipped tip 827e18c4) — run opportunistically, becomes t17 step-5 charter baseline.

## Progress / resume anchor (2026-07-16)
Branch `refactor/shared-struct-module` both repos; worktree
`.worktrees/shared-struct-module` seeded green (canonical ROMs). Fable rulings:
item-4 = proceed with fallback **F** (named const `Act_grid_w_lo`, file-local in
entity_window, no drift-lock, kill-linked); newtypes (5) LAST.

DONE:
- **Item 4** (resequenced first) — `Act.grid_w+1` disp probe: natural `.field+N`
  does NOT compose; byte-neutral fallback `offsetof(Act,grid_w)+1` does. Persistent
  3-test artifact `struct_field_disp_plus_n.rs`; ledger row 1068; language ask
  deferred. Commit sigil `61b0e3e`.
- **Items 1+2** — `engine/structs.emp` (type-only Act/Sec twins + 34-field
  `offsetof==extern` drift wall + sizeof guards). `test_support::act_sec_field_equs()`
  + `structs_module.rs` (wall passes + negative probe). Strict **2257/0** (AEON_DIR at
  worktree), repin clean, byte-neutral. Commits aeon `e1badfe` / sigil `b7ef117`.

DONE (cont.):
- **Item 3.1** — act_descriptor.emp unwound to `use engine.structs.{Act, Sec}` (local
  struct defs + sizeof ensures deleted; ojz_sec constructor + defaults + limit mirrors
  stay). **Module move PROVEN BYTE-NEUTRAL** (act region unchanged, ROMs b335bdc6/
  827e18c4, repin clean, strict 2257/0). Item-7 rehoming done for act's tests:
  act_descriptor_port + tranche4_negative_probes (field-swap now doctors structs.emp)
  + mixed_dac_rom (`structs_ambient_items` + act_descriptor ambient arm, `nth(6)` root;
  drift counts 5→39). Commits aeon `ba05da9` / sigil `c4e332d`.

  **MECHANISM (reuse for 3.2-3.4):** (a) edit the consumer .emp — delete its Act/Sec
  offset consts + ensures, add `use engine.structs.{Act, Sec}`; (b) its port test —
  prepend structs.emp via `with_ambient`, fold `act_sec_field_equs()` into the seam
  equ blob (ONE assembled `Stub` — don't add a 2nd equ section), update any drift-count
  assertion; (c) mixed_dac_rom — add the file to the `match` with `structs_ambient_items`
  (+constants for tile_cache/section which also use engine.constants), bump its
  per-tranche assert-count sites (there are 5-ish copies — grep them all). Gate: affected
  port byte gates both shapes + strict + repin.

NEXT (resume here):
- **Item 3.2** — section.emp (6 Act/Sec consts + ensures die). Test: section_port.rs.
- **Item 3.3** — tile_cache.emp (7 consts + ensures die). Test: tile_cache_port.rs
  (also the two_module flip helper's `tile_cache_value_pairs`).
- **Item 3.4** — entity_window.emp (4 consts + ensures die). The 2 `Act_grid_w+1` sites
  at :845/:1642 → fallback F const `Act_grid_w_lo = offsetof(Act, grid_w) + 1` (comment:
  low byte, grid_w ≤ MAX_ACT_SECTIONS < 256, act-constructor-guarded); file-local, NO
  ensure. Test: entity_window_port.rs.
- **Item 7** kill-linkage: amend row 1068 so `Act_grid_w_lo` + the 2 sites + the pin test
  retire as one unit when the `.field`-in-disp ask ships (name it in the ledger row).
- **Item 6** — hoist TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE} into engine.constants twin;
  kill section+tile_cache local mirrors; discharge tile_cache.emp:8-12 comment.
- **Item 5 (LAST)** — SectionId/GridCoord newtypes (row 1054); flag Fable if it balloons.

Step-0 note on sigil master `5739388`; plan note here (this branch).
