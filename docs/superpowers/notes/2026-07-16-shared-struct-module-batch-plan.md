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

- **Item 3.2** — section.emp DONE (aeon `96bae91` amended in the shared grid-lo consts /
  sigil `050b05b`). 6 Act/Sec consts + ensures deleted; `use engine.structs.{Act, Sec,
  Act_grid_w_lo, Act_grid_h_lo}`; Sec.field/Act.field access; grid_w+1/grid_h+1 → shared
  consts; Sec_len==66 → sizeof(Sec). Rehomed section_port (prepend structs + act_sec_field_equs)
  + entity_window_port flip (section ambient gains structs, union gains act_sec_field_equs).
  Byte-neutral, strict 2257/0.

  **FABLE item-3.2 RULING (Option 2, applied):** `Act_grid_w_lo`/`Act_grid_h_lo` =
  `offsetof(Act, grid_*) + 1` are **shared pub consts in engine/structs.emp** (next to
  struct Act), NOT file-local — the field+N enumeration found **10 sites / 3 consumers /
  2 fields** (grid_w+1 AND grid_h+1), falsifying the single-consumer premise. Rule-1
  amended: no *consumer-specific* detail in the shared module, but a multi-consumer blessed
  sub-field view is layout-adjacent and belongs there. entity_window's planned file-local
  const is SUPERSEDED (shared from birth). Consequent amendments still owed: row 1068
  (correct enumeration + demand + kill-linkage → RE-JUDGE not auto-respell); loop-doc idiom
  line (name the shared home + both consts); item-4 test artifact stays as-is.

DONE (cont.):
- **Item 3.3** — tile_cache.emp DONE (aeon `8c84e87` / sigil `949fca2`). 7 consts + 7
  ensures gone; `use engine.structs.{Act, Sec, Act_grid_w_lo, Act_grid_h_lo}`; bare →
  `Sec.field`/`Act.field`; 6 grid+N sites → shared consts; Sec_len==66 → sizeof(Sec).
  **⚠ Fable's flip-flagged file took the recipe with NO resistance** — `two_module_tail_call_flip`
  passed (structs into tile_cache's ambient, `act_sec_field_equs` into the union;
  `tile_cache_value_equs` byte seam + `tile_cache_value_pairs` flip seam both shed the 7).
  TILE_CACHE_* mirrors UNTOUCHED (item 6). Not in mixed_dac_rom. Byte-neutral, strict 2257/0.
- **Item 3.4** — entity_window.emp DONE (aeon `0cb52c6` / sigil `4acaab0`). 4 consts + 4
  ensures gone; `use engine.structs.{Sec, Act_grid_w_lo}`; bare Sec → `Sec.field`; the 2
  `Act_grid_w+1` sites → SHARED `Act_grid_w_lo` (no file-local). EntityScanState twin
  untouched (row 25). Prepend structs in byte-gate compile + BOTH flip sides;
  `entity_window_equs` shed its 4; `act_sec_field_equs` into `as_constant_equs`. Byte-neutral.
  **→ the Sec/Act file-local offset-const mirror class is EXTINCT.**
- **Item 7** — DONE (sigil `9788738`). Row-1068 CORRECTED (10 sites / 3 consumers / 2 fields;
  shared consts shipped; kill-linkage = RE-JUDGE not auto-respell). Loop-doc idiom line names
  the shared home + both consts.

NEXT (resume here):
- **Item 6** — hoist TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE} into the engine.constants twin
  (`engine/system/constants.emp` — add `pub const`s + drift ensures there); kill the
  file-local mirrors + their 4 ensures in section.emp (lines ~21-24 + ensures 38-41) AND
  tile_cache.emp (lines 14-17 + ensures 55-58); both `use engine.constants.{TILE_CACHE_*}`.
  Discharge tile_cache.emp's deferral comment (the "hoist is deferred to the same wave" at
  lines ~9-13) — UPDATE it, don't leave a stale claim. Tests: section_port/tile_cache_port
  `*_value_equs` already SUPPLY these as plain pairs — they can stay (the values don't move,
  only the .emp home), but `test_support::engine_constant_equs()` GAINS the 4 (constants.emp's
  new drift guards read them) → add to `engine_constant_equs` + the kill-list/constants twin
  rows. Verify byte-neutral + strict.
- **Item 5 (LAST)** — SectionId/GridCoord newtypes (row 1054); seam-level typing
  (FlatIDXY/GetSecPtrXY + entity_window↔section). FLAG Fable BEFORE building if it balloons
  past sst-usability size. If ratified, add its seam-spelling line to the port-loop idiom list
  (Fable's conditional feed-forward from the d83f1ca amendment).

Then: merge packet (sst-usability template — include item-4 finding, the Act_grid_w_lo win,
the sizeof(Sec) stride-guard upgrade Fable flagged, the trip-check outcomes) → Fable gate → merge.

Step-0 note on sigil master `5739388`; plan note here (this branch).
