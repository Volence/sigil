# Bidirectional offset-table (`offsets`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `.emp` `offsets Name { Variant: target, ... }` construct — it emits `dc.w target−base` self-relative word offsets (forward) and introduces named comptime ordinal constants `Name.Variant`/`Name.count` (reverse) — unblocking the Plan-6 data ports.

**Architecture:** A new fixed-width `FixupKind::RelWord16Be` carries the offset as a symbol *difference* in its existing `Expr` target (`Sub(Sym(target), Sym(base))`), resolved at link with a signed-word range check. The `.emp` front end parses the block into a new `OffsetsDecl` AST item, lowers the forward direction through a new `Cell::RelOffset` into that fixup, and resolves the reverse direction (ordinals) as comptime member access — mirroring how enums resolve `Enum.Variant`. Byte output is cross-checked against the AS front end / hand-computed golden.

**Tech Stack:** Rust workspace (`cargo`). Crates: `sigil-ir` (IR + fixups), `sigil-link` (linker/relocation), `sigil-frontend-emp` (the `.emp` front end: lexer/parser/AST/eval/lower), `sigil-frontend-as` (the AS reference front end), `sigil-cli` (end-to-end + byte-diff tests).

**Green gate before EVERY commit (non-negotiable):**
```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Design doc:** `docs/superpowers/specs/2026-07-06-offset-table-design.md` (authoritative for scope/semantics).

**Conventions to respect:**
- Where spec and code disagree, the CODE is authoritative — verify by grep.
- `RelWord16Be` is a FIXED-width fixup (always 2 bytes) — it needs NO relaxation logic (unlike `Fragment::RelaxAbsSym`); it flows through as a plain `DataFragment` + fixup like `Abs16Be`.
- Out-of-range is an ERROR (totality), never a silent truncation.
- Representative code blocks below are marked **EXACT** (verified against the file, copy as-is modulo line drift) or **MIRROR** (adapt to the local idiom of the cited exemplar; the task's test is the exact contract).

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `crates/sigil-ir/src/fixup.rs` | modify | Add `FixupKind::RelWord16Be` + `byte_width` arm + doc + unit test. |
| `crates/sigil-link/src/lib.rs` | modify | `apply_fixup`: add the `RelWord16Be` resolution arm (range check + i16 BE write) + unit tests. |
| `crates/sigil-frontend-emp/src/value.rs` | modify | Add `Cell::RelOffset { base, target }` + `byte_size` arm. |
| `crates/sigil-frontend-emp/src/lower/data.rs` | modify | `stream_data`: handle `Cell::RelOffset` → `RelWord16Be` fixup (68k-only) + unit test. |
| `crates/sigil-frontend-emp/src/ast.rs` | modify | Add `Item::Offsets(OffsetsDecl)`, `OffsetsDecl`, `OffsetsMember`. |
| `crates/sigil-frontend-emp/src/parser.rs` | modify | `item()` dispatch + `offsets_decl()` block parser + parser test. |
| `crates/sigil-frontend-emp/src/eval/mod.rs` | modify | Index `offsets` decls; add `eval_offsets()` producing a `DataBuf` of `RelOffset` cells. |
| `crates/sigil-frontend-emp/src/eval/expr.rs` | modify | `eval_path`: resolve `Name.Variant`→ordinal, `Name.count`→len. |
| `crates/sigil-frontend-emp/src/lower/mod.rs` | modify | `Item::Offsets` lowering arm: eval → `stream_data` → `define_label` → `emit_data`. |
| `crates/sigil-cli/tests/ports.rs` | modify | Byte-diff vs AS / golden (incl. negative offset); end-to-end + totality + ordinal integration tests. |
| `examples/offset_table.emp` | create | A real `.emp` exercising forward + reverse. |
| `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` | modify | Spec entry freezing the `offsets` construct. |

---

## Task 1: `FixupKind::RelWord16Be` in `sigil-ir`

**Files:**
- Modify: `crates/sigil-ir/src/fixup.rs` (enum ~7-35, `byte_width` ~40-47, tests ~60-80)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/sigil-ir/src/fixup.rs`:

```rust
#[test]
fn rel_word_16_be_is_two_bytes() {
    assert_eq!(FixupKind::RelWord16Be.byte_width(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-ir rel_word_16_be_is_two_bytes`
Expected: FAIL — `no variant named RelWord16Be`.

- [ ] **Step 3: Add the variant (EXACT)**

In the `FixupKind` enum, after the `Abs32Be` variant (line ~19), add:

```rust
    /// A self-relative signed **word** offset (`dc.w Target-Base`): the offset
    /// table idiom. Unlike [`Abs16Be`](Self::Abs16Be) (an absolute address
    /// truncated to 16 bits), this writes a *signed relative displacement* — the
    /// [`Fixup`]'s `target` is a symbol **difference** (`Sub(Sym(t), Sym(base))`),
    /// so the folded value IS the offset. Range i16 (`[-0x8000, 0x7FFF]`),
    /// big-endian; overflow is an error (totality). Fixed width — no relaxation.
    RelWord16Be,
```

In `byte_width`, add `RelWord16Be` to the width-2 arm (line ~42):

```rust
            FixupKind::BankPtr16Le
            | FixupKind::BankPtr16Be
            | FixupKind::Abs16Be
            | FixupKind::RelWord16Be => 2,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sigil-ir rel_word_16_be_is_two_bytes`
Expected: PASS.

- [ ] **Step 5: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-ir/src/fixup.rs
git commit -m "feat(ir): RelWord16Be fixup kind (self-relative word offset)"
```

---

## Task 2: Link resolution for `RelWord16Be`

**Files:**
- Modify: `crates/sigil-link/src/lib.rs` (`apply_fixup` match, arms end ~277-280; tests module ~414+)

Context: `apply_fixup` folds `fx.target` into `value: i64` at the top, then matches `fx.kind`. For `RelWord16Be`, `fx.target` is `Sub(Sym(target), Sym(base))`, so `value` is already the offset. Test construction mirrors `pcrel_disp16_measured_from_extension_word` (lib.rs:517) and `pcrel8_out_of_range_diagnoses` (lib.rs:551).

- [ ] **Step 1: Write the failing tests (MIRROR the two cited exemplars)**

Add to the `tests` module in `crates/sigil-link/src/lib.rs`:

```rust
#[test]
fn rel_word_16_be_writes_symbol_difference() {
    // base at offset 0, target at offset 6; word[0] = target - base = 6.
    let sec = Section {
        name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
        labels: vec![
            Label { name: "Base".into(), offset: 0 },
            Label { name: "Tgt".into(), offset: 6 },
        ],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0x00, 0x00],
            fixups: vec![Fixup {
                kind: FixupKind::RelWord16Be,
                offset: 0,
                target: Expr::Binary {
                    op: sigil_ir::expr::BinOp::Sub,
                    lhs: Box::new(Expr::Sym("Tgt".into())),
                    rhs: Box::new(Expr::Sym("Base".into())),
                },
            }],
            span: span(),
        })],
    };
    let linked = link(&[sec], &SymbolTable::new()).unwrap();
    assert_eq!(linked.section("c").unwrap().bytes, vec![0x00, 0x06]);
}

#[test]
fn rel_word_16_be_negative_offset_two_complement() {
    // target BEFORE base: Tgt at 0, Base at 4 → offset -4 = 0xFFFC.
    let sec = Section {
        name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
        labels: vec![
            Label { name: "Tgt".into(), offset: 0 },
            Label { name: "Base".into(), offset: 4 },
        ],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0x00, 0x00],
            fixups: vec![Fixup {
                kind: FixupKind::RelWord16Be,
                offset: 0,
                target: Expr::Binary {
                    op: sigil_ir::expr::BinOp::Sub,
                    lhs: Box::new(Expr::Sym("Tgt".into())),
                    rhs: Box::new(Expr::Sym("Base".into())),
                },
            }],
            span: span(),
        })],
    };
    let linked = link(&[sec], &SymbolTable::new()).unwrap();
    assert_eq!(linked.section("c").unwrap().bytes, vec![0xFF, 0xFC]);
}

#[test]
fn rel_word_16_be_overflow_diagnoses() {
    // Base at 0, target at 0x8000 → +32768 exceeds +0x7FFF → error.
    let sec = Section {
        name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
        labels: vec![
            Label { name: "Base".into(), offset: 0 },
            Label { name: "Far".into(), offset: 0x8000 },
        ],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0x00, 0x00],
            fixups: vec![Fixup {
                kind: FixupKind::RelWord16Be,
                offset: 0,
                target: Expr::Binary {
                    op: sigil_ir::expr::BinOp::Sub,
                    lhs: Box::new(Expr::Sym("Far".into())),
                    rhs: Box::new(Expr::Sym("Base".into())),
                },
            }],
            span: span(),
        })],
    };
    let err = link(&[sec], &SymbolTable::new()).unwrap_err();
    assert!(err.iter().any(|d| d.message.contains("signed-word range")), "got: {:?}", err);
}
```

Note: if `BinOp` is already imported in the test module, drop the `sigil_ir::expr::` prefix. Verify the exact import path with `grep -n "use .*BinOp\|expr::BinOp" crates/sigil-link/src/lib.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sigil-link rel_word_16_be`
Expected: FAIL — non-exhaustive match / unknown variant.

- [ ] **Step 3: Add the resolution arm (EXACT)**

In `apply_fixup`, before the `FixupKind::HeaderChecksum` arm (lib.rs:277), add:

```rust
        FixupKind::RelWord16Be => {
            // A self-relative signed word offset (`dc.w Target-Base`): `target`
            // is a symbol difference, so `value` is already the offset. Range i16.
            if !(-0x8000..=0x7FFF).contains(&value) {
                diags.push(diag(
                    format!("offset out of signed-word range ({value}) in section {section}"),
                    span,
                ));
                return;
            }
            let w = value as i16 as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
```

(If `value` is not already `i64` at this point, mirror `Abs16Be`'s `let v = value as i64;` and check `v`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sigil-link rel_word_16_be`
Expected: PASS (all three).

- [ ] **Step 5: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-link/src/lib.rs
git commit -m "feat(link): resolve RelWord16Be (symbol-difference word offset, i16 range-checked)"
```

---

## Task 3: `Cell::RelOffset` + `stream_data` handling

**Files:**
- Modify: `crates/sigil-frontend-emp/src/value.rs` (`Cell` enum ~139-178, `byte_size` ~183-188)
- Modify: `crates/sigil-frontend-emp/src/lower/data.rs` (`stream_data` ~44-70; add a `tests` module if none)

- [ ] **Step 1: Write the failing test**

Add a test module at the end of `crates/sigil-frontend-emp/src/lower/data.rs` (or extend the existing one):

```rust
#[cfg(test)]
mod rel_offset_tests {
    use super::*;
    use crate::value::{Cell, DataBuf};
    use sigil_ir::backend::Cpu;
    use sigil_ir::{expr::BinOp, Expr, FixupKind};
    use sigil_span::Span;

    #[test]
    fn rel_offset_emits_relword16be_symbol_difference() {
        let mut buf = DataBuf::empty();
        buf.push(Cell::RelOffset { base: "Tbl".into(), target: "Frame0".into() });
        let (bytes, fixups, diags) = stream_data(&buf, Cpu::M68000, Span::default());
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        assert_eq!(bytes, vec![0x00, 0x00], "reserves a 2-byte hole");
        assert_eq!(fixups.len(), 1);
        assert_eq!(fixups[0].kind, FixupKind::RelWord16Be);
        assert_eq!(fixups[0].offset, 0);
        assert_eq!(
            fixups[0].target,
            Expr::Binary {
                op: BinOp::Sub,
                lhs: Box::new(Expr::Sym("Frame0".into())),
                rhs: Box::new(Expr::Sym("Tbl".into())),
            }
        );
    }

    #[test]
    fn rel_offset_in_z80_section_diagnoses() {
        let mut buf = DataBuf::empty();
        buf.push(Cell::RelOffset { base: "Tbl".into(), target: "Frame0".into() });
        let (bytes, _fixups, diags) = stream_data(&buf, Cpu::Z80, Span::default());
        assert_eq!(bytes.len(), 2, "still reserves the hole so sizes line up");
        assert!(diags.iter().any(|d| d.message.contains("offset table")), "got: {diags:?}");
    }
}
```

(Verify `Span::default()` exists; else use the crate's span-construction helper — grep `fn span(` in this crate's tests.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-frontend-emp rel_offset`
Expected: FAIL — `no variant named RelOffset`.

- [ ] **Step 3: Add the `Cell::RelOffset` variant (EXACT)**

In `crates/sigil-frontend-emp/src/value.rs`, inside `enum Cell` after `SymRef { ... }` (line ~177):

```rust
    /// A self-relative signed **word** offset for an `offsets` table entry:
    /// emits `dc.w target - base` (2 bytes) via a `RelWord16Be` fixup. Distinct
    /// from `SymRef` (an absolute pointer) — this is a symbol *difference*.
    RelOffset {
        /// The table's base symbol (the offsets block's own label).
        base: String,
        /// The entry's target symbol.
        target: String,
    },
```

In `impl Cell { fn byte_size }` (line ~183), add `RelOffset` to the width-2 result:

```rust
    pub fn byte_size(&self) -> usize {
        match self {
            Cell::Scalar { width, .. } | Cell::SymRef { width, .. } => *width as usize,
            Cell::RelOffset { .. } => 2,
            Cell::Bytes(b) => b.len(),
        }
    }
```

- [ ] **Step 4: Handle it in `stream_data` (EXACT)**

In `crates/sigil-frontend-emp/src/lower/data.rs`, add a match arm in `stream_data`'s `for cell in &buf.cells` loop (after the `SymRef` arm, ~line 68):

```rust
            Cell::RelOffset { base, target } => {
                // 68k big-endian signed word only (first cut); Z80 diagnosed.
                if cpu != Cpu::M68000 {
                    diags.push(err(
                        span,
                        "[offsets.non-68k] an offset table is a 68k word-offset idiom; \
                         Z80 offset tables are not supported"
                            .to_string(),
                    ));
                    bytes.resize(bytes.len() + 2, 0);
                    continue;
                }
                fixups.push(Fixup {
                    kind: FixupKind::RelWord16Be,
                    offset: bytes.len() as u32,
                    target: Expr::Binary {
                        op: BinOp::Sub,
                        lhs: Box::new(Expr::Sym(target.clone())),
                        rhs: Box::new(Expr::Sym(base.clone())),
                    },
                });
                bytes.resize(bytes.len() + 2, 0);
            }
```

(`Expr`, `Fixup`, `FixupKind`, `BinOp` are already imported at the top of `data.rs` — verified lines 25-26.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p sigil-frontend-emp rel_offset`
Expected: PASS.

- [ ] **Step 6: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-frontend-emp/src/value.rs crates/sigil-frontend-emp/src/lower/data.rs
git commit -m "feat(emp-lower): Cell::RelOffset streams to a RelWord16Be symbol-difference fixup"
```

---

## Task 4: AST + parser for the `offsets` block

**Files:**
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (`Item` enum ~49-72; add decl structs near `StructDecl` ~174-202)
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (`item()` dispatch ~192-221; add `offsets_decl()` near `struct_decl` ~556-590)

- [ ] **Step 1: Write the failing parser test**

Find how this crate's parser tests parse a source string (grep `fn parse_str\|parse_module\|mod tests` in `parser.rs` / `lib.rs`). Add, mirroring an existing item-parse test:

```rust
#[test]
fn parses_offsets_block() {
    let src = "offsets Map { Idle: frame_idle, Shoot: frame_shoot }";
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != Level::Error), "parse errors: {diags:?}");
    let item = file.items.iter().find_map(|it| match it {
        Item::Offsets(o) => Some(o),
        _ => None,
    }).expect("an Offsets item");
    assert_eq!(item.name, "Map");
    assert_eq!(item.members.len(), 2);
    assert_eq!(item.members[0].name, "Idle");
    assert_eq!(item.members[1].name, "Shoot");
}
```

(Match the exact `parse_str` return shape and `Item`/`Level` imports used by neighboring parser tests.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-frontend-emp parses_offsets_block`
Expected: FAIL — `no variant named Offsets`.

- [ ] **Step 3: Add the AST types (EXACT)**

In `crates/sigil-frontend-emp/src/ast.rs`, add to the `Item` enum (after `Struct(StructDecl)`, line ~53):

```rust
    Offsets(OffsetsDecl),
```

Add near `StructDecl` (line ~187):

```rust
/// An `offsets Name { Variant: target, ... }` block: a bidirectional offset
/// table. Forward: emits `dc.w target - Name` per member. Reverse: introduces
/// the comptime ordinal constants `Name.Variant` (0-based) and `Name.count`.
#[derive(Clone, Debug)]
pub struct OffsetsDecl {
    pub public: bool,
    pub name: String,
    pub members: Vec<OffsetsMember>,
    pub span: Span,
}

/// One `Variant: target` entry of an [`OffsetsDecl`].
#[derive(Clone, Debug)]
pub struct OffsetsMember {
    /// The ordinal's name (`Name.Variant`).
    pub name: String,
    /// The target label reference (a path expression).
    pub target: Expr,
    pub span: Span,
}
```

(Match the `#[derive(...)]` set on `StructDecl` in this file — copy whatever it uses.)

- [ ] **Step 4: Add the parser dispatch + block parser (MIRROR `struct_decl`)**

In `parser.rs` `item()` (after the `struct` line ~199):

```rust
    if self.at_kw("offsets") { return Some(Item::Offsets(self.offsets_decl(public))); }
```

Add the `offsets_decl` method, modeled on `struct_decl` (parser.rs:556) — no `(size:)` attribute, and each member is `Ident: expr` instead of `Ident: Type`:

```rust
fn offsets_decl(&mut self, public: bool) -> OffsetsDecl {
    let start = self.span();
    self.bump(); // `offsets`
    let name = self.expect_ident("offsets name");
    self.expect(&Tok::LBrace, "`{`");
    let mut members = Vec::new();
    loop {
        self.skip_newlines();
        if self.at(&Tok::RBrace) { break; }
        let mspan = self.span();
        let mname = self.expect_ident("offset entry name");
        self.expect(&Tok::Colon, "`:`");
        let target = self.expr();
        members.push(OffsetsMember { name: mname, target, span: mspan });
        self.skip_newlines();
        if !self.eat(&Tok::Comma) { break; }
        self.skip_newlines();
        if self.at(&Tok::RBrace) { break; } // trailing comma
    }
    self.skip_newlines();
    self.expect(&Tok::RBrace, "`}`");
    OffsetsDecl { public, name, members, span: start.merge(self.prev_span()) }
}
```

Ensure `OffsetsDecl`/`OffsetsMember` are imported into `parser.rs` (follow how `StructDecl` is imported at the top of the file).

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sigil-frontend-emp parses_offsets_block`
Expected: PASS.

Also confirm no other `match` over `Item` became non-exhaustive: `cargo build -p sigil-frontend-emp` will flag every site (eval/lower). Add temporary `Item::Offsets(_) => {}` arms only where the compiler demands, to be filled by Tasks 5-6. If a match uses a catch-all `_`, nothing to do.

- [ ] **Step 6: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-frontend-emp/src/ast.rs crates/sigil-frontend-emp/src/parser.rs
git commit -m "feat(emp-parse): offsets{} block — AST OffsetsDecl + block parser"
```

---

## Task 5: Reverse direction — `Name.Variant` / `Name.count` comptime ordinals

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/mod.rs` (Evaluator struct ~84-92; `index_items` ~255-283)
- Modify: `crates/sigil-frontend-emp/src/eval/expr.rs` (`eval_path`, 2-segment branch ~171-215)

- [ ] **Step 1: Write the failing test**

Find how eval is tested end-to-end (grep `fn eval_str\|fn compile_str\|emp_candidate` in this crate's tests, or reuse `parse_str` + the evaluator entry point). A robust route is an end-to-end assertion through the CLI harness added in Task 7, but for a focused unit test, use whatever helper evaluates a comptime expression. Test intent:

```rust
#[test]
fn offsets_member_is_its_ordinal() {
    // Given `offsets M { A: t0, B: t1, C: t2 }`, then M.A==0, M.B==1, M.C==2, M.count==3.
    // Use a `const` that reads the ordinal so the value is observable:
    //   const X: u8 = M.B      // expect 1
    //   const N: u8 = M.count  // expect 3
    let src = "\
data t0 = [1]\n\
data t1 = [2]\n\
data t2 = [3]\n\
offsets M { A: t0, B: t1, C: t2 }\n\
const X: u8 = M.B\n\
const N: u8 = M.count\n";
    let vals = eval_consts(src); // helper: returns resolved const values by name
    assert_eq!(vals["X"], 1);
    assert_eq!(vals["N"], 3);
}

#[test]
fn offsets_unknown_member_errors() {
    let src = "\
data t0 = [1]\n\
offsets M { A: t0 }\n\
const X: u8 = M.Nope\n";
    let diags = eval_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("no member")), "got: {diags:?}");
}
```

Adapt `eval_consts`/`eval_diags` to the crate's actual test utilities (grep for an existing test that checks a `const`'s resolved value; copy its harness). The behavioral contract is the assertions.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-frontend-emp offsets_member`
Expected: FAIL — `M.B` unresolved / poison.

- [ ] **Step 3: Index `offsets` decls (MIRROR the enum indexing)**

In `crates/sigil-frontend-emp/src/eval/mod.rs`, add a field to `Evaluator` (next to `enums` ~line 84):

```rust
    pub(crate) offsets: HashMap<&'a str, &'a ast::OffsetsDecl>,
```

Initialize it wherever `enums`/`bitfields` are initialized (`HashMap::new()`), and in `index_items` add (next to the `Enum` arm):

```rust
            ast::Item::Offsets(o) => {
                self.offsets.insert(o.name.as_str(), o);
            }
```

- [ ] **Step 4: Resolve `Name.Variant` / `Name.count` (MIRROR the enum-variant branch)**

In `crates/sigil-frontend-emp/src/eval/expr.rs`, in `eval_path`'s 2-segment branch, AFTER the `self.enums.get(a)` block (~line 215) and before the fall-through, add:

```rust
        // An `Offsets.Member` comptime ordinal, or `Offsets.count`.
        if let Some(decl) = self.offsets.get(a) {
            if b == "count" {
                return Value::Int(decl.members.len() as i128);
            }
            if let Some(index) = decl.members.iter().position(|m| m.name == b) {
                return Value::Int(index as i128);
            }
            self.error(path.span, format!("offsets `{a}` has no member `{b}`"));
            return Value::Poison;
        }
```

(Match the exact `Value::Int` payload type — the report shows `i128`. Confirm with `grep -n "Value::Int" crates/sigil-frontend-emp/src/eval`.)

- [ ] **Step 5: Duplicate-variant check (totality)**

Duplicate member names would make ordinals ambiguous. In `eval_offsets` (added in Task 6) OR at index time, detect duplicates and emit an error once per offsets decl. Add a test:

```rust
#[test]
fn offsets_duplicate_member_errors() {
    let src = "\
data t0 = [1]\n\
offsets M { A: t0, A: t0 }\n\
const X: u8 = M.count\n";
    let diags = eval_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("duplicate")), "got: {diags:?}");
}
```

Implement the check where the decl is first evaluated (a `HashSet<&str>` over member names; on a repeat, `self.error(member.span, format!("duplicate offset entry `{}`", name))`).

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p sigil-frontend-emp offsets_`
Expected: PASS.

- [ ] **Step 7: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-frontend-emp/src/eval/mod.rs crates/sigil-frontend-emp/src/eval/expr.rs
git commit -m "feat(emp-eval): offsets reverse dir — Name.Variant ordinal + Name.count + dup check"
```

---

## Task 6: Forward direction — lower `offsets` to bytes + label

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/mod.rs` (add `eval_offsets`)
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (add the `Item::Offsets` lowering arm, mirroring `lower_data_item` ~218-235)

Context: `lower_data_item` (lower/mod.rs:218) is the exemplar: `eval → stream_data → define_label(name) → emit_data(bytes, fixups)`. For `offsets`, `eval_offsets` builds a `DataBuf` of `RelOffset` cells; each member target is resolved to a symbol name the SAME way `lower_ptr` (eval/emit.rs:339) extracts a name from a `Value` (`FnRef`/`Str`), so qualification matches data-pointer references exactly.

- [ ] **Step 1: Write the failing end-to-end test**

Add to `crates/sigil-cli/tests/ports.rs` (uses `emp_candidate`, defined at line 41):

```rust
#[test]
fn offsets_forward_emits_word_offsets() {
    // Table base at 0; each frame follows. Offsets = frame_addr - table_base.
    // Layout: 3 offset words (6 bytes) then frame0,frame1,frame2 (1 byte each).
    //   frame0 @ 6  -> 0x0006 ; frame1 @ 7 -> 0x0007 ; frame2 @ 8 -> 0x0008
    let emp = "\
section s (cpu: m68k, vma: $000000)\n\
offsets Map in s { F0: frame0, F1: frame1, F2: frame2 }\n\
data frame0 in s = [0x11]\n\
data frame1 in s = [0x22]\n\
data frame2 in s = [0x33]\n";
    let bytes = emp_candidate(emp);
    assert_eq!(
        bytes,
        vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x11, 0x22, 0x33]
    );
}
```

IMPORTANT: the exact placement syntax (`in s`, the `section` line) must match how existing `.emp` tests/examples declare a section and put items in it — check `examples/main.emp` (section decls) and existing `ports.rs` emp tests, and adjust the source string to the real grammar. The byte expectation is the contract; fix the source syntax until it parses, not the expectation (recompute offsets if you change the layout).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-cli offsets_forward_emits_word_offsets`
Expected: FAIL — offsets not lowered (empty/short output or a non-exhaustive-match placeholder from Task 4).

- [ ] **Step 3: Add `eval_offsets` (MIRROR `lower_ptr` name extraction)**

In `crates/sigil-frontend-emp/src/eval/mod.rs`, add:

```rust
/// Evaluate an `offsets` block to a `DataBuf` of `RelOffset` cells (forward
/// direction). Each member's target expression is resolved to a symbol name the
/// same way a pointer field is (`lower_ptr`), so qualification matches. Duplicate
/// member names are diagnosed here (totality).
pub(crate) fn eval_offsets(&mut self, decl: &ast::OffsetsDecl) -> DataBuf {
    let mut seen = std::collections::HashSet::new();
    let mut buf = DataBuf::empty();
    for m in &decl.members {
        if !seen.insert(m.name.as_str()) {
            self.error(m.span, format!("duplicate offset entry `{}`", m.name));
        }
        let val = self.eval_expr(&m.target); // however eval evaluates an Expr in this crate
        let name = match &val {
            Value::FnRef(n) => n.clone(),
            Value::Str(s) => s.clone(),
            _ => {
                self.error(m.span, format!(
                    "offset entry `{}` must reference a label, got {}", m.name, val.type_name()));
                "<unresolved>".to_string()
            }
        };
        buf.push(Cell::RelOffset { base: decl.name.clone(), target: name });
    }
    buf
}
```

(Use the crate's real expr-eval method name — grep `fn eval_expr\|fn eval(` in `eval/`. Reuse the `Value`/`type_name` machinery `lower_ptr` uses. If the duplicate check already lives in Task 5's index step, don't double-report — keep it in exactly one place.)

- [ ] **Step 4: Add the lowering arm (MIRROR `lower_data_item`)**

In `crates/sigil-frontend-emp/src/lower/mod.rs`, add an `Item::Offsets` arm to the item-lowering match (the one that has `ast::Item::Data(decl) => { ... lower_data_item(...) }` at line ~105), and a `lower_offsets_item` fn modeled on `lower_data_item` (line 218):

```rust
fn lower_offsets_item(
    file: &ast::File,
    decl: &ast::OffsetsDecl,
    placement: &Placement,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let buf = eval_offsets_with_root(file, decl, placement.include_root, diags);
    let (bytes, fixups, mut sd) = data::stream_data(&buf, placement.cpu, decl.span);
    diags.append(&mut sd);
    builder.define_label(&decl.name);
    builder.emit_data(&bytes, fixups, decl.span);
}
```

Provide an `eval_offsets_with_root` entry point paralleling `eval_data_with_root` (the one `lower_data_item` calls) that constructs the evaluator, runs `eval_offsets`, and returns `(DataBuf, diags)`. Mirror `eval_data_with_root`'s construction exactly — same `file`, `include_root`, diag threading. (`RelOffset` does not need `here_base`; the base is the symbolic label `decl.name`, resolved at link.)

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sigil-cli offsets_forward_emits_word_offsets`
Expected: PASS (bytes `00 06 00 07 00 08 11 22 33`).

- [ ] **Step 6: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-frontend-emp/src/eval/mod.rs crates/sigil-frontend-emp/src/lower/mod.rs
git commit -m "feat(emp-lower): lower offsets{} — forward dc.w Target-Base emission + base label"
```

---

## Task 7: AS byte-diff reference + negative-offset cross-check

**Files:**
- Modify: `crates/sigil-cli/tests/ports.rs`

Context: the whole-branch review byte-diffs emp output against the AS front end where a byte argument exists. First determine whether the AS front end folds `dc.w Target-Base` for FORWARD references.

- [ ] **Step 1: Investigate AS forward-diff folding**

Run a probe (a throwaway test or a scratch `assemble` call): does `sigil-frontend-as` assemble
```
Tbl:  dc.w F0-Tbl, F1-Tbl
F0:   dc.b $11
F1:   dc.b $22
```
into `00 02 00 03 11 22` without error? Inspect via the `as_reference` helper (ports.rs:28). Record the outcome:
- **Folds correctly** → use `as_reference` directly as the golden in Step 2.
- **Errors ("unresolved word expression")** → AS's `directive_dc_w` (eval.rs:2036) only fixups a bare `Sym`. Use a hand-computed literal `dc.w` golden instead, and note in the test a `// TODO(follow-up): teach AS front-end dc.w Target-Base` — teaching AS is a separate item (it is the reference front end; parity matters long-term but is not required to ship this).

- [ ] **Step 2: Write the byte-diff test**

If AS folds:
```rust
#[test]
fn offsets_byte_identical_to_as_reference() {
    let asm = "\
Tbl:  dc.w F0-Tbl, F1-Tbl, Bwd-Tbl\n\
F0:   dc.b $11\n\
F1:   dc.b $22\n\
Bwd:  dc.b $33\n";
    // (adjust to the AS front-end's exact source dialect used by other ports.rs tests)
    let reference = as_reference(asm);
    let emp = "\
section s (cpu: m68k, vma: $000000)\n\
offsets Tbl in s { F0: f0, F1: f1, Bwd: bwd }\n\
data f0 in s = [0x11]\n\
data f1 in s = [0x22]\n\
data bwd in s = [0x33]\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "offsets vs AS dc.w Target-Base");
}
```

If AS does NOT fold, replace `let reference = as_reference(asm);` with the hand-computed golden:
```rust
    let reference = vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x11, 0x22, 0x33];
```
(recompute for the actual layout).

Additionally add a NEGATIVE-offset case where a target precedes the base (put the base table AFTER some data), asserting the two's-complement word (e.g. a target 4 bytes before base → `0xFF 0xFC`), against AS or golden.

- [ ] **Step 3: Run + verify**

Run: `cargo test -p sigil-cli offsets_byte_identical`
Expected: PASS.

- [ ] **Step 4: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-cli/tests/ports.rs
git commit -m "test(emp): offsets byte-diff vs AS/golden incl. negative offset"
```

---

## Task 8: Totality (overflow error) + reverse-direction integration + example

**Files:**
- Modify: `crates/sigil-cli/tests/ports.rs`
- Create: `examples/offset_table.emp`

- [ ] **Step 1: Overflow-is-an-error test**

```rust
#[test]
fn offsets_overflow_is_a_compile_error() {
    // A target > $7FFF from the base overflows the signed word -> compile error.
    // Force distance with a large reserved data run between base and target.
    let emp = "\
section s (cpu: m68k, vma: $000000)\n\
offsets Tbl in s { Far: far }\n\
data pad in s = [0; 0x8000]\n\
data far in s = [0x99]\n";
    // emp_candidate panics on error diags; assert the compile FAILS with our message.
    let diags = emp_lower_diags(emp); // helper returning diags instead of panicking
    assert!(diags.iter().any(|d| d.message.contains("signed-word range")), "got: {diags:?}");
}
```

Add an `emp_lower_diags` helper next to `emp_candidate` that runs parse→lower→resolve_layout→link and RETURNS the diagnostics (mirror `emp_candidate` at ports.rs:41 but collect diags rather than `unwrap`/panic). Confirm the `[0; N]` array-repeat literal is real `.emp` syntax; if not, use whatever produces an N-byte run (grep examples). The point is a >$7FFF gap.

- [ ] **Step 2: Reverse-direction integration test (ordinal used in data)**

```rust
#[test]
fn offsets_ordinal_usable_as_byte() {
    // Map.Seed == 2 emitted as a dc.b.
    let emp = "\
section s (cpu: m68k, vma: $000000)\n\
offsets Map in s { Idle: a, Shoot: b, Seed: c }\n\
data a in s = [0x11]\n\
data b in s = [0x22]\n\
data c in s = [0x33]\n\
data Id: [u8; 1] in s = [Map.Seed]\n";
    let bytes = emp_candidate(emp);
    // 3 offset words (6B) + a,b,c (3B) + Id (1B == 0x02)
    assert_eq!(bytes.last(), Some(&0x02));
}
```

(Adjust `data Id: [u8;1] = [Map.Seed]` to the exact array-data syntax.)

- [ ] **Step 3: Create the example**

Create `examples/offset_table.emp` — a documented, real module using `offsets` for a mapping-style forward table AND a reverse ordinal id, in the house style of `examples/pitcher_plant.emp` (header comment explaining what each direction replaces vs the legacy `dc.w Target-Base` + hand-synced id constant). Then add a test asserting it compiles:

```rust
#[test]
fn example_offset_table_compiles() {
    let src = include_str!("../../../examples/offset_table.emp");
    let bytes = emp_candidate(src); // or the CLI end-to-end path used by other example tests
    assert!(!bytes.is_empty());
}
```

(Match how other examples are compiled in tests — grep `include_str!("../` in `crates/sigil-cli/tests`.)

- [ ] **Step 4: Run + verify**

Run: `cargo test -p sigil-cli offsets_ example_offset_table`
Expected: PASS.

- [ ] **Step 5: Green gate + commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-cli/tests/ports.rs examples/offset_table.emp
git commit -m "test(emp): offsets totality (overflow error) + ordinal-in-data + example"
```

---

## Task 9: Freeze the construct in the language spec

**Files:**
- Modify: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`

- [ ] **Step 1: Add the spec entry**

Add a subsection under §4 (Types & data layout) — e.g. §4.7 "Offset tables (`offsets`)" — documenting: the surface (`offsets Name { Variant: target, ... }`); forward emission (`dc.w target − Name`, signed-word range, overflow is a compile error — totality); reverse ordinals (`Name.Variant` = 0-based index, `Name.count`; named integer constants, NOT a coercing enum type, with the rationale from the design doc's Decision note); the deferred knobs (`base:` override, `dc.l`, Z80, inline blocks); and the R1 note that this is the DATA table only, separate from (encoding-agnostic) state dispatch. Keep the prose in the doc's established voice and cross-reference §4.5.

- [ ] **Step 2: Commit**

```bash
git add empyrean/docs/SIGIL_SPEC2_LANGUAGE.md
git commit -m "docs(spec): freeze the offsets construct (Plan 7 #3)"
```

(Note: `empyrean/` may be a separate git repo/worktree. If so, commit there per its own convention; otherwise this is a cross-tree edit — confirm the path is under version control before committing.)

---

## Task 10: Whole-branch adversarial review (process finale, not a TDD task)

Per the standing process, after Tasks 1-9 are green:
- [ ] Run the two-stage review (spec-compliance THEN `superpowers:code-reviewer` for code-quality) on the load-bearing tasks (2, 3, 5, 6).
- [ ] Construct + run a cross-feature `.emp` program combining `offsets` with existing features (a real mapping table feeding a struct, an ordinal used as a `subtype`), and byte-diff it against the AS reference wherever a byte argument exists.
- [ ] Verify: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` clean; `sigil emp examples/offset_table.emp --hex` produces the expected image.
- [ ] Update memories (`spec2-progress`, `emp-data-table-dsl-candidates`) and write a handoff note for the next backlog item (#4).
- [ ] **Milestone checkpoint with Volence before merge to master** (`--no-ff` merge + push).

---

## Self-Review

**Spec coverage** (design doc → task):
- New `RelWord16Be` fixup → Task 1. ✓
- Link resolution + signed-word range + overflow error → Task 2 (unit) + Task 8 (e2e). ✓
- `.emp` surface parse → Task 4. ✓
- Forward emission (`dc.w target−base`, base label) → Task 3 (cell/stream) + Task 6 (lower). ✓
- Reverse ordinals (`Name.Variant`, `Name.count`, dup/unknown errors) → Task 5. ✓
- AS byte-diff + negative offset → Task 7. ✓
- Totality (overflow), ordinal-in-data, example → Task 8. ✓
- 68k-only / Z80 diagnosed → Task 3. ✓
- Spec freeze → Task 9. ✓
- Deferred knobs (base override, dc.l, Z80, inline) — explicitly out of scope; Task 3 diagnoses Z80. ✓

**Placeholder scan:** representative code is labelled EXACT vs MIRROR; every MIRROR step names the exemplar (file:line) and gives the behavioral test as the contract. No "TBD"/"add error handling"-style gaps.

**Type consistency:** `Cell::RelOffset { base, target }` (Task 3) is produced in `eval_offsets` (Task 6) and consumed in `stream_data` (Task 3) with matching field names. `FixupKind::RelWord16Be` (Task 1) used identically in Tasks 2, 3. `OffsetsDecl`/`OffsetsMember` field names (`name`, `members`, `target`, `span`, `public`) consistent across Tasks 4, 5, 6. `Value::Int(i128)` per the eval report — verify at Step 4 of Task 5.

**Known verify-points flagged inline** (local idioms the executor must confirm, each guarded by a test): exact `parse_str`/eval test harness shape; `Value::Int` payload type; the section/placement source syntax (`in s`) in `.emp` tests; `[0; N]` / array-data literal syntax; `eval_data_with_root` construction to mirror for `eval_offsets_with_root`; whether AS folds forward `Target-Base`.
