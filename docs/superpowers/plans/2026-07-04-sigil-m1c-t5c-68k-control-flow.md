# Sigil M1.C — T5c: 68k Control Transfer + PC-Relative Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.
> HIGH-LATITUDE: touches the front-end→linker `JmpJsrSym`/`resolve_layout` handoff.

**Goal:** Assemble 68000 control transfer + PC-relative byte-exact vs `asl`: `bra`/`bsr`/`Bcc`
(→ `lower_branch`), `Dbcc` (`dbf`/`dbra`), `jmp`/`jsr` (→ `lower_jmp_jsr_sym`, resolved by the
linker's `resolve_layout` width fixpoint), `Scc` (→ `lower_inst`), and PC-relative EAs
`(d16,PC)`/`(d8,PC,Xn)` (→ `lower_pcrel_ea`). All targets **dotted-local-qualified** (D6).

## Verified interfaces
- `M68kBackend::lower_branch(mnemonic: Mnemonic, size: Size, target: Expr, span) -> Result<DataFragment>` (lib.rs:59) — `bra`/`bsr`/`Bcc`; size from the `.s`→`Size::S`/`.w`→`Size::W` suffix (M1.A: Aeon branches are 100% size-pinned, so read the suffix; no relaxation).
- `M68kBackend::lower_jmp_jsr_sym(is_jsr: bool, target: Expr, span) -> Fragment` (lib.rs:52) — returns a `Fragment::JmpJsrSym` (NOT a `DataFragment`); the linker's `resolve_layout` selects abs.w/abs.l width and lowers it to `Data`.
- `M68kBackend::lower_pcrel_ea(inst: &Instruction, pcd16_offset: u32, target: Expr, span) -> Result<DataFragment>` (lib.rs:101) — `(d16,PC)`.
- `qualify_expr(&self, e: &Expr) -> Expr` (eval.rs, ~1173) — qualifies `.local` → `Scope.local`. **MUST** be applied to every branch/jmp/jsr/pcrel target before lowering (the linker resolves `JmpJsrSym` in GLOBAL scope). The Z80 path already does this in `build_operands`.
- `m68k::Cond` (m68k.rs:49): 16 codes `T,F,Hi,Ls,Cc,Cs,Ne,Eq,Vc,Vs,Pl,Mi,Ge,Lt,Gt,Le`.

## Critical integration — wire `resolve_layout` into the link path
`sigil_link::link()` does NOT call `resolve_layout`; it `unreachable!`s on `JmpJsrSym`
(lib.rs:101). M1.B's own gate composes them as (m1b_gate.rs:93):
```rust
let resolved = sigil_link::resolve_layout(&sections, &stubs, /*dash_a=*/true)?;
let linked   = sigil_link::link(&resolved, &stubs)?;
```
The `asl_snippets` test helper (`assemble_bytes`) currently calls `link(module.sections)`
directly. **Update it to insert `resolve_layout(&module.sections, &SymbolTable::new(), true)`
before `link`** so jmp/jsr snippets don't panic. (`dash_a=true`: the real ASFLAGS include `-A`;
M1.B proved `-A` is irrelevant to jmp/jsr width, but pass `true` to match the reference build.)

## How to emit a raw `Fragment` (jmp/jsr)
`emit_frag` takes `Result<DataFragment>`; `lower_jmp_jsr_sym` returns a `Fragment`. Find how the
front-end pushes a raw `Fragment` into the current section (the `IrBuilder` in `sigil-ir` — look
for a `push_fragment`/`emit_fragment` method, or add one). The Z80 path never needed this
(no variable-length fragment); the m68k jmp/jsr path is the first front-end producer of
`JmpJsrSym`.

## Scope
- `bra`/`bsr`/`Bcc<cc>` with `.s`/`.w` size → `lower_branch`. Move these out of `m68k_out_of_scope`.
- `Dbcc` (`dbf`==`dbra`==`Dbcc(F)`... verify: `dbf`/`dbra` map to `Dbcc(Cond::F)`; `dbeq` etc. to the cc). Register operand `dn` + PC-relative target. Check the M1.A backend/corpus for how symbolic `Dbcc` is encoded: if the corpus takes a resolved `Disp`, FOLD the target → `disp = target - (pc_of_disp_word)` via `self.here()` and emit via `lower_inst` with a `Disp` operand; only reach for a fixup path if fold can't reproduce asl. Verify against asl bytes.
- `jmp`/`jsr` (bare symbol) → `lower_jmp_jsr_sym(is_jsr, qualified_target, span)`; emit the raw `JmpJsrSym` fragment. `jmp`/`jsr` with a NON-bare-symbol EA (e.g. `jmp (a0)`, `jsr (d16,pc)`) → NOT JmpJsrSym; encode via `lower_inst`/`lower_pcrel_ea`.
- `Scc<cc> <ea>` → `lower_inst` (condition-coded straight-line; move out of `m68k_out_of_scope`).
- `(d16,PC)` / `(d8,PC,Xn)` operands (T5 emits a T5b/T5c-deferral diagnostic for these) →
  parse into new atoms; lower via `lower_pcrel_ea` (compute `pcd16_offset`). Qualify the target.

## Steps (TDD)
- [ ] **Step 0 — wire `resolve_layout`** into `asl_snippets`'s `assemble_bytes` helper. Re-run the existing snippet suite — must stay green (no jmp/jsr yet, so resolve_layout is a no-op over pure-Data sections).
- [ ] **Step 1 — snippets first** (under `cpu 68000`, with a local target `Lbl:` so branches/jmp resolve). Cover: `bra.w Lbl`, `bra.s Lbl`, `bsr.w Lbl`, `beq.w Lbl`, `bne.s Lbl`, `bcc.w Lbl`, `dbf d0,Lbl`, `dbeq d1,Lbl`, `jmp Lbl`, `jsr Lbl`, `jmp (a0)`, `scc d0` / `seq d1`, `move.w (Lbl,pc),d0` (PC-rel data read). Put the target label at a known offset. Distinct names.
- [ ] **Step 2 — golden bytes from real asl** (direct `asl`+`p2bin`; note `bra`/`jmp` disp/width depends on the label's offset — keep snippets small and deterministic). Commit.
- [ ] **Step 3 — gate fails.**
- [ ] **Step 4 — mnemonic recognition:** add `bra/bsr/Bcc/Dbcc/Scc/jmp/jsr` to `m68k_mnemonic` (parse the condition-code suffix for `b<cc>`/`db<cc>`/`s<cc>` → `Cond`); remove from `m68k_out_of_scope`. Unit-test cc parsing (all 16).
- [ ] **Step 5 — lowering** in `lower_m68k`: route bra/bsr/Bcc → `lower_branch`; jmp/jsr(bare sym) → `lower_jmp_jsr_sym` + raw-fragment emit; jmp/jsr(EA) → normal; Dbcc → fold-disp via `lower_inst` (or fixup if needed); Scc → `lower_inst`; pc-rel operands → `lower_pcrel_ea`. **Qualify every target with `qualify_expr` first.** Unit-test dotted-local qualification (a `bra.w .loop` inside a global scope resolves to `Global.loop`).
- [ ] **Step 6 — gate green + suite.** asl_snippets PASS; `cargo test --workspace` PASS.
- [ ] **Step 7 — clippy + build clean.**
- [ ] **Step 8 — commit** `feat(sigil-frontend-as): 68k control transfer + PC-relative (branches/jmp/jsr/Dbcc/Scc, JmpJsrSym via resolve_layout, asl-gated)`.

## KNOWN DEEP RISK — flag for T10/M1.D, do NOT solve here
The linker (`resolve_layout`) selects jmp/jsr width AFTER the front-end computes label values. If
a label whose position depends on a variable-width jmp/jsr is **folded** into data bytes at
front-end time (e.g. `dc.l LabelAfterJsr`), and the linker later grows that jsr abs.w→abs.l,
the folded value is stale (the linker shifts the symbol + re-resolves *fixups*, but cannot fix
already-folded bytes). T5c's small snippets won't exercise this; the full-ROM path (T10/M1.D)
will. **If a T5c snippet with a folded-label-after-jmp diverges, STOP and report** — do not
paper over it. Otherwise note it in the commit body as a T10 watch-item.

## Self-Review
- Spec coverage: branches, Dbcc, jmp/jsr, Scc, pc-rel — each snippet-gated. Qualification unit-tested.
- Honest gate: bytes from real asl; the resolve_layout wiring makes jmp/jsr snippets real, not stubbed.
- Escalate if: the raw-`Fragment` emit path isn't cleanly available, Dbcc fold doesn't match asl, or the deep-risk folded-label case appears.
