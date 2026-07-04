# Sigil M1.C — T5b: 68k Explicit-Width Absolute Addressing Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.

**Goal:** Assemble 68000 **explicit-width absolute** data operands `(Sym).w` / `(Sym).l`
byte-exact vs `asl`. This is pervasive in Aeon (every RAM/register/ROM-data access).

**Architecture:** Fold-based (no width selection — the suffix is explicit; no linker fragment).
Parse `(expr).w`/`(expr).l` in `operands.rs::classify` → a new absolute atom; in
`convert_atoms_m68k` fold the address `Expr` and emit `Operand::AbsW(low16 as i16)` /
`Operand::AbsL(i32)`, routed through the existing `lower_inst`. Resolved symbols fold (the
multi-pass converges them); a genuinely-unresolved symbol is an undefined-symbol error (the
existing mechanism) — **no fixup path is needed for M1.C** because external symbols are stubbed
and intra-file forward refs resolve by convergence. (If T10 surfaces a real unresolved absolute,
add an `Abs16Be`/`Abs32Be` fixup fallback then — M1.B already resolves those kinds.)

**Tech Stack:** Rust; asl-diff via `tests/snippets_golden.txt` + real `asl` (the
`gen-snippet-vectors` tool still aborts on the pre-existing `struct_field_indexed` staleness —
verify bytes directly with `aeon/tools/asl -cpu 68000 -q -L -U` → `p2bin`, as T4/T5 did).

---

## Verified asl encodings (this task's oracle)

With `RAMV: equ $FFFF8000` (asl 1.42, `-cpu 68000`):
- `move.w (RAMV).w,d0` → `30 38 80 00` (abs.w mode `111 000`; value truncated to low-16 = `8000`)
- `move.l (RAMV).l,d1` → `22 39 FF FF 80 00` (abs.l mode `111 001`; full 32-bit `FFFF8000`)
- `lea    (RAMV).l,a0`  → `41 F9 FF FF 80 00`

So `.w` → `AbsW(i16)` (fold value, take low 16 bits as `i16`), `.l` → `AbsL(i32)` (full value).

---

## Files
- `crates/sigil-frontend-as/src/operands.rs` — recognize `( expr ) .w|.l` in `classify` → new
  `OperandAtom::M68kAbs { addr: Expr, long: bool }`. **Lexing note:** determine how `).w`/`).l`
  tokenizes (T5 found `.` is an ident-tail char, so `.w` after `)` may be its own token or need
  a small lexer/parse tweak — investigate and handle; a `(reg).w` would be ambiguous but no 68k
  register is a valid absolute base, and `sp`/`a0..a7` already classify as `M68kInd`, so an
  identifier that is NOT a data/address register followed by `.w`/`.l` inside/after parens is an
  absolute).
- `crates/sigil-frontend-as/src/eval.rs` — in `convert_atoms_m68k`, map `M68kAbs{addr,long}`:
  fold `addr` via `self.fold` (or `fold_imm` with the full i32 range); `long==false` →
  `AbsW((v & 0xFFFF) as i16)`, `long==true` → `AbsL(v as i32)`. Apply `qualify_expr` to `addr`
  first (dotted-local). Remove the T5b-deferral diagnostic for absolute atoms.
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — new `m68k_abs_*` blocks.

---

## Steps (TDD)

- [ ] **Step 1 — snippets first** (under `cpu 68000`, with a `Sym: equ $VALUE` so the address
  resolves): cover `.w` and `.l` as source and dest, at abs.w-range and abs.l-range values, e.g.
  with `RAMV equ $FFFF8000` / `VDP equ $C00004` / `ROMTAB equ $12345`:
  `move.w (RAMV).w,d0` / `move.w d0,(RAMV).w` / `move.l (VDP).l,d1` / `move.b (RAMV).w,d2` /
  `lea (ROMTAB).l,a0` / `clr.w (RAMV).w` / `tst.b (RAMV).w` / `cmp.w (RAMV).w,d3`.
- [ ] **Step 2 — golden bytes from real asl** (direct `asl`+`p2bin`, not the gen tool). Commit.
- [ ] **Step 3 — gate fails.** `cargo test -p sigil-frontend-as --test asl_snippets` → FAIL.
- [ ] **Step 4 — parse** `( expr ) .w|.l` → `M68kAbs`. Unit-test the parse (both suffixes; an
  abs.w value that is negative/large; confirm `(a0).w` is NOT misparsed as absolute — address
  registers still win).
- [ ] **Step 5 — convert** in `convert_atoms_m68k` → `AbsW`/`AbsL` (with `qualify_expr` on the
  address). Unit-test `move.w (RAMV).w,d0` produces the exact asl bytes.
- [ ] **Step 6 — gate green + suite.** asl_snippets PASS; `cargo test --workspace` all PASS.
- [ ] **Step 7 — clippy + build clean.**
- [ ] **Step 8 — commit** `feat(sigil-frontend-as): 68k explicit-width absolute addressing ((Sym).w/.l, asl-gated)`.

---

## Self-Review
- Spec coverage: `(Sym).w`/`(Sym).l` as source and dest, abs.w and abs.l ranges, snippet-gated.
- §7.4: only `sigil-isa` `AbsW`/`AbsL` constructed; absolute atom lives in the front-end.
- Honest gate: bytes from real asl. Fold-only (no fixup) is correct for M1.C's stubbed/resolved
  symbol set; a fixup fallback is explicitly out of scope unless a real unresolved absolute
  appears (→ revisit at T10).
- Escalate if: the `).w`/`).l` lexing can't be cleanly handled, or a snippet needs an unresolved
  absolute (fixup) that fold can't cover.
