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

## T2 — `lower/script.rs`: desugar + hidden resume table + body lowering

### Probe (mandated design evidence — throwaway, deleted before commit)

Parsed `proc p (a0: *S) { jbra done\n jbra .top\n move.w #2, $20(a0) }` and
Debug-printed the InstrLines. The synthesis mirrors these EXACT shapes:

- `jbra done` (global ident): `Instr(InstrLine { mnemonic: [Text("jbra")],
  size: None, operands: [Plain { expr: Path(Path { segments: ["done"] }),
  size: None, span }] })`.
- `jbra .top` (dot-local): identical, but the segment string KEEPS the leading
  dot — `Path { segments: [".top"] }` (NOT `"top"`). So the loop-back / local
  epilogue jbra is built as `Path([".__loop$0"])` / `Path([".<name>"])`.
- `move.w #2, $20(a0)`: `Instr(InstrLine { mnemonic: [Text("move")],
  size: Some(Text("w")), operands: [ Imm(Int(2, span)),
  DispInd { disp: Int(32, span), inner: Ind { parts: [(Path(["a0"]), None)],
  size: None, span }, span } ] })`. The displacement is a NUMERIC `Int` (the
  field offset), so the yield store is independent of bare-field-access rules.

`Reg` names print `a0`..`a7`/`d0`..`d7` (inverse of `from_name`); an address
register is detected by the `'a'` first byte of that spelling.

### Owner / module-string (R9b.11 — one source of truth)

`eval_asm_owned` (eval/asm.rs:56–59) builds `Owner::Proc { module:
self.module_id, name }` where `module_id = file.module.path.segments.join(".")`
(eval/mod.rs:453). `lower_script_item` computes the SAME `Owner::Proc { module,
name: script_name }` and derives each resume row's final name via
`Owner::local_symbol("__resume$k")` → `$<module>$<script>$__resume$<k>`. The
table's `DispatchTarget::Label(Expr::Str(final_name))` passes through
`eval_dispatch_with_root`'s Str arm verbatim, so the row targets exactly the
symbol the body's `__resume$k` label definition renames to. `local_symbol` was
exposed `pub(super)`; `proc::ends_in_terminator` likewise (was private).

### RED (before implementation)

`cargo test -p sigil-frontend-emp --test script` — the 4 T1 parse tests passed;
the 6 T2 tests all failed (Item::Script fell into the lowering wildcard → only
the trailing `proc done` emitted, e.g. byte-exact tests got `[0x4E,0x75]` vs the
full 18-byte image; the diagnostic tests got `msgs: []`).

### Byte-vector verification

Probes A/B/C reproduced EXACTLY as hand-derived in the plan — no expectation was
touched:
- A (word_offsets, one yield): `00 04 00 0E 4E 71 31 7C 00 02 00 20 60 02 4E 75 4E 75`.
- B (long_ptrs, ×4 ordinal WORD): `00 00 00 08 00 00 00 12 4E 71 31 7C 00 04 00 20 60 02 4E 75 4E 75`.
- C (loop → `__loop$0` + `jbra` back = `60 F4`): `00 04 00 0E 4E 71 31 7C 00 02 00 20 60 02 60 F4 4E 75`.

### GREEN

`cargo test -p sigil-frontend-emp --test script` → 10 passed (4 T1 + 6 T2), 0
failed. Whole crate: 52-suite run, all ok, 0 failures. `cargo clippy -p
sigil-frontend-emp --all-targets -- -D warnings` clean. Workspace: the only red
is the pre-existing `full_build_reproduces_sound_driver_regions` (`strlen()`
builtin in the sound-driver corpus) — confirmed failing on a clean stash of this
work, i.e. an allowlisted red, not introduced by T2.

### T2 fold-in (spec review): `Item::Script` resolver arms

The review found the ONE contract violation in T2: `Item::Script` was absent
from `resolve/imports.rs`, which only the PROGRAM path (`build_program`, i.e.
CLI `--root`) exercises — the unit-test `lower_module` harness bypasses the
resolve pass entirely, which is why T2's byte tests could not catch it.

1. **`collect_defined` (defined-names map):** a script's hidden table SELF-
   references its base label (`dc.w resume_k - name` rows), so without a
   Script arm ANY script — even unreferenced — failed `report_unresolved`.
2. **`item_pub_name` (pub exports):** `pub script` exported nothing,
   contradicting R9b.8 ("pub script exports it like pub dispatch").

Fix: `ast::Item::Script` arms mirroring the adjacent `Dispatch` arms in both
functions. (Parity check: `resolve/mod.rs`'s injectable-item list is the
comptime TYPE-injection channel — Proc/Dispatch/Offsets are absent there too,
so no third arm is needed.)

Regression tests (the established item-4 program-path pattern:
`crates/sigil-cli/tests/module_resolution.rs`, CLI + tempdir + `--root`,
byte-pinned like the cross-module dispatch tests):
- `script_compiles_unreferenced_under_program_path` — solo module, script
  unreferenced; out.bin = the 18-byte Probe A image. RED (pre-fix):
  `m.emp:1:1: unknown symbol `brain``.
- `cross_module_pub_script_resolves_via_use` — `pub script brain` in
  `engine`, entry `obj` does `use engine.{brain}` + `jmp brain`; 24 bytes =
  jmp abs.w `4E F8 00 06` + 2-gap + the Probe-A image verbatim at LMA 6 (the
  script image is position-relative: RelOffset rows + short jbra). RED
  (pre-fix): `module `engine` has no `pub` name `brain`` + two cascading
  unknown-symbol errors.

GREEN: both new tests pass byte-exact; `cargo test -p sigil-frontend-emp`
(46 ok suites) and `-p sigil-cli` fully green; clippy `-D warnings` clean on
both crates.
