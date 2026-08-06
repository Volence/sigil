# sr.mask plumbing lane — packet (2026-08-06)

Byte-NEUTRAL checker-plumbing parcel. Bases: sigil `686e6f62` / aeon `76013f2`.
Aeon source untouched (checker-only).

## Headline: the lane's result is a CATCH, not a feature

The lane was briefed to build three things. The most valuable outcome is that one
of them — a corpus-dead convenience credit that looked like pure future-proofing —
was built, caught by the B/C lens panel, and **reverted in full**. It would have
shipped an **unbacked error-tier proof discharge into a gated surface**.

The reverted item unioned a module-scope `invariant: preserves(sr.mask)` into
`collect_sr_mask_preservers`. That map's membership is not a hint: a hit makes
`sr_mask_preservers_credit` return true, which **suppresses the error-tier
`[proc.preserves-sr-unbalanced]` obligation and returns early**, skipping even
`sr_writes_round_trip`. On 68k **nothing enforces a module invariant against any
body** — the Z80 arm forces every proc to prove it (`check.extend(invariant_regs)`),
the 68k arm never consults `invariant_regs` at all. So the union credited procs no
checker examines, replacing verified-then-trusted with trusted-outright: an
epistemic REGRESSION dressed as an extension. It also credited `extern proc`s — a
foreign symbol on a promise made by a module that does not own it, directly
contradicting `collect_z80_callee_preserves`' documented rule.

Two further facts make the catch load-bearing rather than academic:
- the `[contract.unknown-register]` accident that neutralises the construct in
  `lower` does **not** protect the corpus surface — `analyze_corpus_with_contracts`
  never calls `validate_module_invariants`, and that surface is gated by the
  warn_tier / contract_closure / dead_save corpus tests;
- the lane's own positive pin routed **around** the front end that rejects its
  input, so it stood guard over an unbacked credit, and nothing pinned the
  rejection — the accident holding it at bay could have been removed silently.

Credit must follow proof. Ledger row 2090 stays OPEN and now carries the full
rationale, the ruling against `extern proc` inheritance, and the **ordering trap**:
SR-family recognition in `validate_module_invariants` is a precondition, but
landing it alone would take the credit live with still no enforcement — enforcement
first, credit second, always gated check-then-credit.

## What shipped

### Item 2 — thread the REAL sets through the register-preserve oracle inputs (KEPT)
`verified_preserves_regs` and `preserve_oracle_inputs` (`lower/proc.rs`) took an
EMPTY `@noreturn` set and EMPTY sr-mask-preservers map behind an inline
justification that the coupling was inert. Both now take the two sets as parameters
and forward them to `check_preserves`; the false justification is replaced with the
honest scope. Real sets supplied at all three call sites:
- primary path — `check_clobbers` gains the two params, passing `ctx.noreturn` /
  `ctx.sr_mask_preservers` (the same sets the primary `check_preserves` consumes);
- corpus path — a per-file `PreserveInputs` bundle built in `analyze_corpus_with_contracts`
  PASS 2 from `collect_noreturn_symbols` / `collect_sr_mask_preservers`, threaded
  through `collect_items` → `proc_node`.

Lens C independently tried to make the real map manufacture a wrong REGISTER credit
and could not: the oracle round re-verifies every register through
`verify_preserved(CallPolicy::Oracle(..))`, so a threaded mask credit cannot invent
a register credit. The threading is sound.

MEASURED inertness: seven-target CRCs identical before/after (table below).

Pins (`preserve_oracle_threading.rs`, 3):
- `oracle_inputs_need_the_real_mask_preservers_map` — `keeper` preserves `a0` +
  `sr.mask` through `jbra Sibling`; real map keeps the `a0` oracle input, empty map
  collapses it to `(∅, ∅)` (the rot).
- `base_credit_needs_the_real_map_when_the_terminal_tail_is_dead` — the
  load-bearing shape: with the terminal `jbra` DEAD, real map credits `a0`, empty
  map discards it.
- `base_credit_is_tail_poisoned_when_the_terminal_tail_is_reachable` — the inert
  shape, asserted for its own reason with a non-vacuity guard proving the real map
  genuinely discharges the mask claim there (so the emptiness is ClobberAll poison,
  not a silent refusal).

### Item 3 — ledger row 2089 (KEPT)
Scope sharpened: the mid-body mask credit is the S2-D7 path-sensitive scope in
disguise, NOT a cheap `terminal_external_tail` widening. The register walk charges
every `Edge::Defer` because it runs a full per-path dataflow; the mask slice is
terminal-only because it has none. Widening the terminal probe to mid-body tails
would credit a leave without proving SR was restored on that path, re-opening the
vacuity the terminal slice closes. Witness citation fixed —
`jbra Parallax_Step5_Vscroll` is at parallax.emp:534, not :190 (proc line 345
correct; verified in-tree by Lens B). Row stays OPEN.

### Item 1 — REVERTED in full
Both the Proc arm and the ExternProc arm, the `module_invariant_covers_mask`
helper, and the three `sr_mask_invariant_tests` pins are gone.
`collect_sr_mask_preservers`' doc comment now records WHY membership is
declaration-only (credit discharges an error-tier obligation, so it must follow a
check that actually ran). The false "the two paths recognize the mask token by its
spelling" sentence is struck from both the code comment and the ledger row — on
68k **both** spellings are hard errors (`invariant_reg_name` is single-segment;
`expand_reglist`/`RegFile::M68k` has no `sr` unit).

## Bars (all own-run, post-revert)
- Byte bar: seven targets byte-identical to the chain-48 goldens (fresh-rebuild
  CRCs below match the pre-edit seed exactly); `refreeze --check` OK, chain len 48.
- Strict `cargo test --workspace --release`: 3450 passed / 0 failed / 4 ignored.
  Closing arithmetic: 3450 + 4 = 3454 == the branch's tracked `#[test]` total 3454.
  Against master's 3451: +3 (the `preserve_oracle_threading` pins) −3 (the reverted
  `sr_mask_invariant_tests`) +3 = 3454. Zero unexplained deltas.
- Clippy `--workspace --all-targets --release -D warnings`: exit 0.
- Warn-tier ID set identical ×7: `{module.path-mismatch, proc.undeclared-fallthrough,
  proc.out-unwritten, proc.clobber-undeclared}`, counts unchanged.
- Negative probes both polarities on every pin; non-vacuity guards in place.

| target | full CRC | anchor CRC |
|---|---|---|
| s4.bin | b5ffb094 | b5be8fef |
| s4.debug.bin | 57fd08f9 | 8448717a |
| demo.bin | cbddc142 | 1a72e3e0 |
| demo.debug.bin | b61f462d | efd16be5 |
| config_a | 61e4e78e | bf1dde89 |
| config_b | 07e3f465 | db32d41b |
| lean | b92cb485 | d32cee18 |

## Findings

### step-3 (design / retrospect)
- **A corpus-dead credit is not a free credit.** The union was justified to itself
  as "future-proofing with teeth, credits nothing today". Corpus-deadness made it
  *byte-safe*, which is not the same as *sound*, and it silenced the reviewer
  instinct that would otherwise have asked what the map member buys. The lesson
  generalises: for any map whose membership DISCHARGES an obligation, adding a
  member is a soundness change regardless of how many corpus sites it touches.
- **A pin that routes around the rejecting front end is a warning sign.** The
  Item-1 positive pin called `collect_sr_mask_preservers` directly because
  `lower_module` rejects `module m (invariant: preserves(sr))` with
  `[contract.unknown-register]`. That detour was reported in the first packet as an
  incidental note; it was in fact the tell that the construct had no supported
  surface. Treat "my pin cannot go through the front door" as a design smell.
- **Row 2089 mid-body extension = S2-D7 in disguise** (census ruling, not
  attempted) — see Item 3 above.

### step-5 (perf / hazard)
- **Per-file `collect_sr_mask_preservers` rebuild in the corpus PASS 2 loop.** The
  corpus path now builds the per-file mask-preservers map once per file, which
  re-evaluates each mask-preserving proc's body (`eval_proc_body`) to compute
  safe-entry labels. Cost scales with the count of mask-preserving procs per file.
  **Not bounded by measurement here** — the earlier packet's "near-zero" claim was
  only true because the construct is corpus-dead, which is not a bound on future
  corpora. Flagged for the record; measure if mask contracts proliferate.

### neither-bucket headline
- **My "structurally confined / tail-poisoned either way" claim was WRONG, and the
  panel refuted it with a shape I had not considered.** I asserted the threading
  could only matter on the oracle-inputs path because a terminal external tail
  poisons every register under `ClobberAll`. That holds only when the tail is
  REACHABLE. `terminal_external_tail` classifies on the last instruction regardless
  of reachability while `verify_preserved` is reachability-based, so a body whose
  terminal Defer is DEAD CODE verifies under the real map and refuses under the
  empty one. Confirmed by probe (real → `{a0}`, empty → `{}`) before rewriting.
  The old test's name asserted something it did not witness — both sides were empty
  for *different* reasons — and it is now split into an honest load-bearing pin plus
  a scoped inert pin with a non-vacuity guard.
