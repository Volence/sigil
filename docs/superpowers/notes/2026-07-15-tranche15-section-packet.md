# Tranche 15 — section.emp port packet (loop-dry, at the merge gate)

Region: `engine/level/section.asm` → `section.emp` (§4 continuous-scroll
streaming engine, 6 procs, biggest engine port yet). Byte-green both shapes;
step-2 modernized + re-based; steps 3-5 below. **Hot-path file** →
`Section_UpdateColumns` runs per-frame; step-5 table per-line + owed OJZ
streaming probe + Fable hot-path second look at the gate.

## Gate artifacts (gate-artifact discipline)
- **section_port** (`crates/sigil-cli/tests/section_port.rs`) — region byte gate
  both shapes; commit `harness(t15): section_port byte gate + pins`.
- **two_module_ownership_flip_{plain,debug}** (`entity_window_port.rs`) — the
  bidirectional symbol-ownership flip proof; commit `test(t15): two-module
  ownership-flip link test`.
- Mem-to-mem lowering feature (`lower_code.rs`, 5 tests) — commit `4ec988f`.
- Gate: `SIGIL_EMP_SECTION` (engine.inc), resume orgs plain `$551A` / debug
  `$619C`. Gate-off canonical `452500/5a47851a`, `460521/b0ceca0b`.

## Step-1 demanded features (recap)
- **Mem-to-mem lowering** (two width-pinned abs operands) — Fable path 1, TDD.
- **VDP command interface** — macro-port rule worked example (see step-4).

## Step-2 checklist (filled)
1. **Branch conversions + wave**: `bra`/`bsr`/`jsr`→`jbra`/`jbsr`, conditionals
   bare. Δ = **7 branches × −2 = −14** (jsr→jbsr size-neutral: both 4 bytes).
   Region `$3EA`→`$3DC`. Downstream: sound_api −14 (same pre-`$10000` bank);
   **org-`$10000` shield absorbed the object bank + game data (0 movement)**.
   Twin lockstep via the ratified method: *let asl relax bare branches to its
   fixpoint, read the chosen widths (10 `.w`, 49 `.s`) from the listing, write
   them explicit* — manual width-setting fights the fixpoint cascade.
2. **Structural width-pins commented**: the two mem-to-mem sites keep explicit
   `.w` with the site comment naming the two-way-RelaxAbsSym gap. No other
   kept widths (the 10 `.w` are asl-fixpoint, not structural pins).
3. **Bare-symbol width spellings**: complete, incl. the vcr_* asm-template
   bodies (immediates via `#{expr}` splice; RAM/label operands bareword).
4. **Brace-indent**: file-wide; the comptime-fn `if`s are inline-conformant
   (the corpus's lead_move/y_term shape); no DEBUG `if`-blocks in the procs.
5. **Idiom-list walk**: `Sst.field` — n/a (Sec/Act are deferred struct-twin,
   reg-relative offset consts). bareword `bankid`/`winptr` — n/a (no banking).
   label-in-immediate — n/a. **typed VDP fns — ADOPTED** (the macro-port rule
   worked example). `assert`/`raise_error` — n/a (no debug asserts in section).
6. **Noticing clause**: section proposes no new house-format item — its idioms
   (VDP macros, mem-to-mem) are already feed-forward entries this tranche.

**Feed-forward additions (this tranche → step-2 checklist):**
- `jbra`/`jbsr` for direct-label transfers (already listed) — section is a heavy
  consumer (5 jbra, 8 jbsr).
- **The typed VDP command interface** (`vdp_comm`/`vdp_comm_reg` with sum-type
  target/op) is now house format for any VDP-command call site.
- **Mem-to-mem move with two pinned-`.w` abs operands** — house format where the
  bare form would demand two-way RelaxAbsSym (ledgered gap).

## Step 3(a) — language/format asks
- **Ceremony scan**: no high lines-per-intent proc. The 23 const mirrors + drift
  ensures (~46 lines) are the deferred-struct-twin + first-VDP-consumer tax, not
  ceremony — they shrink when the Sec/Act shared-struct module + the VDP-macro
  shared home land (both ledgered). NOT a DSL candidate.
- **Comment-as-compensation**: no recurring what-comment shape. The VDP header
  comments explain the bit-trick (a why), not a missing language feature.
- **Escape-hatch census** (by shape): `extern()` ×21 (all drift-lock `ensure`s —
  the standard deferred-mirror shape, ledgered against the shared-home asks);
  bareword link symbols ×~30 (RAM/proc — the resolved-at-link idiom, not a
  detour). No transliteration blocks. No new ask — the counts feed the two
  shared-home rows (VDP macros, Sec/Act struct).
- **Domain-type scan** — **SectionId / GridCoord candidates (R4).** `sec_x`/
  `sec_y` (grid coords) and the flat section id flow as raw `.b`/`.w` through
  FlatIDXY/GetSecPtrXY and across the entity_window seam. A `GridCoord`
  newtype (u8, bounds = grid_w/grid_h) and a `SectionId` newtype (flat id) would
  name intent at the entity_window↔section boundary and catch a sec_x/sec_y
  swap. FP-taste gate: EARNS its place only if the entity_window port adopts it
  too (single-file typing of a cross-seam value is half-typed). → **ledger as a
  cross-file newtype candidate, adopt with the next Sec/Act typing pass** (pairs
  with the shared-struct-module row; see [[emp-sonic-newtype-candidates]]).
- **Noticing clause**: no opportunity class the lines above miss.

## Step 3(b) — reads-wrong / could-be-better
- **Comment-claim audit**: the `RedrawPlanes` header claim "never fires
  mid-traversal (~3 frames synchronous)" — verified against the code: it fires
  only on `Section_Plane_Dirty` (init + cache recovery), confirmed. The
  `.pA_zero`/`.pB_*` "clr.w would RMW-read VDP" comments — verified correct (VDP
  data port read hazard; moveq #0 source avoids it). No false claims.
- **Contract audit**: **FillInitial** — the AS header claimed `Clobbers: none`;
  the body scratches d0. `.emp` declares the truthful `clobbers(d0)` + a site
  comment. **Twin header comment should be corrected too** (finding — the twin
  says "none"). GetSecPtrXY/FlatIDXY contracts verified: both `clobbers(d1)
  out(d0[,a0])`, PRESERVE d2/d3/a2 (see R4). RedrawPlanes/UpdateColumns full-set
  clobbers match the movem save/restore.
- **Name audit**: labels read true (`.pla_fill`/`.pla_next`/`.pA_data`/`.pB_*`
  describe the plane-A fill phases). No misleading names.
- **Magic-number audit**: `$8F80` (VDP autoinc reg $8F ← $80 col-major stride),
  `$8F02` (autoinc $02 row-major), `$2700` (SR interrupt-disable) — all carry
  site comments. **Candidate for named consts** (`VDP_AUTOINC_128`/`_2`, `SR_NO_INT`)
  but the AS twin uses literals; the .emp keeps literals + comments to stay
  byte-isolated → ledger as a VDP-register-const candidate (shared-home class).
- **Cold-reader test**: one frame through `UpdateColumns` reads cleanly from
  headers + comments EXCEPT the four clamp ladders' cross-clamp interaction
  (`.left_clamp_skip`/`.right_clamp_skip2` etc. write the OPPOSITE tracker) — a
  reader must trace the implementation to see right-side streaming updates the
  left tracker. → step-4 construct + a header note (finding).
- **Codename-reference audit**: DONE — `tranche-15 R1`/`(R6)`/`step-3(b)` →
  behavioral reasons (commit `emp(t15 step-3b): scrub session codenames`).
- **Noticing clause**: no reads-wrong class the lines above miss.

## R4 — FlatIDXY clone contract-check + multiply strategy (with numbers)
- **Clone contract**: GetSecPtrXY inlines the same `sec_y*grid_w+sec_x`
  repeated-add that FlatIDXY computes. Contracts are dedup-COMPATIBLE (both
  preserve d2/d3/a2, clobber d0/d1, out d0). BUT the register strategies differ
  deliberately: FlatIDXY uses **d1 as counter, re-reads `Act_grid_w(a2)` each
  iteration**; GetSecPtrXY uses **d2 as counter (stack-saves sec_x), caches
  grid_w in d1** — one memory read vs N. Deduping would force one strategy:
  GetSecPtrXY→FlatIDXY loses the grid_w cache (slower on GetSecPtrXY's hotter
  RedrawPlanes/parallax path); FlatIDXY→GetSecPtrXY's shape needs the stack
  save. **NOT dedup — the two strategies are a deliberate speed/register split;
  a shared helper can't capture both.** (step-5 not-taken, with reason.)
- **Multiply strategy (numbers)**: repeated-add. Grid is 3×3 (act_descriptor
  GRID_W/H=3) → sec_y ≤ 2 → **≤ 2 `add.w` iterations (~8 cy)** vs `mulu` **~70
  cy** vs a per-act grid_w-indexed lookup (build cost + a table per act,
  overkill). Even at MAX_ACT_SECTIONS=48 (max ~7×7 grid, sec_y ≤ 6) → 6 adds
  ~24 cy < 70. **Repeated-add is optimal across every valid grid — KEEP.**
  Frequency: called by entity_window's scan (per-entity, streaming), parallax
  (BG lookup), RedrawPlanes (init only); the streaming rate is unprofiled
  (owed probe below), but the per-call arithmetic wins regardless of frequency.

## Step 4 — construct pass
- **Macro-port rule (worked example — cite):** the VDP command macros were
  designed as an INTERFACE REDESIGN, not transliterated. Wrong-input scan →
  closed target/op vocabularies became `comptime enum VdpTarget/VdpOp` +
  exhaustive-match mappers (the donor's `%100001`-class int consts are
  implementation detail, drift-locked to the .asm truth). Guard upgrade → the
  four `vdp_comm_reg` guards all select ENCODING CASES (behavior), none were
  vocabulary validation, so **none died to types** (honest negative result —
  the rule working). First-consumer duty → section designs the interface every
  later VDP consumer inherits. Taste → the call site reads
  `vdp_comm_reg(d2, VdpTarget.Vram, VdpOp.Write)`, intent-named.
- **Structural-clone scan → the four clamp ladders.** `Section_UpdateColumns`'s
  right/left/bottom/top sections each run: cache-clamp + VDP-wrap-clamp +
  stream-loop (budget-gated) + **cross-clamp** (right updates the left tracker,
  etc.). Four near-identical bodies differing in {tracker pair, direction,
  Draw fn, ±63 sign}. **Candidate:** an `emit_stream_edge(dir, tracker, drawfn)`
  comptime-fn (the emit_piece_loop class). **Deferred — step-4 verb (c) ASK,
  not build:** the four bodies differ in control-flow shape (the row loops call
  `Draw_TileRow_FromCache` with a different buffer-reserve constant AND the
  bottom/top use `.s` loop branches while right/left use `.w`), so a clean
  parameterization needs the cross-fragment-label-scope capability (the
  emit_piece_loop/latch_pad gap) to take the caller's proc-local labels. Ledger
  as a stream-edge-template candidate, blocked on that language ask. The
  cross-clamp interaction gets a header note now (cold-reader finding).

## Step 5 — optimize (hot-path, per-line table)

**`Section_UpdateColumns` (per-frame hot):**
| Line | Outcome |
|---|---|
| Invariant ladder | Camera_X read once (d6), reused across right/left. `Act_grid_w(a0)` read per-edge (right clamp) — invariant over the frame; a hoist saves 1 read/frame but costs a register across the four edges → **not-taken** (marginal, register pressure). |
| Counter/cache audit | `Plane_Buffer_Ptr` is the budget: every stream loop (right/left/bot/top) checks `cmpi.w #PLANE_BUFFER_SIZE-2-…, Plane_Buffer_Ptr` BEFORE `Draw_*`. All four emission paths charge it (Draw_* advances the ptr). **Verified: every path that emits checks the budget** — no asymmetry. (Fable's flagged line — confirmed clean, but see owed probe for the live rate.) |
| Guard-coverage | The budget check is the sole guard against Plane_Buffer overflow; present on all four loops. The act-boundary + cache + VDP-wrap clamps bound the tracker deltas. **All emission paths guarded.** |
| Hardware cross-check | Column/row writes go through the plane buffer (drained in VBlank by VInt), not direct VDP here — so no in-proc DMA/VBlank race. `RedrawPlanes` does direct VDP pokes under `sr #$2700` (see below). |
| Silent-tradeoff | The cross-clamp (right streaming shrinks the left tracker to right−63) is a CHOSEN behavior (VDP 64-cell wrap span) — gets a header note (was uncommented; cold-reader finding). |

**`Section_RedrawPlanes` (init / cache-recovery only, ~3 frames synchronous):**
| Line | Outcome |
|---|---|
| Invariant ladder | Cache origin/stride math (`origin_row × 160`) computed per-column inside `.pla_fill` — invariant over the 64-col loop for the row part but the COLUMN pointer changes; the per-column recompute is necessary (col varies). Not-taken. |
| Counter/cache | The Part-A/Part-B row split (nametable rows start_nt_row..63, then 0..start_nt_row-1) correctly covers the 64-cell ring wrap; data vs zero-fill counts (`data_A`/`zero_A`) sum to count_A. Verified. |
| Guard-coverage | Cache-range check (`Cache_Left_Col`/`Cache_Head_Col`) BEFORE setting the VDP address skips off-screen columns (retain old content, no black flash) — the guard is on the emit path. |
| Hardware cross-check | Runs under `move.w #$2700, sr` (interrupts masked) with direct VDP pokes — correct for an atomic full-plane rewrite (no VBlank mid-write). SR restored at exit. VDP autoincrement set/restored ($8F80 col-major → $8F02 row-major → $8F02 default). **Hardware-correct.** |
| Silent-tradeoff | `moveq #0` zero-source (not `clr.w (a6)`) avoids the VDP data-port RMW read hazard — commented, correct. |

**OWED PROBE (the RescanY debt's sibling):** the live per-frame cost of
`Section_UpdateColumns` under an ACTIVE streaming window (OJZScroll scene) is
UNPROFILED — same reason RescanY sat at 0% in the churn packet (that scene
didn't stream). **Static bound:** per-frame work is O(columns+rows revealed
this frame), hard-capped by the `Plane_Buffer_Ptr` budget (each edge stops at
`PLANE_BUFFER_SIZE-2-(header+cells)`), so it CANNOT exceed the plane-buffer
budget per frame regardless of scroll speed — bounded by construction.
**Owed:** an OJZScroll oracle profile (load worktree `s4.debug.bin`, enter
OJZScroll, profile `Section_UpdateColumns` over N streaming frames at scroll
speed) to attach the real column-streaming rate + lag-frame check. Profiler is
ready (`emulator_set_profiler`); the scene vehicle is OJZScroll (the R-A1
scene-pin hook precedent). Recommend Fable's hot-path second look run it or
ratify the static bound.

## Loop-until-dry
Retrospect pass 2 (post step-4/5): the only new items were the FillInitial-twin
header fix, the cross-clamp header note, and the ledger rows below — all
recorded, no new construct/optimize triggered. **Retrospect is DRY.**

## Ledger rows (this tranche)
1. **Sec/Act shared-struct module** (R3) — section adds Act_sec_grid_ptr/grid_w/
   grid_h/act_bg_layout + Sec_sec_bg_layout/Sec_len offset mirrors. The "2nd
   consumer" trigger is **already met** (entity_window + section + act_descriptor
   game-side `struct Act`/`struct Sec` twins all consume Sec/Act); adoption is
   deferred on **tranche-size grounds**, expected as the next sst-usability-style
   batch, NOT re-gated on a condition that has fired. Unwind set includes the
   act_descriptor.emp game-side struct twins (collapse/re-point when the shared
   engine module stands up).
2. **VDP-macro shared home** (`engine.vdp`/`engine.macros`) — vdp_comm/
   vdp_comm_reg + the six VDP type equs; adopt at the 2nd VDP-macro consumer
   (plane_buffer/tile_cache/load_art).
3. **VDP-register-const candidate** — `$8F80`/`$8F02`/`$2700` as named consts
   (shared-home class, byte-isolated for now).
4. **SectionId/GridCoord cross-file newtype** — adopt with the Sec/Act typing
   pass (single-file typing of a cross-seam value is half-typed).
5. **stream-edge-template** — the four clamp-ladder clones; blocked on the
   cross-fragment-label-scope language ask (emit_piece_loop class).
6. (already filed) two-way-RelaxAbsSym spelling gap (5 sites); Option-A enum→repr
   cast (T4 symmetry, demand point #1); frozen 6-gate mixed define-set.

## Small fixes applied this pass (byte-neutral)
- FillInitial twin header `Clobbers: none` → `Clobbers: d0` (contract audit).
- Cross-clamp header note on UpdateColumns (cold-reader).

## Step 6 — corpus sweep

Enumeration of this tranche's additions across ALL prior `.emp` files; every
outcome named. (The retro macro-interface batch — `vram_art` etc. — is a
SEPARATE post-t15 batch, already ledgered, NOT folded in here.)

1. **Mem-to-mem two-pinned-abs spelling** — **NOT-AN-INSTANCE corpus-wide,
   structural.** Grepped every ported `.emp` and every ported `.asm` twin for
   `move.X (sym).w/.l, (sym).w/.l` (two memory-symbolic operands): **zero hits**
   (section's 2 excluded). The reasoning is structural, not just empirical: a
   ported file containing the pattern could NOT have reached byte-green before
   the lowering feature existed — it would have hit the "two symbolic operands"
   diagnostic — so the ported corpus is provably clean by construction. The 5
   demand sites are all in UNPORTED files (section ×2 — now converted; vblank ×3
   — pending vblank.asm's port). No retrofit; no ledger (the vblank sites convert
   at vblank's port time).

2. **Typed VDP command interface** (`vdp_comm`/`vdp_comm_reg` + VdpTarget/VdpOp)
   — **NOT-AN-INSTANCE.** section.emp is the SOLE `vdp_comm`/`vdpComm` consumer
   in the `.emp` corpus (`grep -rl` = section.emp only). Eyeballed the two
   plausibles: `vdp_init.emp` and `hblank.emp` construct no VDP COMMAND words —
   their `#$8xxx` writes are VDP REGISTER writes (excluded, not command words),
   and runtime command-word construction (dma_queue/plane_buffer) isn't ported.
   Nothing to retrofit; the interface is first-consumer, its shared home
   ledgered for the 2nd.

3. **bool-typed comptime-fn params** — **NOT-RETROFITTED (reasoning carries).**
   The instance-shaped site is sprites.emp's flip family (`y_term`/`size_link`/
   `tile_term`/`x_term`/`emit_piece_loop`, all `xflip/yflip: int`). Call sites
   pass comptime literal 0/1 (`emit_piece_loop(0, 0)`/`(1, 0)`). Same
   identical-domain/no-catch-value reasoning as the macro-interface enumeration:
   xflip/yflip are an unambiguous 0/1 flip selector with NO out-of-vocabulary
   value a `bool` would catch — a `bool` names intent but adds zero safety, so
   it's an at-next-touch cosmetic, not a retrofit. (Distinct from section's
   `clr: bool = true`, where the win was the DEFAULT-PARAM ergonomics at a 3:2
   dominant call value, not type safety.) NOT-RETROFITTED, at-next-touch.

4. **let-asl-relax twin technique + step-2 format items** — **PROCESS/FORMAT,
   fed-forward, no code sweep.** The relax-then-write-explicit lockstep method
   is a process entry (packet step-2 section). The step-2 spelling additions
   (`jbra`/`jbsr` for direct-label transfers, the typed VDP interface, the
   mem-to-mem pinned-`.w` spelling) are FEED-FORWARD checklist entries: future
   ports convert at port time; prior files join the at-next-touch backlog — that
   IS the sweep channel for format items (per the feed-forward rule), not a
   this-tranche code rewrite.

**Sweep verdict:** every addition is not-an-instance (1, 2), not-retrofitted-
with-reason (3), or process/feed-forward (4). No corpus retrofit owed;
no new ledger row from the sweep. Merge-ready pending the hot-path second look.

## Second look + live probe (Fable)

**Hot-path second look — APPROVED.** The clamp-ladder + Plane_Buffer_Ptr budget
lines held under a fresh re-walk. The asymmetric buffer reserves (rows reserve
`4 + PLANE_H_CELLS*2`, columns `8 + PLANE_V_CELLS*2`) verified correct against
`Draw_TileRow_FromCache`'s ACTUAL emission — rows never seam-split (one entry),
so the row reserve is right; columns can split at the NT 63/0 seam (two entries),
so the larger column reserve is right.

**OJZ streaming probe — RUN LIVE (Fable; oracle, worktree s4.debug.bin,
OJZScroll, 120-frame averages, both axes).** The static bound is superseded by
measured numbers (do NOT ratify the bound):
- **Horizontal 8px/f:** `Section_UpdateColumns` = **6,043 cyc/f (4.7%)** inclusive;
  `Draw_TileColumn` = 5,404 inside it → section's own clamp/scan ≈ **640 cyc/f**.
  **Zero lag** (VSync idle 21.9%).
- **Vertical 8px/f:** `Section_UpdateColumns` = **12,679 (9.9%)** incl.
  `Draw_TileRow_FromCache` 11,707 → own ≈ **970 cyc/f**. Rows ≈ 2× columns
  (11.7k vs 5.4k) — the per-cell circular-cache arithmetic in the row path.
- **`Section_FlatIDXY` = 92 cyc/f live** — **R4 keep-repeated-add ratified
  empirically** (the ≤2-iteration multiply is negligible live).
- **`EntityWindow_RescanY` = 257 cyc/f (0.2%) — FIRST nonzero ever** (+RescanRings
  214, TrySpawnRing 161). **The RescanY streaming-profile debt CLOSES** (the
  churn-packet OJZ row — the profile that stayed at 0% for want of a streaming
  scene now has live numbers).
- **Lag driver is NOT section** — VInt_Lag fired under sustained vertical
  streaming, driven by `TileCache_FillRow` **48,939 cyc/f avg (38.2%)** /
  `Tile_Cache_Fill` ~40%. Section's own ≈1k is not the driver; the tile_cache
  domain is (pre-recon ledger jot below).

**Verdict:** section.emp is hot-path-clean; the streaming cost is bounded and
lag-free for its own work; the lag lever is tile_cache, a separate domain.
