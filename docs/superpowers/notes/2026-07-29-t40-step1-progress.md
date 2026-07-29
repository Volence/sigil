# 2026-07-29 — t40 STEP-1 progress (post design-gate endorsement)

Status: **IN PROGRESS toward checkpoint (a).** Design gate COUNTERSIGNED/PASSED
(`0349573`); overseer rulings on §5 open questions recorded below. This note logs the
step-1 findings + the two prerequisites resolved so far, and maps the precise remaining
work so a continuation runs clean.

## Rulings applied (from the countersign)
1. **Surface spelling** = `ensure(cycles(<span>) == N)`; zero new vocabulary beyond
   `cycles(...)`. Spans delimited by their bounding label pair.
2. **Branch bail = HARD** — both `[cycles.ambiguous-branch]` and `[cycles.unknown-op]`.
   No soft escape hatch until a real variable-timing span demands one.
3. **Balanced-exx recognition = wire it now** (rung-2 §4.3 machinery); failing-first:
   unbalanced-exx bails, balanced FILL pair credits `preserves(bc,de)`, non-vacuity on
   the real proc.
4. **di/ei = C3 PROSE** — ratified as argued (checkable half trivial, load-bearing half
   statically unmodelable). The both-ways argument goes in the packet verbatim.
5. **jp-cc structural pin = the cycles ensures ARE the pin** (a future `jr` in a timed
   span fires the hard bail); kill row carries the pin duty; a one-line site comment
   still welcome.
   **SEQUENCING:** step 1 transliterates the literal `rept N / nop` pads +
   `ensure(cycles(...)==N)` VERIFYING them; `pad_to_cycles(...)` is a STEP-2
   modernization. A step-2 derived/hand disagreement is a STOP finding.

## Finding 1 — ZERO operand/instruction gaps (the step-0 "may lack" list is EMPTY)

Every form I flagged at the design gate as a possible wired-set gap ALREADY lowers
cleanly AND to the correct asl-golden bytes (probed via the emp
`(cpu: z80)` proc harness, then removed):

| form | golden | emp lowers |
|---|---|---|
| `exx` | `D9` | ✓ |
| `im 1` | `ED 56` | ✓ |
| `rrca` | `0F` | ✓ (also in sequencer.emp) |
| `ld sp, $1FFE` | `31 FE 1F` | ✓ |
| `ld (nn), ix` | `DD 22 …` | ✓ |
| `ld ix, (nn)` | `DD 2A …` | ✓ |
| `bit 7, (hl)` | `CB 7E` | ✓ |
| `add iy, bc` | `FD 09` | ✓ |
| `add ix, de` | `DD 19` | ✓ (also sequencer/sfx) |

Rung 1–3 already wired the whole driver instruction set. **No one-arm additions
needed** — the transcription can proceed directly against the current lowering. (The
emp path routes to the shared asl-golden `sigil-isa::z80::encode`, so a clean lower is a
correct encode; the windowed oracle is the backstop.)

## Finding 2 — the `exx` count nit (overseer's 15 vs my 14) RECONCILED

14 CODE `exx` instruction sites + 1 COMMENT line (`z80_sound_driver.asm:74`, the FILL
shadow-block prose `; exx / dec hl / … / exx`) = 15 grep lines. The 14 code sites are
7 balanced `exx … exx` pairs (§1.5 of the design note). No functional discrepancy.

## Landed this pass — the `cycles` capability ALGORITHMIC CORE (`z80_cycles.rs`)

Commits `9e60a8b` + `73c0350` (10 tests, no crate regression). The T-state table +
`span_cost` checker + the two hard bails, modeled on `z80_bus.rs`:

- `instr_cost(mnemonic, ops) -> Cost::{Fixed(u16), Ambiguous, Unknown}` over the
  driver's timed-region op subset. Cross-checked against the driver's OWN arithmetic:
  consumer block = 22, timer poll = 30, FILL body = 44 (all exact).
- `Ambiguous` on `jr cc` / `djnz` / `ret cc` / `call cc` (differing taken/not-taken) →
  `[cycles.ambiguous-branch]`; the jp-not-jr discipline as a hard error. `jp`/`jp cc` are
  `Fixed(10)` (the positive control).
- `Unknown` on anything off the demand table → `[cycles.unknown-op]` (loud, never a
  silent default).
- `span_cost(items)` sums a straight-line span; `label_span(items, L1, L2)` carves the
  half-open `[L1.body .. L2)` instruction slice from a proc CodeBuf.

## REMAINING to checkpoint (a) — precise continuation map

1. **Wire `cycles()` into `ensure` + a per-proc pass.** `ensure(cycles(L1,L2) == N)` is
   recognized at proc lowering and deferred to a `check_z80_cycles(proc, &buf, asserts,
   diags)` pass invoked beside `check_z80_preserves` (`lower/proc.rs:148`). The pass calls
   `label_span` + `span_cost`, fires `[cycles.mismatch]` on `cost != N`, and surfaces the
   two bails. Failing-first: pad+4 fires mismatch; `jr`-in-span bails; off-table bails;
   the real DAC loop's three `ensure`s compile green + byte-neutral (self-gate to 0
   bytes). DESIGN CALL to settle: where the `ensure` lives so `.loop`/`.exhaust` resolve
   — inside the proc body (preferred: local labels in scope) vs a qualified module-level
   ensure. Recommend in-body.
2. **Balanced-`exx` recognition** in `z80_preserves` (ruling 3) — the entry-value-bit
   swap so a balanced `exx … exx` is a no-op to the main bank (credits `preserves(bc,de)`
   across FILL/refill); lone `exx` / `ex (sp),hl` stay loud bails. TDD with t24 controls.
3. **The faithful transcription** — `engine/sound/z80_sound_driver.emp`, phase $0000,
   1381 B, `module engine.z80_sound_driver (cpu: z80)`. Per-proc contracts (NO module
   invariant — the sequencer shape; per-proc `preserves(ix)` only where verifiable; ISR
   `clobbers(ix,iy) preserves(af,bc,de,hl)`). The 4 INBOUND trust conversions stated
   explicitly (sfx's SndDrv_SetBank/Snd_RouteClassFlags + sequencer's Snd_StartSample/
   Snd_DacLookup — all conservative subsets of the real defs). 4 OUTBOUND windowed-oracle
   externs (Sequencer_Frame/StopAll, Sfx_StopAll, SfxDispatch). Literal `rept N/nop` pads
   + the three `ensure(cycles(...))` verifying 195/195/194. di/ei + de=$4001 + $2A as C3
   prose in the module header.
4. **Dual-shape windowed oracle** (`sound_driver_port.rs`) — ONE window definition
   ($0000..$0565, identical both shapes) but still TWO shape gates (plain + `-D __DEBUG__`
   twin), mirroring `sound_sequencer_port.rs`. t24 positive control + one-symbol doctor
   negative. Byte movement ZERO is existential — STOP-not-absorb on any drift.
5. **Contract set + t24 controls + checkpoint (a)** packet with the full evidence block.
