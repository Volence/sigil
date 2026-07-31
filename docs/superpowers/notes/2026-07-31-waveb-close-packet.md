# Wave-B close packet (increment 1) — §17 optimization sweep

**Branches:** sigil `wave-b-computed-placement` / aeon `opt-wave-b`, off the Wave-A
masters. Chain: `asl-witness → wave-a-animate-a3 → waveb-plane-buffer-4 →
waveb-plane-buffer-2` (len 4). Strict 2859/0 (4 ignored) at close.

## What landed

- **B-0 — packed placement** (the wave's structural centerpiece, pulled forward from
  the G9 design step, ruled LEAN COMPUTED): Frozen placement = table-anchored islands
  + live-size packing with alignment inference and a relaxation fixpoint; canonical
  sonic4 joins via bootstrapped tables; ONE placement authority for all six targets;
  fold-identity proven byte-for-byte before any consumer moved. The rows-6/58 pin-tax
  class is structurally dead for ROM sections. (`2026-07-31-waveb-b0-computed-placement.md`)
- **plane_buffer #4** — wrap-check hoist: **−1748 cyc/f** (main-loop) at max-H;
  the first packed-placement consumer (+18 B absorbed automatically). Riders: zero-fill
  `clr.w`→`move.w aN`, `adda`→`lea` in Draw_BG_TileColumn.
- **plane_buffer #2** — VBlank column drain move.l pairs: **−744 cyc/f out of the
  VBlank window** (VInt_Level 55%→51% of the window; ~370/entry toward the ~11-entry
  worst case). KEEP ruled on the window rationale, numbers logged.

## Adjudicated skips (numbers in the parcel notes)

- **plane_buffer #3** (command-longword header): ~50–80 cyc/f steady, ~290 worst —
  under every bar, against an entry-format change that reopens the b96c861 tear
  invariant. LOG-AND-SKIP. Reopen: a drive with 10× entry counts.
- **core #1** (register-cached camera + branchless cull): **MEASURED REGRESSION
  +140 cyc/f on the churn drive and REVERTED.** The scene-shape finding: the
  per-dispatch d5/d6 re-cache tax (~24c × ~28 dispatches, object code preserves only
  a0/d7) exceeds the per-entry check savings when ~every live entry is on-camera —
  and no cull-heavy drive exists in the harness. The census's effort model priced the
  transform, not the drive shape. Reopen gated on a cull-heavy scene (big-level
  object spread), where the win side becomes measurable; the branchless window fold
  alone (~5c/axis/entry, zero dispatch tax) can ride any future core parcel.

## Deferred to increment 2 (with their prerequisite named)

- **entity_window #1** and **tile_cache #2**: both need RAM-layout growth — the RAM
  analog of B-0's packing (RAM sections are still hand-pinned `Pin`s). Build "RAM
  packing" as B-0b first, same fold-identity bar.
- **collision_lookup #1-3** (L): the fused lookup + Row80 comptime table; also wants
  a collision-heavy drive for its BEFORE (Player_Main is 668 cyc/f on the OJZ scene —
  the ~30%/sensor lever needs sensor traffic to show).
- **Wave C** (parallax row-35) untouched, next after increment 2.

## Step-3 (language/tooling asks) vs step-5 (engine) findings

**Step-3 class:**
- The packing walk wants the map manifest to own DECLARED ORDER + ISLAND anchors as
  first-class syntax (today: inferred from tables/pins) — the SPEC2:199 end-state.
- `assert_region_matches` is copy-pasted across 36 port gates — a shared
  `test_support` home would have made the pad-tolerance a one-site change.

**Step-5 class:**
- The VBlank window (not total CPU) is the binding budget on scroll drives —
  VInt_Level was 55% of it before pb#2. Future PF parcels should report window
  occupancy, not just self-time.
- The dead-stack/interrupted-PC/mid-VBlank-regs capture-aliasing classes are now
  well-characterized A/B residue; quantum captures alias in-flight VDP writes
  (pb#4's 2-cell off-screen VRAM transient) — code-point anchors dodge all of it
  when the oracle's breakpoint path is healthy.

**Neither-bucket headlines:**
- The refreeze machinery had a bootstrap hole (pins regenerate FROM a build that
  can't build against stale pins) — invisible until the first byte-GROWING parcel;
  A3 dodged it by being size-neutral. B-0 closed it.
- Oracle instability dominated wall-clock: the reload unload-race (silent rejection,
  wedged worker), the ExecuteThread stall after killing a client mid-`run_to`,
  `load_symbols`-then-reload racing the loader, the same-path reload short-circuit,
  and the `screenshot` path parameter being ignored. All logged for the oracle tree;
  the A/B drivers now reload-first + verify the cart identity from the diagnostic.
- Test-order-revealed latencies: three port tests assumed SECTION plain_len ==
  debug_len (a coincidence the first alignment pad broke); the literal-pin-value
  baseline test and chained_resume retired as pin-tax corpses with their protections
  re-homed.
