# Sigil M1.C — T7: struct/_len + Keyword Macro Args + ALLARGS/MOMCPUNAME Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.
> CI runs `cargo clippy --workspace --all-targets -- -D warnings` — run THAT.

**Goal:** (1) Add **keyword macro arguments** (`NAME=value`, the new-for-68k feature that
`deform_table_sine` needs, a T8 prerequisite); (2) verify `struct`/`_len` + the
`if <Name>_len <> N / error` assertion works for 68k; (3) verify `ALLARGS`/`MOMCPUNAME` `pbyte`
dual-CPU dispatch (`db` vs `dc.b`) for the sound data files. Items (2)/(3) already exist from
M0 Plan-4 (Z80) — confirm + snippet-gate them under `cpu 68000`; (1) is the real new work.

**Tech Stack:** Rust; asl-diff via `gen_snippet_vectors` (restored in T6) — regenerate goldens
from real asl.

## Verified asl keyword-arg semantics (probed 2026-07-04)
Macro `tst macro AMP,PER`:
- `tst AMP=7,PER=9` → `07 09` (keyword bind by name)
- `tst 3,4` → `03 04` (positional)
- `tst PER=5,AMP=2` → `02 05` (**order-independent** — bound by name regardless of position)

**Rule:** for each comma-split invocation arg, if it lexes as `Ident(name), Eq, <value...>` and
`name` is a declared parameter, bind that parameter to `<value>` (regardless of position);
otherwise bind positionally in order (skipping already-keyword-bound params). Mixable.

## Files
- `crates/sigil-frontend-as/src/eval.rs` / `src/expand.rs` — `expand_macro` arg binding: before
  the existing positional substitution, partition args into keyword (`name=value`) and
  positional, build the final `param → value-tokens` map, then substitute as today.
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — new blocks.

## Steps (TDD)
- [ ] **Step 1 — snippets first** (`cpu 68000`; regenerate goldens with `gen_snippet_vectors`):
  - `macro_keyword_args`: a `tst macro AMP,PER / dc.b AMP / dc.b PER / endm` invoked as
    `tst AMP=7,PER=9`, `tst 3,4`, `tst PER=5,AMP=2` → expect `07 09 03 04 02 05`.
  - `struct_len_assert`: a `struct` with known field sizes, then `if <Name>_len <> <N> / dc.b 1 /
    else / dc.b 2 / endif` proving `_len` computed correctly (pick N = the real total; the taken
    branch reveals it). Also a field-offset use (`dc.b <Name>_<field>`).
  - `pbyte_momcpuname`: exercise the `pbyte` macro (or a minimal `if MOMCPUNAME="Z80"` branch)
    proving `dc.b` is chosen under `cpu 68000` (and, in a second block under `cpu z80`, `db`).
    Model it on the real `pbyte` in Aeon (`grep -rn 'pbyte macro' /home/volence/sonic_hacks/aeon`).
  - `allargs`: a macro using `ALLARGS` over a variable arg count → `dc.b ALLARGS`-style emission,
    matching real Aeon usage.
- [ ] **Step 2 — regenerate goldens** via `gen_snippet_vectors`; confirm `git diff` touches only
  the new blocks. Commit.
- [ ] **Step 3 — gate fails** (keyword args unbound → wrong bytes or error).
- [ ] **Step 4 — implement keyword-arg binding** in `expand_macro`. Unit-test the three
  keyword/positional/reordered cases. Confirm existing positional-macro tests stay green.
- [ ] **Step 5 — verify struct/_len + pbyte/MOMCPUNAME/ALLARGS** already produce the golden
  bytes; if any gap surfaces (e.g. `_len` for a `cpu 68000` struct, or MOMCPUNAME under 68k),
  fix it minimally. (These are largely M0 Plan-4 features — the snippets are a regression net.)
- [ ] **Step 6 — gate green + suite.** asl_snippets PASS; `cargo test --workspace` PASS.
- [ ] **Step 7 — `clippy --workspace --all-targets -- -D warnings` + build clean.**
- [ ] **Step 8 — commit** `feat(sigil-frontend-as): keyword macro args + struct/_len & pbyte/ALLARGS/MOMCPUNAME 68k gates (asl-verified)`.

## Self-Review
- Spec coverage: keyword args (new), struct/_len assertion, pbyte/MOMCPUNAME, ALLARGS — snippet-gated.
- Honest gate: goldens from `gen_snippet_vectors` (real asl).
- Escalate if: asl's keyword+positional MIX has a corner the rule above doesn't cover, or a
  struct/_len/MOMCPUNAME 68k gap is larger than a minimal fix.
