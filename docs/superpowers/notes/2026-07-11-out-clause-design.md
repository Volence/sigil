# `out(...)` — register-output contracts (S2-D6(e), design)

Volence-directed 2026-07-11 (grew out of the AllocEffect `clobbers(d0)` +
`a1`-output review). Completes the register-contract trichotomy.

## The gap it fills

A proc does one of three things to each register: **preserves** it
(untouched), **clobbers** it (destroyed scratch), or **returns** it (a
result the caller reads). Today `.emp` spells the first two:
- `clobbers(d0, a1)` — scratch (D-P4.9 lint: a write to a register ∉
  `clobbers ∪ params` is `[proc.clobber-undeclared]`, WARN, ACTIVE).
- `preserves(d0-d1/a0)` — untouched (D2.32, the S2-D6(b) syntactic slice).

There is no word for the third. So an OUTPUT register (`AllocEffect`'s `a1`
= the allocated slot) is written, isn't in `clobbers`, isn't a param — and
**already fires a live `[proc.clobber-undeclared]` warning today** (benign,
WARN-tier, so the byte gate passes, but it's real noise, and the contract
is a comment `// Out: a1 = …` that can drift). The AS twin's own header
proves the intent: `Out: a1 = …` / `Clobbers: d0` — a1 is deliberately NOT
a clobber.

## Decisions

**D-out.1 — keyword `out(...)`, plain register list.** `out(d0, a1)` —
comma-separated registers (mirrors `clobbers`, `Vec<String>`; NOT the
`preserves` movem-reglist form — outputs are named single registers, never
movem ranges). Composes freely with `clobbers`/`preserves`/`falls_into`.
Optional (like clobbers/preserves — a `None`/empty contract is legal).

**D-out.2 — semantics: the third partition member.** Registers in `out(...)`
are RESULTS: written by the proc, live-out, read by the caller. They join
`allowed` in `check_clobbers` → an output-register write is no longer
`[proc.clobber-undeclared]` (THE immediate win — silences AllocEffect.a1,
AllocDynamic.a1, and every other real output across the corpus).

**D-out.3 — the checks (extend the active lint; mirror `preserves`' tiers).**
- `[proc.out-unwritten]` (WARN) — an `out`-declared register never written
  on any path is a false claim (a stale `out()` after a refactor). Dual of
  clobber-undeclared.
- `[proc.out-clobbers-overlap]` / `[proc.out-preserves-overlap]` (ERROR) — a
  register in `out` AND (`clobbers` | `preserves`) is a contradiction
  (returned-and-scratch / returned-and-untouched). Mirrors
  `[proc.preserves-clobbers-overlap]`.
- Tier + `@as_compat`: `out` is a DECLARED contract (like `preserves`), so
  its overlap/unwritten checks are NOT silenced by `@as_compat`. (Under
  `@as_compat` the clobbers lint itself is off, so the "out silences
  clobber-undeclared" benefit is moot there — consistent.)

**D-out.4 — BYTE-NEUTRAL.** `out` is metadata: it changes NO codegen. This
is load-bearing — the whole corpus application must leave every byte gate
green (the ports stay byte-exact; only warnings disappear).

**D-out.5 — 68k + Z80.** Output registers are a general calling-convention
concept (unlike `preserves`' movem/`sp`, which is 68k-only). `out` applies
to both CPUs.

**D-out.6 — conditional outputs DEFERRED to S2-D7.** `AllocEffect`'s `a1` is
output-on-success, preserved-on-failure (gated by `Z`). The plain `out(a1)`
says "a1 is a result the caller reads (after checking the success flag)" —
accurate enough for the S2-D6 tier. The richer `out(a1) if z` form (and
linting call sites that read `a1` without first branching on the flag) is a
MACHINE-STATE contract — it rides S2-D7's CCR-liveness dataflow. Spec it as
the future extension; do NOT implement now. Keeps scope tight and the plain
form covers the demand.

## Non-goals (now)

- No dataflow/transitive-liveness (that's the full S2-D6 pass — the `out`
  set becomes an INPUT to it later: "these writes are results, propagate
  them as live-out through call chains").
- No `out` on `script`/`dispatch` bodies (procs only; scripts have their own
  resume-state model).
- No inference (an explicit clause when you want the guarantee/docs; absent
  otherwise — no ceremony tax, adoption tenet).

## Plan

1. **Parser + AST + check** (sigil-frontend-emp, TDD): `out: Option<Vec<String>>`
   on `ProcDecl`; parse the clause; extend `check_clobbers`' `allowed`;
   add out-unwritten + the two overlap checks. Byte-neutral.
2. **Spec** (empyrean SIGIL_SPEC2_LANGUAGE.md working tree, Volence's
   cadence — uncommitted amendment): decision D2.35 / S2-D6(e); §5.1 prose;
   §10 attribute inventory; the S2-D6 ledger row (the (b)/out slice shipped).
3. **Corpus application** (aeon engine .emp + sigil examples): add `out(...)`
   to every proc whose `// Out:` names a register. Re-verify byte gates green
   both shapes + strict workspace + the clobber-undeclared warnings on those
   procs are GONE. aeon side on a branch → merge; byte-neutral so no re-pin.
