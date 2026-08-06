# Residue micro-parcel (b9) — close packet

Three small, unrelated-but-adjacent closures from today's scouting round. Small
parcel, full bars. Branches UNMERGED (overseer merges after a lens panel).

## Commits
- aeon `bc84f5d` — `section: RedrawPlanes declares the SR halves its body earns`
- sigil `199abd50` — `notes+contracts: the invoke-edge scope comment tells the measured truth`

(No merge-state claims. Land-order windows measured below.)

## Headline (numbers first)
- Byte bar: SEVEN targets UNCHANGED — s4 `c2d17ee3`/411096 · s4.debug `6c296656`/423480
  · demo `4a09314e`/91258 · demo.debug `f3e5ed3e`/93955 · config_a `4e34a38a`/423871
  · config_b `b8cce891`/301132 · lean `b92cb485`/379110. Warn tiers unchanged
  (plain 19 / s4-DEBUG 18).
- Item 1 LANDED (aeon contract flip, byte-identical, checker-verified).
- Item 2 NOT LANDED — the naive fix is measured to break the residue-empty gate
  (holes, not firings). STOP-AND-REPORT; the fix wants an equ/link-boundary
  exclusion design + its own gate run.
- Item 3 LANDED (sigil docs: design CLOSED, invoke-residue BLOCKED, tidiness lint
  parked; scope comment rewritten to the measured truth).

## Item 1 — Section_RedrawPlanes' honest SR contract (aeon) — DONE
`clobbers(d0-d4/d6/a0-a6, sr) out(d5, d7)` → `clobbers(d0-d4/d6/a0-a6/sr.ccr)
out(d5, d7) preserves(sr.mask)`. The body earns the split: `move.w sr,-(sp)` save,
`move.w #$2700,sr` mask, `move.w (sp)+,sr` restore (mask round-trips, no return in
the span), post-restore `cmp/bge` tracker clamp dirties only CCR, `rts`. Spelling
matches the shipped precedents QueueDMA_Deferrable (`preserves(sr.mask)`) and
Sound_DrainSfxRing (`clobbers(…/sr.ccr) preserves(sr.mask)` — the exact sr.ccr-in-
reglist form). Header comment rewritten to the present-tense partition fact.

Checker VERIFIED it (build succeeds; a refused preserves slice would error
`[proc.preserves-*]`). No slice refused. Byte bar 7/7 unchanged.

Caller chain (Section_UpdateColumns :467, no SR clause, jbsr's this proc): NO
lint/closure change. The callee now clobbers LESS of SR than the old bare `sr`
(mask preserved, only ccr escapes), so the caller's obligation strictly shrank —
nothing new fires. Is UpdateColumns' own missing SR clause flaggable now? NO:
only PRESERVES claims are verified; a missing SR clause / bare `clobbers` is
permission, not assertion. Warn tiers confirm it (plain 19 / DEBUG 18, unchanged).

Gap-ledger row 2088 reworded: the RedrawPlanes half is CLOSED (this parcel); the
sibling half stays OPEN and is now what the row names — QueueDMA_Critical/Important
cannot declare the `preserves(sr.mask)` their shared-core-via-tail earns because the
68k PRESERVES closure does not credit a tail transfer to a preserving callee. Cited
the Z80 precedent: `z80_preserves` already credits a callee's declared preservation
across its external-tail / `Edge::Defer` arm (the `callee_preserves` oracle) — the
design the 68k credit should port.

## Item 2 — the invoke call-edge closure blindness (sigil) — NOT LANDED (STOP-AND-REPORT)
The fix (add `CodeOperand::AbsSym` to `call_target_sym` + `calls.rs::direct_target`)
was trialled and MEASURED across all seven shapes via
`corpus_closure_residue_is_empty_the_error_gate`, then REVERTED.

Measured effect (identical in every shape — sonic4 plain/debug, demo plain/debug,
config_a, config_b, lean):
- NEW FIRINGS: `[]` — ZERO everywhere. The two corpus invokers (game_loop.emp:39
  `invoke Game.debug_tick`, boot.emp:323 `invoke Game.boot_hook`) lower to
  `jsr (bound_proc).l`; the bound targets are real procs, and both invokers already
  declare full-universe clobbers, so charging them adds nothing. The task's
  prediction (ZERO new firings) is CONFIRMED.
- NEW HOLES: exactly `{MDDBG__ErrorHandler, MDDBG__ErrorHandler_PagesController}`
  in EVERY shape (plain included). These are NOT invoke edges — they are the
  `assert`/`raise_error`/`raise_exception` diagnostics desugars, which expand
  (during the corpus walk's eval, DEBUG-gated for `assert`, and via at least one
  ungated raise path in plain) to `jsr (MDDBG__ErrorHandler).l` /
  `jmp (…_PagesController).l`. Those targets are contractless
  `pub equ … = extern("ErrorHandlerBlob")+off` LINK BOUNDARIES (neither proc nor
  extern proc, error_handler.emp:84+), so recognizing them registers them as
  `unresolved_callees` — which `corpus_closure_residue_is_empty_the_error_gate`
  (contract_closure_corpus.rs:378) treats as a build error.

So: the invoke edges resolve cleanly; the collateral abs-long desugar edges break
the residue-empty gate. The old scope-comment rationale (a) ("empty interface env
makes it moot") is stale — the per-shape gates walk BOUND envs. Rationale (b) (the
MDDBG holes) is REAL and now measured.

THE FIX (design, needs its own gate run — beyond a tight residue hunk + a
soundness-relevant hole-semantics call): an abs-long-indirect-to-known-equ/link-
symbol EXCLUSION. A `jsr (Sym).l` naming a declared equ/const/link boundary is a
RESOLVED boundary (⊥ for clobbers — not a hole; noreturn crash handlers anyway),
while a bare-`Sym` call to an unknown name STAYS a hole (`MysteryAsmRoutine`).
Thread the equ/const/link-symbol name set into the closure hole determination.
Author-based filtering is insufficient (only `assert` is tagged `AssertDesugar`;
`raise_error`/`raise_exception` carry the ambient `User` author).

Landed instead: the stale `call_target_sym` scope comment is rewritten to this
measured reason; the ledger records the residue as OPEN with the enumeration.
Pins/probes NOT added (they belong with the exclusion design). No sigil behavior
changed — Item 2's code is a pure doc-comment.

## Item 3 — queue/ledger truth (sigil docs) — DONE
Three gap-ledger rows appended (campaign-gap-ledger.md tail):
- (a) S2-D6 checked-clobbers lint DESIGN **CLOSED** — the "partially absorbed by
  B′, needs what-remains verification" queue framing was stale (6th stale-plan-item
  instance). Design shipped at merge `54f4eea4` (2026-08-02, "THE CHECKED-CLOBBERS
  LINT FULL CLOSURE (S2-D6)"): U1 ISA-derived write model, U2/U3 already shipped by
  contract-grammar G3, U4 `@allow(clobbers.unanalyzable, reason)`, negative probe
  fires. Cited `2026-08-04-finish-line-state-audit.md` §4 (:62-96, the K1-class
  finding) which already established this once.
- (b) `[proc.preserves-completeness]` tidiness lint — S-sized row, parked with its
  caveat: a declared clobber never written is silently accepted (sound — permission,
  not assertion), so this is WARN-tier tidiness at most, and BLOCKED on an
  annotation story (Collision_GetType deliberately over-declares d3 by the
  sensor-register convention, gap-ledger row 1068 — the lint needs a `@allow`-style
  opt-out before it can fire without a false tightening).
- (c) The invoke-edge is the last S2-D6 §2 residue — recorded as **OPEN/BLOCKED**
  (NOT closed, per Item 2's measured outcome), with the enumeration and the
  equ/link-boundary fix. The 54f4eea4 merge message itself names the invoke-edge as
  the ONE accepted deviation ("trialled, reverted with reasoning"), so this row is
  the honest continuation of that deviation, not a new claim.

## Measured land-order windows
- Item 1 (aeon): the `sr.mask`/`sr.ccr` spelling shipped in sr-split, so it parses
  and checks under CURRENT master sigil. VERIFIED GREEN: the byte bar + corpus gates
  ran against aeon b9 using the b9 sigil binary (master-equivalent for this spelling)
  — 7/7 CRCs unchanged, corpus gates green. aeon-first is GREEN; no sigil change is
  required for Item 1 to land.
- Item 2 (sigil): NOT LANDED (reverted). No window.
- Item 3 (sigil): docs + doc-comment only — no build/gate coupling.
- Both-window evidence: aeon b9 built clean under b9 sigil (byte bar); sigil corpus
  gates (contract_closure_corpus 19, warn_tier_corpus 3, out_verify_corpus 4,
  slot_type_corpus 5) + frontend-emp (corpus_contracts/contract_closure/calls/
  preserves) all green reading the aeon b9 tree. Clippy: the one `redundant_closure`
  warning is in mul_lower.rs (b6's lane, off-limits, pre-existing) — not this parcel.

## Verification
- Byte bar: capture_goldens.sh (SIGIL_BUILD/SIGIL_EMIT exported, AEON_DIR=b9) — 7/7
  CRCs unchanged; canonical s4/s4.debug restored.
- Strict corpus family: contract_closure_corpus 19/0, warn_tier_corpus 3/0,
  out_verify_corpus 4/0, slot_type_corpus 5/0 — all reading aeon b9.
- frontend-emp: corpus_contracts 29/0, contract_closure 29/0, calls 34/0,
  preserves 44/0.
- (Full `cargo test --workspace` not run to completion here — the native-ROM build
  tests exceed the session's foreground cap; the changes touch only corpus-gate
  inputs + a doc-comment, and every gate those changes could move is green above.)

## Step-3 vs step-5 split
- STEP-3 (retrospect / correctness truth): Item 1's honest SR contract (the body was
  always stronger than the bare-`sr` contract; the split makes the truth checker-
  verified). Item 2's measurement is a step-3 finding: the invoke closure blindness
  is real, but the naive fix's collateral (MDDBG link-boundary holes) is the deeper
  step-3 truth — the residue can't close without the equ-boundary semantic. Item 3
  is pure step-3 ledger hygiene (stale-plan-item catch #6; design CLOSED, residue
  BLOCKED honestly).
- STEP-5 (engine optimization): NONE. No hot-path or byte-changing engine work in
  this parcel — all three items are contract/comment/ledger truth.

## Neither-bucket headline
The parcel's real yield is a **scope correction discovered by measurement**: Item 2
looked like a tight one-line matcher extension, but running it against every shipped
shape proved it breaks the residue-empty gate via the diagnostics-desugar's
abs-long calls to contractless `pub equ` link boundaries — a class the task's
"ZERO new firings expected" framing did not anticipate (firings ARE zero; the
breakage is holes). The correct fix is a hole-semantics design decision
(equ/link-boundary is a resolved boundary, not a hole), which belongs to the
overseer/lens panel, not a residue micro-parcel. Enumerated and reverted BEFORE
landing, exactly as the parcel's STOP-AND-REPORT clause directed.
