# Item-level guards + data capacity (`ensure` / `max_size`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `ensure(...)`/`ensure_fatal(...)` legal at item position (top level + `section {}` bodies), evaluated in item order at lowering time with `here()` valid — plus an always-on `(max_size: expr)` capacity attribute on `data` items. Closes Plan 7 backlog #5 (research T2-b).

**Architecture:** A new `Item::Ensure(EnsureDecl)` stores the *whole call expression*; lowering evaluates it through the existing evaluator (whose `eval/call.rs` already special-cases `ensure`/`ensure_fatal` — arity, interpolation, and the `aborted` flag all come free), with `here_base = placement.origin + builder.current_offset()` exactly as data items thread it. `ensure_fatal` failure stops lowering the module's remaining items. `max_size` is a `DataDecl` field checked where the checked buffer's byte length is produced. Guards emit zero bytes — byte-neutrality is tested.

**Tech Stack:** Rust workspace (`cargo`). Crates: `sigil-frontend-emp` (parser/AST/eval/lower — all the work), `sigil-cli` (end-to-end tests).

**Green gate before EVERY commit (non-negotiable):**
```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Design doc:** `docs/superpowers/specs/2026-07-07-spec2-plan7-item5-ensure-capacity-design.md` (authoritative for scope/semantics — read it first; D5.5 lists what NOT to build).

**Conventions to respect:**
- Where spec and code disagree, the CODE is authoritative — verify by grep.
- Guards emit NO bytes and must not perturb any offset/label/fixup.
- Out-of-range/overflow is an ERROR (totality), never silent.
- Code blocks below are **EXACT** (verified 2026-07-07, copy modulo line drift) or **MIRROR** (adapt to the cited exemplar's local idiom; the task's test is the contract).
- Line numbers were verified 2026-07-07 against master at `ba3fb98`; re-grep if drifted.

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `crates/sigil-frontend-emp/src/ast.rs` | modify | `Item::Ensure(EnsureDecl)` + `EnsureDecl`; `DataDecl.max_size` field. |
| `crates/sigil-frontend-emp/src/parser.rs` | modify | Contextual `ensure`/`ensure_fatal` item dispatch; `(max_size: expr)` in `data_decl`; `OPENERS` fix. |
| `crates/sigil-frontend-emp/src/eval/guards.rs` | modify | `pub(crate) fn eval_item_guard(...)` harness (mirrors `eval_data_with_root`). |
| `crates/sigil-frontend-emp/src/layout.rs` | modify | `max_size` check where the data `DataBuf` is produced (`eval_data_with_root` path). |
| `crates/sigil-frontend-emp/src/lower/mod.rs` | modify | `Item::Ensure` arms in the top-level loop (~118) and `lower_section_items` (~223); fatal-stop. |
| `crates/sigil-frontend-emp/tests/parser_items.rs` (or the existing parser test file for items) | modify | Parser tests. |
| `crates/sigil-frontend-emp/tests/eval_guards.rs` | modify | `max_size` + item-guard eval tests. |
| `crates/sigil-frontend-emp/tests/lower_guards.rs` | create | Lowering-order, `here()`, fatal-stop, byte-neutrality tests. |
| `crates/sigil-cli/tests/ports.rs` | modify | End-to-end: multi-module prelude-const guard; aeon-shaped guard ports. |
| `examples/guards.emp` | create | Real `.emp` exercising item guards + `max_size`. |

Branch: worktree `sigil/.worktrees/plan7-item5-ensure-capacity`, branched from `master`.

---

## Task 0: Seam re-verification (10 min, no commit)

- [x] Confirm the seams this plan cites still hold (they were live-verified 2026-07-07 on `ba3fb98`):

```bash
grep -n 'expected a declaration' crates/sigil-frontend-emp/src/parser.rs   # item() tail, ~220
grep -n 'fn data_decl' crates/sigil-frontend-emp/src/parser.rs             # ~659
grep -n 'const OPENERS' crates/sigil-frontend-emp/src/parser.rs            # ~239 — note: "offsets" missing
grep -n 'fn eval_guard' crates/sigil-frontend-emp/src/eval/guards.rs       # ~21, has `fatal` + self.aborted
grep -n 'fn eval_data_with_root' crates/sigil-frontend-emp/src/layout.rs   # ~997 — the harness to mirror
grep -n 'here_base = placement.origin' crates/sigil-frontend-emp/src/lower/mod.rs  # ~258
```

If any seam moved materially, note it in the task report; do not redesign.

---

## Task 0.5 (PRE-TASK, audit fix): resolver misses section-nested items under `--root`

From the 2026-07-07 post-merge adversarial audit of #4. In
`crates/sigil-frontend-emp/src/resolve/imports.rs:53-88`, `exported_names`/`defined_names` iterate
only top-level `file.items` — no `ast::Item::Section` arm — so ANY `data`/`proc`/`offsets` nested
inside `section {}` never enters the rename map and `report_unresolved`
(`resolve/mod.rs:318-352`) rejects references to it as `unknown symbol`. Single-file mode works;
`--root` fails. Repro (audit-verified): a section-nested `offsets T { A: X, B: Y }` +
section-nested `data X/Y` → single-file `00 04 00 05 AA BB`, `--root` → 3 unknown-symbol errors
(including the table's OWN base label `T`).

- [x] TDD: failing test in the module-resolution test file (grep `module_resolution` under tests/):
      the repro above via the `--root`/`build_program` driver, asserting the exact 6 bytes; plus a
      section-nested `proc go { jmp Helper }` → sibling section-nested `proc Helper` case.
- [x] Fix: add the `Item::Section(sec)` recursion arm to BOTH collectors (check whether `pub` is
      legal on section-nested items and keep the two collectors' pub-handling consistent).
- [x] Verify the previously-deferred **cross-module offsets target** now works (a `use`d target
      from another module inside an `offsets` block) — byte-check it top-level AND section-nested;
      if some other seam still blocks it, a clean error naming the TARGET (not the base label) is
      the acceptable fallback.
- [x] Green gate + commit: `fix(resolve): recurse into section items in def/export collectors`.

## Task 0.6 (PRE-TASK, audit fix): exported proc labels (`foo.entry`) rejected under `--root`

From the same audit. `report_unresolved` accepts only `$`-prefixed hygiene locals or rename-map
keys; exported labels (`export .entry:` → emitted as dotted `foo.entry`) are neither, so ANY
`--root` build referencing one fails with `unknown symbol \`foo.entry\`` (audit repro: single
module, `proc foo { export .entry: ... bra.w foo.entry }` — single-file gives `60 00 FF FE`,
`--root` errors). Decision (Fable): fix by teaching the RENAME pass dotted symbols — split a
dotted label/fixup-target at the FIRST dot; if the owner segment is in the rename map, rewrite to
`<renamed-owner>.name` on BOTH the definition and reference sides. This (a) fixes the false
rejection, (b) module-qualifies the owner so two modules' private `proc foo { export .entry }`
can no longer collide in the flat link table (the audit's latent finding #2), and (c) makes
cross-module exported-label references (`use a.foo` + `bra.w foo.entry`) resolve for free.
`report_unresolved` then accepts dotted symbols whose owner segment is known.

- [x] TDD: failing tests — (1) the audit repro under `--root` → exact bytes `60 00 FF FE`;
      (2) two modules each with private `proc foo { export .entry: }`, cross-referenced via their
      pub wrappers → no duplicate-symbol link error, byte-verified; (3) cross-module
      `use a.foo` + `bra.w foo.entry` → links correctly (byte-check).
- [x] Fix in `resolve/rename.rs` (+ `report_unresolved`) per the decision above.
- [x] Green gate + commit: `fix(resolve): module-qualify dotted exported labels — Owner.name defs,
      refs, and fixup targets`.

---

## Task 1: AST + parser — item-level guards

**Files:**
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (Item enum ~49-74)
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (`item()` ~192-222, `recover_to_next_decl` OPENERS ~239)
- Test: the existing parser-items test file (grep `Item::Offsets` under `crates/sigil-frontend-emp/tests/` and add alongside)

- [x] **Step 1: Write the failing parser tests** (adapt assertion style to the neighboring `offsets` parser tests):

```rust
#[test]
fn item_level_ensure_parses() {
    let src = "module m\nensure(1 == 1, \"ok\")\ndata T: [u8; 1] = [1]\n";
    // parse(src) per the file's local helper
    // assert: no diagnostics; items contain Item::Ensure(e) with e.fatal == false
    // and e.call matching an Expr::Call whose callee is `ensure`.
}

#[test]
fn item_level_ensure_fatal_parses_in_section() {
    let src = "module m\nsection s\n\nensure_fatal(2 > 1, \"ok\")\n";
    // assert: no diagnostics; the guard item present with fatal == true.
    // NOTE: verify how `section` bodies are represented (marker vs block) by
    // reading section_decl first; place the assertion on whichever item list
    // the guard lands in.
}

#[test]
fn pub_on_guard_is_diagnosed() {
    let src = "module m\npub ensure(true, \"x\")\n";
    // assert: diagnostic "`pub` is not valid on this declaration" (same message
    // as pub use/section, parser.rs:196).
}

#[test]
fn ensure_ident_not_followed_by_paren_still_errors_as_declaration() {
    let src = "module m\nensure\n";
    // assert: "expected a declaration" diagnostic (contextual opener only fires on `(`).
}
```

- [x] **Step 2: Run to verify failure** — `cargo test -p sigil-frontend-emp item_level_ensure` → FAIL (no `Item::Ensure`).

- [x] **Step 3: Add the AST node (EXACT shape, adjust doc style):**

In `ast.rs` after `Item::Newtype` (~73):

```rust
    /// An item-position `ensure(...)` / `ensure_fatal(...)` guard (§6.5, D5.1).
    Ensure(EnsureDecl),
```

And near `DataDecl`:

```rust
/// An item-position guard: `ensure(cond, "msg")` / `ensure_fatal(cond, "msg")`
/// between items. `call` is the WHOLE call expression — evaluation reuses the
/// evaluator's guard special-case (arity, interpolation, `aborted`).
#[derive(Debug, Clone, PartialEq)]
pub struct EnsureDecl {
    /// True for `ensure_fatal`.
    pub fatal: bool,
    /// The full `ensure(...)` call expression.
    pub call: Expr,
    /// Span of the whole item.
    pub span: Span,
}
```

- [x] **Step 4: Parser dispatch (MIRROR of the `comptime` peek2 pattern, parser.rs:208-217):**

In `item()`, before the `section` line (~218):

```rust
        if (self.at_kw("ensure") || self.at_kw("ensure_fatal"))
            && matches!(self.peek2(), Tok::LParen)
        {
            if public {
                let sp = self.prev_span();
                self.diag_at(sp, "`pub` is not valid on this declaration");
            }
            let start = self.span();
            let fatal = self.at_kw("ensure_fatal");
            let call = self.expr(); // parses the whole `ensure(...)` call
            let span = start.merge(self.prev_span());
            self.expect_line_end();
            return Some(Item::Ensure(EnsureDecl { fatal, call, span }));
        }
```

(Verify `self.expr()` starting at the ident indeed yields the `Call` — that is how call expressions parse everywhere else; if the local idiom differs, follow it. Also check whether the `pub`-rejection for `use`/`section` at ~194 should simply gain these two keywords instead — prefer the smallest diff.)

- [x] **Step 5: OPENERS fix (EXACT):** in `recover_to_next_decl` (~239) extend the array:

```rust
        const OPENERS: [&str; 15] = ["use", "const", "enum", "bitfield", "struct",
                                     "vars", "data", "proc", "comptime", "section", "pub",
                                     "newtype", "offsets", "ensure", "ensure_fatal"];
```

(`"offsets"` was missing — pre-existing recovery gap, D5.6.)

- [x] **Step 6: Run tests** — `cargo test -p sigil-frontend-emp` → the new tests PASS, zero regressions.

- [x] **Step 7: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add -A && git commit -m "feat(emp-parse): item-position ensure/ensure_fatal guards + OPENERS recovery fix"
```

---

## Task 2: Lowering — evaluate item guards in order, fatal stops the module

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/guards.rs` (add the harness fn)
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (top-level match ~118-182, `lower_section_items` ~213-245)
- Test: create `crates/sigil-frontend-emp/tests/lower_guards.rs`

- [x] **Step 1: Write the failing tests** (harness: mirror how `lower_module` is driven in existing lower tests — grep `lower_module(` under `tests/`):

```rust
//! Item-position guards: evaluated in item order at lowering time, zero bytes,
//! `ensure_fatal` stops the module's remaining items (D5.2/D5.3).

#[test]
fn failing_top_level_ensure_diagnoses_with_interpolation() {
    // module m / const N = 3 / ensure(N == 4, "want 4, got {N}") / data T: [u8;1] = [1]
    // assert: exactly one diagnostic containing "want 4, got 3".
}

#[test]
fn passing_guards_are_byte_neutral() {
    // Lower the SAME data twice: once with guards interleaved (top level and
    // inside a `section (vma: $8000)` block), once without. Compare the lowered
    // module's sections/bytes/labels/fixups for equality.
}

#[test]
fn here_in_guard_sees_current_position() {
    // section s (vma: $8000) containing: data A: [u8; 4] = [...] then
    // ensure(here() == $8004, "pos {here()}") — assert: no diagnostics.
    // (Check the existing here()-tests for whether interpolating a call is
    // supported; if not, use a plain message — the CONDITION is the contract.)
}

#[test]
fn ensure_fatal_stops_remaining_items() {
    // ensure_fatal(false, "boom") followed by ensure(false, "later").
    // assert: "boom" present, "later" ABSENT.
}

#[test]
fn plain_ensure_failure_continues() {
    // ensure(false, "first") followed by ensure(false, "second").
    // assert: BOTH messages present.
}

#[test]
fn guard_sees_offsets_ordinals() {
    // offsets Idx { A: T1, B: T2 } (+ the two labeled data items) then
    // ensure(Idx.count == 2, "count {Idx.count}") — assert: no diagnostics.
}

#[test]
fn unknown_name_in_guard_condition_diagnoses_without_crash() {
    // ensure(nonexistent > 0, "x") — assert: an unknown-name diagnostic exists;
    // no panic; lowering of following items continues (non-fatal).
}
```

- [x] **Step 2: Run to verify failure** — `cargo test -p sigil-frontend-emp --test lower_guards` → FAIL (guard items currently fall into the `_ => {}` arm, silently ignored).

- [x] **Step 3: Add the evaluation harness (MIRROR of `eval_data_with_root`, layout.rs:997-1010):**

In `eval/guards.rs`:

```rust
/// Evaluate one item-position guard (D5.2). Builds a fresh evaluator over the
/// file (same harness as a data item: eval stack + `here_base`), evaluates the
/// stored call expression — the `ensure`/`ensure_fatal` special-case in
/// `eval/call.rs` does arity/interpolation/abort — and returns
/// `(continue_lowering, diagnostics)`: `continue_lowering` is false only when a
/// failing `ensure_fatal` set the abort flag (D5.3).
pub(crate) fn eval_item_guard(
    file: &crate::ast::File,
    decl: &crate::ast::EnsureDecl,
    here_base: u32,
    include_root: Option<&std::path::Path>,
) -> (bool, Vec<sigil_span::Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.set_here_base(here_base);
        if let Some(root) = include_root {
            ev.set_include_root(root); // use the exact setter eval_data_with_root uses
        }
        let mut env = Env::new(); // match eval_data_with_root's env construction exactly
        let _ = ev.eval_expr(&decl.call, &mut env);
        let aborted = ev.aborted; // if private, add a pub(crate) accessor `fn was_aborted`
        (!aborted, ev.into_diags()) // use the exact diags-draining idiom of eval_data_with_root
    })
}
```

Follow `eval_data_with_root` line-for-line for the harness details (env/type-pass/diag draining); the comments above mark the three places to check.

- [x] **Step 4: Wire the lowering arms.** In `lower/mod.rs` top-level loop, after the `Item::Offsets` arm (~160), replacing coverage the `_ => {}` arm was giving:

```rust
            ast::Item::Ensure(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu, default_name);
                let here = next_lma + builder.current_offset(); // default section: VMA == LMA
                let (cont, mut d) =
                    crate::eval::guards::eval_item_guard(file, decl, here, opts.include_root.as_deref());
                diags.append(&mut d);
                if !cont { break; } // ensure_fatal: stop the module's remaining items (D5.3)
            }
```

CAREFUL with `here` in the default section: data items compute `here_base = placement.origin + builder.current_offset()` where `origin` is `next_lma` (lower/mod.rs:126-128, 258). Match that exactly. In `lower_section_items` (~223) add the same arm with `placement.origin + builder.current_offset()` and return the stop signal to the caller (change the fn to return `bool`, or thread a `&mut bool` — pick the smallest diff; a fatal inside a section block must also stop the module's remaining top-level items).

- [x] **Step 5: Run tests** — `cargo test -p sigil-frontend-emp --test lower_guards` → PASS; full package green.

- [x] **Step 6: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add -A && git commit -m "feat(emp-lower): evaluate item-position guards in order — here()-aware, fatal stops module"
```

---

## Task 3: `(max_size: expr)` on data items

**Files:**
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (`DataDecl` ~263-274)
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (`data_decl` ~659-669)
- Modify: `crates/sigil-frontend-emp/src/layout.rs` (where the data `DataBuf` is produced/returned — inside the `eval_data_with_root` path so ALL data items are covered)
- Test: `crates/sigil-frontend-emp/tests/eval_guards.rs` (append a `max_size` section)

- [x] **Step 1: Write the failing tests:**

```rust
// ---- (max_size:) capacity attribute (D5.4) ------------------------------

#[test]
fn max_size_fitting_is_silent() {
    // data T (max_size: 4): [u8; 4] = [1,2,3,4]  → no diagnostics (== N passes).
}

#[test]
fn max_size_overflow_is_an_error_with_over_by() {
    // data T (max_size: 3): [u8; 5] = [1,2,3,4,5]
    // assert: one diagnostic containing "5 bytes", "max_size 3" and "over by 2 bytes".
}

#[test]
fn max_size_expr_may_reference_a_const() {
    // const BUF = 8 / data T (max_size: BUF): [u16; 4] = [...] → silent.
}

#[test]
fn max_size_negative_is_an_error() {
    // data T (max_size: -1): [u8; 1] = [1] → error naming max_size.
}

#[test]
fn max_size_non_int_is_an_error() {
    // data T (max_size: "big"): [u8; 1] = [1] → error "max_size must be a comptime integer".
}
```

- [x] **Step 2: Run to verify failure** — parse error on `(max_size:` → FAIL as expected.

- [x] **Step 3: AST field (EXACT):** add to `DataDecl`:

```rust
    /// Optional `(max_size: expr)` capacity bound (D5.4): the checked buffer's
    /// byte length must not exceed it. Always-on; overflow is an error.
    pub max_size: Option<Expr>,
```

(Fix all construction sites the compiler flags — tests included — with `max_size: None`.)

- [x] **Step 4: Parser (MIRROR of `struct_decl`'s `(size: expr)` parsing — grep `size:` in `struct_decl` and copy its shape):** in `data_decl` after `let name = ...` (~662):

```rust
        let max_size = if self.at(&Tok::LParen) {
            self.bump();
            self.expect_kw("max_size"); // use the exact kw-expect idiom struct_decl uses for `size`
            self.expect(&Tok::Colon, "`:`");
            let e = self.expr();
            self.expect(&Tok::RParen, "`)`");
            Some(e)
        } else {
            None
        };
```

- [x] **Step 5: Enforcement (in the `eval_data_with_root` path, layout.rs):** after the checked `DataBuf` is produced and its byte length known, when `decl.max_size` is `Some(e)`: evaluate `e` with the same evaluator; require `Value::Int(n)` with `n >= 0` (else the two errors from the tests); if `buf_len > n`, error:

```rust
format!("data `{}` is {} bytes — exceeds max_size {} (over by {} bytes)", decl.name, buf_len, n, buf_len as i128 - n)
```

NOTE: `eval_data_with_root` currently takes a `name: &str` and looks the decl up — thread the decl (or its `max_size`) through whichever way is the smallest diff. Keep the check HERE (not in `lower/mod.rs`) so both top-level and section-nested items are covered by one code path.

- [x] **Step 6: Run tests** — all five PASS; package green.

- [x] **Step 7: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add -A && git commit -m "feat(emp): (max_size:) capacity attribute on data items — always-on over-by-N error"
```

---

## Task 4: Corpus, example, end-to-end

**Files:**
- Modify: `crates/sigil-cli/tests/ports.rs` (follow its existing helpers for compiling multi-file programs — grep `build_program` / `--root` usage)
- Create: `examples/guards.emp`

- [x] **Step 1: Write the failing end-to-end tests:**

```rust
#[test]
fn item_guard_sees_prelude_const_across_modules() {
    // Two files under a temp root (mirror the existing multi-module test setup):
    //   prelude module: `pub const MAX_OBJS = 32`
    //   game module: `ensure(MAX_OBJS % 8 == 0, "objs {MAX_OBJS}")` + one data item.
    // Build via the same driver the item-4 tests use; assert: success, correct bytes.
}

#[test]
fn guards_are_byte_neutral_end_to_end() {
    // Same single-file program with and without interleaved guards + a passing
    // (max_size:) — assert the two output binaries are IDENTICAL.
}

#[test]
fn aeon_shaped_guard_ports() {
    // Transcribe two real aeon guard shapes (divisibility + here()-limit):
    //   ensure(256 % PERIOD == 0, "…") and ensure_fatal(here() <= $8000, "…")
    // in a section with vma set so the here() guard passes; assert success.
}
```

- [x] **Step 2: Run to verify failure**, then make them pass (they should pass immediately if Tasks 1–3 are correct — if so, verify they FAIL when a guard is deliberately flipped, to prove the tests bite).

- [x] **Step 3: Write `examples/guards.emp`** — a small real file: a prelude-style const, a divisibility `ensure`, a `data ... (max_size: ...)` around a folded table, an `offsets` block guarded by `.count`, one `ensure_fatal(here() <= ...)` in a `vma:` section. Confirm `cargo run -p sigil-cli -- emp examples/guards.emp --hex` succeeds.

- [x] **Step 4: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add -A && git commit -m "test(emp): item-guard corpus — multi-module prelude guard, byte-neutrality, aeon-shaped ports; guards example"
```

---

## Task 5: Whole-branch review + handoff

- [ ] **Step 1: Two-stage review** (the established process): dispatch `superpowers:code-reviewer` for (a) design-doc compliance (D5.1–D5.6 — especially that NOTHING from D5.5 was built), then (b) code quality. Fix findings; green gate + commit per fix.
- [ ] **Step 2: Adversarial pass:** construct and run programs probing: guard as the FIRST item (before any section — `here()` in the default section), guard as the LAST item, `ensure_fatal` inside a section block stopping later top-level items, a guard whose message itself fails to interpolate (must diagnose, not crash — eval_guards.rs:104 already covers fn-body; verify item-level), `data` named `ensure` (contextual-opener non-regression), and `max_size` combined with a guard on the same item.
- [ ] **Step 3: Verify the s4.bin harness is untouched:** `cargo test -p sigil-cli` (the `m1d_rom`/`m1d_debug_rom` tests) — green.
- [ ] **Step 4: Write the completion handoff note** `docs/superpowers/notes/2026-07-07-spec2-plan7-item5-complete-handoff.md` (mirror item 3's completion note: what shipped, decisions taken, spec-delta text for Fable to lift, what #6 walks in with).
- [ ] **Step 5: STOP — do NOT merge to master.** Report back for the Volence/Fable checkpoint (established cadence).
