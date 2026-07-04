# Sigil M1.C — T8: `set` / `:=` Reassignable Accumulators Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.
> CI runs `cargo clippy --workspace --all-targets -- -D warnings` — run THAT.

**Goal:** Add AS reassignable symbols `name set value` and `name := value` (currently only the
single-assignment `name = value` equate exists). Used broadly (`:=` ×149 — band counters,
`OE_PREV_X` monotonic-sort checks, the deform accumulators). Prerequisite for T8b.

**Semantics (probed vs real asl 2026-07-04):** reassignable + **imperative / emission-order** —
`i set 0 / dc.b i / i set i+5 / dc.b i / j := 10 / dc.b j / j := j*2 / dc.b j` → `00 05 0A 14`.
`set` and `:=` behave identically. Each directive redefines the symbol to the RHS folded with
the symbol's CURRENT value; expressions read the value at their point of use. (Contrast `=`:
single-assignment.)

## Files
- `crates/sigil-frontend-as/src/eval.rs` — dispatch `name set <expr>` and `name := <expr>` (in
  `exec_one`, alongside the existing `body[1]==Eq` equate check at ~line 491); a
  `directive_set` that folds the RHS (using the current env, so `i set i+5` reads the current
  `i`) and **redefines** the symbol via `self.env.define` (overwrite allowed).
- `crates/sigil-frontend-as/src/lexer.rs` — confirm `:=` tokenizes; if it lexes as `Colon` then
  `Eq` (two tokens), handle that shape, or add a `Punct::ColonEq`. (Check first.)
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — new blocks (regen via `gen_snippet_vectors`).

## Multi-pass note
The front-end re-emits each pass with env seeded from the prior pass. A `set`/`:=` symbol's value
is emission-order within a pass; its LAST value at pass end seeds the next pass. For accumulators
that are fully determined by emission order (e.g. a rept counter), this converges immediately. Do
NOT treat `set`/`:=` symbols as forward-referenceable equates — they are imperative; a use before
the first `set` reads whatever the seed/last-pass value is (match asl: an undefined-first-use is
an error, but a rept accumulator is always `set` before use).

## Steps (TDD)
- [ ] **Step 1 — snippets first** (`cpu 68000`, `padding off`; regen with `gen_snippet_vectors`):
  - `set_accumulator`: `i set 0 / dc.b i / i set i+5 / dc.b i` → `00 05`.
  - `coloneq_accumulator`: `j := 10 / dc.b j / j := j*2 / dc.b j` → `0A 14`.
  - `set_in_rept`: `k set 0 / rept 4 / dc.b k / k set k+1 / endr` → `00 01 02 03` (the deform
    accumulator pattern, minus the float — proves `set` works inside `rept`, a T8b prerequisite).
- [ ] **Step 2 — regen goldens** via `gen_snippet_vectors`; `git diff` touches only new blocks. Commit.
- [ ] **Step 3 — gate fails** (`set`/`:=` unrecognized → error or wrong bytes).
- [ ] **Step 4 — implement** `set`/`:=` dispatch + `directive_set` (fold RHS with current env,
  redefine). Unit-test: reassignment, `i set i+5` self-reference, `:=` parity with `set`, and a
  `set` inside `rept` accumulating.
- [ ] **Step 5 — gate green + suite.** asl_snippets PASS; `cargo test --workspace` PASS.
- [ ] **Step 6 — `clippy --workspace --all-targets -- -D warnings` + build clean.**
- [ ] **Step 7 — commit** `feat(sigil-frontend-as): set/:= reassignable accumulators (asl-gated)`.

## Self-Review
- Spec coverage: `set`, `:=`, reassignment, self-reference, in-`rept` accumulation — snippet + unit gated.
- Honest gate: goldens from `gen_snippet_vectors` (real asl).
- Escalate if: `:=` tokenization is ambiguous, or `set`/`:=` symbols interact badly with the
  multi-pass convergence (e.g. an accumulator that references a forward label).
