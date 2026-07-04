# Sigil M1.C — T5: 68k Fixed-Length EA Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.

**Goal:** Extend `sigil-frontend-as` to assemble the 68000 **register-indirect address-mode
family** (all fixed-length, no width selection, no fixups) byte-exact vs `asl`, building on T4's
straight-line core: `(An)`, `(An)+`, `-(An)`, `(d16,An)`, `(d8,An,Xn)`, plus `lea`/`pea`.

**Architecture:** Add syntactic 68k EA recognition to `operands.rs::classify` (these forms don't
collide with any Z80 syntax, so no CPU-aware signature change is needed) producing new
`OperandAtom` variants; map them to `m68k::Operand` in `convert_atoms_m68k`; everything still
flows through `M68kBackend::lower_inst` (fixed-length). **Out of scope (→ T5b):** absolute
addressing + abs.w/abs.l width, branches, jmp/jsr, Dbcc/Scc, PC-relative — anything
variable-length or fixup/linker-integrated.

**Tech Stack:** Rust; asl-diff via `tests/snippets_golden.txt` + real `asl` (see T4/T2; the
`gen-snippet-vectors` tool currently aborts on the pre-existing `struct_field_indexed` z80
staleness — verify new snippet bytes directly with `aeon/tools/asl -cpu 68000 -q -L -U` →
`p2bin`, exactly as T4 did, until T6 fixes that snippet).

---

## API reference (verified)

`sigil-isa/src/m68k.rs` `Operand` variants this task produces:
- `Ind(u8)` = `(An)`, `PostInc(u8)` = `(An)+`, `PreDec(u8)` = `-(An)`
- `Disp16An(i16, u8)` = `(d16,An)`
- `Disp8AnXn { d: i8, an: u8, xn: Xn, long: bool }` = `(d8,An,Xn)`; `Xn::D(u8)` / `Xn::A(u8)`,
  `long` = index register size (`.w` → false, `.l` → true).
- (`lea`/`pea` take an EA source; `lea <ea>,An`, `pea <ea>`.)

`operands.rs` today (T4 state): `OperandAtom { RegOrCond(String), IndReg(String),
Indexed{reg:IndexReg,disp:Expr}, Mem(Expr), Value(Expr), AfShadow, Imm(Expr) }`. `classify`
(line 65) already branches on `#` (Imm), single-ident regs, `(...)`, and bare expr. `split_commas`
(line 46) keeps commas inside parens grouped — so `(d,a0,d1)` arrives at `classify` intact.

`eval.rs` (T4 state): `convert_atoms_m68k(mnemonic, size, atoms, span)` maps atoms → `Vec<Operand>`;
`m68k_data_reg`/`m68k_addr_reg` recognize `d0–d7`/`a0–a7`/`sp`. `lower_m68k` builds the
`Instruction` and calls `lower_inst`.

---

## New OperandAtom variants (68k EA)

Add (names illustrative — follow the existing String/Expr-carrying style; these hold raw
register names + unfolded displacement `Expr`s, resolved in `convert_atoms_m68k`):
- `M68kInd(String)` — `(a0)`
- `M68kPostInc(String)` — `(a0)+`
- `M68kPreDec(String)` — `-(a0)`
- `M68kDisp { disp: Expr, an: String }` — `(d16,a0)`
- `M68kIdx { disp: Expr, an: String, xn: String, xlong: bool }` — `(d8,a0,d1.w|.l)`

`expand.rs::punct_str` and any exhaustive `OperandAtom` match may need arms — build will tell you.

---

## Steps (TDD)

- [ ] **Step 1 — asl-diff snippets first.** Add 68k blocks (under `cpu 68000`, matching T4's
  block conventions). Cover every EA mode as **both source and destination** where legal, e.g.:
  `move.w (a0),d0` / `move.w d0,(a0)` / `move.w (a0)+,d1` / `move.w d1,(a0)+` /
  `move.w -(a1),d2` / `move.w d2,-(a1)` / `move.w (4,a0),d3` / `move.w d3,(6,a0)` /
  `move.w (6,a0,d1.w),d4` / `move.w (8,a0,a2.l),d5` / `clr.w (a0)` / `tst.b (a1)+` /
  `add.w (a0),d0` / `lea (a0),a1` / `lea (4,a0),a1` / `lea (6,a0,d1.w),a2` / `pea (a0)` /
  `pea (4,a0)`. Distinct names.

- [ ] **Step 2 — golden bytes from real asl.** For each snippet run `aeon/tools/asl -cpu 68000
  -q -L -U -olist <lst> -o <p> <asm>` then `p2bin <p> <bin>` (as T4 did) and paste the bytes
  into the block. (Do NOT rely on `gen-snippet-vectors` until T6 — it aborts on
  `struct_field_indexed`.) If asl rejects a form, it's likely out of scope — drop it. Commit.

- [ ] **Step 3 — gate fails.** `cargo test -p sigil-frontend-as --test asl_snippets` → FAIL.

- [ ] **Step 4 — parse the EA forms** in `classify`:
  - `-(a0)`: `g` starts with `Punct::Minus` then `(reg)` → `M68kPreDec`.
  - `(a0)+`: `g` is `(...)` followed by a trailing `Punct::Plus` → `M68kPostInc`.
  - `(reg)` where reg ∈ `a0..a7`/`sp`: `M68kInd`. (Keep the existing Z80 `hl/bc/de/sp` branch;
    add an `a0..a7` recognizer — a name starting with `a` + digit is unambiguously 68k.)
  - `(d,a0)`: parenthesised inner splits into `[expr] , [a0]` → `M68kDisp`.
  - `(d,a0,xn.w|.l)`: inner splits into `[expr] , [a0] , [xn(.w|.l)]` → `M68kIdx` (parse the
    optional `.w`/`.l` on the index reg; default `.w` → `xlong=false`). Reuse/extend the
    parenthesised-inner comma splitter; fold nothing here (keep `Expr`s raw).
  Unit-test each shape parses to the right atom.

- [ ] **Step 5 — convert in `convert_atoms_m68k`:** map the new atoms →
  `Ind/PostInc/PreDec/Disp16An/Disp8AnXn`, folding displacement `Expr`s via `self.fold_imm`
  with the correct ranges (`Disp16An` d ∈ i16; `Disp8AnXn` d ∈ i8; index `Xn::D`/`Xn::A` from
  the reg name, `long` from the `.w`/`.l`). Register names → numbers via `m68k_addr_reg`/
  `m68k_data_reg`. Add `lea`/`pea` to `m68k_mnemonic` and REMOVE them from `m68k_out_of_scope`
  (T4 deferred them). Keep rejecting absolute/PC-relative/branch atoms with T5b-naming
  diagnostics.

- [ ] **Step 6 — gate green + suite.** `cargo test -p sigil-frontend-as --test asl_snippets`
  PASS; `cargo test --workspace` all PASS (Z80 regressions none; crate-graph green).

- [ ] **Step 7 — clippy + build.** `cargo clippy --workspace -- -D warnings && cargo build
  --workspace` clean.

- [ ] **Step 8 — commit.** `feat(sigil-frontend-as): 68k fixed-length EA modes ((An) family + lea/pea, asl-gated)`

---

## Self-Review
- Spec coverage: `(An)`/`(An)+`/`-(An)`/`(d16,An)`/`(d8,An,Xn)` + `lea`/`pea`, each snippet-gated
  as source and dest. Absolute/branches/jmp-jsr/PC-rel explicitly deferred to T5b (rejected with
  a T5b-naming diagnostic, not faked).
- Honest gate: bytes from real asl; a form asl rejects is dropped, not stubbed.
- §7.4: only `sigil-isa` operand types constructed; EA atoms live in the front-end.
- Escalate if: an EA form's disp-size selection (`Disp16An` vs an index form) is ambiguous vs the
  corpus, or `lea`/`pea` need an operand class declared out of scope.
