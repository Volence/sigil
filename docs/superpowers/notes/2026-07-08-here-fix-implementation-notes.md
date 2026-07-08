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
