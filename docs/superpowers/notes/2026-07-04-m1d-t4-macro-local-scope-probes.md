# M1.D T4 — macro-internal `.`-local label scoping : live-asl probe matrix

**Tool:** `aeon/tools/asl` (asl 1.42 Beta Bld 212), `-cpu 68000 -xx -n -q -c -A -L -U`.
**Date:** 2026-07-04. Probe sources: `scratchpad/probe_maclocal.sh` (+ the P1/P3/P4 raw
`.lst` in `scratchpad/mac2/`).

## Why this exists

First `m1c_rom` full-ROM emit run (T4). `link()` fails with 14 "symbol … redefined
by section … (already defined by same section)" diagnostics (the T3 duplicate-
section-symbol hardening firing). Every colliding name is a **macro-internal `.`-local**:
`.wait_z80` (`stopZ80`, `macros.asm:228`), `.done` (`queueStaticDMA`, `macros.asm:299`,
expanded **7×** inside `Enqueue_Dirty_Buffers`), `.rt_static_tag` (`reloadAnimTimer`,
`animate.asm:54`), `.aovutag` (a touch/collision macro). sigil qualifies a `.`-local to
the caller's nearest global label (`self.scope`) and pushes it into the section labels;
two expansions in one global scope collide.

The `m1c_full` *recon* never surfaced this — it stops before `link()`. This is the
first-diff the emit path was armed to find.

## The decisive probes

| probe | construction | asl result |
|---|---|---|
| **P1** | macro with internal `.done` (def + `beq.s .done` both inside), expanded **twice** under ONE global label `Foo` | **0 errors.** Each `beq.s .done` binds to *its own expansion's* `.done`: expansion-1 `6702` (PC 0→4), expansion-2 `6702` (PC 4→8). **No `Foo.done` in the symbol table.** |
| **P3** | the SAME shape but `.done` written **twice at source level** (no macro) under `Foo` | **hard error** `#1000 symbol double defined: Foo.done`. `Foo.done` IS a symbol-table entry. |
| **P2** | macro expanded once each under `Foo` and `Bar` | resolves per-scope, no error; neither `Foo.done` nor `Bar.done` is a user symbol |
| **P4** | macro expanded **once** under `Foo` | resolves; **no `Foo.done` in the symbol table** |

## The rule (settled)

- A `.`-local defined **inside a macro expansion** is scoped to **that expansion**
  (unique per expansion). Redefinition across expansions in one global scope is legal;
  each internal reference binds within its own expansion. The label is **not** a
  caller-qualified user symbol (invisible in the symbol table).
- A `.`-local defined **at source level** is scoped to the enclosing global label, and
  **redefining it is a hard error**.

This is confirmed against the real ROM: `boot.asm` has BOTH a source-level `.wait_z80`
(line 64 → `Cold_Boot.wait_z80` @ `0x260`, present in `s4.lst`'s symbol table) AND a
`stopZ80` expansion (line 124, internal `.wait_z80` @ `0x2B4`, **absent** from the
symbol table). sigil wrongly qualified the macro one to `Cold_Boot` → collision with the
source-level one. Symbol-table membership is **byte-irrelevant** for A1 (`emit_rom`
never writes the symbol table into `s4.bin`; convsym appends nothing) — only the
*resolution* (branch bytes) matters.

The aeon `macros.asm:305` comment ("expand each macro at most once per global-label
scope") is over-cautious convention, **not** an asl constraint — `queueStaticDMA.done`
expands 7× in one scope and asl assembles it clean (P1 + the real ROM both prove it).

## Fix direction (implemented in T4)

In `expand_macro_inner`, run the body under a **fresh per-expansion scope** (a reserved,
non-user name) instead of the caller's `self.scope`; save/restore around `exec`. A
per-pass monotonic counter names each expansion. All aeon macro `.`-locals are
def+ref within the same expansion and branched to by **fixed-length short branches**
(`beq.s`/`bne.s`), so this change touches **no layout/length** — zero convergence risk.

Known limitations (none reached by aeon, documented at the call site): a macro that
references a caller-scope `.`-local, or defines a NON-dotted global label expected to
become the outer scope, would diverge — aeon does neither.

## Goldens

`t4_maclocal_twice` (P1: macro `.done` ×2 in one scope → `6702 4E71 6702 4E71`),
`t4_maclocal_fwd` (the forward-ref-within-expansion short branch). `gen_snippet_vectors`
must churn only these.
