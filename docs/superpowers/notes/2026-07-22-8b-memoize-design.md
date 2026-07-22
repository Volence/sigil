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

## Build readiness (overseer-approved 2026-07-22, 4 riders)

Approved to build. Rider rulings folded in:
- **R1 — bounds-compare is LOAD-BEARING, not belt-and-braces.** `Head/Left` (and `Top/Bottom`) move
  WITHOUT a claim on a prefetch-success fill (`Head_Col` commits at `tile_cache.emp:795`, all-hits,
  zero decompress, gen unchanged) — the bounds fields are then the ONLY memo-killer. Comment it as
  load-bearing; never simplify it away.
- **R2 — skip must be ARCHITECTURALLY equivalent at `.row_done`/`.col_done`**, not just cache-RAM
  equivalent: explicitly verify no live register / CC flows from the walk body into the join consumers
  (the A/B checks cache RAM; a liveness bug corrupts elsewhere). Trace the join's live-in set.
- **R3 — hook EXACTLY the 2 proven gen sites** (DecompressBlock claim + InvalidateStaging; overseer
  verified `Block_Stage_Keys` has exactly 3 touchers: FindStagedBlock read-only, InvalidateStaging
  sentinel-write, DecompressBlock sole claim/record). Add a regression test asserting the 3-toucher
  property so a future 4th toucher fails loudly.
- **R4 — move.l riders:** prove per-RUN evenness (wrap-split run-1/run-2 can be individually odd with
  an even TOTAL) or handle odd tails; the A/B MUST include wrap-crossing positions (the 1.3
  column-preserving-wrap lesson — full identity bar, byte gate can't catch a shared-twin bug).

**RAM home (decided):** append the gen word + the two memo triples at the RAM end, after
`Dynamic_Live_Pending_Count` (`engine/ram.asm:474`) in the release-pad region — shifts only
`Engine_RAM_End` + game RAM (Object_RAM at `:222` is fixed and unaffected) = narrowest ripple.
9 words: `Block_Stage_Gen` + `Pfx_Memo_{Row,L,H,Gen}` + `Cs_Memo_{Col,T,B,Gen}` (even-aligned).

**Turnkey implementation sequence:**
1. R3 test first (guards the parcel) — assert `Block_Stage_Keys` 3-toucher property.
2. RAM append (9 words, end, minimal ripple) + init memo gens to a sentinel (Tile_Cache_Init +
   InvalidateStaging) so frame 1 always scans.
3. Gen-bump hooks: `addq.w #1, Block_Stage_Gen` after `DecompressBlock:270` and in
   `InvalidateStaging:240`.
4. `.pfx_scan` check (at `.pfx_go` entry) + record (at the `:1114` all-hits exit only) — R1 bounds
   compare (Row+L+H+Gen), R2 liveness trace of `.row_done` live-in.
5. `.cs_scan` check (at `.cs_have_col`) + record (at `:1206` all-hits exit) — same, Col+T+B+Gen,
   R2 liveness of `.col_done` live-in.
6. Build both shapes → diagonal A/B in OJZ scroll (cache-RAM byte-identity, Debug_Scene_Freeze +
   Camera-poke) + idle/re-probe-cost drop check.
7. R4 move.l riders (tile_cache 1523/1532/1608-1619 + plane_buffer 334/340) — own bisectable
   commits, per-run evenness proven, wrap-crossing A/B.
8. 5-site ripple (RAM shift is game-RAM-only) + PROVENANCE re-baseline (native flow) → attack-the-diff
   → merge.

## Open items for implementation
1. RAM allocation for `Block_Stage_Gen` + the 8 memo words (find a home in the tile_cache RAM block;
   even-align).
2. Exact placement of the check/record in each scan (the check goes at `.pfx_go`/`.cs_have_col`;
   the record at the all-hits exits — confirm no path reaches the exit after a decompress).
3. Confirm the demand-fill (FillColumn/FillRow) claim path ALSO routes through `DecompressBlock`
   (so gen bumps on demand stages too — it should; verify no direct-stage bypass).
4. Collision-run `move.b`→wider pairing: verify collision cells are contiguous + alignment (finding
   #1's collision segment note said they are, but re-verify at 1608-1619).

## Implementation progress (2026-07-22)

**Memoize (steps 1–6) DONE + verified.** Commits: sigil `pass3-8b-memoize` (R3 guard
`92f94a9`, ripple `5be9353`); aeon `pass3-8b-memoize` `8146ba5`. Region grows +0x90
both shapes; NEW debug 428968 (OLD canonical 428768).
- R3: `Block_Stage_Keys` 3-toucher guard — passes untouched, proven to fire on an
  injected 4th toucher.
- R1/R2 satisfied: bounds-compare documented load-bearing at both check sites; the
  `.row_done`/`.col_done` joins reload every live value from memory (corner targets
  written before the skip, budget untouched on the all-hits path) — no register/CC
  flows from the skipped walk.
- Byte gate `tile_cache_port` PASS both shapes + two-module flip. Full paired strict
  **2457/0/1** (baseline 2456 + R3). Ripple: pins (repin), engine.inc orgs (+0x90),
  mixed_dac_rom collision_lookup tail-call disp F3AA→F31A / F2EA→F25A, repin_pins
  SOUND_API base +0x90.
- **A/B PASS (oracle, OJZ scroll, DEBUG shape, controller-driven frame-anchored).**
  OLD (canonical d5d8e163) vs NEW: after an identical `press(right,40)` (cache slid
  cols 70–149, real staging) all 5 sampled windows + bounds byte-identical; after an
  identical diagonal `press(right,24)+press(right,down,20)` (cache slid cols 66–145 ×
  rows 26–85 — row scan + col scan + corner) bounds + 2 windows byte-identical.
  Note: `Debug_Scene_Freeze`+camera-poke does NOT slide the cache window (the slide
  lives in the skipped Camera_Update); the working method is controller input +
  frame-anchoring, cache slides naturally.

**Remaining:** step 7 move.l riders (own bisectable commits, per-run evenness + wrap-
crossing A/B), step 8 final ripple + PROVENANCE re-baseline, then HOLD for overseer
attack-the-diff before merge.

## Step-7 move.l rider analysis (R4 alignment audit, pre-implementation)

Current line numbers (post-memoize): NT copy `tile_cache.emp:1587/1596`
(`.fr_nt_run1/2`), collision `tile_cache.emp:1672-1684` (`.fr_ci_run1/2`),
plane_buffer drain `plane_buffer.emp:334/340` (`.err_run1/2`). All three share the
SAME shape: a wrap-split copy of `n_total` cells into a circular buffer, split into
run-1 (`n1`) + run-2 (`n2`) at the physical wrap — n1/n2 are individually arbitrary
(odd or even), n_total even or odd. So every candidate needs PER-RUN odd-tail
handling (`n>>1` move.l + `n&1` move.w), never a whole-run pairing (R4).

- **NT (1587/1596): move.l-SAFE.** Words; dest `= (a4, (row_off+phys_start)*2)` and
  src `a0` staging — both word-aligned (the ×2 makes dest even). move.l on a word-
  aligned (not necessarily long-aligned) EA is legal on 68000. Big-endian long copy
  preserves the two-word order byte-for-byte. → implement with per-run odd tail.
- **plane_buffer (334/340): move.l-SAFE pending a2 check.** Words; src `a1 = (a0,
  phys*2)` word-aligned. Must confirm dest `a2` (Plane_Buffer write ptr) is word-
  aligned at entry AND re-test the `$E000` Plane-B edge-vector case (ledger row 1078)
  — the design's named alignment trap. → implement + that specific wrap-crossing A/B.
- **Collision (1672-1684): move.l-UNSAFE — SKIP (design's "where alignment permits"
  carve-out).** BYTE copies whose dest index `= coll_row_offset + phys_col` is
  arbitrary → an odd phys_col makes move.w/move.l a MISALIGNED access (68000 address
  error), and the loop interleaves plane-A (a1) and plane-B (a4) one byte each so
  consecutive same-stream bytes aren't even adjacent in the emission. Leave as move.b;
  document the skip (kill-list style) so it isn't silently dropped.

Each byte-changing rider = full cycle (twin .emp+.asm edit → build both shapes →
repin → engine.inc/mixed_dac_rom/repin_pins hand-edit → byte gate → wrap-crossing
A/B both ROMs). They share ONE final PROVENANCE re-baseline with the memoize.
