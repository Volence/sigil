# Sigil M1.C — T2: Expression Completeness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.

**Goal:** Add the mainline AS expression features still missing from `expr.rs`: the `||`/`&&`
operators, string-literal comparison in `if`-conditions, and the correct value-context
comparison result (0/1 vs AS's 0/−1), settled empirically against real `asl`.

**Architecture:** Operators extend the existing Pratt parser (`expr.rs`) + `sigil_ir::BinOp`
fold, kept **numeric and AS-neutral** (§7.4 — no AS-specific 0/−1 baked into IR). String
comparison is a **front-end condition concern** that folds to a constant before any numeric
`Expr` is built (strings never enter `sigil_ir::Expr`). The 0/−1 question is answered by
asl, not assumed.

**Tech Stack:** Rust; asl-diff via the existing snippet harness (`tests/asl_snippets.rs` +
`tests/snippets_golden.txt` + `cargo run -p sigil-frontend-as --bin gen-snippet-vectors`,
which shells `aeon/tools/asl -cpu 68000 -q -L -U` → `p2bin`).

Spike 0 findings (`docs/superpowers/notes/2026-07-03-m1c-spike0-findings.md`): mainline real
uses are `<>` ×70 (already lexes to `Ne` — lexer.rs:115 — and folds), `||` ×5 (3 in
`dac_samples.asm` = mainline, 2 debug), `&&` ×4 (all debug). `mod`/`!=`/`~`/`~~` have **zero**
real uses — out of scope.

---

## Scope (precise)

1. **`||` and `&&` binary operators.** Not currently lexed or parsed. `||` is mainline
   (dac_samples), `&&` is debug-only but trivially folds alongside — include both.
2. **String comparison in `if`-conditions:** `"a" = "b"` and `"a" <> "b"` → boolean. Used by
   macro guards like `if "AMPLITUDE" = ""` (the deform_table_sine/triangle macros). At T2,
   test with literal strings; macro-param substitution into the string is T7's job.
3. **Value-context comparison result:** settle `dc.b (5 < 9)` etc. against asl. If asl emits
   `0xFF` (−1), make comparisons yield −1 **in the front end** (wrap the comparison `Expr` in
   `UnOp::Neg`, since IR `Eq/Ne/Lt/...` fold to 0/1) — do NOT change IR's neutral 0/1 fold.
   If asl emits `0x01`, the existing 0/1 is already correct and no Neg-wrap is needed.

Out of scope: `mod`, `!=`, `~`, `~~` (0 real uses); the `!`=bitwise-or operator (debug-only →
T9); `\{expr}` interpolation (already works via `interp_string` for error/fatal).

---

## Files

- `crates/sigil-frontend-as/src/token.rs` — add `Punct::OrOr`, `Punct::AndAnd` if absent.
- `crates/sigil-frontend-as/src/lexer.rs` — lex `||`→OrOr, `&&`→AndAnd (2-char, like `<<`).
- `crates/sigil-ir/src/expr.rs` — add `BinOp::LogOr`, `BinOp::LogAnd` with **neutral 0/1**
  fold (`a||b` = `((a!=0)|(b!=0))`, `a&&b` = `((a!=0)&(b!=0))`), mirroring the existing
  comparison `bool_val` convention. Precedence below comparisons.
- `crates/sigil-frontend-as/src/expr.rs` — map OrOr/AndAnd in `infix_bp`; precedence: `&&`
  tighter than `||`, both looser than comparisons (AS ladder).
- The `if`-condition evaluator (find it: `exec_if` / wherever the condition `Expr` is folded
  in `eval.rs`) — before numeric parse, detect `Str <cmp> Str` and fold to a bool constant.
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — add asl-diff snippets (below).

---

## Task steps (TDD)

- [ ] **Step 1 — asl-diff snippets first (they define truth).** Add these blocks to
  `snippets_golden.txt` (asm only; regenerate bytes in Step 2). Match the existing snippet
  conventions in that file (CPU directive, `padding` state, section/org as the others do):

  - `or_or_operator`: an `if (0 || 0)` / `dc.b` / `else` / `dc.b` / `endif` that exercises
    `||` truthiness, plus a `dc.b (1 || 0)` value.
  - `and_and_operator`: same shape for `&&`.
  - `value_context_lt`: `dc.b (5 < 9)` and `dc.b (9 < 5)` — **the 0/−1 discriminator.**
  - `value_context_ne`: `dc.b (3 <> 3)` and `dc.b (3 <> 4)`.
  - `string_eq_true` / `string_eq_false`: `if "a" = "a"` vs `if "a" = "b"` selecting
    different `dc.b`.
  - `string_ne`: `if "a" <> "b"` selecting a `dc.b`.

- [ ] **Step 2 — generate goldens from asl.** Run
  `cargo run -p sigil-frontend-as --bin gen-snippet-vectors` (needs `aeon/tools/asl`).
  **Record what asl emitted for `value_context_lt`/`value_context_ne`** — `0xFF` means AS
  comparisons are −1 (do the Neg-wrap in Step 5); `0x01` means 0/1 (skip the Neg-wrap).
  Commit the regenerated `snippets_golden.txt`.

- [ ] **Step 3 — run the gate, watch it fail.**
  `cargo test -p sigil-frontend-as --test asl_snippets` — FAILS (new operators/strings
  unimplemented: parse returns `None` or the wrong branch/bytes).

- [ ] **Step 4 — implement `||`/`&&`.** Add the Punct + lexer + IR `BinOp::LogOr/LogAnd`
  (neutral 0/1 fold) + `infix_bp` mapping. Unit-test the IR fold (mirror the existing
  `bin(Eq,3,3)` tests in `sigil-ir/src/expr.rs`).

- [ ] **Step 5 — implement string comparison + settle 0/−1.** In the `if`-condition path,
  fold `Str <=/<>> Str` to a constant. If Step 2 showed asl emits −1 for value-context
  comparisons, wrap comparison/`LogOr`/`LogAnd` results in `UnOp::Neg` **in the front-end
  lowering only** (keep IR at 0/1). Add a focused unit test that `dc.b (5<9)` produces the
  exact asl byte.

- [ ] **Step 6 — gate green.** `cargo test -p sigil-frontend-as --test asl_snippets` PASS,
  and `cargo test -p sigil-frontend-as` (all unit + integration) PASS. No Z80 regressions.

- [ ] **Step 7 — clippy + workspace.**
  `cargo clippy --workspace -- -D warnings && cargo build --workspace` clean. (BinOp is a
  shared IR type — confirm no other crate breaks on the new variants; add arms if a match is
  non-exhaustive.)

- [ ] **Step 8 — commit.**
  `git commit -m "feat(sigil-frontend-as): ||/&&, string comparison, value-context comparison mask (asl-gated)"`

---

## Self-Review

- Spec coverage: `||`/`&&` (scope 1), string comparison (scope 2), 0/−1 settle (scope 3) all
  have snippet + unit tests. `<>` needs no work (already lexes/folds).
- §7.4: strings never enter `sigil_ir::Expr`; the 0/−1 mask lives in the front-end (Neg-wrap),
  IR `BinOp` stays neutral 0/1. LogOr/LogAnd are generic operators, not AS-specific.
- Empiricism: the 0/−1 decision is made from asl output (Step 2), never assumed.
