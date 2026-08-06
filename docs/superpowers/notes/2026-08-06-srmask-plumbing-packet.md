# sr.mask plumbing lane — packet (2026-08-06)

Byte-NEUTRAL checker-plumbing parcel (three items). Bases: sigil `686e6f62` /
aeon `76013f2`. Aeon source untouched (checker-only).

## What shipped

### Item 1 — module-`invariant` union in `collect_sr_mask_preservers` (closes ledger row 2090)
`collect_sr_mask_preservers` (`lower/mod.rs`) now unions the module-scope
`invariant` into the mask-preserver membership: a module carrying
`invariant: preserves(sr)` / `preserves(sr.mask)` credits EVERY proc (and
`extern proc`) as a mask-preserver by §3.2 inheritance, even with no explicit
`preserves`. A new `module_invariant_covers_mask` reads the coverage from the raw
module attr form (`preserves(...)` args joined by `.`, matching the `sr` / `sr.mask`
spellings — `sr.ccr` excluded) — independent of the reglist expansion
`validate_module_invariants` runs, since SR is not a movem register.
Corpus-dead (no 68k module carries a mask invariant), so it credits nothing on the
frozen corpus.

Pins (unit, `lower::sr_mask_invariant_tests`): positive — both `sr`/`sr.mask`
spellings credit every proc; negative twins — an `sr.ccr` invariant and a
no-invariant module credit only self-declared procs. The self-declared `keeper` in
each negative is the non-vacuity guard.

### Item 2 — thread the REAL sets through the register-preserve oracle inputs
`verified_preserves_regs` and `preserve_oracle_inputs` (`lower/proc.rs`) took an
EMPTY `@noreturn` set and EMPTY sr-mask-preservers map with an inline justification
that the coupling was inert. Both now take the two sets as parameters and forward
them to `check_preserves`; the false justification comments are replaced with the
present-tense fact (real sets threaded, inertness measured not assumed). Real sets
supplied at all three call sites:
- primary path — `check_clobbers` gains the two params, passes `ctx.noreturn` /
  `ctx.sr_mask_preservers` (the same sets the primary `check_preserves` at
  `proc.rs:181` consumes).
- corpus path (`corpus_contracts.rs`) — a new per-file `PreserveInputs` bundle
  built in PASS 2 from `collect_noreturn_symbols` / `collect_sr_mask_preservers`
  (the SAME sources the primary `lower` checker uses), threaded through
  `collect_items` → `proc_node` to the `preserve_oracle_inputs` and
  `verified_preserves_regs` call sites.

MEASURED inertness: corpus verdicts byte-identical ×7 before/after (all seven ROM
CRCs matched the Step-0 seed exactly). The threading is inert on the frozen corpus,
proven per-shape, not assumed.

Pin (integration, `preserve_oracle_threading.rs`): `keeper` preserves `a0` AND
`sr.mask`, leaving through `jbra Sibling` where `Sibling` preserves the mask. With
the real map naming `Sibling` the oracle input carries `a0`; with an empty map the
mask-tail refuses (`[proc.preserves-sr-unbalanced]`) and the input collapses to
`(∅, ∅)` — the rot polarity the threading prevents.

### Item 3 — ledger rows 2089/2090
Row 2090 CLOSED (Item 1 above). Row 2089 (terminal-only sr.mask credit) sharpened
and its witness citation fixed (`jbra Parallax_Step5_Vscroll` is at
parallax.emp:534, not :190; proc line 345 correct). Stays OPEN.

## Bars (all own-run)
- Byte bar: seven targets byte-identical to the chain-48 goldens (fresh rebuild
  CRCs matched the Step-0 seed exactly; `refreeze --check` OK, chain len 48).
- Strict: `cargo test --workspace --release` — 3452 passed / 0 failed / 4 ignored.
  Closing arithmetic: 3452 + 4 = 3456 == tracked `#[test]` 3454 + the untracked new
  test file's 2 = 3456. Zero unexplained deltas.
- Clippy `--workspace --all-targets --release -D warnings`: exit 0, no Rust
  lints (only clownlzss-sys C vendor cc warnings).
- Warn-tier ID set identical ×7: `{module.path-mismatch, proc.undeclared-fallthrough,
  proc.out-unwritten, proc.clobber-undeclared}` (byte-identical builds ⇒ identical
  warnings; counts unchanged from the seed).
- Negative probes both polarities on every new pin; non-vacuity guards
  (self-declared `keeper`; the real-vs-empty map polarities differ by construction).

## Findings

### step-3 (design / retrospect)
- **The rot the Item-2 justification named is STRUCTURALLY confined to
  `preserve_oracle_inputs`, not `verified_preserves_regs`.** The census while
  building the pin found: for a proc that mask-claims through a TERMINAL external
  tail, the base `verified_preserves_regs` (ClobberAll) is tail-poisoned on EVERY
  register regardless of the mask verdict — the terminal Defer clobbers all under
  ClobberAll, so a0 is uncredited with the real map too. The register credit for a
  tailing proc comes only from the corpus oracle round, whose input is
  `preserve_oracle_inputs`; THAT is where the empty-map sr error zeroes an
  otherwise-deferrable credit. The pin encodes both facts (the rot pin on
  `preserve_oracle_inputs`, a corroborating pin proving the base path's
  tail-poison inertness). So the threading is inert on the base path for a deeper
  structural reason than the original comment stated, and load-bearing only on the
  oracle-inputs path.
- **Row 2089 mid-body extension = S2-D7 in disguise (census ruling, not attempted).**
  The register walk charges every `Edge::Defer` because it runs a full per-path
  dataflow (`verify_preserved` over all edges); the mask slice is terminal-only
  precisely because it has NO dataflow. Widening `terminal_external_tail` to
  mid-body tails would credit a leave without proving SR was restored on that path —
  re-opening the vacuity the terminal slice closes. The honest mid-body mask credit
  IS the S2-D7 mask-dataflow half. Row sharpened to say exactly this; not a cheap
  extension.

### step-5 (perf / hazard)
- **Per-file `collect_sr_mask_preservers` rebuild added to the corpus PASS 2 loop.**
  The corpus path now rebuilds the per-file mask-preservers map once per file (it
  re-evaluates each mask-preserving proc's body via `eval_proc_body` to compute
  safe-entry labels). Cost is bounded by the count of mask-preserving procs per
  file (near-zero on the frozen corpus — no per-proc O(n²) blowup, the map is built
  once per file and threaded, not per proc). Acceptable; flagged for the record.

### neither-bucket headline
- **Module-invariant representation mismatch.** A 68k module `invariant:
  preserves(sr)` does not survive `validate_module_invariants` today — SR is not a
  movem register, so `expand_reglist` reports `[contract.unknown-register]`. The
  Item-1 union deliberately reads the RAW attr form
  (`module_invariant_covers_mask`), so the mask-preserver membership recognizes the
  invariant by its spelling independent of that reglist path. This is why the union
  is corpus-dead AND why the pins unit-test `collect_sr_mask_preservers` directly
  rather than through `lower_module` (which would also emit the unknown-register
  error). If a 68k mask invariant ever becomes a real construct, `validate_module_invariants`
  will need SR-family recognition too — out of this parcel's scope, noted here.
