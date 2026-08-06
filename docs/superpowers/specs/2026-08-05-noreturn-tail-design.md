# The noreturn-tail model — `@noreturn` + authored divergence (2026-08-05)

Status: RULED (Fable). Serves the three ledgered analysis consumers (B′-0b
out-cond transitive charge · B′-2 stack-delta tail charge · cycle_budget's
unbounded-transfer) plus the ccr_bracket_refusal advisory enablement (bare-sr
packet's four-item list). Scout ground truth 2026-08-05: the `attrs` channel
already parses `@`-attributes on proc decls; three analyses already consume
per-target symbol maps; the desugar's rail items already carry
`ItemAuthor::AssertDesugar`; the Abandon family is CLOSED (edge-split), and
Section_RedrawPlanes is NOT this parcel (see §6).

## 1 · Two mechanisms, one model

**(a) `@noreturn` on proc declarations** (and `extern proc` sigs), via the
existing attrs channel. It is a CHECKED claim, not an annotation: a
`@noreturn` proc body must contain NO `Return` edge and NO fall-off-the-end
(`[noreturn.returns]` refusal on any rts-class exit, conditional returns
included, and on `FallOff` — control off the end of the body "returns" into
the successor) — every path must leave by transfer or loop. Two refinements
the first panel round proved necessary: (a) an unconditional transfer to a
TRAILING local label (a label that closes the body with no instruction after
it) IS a fall-off in different spelling — the check must resolve it as such,
not accept it as an external `Defer` (the Z80 edge builder already models
this; the 68k `Cfg::edges` unification is a separate ledgered parcel with
cross-analysis blast radius); (b) `falls_into` composes: a `@noreturn` proc's
FallOff into a successor that is itself `@noreturn`-declared is honest and
accepted; into anything else it is refused. That check is cheap on the existing Cfg and makes
the attribute self-verifying at the declaration site; what cannot be verified
is the TRANSITIVE claim (the target it jumps to could return), which is
exactly as trusted as every other declared contract in the closure — the
compositional-trust argument the z80-parity packet already articulates.

**(b) Authored divergence for compiler-emitted rails.** The assert /
raise_error / raise_exception desugars end in `jmp (pages).l` into a blob
offset that has NO proc surface (`pub equ` — nothing to hang `@noreturn` on).
No corpus annotation is needed and none is added: the items carry
`ItemAuthor::AssertDesugar`, the compiler authored them knowing they diverge,
and per the codeitem-author §2 invariant the obligation REDIRECTS to the
author's own contract — which is precisely "this rail never returns."
Analyses treat an AssertDesugar-authored unconditional `jmp` as a divergent
terminal. No new item field, no new syntax; the author enum is the carrier.

A hand-written `jmp` to the same blob symbol stays a plain `Defer` — a human
claiming divergence spells `@noreturn` on a proc or accepts the conservative
treatment. The two mechanisms deliberately do not blur.

## 2 · Consumers wired in this parcel

1. **cycle_budget**: a `Defer` whose target is `@noreturn`-declared, or an
   AssertDesugar-authored terminal `jmp`, becomes a TERMINAL edge — charged
   its own instruction cost, then the path CLOSES (like Return). The failing
   arm of a DEBUG assert stops poisoning whole-proc budgets: an
   `@budget`-annotated proc with assert rails becomes measurable in DEBUG
   shapes. `[cycles.unbounded-transfer]` keeps firing for un-marked named
   tails — the refusal's message gains a pointer to `@noreturn`.
2. **ccr_bracket_refusal — the advisory, enabled** (the bare-sr packet's
   four items, all owed): (i) the walk stops refusing at divergent terminals
   (authored rails + `@noreturn` targets) — flags at a diverged exit are
   nobody's; (ii) the walk gains local-label awareness — a `bra`/`jbra` to a
   LOCAL label is intra-proc flow, not `Leaves` (scout: today it refuses on
   Section_RedrawPlanes' `jbra .pB_skip`-class jumps, which would FP on
   nearly every real proc); route it through the existing Cfg rather than
   growing a second local-label matcher; (iii) new warn id + DEBUG-shape
   warn-tier baseline rows (plain tiers must be UNCHANGED; state the new
   DEBUG counts and every proc named); (iv) the fixture audit the packet
   lists (lower_proc.rs unbalanced-SR fixtures, diag_assert_vector's
   `preserves(sr)` vector proc, diag_desugar parity fixtures) + detector
   tests. If the fixture audit uncovers a fixture that NEEDS the old
   behavior, that is a finding to report, not to paper over.
3. **B′-2 stack-delta charge**: with divergence marked, a named tail
   transfer to a NON-noreturn target is a real tail call and its stack delta
   becomes chargeable. CENSUS FIRST: enumerate every corpus push-then-tail
   site; if zero legitimate sites fire, enable the charge with a pin; if any
   legitimate arg-passing tail exists, leave the charge OFF and record the
   site in the ledger row (do not invent an annotation for it in this
   parcel).
4. **B′-0b out-cond transitive half**: implement per its own ledger row
   (2059) if the noreturn distinction is the only missing piece; otherwise
   record precisely what else is missing on the row.

## 3 · Adoption census (byte-neutral — attributes and analyses only)

`@noreturn` lands on: the 12 exception-vector stubs + the ErrorHandlerBlob
wrapper (error_handler.emp), `GameLoop`, `EntryPoint` — the scout's census,
every one already documented noreturn in prose. Their existing "nominal"
clobber contracts stay untouched in this parcel (the ledger-1068 widening
convention is a separate question; note it, don't churn it). The `[noreturn.
returns]` check must pass on all of them as-written — if any stub fails the
check, STOP and report.

## 4 · What this parcel must NOT do

- No `Cfg` edge-variant changes (Return/FallOff/Defer stay; divergence is a
  property consulted at consumption points, not a fourth edge kind — the
  edge-split packet owns that shape and closed it).
- No blob-offset proc surfaces, no `@noreturn` on `equ`.
- No preserves-through-tail-transfer credit (the QueueDMA sibling class) —
  that is the 68k mirror of z80_preserves' Defer arm, a separate ledgered
  lane; this parcel only leaves the ledger row corrected to say the credit
  mechanism exists on the Z80 side as precedent.

## 5 · Bars

Byte bar seven targets identical (attributes and warn-tier changes only —
plain tiers unchanged, DEBUG tiers gain ONLY the new advisory's rows, every
delta named). Full strict with closing arithmetic; negative probes for
`[noreturn.returns]` both polarities; terminal-edge budget pin (an
@budget + assert-rail fixture that refuses before and verifies after);
advisory detector pins. Ledger: noreturn row CLOSED; B′-0b/B′-2 rows updated
per §2.3/2.4 outcomes; the bare-sr enablement row CLOSED.

## 6 · Explicitly out of scope, routed elsewhere

Section_RedrawPlanes' honest flip (provable TODAY, no new machinery — scout
verified the mask slice passes) and the invoke call-edge closure fix
(checked-clobbers residue) ride the residue micro-parcel, not this lane.
