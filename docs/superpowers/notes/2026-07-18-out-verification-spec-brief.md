# out()-verification arc ("G4.5") — build spec brief

**Overseer-authored, 2026-07-18.** Phase-1 item #1 of the pre-t18 roadmap. Gates the
WARN→ERROR flip (roadmap #4) and pass-3. Scope ratified by Volence: verify `out()`
now (error tier), build the edge-sensitive return-path split ONCE, share it with
item #2 (edge-sensitive conditional-out crediting), and close the latent D1c `!cc`
silent-miss in the same coupled area.

> **Checkpoint-A ruling (2026-07-18): #1 ACTIVELY verifies the conditional out too (ruling
> A).** The "uncond now" phrasing elsewhere meant the load-bearing must-def/§6-consumed outs
> are the primary target — NOT that conditional outs are skipped. #1 builds a small,
> ⊤-on-unknown cc-abstract dataflow (moveq immediate-fold + branch-split; every other
> cc-write → ⊤) and actively accepts the corpus's one conditional out
> (`FindStagedBlock out(a1 if eq)`), not a vacuous skip. Callee-side polarity: the obligation
> holds at a return UNLESS cc is provably-not-the-out-condition (unknown ⇒ obligation ⇒ fire
> if unproduced) — sound. The cc lattice + transfer is the reusable piece item #2 consumes
> caller-side; #2's callee-cc→caller-edge mapping is DISTINCT and not built here. Guarded by a
> trap test: a conditional out unproduced on a ⊤-cc return must FIRE, and a mutation treating
> ⊤ as provably-false must make that test go green.

Off masters: **sigil `9237963`, aeon `e55d7f1`** (G4 merged). Byte-neutral (lint/metadata
only — zero codegen change).

> **CORRECTED 2026-07-18 after a pre-code adversarial soundness review** (5 findings, 3
> would defeat the checker; the two mechanism claims re-verified in code). The v1 "reuse
> `must_defined_in`" plan was UNSOUND in two ways — that function is **width-blind**
> (`instr_written_regs`, `lower/proc.rs:509`, credits a `.w`/`.b` dest write and auto-inc
> bases as a full definition) and **param-seeded** (credits an out register that is merely a
> passed-through param). Neither means "produced a full-width live-out value." The corrected
> mechanism (§3) is a **width-aware, no-param-seed, `Edge::Defer`-checking sibling of
> `preserves.rs`** — the true structural dual — sharing only the `call_unconditional_outs` /
> `callee_uncond_out` map with the consumers (that is the anti-drift primitive, not the
> def-set analysis). Review + adjudication live in the campaign log tail; findings folded
> below.

---

## 1. The problem (the trust seam, verified)

`corpus_contracts.rs:168-169` builds `callee_out` directly from `node.out`, which is
`expand_reglist_regs(p.out)` (`corpus_contracts.rs:443`) — **declared source text, never
checked against the proc body.** That map + its `callee_uncond_out` subset (`:179-190`)
feed two ERROR gates through the shared `call_unconditional_outs` primitive
(`calls.rs:71`): must-def credits uncond `out()` as a *definition* (`calls.rs:158-160`),
and §6 `[call.result-invalid-path]` credits it as a taint-killing redefine
(`flag_check.rs:466`). Nothing proves the callee actually produces what it claims.

Existence proof the trust is misplaced: `TileCache_FindStagedBlock` shipped `out(a1)` that
was really `out(a1 if eq)` (hand-corrected during G4 Stage B). A verifier must
independently reject the unconditional form and accept only the conditional one.

## 2. The property to verify

For a proc `P` declaring `out(rN)`, `rN` is *produced* on a return path iff, at that return,
`rN` holds a **full-width value written on this pass through P**, sourced by one of:

- a **full-width (`.l`) body write** of `rN` (a `.w`/`.b` write does NOT verify — it leaves
  the high word stale, exactly `preserves.rs`'s `is_long` rule, `:340,400`; **Finding 1**);
- a **callee's UNCONDITIONAL `out(rN)`** at a `jsr`/`jbsr`/`bsr` on this path (the
  Load_Object←AllocDynamic shape), credited via the shared map;
- a **tail-transfer target's UNCONDITIONAL `out(rN)`** at an `Edge::Defer` (`jbra`/`jmp`/
  `bra` to a known proc) — a tail transfer IS a return of P from the caller's view, so it is
  a required return path (**Finding 3**). An unresolved/external tail target ⇒ cannot verify
  ⇒ FIRE.

**A param is NOT a production.** Do not seed the def set with `P`'s params (unlike
`must_defined_in`, which correctly seeds them for input-coverage). An `out(rN)` where `rN` is
`P`'s own param must be produced by an actual write/callee-out on every path, or it fires —
this catches a cursor `out(a4)` that is un-advanced on an early-exit path (**Finding 2**). A
genuine passthrough-out is a mislabeled `preserves` and SHOULD fire for adjudication.

Then:

- **Unconditional `out(rN)`**: `rN` produced on EVERY return path (every `Edge::Abandon` AND
  every `Edge::Defer`).
- **Conditional `out(rN if cc)`**: `rN` produced on every return path reached via the
  `cc`-SUCCESS edge. `!cc` return paths carry no production obligation.

This is a MUST analysis (produced-on-all-required-paths = intersection). Soundness polarity:
the consumers treat a verified out as a POSITIVE fact ("rN holds a produced value after the
call"). A dishonest out ⇒ must-def falsely credits rN defined ⇒ **D1b false NEGATIVE** (the
dangerous direction). Therefore the verifier must **only bless a proven full-width
production; when in doubt, FIRE.** Never over-approximate "produced."

**Property boundary (Finding 5, document it):** this proves `rN` holds a full-width value
*produced on this pass*, NOT that the value is *semantically correct*. A proc that produces
`rN` then stomps it with an unrelated value before `rts` still verifies (the stomped write is
itself a production). Value-provenance is out of scope — no definedness analysis models it.

## 3. Mechanism — a width-aware sibling of `preserves`, NOT a `must_defined_in` reuse

Build a forward MUST-produce dataflow over the SAME lightweight `Cfg`
(`flag_check::Cfg`/`Edge`) `preserves.rs` and `calls.rs` already use. Model it on
`preserves::verify_preserved` (the true structural dual — a per-register value property proven
on every return path), NOT on `must_defined_in`:

- **Width-aware** (Finding 1): only a full-width (`.l`) write of an address/data register
  produces it; a `.w`/`.b` write does not. Use `instr_size` exactly as `preserves.rs:340`.
- **No param seed** (Finding 2): entry state credits NOTHING; production must come from a
  write/callee-out/tail-out on the path.
- **Callee-out credit** at `jsr`/`jbsr`/`bsr` via `call_unconditional_outs` (the shared map).
- **Defer credit** at `Edge::Defer` (Finding 3): if the tail target is a known proc, credit
  its UNCONDITIONAL out; else the edge is an unverifiable return ⇒ that out register fails.
  Note `call_unconditional_outs` today returns `None` for tail mnemonics (`calls.rs:76`) — so
  this analysis needs its OWN Defer-target lookup into `callee_uncond_out`, not that helper.
- At every return (`Edge::Abandon` and `Edge::Defer`), each declared out register must be in
  the produced set; MUST-join across paths is intersection.

Emit **`[proc.out-unverified]`** at **error tier**, exactly like
`[proc.preserves-unverifiable]`. Do NOT introduce a warn window — the flip must not depend on
one, and the `preserves` precedent is error-tier for a declared-but-unprovable contract.

**Shared-primitive discipline (guardrail, mandatory):** what must not drift between the
callee-side producer check and the caller-side consumers (must-def, §6) is the definition of
*which registers a callee unconditionally outputs* — the `callee_uncond_out` map + the
`call_unconditional_outs` accessor. Both sides read that ONE map. The def-set/production
DATAFLOW legitimately differs (D1b wants width-blind, param-seeded definedness for
input-coverage; out-honesty wants width-aware, no-param production) — that is not drift, it is
two questions. Do not force-share the dataflow; DO force-share the map.

## 4. Edge-sensitive return-path split (shared with item #2)

Build the cc-conditioned return-edge analysis ONCE. Both of these consume it:

- **Callee-side (this item):** verify `out(rN if cc)` — `rN` produced on the cc-success
  return paths only.
- **D1c silent-miss (this item, the coupled close) — ruled the SIMPLE sound way (Finding
  4).** `destroys_value` (`calls.rs:239-250`) today returns `false` (suppresses the
  live-clobber firing) whenever `reg ∈ callee_out`, using the FULL `callee_out`. For a
  conditional `out(rN if cc)`, `rN` is trash on the `!cc` path AND the produced-result on the
  cc path — either way the caller's OLD held value in `rN` is destroyed on every path — so
  excusing it from the D1c firing is a **silent miss**. **Fix: switch `destroys_value` to read
  `callee_uncond_out` instead of the full `callee_out`** — only an UNCONDITIONAL out excuses a
  register (it is genuinely the produced result on all paths); a conditional-out register is
  treated as a clobber and D1c FIRES when it is held-live and read after the call. This needs
  NO cc-edge identification, so it does not ride `invalid_edge` (which bails to `None` and
  would silently degrade back to the miss — the review's Finding 4). It is false-positive-
  leaning: a caller that legitimately consumes the conditional result on the cc path may fire;
  today's only register-conditional out is `FindStagedBlock a1`, so the surface is tiny.
  Adjudicate any such firing; DEFER true cc-edge precision to item #2 only if the corpus shows
  real noise. (Rationale for the simple form over edge-precision: a sound close with a
  near-empty false-positive surface beats a precise close that silently no-ops on the
  `invalid_edge` bail. Prime directive: never trade the alarm for the miss.)
- **Caller-side crediting (item #2, next):** credit `out(rN if cc)` as a definition on its
  cc-success edge (the FillColumn→CopyBlockColumn a1 false alarm). Item #2 builds the cc-edge
  identification (and may then refine the D1c close above to edge-precise). #1 lands the
  callee-side verifier + the simple D1c close.

CFG note: the cc-guard rides `cond_callees` (`corpus_contracts.rs:114, 179-190`) and the
parser folds `out(rN if cc)`'s rN into `node.out`'s reglist — so the raw `node.out` already
contains conditional-out registers; `callee_uncond_out` subtracts them. The edge-sensitive
analysis needs the branch structure of `P`'s own body to know which return edges are
cc-success — use the existing `Cfg`/`Edge` machinery (`flag_check.rs`), do not invent a
second CFG.

## 5. Test matrix (mandatory — both directions + trap)

Per the guardrail directive for trusted-core changes:

1. **still-fires**: an unconditional `out(rN)` whose body does NOT produce rN on some return
   path ⇒ `[proc.out-unverified]` fires. Include a callee-sourced positive: `out(rN)` where
   rN comes from a callee `out(rN)` ⇒ verifies (does NOT fire) — the Load_Object shape.
2. **width (Finding 1)**: `out(d0)` produced only by a `.w`/`.b` write on some path ⇒ FIRES;
   the same body with a `.l` write ⇒ verifies. (Directly mirrors `preserves`'s `is_long`.)
3. **no-param-seed (Finding 2)**: a proc with param `a4` and `out(a4)` that advances a4 via
   `(a4)+` on the main path but early-exits (`beq`) BEFORE the advance on another ⇒ FIRES on
   the bail path; the version that advances on all paths ⇒ verifies.
4. **Defer (Finding 3)**: `out(a1)` produced by `jbra ProducesA1` where ProducesA1 declares
   `out(a1)` ⇒ verifies; the same tail to a proc that does NOT declare `out(a1)`, or to an
   unresolved/external symbol ⇒ FIRES.
5. **now-clears**: `TileCache_FindStagedBlock out(a1 if eq)` verifies; the unconditional
   `out(a1)` form on the same body FIRES (the existence-proof regression, both directions).
6. **D1c close (Finding 4)**: a synthetic caller holding a live value in `a1` across a
   conditional-out callee, read after the call ⇒ D1c NOW fires (was suppressed via full
   `callee_out`); the same caller across an UNCONDITIONAL-out callee still does NOT fire.
7. **trap/mutation**: prove each guard is LOAD-BEARING — the test suite must FAIL under a
   wrong impl. At minimum: (a) revert width-awareness (credit any width) ⇒ test 2 must break;
   (b) re-add the param seed ⇒ test 3 must break; (c) skip the Defer edge ⇒ test 4 must break;
   (d) revert `destroys_value` to full `callee_out` ⇒ test 6 must break. If any wrong impl
   stays green, that guard is not load-bearing.

## 6. Corpus adjudication + honest retrofit

Run over the aeon corpus (~24 out()-declaring procs; the tricky ones: `Load_Object out(a1)`
callee-sourced, `FindStagedBlock out(a1 if eq)` conditional, the `out(d5,a4)` SAT-cursor
procs where a4 is an in-out param). Expect a possible residue like the a0 case — that is the
CHECKPOINT, not a failure. **STOP and report any residue for adjudication; do NOT
self-clear.** Retrofit rule (non-negotiable, the campaign standard): a true-but-unverifiable
contract makes THE VERIFIER GROW, or the label is corrected to the honest form — **never lie
the label** (no removing a real out(), no fake unconditional→conditional to dodge a firing
unless the code genuinely is conditional and you can prove it).

## 7. Gates (bisectable, byte-neutral)

- Each mechanism change is its OWN bisectable commit (verifier / D1c-edge / retrofit / any
  flip-pin) — the campaign's commit hygiene.
- Byte-neutral: contract/lint metadata only. Prove at ROM level — canonical CRCs EXACT both
  shapes: plain `8984e510`/`453533`, debug `c80465dc`/`461554`. Not just workspace strict.
- Paired strict from tips (failures-first, explicit pass/fail counts), both critical pins
  green (`corpus_has_zero_dropped_instructions`, `corpus_closure_residue_is_empty_the_error_gate`).
- clippy clean.

## 8. STOP-don't-bank forks for the implementer

Surface these to the overseer; do not pick silently:

- **Param-passthrough out** — a proc declaring `out(rN)` where rN is its own param never
  re-written now FIRES (no param seed, §2/§3). That is intended (it is a mislabeled
  `preserves`). If a firing looks like a legitimate passthrough-out convention, STOP and
  report it for a ruling — do NOT silence it by re-adding the seed.
- **Callee-sourced out through a NON-uncond callee** — if a proc's out is sourced from a
  callee's CONDITIONAL out, the production is itself conditional; adjudicate, don't assume.
- **D1c false positives from the simple close (§4)** — report the count. If a legitimate
  conditional-result consumer fires and the noise is real, that is the trigger to pull item
  #2's cc-edge precision forward — a ruling, not a silent revert to full `callee_out`.
- **Any residue at all** — the a0-case discipline: report the firing list + per-firing
  adjudication BEFORE any retrofit or flip. A true-but-unverifiable out ⇒ the VERIFIER GROWS
  (or the label is corrected to its honest form) — NEVER lie the label.

## References
- Roadmap: `docs/superpowers/notes/pre-t18-roadmap.md` (Phase 1 item #1).
- Consumers: `crates/sigil-frontend-emp/src/calls.rs`, `flag_check.rs`, `corpus_contracts.rs`.
- Reuse target / precedent: `crates/sigil-frontend-emp/src/preserves.rs`.
- Campaign log: `spec2-progress.md` (memory; the G4 Stage-B tail entry names this arc).
