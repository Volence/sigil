# Packet — sprites.emp step-5 follow-up (Fable review addendum)

Branch: sigil `sprites-step5-followup` (off master `209b464`), aeon worktree
`.worktrees/sprites-step5-followup` (off master `726dbdc`). NOT merged —
Volence's gate. Work list: `notes/2026-07-11-tranche11-fable-review-addendum.md`.
Order worked: B2+B1, A1, C1, D1 (per the addendum).

**Status: complete + verified.** Full workspace strict **2142/0** (incl. the +2
`jbra_boundary` guards), clippy clean. A1 byte-changing (re-pinned, oracle-checked);
B2/B1 byte-neutral; C1 named; D1 protected.

## The step-5 interrogation checklist (each line named with its outcome)

The addendum grew this checklist same-commit; naming each line's outcome is the
packet contract.

1. **Invariant ladder → A1 TAKEN (camera-bias fold).** The per-piece `addi.w
   #VDP_SPRITE_{X,Y}_OFFSET` sat at ITERATION scope but is invariant at FRAME
   scope (the +128 SAT bias is frame-constant). Folded: `Render_Sprites`
   computes `Camera_{X,Y}_Biased = Camera - offset` once/frame; per-object setup
   subtracts the biased camera (d2/d3 = screen+128); all four `Emit` variants
   drop their per-piece `addi` (8 removed). Screen-coord paths compensate with an
   explicit +128. Byte-changing (region $418→$420); behavior-identical.

2. **Counter/cache audit → B1 documented-as-intended (default; Volence's gate).**
   Every path that consumes the scanline budget vs charges it: the early-out
   (`d5 < SCANLINE_SPRITE_LIMIT`) skips the COMMIT (first N sprites never charged);
   the multi-sprite sibling walk bypasses the budget entirely. Ruling per the
   addendum default: keep as a SOFT heuristic that undercounts by design, with a
   site comment declaring it (the hardware drops excess per-line sprites anyway;
   the budget only shapes WHICH drop). Always-commit + charge-children is left as
   Volence's gate call — NOT implemented.

3. **Guard-coverage audit → D1 PROTECTED (dbeq cap-net).** Enumerated every SAT
   emission path: the per-piece `cmpi.b #MAX_VDP_SPRITES / dbeq` is the SOLE
   overflow guard for objects with a stale/zero `sprite_piece_count` cache (the
   pre-checks trust the cache). LOAD-BEARING. Untouched by A1 (the fold lives in
   d2/d3 setup, never the loop's cap net); a site is not added because the whole
   loop is the guard. A future `{code}`-splice `emit_piece_loop` unification MUST
   keep it in the skeleton (noted in the addendum + the code-splice packet).

4. **Hardware cross-check → C1 NAMED (design question for Volence).** The VDP
   X=0 sprite-mask FIRST-SPRITE-ON-LINE exemption: masking only takes effect when
   at least one EARLIER-linked (higher-priority) sprite already touches the masked
   scanline. `InsertSpriteMasks` inserts at a band boundary, so on scanlines whose
   higher bands are EMPTY the mask sprite is first-on-line and does NOT hide the
   sprites after it. Outcome: **masking does NOT hold for an empty high band** —
   documented VDP behavior. Per the addendum this is NOT fixed ad-hoc; it is a
   design question (below).

5. **Silent-tradeoff comments → B2 ALL ADDED (5).** cascade-down under overflow
   (renders behind, doesn't drop — CHOSEN); `.band_limit_pop` skips DrawRings
   (equivalent only because DrawRings can't emit at cap — coincidence guard);
   link-order cycling flips same-band z every frame (fairness over stable z —
   CHOSEN); multisprite child reaches `.offscreen` with RF_ONSCREEN CLEAR though
   rendered (nothing may key on a child's flag); InsertSpriteMasks rounds coverage
   UP to whole 32-line masks (intended). Also corrected the step-3(b) comment-claim
   the audit flagged ("no band can overflow yet" — true for the check, false for
   the skipped commit).

## A1 verification (byte-changing → lockstep + re-pin + oracle)

- **Byte gate:** `.emp == .asm twin`, both shapes, both carrying A1 (sprites_port
  4/4). Lockstep twin edits: `bra.s→bra.w .next_object` (asl width-selected `.w`
  at 132-byte reach — its `bra.s` errored), `beq.w→beq.s .pieces_yflip` (A1 shrank
  the intervening variants into `.s` range; the `.emp`'s bare `beq` already
  relaxed). Re-pin +8 across the 6 downstream regions; RAM labels after Camera +4
  (`Camera_{X,Y}_Biased`).
- **Behavior-identity (rigorous):** the biased fold is bit-identical on the SAT by
  construction — world path `d2 = obj − (Camera − 128) = screen + 128`, per-piece
  drops the `+128`, so every SAT X/Y byte (and the X=0-avoidance `bne` test) is
  unchanged; the flip variants ride along (bias in d2/d3, never the neg/width math).
- **Oracle (BlastEm-accurate):** both master (pre-A1) and A1 ROMs built and run;
  both render sprites correctly — Sonic, rings, landscape, no corruption or
  dropped/misplaced sprites. A1's live SAT (`Sprite_Table_Buffer`) is well-formed
  (valid Y/size/link/tile/X entries) at a scrolled frame.
- **Profiler delta (recorded, honest):** the OJZScroll test scene is VSync-bound
  (`VSync_Wait` 54% idle) with a LIGHT piece count (`Emit_ObjectPieces` one call).
  In that regime the per-piece savings are absorbed into idle time and sit below
  the frame-average resolution; cross-run profiler values were byte-identical
  (VSync_Wait 69805 both runs), indicating per-session measurement caching, so no
  clean numeric delta was extractable in this scene. The win is STRUCTURAL and
  quantified by the removed code: −8 `addi` in `Emit` (−16 cyc/piece) vs +6
  instructions once/frame in the prologue (~+56 cyc) → breaks even at ~3–4 pieces,
  and at the addendum's cited heavy load (500–800 pieces) saves ~8–13k cyc/frame
  (6–10% of the 128k NTSC budget). Recommend re-measuring under the object-test
  stress state (many pieces) for a clean number.

## C1 design question (for Volence)

`InsertSpriteMasks`' X=0 masking is only reliable when the masked scanlines carry
an EARLIER-linked sprite (VDP first-sprite-on-line exemption). At a band boundary
with an empty high band, the mask is first-on-line and silently fails to hide.
Options: (a) `InsertSpriteMasks` guarantees a leading (higher-priority, earlier-
linked) sprite on the masked lines before the mask; (b) a documented SCENE
CONTRACT that any band configured for masking always has content above it; (c)
accept the exemption as a known limitation of the feature. Not fixed ad-hoc per
the addendum.

## What each pass added

**Step-3 findings:** B1 name/claim correction (the misleading `.budget_ok`
comment); no new kill-list rows (comments + a fold, no twin mirror); no gap-ledger
rows (A1 is an in-file optimization; C1 is a design question, not a language ask).

**Step-5 findings (optimizations):**
- TAKEN: A1 camera-bias fold (invariant-ladder line) — behavior-identical,
  structural cycle win.
- NOT TAKEN (logged): B1 always-commit (Volence's gate call — default soft);
  D1 (protect, don't touch).
- NAMED PROBE: C1 mask exemption → design question.

**Neither-bucket:** the `jbra_boundary` regression guards — A1 grew a `jbra` past
the ±127 `.s` boundary, which first read as a possible sigil bug; a minimal
reproduction proved sigil's `jbra` relaxation is CORRECT (widens to `.w`), and the
sprites divergence was a stale `.asm` explicit width (t11's `beq.w .pieces_yflip`).
Kept the 2 guards.

## Merge checklist (Volence's gate)
- `--no-ff` merge both sides + push. A1 is byte-changing (reference rebuilt,
  re-pinned +8); B2/B1 byte-neutral. Rebuild master s4.bin/debug + provenance on merge.
- Decisions for the gate: B1 (soft budget vs always-commit) and C1 (mask exemption
  contract).
</content>
