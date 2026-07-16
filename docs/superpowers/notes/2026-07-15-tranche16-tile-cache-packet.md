# Tranche 16 — tile_cache.emp port packet (loop-dry, at the merge gate)

Region: `engine/level/tile_cache.asm` → `tile_cache.emp` (§4.7 the 2D
block-decompression tile cache, 15 procs + 1 data table; the biggest region
ported yet, ~2.4× section). section.emp's paired streaming sibling.
**Precedent-setting:** the first port whose STEP-5 is the tranche headline — the
file is the measured vertical-streaming lag driver (ledger row 1057), not a
byte-faithful transcription with an optional optimize pass. Byte-green both
shapes; steps 1-5 landed + Fable-countersigned in prior sessions; this packet is
the exit-measurement + loop-dry + step-6 wrap at the merge gate.

## Gate artifacts (gate-artifact discipline)
- **tile_cache_port** (`crates/sigil-cli/tests/tile_cache_port.rs`, 4 tests):
  `tile_cache_region_matches_reference` + `_debug_region_matches_reference`
  (region byte gate BOTH shapes — shape-varying) + `two_module_tail_call_flip_plain`
  + `_debug` (the flip proof, below). All 4 green at HEAD (`e9c84f0`).
- **Flip proof (DQ3, proof-mechanism feed-forward — the campaign's FIRST TAIL-CALL
  flip):** `two_module_tail_call_flip_{plain,debug}` binds the
  `entity_window_port` two-module template — with both gates on,
  `collision_lookup.emp`'s tail-call `jbra Tile_Cache_GetCollision` resolves to
  tile_cache.emp's pinned symbol (plain `0x4336` / debug `0x4F00`), asserting the
  resolved displacement BOTH shapes (the jbra→bra.w relaxation lands on the
  pinned target).
- Gate: `SIGIL_EMP_TILE_CACHE` (`engine.inc:312`, `ifndef` — default build is the
  AS twin). Region `TILE_CACHE`: plain_base `0x42FA` len `0x996`, debug_base
  `0x4EC4` len `0xA56` (`pins.rs:88`).
- **Full strict gate:** `SIGIL_STRICT_GATE=1 AEON_DIR=<worktree> cargo test
  --workspace` = **194 suites / 2252 / 0**. `repin --check` (AEON_DIR=worktree) =
  `pins.rs unchanged`.
- **Paired-state gate:** the strict suite above is run with `AEON_DIR` pointed at
  THIS branch's worktree tree (`.worktrees/port-tranche16`), not aeon master.

## Provenance (branch reference → merge re-baseline)
- **Branch ROMs (rebuild-verified this session):** plain **452638 / crc32 99fd3a55**,
  debug **460661 / a48fb0df**. Build order DEBUG-first (`DEBUG=1 ./build.sh` →
  cp `s4.debug.bin`+`s4.debug.lst`), plain last (build.sh writes `s4.bin`
  regardless of DEBUG — the repin trap; a stale debug-ROM-in-plain-slot caused
  one transient byte-gate failure this session, resolved by rebuilding plain).
- **Gate-off canonical (master, pre-merge):** plain 452500 / 5a47851a, debug
  460521 / b0ceca0b. These are the numbers the branch re-baselines FROM; the
  branch numbers above become canonical AT MERGE.
- **Symbol-appendix / ROM-size note:** debug (460661) exceeds plain (452638) by
  8023 bytes. That delta is DEBUG-only content — the region itself is shape-
  VARYING (debug_len 0xA56 vs plain 0x996 = **+0xC0**, the DEBUG `assert.l`/`assert.w`
  block at the raw-copy path that self-gates to ZERO plain bytes) plus the debug
  convsym symbol appendix. Gate-off neutrality (both shapes rebuild to the
  canonical CRCs above) is the proof that NO debug content or symbol leaks into
  the plain convsym appendix — the plain ROM (99fd3a55) is byte-stable across the
  whole tranche including the Wave-2 growth.

## Step-1 demanded features (recap)
- **`coll_src_row_base(reg: Reg)`** comptime-fn — the `.emp` counterpart of the AS
  `collSrcRowBase` macro (`lsr #1` tile→coll row, `lsl #4` ×BLOCK_COLL_COLS),
  designed under the macro-port rule: typed `Reg` param, `ensure(BLOCK_COLL_COLS
  == 16)` replaces the macro's `if <>16 / error` (guard died to a comptime
  error). File-local (single-file consumer). Kill-list row 26.

## Step-2 checklist (recap + this-pass conformance)
1. **Branch conversions**: the transcribed body went new-style at step 2. This
   session's loop-dry caught **3 residual bare `bra` in the Wave-2-added code**
   (`WarmupBelowRow:.scan`, prefetch `.pfx_skip`/`.pfx_scan`) → `jbra`. Byte-
   neutral (twin spells `bra.s`; `.emp` bare `bra` and `jbra` both relax to
   `bra.s` — verified: tile_cache_port 4/4, repin unchanged). No other bare
   `bra`/`bsr`, no `jmp`/`jsr` (no computed targets), no explicit `.s`/`.w`.
2. **Structural width-pins commented**: none kept (no macro-transliteration or
   stride-locked jump-table blocks in this file).
3. **Bare-symbol width spellings**: complete, incl. the row-wrap sentinels
   `Tile_Cache_Nametable+TILE_CACHE_NT_SIZE` / `Tile_Cache_Collision+TILE_CACHE_COLL_SIZE`
   in the bare `sym + const` form (asl width rule picks abs.l for the non-word
   base $FFFF2580; the paren-override can't defer a link base — ledger row 1004,
   section.emp:303 precedent). Site comments name the gap at each.
4. **Brace-indent**: file-wide, incl. the three comptime-fn bodies.
5. **Idiom-list walk**: `Sst.field` — n/a (Sec/Act are the deferred struct-twin,
   reg-relative offset consts + `ensure(Sec_len==66)`). bareword bankid/winptr —
   n/a. label-in-immediate — n/a. typed VDP fns — n/a (RAM-only file, ZERO VDP
   command words — ledger row 1052 corrected to drop tile_cache from the VDP-macro
   consumer list). `assert`/`raise_error` — ADOPTED (the DEBUG assert block, DQ6).
   contract reglists in movem-RANGE form — present (`clobbers(d0-d7/a0-a6)` etc.).
6. **Noticing clause**: no new house-format item proposed.

## Step 3(a) — language/format asks (filled)
- **Ceremony scan**: the ×80 shift-add idiom is the one repeated-emission shape
  (5 sites, 2 forms) → addressed by `mul_cache_stride` build (step 4 / loop-dry).
  No other high lines-per-intent proc.
- **Comment-as-compensation**: no recurring what-comment shape. The a5/a6-survives-
  DecompressBlock comment is a WHY (a load-bearing cross-call reliance), not a
  language gap — it is the S2-D6 checked-clobbers lint's demand data (ledger 1061).
- **Escape-hatch census** (by shape): `extern("S4LZ_DecompressDict")` ×1 (the
  unported AS decompressor — necessary link edge, not a detour);
  `extern("Block_Stage_Buffers")` in the `BlockStage_PtrTable` comptime-for (the
  ROM ptr-table base). The Sec/Act `extern()` drift-lock ensures are the standard
  deferred-mirror shape (feed the row-1051 shared-struct ask). No new ask.
- **Domain-type scan**: block coords (`sec_x`/`sec_y`/`block_index`), cache cols/rows,
  nametable words flow as raw `.w`. Same GridCoord/VramTile candidates section
  raised (R4 of t15) — no NEW type earns its place from tile_cache alone; the
  newtypes are gated on multi-file adoption (entity_window/section boundary).
- **Noticing clause**: no new interrogation-line class exposed.

## Step 3(b) — reads-wrong / could-be-better (filled)
- **Comment-claim audit**: verified against the code as it now stands —
  (a) the a5/a6-survives-DecompressBlock claim (FillRow) — TRUE: DecompressBlock's
  license `d0-d7/a0/a2-a4` excludes a5/a6, transitive callee S4LZ verified
  a5/a6-clean (grep). (b) "coll_src_row_base makes the misencoding impossible" —
  TRUE: the parity-safe halve+scale is in the fn, drift-locked. (c) WarmupBelowRow
  "RETURNS cleanly / terminates" — TRUE (live step_out + Init reaches Game_State 6).
- **Contract audit**: FillRow `clobbers(d1-d7/a0-a6)` (a5/a6 hold row bases across
  the per-block call — widened from a0-a4, sole caller Tile_Cache_Fill licenses
  a0-a6). WarmupBelowRow `clobbers(d0-d7/a0-a4)`, preserves a5/a6 (touches only
  a0-a4). Both bodies stay inside license; headers don't disclaim the license.
- **Name audit**: labels say what they do (`.pfx_scan`/`.fr_col_loop`/`.done_coll`).
- **Magic-number audit**: `#$F`/`#$FFF0`/`#$FFFF` block-masks & sentinels — all
  carry a site comment or a named const (`BLOCK_TILE_SIZE`, `TILE_CACHE_*`).
- **Cold-reader test**: one V-fill frame traces cleanly through the headers.
- **Codename-reference audit**: **8 session-codename comments cleaned this pass**
  (`Wave-2 (ii)`, `(i)`/`(ii)` mechanism labels, `step-5 hoist` ×3, `S2-D6`) →
  behavioral phrasing (the reason was already adjacent). Judgment-KEEP: the
  `§4.7`/`§5` design-doc section refs (durable anchors, behavioral reason present).

## Step 4 — construct pass (filled)
- **`decompose_block(col, row)`** — world-tile → sec_x=d0/sec_y=d1/block_index=d2;
  pure-dedup helper (no AS macro; donor self-flagged "(same pattern as FillColumn/
  FillRow)"). ADOPTED at **4 sites** (FillColumn/FillRow + WarmupBelowRow + the
  Wave-2(i) prefetch scan — superseding step-4's "2 sites, prefetch inline" note).
  Only FillAll's block-COORD variant (`lsr #4` not `#8`) stays inline. Kill row 27
  (corrected to 4 sites).
- **`mul_cache_stride(dst, scratch)`** — ×80 cache-row-stride via shift-add,
  form-(a) scratch-reg variant. BUILT this session (ledger row 1060 revisit:
  form-(a) sites survived step-5) + adopted at GetTile/GetCollision/FillRow
  cache_row_offset (3 sites), `ensure(TILE_CACHE_STRIDE==80)` drift lock, byte-
  neutral. CopyBlockColumn's form-(b) single-temp ×80 (×2) kept inline BOTH sides
  (register-pressure variant, deliberately uncovered). Kill row 28.
- **`BlockStage_PtrTable`** `pub data ... = comptime for i in 0..SLOTS { base + i*stride }`
  — KEEP (natural construct, reads well, 1 site; no adopt/build improves it).
- **STRUCTURAL clone (noticing): `WarmupBelowRow` scan ↔ Fill's prefetch `.pfx_scan`**
  — both scan block cols [Left..Head] at a target row, grid-guard sec_x,
  FindStagedBlock, DecompressBlock. DECISION: **KEEP as two procs** — the varying
  terms are meaningful control flow (whole-row/no-budget/reg-save-across-call vs
  k=1/budget-charged/single-shot-exit), so an `emit`-template with continue/budget/
  save holes would obscure more than it names (taste gate: must read BETTER). Flagged.

## Step 5 — optimize: the 3-regime A/B (the tranche headline)

Steps 5 = TWO WAVES, both Fable-gated in prior sessions: **W1** = FillRow lea
hoist (row bases a5/a6, size-neutral); **W2(i)** = staged-count-aware k=1 row
prefetch; **W2(ii)** = `TileCache_WarmupBelowRow` Init-time below-row pre-stage.
This session discharged the **exit measurement** — the 3-regime A/B (ROM
a48fb0df hash-verified before measuring; state-counter method per ledger 1062).

**Per-proc step-5 interrogation (hot procs), each line's outcome:**
- **Invariant ladder** — FillRow row bases: TAKEN (W1 hoist, a5/a6 row-scoped).
  FillColumn/CopyBlockColumn base leas (4, re-loaded per-block): NOT-TAKEN,
  DEFERRED — off the vertical charter (CopyBlockColumn=0 on the vertical path),
  bundled into the H-column follow-up dossier (Fable-requested analysis done).
- **Counter/cache audit** — `Cache_Fill_Budget` (6/frame): all writers/readers
  charge it; FillColumn (priority) → FillRow → leftover prefetch, each `subq` on
  decompress. Prefetch reads leftover only (`beq .fill_return`). Symmetric.
- **Guard-coverage audit** — grid guards (sec_x<grid_w, sec_y<grid_h) on every
  decompress emission path (Fill prefetch, WarmupBelowRow, both directions).
  `Block_Stage_Next` mod-12 round-robin evict is the sole cache guard — LOAD-BEARING.
- **Hardware cross-check** — RAM-only file, no VDP/DMA facing behavior; n/a.
- **Silent-tradeoff comments** — the below-row down-scroll assumption
  (WarmupBelowRow), the k=1 prefetch cadence, and the a5/a6 cross-call reliance
  all carry CHOSEN-tradeoff site comments.
- **Regime (a) cold control — EXIT PROOF, PASS.** Camera_Y=$00900000 +16px/f
  through cold-start + a 0x5x crossing + the 0x6x onset (144→320px). VInt_Lag
  breakpoint **hits=0** (`breakpoint_list`). Decompresses flat 0-1/frame; profiler
  VInt(lag)=0 / VSync idle 53.8%. **PROBE-VALIDATION ARTIFACT (Fable rider 3):**
  sentinel-invalidation of all 12 staging keys → forced cold crossing → VInt_Lag
  FIRED — proving the detector is live and "zero hits" is a validated non-event.
- **Regime (b) steady — NO REGRESSION.** By construction (W2 is leftover-budget
  row-prefetch retargeting + Init-only warmup, neither on the FillRow per-cell
  loop) + measured (no overrun, 53.8% idle). FillRow steady = W1's 23815 unchanged.
- **Regime (c) diagonal — NOT-TAKEN-WITH-REASON (condition 2).** The diagonal
  lags on H-column-crossing frames, but via the **pre-existing H-column spike**
  (a fresh block_x column cold-fills ~5 blocks in one FillColumn frame — the
  horizontal analog of the OLD vertical spike), NOT V-tag thrash: in EVERY lag
  frame the pre-staged V-row tags (60-65,70,71) SURVIVED. Code-verified pre-existing
  + out-of-scope (prefetch row-only + leftover-budget after FillColumn's priority;
  donor had no horizontal prefetch). **Fable ruling (this session): Option 1 —
  proceed; the H-column spike is LEDGERED (dossier + fix template + scheduling
  trigger), the slot ruling CLOSED empirically.** NOT recorded as a regime-(c) pass.

**W1-skip-by-construction (recorded per condition):** the symmetric W1 (pre-W2)
run was DELIBERATELY skipped (Fable ruling, prior session), not a silent omission
— the W2(i) Keys timeline is standalone mechanistic proof (next-row tags build one
per quiet frame), and the W1 spike is forced BY CONSTRUCTION (old prefetch warms
≤1 block/frame, a crossing needs ~5-6, so the remainder MUST decompress on demand
— arithmetic, corroborated by the R2 probe + positive-control cycle data). The
symmetric protocol stays on file for anyone wanting the W1 run later.

## Loop-until-dry
- **Pass 1** (this session): built `mul_cache_stride` (3 sites); 3 `bra`→`jbra`;
  8-site codename-comment cleanup; FillColumn/CopyBlockColumn hoist ANALYSIS
  (deferred to H-dossier); warmup/prefetch clone (KEEP). All byte-neutral.
- **Pass 2**: DRY. No residual session codenames (grep clean bar the judgment-keep
  `§` refs), no new algorithmic finding, gates green (2252/0, repin unchanged,
  tile_cache_port 4/4).

## Step 6 — corpus sweep (enumeration, every site named)
Trigger = a new thing PRIOR FILES could use. t16's additions (`decompose_block`,
`mul_cache_stride`, `coll_src_row_base`) are all file-local tile-cache idioms;
enumerated across all 31 prior `.emp` files to PROVE (not assume) uniqueness:
- **`decompose_block` shape** (world-tile → section+block, `lsr #8` + `andi #$F`):
  ZERO co-occurrence outside tile_cache. 3 keyword-candidates examined, all
  NOT-AN-INSTANCE — `section.emp` takes sec_x/sec_y as INPUTS to a grid-multiply;
  `act_descriptor.emp`'s are struct FIELDS; `entity_window.emp` does camera→section
  (one level, no block_index, SECTION_SIZE_SHIFT granularity). No retrofit.
- **`mul_cache_stride` (×80 shift-add)**: ZERO `lsl #6`→`lsl #4` sequences
  outside tile_cache. 1 candidate: `section.emp` uses TILE_CACHE_STRIDE as a
  const-def + compile-time `lea STRIDE*2(a1)` displacement — NOT-AN-INSTANCE
  (no runtime register multiply). No retrofit.
- **`coll_src_row_base`**: BLOCK_COLL_COLS-specific, single-file; no sweep.
- **Stale-comment (codename-reference) class**: t16 cleaned its own file; the
  corpus backlog is enumerated — **15 sites / 7 files** (entity_window ×1,
  sound_api ×2, hblank ×1, sonic_anims ×1, mt_bank ×5, sfx_bank ×4, test_particle
  ×1; a subset of the ledgered ~40/16 audit). OUTCOME: **AT-NEXT-TOUCH** per the
  standing codename-audit ruling — NOT force-retrofitted in t16 (force-touching 7
  files' comments violates keep-tranches-small + the existing at-next-touch ruling).

## Ledger rows (this tranche, this session)
- **1063 H-column crossing amortization dossier** — mechanism + 2 measured lag
  events (V-tags survived) + code-path proof (pre-existing, out-of-scope) + fix
  template (W2 (i)/(ii) mirrored 90°) + the bundled FillColumn/CopyBlockColumn
  base-lea hoist analysis + scheduling trigger (≳12px/f horizontal / plane_buffer
  perf work).
- **1064 slot-ruling closure** — the Wave-2b escalation trigger (pre-staged tags
  vanish) tested in the regime built to provoke it (diagonal) and DID NOT FIRE;
  tags-survived evidence; 12-slot depth empirically sufficient; §4 open question
  (SLOTS↑ / eviction-order) NOT needed.
- **1060 CLOSED** — mul_cache_stride built; decompose-inline note superseded (4 sites).
- Kill-list **row 28** (mul_cache_stride) + **row 27 corrected** (4 decompose sites).

## What each pass added (step-3 vs step-5, per pass)
- **Prior-session passes** (recap): step-1 demanded `coll_src_row_base` + the flip
  proof; step-4 built `decompose_block` + kept BlockStage_PtrTable; step-5 W1
  (FillRow hoist, S2-D6 demand-data ledger 1061) + W2(i)/(ii) (the crossing-
  amortization mechanisms, Keys-timeline mechanistic proofs).
- **This session — STEP-3 findings**: (3a) mul_cache_stride ceremony → build;
  (3b) 8-site codename cleanup, comment-claim/contract audits clean; step-4
  warmup/prefetch clone KEEP; decompose_block confirmed 4-site adopted. Kill row
  28 + row-27 correction; ledger 1060 closed.
- **This session — STEP-5 findings**: 3-regime A/B DISCHARGED — (a) exit proof
  by non-event (0 VInt_Lag, sentinel-validated detector) TAKEN; (b) no-regression
  confirmed; (c) diagonal H-column lag NOT-TAKEN (pre-existing, out-of-scope,
  ledgered dossier 1063 + slot-ruling closure 1064); FillColumn hoist NOT-TAKEN
  (deferred to the H-follow-up).
- **Neither bucket (own headline)**: probe outcomes (the A/B tables, the
  detector-validation), the frame-advance-non-determinism tooling jot (ledger 1062).

## Owed at the merge gate
- **Fable's FillRow line-by-line second look** (hot-path ceiling review) — owed at
  the gate per the port-loop hot-path rule; the checklist above is the floor.
- Volence's gate on this packet → `--no-ff` merge both sides + push coupled
  (aeon + sigil together, paired-state), re-baseline canonical to 452638/99fd3a55,
  460661/a48fb0df.
