# Design — Sigil Spec 2 Plan 3 (types & layout engine)

**Status:** approved by Volence 2026-07-05. Branch `spec2-p3-types-layout` (off master `ff2c387`,
which is Plan 2 merged). This is the *design* doc; the detailed T-task plan is produced next via
`superpowers:writing-plans`.

**Authority:** `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §4.1 (erasing type constructors), §4.3 (struct
layout), §4.4 (bitfields + enums + comptime sum types), §4.5 (typed `data` emission), §8.3 (byte
identity), §9 (diagnostics), §10 (concept inventory + reserved words), decisions **D2.9–D2.14**, and
Appendices C & E. Orientation: `docs/superpowers/notes/2026-07-05-spec2-plan3-types-layout-handoff.md`.

## 1. Goal

Make the Plan-2 placeholders real: struct/bitfield/enum **layout**, the erasing type-system surface
(`newtype`, `fixed<I,F>`, `where` refinements), comptime **sum types + exhaustive `match`**, and a
checked **`Value::Data`**. Everything is comptime-side and **erasing** — annotating a byte-exact port
with these types changes zero ROM bytes (§8.3), so the diff harness stays green.

The crate `sigil-frontend-emp` keeps depending on **`sigil-span` only**. No IR/backend/Core
dependency enters in Plan 3 — the layout engine is pure front-end logic.

## 2. Scope

**IN:**
- **Layout engine** (new `src/layout.rs`): struct declaration-order byte layout (no implicit padding,
  §4.3), `(size: N)` verification with a field-by-field diff on mismatch, `@ offset` field asserts,
  `[layout.odd-field]` default-on warning (word/long at an odd offset), `sizeof(T)`/`offsetof(T, f)`.
  Bitfield layout MSB→LSB, width-sum == repr width, `@N` bit anchors.
- **Type table + checking (comptime side):** `newtype` (distinct type; same-type ops inherit the
  underlying's arithmetic incl. wrapping; cross-type mix = error; explicit `Name(x)` reinterpret;
  `fix8_8`/`fix16_16` become stdlib aliases). `fixed<I,F>` scale-typed arithmetic (equal-scale add/sub
  transparent; scale mismatch incl. the multiply-doubled scale = error naming `rescale<I,F>`; no
  auto-rescale). Refinement `T where LO..HI` (constant bounds only, **no solver**).
- **Struct/enum/bitfield values upgraded from Plan-2 placeholders:** struct literals checked (field
  names/types vs decl, `(size:)`, defaults/missing-field errors — §4.5 "no silent zero-fill unless
  `= 0`"); bitfield construction range-checks every field; enum casting closed (out-of-range int →
  explicit `unchecked`).
- **Comptime sum types + `match` (D2.14):** payload-carrying `comptime enum` variants as tagged
  comptime values; `match` deconstructs and is **exhaustive**; stdlib `Result`/`Option`; §6.8 `?` as
  sugar over `Result`.
- **`Value::Data`** — a checked, CPU-neutral structured buffer + the `Data.empty`/`++` monoid (§6.8).
  Emission range-checks (i128 → sized primitive) live here.
- **`data NAME: T = expr`** — evaluate + check the initializer against `T`, produce the checked `Data`.

**OUT (later plans):**
- Byte-order serialization, `Fixup` kinds, section placement, `Code` values, `asm{}`, procs, hygiene
  — **Plan 4** (`IrStreamer` via sigil-isa + Core IR). Plan 3 produces *checked* `Data`; it does not
  lower to the linked ROM.
- Runtime-value type checking (types carried through register moves & instructions) — rides Core's
  **S2-D6/D7** dataflow pass, not Plan 3 (§4.1). Plan 3 checks **comptime** values directly.
- `embed`/`import`/`zx0` + `as.*` float — Plan 5. `@as_compat` + mixed build + port diff — Plan 6.

## 3. The six load-bearing decisions

### D-P3.1 — Type representation: a resolved `Ty` enum + lazy file-level type table
A resolved comptime type lives as:
```
enum Ty {
    Prim { width: 1 | 2 | 4, signed: bool },   // u8 i8 u16 i16 u32 i32
    Ptr(Box<Ty>),
    Array(Box<Ty>, usize),                      // [T; N], N resolved from a comptime const
    Tuple(Vec<Ty>),
    Struct(String),    // name → type table
    Bitfield(String),
    Enum(String),
    Newtype(String),   // table holds underlying Ty + optional refinement bounds
    Fixed { i: u32, f: u32 },
    Refined { inner: Box<Ty>, lo: i128, hi: i128 },
}
```
A `types` index sits alongside the existing `consts`/`fns`/`enums` indices in `Evaluator`. §10 frames
`fixed<I,F>` and `T where LO..HI` as the two parameterized forms of `newtype`; both are `Ty` variants.

### D-P3.2 — Where checking happens: lazy + memoized, mirroring `resolve_const`
`resolve_type(name) -> Ty` and `layout_of(ty) -> Layout` compute on demand with memoization + cycle
detection. A struct containing itself **by value** is an infinite-size error (report the chain); **by
`*T`** is fine (pointer is fixed width). `sizeof`/`offsetof`/`(size:)` trigger layout-on-demand. This
reuses the Plan-2 lazy+memoized pattern rather than adding a separate eager pass.

### D-P3.3 — newtype / sized arithmetic: a `Value::Typed { ty, val }` wrapper
Bare `int` keeps the Plan-2 rule (unbounded i128, overflow = error, D-P2.1). A value acquires a
*sized* nominal type only via a `newtype` constructor `Name(x)`, `fixed<>`, a refinement, or a typed
annotation (`const`/`data`/param/struct-field/bitfield-field). Then arithmetic **wraps at the
underlying width** (`Angle + Angle` wraps as `u8`, D2.9). Rules:
- same nominal type → wrap at underlying width, result keeps the type;
- typed ⊕ bare-literal → coerce the literal into the type, keep type, wrap;
- **different nominal types → cross-type error** (D2.9);
- `fixed<I,F>` ± same scale → transparent; `×` → doubles the scale to `fixed<2I,2F>`; landing a
  doubled-scale value in a `fixed<I,F>` slot → **scale-mismatch error naming `rescale<I,F>`**
  (D2.10, Appendix E); no auto-rescale.

Erasure: `Typed` unwraps to raw bytes at emission — this is what makes the annotations byte-neutral
(§8.3). *Note:* Appendix E's register examples (`Angle` in `d0`, the `muls` sequence) are **runtime**
checks that ride S2-D6/D7 and are OUT of Plan 3; Plan 3 exercises the same rules on **comptime**
values (e.g. `const c: Angle = Angle(200) + Angle(100)` wraps to 44).

### D-P3.4 — `match`: closed-enum exhaustiveness, no solver
New `Pattern` AST: `Wildcard | Binding(name) | Variant { path, subpats }`. v1 matches over comptime
enums + `_`. Exhaustiveness: arms must cover every declared variant (or include `_`); a missing
variant is a compile error **naming the missing variants** (tenet 7). Closed enums make this finite —
no solver. Payload binds push into a fresh per-arm scope (reuses the Env scope mechanism).

### D-P3.5 — `Value::Data` shape: structured checked cells, CPU-neutral *(Plan 3/4 seam)*
Plan 3's `Data` is **not** a flat byte blob — byte order and pointer fixups are Plan 4's job and need
field structure. It is an ordered list of range-checked cells:
```
Value::Data(DataBuf)
struct DataBuf { cells: Vec<Cell>, size: usize }   // size in bytes, CPU-neutral
enum Cell {
    Scalar { value: i128, width: 1 | 2 | 4, signed: bool },  // already range-checked
    Bytes(Vec<u8>),                                          // width-1 runs: byte/bytes/++
    SymRef { name: String, width: 1 | 2 | 4 },               // pointer-typed field placeholder
}
```
Plan 3 validates ranges/offsets/`(size:)` and lays cells out in declaration order. **Plan 4's
`IrStreamer` commits endianness (68k BE / z80 LE) and resolves `SymRef` to the right `Fixup` kind.**
The `Data.empty`/`++` monoid concatenates cell lists (+ sizes). This keeps Plan 3 IR/backend-free.

### D-P3.6 — Range-check is ONE mechanism
A single `check_in_range(val, lo, hi, span, context) -> Result` backs: bitfield fields (width `W` ≡
refinement `0..2^W`, §4.4), `where LO..HI` clauses, newtype refinements, and enum casts. Implemented
once in the layout/type module, called from every site (§4.1: "the same mechanism, not a special
case"). Every failure is a named, spanned diagnostic with interpolated values (§9).

### D-P3.7 — Pointer width default *(recorded call)*
Pointer width for struct sizing = **4 bytes (68k `Abs32`)** in Plan 3. Section-CPU plumbing (z80 =
2-byte bank ptr) is Plan 4. `(size:)`/`@offset` asserts catch any mismatch loudly, so a wrong default
cannot pass silently. Revisited when sections carry CPU (Plan 4).

## 4. Grammar gaps Plan 3 must close first (T1)

Verified against the Plan-1 lexer/parser/AST (`Type` is only `Named/Ptr/Array/Tuple`; no `Match`, no
`Newtype` item, no `where`, no `fixed<I,F>`):
- `newtype Name = Underlying [where LO..HI]` → new `Item::Newtype`.
- `fixed<I, F>` parameterized type constructor → extend `Type` (`fixed` is a contextual type
  keyword, §10).
- `T where LO..HI` refinement clause → extend `Type` (`where` is contextual, §10).
- `match e { Pat => arm, ... }` expression + `Pattern` AST (variant destructuring incl. payload
  binds) → new `Expr::Match`. Exhaustiveness is a Plan-3 *semantic* check (D-P3.4).
- `comptime enum` payload-carrying variants (`comptime enum Token { Literal(string), Arg(...) }`) —
  extend `EnumDecl` variants to carry optional payload types.
- `rescale<I,F>(x)`, `sizeof(T)`, `offsetof(T, field)` — call-like forms whose first argument is a
  **type**, not an expr. Small parser affordance: special-case these callees to parse a `Type` in
  argument position (mirrors how §6.8 builtins were special-cased in the evaluator).

Do T1 as its own reviewed sub-task(s) up front, then build the engine on it — the T6a-before-T6b
shape that worked in Plan 2.

## 5. Task shape (TDD, commit-per-task)

- **T0** — refactor `eval.rs` (2237 lines) → `eval/` module tree (`eval/{env,expr,call,control,`
  `builtins,guards}.rs` per the Plan-2 final review). Behavior-neutral; 258 tests stay green.
- **T1** — frontend grammar (§4 above). AST + parser + tests. *(reviewed sub-task)*
- **T2** — `Ty` model + type table + lazy resolution/layout scaffold (D-P3.1, D-P3.2) + the shared
  `check_in_range` (D-P3.6).
- **T3** — struct layout + `(size:)`/`@offset`/`sizeof`/`offsetof` + odd-field warning. *(load-bearing)*
- **T4** — bitfield layout + field range-checks + `where`/newtype refinements + enum-cast checks.
  *(load-bearing)*
- **T5** — newtype distinct-type + arithmetic (`Value::Typed`, D-P3.3) + `fixed<I,F>` scale +
  `rescale`. *(load-bearing)*
- **T6** — comptime sum types + exhaustive `match` (D-P3.4) (+ `Result`/`Option`, `?`).
  *(load-bearing)*
- **T7** — `Value::Data` + monoid + emission range-checks + checked `data`/struct-literal values
  (D-P3.5).
- **T8** — corpus (Appendix E worked exhibit + the struct/bitfield exhibits from A–D) + final
  whole-branch review.

## 6. Process (kept from Plan 2 — it caught a CRITICAL bug)

- Subagent-driven with **two-stage reviews** (spec compliance THEN code-quality via
  `superpowers:code-reviewer`) on the load-bearing tasks; TDD per task; commit after each; green gate
  (`cargo test -p sigil-frontend-emp` + `cargo clippy --workspace --all-targets -- -D warnings`)
  before every commit.
- **Whole-branch review at the end** — in Plan 2 it caught a CRITICAL cross-feature bug the six
  isolated reviews missed.
- Ground semantics in the **spec**, not intuition. Where AS/asl is the reference (integer widths,
  struct/endstruct offset identity §8.3, bitfield packing), cross-check. Record every design call in
  the plan doc, continuing the `D-P3.x` numbering.
- Keep `sigil-frontend-emp` depending on `sigil-span` **only**.
- Milestone boundary: Plan 3 is a milestone — no merge to master without a Volence checkpoint.

## 7. Acceptance

- A `.emp` program using structs (`(size:)`/`@offset`), bitfields (range-checked construction), enums
  + `match` (exhaustive), a `newtype`/`fixed`/`where` type, and `sizeof`/`offsetof` evaluates and
  **checks** correctly; layout offsets match AS `struct/endstruct` (§8.3 byte-identity argument).
- Out-of-range bitfield/refinement construction, a `(size:)` mismatch, a scale mismatch, and a
  non-exhaustive `match` each produce a **named, spanned** diagnostic with interpolated values (§9).
- `cargo test -p sigil-frontend-emp` green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean; crate still `sigil-span`-only; every emitted-byte exhibit byte-neutral vs its untyped form.
