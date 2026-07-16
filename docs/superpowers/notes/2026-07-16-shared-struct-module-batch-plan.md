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

## Resume anchor
Branch open, worktree seeding (bg `big47wmxs`). Next: rm pre-built ROMs in the
worktree, first build, then item 1. Step-0 note committed (sigil master 5739388).
