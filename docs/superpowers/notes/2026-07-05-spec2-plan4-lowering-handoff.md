# Handoff — Sigil Spec 2 Plan 4 (IrStreamer + lowering)

**Purpose:** orient a fresh session (or Volence) to start Plan 4 cleanly, the way the Plan 2 → Plan 3
handoffs did. Written 2026-07-05 right after Plan 3 (types & layout engine) merged to master
(`4ff5a0a`). This is the *orientation* doc; a detailed T-task plan doc should be written
(`superpowers:writing-plans`) after the ⚠️ prerequisite below is resolved and the early decisions are
settled, mirroring `empyrean/docs/plans/2026-07-05-sigil-spec2-p3-types-layout.md`.

## ⚠️ PREREQUISITE — Plan 4 is BLOCKED on Sigil Core's IR/backend being ready for `.emp` lowering

Unlike Plans 2 & 3 (pure front-end, `sigil-frontend-emp` depends on `sigil-span` ONLY), **Plan 4 is
the plan where the `.emp` front-end first reaches the IR/backend.** It needs Core to provide:
- an **IR streaming** target (the `IrStreamer` the spec names — §9 reserves `ProvFrame::Comptime`),
- **`Fixup`** kinds (`Abs32Be`, `BankPtr16Le`, …) to resolve `Value::Data`'s `SymRef` cells,
- **section placement** (`Section.vma_base`, the `phase`/`dephase` → `vma:` lowering, §7.1),
- the **instruction encoder / operand classes** (`sigil-isa`) that `asm{}` splices type-check against,
- branch/EA relaxation (Core §5.4) and `@as_compat` encoder-choice reproduction (Core §5.6, §8.2).

**First action for the Plan-4 agent:** assess Core readiness. The AS-source byte-exact assembler track
(M0→M1.D) is COMPLETE and merged (`s4.bin` byte-exact, see `docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md`
and the memory note `sigil-m0-core-progress.md`). But confirm the Core IR + `sigil-isa` expose a
stable **programmatic** streaming/encoding API that a NEW front-end can emit into (M1.C was the AS
front-end; Plan 4 is a SECOND front-end into the same IR). If that API isn't ready, Plan 4 waits or
its first task is defining that seam WITH the Core owner. Do NOT start lowering against a moving IR.

## What Plan 3 shipped (the foundation Plan 4 extends)

`sigil-frontend-emp` (still `sigil-span`-only) now produces fully **checked comptime values** from
parsed `.emp`: the `Value` model (`src/value.rs`), the tree-walking evaluator (`src/eval/` module
tree — `mod`, `env`, `expr`, `call`, `control`, `builtins`, `guards`, `pattern`, `typed`, `literals`,
`emit`), and the types/layout engine (`src/layout.rs`). **446 tests.** Concretely Plan 4 builds on:
- **`Value::Data(DataBuf)`** — the checked, **CPU-neutral** emission product. `DataBuf { cells, size }`
  where `Cell = Scalar { value, width, signed } | Bytes(Vec<u8>) | SymRef { name, width }`. Ranges are
  already checked; layout/offsets/`(size:)` already verified. **This is the primary Plan-3→Plan-4
  hand-off object** (see the seam below).
- **`Value::Typed`** (newtype/fixed/refinement — erasing), struct/bitfield/enum **layout**
  (`layout_of_struct`/`layout_of_bitfield`, `sizeof`/`offsetof`), the one `check_in_range` mechanism,
  comptime sum types + exhaustive `match`, `data NAME: T = expr` items (`resolve_data`/`eval_data`).
- Plan 2's comptime core: pure exprs, `const`, `comptime fn` (recursion-bounded), control flow,
  §6.8 builtins, lambdas/`|>`, guards, step budget.

## The Plan 3 / Plan 4 seam (settle this FIRST — it's the spine of Plan 4)

§4.5 / decision **D-P3.5** split `data` emission deliberately:
- **Plan 3 (DONE):** `data` produces a checked, typed `Value::Data` — a CPU-neutral cell list with
  every scalar range-checked and laid out per the struct/array/bitfield layout. Pointer-typed fields
  became `Cell::SymRef { name, width: 4 }` placeholders (68k `Abs32` default, D-P3.7). NO endianness
  committed, NO address resolution.
- **Plan 4 (THIS):** the **`IrStreamer` serializes each `Cell` into its section**: `Scalar`/`Bytes`
  emit with the enclosing section's CPU byte order (68k big-endian, Z80 little-endian, §4.5/§7.2), and
  each `SymRef` resolves to the correct `Fixup` kind (`Abs32Be`, `BankPtr16Le`, `BankPtr16Be`, …) by
  the reference's CPU/section context (§7.2 — the `convsym` z-filter class is unrepresentable). The
  `data`-item entry (`layout::eval_data`) already returns the `DataBuf`; Plan 4 walks it to bytes+fixups.

Because Plan 3 kept the cells STRUCTURED (not a flat blob), Plan 4 has exactly what it needs for byte
order + fixups. Confirm this contract in the Plan-4 plan doc before writing the streamer.

## ⚠️ Grammar / value-model gaps — Plan 4 must extend the front-end first (like every prior plan)

VERIFIED pattern from Plans 2 & 3: each plan opened by extending the Plan-1 surface. Plan 4 needs
(verify current parse state against `src/ast.rs`/`src/parser.rs` — some may partially parse):
- **`asm { … }` templates + `{expr}` splices (§6.2)** — the ONE metaprogramming shape besides
  `comptime fn` (D2.6). Splices accept integers/registers/`Width`/`Cc`/labels/`Code`/`Data` and are
  **typed by operand class** (a wrong-kind splice is a named error — the Racket `~describe` behavior,
  via the backends' operand classes). `Expr::Asm`/`AsmStmt` exist in the AST but evaluate to `Poison`
  today (Plan 2/3 placeholder) — Plan 4 makes `asm{}` produce a **`Value::Code`**.
- **`Value::Code`** and the operand-class comptime values **`Width` (.b/.w/.l), `Cc`, `Dreg`/`Areg`/
  `Reg`, `Operand`** — all deferred by Plan 2 (D-P2.2: "represent absent, not stubbed"). Plan 4 makes
  them constructible. `Code` is a monoid (`Code.empty`/`++`, §6.8) like `Data`.
- **Procs (§5.1)** — `proc name(params) clobbers(...) falls_into ... { <asm body> }` parse today, but
  the body is `Vec<AsmStmt>` that is NOT lowered. Plan 4 lowers proc bodies to IR (instruction
  encoding via `sigil-isa`), declaration-order placement, **declared-fallthrough** enforcement, the
  `clobbers` lint.
- **Label hygiene (§5.3)** — `.name:` labels fresh-per-`asm{}`-instantiation; `export .name:` for
  caller-visible ones. "The entire hygiene model" — implement as the one rule.
- **`patch name: T` / `bind name = expr` (§6.4)** — typed emit-forward-bind-later slots →
  an IR `Fixup`; unbound/double-bound is an error. `patch`/`bind` are contextual keywords (§10) —
  check parse state.
- **Sections (§7)** — `section name (cpu: z80, vma: $8000) { … }` placement + the cross-CPU fixup
  selection (§7.2). `SectionDecl` parses; Plan 4 lowers placement + `here()`/`vma:`.

Do the front-end/value-model extension as its own reviewed sub-task(s) up front (the T1-before-engine
shape that worked in Plans 2 & 3), then build the streamer/lowering on it.

## Scope

**IN (Plan 4):** `Value::Code` + `asm{}` instantiation (operand-class-checked splices) + the `Code`
monoid; `IrStreamer` serializing `Value::Data` (byte order + `SymRef`→`Fixup`) and `Code` (instruction
encoding via `sigil-isa`) into sections; proc lowering (bodies → IR, placement, declared-fallthrough,
`clobbers` lint); label hygiene (fresh-per-instantiation + `export`); `patch`/`bind` → `Fixup`; section
placement + `vma:`/`here()` + cross-CPU fixup selection (§7.2); `ProvFrame::Comptime` provenance so an
error inside a generated table names the generator call site (§9). **This is the plan where
`sigil-frontend-emp` gains an IR/backend dependency — the `sigil-span`-only invariant ends here, by
design.**

**OUT (later plans):** `embed`/`import`/`zx0` capability sandbox + `as.*` bit-compat float — **Plan 5**.
`@as_compat` + mixed `.asm`+`.emp` build + port diff — **Plan 6**. The S2-D6/D7 **register/machine-state
contract lints** (checked clobbers, CCR liveness, stack-delta, cycle budgets, stopZ80 pairing) ride
Plan 4's lowering (it provides instruction-level knowledge) but are a **lint layer** — schedule them as
a distinct sub-milestone, not gating the core streamer. Cross-module `use`/prelude resolution still
deferred (S2-D3; the prelude is data, finalized at first real port).

## Design decisions to make early (load-bearing)

1. **The Core IR seam.** What exact API does `IrStreamer` emit into? (Instruction records? A byte+fixup
   stream? Reuse the M1.C AS front-end's lowering path or a parallel one?) Settle WITH the Core owner.
2. **`Value::Code` representation.** A quoted instruction template with typed holes, or a lowered-IR
   fragment with splice points? How `asm{}` splices type-check operand classes against `sigil-isa`.
3. **The crate dependency change.** `sigil-frontend-emp` gains a dep on the IR/`sigil-isa` crate(s).
   Decide the crate boundary so the pure comptime evaluator stays testable in isolation (maybe a
   `sigil-frontend-emp` core + a `sigil-lower` crate that depends on both it and the backend).
4. **Byte-order + fixup resolution** for `Value::Data` cells (the seam above) — the `SymRef`→`Fixup`
   mapping table by CPU/section.
5. **Label hygiene mechanism** (fresh-per-instantiation) — how `asm{}` instantiation stamps unique
   labels and `export` opts out.
6. **`@as_compat` reproduction readiness** (Core §5.6) — data-only `.emp` files can lower immediately;
   instruction-bearing files need the encoder-choice reproduction, an M1 deliverable — confirm it exists.

## Suggested task shape (turn into a real plan doc)

Rough, TDD, commit-per-task, mirroring Plans 2 & 3's cadence:
- **T0** — confirm the Core IR/`sigil-isa` seam; stand up the crate boundary (`sigil-lower` or the dep
  addition) with a trivial round-trip (emit one `dc.b` byte + one fixup, diff a tiny section).
- **T1** — front-end: `asm{}` + `{}` splices, `Value::Code` + operand-class values (`Width`/`Cc`/`Reg`/
  `Operand`), `Code` monoid; `patch`/`bind` parse if not already. *(reviewed sub-task)*
- **T2** — `IrStreamer` for `Value::Data`: byte order + `SymRef`→`Fixup`. *(load-bearing — the seam)*
- **T3** — `asm{}` instantiation → `Code` → IR (instruction encoding via `sigil-isa`, splice typing).
- **T4** — proc lowering: bodies → IR, declaration-order placement, declared-fallthrough, `clobbers`.
- **T5** — label hygiene (fresh-per-instantiation + `export`); `patch`/`bind` → `Fixup`.
- **T6** — sections: placement, `vma:`/`here()`, cross-CPU fixup selection (§7.2).
- **T7** — corpus: a small `.emp` module lowered to bytes and diffed (the Appendix D pitcher-plant
  everyday case is the natural target once procs lower) + whole-branch review.
- **(later sub-milestone)** — S2-D6/D7 contract lints.

## Process to keep (it worked in Plans 2 & 3 — caught ~16 defects in Plan 3 alone)

- Subagent-driven with **two-stage reviews** (spec compliance THEN code-quality via
  `superpowers:code-reviewer`) on the load-bearing tasks; TDD per task; commit after each; green gate
  (`cargo test` + `cargo clippy --workspace --all-targets -- -D warnings`) before every commit.
- **Add a whole-branch review at the end** — in Plans 2 & 3 it caught the CRITICAL cross-feature bug
  the isolated reviews missed (Plan 3: a self-referential newtype `where` bound crashing via native
  stack overflow — the guard covered one re-entry path but not a sibling). Byte-diff against a
  reference wherever a byte argument exists (§8.3).
- Ground semantics in the **spec** (`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §5/§6.2/§6.4/§7/§9 + D2.6)
  and, where AS/`sigil-isa` is the reference (instruction encoding, operand widths, byte order, fixup
  kinds), cross-check against Core. Record every design call in the plan doc (`D-P4.x` numbering).
- Milestone boundary: Plan 4 is a milestone — do NOT merge to master without a Volence checkpoint,
  same as Plans 2 & 3.

## Acceptance sketch

- A small `.emp` module (data items + a `comptime fn` emitting `asm{}` + a `proc`) lowers to bytes +
  fixups and **byte-diffs clean** against a reference (§8.3); a wrong-kind `asm{}` splice, an unbound
  `patch`, a fallthrough-into-a-separated-proc, and a cross-CPU pointer without `winptr` each produce a
  **named, spanned** diagnostic (§9) with `ProvFrame::Comptime` provenance where generated.
- `cargo test` green; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Reference: the merged Plan 3 handoff `docs/superpowers/notes/2026-07-05-spec2-plan3-types-layout-handoff.md`
  (`## FINAL STATE`) for what the front-end already provides.
