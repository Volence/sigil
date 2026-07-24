# t20 — TRANCHE CLOSE PACKET (dma_queue / load_art conversion)

**Second tranche under the corrected LEAN amendment (in-tranche step 5 FULL;
standalone parcels defer post-conversion).** Scope:
`engine/system/dma_queue.asm` → `dma_queue.emp` (FIRST — the QueueDMA
ownership flip), then `engine/level/load_art.asm` → `load_art.emp`, full loop
`0 → 1 → 2 → (3→4→5)* → 6`.

Branch tips at close: **aeon-t20 `2d31c49` / sigil-t20 `<this commit>` (+ this packet)**.
Branch ROMs at close: **plain `f3e333d3`/421159 · debug `20a1fe4b`/429190**
(PROVENANCE re-baseline at merge; byte-delta table below).
Full paired strict at every byte-changing commit; final: **2531/0**
(baseline 2509 + 6 spelling probes + 3 disp_sym_ind + 4 dma_queue_port +
3 load_art_port + 2 dplc flips + 2 bg_anim flips + 2 mixed_tranche20).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **dma_queue.emp** (Init/QueueDMA_×3+transfer/Process_×3/Drain_Budgeted) | byte-identical FIRST compile both shapes → modernized (0-byte) → contract-exact ownership flip (rows 31+32 killed, 4 flip tests) → full step-5 interrogation (no cut ≥1k) |
| **load_art.emp** (Art_Decompress/Level_LoadArt) | byte-identical FIRST compile both shapes (raise_error blob matched the twin macro tower) → modernized (−4 lockstep relaxation wave) → PROVEN preserves(d6/a4-a6) contract |
| **2 demanded features** (TDD-first, divs.w shape) | `CodeOperand::DispSymInd` (label-as-d16 EA, `jmp .jump_table(a1)`) + movep-aware field-overrun width |
| **closure** | `resolve_callee_key` — `Owner.label` tail targets resolve to the owner proc's contract (corpus-error-gate-demanded) |
| **sigil-link** | width-4 value cells take the signed∪unsigned union window (mixed-arm catch; overseer-endorsed; ledgered) |
| **mixed acceptance** | NEW independent tranche20 arm — full-ROM splice of BOTH regions matches the shipped ROMs, both shapes |

## Step-0 (design note `2026-07-24-t20-step0-design.md`, committed before code)

Probes at the real binding class; outcomes: P2/P3/P6 passed as-found
(trap filler, CCR-under-restored-SR, fold pre-fill incl. slot-0 `(a0)`
collapse); P1/P4/P5 FAILED as-found → the two demanded features (below);
P7 settled the transfer-core shape (export-label tail inside
QueueDMA_Deferrable — explicit fallthrough — chosen by the tie rule).
Trip-check catches beyond the brief: kill row 31 dies alongside 32; ledger
row 1052's shared-VDP-home kill was ALREADY EXECUTED (engine.vdp exists —
dma_queue is a plain vdp_comm_reg consumer, whose fn header had anticipated
the three call sites).

## Step-1 gate lists (artifacts — all EXECUTED)

**dma_queue** (region plain $1C54/$302 · debug $1CD6/$306 — shape-DEPENDENT,
+4 debug = the `if DEBUG == 1` overflow bump in `.full`):
- byte gates both shapes: `dma_queue_port::dma_queue_region_matches_reference`
  + `::dma_queue_debug_region_matches_reference` — green FIRST compile, re-green
  after every later step.
- negative probes: `doctored_dma_critical_slots_fires_its_guard` (8→7) +
  `doctored_dmaentry_field_fires_its_guard` (SizeH offset 1→2 — the
  swapped-adjacent-field class the per-field wall exists for). Both fire
  NAMING the symbol.
- **ownership-flip link tests (the headline obligation, both shapes each):**
  `dplc_port::two_module_ownership_flip_{plain,debug}` (Important AND
  Deferrable flip together) + `bg_anim_port::two_module_ownership_flip_{plain,debug}`.
  All three extern decls deleted same-commit; kill rows 31/32 KILLED with
  these artifacts named.
- contract: `(d1: u32, d2: u16, d3: u16) clobbers(d0-d4/a1-a2)
  out(carry: dropped)` — the pinned extern contract exactly; NO widening.
- region pin `pins::DMA_QUEUE` (repin-derived); gate `SIGIL_EMP_DMA_QUEUE` +
  per-shape engine.inc orgs; gate-off dual rebuild reproduced the
  then-canonical CRCs exactly (eab19b3f/f1c1aa12).
- mixed acceptance: `mixed_dac_rom::mixed_tranche20_{rom,debug_rom}_matches_assembled_reference`.
- new shared twins: `DMAEntry` in engine.structs (sizeof + 11-field offsetof
  wall) + DMA/ART consts in engine.constants (9 ensures). Ripple: hardcoded
  assert counts updated (act_descriptor 39→51 ×7 sites, constants count 57→66).

**load_art** (region plain $60A8 · debug $6D32; $68→$64 / $B2→$AE at step 2 —
shape-DEPENDENT, debug surplus = the `.drop_page` raise_error expansion):
- byte gates both shapes: `load_art_port::load_art_region_matches_reference` +
  debug — green FIRST compile (the `raise_error` expansion byte-matched the
  twin's RaiseError macro blob — row-21 twin-parity held again).
- negative probe: `doctored_art_ver_zx0_fires_its_guard` (2→3, the
  version-dispatch compare's value).
- contract: `clobbers(d0-d5/d7/a0-a3) preserves(d6/a4-a6)` — the movem
  round-trip PROVES what the twin header only narrated; the closure
  error-gate FORCED the declaration (fired on a4/a5/a6 until declared).
- region pin `pins::LOAD_ART`; gate `SIGIL_EMP_LOAD_ART`; gate-off CRCs exact.
- externs with drift guards + kill rows: S4LZ_Decompress (row 38),
  ZX0_Decompress (row 39), VSync_Wait (row 29 second site).
  QueueDMA_Critical + BG_Init resolve module-to-module (no decls — the
  port-order dividend the brief planned).

## Byte-delta table (measured, not predicted)

| Change | Δ plain | Δ debug | Absorbed by |
|---|---|---|---|
| dma_queue steps 1-2 | 0 | 0 | — (byte-identical; step 2 converted 10 conditionals + 4 tails with zero delta — every twin width was already minimal) |
| load_art step-2 relaxations (`bsr.w Art_Decompress`→`.s` in-region backward; `bsr.w BG_Init`→`.s` next-placement; twin lockstep) | −0x4 | −0x4 | repin (LOAD_ART len; BG/BG_ANIM/SOUND_API bases; SOUND_* ×3 + BG_INIT pins), engine.inc orgs ×4 gate blocks, repin_pins changelog row |
| debug ROM total | — | −14 | −4 code + convsym appendix delta (symbol-digit shift; measured) |
| twin codename scrub + contract-header fix + marker comment | 0 | 0 | none (CRCs unchanged, verified by dual rebuild) |

## Step-2 filled checklist (per file — all seven items walked)

1. Branch conversions: dma_queue all-bare + jbra ×4 (2 entry tails →
   `.transfer`, 2 Process tails → Drain_Budgeted_Queue), ZERO byte delta;
   load_art all-bare + jbra/jbsr, −4 wave (deltas above, twin shrunk in
   lockstep). `jmp .jump_table(a1)` stays `jmp` (computed target).
   Decompressor tails ride `jbra` (engine-internal fixed placement — the t19
   checklist-1 ruling; the game_loop cross-section exception does not apply).
2. Width pins with site comments: TWO structural classes, both commented at
   site — the `jt_slot` `bra.w` (stride-locked slot: lea.l 6 + lea.w 4 +
   bra.w 4 = sizeof(DMAEntry)) and slot-0's `bra.w .done` (4-byte slot head).
   Nothing else kept a width.
3. Bare-symbol width-rule: complete — all RAM refs bare (abs.w), VDP_CTRL
   bare (abs.l), `Art_Staging_Buffer` bare (abs.l — $FFFF0000 is OUTSIDE the
   abs.w window; the twin's `.l` confirmed, spelling stays bare). The FOUR
   `(DMA_*_Slot).w` pinned dests are the row-1046 exception (link-imm source
   + bare dest = [lower.imm-link]), each site-commented; demand +4 ledgered.
4. Brace-indent: file-wide, both files.
5. Idiom list walked: `DMAEntry.SizeH(a1)`-class struct-field displacements
   (the Sst.field idiom, movep-compatible after the width fix) +
   `Act.act_art_pool_pages(a4)`; file-local derived offsets (ENTRY2_*) per
   the structs.emp boundary rule; label-in-immediate (`#DMA_Critical`,
   `#Art_Staging_Buffer`); typed VDP `vdp_comm_reg` ×3 (clr defaulting
   matches the twin's 1/0 args); contract reglists in RANGE form throughout;
   `(Sym).w` operand-override ONLY at the four row-1046 sites.
6. Type-layer walk: ADOPTED `(a0: *Act)` (Level_LoadArt); typed proc params
   `(d1: u32, d2: u16, d3: u16)` on the QueueDMA trio (grammar requires
   types; widths = the twin's documented In sizes). LOGGED-not-typed with
   reasons (one ledger batch row): slot-cursor/queue-end u16 word-address
   class (cmpa-compared but movea/suba arithmetic), d2 VRAM dest +
   page-stride add (VramAddr-in-arithmetic, A4-i), page count/table cursor,
   wrapper version byte (closed ART_VER_* vocabulary but runtime ROM data,
   below the F1 register-slot bar).
7. Noticing: ONE proposed addition — **the named-drain-label jump table** as
   the house spelling for stride-locked tables (named entry points over the
   twin's `bra.w .end-.c*8` arithmetic; byte-identical, proven by probe P5;
   cold reader sees which entry drains N). Proposed for the step-2 checklist.

## PER-PASS: step-3 vs step-5

**Pass — steps 0-2 (per file):**
- *step-3 flavored:* two demanded features (DispSymInd; movep overrun width)
  shipped TDD-first with byte-parity probes; the closure exported-label-tail
  resolution; the width-4 union window (mixed-arm catch — the standalone
  carrier had masked the signed fold; the REAL ram.asm chain exposed it);
  load_art's preserves() forced-and-proven by the closure gate.
- *step-5:* the −4 relaxation wave (size win, not cycle win).

**Pass 1 — 3(a) (all interrogation lines run, outcomes):**
- *Ceremony scan:* the jump table = ~20 lines for an 8-slot dispatch →
  `jump_table` construct ASK ledgered with demand 1 (under the build bar;
  future tables accumulate).
- *Comment-as-compensation:* the row-1046 pinned-width what-comments ×4 are
  compensation for [lower.imm-link] → demand increment ledgered.
- *Escape-hatch census:* extern decls ×3 (boundary class, kill rows 29/38/39);
  `(Sym).w` overrides ×4 (row-1046); zero call-expr escapes; drift-lock
  ensures are the standard twin tax.
- *Domain-type scan:* the item-13 candidate batch (ledgered; see step-2
  item 6).
- *Noticing:* nothing beyond the named-drain-label proposal (carried in
  step-2 item 7).

**Pass 1 — 3(b) (all lines run):**
- *Comment-claim audit:* "src always above dst" (compact copy) VERIFIED
  (a0>a3 strictly on entry, equal advance); the 128KB-check comment VERIFIED
  under the documented non-zero-length precondition; marker word/offsets
  VERIFIED against DMAEntry; the "~64 cycles/entry, ~514 for all 8" estimate
  FLAGGED to lens C1 (static arithmetic suggests ~72/entry; the `~` is
  honest but the number deserves a check). `bclr #23` "RAM source safety"
  kept verbatim and FLAGGED to C3 (hardware rationale not re-derived here).
- *Contract audit:* all seven procs verified body-within-license (and
  mechanically by the corpus closure error-gate); load_art's over-claim
  fixed twin-side; the dplc dead-save reliance note carried on the license.
- *Name audit:* `.drain_N` = drains N entries; `.transfer`; `.finish_entry`;
  ENTRY2_*; no renames needed.
- *Magic-number audit:* `#$93979695` gained its site comment (both twins had
  none); everything else named or commented.
- *Cold-reader test:* one enqueue + one Critical drain traced on headers
  alone — the slot-stride/entry-count relationship is stated at the table.
- *Codename audit:* twin scrub EXECUTED zero-byte — dma_queue.asm "item 11:"
  ×4 → behavioral facts; load_art.asm "Fable rider" dropped + the d0-d7
  header over-claim replaced with the proven license (aeon `e219e78`).

**Pass 1 — step-4 (all adjudications named):**
- SR-mask bracket → **NOT BUILT**; census updated to 7 save-sites / 4 files;
  dma_queue's 1-save/3-restore topology is the strongest pair-use-hazard
  instance yet (ledger row updated with the design question).
- Process_DMA_Important/Deferrable + QueueDMA entry-pair clones → **DEFERRED**
  (row-966 class: the AS twin cannot express the unification; divergent twin
  shapes raise lockstep cost; dies with the twin at Spec 5).
- fill_slot_markers / jt_slot / jt_filler / dma_send_entry → **BUILT** (step-0
  designed, step-1 shipped); kill row 40 attached (the same-commit rule paid
  at the 3(c) commit).
- vdp_comm_reg / assert-raise_error → **ADOPTED** (macro-port tax already
  paid at t15/t12).
- delete verb: **DMA_Overflow_Count (+ the DMA_Peak_*/Bytes_ThisFrame
  family) is WRITE-ONLY in code — ruled FEATURE scaffolding, KEEP**: debug
  profiling counters' reader is the emulator memory watch, not engine code
  (the Camera_Pan_Offset verb-(d) shape; surfaced here for possible
  override).

**Pass 1 — step-5 (FULL interrogation, per hot proc; heat: QueueDMA_* =
per-enqueue gameplay; Process_DMA_*/Drain_Budgeted = per-frame INSIDE the
VBlank window; Init/Level_LoadArt = cold):**
- *Invariant ladder:* Drain_Budgeted's `lea VDP_CTRL` already hoisted; the
  per-entry DMA_Budget_Remaining memory round-trip is the only
  below-its-scope item → LOG-not-cut (≤360 cyc/f worst AND needs a license
  widening that ripples both tail-callers; ledgered). Level_LoadArt's
  per-page `#Art_Staging_Buffer` reload: a1/d1 are callee-clobbered, only a
  d5 stash helps ≈ 8 cyc/page init-cold → LOG-not-cut. Process_DMA_Critical
  is straight-line (n/a). QueueDMA transfer core is loop-free (n/a).
- *Counter/cache audit:* DMA_Budget_Remaining — writers vblank(reset) +
  Drain(charge-before-send on EVERY send path) ✓; the charge-before-
  size-check overshoot is the pre-existing row-1468(3) note, inherited not
  re-ledgered. Critical drains UNBUDGETED by design (must-send class,
  bounded 8 entries — documented). Slot vars: writers/readers balanced
  (Init/enqueue/drain-reset/compact). DMA_Overflow_Count: debug-only
  writer, emulator reader (above).
- *Guard-coverage:* the queue-full check covers all three entries (single
  shared core) ✓; the split path re-checks the second slot ✓ — its
  carry-CLEAR-on-half-enqueue edge is SHIPPED behavior, comment carried,
  ledger row exists, NOT fixed (brief hard line). Level_LoadArt size-0 stub
  skip precedes decompress ✓; compaction's nothing-sent guard ✓.
- *Hardware cross-check:* drain stream order verified — the Command long's
  DMA-trigger word is the LAST word written per entry (movep interleave puts
  Command at +10..13; the drain's final move.w carries CD5) ✓ static.
  `bclr #23` RAM-source masking + the VBlank budget model + Z80/bus
  interaction → NAMED for lens C3 (mandatory).
- *Silent-tradeoff comments:* the one-slot split edge (comment + ledger);
  Critical unbudgeted (header); compaction persistence ("persist to the
  next frame" — carried); load_art release retry-loop-forever on a
  permanently-full queue (header documents the choice + DEBUG halts).
- *Threshold ruling:* **NO CUT — nothing within reach of ≥1k cyc/f**
  statically (largest identified item ≈ 340 cyc/f worst-case — C1-corrected
  to magnitude-only, d0 license-neutral); no live probes needed for a cut
  decision (no candidate is near the bar). C1 re-derived the header cycle
  claim (72/entry, not 64 — comment fixed both twins at adjudication).
- *Type-layer rider:* no register reshuffles taken → no blessings moved.

**Pass 2: EMPTY at all three steps (pass-1 output was comments/docs/rows;
re-walk surfaced nothing) → dry claim → panel dispatched (below).**

## PANEL ROUND (A1+B1+C1+C2+C3 — C3 mandatory; read-only; one round)

**DRY STOOD** (t18/t19 precedent: adjudication yielded comments, one comptime
ensure, and ledger corrections — no algorithmic, construct, or optimization
re-work). All findings adjudicated at the gate:

- *C2 (correctness):* **ZERO real findings** — every gate-blind checklist line
  re-derived clean: the 14-byte slot stride and 8-byte drain stride exact; the
  ENTRY2_* split-pair writes (incl. the movep.w-after-movep.l overwrite order);
  the 128KB boundary carry chain with no intervening flag-clobber; the carry
  pin surviving the SR restore AND an IRQ landing before rts (rte restores
  stacked CCR); the retry path leaving d6/a6/d7 intact (dbf only on success);
  Drain cursor arithmetic on all four exits; compact's strict src>dst. The
  $2700 mask proven LOAD-BEARING for gameplay enqueues (an unmasked IRQ6
  drain between slot read and commit would resurrect drained data — the mask
  makes gameplay and ISR enqueues mutually exclusive). One observation (F7):
  the abs.w-window coupling (4-byte lea slot arithmetic + the slot-var w-word
  round-trip) is backstopped by ram.asm's phase + overflow check — invariant
  lives off-file, noted here.
- *C1 (perf):* verdict ENDORSE-WITH-TEETH — the no-cut ruling and all three
  log-not-cut magnitudes CONFIRMED; two corrections SHIPPED: the header's
  "~64 cyc/entry, ~514/8" was ~12% low (real: 72/entry, 576/8-drain, ~670
  whole-proc — comment fixed BOTH twins), and the Drain budget-hoist deferral
  is MAGNITUDE-ONLY, not license-blocked (d0 is already licensed; ledger row
  corrected — a clean post-conversion-sweep candidate at ~20 cyc/entry).
  Root-cause note banked: the 72-cyc VDP send is irreducible bus traffic
  (3 longs + 1 word = exactly 7 words; movem can't target a fixed port), so
  no ≥1k lever exists in this file family.
- *C3 (hardware):* ONE real finding — **the budget model has no code-level
  coupling to the physical window**: 7200 B ≈ 35.1 of ~38 NTSC VBlank lines,
  but Critical drains unbudgeted on top (worst gameplay ≈ 1664 B ≈ 8.1
  lines) + the ledgered overshoot + uncounted CPU VDP writes → a saturated
  frame can hit ~43 lines. Degrades gracefully (lag frame), but ledgered
  with fix candidates for the post-conversion budget parcel (headroom
  reserve or a Critical charge); PAL V30 edge flagged. Everything else
  ALL-CLEAR with mechanisms: register/command ordering + the trigger-last
  final word verified; back-to-back drain entries safe BY CONSTRUCTION (the
  DMA arbitrates the 68k off the bus — no completion poll needed); bclr #23
  identified precisely as DMD1 defense (comment upgraded both twins); the
  drain's flip-flop-clear precondition documented (satisfied by construction
  — all VInt writers pair their words; $8F02 cannot strand); enqueue mask
  latency bounded at ~0.7 scanline worst (raster effects at most 1 line
  late); Level_LoadArt's display-off drain re-derived (8 KB ≈ 40 lines at
  full rate vs a 262-line display-off frame — the "extended VBlank" wording
  replaced with the real mechanism, both twins).
- *A1 (cold reader):* TWO real findings, both SHIPPED zero-byte: (1) the
  jump-table's physical stride had NO guard tying it to sizeof(DMAEntry) —
  the count ensure was blind to a layout change desyncing the dispatch index
  → `ensure(sizeof(DMAEntry) == 14, ...)` added naming the stride
  dependence; (2) the one clr=true vdp_comm_reg site rode the implicit
  default while the twin was explicit at all three → explicit `true` passed
  (the odd-one-out is now the loudest site). Asks ledgered: based
  struct-field displacement (`DMAEntry[1].field(aN)` — would delete the
  ENTRY2_* consts) and a movep-aware field type (would retire the recurring
  offset-map comments); the fold-cannot-emit-interior-labels limit named
  under the jump_table ask. Cold-reader fixes shipped: the d5/a3
  license-via-BG_Init note in Level_LoadArt's header. Slot-8 asymmetry and
  prose-magic-numbers accepted as observations.
- *B1 (corpus):* TWO real findings, both LEDGER corrections: (1) the
  jump_table construct demand is ≥3, not 1 — collision.emp:188 and
  animate.emp:142 already hand-spell bra.w tables (row updated: design-pass
  candidate, not an accumulation wait); (2) the row-966 "twin cannot express
  it" deferral rationale is STALE — perform_dplc already unifies against a
  longhand twin — so the pair-dedup deferral now stands honestly on SIZE
  (3-7-line bodies vs dplc's 30; wake condition: template at ~10+ lines).
  Direction-(a) checks all clean: the new helpers REUSE the fold-emit and
  Label-template idioms; no corpus re-hand-rolls of the new shapes; all four
  file-local consts single-sourced (the 8192 numeric collision with
  BG_LAYOUT_SIZE ruled semantically distinct — do not dedup).

## Step-6 corpus sweep (enumeration, per-site outcomes)

Additions with potential prior-file reach, every site named:
1. **DispSymInd (label-as-d16 EA)** — corpus census of jump/table dispatch:
   animate.emp:140 `jmp .cc_table-4(pc,d0.w)` is the pc-indexed EA (already
   supported, different class); hblank.emp's slot is a runtime-patched
   `jmp xxx.l` → both NOT-AN-INSTANCE. No other d16(An) tables exist.
2. **movep-aware access width** — corpus census: zero movep instructions in
   any other .emp (only structs.emp's layout comment mentions it) →
   NOT-AN-INSTANCE; buffers.asm (the other movep user) stays .asm and
   inherits the fix at its own port.
3. **Width-4 signed∪unsigned value cells** — corpus census of
   `move.l #<RAM-sym>` immediates: load_art.emp:112 is the ONLY site (a
   prior instance would have failed loudly at link; none did) →
   NOT-AN-INSTANCE elsewhere.
4. **Exported-label tail targets (resolve_callee_key)** — corpus census of
   `export .`: dma_queue's `.transfer` is the corpus's FIRST → no sweep.
5. **DMAEntry struct twin** — second potential consumer is buffers.asm
   (Static_Sprite_DMA writer uses the same interleave) → LEDGERED
   at-next-touch: its port adopts `DMAEntry.field` instead of re-mirroring.
6. **Named-drain-label jump-table spelling** — file-unique for now (census
   in item 1) → carried as the step-2 checklist proposal, no retrofit sites.
7. **fill_slot_markers / dma_send_entry helper shapes** — no corpus
   re-hand-rolls found (B1 lens confirms independently).

## NEITHER-BUCKET HEADLINES

- **The QueueDMA ownership flip landed exactly as briefed:** three extern
  decls deleted same-commit, four flip tests green, contract carried
  verbatim — and the port-order dividend paid (load_art calls
  QueueDMA_Critical module-to-module with no extern churn).
- **Two demanded features in one tranche** (the divs.w shape, TDD-first with
  byte-parity probes): symbolic d16 displacement (`jmp .jump_table(a1)`)
  and movep-aware access width. Plus the closure learned exported-label
  tails and sigil-link learned signed 32-bit value cells — four
  language/toolchain extensions from one file pair.
- **The load_art proven-preserves contract is the tranche's verifier
  highlight:** the closure error-gate refused the transcribe until
  `preserves(d6/a4-a6)` was declared, then the movem round-trip PROVED it —
  documentation that outran proof got pulled UP to proof, the reverse of
  t19's blocked-tightening case; the twin's d0-d7 over-claim is fixed.
- **The mixed arm earned its place immediately:** the width-4 signed-fold
  gap was invisible to the standalone gate (synthetic carrier masked the
  sign) and bit only against the REAL ram.asm equ chain — the exact class
  the paired-state doctrine predicts.
- **Named-drain-label jump table:** byte-identical to arithmetic rept
  (probe-proven BEFORE the port), more readable, no new language surface —
  proposed as house format for the stride-locked-table class.
- **Process:** one worktree-vs-main path bite caught same-minute (structs.emp
  header edit hit aeon main; reverted, main verified clean).

## POST-MERGE QUEUE (for the record)
- item-13 wave-2 (A4-i-gated): + the t20 candidate batch (slot-cursor/
  queue-end words, VramAddr-in-arithmetic sites, page count/cursor).
- dma_queue rollback parcel (the 128KB one-slot carry-clear edge + BgAnim
  LastStep poison — rows 1013/1467): now .emp-side work when taken.
- Cumulative mixed-ladder extension (standalone test-infrastructure job,
  overseer-ratified scope).
- Sprites-hardening parcel: PARKED (post-conversion).
