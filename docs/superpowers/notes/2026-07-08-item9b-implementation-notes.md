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
