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

4. **Hardware cross-check → C1 ACCEPTED + documented + ledgered (Volence's gate
   ruling).** The VDP X=0 sprite-mask FIRST-SPRITE-ON-LINE exemption: masking only
   takes effect when at least one EARLIER-linked (higher-priority) sprite already
   touches the masked scanline. `InsertSpriteMasks` inserts at a band boundary, so
   on scanlines whose higher bands are EMPTY the mask sprite is first-on-line and
   does NOT hide the sprites after it — masking does NOT hold for an empty high
   band (documented VDP behavior). Volence's ruling: **ACCEPT the exemption as a
   known limitation, document it at the site, and ledger the leader-sprite fix**
   (a future guaranteed leading sprite on the masked lines) as consumer-gated. No
   ad-hoc fix. Site comment added to `InsertSpriteMasks`; gap-ledger row added.

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
- **Oracle (BlastEm-accurate) — PIXEL-IDENTICAL, frame-locked:** master (pre-A1)
  and A1 ROMs, both reset to the same deterministic settled frame (`Camera_X` =
  `$0060`), produced a **byte-identical Sprite_Table_Buffer** (the literal SAT the
  VDP renders): `00E8 0501 A3EC 0118 | 0085 0502 03E8 014C | 0085 0500 03E8 015C
  | 0000 …` on both, and **byte-identical framebuffer PNGs** (same md5
  `a03fbdf5…`). Identical SAT ⇒ pixel-identical sprites. Empirical confirmation of
  the algebraic proof.
- **Profiler delta (recorded, honest) — corrected math:** the OJZScroll test scene
  is VSync-bound (`VSync_Wait` 54% idle) with a LIGHT piece count (3 sprites in the
  SAT). In that regime the savings are absorbed into idle time and below the
  frame-average resolution; cross-run profiler values were byte-identical
  (`VSync_Wait` 69805 both runs) — a profiler measurement-caching bug (stale data
  after ROM reload; gap-ledgered), so no clean numeric delta was extractable here.
  The win is STRUCTURAL: −8 `addi` in `Emit` = **−16 cyc/piece** (2 addis × 8 cyc),
  vs **+64 cyc/frame** once in the prologue (2× [`move.w abs`+`subi`+`move.w abs`]).
  Pieces are HARD-CAPPED at `MAX_VDP_SPRITES = 80` (the SAT ceiling), so the win is
  bounded: **~1k cyc/frame ceiling** (80 × 16 = 1280, minus the 64 prologue) at a
  full SAT — **break-even ~4 pieces**. NOT 8–13k (that assumed an impossible
  500–800 pieces; the 80-sprite SAT cap is the real ceiling). A clean stress-scene
  re-measure (near-full SAT) is gap-ledgered.

## C1 ruling (Volence: accept + document + ledger)

`InsertSpriteMasks`' X=0 masking is only reliable when the masked scanlines carry
an EARLIER-linked sprite (VDP first-sprite-on-line exemption). At a band boundary
with an empty high band, the mask is first-on-line and silently fails to hide.
**Ruling: ACCEPTED as a known limitation** — documented at the `InsertSpriteMasks`
site, and the leader-sprite fix (a future guaranteed leading sprite on the masked
lines) is gap-ledgered (consumer-gated: build it when a scene actually needs
masking over a potentially-empty band). No ad-hoc fix.

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

## Gate rulings applied (Volence 2026-07-11)
- **B1 — keep SOFT** (the soft-budget comment stands; always-commit NOT taken).
- **C1 — ACCEPT** the mask exemption; documented at the site; leader-sprite fix
  gap-ledgered.
- **A1 — pixel-identical verified** (SAT + framebuffer byte-identical); cycle math
  corrected (16 cyc/piece, ~1k/frame ceiling at the 80-sprite SAT cap).
- Gap-ledger: profiler measurement-caching bug + A1 stress-scene re-measure +
  C1 leader-sprite fix (all recorded).
- Merge: `--no-ff` both sides + push; rebuild master s4.bin/debug + provenance.
</content>
