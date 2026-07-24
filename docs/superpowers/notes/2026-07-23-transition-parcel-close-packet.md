# Boundary-crossing transition parcel — CLOSE PACKET

The first deliberate behavior-fix parcel on ported parallax code. Five mechanism
bugs in the section-boundary transition state machine, closed with a standing rig
capability and one ratified sigil-ISA addition. Branch `parallax-transition-parcel`
(both repos). **All five items closed with rig A/B evidence; full paired strict
2490/0; ready for the merge queue.**

## Headline

- **Framing:** every bug here is a **real defect in the transition MECHANISM,
  latent in shipped DATA** — shipped play fires NO transition (all OJZ sections
  share `sec_parallax_config = 0` → every crossing calls `StartTransition(current)`
  → no-op) AND all shipped configs share render mode. So none of B6/B2/B3/B1 is a
  visible shipped artifact; each is provable only via the rig's staged transition,
  and each hardens the mechanism before per-section configs (differing modes/
  factors) are reintroduced. Shipped-config invariance is proven per fix.
- **Standing capability built:** the crossing-drive rig (poke-driven, ROM-
  preserving, wedge-avoiding) — `notes/2026-07-23-crossing-drive-rig-protocol.md`.
  Reusable for all future transition work.
- **Toolchain:** demanded + built `divs.w`/`divu.w` in sigil (gate-ratified Option
  A; the abs-sym precedent). The only ISA addition riding this parcel.

## Canonical re-baseline (byte-changing, fresh dual builds)

| | plain `s4.bin` | debug `s4.debug.bin` |
|---|---|---|
| pre-parcel | `00f609a5` / 421089 | `80d14183` / 429134 |
| **post-parcel** | **`0bfa5b79` / 421161** | **`9d962703` / 429204** |

`EndOfRom` = `0x5DB60` UNCHANGED throughout (all growth absorbs in padding before
`org $10000`). Per-fix CRCs in the evidence log. **PROVENANCE re-baseline pending
at merge** (per the brief — byte-changing).

## Items (in dispatch order)

0. **Crossing-drive rig — PROVEN.** OJZ default-boot scene + `Debug_Scene_Freeze`
   (freezes camera + entity scan, NOT parallax) + poke the transition state
   directly (all sections share config 0, so a config change must be staged). New
   reusable caution banked: the sit-on-breakpoint gotcha (must `step 1` off a BP
   before `resume` or the body doesn't run). Rig commit `63a3847`.
1. **B6 — promote-frame CC-clobber (rebuild skip). CLOSED, aeon `2f9cedf`.**
   Reorder so `move.l d0,Current` is last, restoring `.config_resolved`'s Z-from-d0
   invariant. **Length-NEUTRAL** (pure reorder — zero region-slide ripple).
   A/B: promote-frame Hscroll_Buffer sentinel survived → overwritten.
2. **Window-slide rider — CLOSED (observed).** Real `EntityWindow_Slide` via the
   scroll-target drive; anchor (0,0)→(1,0) single-axis, DEBUG single-axis assert
   held, migration ran. Per-section mask VALUE audit deferred (poked-teleport
   confound; entity-window scope) → ledger row. Commit `cfbb7d1`.
3. **B2 — active-config mode contract. CLOSED, aeon `7482ebf`.** Gate: Option B +
   sub-decision (i). One `Parallax_Active_Config` accessor (Target while
   frames>0); routed the two straggler consumers (HScroll DMA length in
   buffers.asm + Vscroll_Write) through it — all five mode/format/length/stride
   consumers now agree on Target-from-frame-0. A/B (per-cell rig fixture): HScroll
   DMA before=LINE(896 B, Current) → after=CELL(112 B, active) = coherent.
   +0x10 parallax; full ripple.
4. **B3 — frames-remaining ramp + `divs` in sigil. CLOSED, sigil-core `714c5db`
   → aeon `f4d6aea`.** `step = (target−current)/frames_remaining` (`divs.w`),
   converges EXACTLY by the last window frame → promote `.snap_b` is a no-op.
   Demanded/built `divs.w`/`divu.w` in sigil (TDD, asl-verified encode tests,
   zero new clippy, sigil-core lands before the consuming aeon commit). Invariant
   (contract comment): frames_remaining≥1 on the divide path (divide-by-zero
   structurally unreachable); `ext.l` gap, quotient fits a word (no overflow).
   Perf: `divs` ~120-158 cyc × bands, TRANSIENT (transition windows only) —
   gate-accepted. A/B: before residual 211 px snaps at promote (pop); after
   converged by frames_remaining=1, promote no-op. +0x8 parallax; full ripple.
5. **B1 — re-cross cancel branch. CLOSED, aeon `1fc0897`.** StartTransition's
   `a0==Current` no-op becomes `.recross_current`: cancel a staged transition
   (clear Target/frames, snap bands back, restore current's mode). A/B (re-cross
   via the real CheckBoundary path, `Prev_Sec_X` poke; a0=Current confirmed):
   before continues, after cancels. +0x1C parallax; full ripple.

## Per-fix pass-buckets (house format)

- **Correctness (the parcel's spine):** B6 (Z-from-d0 invariant restored), B2
  (five-consumer active-config coherence — the tear), B3 (exact convergence, no
  pop), B1 (re-cross cancel). All four are correctness-hardening on a latent
  mechanism, each with a live A/B on the canonical (or fixture) ROM.
- **Toolchain (step-neither):** `divs.w`/`divu.w` sigil ISA — a demanded feature
  made real (mirrors `muls`; the only ISA addition; own commit before its consumer).
- **Ripple discipline:** 5-site doctrine exercised 3× (B2 +0x10, B3 +0x8, B1
  +0x1C — B6 was length-neutral). Each: `repin`→pins.rs, hand-edit engine.inc (2
  orgs) + repin_pins.rs delta-chain; mixed_dac/repin.toml untouched (no sound-
  content ref / no region added). EndOfRom stable every time.

## Ledger + kill-list (gate-directed)

- gap-ledger: EntityWindow_Slide per-section mask VALUE audit deferred (natural-
  scroll, entity-window scope); B2 (ii) engine-owned per-frame mode-register write
  (consolidation follow-up). Commit `ec1acae`.
- twin-scaffolding kill-list row 35: the OJZ harness mode-register force-write
  (`ojz_scroll_test.asm` :234-273), kill condition = (ii) ships.
- Constraint-5 finding: pc-relative indexed `(d8,PC,Xn)` EXISTS in BOTH frontend-as
  AND `.emp` lowering (value.rs `PcRelIdx`) — no emp-side gap, no ledger row (the
  reciprocal-table workaround would have been feasible; `divs` chosen as cleaner).

## Merge (sequential queue)

- Branch tips: aeon `1fc0897`, sigil `6f5166a`. Both worktrees clean; full paired
  strict **2490/0**.
- Coordinate with the item-13 parcel: ready-first-goes-first; the second re-verifies
  on the moved master (fetch `origin/master`, not local). Byte-changing ⇒ PROVENANCE
  re-baseline at merge (plain `0bfa5b79`/421161 · debug `9d962703`/429204).
- New sigil ISA (`divs`/`divu`) is self-contained (own commit `714c5db`, +2 tests)
  — no cross-dependency with item-13's disjoint files.
