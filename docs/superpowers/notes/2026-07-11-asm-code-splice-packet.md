# Packet — `asm{}` Code-splice (`{expr}` at statement position)

Branch: sigil `asm-code-splice` (off master `8ac1743`), aeon worktree
`.worktrees/asm-code-splice` (off master `726dbdc`). NOT merged — Volence's
gate. Spec: `specs/2026-07-11-asm-code-splice-design.md` (ratified). Sequenced
post-t11-merge (t11 landed on both masters first).

**Status: complete + verified.** Full workspace strict **2148/0** (the +8
asm_splice tests), clippy clean, both retrofits byte-neutral.

## What shipped

**The feature (C1-batch class, byte-neutral, TDD).** `AsmStmt::Splice(Expr)` —
a `{expr}` at STATEMENT position inside an `asm{}` block evaluates `expr` in the
enclosing comptime scope; it must yield `Code`, whose items are inlined in
place.
- `ast.rs` — `AsmStmt::Splice(Expr)` (after `If`).
- `parser.rs` `asm_stmt()` — the `{` arm (gated on `splices_allowed`, like
  operand splices), before the `Instr` fallthrough.
- `eval/asm.rs` — the `Splice` arm REUSES `AsmStmt::Call`'s Code-append path
  (`buf.items.extend(inner.items)` + the `[prov.comptime]` call-site note);
  `Code.empty()` → nothing, `Data` → steering error, other → type error
  naming Code.
- No lowering change (inlined items are already-lowered forms). Hygiene
  UNCHANGED — per-block scope; the skeleton owns its labels, holes are
  label-free.

**Tests** (`tests/asm_splice.rs`, 8): splice inlines a helper's Code; empty
splice emits nothing; **two label-carrying template instantiations in one proc
do not collide** (the t11 wall, inverted — the acceptance shape); nested
splice; non-Code error; Data steering error; splice-outside-asm-template probe
(→ parse error, the accepted asm-block-only v1 boundary); conditionally-empty
Reg splice (the aabb pattern).

**Acceptance retrofits** (spec §5, aeon):
1. `sprites.emp emit_piece_loop` — un-reverted as the §2 skeleton-with-holes.
   The loop-back `.piece_loop` + `dbeq` live in ONE block; the four variants'
   differences factor into label-free `{y_term}`/`{size_link}`/`{tile_term}`/
   `{x_term}` splices (each helper returns straight-line Code via `return
   asm{...}`; `x_term`'s `.x_ok` is self-contained). **BYTE-IDENTICAL** to the
   four inline variants, both shapes (sprites_port zero diff).
2. `aabb.emp` — the conditional zero-copy move is a `{lead_move(adim,cdim)}`
   splice of a conditionally-empty helper. **Byte-neutral** (collision_port +
   rings_port green; all call sites alias → empty splice), and the
   formerly-latent-dead move is now real code for the first non-aliasing
   caller.

**Bookkeeping** (§6): gap-ledger CLOSES the t11 emit ask (shipped via this
feature; the chosen fix was Code-fragment splice, not cross-fragment label
scope) + ADDS two zero-demand rows (cross-fragment per-instantiation label
scope; Data-splice-into-code). Port-loop step-4 inventory extended with "loop
templates via `{code}` splice (skeleton-with-holes)". No kill-list rows (no
twin mirror; the retrofits ride existing gates).

## What each pass added

**Build (TDD) findings:**
- The core mechanism was RED-first (`splice_inlines_helper_code` — "expected an
  instruction, found LBrace" → implement → green). The remaining §4 cases +
  the aabb-pattern test lock the surface.
- **`splices_allowed` is the natural v1 boundary.** Splices are legal only
  inside `asm{ }` block expressions (`asm_expr`, splices_allowed=true) — which
  is exactly where comptime-fn templates put them (`return asm{...}`). Plain
  proc bodies gate false, so a proc-body `{...}` is a clean parse error, not a
  splice. This matches the spec's §4 probe outcome ("asm-block-only acceptable
  v1") — no work needed for proc-body position.
- **`return asm { }` yields empty Code** (the empty-splice helper spelling) —
  confirmed by `empty_splice_emits_nothing`. The t11 Unit result was specific
  to if-BRANCH *statement* position (`if c { asm{} }`), not `return`.

**Acceptance findings:**
- The skeleton-with-holes retrofit is DRY without indirection cost: the
  invariant reads/epilogue/dispatch appear once, the flip logic is four named
  term-helpers, and it stays fully unrolled (zero JSR per piece) — the perf
  intent the four inline variants encoded is preserved, byte-for-byte.
- The aabb rewrite fixes a latent-dead path *and* documents it: `if c { asm{} }`
  in a `head ++ …` position silently drops the branch (Unit left-identity), so
  the move only ever "worked" because callers aliased. The splice makes the
  intent executable.

## Neither-bucket
- Minor pre-existing parser quirk observed (not this feature): a single-line
  `proc p() { call() }` trips the Call arm's `expect_line_end` on the closing
  brace; multi-line proc bodies are unaffected. Left as-is (out of scope).

## Merge checklist (Volence's gate)
- `--no-ff` merge both sides + push. Feature is byte-neutral everywhere; no
  reference-ROM rebuild or re-pin needed (the retrofits produce identical
  bytes). Separate commits: feature (`7dcf65e`), retrofits, docs.
- Same wave as the diagnostics construct (same `eval/asm.rs` territory) if you
  want them together.
</content>
