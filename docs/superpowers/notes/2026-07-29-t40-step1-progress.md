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

## Finding 3 — BALANCED-`exx` RECOGNITION IS NOT DEMANDED BY THE DRIVER (needs adjudication)

Ruling 3 directed "wire the balanced-`exx` recognition now — demanded by this file's
honest contracts." On close inspection of the tree, it is **NOT** load-bearing for the
driver's contract set, and I have NOT built it (building an unexercised feature is
speculative generality — the adoption-over-cleverness bar). Evidence:

- Today `z80_preserves` treats `exx` CONSERVATIVELY: a lone `exx` clobbers bc/de/hl
  (`z80_preserves.rs:157-160`). The balanced-pair recognition would UPGRADE a
  balanced `exx … exx` to a no-op so `preserves(bc,de)` is credited across it.
- **That credit is only needed by a RETURNING proc that DECLARES `preserves(bc/de/hl)`
  AND routes it through an `exx` pair.** The driver has NONE: the only two exx-using
  procs are `SndDrv_Sample` and `SndDrv_TimerATick`, and BOTH are non-returning loop
  entries (0 `ret` each; Sample `jp .loop`, TimerATick `jp SndDrv_Sample.afterPoll`).
  A python scan confirms: `procs with BOTH exx and ret: NONE`.
- The driver has NO module `invariant` (Finding: the sequencer shape), so nothing is
  INHERITED onto the exx procs either. `check_z80_preserves` returns early on an empty
  checklist — the conservative exx clobber is never consulted for a preserve proof.
- Therefore the conservative treatment blocks nothing and produces no spurious
  `[proc.preserves-unverifiable]`. The byte oracle is likewise unaffected (exx encodes
  to `D9` regardless).

**Recommendation:** DEFER the balanced-`exx` recognition to a file that exercises it
(the recon named it for the shadow model generally; no ported/pending file DECLARES a
preserve across an exx pair yet — the seam sub-tranche or a future streaming refactor is
its natural home). The rung-2 §4.3 machinery stays represented-not-wired, exactly as it
has been, until a real preserve-across-exx demand appears. Overseer: confirm the defer,
or direct me to wire it as forward-compat despite no current demand (I will if ruled,
with the failing-first unbalanced-bail / balanced-credit tests — but flagging that it
adds a lint path no current contract needs).

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

1. **Wire `cycles()` into `ensure` + a per-proc pass.** `ensure(cycles(L1,L2) == N)`
   deferred to `check_z80_cycles(proc, &buf, asserts, diags)` beside `check_z80_preserves`
   (`lower/proc.rs:148`); the pass calls the LANDED `label_span` + `span_cost`, fires
   `[cycles.mismatch]` / the two bails. SCOPE (measured this pass): the deferral needs a
   new residual `Value::CycleSpan`-class value (mirroring `LinkExpr`) that `==` folds to a
   deferred CycleAssert, a `take_cycle_asserts()` drain, and a NEW return element threaded
   through `eval_proc_body` (which today returns only `(buf, diags, counter)` — no
   assert drain) into `proc.rs`. A `Value` variant touches ~18 files' matches (mostly
   non-exhaustive `if let`, but must be swept). Invasive but bounded; ripple-scoped so a
   continuation lands it cleanly. DESIGN CALL: the `ensure` lives IN the proc body (local
   `.loop`/`.exhaust` in scope). Failing-first: pad+4 mismatch, jr-in-span bail,
   off-table bail, real loop green + byte-neutral (self-gates).
2. **Balanced-`exx` recognition** — DEFERRED pending adjudication (Finding 3): not
   demanded by the driver (no returning proc crosses an exx pair with a declared
   preserve). Wire only if the overseer rules forward-compat despite no demand.
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
