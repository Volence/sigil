# TRANCHE 21 BRIEF — buffers / vblank conversion (LEAN tranche)

**Dispatch: overseer-cut 2026-07-24 (Volence present; t19+t20 merged and
swept — no coordination partner). Single-lane.** Third tranche under the
corrected LEAN amendment (in-tranche step 5 FULL; standalone parcels on
other files defer post-conversion).

**Canonical sources (read before cutting code):**
`docs/superpowers/notes/campaign-port-loop.md` (re-read at EVERY step
boundary), the t20 close packet (`notes/2026-07-24-t20-close-packet.md` —
freshest process template, incl. the demanded-feature TDD pattern and the
ownership-flip link-test shape), the campaign gap-ledger + kill-list for
the step-0 sweep.

## Scope (port ORDER MATTERS)

1. **`engine/system/buffers.asm`** (183 L; `Init_SpriteTable`,
   `BuildStaticDMA`, `PlaneMapToVRAM`, `Enqueue_Dirty_Buffers`)
   → `buffers.emp` — FIRST.
2. **`engine/system/vblank.asm`** (194 L; `VBlank_Handler`, `VInt_Level`,
   `VInt_Lag`, `VSync_Wait`) → `vblank.emp` — SECOND (VInt_Level/VInt_Lag
   call `Enqueue_Dirty_Buffers`, which becomes .emp-owned in file 1;
   porting in this order makes vblank a normal .emp→.emp caller with NO
   buffers extern churn — the t20 dma_queue-first logic).

After both files, vblank.emp's ONLY remaining extern is the shape-gated
`Sound_DebugMirror` (engine/debug/sound_debug.asm; `__DEBUG__` ×
`SOUND_DRIVER_ENABLED` × `SOUND_DBG_MIRROR` nest — decl gated to that
shape, drift-guard comment, kill row added). Every other callee is
.emp-owned already: Process_DMA_* (t20), stopZ80/startZ80 via z80_bus.emp
(t19), Flush_VDP_Shadow (vdp_init.emp), Vscroll_Write (parallax.emp),
VInt_DrawLevel (plane_buffer.emp), Read_Controllers (controllers.emp).

Full loop `0 → 1 → 2 → (3 → 4 → 5)* → 6 → merge`; steps 0/1/2 per file;
one dry-panel; step 6 once; one close packet.

## Mechanics (standing bars — t20 values)

- Branch `port-tranche21` both repos, worktrees aeon-t21/sigil-t21; **seed
  aeon worktree's `games/sonic4/data/editor/` by rsync from main and
  verify canonical CRCs before any code**.
- **Canonical: plain `f3e333d3`/421159 · debug `20a1fe4b`/429190** (masters
  aeon `236f959` / sigil `4c75adf`). Strict baseline **2531/0** paired,
  AEON_DIR at the branch tree. One shape per build invocation.
- cwd resets every Bash call — cd explicitly; explicit-path commits only;
  never push; failures-first test output; STOP with named probe lists for
  anything needing the emulator (oracle MCP is overseer-only — subagent
  calls deadlock).

## THE HEADLINE OBLIGATIONS

### 1. The VSync_Wait ownership flip (kill row 29 DIES)

`vblank.asm` porting flips `VSync_Wait` ownership under TWO existing .emp
callers. Per the proof-mechanism feed-forward rule, one persisted
two-module link test PER FLIPPED CALLER:

- **game_loop.emp** (extern decl at :12, `clobbers(d0)`, jbsr at :29) —
  link test + extern decl DELETED same-commit.
- **load_art.emp** (extern decl at :25, two call sites :122/:144 — the
  register-discipline block :81-92 depends on VSync_Wait touching neither
  a6 nor the staging data) — link test + decl deleted same-commit.
- Kill-list row 29 explicitly names this port as its kill condition —
  update the row; the §11 Q4 collision check fires if decl+proc coexist.
- The .emp proc signature carries the SAME contract the decls pinned
  (`() clobbers(d0)`) — VSync_Wait also clobbers CCR per load_art's
  comments; contract-grammar treats CCR per house rules. Widening is a
  step-3 finding to surface, never a silent change.

### 2. Two NEW mixed-build binding classes — step-0 probes REQUIRED

Neither exists anywhere in the tree yet; each is a DEMANDED feature
(TDD-first, movep/DispSymInd precedent) if the probe fails:

- **`rte` in the .emp frontend** — `VBlank_Handler` terminates with rte.
  No .emp file emits rte today (hblank.emp only stores $4E73 as DATA into
  the RAM slot). The ISA assembles rte from .asm; the question is the .emp
  surface spelling. Probe at the real class (proc ending in rte, no
  implicit epilogue interference).
- **`dc.l <emp-proc>` from .asm** — `vectors.asm:38` takes
  `dc.l VBlank_Handler` (the IRQ6 vector). No .asm data directive
  references an .emp-exported symbol anywhere yet. Same class:
  `boot.asm:158` + `games/sonic4/test/ojz_scroll_test.asm:140`
  `move.l #VInt_Level, (VInt_Ptr).w` (.asm immediate ref to .emp symbol).
  Probe both spellings (data directive + immediate operand).

## Step-0 hazard pre-sweep (overseer findings — verify and complete;
const-keyed trip-check still owed)

**buffers.asm**
- **DMAEntry struct adoption**: t20's ledger ride "buffers.asm DMAEntry
  adoption at-next-touch" TRIGGERS now — `.build_entry` writes
  `DMAEntry_Reg94/Reg93/SizeL/SizeH/Command` via `movep.l`/`movep.w`
  (t20's movep-aware width checking is live; the struct twin + offsetof
  wall already exist). Adopt the .emp struct; twin keeps its spelling.
- **`queueStaticDMA` macro** (engine/macros.asm:292, 7 call sites) — the
  macro-port rule: design the .emp shape (template/comptime-fn class,
  clamp_camera_axis precedent) in the step-0 note BEFORE code. It carries
  the CCR-surgery carry contract (`andi.b #$FE, ccr` / `ori.b #1, ccr`,
  frontend support confirmed in t20) mirroring QueueDMA_Critical: carry
  SET = dropped. The dirty-bit retry protocol (clear ONLY on carry-clear,
  leave set to retry next VBlank) is a load-bearing contract — preserve
  the honest comments present-tense.
- **`.build_entry` bsr-then-fallthrough**: six `bsr.w .build_entry` then
  the SEVENTH entry FALLS THROUGH into it (line 83-88). Name the .emp
  spelling for call-then-fallthrough-into-local-tail in the step-0 note
  (structural shape; twin lockstep is byte-level).
- **VDP macro adoption**: `dmaSource`/`dmaLength`/`vdpComm`/
  `vdpCommDelta`/`planeLoc` — .emp counterparts shipped t15/t19/t20
  (vdp_comm_reg, engine.vdp VDP_DATA/VDP_CTRL, DMA consts). Ledger row
  1052 (corrected) predicted the shared-home adoption lands with this
  file class — verify which are already shared vs still needed.
- `Parallax_Active_Config` — parallax.emp-owned, normal import; its
  "Z reflects d0" out-flag contract is relied on at :172-173 (beq after
  jsr). Verify the .emp contract spells that; if it doesn't, that's a
  step-3 finding on parallax, not a silent local assumption.
- The `jsr Parallax_Active_Config` is a plain static call — jbsr per the
  idiom list (jsr is for computed targets only).
- HScroll mode-length tear comment (:166-171) — present-tense contract
  fact, keep.
- RAM/consts: Sprite_Table_Buffer, Static_Pal_Line0-3, Static_Sprite_DMA,
  Static_Hscroll_Cell/Line, Palette_Dirty, Sprite_Table_Dirty,
  VRAM_SPRITE_TABLE, VRAM_HSCROLL_TABLE, PLANE_H_CELLS — shape-aware.
- Consumers needing nothing special: boot.asm bsr.w
  Init_SpriteTable/BuildStaticDMA + game-side jsr Init_SpriteTable
  (demo_state.asm, object_test_state.asm) — .asm→.emp exercised class.

**vblank.asm**
- **IRQ6 entry**: full `movem.l d0-a6, -(sp)` save + `rte`. Contract
  style: hblank.emp's HBlankHandler convention ("CPU-STATE ONLY:
  clobbers() covers the save/restore + rte") is the precedent — mirror
  its prose. `VInt_Ptr` is a RAM function pointer dispatched via
  `movea.l` + `jsr (a0)` — computed target, jsr STAYS. Type layer: a
  VBlankHandler-class bless (HBlankHandler analog) is the candidate —
  adjudicate build-vs-log at step 4 (A4-i gate rules apply; VInt_Ptr
  stores happen in .asm boot/test code, so a typed pointer may be
  documentary — demand data decides).
- **Comptime shapes**: SOUND_DRIVER_ENABLED defaults ON (build.sh :50) —
  BOTH canonical shapes carry the flag-bracket arms; the byte gate binds
  them. The `ifndef SOUND_DRIVER_ENABLED` full-fence arms and the
  `SOUND_DBG_MIRROR` nest are OFF in both canonical shapes → t19
  comptime-select guard-fn idiom + compile/gate-off probes per arm
  (sound-off CRC pair as the gate-off artifact; the mirror arm at least
  compiles + links with the gated Sound_DebugMirror extern).
- **The SR-mask bracket** (VSync_Wait :182-186:
  `move.w sr,-(sp)` / `#$2700` / restore around the flag+Ready pair) —
  the t20 step-4 census called dma_queue the strongest pair-hazard
  instance and did NOT build the construct; this file ADDS the
  cleanest-shaped site. Re-adjudicate at step 4 with the new demand
  count; demand data either way.
- **Atomicity/ordering contracts — preserve present-tense, verbatim
  intent**: the VSync_Wait stale-flag clear + atomic pair comment (the
  b96c861 torn-drain hazard), VInt_Lag's do-NOT-drain-Plane_Buffer block
  (:126-136 — Ptr-gated vs dirty-flag-gated distinction), the consume-once
  controller latch race note (:91-94), the §4.6 VSRAM-after-HScroll
  ordering, and the whole SND_CTRL_DMA_ACTIVE flag-bracket model
  (raise-before-ANY-VDP-work / clear-after-last-DMA — bracket ORDER is
  load-bearing). These comments are contract, not narration.
- `SND_Z80_BASE+SND_CTRL_DMA_ACTIVE` `.l` absolute writes inside
  stopZ80/startZ80 brackets — z80_bus.emp import; verify the extern-sum
  abs operand class (t19 row-1004: inline extern-sum abs can't ride the
  bare width rule — spell accordingly).
- Debug sites shape-aware: `Lag_Frame_Count` (+`Cache_Pfx_Lag_Flag`
  interplay is tile_cache-side, untouched), `DMA_Bytes_ThisFrame` clear
  in VSync_Wait, the Sound_DebugMirror nest.
- `DMA_Budget_Default → DMA_Budget_Remaining` reload (:69) — dma_queue.emp
  consts/symbols from t20; normal import.
- **Behavior fences**: the t20 C3 marquee (VBlank budget model uncoupled
  from the physical ~38-line window) is LEDGERED as post-conversion
  budget-parcel input — do NOT fix in this port; carry honest comments.
  Same for anything the torn-drain/lag design implies: port faithfully.

## Step-5 / panel notes

- vblank IS the VBlank window: **C3 MANDATORY** (A1+B1+C1+C2+C3).
- Hot paths: VInt_Level runs every normal frame (the whole per-frame VDP
  pipeline), VInt_Lag on lag frames, Enqueue_Dirty_Buffers per-frame from
  both. VSync_Wait spins per-frame but is trivially cheap; Init/Build/
  PlaneMapToVRAM are boot/init-cold. Profile-first via overseer probes
  where live numbers matter (named probe list to the overseer — oracle is
  overseer-only); the ≥~1k cyc/f threshold stands. Expect measured no-cut
  on the handler shells (they are bsr sequences); the interrogation lines
  still run per proc.
- Step-6 candidates to watch: the queueStaticDMA .emp shape may
  back-propagate; the SR-bracket adjudication; the rte/vector binding
  classes feed the mixed-build ladder ledger row.

## Acceptance

Per-file step-1 gate lists with named artifacts (byte gates both shapes,
region pins, mixed-build acceptance incl. the two NEW binding classes,
negative probes, gate-off/sound-off CRCs, the TWO VSync_Wait flip link
tests); full paired strict green from the branch tree at every
byte-changing commit; dry = full 3→4→5 circuit empty then a clean panel
round (C3 active); step-6 enumeration; close packet per house format
(per-pass step-3/step-5 breakdown + neither-bucket headlines);
ledger/kill rows same-commit (row 29 killed; Sound_DebugMirror row born;
DMAEntry at-next-touch ride retired). STOP at the merge gate — the
overseer countersigns (fresh strict, dual rebuild, hot-path second look
on vblank.emp + Enqueue_Dirty_Buffers) and runs the merge sequence +
PROVENANCE re-baseline. Checkpoint discipline (a)/(b)/(c): STOP after
steps 0-2 (checkpoint (a)) and report branch tips, CRCs, strict count,
demanded-feature outcomes, and open questions before entering the loop.
