# Sigil M1.C — T4: 68k Straight-Line Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.

**Goal:** Make `sigil-frontend-as` assemble the common **straight-line** 68000 instruction set
(register-direct + immediate operands) byte-exact vs `asl`, replacing the T1 `lower_m68k`
stub. Memory/absolute addressing modes, PC-relative, and all control-flow (branches, jmp/jsr,
Bcc/Dbcc/Scc) are **T5**.

**Architecture:** Parse `mn` → `(m68k::Mnemonic, m68k::Size)`; parse operands into the existing
`OperandAtom`s plus a new `Immediate(Expr)` atom for `#expr`; convert atoms → `m68k::Operand`
(`Dn`/`An`/`Imm`); build `m68k::Instruction { mnemonic, size, ops }` and call
`M68kBackend::lower_inst`. All CPU-specific code lives in the front-end; IR/backend untouched.

**Tech Stack:** Rust; asl-diff via `tests/snippets_golden.txt` + `gen-snippet-vectors` (real
`asl -cpu 68000`). The M1.A backend (`sigil-backend-m68k`) already encodes `lower_inst`
byte-exact — T4 only feeds it correct `Instruction`s.

---

## API reference (verified via T4 grounding, 2026-07-03)

`sigil-isa/src/m68k.rs`:
- `Mnemonic` (line 31): `Move, Movea, Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls,
  Addi, Subi, Andi, Ori, Eori, Cmpi, Moveq, Addq, Subq, Asl, Asr, Lsl, Lsr, Rol, Ror, Btst,
  Bset, Bclr, Clr, Neg, Not, Tst, Tas, Swap, Ext, Lea, Pea, Nop, Rts, Rte, Trap, MoveToSr,
  MoveFromSr, AndiCcr, OriCcr, …` plus control-flow variants (`Jmp, Jsr, Bra, Bsr, Bcc(Cond),
  Dbcc(Cond), Scc(Cond)`) that are **T5, not T4**.
- `Size` (line 61): `B, W, L, S`.
- `Operand` (line 77): T4 uses `Dn(u8)`, `An(u8)`, `Imm(i32)`. (Memory/EA variants are T5.)
- `Instruction { mnemonic: Mnemonic, size: Size, ops: Vec<Operand> }` (line 104).

`sigil-backend-m68k/src/lib.rs`:
- `M68kBackend::lower_inst(&self, inst: &Instruction, span) -> Result<DataFragment, LowerError>`
  (line 44) — the T4 entry point.

Front-end (`sigil-frontend-as`):
- `lower_m68k` stub at `eval.rs` (added in T1) — replace it.
- `mnemonic()` (Z80) at `eval.rs:~1241`; Z80 reg helpers `reg8/reg16/cond_word` at `~1263`.
- `OperandAtom` + `parse_operands` in `operands.rs`; Z80 `convert_atoms` at `eval.rs:~1057`.
- `self.fold(&expr)`/`self.fold_imm(...)` fold operand expressions; `self.m68k` backend field.

---

## Scope

**In:** mnemonic+size parse; the unconditional straight-line mnemonic set; `d0–d7`/`a0–a7`/`sp`
register recognition; `#expr` immediate operand; `convert_atoms_m68k` for `Dn`/`An`/`Imm`;
`lower_m68k` via `lower_inst`; asl-diff gate on the common instructions.

**Out (→ T5):** memory/indirect/predec/postinc/displacement/indexed/PC-relative EA modes,
**absolute addressing + its abs.w/abs.l width selection**, `lea`/`pea` (need EA), branches,
`jmp`/`jsr`, `Bcc`/`Dbcc`/`Scc`, `movem`/`movep`, dotted-local qualification. If a snippet
needs any Out item, it belongs in T5.

---

## Files
- `crates/sigil-frontend-as/src/operands.rs` — add `OperandAtom::Immediate(Expr)`; recognize a
  leading `#` (verify the lexer emits a `Punct` for `#`; if not, add it) → `Immediate`.
- `crates/sigil-frontend-as/src/eval.rs` — `split_mnemonic_and_size`, `m68k_mnemonic`,
  `m68k_data_reg`/`m68k_addr_reg`, `convert_atoms_m68k`, real `lower_m68k`.
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — 68k snippets (cpu 68000).

---

## Steps (TDD)

- [ ] **Step 1 — asl-diff snippets first.** Add 68k blocks (under `cpu 68000`; match existing
  block format). Cover the straight-line set with reg/imm operands, e.g.:
  `move.w d0,d1` / `moveq #5,d0` / `move.l d2,d3` / `add.w d1,d2` / `addq.w #1,d0` /
  `sub.l d4,d5` / `cmp.w d6,d7` / `and.w d0,d1` / `or.l d2,d3` / `eor.w d4,d5` /
  `asl.w #2,d0` / `lsr.l #1,d1` / `swap d0` / `ext.w d0` / `ext.l d1` / `clr.b d2` /
  `neg.w d3` / `not.l d4` / `tst.w d5` / `nop` / `rts` / `rte`.
  Give each block a distinct name.

- [ ] **Step 2 — generate goldens from asl.**
  `cargo run -p sigil-frontend-as --bin gen-snippet-vectors`. If asl rejects a snippet (e.g. an
  operand form that is actually a T5 EA), remove it from T4 and note it for T5. Commit the
  regenerated `snippets_golden.txt`.

- [ ] **Step 3 — run the gate, watch it fail.**
  `cargo test -p sigil-frontend-as --test asl_snippets` → FAILS (lower_m68k is the stub).

- [ ] **Step 4 — mnemonic + size parse.** Implement `split_mnemonic_and_size(s: &str) ->
  (&str, Option<Size>)` (strip a trailing `.b`/`.w`/`.l`/`.s`; `.s` is short-branch only, so
  under the T4 set treat a `.s` as an error/none) and `m68k_mnemonic(base: &str) ->
  Option<Mnemonic>` covering the straight-line set above. Default size when the mnemonic
  carries none (`nop`/`rts`/`swap`/`ext`… — use the size the backend expects; `swap`/`ext`
  encode their own size — confirm against `lower_inst`/the m68k corpus). Unit-test the split
  (`"move.w"` → `("move", W)`, `"moveq"` → `("moveq", None)`, `"clr.b"` → `("clr", B)`).

- [ ] **Step 5 — register + immediate operands.** `m68k_data_reg("d3")` → `Some(3)`,
  `m68k_addr_reg("a5")`/`"sp"` → `Some(5)`/`Some(7)`. In `operands.rs`, a leading `#` marks
  `Immediate(Expr)`. Unit-test recognition.

- [ ] **Step 6 — convert + lower.** `convert_atoms_m68k(mnemonic, atoms, span) ->
  Option<Vec<Operand>>` maps `RegOrCond("d3")`→`Dn(3)`, `RegOrCond("a5")`/`"sp"`→`An(5)`/`An(7)`,
  `Immediate(e)`→`Imm(self.fold_imm(e,…) as i32)`. Reject (diagnostic) any memory/EA atom with
  "68k addressing mode not yet supported (T5)". Then implement `lower_m68k`:
  1. `let (base, size_opt) = split_mnemonic_and_size(mn);`
  2. `let mnemonic = m68k_mnemonic(base)` else error "not a 68k mnemonic".
  3. `let ops = self.convert_atoms_m68k(mnemonic, &atoms, span)?;`
  4. `let size = size_opt.unwrap_or(<default for this mnemonic>);`
  5. `let inst = Instruction { mnemonic, size, ops };`
  6. `let f = self.m68k.lower_inst(&inst, span); self.emit_frag(f, span);` (mirror how
     `lower_z80` calls `emit_frag`; adapt for the `Result<DataFragment, LowerError>` return —
     on `Err`, push a diagnostic).

- [ ] **Step 7 — gate green + full suite.**
  `cargo test -p sigil-frontend-as --test asl_snippets` PASS; `cargo test --workspace` all PASS
  (no Z80 regressions; crate-graph still one-way).

- [ ] **Step 8 — clippy + build.**
  `cargo clippy --workspace -- -D warnings && cargo build --workspace` clean.

- [ ] **Step 9 — commit.**
  `git commit -m "feat(sigil-frontend-as): 68k straight-line core (reg/imm operands via lower_inst, asl-gated)"`

---

## Self-Review
- Spec coverage: mnemonic+size, reg+imm operands, lower_inst path, asl-diff gate — all present.
  Control-flow + EA modes explicitly deferred to T5 (no silent gap; snippets that need them are
  moved, not faked).
- §7.4: only `sigil-isa`/`sigil-backend-m68k` public types are constructed; no AS concept
  enters IR. `Immediate` atom lives in the front-end.
- Honest gate: bytes come from real asl; a snippet asl rejects is removed, not stubbed green.
- Escalate if: a straight-line mnemonic's default size is ambiguous vs the backend, or `lower_inst`
  needs an operand form T4 declared out of scope.
