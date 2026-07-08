# Handoff — Spec 2 · Plan 7 backlog #5 COMPLETE: item-level guards + data capacity

Written 2026-07-07 (Opus, implementer). Branch `plan7-item5-ensure-capacity` (worktree
`sigil/.worktrees/plan7-item5-ensure-capacity`), branched from master `ba3fb98`. **NOT merged** —
stops here for the Volence/Fable checkpoint (established cadence). Design doc:
`docs/superpowers/specs/2026-07-07-spec2-plan7-item5-ensure-capacity-design.md` (D5.1–D5.6).

## What shipped (7 commits on top of `ba3fb98`)

| # | Commit | What |
|---|--------|------|
| 0.5 | `faf5191` | `fix(resolve): recurse into section items in def/export collectors` |
| 0.6 | `c5228e5` | `fix(resolve): module-qualify dotted exported labels — Owner.name defs, refs, and fixup targets` |
| 1 | `78aef68` | `feat(emp-parse): item-position ensure/ensure_fatal guards + OPENERS recovery fix` |
| 2 | `f2e287b` | `feat(emp-lower): evaluate item-position guards in order — here()-aware, fatal stops module` |
| 3 | `6e0d78d` | `feat(emp): (max_size:) capacity attribute on data items — always-on over-by-N error` |
| 4 | `41b283c` | `test(emp): item-guard corpus — multi-module prelude guard, byte-neutrality, aeon-shaped ports; guards example` |
| — | (plan checkbox commit + review-fix commit, see below) | |

### Feature surface (D5.1–D5.4, D5.6)
- **Item-position guards.** `ensure(cond, "msg")` / `ensure_fatal(cond, "msg")` are now legal at the
  top level and inside `section {}` bodies. Contextual opener (§10 policy): the keyword is a guard
  ONLY when immediately followed by `(`; `ensure` stays usable as an ordinary name everywhere else
  (`data ensure: … = …` parses as a normal data item — pinned by `ensure_usable_as_ordinary_data_name`).
  `pub` on a guard is an error. No `comptime block {}` item wrapper was added (D5.1, no-ceremony-tax).
  New AST node `Item::Ensure(EnsureDecl { fatal, call, span })` stores the WHOLE call expression, so
  lowering reuses the evaluator's existing `ensure`/`ensure_fatal` special-case (arity, interpolation,
  `aborted`) with zero new eval logic.
- **Lowering-time, in-order, position-aware (D5.2).** Guards evaluate during lowering exactly where
  they sit, with `here_base = placement.origin + builder.current_offset()` (top level: default section
  VMA==LMA==`next_lma`; section-nested: the section's VMA base) — identical threading to data items.
  `here()` is valid in the condition AND in `{here()}` message interpolation (verified failing-path
  renders the call). Guards emit ZERO bytes: byte-neutrality proven both at the lowered-module level
  (`passing_guards_are_byte_neutral`, span-free fingerprint) and end-to-end
  (`guards_are_byte_neutral_end_to_end`, flat linked image).
- **Failure semantics (D5.3).** `ensure` failure = error diagnostic (interpolated), lowering continues
  so every guard reports. `ensure_fatal` failure = error diagnostic, then STOP lowering the module's
  remaining items — including from inside a section block (`lower_section_items` now returns `bool`; a
  `false` propagates a `break` to the top-level loop). Already-emitted bytes/labels are left intact.
- **`(max_size: expr)` capacity (D5.4).** `data Name (max_size: E) [: Ty] = value` (attribute BEFORE the
  `: Ty` annotation, mirroring `struct Name (size: E)`). `E` must comptime-evaluate to an int `>= 0`;
  checked against the item's checked-buffer byte length (`DataBuf.size`). Overflow is an error phrased
  exactly per §7.3: `` data `Name` is M bytes — exceeds max_size N (over by K bytes) ``. Always-on.
  Enforced in `check_max_size` on the `eval_data_with_root` **and** `eval_data_captures` paths, so both
  top-level and section-nested data items are covered by one check.
- **D5.6 drive-by.** `recover_to_next_decl`'s `OPENERS` gained `"offsets"` (pre-existing recovery gap),
  `"ensure"`, `"ensure_fatal"`. NOTE (small necessary addition beyond the plan's literal OPENERS edit):
  because `ensure`/`ensure_fatal` are contextual, recovery would spin on a BARE `ensure` (item() won't
  consume it) — so recovery skips a non-`(` occurrence of these two keywords rather than stopping on it.
  Covered by `ensure_ident_not_followed_by_paren_still_errors_as_declaration` (which hangs without the
  fix).

### Audit pre-tasks (unblocked #5's multi-module corpus)
- **0.5 — section-nested resolver.** `resolve/imports.rs` `exported_names`/`defined_names` iterated only
  top-level `file.items`; section-nested `data`/`proc`/`offsets` never entered the rename map, so
  `--root` builds rejected references to them (single-file worked). Fix: both collectors now recurse
  into `Item::Section`. Byte-checks: section-nested offsets/data → `00 04 00 05 AA BB`; section-nested
  proc cross-ref → `4E F8 00 04 4E 75`; cross-module section-nested offsets target → `00 04 00 05` +
  `AA`/`BB` (identical to single-file). The §4.7 cross-module-target deferral is now discharged
  top-level AND section-nested.
- **0.6 — dotted exported labels.** `export .entry:` emits `Owner.name` (`foo.entry`), which was neither
  a `$`-hygiene local nor a rename-map key, so `report_unresolved` rejected every `--root` reference to
  it. Fix: new `rename::canonicalize_name` splits a dotted name at the FIRST dot; if the owner segment
  is in the rename map, it module-qualifies to `<renamed-owner>.name` on label defs, `Expr::Sym` refs,
  and fixup targets. `report_unresolved` now accepts a name iff `canonicalize_name` resolves it. This
  (a) fixes the false reject, (b) module-qualifies the owner so two modules' private
  `proc foo { export .entry }` no longer collide in the flat link table (latent finding #2), and
  (c) makes cross-module `use a.foo` + `jmp foo.entry` resolve. Byte-checks: audit repro → `60 00 FF FE`;
  two-module collision → links; cross-module dotted ref → `jmp` relaxes to `4E F8 00 08` (a.foo.entry).

## Decisions taken (implementer, within Fable's design)
- **max_size lookup, not signature change.** Rather than thread `max_size` through
  `eval_data_with_root`'s 5 callers (2 src + 3 test), `check_max_size` looks the decl up in `ev.datas`
  (already indexed, recurses sections) — smallest diff, covers all paths. Mirrored into the twin
  `eval_data_captures` for consistency (test-only today, but kept in lockstep to prevent future drift).
- **Byte-neutrality test compares a span-free fingerprint** (section name/cpu/vma/lma + labels + LINKED
  bytes), because two source texts legitimately differ in span offsets while producing identical output.
- **Test placement drift from the plan.** The plan named `ports.rs` for the multi-module prelude-guard
  test, but `ports.rs` is in-process single-file (no `--root` driver). Put the multi-module test in
  `module_resolution.rs` (the CLI `--root`/`--prelude` driver, the item-4 tests' home); kept
  byte-neutrality + aeon-shaped single-file tests in `ports.rs` via its `emp_candidate` helper.
- **max_size error span** anchors at `file.module.span` (the evaluator's available span for a resolved
  data item), same as the existing `no data item named` diagnostic. A per-item span would need threading
  the decl span into the eval path — deferred, not load-bearing.

## Test state (IMPORTANT — briefing undercounted the known failures)
`cargo test --workspace --no-fail-fast`: **1085 passed, 4 failed.** All 4 failures are in
`sigil-harness`, on the AS full-ROM reproduction path, ALL with the identical root cause
`strlen(): could not evaluate string builtin` — the sibling aeon repo's sound driver was refactored
upstream 2026-07-07 (NOT this branch's code; the diff touches no `sigil-frontend-as` or harness code):
- `full_build_reproduces_sound_driver_regions` (m0_regions) — the one the briefing named
- `vector_table_matches_reference_rom_first_256_bytes` (m1c_vector_table)
- `full_debug_rom_matches_assembled_reference` (m1d_debug_rom)
- `full_rom_matches_assembled_reference` (m1d)

**Verified: the identical 4 fail on clean master** (`cargo test --workspace --no-fail-fast` → 1056
passed, 4 failed, same names). My branch adds 29 passing tests (1056→1085, incl. review-driven additions) and introduces ZERO new
failures. (The briefing's "one known failure" reflected a plain `cargo test` run, which fail-fasts at
the first failing test binary and never runs the other 3 harness binaries — use `--no-fail-fast` to see
all 4.) `cargo clippy --workspace --all-targets -- -D warnings` is clean.

## Reviews (Task 5, two-stage)
- **Stage 1 — spec-compliance (superpowers:code-reviewer):** verdict **CLEAN.** All of D5.1–D5.6
  satisfied; grep confirms NOTHING from D5.5 was built (`ensure_warn`/warn tier, `sizeof(data-item)`,
  link-time label asserts, `fits_within` — hits only in the copied design markdown, never in source).
  Audit fixes do not overreach. Sole recommendation: commit a `data ensure` non-regression test — done
  (`ensure_usable_as_ordinary_data_name`).
- **Stage 2 — code-quality (superpowers:code-reviewer):** **no MUST-FIX.** Findings and dispositions:
  - SHOULD-FIX: `check_max_size` matched `Value::Int` directly, rejecting `Typed`-wrapped (newtype)
    bounds contra §8.3's "Typed erases to its stored int". **FIXED** — now `as_stored_int()` with the
    `Poison` early-return retained (D-P2.9); TDD'd via `max_size_accepts_newtype_wrapped_int`.
  - NIT `EnsureDecl.fatal` unread: kept (the plan's EXACT AST shape mandates it) and made load-bearing —
    a `debug_assert!` in `eval_item_guard` checks the keyword agrees with the stored call's callee.
  - NIT dotted-owner acceptance defers unknown-local detection to link time (`foo.typo` → link-time
    undefined symbol, not a resolve-time error): intentional; comment added stating acceptance
    guarantees rewritability, not existence.
  - NIT `split_once('.')` single-segment-owner invariant: comment added on `canonicalize_name`.
  - NIT `max_size` Expr clone per evaluation: left as-is per the reviewer's own "not worth churning"
    (only items declaring a bound pay it, and it sidesteps an `ev` borrow conflict).
- **Adversarial probes (all pass):** guard as FIRST item (`here()==0` in default section); guard as LAST
  item; `ensure_fatal` in a section stopping later top-level items; message interpolation failure
  (unknown name in `{}` → best-effort `<?>` + diagnostic, no crash); `data ensure` contextual-opener
  non-regression; `max_size` + guard on adjacent items; `ensure_fatal` as the only item; failing-guard
  call interpolation (`{here()}` renders `2`).

## Spec delta (for Fable to lift into `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` post-merge)
- **§6.5** gains: "Guards are also legal at item position (top level and inside `section {}`), evaluated
  in item order against the current position (`here()` valid); `ensure` reports and continues,
  `ensure_fatal` stops the module's remaining items." Contextual opener (fires only on `ensure`/
  `ensure_fatal` immediately followed by `(`); `pub` on a guard is an error.
- **§4.3-adjacent data syntax** gains `data Name (max_size: expr) [: Ty] = value`: `expr` a comptime
  int `>= 0`, checked against the checked-buffer byte length, overflow phrased with §7.3's
  "over by N bytes". Always-on (inherent, not a remembered guard).
- **D2.20 ledger row:** `ensure_warn` + item-level `comptime block {}` deferred (D5.5).
- **Note the resolver semantics now generalized by 0.5/0.6** (cross-module refs to section-nested pub
  items resolve; exported `Owner.local` labels are module-qualified to `Module.Owner.local` in the flat
  link table). These are §3.2 / §5.2 clarifications worth a spec line.

## What #6 walks in with
- Item-position guards + `(max_size:)` are the closing pieces of research T2-b — the ~195 aeon
  `error`/`fatal` guards and the S2 buffer-fit checks now have a direct port target
  (`examples/guards.emp` is the worked exhibit).
- `#6 state-machine + SST overlay` is next. D5.5 explicitly parks `fits_within(buffer)` (relating two
  items) for #6's RAM-buffer/overlay story — that is the natural capacity follow-on once overlays exist.
- The resolver is now section-nesting-correct and dotted-label-correct under `--root`; #6's overlay
  labels inherit both fixes for free.
- `canonicalize_name` (dotted-owner module-qualification) is the seam any future dotted-symbol class
  (e.g. `Struct.field` link symbols, if they ever become link-visible) should route through.
