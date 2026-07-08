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

## T2 — inline bodies lower as anonymous procs after the table

RED evidence (`cargo test -p sigil-frontend-emp --test dispatch`, 5 new tests,
all failed — 21 passed; 5 failed):
- `inline_body_lowers_after_table_byte_exact` — link error, unresolved fixup:
  ```
  link: [Diagnostic { level: Error, message: "unresolved target expression
  for fixup in section text at offset 0", ... }]
  ```
- `inline_body_long_ptrs_byte_exact` — link error, unresolved symbol:
  ```
  link: [Diagnostic { level: Error, message: "unresolved symbol
  `__dispatch$m$R$A` for fixup in section text at offset 0", ... }]
  ```
- `section_nested_inline_body_lowers` — link error, unresolved target
  expression in section `code` at offset 0.
- `inline_body_without_terminator_warns_fallthrough` — `msgs: []`, expected 1
  `[dispatch.body-fallthrough]` (found 0).
- `empty_inline_body_emits_row_and_warns` — `diags: []`, no
  `[dispatch.body-fallthrough]` warning.

The byte tests failed because the `__dispatch$…` label the table row targets was
never DEFINED and its body never emitted (label undefined → unresolved fixup at
link). The warning tests failed because `check_member_body_fallthrough` did not
exist.

GREEN:
- All 5 new tests pass. Full dispatch suite: `cargo test -p sigil-frontend-emp
  --test dispatch` → 26 passed; 0 failed (21 pre-existing + 5 new).
- Whole crate: `cargo test -p sigil-frontend-emp` → all suites pass, 0 failures.
- `cargo clippy -p sigil-frontend-emp --all-targets -- -D warnings` → clean.

Byte derivation (hand-checked, not reverse-fit from output):
- `word_offsets` row = big-endian `target - table_base`. Table is 2 rows × 2
  bytes = 4 bytes. Init's body starts at +4 → `00 04`; wait (named proc) at +6
  → `00 06`. `rts` = `4E 75`. Image: `00 04 00 06 4E 75 4E 75`.
- `long_ptrs` row = big-endian absolute address, link base 0. Table 2 rows × 4
  = 8 bytes. Body A at +8 → `00 00 00 08`; b at +10 → `00 00 00 0A`. Image:
  `00 00 00 08 00 00 00 0A 4E 75 4E 75`.
- Empty body: 1 row × 2 bytes → `00 02`; body emits no bytes, so the label sits
  at +2 (whatever follows), and the fallthrough warning fires (no terminator).

Implementation:
- proc.rs: extracted `ends_in_terminator(buf, cpu)` (the shared last-mnemonic
  heuristic) and rewrote `check_undeclared_fallthrough` to call it (behavior
  identical, no message change). Added `check_member_body_fallthrough` (R9a.4)
  emitting `[dispatch.body-fallthrough]` at the member span, the member-flavored
  mirror of the proc lint.
- lower/mod.rs: `lower_dispatch_item` gained `as_compat: bool` and
  `asm_counter: &mut u32` params. After the `emit_data` call it iterates the
  members in declaration order (R9a.1); each `DispatchTarget::Body` defines its
  hygienic `dispatch_body_label`, then lowers through the SAME
  `eval_proc_body(&[], …)` + `lower_code_buf` path a named proc takes (D-P4.1,
  R9a.3 — empty params, no clobbers/falls_into surface), threading the module
  `asm_counter`. The `@as_compat`-gated `check_member_body_fallthrough` runs
  after lowering. Both call sites (top-level loop + `lower_section_items`) pass
  the new args. Doc comment extended with the 9a sentence.

## T3 — coverage: hygiene, comptime calls, duplicate members

Three new tests appended to the section-9 group in `tests/dispatch.rs`, plus
one comment (no behavior change) in `lower/mod.rs`. All three passed on FIRST
run — no RED phase, per the task brief (Tasks 1-2 already landed the behavior
under test; this task is coverage, not new implementation).

- `inline_bodies_local_labels_are_hygienic_per_member` — already-green. Two
  bodies each declare `.top`; per-instantiation hygiene (D-P4.6) keeps them
  from colliding, and a backward `jbra .top` relaxes to `bra.s` (smallest
  rung). Confirms the anonymous-proc-per-body model actually isolates local
  labels, not just that lowering succeeds.
- `inline_body_statement_comptime_call_expands` — already-green. A
  statement-position `epi()` comptime call inside a body threads the module
  `asm_counter` through `eval_proc_body` exactly as it would in a named proc
  body. Written on its own line per the parser's statement-call-needs-its-own-
  line rule (verified in the Task 2 review — `A: { epi() }` on one line is a
  parse error).
- `duplicate_member_with_body_still_errors` — already-green (positive
  control). `validate_dispatch`'s duplicate-name check runs over member names
  only, before any target-shape inspection, so a `Body` member colliding with
  a `Label` member errors exactly like two `Label` members. This test exists
  to pin that target-shape-agnosticism explicitly, not because a regression
  was suspected.

Byte derivation for the hygiene test (hand-checked): table = 2 rows × 2 bytes
= 4. A's body at +4: `nop` = `4E 71`, then `jbra .top` back to +4 — displacement
from the branch's next instruction (+8) to +4 is −4, fits `bra.s` (byte
displacement) = `60 FC`. B's body at +8: `rts` = `4E 75`, its own `.top` (no
jump). Image: `00 04 00 08 4E 71 60 FC 4E 75`. Comptime-call test: 1 row × 2
bytes = `00 02`, then the `epi()`-expanded body is a single `rts` = `4E 75`.
Image: `00 02 4E 75`.

Results:
- `cargo test -p sigil-frontend-emp --test dispatch` → 29 passed; 0 failed (26
  pre-existing + 3 new).
- Whole crate: `cargo test -p sigil-frontend-emp` → all suites pass, 0
  failures.
- `cargo clippy -p sigil-frontend-emp --all-targets -- -D warnings` → clean.

Implementation (comment only, no behavior change):
- lower/mod.rs (~line 564, in the inline-body lowering loop): added a
  two-line comment above `let Some(buf) = buf else { continue };`
  distinguishing "body failed to evaluate → skip" from "empty body still
  reaches the fallthrough lint below" (Task 2 code-quality review fold-in).

## T5 — gate + byte-diff probe vs master (controller-run)

- `cargo test --workspace --no-fail-fast`: exactly the 4 allowlisted sigil-harness reds
  (full_build_reproduces_sound_driver_regions, vector_table_matches_reference_rom_first_256_bytes,
  full_debug_rom_matches_assembled_reference, full_rom_matches_assembled_reference — the aeon
  sound-driver strlen drift), ZERO new failures.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- Exhibit: `pitcher_plant.emp --root examples/game --prelude prelude` → built: 340 bytes, exit 0,
  zero diagnostics, on BOTH branch and master (359d9cd) — `cmp` BYTE-IDENTICAL (the 9a
  byte-exactness bar: programs not using inline bodies are unchanged).
- Spec §5.5 "Reserved seam" paragraph replaced with the shipped inline-bodies contract in the
  empyrean WORKING TREE (uncommitted, Volence's cadence); design doc D9.1 marked shipped.

9a complete on branch plan7-item9: T1 247c932 (+1b0febf review fold-in), T2 d48a3e8,
T3 19e6e57, docs 05ecc90/66e543f. Two-stage reviews passed on T1 (spec ✅, quality ✅ after
doc-reorder fold-in) and T2 (spec ✅ incl. 4 adversarial probes, quality ✅ no fix-first).
