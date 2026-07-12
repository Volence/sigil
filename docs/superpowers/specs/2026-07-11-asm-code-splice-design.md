# `asm{}` Code-splice — `{expr}` at statement position (mini-spec)

2026-07-11 · Fable · status: RATIFIED (Volence, 2026-07-11 — "implement it
then fix the function"). The Plan-4 `CodeItem::Inline` gap, now demanded.

## 1. Problem — two demand points from tranche 11

(a) **The `emit_piece_loop` wall** (t11 packet): a comptime loop template
whose body varies conditionally cannot be assembled by `++`-concatenating
`asm{}` fragments — each block is its own hygiene scope, so the loop-back
label defined in one fragment is invisible to the `dbeq` in another.
Result: the approved `emit_piece_loop(xflip, yflip)` dedup was built,
link-failed (`.piece_loop` unresolved), and reverted to four byte-exact
inline variants.

(b) **aabb.emp's latently dead conditional move**: an `asm{}` in
if-branch position is a statement yielding Unit — the would-be-emitting
branch is never taken; it only "works" because every call site aliases
the registers. Same gap from the other side: no way to hand a
conditionally-empty Code value to the emission stream.

## 2. Surface

`{expr}` at STATEMENT position inside an `asm { }` block:

```
comptime fn emit_piece_loop(xflip: bool, yflip: bool) -> Code {
    return asm {
        .piece_loop:
            ...                      // shared reads, written once
            {x_term(xflip)}          // holes: comptime exprs of type Code
            {y_term(yflip)}
            ...
            dbeq    d4, .piece_loop  // label + branch in the SAME block
    }
}
```

Same spelling as the existing mnemonic/size splice (`TextOrSplice`), one
level up. `{` at statement position in asm context is currently free
(nothing else starts with it there) — verify with a parser probe before
building.

## 3. Semantics

- `expr` is evaluated in the enclosing comptime scope, at block-evaluation
  time, and must yield `Code`. Its items are inlined in place, in order.
- **`Code.empty()` splices to NOTHING** — this is the aabb idiom: a helper
  returns either the move or empty, and the call site is one splice.
- `Data` value → steering error ("data belongs in `dc`/`bytes()`; a Data
  splice is unbuilt — ledger demand if you hit this"). Other values →
  type error naming Code.
- **Hygiene is deliberately UNCHANGED.** Block scope stays per-block;
  per-instantiation label freshening stays as-is. The spliced value's own
  labels were already resolved/freshened within its producing block; the
  skeleton's labels resolve within the skeleton block. A fragment can
  neither define nor reference a skeleton label — that is the FEATURE
  (the loop structure lives in one visible place), not a limitation.
  The alternative fix (cross-fragment per-instantiation label scope)
  stays LEDGERED at zero demand; its ratifying case would be a
  conditional fragment that must OWN a label the skeleton branches into.
- Nesting: a spliced fragment may itself contain splices (it's just
  evaluation). No new recursion concerns — comptime eval already bounds.

## 4. Implementation map (C1-batch class, byte-neutral)

- `ast.rs` — `AsmStmt::Splice(Expr)` (new variant, after `If`).
- `parser.rs` `asm_stmt()` — on `{`, parse expr + closing `}` as Splice
  (alongside the `let`/trap arms).
- `eval/asm.rs` — new arm: eval expr; `Value::Code(buf)` → append items
  inline (the same append path `AsmStmt::Call`'s Code return uses today —
  reuse it, don't fork it); non-Code → the §3 errors.
- No lowering change (inlined items are already-lowered forms).
- Tests (`tests/` next to the existing asm/eval suites): empty splice
  emits nothing; two instantiations of a label-carrying template in one
  proc don't collide; splice inside a spliced fragment; non-Code value
  error; Data value steering error; splice at proc-body statement
  position outside `asm{}` — decide by probe: if `AsmStmt` grammar is
  shared it comes for free (fine, same semantics); if not, asm-block-only
  is acceptable v1.

## 5. Acceptance — the reverted work is the vector

1. **sprites.emp `emit_piece_loop` retrofit**: un-revert, restructured as
   the §2 skeleton. The four inline variants (current file state) are the
   byte-exact reference — DEBUG + plain shape byte gates green, zero
   diff. Before writing the template, diff the four variants to confirm
   they differ ONLY in the x/y term positions (t11's approved design says
   so); a structural quirk beyond the holes → third parameter or that
   variant stays inline, commented.
2. **aabb.emp conditional-move fix**: rewrite the Unit-branch `if` as a
   splice of a conditionally-empty helper. Byte-neutral TODAY (all call
   sites alias → empty splice) — and the latent dead path becomes real,
   working code for the first non-aliasing caller.
3. Strict suite + clippy clean; mixed-build per house rules.

## 6. Bookkeeping shipped with it

- Gap-ledger: CLOSE the t11 ask (this spec); ADD "cross-fragment label
  scope" (zero demand, escalation path per §3); ADD "Data splice"
  (zero demand).
- Port-loop step-4 construct inventory: extend the comptime-fn-helpers
  line with "loop templates via `{code}` splice (skeleton-with-holes)".
- No kill-list rows (no twin mirror; the retrofits ride existing gates).

## 7. Sequencing

Post-t11-merge wave, alongside the diagnostics construct (same
`eval/asm.rs` territory, both byte-neutral) — separate commits, one
branch is fine. The sprites/aabb retrofits are this feature's step-6
sweep obligation and land in the same wave.
