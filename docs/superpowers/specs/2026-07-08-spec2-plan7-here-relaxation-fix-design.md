# Design — Plan 7: the `here()`-vs-relaxation fix (NOTE-1, RATIFIED)

Date: 2026-07-08 (Fable, morning session). Volence ratified the FIX at the overnight
checkpoint (fix the footgun, do not document the constraint — the principle is now in
[[emp-language-design-principles]]). Background: item-8 design doc's ledger NOTE-1.
This doc makes the agreed sketch precise; opus implements on a worktree branch.

## The defect, restated on verified code

`here()` resolves eagerly to `origin + builder.current_offset()` (lower/mod.rs:197, :337,
:378) — a mid-lowering cursor in which every relaxable fragment (`JmpJsrSym`,
`RelaxAbsSym`, `RelaxLadder`) is counted at its BASELINE rung (jmp/jsr: 4 bytes; jbra: 2).
When `resolve_layout` later grows a rung, labels shift at link (relax.rs
`shift_breakpoints`/`shift_offset`) but any already-materialized `here()` **integer** does
not: an emitted value is stale by Σ(growth before it), and an
`ensure(here() <= …)`/`ensure_fatal` budget guard silently checks the smaller number
(guards.emp:58 and ports.rs:478 are live consumers of the idiom).

## Verified code facts this design stands on (T0, 2026-07-08)

- Green gate on master d95c94b: exactly the 4 allowlisted sigil-harness reds; clippy
  clean. Working tree clean.
- Labels are (name, baseline offset) per section; `resolve_layout` shifts them and
  `link()` resolves `origin + offset` (relax.rs:369–377, link lib.rs:57–73). **So a label
  is always relaxation-correct within its section** — the fix's core invariant is
  therefore: *`here()` must be exactly as accurate as a label defined at the same
  position.*
- Section `lma`/`vma_base` are FIXED inputs baked by the frontend at baseline prefix sums
  (`next_lma += builder.current_offset()`, lower/mod.rs:208). Growth in an earlier
  section does NOT move a later section's origin — that staleness (image overlap via
  `flatten`, stale label VMAs in later default sections) affects **labels identically**
  and is a PRE-EXISTING, SEPARATE hazard → ledger entry L-H.1 below, not this fix. The
  label-equivalence invariant makes `here()` no worse than labels under it.
- IR `Expr` (sigil-ir expr.rs) already carries full comparison/logical/arith ops folding
  to 0/1 via a symbol-lookup closure — a link-time assertion needs NO new expression
  vocabulary. `Expr::Int` is i64.
- `Cell::SymRef { name, width, windowed }` already emits an address-of-symbol data cell;
  D-P4.5 kind selection reads (width, section CPU, windowed).
- `ensure`/`ensure_fatal` are non-shadowable special calls (eval/call.rs:48–49) reachable
  from BOTH item position (`EnsureDecl` → `eval_item_guard`) and any expression context
  (a guard inside a data item's expression); the message is evaluated/interpolated only
  on failure (eval/guards.rs).
- The evaluator runs on a separate stack with no builder access; it communicates with the
  lowering pass by returned values + collected `diags` (guards.rs:32–41). Deferred
  asserts ride the same channel (a new collected list), not a builder handle.
- `Value` is an exhaustive Rust enum — adding a variant forces a compiler-driven audit of
  every match site. This is the safety net that makes the lift sound.
- Z80 sections contain no relaxable fragments today (the `jr → jp` ladder is deferred),
  so a Z80-section position is always exact; the provisional machinery is 68k-only in
  practice until that ladder lands (cross-ref L-H.4).

## Decisions

- **D-H.1 (when `here()` is symbolic — the exact/provisional split).** The lowering pass
  classifies the position it hands the evaluator: **Exact** if the currently-open section
  record contains no relaxable fragment (`JmpJsrSym` | `RelaxAbsSym` | `RelaxLadder`) at
  an earlier offset, else **Provisional**. Exact → `eval_here` returns `Value::Int`
  exactly as today (byte-identical path; guards.emp and every existing program with no
  relaxable-before-`here()` are untouched). Provisional → it returns a **link-time
  value** (D-H.2) anchored to an anonymous label at the position. Same-section tracking
  suffices: origin staleness from earlier sections hits labels and `here()` equally
  (label-equivalence invariant), and is L-H.1's problem. `IrBuilder` grows a
  "current section has a relaxable so far" query to answer this.
- **D-H.2 (`Value::LinkExpr` — partial evaluation, not a new sublanguage).** New variant
  `Value::LinkExpr(sigil_ir::expr::Expr)`: an integer known only after `resolve_layout`.
  A provisional `here()` yields `LinkExpr(Sym(<anchor label>))`. The comptime operators
  that IR `Expr` can represent — `+ - * / % << >> & | ^ == != < > <= >= && ||`, unary
  neg/not — LIFT: any operand mix of `Int`/`LinkExpr` where at least one side is
  `LinkExpr` builds the residual `Expr` tree instead of folding (an `Int` lifts via
  `Expr::Int`; range-check i128 → i64 on lift, error on overflow). Everything else that
  touches a `LinkExpr` — array length, `rept` count, `if`/`while` condition, index,
  slice bound, `.map`/builtins, `max_size`, string interpolation outside guard messages,
  charmap, float ops — is the loud error **`[here.provisional]`**: *"`here()` after a
  size-relaxable instruction (jbra/jbsr, an unsized branch, or a bare jmp/jsr) is a
  link-time value; it cannot size or steer comptime evaluation — pin branch sizes
  (bra.s/bra.w, jmp) before this point, or restructure so the value is only emitted or
  guarded"*. The §7.1 `rept $38 - here()` gap-fill idiom therefore still works wherever
  it works today (exact positions) and refuses loudly where it was silently wrong.
- **D-H.3 (data-emitted `here()` — the SymRef lowering).** A **plain** provisional
  `here()` value landing in a data cell becomes `Cell::SymRef { name: <anchor>, width:
  <declared cell width>, windowed: false }` — an ordinary symbol-address emission through
  the existing D-P4.5 fixup selection (width 4 → `Abs32Be`, width 2 → `Abs16Be`; width 1
  → error, no 8-bit absolute kind exists). Inside a data item the anchor is **the item's
  own label** (`here()` names the item's start per §7.1, and `lower_data_item` defines
  `decl.name` at exactly that byte) — no anonymous label needed on this path. A
  provisional `here()` that has been ARITHMETICALLY combined (`LinkExpr` non-`Sym` tree)
  and then emitted is `[here.provisional]` for now — the general link-expr data cell is
  deferred (L-H.2) until a real consumer shows up; the guard path (D-H.4) is where
  arithmetic matters and is fully supported.
- **D-H.4 (deferred guards — link-time assertions).** `eval_guard` on a condition that
  evaluates to `LinkExpr` (instead of `Bool`) DEFERS: it records a pending
  `LinkAssert { cond: Expr, message: Vec<MsgPart>, fatal: bool, span: Span }` on the
  evaluator (drained by the lowering pass into the module, like diags) and returns
  `Value::Unit` — the guard is neither passed nor failed yet. Works uniformly for
  item-position guards and guards inside data-item expressions. `Module` (sigil-ir)
  gains `link_asserts: Vec<LinkAssert>`; multi-module `build_program` concatenates.
- **D-H.5 (guard messages — parts, interpolated eagerly, folded lazily).** The deferred
  guard's message is evaluated and `{expr}`-interpolated at DEFER time (comptime env is
  about to disappear), into `MsgPart::Text(String)` runs — EXCEPT a placeholder whose
  value is itself `LinkExpr`, which becomes `MsgPart::Expr(ir::Expr)` and is folded and
  rendered at link on failure. So `ensure_fatal(here() <= $9000, "overran: at {here()}")`
  reports the REAL final address. A non-string message / arity errors stay comptime
  errors exactly as today (they don't defer).
- **D-H.6 (link-time evaluation seam).** A new sigil-link entry point evaluates the
  program's `LinkAssert`s against the **post-`resolve_layout` symbol table** (the same
  labels `link()` resolves — implementer's choice whether `link()` exports its table or
  the checker rebuilds it; the contract is: identical values). Runs after a successful
  `link()` in every emp link tail (`link_sections` and the `--map`/`emit_rom` seam,
  sigil-cli main.rs:197–209, :435). ALL failing asserts are collected and reported (not
  first-failure). Condition folds to 0 → failure; nonzero → pass; `Fold::Poison`
  (unresolved sym in the cond) → an internal-contract error naming the assert's span
  (cannot happen if the anchor was defined — test it anyway).
- **D-H.7 (deferred-fatal semantics — SPEC NOTE, ratified).** A deferred `ensure_fatal`
  **cannot stop lowering early** — lowering already finished by the time it is evaluated.
  Deferred "fatal" = fail-the-build-at-link. At link, `ensure` and `ensure_fatal` are
  therefore identical in effect (an Error diagnostic fails the build); the `fatal` flag
  is retained on `LinkAssert` for diagnostic wording/spec fidelity. The D5.3
  stop-remaining-items behavior remains comptime-only (exact-position guards keep it —
  they never defer). This note must be lifted into the empyrean spec (task 2 of the
  handoff, D-row alongside NOTE-1).
- **D-H.8 (anonymous anchor labels).** Item-position guards have no natural label; when
  the position is provisional AND the guard actually evaluated `here()`, the lowering
  pass defines an anonymous label at the guard's cursor: reserved-namespace name,
  **program-unique** (module-qualified + countered, e.g. `__here$<module>$<n>` — `link()`
  has whole-program duplicate-label detection, so two modules must never mint the same
  name; the exact scheme is the implementer's, the uniqueness + unparsable-by-both-
  frontends contract is fixed). Define the label only when used (the evaluator reports
  use), so `--map` output and symbol tables aren't polluted by every guard.
- **D-H.9 (what does NOT change).** Exact-position behavior is byte-identical (same code
  path, `Value::Int`). `here()` outside lowering stays an error. Offsets/dispatch items
  (no `here_base`) unchanged. Proc/instruction-operand contexts don't thread `here_base`
  today and continue not to. `const` items unchanged. Z80: always exact today (verified
  fact above). `(max_size:)` capacity checks are length-based and `SymRef` cells have
  fixed width — unaffected.

## Acceptance (what "done" means)

- **Unit/integration, RED-first:** (1) a REPRO test that FAILS on master's semantics —
  jbra to a far target before `ensure_fatal(here() <= N, …)` where the guard passes
  stale (baseline) but must fail final (growth pushes past N) → with the fix, the build
  FAILS at link with the guard's message; (2) the mirror positive case (budget still met
  after growth → passes, zero diagnostics); (3) data-emitted `here()` after a jbra →
  emitted address equals the FINAL shifted VMA (byte-level assert), via the item-label
  SymRef; (4) `{here()}` in a deferred message renders the final address; (5)
  `[here.provisional]` on: rept count, array length, if condition, width-1 emission,
  arithmetic-then-emit; (6) exact-position guards + gap-fill rept byte-identical to
  master (guards.emp compile + existing ports.rs §guards pin stay green untouched); (7)
  two modules each deferring a guard → no duplicate-label collision; (8) `ensure` vs
  `ensure_fatal` both fail the build at link (D-H.7), and a deferred fatal does NOT
  suppress later items' lowering.
- **Byte-diff probe at review:** master-vs-branch images for a corpus sweep (all
  examples/ + examples/game with the standing pitcher_plant invocation) — zero byte
  diffs anywhere (no program in the corpus has a provisional `here()`); pitcher_plant
  340-byte pin green.
- Green gate per commit: workspace tests with ONLY the 4 allowlisted reds; clippy
  `-D warnings` clean. No `cargo fmt` sweeps.

## Deferred (ledger)

- **L-H.1 (PRE-EXISTING, now recorded): cross-section origin staleness under growth.**
  Section `lma`/`vma_base` are baseline-baked; growth in an earlier section can (a)
  overlap images silently in the no-map `flatten` path (`flatten_checked` guards only
  the `--map` path) and (b) leave a later VMA==LMA default section's labels at stale
  VMAs. Affects labels and `here()` equally (invariant). Needs its own design: either
  link-time section re-placement or a loud refusal. Candidate for #7 (bank/window
  placement) since that work re-opens section placement anyway — raise with Volence
  there.
- **L-H.2: general link-expr data cells** (emit `LinkExpr` arithmetic, not just plain
  `here()`), e.g. `word(Table_End - here())` under a provisional position. Needs a
  `Cell`/fixup shape carrying a full `Expr` + width; demonstrated need only.
- **L-H.3: link-time re-interpolation of arbitrary env values** in deferred messages
  (D-H.5 freezes comptime parts eagerly — fine for every real idiom seen).
- **L-H.4: Z80 `jr → jp` ladder** makes Z80 positions provisional — D-H.1's tracking is
  CPU-agnostic already; re-audit D-H.3's width/kind table for Z80 (BankPtr16Le vs
  absolute) when that lands.
- **L-H.5: `here()` in instruction-operand position** (procs) — never supported; if a
  demand appears, it lands on the same anchor-label mechanism.
- **L-H.6 (from the code-quality review, quality-only):** `interpolate` /
  `interpolate_parts` in eval/guards.rs are near-identical brace lexers (eager vs
  deferred rendering) — parity verified on all edges today; extract the shared scanner
  when either is next touched. Also: `eval_data_captures` deliberately drops deferred
  `LinkAsserts` (test-only caller, always exact-position) — must drain them like
  `eval_data_with_root` if it ever gains a lowering-path caller (contract comment in
  place).
