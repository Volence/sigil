# Parcel 8b — FindStagedBlock scan memoize (design gate)

Branch `pass3-8b-memoize` (worktree `aeon-8b`, off master `57d5e19`, seeded canonical
748ca5ba/d5d8e163). One parcel: the memoize + the folded `move.l`-pairing riders (from
dissolved Parcel C), one 5-site ripple + PROVENANCE re-baseline, attack-the-diff pre-merge.
**Scene: OJZ scroll (8b is scroll-path)** — diagonal A/B (per the new standing scene rule).

## The problem (review finding #3, measured 5.2% max-V)

`.pfx_scan` (vertical prefetch, `tile_cache.emp:1112`) and `.cs_scan` (horizontal prefetch,
`:1204`) each stage the FIRST unstaged block per frame (k=1) then bail. When the target line is
**fully warm**, the scan instead walks the *entire* block range calling `FindStagedBlock` on every
cell (all hits, no decompress) — pure wasted re-probing, every frame, ~2-6.6k cy. The memoize skips
that walk while the line is provably unchanged.

## Mechanism

**(1) A staging generation word** `Block_Stage_Gen` (new RAM word). Bumped on **every** claim in
`DecompressBlock` (right after `move.w d5, Block_Stage_Next` at `:270`: `addq.w #1, Block_Stage_Gen`).
`InvalidateStaging` (`:234`) bumps it too (so all memos die on invalidate) — one `addq.w #1` beside
the `clr.w Block_Stage_Next` at `:240`.

**(2) Per-axis keyed memo** (new RAM, two triples):
- Row memo (`.pfx_scan`): `Pfx_Memo_Row` (target row `d7`) · `Pfx_Memo_L` (Cache_Left_Col) ·
  `Pfx_Memo_H` (Cache_Head_Col) · `Pfx_Memo_Gen`.
- Col memo (`.cs_scan`): `Cs_Memo_Col` (target col `d6`) · `Cs_Memo_T` (Cache_Top_Row) ·
  `Cs_Memo_B` (Cache_Bottom_Row) · `Cs_Memo_Gen`.

**(3) Check** — at each scan's `.pfx_go`/`.cs_have_col` entry (target + range known, before the
`FindStagedBlock` walk): if `target == memo_target && range_bounds == memo_bounds && Block_Stage_Gen
== memo_gen` → skip straight to `.row_done`/`.col_done`. (Init the memo_gen to a sentinel ≠ any real
gen so the first frame always scans.)

**(4) Record** — when a scan reaches its "whole line already staged" exit (the `bgt .row_done` at
`:1114` / `bgt .col_done` at `:1206`) **without having decompressed** (i.e. the all-hits path):
store (target, bounds, current gen) into the memo. Do NOT record on the decompress-and-bail path
(the line still has unstaged blocks).

## Soundness (why gen-on-every-claim is the load-bearing guard)

A memo can only survive a frame in which **no block was claimed** (gen unchanged). Any staging —
demand fill OR either prefetch, on any axis — bumps gen and kills every memo. So a memo persists
only across fully-warm frames, which is exactly when the re-probe walk finds all-hits and skipping
is behavior-identical. The **key is (target, gen)**; the **range-bound compare (L/H, T/B) is
belt-and-braces** for a window that shifts without a claim (overseer's obligation: keep it, it's a
2-word compare). `Block_Stage_Gen` wraps harmlessly (a stale memo whose gen collides after 65536
claims still requires target+bounds to match; and any real claim in between already killed it).

**Obligations satisfied:** memo KEYED (target+bounds), not boolean ✓; per-axis (two independent
triples) ✓; gen bump on EVERY claim ✓; dies on InvalidateStaging ✓; diagonal A/B to prove zero
behavior change ✓.

## Folded move.l-pairing riders (dissolved Parcel C survivor)

Own bisectable commit(s), even-word-alignment verified, sharing 8b's ripple:
- `tile_cache.emp:1523/1532` — FillRow nametable `move.w (a0)+,(a1)+` runs → `move.l` pairs + odd-word
  remainder (src=staging slot `intra_col*2`, dest=cache col ×2, both even within a run).
- `tile_cache.emp:1608-1619` — FillRow collision `move.b (a3)+,(a1)+` / `move.b (a2)+,(a4)+` byte runs
  → `move.w`/`move.l` where alignment permits (verify — collision cells are byte-packed).
- `plane_buffer.emp:334/340` — Draw_TileRow drain `move.w (a1)+,(a2)+` → `move.l` pairs (VDP autoinc
  is per-word; the $E000 Plane-B edge-vector case from ledger row 1078 is the alignment trap to test).

## Verification plan

- **Memoize:** diagonal A/B in the OJZ scroll scene (old vs new ROM), cache-RAM byte-identity under
  the canonical Debug_Scene_Freeze + Camera-poke method (the memo is a pure skip — settled cache must
  be byte-identical); plus a lag-counter/idle check that the re-probe cost drops (target: recover most
  of the 5.2% max-V FindStagedBlock cost). The memo NEVER changes what gets staged — only whether the
  all-hits walk runs — so the streamed result is identical by construction; the A/B proves it.
- **move.l riders:** full identity bar per commit (byte gate can't catch a bug both twins share —
  [[wrap-split-preserve-column-offset]]); the copy output must be byte-identical (it's a pure
  copy-primitive swap). Even-word-alignment asserted at each site.
- Byte-CHANGING parcel → 5-site repin ripple + PROVENANCE re-baseline (native artifact flow, no cp).

## Open items for implementation
1. RAM allocation for `Block_Stage_Gen` + the 8 memo words (find a home in the tile_cache RAM block;
   even-align).
2. Exact placement of the check/record in each scan (the check goes at `.pfx_go`/`.cs_have_col`;
   the record at the all-hits exits — confirm no path reaches the exit after a decompress).
3. Confirm the demand-fill (FillColumn/FillRow) claim path ALSO routes through `DecompressBlock`
   (so gen bumps on demand stages too — it should; verify no direct-stage bypass).
4. Collision-run `move.b`→wider pairing: verify collision cells are contiguous + alignment (finding
   #1's collision segment note said they are, but re-verify at 1608-1619).
