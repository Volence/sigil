# Contract-grammar v2 — G2 CHECKPOINT (consumption check works; pre-retrofit)

**2026-07-17, Opus.** The checkpoint the parcel calls for: the §6 flag-result
must-use machinery is built + TDD-green and runs over the **real aeon corpus**,
BEFORE the call-site retrofit adjudication. Per the standing rule, the list of
every flag-result call site + my proposed consume-vs-discard ruling goes here
for Fable's gate (via Volence).

Branches (isolated, byte-neutral): sigil `feat/contract-grammar-g2` (5 commits),
aeon `feat/contract-grammar-g2` (empty — no retrofit yet).

## What is built (sigil, all TDD)

| Piece | Spec | Tests |
|---|---|---|
| `out(carry: name)` + `out(rN if cc)` grammar (proc/extern/type) | §6/§2 | 7 |
| `@discards(name)` trailing call attribute | §6/§11 Q3 | 2 |
| `[call.flag-result-unused]` — CFG consumption check | §6 | 7 |
| `[call.result-invalid-path]` — conditional register result | §6/D2.35 | 3 |
| whole-corpus wiring (`analyze_corpus.flag_firings`) | §11 Q2 | 3 |

frontend-emp **1440/0**, clippy clean. G1 closure pin still green.

## §11 implementation decisions (recorded)

1. **CFG granularity (Q1)** — a LIGHTWEIGHT CFG over the evaluated CodeBuf with
   real joins (visited-set BFS reachability of an abandon-without-consumer
   path). NOT straight-line (the pre-registered requirement). Loops terminate on
   the visited set.
2. **Where it runs (Q2)** — the whole-corpus frontend walk, post-closure (the
   check needs cross-module contract knowledge — a callee's `out(carry:)` may be
   in a different module than its caller, e.g. RingBuffer_Add). Reuses the real
   evaluated CodeBufs (zero drift) and `instr_written_regs`.
3. **@discards attachment (Q3)** — trailing-attribute-on-call
   (`jbsr Queue @discards(dropped)`). Parses cleanly. Matched to the call by
   source span (AST InstrLine.span == CodeBuf Instr.span for direct calls).

## Modeling stance (the one design call that matters)

`writes_carry` (the redefine set) is a curated **allowlist** of CC-writing 68000
ops + the call mnemonics; an unrecognized mnemonic is **CC-transparent**
(false-negative-leaning — never fires on an instruction it does not model). This
is exactly what the dplc `movem.l (sp)+` between the call and its `bcs` requires
(`movem` preserves CCR — the code's own hazard note). Consumers (`bcs`/`bcc`/
`bhi`/`bls` + ADDX-class) are checked first. `sr`/full-CCR liveness stays S2-D7 —
per-call-site carry def-use only (§6 scope fence).

## Firing reconciliation — DECLARED-contract dry-run

Dry-run (contracts temporarily declared, then reverted): with all three
`out(carry:)` contracts in place, the check reports **0 flag-result firings**,
and the G1 closure residue is **UNCHANGED** (flags are not register-file
members, so §1 is unaffected — the closure pin still asserts the exact 6-row G3
handoff). This is the evidence the rulings below are correct.

## The retrofit I propose (3 callees, byte-neutral contract text)

| Callee | Where | Contract to add | Why |
|---|---|---|---|
| `QueueDMA_Important` | dplc.emp:28 (extern) | `out(carry: dropped)` | parcel-mandated; `.asm` doc "Out: carry = dropped" |
| `QueueDMA_Deferrable` | dplc.emp:30 (extern) | `out(carry: dropped)` | parcel-mandated |
| `RingBuffer_Add` | rings.emp:49 (pub proc) | `out(carry: full)` | rings.emp:44 already documents "carry clear = success, carry set = buffer full"; the "RingBuffer_Add class" the parcel names |

## Every `.emp` call site of a flag-result callee + proposed ruling

| # | Call site | Callee | Consumer on every path? | **Ruling** |
|---|---|---|---|---|
| 1 | dplc.emp:75 `jbsr {queue}` (Perform_DPLC, `perform_dplc(QueueDMA_Important)`) | QueueDMA_Important | `bcs .done` (line 77), after the CC-transparent `movem.l (sp)+` (line 76) | **CONSUME** — no `@discards` |
| 2 | dplc.emp:75 `jbsr {queue}` (Perform_DPLC_Deferrable, `perform_dplc(QueueDMA_Deferrable)`) | QueueDMA_Deferrable | same template, `bcs .done` | **CONSUME** — no `@discards` |
| 3 | entity_window.emp:1005 `jbsr RingBuffer_Add` | RingBuffer_Add | `bcs .gated` (line 1006) | **CONSUME** — no `@discards` |

**Zero `@discards` sites.** Every corpus caller of a flag-result callee already
branches on the carry. The mechanism catches the bug class (Palette_Dirty /
load_art) without needing any opt-out in aeon today — the clean outcome.

### Not-in-scope, explicitly checked

- **plane_buffer.emp** — the three "silently drops if buffer full" procs
  (`Draw_TileColumn` / `Draw_TileRow_FromCache` / `Draw_BG_TileColumn`) call NO
  queue routine; they drop internally with `Out: none` (design), so they are not
  flag-result callees and get no contract. The `.asm`-tier silent-drop bugs
  (`buffers.asm` Palette_Dirty/Sprite_Table_Dirty, `load_art`) are the concurrent
  **aeon `fix/silent-drop-class`** parcel's job — no file overlap; cite each
  other at merge.
- **Conditional register results** (`out(rN if cc)` + `[call.result-invalid-path]`)
  — grammar + check built + TDD'd, but ZERO corpus sites declare one. Forward
  machinery, inert on the real corpus (the G1 subcontract-check precedent).

## Proposed path past the checkpoint (for Fable's ruling via Volence)

1. Apply the 3 `out(carry:)` decls (byte-neutral contract text only).
2. Adjudicate the 3 call sites as **CONSUME** (above) — no `@discards`.
3. Re-run: expect **0 flag firings** (dry-run confirmed) + ROMs byte-identical
   + G1 closure pin unchanged.
4. Add a strict-gated corpus pin asserting **0 flag-result firings** (the
   regression guard, mirroring the G1 residue pin).

**Recommendation:** the rulings are unambiguous (all three sites consume via a
carry branch; the dry-run proves 0 firings). Barring a Fable objection,
proceeding with the retrofit + the zero-firing pin is safe. G1+G2 together are
the pass-3 gate — the packet will say so explicitly.
