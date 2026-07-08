# Plan 7 #9b — script/yield MVP: implementation notes

Branch plan7-item9 (worktree .worktrees/plan7-item9). Plan:
docs/superpowers/plans/2026-07-08-spec2-plan7-item9b-script-yield.md.
RED evidence recorded per task, per the 2026-07-08 here-fix precedent.

## T1 — script parses (decl, `loop`, `yield`, `shows`)

RED evidence:
- `cargo test -p sigil-frontend-emp --test script` failed to COMPILE (the AST
  types the tests reference do not exist yet):
  ```
  error[E0433]: cannot find `ScriptStmt` in `ast`
  error[E0599]: no variant, associated function, or constant named `Script`
    found for enum `Item` in the current scope
   --> crates/sigil-frontend-emp/tests/script.rs:64:45
  ```

GREEN:
- New tests `script_decl_parses_with_loop_yield_and_shows` and
  `script_requires_encoding_attr` pass:
  `cargo test -p sigil-frontend-emp --test script` → 2 passed; 0 failed.
- Whole crate: `cargo test -p sigil-frontend-emp` → 46 suite-result lines,
  all ok, 0 failures. `cargo clippy -p sigil-frontend-emp --all-targets --
  -D warnings` clean.

Implementation:
- ast.rs: `ScriptDecl` / `ScriptStmt` (Asm/Loop/Yield) / `ScriptLabel` added
  after `ProcDecl`; `Item::Script(ScriptDecl)` variant after `Item::Proc`.
- parser.rs: `item()` dispatches `script` after `proc`; `"script"` added to
  the `OPENERS` recovery const (16→17; unconditional opener like `proc`, no
  lookahead special-case — verified the `ensure` guard branch is unaffected).
  New `script_decl` / `script_label` / `script_body`. `asm_body`'s loop
  interior factored into `asm_stmt(splices_allowed) -> Option<AsmStmt>`,
  called by BOTH `asm_body` and `script_body` (behavior-identical for procs;
  whole-crate suite green confirms).
- lower/mod.rs: `// #9b Task 2` breadcrumbs on the wildcard arms of the
  top-level loop and `lower_section_items` (where the desugar will hook).
  `Item::Script` is inert everywhere else (falls into existing wildcards /
  `if let` guards in resolve/eval).

### T1 review fold-in (fix-first: 1 Important + 3 minors)

1. **Loop-nesting depth guard (Important).** `script_body`'s `loop` arm
   recursed unguarded. RED evidence, empirical (pre-guard code):
   - New test `deep_loop_nesting_is_an_error_not_an_abort` (600 nested
     `loop {`) failed `assertion failed: !diags.is_empty()` — 600-deep
     recursion parsed silently, no diagnostic.
   - At reviewer scale (temporary 50k-nesting probe, deleted after use):
     ```
     thread 'probe_50k_nested_loops' has overflowed its stack
     fatal runtime error: stack overflow, aborting
     (signal: 6, SIGABRT: process abort signal)
     ```
   Fix: the `loop` arm is now gated on the SAME `block_depth` counter /
   `MAX_EXPR_DEPTH` ceiling as `stmt_block` (increment on entry, ceiling →
   "block nesting too deep (max 128)" + consume-to-balanced-`}`, decrement
   on exit). `stmt_block`'s recovery scan was lifted verbatim into a shared
   `skip_unparsed_block` helper used by both (pure move; the parser_bodies
   deep-nesting tests guard the stmt_block side). GREEN: the 600-nesting
   test passes (one depth diagnostic, no flood, following `const GOOD`
   still parses); the re-run 50k probe printed `diags: 1 — first:
   Some("block nesting too deep (max 128)")` and exited cleanly.
2. **`yield` line-end parity (minor).** `expect_line_end()` →
   `expect_line_end_or_rbrace()` (the instruction-line rule), so
   `{ yield }` parses like `{ nop }`. RED: `yield_tolerates_same_line_close`
   failed with `expected end of line` plus a cascading
   "expected `}`, found Eof". GREEN: passes.
3. **Construct-neutral encoding-attr wording (minor).**
   `dispatch_encoding_attr` is shared with `script`, so its two messages no
   longer say "dispatch": "this declaration requires an `(encoding:
   word_offsets | long_ptrs)` attribute" / "expected `encoding:` in the
   attribute list". No test pinned the old wording (verified:
   `dispatch_requires_encoding` and `script_requires_encoding_attr` only
   check for "encoding" / the encoding names).
4. **`param_list` extraction (optional — done).** The proc/script param
   loops were byte-identical including the surrounding paren expects — a
   clean lift into `fn param_list(&mut self) -> Vec<(String, Type, Span)>`
   used by both `proc_decl` and `script_decl`.

Post-fold-in: `cargo test -p sigil-frontend-emp --test script` → 4 passed;
whole crate → 46 suite-result lines, all ok, 0 failures; clippy
`-D warnings` clean.
