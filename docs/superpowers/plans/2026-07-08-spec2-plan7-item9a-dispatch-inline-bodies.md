# Spec 2 · Plan 7 #9a — `dispatch` inline member bodies: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the reserved `Member: { … }` dispatch seam (reserved since #6) as D9.1's small increment: an inline body is sugar for an anonymous per-member proc — hygienic label, same encoding row as a named target. NO state/yield semantics (that is 9b).

**Architecture:** Parser grows a `DispatchTarget::Body(Vec<AsmStmt>)` alternative on `DispatchMember` (replacing the reserved-seam parse error). `eval_dispatch_with_root` emits the table row for a body member against a hygienic label (`__dispatch$<module>$<table>$<member>`, the `__here$` `$`-name precedent). `lower_dispatch_item` then lowers each body immediately after the table, in member order, through the SAME `eval_proc_body` + `lower_code_buf` machinery a named proc uses (D-P4.1 — no instruction lowering re-implemented).

**Tech Stack:** Rust workspace, crate `sigil-frontend-emp` (parser.rs, ast.rs, layout.rs, lower/mod.rs, lower/proc.rs), tests in `crates/sigil-frontend-emp/tests/dispatch.rs`.

**Ratified basis:** `docs/superpowers/specs/2026-07-08-spec2-plan7-item9-scripted-states-design-draft.md` D9.1/D9.4 (status RATIFIED). Handoff: `docs/superpowers/notes/2026-07-08-item9-implementation-handoff.md`.

**Worktree:** `/home/volence/sonic_hacks/sigil/.worktrees/plan7-item9` (branch `plan7-item9` off master 359d9cd). All paths below are relative to it. Run all commands from the worktree root.

**Notes file (RED evidence, non-negotiable):** append per-task RED/GREEN evidence to `docs/superpowers/notes/2026-07-08-item9a-implementation-notes.md`, following the format of `docs/superpowers/notes/2026-07-08-here-fix-implementation-notes.md` (task heading → "RED evidence:" bullets citing the exact failing test + failure text, then GREEN results).

---

## Design rulings made in this plan (record in the notes file; spec-review scrutinizes these)

- **R9a.1 — placement:** inline bodies lower immediately AFTER the table, in member declaration order. Deterministic, local, and keeps `word_offsets` rows tiny. Programs without inline bodies are byte-identical to master (probed in Task 5).
- **R9a.2 — hygienic label:** `__dispatch$<module_id>$<table>$<member>` where `module_id = file.module.path.segments.join(".")`. `$` is unlexable by both frontends (the `__here$<module>$<n>` precedent, D-H.8). No counter needed: duplicate members are already a `validate_dispatch` error, and duplicate table names already collide at link's whole-program duplicate-label detection — same-name uniqueness is exactly today's story.
- **R9a.3 — no contract surface on bodies:** no params, no `clobbers(...)`, no `falls_into`. Consequence: NO typed-register bare field access inside a body (`timer(a0)` won't resolve — same as a paramless proc today); qualified `Overlay.field(aN)` still works. A member needing contracts binds a named proc instead. Documented in the spec note (Task 4).
- **R9a.4 — fallthrough lint:** a body that can reach its closing `}` without an unconditional terminator warns `[dispatch.body-fallthrough]` (member-flavored mirror of `[proc.undeclared-fallthrough]`, same last-mnemonic heuristic, silenced under `@as_compat` like the proc lint).
- **R9a.5 — kind check:** a `Body` member skips `[dispatch.target-not-code]` — it is code by construction.
- **R9a.6 — cross-module:** unchanged. `pub dispatch` exports the table NAME only (resolve/imports.rs:78); bodies are module-local and never re-emitted in a consumer.

---

### Task 1: AST + parser — `Member: { … }` parses; table row targets the hygienic label

**Files:**
- Modify: `crates/sigil-frontend-emp/src/ast.rs:293-302` (DispatchMember → add `DispatchTarget` enum)
- Modify: `crates/sigil-frontend-emp/src/parser.rs:648-689` (`dispatch_decl`)
- Modify: `crates/sigil-frontend-emp/src/layout.rs:1620-1696` (`eval_dispatch_with_root`) + add `dispatch_body_label`
- Test: `crates/sigil-frontend-emp/tests/dispatch.rs` (append a new test group)

- [ ] **Step 1: Write the failing tests**

Append to `crates/sigil-frontend-emp/tests/dispatch.rs`:

```rust
// ---- 9. inline member bodies (Plan 7 #9a — D9.1) --------------------------

#[test]
fn inline_body_member_parses_and_lowers_clean() {
    // 9a resolves the seam reserved since #6: `Member: { … }` is sugar for an
    // anonymous per-member proc. Mixing body and label members is legal.
    let src = "\
module m
dispatch Routines (encoding: word_offsets) {
    Init: { rts },
    Wait: wait,
}
proc wait() { rts }
";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let errs: Vec<_> = diags.into_iter().map(|d| d.message).collect();
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sigil-frontend-emp --test dispatch inline_body_member_parses_and_lowers_clean`
Expected: FAIL — the parse error vec contains the reserved-seam diagnostic `dispatch member bodies (`Member: { … }`) are reserved for scripted states (backlog #9) — bind a proc label instead`. Record this exact message as the RED evidence.

- [ ] **Step 3: AST change**

In `crates/sigil-frontend-emp/src/ast.rs`, replace the `DispatchMember` struct (lines 293-302) with:

```rust
/// One `Member: target` / `Member: { … }` entry of a [`DispatchDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchMember {
    /// The member's name (`Name.Member`).
    pub name: String,
    /// The member's right-hand side: a label reference or an inline body.
    pub target: DispatchTarget,
    /// Span of the whole member.
    pub span: Span,
}

/// A dispatch member's right-hand side (Plan 7 #9a — D9.1).
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchTarget {
    /// `Member: target` — a label reference (path / string / comptime expr).
    Label(Expr),
    /// `Member: { … }` — an inline body: sugar for an anonymous per-member
    /// proc (hygienic label, same encoding row as a named target). NO
    /// state/yield semantics — that is 9b's `script` construct (D9.2).
    Body(Vec<AsmStmt>),
}
```

Also update the `DispatchDecl` doc comment just above (ast.rs:255-256): the sentence "…is reserved for a future backlog item (#9) and is a parse error here…" becomes "…is the 9a inline-body form: sugar for an anonymous per-member proc (D9.1)."

- [ ] **Step 4: Parser change**

In `crates/sigil-frontend-emp/src/parser.rs`, `dispatch_decl` (lines 666-680), replace the reserved-seam arm:

```rust
            if self.at(&Tok::LBrace) {
                // 9a (D9.1): `Member: { … }` — an inline body, sugar for an
                // anonymous per-member proc. Same statement grammar as a
                // `proc` body (labels, instruction lines, comptime calls).
                self.bump(); // `{`
                let body = self.asm_body(/* splices_allowed = */ false);
                self.expect(&Tok::RBrace, "`}`");
                members.push(DispatchMember {
                    name: mname,
                    target: DispatchTarget::Body(body),
                    span: mspan.merge(self.prev_span()),
                });
            } else {
                let target = self.expr();
                members.push(DispatchMember {
                    name: mname,
                    target: DispatchTarget::Label(target),
                    span: mspan,
                });
            }
```

Update the `dispatch_decl` doc comment (parser.rs:648-652): "`Member: { ... }` (inline body) is a reserved-but-rejected form (D6.B6), not an alternate member shape" becomes "`Member: { … }` (inline body, 9a — D9.1) parses the same statement grammar as a `proc` body". Import `DispatchTarget` in parser.rs's `use crate::ast::…` list. If `skip_balanced_braces` is now unused (`grep -n skip_balanced_braces crates/sigil-frontend-emp/src/parser.rs`), delete it; if other callers exist, leave it.

- [ ] **Step 5: eval change — table row for a body member**

In `crates/sigil-frontend-emp/src/layout.rs`, add near `eval_dispatch_with_root`:

```rust
/// The hygienic label of a dispatch member's inline body (Plan 7 #9a).
/// `$` is unlexable by both frontends (the `__here$<module>$<n>` precedent,
/// D-H.8), so it can never collide with a user symbol; module+table+member is
/// program-unique (duplicate members are a `validate_dispatch` error, and
/// duplicate table names are whole-program duplicate-label link errors — the
/// same story as the table's own base label today).
pub(crate) fn dispatch_body_label(module: &ast::Path, table: &str, member: &str) -> String {
    format!("__dispatch${}${table}${member}", module.segments.join("."))
}
```

Then in `eval_dispatch_with_root` (layout.rs:1634-1678), wrap the existing target-name extraction — the three existing arms move under `DispatchTarget::Label` UNCHANGED (only `member.target` becomes the bound `target`); the `Body` arm is new and skips the kind check (R9a.5 — code by construction):

```rust
            let name = match &member.target {
                // 9a: an inline body's row targets the anonymous proc's
                // hygienic label; it is code by construction, so the
                // [dispatch.target-not-code] kind check does not apply.
                ast::DispatchTarget::Body(_) => {
                    dispatch_body_label(&file.module.path, &decl.name, &member.name)
                }
                ast::DispatchTarget::Label(target) => match target {
                    ast::Expr::Path(p) => { /* existing arm, verbatim */ }
                    ast::Expr::Str(s, _) => s.clone(),
                    other => { /* existing arm, verbatim */ }
                },
            };
```

- [ ] **Step 6: Fix remaining compile errors mechanically**

`cargo build -p sigil-frontend-emp 2>&1 | head -50` — the compiler lists every other `member.target` consumer. Expected: only parser.rs construction sites (fixed in Step 4) and layout.rs (Step 5); eval/mod.rs indexing (`self.dispatches.insert`, eval/mod.rs:517) is name-based and needs no change. Fix any stragglers by matching on `DispatchTarget::Label`/`Body` with the obvious meaning; do NOT add new behavior here.

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test -p sigil-frontend-emp --test dispatch`
Expected: `inline_body_member_parses_and_lowers_clean` PASS, all pre-existing dispatch tests PASS. (Lowering is clean; the body emits no code yet — that is Task 2. The table row references a not-yet-defined label, which only link would notice; this test deliberately does not link.)

- [ ] **Step 8: Record RED evidence in the notes file, then commit**

```bash
git add crates/sigil-frontend-emp/src/ast.rs crates/sigil-frontend-emp/src/parser.rs \
        crates/sigil-frontend-emp/src/layout.rs crates/sigil-frontend-emp/tests/dispatch.rs \
        docs/superpowers/notes/2026-07-08-item9a-implementation-notes.md
git commit -m "feat(frontend-emp): 9a — dispatch inline member bodies parse (DispatchTarget::Body), rows target hygienic labels"
```

---

### Task 2: Lower inline bodies as anonymous procs after the table

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs:188-201` (top-level call site), `:341-343` (section call site), `:504-542` (`lower_dispatch_item`)
- Modify: `crates/sigil-frontend-emp/src/lower/proc.rs:140-177` (extract terminator check, add member-flavored lint)
- Test: `crates/sigil-frontend-emp/tests/dispatch.rs`

- [ ] **Step 1: Write the failing tests**

Append to `crates/sigil-frontend-emp/tests/dispatch.rs`:

```rust
#[test]
fn inline_body_lowers_after_table_byte_exact() {
    // R9a.1: bodies lower immediately after the table, in member order. The
    // Init row points at the anonymous proc (+4); Wait at the named proc (+6):
    //   table:       00 04  00 06
    //   Init's body: 4E 75          (rts)
    //   wait:        4E 75          (rts)
    let src = "\
module m
dispatch Routines (encoding: word_offsets) {
    Init: { rts },
    Wait: wait,
}
proc wait() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x04, 0x00, 0x06, 0x4E, 0x75, 0x4E, 0x75]);
}

#[test]
fn inline_body_long_ptrs_byte_exact() {
    // Same shape under long_ptrs: 2 rows × 4 bytes, then the bodies.
    //   table: 00 00 00 08  00 00 00 0A
    //   A:     4E 75   b: 4E 75
    let src = "\
module m
dispatch R (encoding: long_ptrs) {
    A: { rts },
    B: b,
}
proc b() { rts }
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0A, 0x4E, 0x75, 0x4E, 0x75]
    );
}

#[test]
fn inline_body_without_terminator_warns_fallthrough() {
    // R9a.4: a body that can reach its `}` without an unconditional terminator
    // warns [dispatch.body-fallthrough] (mirror of [proc.undeclared-fallthrough]).
    let src = "\
module m
dispatch R (encoding: word_offsets) {
    A: { nop },
}
";
    let msgs = msgs(src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("[dispatch.body-fallthrough]")).count(),
        1,
        "msgs: {msgs:?}"
    );
}

#[test]
fn empty_inline_body_emits_row_and_warns() {
    // An empty body is legal (its label sits at whatever follows) but cannot
    // terminate, so the fallthrough warning fires; the table row still emits.
    let src = "\
module m
dispatch R (encoding: word_offsets) {
    A: { },
}
";
    let (module, diags) = lower(src);
    assert!(diags.iter().any(|m| m.contains("[dispatch.body-fallthrough]")), "diags: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02]);
}

#[test]
fn section_nested_inline_body_lowers() {
    let src = "\
module m
section code (vma: $100) {
    dispatch R (encoding: word_offsets) {
        A: { rts },
    }
}
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02, 0x4E, 0x75]);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p sigil-frontend-emp --test dispatch`
Expected: the two byte-exact tests FAIL at link ("undefined symbol `__dispatch$m$Routines$Init`"-class error from `resolve_layout`/`link`, or a byte mismatch), the two warning tests FAIL (no such diagnostic), section test FAILS. Record exact failure text.

- [ ] **Step 3: Extract the terminator check + add the member lint in proc.rs**

In `crates/sigil-frontend-emp/src/lower/proc.rs`, extract the shared core out of `check_undeclared_fallthrough` (lines 146-150) and add the member-flavored lint:

```rust
/// True when the buf's LAST instruction is an unconditional terminator — the
/// shared core of the proc- and dispatch-body fallthrough lints (same
/// last-mnemonic heuristic, S2-D6/D7 defers full reachability).
fn ends_in_terminator(buf: &crate::value::CodeBuf, cpu: Cpu) -> bool {
    buf.items
        .iter()
        .rev()
        .find_map(|it| match it {
            CodeItem::Instr { mnemonic, .. } => Some(mnemonic.as_str()),
            _ => None,
        })
        .is_some_and(|m| is_terminator(m, cpu))
}

/// 9a (R9a.4): a dispatch member's inline body is an anonymous proc with no
/// `falls_into` surface — a body that can reach its closing `}` without an
/// unconditional terminator runs into the next member's body (or whatever
/// follows the dispatch). Member-flavored mirror of
/// [`check_undeclared_fallthrough`]; silenced under `@as_compat` by the caller,
/// like every modernization lint.
pub(super) fn check_member_body_fallthrough(
    table: &str,
    member: &crate::ast::DispatchMember,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    if !ends_in_terminator(buf, cpu) {
        push(
            diags,
            Level::Warning,
            member.span,
            format!(
                "[dispatch.body-fallthrough] dispatch `{table}` member `{}`'s inline body can \
                 reach its closing `}}` without an unconditional terminator — it will run into \
                 whatever follows it",
                member.name
            ),
        );
    }
}
```

Rewrite `check_undeclared_fallthrough`'s lines 146-150 to use `ends_in_terminator(buf, cpu)` (behavior identical).

- [ ] **Step 4: Lower the bodies in `lower_dispatch_item`**

In `crates/sigil-frontend-emp/src/lower/mod.rs`, change `lower_dispatch_item`'s signature and append the body loop after the existing `emit_data` (line 541):

```rust
fn lower_dispatch_item(
    file: &ast::File,
    decl: &ast::DispatchDecl,
    placement: &Placement,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
) {
    // …existing [dispatch.non-68k] guard, eval, stream, define_label,
    // emit_data — UNCHANGED…

    // 9a (D9.1, R9a.1): inline bodies lower immediately after the table, in
    // member order, as anonymous procs — hygienic label, then the SAME
    // eval_proc_body + lower_code_buf path a named proc takes (D-P4.1). No
    // params / clobbers / falls_into surface (R9a.3): a member needing a proc
    // contract binds a named proc instead.
    for member in &decl.members {
        let ast::DispatchTarget::Body(body) = &member.target else { continue };
        let label = crate::layout::dispatch_body_label(&file.module.path, &decl.name, &member.name);
        builder.define_label(&label);
        let (buf, mut ds, next_counter) =
            crate::eval::eval_proc_body(file, &label, &[], body, member.span, *asm_counter, placement.cpu);
        *asm_counter = next_counter;
        diags.append(&mut ds);
        let Some(buf) = buf else { continue };
        lower_code_buf(&buf, placement.cpu, as_compat, builder, diags);
        if !as_compat {
            proc::check_member_body_fallthrough(&decl.name, member, &buf, placement.cpu, diags);
        }
    }
}
```

(Check `eval_proc_body`'s exact param types at `crates/sigil-frontend-emp/src/eval/` before writing — the params argument is the proc-param slice; pass an empty slice `&[]`. Match `lower_proc`'s call at lower/proc.rs:81-82 exactly.)

Update BOTH call sites to pass the two new arguments:
- lower/mod.rs:188-201 (top level): `lower_dispatch_item(file, decl, &Placement { … }, as_compat, &mut builder, &mut diags, &mut asm_counter);`
- lower/mod.rs:341-343 (in `lower_section_items`): `lower_dispatch_item(file, decl, placement, as_compat, builder, diags, asm_counter);`

Also extend `lower_dispatch_item`'s doc comment (lines 504-511) with one sentence: "9a: after the table, each `Member: { … }` inline body lowers as an anonymous proc at `__dispatch$<module>$<table>$<member>`, in member order (R9a.1-R9a.4)."

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p sigil-frontend-emp --test dispatch`
Expected: ALL PASS (new group + all pre-existing).

- [ ] **Step 6: Record RED evidence, then commit**

```bash
git add crates/sigil-frontend-emp/src/lower/mod.rs crates/sigil-frontend-emp/src/lower/proc.rs \
        crates/sigil-frontend-emp/tests/dispatch.rs \
        docs/superpowers/notes/2026-07-08-item9a-implementation-notes.md
git commit -m "feat(frontend-emp): 9a — inline dispatch bodies lower as anonymous per-member procs after the table"
```

---

### Task 3: Coverage — hygiene across bodies, comptime calls, local labels, jbra

**Files:**
- Test: `crates/sigil-frontend-emp/tests/dispatch.rs`

- [ ] **Step 1: Write the tests (some may already pass — that is fine; they pin behavior)**

```rust
#[test]
fn inline_bodies_local_labels_are_hygienic_per_member() {
    // Two bodies each declare `.top` — each body is its own anonymous proc, so
    // the labels must not collide (per-instantiation hygiene, D-P4.6), and a
    // backward jbra inside a body relaxes as usual.
    let src = "\
module m
dispatch R (encoding: word_offsets) {
    A: {
        .top:
        nop
        jbra .top
    },
    B: {
        .top:
        rts
    },
}
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    // table 4 bytes; A: nop (4E 71) + bra.s -4 (60 FC); B: rts (4E 75).
    assert_eq!(
        linked_bytes(&module),
        vec![0x00, 0x04, 0x00, 0x08, 0x4E, 0x71, 0x60, 0xFC, 0x4E, 0x75]
    );
}

#[test]
fn inline_body_statement_comptime_call_expands() {
    // A statement-position comptime call inside a body goes through the same
    // asm{}-instantiation machinery as in a proc (asm_counter threading).
    let src = "\
module m
comptime fn epi() -> Code {
    return asm {
        rts
    }
}
dispatch R (encoding: word_offsets) {
    A: { epi() },
}
";
    let (module, errs) = lower(src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x02, 0x4E, 0x75]);
}

#[test]
fn duplicate_member_with_body_still_errors() {
    // validate_dispatch is target-shape-agnostic: duplicate names error even
    // when one of them is a body member.
    let src = "\
module m
dispatch R (encoding: word_offsets) {
    A: { rts },
    A: a,
}
proc a() { rts }
";
    let msgs = msgs(src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("duplicate dispatch member")).count(),
        1,
        "msgs: {msgs:?}"
    );
}
```

- [ ] **Step 2: Run them; verify expected status**

Run: `cargo test -p sigil-frontend-emp --test dispatch`
Expected: `duplicate_member_with_body_still_errors` PASSES already (validate_dispatch never looks at targets). The other two should PASS if Tasks 1-2 are correct — if either FAILS, that is a real bug in the implementation: STOP, record it in the notes file, and fix (the likely culprits: hygiene counter not threaded per body → label collision; jbra byte expectations wrong → check the relax rung actually chosen with the linked bytes in the failure output).

- [ ] **Step 3: Record outcomes in the notes file (including "already-green" statuses — they are positive controls), then commit**

```bash
git add crates/sigil-frontend-emp/tests/dispatch.rs \
        docs/superpowers/notes/2026-07-08-item9a-implementation-notes.md
git commit -m "test(frontend-emp): 9a — hygiene, comptime-call, duplicate-member coverage for inline bodies"
```

---

### Task 4: Docs — spec §5.5, design doc status, code comments

**Files:**
- Modify: `/home/volence/sonic_hacks/empyrean/docs/SIGIL_SPEC2_LANGUAGE.md:331-335` (the "Reserved seam" paragraph) — **WORKING TREE ONLY, NEVER COMMIT empyrean** (Volence's docs cadence)
- Modify: `docs/superpowers/specs/2026-07-08-spec2-plan7-item9-scripted-states-design-draft.md` (status line)

- [ ] **Step 1: Replace the spec's "Reserved seam" paragraph (empyrean, uncommitted)**

Replace lines 331-335 of `SIGIL_SPEC2_LANGUAGE.md` ("**Reserved seam (backlog #9).** …") with:

```markdown
**Inline member bodies (backlog #9a, shipped 2026-07-08).** `Member: { … }` is sugar for an
anonymous per-member proc: the body lowers immediately after the table (member order) at a
hygienic label (`__dispatch$<module>$<table>$<member>` — `$`-names are unlexable by both
frontends), and the member's row is the ordinary encoding row for that label. Same statement
grammar as a `proc` body (labels, instructions, comptime calls); NO params / `clobbers` /
`falls_into` surface — so no bare typed-register field access inside a body (qualified
`Overlay.field(aN)` works); a member needing a proc contract binds a named proc. A body that
can reach its `}` without an unconditional terminator warns `[dispatch.body-fallthrough]`
(`@as_compat`-silenced). Bodies are code by construction (`[dispatch.target-not-code]` cannot
apply) and are module-local (`pub dispatch` exports the table name only). NO state/yield
semantics here — that is the `script` construct (#9b). First-class continuation engines
(SCE's `move.l #.label, code_addr(a0)`) need proc-name-as-value, not a table — that is the
separate `code: init` bareword deferral, not a dispatch knob.
```

- [ ] **Step 2: Mark 9a shipped in the design doc**

In `docs/superpowers/specs/2026-07-08-spec2-plan7-item9-scripted-states-design-draft.md`, on the D9.1 bullet, append: "**(9a shipped on branch plan7-item9, 2026-07-08 — see the 9a implementation notes.)**"

- [ ] **Step 3: Commit (sigil repo only; verify empyrean stays uncommitted)**

```bash
git -C /home/volence/sonic_hacks/empyrean status --short   # informational only — DO NOT commit
git add docs/superpowers/specs/2026-07-08-spec2-plan7-item9-scripted-states-design-draft.md \
        docs/superpowers/notes/2026-07-08-item9a-implementation-notes.md
git commit -m "docs: 9a — spec §5.5 seam resolved (empyrean working tree), design doc status"
```

---

### Task 5: Gate + byte-diff probe vs master

- [ ] **Step 1: Full workspace gate**

Run: `cargo test --workspace --no-fail-fast 2>&1 | tail -20`
Expected: exactly the 4 allowlisted sigil-harness reds (m0_regions, m1c_vector_table, m1d_debug_rom, m1d_rom — the aeon sound-driver strlen drift), ZERO new failures.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 2: Acceptance exhibit**

Run: `cargo run -p sigil-cli -- emp examples/game/badniks/pitcher_plant.emp --root examples/game --prelude prelude`
Expected: `built: 340 bytes`, exit 0, zero diagnostics.

- [ ] **Step 3: Byte-diff probe vs master** (byte-exactness bar: programs not using inline bodies must be byte-identical)

```bash
cargo run -p sigil-cli -- emp examples/game/badniks/pitcher_plant.emp --root examples/game --prelude prelude -o /tmp/claude-1000/-home-volence-sonic-hacks-sigil/5392e94b-62cf-411c-8872-1db8e3f83a5b/scratchpad/pp_branch.bin
git -C /home/volence/sonic_hacks/sigil stash list  # sanity: main checkout is master, clean
(cd /home/volence/sonic_hacks/sigil && cargo run -p sigil-cli -- emp examples/game/badniks/pitcher_plant.emp --root examples/game --prelude prelude -o /tmp/claude-1000/-home-volence-sonic-hacks-sigil/5392e94b-62cf-411c-8872-1db8e3f83a5b/scratchpad/pp_master.bin)
cmp /tmp/claude-1000/-home-volence-sonic-hacks-sigil/5392e94b-62cf-411c-8872-1db8e3f83a5b/scratchpad/pp_branch.bin /tmp/claude-1000/-home-volence-sonic-hacks-sigil/5392e94b-62cf-411c-8872-1db8e3f83a5b/scratchpad/pp_master.bin && echo BYTE-IDENTICAL
```

Expected: `BYTE-IDENTICAL`. (The `--root examples` corpus is unusable — four pre-existing `module m` collisions; examples/game is the corpus root.)

- [ ] **Step 4: Record the gate results in the notes file, commit the notes**

```bash
git add docs/superpowers/notes/2026-07-08-item9a-implementation-notes.md
git commit -m "docs(notes): 9a gate — workspace green (4 allowlisted reds), clippy clean, exhibit byte-identical to master"
```

---

## Self-review (done at plan-writing time)

- **Spec coverage:** D9.1 fully (sugar, hygienic label, same encoding row, no state semantics); D9.4's "small, own commit(s)" staging honored; watch-outs from the handoff (byte-exactness probe, examples/game root) are Task 5.
- **Placeholders:** none — every step has code or an exact command + expected output.
- **Type consistency:** `DispatchTarget::{Label,Body}` used consistently across Tasks 1-2; `dispatch_body_label` signature identical at definition (Task 1 Step 5) and use (Task 2 Step 4); `check_member_body_fallthrough` params match the call.
- **Known unknowns for the implementer:** exact `eval_proc_body` param types (verify at crates/sigil-frontend-emp/src/eval/ before Task 2 Step 4); whether `Path` is imported in layout.rs's scope for `dispatch_body_label` (it evaluates `file.module.path` — use `ast::Path`); `nop` encoding assumed 4E 71 and `bra.s -4` assumed 60 FC in Task 3 (verify against the backend if the byte test fails — adjust the EXPECTATION only if hand-derivation confirms).
