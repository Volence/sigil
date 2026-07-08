# Overnight completion checkpoint — #8 + pitcher-plant tranche + #9 draft (2026-07-08, Fable)

The pre-authorized overnight scope (handoff `2026-07-08-spec2-plan7-item8-overnight-handoff.md`)
landed in full. NOTHING merged, nothing pushed. Two stacked, separately-checkpointable branches:

## Branch 1 — `plan7-item8-jbra-relaxation` (off master 6775a81, 12 commits, HEAD `7a2be7c`)
Whole-branch adversarial review: **CHECKPOINT-READY, zero defects.**
- **Step-0 audit of the #6 merge** found 1 real DEFECT (cross-module bare-window overlays
  silently rebinding in the consumer's namespace — $8 vs $2 displacement) → fixed as
  pre-tasks: **definition-site binding** (`d9d4416`+`4eb941c`) + once-per-compile dedup of
  duplicate cross-module diagnostics (`57c53b3`). Byte auditor: clean.
- **Core `Fragment::RelaxLadder`** (`e6c515f`+`53e5f99`): ordered complete-encoding
  candidates, reach derived from FixupKind (PcRel8 i8∧disp≠0 / PcRelDisp16 i16 / Abs16Be
  asl-rule / Abs32Be always), grow-only rung fixpoint (old fragments byte-identical),
  convergence-sweep `[branch.out-of-reach]` with signed distance, PcRel8 disp-0 link guard.
- **`jbra`/`jbsr`** (`b3d63c3`): 4-rung ladders (bra.s→bra.w→jmp abs.w→abs.l), emp-only
  (AS frontend still rejects — tested), `[jbra.sized]`/`[jbra.label-only]`/`[branch.non-68k]`,
  jbra = fallthrough terminator, template hygiene proven per-instantiation by bytes.
- **Unsized branch relaxation** (`e8e6b4f`+`e76daeb`): all Bcc + bra/bsr relax .s↔.w in
  non-@as_compat (candidates from the SAME encoder as sized pins); @as_compat keeps
  `[branch.missing-size]` verbatim; out-of-reach message steers both classes.
- **Exhibit** `examples/reach_branches.emp` + full-image byte pin (`c71156a`, 250 B,
  independently re-derived by the controller).
- Design doc + erratum (AS DOES emit PcRel8 fixups; guard safe on unencodability) + the
  **NOTE-1 ledger item: `here()` reads pre-relaxation baseline offsets** (pre-existing via
  JmpJsrSym, now routine; the `ensure_fatal(here() <= $9000)` budget idiom can under-detect
  by ≤ Σ(growth) — spec-or-fix decision for Volence).

## Branch 2 — `plan7-pitcher-plant-tranche` (stacked on #8 @c71156a; 14 commits, HEAD `8df5a5e`)
Whole-branch adversarial review: **CHECKPOINT-READY, zero defects.**
**The standing acceptance exhibit COMPILES END-TO-END:**
`sigil emp examples/game/badniks/pitcher_plant.emp --root examples/game --prelude prelude`
→ exit 0, zero diagnostics, **340 bytes — hand-derived FIRST, matched the compiler on the
first comparison**, pinned in `crates/sigil-cli/tests/pitcher_plant_acceptance.rs` with an
anti-echo corruption test.
- U1 bare directive-style statement calls (mnemonics win) + `Reg` params/splices.
- U2 named args (real gap: positional-after-named silently mis-bound — fixed; paren-only).
- U3 `Value::Label` label values (barewords/dotted; fixed qualified-string refs; label_ctx-gated).
- U4 `Item.field` BOTH halves — field-ADDRESS operands (SymOff→RelaxAbsSym Add targets,
  width follows the SUM) incl. cross-module type-only stub injection, and data-item comptime
  field VALUE reads (lazy, cycle-guarded; built because gap-analysis a4 was wrong — the
  mechanism never existed). Review-caught shadow defect fixed (local wins coherently).
- U5 the game corpus (prelude with Sst/ObjDef/types/enums/helpers/engine stubs; exhibit
  moved to badniks/; only sanctioned exhibit edit = the two vel/frame lines; corpus root
  amended to `examples/game/` — `--root examples` collides on 4 pre-existing `module m` files).
- U6 the 340-byte pin. U7 review clean.
- **#9 design DRAFT (`8df5a5e`, design ONLY per scope):** D9.1 dispatch inline bodies as
  anonymous-proc sugar; D9.2 `script`/`yield` coroutines lowering to hidden dispatch tables
  + a typed resume slot; D9.3 byte-command DSL deferred with a sound-migration gate;
  5 open questions for Volence before 9b.

## Gate discipline (held throughout)
Every commit: `cargo test --workspace --no-fail-fast` → exactly the 4 allowlisted
sigil-harness reds (upstream aeon strlen drift) + `cargo clippy --workspace --all-targets
-- -D warnings` clean. Strict TDD with recorded RED evidence per task; two-stage reviews on
all load-bearing tasks (every one SPEC-COMPLIANT/APPROVED, fold-ins committed); controller
independently re-verified the audit repro, both exhibits' bytes, and the acceptance run.

## Merge order for the checkpoint
Merge #8 first (`plan7-item8-jbra-relaxation`), then the tranche fast-forwards onto it
(stacked at c71156a; the only #8 commit it lacks is the docs-only `7a2be7c`).
Post-merge Fable spec work queued: lift D2.18 implementation notes + the D-PP decisions +
NOTE-1 + the expanded ledgers into `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`.
