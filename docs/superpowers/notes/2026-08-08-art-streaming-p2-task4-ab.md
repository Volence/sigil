# A/B evidence — art-streaming P2a Task 4: page-in request queue + cancel/flush + init-path routing

**Parcel:** `art-streaming-p2-task4`. Replaces the Task-3 DEBUG self-test scaffold
in `engine/level/page_in.emp` with the real page-in request FIFO + landing
handshake + cancel/flush, and reroutes level init (`Level_LoadArt`) through the
streaming path. ADDITIVE + REFACTOR parcel (a new mechanism plus a scaffold
deletion), not a byte-neutral optimization, so "A/B" documents the intended byte
movement and the mechanism, not before/after semantic equivalence of relocated
code. This COMPLETES phase P2a.

## What changed

Aeon (`feat/art-streaming-p2`):
- `engine/level/page_in.emp` — the Task-3 self-test scaffold (`.find_largest` /
  `.verify` / the hardwired largest-page test, `DEBUG && HAS_ACT_ART_POOL`-gated)
  is DELETED. `PageIn_Process` becomes the real dispatcher: Suspended -> resume;
  Land_Pending -> retry landing; Staging_Busy -> hold; else pop the FIFO head
  (demand before prefetch, two-pass scan over the front-packed array), resolve
  `page_id -> wrapper ptr` via `PageIn_Pool_Table` + `dest = page_id << 13` +
  `length = wrapper size word`, then either FALL into `ZX0R_Decompress` (ZX0 form)
  or enqueue a direct ROM->VRAM DMA (raw form, dormant in v1). On decode completion
  `.after`/`.land` queues an Important staging->VRAM DMA and raises
  `PageIn_Staging_Busy` (a full queue -> `PageIn_Land_Pending`, retry next frame).
  The frozen `PageIn_BankRegs`/`PageIn_Resume` @continuation procs are UNTOUCHED.
  New procs (ride behind `PageIn_Process` in the one page_in section, no new
  map.toml entry): `PageIn_EnqueueLanding` (brackets `QueueDMA_Important` with a
  `movem.l d3-d4` save/restore so the dispatcher's clobber set stays `d0-d2/a0-a3`
  and VSync_Wait's folded license never widens), `PageIn_Enqueue` (FIFO append,
  carry-on-full B&R rollback), `PageIn_Flush` (main-loop-only cancel: clears
  Suspended/InFlight/Land_Pending + empties the FIFO, LEAVES Staging_Busy;
  `Dbg_PageIn_Flushes++`).
- `engine/level/load_art.emp` — `Level_LoadArt` rewritten to drive the pool load
  through the FIFO: `PageIn_Flush`; set `PageIn_Pool_Table`; raise the DMA window
  budget to `DMA_BUDGET_BLANKED_INIT` (a 256-tile page's Important landing DMA,
  8192 B, exceeds the active-display 6144 window — display-off runs full-rate);
  enqueue every page; spin `VSync_Wait` until all resident; restore the budget;
  tail `BG_Init`. The direct `Art_Decompress` decode loop + the DEBUG-only
  `raise_error` drop handler are GONE (LOAD_ART len converges to 0x70 both shapes).
  `Art_Decompress` itself is KEPT — `engine/debug/compression_selftest.emp` still
  calls it for the S4LZ-vs-ZX0 dispatch coverage (grep-confirmed; the block tier
  uses `S4LZ_DecompressDict`, not this).
- `engine/system/vblank.emp` — `VInt_Level` gains the Staging_Busy release right
  after `Process_DMA_Important`: once the Important queue empties (slot == base),
  `clr.b PageIn_Staging_Busy` so the dispatcher can start the next decode. (VBLANK
  len grows plain 0x1B0->0x1D0 / debug 0x1C0->0x1D0, converging.)
- `engine/system/constants.emp` — `DMA_BUDGET_BLANKED_INIT`, `PAGEIN_QUEUE_SLOTS`,
  `PGRQ_DEMAND_BIT`, `PGRQ_DEMAND`.
- `engine/structs.emp` — `PageInReq` (4-byte FIFO slot: `pr_page_id`/`pr_flags`/
  `pr_pad`).
- `engine/ram.emp` — DELETE `Dbg_PageIn_Test_Cycles`/`Dbg_PageIn_Test_Done` (2
  DEBUG scaffold bytes, `@shape_divergent`). ADD the release FIFO + landing state
  with the bookmark record at the RAM tail: `PageIn_Staging_Busy`,
  `PageIn_Land_Pending`, `PageIn_Queue_Count`, `PageIn_Cur_Flags`, `PageIn_Queue`
  ([PageInReq; 8]), `PageIn_Cur_Dest`, `PageIn_Cur_Bytes`, `PageIn_Cur_Page`,
  `PageIn_Pool_Table` (game-RAM-only ripple past the bookmark record).

Sigil (`master`) — this parcel: NO source change (page_in is already registered;
the page_in region already spans `[PageIn_Process, BG_Init)`; the new procs ride
inside it). Baselines + goldens only: `crates/sigil-harness/src/pins.rs`
(regenerated), `crates/sigil-harness/tests/repin_pins.rs` (hand baseline brought
current), the seven goldens + off-canonical size tables + provenance chain
(refreeze), and any port/native anchor stub tables the byte movement shifts.

## Byte movement (from `repin` + region diff)

- **VBLANK** len +0x20 plain / +0x10 debug (the Staging_Busy release), converging
  to 0x1D0. Every engine region downstream slides +0x20 plain / +0x10 debug
  (MATH/DPLC/CORE/ANIMATE/RINGS/DELETE_OBJECT).
- **LOAD_ART** len 0x64->0x70 plain / 0xB0->0x70 debug (converges: no more
  shape-divergent raise_error).
- **PAGE_IN** len 0x5A->0x196 plain / 0x166->0x1A6 debug (FIFO/procs replace the
  DEBUG-only scaffold). SOUND_API (below page_in) slides +0x170 plain / +0x10
  debug.
- **RAM (DEBUG):** the 2 deleted scaffold bytes shrink the `@shape_divergent`
  block, pulling Object_RAM + the tail bookmark record -0x2 (PLAYER_1/
  DYNAMIC_SLOTS/PAGE_IN_IN_FLIGHT debug). Plain RAM unchanged; the release FIFO
  additions ride the RAM tail (game-RAM-only).
- **ASSEMBLED_LEN / DEBUG_ASSEMBLED_LEN hold** (engine growth absorbed by
  `org $10000`).

## Design note — init lands via Important (budget raise)

The plan lands the decoded page via an Important-priority staging->VRAM DMA and
routes init through that same path. An Important drain is byte-budgeted
(`DMA_BUDGET_NTSC` = 6144), and a 256-tile page is 8192 B, so a budgeted drain
could never move a whole page. `Level_LoadArt` runs display-OFF (full per-line DMA
rate on every line), so it raises the window budget to `DMA_BUDGET_BLANKED_INIT`
(= one page + a normal frame's charges) for the bulk load and restores the
active-display budget before display-on. This keeps the Important + Staging_Busy
handshake LIVE and exercised at init rather than dormant until P2b.
