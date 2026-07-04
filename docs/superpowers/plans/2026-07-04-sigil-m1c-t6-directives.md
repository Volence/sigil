# Sigil M1.C — T6: Data / Reserve / Align / Org Directives Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.
> CI runs `cargo clippy --workspace --all-targets -- -D warnings` — run THAT (with `--all-targets`).

**Goal:** Add the 68k data/layout directives Aeon uses: `dc.w`, `dc.l`, `ds.b`/`ds.w`/`ds.l`,
`align` (arbitrary boundary incl `align $8000`), `org`; and restore `gen-snippet-vectors` by
fixing the stale `struct_field_indexed` snippet. Spike 0: `even` has 0 real uses — out of scope.

**Architecture:** Mirror `directive_db`/`directive_dw` (eval.rs:880/896). New directives dispatch
in `dispatch()` (eval.rs:779) and register in `is_op_keyword` (eval.rs:1632). Everything emits
through the existing `IrStreamer` (`emit_data`/`emit_fill`/`reserve`).

**Tech Stack:** Rust; asl-diff via `snippets_golden.txt`. **Once `struct_field_indexed` is
fixed, `gen-snippet-vectors` works again — use it to regenerate goldens** (it re-runs real asl).

## Verified asl semantics (`-cpu 68000`)
- `dc.w $1234` → `12 34` (**big-endian** 16-bit; distinct from Z80 `dw` which is little-endian).
- `dc.l $12345678` → `12 34 56 78` (big-endian 32-bit).
- `ds.b 3` → `00 00 00` (reserve N units, zero-filled by p2bin). `ds.w N` = 2N zero bytes,
  `ds.l N` = 4N.
- `align 2` at an even offset → no fill; padding is zero bytes. **Probe `align` with real
  padding and `align $8000` to confirm the fill byte (expected `0x00`) and boundary math.**
- `org N` sets the assembly offset/PC. **Check the 4 real `org` sites in Aeon** (`grep -rn
  '\borg\b' games engine --include='*.asm'` in `/home/volence/sonic_hacks/aeon`) to see whether
  they set an absolute address or interact with `phase`; match that semantics. (M0 used `phase`
  for VMA; `org` may just set the current offset within a section — verify vs asl.)

## Scope
`dc.w`/`dc.l` (BE), `ds.b`/`ds.w`/`ds.l` (zero-fill), `align <n>` (pad to next multiple of n with
`0x00`), `org` (per the Aeon usage you confirm). Honor the current `padding` state where relevant
(Aeon sets `padding off` globally → `dc.b` odd runs are NOT auto-even-padded; verify `dc.w`/`dc.l`
are unaffected since they're already even-sized). NOT `even` (0 uses).

## Steps (TDD)
- [ ] **Step 1 — fix `struct_field_indexed`** so `gen-snippet-vectors` runs. Run
  `cargo run -p sigil-frontend-as --bin gen-snippet-vectors` and read the abort. The snippet is
  `cpu z80` with a `struct`/`ds.b`/`ds.w` body + a `db`. Determine what asl rejects (likely
  `ds.b`/`ds.w` as struct-field sizers under `cpu z80` — asl's z80 pseudo-op set differs). Fix by
  giving that snippet the CPU context asl accepts (probe: does asl accept `ds.b` struct fields
  under `cpu 68000`? if so, move the struct+test to `cpu 68000`, adjusting the instruction to a
  68k one), keeping its byte-exactness. Re-run the tool → it must complete and rewrite goldens
  with NO diff for the other blocks (prove it's a no-op regen for everything else).
- [ ] **Step 2 — snippets first** (under `cpu 68000`): `dc.w $1234,$5678`, `dc.l $12345678`,
  `ds.b 3`, `ds.w 2`, `ds.l 1`, `align 2` (from an odd offset — put a `dc.b 1` before it),
  `align $10` (from a non-aligned offset), `org` per Aeon usage, and a `dc.b`-odd-run to confirm
  `padding off` leaves it unpadded. Distinct names.
- [ ] **Step 3 — golden bytes via `gen-snippet-vectors`** (now working). Commit.
- [ ] **Step 4 — gate fails** for the new blocks.
- [ ] **Step 5 — implement** `dc.w`/`dc.l` (BE — `to_be_bytes`), `ds.*` (emit `emit_fill(count*unit, 0)`
  or a `Reserve`), `align` (compute pad = `(-offset).rem_euclid(n)`, `emit_fill(pad, 0)`), `org`
  (set the section offset per verified semantics). Dispatch + `is_op_keyword`. Unit-test each.
- [ ] **Step 6 — gate green + suite.** asl_snippets PASS; `cargo test --workspace` PASS.
- [ ] **Step 7 — `cargo clippy --workspace --all-targets -- -D warnings` + build clean.**
- [ ] **Step 8 — commit** `feat(sigil-frontend-as): dc.w/dc.l/ds.*/align/org directives + fix struct snippet (asl-gated)`.

## Self-Review
- Spec coverage: dc.w/dc.l/ds.*/align/org, snippet-gated; struct snippet fixed (tool restored).
- Honest gate: goldens from `gen-snippet-vectors` (real asl) once the tool runs; if it still can't,
  hand-verify per-snippet as prior tasks did and say so.
- Escalate if: `org` semantics vs `phase` are ambiguous, or `align`'s fill byte isn't `0x00`.
