# here()-vs-relaxation fix — implementation notes

Design: docs/superpowers/specs/2026-07-08-spec2-plan7-here-relaxation-fix-design.md
Branch: plan7-here-relaxation-fix (worktree here-relaxation-fix)

Per-task RED evidence + implementer-choice decisions below.

## T1 — Value::LinkExpr variant + operator lifting + [here.provisional] error

RED evidence:
- `link_expr_tests` in eval/expr.rs: before `lift_to_link_expr`/`ast_binop_to_ir`
  existed the test module did not compile (`cannot find function lift_to_link_expr`),
  i.e. RED = compile failure. After impl: 3/3 green.
- The end-to-end provisional-refusal RED (a `LinkExpr` reaching if/rept/array-length
  producing `[here.provisional]` instead of the OLD generic "must be bool/int") is
  proven RED-first by the T6 acceptance integration tests (they FAIL on master
  because master's `here()` never yields a `LinkExpr` — it folds to a stale Int and
  the guard/steer silently succeeds).

Implementer choices:
- LinkExpr wraps `sigil_ir::expr::Expr` (i64 leaf). `lift_to_link_expr` range-checks
  i128→i64 on each Int operand; overflow / non-int → error prefixed `[here.provisional]`.
- Logical `!x` on a link value has no IR node → lifts to `x == 0` (neutral 0/1 truth).
- `&&`/`||` with a provisional operand cannot short-circuit → routed through the
  same non-short-circuit lift (`lift_binary`) building `LogAnd`/`LogOr`.
- Refusal choke point = `reject_if_provisional` wired into: if/while cond, for
  iterable, range bounds (rept), array-length/refinement bound (`eval_const_index`),
  `slice` bounds, `math.*/as.*` args. Emit path (width-1 / arithmetic-then-emit) is T3.

## T2 — provisional-position query + exact/provisional split + symbolic eval_here

RED evidence:
- tests/here_provisional.rs: with T2 stashed (T1 only), `provisional_here_as_array_
  length_refuses` + `provisional_here_in_if_condition_refuses` FAIL (here() folds to
  a stale Int, no [here.provisional]); `exact_here_still_folds_and_steers` PASSES.
  After T2: all 3 green. (verified via `git stash` round-trip)
- sigil-ir builder: `section_has_relaxable_flips_after_a_relaxable_fragment` —
  RED = the query didn't exist before this task.

Implementer choices:
- IrBuilder::section_has_relaxable() = any JmpJsrSym|RelaxAbsSym|RelaxLadder in the
  OPEN section. here() at current_offset() is provisional iff any relaxable precedes
  it (all relaxables are emitted before the data/guard item that queries here()).
- HerePos { base, anchor } threaded through eval_data_at/with_root/captures replacing
  the bare `here_base: Option<u32>`. anchor=None => exact (Value::Int, byte-identical);
  anchor=Some => provisional (Value::LinkExpr(Sym(anchor))).
- For a data item the provisional anchor is the item's OWN label (decl.name), defined
  at the item's start byte (D-H.3). Item-guard anonymous anchors are T4.
- eval_here: Some(_)+anchor => LinkExpr(Sym), sets here_used; Some(vma) => Int(vma).
- here_anchor_used()/#[allow(dead_code)] until T4 wires the guard anchor-minting.

## T3 — data-emitted plain here() -> SymRef via item label (D-H.3)

RED evidence:
- tests/here_provisional.rs::provisional_here_emits_item_final_vma_as_symref:
  disabling the lower_to_data LinkExpr interceptor -> the LinkExpr reaches
  lower_prim -> "expected int" lower error -> test panics (confirmed by patching
  out the interceptor and re-running). After T3: emits $00008004 (H's final VMA
  after jbra grows bra.s->bra.w), NOT the stale baseline $8002.
- u8-field and arithmetic-then-emit refusals: RED = no interceptor => generic
  "expected int" / silent, no [here.provisional].

Implementer choices:
- lower_to_data intercepts Value::LinkExpr before per-type lowering. A plain
  Sym(anchor) => Cell::SymRef { name: anchor, width: self_ref_width(ty) }; the
  D-P4.5 selection widths it (2->Abs16Be, 4->Abs32Be). width==1 => error.
- self_ref_width: Ptr=>4, Prim=>declared width, Newtype/Refined=>underlying, else
  a loud "needs u16/u32/pointer" error.
- A non-Sym residual tree (arithmetic-then-emit) => here_provisional_error (L-H.2).

## T4 — LinkAssert record + deferral + MsgPart interpolation + Module plumbing

RED evidence:
- tests/here_provisional.rs::provisional_item_guard_defers_and_mints_anchor:
  disabling the LinkExpr-cond defer dispatch in eval_guard -> the guard folds/
  passes at comptime, m.link_asserts empty, no __here$ anchor -> test FAILS
  (verified via python patch+restore; git checkout is NOT used on tracked files).
- exact_item_guard_does_not_defer proves the exact path is untouched (no defer).

Implementer choices:
- sigil-ir: new assert.rs — LinkAssert { cond: Expr, message: Vec<MsgPart>, fatal,
  span } + MsgPart { Text(String), Expr(ir::Expr) }. Module gains link_asserts.
- IrBuilder gains link_asserts + push_link_assert; finish() carries them.
- Evaluator gains link_asserts + take_link_asserts. eval_guard: a LinkExpr cond
  -> defer_guard: freeze message via interpolate_parts (comptime {expr} -> Text,
  LinkExpr {here()} -> Expr), push LinkAssert, return Unit.
- eval_item_guard now returns ItemGuardOutcome { cont, diags, link_asserts,
  anchor_used } and takes a HerePos. lower_item_guard (new, shared by top-level +
  section arms) mints the anonymous anchor __here$<module>$<n> (`$` unlexable),
  defines it on anchor_used, drains asserts. Counter threaded through
  lower_section_items.
- Data-item guards: eval_data_with_root now returns (buf, asserts, diags);
  lower_data_item drains them (anchor = item's own label).
- build_program returns (sections, link_asserts, diags); rename_module also
  canonicalizes each LinkAssert's cond + lazy message Exprs. Test/CLI callers
  updated to the 3-tuple (CLI's _link_asserts consumed in T5).
- D-H.7: deferred guards never stop lowering; only a comptime-exact fatal aborts.

## T5 — sigil-link assert checker + CLI wiring into BOTH link tails (D-H.6/D-H.7)

RED evidence:
- sigil-link lib tests link_assert_*: RED by non-existence (check_link_asserts /
  build_symbol_table / render_assert_message did not exist).
- End-to-end probe (manual): /tmp/budget_fail.emp (jbra + ensure_fatal(here() <=
  $8003)) exits 1 with "overran at 32772" ($8004 post-growth); budget_ok ($9000)
  exits 0. On master, here() folds to $8002 baseline so both would pass silently.

Implementer choices:
- check_link_asserts(resolved, stubs, asserts) rebuilds the post-relaxation symbol
  table (build_symbol_table, identical values to link()'s Pass 1 — D-H.6's
  contract) and folds each cond: nonzero=pass, 0=fail (render message, lazy Expr
  parts folded to final addresses), Fold::Poison=internal-contract error.
- link_to_image now runs the checker after link() on the resolved sections, folding
  failures into its Err. Both tails (link_sections no-map, link_rom map) share it.
- link_rom's error channel changed String -> Vec<Diagnostic> so deferred-guard
  failures render with spans (path:line:col) like the no-map tail; emit_rom region
  errors wrap as a null-span diagnostic.
- compile_emp (single-file) passes module.link_asserts; run_emp_program passes the
  concatenated build_program asserts to both tails.

## T6 — acceptance-list integration tests (design items 1-8, CLI level)

File: crates/sigil-cli/tests/here_relaxation_fix.rs (8 tests). Items 3+5 stay
unit-side in sigil-frontend-emp/tests/here_provisional.rs; item 6's byte pin is
ports.rs::example_guards_compiles (untouched).

RED evidence (verified by patching the eval_here provisional arm back to master
semantics — python patch + os.replace restore + touch; NOT git checkout):
- FAILED under master semantics: budget_guard_fails_at_link_after_growth,
  budget_guard_fails_via_cli_binary (item 1 — stale $8002 <= $8003 passes
  silently), deferred_message_mixes_frozen_text_and_final_address (item 4),
  deferred_fatal_does_not_suppress_later_items (item 8b — master's eager fatal
  aborts lowering, Tail never lowers).
- PASSED either way (positive controls, by design): item 2 mirror, item 7
  two-module no-collision, guards.emp zero-diagnostics, item 8a all-collected
  (its budget fails even at baseline, so master's eager path also reports both).

Notes:
- item 7 proves D-H.8 uniqueness end-to-end: two modules each mint
  __here$<module>$0; a collision would trip link()'s duplicate-label detection.
- item 8b splits the pipeline: lower/link succeed (Tail bytes AB CD present at
  s[2..4]) and check_link_asserts then fails the build — deferral never stops
  lowering (D-H.7).
- Process gotcha recorded: os.replace restores an OLD mtime; cargo reused the
  patched rlib until a `touch`. Future probes: touch after restore.

## Final gate (post-T6, 2026-07-08)

- cargo test --workspace --no-fail-fast: 1295 passed across 94 green suites;
  EXACTLY the 4 allowlisted sigil-harness reds (full_build_reproduces_sound_
  driver_regions, vector_table_matches_reference_rom_first_256_bytes,
  full_debug_rom_matches_assembled_reference, full_rom_matches_assembled_reference).
- cargo clippy --workspace --all-targets -- -D warnings: clean.
- pitcher_plant acceptance invocation: exit 0, zero diagnostics, "built: 340
  bytes"; -o output verified 340 bytes on disk.
- examples/guards.emp single-file: exit 0, zero diagnostics, 13 bytes (ports.rs
  byte pin unchanged).
- examples/ corpus sweep: dispatch/guards/offset_table/reach_branches/sst_overlay
  compile with unchanged results; composition_pitcher_plant.emp + main.emp fail
  single-file with PRE-EXISTING diagnostics (examples/ is git-identical to master
  d95c94b; the error classes involved are untouched by this branch). The
  master-vs-branch byte-diff probe proper is the review-time step per the design;
  the standing byte pins (ports.rs, jbra_relaxation.rs, pitcher_plant 340) all
  hold, and no corpus program has a provisional here().

## Review fold-in (post two-stage review, NOTE-1 + NOTE-2)

NOTE-1 — specific [here.provisional] at the remaining int-consumer sites.
RED evidence (tests written first; exact generic messages captured):
- byte(here()):        "`byte` expects an integer, got link-expr"
- bytes([here()]):     "`bytes` element must be an integer, got link-expr"
- (max_size: here()):  "`max_size` must be a comptime integer, got link-expr"
- vma: here():         already loud ("no current position" — attr eval carries no
  here_base, so a LinkExpr cannot form there); pinned as the representative test,
  and eval_attr_int is fronted anyway so a future position-threaded attribute
  cannot regress to the generic message.
Sites fronted with reject_if_provisional: builtins.rs (bytes element + the shared
single-int-arg helper serving byte()), literals.rs (bitfield field value),
call.rs (enum discriminant + eval_single_int_arg), sandbox.rs (embed/import
numeric arg), layout.rs (check_max_size + eval_attr_int), asm.rs (immediate,
displacement, operand splice via classify_operand_splice).

NOTE-2 — eval_data_captures: contract comment added recording the deliberate
LinkAssert drop (non-lowering callers, always here: None; the lowering pass
drains via eval_data_with_root).

NOTE-3 — interpolate/interpolate_parts lexer duplication: NOT refactored per
reviewer instruction (recorded as a ledger item on their side).

Post-fold-in gate: workspace tests = exactly the 4 allowlisted harness reds;
clippy -D warnings clean; pitcher_plant 340 bytes exit 0.

Process incident (recorded): the session's cwd reset to the MAIN checkout after a
reconnect; one heredoc append created a stray tests/here_provisional.rs there
before the compile error exposed it. Removed immediately (main checkout restored
to its pre-existing state: only the coordinator's untracked design doc). All
subsequent commands re-anchored with an explicit cd to the worktree.
