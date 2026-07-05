# Handoff — Sigil Spec 2 Plan 3 (types & layout engine)

**Purpose:** orient a fresh session (or Volence) to start Plan 3 cleanly, the way the Plan 2
handoff did. Written 2026-07-05 right after Plan 2 (the comptime evaluator) landed on-branch. This
is the *orientation* doc; a detailed T-task plan doc should be written (superpowers:writing-plans)
before execution, mirroring `empyrean/docs/plans/2026-07-05-sigil-spec2-p2-emp-evaluator.md`.

## Prerequisite: Plan 2 must be checkpointed first

Plan 3 is **blocked on Plan 2** (it builds directly on the `Value` model + evaluator). Plan 2 is
DONE on branch `spec2-p2-emp-evaluator` (HEAD `db75176`, off master `11aaf0d`) but **NOT merged** —
it's a milestone awaiting Volence's review. So:

- **Branch Plan 3 off master AFTER Plan 2 merges** (preferred), OR off `spec2-p2-emp-evaluator` if
  starting before the merge. Do not start Plan 3 on a stale master.
- Read the Plan 2 handoff `docs/superpowers/notes/2026-07-05-spec2-plan2-evaluator-handoff.md` and
  its decision list **D-P2.1..D-P2.19** in the Plan 2 plan doc first — Plan 3 replaces several
  deliberate Plan-2 placeholders (below).

## What Plan 2 shipped (the foundation Plan 3 extends)

`sigil-frontend-emp` (depends on `sigil-span` ONLY — keep it that way in Plan 3; the layout engine
is still pure front-end logic, no IR/backend/Core dep): `src/value.rs` (the comptime `Value` model)
and `src/eval.rs` (~2160-line tree-walking evaluator). 258 tests. Plan 2 evaluates pure exprs,
consts, `comptime fn`, control flow, §6.8 builtins, lambdas/`|>`, guards. It deliberately left these
**Plan-3-shaped placeholders** you will now make real:

- **struct/enum are VALUE-only** (D-P2.14): `Ty{...}` → `Value::Struct{ty_name, fields}` with NO
  layout, NO `(size:)` check, NO field/type validation, NO default-fill; `E.V` → nullary
  `Value::Enum`. Plan 3 adds the layout + all the checking + payload variants.
- **`Value` kinds `Data`/`Code`/`Width`/`Cc`/`Reg`/`Operand` are absent** (D-P2.2). Plan 3 makes
  **`Data`** constructible (checked byte buffer + the `Data.empty`/`++` monoid, §6.8). `Code` and the
  operand-class values stay deferred to **Plan 4** (asm/backend).
- **comptime `int` is `i128`, "range-checked on emission"** (D-P2.1): Plan 3 owns the emission
  range-check (i128 → sized `u8/i8/u16/i16/u32/i32` with out-of-range → diagnostic).
- **`none` → `Unit`** is a placeholder that meets real `Option`/sum-types here.

## What Plan 3 is (from the plan sequence)

> **Types & layout engine:** struct/bitfield/enum **layout**, `sizeof`/`offsetof`/`@offset`
> assertions, `Data` values, and the type-system surface (`newtype`, `fixed<I,F>`, refinement
> `where`, comptime sum types + exhaustive `match`). All comptime-side + erasing.

Authority: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` **§4.1** (newtype/fixed/refinement — erasing),
**§4.3** (struct layout, no implicit padding, `(size:)`, `@offset`, `sizeof`/`offsetof`), **§4.4**
(bitfields MSB→LSB + `@N` anchors; enums closed; comptime sum types + `match`, D2.14), **§4.5**
(typed `data` emission — the CHECKING half), and the **D2.9–D2.14** decision records.

## ⚠️ Grammar gaps — Plan 3 must extend the frontend first (like lambdas/`|>` in Plan 2)

VERIFIED 2026-07-05 against the Plan-1 lexer/parser/AST: `struct`/`bitfield`/`enum` **decls already
parse** (they're `Item` variants with all fields), but the D2.9–D2.14 amendment surface does **NOT
parse** — there is no `Newtype` item, no `Match` node (expr or stmt), no `where` refinement clause on
types, no `fixed<I,F>` type, no `rescale`. `Type` is only `Named/Ptr/Array/Tuple`. So Plan 3, like
Plan 2 (which had to add lambdas + `|>`), begins with a **frontend grammar extension**:

- `newtype Name = Underlying [where LO..HI]` item → new `Item::Newtype` + AST.
- `fixed<I, F>` as a parameterized type constructor → extend `Type` (contextual `fixed` keyword,
  §10 reserved-word note).
- `T where LO..HI` refinement clause on a type → extend `Type` (contextual `where`).
- `match e { Pat => arm, ... }` expression + patterns (variant destructuring, incl. payload binds)
  → new `Expr::Match` + a `Pattern` AST. Exhaustiveness is a Plan-3 *semantic* check.
- `rescale<I,F>(x)` — a call-like form (may reuse `Call` or a dedicated node).
- `sizeof(T)` / `offsetof(T, field)` — parse as calls (callee `sizeof`/`offsetof`) taking a **type**
  argument; note `T` is a `Type`, not an `Expr` — decide how a type appears in argument position
  (a small parser affordance, like the §6.8 builtins were special-cased in the evaluator).

Do the frontend extension as its own reviewed sub-task(s) up front, then build the engine on it —
exactly the T6a-before-T6b shape that worked in Plan 2.

## Scope

**IN (Plan 3):**
- **Layout engine (new module, e.g. `src/layout.rs`):** struct declaration-order byte layout (NO
  implicit padding — §4.3), `(size: N)` verification with a field-by-field diff on mismatch,
  `@ offset` field asserts, `[layout.odd-field]` default-on warning for word/long at an odd offset,
  `sizeof(T)`/`offsetof(T, f)`. Bitfield layout MSB→LSB, width-sum == repr width, `@N` bit anchors.
- **Type table + checking (comptime side):** `newtype` (distinct type; same-type ops auto-inherit
  the underlying's arithmetic incl. wrapping; cross-type mix = error; explicit `Name(x)` reinterpret;
  `fix8_8`/`fix16_16` become aliases). `fixed<I,F>` scale-typed arithmetic (equal-scale add/sub
  transparent; scale mismatch incl. the multiply-doubled scale = error naming the required
  `rescale<I,F>`; no auto-rescale). Refinement `T where LO..HI` (constant bounds only, **no solver**;
  construct-out-of-range = comptime error; generalizes the bitfield field-range check).
- **Struct/enum/bitfield VALUES upgraded from Plan-2 placeholders:** struct literal now checked
  (field names/types vs decl, `(size:)`, defaults/missing-field errors — §4.5 "no silent zero-fill
  unless `= 0`"); bitfield construction range-checks every field (killing the unchecked `vram_art`
  bit-math class); enum casting closed (out-of-range int → explicit `unchecked` required).
- **Comptime sum types + `match` (D2.14):** payload-carrying `comptime enum` variants as tagged
  comptime values; `match` deconstructs and is **exhaustive** (missing variant = compile error);
  stdlib-style `Result`/`Option`; §6.8 `?` is sugar over `Result` (fold in here if cheap — it was
  deferred from Plan 2).
- **`Value::Data`** — a checked byte buffer as a comptime value + the `Data.empty`/`++` monoid
  (§6.8). Emission **range-checks** (i128 → sized primitive) live here.
- **`data NAME: T = expr`** — evaluate + **check** the initializer against `T` and produce the
  checked `Data` value / layout. (See the Plan 3/4 seam below for what's NOT here.)

**OUT (later plans):**
- **The actual IR streaming / byte emission into a section, `Code` values, `asm{}`, procs, hygiene,
  fixups, byte order — Plan 4** (`IrStreamer` via sigil-isa + Core IR). Plan 3 computes layouts and
  produces *checked* `Data` comptime values; it does not lower them to the linked ROM.
- **Runtime-value type checking** (newtype/fixed/refinement carried through register moves &
  instructions) — rides Core's **S2-D6/D7** dataflow pass, NOT Plan 3. Plan 3 checks **comptime**
  values directly (§4.1: "Checking on comptime values is direct; checking on runtime values … rides
  the … dataflow pass").
- `embed`/`import`/`zx0` + `as.*` float — **Plan 5**. `@as_compat` + mixed build + port diff —
  **Plan 6**. Cross-module `use`/prelude resolution — still deferred.

### ⚠️ Decision to settle EARLY — the Plan 3 / Plan 4 seam for `data` emission

§4.5 says `data` items "emit checked bytes into their section" with CPU byte order + pointer fixups.
The **layout + value checking** is unambiguously Plan 3; the **byte-order serialization + `Fixup`
kinds + section placement** is Plan 4 (needs the IR/backend). Recommended split: Plan 3's `data`
produces a fully-checked, typed `Value::Data` (a byte buffer with the field values validated and
laid out per the struct layout, in a CPU-neutral form) and asserts `(size:)`/ranges; Plan 4's
`IrStreamer` serializes that `Data` into the section with the right endianness + fixups. **Confirm
this boundary in the plan doc before starting** — it decides whether `Value::Data` carries raw bytes
or a structured field list.

## Carry-forward from Plan 2 (fold in where natural)

- **Split `eval.rs` (~2160 lines) into an `eval/` module tree** — the final Plan-2 whole-branch
  review recommended it as a post-checkpoint follow-up (seams sketched there:
  `eval/{env,expr,call,control,builtins,guards}.rs`). Plan 3 adds a lot of new code (layout,
  match, type checks) — do this split at the START of Plan 3 so the new code lands in focused
  modules, not by growing the 2160-line file further. (Pure refactor, no behavior change — verify
  258 tests still green.)
- `none` → `Unit` placeholder becomes real `Option` here.
- Array PARAM types must currently be sized `[T; N]` (`[T]` doesn't parse) and array length is not
  type-checked — Plan 3's type engine is where `[T; N]` length checking would land if wanted.

## Design decisions to make early (load-bearing)

1. **Type representation.** How does a resolved type live at comptime? A `Ty` enum (Prim(width,
   signed) / Ptr / Array(elem, len) / Named(struct/bitfield/enum/newtype) / Fixed(i,f) /
   Refined(inner, lo, hi) / Tuple). Built into a file-level type table alongside the const/fn table.
2. **Where checking happens.** A separate layout/type pass over the item table vs. lazily during
   evaluation (mirror the lazy+memoized `resolve_const` pattern from Plan 2 for `sizeof`/layout).
3. **`newtype` erasing arithmetic.** Same-type ops inherit the underlying's behavior *including
   wrapping* (`Angle + Angle` wraps as `u8`) — this is the first place comptime arithmetic is
   **sized/wrapping** rather than the unbounded-`i128`-overflow-is-error rule (D-P2.1). Decide how
   sized wrapping arithmetic coexists with comptime `int`.
4. **`match` exhaustiveness + binding.** Pattern AST, variant/payload binding into a scope, the
   exhaustiveness algorithm (closed enums make this finite — no solver).
5. **`Value::Data` shape** (see the Plan 3/4 seam decision above).
6. **Refinement/bitfield range-check** is ONE mechanism (§4.4: "each field's width **is** a
   refinement") — implement once, reuse for bitfield fields, `where` clauses, and enum casts.

## Suggested task shape (turn into a real plan doc)

Rough, TDD, commit-per-task, mirroring Plan 2's cadence:
- **T0** — refactor `eval.rs` → `eval/` module tree (carry-forward; behavior-neutral).
- **T1** — frontend grammar: `newtype`, `where`, `fixed<I,F>`, `match` + patterns, `sizeof`/
  `offsetof`/`rescale` arg forms (AST + parser + tests). *(Reviewed like T6a.)*
- **T2** — type table + `Ty` model + type resolution/checking scaffold.
- **T3** — struct layout + `(size:)`/`@offset`/`sizeof`/`offsetof` + odd-field warning. *(load-bearing)*
- **T4** — bitfield layout + field range-checks; the shared refinement mechanism + `where` + newtype
  refinements + enum-cast checks. *(load-bearing)*
- **T5** — newtype distinct-type + same-type/cross-type arithmetic; `fixed<I,F>` scale checking +
  `rescale`. *(load-bearing)*
- **T6** — comptime sum types + exhaustive `match` (+ `Result`/`Option`, `?`). *(load-bearing)*
- **T7** — `Value::Data` + monoid + emission range-checks + checked `data`/struct-literal values.
- **T8** — corpus (Appendix E worked exhibit + the struct/bitfield exhibits from A–D) + final
  whole-branch review.

## Process to keep (it worked in Plan 2)

- Subagent-driven with **two-stage reviews** (spec compliance THEN code-quality via
  `superpowers:code-reviewer`) on the load-bearing tasks, TDD per task, commit after each, green
  gate (`cargo test -p sigil-frontend-emp` + `cargo clippy --workspace --all-targets -- -D warnings`)
  before every commit. **Add a whole-branch review at the end** — in Plan 2 it caught a CRITICAL
  cross-feature bug (a lambda `return` leaking through `map`) that the six isolated reviews missed.
- Ground semantics in the SPEC, not intuition. Where AS/asl is the reference (integer widths,
  struct/endstruct offset identity §8.3, bitfield packing), cross-check. Record every design call in
  the plan doc's decision list (continue the `D-P3.x` numbering).
- Keep `sigil-frontend-emp` depending on `sigil-span` ONLY.
- Milestone boundary: Plan 3 is a milestone — do NOT merge to master without a Volence checkpoint,
  same as Plan 2.

## Acceptance sketch

- A `.emp` program using structs (with `(size:)`/`@offset`), bitfields (range-checked construction),
  enums + `match` (exhaustive), a `newtype`/`fixed`/`where` type, and `sizeof`/`offsetof` evaluates
  and **checks** correctly; layout offsets match AS `struct/endstruct` (§8.3 byte-identity argument).
- Out-of-range bitfield/refinement construction, a `(size:)` mismatch, a scale mismatch, and a
  non-exhaustive `match` each produce a **named, spanned** diagnostic with interpolated values (§9).
- `cargo test -p sigil-frontend-emp` green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean; crate still `sigil-span`-only.
</content>
</invoke>
