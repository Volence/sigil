# The contract unification — one record, one pass (S2-D6 + S2-D7 + contexts)

**Status: FULL DRAFT (Fable, 2026-08-03) — porter-grade; awaiting Volence's slot
ruling + gate.** Supersedes the same-day skeleton. Origin: the 2026-08-03
memory-safety conversation (gap-ledger "Memory-safety design round"). Absorbs
deferred rows **S2-D6** and **S2-D7** (decided 2026-07-02, siblings sharing one
dataflow machinery) and adds the **contexts** facet (`requires` + brackets),
generalizing S2-D7(d)'s hard-coded pairing lints. Companion plan:
`2026-08-03-finish-line-plan.md` §2.

## §0 — Mission and shape

A proc has ONE **contract** — a per-proc record of every checked machine-level
fact — verified and propagated by ONE dataflow engine walking the call graph.
The FP north star (design, not surface): the contract is an effect row; each
facet is a monoid; propagation is one fold. Every facet is **erasing** — this
entire spec is lint/metadata layer, zero codegen, byte-neutral by construction.

Hard lines (inherited, non-negotiable):
- NEVER change instructions or add memory traffic. No spilling. Pressure is a
  compile error, not a transformation.
- No solver (tenet 7 bar): every check is dataflow over the CFG + call graph.
- `@as_compat` does NOT silence declared-contract violations (D2.32/D2.35
  precedent). It does soften *inferred-only* findings (§6 tier map).

## §1 — The contract record

```
Contract {
    clobbers:  RegSet          // scratch — destroyed
    preserves: RegSet          // untouched (saved/restored)
    out:       RegSet          // results — written, live-out
    stack:     Delta           // net SP change on every rts path (0 unless declared)
    ccr:       FlagFacts       // which flags are meaningful at exit (P4)
    requires:  CtxSet          // contexts that must be active at every call site
    cycles:    Option<Budget>  // worst-case path cost ceiling (P5)
}
```

Facet composition up the call graph (callee into caller, at each call site):
clobbers ∪ · preserves ∩ · out = live-out dataflow · stack Σ · ccr kill/gen ·
requires ∪ (minus contexts the caller's enclosing brackets discharge) · cycles Σ
along paths. One traversal computes all facets; a facet not yet built simply
isn't populated (parcels are independent).

Existing surface is UNCHANGED: `clobbers(...)`, `preserves(...)`, `out(...)`
keep their exact D2.32/D2.35 spellings, checks, and diagnostics. New surface in
this spec: `requires(...)`, the `context` item, `with <ctx> { }`, `grants(...)`,
`calls(...)`, `@budget`/`@cycles_exact` (P5, the S2-D7(c) spellings).

## §2 — Inference: declarations are assertions

Per S2-D6(a): the engine computes each proc's ACTUAL facet values from its
lowered instructions and propagates transitively. A written annotation is a
CHECKED ASSERTION against the computed truth — never the source of truth.

Declaration obligations (the only two places annotations are *required*):
1. **`pub` procs** must declare their register facets (`clobbers`/`preserves`/
   `out`) and any non-empty `requires`. Diagnostic on omission:
   `[contract.pub-undeclared]` (warn in P1, error from P2 — corpus adoption
   window). Rationale: exported contracts are documentation; the Haskell
   signatures-on-exports discipline.
2. **Blind computed calls** (§5).

Everything else may stay bare; `--report contracts` (§7) prints derived
contracts in copy-paste-able annotation form so authors can promote them.

## §3 — Contexts (the new facet)

### 3.1 Declaration — an item-level contextual opener (S2-D1 policy)

```
// Acquired context: compiler-owned bracket, acquire/release are Code-valued
// comptime exprs (typically existing comptime fns).
context z80_stopped {
    acquire = stop_z80()
    release = start_z80()
}

// Granted context: no acquire/release — entered by hardware or a dispatcher,
// asserted at a root proc via grants(...).
context vblank { granted }
```

`context` follows the contextual-item-opener rule (keyword only at item
position). Contexts are module-scoped names, `use`-importable, `pub`-able.
Engine declares `z80_stopped`, `ints_off` (acquired), `vblank` (granted);
games may declare their own. S2-D7(d)'s disableInts/enableInts pairing lint
SHIPS AS a declared context, not a bespoke check.

### 3.2 Introduction forms

**Acquired:** `with z80_stopped { ... }` — splices acquire before the body and
release after it, and PROVES the pairing:
- Every path through the body reaches the release. A branch, `rts`, or
  fallthrough that exits the region is `[context.escape]` (error). Backward
  branches within the region are fine; a loop label targeted from outside the
  region is `[context.entry-skip]` (error — entering mid-region skips acquire).
- Nesting is legal (inner brackets of other contexts); re-entering an
  already-active acquired context is `[context.reacquire]` (error by default —
  the Z80 bus request is not reentrant; a future `reentrant` context property
  can relax this if a real context needs it).
- The bracket splices the SAME bytes the manual pair does today — adoption of
  `with z80_stopped` over manual `stop_z80() ++ … ++ start_z80()` is
  byte-neutral, proven by the ×6 gate on the adoption parcel.

**Granted:** `proc VBlank_Entry() grants(vblank) { ... }` — the proc asserts
its body executes only inside the context. The grant is a TRUST ROOT (the
assembler cannot verify hardware dispatch); it is greppable, auditable, and the
lens panels' business. `grants` on a proc reachable from a non-granting caller
is NOT an error (the assertion is about when it runs, not who names it) — but
`--report contracts` lists all grant roots for audit.

### 3.3 Requirement checking

`requires(ctx, ...)` on a proc means: every call site must be dominated by an
active bracket/grant for each ctx. Propagation: a caller inherits its callees'
requirements minus those discharged by its own enclosing brackets/grants; the
residue must appear in the caller's own `requires` (or the caller is `pub` and
must declare it — §2). An unsatisfied requirement at a call site is
`[context.unsatisfied]` (error, names the context, the call, and the nearest
discharging construct it found). Direct hardware access can be tied to contexts
later (e.g. Z80-RAM address-space ops requiring `z80_stopped`) — OUT of P3
scope, ledgered as a P3 follow-on so the parcel stays bounded.

## §4 — Register + machine-state facets (upgrades to shipped slices)

- **clobbers (P1):** computed write sets from lowered instructions, INCLUDING
  auto-inc/dec `(An)+`/`-(An)` address-register modification — this closes the
  D2.35-ledgered detection gap in both `[proc.clobber-undeclared]` and
  `out`-writability. Transitive propagation through `jsr`/`bsr`/`jbsr` and
  `dispatch` metadata (§5). Caller-liveness: a live register (written before
  the call, read after it on some path) clobbered by the callee is
  `[contract.live-clobbered]` (error; the S2-D6(a) headline check). This is
  also the t24 panel's RE-SHAPED `save_across` ask — caller-side liveness in
  the contract layer, the check that finds the dead `d0` oversave and refuses
  an undersave, with the emitted save set left to the author (the derived-
  save-set construct was rejected there for making remote contracts
  byte-affecting; this spec honors that rejection).
- **preserves (P2):** dataflow-verified — arbitrary save idioms (single-reg
  `move`s, split movems, stack slots) accepted; the D2.32 literal-movem-pair
  restriction retires; `[proc.preserves-missing-pair]` retires with it. Two
  ledgered demands are REQUIREMENTS of this parcel, not nice-to-haves:
  (i) **never-written = verified-preserved** (t24 panel ask — the strongest
  possible proof; the write set already knows the answer); (ii) **callee-
  declared preserves are credited through calls** (the t30 G2 STOP blocker:
  `verify_preserved` conservatively clobbers all registers at any call, so a
  preserving call before `rts` makes the caller's own `preserves` unprovable —
  TestChurnObj_Main's contract cannot close today; the fix threads corpus
  contracts into the verifier, deferring — not erring — on unresolved synthetic
  callees in per-file lowering). Existing diagnostics keep their names.
- **out (P2/P4):** live-out propagation through call chains lands in P2; the
  conditional form `out(reg) if <cc>` lands in P4 (needs CCR) — and it MUST
  compose with `clobbers` on the same register: `clobbers(d0, a1) out(a1 if eq)`
  means "a1 is a result on the eq edge, destroyed scratch on every other" (the
  t24 AllocDynamic finding: the honest contract is rejected today by
  `[proc.out-clobbers-overlap]`; the overlap check relaxes to fire only on an
  UNconditional out+clobbers pair).
- **stack (P4):** S2-D7(b) — SP delta 0 on every path to `rts`, merging paths
  agree, `link`/`unlk` paired. `[stack.unbalanced]`, `[stack.merge-mismatch]`.
- **ccr (P4):** S2-D7(a) — a conditional branch whose flags come from an
  unintended intervening instruction (`move` sets flags; `movea`/`lea` don't)
  is `[ccr.stale-flags]` (warn-tier — the classic silent-insertion bug);
  `out(reg) if <cc>` + caller-side flag-check linting land here.
- **cycles (P5):** S2-D7(c) — `@budget(cycles: N)` errors when the worst-case
  path exceeds N; `@cycles_exact` proves equal-cost paths (retires hand-counted
  NOP pads); per-instruction cycle inlays ride the existing report machinery.
  hblank is the ledgered first consumer; DMA-window budget asserts
  (`dma_cycles < vblank_budget`) are the second.

## §5 — Computed calls: precise where declared, loud where blind

1. **`dispatch`/`offsets`-driven calls:** the construct declares its member set
   — propagation is the exact facet-join over all members. No annotation needed.
   This is the common case (the engine is table-less one-level dispatch, but
   dispatch-construct adoption from the L9/round work covers the ported tables).
2. **Bare computed calls** (`jsr (a1)` where a1's target set is unknown): the
   site must carry `calls(clobbers(...), requires(...))` — a site attribute
   declaring the assumed contract of whatever is called — or the explicit
   maximal barrier `calls(clobbers(all))`. An unannotated blind call is
   `[contract.blind-call]`: error in new-style files, warn under `@as_compat`
   (where the analysis then assumes clobbers(all), requires(∅), preserving
   soundness for register facets; requires(∅) at a blind call is the one
   knowingly-unsound assumption — recorded here, mitigated by the warn).
3. Aeon's SST-slot dispatch root (`movea.l d0,a1; jsr (a1)` in the object loop)
   gets a `calls(...)` annotation naming the object-code contract — one line,
   one site, and the whole object corpus inherits checking.

## §6 — Tier map

| Finding | Tier | `@as_compat` | `@allow` |
|---|---|---|---|
| Declared-contract violation (any facet) | error | no effect | no |
| `[contract.live-clobbered]` | error | softens to warn | yes, per-site |
| `[context.unsatisfied]` / `[context.escape]` / `[context.entry-skip]` / `[context.reacquire]` | error | no effect (contexts are always declared surface) | no |
| `[contract.blind-call]` | error | softens to warn | yes |
| `[contract.pub-undeclared]` | warn (P1) → error (P2+) | softens to warn | yes |
| `[ccr.stale-flags]` | warn | softens to off | yes |
| `[stack.*]` | error | softens to warn | yes |
| Budget overrun | error | n/a (new-style attr) | no |

### §6.1 — WHICH tier stops WHAT (footnote, 2026-08-07)

The table above gives the SEVERITY of a finding. It does not say what a finding
stops, and the two are independent. Enforcement is **two-tier**:

* **Per-file declared-contract checks are BUILD diagnostics.** They run during
  lowering, on one module at a time, and error tier stops the build. "The checker
  caught it, so the build stopped" is true here.
* **The corpus CLOSURE — out-verification, context closure, live-clobbered, the
  budget walks — needs the whole call graph** and cannot be computed from one
  file. It was therefore CI-only for its whole life: it stopped merges, never
  builds. **Since 2026-08-07 it runs on every build too** (`sigil build` invokes
  it before linking; aeon's `build.sh` defaults it on, `CONTRACTS=0` is the
  documented emergency hatch). So closure findings now gate the build as well —
  but by a different mechanism, and with different semantics, than the per-file
  tier.

**The closure gate is a RATCHET, not an assert-empty.** Two families carry a
frozen baseline; a firing outside it fails, and a baseline row that stops firing
fails as a stale pin (the narrowing direction is destructive — the same closure
feeds the dead-save walk). Families that are zero-firing are asserted empty with
no baseline at all. One copy of each baseline is shared by the gate and the CI
gates.

**Baselines can be shape-dependent and the pin must say so.** `[call.live-
clobbered]` fires 20 times in the plain shape family and 25 in the debug family
— debug-gated code the plain shapes never assemble. A flat baseline is wrong, not
merely coarse; the pin is shape-invariant rows plus a per-family addition.

**Suppression is two-class, and the table's `@allow` column is about the second
class only.** A violation of a DECLARED contract has no suppression flag — you
either meet the contract or change it. INFERRED-ONLY findings (`[stack.*]`,
live-clobbered, blind-call, …) take `@as_compat` softening and per-site `@allow`
by ratified design; that is what the two right-hand columns encode. Measured
2026-08-07: the corpus has exactly **one** `@allow` and **zero** `@as_compat`, so
both hatches are ratified surface with no adopters.

**Baselined residue is not a bug list.** A pinned row is one of three things, and
the ledger tracks which:

1. **Loose contract** — the declaration over-claims. Burn it down.
2. **Verifier-model gap** — the contract is right and the analysis cannot see it
   (e.g. a register advanced only through `(An)+`, which the write detection does
   not model). It stays pinned until the model grows; editing engine code to
   please a checker is barred.
3. **Language-surface gap** — the contract cannot be *expressed*. The declaration
   is as close as the surface allows, and closing it is a language question.

The `Collision_Probe*` cluster is under adjudication and spans categories; it is
cited here as an example of why the taxonomy has three buckets rather than two,
not as a settled classification.

## §7 — Reporting

`--report contracts`: every proc's DERIVED contract in valid annotation syntax
(promote-by-paste), grant roots, blind-call sites, per-context bracket census.
Rides the report machinery the RAM-map-report (T1 tooling) established.

## §8 — Parcels (each = one porter brief; standard bars per the plan §0)

- **P1 — engine + clobbers.** CFG + call-graph construction over lowered
  procs; write-set computation (incl. auto-inc/dec); transitive clobber
  propagation; `[contract.live-clobbered]`; `[contract.blind-call]` with
  `calls(...)` surface; `[contract.pub-undeclared]` at warn; `--report
  contracts` v1. Implementation anchor: the D2.32/D2.35 check sites show where
  proc-attribute checks hook lowering; the engine is a new pass over the same
  lowered artifacts ("Plan 4+ lowering provides instruction-level knowledge" —
  the ledger's own note). Corpus step: annotate the object-loop dispatch root
  (§5.3). Tests: derived-write-set unit vectors per addressing mode (incl.
  `(An)+`), a live-clobbered positive+negative pair, blind-call tiering, report
  golden. Byte-neutral ×6.
- **P2 — preserves/out dataflow.** Retire the movem-literal restriction;
  live-out propagation; `[contract.pub-undeclared]` to error + the corpus
  annotation sweep it forces (mechanical, porter does it in the same parcel).
  Tests: each historical preserves diagnostic re-proven under dataflow, a
  non-movem save idiom positive, out-through-tail-call.
- **P3 — contexts.** `context` item (acquired + granted), `with` bracket with
  escape/entry-skip/reacquire proofs, `requires` propagation,
  `[context.unsatisfied]`. Ledgered demand this parcel CLAIMS: the t21 panel's
  `z80_held(code)`/DMA-window bracket ask (demand 6 — vblank.emp hand-spells
  the paired arms four times; the `sr_masked` bracket-template class composes
  as nested `with` blocks). vblank.emp's `SOUND_DRIVER_ENABLED`-conditional
  arm shape is a named adoption case: the bracket must compose under a
  comptime `if` (whether the arms live inside or around the bracket is the
  porter's call, decided against bytes). Corpus step: engine declares
  `z80_stopped`/`ints_off`/`vblank`; the ~10 `z80_bus` consumer files adopt
  `with z80_stopped` (byte-neutral, ×6-proven); `stop_z80`/`start_z80` go
  non-`pub` (the bracket is the only door). Tests: pairing proofs (escape via
  bra/rts/fallthrough, mid-region entry), nesting, reacquire, requires
  propagation chain ×3 deep, granted-root discharge.
- **P4 — machine state.** Stack-delta + CCR facets; `out(reg) if <cc>` +
  caller flag-check lint; C2 of the Option spec lands here (guard-dominance).
  Tests: link/unlk pairing, merge mismatch, movea-doesn't-set-flags negative,
  the classic silent-insertion positive.
- **P5 — cycles.** `@budget`/`@cycles_exact`, 68k+Z80 cycle tables (Z80 tables
  exist in the DAC driver's hand-counted heritage — the spec is the
  authority, not the comments), path-cost worst-case over the P1 CFG.
  First consumers: hblank ceiling, DMA-window assert. Tests: known-cost
  golden procs both CPUs, over-budget error, equal-cost proof.
- **F — follow-on (gated):** S2-D6(c) `reg alias = a0` proc-local aliases;
  S2-D6(d) `scratch` splice kind ("any register not live here" — kills
  pass-registers-as-macro-params). Build only on splice demand during P1–P5
  adoption; otherwise record the park at Spec-2 close. Parked-not-planned
  stays: virtual `%tmp` registers.

## §9 — Out of scope (recorded)

VDP control-port write-sequence tracking (needs value analysis — S2-D7's own
exclusion), object-lifetime invariants (engine/Oracle territory), memory-region
read/write effect facets (a future spec if demand appears; the write-to-ROM
lint idea from the design round waits there), runtime enforcement of any kind.
