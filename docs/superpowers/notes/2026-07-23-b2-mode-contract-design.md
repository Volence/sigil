# B2 — parallax transition mode-contract design pass (GATE DELIVERABLE)

The transition state machine's "active config" coherence. B2 defines the contract
that B3 (scroll ramp) and B1 (cancel branch) land inside. **This note is the gate
checkpoint — no B2/B3/B1 code cut until the gate rules.**

## Framing (important for priority)

In shipped play **no transition ever fires**: every OJZ act1 section has
`sec_parallax_config = 0` → all inherit `ParallaxConfig_OJZ_Default`, so a
boundary crossing calls `StartTransition(OJZ_Default) == Current` → no-op
(established during rig build). AND every shipped parallax_config uses the SAME
render mode (per-line H — deform table non-null — + whole-plane V). So B2's tear
is **doubly latent**: (1) no config-changing crossing occurs, and (2) even if one
did, no two shipped configs differ in mode. B2/B3/B1 (like the already-fixed B6)
are **real bugs in the transition MECHANISM, latent in current DATA, provable now
only via the rig's staged transition.** The parcel hardens the mechanism before
per-section configs (differing modes) are reintroduced — it does not fix a
visible shipped artifact. The gate should weigh scope with that in mind.

## The incoherence (consumer map, verified this session)

During a smooth transition (`Transition_Frames > 0`), five consumers pick a
config. Today they DISAGREE:

| # | Consumer | Config read | Where |
|---|---|---|---|
| 1 | Band factors + layout (lerp destination) | **Target** | parallax.emp Step 1 (:367-372) |
| 2 | Fill format (Fill_PerLine 224ln / Fill_PerCell 28cell) | **Target** (a0=active) | parallax.emp :564-579 |
| 3 | VDP mode-set-3 register ($0B: H %10/%11, V bit 2) | **Target** @ frame 0 | StartTransition :252-268 (+ harness re-force every frame, ojz_scroll_test.asm :234-273) |
| 4 | **HScroll DMA length** (112 B cell / 896 B line) | **Current** ✗ | buffers.asm :168 |
| 5 | **Vscroll_Write** (per-column vs whole-plane VSRAM) | **Current** ✗ | parallax.emp :304 |

Consumers 1-3 already commit to **Target from frame 0**; the buffer is *built* and
the VDP is *told* to render in Target's mode immediately. But #4 ships that buffer
with **Current's** DMA length, and #5 emits VSRAM in **Current's** V-mode.

**Tear mechanics** when Current and Target modes differ:
- Current per-cell (112 B) → Target per-line: fill writes 896 B/224 lines, VDP
  reads per-line, but DMA copies only 112 B → 28 lines updated, 196 stale = the
  ≤16-frame tear (persists the whole window, not just one frame).
- V-mode mismatch: `Vscroll_Write` lays VSRAM per-column while the register
  (bit 2) says whole-plane (or vice-versa) → VSRAM read with the wrong stride.

## The design question

Structural attributes (mode, band layout, count) CANNOT lerp; only Plane B scroll
(a per-band scalar) can. So a structural change between two configs must "jump" at
one instant. The contract decides WHERE that jump lands and forces all five
consumers to agree around it.

### Option A — defer the switch to completion (active = Current until promote)
All five consumers use **Current**'s mode/format/length until `Transition_Frames`
hits 0; at the promote frame mode + layout + the final scroll snap flip together.
- Requires changing #1/#2/#3 (currently Target-at-frame-0) to hold Current, while
  still lerping scroll toward Target's factors → the builder must sample Target's
  band factors but emit in Current's mode/layout. Invasive; fights the current
  design. The structural jump lands at the promote frame (compounds with B3's
  snap frame).

### Option B — commit at frame 0 (active = Target from frame 0) — RECOMMENDED
Define ONE accessor **`Parallax_Active_Config = (Transition_Frames > 0) ?
Target_Config : Current_Config`** and route ALL five consumers through it. #1/#2/#3
already do this; the fix is to make #4 (HScroll DMA length) and #5 (Vscroll_Write)
read the active config too. The new config's mode + layout take effect at frame 0
(the transition commits structurally at once); the window then smooths only Plane
B scroll. The promote frame becomes structurally a NON-EVENT (mode/layout already
switched) — it only finalizes the scroll.

**Why B:**
- Minimal + consistent: aligns #4/#5 with the existing majority (#1/#2/#3), rather
  than dragging three consumers back to Current.
- Coherent every frame: the buffer is always a valid Target-mode buffer with
  lerping scroll; no format/length/stride mismatch at any frame.
- Simplifies B3 and B1: with the promote frame structurally inert, **B3** is a
  pure scroll-ramp concern (converge Plane B by the last frame; no mode
  entanglement) and **B1** (cancel) is a pure staging concern (re-point Target /
  clear frames). Both operate on scroll + staging, orthogonal to the now-coherent
  mode contract.
- For the common case (two configs differ ONLY in Plane B factors — same mode +
  layout, e.g. a BG-depth variant) there is NO structural jump at all under
  either option; the lerp fully smooths it. B's frame-0 commit only matters for
  mode/layout-differing configs, and there it is the cleaner "commit then smooth."

### Sub-decision for the gate: who owns the per-frame mode-register write?
Today the ENGINE (`StartTransition`) writes mode-set-3 once at frame 0, and the
TEST HARNESS (`ojz_scroll_test.asm` :234-273) force-writes reg $0B every frame
from the active config as a workaround for the same-config short-circuit. Under
Option B's single-accessor contract, the cleanest home is the **parallax engine**
owning a per-frame (or on-active-change) mode-register write from
`Parallax_Active_Config`, retiring the harness force-write. This is a slightly
larger change (touches the harness + adds an engine step). Options:
  (i) minimal — fix only #4/#5 to read active; leave the register as-is (frame-0
      set + harness force-write). Closes the tear; leaves the harness workaround.
  (ii) full — move the active-mode register write into the parallax engine,
      retire the harness force-write. Cleaner contract; wider diff.
**Recommendation: (i) for this parcel** (closes the tear with the tightest diff),
**ledger (ii)** as a follow-up consolidation (the harness workaround becomes dead
once the engine owns it — twin-scaffolding-kill-list candidate).

## Proposed contract (Option B, minimal)

1. Introduce `Parallax_Active_Config` — a single accessor/inline:
   `Transition_Frames > 0 → Target_Config, else Current_Config`. (Comptime helper
   or a tiny proc; both twins.)
2. Route consumer #4 (buffers.asm HScroll DMA-length select, :168) and #5
   (Vscroll_Write, parallax.emp :304) through it.
3. Leave #1/#2/#3 as-is (already active-config-correct). Register write stays
   frame-0 + harness (sub-decision (i)); ledger the engine-owned register write.
4. Verify with the rig: stage a transition between a per-line config and a
   per-cell config (engineered via a poked Target pointer to two mode-differing
   configs, or a temporary rig-only fixture), and A/B the HScroll DMA length +
   VSRAM stride during the window (before: Current's length/stride mid-window;
   after: Target's, coherent). NOTE: needs two mode-differing configs — may
   require a rig-only fixture since shipped configs share mode. Flag to the gate.

## How B3 and B1 land inside this contract

- **B3 (frames-remaining ramp):** pure Plane B scroll. Replace the fixed `>>4`
  geometric lerp (residual `(15/16)^16 ≈ 0.356` + promote-frame snap) with a
  frames-remaining-aware step so scroll converges by the last window frame. Under
  Option B the promote frame is structurally inert, so B3 is isolated to the
  `.write_b`/`.snap_b` band-loop math — no mode coupling.
- **B1 (re-cross cancel):** pure staging. Add a cancel branch to `StartTransition`:
  if `a0 == Current_Config` AND a transition is staged (Target set / frames > 0),
  clear Target + frames and snap to Current. Under Option B the "active config"
  reverts to Current the instant frames→0, so cancel = clear-and-snap, and all
  five consumers coherently follow active back to Current. B1 must also restore
  the mode register to Current's (since active reverts) — one write, consistent
  with the contract.

## Gate asks

1. Ratify **Option B** (active = Target from frame 0; single accessor; route #4/#5)
   vs Option A (defer to promote).
2. Ratify register-owner sub-decision **(i)** (minimal, ledger the engine-owned
   write) vs **(ii)** (full, now).
3. Bless the rig approach for B2's live A/B given no shipped mode-differing config
   pair (rig-only fixture vs poked two-config pointers).
4. Confirm B3/B1 fit as described (scroll-only / staging-only inside the contract).
