# Design — Plan 7 backlog #5: item-level guards + data capacity (`ensure` / `max_size`)

Fable, 2026-07-07. Grounded in code by live probes (not memory) the same day.
Backlog title was "`assert!` / capacity refinements" (research T2-b); the frozen spec's names are
**`ensure`/`ensure_fatal`** (§6.5) and **budgets** (§7.3). This item is a *gap-closure*, not a new
subsystem — most of T2-b already shipped in Plans 2–4 and item #4.

## Already shipped — do NOT rebuild (verified 2026-07-07)

| Piece | Where | Status |
|---|---|---|
| `ensure(cond, "msg {interp}")` / `ensure_fatal` in `comptime fn` bodies + comptime blocks in procs | `eval/guards.rs`, `eval/call.rs:48-49`; tests `eval_guards.rs` | DONE (Plan 2) — arity/type errors, interpolation incl. escapes, fatal sets `self.aborted` |
| `sizeof(T)` / `offsetof(T, field)` | `eval/expr.rs:85-118`; tests `eval_layout.rs` | DONE (Plan 3) |
| Struct `(size: N)` + `@ 0xNN` field-offset assertions | `layout.rs`; tests `eval_layout.rs` | DONE (Plan 3) |
| Region budgets, linker "over by N bytes" | `sigil-ir/src/map.rs:50-53` | DONE (item #4, §7.3) |
| AS `error`/`fatal` directives (ported guards stay byte-exact in `.asm` until per-file port) | `sigil-frontend-as/src/eval.rs:1688-1692` | DONE (M1C/D) |

## The actual gaps (probed live)

1. **No item-level guard position.** `ensure(...)` at module scope → `expected a declaration,
   found Ident("ensure")`. `comptime block { ... }` is statement-level only (`parser.rs:1121`).
   Aeon's ~195 guards (147 `error` + 48 `fatal`) are TOP-LEVEL, between items — the entire port
   surface for them is missing.
2. **No always-on capacity check on a data item.** The S2 "17 buffer-fit checks" class: a blob of
   *computed* size (compression output, folded table) that must fit a fixed buffer. Region budgets
   cover *sections*; nothing covers one item.

## Decisions (Fable, autonomous per established cadence)

### D5.1 — Item-level guards: bare `ensure(...)` / `ensure_fatal(...)` as items
Legal at top level and inside `section { }` blocks. Contextual item opener per the §10
reserved-word policy (ident `ensure`/`ensure_fatal` followed by `(` at item position) — `ensure`
stays usable as an ordinary name everywhere else. `pub` on a guard is an error (mirrors
`use`/`section`, `parser.rs:194-197`). No `comptime block { }` item wrapper — bare form only
(no-ceremony-tax principle); item-level comptime blocks stay out until demanded.

### D5.2 — Evaluation: lowering-time, in item order, position-aware
Each guard item is evaluated during lowering exactly where it sits, with `here_base =
placement.origin + builder.current_offset()` (the same threading as data items,
`lower/mod.rs:258`). Consequences:
- `here()` is legal in a guard ⇒ the AS `if * > limit / fatal` class ports directly.
- Guards see consts, enums, `offsets` ordinals + `.count`, comptime fns, and (multi-module)
  prelude/`use`-resolved ambient names — free via the existing evaluator.
- Guards emit ZERO bytes. A program with guards is byte-identical to one without (tested).
- Conditions needing *final link-time* addresses of labels are out of scope — that class belongs
  to region budgets (shipped) and `no_straddle` (item #7). A guard referencing an unresolvable
  name diagnoses through the normal unknown-name path.

### D5.3 — Failure semantics: `ensure` continues, `ensure_fatal` stops the module
`ensure` fail = error diagnostic (interpolated message), lowering continues so every guard
reports (matches fn-body semantics and AS `error`). `ensure_fatal` fail = error diagnostic, then
**stop lowering the remaining items of that module** (AS `fatal` parity; the evaluator's existing
`aborted` flag is the signal). Compile fails on any error either way.

### D5.4 — Capacity: `data Name (max_size: expr) [: T] = value`
Attribute position mirrors `struct Name (size: expr)`. `expr` must comptime-evaluate to an int
`>= 0`. Checked against the item's **checked-buffer byte length** after evaluation; overflow is an
error phrased like §7.3: `` data `Name` is M bytes — exceeds max_size N (over by K bytes) ``.
Always-on (inherent, not a remembered guard). Exact-size assertion is NOT added for data — the
declared type already IS the exact size (totality); `max_size` exists precisely for
computed/inferred sizes (`zx0(embed(...))`, folded tables).

### D5.5 — Explicitly out of scope (recorded so nobody rebuilds them into #5)
- **`ensure_warn`** — frozen spec has error+fatal only; a warn tier is additive later (ledger note).
- **`sizeof(data-item)`** — sizes are inherent in types; computed tables use the const-array idiom
  (`const XS = [...]` → guard on `XS.len` → `data T: ... = XS`). Revisit only on migration demand.
- **Link-time asserts over final label addresses** — region budgets own that class; bank/window
  (`no_straddle`) arrives with item #7.
- **`fits_within(buffer)` relating two items** — needs the RAM-buffer/overlay story (item #6).

### D5.6 — Drive-by fix folded in
`recover_to_next_decl`'s `OPENERS` (`parser.rs:239-241`) is missing `"offsets"` (pre-existing:
error recovery can swallow a following `offsets` block). Add `"offsets"`, `"ensure"`,
`"ensure_fatal"` together.

## Porting story (why this closes T2-b)
`if cond / error "m"` → `ensure(!cond, "m")`; `fatal` → `ensure_fatal`; `if * > X / fatal` →
`ensure_fatal(here() <= X, ...)`; buffer-fit `if (End-Start) > BUF / error` → `(max_size: BUF)` on
the item; `(End-Start)/n == COUNT` asserts are already inherent in typed arrays and
`offsets .count`. Final-pass gating (`MOMPASS`) has no equivalent and none is needed (§6.5).

## Spec delta (for Fable to lift into `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` post-merge)
§6.5 gains: "Guards are also legal at item position (top level and inside `section {}`), evaluated
in item order against the current position (`here()` valid); `ensure_fatal` stops the module's
remaining items." §4.3-adjacent data syntax gains `(max_size: expr)` with the §7.3 "over by N
bytes" phrasing. D2.20 ledger row: `ensure_warn` + item-level comptime blocks deferred.
