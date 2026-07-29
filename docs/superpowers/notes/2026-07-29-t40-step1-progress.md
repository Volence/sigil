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

## Landed — the cycles `ensure` channel, EAGER (Ruling 2), commit `74504b8` (5 tests)

Ruling 2 said investigate the lighter channel first. Eager is not only feasible but
SUPERIOR, so it is CHOSEN (no `Value::CycleSpan` residual, no `eval_proc_body`
threading, no `proc.rs` change, no ~18-file match sweep):
- Z80 encodings are fixed-width (no relaxation) → the partial CodeBuf snapshot is EXACT
  for any span textually preceding the ensure (the annotation's natural place).
- A local-label arg (`.loop`) already evaluates to `Value::Label(owner-mangled)` — the
  SAME name the buf's `CodeItem::Label` carries — so span carving is a direct match.
- **`cycles()` returns a plain `Int` → composes via `+`/`==` FOR FREE, which the driver
  NEEDS** (see the multi-span finding below). The residual channel would have needed
  arithmetic on residuals.

Mechanism: `cycle_scope: Option<Vec<CodeItem>>` on the evaluator, snapshotted around a
body-position `ensure` (guards are rare → cheap); `cycles(L1,L2)` reads it, carves
`[L1,L2)` via `label_span`, sums via `span_cost`, returns `Int`. The `AsmStmt::Call` arm
now accepts a `Unit`-returning body guard. Failing-first (`t40_cycles.rs`): correct count
passes; doctored count fires the ensure's own message; `jr cc` in span →
`[cycles.ambiguous-branch]`; off-table op → `[cycles.unknown-op]`; `jp cc`=10 positive
control. Full strict gate **2871/0/1** (2856 + 10 z80_cycles + 5 t40_cycles).

## Finding 4 — the DAC loop's three paths are NOT three single textual spans

FILL is the one clean single span: `cycles(.loop, .exhaust) == 195` (the fall-through;
its not-taken `jp cc`s count at 10 each = exactly the driver's arithmetic). But DRAIN and
DRAINING_TAIL are **shared-prefix + separate-body SUMS**: DRAIN = prefix(`.loop`..the
`jp nz,.drain`, =109) + drain-body(`.drain`..return, =86) = 195; DRAINING = prefix +
draining-body. Expressing them needs `cycles(A,B) + cycles(C,D) == N` composition (the
eager `Int` channel does this natively) and MAY need one or two byte-neutral cut labels in
the `.emp` at the prefix/body boundaries (labels emit no bytes → byte-oracle-safe; a
`.emp`-only label is fine since twin lockstep is byte-level, not text-level). This is the
concrete shape the transcription's three ensures take — recorded so the port writes them
right the first time.

## REMAINING to checkpoint (a) — precise continuation map

1. ~~Wire `cycles()` into `ensure`~~ — DONE (eager channel, commit `74504b8`, 5 tests).
2. ~~Balanced-`exx` recognition~~ — DEFERRED (Ruling 1; gap-ledger row [t40 step-1]).
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

---

## CHECKPOINT (a) — DONE (2026-07-29). Deliverables committed:
- aeon `port-tranche40` `60f15a9`: `engine/sound/z80_sound_driver.emp` (the 1381-B
  transcription). sigil `port-tranche40` `56e22f4`: `crates/sigil-cli/tests/
  z80_sound_driver_port.rs` (7 tests). No `.asm` touched.
- **Windowed gates BOTH shapes GREEN** (7/7): plain 1381 B == twin, debug 1381 B ==
  twin, both-same-size, both-shapes-DIFFER (9 bytes), 2 doctor positives (link +
  const), 1 doctor-both-equal.
- **Whole-ROM CRCs REPRODUCED EXACTLY** (byte movement ZERO): dual gate-off rebuild
  plain `4b66cace`/421041, debug `1c256b3b`/429102.
- **Full strict suite 2878/0/1** (`--no-fail-fast`) = the 2871/0/1 bar + the 7 new
  driver-port tests.
- **Cycles-ensure non-vacuity on the REAL span**: doctoring the `.drain` pad 19->20
  nops fires `DRAIN pass must equal FILL (195 T-states)` (199 != 195); reverted.
- **Contract firing arc 3 -> 0**: the honest-contract checker fired at first compile —
  `[proc.out-clobbers-overlap]` on `Snd_DacLookup` (h,l) and `Snd_RouteClassFlags` (a):
  a register is either an `out` result or clobbered scratch, not both. Driven to zero by
  dropping the out registers from the clobbers clause (out(hl,carry) preserves(bc,ix,iy);
  out(a) preserves(bc,de,hl,ix,iy)). Preserve-checker non-vacuity separately proven:
  injecting `ld c,0` into `Snd_RouteClassFlags` fires `[proc.preserves-unverifiable]
  declares preserves(c) but c is written and not restored`; reverted.
- **4 inbound trust conversions VERIFIED + stated** at each def site (all conservative
  subsets of the real closure; sfx SndDrv_SetBank/RouteClassFlags + sequencer
  Snd_StartSample/DacLookup). 4 outbound externs declared (Sequencer_Frame preserves(iy);
  StopAll/StopAll/Dispatch bare).

## CORRECTION (tree wins) — the driver window is NOT byte-identical across shapes
The step-0 §2 premise "IDENTICAL in BOTH shapes / the first Z80 port where plain and
debug windows are the SAME BYTES" is FALSE at the byte level. The window is the same
SIZE (1381 B) and POSITION ($0000) both shapes (the driver's own code has zero
`__DEBUG__` content), but its BYTES differ: the driver `call`s three callees that live
AFTER the sequencer's +$7E debug growth — `Sequencer_StopAll` ($CB2->$D30),
`Sfx_StopAll` ($11AA->$1228), `SfxDispatch` ($E5D->$EDB) — so five call sites (2 + 2 +
1) emit different operand bytes plain vs debug (9 bytes total; SfxDispatch shares its
$0E high byte). `Sequencer_Frame` ($0565, the sequencer base) is the ONE shape-invariant
target. Verified in s4.lst vs s4.debug.lst. The oracle therefore feeds PER-SHAPE link
addresses and gates both shapes as genuinely-different byte images (a
`plain_and_debug_shapes_differ` test asserts the 9-byte delta), NOT "same bytes". Also:
step-0 §1.1's row `Z80_Sound_End = $0565` is wrong — $0565 is the sequencer base
(Sequencer_Frame); the true `Z80_Sound_End` is $1BFA (the whole-blob end). Neither
correction changes the window boundary ($0000..$0565) or the deliverables.

---

## STEP 2 (modernize) — DONE. The pad_to_cycles derived-pad capability (the rung-4 payoff)
- **Built `pad_to_cycles(target, measured)`** (sigil `4a88c4e`): a comptime pad emitter
  returning `(target - measured)/4` real `nop` instructions, where `measured` is DERIVED
  from `cycles(...)` spans + the fixed pre-pad/trailing cost. The eager `cycle_scope` is
  extended to the pad statement-call (it reads `cycles()` exactly as an `ensure` does).
  Loud errors on a negative pad (`measured exceeds target`) or a non-multiple-of-4 pad
  (nop granularity). 4 TDD tests (emit-count via a following `cycles()` span, derives-
  from-a-span, over-budget, non-mult-of-4).
- **Applied to the two DAC timing pads** (aeon `c737fd5`): the literal `rept 19/nop`
  (DRAIN) → `pad_to_cycles(195, cycles(.loop,.fill_body) + 10)` = 19 nops; the literal
  `rept 21/nop` (DRAINING) → `pad_to_cycles(194, cycles(.loop,.dma_check) +
  cycles(.draining,.draining_pad) + 10)` = 21 nops (one `.draining_pad` cut label added).
- **DERIVED == HAND, byte movement ZERO** (STOP-not-absorb honored): the windowed byte
  gate proves the derived pads are byte-IDENTICAL to the literal `rept 19/21` in BOTH
  shapes. A count mismatch would have failed the gate = a STOP finding; it did not.
- The three cycle-balance ensures still verify FILL 195 / DRAIN 195 / DRAINING 194, now
  cross-checking the derived pads (complementary: the pad derivation uses the prefix span
  + a literal trailing 10; the ensure uses the full body span — a trailing-jp edit would
  be caught by the ensure).
- House-format/step-2: the file is conformant from fresh transcription (panel A confirms
  clause-order, brace-indent, compound-displacement parenthesization, present-tense prose).
- **Full strict suite 2882/0/1** (`--no-fail-fast`) = 2878 + the 4 new pad_to_cycles tests.
  Frozen behavior: no crate regression. ROM CRCs unchanged (the `.asm` is untouched; the
  oracle proves `.emp`==`.asm` bytes).

## THE DRY PANEL (A1 · B1 · C1 · C2 · C3) — 3 fresh read-only lens subagents
- **A (ceremony / house-format) → NOTHING NEW.** All 5 dimensions clean vs the sequencer/
  sfx reference: contract clause-order (`out → [clobbers] → preserves`, `falls_into`
  trailing) on all 20 procs; brace-indent; every `+k` displacement parenthesized; zero
  history-narration comments; module-header prose present-tense/complete.
- **B (corpus-pattern) → NOTHING NEW.** extern-decl sparse style matches sequencer;
  `export .afterPoll:` / `SndDrv_Sample.afterPoll` mirrors the sequencer `export .fetch:`
  idiom; `falls_into`, module/section shape, and all idiom spellings have corpus
  precedent. (`pad_to_cycles` is the one new construct — no precedent to diverge from.)
- **C1 (T-state) → CONFIRMS, nothing new.** Walked the FILL span instruction-by-
  instruction (22+30+30+27+44+22+20 = 195) against z80_cycles.rs and the .asm header;
  DRAIN 109+86=195, DRAINING 82+112=194; both pad derivations (19, 21) re-derived. Table
  is the documented driver-demand SUPERSET (a few unused ops present), not a gap — every
  op in the three spans is covered, no reachable `[cycles.unknown-op]`.
- **C2 (contracts) → CONFIRMS, nothing new.** Snd_DacLookup decl `preserves(ix)` ⊆ real
  `preserves(bc,ix,iy)` (safe subset); RouteClassFlags `out(a)` honest; SndDrv_Sample's
  clobbers-all/no-preserves correct (jp into TimerATick clobbers iy). The out-clobbers-
  overlap resolution (dropping af/de from the clobbers clause, implicit scratch) is sound.
- **C3 (hardware prose) → THREE byte-neutral findings, ALL FIXED (comments only):**
  (b) the header's "$2A re-parked ... the sole $4000-touching paths" OVER-CLAIMED (it
  omitted Snd_LoadSong + init + the idle loop, which also write $4000) → corrected to the
  honest fuller set (init/idle/tick/.stop/StartSample/LoadSong/Timer-A programs).
  (c) the "banked in-frame CODE is a proven crash hazard" line was imported from
  sound_sequencer.emp and has no driver-.asm backing → reframed as the corpus resident-
  code discipline (attributed to sequencer.emp) with the .asm-BACKED half (the
  DacSampleTable BANK CONTRACT) stated separately.
  (d) the .asm header's INHERENT PITCH ASYMMETRY block (DRAIN ~29 cents / DRAINING ~38
  cents, math verified correct by C) was DROPPED in transcription → restored to the .emp
  header. Byte gate re-run GREEN after all three (comments emit nothing).

## DRY DETERMINATION — the panel converged
A + B returned nothing new; C1/C2 confirmed the porter; C3's three prose findings were
byte-neutral and are RESOLVED in-place (no code/contract/byte change, no re-opened cycle).
The panel is DRY. Byte movement stayed ZERO throughout (windowed gates GREEN both shapes
after every edit; the `.asm` canonical untouched, so ROM CRCs 4b66cace/1c256b3b hold by
construction).

## CHECKPOINT (b) — the loop is dry. Evidence:
- Deliverables: aeon `z80_sound_driver.emp` (transcription + derived pads + C3 prose
  fixes); sigil `z80_sound_driver_port.rs` (7 gates) + `pad_to_cycles` capability (4
  tests) + this note + kill-list row 84.
- Windowed gates 7/7 GREEN both shapes; byte movement ZERO (derived pads byte-identical
  to the literal `rept 19/21`; C3 fixes comment-only).
- Full strict suite **2882/0/1** (`--no-fail-fast`).
- The three cycle-balance ensures verify FILL 195 / DRAIN 195 / DRAINING 194 on the real
  spans (C1 re-walked); pad_to_cycles non-vacuity proven (doctored `.drain` pad fires);
  contract firing arc 3→0 + preserve-checker non-vacuity (both from checkpoint a).
- Panel A1/B1/C1/C2/C3 all resolved → dry. Kill row 84 tracks the driver twin (seam
  sub-tranche kill). STOP here per the brief; gate (c) + the rebase (t39 merged: aeon
  23e6ca7 / sigil c6d3ec5, 68k-side, zero overlap — NOT self-rebased) + the merge are the
  overseer's.
