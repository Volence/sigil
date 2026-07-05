# M1.D T2 — bare-symbol absolute EA + `END` directive — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clear the final 7 recon diagnostics — 6 bare-symbol absolute-EA sites (`lea Sym, a0`) + the `END` directive — so `m1c_full` reaches **0 diagnostics**, arming the full-ROM emit path (T4).

**Architecture:** A bare-symbol absolute EA is a width-variable instruction; asl width-selects abs.w/abs.l via the same pinned `asl_width_rule` as jmp/jsr (probe-verified — see `docs/superpowers/notes/2026-07-04-m1d-t2-abs-ea-end-probes.md`). We fold the address in the front-end and select the width there (reusing the one rule), rather than deferring to a resolve_layout fragment — this is the T3 front-end width-selection mechanism applied to the absolute-EA class. `END` is an emission no-op.

**Tech Stack:** Rust workspace (`sigil-ir`, `sigil-link`, `sigil-frontend-as`); real-`asl` snippet goldens via `gen_snippet_vectors`; strict gates via `SIGIL_STRICT_GATE=1 AEON_DIR=…`.

---

## File Structure

- `crates/sigil-ir/src/lib.rs` (or a new `crates/sigil-ir/src/width.rs`) — **new home** for `AbsWidth` + `asl_width_rule` (moved from sigil-link; single source of truth for front-end + linker).
- `crates/sigil-link/src/relax.rs` — **modified**: delete the local `AbsWidth`/`asl_width_rule` defs, use the `sigil-ir` ones; `sigil-link/src/lib.rs` re-export unchanged in spelling.
- `crates/sigil-frontend-as/src/eval.rs` — **modified**: `dispatch` gains `"end" | "END"` no-op; `convert_one_atom_m68k` gains a shared `abs_ea_from_expr` helper replacing the two "out of scope" errors.
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — **modified**: 6 new `t2_*` blocks.

---

## Task 1: Relocate `asl_width_rule` + `AbsWidth` into `sigil-ir`

The front-end cannot depend on `sigil-link` (one-way crate graph enforced by `crate_graph.rs`). Move the width rule to `sigil-ir`, which both crates already depend on. `sigil-link` re-exports so its code and the M1.B boundary-sweep tests are untouched.

**Files:**
- Create: `crates/sigil-ir/src/width.rs`
- Modify: `crates/sigil-ir/src/lib.rs` (add `mod width;` + re-export)
- Modify: `crates/sigil-link/src/relax.rs:4-38` (delete local defs; import from `sigil-ir`)

- [ ] **Step 1: Create the width module in sigil-ir**

Create `crates/sigil-ir/src/width.rs` with the definitions verbatim from `relax.rs` (keep the boundary-sweep-proven comment):

```rust
//! asl's `abs.w` vs `abs.l` selection for a 68000 absolute address. Shared by
//! the front-end (bare-symbol absolute EA + jmp/jsr width, M1.D T2/T3) and the
//! linker's `resolve_layout` (M1.B). Single source of truth — the front-end
//! cannot depend on `sigil-link`, and a second copy would be drift-prone.

/// The chosen absolute-addressing width for a width-variable 68000 form
/// (`jmp`/`jsr` target, or a bare-symbol absolute EA).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsWidth {
    /// `abs.w`: opcode word + 2-byte operand.
    W,
    /// `abs.l`: opcode word + 4-byte operand.
    L,
}

impl AbsWidth {
    /// Total length in bytes of a 2-byte-opcode `jmp`/`jsr` at this width
    /// (4 for `.w`, 6 for `.l`). Used by the linker's fragment-length math.
    pub fn inst_len(self) -> u32 {
        match self {
            AbsWidth::W => 4,
            AbsWidth::L => 6,
        }
    }
}

/// asl's `abs.w` vs `abs.l` selection for a 68000 absolute address. Confirmed
/// byte-for-byte against asl 1.42 by a boundary sweep of `jmp $ADDR` (with AND
/// without `-A` — identical results, so `-A` is irrelevant to width) and
/// re-confirmed for the general absolute EA in M1.D T2 (`lea`/`move` probes).
/// `abs.w` iff the 24-bit address sign-extends losslessly from 16 bits:
/// `[0, 0x7FFF] ∪ [0xFF_8000, 0xFF_FFFF]`. Examples: $7FFF→.w, $8000→.l,
/// $FF8000→.w (= -$8000 sign-extended), $FFFFFE→.w.
pub fn asl_width_rule(target: i64, _dash_a: bool) -> AbsWidth {
    let a = (target & 0xFF_FFFF) as u32;
    if a <= 0x7FFF || a >= 0xFF_8000 {
        AbsWidth::W
    } else {
        AbsWidth::L
    }
}
```

- [ ] **Step 2: Wire the module into sigil-ir's lib.rs**

In `crates/sigil-ir/src/lib.rs`, add the module declaration next to the other `mod` lines and re-export at the crate root (match the existing re-export style — check how `pub use` is written for other items and mirror it):

```rust
mod width;
pub use width::{asl_width_rule, AbsWidth};
```

- [ ] **Step 3: Delete the local defs in relax.rs and import from sigil-ir**

In `crates/sigil-link/src/relax.rs`, delete lines 4–38 (the `AbsWidth` enum, its `impl`, and the `asl_width_rule` fn — everything from `/// The chosen absolute-addressing width` through the closing `}` of `asl_width_rule`). Keep the top module comment (lines 1–2). Then add `AbsWidth` and `asl_width_rule` to the existing `use sigil_ir::{…}` import line (currently `use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SymbolTable, SymbolValue};`):

```rust
use sigil_ir::{
    asl_width_rule, AbsWidth, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section,
    SymbolTable, SymbolValue,
};
```

Leave `crates/sigil-link/src/lib.rs:14` (`pub use relax::{asl_width_rule, resolve_layout, AbsWidth};`) unchanged — `relax` now re-exports the `sigil-ir` items transparently, so downstream spellings (`sigil_link::asl_width_rule`, `sigil_link::AbsWidth`) still resolve.

- [ ] **Step 4: Build and run the linker's boundary-sweep + workspace tests**

Run: `cargo test -p sigil-ir -p sigil-link`
Expected: PASS — in particular `relax.rs`'s `asl_width_rule` boundary tests (`$7FFF`→W, `$8000`→L, `$FF8000`→W, `-A` irrelevance) and `inst_len` still pass, now exercising the moved definition.

Run: `cargo test --workspace`
Expected: PASS (no behavior change; pure relocation).

Run: `cargo test -p sigil-cli --test crate_graph crate_graph_is_one_way`
Expected: PASS — the one-way graph is intact (no new frontend→link edge was added; sigil-ir is upstream of both).

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-ir/src/width.rs crates/sigil-ir/src/lib.rs crates/sigil-link/src/relax.rs
git commit -m "refactor(sigil-ir): move asl_width_rule + AbsWidth to sigil-ir (shared by front-end + linker; T2 prep)"
```

---

## Task 2: `END` directive no-op + golden

Probe: `END` and `END <sym>` both emit zero bytes (`docs/…/2026-07-04-m1d-t2-abs-ea-end-probes.md`). Aeon's only use is the bare `END` at `main.asm:446`.

**Files:**
- Modify: `crates/sigil-frontend-as/tests/snippets_golden.txt` (append `t2_end_noop`)
- Modify: `crates/sigil-frontend-as/src/eval.rs:1618` (add dispatch arm after `"BINCLUDE"`)

- [ ] **Step 1: Add the failing snippet golden**

Append to `crates/sigil-frontend-as/tests/snippets_golden.txt`:

```
=== t2_end_noop ===
	cpu 68000
	padding off
	org 0
	nop
	END
--- bytes ---
4E 71
```

- [ ] **Step 2: Run the snippet test to verify it fails**

Run: `cargo test -p sigil-frontend-as --test asl_snippets`
Expected: FAIL — `snippet `t2_end_noop` diverged from golden` (the front-end currently routes `END` to m68k lowering → an error diagnostic → `assemble` returns `Err`, so `.expect("assemble")` panics). Either a panic or a mismatch confirms the gap.

- [ ] **Step 3: Add the no-op dispatch arm**

In `crates/sigil-frontend-as/src/eval.rs`, in the `dispatch` match (after the `"BINCLUDE" => self.directive_binclude(rest, span),` arm at line 1618), add:

```rust
            // `END` (asl's end-of-source / entry-point directive). Emits no
            // bytes — bare `END` and `END <entrypoint>` are both emission
            // no-ops (probe: 2026-07-04-m1d-t2-abs-ea-end-probes.md). Aeon's
            // only use is the bare `END` at main.asm:446. Exact-case like
            // `BINCLUDE`; does not collide with the `endif`/`endm`/`endr`/
            // `endcase` block closers (handled in block scanning, not dispatch).
            "end" | "END" => {}
```

- [ ] **Step 4: Run the snippet test to verify it passes**

Run: `cargo test -p sigil-frontend-as --test asl_snippets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-frontend-as/src/eval.rs crates/sigil-frontend-as/tests/snippets_golden.txt
git commit -m "feat(sigil-m1d): END directive emission no-op (T2)"
```

---

## Task 3: Bare-symbol / bare-expression absolute EA lowering + goldens

Replace the two "out of scope for T5" errors in `convert_one_atom_m68k` with a shared helper that folds the address and width-selects abs.w/abs.l via `asl_width_rule`. Use `self.fold()` directly (NOT `fold_imm`, which returns 0 on Poison → would pick optimistic abs.w); an unresolved-this-pass symbol picks pessimistic abs.l (asl's forward-symbol behavior; keeps the fixpoint shrink-only) and records the poison ref so a genuinely-undefined symbol still errors on the converged pass.

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs:2574-2599` (the `Value(Expr::Sym)` fall-through + `Value(_)` branches in `convert_one_atom_m68k`)
- Add: a private `abs_ea_from_expr` method near `convert_one_atom_m68k`
- Modify: `crates/sigil-frontend-as/tests/snippets_golden.txt` (append 5 `t2_*` blocks)

- [ ] **Step 1: Add the failing snippet goldens**

Append to `crates/sigil-frontend-as/tests/snippets_golden.txt` (bytes are the asl-verified probe results; a `gen_snippet_vectors` run will confirm them):

```
=== t2_lea_abs_w ===
	cpu 68000
	padding off
	org 0
Low:	equ $100
	lea Low, a0
	rts
--- bytes ---
41 F8 01 00 4E 75
=== t2_lea_abs_l ===
	cpu 68000
	padding off
	org 0
High:	equ $10000
	lea High, a0
	rts
--- bytes ---
41 F9 00 01 00 00 4E 75
=== t2_lea_abs_boundary ===
	cpu 68000
	padding off
	org 0
S:	equ $FF8000
	lea S, a0
	rts
--- bytes ---
41 F8 80 00 4E 75
=== t2_move_abs_w ===
	cpu 68000
	padding off
	org 0
L:	equ $100
	move.w L, d0
	rts
--- bytes ---
30 38 01 00 4E 75
=== t2_move_abs_l ===
	cpu 68000
	padding off
	org 0
H:	equ $10000
	move.w H, d0
	rts
--- bytes ---
30 39 00 01 00 00 4E 75
```

- [ ] **Step 2: Run the snippet test to verify it fails**

Run: `cargo test -p sigil-frontend-as --test asl_snippets`
Expected: FAIL — the 5 new blocks panic in `.expect("assemble")` (the bare-symbol EA currently errors "out of scope for T5").

- [ ] **Step 3: Add the `abs_ea_from_expr` helper**

In `crates/sigil-frontend-as/src/eval.rs`, add this method inside the `impl Asm` block, immediately before `fn convert_one_atom_m68k` (near line 2549). It qualifies the expression, folds it, width-selects, and returns the resolved operand:

```rust
    /// Lower a bare (unsuffixed) absolute-address EA operand — a symbol or an
    /// expression used where a 68k EA is expected, e.g. `lea Sym, a0` or
    /// `move.w Sym, d0`. asl width-selects abs.w/abs.l via `asl_width_rule`
    /// (probe-verified EA-general in M1.D T2). We fold + select in the front
    /// end (the T3 width-selection mechanism for the absolute-EA class), so the
    /// instruction's Data fragment carries the true encoded length and the
    /// multi-pass fixpoint converges. Uses `self.fold` (not `fold_imm`): an
    /// unresolved-this-pass symbol folds to Poison → pessimistic abs.l (matching
    /// asl's forward-symbol width guess, keeping convergence shrink-only) and is
    /// recorded in `poison_refs` so a genuinely-undefined symbol still errors on
    /// the converged pass.
    fn abs_ea_from_expr(&mut self, e: &Expr, span: Span) -> M68kOperand {
        let qualified = self.qualify_expr(e);
        match self.fold(&qualified) {
            Fold::Value(v) => match asl_width_rule(v, false) {
                AbsWidth::W => M68kOperand::AbsW((v & 0xFFFF) as i16),
                AbsWidth::L => M68kOperand::AbsL(v as i32),
            },
            Fold::Poison => {
                for name in self.unresolved_names(&qualified) {
                    self.poison_refs.push((name, span));
                }
                // Pessimistic abs.l while unresolved; the converged pass re-folds
                // to a real value (or errors via poison_refs above).
                M68kOperand::AbsL(0)
            }
        }
    }
```

- [ ] **Step 4: Route the two branches through the helper**

In `convert_one_atom_m68k`, replace the `else { self.err(… "out of scope for T5" …); return None; }` tail of the `OperandAtom::Value(Expr::Sym(name))` arm (lines ~2583-2591) so it returns the folded absolute EA instead of erroring. The arm becomes:

```rust
            OperandAtom::Value(Expr::Sym(name)) => {
                if let Some(n) = m68k_data_reg(name) {
                    M68kOperand::Dn(n)
                } else if let Some(n) = m68k_addr_reg(name) {
                    M68kOperand::An(n)
                } else if name == "sr" {
                    M68kOperand::Sr
                } else if name == "ccr" {
                    M68kOperand::Ccr
                } else {
                    // Bare symbol in EA position = absolute address; asl
                    // width-selects abs.w/abs.l (M1.D T2).
                    self.abs_ea_from_expr(&Expr::Sym(name.clone()), span)
                }
            }
```

And replace the `OperandAtom::Value(_)` arm (lines ~2593-2599) — bind the inner expression and route it through the same helper:

```rust
            OperandAtom::Value(e) => {
                // Bare numeric/expression operand = 68k absolute addressing;
                // width-selected like the bare-symbol case above (M1.D T2).
                self.abs_ea_from_expr(e, span)
            }
```

Add the imports the helper needs. At the top of `eval.rs`, add `asl_width_rule` and `AbsWidth` to the existing `use sigil_ir::{…}` line, and ensure `Fold` is in scope (it is already used by `fold_imm` via `self.fold`; confirm the `use sigil_ir::expr::Fold;` or equivalent import exists — if `fold_imm` matches on `Fold::Value`/`Fold::Poison`, the import is already present).

- [ ] **Step 5: Run the snippet test to verify it passes**

Run: `cargo test -p sigil-frontend-as --test asl_snippets`
Expected: PASS — all `t2_*` blocks match, and every pre-existing golden still matches (the change only affects the previously-erroring bare-EA path).

- [ ] **Step 6: Regenerate goldens from real asl (non-circularity check)**

Run: `AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -p sigil-frontend-as --bin gen_snippet_vectors`
Then: `git diff --stat crates/sigil-frontend-as/tests/snippets_golden.txt`
Expected: the working tree is **unchanged** (or churns only the byte lines of the 6 new `t2_*` blocks if a byte was mistyped — in which case the real-asl bytes win; re-run the test). Every pre-existing block must regenerate byte-identical (the non-circularity invariant). If any pre-existing block churns, STOP — something perturbed an unrelated path.

- [ ] **Step 7: Commit**

```bash
git add crates/sigil-frontend-as/src/eval.rs crates/sigil-frontend-as/tests/snippets_golden.txt
git commit -m "feat(sigil-m1d): bare-symbol absolute EA width-select (abs.w/abs.l via asl_width_rule; T2)"
```

---

## Task 4: Recon-0 verification + full strict gates

**Files:** none (verification only).

- [ ] **Step 1: Run the full-ROM recon — expect 0 diagnostics**

Run: `AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -p sigil-harness --example m1c_full`
Expected: `ASSEMBLED OK: N sections` (NOT `FAILED: … diagnostics`). This is the T2 acceptance bar — the recon now digests the entire real tree with zero diagnostics, arming `m1c_rom` for T4.

If any diagnostics remain, run with `FILTER=` to inspect them and classify: if they are newly-*exposed* (previously masked by the EA/END failures) rather than regressions, bucket them (record in the spec + memory) — the same discipline T1 used for `END`. Do not paper over them.

- [ ] **Step 2: Run all strict gates**

Run: `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness`
Expected: PASS — `m1b_gate` (5 ok: checksum 0x18E + s4budget/oracle), `m1c_vector_table` (1), M0 harness (with `--ignored` where gated). The T2 front-end change touches only the bare-EA path, so the vector-table (dc.l data) and hand-built-IR (m1b) gates must be unaffected.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean (no warnings). Note `--all-targets` — CI runs it; plain clippy misses test-code lints.

- [ ] **Step 3: Update the spec + memory note**

Mark T2 ✅ DONE in `docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md` (record: recon 7→0, the AbsWidth relocation to sigil-ir, any newly-exposed items). Update the `sigil-m0-core-progress` memory note's M1.D paragraph to reflect recon-0 and the next step (T3).

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md
git commit -m "docs(sigil-m1d): T2 done — recon 7→0; abs-EA width-select + END; AbsWidth→sigil-ir"
```

---

## Self-Review notes

- **Spec coverage:** T2 spec = 6 EA sites (Task 3) + `END` (Task 2) + shared width rule (Task 1) + recon-0 acceptance (Task 4). Covered.
- **Type consistency:** `AbsWidth`/`asl_width_rule` (sigil-ir, re-exported by sigil-link) used uniformly; `abs_ea_from_expr` returns `M68kOperand` (`AbsW(i16)`/`AbsL(i32)`), matching the existing `M68kAbs` branch's construction exactly.
- **Non-circularity:** every byte-affecting change (Tasks 2, 3) lands with a real-asl golden and a `gen_snippet_vectors` no-op check (Task 3 Step 6).
- **Ordering:** Task 1 (relocation) precedes Task 3 (which imports the moved rule). Task 4 gates the whole.
- **Watch (carry to T3/T4):** the folded absolute-EA *values* for the 6 real sites may be stale by the object-bank +22 (F2) until T3 re-flows; T2's bar is recon-0 + goldens, and all 6 sites are high-addr → abs.l so the *width* is stable. If Task 4 Step 1 surfaces a NON-EA/END diagnostic, treat it as newly-exposed and bucket it.
```
