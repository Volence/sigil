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

---

## §7 — POST-B′-0 RULINGS (Fable, 2026-08-04, Volence-directed) — the panel's escalations

B′-0 executed (branch pair `bprime-0-condout`, sigil `9a1b1258` / aeon
`39dd927`, gate-green, unmerged). Its close packet §8 escalated a real cost:
the relax removed the only rule forcing `out(rN if cc)` to have one meaning,
so "destroyed on every edge" (register also in `clobbers`) and "survives the
¬cc edge" (register absent from `clobbers`) are now both legal and NEITHER is
verified. These rulings close that, plus the panel's ledgered holes.

### §7.1 — The semantics, now normative

`out(rN if cc)` is a TWO-part contract:

1. **On the cc edge:** rN carries a valid result (already verified by
   out_verify's edge-sensitive credit).
2. **On every ¬cc return path:** governed by `clobbers` membership —
   - rN **∈ clobbers**: no claim. rN is indeterminate on failure edges;
     callers save on every path. (The AllocDynamic shape.)
   - rN **∉ clobbers**: rN is PRESERVED (entry value intact) on every ¬cc
     return path — the exhaustive-license reading, now a CHECKED claim, not
     prose. (The AllocEffect shape.)

**A proc with NO `clobbers` clause at all makes no survives claim, and the gate
is silent** (RULED, Fable 2026-08-04 — previously inherited convention rather
than a ruling). Part 2 is `clobbers` MEMBERSHIP, and membership is only readable
against a set the author actually wrote; an absent clause states nothing about
the failure edges. Firing there would charge an obligation the author never
incurred — the same error polarity that rules out firing at an unclassifiable
cc (§7.2). It would also make `out(… if cc)` the language's first construct
whose presence MANDATES another annotation, pushing authors toward hastily
written wrong `clobbers` lists, which D2.32 holds to be worse than none. The
abuse path — dropping the clause to buy silence — is trapped instead by the
pinned `survives_claim_sites` assertion in the corpus test, so the escape cannot
be taken quietly. Pinned by `no_clobber_contract_means_no_survives_claim`.

### §7.2 — B′-0b: the survives-claim verifier (RULED: build the DUAL — lens C's option (b))

**Option (a) is REJECTED on polarity grounds:** requiring a cond-out register
in `clobbers` to be proven written on some ¬cc path would fire on
conservative contracts. Over-broad clobbers is pessimism, not unsoundness —
the house never forces a weaker claim to be sharpened.

**Option (b) is RULED IN, error tier:** a cond-out register ABSENT from
`clobbers` must be PROVABLE preserved on every ¬cc return path, by the same
proofs `preserves` accepts (save/restore round-trip, never-written, and the
callee-preserves oracle so a preserving call does not kill the proof).
Diagnostic: `[proc.out-cond-survives-unverifiable]`, mirroring
`[proc.preserves-unverifiable]` — same rationale ("a wrong contract is worse
than none"), same tier, NOT `@as_compat`-silenced. **Error tier is acceptable
precisely BECAUSE of B′-0:** the honest downgrade for an unprovable
survives-claim now exists and is free — add the register to `clobbers` and
keep `out(rN if cc)`. Before B′-0 there was no escape hatch; now the checker
and the relax form one coherent feature.

B′-0b parcel scope (one sitting, rides out_verify + preserves machinery):
- The verifier above (68k; the Z80 arm stays dead until `VALID_CCS` goes
  CPU-parametric — the ledger row's must-test instruction stands).
- `tile_cache.emp:130` `TileCache_FindStagedBlock` flips to the honest
  `clobbers(d3-d4/a1) out(a1 if eq)` — with the verifier landing, the current
  dishonest form FIRES (a1 is written before the probe on every path), so the
  corpus edit and the checker land together, and `calls.rs`'s documented D1c
  false positive at `TileCache_FillRow` dissolves.
- `AllocEffect` is the REQUIRED passing witness: its survives-claim must
  PROVE (pool test precedes the pop; a1 unwritten on the `.full` path). A
  test pins both directions (AllocEffect-shape passes, hoisted-pop shape
  fires).
- The register-keyed nit closes here: the B′-0 exemption re-keys on
  (register, has-unconditional-mention), so `out(a1, a1 if eq) clobbers(a1)`
  — a genuine unconditional contradiction — errors again.

### §7.3 — B′-0c: the closure-soundness batch (RULED: all GO, one sitting)

All mechanical, with rationale already recorded in the code or the panel rows:
1. `corpus_contracts::contract_type_bound` unions `expand_reglist_regs(sig.out)`
   (conditional included — conservative) into the returned effect, matching
   `extern_node`'s recorded reasoning. Kills the delete-a-load-bearing-save
   failure mode. Highest priority in the batch.
2. `resolve/contract.rs::contract_of_sig` subtracts `out_cond` registers from
   `out` — a bound promising `out(rN)` unconditionally is NOT satisfied by a
   conditional target.
3. `closure.rs::subcontract_violations` clobbers test becomes
   `target.clobbers ⊆ bound.clobbers ∪ bound.out` — a bound's own out
   licenses writing.
4. D1c gate teeth: `out_verify_corpus` asserts a baseline firing list
   (allowlisting the documented `Load_Object @ AllocDynamic :: a1` FP until
   B′-0b dissolves the class), so a live-clobbered regression fails the
   suite instead of printing.
5. `corpus_contracts.rs:292` canonicalizes cond names through the register
   file (the same raw-vs-canonical bug B′-0 fixed in `check_out`, on a
   shipping ERROR gate).
6. `ProcDecl::unconditional_outs()` accessor; all six subtract sites migrate.
   Closes the re-learned-six-times class structurally.

### §7.4 — Order amendment

B′-0 merges at the next quiet engine window (first in the queue). B′-0b and
B′-0c branch from the post-merge master and may run in parallel with each
other and with the D-batch / T lanes; both precede B′-1 (the contexts parcel
should land on a contract layer whose known holes are closed). Everything
remains byte-neutral ×6 against the then-current chain.
