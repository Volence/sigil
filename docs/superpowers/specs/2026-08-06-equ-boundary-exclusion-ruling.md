# The equ-boundary exclusion ruling + invoke-edge closure (2026-08-06)

Status: RULED (Fable). Closes ledger row 2163 (the last S2-D6 §2 residue),
unblocked by b8's authorship model. Scout ground truth 2026-08-06:
`call_target_sym` (corpus_contracts.rs:1506-1511) recognizes only a bare
`CodeOperand::Sym`; an `invoke` lowers to `jsr (sym).l` = `CodeOperand::AbsSym`
(eval/asm.rs:728-759, 2181-2240), so both corpus invoke edges
(game_loop.emp:40 `Game.debug_tick`, boot.emp:324 `Game.boot_hook`) are
invisible to the transitive-clobber closure; b9 MEASURED the naive AbsSym
widening (zero new firings from the invokers, but the AssertDesugar rails'
`jsr (MDDBG__ErrorHandler).l` / `jmp (…_PagesController).l` become
`unresolved_callees` holes in EVERY shape) and reverted it — only the
documentation landed (commit 199abd50); the raise rails are stamped
`ItemAuthor::AssertDesugar` end to end incl. the terminal (eval/asm.rs:856-859,
935-938; pinned in diag_desugar.rs:521-570); the ONLY abs-long calls naming
equ boundaries anywhere in the `.emp` corpus are those authored rails;
`transfer_target_sym` (flag_check.rs:217-225) already reads AbsSym.

## 1 · The ruling — the exclusion is scoped by AUTHORSHIP, not by symbol kind

The ledger row sketched an "abs-long-indirect-to-known-equ/link-symbol
exclusion". REFINED: that shape would silently bless a future HAND-WRITTEN
`jsr (SomeEqu).l` — an unanalyzable call into a contractless link boundary is
exactly what the residue gate exists to surface, and "the symbol is an equ"
does not make the call innocent; it makes it undeclarable. The corpus census
says the equ-called set is precisely the compiler-authored rails, so the
principled and the sufficient exclusion coincide:

**An instruction authored `ItemAuthor::AssertDesugar` contributes NO callee
edge to the closure.** Per the codeitem-author invariant, authorship REDIRECTS
the obligation to the author's own contract — and the compiler's contract for
the rail is "this rail diverges; it owes the return path nothing" (the same
premise the noreturn model already consumes at cycle_budget.rs:715-717). The
rail's `jsr (HANDLER).l` and terminal `jmp (PAGES).l` are both inside that
authored divergent region; neither is a return-path effect of the proc that
hosts the assert. A hand-written call or jmp to the SAME blob symbols stays a
plain collected edge — a human claiming divergence spells `@noreturn` or
declares an `extern proc`; the two mechanisms do not blur (noreturn spec §1b's
sentence, applied to calls).

Consequences, stated so the gate's meaning is explicit:
- **Recognized AbsSym callee → resolved like a bare Sym**: a proc charges its
  contract; an `extern proc` charges its declared surface; anything else —
  INCLUDING an equ symbol — is an `unresolved_callees` HOLE. The honest exit
  for a future hand-written equ call is declaring an `extern proc` contract
  for the boundary, not a carve-out.
- The residue-empty gate stays the theorem "every call edge the corpus ships
  is contract-charged or compiler-owned", which is STRONGER than today's
  "every call edge we happen to recognize…".

## 2 · The build (lane t-invoke)

1. Widen the closure's callee recognition to AbsSym via the ONE existing
   AbsSym-aware extractor (`transfer_target_sym`) or a shared helper — do not
   add a fourth bare-Sym matcher spelling (`call_target_sym`,
   preserves.rs `call_target`:1405, calls.rs `direct_target` are already
   three; consolidate `call_target_sym` onto the shared form, LEAVE the other
   two and ledger their unification as the family row's next instance —
   widening the preserves oracle's recognizer changes conservative-but-sound
   behavior and is not this parcel).
2. Apply §1's authorship exclusion at the closure's collection sites
   (corpus_contracts.rs:765, :1345 — census any others).
3. Re-run b9's measurement as gates: residue empty ×7 shapes; the two invoke
   edges PRESENT in the closure (assert their existence — the edge being live
   must be pinned, not inferred from silence); zero new firings.
4. **Revert probe**: with the authorship exclusion stripped, the residue gate
   FAILS naming exactly {MDDBG__ErrorHandler, MDDBG__ErrorHandler_
   PagesController} — b9's measurement becomes executable.
5. **Non-vacuity pin**: a synthetic invoke whose bound hook clobbers a
   register the invoker does not declare FIRES the transitive-clobber
   diagnostic (the corpus invokers declare full-universe clobbers, so corpus
   silence alone would prove nothing).
6. Synthetic pins both polarities for the hand-written case: `jsr (equ).l`
   authored User → hole fires; the same shape authored AssertDesugar → no
   hole.
7. `call_target_sym`'s scope comment rewritten to the present-tense fact;
   ledger row 2163 CLOSED with the measurements; the optional-inversion row
   untouched.

## 3 · What this lane must NOT do

- No symbol-kind (equ-set) exclusion anywhere in the closure.
- No widening of preserves.rs `call_target` or calls.rs `direct_target`
  (ledger the family, §2.1).
- No Z80 half (no invoke exists on Z80; the z80 residue gate is untouched).
- No contract surface invented for the MDDBG boundaries.

## 4 · Bars

Byte bar seven targets identical (analysis + tests only). Full strict with
closing arithmetic; refreeze --check chain 48; repin unchanged; warn tiers
id-identical ×7 (closure firings are error-tier). Merge position: LAST in the
tail-seams queue (after t-edges and t-credit), rebased and re-measured.
