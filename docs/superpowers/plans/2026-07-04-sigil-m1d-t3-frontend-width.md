# M1.D T3 — Front-End jmp/jsr Width Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move 68k `jmp`/`jsr` abs.w/abs.l width selection out of the linker's
`resolve_layout` and into the front-end multi-pass loop, so the cursor advances by the
true instruction width — fixing the F2 stale-fold (folded pointers after a grown jmp) and
stale-downstream-LMA (Z80 driver placed 22 bytes early) defects by construction.

**Architecture:** The front-end folds each bare-symbol `jmp`/`jsr` target from the
current-pass env, picks the width via the pinned `asl_width_rule` (**probe-verified
grow-only: unknown-this-pass target → abs.w optimistic**, `2026-07-04-m1d-t3-jmpjsr-width-probes.md`),
and emits a **finished `Fragment::Data`** (opcode word + `Abs16Be`/`Abs32Be` fixup — link
resolves the value as today), advancing the cursor by the true width (4 or 6). Optimistic-
abs.w start makes the existing `env == prev` fixpoint inherently grow-only = asl's least
fixpoint. This is the exact mechanism T2 already applied to the absolute-EA class
(`abs_ea_from_expr`), now generalized to jmp/jsr and unified (T2's Poison→abs.l is flipped
to abs.w for asl-faithfulness). `resolve_layout` stays the live relaxer for hand-built IR
(m1b_gate) and becomes identity on the front-end path (no `JmpJsrSym` survives).

**Tech Stack:** Rust workspace (`sigil-frontend-as`, `sigil-backend-m68k`, `sigil-link`),
real `asl 1.42` golden oracle via `gen-snippet-vectors`, `SIGIL_STRICT_GATE` reference gates.

**Reference / probe:** `docs/superpowers/notes/2026-07-04-m1d-t3-jmpjsr-width-probes.md`
(the decisive `org $7FFA; jmp T; T:` → `4EF8 7FFE` least-fixpoint result and the design
decisions). Spec §T3: `docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md`.

**Process guardrails (every task):**
- Before any commit: `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness` + `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings`.
- Keep the aeon tree clean (do NOT build it; the reference is pinned at clean `9bacc93`).
- `gen-snippet-vectors` must regenerate as a no-op except the new `t3_*` blocks (non-circularity).
- m1b_gate (5 ok) and m1c_vector_table (1) must stay green throughout.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `crates/sigil-backend-m68k/src/lib.rs` | 68k lowering helpers | **Add** `lower_jmp_jsr_abs(is_jsr, target, width, span) -> DataFragment` (opcode+fixup at a chosen width) |
| `crates/sigil-frontend-as/src/eval.rs` | front-end driver | **Modify** the `lower_m68k` jmp/jsr branch (fold→width→finished Data→true-width advance); flip `abs_ea_from_expr` Poison→abs.w; raise `PASS_CAP` |
| `crates/sigil-frontend-as/tests/snippets_golden.txt` | asl golden vectors | **Add** 5 `t3_*` blocks |
| `crates/sigil-frontend-as/tests/stale_fold_repro.rs` | F2 reproducer | **Delete** 2 `*_documents_current_bug` tripwires; **un-ignore** the 2 correct tests |
| `crates/sigil-link/src/lib.rs` | linker | **Modify** `link()` symbol redefinition → diagnostic |
| `crates/sigil-frontend-as/src/eval.rs` | front-end | **Modify** `open_section_if_needed` `sec{vma}` auto-name → disambiguate collisions |

`resolve_layout` (`sigil-link/src/relax.rs`) is **unchanged** — it keeps its private
`lower_jmp_jsr` and the Org+JmpJsrSym guard for the hand-built-IR path. The guard simply
never fires on the front-end path anymore (no `JmpJsrSym` emitted there).

---

## Task 1: Backend width-parameterized jmp/jsr lowering

**Files:**
- Modify: `crates/sigil-backend-m68k/src/lib.rs` (add method after `lower_jmp_jsr_sym`, ~line 54)
- Test: same file's `#[cfg(test)] mod tests`

The front-end needs to build the finished abs.w/abs.l `jmp`/`jsr` Data fragment itself
(not defer to `resolve_layout`). The byte layout mirrors `relax.rs::lower_jmp_jsr`
(jmp 4EF8/4EF9, jsr 4EB8/4EB9; `.l = .w|1`; operand at offset 2; `Abs16Be`/`Abs32Be`),
but here it's driven by a front-end-chosen `AbsWidth`.

- [ ] **Step 1: Write the failing test**

Add to `crates/sigil-backend-m68k/src/lib.rs` `mod tests`:

```rust
#[test]
fn lower_jmp_jsr_abs_builds_absw_and_absl() {
    use sigil_ir::{AbsWidth, FixupKind};
    let w = M68kBackend.lower_jmp_jsr_abs(false, Expr::Sym("T".into()), AbsWidth::W, span());
    assert_eq!(w.bytes, vec![0x4E, 0xF8, 0x00, 0x00]);
    assert_eq!(w.fixups.len(), 1);
    assert!(matches!(w.fixups[0].kind, FixupKind::Abs16Be));
    assert_eq!(w.fixups[0].offset, 2);

    let l = M68kBackend.lower_jmp_jsr_abs(true, Expr::Sym("T".into()), AbsWidth::L, span());
    assert_eq!(l.bytes, vec![0x4E, 0xB9, 0x00, 0x00, 0x00, 0x00]);
    assert!(matches!(l.fixups[0].kind, FixupKind::Abs32Be));
    assert_eq!(l.fixups[0].offset, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-backend-m68k lower_jmp_jsr_abs_builds`
Expected: FAIL — `no method named lower_jmp_jsr_abs`.

- [ ] **Step 3: Implement the method**

Add after `lower_jmp_jsr_sym` (imports at the top of the file already bring in `DataFragment`,
`Expr`, `Fixup`, `FixupKind`, `Span`; add `AbsWidth` to the `sigil_ir` use if not present):

```rust
/// Lower a bare-symbol `jmp`/`jsr` at an ALREADY-CHOSEN width to a finished
/// `DataFragment` (opcode word + `Abs16Be`/`Abs32Be` fixup carrying `target`).
///
/// This is the front-end's width-selected path (M1.D T3): the front-end folds
/// the target and picks `width` via `asl_width_rule` in its own pass loop, so
/// the fragment's byte length is final and the cursor advances truthfully — no
/// deferral to `resolve_layout`. Byte layout matches the linker's private
/// `lower_jmp_jsr` (jmp 4EF8/4EF9, jsr 4EB8/4EB9; `.l = .w | 1`; operand at
/// offset 2). The value is still resolved by `link()`'s fixup pass.
pub fn lower_jmp_jsr_abs(
    &self,
    is_jsr: bool,
    target: Expr,
    width: AbsWidth,
    span: Span,
) -> DataFragment {
    let base: u16 = if is_jsr { 0x4EB8 } else { 0x4EF8 };
    match width {
        AbsWidth::W => DataFragment {
            bytes: vec![(base >> 8) as u8, (base & 0xFF) as u8, 0, 0],
            fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 2, target }],
            span,
        },
        AbsWidth::L => {
            let op = base | 0x0001;
            DataFragment {
                bytes: vec![(op >> 8) as u8, (op & 0xFF) as u8, 0, 0, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 2, target }],
                span,
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sigil-backend-m68k lower_jmp_jsr_abs_builds`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-backend-m68k/src/lib.rs
git commit -m "feat(sigil-backend-m68k): width-selected lower_jmp_jsr_abs for front-end path (T3)"
```

---

## Task 2: Front-end folds + width-selects jmp/jsr in the pass loop

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs` — the jmp/jsr branch in `lower_m68k`
  (currently ~2167-2178), `abs_ea_from_expr` (~2573-2589), `PASS_CAP` (line 25).

This is the load-bearing change. The behavioral proof is Task 4 (stale_fold_repro flips
green); this task makes it compile and keeps every existing gate green.

- [ ] **Step 1: Replace the jmp/jsr bare-symbol lowering**

In `lower_m68k`, replace the `if let [OperandAtom::Value(e)] = atoms.as_slice() { … }` body
(the block that calls `lower_jmp_jsr_sym` + `emit_fragment(frag, 4)`) with:

```rust
if let [OperandAtom::Value(e)] = atoms.as_slice() {
    let target = self.resolve_dollar(&self.qualify_expr(e));
    let is_jsr = matches!(mnemonic, M68kMnemonic::Jsr);
    // Width selection in the front-end pass loop (M1.D T3): fold the target
    // from the current-pass env and pick abs.w/abs.l via `asl_width_rule`.
    // Unknown-this-pass (Poison) → abs.w (OPTIMISTIC) — probe-verified as asl's
    // least fixpoint (grow-only): the multi-pass `env == prev` loop then only
    // ever grows a width W→L (label addresses are monotone-nondecreasing across
    // passes), so it converges to exactly asl's minimal widths. The finished
    // Data fragment carries the true length, so the cursor advances truthfully
    // and downstream section LMAs (`phys_base`) are correct by construction.
    // See docs/superpowers/notes/2026-07-04-m1d-t3-jmpjsr-width-probes.md.
    let width = match self.fold(&target) {
        Fold::Value(v) => asl_width_rule(v, false),
        Fold::Poison => {
            for name in self.unresolved_names(&target) {
                self.poison_refs.push((name, span));
            }
            AbsWidth::W
        }
    };
    let frag = self.m68k.lower_jmp_jsr_abs(is_jsr, target, width, span);
    self.emit_frag(Ok(frag), span);
    return;
}
```

Notes for the implementer (verified against eval.rs):
- `emit_frag(&mut self, frag: Result<DataFragment, LowerError>, span)` unwraps `Ok` and calls
  `self.emit` → `builder.emit_data(bytes, fixups, span)`, which advances the section cursor by
  `bytes.len()` (4 for abs.w, 6 for abs.l) — the true-width advance, which is the whole point.
  `lower_jmp_jsr_abs` returns a bare `DataFragment`, so wrap it: `self.emit_frag(Ok(frag), span)`.
- `asl_width_rule`, `AbsWidth`, `Fold` are already imported (used by `abs_ea_from_expr`). No
  `Fragment` import is needed (the finished path never constructs a `Fragment` here).
- `fold`, `unresolved_names`, `resolve_dollar`, `qualify_expr` all exist as `&self`/`&mut self`
  methods on `Asm` (`fold` is `&self`; `poison_refs.push` needs `&mut self` — the surrounding
  `lower_m68k` already has `&mut self`, so compute `width` in a `match self.fold(&target)` and
  push inside the `Poison` arm as shown).
- `resolve_dollar` mirrors the branch path (`lower_m68k_branch`) for a control-transfer
  target. If it causes ANY existing golden to churn, drop it back to bare `qualify_expr`
  (the old JmpJsrSym path used qualify only) and note why.
- Delete the now-stale comment block above the branch that references
  `lower_jmp_jsr_sym` / "width chosen later by the linker's resolve_layout" and replace with
  a one-line pointer to the new behavior.

- [ ] **Step 2: Unify `abs_ea_from_expr` — flip Poison to abs.w (optimistic)**

In `abs_ea_from_expr`, change the `Fold::Poison` arm from `M68kOperand::AbsL(0)` to
`M68kOperand::AbsW(0)` and update the doc comment. The probe
(`org $7FFA; lea T,a0; T:` → `41F8 7FFE`) proves the absolute-EA class is ALSO grow-only /
least fixpoint, so optimistic abs.w is the asl-faithful start here too:

```rust
        Fold::Poison => {
            for name in self.unresolved_names(&qualified) {
                self.poison_refs.push((name, span));
            }
            // Optimistic abs.w while unresolved (M1.D T3): asl selects the least
            // fixpoint for the absolute-EA class too (probe: lea at $7FFA → 41F8
            // 7FFE, abs.w). The multi-pass loop then only grows W→L, converging
            // to asl's minimal width. The converged pass re-folds to the real
            // value (or errors via poison_refs above).
            M68kOperand::AbsW(0)
        }
```

Also update the method's top doc comment: the sentence claiming "unresolved-this-pass symbol
folds to Poison → pessimistic abs.l (matching asl's forward-symbol width guess)" is now
FALSE — replace with the grow-only/optimistic description and cite the T3 probe.

- [ ] **Step 3: Raise `PASS_CAP`**

Width growth consumes extra convergence passes. `env == prev` remains the real convergence
signal; `PASS_CAP` only backstops runaway. Change line 25:

```rust
const PASS_CAP: usize = 16;
```

Update its doc/context if any references "8". (Task 7 measures the real ROM's pass count;
16 is a safe margin over the recon's current need plus width growth. If the full ROM needs
more, Task 7 raises it further with the measured number recorded.)

- [ ] **Step 4: Build + run the existing gates (no goldens yet)**

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: PASS. The existing `t2_*`/`t1_*` and unit goldens still pass (front-end path now
emits finished Data; resolve_layout is identity for those snippets — same final bytes).
`asl_snippets.rs` runs `resolve_layout` then `link`; with no `JmpJsrSym` it is a pass-through.

Run: `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness 2>&1 | tail -20`
Expected: m1b_gate (5 ok) + m1c_vector_table (1) green.

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail`
Expected: clean. If `lower_jmp_jsr_sym` / `emit_fragment` are now unused on the front-end
path, they remain used by tests / the hand-built path — do NOT delete them (m1b_gate and the
builder unit test construct `JmpJsrSym`). If clippy flags a genuinely-dead private item,
record it; do not remove public API.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-frontend-as/src/eval.rs
git commit -m "feat(sigil-m1d): front-end jmp/jsr width selection in the pass loop; unify abs-EA to grow-only optimistic (T3)"
```

---

## Task 3: Add `t3_*` snippet goldens (real asl)

**Files:**
- Modify: `crates/sigil-frontend-as/tests/snippets_golden.txt` (append after the last `t2_*` block, ~line 1320)

Small, unambiguous width cases that lock the front-end path's byte output to real asl. The
grow-to-abs.l + stale-fold behavioral proof is Task 4 (hermetic, `phase`-based, tiny output);
the decisive boundary proof lives in the probe notes doc. (Grow cases via `org` produce
32KB+ bins — impractical as goldens; the `phase` reproducer covers growth hermetically.)

- [ ] **Step 1: Append the golden blocks**

Append verbatim (bytes are from the committed probe matrix):

```
=== t3_jmp_abs_w ===
	cpu 68000
	padding off
	org 0
Low:	equ $100
	jmp Low
	rts
--- bytes ---
4E F8 01 00 4E 75
=== t3_jmp_abs_l ===
	cpu 68000
	padding off
	org 0
High:	equ $10000
	jmp High
	rts
--- bytes ---
4E F9 00 01 00 00 4E 75
=== t3_jsr_abs_w ===
	cpu 68000
	padding off
	org 0
Low:	equ $100
	jsr Low
	rts
--- bytes ---
4E B8 01 00 4E 75
=== t3_jsr_abs_l ===
	cpu 68000
	padding off
	org 0
High:	equ $10000
	jsr High
	rts
--- bytes ---
4E B9 00 01 00 00 4E 75
=== t3_jmp_fwd_low ===
	cpu 68000
	padding off
	org 0
	jmp Fwd
	nop
Fwd:	rts
--- bytes ---
4E F8 00 06 4E 71 4E 75
```

- [ ] **Step 2: Run the snippet gate (uses committed bytes, no asl)**

Run: `cargo test -p sigil-frontend-as --test asl_snippets 2>&1 | tail -20`
Expected: PASS including the 5 new `t3_*` cases.

- [ ] **Step 3: Regenerate from real asl — must churn ONLY the new blocks**

Run: `AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -p sigil-frontend-as --bin gen-snippet-vectors`
Then: `git diff --stat crates/sigil-frontend-as/tests/snippets_golden.txt`
Expected: the file is byte-identical to what you wrote (the 5 `t3_*` bytes match real asl;
no other block changed). If `git diff` shows churn in non-`t3_` blocks, STOP — the
non-circularity invariant is violated; investigate before proceeding.

Run: `git diff crates/sigil-frontend-as/tests/snippets_golden.txt`
Expected: empty (or only whitespace-identical). If the regen reformats spacing, commit the
regenerated form.

- [ ] **Step 4: Commit**

```bash
git add crates/sigil-frontend-as/tests/snippets_golden.txt
git commit -m "test(sigil-m1d): t3_* jmp/jsr width goldens (asl-verified; regen no-op) (T3)"
```

---

## Task 4: Flip the stale-fold reproducer green (the F2 acceptance)

**Files:**
- Modify: `crates/sigil-frontend-as/tests/stale_fold_repro.rs`

The two `#[ignore = "flips green in T3"]` tests assert asl-correct output; they must now
PASS by default. The two `*_documents_current_bug` tripwires pin the OLD wrong output and
must be DELETED.

- [ ] **Step 1: Delete the tripwires and un-ignore the correct tests**

- Remove `fn dc_l_after_grown_jmp_documents_current_bug()` (and its `#[test]`) entirely.
- Remove `fn downstream_section_lma_documents_current_bug()` (and its `#[test]`) entirely.
- Remove the `#[ignore = "…"]` attribute line above `fn dc_l_after_grown_jmp_folds_correctly()`.
- Remove the `#[ignore = "…"]` attribute line above `fn downstream_section_lma_reflows_after_growth()`.
- Delete the now-unused `SINGLE_CURRENT_BUGGY` const (referenced only by the deleted test).
- Update the module doc comment: replace the "flips green in T3" / "T3 deletes them" framing
  with a past-tense statement that T3 landed the front-end width selection and these assert
  the now-correct output. Keep the defect explanation as the historical rationale.

- [ ] **Step 2: Run the reproducer — both must pass by default**

Run: `cargo test -p sigil-frontend-as --test stale_fold_repro 2>&1 | tail -20`
Expected: `dc_l_after_grown_jmp_folds_correctly` PASS (flatten = `4EF9 0001 0006 0001 0006`),
`downstream_section_lma_reflows_after_growth` PASS (SecondSection lma = `$0A`). 2 passed, 0
ignored, 0 filtered-out tripwires.

- [ ] **Step 3: Commit**

```bash
git add crates/sigil-frontend-as/tests/stale_fold_repro.rs
git commit -m "test(sigil-m1d): stale-fold + downstream-LMA reproducers flip green (F2 closed; T3)"
```

---

## Task 5: Harden `link()` symbol redefinition into a diagnostic

**Files:**
- Modify: `crates/sigil-link/src/lib.rs` — Pass 1 of `link()` (~lines 53-61), the doc (~47-49).

Audit-flagged: `link()` silently last-write-wins on a name defined by multiple
sections/stubs. At full-ROM link this masks real collisions. Emit a diagnostic instead.

- [ ] **Step 1: Write the failing test**

Add to `crates/sigil-link/src/lib.rs` `mod tests` (or the nearest link-test module):

```rust
#[test]
fn link_reports_duplicate_symbol_definition() {
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SymbolTable};
    let mk = |name: &str| Section {
        name: name.into(),
        cpu: Cpu::M68000,
        vma_base: Some(0),
        lma: 0,
        labels: vec![Label { name: "Dup".into(), offset: 0 }],
        fragments: vec![Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() })],
    };
    let err = link(&[mk("a"), mk("b")], &SymbolTable::new()).unwrap_err();
    assert!(err.iter().any(|d| d.message.contains("Dup") && d.message.to_lowercase().contains("redefin")),
        "got: {:?}", err);
}
```

(Reuse the module's existing `sp()`/`span()` helper; if none, add `fn sp() -> Span { Span { source: SourceId(0), start: 0, end: 0 } }`.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p sigil-link link_reports_duplicate_symbol_definition`
Expected: FAIL — link currently returns `Ok` (last-write-wins).

- [ ] **Step 3: Implement the collision check**

In `link()` Pass 1, before `syms.define(&label.name, …)`, detect a name already defined by a
PRIOR section/stub and push a diagnostic. Because `stubs` legitimately pre-defines external
leaf symbols, only flag a clash between two SECTION labels (a section label colliding with a
stub is the intended external-resolution case — do not flag that). Track section-defined
names in a local set:

```rust
    let mut syms = stubs.clone();
    let mut defined_here: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for sec in sections {
        let origin = sec.vma_origin();
        for label in &sec.labels {
            if let Some(prev) = defined_here.insert(label.name.clone(), sec.name.clone()) {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!(
                        "symbol `{}` redefined by section `{}` (already defined by section `{}`)",
                        label.name, sec.name, prev
                    ),
                    primary: Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
                });
            }
            syms.define(&label.name, SymbolValue::Int((origin + label.offset) as i64));
        }
    }
    if diags.iter().any(|d| d.level == Level::Error) {
        return Err(diags);
    }
```

Confirm `Level` and `Diagnostic` are already imported in this file (they are — `link` returns
`Vec<Diagnostic>`). Update the doc comment at lines 47-49 to state redefinition is now a hard
diagnostic (drop the "lands in Plan 4" note).

- [ ] **Step 4: Run to verify it passes + no regression**

Run: `cargo test -p sigil-link 2>&1 | tail -20`
Expected: the new test PASS; all existing link/relax tests still PASS (they use distinct
label names per section).

Run: `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness 2>&1 | tail`
Expected: m1b_gate + m1c_vector_table still green (real sections have unique labels).

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/src/lib.rs
git commit -m "feat(sigil-link): duplicate section-symbol definition is a hard diagnostic (T3 hardening)"
```

---

## Task 6: Disambiguate `sec{vma}` auto-name collisions

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs` — `open_section_if_needed` (~1642-1655).

Audit-flagged: `open_section_if_needed` names each auto-opened section `sec{vma_base}`. Two
sections phased at the same VMA base (e.g. a future second bank re-phased at the same address,
or an `org`-reopened region) would collide — and with Task 5, `link()` now HARD-ERRORS on the
duplicate labels that would follow. Add an ordinal suffix on repeat.

- [ ] **Step 1: Write the failing test**

Add to `eval.rs` `mod tests`. Two phased blocks at the same VMA base must yield distinct
section names:

```rust
#[test]
fn duplicate_vma_base_sections_get_distinct_names() {
    // Two `phase $8000` blocks separated by `dephase` both auto-open at vma_base
    // 0x8000; their section names must not collide.
    let src = "\
        cpu 68000\n\
        phase $8000\n\
        dc.b 1\n\
        dephase\n\
        phase $8000\n\
        dc.b 2\n\
        dephase\n";
    let module = crate::run(src, &crate::Options::default()).expect("assemble");
    let names: Vec<&str> = module.sections.iter().map(|s| s.name.as_str()).collect();
    let unique: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "section names collided: {names:?}");
}
```

(Verify the exact `phase`/`dephase` spelling and that `run`/`Options` are reachable from the
test module — mirror an existing `eval.rs` test's imports. If `phase`/`dephase` isn't the
right shape to open two same-base sections, use two `org $8000 … org` reopen blocks that the
front-end turns into distinct auto-sections; the invariant under test is name-uniqueness.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p sigil-frontend-as duplicate_vma_base_sections_get_distinct_names`
Expected: FAIL — both sections named `sec32768`.

- [ ] **Step 3: Implement ordinal disambiguation**

Add a `used_section_names: std::collections::HashMap<String, u32>` field to `Asm` (init empty
in `Asm::new`). In `open_section_if_needed`, after computing `let name = format!("sec{vma_base}")`:

```rust
            let base = format!("sec{vma_base}");
            let name = match self.used_section_names.get_mut(&base) {
                Some(n) => {
                    *n += 1;
                    format!("{base}#{n}")
                }
                None => {
                    self.used_section_names.insert(base.clone(), 0);
                    base
                }
            };
```

The first occurrence keeps the bare `sec{vma}` name (so the M0 harness / gate that keys on
`sec0`/`sec32768` is unaffected — verify those regions open exactly once each); repeats get
`sec{vma}#1`, `#2`, …

- [ ] **Step 4: Run to verify it passes + no regression**

Run: `cargo test -p sigil-frontend-as duplicate_vma_base_sections_get_distinct_names`
Expected: PASS.

Run: `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness 2>&1 | tail`
Expected: m0/m1b/m1c gates green — the two real regions (`sec0`, `sec32768`) each still open
once, so their names are unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-frontend-as/src/eval.rs
git commit -m "fix(sigil-m1d): disambiguate duplicate sec{vma} auto-section names (T3 hardening)"
```

---

## Task 7: Full-workspace verification + recon pass-count check

**Files:** none (verification + possible `PASS_CAP` tune).

- [ ] **Step 1: Full strict gate + workspace + clippy**

```bash
SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness 2>&1 | tail -25
cargo test --workspace 2>&1 | tail -25
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail
```
Expected: all green; clippy clean. Record the counts.

- [ ] **Step 2: Recon still assembles (arms T4)**

```bash
AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -p sigil-harness --example m1c_full 2>&1 | tail
```
Expected: `ASSEMBLED OK: 8 sections`, 0 diagnostics (T3 must not regress recon-0). If it now
reports a non-convergence error, `PASS_CAP` is too low — raise it to the measured need + a
small margin and note the number in the commit + memory.

- [ ] **Step 3: Verify the aeon tree is still clean**

```bash
git -C /home/volence/sonic_hacks/aeon status --short
```
Expected: empty (we never built it).

- [ ] **Step 4: Commit any PASS_CAP tune (only if Step 2 required it)**

```bash
git add crates/sigil-frontend-as/src/eval.rs
git commit -m "fix(sigil-m1d): raise PASS_CAP to N for width-growth convergence (measured; T3)"
```

- [ ] **Step 5: Update spec + memory**

- Mark T3 ✅ DONE in `docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md` §T3 with:
  the probe finding (asl is grow-only/least-fixpoint — optimistic abs.w, NOT the spec's
  expected pessimistic-long), the fragment decision (finished Data, no JmpJsrSym on the
  front-end path), the abs-EA unification, the PASS_CAP value, and the two hardening items.
- Update the `sigil-m0-core-progress` memory note's M1.D paragraph: T3 done, the grow-only
  finding, stale_fold_repro green, next = T4 (first full-ROM sha256).

---

## Self-Review notes (spec coverage)

- **F2 stale-fold** → Task 2 (true-width cursor) + Task 4 (reproducer green). ✅
- **F2 downstream-LMA (unflagged half)** → Task 2 (`phys_base` accumulates true `current_offset`)
  + Task 4 (`downstream_section_lma_reflows_after_growth`). ✅
- **Probe-first open semantic** → done pre-plan; committed
  `2026-07-04-m1d-t3-jmpjsr-width-probes.md` (refuted the spec's expected pessimistic-long;
  established grow-only optimistic). ✅
- **$FF8000 oscillation unreachable** → probe grep (22 targets, all code labels); documented
  in Task 2 Step 1 comment + probe doc; PASS_CAP backstop. ✅
- **PASS_CAP growth-aware** → Task 2 Step 3 (raise to 16) + Task 7 Step 2 (measure/tune). ✅
- **resolve_layout as verification assert + live relaxer for m1b** → unchanged; identity on
  front-end path (no JmpJsrSym), live for hand-built IR (m1b_gate untouched). Recorded. ✅
- **Fragment representation decision recorded** → probe doc "Design decisions" + this plan's
  Architecture. ✅
- **link() symbol redefinition** → Task 5. ✅
- **sec{vma} collision** → Task 6. ✅
- **Acceptance (goldens + gates green, 11 sites → 4EF9)** → Tasks 3/4/7; the `$1012E` full-ROM
  spot-assert lands in T4's `m1c_rom` gate (needs the emit path, out of T3 scope — T3 arms it).
```
