# Handoff — Sigil Spec 2 Plan 2 (.emp comptime evaluator) — overnight autonomous run

**Context (why this doc exists):** Volence went to bed after M1.D shipped and asked me to
"follow the docs and try to get a decent amount done" on the next milestone — the `.emp`
language. This handoff records where things stand and what I'm doing autonomously so any
fresh session (or Volence at wake-up) can pick up cleanly.

## What just shipped (M1.D — DONE, on master)

Sigil's byte-exact AS assembler is COMPLETE and merged: full ROM byte-exact for BOTH the
non-debug (`m1d_rom`) and `__DEBUG__` (`m1d_debug_rom`) builds. master HEAD `11aaf0d`
(merge `e7d4f98` + the `emit_s4_rom` DEBUG=1 tool). See the memory note
`sigil-m0-core-progress.md` (auto-loaded) and `docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md`.
That's a SEPARATE track (the AS-source assembler). This handoff is the OTHER track.

## What this is: the `.emp` surface language (Spec 2)

`.emp` is the modern surface language that will replace hand-written AS source — a small,
learnable, Haskell-flavored language whose whole metaprogramming story is ONE pure
functional `comptime fn`. Design authority: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`
(approved). Design taste (memory): adoption-over-cleverness, no ceremony tax, FP/types,
totality ("illegal states don't compile"), one-page-learnable. Volence's favorite language
is Haskell — lean into type-system-as-erasing-proof-layer + pure-functional comptime.

**Plan sequence** (from the Plan 1 doc):

| Plan | Delivers | Status |
|---|---|---|
| 1 | Lexer + parser + AST | ✅ MERGED (`sigil-frontend-emp`, 55 tests) |
| **2** | **Comptime value evaluator** (pure exprs, `ensure`, step budget, `comptime for/if/while`, string/array builtins §6.8) | **← THIS RUN** |
| 3 | Types & layout engine (struct/bitfield/enum layout, `sizeof`/`@` asserts, `Data` values) | blocked on 2 |
| 4 | `IrStreamer` + lowering (procs, hygiene, `asm{}` via sigil-isa) | blocked on 3 + Core |
| 5 | Capability sandbox (`embed`/`import`/`zx0`) + `as.*` float | blocked on 4 |
| 6 | `@as_compat` + mixed `.asm`+`.emp` build + port diff | blocked on Core done |

## Plan 2 scope (grounded in spec §6 + §4.2 + §6.8)

Build the comptime value **evaluator** in the existing `sigil-frontend-emp` crate. It takes
the parsed AST and evaluates comptime expressions to comptime `Value`s. Concretely:

IN scope:
- A `Value` model for comptime values (§4.2): `int` (arbitrary precision at comptime —
  pragmatic `i128` for v1, range-checked on emission later), `float` (f64), `string`,
  `bool`, arrays (carry `len`/`map`/`filter`/`fold`), and comptime struct/enum VALUES
  (field maps / tagged variants — NO byte layout yet, that's Plan 3).
- Pure expression evaluation: all AST `BinOp`/`UnOp` (arith, bitwise, shift, comparison
  folding to bool, `&&`/`||`, `++` concat), ranges `LO..HI`, array/struct/tuple literals,
  paths/const lookups.
- Environment: lexical scopes; immutable `let`; `comptime var` mutable **only** inside a
  `comptime block`/comptime-fn body (§6.3 — no module-level mutable state).
- Control flow as expressions: `comptime if/else`, `comptime for` (over range or array →
  yields an array), `comptime while`.
- `comptime fn` evaluation: call user fns (expression-bodied and block-bodied) with args.
- Builtins (§6.8): arrays `len/map/filter/fold`; strings `len/find/slice/val`
  (`find` is STANDARD — no last-char bug); `Data`/`Code` monoids (`.empty` + `++`) MAY be
  stubbed/opaque until Plan 3/4 makes them constructible.
- Guards (§6.5): `ensure(cond, "msg {interp}")` → error `Diagnostic`; `ensure_fatal` aborts.
- Step budget (§6.7): bounded evaluation; on exceed, name the innermost non-terminating
  call chain (not an opaque quota error).
- Functional glue (D2.12) `|>` and lambdas `|x| expr` — needed to make `map/filter/fold`
  usable; they ERASE (compose generators). `?`/`Result` is Plan 3-ish (sum types) — defer
  unless cheap.

OUT of scope (later plans): struct/bitfield/enum LAYOUT + byte emission + `sizeof`/`offsetof`
(Plan 3); `asm{}` instantiation + `IrStreamer` lowering (Plan 4); `embed`/`import`/`zx0` +
`as.*` bit-compat float (Plan 5); `use`/prelude name resolution across modules (evaluator
resolves within a file for now). `math.*` basic float ops are fine; `as.*` is Plan 5.

## Carry-forward from Plan 1 to fold in where natural

- `af'` Z80 shadow-register apostrophe is unlexable — add `'` lexing (small).
- `path()` can emit an inverted span (end < start) when `expect_ident` consumes nothing —
  fix before a renderer asserts start ≤ end.
- Drop-glue stack overflow on huge flat operator chains (~200k terms) — iterative `Drop`
  or arena; natural alongside the evaluator (the evaluator walks these trees anyway).
- Coverage-hole tests: Z80 lines in `cpu: z80` sections, section-in-section, `pub` in
  sections (work by hand, unpinned).
- Split `parser.rs` (~1400 lines) before lowering grows it (optional; low priority).

## Process (established, keep it)

- Feature branch `spec2-p2-emp-evaluator` (off master `11aaf0d`). NOT master.
- TDD: failing test → implement → green, per feature. Every `Value`/eval behavior pinned by
  a test. `cargo test -p sigil-frontend-emp` + `cargo clippy --workspace --all-targets -D
  warnings` green before each commit.
- Two-stage review (spec + code-quality) via `superpowers:code-reviewer` subagents on the
  load-bearing chunks — the Plan-1 loop caught ~15 real bugs. Ground eval semantics in the
  SPEC, not intuition (mirror the Core "probe-first" discipline: where asl/AS semantics are
  the reference — e.g. integer wrapping widths, `val` string→int, `find` — cross-check).
- `sigil-frontend-emp` depends on `sigil-span` ONLY (+ nothing new) — keep the crate a clean
  deletable unit. NO IR/backend dependency in Plan 2.
- Update THIS handoff + the `spec2-progress` memory as tasks land. Write the Plan 2 plan doc
  in `empyrean/docs/plans/`.

## Milestone-boundary note

Plan 2 is a milestone; per the standing convention I will NOT merge to master without a
Volence checkpoint. Overnight I implement + commit on the branch, keep everything green, and
leave a clear state for the wake-up review.

## Progress log (updated as I go)

- (start) Branch created off `11aaf0d`. Reading done: spec §4/§6/§6.8, Plan 1 carry-forward,
  current crate state (lexer/ast/parser, 55 tests, `Diagnostic{level,message,primary}`).
- **Setup.** Verified Plan-1 AST has NO lambda/`|>` node (lexer only has single `|`=BitOr) and
  `parse_expr_for_tests(src)->Expr` exists for expr-level tests; `true/false/none` are plain
  `Path`s (no `Expr::Bool`). Recorded decisions **D-P2.12** (T6 must extend the frontend with
  lambdas + `|>`) and **D-P2.13** (`i64` literal → `i128` widening) in the plan doc.
- **T1 DONE** (`afe0089`). `value.rs` (`Value` enum per D-P2.2 + `Display` + `type_name`) and
  `eval.rs` (`Env` scope-chain, `Binding`, `AssignError{NotFound,Immutable}`, `Evaluator`
  {diags,steps,call_stack}, `STEP_BUDGET=5_000_000`, `eval_const` stub). Lambda lives in
  `value.rs` (Env is cheaply/independently clonable). 25 unit tests. Self-reviewed by me — solid.
- **T2 DONE** (impl `ab61c56`, review fixes `f549a57`). Pure `Evaluator::eval_expr` — all
  BinOp/UnOp with CHECKED i128 arithmetic (overflow=error, D-P2.1), div/mod-by-zero, D-P2.3
  promotion, comparisons→Bool (total, structural ==), short-circuit `&&`/`||`, `++` concat
  (Str/Array), ranges, array/tuple literals, path true/false/none + env lookup, Poison
  discipline (D-P2.9). Two-stage review PASSED (spec ✅ compliant; quality: Approve — all
  Minor). Fixes folded: merged `unop_type_error`→`operand_type_error`, `eval_equality`→`&self`,
  backfilled direct add/mul/sub/neg-overflow + wrong-type bitwise/shift tests. **102 tests**,
  clippy clean. Call/StructLit/If/For/Asm return Poison placeholders for later tasks.
- **T3 DONE** (`4ec3ad5`). `Evaluator<'a>` gained a borrowed file index (`consts`/`enums`
  HashMaps) + owned `const_memo` + `in_progress` stack; `with_file()` builds it, `new()` stays
  the empty-program ctor (T2 tests still green). Lazy memoized const resolution with cycle
  detection (`cyclic const definition: A -> B -> A`, Poison memoized to suppress cascade).
  `eval_path` now resolves single names to file consts (env shadows consts) and `E.V` to nullary
  `Value::Enum`. Folded in **D-P2.14**: `StructLit` → `Value::Struct` (value-level only, no
  layout/field/default checks) and enum-variant paths. Self-reviewed by me (cycle logic +
  lifetime plumbing correct). New `tests/eval_consts.rs` (10 tests); crate green, clippy clean.
- **T4 DONE** (impl `1a54e4b`, fix `fc40054`, quality `2dcc134`). Comptime fn calls: `fns` index,
  `enum Flow{Normal,Return}` + `exec_stmts`, `eval_if` (stmt+expr position — **D-P2.15** moved
  if/else here), positional+named arg binding (missing/extra/unknown/duplicate diagnostics),
  fresh pure per-call env, recursion via `MAX_CALL_DEPTH=512` on a 64MB scoped thread + step-budget
  `abort()` naming the call chain (**D-P2.16**). Two-stage review: spec review caught a **CRITICAL**
  — `return` in a call-ARGUMENT expression leaked into the callee (`callee(if c {return 7} else {2})`
  returned from callee not caller); fixed with a `pending_return` guard in `eval_call`. Quality
  review (Approve) → added an `eval_operand` helper that DRYs the pending_return check across all 5
  operand sites + an INVARIANT doc so T5's new arms can't reintroduce the leak; extracted
  `MAX_CHAIN_FRAMES`, fixed spurious arity diags after mid-arg return, preserved thread panic. New
  `tests/eval_fns.rs` (23 tests). **135 tests total**, clippy clean.
- **T5 DONE** (impl `c2430d5`, quality `4f9321e`). Control flow: `comptime_ctx` mutability-context
  counter, `Stmt::Var`/`Assign`/`ComptimeBlock`/`While` + `Expr::For`/`Stmt::For`. `for` → Array of
  per-iteration body values (Range/Array iterables, lazy range), `while` → Unit (step-budget
  bounded), comptime var/assign honor mutability + the `eval_operand`/pending_return invariant.
  Two-stage review: spec ✅ (live-probed all 4 return-in-operand cases, ctx balance, and confirmed
  the infinite-`while` test genuinely hits the 5M budget → "step budget exceeded"); quality Approve
  → unified the Range/Array loop bodies, added `exec_scoped`/`exec_comptime_scoped` helpers (killed
  the 4× push/exec/pop idiom + made ctx edit-proof), +2 coverage tests. New
  `tests/eval_control_flow.rs` (17 tests). **153 tests total**, clippy clean.
- **T6 DONE** — split per **D-P2.17**.
  - **T6a** (frontend, impl `623f8bc`, fix `4af9f2d`): `|>` token, `|params| e` lambdas at primary
    position (no zero-param — collides with `||`), `Expr::Lambda`, pipe desugars in the PARSER to a
    Call (piped value = first arg). Self-review caught + fixed a real bug: `a |> f + b` orphaned
    `+ b`; fixed by parsing the pipe target with `expr_bp(1)` so pipe is consistently the loosest
    layer (`a + b |> f` = `f(a+b)`; invalid target = clean diagnostic). 19 parser + 11 lexer tests.
  - **T6b** (evaluator, impl `fa9f237`, fixes `d507332`): added `Value::FnRef` (first-class fn
    refs — makes `bands.map(band_entry)` work), `Expr::Lambda`→`Value::Lambda` (captures env),
    `apply_callable` + a factored `call_fn_with_values` (FnRef recursion is depth/budget-bounded),
    §6.8 builtins (array/range `len/map/filter/fold`, string `len/find/slice/val` per **D-P2.18** —
    char-indexed, standard `find`, half-open `slice` with OOB error, `val`=int-literal parse), struct
    field access + `.len`/`.val` bare paths, builtin-before-user dispatch (not shadowable). Two-stage
    review: spec ✅ (probed find/slice/val + recursion-safety); quality Approve → tightened
    `parse_emp_int` (reject `+5`/`$-5`), added bare `s.val`, fixed a map/fold Poison-cascade
    (short-circuits per D-P2.9), +24 diagnostic-path/edge tests. `tests/eval_builtins.rs` 58 tests.
  - **Checkpoint: 223 tests total, clippy clean, still `sigil-span`-only.**
- **T7 DONE** (impl `210ffc7`, polish `d2def6d`). `ensure`/`ensure_fatal` special-cased in
  `eval_call` (before builtins/user fns, not shadowable) → `eval_guard`: arity/type checks, passing
  guard silent, failing `ensure` emits an interpolated error + Poison (D-P2.8), `ensure_fatal` sets
  `aborted` with the interpolated text as the sole reason. `{interp}` (D-P2.19): `interpolate`/
  `interp_one` walk the message, `{{`/`}}` escape, `{expr}` is lexed+parsed (reusing `Parser::expr`)
  + evaluated in the current env, spliced via Display — strings spliced UNQUOTED (review polish);
  best-effort `<?>` + diagnostic on parse/eval failure. Self-reviewed by me (guard/interp logic +
  the unquoted-string fix). New `tests/eval_guards.rs` (13 tests). Crate green, clippy clean.
- **T8 DONE** (impl `03b1a0c`, seam fixes `db75176`). `tests/eval_corpus.rs` (9 end-to-end
  programs: recursion, parallax-style monotonic fold w/ comptime var+for+ensure+interp, deform_sine
  guard+table skeleton, functional pipeline `|> map |> filter |> fold` w/ lambdas AND fn-refs, const
  dependency graph, string processing) — every asserted value matched; the corpus surfaced NO
  evaluator bugs (only a correct clarification that `ensure` is non-fatal so the fn continues).
  Carry-forwards: `af'` lexes as one ident (trailing `'`), `path()` span now `start.merge(...)` so a
  missing-ident recovery can't invert it. Note: array PARAM types must be sized `[int; N]` (`[int]`
  doesn't parse); length not yet type-checked (Plan 3).
- **WHOLE-BRANCH REVIEW** (final gate) found 3 cross-feature seam bugs the 6 isolated reviews missed,
  all fixed in `db75176`:
  1. **CRITICAL** — a `return` inside a lambda body (via an expression-position `if`/`for`) leaked
     through `map`/`filter`/`fold` and became the ENCLOSING fn's return (silent wrong value, zero
     diagnostics). Fixed: `apply_callable` takes `pending_return` after the body → the `return`
     yields from the lambda. Repro `go()` now correctly = `Int(3)`.
  2. **IMPORTANT** — a locally/const-bound lambda/fn-ref couldn't be called by name (`let f=|x|x+1
     f(10)` → "unknown function"). Fixed: `eval_call` resolves a single-segment callee as a value
     (env→const) between the builtin check and the `fns` lookup, routing Lambda/FnRef through
     `apply_callable`; non-callable value → "not callable".
  3. **IMPORTANT** — range builtins materialized eagerly with no step charge (`(0..3e9).map` OOMs).
     Fixed: range `len` is O(1); range map/filter/fold consume lazily charging a step per element →
     a huge range hits the step budget, not the allocator.
  `tests/eval_builtins.rs` +8 (66). **Module-split of eval.rs (~2160 lines) recommended as a
  POST-checkpoint follow-up** (cohesive now, no correctness reason to split before the milestone).

## FINAL STATE (Plan 2 complete, on branch, NOT merged)

- **All 8 tasks done. 258 tests pass. `cargo clippy --workspace --all-targets -- -D warnings` clean.
  Crate still depends on `sigil-span` ONLY.** Branch `spec2-p2-emp-evaluator`, HEAD `db75176`, off
  master `11aaf0d` (master untouched — M1.D track intact). Nothing pushed; empyrean repo not pushed.
- **Acceptance met:** `const`/`comptime fn` programs evaluate to expected `Value`s (corpus);
  `ensure` failure and non-termination each produce a NAMED, spanned diagnostic; the two-stage +
  whole-branch review loop caught a CRITICAL silent-wrong-value bug and fixed it.
- **Milestone gate:** per the standing convention I did NOT merge to master — Plan 2 needs Volence's
  checkpoint. Suggested review path: skim the plan doc's D-P2.1..D-P2.19 decisions, read
  `tests/eval_corpus.rs` (the readable acceptance proof), then decide merge.
- **Known follow-ups (non-blocking, for Plan 3+):** (1) split `eval.rs` into an `eval/` module tree
  (seams sketched in the final review); (2) `none`→`Unit` is a placeholder that meets real
  Option/sum-types in Plan 3; (3) everything in the OUT-of-scope list below is deliberately deferred.
