# Tranche 9 checkpoint packet — animate.asm → animate.emp (2026-07-10)

**Status: loop DRY, awaiting Volence's gate. NOT merged.**
Branches: sigil `port-tranche9` (from `3e53683`), aeon worktree
`sigil-emp-tranche9` (from `07e465c`). Strict workspace **2054 passed / 0
failed**, clippy clean. Pins: plain **`3b0357ad…`** / debug **`8cd33561…`**
(PROVENANCE tail has full hashes + the slide table).

## What shipped

`engine/objects/animate.asm` (the animation interpreter: `AnimateSprite`,
`AnimateSprite_PerFrame`, the $F7-$FF control-code/event dispatchers,
`RefreshSpritePieceCount`, the `reloadAnimTimer` macro) →
`engine/objects/animate.emp` under `SIGIL_EMP_ANIMATE`. Region base plain
`$2D78` / debug `$3032`; len **0x308** after step 2 (see below), shape-
invariant. The leanest cross-seam surface of the campaign: NO RAM cells, two
inbound code labels (`DeleteObject`, `Sound_PlaySFX`), no game-owned or
module-local mirrors.

- **Step 0**: design note `2026-07-10-tranche9-animate-design.md` — all 7
  handoff hazards settled before code; baseline pins verified in the fresh
  worktree (editor data seeded, both shapes rebuilt to the exact pins).
- **Step 1**: byte-exact both shapes at len 0x312 against the t8 pins,
  including the **AF_\* truth re-home** animate.asm → `engine/constants.asm`
  (byte-neutral, proven by rebuild; script data files keep their truth when
  the gate strips animate.asm from the AS side). `reloadAnimTimer` →
  module-local `comptime fn reload_anim_timer(src: Reg) -> Code` — the
  utag-death pattern's third exhibit (hygiene obsoletes the `tag` param),
  first template carrying MEMORY operands (`Sst.anim_timer(a0)`) and a
  module const (`#DUR_DYNAMIC`) inside `asm {}`. Constants twin 24 → 30
  (consumed-only animation block). sonic_anims row-3 consolidation (3
  locals + ensures → `use engine.constants`). Harness `animate_port.rs`:
  reference gates both shapes, SND combo probe vs a fresh AS-twin oracle,
  doctored `AF_SET_FIELD` guard probe, outbound bare-`jsr` consumer
  (player_common's exact shape, abs.w relaxation proof), structural
  anchors; mixed tranche-9 ladder + acceptance both shapes.
- **Step 2**: full house format. ONE spelling byte-change
  (`jmp DeleteObject` → `jbra`, length-neutral bra.w) — and the bare-Bcc
  relaxation found **five suboptimal hand widths** (`bhi.w` ×2, `bhs.w` ×2,
  one `bra.w` tail-call): region 0x312 → 0x308, **the rule's first real
  shrink** (t7/t8 hand widths were all optimal). The −10 slid every
  downstream engine pin to `org $10000`; the generalized re-pin rule
  executed in full (gate orgs, region bases, label VMAs, the tranche-5
  game_loop byte-pin displacement — every value from listings). Pinned
  exceptions commented in place: the two 9-entry `bra.w` dispatch tables
  (load-bearing 4-byte slots), the template's `bne.s` (macro-twin lock).
- **Steps 3/4**: retrospect recorded (below); back-prop sweep **EMPTY** —
  prior ports carry no static jmp/jsr outside documented exemptions (rings'
  row-16 transliteration, game_loop's row-9 macro mirror).
- **Step 5**: **no optimization taken, reasons recorded** (ledger) —
  hot path already minimal; the one big candidate is the D8 headline below.
  **LIVE-VERIFIED in oracle**: AnimateSprite traced instruction-level on
  the real player (anim-change path, DUR_DYNAMIC → d3 hold, mapping-frame
  write, RefreshSpritePieceCount), Walk script cycling in the real game
  state with real art; collision + rings (both slid −10) live under Sonic.
- Loop pass 2 retrospect: EMPTY → dry.

## HEADLINES for the gate (decisions only Volence can make)

1. **`AnimateSprite_PerFrame` is DEAD — zero callers** (only its own
   self-loop). It is 404 bytes (~52%) of the region: an exported engine API
   for the S3K-style per-frame-duration script format, documented, never
   used. Options: (a) keep (future content may want the format; it stays
   byte-locked at zero maintenance until touched), (b) delete (−404 bytes,
   another full upstream re-pin — cheap now that the sweep is routine, and
   the .emp+twin+harness all shrink). Ported faithfully for now.
2. **Kill row 3 CLOSED by consolidation; row 2 re-homed into row 1's
   class** — the written kill ("animate.asm ports → flip") was unexecutable
   exactly as the row-13 lesson predicted (pitcher_plant/anims.asm + the
   gate-off twins read AF_* AS-side in both build shapes). AF_* truth now
   lives in `engine/constants.asm`, ONE home, dies at row 1's flip.
3. **The bare-Bcc rule shrank a region for the first time** — and exposed a
   surface asymmetry now recorded as procedure: the sigil AS front-end
   deliberately PINS branch widths (".asm is explicit-width ground"), so
   when .emp relaxation shrinks below the twin's hand widths, the twin
   re-spells the changed sites explicitly at the new optimal widths
   (asl verified to agree — same hashes from bare and explicit spellings).

## What each pass added (step-3 vs step-5, per loop pass)

**Pass 1 — step 3 (asks / reads-wrong / rows / ledger):**
- ASK (diagnostics, 1 data point): unexported-label hint — an
  `Owner.label` reference that misses while `Owner` has a non-exported
  `.label` should name the fix (`export .label:`). The link error today is
  a bare "unresolved symbol".
- AnimId/FrameId newtypes: demand data point AGAINST (interpreter-side) —
  raw byte arithmetic would only gain cast ceremony here; the demand moment
  stays the module boundary (construct-walk #3 thread).
- Interpreter-duplication note: the two interpreters share ~90% of the
  control-code machinery; template-unifying the .emp is deferred while the
  flat .asm twin must lockstep (divergent source shapes = higher tax).
  Moot if D8 resolves as delete.
- Kill list: row 2 re-worded (truth `engine/constants.asm`), row 3 closed,
  row 5 grows `animate.asm` (+ the macro↔template lockstep pair).
- Ledger: bare-Bcc shrink-lockstep procedure recorded (headline 3).

**Pass 1 — step 5 (optimizations taken / not taken):**
- TAKEN: none (the step-2 −10 shrink was format-driven, not step-5).
- NOT taken, with numbers (ledger): the ~56c hot-path flip-sync (equal-cost
  alternatives, behavior-load-bearing); the event-chain d1 re-derivation
  (~16c, cold); the bra.w dispatch tables (≈cost-neutral vs offset table,
  cold); `andi.w #$FF` in the dispatchers verified LOAD-BEARING (clears the
  `add.w d0,d0` high byte for anim ids ≥ $80) — reads-dead but isn't.
- D8 (dead PerFrame) deferred to the gate (headline 1).

**Pass 2 — retrospect: EMPTY (dry).**

**Neither-bucket (step-1 demanded features / probe outcomes / live):**
- **DEMANDED + SHIPPED: pc-rel target addend** — `jmp .cc_table-4(pc,d0.w)`
  (the −4 lands inside the jmp itself; no relocated label can express it).
  Parser: `.local` operand atoms take binary continuations
  (`binary_continue` split from `expr_bp`); eval folds the comptime addend;
  `CodeOperand::PcRel{,Idx}` carries it; lowering emits `Sym ± n` through
  the existing `PcRelDisp8/16` fixup fold. Global-label `Sym±n(pc,…)` rides
  the same path.
- **First real consumer of spec §5 `ProcName.label`** — the PerFrame
  table's `bra.w AnimateSprite.cc_delete` (cross-proc shared delete stub),
  via `export .cc_delete:`. Worked as specified once exported (the
  discoverability gap is the step-3 ask).
- **First cross-seam bare `jmp`/`jsr` abs.w relaxations** — both directions
  proven: the .emp's `jbra DeleteObject` → bra.w with the per-shape
  displacement, and the AS-side bare `jsr AnimateSprite` (player_common's
  shape, undefined in-unit) → `4EB8 base` in the outbound probe AND the
  full mixed acceptance.
- Template with memory operands + module consts inside `asm {}`: worked
  first compile (aabb only ever spliced registers/labels).
- SND combo probe: .emp == fresh AS-twin oracle at SND=0 and SND=1.
- Live (oracle): anim-change traced (prev_anim $FF→0, timer←d3=8 via
  DUR_DYNAMIC, mapping_frame←7, piece_count←5), Walk cycling in the real
  game state; A/B vs master confirmed the idle-state prev_anim=$FF
  sentinel is pre-existing behavior, not a port artifact. AF_SOUND /
  AF_CALLBACK paths have no live consumer in current content — covered by
  the byte gates + the SND oracle (recorded, not hidden).

## Merge checklist (post-gate)

- If D8 = delete: one more loop pass (delete → re-pin → re-verify) BEFORE
  merge; if keep: merge as-is.
- `--no-ff` merge both sides + push; rebuild master ROMs; PROVENANCE tail
  already carries the tranche-9 re-baseline; worktree removal at close.
- Empyrean amendment stack (Volence's cadence, uncommitted) grows: pc-rel
  addend operand form, the export-label first-consumer note, the bare-Bcc
  shrink-lockstep procedure.
