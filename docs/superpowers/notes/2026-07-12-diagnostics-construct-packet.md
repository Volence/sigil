# Diagnostics construct (`assert` / `raise_error`) — build packet

Date 2026-07-12. Subagent-driven build of the `.emp` diagnostics construct
(spec `specs/2026-07-11-emp-diagnostics-construct-design.md`, plan
`plans/2026-07-11-emp-diagnostics-construct.md`). Branch `diag-construct` off
BOTH masters (post-splice: emit_piece_loop + A1 fold already under it).
**NOT merged — Volence's gate.**

## Headline

`assert.<w> src, cond [, dest]` and `raise_error "<fstring>"` are now `.emp`
statement constructs. `assert` self-gates to zero bytes in the plain shape;
`raise_error` is unconditional. The construct's output is **byte-IDENTICAL to
vladikcomper's real `debugger.asm` `assert`/`RaiseError` macro** — proven at the
CLI level by assembling the unmodified 25KB macro tower through the AS front-end
and byte-diffing. rings.emp + core.emp retrofitted (kill row 16), byte-neutral.

**Gates:** full strict **2208/0**, clippy clean, both aeon shapes build
(default 451390 / DEBUG 459174, DEBUG md5-identical to the s4.debug.bin oracle),
DEBUG-shape port gates green, `repin --check` untouched (no re-pin — byte-neutral).

## Commits (branch `diag-construct`, both repos)

Sigil: `95fcb26` FSTRING encoder · `cd7a8fb` encoder review-fixes · `cb32a88`
AST+parser · `693ea85` verbatim source-slice · `7e64e05` parser polish · `f504f58`
desugar+gating · `570625b` desugar cleanups · `e6ee544` AS-twin vectors · `8358beb`
vectors portability · `84e992c` port-test doc refresh · `eda533b` bookkeeping.
Aeon: `452c7c1` rings+core retrofit · `cff8304` comment refresh.

## Per-task findings

**Task 1 — FSTRING encoder (pure Rust).** Byte constants sourced from
`debugger.asm` (verified against source, cross-checked by review). Review folded
in: narrowed a module-scope `#![allow(dead_code)]` to per-item; made an
out-of-range control param a loud error (never silent `& 0xFF` truncation);
locked the encode_fstring terminator seam.

**Task 2 — AST + parser + the operand-spelling design call.** Cond validated at
parse against the 16 Bcc codes; width required; `raise_error` one-string-only.
**DESIGN CALL (ratified by Volence):** the auto-message embeds operand SPELLINGS
verbatim (§4.4). Token-reconstruction (the first cut) reprinted non-decimal
literals in decimal (`#$8000`→`#256`) — a silent divergence from the AS twin.
Slice-at-eval was verified INFEASIBLE (`Evaluator`/`lower_module`/`File` carry no
source; source→line resolution lives at the outer layer). Resolved by slicing the
verbatim source at PARSE (`Rc<str>` on `Parser`) — byte-exact by construction for
ALL operand forms. Ledgered SCAFFOLDING-ERA (the divergence only bites dual-build
sites; lifts at Spec 5). A `debug_assert` guards the slice fallback.

**Task 3 — desugar + DEBUG gating + a real spec finding.** The 11-step §4.2
expansion synthesized as `AsmStmt`s; hygienic `.skip`/`.raise` labels; DEBUG read
from the comptime env like the `If` arm; reverse-order arg push for `raise_error`.
Golden bytes obtained by ASSEMBLING the transliteration (never hand-derived).
**SPEC FINDING (owed to you): the §4.5 / §4.2-step-9 parity DIRECTION word is
inverted.** From `debugger.asm:264` (`((((*)&1)!1)*_eh_align_offset)`) and all four
transliteration blocks: the align pad fires at an **EVEN** flag offset (to
word-align the following `jmp`), not odd. Concrete byte examples in both spec
sections are correct; only the direction word is wrong. Verified table:

| site | msg_len | parity | bytes |
|---|---|---|---|
| rings `.b d4,eq,#0` | 50 | EVEN | `$A0,$00` pad |
| core `.l a0,hs,#Object_RAM` | 59 | ODD | `$20` |
| core `.l a0,lo,#Object_RAM_End` | 63 | ODD | `$20` |
| core `.w d7,lo,#NUM_DYNAMIC` | 60 | EVEN | `$A0,$00` pad |

**SEMANTIC CALL (reviewer + you: keep as-is).** Cond/src-register validation runs
BEFORE the DEBUG gate, so a malformed assert errors even in a DEBUG=0 release build
— stricter than AS (where an `ifdef __DEBUG__` assert never compiles in release).
Kept per §5 (unconditional checks) + loud-over-silent + DEBUG=0 being the CI shape.

**Task 4 — AS-twin acceptance vectors.** The STRONGEST gate: the AS reference
assembles the **real unmodified `debugger.asm` macro** through `sigil_frontend_as`
(the AS front-end supports the full tower — `macro`/`switch`/`while`/`strstr`/
`val`/`!align`/nested macros), so ZERO hand-transcribed bytes. 4 positive vectors
byte-identical (`.b` tst / `.w` #symbol / `.l` no-pad-parity / `raise_error` arg),
5 negatives reject with steering messages. A negative-control experiment (break one
vector → confirmed FAIL at the predicted bytes → revert) proved it isn't
false-confidence. Review fix: replaced a hardcoded path with the sibling
`AEON_DIR` + `SIGIL_STRICT_GATE`-skip convention; single-sourced the shared symbol
addresses.

**Task 5 — rings+core retrofit (byte-neutral).** +6/−89. Both `.emp` blocks
collapse to the construct; mechanics comments deleted, behavior kept; operand
spellings identical to the twins. DEBUG-shape port gates
(`rings_debug_region_matches_reference`, `core_debug_region_matches_reference`,
`debug_shape_length_diverges`) are load-bearing — a negative control (doctor
`#Object_RAM`→`#Object_RAM_End`) reddened the core gate, confirming the assert
region is genuinely covered. Scope held: rings+core ONLY; the retro-review asserts
(DeleteObject range, animate AF_CHANGE/AF_BACK underflow) are untouched, ledgered
for a later sweep.

**Task 6 — bookkeeping.** kill row 16 KILLED + new row 21 (twin-parity emission,
dies at Spec 5); closed the "assert/diagnostics demand 1/2" ledger row; added
demand-0 rows + the scaffolding-era spelling note + the retro-review follow-ons;
port-loop construct inventory += `assert`/`raise_error`.

## File-over-plan discrepancies (all trusted the file)

- debugger.asm equates at 53-107 / 120-122 / 746, NOT the plan's "85-130".
- `lower/script.rs` synthesis pattern at ~799-852, not "380-540".
- The plan's "slice the source at the operand span at parse" dead-ended (`Parser`
  held only tokens); resolved by threading `Rc<str>` — a change the plan didn't
  anticipate, ratified by you.
- Spec §4.5/§4.2-step-9 parity direction inverted (above).

## Owed to you (spec-side + gate)

1. **Spec §4.5 + §4.2-step-9 amendment:** flip the parity direction word to
   "even" (byte examples already correct).
2. **Spec §4.4 amendment:** state spellings are the verbatim source substring
   (byte-exact, no radix restriction), sliced at parse.
3. **Empyrean §4.9 `table` paste** (`57e2bce`) — still local, awaiting your push.
4. **Concurrent oracle-profiler ledger change** — an unrelated in-flight edit
   (oracle profiler caching bug FIXED, oracle `linux-port`, "CLOSE on merge") sat
   uncommitted in `campaign-gap-ledger.md` all session; I kept it OUT of every
   diag commit (stashed during my ledger edit, restored after). It's still
   uncommitted for you to commit/push as you see fit.
5. **Merge gate:** `--no-ff` merge `diag-construct` both sides + push, at your call.

## Next (per plan §9)

Step-6 sweep already discharged (rings+core, this build). Next candidate:
entity_window.asm port (11 assert sites) — the ratifying demand — now converts
with one-line asserts.
