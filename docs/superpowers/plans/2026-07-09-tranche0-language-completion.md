# Tranche 0 — Language-Completion Sprint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the nine ratified-but-unbuilt Spec-2 language items up front (per the kickoff's TRANCHE 0 brief) so the 68k engine-port campaign converts against a finished language.

**Architecture:** All work lands in `crates/sigil-frontend-emp` (lexer → parser → AST → eval → lower), with small spill-over into `sigil-cli` (the `test` subcommand + `--deny-todo`), the link-assert channel (for `[layout.odd-item]`), and `examples/game/prelude.emp` (ObjDef defaults). Every item: failing test first, minimal implementation, full-workspace green, commit, two-stage review.

**Tech Stack:** Rust workspace (`cargo test --workspace`), designs from `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md @ eeb091e` (D2.29 §4.8, §4.7, D2.30 §5.6, S2-D11 a/d/e, S2-D13h).

**Acceptance gate (whole tranche):** a new acceptance test builds a `code_word→word_offsets`-patched copy of `examples/previews/pitcher_plant_script_next.emp` with **zero diagnostics**; pinned exhibits + the full byte harness stay green after every item. `code_word` itself is deliberately NOT built (rides the first scripted-object port).

**Worktree:** `.worktrees/tranche0-language` (branch `tranche0-language` off master `1fe2406`). Baseline: 1654 tests, 0 failures.

**Standing conventions (apply to every task):**
- Diagnostics: `Level::Error`/`Level::Warning`, id embedded as `[category.id]` prefix in the message string.
- Negative tests assert `diags.iter().any(|d| d.message.contains("[the.id]"))` plus a key phrase — never the full message.
- After each item: `cargo test --workspace --quiet` must be 100% green before commit.
- After each commit: dispatch superpowers:code-reviewer (stage 1), apply/argue findings (stage 2, superpowers:receiving-code-review), commit fixes.

---

### Task 1: `todo!` / `unreachable!` (S2-D11e)

**Design (pinned):** Statement-position `todo!`, `todo!("msg")`, `unreachable!`, `unreachable!("msg")` in proc bodies (and script bodies via the shared `asm_stmt` grammar). Both assemble to the 68k ILLEGAL opcode `$4AFC` (a guaranteed trap) so WIP files build and run to the hole. 68k-only in v1 (`[todo.non-68k]` error in Z80 sections, matching the script/offsets precedent). Each `todo!` site emits a `Level::Warning` `[todo.present]` diagnostic (with the message text if given) — the build succeeds but every hole is named; `unreachable!` emits **no** diagnostic (it is a permanent, intentional trap). A new `--deny-todo` flag on the `emp`/`build` CLI subcommands promotes `[todo.present]` to `Level::Error`.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (in `asm_stmt` — bareword + `!` recognition)
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (new `AsmStmt` variant `Trap { kind: TrapKind, message: Option<String>, span }`)
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs` (emit `$4AFC` big-endian; collect `[todo.present]`)
- Modify: `crates/sigil-cli/src/main.rs` (`--deny-todo` flag threading)
- Test: `crates/sigil-frontend-emp/tests/todo_trap.rs` (new)

- [ ] **Step 1: Write failing tests** in `crates/sigil-frontend-emp/tests/todo_trap.rs` (copy the `lower(src)`/`parse_str(src)` helper shape from `tests/script.rs`):

```rust
// 1. todo! in a proc body emits exactly the ILLEGAL word $4AFC at its position
//    (compare lowered section bytes around it, pattern from lower_code tests).
#[test]
fn todo_lowers_to_illegal_opcode() { /* body: proc p () { nop \n todo! \n nop } → bytes 4E71 4AFC 4E71 */ }

// 2. todo! emits [todo.present] warning carrying the optional message
#[test]
fn todo_reports_present_diagnostic_with_message() { /* todo!("wire the spawn") → warning contains "[todo.present]" and "wire the spawn" */ }

// 3. unreachable! emits the same trap but NO diagnostic
#[test]
fn unreachable_is_silent() { /* assert bytes 4AFC; assert no [todo.present] */ }

// 4. todo! in a Z80 section is [todo.non-68k] error
#[test]
fn todo_in_z80_section_is_error() {}

// 5. script bodies accept todo! (shared asm_stmt grammar)
#[test]
fn todo_inside_script_body_parses_and_lowers() {}
```

- [ ] **Step 2:** Run `cargo test -p sigil-frontend-emp --test todo_trap` — expect FAIL (parse errors on `todo!`).
- [ ] **Step 3:** Implement: parser (recognize `Ident("todo"|"unreachable")` immediately followed by `!` at statement position in `asm_stmt`, before mnemonic dispatch — mnemonics win rule is untouched since no 68k/Z80 mnemonic contains `!`; optional `("msg")` string arg), AST variant, lowering (emit two bytes `0x4A, 0xFC` via the data/instr emission path used by `eval_proc_body`; push `[todo.present]` warning for `todo!` only; `[todo.non-68k]` in Z80 placement).
- [ ] **Step 4:** Run the new test file, then `cargo test --workspace --quiet` — all green.
- [ ] **Step 5:** Add `--deny-todo` to `sigil-cli` (both `emp` and `build` paths): thread a `deny_todo: bool` into lowering (or post-filter: promote diagnostics whose message starts with `[todo.present]` to errors in the CLI driver — choose whichever is smaller in the existing driver shape; post-filter preferred, it keeps the frontend flag-free). Add a CLI-level test in `crates/sigil-cli/tests/` (pattern from `end_to_end.rs`): a module with `todo!` builds exit-0 without the flag, exit-nonzero with it.
- [ ] **Step 6:** Full suite green → commit `feat(emp): todo!/unreachable! statement traps (S2-D11e) — $4AFC ILLEGAL, [todo.present] per site, --deny-todo`.
- [ ] **Step 7:** Two-stage review (code-reviewer subagent → apply findings → commit fixes).

---

### Task 2: `///` doc comments — parse and attach (S2-D11d)

**Design (pinned):** `///` (exactly three slashes then anything) lexes as **doc trivia**, collected by the lexer into a side-channel (`Vec<(Span, String)>` alongside the token stream — comments today are discarded entirely at `lexer.rs:90-92`, and threading a token through every parser site is the risky path). After parsing, an attach pass pairs each contiguous `///` run with the item whose first token starts on the next non-doc line; the text lands on the item as `docs: Option<String>` (lines joined with `\n`, leading `/// `/`///` stripped). Runs that precede no item (end of file, or followed by a non-item line such as an instruction inside a body) attach nowhere and emit a `Level::Warning` `[doc.dangling]`. `//!` stays an ordinary comment (spec: promoting it later is non-breaking; out of scope). No hover/output surface — Spec 3's seam; this item is parse-and-attach + tests only.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lexer.rs` (doc-comment side channel)
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (docs field — on the `Item` wrapper if it's a struct, else a parallel `File.item_docs: Vec<(usize, String)>` map; prefer whichever the AST shape makes additive)
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (attach pass at `parse_file` exit)
- Test: `crates/sigil-frontend-emp/tests/doc_comments.rs` (new)

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn doc_comment_attaches_to_following_item() { /* /// docs\nconst A: u8 = 1 → item A carries "docs" */ }
#[test]
fn multi_line_doc_run_joins() { /* two /// lines → "line1\nline2" */ }
#[test]
fn doc_before_pub_item_attaches() { /* /// d\npub proc … */ }
#[test]
fn dangling_doc_warns() { /* /// d at EOF → [doc.dangling] warning */ }
#[test]
fn ordinary_comments_unaffected() { /* // and /* */ still discarded, no docs */ }
#[test]
fn preview_style_script_doc_attaches() { /* /// above `script brain … {}` */ }
```

- [ ] **Step 2:** Run — expect FAIL (no docs field / no side channel).
- [ ] **Step 3:** Implement lexer split (`//` handler checks for a third `/`; `////…` = 4+ slashes stays ordinary comment, Rust precedent), side channel, attach pass keyed on item start spans.
- [ ] **Step 4:** New tests + full workspace green.
- [ ] **Step 5:** Commit `feat(emp): /// doc comments — lex side-channel + attach-to-item (S2-D11d), [doc.dangling]`.
- [ ] **Step 6:** Two-stage review.

---

### Task 3: struct-literal rest-fill `..` + ObjDef defaults (S2-D13h, struct half)

**Design (pinned — decision made this sprint, flag at checkpoint):** Field **defaults already shipped** (parser `= default` on `StructField`, checked-literal fallback in `eval/literals.rs:139-147`, test `struct_default_fills_omitted_field`). What this task adds:
1. **`..` rest-fill marker** as the last member of a struct literal (`ObjDef{ code: brain, .. }`, optional trailing comma before it): with `..`, omitted defaulted fields fill from their defaults; omitted fields **without** defaults still error `[struct.missing-field]`.
2. **Semantic tightening:** omitting a defaulted field **without** `..` becomes `[struct.missing-field]` too (message extended: "add the field, or elide it explicitly with `..`"). Rationale: elision must be a visible one-token act on the page (the spec's byte-visible-acts taste); silently-filled fields were shipped mechanism, never ratified surface, and nothing outside one unit test uses them. **This changes the tested behavior of `struct_default_fills_omitted_field` — update that test to use `..`, and record the tightening in the tranche notes for Volence's checkpoint.**
3. **Prelude adoption:** `ObjDef.vel` gains default `Vel{ x: 0, y: 0 }`, `ObjDef.frame` gains default `0` in `examples/game/prelude.emp` (`anim` stays required — a real per-object choice). Fix the stale "struct literals have no defaults (Plan 7 ledger item)" comment block (prelude lines 100-104). Byte-neutral: all existing exhibit literals provide every field.
4. `..` in a literal whose struct has **no** defaulted-and-omitted fields is allowed (harmless, supports refactoring); `..` not in last position or repeated = parse error. Comptime-fn **parameter** defaults stay ledgered (D2.22d) — out of scope, note it.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs:1841-1862` (struct-literal member loop: accept `Tok::DotDot` — check the lexer has a `..` token (used by ranges); if ranges lex `..` already, reuse it)
- Modify: `crates/sigil-frontend-emp/src/ast.rs:613-621` (`Expr::StructLit` gains `rest: bool`)
- Modify: `crates/sigil-frontend-emp/src/eval/literals.rs:62-171` (fallback branch keyed on `rest`)
- Modify: `examples/game/prelude.emp:100-122`
- Test: extend `crates/sigil-frontend-emp/tests/eval_data.rs` (beside the existing default tests at :229-249)

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn rest_fill_fills_defaulted_fields() { /* struct S { a: u8, b: u8 = 7 } … S{ a: 1, .. } → b == 7 */ }
#[test]
fn omitted_defaulted_field_without_rest_is_error() { /* S{ a: 1 } → [struct.missing-field] mentioning `..` */ }
#[test]
fn rest_fill_does_not_cover_defaultless_fields() { /* S{ .. } with defaultless a → [struct.missing-field] for a */ }
#[test]
fn rest_not_last_is_parse_error() {}
```

- [ ] **Step 2:** Run — expect FAIL. **Step 3:** Implement. **Step 4:** Update `struct_default_fills_omitted_field` to the `..` spelling; full workspace green (prelude edit must not disturb any pinned exhibit — the acceptance suites prove it).
- [ ] **Step 5:** Commit `feat(emp): struct-literal `..` rest-fill + explicit-elision tightening + ObjDef vel/frame defaults (S2-D13h struct half)`.
- [ ] **Step 6:** Two-stage review.

---

### Task 4: `align N` item opener (D2.29 core)

**Design (from §4.8, verbatim):** `align N` at item position (top level and inside `section {}`) pads the current position to the next multiple of `N` with `$00` fill. `N` comptime-evaluates to a **positive** int (0/negative/non-int = error). Contextual opener per the `equ` precedent: keyword only at item position; `align` stays usable as an ordinary identifier elsewhere. At a **provisional** position (size-relaxable instruction earlier in the section) → `[align.provisional]` error steering toward pinned branch sizes. AS-parity vectors required: the same layout through `sigil-frontend-as`'s `align` and through `.emp` `align` must produce identical bytes (first check how the AS front-end implements `align` — grep `align` in `crates/sigil-frontend-as/src/` — and reuse its padding arithmetic if exposed).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs:209-259` (item dispatch — add the `align` arm after `equ`'s pattern; also add to `recover_to_next_decl`'s opener list at :275-301)
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (new `Item::Align { n: Expr, span }`)
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (+ `layout.rs` position plumbing: `pad = (n - pos % n) % n` zero bytes; provisional check via the same position-tracking `here()` uses — `HerePos.anchor.is_some()` = provisional)
- Test: `crates/sigil-frontend-emp/tests/align.rs` (new) + one AS-parity vector in the harness style (`crates/sigil-harness` or the existing AS-vs-emp comparison tests — mirror how existing parity tests are wired)

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn align_pads_to_boundary_with_zero_fill() { /* data [u8;3] then align 4 then data → second item at 4, gap = 00 */ }
#[test]
fn align_at_aligned_position_emits_nothing() {}
#[test]
fn align_2_is_the_even_translation() { /* odd-length item + align 2 → one 00 byte */ }
#[test]
fn align_non_positive_is_error() { /* align 0 → error naming positive requirement */ }
#[test]
fn align_after_relaxable_branch_is_align_provisional() { /* proc with unsized bra then align → [align.provisional] */ }
#[test]
fn align_as_identifier_still_works() { /* const align = 2 … or a data item named align — per equ's test shape */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** AS-parity vector: assemble a small AS-source fixture with `align 4`/`even` through `sigil-frontend-as`, the equivalent `.emp` through the emp path, byte-compare. **Step 5:** Full workspace green; commit `feat(emp): align N item opener (D2.29) — $00 fill, [align.provisional], AS-parity vectors`.
- [ ] **Step 6:** Two-stage review.

---

### Task 5: `[layout.odd-item]` companion check (D2.29 amendment)

**Design (from §4.8):** Never *inserts* alignment; diagnoses. A 68k `proc` at an odd **final** address = `Level::Error`; a data item whose type carries word/long cells at an odd final address = `Level::Warning`; both messages carry the machine-applicable fix-it text `insert \`align 2\` before …`. Z80 sections exempt; `@as_compat` modules exempt. Final addresses exist only post-link (D2.25 placement fixpoint), so implement on the **link-assert channel**: lowering records one parity assertion per eligible item (cond: `(Sym(item_label) & 1) == 0`, evaluated after final placement — the same machinery deferred `ensure` guards ride, `eval/guards.rs` `LinkAssert`). If `LinkAssert` is error-only today, add a `level` field (additive; existing constructors pass `Error`). Scripts count as procs (they emit code); `offsets` tables get the data-item warning (word cells by construction). Suppress the assertion entirely when the item directly follows an `align 2`+ (cheap: position known even → skip only when the *lowered offset within section* is even AND the section base is pinned-even… simpler and correct: always record; the assert is free at link).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (+ `lower/code.rs`, `lower/data.rs`: record parity asserts per item)
- Modify: the `LinkAssert` type + its link-side evaluator (follow `eval/guards.rs:162-189` → wherever `sigil-link` folds them)
- Test: `crates/sigil-frontend-emp/tests/layout_odd_item.rs` (new; drive via the same full-link path the deferred-guard tests use)

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn odd_proc_is_error_with_fixit() { /* [u8;1] data then proc → error "[layout.odd-item]" + "insert `align 2`" */ }
#[test]
fn odd_word_bearing_data_is_warning() { /* [u8;1] then [u16;2] item → warning */ }
#[test]
fn odd_pure_byte_data_is_silent() { /* [u8;1] then [u8;3] → no diagnostic */ }
#[test]
fn align_2_silences_it() { /* [u8;1], align 2, proc → clean */ }
#[test]
fn z80_and_as_compat_are_exempt() {}
```

- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** Full workspace green — **expect fallout**: any existing test/exhibit with an odd-placed word item now warns; triage each (they are real findings — fix fixtures with `align 2` only where the fixture's intent is layout-neutral; a pinned byte exhibit must NOT gain bytes — exempt via `@as_compat` where applicable or restructure the fixture). **Step 5:** Commit `feat(emp): [layout.odd-item] link-time parity check (D2.29 amendment) — proc=error, wordy data=warning, align-2 fix-it`.
- [ ] **Step 6:** Two-stage review.

---

### Task 6: inline `offsets` bodies (§4.7 mixed form)

**Design (from §4.7, settled 2026-07-06):** Member grammar grows the `data`-item shape: `Name: Type = value` → **inline body**, emitted after the table in declaration order, table word targeting it; `Name: label-expr` (no `=`) → by-reference (today's form), freely mixed. The declared length stays the terminator guard (`[u8; 4] = [7, 0, 1]` = type error, exactly as `data`). **REQUIRED test:** in-block ordinal self-reference — `Shoot: [u8; 6] = [4, 2, 3, 4, $FD, Ani.Idle]` where `Ani.Idle` is ordinal 0 of the *same* block (well-founded: ordinals come from declaration position; register ordinals before evaluating bodies). Inline bodies get hidden per-member labels (hygienic `Name$member` style — the `__here$` precedent); the table's RelOffset cells target them. Parse disambiguation: after `Name:`, speculatively parse a `Type`; if the next token is then `=`, it's inline; otherwise rewind and parse the target expression (the token stream is an indexed Vec — save/restore the position).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs:657-679` (`offsets_decl` member loop)
- Modify: `crates/sigil-frontend-emp/src/ast.rs:248-272` (`OffsetsMember` gains `body: Option<(Type, Expr)>`)
- Modify: `crates/sigil-frontend-emp/src/layout.rs` (`eval_offsets_with_root`) + `crates/sigil-frontend-emp/src/lower/data.rs` (emit bodies after table)
- Test: `crates/sigil-frontend-emp/tests/eval_offsets.rs` + `lower_data.rs` extensions

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn inline_bodies_emit_after_table_in_decl_order() { /* 3 members, 2 inline 1 by-ref: words then bodies; byte-exact expected buffer */ }
#[test]
fn inline_body_table_word_is_self_relative_offset() { /* word == body_pos - table_base */ }
#[test]
fn in_block_ordinal_self_reference() { /* the Shoot/Ani.Idle REQUIRED test — ordinal reads 0 */ }
#[test]
fn short_initializer_is_type_error() { /* [u8;4] = [7,0,1] → the data-item length error, same id */ }
#[test]
fn mixed_inline_and_reference_members() {}
```

- [ ] **Step 2:** FAIL. **Step 3:** Implement (parser speculation, ordinal pre-registration order check, emission). **Step 4:** Full workspace green. **Step 5:** Commit `feat(emp): inline offsets bodies (§4.7 mixed form) — table-then-bodies emission, in-block ordinal self-reference`.
- [ ] **Step 6:** Two-stage review.

---

### Task 7: `yield shows <label>` (D2.30a)

**Design (pinned):** The per-site epilogue override is now spelled `yield shows <label>` (mirroring the header's `shows`); the bare-label form `yield <label>` is **retired**. Grammar after `yield`: kw `shows` → `script_label()` (epilogue override, both `Name` and `.local` accepted, unchanged semantics); `Tok::Dot` → named resume (Task 8 — until Task 8 lands, parse it and emit a temporary "not yet implemented" error only if Task 8 is somehow skipped; in practice do Tasks 7+8 back-to-back but commit separately); bare `Ident` → **targeted parse error**: "`yield <label>` was retired — `yield shows <label>` overrides the epilogue; `yield .label` names the resume point". Header `shows` clause unchanged.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs:955-970` (yield arm of `script_body`)
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (`ScriptStmt::Yield` shape: `epilogue: Option<ScriptLabel>` stays, plus Task 8's `resume: Option<ScriptLabel>` — add both fields now, parser fills `resume` in Task 8)
- Modify: `crates/sigil-frontend-emp/src/lower/script.rs` (desugar reads the new field name — mechanical)
- Test: `crates/sigil-frontend-emp/tests/script.rs` extensions; migrate any existing test using the old `yield Label` spelling

- [ ] **Step 1: Failing tests:** `yield_shows_overrides_epilogue` (bytes identical to the old bare-label form — pin by comparing against a `shows`-in-header variant), `retired_bare_label_yield_is_targeted_error`, `yield_shows_dot_local_works`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement; migrate in-tree uses of the retired spelling (grep `yield [A-Z]` in tests + `examples/`). **Step 4:** Full workspace green (script acceptance byte-pins must be unaffected — the pinned exhibit uses only bare `yield`). **Step 5:** Commit `feat(emp): yield shows <label> per-site epilogue (D2.30a) — bare-label yield retired with fix-it`.
- [ ] **Step 6:** Two-stage review.

---

### Task 8: `yield .label` named resume (D2.30b)

**Design (pinned):** `yield .label` = store **`.label`'s** resume ordinal (pre-scaled by encoding) into the resume slot, `jbra` the epilogue — "frame over; next frame, continue at `.label`". The named target **becomes (or joins)** a resume-table member: first `yield .x` for a user label `.x` appends a table member targeting `.x`'s hygienic name; later `yield .x` reuse it. **No resume point is created at the yield's own site** (code after it is reached only by branching to a label). Target must be a user label defined somewhere in the script body (undefined → error naming the label; the desugar walk already sees all labels — do a pre-pass collecting label names). Member ordering: entry 0, then members in first-need order (bare-yield sites and named targets interleaved as encountered) — the table is hidden, so ordering is unobservable except through bytes; **pin it in a test**. Plus the ratified note-tier lint: bare `yield` immediately followed by `jbra .x` → note suggesting `yield .x` (message id `[script.yield-jbra]`, `Level::Warning`? — no: the spec says *note-tier*; if the Diagnostic type has only Error/Warning, use Warning with a "note:" message prefix and record the tier gap in the tranche notes).
**Frame-count proof obligation:** `yield .x` byte-behavior equals `yield` + `jbra .x` minus the wasted jump — write a test lowering both spellings and asserting the resume table targets differ exactly as designed (old: resume at instruction after yield; new: resume at `.x`).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (fill `resume` on `Tok::Dot` after `yield`)
- Modify: `crates/sigil-frontend-emp/src/lower/script.rs:35-157` + the `Desugar` walk (member allocation keyed by target label; label pre-pass; the lint)
- Test: `crates/sigil-frontend-emp/tests/script.rs` extensions

- [ ] **Step 1: Failing tests:** `yield_dot_label_stores_target_ordinal` (byte-level: the moved immediate is the target member's scaled ordinal), `repeated_yield_dot_same_label_shares_member` (table row count), `yield_dot_undefined_label_is_error`, `yield_dot_creates_no_site_resume_point` (table row count vs bare yield), `bare_yield_jbra_pair_lints`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** Full workspace green — the pinned script exhibit (bare yields only) must be byte-identical. **Step 5:** Commit `feat(emp): yield .label named resume (D2.30b) — shared resume-table members, zero-cost park, yield+jbra lint`.
- [ ] **Step 6:** Two-stage review.

---

### Task 9: `wait_frames #N, <slot>` (D2.30c)

**Design (pinned):** Contextual statement opener in script bodies only (beside `loop`/`yield`). Grammar: `wait_frames #<expr>, <field-displacement-operand>` (e.g. `wait_frames #WAIT_TIME, timer(a0)` — the slot operand reuses the ordinary displacement-operand grammar/typing so `timer(a0)` resolves through the param's `*Sst`). Pure expansion of the documented tick idiom, exactly (same frame accounting as the v1 exhibit — N=64 parks 63 drawn frames and proceeds on the 64th tick):

```
    move.<w> #N, <slot>       ; <w> from the slot field's type (u8→b, u16→w)
.wf$k:                        ; hidden resume-table member (the __here$ hygiene precedent)
    subq.<w> #1, <slot>
    beq     .wfdone$k
    yield   .wf$k             ; self-resuming park (Task 8's machinery)
.wfdone$k:
```

`N` comptime-known → range-check against the slot width and require `N ≥ 1` (`N = 0` would underflow-park ~2^width frames; error). No dispatcher protocol, no value-carrying yields (still 9c-gated). Requires Task 8 (self-resuming `yield .wf$k`) and an epilogue in scope (same `[script.no-epilogue]` rule as bare yield).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (`script_body` arm; new `ScriptStmt::WaitFrames { n: Expr, slot: <operand repr>, span }`)
- Modify: `crates/sigil-frontend-emp/src/ast.rs`, `crates/sigil-frontend-emp/src/lower/script.rs` (desugar to the five-statement expansion, feeding Task 8's member allocation)
- Test: `crates/sigil-frontend-emp/tests/script.rs` extensions

- [ ] **Step 1: Failing tests:** `wait_frames_expands_to_tick_idiom` (byte-compare against the hand-written five-line spelling in a sibling script — MUST be byte-identical), `wait_frames_u16_slot_uses_word_width`, `wait_frames_zero_is_error`, `wait_frames_outside_script_is_error` (proc body → not recognized, ordinary unknown-mnemonic path or targeted error), `wait_frames_needs_epilogue`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** Full workspace green. **Step 5:** Commit `feat(emp): wait_frames #N, <slot> declarative park (D2.30c) — exact tick-idiom expansion`.
- [ ] **Step 6:** Two-stage review.

---

### Task 10: `comptime test` blocks + `sigil test` runner (S2-D11a)

**Design (pinned):** Item-position `comptime test "name" { <comptime statements> }` — colocated with the comptime fns it exercises, **stripped from emission always** (zero bytes, zero cost in normal builds; parsed + type-sanity only). New CLI subcommand `sigil test <entry> [--root/--map …]` (same module-resolution flags as `emp`): compiles the tree, then evaluates each test block's body as a comptime block (the comptime-fn-body evaluator with no params); a failing `ensure`/abort inside = test FAILED (report the guard's message), otherwise ok. Output `test <module>::<name> ... ok|FAILED`, summary line, exit nonzero on any failure. **Negative variant:** `comptime test "name" (expect_error: "[diag.id]") { … }` — the body is *expected* to diagnose: evaluate it capturing diagnostics; PASS iff some captured diagnostic contains the id substring (captured diags are then swallowed); FAIL (with the actual diags echoed) otherwise. This absorbs research T3-g `EXPECT`. Duplicate test names within a module: error. `pub` on a test: error (tests aren't exportable).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (extend the existing `comptime` dual-check at item dispatch: `comptime enum` / `comptime fn` / now `comptime test`)
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (`Item::ComptimeTest { name, expect_error: Option<String>, body, span }`)
- Modify: `crates/sigil-frontend-emp/src/eval/mod.rs` (a `run_tests` entry point walking modules; body eval reusing the comptime-fn machinery with diag capture)
- Modify: `crates/sigil-cli/src/main.rs:29-34` (new `test` subcommand)
- Test: `crates/sigil-frontend-emp/tests/comptime_test.rs` (new) + a CLI test in `crates/sigil-cli/tests/`

- [ ] **Step 1: Failing tests:** `test_block_parses_and_emits_nothing` (module with a test block → identical bytes to without), `passing_test_reported_ok`, `failing_ensure_reported_failed_with_message`, `expect_error_passes_on_matching_diag`, `expect_error_fails_on_clean_body`, `duplicate_test_name_is_error`; CLI: exit 0 all-pass, exit 1 any-fail.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** Full workspace green. **Step 5:** Commit `feat(emp+cli): comptime test blocks + sigil test runner (S2-D11a) — expect_error variant, stripped from emission`.
- [ ] **Step 6:** Two-stage review.

---

### Task 11: Tranche acceptance + notes + checkpoint packet

- [ ] **Step 1:** New acceptance test `crates/sigil-cli/tests/tranche0_acceptance.rs` (pattern from `pitcher_plant_script_acceptance.rs`): read `examples/previews/pitcher_plant_script_next.emp`, substitute the one header line `(encoding: code_word, base: ObjCodeBase)` → `(encoding: word_offsets)` (string-replace, assert exactly one replacement so drift in the preview is caught), write to a temp root beside copies of the game prelude/badniks deps it imports, build via the CLI with **zero diagnostics** asserted. This is the brief's "everything except its `code_word` line builds" demonstrated mechanically.
- [ ] **Step 2:** Full `cargo test --workspace` + `cargo clippy --workspace` + the strict harness gates (the 15 gates from the master validation ritual — see the compression-builtins completion note for the exact commands) — all green.
- [ ] **Step 3:** Write `docs/superpowers/notes/2026-07-09-tranche0-complete.md`: per-item status, decisions made autonomously (the `..` tightening from Task 3; the note-tier gap if hit in Task 8; anything else), review findings summary, fallout triaged in Task 5, and the checkpoint ask.
- [ ] **Step 4:** STOP — Volence checkpoint before merge (per the brief). Do not merge to master.

---

## Self-review notes

- **Spec coverage:** 9 brief items → Tasks 1-10 (D2.29 is two tasks: core + companion check; D2.30 is three tasks; S2-D11a/d/e are Tasks 10/2/1; §4.7 Task 6; S2-D13h Task 3). `code_word` excluded by design. Acceptance = Task 11. ✔
- **Known unknowns flagged inline:** the exact `LinkAssert` extension shape (Task 5), the parser save/restore for type speculation (Task 6), whether `..` already lexes (Task 3), AS-frontend align internals (Task 4). Each task's Step 3 starts by reading the named file region before editing.
- **Order rationale:** 1-2 warm-ups (independent); 3 independent; 4→5 (align before the check that recommends it); 6 independent; 7→8→9 (grammar slot → machinery → consumer); 10 independent; 11 last.
