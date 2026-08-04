# Track B′ — the contract DELTA spec (what contract-grammar v2 does not already do)

**Fable, 2026-08-04. SUPERSEDES `2026-08-03-contract-unification-spec.md` as the
Track B work order.** The 2026-08-03 spec was written against a stale premise
(D2.32/D2.35 as "syntactic slices," S2-D6/D7 "unbuilt"); the overseer's state
audit (`notes/2026-08-04-finish-line-state-audit.md`) found **contract-grammar
v2** already ships most of it, and my own verification this session confirmed
and extended that finding. The old spec REMAINS VALID as the design rationale
and vocabulary (facets, composition, tier philosophy) — cited below as
"U-spec §N" — but its parcel list is dead. This document is the buildable
remainder, verified own-run against the code.

## §0 — The verified baseline (do not rebuild any of this)

| U-spec parcel | Status | Evidence (verified 2026-08-04) |
|---|---|---|
| P1 engine, write sets, transitive propagation | **SHIPPED** | `closure::compute_closure` over `lower::proc_written_registers`; `corpus_contracts.rs` whole-corpus walk |
| P1 caller liveness | **SHIPPED** | `[call.live-clobbered]` (D1c) + `[call.input-undefined]` (D1b), `calls.rs`, real CFG joins |
| P1 blind-call surface | **SHIPPED** (different spelling) | `jsr (aN) as Type` indirect-site bounds; `None` = ⊤ |
| P2 dataflow preserves | **SHIPPED** | `preserves.rs` forward dataflow (movem, single-reg, stack-slot idioms, "or never written") + `z80_preserves.rs` |
| P2 never-written-as-proof (t24 ask) | **SHIPPED** | `preserves.rs:5` — proof by absence of writes |
| P2 callee-preserves crediting (t30 G2 STOP) | **SHIPPED** | `preserves.rs:45-60` "CALLEE-PRESERVES ORACLE (§5)" — credits only closure-proven callee preserves, cycle-safe |
| P4 CCR | **SHIPPED** | `flag_check::{check_flag_unused, check_result_invalid_path}` + the shared `Cfg` |
| P4 conditional out | **SHIPPED** | `ast.rs` `out(rN if cc)` (`out_cond`, distinct from the unconditional set) + `out_verify::CondOutMap`; flag results `out(carry: name)` too |
| P3's hazard *instance* | **SHIPPED** (inference tier) | `[bus.*]` in `z80_bus.rs`: Stopped/Running/Unknown lattice, MUST dataflow, fixpoint, zero-FP stance, resolved-operand keying |
| Dead-save analysis | **SHIPPED** (unasked-for bonus) | `preserves::find_dead_saves`, `callee_clobbers` shared convention |

t24's third ask (conditional out coexisting with `clobbers` on the same
register — the AllocDynamic shape) is LIKELY closed by construction:
`[proc.out-clobbers-overlap]` iterates the UNCONDITIONAL `out` set and
`out_cond` is a separate field. **B′-0 pins this instead of assuming it.**

## §1 — B′-0: the AllocDynamic pinning test (trivial, do first)

A regression test asserting `proc f() clobbers(d0, a1) out(a1 if eq)` compiles
clean (no `[proc.out-clobbers-overlap]`), plus the negative: unconditional
`out(a1)` + `clobbers(a1)` still errors. If the positive FAILS, the relax is a
one-sitter scoped exactly by the t24 row (fire the overlap only on the
unconditional set) — do it in the same parcel. Then update the honest contract
at the real site: `core.emp`'s `AllocDynamic` gains `clobbers(d0, a1) out(a1 if
eq)` and drops the failure-edge site comment that stood in for it (byte-neutral
— contracts are metadata). Close the gap-ledger t24 row (~:1603) same commit.

## §2 — B′-1: generalized contexts (the surviving headline)

U-spec §3 stands as written EXCEPT re-scoped as an EXTENSION of the shipped
machinery, not a new engine:

- The `context` item (acquired / granted), `with <ctx> { }` brackets with the
  escape/entry-skip/reacquire proofs, and `requires(...)` propagation — all per
  U-spec §3.1–3.3, unchanged surface.
- **Implementation base:** `z80_bus.rs`'s CFG dataflow is the pattern AND the
  substrate — a declared context generalizes its lattice (per-context
  Held/NotHeld/Unknown), and `closure.rs` carries `requires` residues exactly
  as it carries register effects today. The `[bus.*]` net STAYS as the
  inference tier for the Z80/VDP instance; the bracket layer adds the DECLARED
  tier above it.
- **The declared tier closes the shipped net's recorded gap:** `z80_bus.rs`'s
  zero-FP stance seeds proc entry as `Unknown`, deliberately not flagging an
  unpaired toggle at the top of a proc. Inside a `with z80_stopped {}` region
  the state is DECLARED, not inferred — the bracket makes the entry state
  definite, so the pairing check is total where brackets are adopted. State
  this in the parcel packet: it is the concrete soundness win that justifies
  the construct beyond ergonomics.
- Corpus adoption per U-spec §8-P3 (the ~10 `z80_bus` consumers, `stop_z80`/
  `start_z80` go non-`pub`, the t21 `z80_held` demand row discharged),
  byte-neutral ×6 against the CURRENT refreeze chain (33 at audit time — never
  a packet-quoted CRC).

## §3 — B′-2: stack-delta (S2-D7(b)) — verified absent, build

Per U-spec §4-stack: SP delta 0 on every path to `rts`, merging paths agree,
`link`/`unlk` paired. `[stack.unbalanced]`, `[stack.merge-mismatch]`. Builds on
the same `flag_check::Cfg`; `preserves.rs` already models stack slots and
linear deltas — reuse its delta tracking rather than writing a second one
(the module's own header describes the linear-delta model; extraction into a
shared helper is in-scope if the borrow is awkward). Error-tier, softened
under `@as_compat` per U-spec §6.

## §4 — B′-3: cycle budgets (S2-D7(c)) — verified absent, build

Per U-spec §4-cycles: `@budget(cycles: N)` worst-case-path ceiling +
`@cycles_exact` equal-cost proof. Substrate notes: per-instruction T-state
knowledge EXISTS on the Z80 side (`pad_to_cycles` — recently gained the
unconditional-`jr` arm and a DENSE mode); the 68k table is new work (the ISA
crate is where it belongs, next to the operand metadata). hblank ceiling +
DMA-window assert are the first consumers (ledgered). This is the largest B′
parcel; it may split 68k/Z80 if the porter finds the tables dominate.

## §5 — B′-4: report + F disposition

- `--report contracts` per U-spec §7, EXTENDING `emp_contracts` (the existing
  driver already prints firings + boundary stats; promote it to the report
  surface, add derived-contract-as-annotation output and the grant/bracket
  census once B′-1 lands). Coordinate with T1 (RAM-map report) per the plan's
  whichever-lands-second-conforms rule.
- F (reg aliases / scratch splices) disposition unchanged: demand-gated,
  recorded park at close if no demand.

## §6 — Order and bars

B′-0 (trivial, immediately) → B′-1 → B′-2 ∥ B′-3 → B′-4. All byte-neutral ×6
against the current chain; strict suite failures-first with counts; the
standing porter-loop bars; era lens panel on merge. U-spec §6's tier map
governs new diagnostics.
