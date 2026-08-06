# 68k preserves-through-tail credit (2026-08-06)

Status: RULED (Fable). Closes ledger rows 2079 + 2088 (sibling-credit half).
Scout ground truth 2026-08-06: the Z80 credit is the `CalleePreserves` oracle
consulted at `z80_preserves.rs`'s `Edge::Defer` arm (:486-501), FallOff arm
(:456-475), empty-body arm (:316-334), and `ever_clobbered` pre-pass
(:353-367); the 68k register-preserves walk treats `ExitKind::Defer` as NOT an
obligation site (`ReturnScope::AllReturns` → `ends_this_body()` false,
preserves.rs:244-249, :378-385) — neither refused, nor charged, nor credited;
`check_preserves_sr_ccr`'s `sr_tail_refusal` (proc.rs:1888-1896) refuses every
tail; `closure.rs` already reasons clobbers through tails (row 2079); the
customer class is real and larger than the QueueDMA pair (§4).

## 1 · The two defects this closes (one soundness, one capability)

**(a) The 68k has the vacuity hole the Z80 already closed.** A 68k proc whose
every exit is a tail transfer has ZERO obligation sites under
`ReturnScope::AllReturns`, so any `preserves(rN)` on it verifies VACUOUSLY —
the exact §13.4 hole `z80_preserves` closed ("a proc that ONLY tail-transfers
used to pass `preserves` vacuously"). Row 2079 states it for `check_preserves_sr`
in so many words. Corpus-dead today only because no tail-only 68k proc declares
`preserves` — which is defect (b):

**(b) Honest contracts are inexpressible.** `QueueDMA_Critical`/`Important`
reach `QueueDMA_Deferrable.transfer` by `jbra` and genuinely keep
`preserves(sr.mask)` through it — `dma_queue.emp:80-92` documents the demand in
prose because the checker cannot credit it. Six sound-API siblings are the same
shape (§4).

## 2 · The mechanism — mirror the Z80 credit, same trust argument

Give the 68k register-preserves walk a callee-preserves oracle (the 68k
`CalleePreserves` analog: local proc's declared `preserves` ∪ module
`invariant`; `extern proc`'s declared `preserves`; absent/indirect callee
preserves NOTHING) and consult it exactly where the Z80 does:

1. **`Edge::Defer` exit**: recover the tail-callee symbol from the transfer's
   operands — use the AbsSym-aware extractor (`transfer_target_sym`,
   flag_check.rs:217-225), not a new bare-`Sym` matcher (the extractor family
   already has three divergent spellings; see the equ-boundary ruling §2.1).
   `rN` survives this exit iff the entry bit is intact at the jump AND the
   callee preserves `rN`. The Defer exit BECOMES an obligation site — that
   closes hole (a) in the same motion that grants the credit. A Defer whose
   target is unrecoverable (computed jmp) or resolves to no contract-bearing
   proc/extern (an equ boundary) preserves NOTHING — conservative, and
   consistent with the equ-boundary ruling.
2. **`@noreturn` composition**: a Defer whose target is `@noreturn`-declared
   (or an AssertDesugar-authored divergent rail, per the noreturn model)
   CLOSES the path — a diverging exit never returns to the caller, so it
   carries no preserves obligation. Without this, adopting the credit would
   false-fire on every proc whose error rail tails into the handler blob.
3. **`falls_into` threading, all three arms**: the 68k register-preserves walk
   gets what d29d63a0 gave only the sr.ccr path — empty-body successor
   consult, FallOff-arm successor consult (entry bit ∧ successor preserves),
   mirroring `z80_preserves.rs:316-334`/`:456-475`. `verify_preserved` grows
   the `falls_into`/oracle parameters; `proc.rs:1460` threads them.
4. **The sr.mask half**: `sr_tail_refusal` keeps refusing `falls_into` and
   run-off-the-end, but an UNCONDITIONAL external tail to a callee whose
   contract declares `preserves(sr.mask)` (or bare `preserves(sr)`) is
   CREDITED for the mask claim — the sibling's own body must still round-trip
   or never touch SR outside the credit (the existing slice logic decides; the
   credit only discharges the tail edge). The CCR half stays refused at tails
   (the flags the caller sees are written by the sibling's own body at the
   jump; no tail credit can make an sr.ccr claim true) — refusal text
   unchanged, and the ccr advisory keeps its current tail behavior.

**Trust**: identical to the Z80 note (z80_preserves.rs:50-59) — a local
callee's declared preservation is itself proven by this same per-proc check,
an extern's is the extern trust boundary, and a caller's own write always
clears the entry bit first, so the credit can never mask a local clobber.
Restate this in the 68k oracle's doc comment; do not hand-wave it.

## 3 · Sequencing with the Cfg::edges unification lane

This lane consumes Defer edges as "genuine external transfer". The t-edges
lane (its own spec, same arc) reclassifies trailing-local-label transfers
Defer → FallOff. **t-edges merges FIRST**; this lane rebases over it and
re-measures, so the credit never sees a trailing-label pseudo-Defer. Until the
rebase, do not special-case trailing labels here — that would duplicate the
narrow b8 fix a third time.

## 4 · Adoption census (byte-neutral — contract text and analyses only)

Adopt what the checker PROVES; anything it refuses is a finding, not a forced
edit. Candidates, measured by the scout:

- `QueueDMA_Critical` + `QueueDMA_Important` (dma_queue.emp:94-104):
  `+ preserves(sr.mask)`; retire the prose apology at dma_queue.emp:80-92 down
  to a present-tense contract fact.
- Sound-API tails into `Sound_PostByte () preserves(sr)` — `Sound_Ping`,
  `Sound_PlaySample`, `Sound_StopMusic`, `Sound_SetTempo`, `Sound_FadeOut`,
  `Sound_FadeIn` (sound_api.emp:160-439): `+ preserves(sr.mask)` each, IF the
  body slice proves clean; their "SR restored" prose becomes checked.
- `Sound_PlayRing () clobbers(d0)` → `jbra Sound_PlaySFX` (preserves(d1/a0)):
  the REGISTER credit's first customer — `+ preserves(a0)` (and `d1` if the
  body proves clean), making sound_api.emp:377-379's prose argument checked.
- Census the rest of the corpus for tail-into-preserving-callee procs beyond
  these (animate.emp/games jbra sites mostly tail into clobber-heavy targets —
  expect nothing, but MEASURE and record the count).

## 5 · What this parcel must NOT do

- No stack-balance Defer charge (row 2099 stays OPEN; `BalanceObserver`
  untouched).
- No partial-width preserves model (row 2142 stays OPEN; if a candidate site
  fails on width, record it on the row).
- No `Edge::TailOut`/`BranchOut` split (row 2147's shape — consult targets at
  the consumption point, per the noreturn model's "property, not edge kind").
- No CCR-through-tail credit (§2.4), no new syntax, no annotation for the
  credit — it is inference over declared contracts.

## 6 · Bars

Byte bar seven targets identical (contract text and checker changes only).
Full strict with closing arithmetic; warn tiers id-identical ×7 (preserves is
error-tier; if any adoption moves a warn id, that is a STOP-and-report).
Tests, paired fires/holds per convention (z80_contracts.rs style, specific
diagnostic id + register token in every assertion):
- the 68k vacuity regression pair — tail-only proc with a false `preserves`
  FIRES (pre-fix it passes; state that in the test comment as the watched
  fail), honest tail-only proc HOLDS;
- credit pair — tail to preserving callee HOLDS, tail to clobbering callee
  FIRES;
- `@noreturn`-target Defer carries no obligation (pair with a non-noreturn
  control);
- `falls_into` empty-body + FallOff pairs (68k mirror of the Psg_EmitDivisor
  pins);
- sr.mask-through-tail pair (QueueDMA shape synthetic + the corpus line as
  live witness);
- indirect/computed tail preserves nothing (negative control).
Ledger: rows 2079 + 2088 CLOSED with the adoption counts; any width or refusal
findings appended to their standing rows, not new ones.
