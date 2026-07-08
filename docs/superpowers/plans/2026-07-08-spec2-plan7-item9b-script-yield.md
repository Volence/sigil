# Spec 2 · Plan 7 #9b — `script`/`yield` MVP: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The ratified coroutine construct (D9.2 + D9.6): `script` bodies with bare `yield`, lowering onto a HIDDEN dispatch-encoded resume table + a typed Sst resume slot; pitcher_plant's brain rewritten as the exhibit alongside the proc version.

**Architecture:** A `script` is a new item that DESUGARS (in a new `lower/script.rs`) into (a) a synthesized hidden dispatch table emitted at the script's name via the SHIPPED `eval_dispatch_with_root` machinery, and (b) ONE flattened proc-shaped body lowered via the SHIPPED `eval_proc_body` + `lower_code_buf` path. `yield` becomes `move.w #<scaled ordinal>, <resume_off>(aP)` + `jbra <epilogue>` + a resume-label definition; `loop { }` becomes a hidden label + `jbra` back. Resume labels ride ORDINARY proc-local label hygiene: the desugar synthesizes `AsmStmt::Label { name: "__resume$<k>" }` entries (a `$`-containing name a user cannot lex), hygiene renames them deterministically to `$<module>$<script>$__resume$<k>`, and the hidden table's rows target those exact strings via `DispatchTarget::Label(Expr::Str(..))` (the Str arm of target extraction takes names verbatim). Single-eval flattening means user labels (`.tick`) work ACROSS yield boundaries for free.

**Tech Stack:** crate `sigil-frontend-emp` (ast.rs, parser.rs, new lower/script.rs, lower/mod.rs, layout.rs untouched); tests in `crates/sigil-frontend-emp/tests/script.rs` (new); exhibit + prelude in `examples/game/`.

**Ratified basis:** design doc D9.2/D9.3(defer)/D9.4/D9.5/D9.6 (status RATIFIED); rulings on the unfrozen surface are delegated (Volence, checkpoint 2026-07-08) and frozen below as R9b.x. 9a landed on this branch (HEAD 946ae04) — `DispatchTarget`, `dispatch_body_label`, `ends_in_terminator` all exist.

**Worktree:** `/home/volence/sonic_hacks/sigil/.worktrees/plan7-item9` (branch `plan7-item9`). Notes file (RED evidence): `docs/superpowers/notes/2026-07-08-item9b-implementation-notes.md` (new), here-fix format.

---

## Design rulings frozen by this plan (R9b.1–R9b.12 — spec-review scrutinizes these)

- **R9b.1 — surface:**
  ```
  script <name> (<params>) (encoding: word_offsets|long_ptrs) [shows <label>] {
      <ScriptStmt>*
  }
  ```
  `ScriptStmt := loop { ScriptStmt* } | yield [<label>] | <AsmStmt>`. Params exactly as `proc` (typed registers). The encoding knob is REQUIRED and reuses `dispatch_encoding_attr` verbatim (same errors) — the table is hidden but the ENGINE dispatcher indexes it, so the encoding is part of the engine contract and may not default (R1: enable, don't impose). `shows`/`yield` labels accept a bare ident (`Draw_Sprite`) or a dot-local (`.rearm`). No `clobbers`/`falls_into` on scripts. `script` is a contextual item opener per the §10 closed-set policy (like `offsets`/`dispatch`).
- **R9b.2 — layout:** the script's name labels the HIDDEN TABLE's first byte; the flattened body follows immediately. Table row k targets resume point k; **member 0 is the script entry** (top of body) — the engine's "initial resume = 0" convention. Ordinals stored by yields are `index × encoding.scale()` (the same pre-scaling as visible dispatch ordinals; long_ptrs scripts store the ×4 ordinal WORD, not a pointer — the slot is uniformly 2 bytes).
- **R9b.3 — resume slot discovery (D9.3):** among the script's params, exactly ONE must be an address register typed `*S` where struct `S` has exactly ONE field of type `Ty::Newtype("ScriptPc")`. That field is the resume slot; its offset comes from `layout_of_struct`. Errors: `[script.no-resume-slot]` (no such param/field), `[script.ambiguous-resume-slot]` (two candidate params, or two ScriptPc fields), `[script.resume-width]` (the field's size ≠ 2).
- **R9b.4 — ScriptPc is a PRELUDE newtype, not a compiler builtin:** `pub newtype ScriptPc = u16` in the game prelude; the construct recognizes the newtype NAME in field-type position. (No builtin named types exist beyond primitives; prelude injection is the established channel. A user redefining ScriptPc gets whatever they declare — the width check still guards the store.)
- **R9b.5 — yield lowering (D9.6):** `yield [label]` desugars, in order, to: `move.w #<ordinal>, <off>(aP)` (synthesized `AsmStmt::Instr`), `jbra <epilogue>` (synthesized exactly as the parser shapes a `jbra <label>` line), then `AsmStmt::Label { name: "__resume$<k>", export: false }`. Per-site `yield <label>` overrides the `shows` epilogue for that site only. A `yield` with NO effective epilogue (no per-site label AND no `shows`) is error `[script.no-epilogue]` — an object that never draws is the footgun (D9.6 verbatim); NEVER a silent rts.
- **R9b.6 — loop lowering:** `loop { … }` desugars to `AsmStmt::Label { name: "__loop$<d>" }` + flattened body + `jbra .__loop$<d>` (dot-local ref through ordinary hygiene). Nesting allowed (`d` = per-script loop counter). No `break` (9c).
- **R9b.7 — the equivalence with `routine` (D9.5):** the game prelude's Sst field `routine: u16 @ $20` is RENAMED to `resume: ScriptPc @ $20` — same offset, same width, so every existing program is byte-identical. The `routine` HELPER keeps its name (manual procs keep using `routine`, D9.5 verbatim) and now writes `Sst.resume`. This makes "the script PC IS the routine pointer storage" literal in the corpus.
- **R9b.8 — hidden means hidden:** no `Name.Member`/`Name.count` ordinals are exposed for scripts (nothing indexes `Item::Script` in `index_items`); the table's base label (the script name) IS exposed (it's the engine's handle; `pub script` exports it like `pub dispatch`).
- **R9b.9 — guards mirror dispatch:** `[script.non-68k]` before any work (Z80 scripts do not exist in v1); fallthrough off the end of the flattened body warns `[script.fallthrough]` (via the shared `ends_in_terminator`, `@as_compat`-gated — parity, not expectation).
- **R9b.10 — no `wait_frames` in 9b:** bare `yield` only (ratified ruling 2); the exhibit hand-writes its timer ticks. Value-carrying yields, `for`, script-calls-script, `break` = 9c (own short design note later, no new ratification needed).
- **R9b.11 — hygiene contract:** resume/loop label names contain `$` so users cannot collide with them; they pass through `LabelScope` like any proc-local label and come out as `$<module>$<script>$__resume$<k>`. The desugar computes the SAME final names for the table rows via the hygiene `Owner` API (`Owner::Proc { module, name }.local_symbol(..)`) — one source of truth; a mismatch is caught by the byte tests as an unresolved link symbol.
- **R9b.12 — exhibit:** `examples/game/badniks/pitcher_plant_script.emp` — a SIBLING module (own `Def`, same art constants pattern), brain as one `script`; the proc version stays untouched and stays byte-identical (its acceptance pin is the regression guard for R9b.7). Equivalence argued in the notes file (state-per-state mapping) + the new exhibit's full image pinned in a CLI acceptance test after hand-verification of the table + resume stores.

---

## Pre-derived byte references (hand-derived at plan time; tests use these)

Minimal module used by T2's byte tests:

```
module m
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
```

**Probe A (word_offsets, one yield):** body `nop / yield / rts`, `shows done`, `proc done() { rts }` after the script.

| offset | bytes | what |
|---|---|---|
| 0 | `00 04  00 0E` | table: entry=+4, resume1=+14 |
| 4 | `4E 71` | nop (entry segment) |
| 6 | `31 7C 00 02 00 20` | move.w #2,$20(a0) — ordinal 1×2 |
| 12 | `60 02` | jbra done → bra.s +2 (done at 16, PC+2=14) |
| 14 | `4E 75` | __resume$1: rts |
| 16 | `4E 75` | done: rts |

Full: `00 04 00 0E 4E 71 31 7C 00 02 00 20 60 02 4E 75 4E 75`.
(move.w #imm,(d16,A0) = 0011 000 101 111100 = 0x317C; imm word; disp word $0020.)

**Probe B (long_ptrs, same body):** rows 4 bytes, ordinals ×4 → stored `#4`:
`00 00 00 08  00 00 00 12  4E 71 31 7C 00 04 00 20 60 02 4E 75 4E 75` (entry at 8, resume1 at $12=18, done at 20 → bra.s at 16, PC+2=18, disp +2).

**Probe C (word_offsets, loop):** body `loop { nop / yield }`, `shows done`:

| offset | bytes | what |
|---|---|---|
| 0 | `00 04  00 0E` | table |
| 4 | `4E 71` | __loop$0: nop |
| 6 | `31 7C 00 02 00 20` | yield store |
| 12 | `60 02` | jbra done (done at 16) |
| 14 | `60 F4` | __resume$1: jbra .__loop$0 → bra.s −12 (target 4, PC+2=16) |
| 16 | `4E 75` | done: rts |

Full: `00 04 00 0E 4E 71 31 7C 00 02 00 20 60 02 60 F4 4E 75`.

If a byte test fails: hand-re-derive BEFORE touching expectation or code; the most likely genuine failure is a resume-label/table-target name mismatch, which appears as an unresolved `$m$...` symbol at link, not a byte diff.

---

### Task 1: AST + parser — `script` parses (decl, `loop`, `yield`, `shows`)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (Item::Script + ScriptDecl/ScriptStmt/ScriptLabel, near ProcDecl)
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (item() dispatch ~line 204 region, OPENERS const ~line 257, new script_decl + script_body fns near proc_decl)
- Create: `crates/sigil-frontend-emp/tests/script.rs`

- [ ] **Step 1: failing tests.** New file `tests/script.rs` with the same helper block as tests/dispatch.rs lines 14-51 (`lower`, `msgs`, `linked_bytes` — copy them; note the doc comment saying they mirror dispatch.rs), plus:

```rust
// ---- 1. parsing (Plan 7 #9b — R9b.1) --------------------------------------

#[test]
fn script_decl_parses_with_loop_yield_and_shows() {
    let src = "\
module m
script brain (a0: *S) (encoding: word_offsets) shows Draw_Sprite {
    nop
    loop {
        .tick:
        subq.b  #1, d0
        yield
        yield .tick
    }
}
";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let Some(sigil_frontend_emp::ast::Item::Script(s)) = file.items.first() else {
        panic!("expected Item::Script, got {:?}", file.items.first())
    };
    assert_eq!(s.name, "brain");
    assert_eq!(s.params.len(), 1);
    assert!(matches!(s.encoding, sigil_frontend_emp::ast::DispatchEncoding::WordOffsets));
    let ep = s.epilogue.as_ref().expect("shows clause");
    assert_eq!((ep.name.as_str(), ep.local), ("Draw_Sprite", false));
    // body: nop, then a loop containing [.tick label, subq, bare yield, yield .tick]
    assert_eq!(s.body.len(), 2);
    let sigil_frontend_emp::ast::ScriptStmt::Loop { body, .. } = &s.body[1] else {
        panic!("expected loop, got {:?}", s.body[1])
    };
    assert_eq!(body.len(), 4);
    assert!(matches!(&body[2],
        sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: None, .. }));
    let sigil_frontend_emp::ast::ScriptStmt::Yield { epilogue: Some(l), .. } = &body[3] else {
        panic!("expected yield .tick, got {:?}", body[3])
    };
    assert_eq!((l.name.as_str(), l.local), ("tick", true));
}

#[test]
fn script_requires_encoding_attr() {
    let src = "\
module m
script s (a0: *S) {
    yield
}
";
    let (_, perrs) = parse_str(src);
    let msgs: Vec<_> = perrs.iter().map(|d| d.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("encoding")),
        "expected the dispatch-style required-encoding error, got: {msgs:?}"
    );
}
```

(Check the actual visibility path for `ast` from integration tests — `sigil_frontend_emp::ast::…` — mirror how tests/parser_decls.rs names AST types and adjust paths accordingly.)

- [ ] **Step 2: run to verify they fail.** `cargo test -p sigil-frontend-emp --test script` → FAIL: `Item::Script` doesn't exist / "expected a declaration" parse error on `script`. Record exact output.

- [ ] **Step 3: AST.** In ast.rs, after `ProcDecl`:

```rust
/// A `script name(params) (encoding: E) [shows label] { body }` declaration
/// (Plan 7 #9b — D9.2/D9.6). A script is a coroutine: `yield` saves a typed
/// resume point (the object's next-frame state) and exits through the
/// per-frame epilogue; the compiler emits a HIDDEN dispatch-encoded resume
/// table at the script's name, followed by the body's resume segments.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptDecl {
    /// Whether the script's table label is exported (`pub script`).
    pub public: bool,
    /// The script's name — the hidden table's base label (the engine handle).
    pub name: String,
    /// Parameters, exactly as [`ProcDecl::params`] (typed register bindings).
    pub params: Vec<(String, Type, Span)>,
    /// The hidden table's emission/ordinal-scaling encoding (required — the
    /// engine dispatcher indexes the table, so this is engine contract).
    pub encoding: DispatchEncoding,
    /// The declared per-frame epilogue (`shows <label>`), overridable per
    /// yield site. A bare `yield` with no epilogue in scope is an error.
    pub epilogue: Option<ScriptLabel>,
    /// The script's statements.
    pub body: Vec<ScriptStmt>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A statement within a `script` body (R9b.1).
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptStmt {
    /// Any ordinary proc-body statement (label / instruction / comptime call).
    Asm(AsmStmt),
    /// `loop { … }` — unconditional loop (hidden label + `jbra` back).
    Loop {
        /// The loop's statements.
        body: Vec<ScriptStmt>,
        /// Span of the whole loop.
        span: Span,
    },
    /// `yield [label]` — save the resume point, exit via the epilogue (D9.6).
    Yield {
        /// Per-site epilogue override; `None` uses the `shows` declaration.
        epilogue: Option<ScriptLabel>,
        /// Span of the statement.
        span: Span,
    },
}

/// An epilogue label reference: `Draw_Sprite` (global) or `.rearm` (local).
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptLabel {
    /// The label name (without the leading dot for locals).
    pub name: String,
    /// True for the `.name` (proc-local) form.
    pub local: bool,
    /// Span of the reference.
    pub span: Span,
}
```

Add `Script(ScriptDecl)` to `enum Item` (after `Proc(ProcDecl)`) with doc `/// `script ...` declaration (Plan 7 #9b).`.

- [ ] **Step 4: parser.** In parser.rs: add to `item()` after the `proc` line: `if self.at_kw("script") { return Some(Item::Script(self.script_decl(public))); }`. Add `"script"` to the `OPENERS` const (array length bumps). Check the recovery logic around line 274 (the #5 lesson: `ensure`-style openers need the `(`-lookahead exception) — `script` is an unconditional opener like `proc`, so it goes in OPENERS only; verify recovery doesn't need a lookahead special-case for it. New fns modeled on `proc_decl`/`dispatch_decl`/`asm_body`:

```rust
    /// Parse a `script name(params) (encoding: E) [shows label] { body }`
    /// declaration (Plan 7 #9b — R9b.1). Params parse exactly as `proc`
    /// params; the `(encoding: E)` attribute is REQUIRED (dispatch's rule —
    /// the hidden table is engine contract); `shows` declares the per-frame
    /// epilogue (D9.6), overridable per yield site.
    fn script_decl(&mut self, public: bool) -> ScriptDecl {
        let start = self.span();
        self.bump(); // `script`
        let name = self.expect_ident("script name");
        self.expect(&Tok::LParen, "`(`");
        let mut params = Vec::new();
        if !self.at(&Tok::RParen) {
            loop {
                let pspan = self.span();
                let pname = self.expect_ident("parameter (register) name");
                self.expect(&Tok::Colon, "`:`");
                let pty = self.ty();
                params.push((pname, pty, pspan));
                if !self.eat(&Tok::Comma) { break; }
                if self.at(&Tok::RParen) { break; } // trailing comma
            }
        }
        self.expect(&Tok::RParen, "`)`");
        let encoding = self.dispatch_encoding_attr();
        let epilogue = if self.eat_kw("shows") { Some(self.script_label()) } else { None };
        self.expect(&Tok::LBrace, "`{`");
        let body = self.script_body();
        self.expect(&Tok::RBrace, "`}`");
        ScriptDecl { public, name, params, encoding, epilogue, body, span: start.merge(self.prev_span()) }
    }

    /// Parse an epilogue label reference: `Draw_Sprite` or `.rearm`.
    fn script_label(&mut self) -> ScriptLabel {
        let start = self.span();
        let local = self.eat(&Tok::Dot);
        let name = self.expect_ident("epilogue label");
        ScriptLabel { name, local, span: start.merge(self.prev_span()) }
    }

    /// Body of a `script` (R9b.1): the `proc` statement grammar plus two
    /// contextual statement openers — `loop { … }` and `yield [label]`.
    /// Neither collides with real code: no 68k/Z80 mnemonic is named `loop`
    /// or `yield`, and a comptime CALL is only recognized with an adjacent
    /// `(` (so a fn named `yield` is unreachable here anyway — fine).
    fn script_body(&mut self) -> Vec<ScriptStmt> {
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) || self.at(&Tok::Eof) { break; }
            if self.at_kw("loop") {
                let start = self.span();
                self.bump(); // `loop`
                self.expect(&Tok::LBrace, "`{`");
                let body = self.script_body();
                self.expect(&Tok::RBrace, "`}`");
                out.push(ScriptStmt::Loop { body, span: start.merge(self.prev_span()) });
                continue;
            }
            if self.at_kw("yield") {
                let start = self.span();
                self.bump(); // `yield`
                let epilogue = if self.at(&Tok::Dot) || matches!(self.peek(), Tok::Ident(_)) {
                    Some(self.script_label())
                } else {
                    None
                };
                self.expect_line_end();
                out.push(ScriptStmt::Yield { epilogue, span: start.merge(self.prev_span()) });
                continue;
            }
            // Everything else is one ordinary proc-body statement. Reuse
            // asm_body by parsing a single statement: factor asm_body's loop
            // body into `asm_stmt(splices_allowed) -> Option<AsmStmt>` and
            // call it from BOTH asm_body and here (do NOT duplicate the
            // label/call/instruction dispatch — extract it).
            if let Some(stmt) = self.asm_stmt(/* splices_allowed = */ false) {
                out.push(ScriptStmt::Asm(stmt));
            }
        }
        out
    }
```

The `asm_stmt` extraction: pull the interior of `asm_body`'s loop (parser.rs:882-910 — the label / statement-call / instr_line dispatch) into a `fn asm_stmt(&mut self, splices_allowed: bool) -> Option<AsmStmt>` returning one statement; `asm_body` keeps its loop + splice-ctx save/restore and calls it. Behavior-identical for procs (whole crate suite is the guard).

CAUTION (yield-epilogue ambiguity): `yield` followed on the SAME line by an ident is a per-site epilogue; a bare `yield` is followed by a newline/`}`. There is no other legal continuation, so the `matches!(self.peek(), Tok::Ident(_))` lookahead is unambiguous. `expect_line_end()` after enforces one-statement-per-line like instructions.

- [ ] **Step 5: compile + make Item::Script inert everywhere it must not crash.** `cargo build -p sigil-frontend-emp` and fix non-exhaustive matches. For THIS task only, `Item::Script` may fall into existing wildcard arms (index_items, validate_*, imports) — lowering support is Task 2; but grep for every `match` over `ast::Item` (`grep -rn "Item::Proc" crates/sigil-frontend-emp/src | grep match -A2` is unreliable — instead `grep -rn "ast::Item::" crates/sigil-frontend-emp/src/{lower,resolve,eval}` and eyeball each site) and leave a `// #9b Task 2` breadcrumb ONLY where lowering will hook in (lower/mod.rs top-level + section loops).

- [ ] **Step 6: run tests.** `cargo test -p sigil-frontend-emp --test script` → both pass. Whole crate + clippy clean.

- [ ] **Step 7: notes (T1 RED/GREEN) + commit.**

```bash
git add crates/sigil-frontend-emp/src/ast.rs crates/sigil-frontend-emp/src/parser.rs \
        crates/sigil-frontend-emp/tests/script.rs \
        docs/superpowers/notes/2026-07-08-item9b-implementation-notes.md
git commit -m "feat(frontend-emp): 9b — script/yield/loop/shows parse (Item::Script, contextual opener)"
```

---

### Task 2: `lower/script.rs` — desugar + hidden table + body lowering (the core)

**Files:**
- Create: `crates/sigil-frontend-emp/src/lower/script.rs`
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (declare module; add `Item::Script` arms to the top-level loop and `lower_section_items`, mirroring the dispatch arms' new 7-arg shape)
- Modify: `crates/sigil-frontend-emp/src/lower/hygiene.rs` ONLY if `Owner`/`local_symbol` need `pub(super)` exposure
- Test: `crates/sigil-frontend-emp/tests/script.rs`

- [ ] **Step 1: failing byte tests** (the pre-derived probes; the shared preamble is a `const PRELUDE_S: &str` holding the `module m` + `newtype ScriptPc` + `struct S` block from "Pre-derived byte references"):

```rust
// ---- 2. lowering: hidden table + resume segments (R9b.2/R9b.5/R9b.6) ------

const SCRIPT_TYPES: &str = "\
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
";

#[test]
fn one_yield_word_offsets_byte_exact() {
    // Probe A (see plan): table [entry=+4, resume1=+14]; yield stores the
    // ×2 ordinal (#2) into resume ($20(a0)) then jbra's the epilogue.
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
    yield
    rts
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x0E, // table
            0x4E, 0x71, // nop
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20, // move.w #2,$20(a0)
            0x60, 0x02, // jbra done → bra.s +2
            0x4E, 0x75, // __resume$1: rts
            0x4E, 0x75, // done: rts
        ]
    );
}

#[test]
fn one_yield_long_ptrs_byte_exact() {
    // Probe B: 4-byte rows; the stored ordinal scales ×4 (#4).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: long_ptrs) shows done {{
    nop
    yield
    rts
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x12, // table
            0x4E, 0x71,
            0x31, 0x7C, 0x00, 0x04, 0x00, 0x20,
            0x60, 0x02,
            0x4E, 0x75,
            0x4E, 0x75,
        ]
    );
}

#[test]
fn loop_desugars_to_label_plus_jbra_back() {
    // Probe C: yield's resume point is the loop-bottom jbra, which jumps
    // back to the hidden loop label (bra.s −12).
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    loop {{
        nop
        yield
    }}
}}
proc done () {{ rts }}
"
    );
    let (module, errs) = lower(&src);
    assert!(errs.is_empty(), "unexpected diagnostics: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![
            0x00, 0x04, 0x00, 0x0E,
            0x4E, 0x71,
            0x31, 0x7C, 0x00, 0x02, 0x00, 0x20,
            0x60, 0x02,
            0x60, 0xF4, // __resume$1: jbra .__loop$0 → bra.s −12
            0x4E, 0x75,
        ]
    );
}

// ---- 3. diagnostics (R9b.3/R9b.5/R9b.9) ------------------------------------

#[test]
fn bare_yield_without_epilogue_errors() {
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) {{
    yield
}}
"
    );
    let msgs = msgs(&src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("[script.no-epilogue]")).count(),
        1,
        "msgs: {msgs:?}"
    );
}

#[test]
fn script_without_scriptpc_field_errors() {
    let src = "\
module m
struct S (size: 2) { x: u16 }
script brain (a0: *S) (encoding: word_offsets) shows done {
    yield done
}
proc done () { rts }
";
    let msgs = msgs(src);
    assert!(
        msgs.iter().any(|m| m.contains("[script.no-resume-slot]")),
        "msgs: {msgs:?}"
    );
}

#[test]
fn script_fallthrough_warns() {
    let src = format!(
        "module m\n{SCRIPT_TYPES}\
script brain (a0: *S) (encoding: word_offsets) shows done {{
    nop
}}
proc done () {{ rts }}
"
    );
    let msgs = msgs(&src);
    assert_eq!(
        msgs.iter().filter(|m| m.contains("[script.fallthrough]")).count(),
        1,
        "msgs: {msgs:?}"
    );
}
```

- [ ] **Step 2: run to verify all fail** (Item::Script currently ignored by lowering → empty output / missing diagnostics). Record exact failures.

- [ ] **Step 3: implement `lower/script.rs`.** Skeleton (adapt names to real APIs — verify `Owner`'s path/visibility in lower/hygiene.rs and the module-string it is built with in eval/asm.rs's `eval_asm_owned` owner construction; they MUST match, byte tests enforce):

```rust
//! Lower an [`Item::Script`](crate::ast::Item::Script) (Plan 7 #9b — D9.2,
//! D9.6, rulings R9b.1–R9b.12 in the 9b plan). A script desugars to:
//!
//! 1. a HIDDEN dispatch-encoded resume table at the script's name (member 0 =
//!    the entry segment; one member per yield), synthesized as a
//!    [`DispatchDecl`] and evaluated by the SHIPPED `eval_dispatch_with_root`
//!    (Str targets carry the resume labels' final hygienic names verbatim);
//! 2. ONE flattened proc-shaped body — `yield` becomes
//!    `move.w #<scaled ordinal>, <resume_off>(aP)` + `jbra <epilogue>` +
//!    a `__resume$<k>` label; `loop {}` becomes `__loop$<d>` + `jbra` back —
//!    lowered through the SHIPPED `eval_proc_body` + `lower_code_buf` path,
//!    so user labels work ACROSS yield boundaries (single hygiene scope).
//!
//! The resume slot (D9.3): the unique `ScriptPc`-typed field of the unique
//! `*Struct` address-register param; the stored value is the member ordinal
//! pre-scaled by the encoding (long_ptrs scripts store the ×4 ordinal WORD —
//! the slot is uniformly 2 bytes; the engine indexes the table with it).

use crate::ast::{self, AsmStmt, DispatchTarget, InstrLine, Operand, ScriptStmt, TextOrSplice};
// … plus eval_proc_body, layout Evaluator access, hygiene Owner, Span/Diagnostic imports
// mirroring lower/mod.rs and lower/proc.rs.

pub(super) fn lower_script_item(
    file: &ast::File,
    decl: &ast::ScriptDecl,
    placement: &super::Placement,
    as_compat: bool,
    builder: &mut sigil_ir::IrBuilder,
    diags: &mut Vec<sigil_span::Diagnostic>,
    asm_counter: &mut u32,
) {
    // 1. Guards: 68k only (mirror [dispatch.non-68k]).
    // 2. Resume slot discovery (R9b.3): scan params for `*S` on an address
    //    register; layout_of_struct(S); collect fields with
    //    Ty::Newtype("ScriptPc"); enforce uniqueness + size == 2.
    //    (Use the same evaluator/layout probe pattern eval_proc_body's param
    //    binding uses — see eval/mod.rs ~1043 — to resolve the type.)
    // 3. Desugar walk: flatten(decl.body) producing (Vec<AsmStmt>, yield_count),
    //    threading a per-script loop counter and yield counter; on Yield:
    //    epilogue = site override or decl.epilogue or error [script.no-epilogue]
    //    (error once per yield site, at the yield's span).
    // 4. Compute resume label FINAL names via the hygiene Owner for
    //    (module_id, decl.name): entry `__resume$0` + one per yield.
    // 5. Synthesize the hidden table: DispatchDecl { name: decl.name.clone(),
    //    encoding: decl.encoding, members: R0..Rn → DispatchTarget::Label(
    //    Expr::Str(final_name, span)) } and lower it EXACTLY as
    //    lower_dispatch_item's table half does (eval_dispatch_with_root →
    //    stream_data → define_label(decl.name) → emit_data).
    // 6. Prepend AsmStmt::Label("__resume$0") to the flattened body; call
    //    eval_proc_body(file, &decl.name, &decl.params, &flat, decl.span,
    //    *asm_counter, placement.cpu); thread counter; lower_code_buf.
    // 7. if !as_compat: [script.fallthrough] via proc::ends_in_terminator
    //    (expose it pub(super) — it is private today).
}
```

Desugar synthesis details (mirror the parser's shapes EXACTLY; write a scratch probe first — parse `"proc p (a0: *S) { jbra done\n jbra .top\n move.w #2, $20(a0) }"` and Debug-print the InstrLines — then encode what you see):
- yield store: `AsmStmt::Instr(InstrLine { mnemonic: vec![TextOrSplice::Text("move".into())], size: Some(TextOrSplice::Text("w".into())), operands: vec![Operand::Imm(<int expr of ordinal>), <the DispInd shape the probe shows for $20(a0)>], span: yield_span })` — the displacement is the NUMERIC field offset (an int literal expr), not the field name, so the store is independent of bare-field-access rules.
- yield jbra + loop-back jbra: whatever operand shape the probe shows for `jbra done` / `jbra .top` (global vs dot-local).
- resume/loop labels: `AsmStmt::Label { name: format!("__resume${k}"), export: false, span }` / `"__loop${d}"` — `$` makes them un-writable by users; hygiene renames both def and the table's Str targets must match (compute via the Owner API, R9b.11).
- loop: `[Label(__loop$d), …body…, Instr(jbra .__loop$d)]`.

lower/mod.rs: `mod script;` + two `ast::Item::Script(decl) => { ensure_default(…); script::lower_script_item(file, decl, &Placement{…}, as_compat, &mut builder, &mut diags, &mut asm_counter); }` arms mirroring the Dispatch arms (top-level ~line 188 region and lower_section_items ~line 341 region — same argument sources as dispatch's).

- [ ] **Step 4: run the tests.** All Task-2 tests pass; whole crate + clippy clean. If a byte test fails with an unresolved `$m$…` symbol: the Owner module-string mismatch (R9b.11) — fix the name computation, never the test.

- [ ] **Step 5: notes (T2 RED/GREEN incl. any probe output that drove the synthesis shapes) + commit.**

```bash
git add crates/sigil-frontend-emp/src/lower/script.rs crates/sigil-frontend-emp/src/lower/mod.rs \
        crates/sigil-frontend-emp/src/lower/proc.rs crates/sigil-frontend-emp/tests/script.rs \
        docs/superpowers/notes/2026-07-08-item9b-implementation-notes.md
git commit -m "feat(frontend-emp): 9b — script lowering: hidden resume table + yield/loop desugar onto proc machinery"
```

---

### Task 3: Coverage — overrides, locals, nesting, guards, edges

**Files:** Test: `crates/sigil-frontend-emp/tests/script.rs` (+ whatever small fixes fall out)

- [ ] **Step 1: add tests** (derive bytes the same way as the probes when byte-exact; otherwise assert diagnostics/link success):

1. `yield_per_site_epilogue_overrides_shows` — `shows done` + one `yield other`; assert the jbra targets `other` (byte-derive: two procs after the script, different offsets — SHOW the arithmetic in a comment).
2. `yield_local_epilogue_resolves` — `yield .fin` with `.fin:` + `rts` at the script's end; errs empty, link succeeds (labels cross yield segments — the single-hygiene-scope guarantee).
3. `nested_loops_get_distinct_labels` — `loop { loop { nop / yield } }`; errs empty; two distinct `__loop$` labels (link succeeds; assert byte length).
4. `user_label_crosses_yield_boundary` — `.tick:` before a yield, `jbra .tick` after it; errs empty (THE load-bearing hygiene property, R9b's single-eval rationale).
5. `zero_yield_script_emits_entry_only_table` — body `rts`, no yield: table = 1 row (entry), then rts; byte-exact (`00 02 4E 75`).
6. `script_in_z80_section_errors` — `[script.non-68k]`, no panic.
7. `script_under_as_compat_silences_fallthrough` — mirror the dispatch @as_compat test's module-attr spelling.
8. `ambiguous_resume_slot_errors` — struct with TWO ScriptPc fields → `[script.ambiguous-resume-slot]`.
9. `resume_width_errors` — `newtype ScriptPc = u32` variant → `[script.resume-width]`.
10. `comptime_call_inside_script_expands` — a `comptime fn` emitting `rts` called on its own line inside the script (statement calls need their own line — known asm_body property).

- [ ] **Step 2: run; diagnose any failure against the code (fix code for real bugs — report DONE_WITH_CONCERNS; fix expectations only with shown arithmetic).**
- [ ] **Step 3: notes + commit** (`test(frontend-emp): 9b — epilogue overrides, hygiene-across-yields, guards, edge coverage`).

---

### Task 4: Game prelude — `ScriptPc` + the `resume` field (R9b.4/R9b.7)

**Files:**
- Modify: `examples/game/prelude.emp` (add `pub newtype ScriptPc = u16`; rename the Sst field `routine: u16 @ $20` → `resume: ScriptPc @ $20`; update the `routine` helper's store to `Sst.resume`)
- Test: existing `crates/sigil-cli/tests/pitcher_plant_acceptance.rs` is the guard — DO NOT EDIT IT.

- [ ] **Step 1: make the three prelude edits** (same offset, same width — D9.5's "same storage", now literal).
- [ ] **Step 2: verify byte-neutrality:** `cargo test -p sigil-cli --test pitcher_plant_acceptance` passes UNCHANGED (340 bytes, same image), and `cargo run -p sigil-cli -- emp examples/game/badniks/pitcher_plant.emp --root examples/game --prelude prelude` → 340 bytes, exit 0.
- [ ] **Step 3: notes + commit** (`feat(game-prelude): 9b — ScriptPc newtype; Sst routine slot becomes resume: ScriptPc (same storage, D9.5)`).

---

### Task 5: The exhibit — pitcher_plant's brain as a script (R9b.12)

**Files:**
- Create: `examples/game/badniks/pitcher_plant_script.emp`
- Create: `crates/sigil-cli/tests/pitcher_plant_script_acceptance.rs` (mirror the existing acceptance test's CLI-invocation shape + `assert_byte_identical` helper)

- [ ] **Step 1: write the exhibit.** A sibling module `module badniks.pitcher_plant_script in obj_bank` with its own consts/`offsets Ani`/ani data/`Def` (mirroring pitcher_plant.emp; `Def.code: brain`) and the brain as ONE script — the state-per-state translation of init/wait/shoot (seed stays a proc; it is a separate object, not a state):

```
script brain (a0: *Sst) (encoding: word_offsets) shows Draw_Sprite {
    move.b  #WAIT_TIME, timer(a0)          // init (entry segment)
    loop {
        .wait_tick:
        subq.b  #1, timer(a0)
        beq     .check
        yield                               // wait a frame…
        jbra    .wait_tick                  // …then keep counting
        .check:
        move.w  Player_1.x_pos, d0
        sub.w   x_pos(a0), d0
        facing_abs d0
        cmp.w   #ATTACK_RANGE, d0
        bhi     .rearm
        move.b  #SHOOT_WINDUP, timer(a0)
        anim    Ani.Shoot
        .windup_tick:
        subq.b  #1, timer(a0)
        cmpi.b  #FIRE_FRAME, timer(a0)
        bne     .no_fire
        spawn(SeedDef, offset: Vec{ x: -16, y: -4 }, flip: inherit)
        .no_fire:
        tst.b   timer(a0)
        beq     .rearmed
        yield
        jbra    .windup_tick
        .rearmed:
        anim    Ani.Idle
        .rearm:
        move.b  #WAIT_TIME, timer(a0)
        yield                               // resume at loop bottom → top
    }
}
proc seed (a0: *Sst) { … verbatim from pitcher_plant.emp … }
```

(vars overlay `PitcherPlantV`/`timer` and all consts as in the proc version; adjust to what actually resolves — the module compiles standalone via `--root examples/game --prelude prelude`.)

- [ ] **Step 2: compile it:** `cargo run -p sigil-cli -- emp examples/game/badniks/pitcher_plant_script.emp --root examples/game --prelude prelude` → exit 0, zero diagnostics. Iterate on the exhibit source (NOT the compiler) until clean; if a COMPILER defect surfaces, stop and report it (DONE_WITH_CONCERNS/BLOCKED) — do not work around silently.
- [ ] **Step 3: pin it:** acceptance test invoking the CLI exactly like pitcher_plant_acceptance.rs, asserting exit 0 + zero diagnostics + the byte LENGTH, plus a structural pin of the first table row bytes (row 0 = entry offset). Full-image hand-derivation is the controller's follow-up (do not fake it) — pin `len` + table row 0 + the presence of exactly `yield_count + 1` table rows via the row bytes.
- [ ] **Step 4: equivalence note.** In the notes file: the state mapping (init→entry segment; wait→`.wait_tick` resume loop; shoot→`.windup_tick` resume loop; `routine wait`/`routine shoot` stores ↔ yield ordinal stores into the SAME `resume @ $20` slot), and what differs (per-frame `jbra Draw_Sprite` now flows through ONE declared epilogue; state ids are hidden ordinals instead of proc addresses).
- [ ] **Step 5: commit** (`feat(examples): 9b exhibit — pitcher_plant brain as a script (alongside the proc version) + acceptance pin`).

---

### Task 6: Docs — spec §5.6, design-doc status

- [ ] **Step 1 (empyrean WORKING TREE — NEVER COMMIT):** add a new `### 5.6 Scripted states (script)` section to `/home/volence/sonic_hacks/empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` after §5.5, covering: the R9b.1 surface; D9.6 epilogue semantics (bare yield without epilogue = error); the hidden-table contract (name = base label, member 0 = entry, stored value = pre-scaled ordinal word for BOTH encodings); the ScriptPc typed-slot contract (prelude newtype, unique-field discovery, width 2, the D9.5 routine-equivalence sentence); what is NOT in 9b (value yields / for / break / script-calls-script = 9c; byte-command DSL gated per D9.3); the diagnostics list. Also update the `§10` construct inventory (add `script` to the contextual-opener list) and the D9-era decision-record row if one exists (else add a D2.24-style row noting 9a+9b shipped 2026-07-08 on branch plan7-item9).
- [ ] **Step 2:** mark D9.2/D9.6 shipped in `docs/superpowers/specs/2026-07-08-spec2-plan7-item9-scripted-states-design-draft.md` (same style as the 9a annotation).
- [ ] **Step 3:** `git -C /home/volence/sonic_hacks/empyrean status --short` (verify uncommitted); commit the sigil-side doc change (`docs: 9b — spec §5.6 script (empyrean working tree), design doc statuses`).

---

### Task 7: Gate + byte-diff probes (controller runs)

- [ ] Full `cargo test --workspace --no-fail-fast` → exactly the 4 allowlisted reds; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Byte-diff probes vs master: pitcher_plant.emp (must be BYTE-IDENTICAL — R9b.7's bar) and any other examples/game entries; the script exhibit exists only on-branch.
- [ ] Notes + commit; then the WHOLE-BRANCH adversarial review (9a+9b together) before calling the branch checkpoint-ready.

---

## Self-review (plan-writing time)

- **Spec coverage:** D9.2 (script surface, hidden table, typed slot, engine-unimposed dispatcher) → T1/T2/T4; D9.6 (epilogue, bare-yield error, per-site override) → T2/T3; D9.4 staging (loop/straight-line, comptime helpers legal, exhibit alongside proc version) → T1/T3/T5; D9.5 equivalence → T4/T5; ratified ruling 5 (both shipped encodings) → T2 probes A+B; D9.3 deferral respected (no DSL anywhere).
- **Placeholders:** Task 2's skeleton deliberately lists numbered responsibilities instead of full Rust (the exact Owner/eval APIs must be read in-tree; the plan pins the CONTRACT and the byte-level outcomes that verify it). Everything else has code or exact commands.
- **Type consistency:** `ScriptDecl`/`ScriptStmt`/`ScriptLabel` shapes match between T1 AST and T1 tests; `lower_script_item`'s 7-arg signature mirrors the post-9a `lower_dispatch_item` (AT the clippy ceiling — if anything needs an 8th arg, bundle into a ProcCtx-style struct instead, per the T2-quality-review flag).
- **Known unknowns for the implementer:** the parser's exact `jbra .top` / `$20(a0)` operand AST shapes (probe-then-mirror is mandated in T2); `Owner`'s visibility + the module-string eval uses (byte tests catch mismatch); the ast-path spelling in integration tests (mirror parser_decls.rs).
