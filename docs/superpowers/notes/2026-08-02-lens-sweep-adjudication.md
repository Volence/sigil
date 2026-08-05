# The modernization + dry-panel lens sweep — adjudication packet

**Panel (2026-08-01):** 14 read-only seats over the 118-file / ~29.7k-line converted
corpus. Ratified composition extended live by Volence: A (ceremony) · A2 (comment
TRUTH — new lens, minted this sweep) · B1 (construct reuse) · B2 ×2 (cross-file dup,
code-first + data-first walks) · C1 ×2 (instruction perf, opposite walk orders) ·
C2 ×2 (gate-blind hazards, forward + reverse/Z80-first) · C3 ×2 (hardware timing,
outward + handler-first) · C4 ×2 (algorithmic altitude — new lens, both directions)
· C5 (space/footprint — new lens). Overseer own-verified every load-bearing citation
before this packet (parallax bracket, camera sites, SeqChannel prefix, sound-header
truth, 5-file .asm survivor census).

**Headline verdict, unanimous across altitudes:** the engine core is genuinely
hardened and tight — stride/size hazards are empty BY CONSTRUCTION (the ensure
discipline), the §17 hot paths hold at instruction AND technique altitude, the data
layer's magic numbers all carry derivations. The actionable set is small, precise,
and mostly meta (comments, contracts, test scaffolding).

## SECTION 1 — byte-changing fix parcels (full bars: A/B evidence via ab_runner,
re-pin, refreeze; each is its own small parcel or one combined sweep-fix parcel)

- **F-1 · parallax Z80-bracket bug (BUILD, S).** parallax.emp ~:409 — the reg-$0B
  live write's stop_z80/start_z80 is the ONLY unmasked must-be-atomic Z80 hold in
  the tree; a mid-bracket IRQ6 runs VBlank's own stop/start pair and returns with
  the bus RELEASED, so the pending VDP_CTRL write lands with the Z80 live on its
  bank window. The site's comment proves flip-flop atomicity (C3-1 verified that
  property and passed it) but nothing covers the bus latch (C3-2 caught it; seats
  verified DIFFERENT properties — union is the truth). Fix = the house sr_masked
  bracket (sound_api ×3, bg.emp, section.emp all do it, citing this exact hazard).
- **F-2 · camera modernization (BUILD, S-M).** Two seats independently:
  camera.emp .apply_x/.apply_y promote a word step to 16.16 via ext.l + lsl.l#8 ×2
  (~52 cyc) where engine.coords.pixels_to_coord (swap+clr.w, 8 cyc) is the blessed
  idiom the file itself never imported — the site comment says "split for AS":
  a transliteration fossil. ~44-88 cyc/frame. Same parcel: hoist the act-constant
  clamp bound to Camera_X/Y_Max RAM words at init (+invalidate on act change,
  ~100-124 cyc/frame) and the clamp shift form swap+lsr (~32 cyc/frame).
- **F-3 · Section_RedrawPlanes audio bracket (BUILD, S).** C3-1: the mid-play
  cache-recovery redraw masks IRQs for a ~3-frame VDP poke storm without raising
  SND_CTRL_DMA_ACTIVE — the Z80 stays on FILL and contends → DAC underrun risk on
  a rare path. Fix = the same flag bracket VInt_Level uses.
- **F-4 · VInt_DrawLevel autoincrement hardening (BUILD, S, ride-along).** The
  empty-buffer fast path returns without restoring reg $0F=2; safe only by an
  ambient invariant nothing asserts. Restore on .reset too (or DEBUG-assert).

## SECTION 2 — measure first (the new T2/T3 tools exist for exactly this)

- **M-1 · VBlank budget window-awareness (C3-2).** Claim: Critical (~1.6KB) +
  plane drain (~1.5KB) ride the window uncharged against DMA_BUDGET_NTSC=7200 →
  worst-confluence frame >10KB vs ~7.5KB window; plus the last-entry overshoot in
  Drain_Budgeted_Queue. Oracle profiler on a constructed worst frame FIRST; retune
  only if measured real.
- **M-2 · BLOCK_STAGE_SLOTS=16 (C5 R1, 12,288B = the biggest lower_ram item, the
  ONLY discretionary lever on the 81.5% region).** 16→12 reclaims 3KB but trades
  re-decompression under fast scroll. Decompress-thrash measurement first; never
  a blind cut. (C5 R2 window rebalance: zero-sum, skip unless lower_ram's window
  itself becomes the constraint.)

## SECTION 3 — byte-neutral parcels

- **N-1 · THE COMMENT SWEEP (BUILD, M — combined truth + history).**
  (a) TRUTH (A2, all verified): 5 sound headers claim the INVERSE of the build
  reality ("asm twin canonical … NOT wired" — twins deleted, .emp is the only
  source); 4 masks-hazard sites attribute live safety to deleted AS guards
  (seq_opcode_tab, dac_sample_tab, sfx_blob_win_tab, sound_tables_z80); ~15 more
  files assert live lockstep with deleted twins; dac_banks' main.asm skip-arm;
  demo constants' config/game.asm refs; error_handler's vectors.asm/engine.inc;
  compression_selftest's user-facing ensure message ("re-widen in BOTH twins");
  C5's ram.emp:76 "9216 (12×768)" vs actual 12288 (16×768); repin.toml:400-402
  --verbose comment (comment-only edit, sanctioned). This class BIT this sweep:
  it misled both C1 seats into a wrong off-limits rationale.
  (b) HISTORY (Lens A): ~65 hits / ~40 files of parcel/kill-row/byte-delta
  narration violating the house rule — rewrite to present-tense contract fact or
  delete. Zero brace-indent drift; that class is clean.
- **N-2 · SeqChannelBase prefix (BUILD, M).** sound_constants: SfxChannel/
  SeqChannel hand-duplicate a ~33-field prefix with only 13 offset-asserts.
  Preferred: single-author the prefix (nesting/flatten pattern per ParallaxCfg),
  byte-neutral by construction, asserts become redundant. Fallback if the
  construct can't flatten cleanly: complete the assert coverage to every shared
  field (S). NOTE: this is sound_constants (68k-side struct decls) — layout-locked,
  not timing-locked; byte gate is the proof.
- **N-3 · test-scaffolding folds + mapping-DSL home (BUILD, S-M, one parcel).**
  Triple-corroborated (A, B2-2; B2-1 the emitters): shared mapping-DSL module
  (MapPiece/MapFrame1/spr_size/centered copied verbatim demo_data↔test_mappings);
  emitter_tick shared body (test_emitter/test_stress_emitter drift); test_obj
  init-prolog comptime fn (~7 files); vdp_reg+bytes_to_lcnt hoist to engine.vdp
  (already-ledgered 3-site debt); interact_off to the engine. (Header structs:
  weak, skip — per-game ownership defensible.)
- **N-4 · dac_sample_tab [DacSample; 10] retrofit (BUILD, S-M).** B1's one clean
  finding. Precondition: byte-verify the [Struct;N] lowering emits the Z80-side
  little-endian u16s identically inside the (cpu: z80, vma:) section.

## SECTION 4 — structural (sigil-side, the sweep's biggest single item)

- **S-1 · THE CHECKED-CLOBBERS LINT (S2-D6): callee's actual register writes ⊆
  declared clobbers/preserves, verified mechanically (BUILD — own spec+parcel,
  M-L).** C2-1's noticing-clause verdict: the dominant remaining gate-blind
  surface is prose-only clobber-subset reliance (the LOAD-BEARING & INVISIBLE
  a5/a6 hoist across the decompressor; the exhaustive-license preserves under
  children/dplc; the splice-template contracts). One lint closes the entire
  class the byte gate structurally cannot see. Recommended scheduling: its own
  arc item immediately after the sweep parcels, before/alongside A1/A2.

## SECTION 5 — gap-ledger adds (language demand, corpus-proven this sweep)

computed-name extern() (2nd strong consumer: error_handler's 45-line MDDBG__
table) · conditional-out on a clobbered register ([proc.out-clobbers-overlap]
forces AllocDynamic's knowingly-false contract) · the imm-link lowering gap (4
manual .w pins in core.emp) · cross-module newtype through the harvest seam (the
blocker on SongId/SfxId reaching the sound modules) · bulk offsets-ordinals-match-
extern assert (sonic_anims' 12 ensure lines) · counted/flex-array-with-header
struct (entity_data's ObjTypeTableN) · table sentinel-record-list (already
ledgered; 3 sites confirmed). A2 verified ZERO stale deferrals — every existing
"awaits X" cites a genuinely unshipped blocker.

## SECTION 6 — declines / deferrals (recorded)

mulu-vs-repeated-add (section.emp — code says deliberate; VOLENCE TASTE CALL,
lean keep) · TouchResponse near-bit (C4-1 recommends against, coupling) ·
parallax idle dirty-gate (helps idle only; revisit at content era) · camera-px
compute-once + center-derived-twice (instruction trivia) · staging-probe index
(F1: profiler-gated) · dead count byte in type tables (fold-opportunistic at next
entity-gen touch) · AABB neg.w $8000 + parallax word-gap (harden-when-touched
notes ride N-1's files) · Sound_Dbg_Mirror 176B (conscious layout-parity
tradeoff, keep) · ROM space (honestly empty — pipeline already compresses/dedups/
strips) · HBlank raster contract (no armed consumer; doc note rides N-1).

## OPEN RULINGS FOR VOLENCE

- **R-A · the `dispatch` construct has ZERO corpus consumers** (every site uses
  offsets + typed jsr cast, arguably cleaner): retrofit the sites onto it, or
  retire the construct? (Construct-health; no lean — taste.)
- **R-B · mulu-vs-repeated-add** (above — lean keep as-written). — CLOSED
  2026-08-05, mul-lowering parcel: resolved STRUCTURALLY per Volence's own
  proposal (the 2026-08-03 design). `mul_bounded` makes the choice the cost
  model's; at the site's bound the worst-vs-worst verdict is mulu (ceiling 70
  vs loop 28 + 18·M — the loop wins only through M = 2), so the code-review
  argument is now a computed fact. The section.emp site itself stays
  as-written until a byte-changing parcel adopts (word-width contract gap —
  see the mul-lowering ledger rows).
- **R-C · scheduling**: sweep parcels (Sections 1+3) → S-1 lint → A1/A2 arc, or
  S-1 first? (Lean: parcels first — small, evidence-fresh; lint spec in parallel.)

**Loop-until-dry:** after the accepted parcels land, one FRESH panel round must
return nothing new before the sweep is declared dry (per the ratified rule — this
round finding things means we go again).

**Tooling note:** T1-T7 all closed this session (T2 memory_hash + T3 ab_runner in
oracle main 250428c; T4/T1 sigil 2b4f1b35; T5 already-gone recorded, T6/T7 sigil
4aac6a86). Section 1 parcels are the first real consumers of the ab_runner bars.
