# t21 — TRANCHE CLOSE PACKET (buffers / vblank conversion)

**Third tranche under the corrected LEAN amendment.** Scope:
`engine/system/buffers.asm` → `buffers.emp` (FIRST), then
`engine/system/vblank.asm` → `vblank.emp` (the VSync_Wait ownership flip),
full loop `0 → 1 → 2 → (3→4→5)* → 6`.

Branch tips at close: **aeon-t21 `fda4921` / sigil-t21 `95d7d5c` (incl. this
packet; the aeon tip's final commit is the overseer's byte-neutral
whitespace fix on the `.hs_done` jbra, CRC-verified)**. Merged as **aeon
`0a17462` / sigil `258308a`** (`port-tranche21` --no-ff).
Branch ROMs at close: **plain `4745cbc3`/421157 · debug `0b7c4804`/429202**
(PROVENANCE re-baselined at merge; byte-delta table below).
Full paired strict at every byte-changing commit; final: **2553/0**
(baseline 2531 + 8 spelling probes + 3 buffers_port + 5 vblank_port +
4 flip tests + 2 mixed_tranche21 + step-6 additions; overseer-countersigned
from both branch tips AND merged masters).
Overseer P-O1 probe (merge-gate oracle session, 120-frame ambient OJZ avg):
VInt_Level inclusive **6836 cyc/f (5.3%)** (Process_DMA_Critical 4103 of it)
· Enqueue_Dirty_Buffers **766 (0.6%)** · VSync_Wait 100563 = parked idle
spin (headroom, not cost) · **VInt_Lag absent = zero lag frames** — P-O2's
condition never arose. Banked as budget-parcel input.

## Scoreboard

| Workstream | Outcome |
|---|---|
| **buffers.emp** (Init_SpriteTable/BuildStaticDMA/PlaneMapToVRAM/Enqueue_Dirty_Buffers) | byte-identical FIRST compile both shapes → modernized (−0xA lockstep relaxation wave) → DMAEntry adoption (t20 ride retired) + queue_static_dma (macro-port redesign: entry-only, Critical-bound) |
| **vblank.emp** (VBlank_Handler/VInt_Level/VInt_Lag/VSync_Wait) | byte-identical both shapes (2 lower-demanded spelling fixes) → contract-exact VSync_Wait flip (row 29 killed, 4 flip tests) → sound-off/mirror arms proven by twin-parity gates |
| **0 demanded features** | ALL SEVEN step-0 probes passed as-found (rte; dc.l/imm cross-frontend; Label-arg splice; link-expr shift/mask; bsr-tail fallthrough) — the first tranche since the probe rule shipped where the frontend needed nothing |
| **2 constructs built in-loop** | `sr_masked(code)` (engine.irq, paired-use-only; P9 Code-arg probe) + `out(d0, zero: inert)` on Parallax_Active_Config (flag-result grammar's second consumer) |
| **mixed acceptance** | NEW tranche21 arm — the campaign's FIRST `.asm` data-directive (`dc.l VBlank_Handler`) and immediate (`move.l #VInt_Level`) references to .emp procs, at full-ROM scale, both shapes |

## Step-0 (design note `2026-07-24-t21-step0-design.md`, committed before code)

Probes at the real binding class; ALL SEVEN passed as-found (P5 pass-as-
spelling-change: module-level decl-gating absent → ungated decl + gated
call, overseer-endorsed). Trip-check catches: t20 DMAEntry at-next-touch
ride TRIGGERED (retired at buffers s1); row 1052 second half TRIGGERED
(four derivation fns joined engine.vdp — row CLOSED); parallax_config
2nd-consumer move to engine.structs (row-1051 class); kill row 29 flip
planned with per-caller link tests.

## Step-1 gate lists (artifacts — all EXECUTED)

**buffers** (region plain $1F56 · debug $1FDC; len $262→$258 at step 2 —
shape-INVARIANT):
- byte gates both shapes: `buffers_port::buffers{,_debug}_region_matches_reference`
  — green FIRST compile, re-green after step 2.
- negative probe: `doctored_vram_sprite_table_fires_its_guard` ($B800→$D800
  fires NAMING the constant).
- new shared twins: parallax_config → engine.structs (13-field wall moved);
  VRAM_SPRITE_TABLE/VRAM_HSCROLL_TABLE/PLANE_H_CELLS → engine.constants
  (3 ensures); dma_source/dma_length/vdp_comm_delta/plane_loc → engine.vdp.
  Ripple: drift-wall counts 51→65 (×7 sites), constants count 66→69,
  PLANE_H_CELLS/parallax_config value-seam hoists (3 test files deduped).
- region pin `pins::BUFFERS` (repin); gate `SIGIL_EMP_BUFFERS` + per-shape
  engine.inc orgs; gate-off dual rebuild reproduced then-canonical CRCs.
- contracts: twin headers verbatim (Init d0-d2/a0 · Build d0-d3/d5/a0 ·
  PlaneMap typed params + d0-d4/a1/a5-a6 · Enqueue d0/a1-a2).
**vblank** (region plain $21AE/$130 · debug $2234/$138 — debug +8 = the two
`if DEBUG == 1` arms):
- byte gates both shapes: `vblank_port::vblank{,_debug}_region_matches_reference`.
- off-canonical arms (NO reference ROM exists for those shapes):
  `vblank_sound_off_twin_parity_{plain,debug}` + `vblank_mirror_shape_twin_parity`
  — full AS-side ROM at the same defines as oracle, region self-located by
  its own labels, cross-seam symbols fed from the same module's label table.
- **ownership-flip link tests (the headline obligation, both shapes each):**
  `game_loop_port::two_module_ownership_flip_{plain,debug}` +
  `load_art_port::two_module_ownership_flip_{plain,debug}`. BOTH extern
  decls deleted same-commit; kill row 29 KILLED with these artifacts named.
- contract: `VSync_Wait () clobbers(d0) preserves(sr)` — the pinned decl
  contract EXACTLY plus the lint-mandated, save/restore-enforced sr claim
  (sound_api ×4 convention; surfaced, overseer-endorsed).
  `VBlank_Handler () clobbers() preserves(d0-d7/a0-a6)` — the hblank
  CPU-STATE-ONLY convention, movem-enforced.
- mixed acceptance: `mixed_tranche21_{rom,debug_rom}_matches_assembled_reference`
  (vectors.asm's REAL `dc.l VBlank_Handler` + boot/ojz `move.l #VInt_Level`).
- region pin `pins::VBLANK`; gate `SIGIL_EMP_VBLANK`; gate-off CRCs exact.
- extern with drift guard + kill row: Sound_DebugMirror (row 42, ungated
  decl + triple-gated call).

## Byte-delta table (measured, not predicted)

| Change | Δ plain | Δ debug | Absorbed by |
|---|---|---|---|
| buffers step 1 (+ struct/const/vdp twins, gates) | 0 | 0 | — (byte-identical; gate-off dual rebuild exact) |
| buffers step-2 relaxations (5× `bsr.w .build_entry`→`.s`, first stays .w at 132-byte reach; `jsr`→`bsr.w` length-neutral; twin lockstep) | −0xA | −0xA | repin (BUFFERS len; every downstream base −0xA), engine.inc org table (24 blocks), repin_pins.rs (11 asserts + changelog row) |
| ROM totals | 421159 unchanged (tail padding absorbs); CRC f3e333d3→13dfdfc5 | 429190→429204 (−10 code + convsym appendix delta); CRC 20a1fe4b→dbba0fc9 | PROVENANCE re-baseline at merge |
| vblank steps 1-2 | 0 | 0 | — (canonical shapes unchanged; sound-OFF shape relaxes the VInt_Lag call to bsr.s — twin ifdef-width lockstep, t19 precedent) |
| loop pass 1 (comments, contracts, sr_masked adoption) | 0 | 0 | — (dual rebuild exact: 13dfdfc5/dbba0fc9) |
| panel adjudication + step 6 (`.hs_done` rename) | −2 (appendix) | −2 (appendix) | NOTHING — code bytes identical (repin --check clean, no pin/org/ASSEMBLED_LEN moved); CRCs 13dfdfc5→4745cbc3 / dbba0fc9→0b7c4804 |

## Step-2 filled checklist (per file — all seven items walked)

1. Branch conversions: buffers all-bare + jbra/jbsr, −0xA wave (twin
   lockstep); vblank all-bare + jbra/jbsr, ZERO canonical delta (all call
   targets beyond .s reach); the sound-OFF-only VInt_Lag relaxation rides
   twin ifdef widths. `jsr (a0)` stays (computed target, blessed).
2. Width pins with site comments: THREE (vblank's mem-to-mem two-symbolic
   pairs — the t15 pinned class, each site-commented). Buffers kept none.
3. Bare-symbol width-rule: complete — RAM bare (abs.w), VDP_DATA/VDP_CTRL
   bare (abs.l), SND_DMA_ACTIVE_SLOT bare over the extern-sum equ (abs.l —
   t19 row-1004 spelling).
4. Brace-indent: file-wide, both files + engine.irq.
5. Idiom list walked: DMAEntry.field displacements (movep-aware width from
   t20) ✓; parallax_config.field via the moved shared struct ✓;
   label-in-immediate n/a (SRC_* are equ-immediates — the link-time
   dma_source arm) ✓; typed VDP fns (vdp_comm ×7 folds + the four new
   derivation fns) ✓; contract reglists RANGE form throughout ✓; no
   operand-override widths beyond item-2's three ✓.
6. Type-layer walk: ADOPTED `jsr (a0) as VBlankHandler` (honest-⊤ type) +
   PlaneMapToVRAM typed params `(a1: *u8, d0: u32, d1: u16, d2: u16)`.
   LOGGED with reasons (ledger): VramAddr-class d2/d0 command folds (A4-i);
   PlaneMapToVRAM dbf counters (shift/add class); VInt_Ptr typed RAM cell
   (no typed extern-data decl grammar; stores are .asm-side — documentary).
7. Noticing: ONE proposed addition — the **off-canonical twin-parity gate**
   (full AS-side assembly at the same defines as the oracle for comptime
   arms with no reference ROM; region self-located by its own labels).
   Proposed as the standing gate-artifact class for shaped files.

## PER-PASS: step-3 vs step-5

**Pass — steps 0-2 (per file):**
- *step-3 flavored:* zero demanded features (all seven probes as-found —
  the frontend/link surface was already complete for both NEW mixed-build
  binding classes); the P5 spelling ruling (ungated decl + gated call);
  the two lower-demanded vblank spellings (mem-to-mem pins; preserves
  contract spellings) — documentation-outran-proof pulled UP to proof.
- *step-5:* the −0xA relaxation wave (size win).

**Pass 1 — 3(a) (all interrogation lines run, outcomes):** see
`2026-07-24-t21-loop-pass1.md` — BuildStaticDMA ceremony DEFERRED (size
bar, ledgered); comment-compensation + escape census → demand-increment
row; twin-parity-gate noticing proposal.

**Pass 1 — 3(b) (all lines run):** THREE real fixes shipped zero-byte:
the STALE "Z80 already stopped" claim (both twins — the MegaPCM-2 flag
bracket leaves the Z80 free); ram.asm's WRONG Cache_Pfx_Lag_Flag writer
claim (store is tile_cache's Frame_Counter-delta gate — overseer R3 + row
1067 confirmed); the twin's `b96c861` codename scrubbed. The "~34-40 cy"
figure FLAGGED to C1. Boot-order, torn-drain mechanism, −42-byte arm all
VERIFIED.

**Pass 1 — step-4 (all adjudications named):**
- **BUILT `sr_masked(code)`** (engine.irq, paired-use-only fence with named
  exclusions; P9 probe; VSync_Wait adopted in-tranche; kill row 43;
  t19/t20 SR-bracket row → RESOLVED-PARTIAL).
- **BUILT `out(d0, zero: inert)`** on Parallax_Active_Config (R5; `zero` in
  VALID_FLAGS; out_verify credits both paths; d0 moved clobbers→out per
  `[proc.out-clobbers-overlap]`; both consumers verified flag-consuming).
- **DEFERRED** BuildStaticDMA template (size bar).
- **LOG** VInt_Ptr typed cell (documentary; demand row).
- **ENUMERATED** queue_static_dma back-prop: zero other sites either side.
- **KEEP** PlaneMapToVRAM (verb-(d), overseer R2; comments + ledger row;
  Volence-override flag open).

**Pass 1 — step-5 (FULL interrogation, per hot proc; heat: VInt_Level
every normal frame · VInt_Lag lag frames · Enqueue per-frame both paths ·
VSync_Wait per-frame spin · Init/Build/PlaneMap cold):** full per-line
outcomes in `2026-07-24-t21-loop-pass1.md`. Threshold ruling: **NO CUT —
nothing within reach of ≥1k cyc/f** (largest: Enqueue slot-cursor threading
≈50-100 cy/f worst AND breaks the template's per-site carry protocol —
LOG-not-cut). Named probes P-O1/P-O2 are overseer-run, informational only.

**Pass 2: EMPTY at all three steps** (pass-1 output was comments/contract
metadata/constructs; the fresh re-walk surfaced nothing) → dry claim →
panel dispatched (below).

## PANEL ROUND (A1+B1+C1+C2+C3 — C3 mandatory; read-only; one round)

**DRY STOOD** (t18/t19/t20 precedent: adjudication yielded comments, one
comptime ensure, one comptime-only composition, a label rename, ledger rows,
and step-6-class consolidations — no algorithmic, construct, or optimization
re-work). Every finding adjudicated:

- *A1 (cold reader, 10):* FIVE real. (1) the 4-site sound flag-bracket
  ceremony — a genuine pass-1 3(a) blind spot → LEDGERED as the
  `dma_window_open/close` / `z80_held(code)` construct ask (merged with
  B1-F4; demand 6; hangs on the unprobed comptime-if-inside-Code-fn
  question; building at adjudication would re-open the loop — t20
  jump_table precedent). (2) palette-ladder = second block-ceremony site →
  demand JOINED to the BuildStaticDMA ledger row (2 sites / 11 blocks).
  (3) queue_static_dma had NO declared register surface → prose Clobbers
  line + IRQ-containment sentence SHIPPED; Code-fn contract-annotation ask
  LEDGERED. (4) sr_masked's label-free rule unenforced → validation ask
  LEDGERED (sole consumer clean). (5) width-annotation what-comment family
  (×8, third and largest comment-compensation family) → LEDGERED.
  Observations shipped zero-byte: `ensure(sizeof(DMAEntry) == 14)` beside
  the template (the t20 A1 stride-ensure precedent); DEBUG/__DEBUG__
  vocabulary fix; VBlank_Handler contract-comment inversion reworded;
  `.no_hscroll`→`.hs_done` rename ACCEPTED (the label named a condition
  that never holds on its main edge; twin lockstep; the tranche's only
  post-step-2 CRC delta — appendix-only). Kept as-is: the clobber-union
  provenance comments (C2 verified them correct).
- *B1 (corpus, 7):* ONE real + ledger-corrections. F1: the step-0
  trip-check claim "no .emp mirrors" was FALSE for 2 of 3 consts (the check
  only looked at constants.emp) — PLANE_H_CELLS ×2 + VRAM_SPRITE_TABLE ×1
  file-local mirrors existed → RETIRED to the shared twin at step 6
  (byte-neutral; design note corrected). F2 VDP_CTRL_OFF ×2: the written
  derives-stay-local rule honored; the rule-conflict with the 2nd-consumer
  practice FLAGGED for Volence. F3: vdp_comm now COMPOSES vdp_comm_delta
  (shipped, comptime-only). F4 → merged into the A1-1 ask. F6: irq.emp
  header now NAMES all exclusions + the structural CCR-out rule (a bracket
  with a carry out-contract can never adopt — the restore overwrites CCR).
  F7: consumers line added. SR census independently verified: 2 adoptable
  (as claimed), 5 exclusions justified, no unclaimed site.
- *C1 (perf): ENDORSE-WITH-CORRECTIONS; the no-cut ruling STANDS.* The
  named input resolved: the mask bracket is EXACTLY 46 cy (14+16+16) — the
  twin comment's "~34-40" was wrong, fixed BOTH twins. Slot-threading log
  magnitudes corrected on the ledger (~24 cy/subsequent enqueue, ~120 cy/f
  worst, max SIX live enqueues — the two HScroll entries are mutually
  exclusive). NEW log-grade find: `andi.b #$FE, ccr` in queue_static_dma is
  provably redundant (the slot-var MOVE clears C; 20 cy + 4 B ×7
  expansions) → KEPT as the explicit contract spelling, comment upgraded
  both twins, lockstep cut ledgered for a future scavenge. "−42 bytes" twin
  claim VERIFIED; shell costs re-derived (~400/460/400 cy/f), movem-narrow
  correctly not taken (80 cy for a broken transparency contract).
- *C2 (correctness): ZERO real bugs in the port.* Every gate-blind line
  re-derived clean: the movep interleave incl. the load-bearing
  source-before-length overwrite order (verified at all five corpus
  sites); Init's 640-byte arithmetic; every CC-test/branch adjacency; the
  splice-vs-core protocol identity + the SR-mask asymmetry ruled SAFE BY
  CONTAINMENT (non-pub, IRQ-only consumer — sentence shipped into the fn
  header); loop exits; sr_masked bracket placement; movem symmetry; the
  out(zero:) reliance chain; all four comptime shapes vs the twin's nests.
  ONE pre-existing latent hazard LEDGERED: the per-cell HScroll packed
  112-byte table vs the VDP's 32-byte cell-mode fetch stride — an
  UN-HYPOTHESIZED mechanism for the documented DEFERRED_WORK "lines 28-223
  pinned" symptom (mode CLOSED, shipped content safe via DeformTable_Zero;
  content-authoring trap noted). Observations shipped: PlaneMapToVRAM's
  carry-blind row-advance precondition line; VSync_Wait's IPL<6
  precondition line.
- *C3 (hardware): all three NAMED INPUTS verified clear* — bracket order
  exact in both handlers (raise before ANY VDP work, close after the last
  DMA, no VDP work reachable after the close), budget reload has no stale
  window, lag path touches nothing plane-buffer-owned. ONE real finding:
  the "dirty palette/sprite/HScroll buffers ARE dirty-FLAG gated" claim
  was FALSE for HScroll (unconditional every-frame enqueue; only the MODE
  is conditional) — the real safety mechanism is static-dest
  value-tear-only (one-frame scroll shear, self-healing, never a garbage
  VDP destination) → comment rewritten BOTH twins; value-tear class
  LEDGERED. ONE ledger-candidate: the palette/SAT producer invariant
  "flag set after complete write" is necessary-not-sufficient (set-flag-
  LAST, no post-flag rewrite within a tick, is the sufficient form) —
  producer-audit rider LEDGERED. Flag-raise latency, IRQ nesting,
  flip-flop discipline, Z80 bus brackets: all-clear with mechanisms.

## Step-6 corpus sweep (enumeration, per-site outcomes — EXECUTED)

1. **sr_masked** → Sound_PostByte + Sound_PlayMusic RETROFITTED (nested
   `{stop_z80()}`/`{start_z80()}` splices inside the Code argument compose
   clean; byte-neutral, sound_api gates green; test plumbing: irq ambient
   added to sound_api_port, tranche5_negative_probes, mixed_dac_rom's
   placed-module hook). The 5 excluded sites stay inline per the module
   header. Kill row 43's consumer list now matches reality.
2. **out(zero:) flag-result class** — census of Z-out prose:
   core.emp AllocDynamic/AllocEffect NOT-AN-INSTANCE (the dependency is
   already contract-encoded via `out(a1 if eq)` — double-encoding refused);
   **Section_GetSecPtrXY RETROFITTED** `out(d0, a0, zero: none)` (all 3
   .emp callers beq immediately — section, parallax, entity_window);
   **Load_Object RETROFITTED** `out(a1, zero: success)` — entity_window's
   bne consumes; Load_ObjectList's deliberate per-entry ignore now carries
   the corpus's FIRST `@discards(success)` (the must-use escape, now
   exercised). out_verify + corpus closure green.
3. **dma_source/dma_length/vdp_comm_delta/plane_loc** → zero other users
   either side — NOT-AN-INSTANCE (B1 census concurs; dma_queue's runtime
   lsr/bclr is the register-domain analogue, not an instance).
4. **queue_static_dma** → zero sites (step-4 enumeration; B1 concurs).
5. **Twin-parity gate class** → process addition, no retrofit sites.
6. **Const consolidation (panel B1-F1 fold-in)**: PLANE_H_CELLS retired
   from plane_buffer.emp + section.emp, VRAM_SPRITE_TABLE from bg.emp —
   all three now `use engine.constants`; bg_port gained the constants
   ambient + full truth blob (doctor moved after the extends so the
   negative probe doctors shared pairs).

## NEITHER-BUCKET HEADLINES

- **Zero demanded features:** every probe passed as-found — the first
  tranche since the probe rule shipped where BOTH new mixed-build binding
  classes (`rte` procs; `.asm` data/immediate refs to .emp symbols) already
  worked. The mixed-build ladder's remaining gap set shrank by observation,
  not construction.
- **The VSync_Wait flip landed exactly as briefed:** two decls deleted
  same-commit, four flip tests green, contract carried verbatim plus the
  lint-mandated `preserves(sr)` (surfaced, endorsed) — and the
  buffers-first order paid its dividend (vblank calls Enqueue
  module-to-module with zero extern churn).
- **The off-canonical twin-parity gate** (sound-off + mirror arms proven
  against a full AS-side assembly at the same defines, self-locating the
  region by its own labels) — named as a reusable artifact class for
  comptime-shaped files whose arms ship no reference ROM.
- **queue_static_dma is the macro-port rule working as intended:** the
  donor's 3-symbol interface allowed a slot/end pair mismatch; the .emp
  interface is entry-only with the pair unrepresentable — and the P4 probe
  banked two probe-construction lessons (in-module AS reference; the
  sign-extension window) without any language change.
- **The panel round earned its keep again** (7/7-style): C3 caught a false
  safety claim (HScroll flag-gating) the porter AND pass-1's own
  comment-claim audit walked past twice; C1 settled the flagged cycle
  figure at exactly 46 cy; B1 caught the step-0 trip-check's
  single-location blind spot; A1's ceremony scan found the 4-site
  flag-bracket the in-tranche scan missed; C2 re-derived the whole
  gate-blind checklist clean — zero real bugs in the port itself.
- **Process:** repin corrected the porting agent's hand-derived vblank
  region end (HBlank_Install, not the hblank resume org) — the tool is the
  authority; the note records the correction.
