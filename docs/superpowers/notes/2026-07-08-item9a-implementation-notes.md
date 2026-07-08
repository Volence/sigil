# Plan 7 #9a — dispatch inline member bodies: implementation notes

Branch plan7-item9 (worktree .worktrees/plan7-item9). Plan:
docs/superpowers/plans/2026-07-08-spec2-plan7-item9a-dispatch-inline-bodies.md.
RED evidence recorded per task, per the 2026-07-08 here-fix precedent.

## T1 — DispatchTarget::Body parses; table rows target hygienic labels

RED evidence:
- `cargo test -p sigil-frontend-emp --test dispatch inline_body_member_parses_and_lowers_clean`
  failed at the parse assert with the reserved-seam diagnostic:
  ```
  thread 'inline_body_member_parses_and_lowers_clean' panicked at
  crates/sigil-frontend-emp/tests/dispatch.rs:497:5:
  parse: [Diagnostic { level: Error, message: "dispatch member bodies
  (`Member: { … }`) are reserved for scripted states (backlog #9) — bind a
  proc label instead", primary: Span { source: SourceId(0), start: 64, end: 65 } }]
  test result: FAILED. 0 passed; 1 failed; ...
  ```

GREEN:
- New test `inline_body_member_parses_and_lowers_clean` passes.
- Full dispatch suite: `cargo test -p sigil-frontend-emp --test dispatch` →
  21 passed; 0 failed (was 20 pre-existing + 1 new).
- Whole crate: `cargo test -p sigil-frontend-emp` → all suites pass,
  0 failures. `cargo clippy -p sigil-frontend-emp --all-targets -- -D warnings`
  clean.
- Pre-existing `tests/parser_decls.rs::dispatch_reserves_inline_body_form`
  asserted the now-reversed reserved-seam contract; rewritten to
  `dispatch_inline_body_parses_as_body_target` — asserts the inline body parses
  clean into `DispatchTarget::Body` and coexists with a `Label` member.

Implementation:
- ast.rs: `DispatchMember.target` is now `DispatchTarget` (new enum:
  `Label(Expr)` / `Body(Vec<AsmStmt>)`); doc comments updated for D9.1.
- parser.rs: `dispatch_decl` parses `Member: { … }` via `asm_body` (same
  grammar as a `proc` body, splices_allowed=false) into `DispatchTarget::Body`;
  the label arm wraps `DispatchTarget::Label`. The now-dead
  `skip_balanced_braces` recovery helper (its only caller was the reserved
  seam) was deleted.
- layout.rs: added `dispatch_body_label(module, table, member)` →
  `__dispatch$<module>$<table>$<member>` (R9a.2). `eval_dispatch_with_root`'s
  target-name extraction is now a two-level match: a `Body` arm yields the
  hygienic label and skips the `[dispatch.target-not-code]` kind check (R9a.5 —
  code by construction); the three existing `Label` arms are unchanged.
- lower/mod.rs body lowering is Task 2 — untouched here.
