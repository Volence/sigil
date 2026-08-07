# Follow-up lane brief — the `Collision_Probe*` cluster

Ruled 2026-08-07. Two steps, **in this order**. The second is not worth running
until the first is answered, because the first may dissolve most of the cluster.

Prepend `notes/porter-brief-boilerplate.md`. Note the two rules added this
session: `git checkout -- <file>` is forbidden in a lane worktree, and a lens
panel is dispatched only against a clean worktree with the review SHA named.

## What is actually true about the source — re-derived, not relayed

Two separate readings of `games/sonic4/player/player_sensors.emp` got this wrong
before it was checked, so the verified facts come first and any claim that
contradicts them is wrong regardless of who made it:

* `probe_core` contains **four** `rts`. `.done` is the **only proc-level
  return**. The other three are returns of the internal `.cell` subroutine,
  reached by `jbsr .cell` from inside the same template.
* The `moveq #16, d0` near the top is followed by `sub.w d3, d0` and **falls
  into `.done`**. It is not an exit.
* The genuine-miss sentinel is **32, not 16**: `.nothing` is
  `moveq #32, d0 / moveq #0, d1 / moveq #0, d2 / jbra .done`. **A real miss
  writes all three registers.** So the "both-miss tie returns a stale angle"
  story is false as stated — a both-miss returns zeros.
* `.cl_hanging` ends `moveq #16, d0 / rts` — a **`.cell` return**, with `d1`/`d2`
  unwritten on that arm. Its caller tests `cmpi.w #16, d0 / beq .full_back`, and
  `.full_back` pushes `d1`/`d2` as the primary angle/attr.

## STEP 1 — the verifier hypothesis. Confirm or refute FIRST

`[proc.out-unverified]` fires `out(d0)` for all four `Collision_Probe*` procs
even though `d0` is written on every proc-level path. The named hypothesis:

> the verifier treats a LOCAL-SUBROUTINE `rts` (the `rts` ending a `.cell`
> reached by `jbsr .local` inside a proc body) as a PROC-LEVEL return, because it
> has no local-call model.

Confirm or refute it against `out_verify.rs` and the CFG builder. If confirmed it
is a **new model-gap class**, and the sequence is:

1. Fix if tractable; otherwise pin it with a ledger row that names the class.
2. **Re-run the 30-row residue afterwards** and report how much of the cluster it
   dissolves. The `d1`/`d2` rows may be inflated by the same hole, so their
   classification is not settled until this number is known.

Do not classify anything else in the cluster before this step reports.

## STEP 2 — the behavioural trace, only after step 1

Force the hanging case in the oracle and trace whether the unwritten `d1`/`d2`
escape `.cl_hanging` → `.full_back` past `Player_SensorFloor`'s angle-resolution
policy and the callers' distance gating into SST state.

Decides between: **real bug** (fix = the `.cl_hanging` arm writes all three;
byte-changing, so full A/B + refreeze), **benign by downstream filter**
(document + redesign the contract), or **contract-only**.

Drive notes: the standing anchored recipe is in
`notes/2026-08-06-migmask-ab.md`; the instrument traps that cost real time are in
`notes/2026-08-07-mulw-parallax-ab.md` — a `wait_for_break` can halt BEFORE the
breakpoint, and resuming with the PC on a breakpoint re-triggers it without
executing. Read a tick counter at two consecutive anchors before treating
anything as evidence.

## Gated behind BOTH results

The value-conditional-output language ask (`out(rN if rM != #K)`) stays
demand-gated. If step 1 dissolves the rows, or step 2 lands on "real bug", the
demand site disappears. Do not spec it until both have reported.
