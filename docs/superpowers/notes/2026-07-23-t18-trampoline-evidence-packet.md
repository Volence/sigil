# t18 — HBlank RAM jmp-slot trampoline: EVIDENCE PACKET (porting agent → gate)

**Row 1088 executed.** The ratified S3K-style RAM jmp-slot trampoline replaces
the `HBlank_Dispatch`/`HBlank_Null` ROM dispatch pair. Byte-changing; strict
green; oracle synthetic-handler live-verify complete (5/5 criteria + injection
method). Branch tips: **aeon-t18 `40cb454` / sigil-t18 `c8619be`** (committed,
not pushed).

---

## 1. What was built (as ratified — Q1(a)/Q3 bindings, step-0 note `77d342a`)

- **`HBlank_Vector_Slot`** — a 6-byte executable RAM slot at the **RAM TAIL**
  (`0xFFB074`, last engine-RAM symbol before `Engine_RAM_End`). Idle = `rte`
  (`$4E73`); armed = `jmp handler.l` (`$4EF9` + 4-byte target). The old
  `HBlank_Handler_Ptr` `ds.l 1` cell becomes a **4-byte reserved pad in place**.
  - **Tail placement (improvement over the note's implied in-place shift):**
    ripples ZERO existing RAM addresses (the two established tail-placement
    precedents — `Dynamic_Live`, `Pfx_Memo` — with their own rationale comments).
    Live-confirmed: `PLAYER_1`/`DYNAMIC_SLOTS` baselines UNCHANGED in
    `repin_pins.rs`.
- **Vector `$70` → `HBlank_Vector_Slot`** (RAM), not a ROM proc.
- **`HBlank_Install(a0=handler, d0.b=counter)`** — patch slot to `jmp a0`,
  program reg `$0A` (HInt line counter), set reg `$00` IE1 — **all through the
  VDP shadow (+ dirty mask)** so a later `Flush_VDP_Shadow` never reverts the
  enable (Q1 binding #3, shadow coherence). The slot write is a direct RAM store
  (instant), so the jmp target is live the moment IE1 takes effect.
- **`HBlank_Uninstall()`** — idle `rte` back to the slot FIRST (instant, so a
  late in-flight HInt no-ops), then clear IE1 through the shadow.
- **`HBlankHandler` contract** — interrupt-transparency (observable
  clobbers = ∅), `rte`-terminated; the `as HBlankHandler` bless moves to
  `HBlank_Install`'s target argument (Q3 correction honored — NOT blanket movem).
- **boot** — idle-init the slot BEFORE interrupts unmask (Q1 binding #4). An
  8-byte `move.l #$4E734E73,(HBlank_Vector_Slot).w` keeps boot **byte-neutral**
  (fills both slot words with `rte`) so `vdp_init` and everything above hblank do
  NOT re-pin — the ripple is confined to the hblank region and below.

**Twin discipline:** the `.asm` twin carries explicit `.w` widths (sigil-AS pins
branch width, no relaxation). One idiom catch: **AS `^` is exponentiation, not
XOR** — the IE1-clear mask uses `#~HBLANK_IE1_BIT` (bitwise NOT) in both twins
(`.emp` `~` and AS `~` both yield `$EF`; `^` silently produced `$00`, caught by
the byte gate at region offset `0x39`).

---

## 2. Ripple (5-site, confined to hblank-and-below)

- **hblank region `0x12` → `0x48`** (+`0x36`). Every gated engine region below
  (`controllers`..`sound_api`, 18 regions) slides **+`0x36` both shapes**;
  regions past the `org $10000` data boundary absorb it (unchanged).
- `engine.inc`: 36 resume orgs bumped +`0x36` (script-applied, cross-checked
  against repin's paste blocks; `vdp_init` + data-bank orgs untouched).
- `pins.rs` regenerated (repin): `HBLANK` len; `H_BLANK_VECTOR_SLOT` (new,
  per-shape RAM pin `plain 0xFFFFB074 / debug 0xFFFFB098`); `HBLANK_UNINSTALL_OFF
  = 0x2C` replaces `HBLANK_NULL_OFF`; `H_BLANK_HANDLER_PTR` removed.
- `repin.toml`: hblank `start HBlank_Dispatch→HBlank_Install`; symbol
  `HBlank_Handler_Ptr→HBlank_Vector_Slot`; offset `HBLANK_NULL_OFF→HBLANK_UNINSTALL_OFF`.
- `repin_pins.rs`: +`0x36` baselines (ANIMATE/RINGS/CORE/DPLC/DELETE_OBJECT/
  SOUND_API bases) + t18 narrative; `PLAYER_1`/`DYNAMIC_SLOTS` unchanged.
- Harness tests retargeted: `hblank_port.rs` (byte gate + new cross-seam
  [writes `HBlank_Vector_Slot`/`VDP_Shadow_Table`/`VDP_Dirty_Mask`] + bare-name
  export proof `dc.l HBlank_Install/HBlank_Uninstall`); `hblank_negative_probes.rs`
  (probes retargeted; standalone missing-symbol now names `HBlank_Vector_Slot`);
  `m1c_vector_table` + `m1c_root.asm` (`$70` → `HBlank_Vector_Slot`);
  `mixed_dac_rom.rs` (hblank/controllers/game_loop block pins → reference-window
  equality, dropping hardcoded-base fragility); `parallax_port.rs`
  (`Section_GetSecPtrXY` VMA sourced from pins).

**Full paired strict `AEON_DIR=aeon-t18 cargo test --workspace`: 2488 / 0.**
**Fresh dual rebuild — NEW BRANCH CANONICAL: plain `83853176`/421089 ·
debug `ea6d1543`/429134** (supersedes step-2's `4e2d5f72`/`96339378`; plain size
neutral — hblank growth absorbed by `org $10000`; debug +2 = convsym symbol
appendix).

---

## 3. Oracle synthetic-handler live-verify (Q1(a)) — 5/5

**Injection method (binding #1):** oracle-inject. The synthetic handler
(`move.l #$C0000000,($C00004)` / CRAM writes / `rte` — clobbers nothing, so
interrupt-transparent trivially) poked into `Sound_Dbg_Mirror` (`0xFFAF4C`, dead
RAM in the plain build). `HBlank_Install`/`Uninstall` **emulated** (poke slot +
VDP shadow — the exact ops the procs perform): no 68k register-write tool exists
to PC-hijack the real proc, and the real install/uninstall CODE is already proven
byte-identical by the strict byte gate. Counter = `0x64` → HInt line ~100.

**Boot state (pre-install, plain s4.bin):** vector `$70` = `FFFFB074` ✓; slot =
`4E734E730000` (idle rte + defensive 2nd rte from the byte-neutral `move.l`) ✓;
reg `$00` shadow = `04` (IE1 off) ✓ — Q1 binding #4 satisfied live.

| # | Criterion | Evidence |
|---|---|---|
| i | HInt fires at the programmed scanline | breakpoint at the **slot** (`0xFFB074`) fired mid-frame after resume (the game's `Flush_VDP_Shadow` enabled IE1 from the shadow) |
| ii | entry via the RAM-slot `jmp` | at the slot break `SR=0x2404` (IRQ level 4 = HBlank context); single-step of the slot word → `PC=0xFFFFAF4C` (the handler) |
| iii | clean `rte` | stepped the handler's CRAM writes + `rte` → `PC=0x22DC` (`VSync_Wait+20`, interrupted game code); SP + 6 (interrupt frame popped); mask dropped from 4 |
| iv | uninstall → slot `$4E73`, IE1 off, no further HInts | emulated uninstall; read-back slot = `4E73`, reg `$00` = `04`; across **2 frames** post-Flush the handler breakpoint stayed **silent** (both broke at the scanline target `Process_DMA_Critical`, never the handler) |
| v | human-visible raster artifact | the all-CRAM-fill handler visibly reddened the display mid-frame — the HInt's CRAM write demonstrably alters the raster (screenshot `2026-07-23-t18-trampoline-hint-raster-artifact.png`) |

**(v) honest nuance:** a pixel-perfect *persistent split-line* was not cleanly
capturable because the OJZ scroll scene does not re-DMA the palette per frame
(once CRAM is written it persists across frames) and `run_to_scanline`'s
≥1-frame granularity cannot stop mid-frame on the single first-armed frame. The
color-change artifact (handler → CRAM → visible display change) plus the rigorous
i–iv proof (breakpoint at slot, stepped jmp→handler, stepped clean rte, silent
post-uninstall) fully establish the dispatch mechanism.

---

## 4. Window-slide mask-migration rider (gate addition #2) — ATTEMPTED, CARRIED FORWARD

Genuine bounded attempt this session: the OJZ scroll scene is **input-static**
(no auto-scroll; 5 s with a `EntityWindow_Slide` breakpoint = no fire, Camera_X
unchanged at 96). Switched to the debug ROM; a direct **Camera_X poke reverts**
(un-frozen `Camera_Update` overwrites it), and `Debug_Scene_Freeze` *skips*
`EntityWindow_Scan` — the very code that fires the slide — so freezing can't
drive one either. The first section boundary is ~2048 px away, so a clean
deterministic slide needs the scroll target driven across it (a long held-input
scroll, which carries the §D **press-onto-breakpoint** wedge risk) or a
frame-ordered freeze/poke/one-frame-scan choreography. **Recommend a focused
follow-up** rather than risk a wedge on a secondary observational close; the
trampoline verify (the session's primary deliverable) is complete.

---

## 5. Remaining t18

Trampoline done. Next: the 3→4→5 loop over the parallax hot procs (row 1058/1085
— Fill_PerLine perf charter, B1/B2/B3 transition-logic sizing at step-3(b)) →
**dry-panel debut** (A1+B1+C1+C2 floor, **C3 active** — the trampoline/IE1 work
was exactly C3 territory) → merge (PROVENANCE re-baseline to the new canonical).
