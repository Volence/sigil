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
