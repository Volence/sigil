# Contract-grammar v2 — G2 packet

**2026-07-17, Opus.** G2 of the diagnostics-tier build (spec
`2026-07-17-contract-grammar-v2-design.md` §10 row 2): the §6 flag-result
must-use net — `out(carry:)` grammar + `[call.flag-result-unused]` + `@discards`
+ the conditional-register-result sibling — plus the byte-neutral aeon retrofit.
**G1+G2 together are the pass-3 gate; with this packet's merge, the pass-3 gate
is ARMED.** This packet is the merge checkpoint for Volence's gate.

Branches (isolated worktrees, byte-neutral throughout):
`sigil feat/contract-grammar-g2` (8 commits) · `aeon feat/contract-grammar-g2`
(1 commit).

## Gates (artifacts, not adjectives)

- **Paired strict** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon-g2> cargo test
  --workspace` = **204 suites / 2338 / 0** (baseline 202/2305; +33 = the new
  §6 tests). Failures-first: 0. 1 ignored (pre-existing).
- **Byte gates both shapes**: aeon ROMs rebuilt post-retrofit are
  byte-IDENTICAL to canonical — plain **8b71f0c5 / 453519**, debug
  **217224d3 / 461540**. Every G2 change is lint/metadata (grammar + checks emit
  nothing; the retrofit is contract text). No re-pin, no provenance change.
- **G1 closure pin UNCHANGED**: `corpus_closure_residue_is_the_g3_handoff` still
  passes — the flag retrofit is closure-neutral (a flag is not a register-file
  member, so §1's transitive clobber math is untouched; the exact 6-row G3
  residue holds).
- **G2 corpus pin** (strict-gated): `corpus_flag_results_are_all_consumed` — 0
  `[call.flag-result-unused]` / `[call.result-invalid-path]` firings over the
  real corpus. The regression guard.
- **frontend-emp unit** = 1440/0 (grammar 7, @discards 2, flag_check 12, corpus
  wiring 3), clippy clean workspace-wide.
- **TDD**: every check watched fail first — the drop-fires / bcs-consumes /
  movem-transparent / join-every-path / @discards-suppresses vectors, the two
  invalid-path vectors, and the two Fable-rider redefine vectors (addx / move-to-sr).

## What shipped

**sigil (the machinery):**
- Grammar (`ast.rs`/`parser.rs`): `out(carry: name)` flag results + `out(rN if
  cc)` conditional register results on proc / extern-proc / contract-type decls;
  `@discards(name)` trailing call attribute. `[proc.out-flag-invalid]` /
  `[proc.out-cond-invalid]` lowering validation.
- `flag_check.rs`: a lightweight CFG over the evaluated CodeBuf with real joins
  (visited-set BFS) + the carry consume/redefine tables. `check_flag_unused`
  (§6 must-use) and `check_result_invalid_path` (D2.35 sibling).
- `corpus_contracts.rs`: builds the flag/conditional-callee maps from every
  decl, captures each proc's CodeBuf + `@discards` spans, runs both checks
  post-closure. `ContractReport.flag_firings`. `emp_contracts` bin surfaces them.

**aeon (the retrofit, byte-neutral):**
- `out(carry: dropped)` on `QueueDMA_Important` / `QueueDMA_Deferrable` (externs,
  dplc.emp); `out(carry: full)` on `RingBuffer_Add` (pub proc, rings.emp).
- Every `.emp` flag-result call site CONSUMES the carry — **zero `@discards`**.

## The §11 decisions (all ratified by Fable)

1. **CFG granularity (Q1)** — a LIGHTWEIGHT CFG over the emitted instruction list
   with REAL JOINS (visited-set BFS reachability of an abandon-without-consumer
   path). Never straight-line (the pre-registered requirement, the stale-1030
   trap). Loops terminate on the visited set.
2. **Where it runs (Q2)** — the whole-corpus frontend walk, post-closure. The
   check needs cross-module contract knowledge (RingBuffer_Add's `out(carry:)`
   lives in a different module than its entity_window caller). Reuses the real
   evaluated CodeBufs (zero drift) + `instr_written_regs`.
3. **@discards attachment (Q3)** — trailing-attribute-on-call
   (`jbsr Queue @discards(dropped)`). Matched to the call by source span
   (AST InstrLine.span == CodeBuf Instr.span for direct calls).

## The Fable gate (checkpoint → rider → retrofit)

The pre-retrofit checkpoint (`…-g2-checkpoint.md`) presented every call site +
consume/discard ruling; Fable ratified the design and rode ONE correctness fix
BEFORE the retrofit:

- **`writes_carry` rider (dd96742):** the ADDX-class (`addx/subx/negx/abcd/sbcd/
  roxl/roxr`) read the EXTEND flag X, not the callee's carry C, and CLOBBER C —
  so for a carry result they are REDEFINES, not consumers. They were wrongly in
  `consumes_carry` (checked first → pruned), so an `addx` between a call and its
  `bcs` would end the real window while the check thought it open. Moved to
  `writes_carry`; `consumes_carry` is now ONLY the carry-testing branches/sets.
  Added `writes_ccr_operand` (a CCR/SR destination writes carry directly —
  move/andi/ori/eori to ccr/sr). Recorded that `btst/bset/bclr/bchg` are
  DELIBERATELY transparent (write only Z, never C). Two negative vectors added.
  Corpus dry-run under the stricter set: still 0 firings.

The **modeling stance** Fable ratified: `writes_carry` is an ALLOWLIST; an
unrecognized mnemonic is CC-TRANSPARENT (false-negative-leaning — never a
spurious fire on an unmodeled instruction). This is the correct polarity for an
error-tier check, and the dplc `movem.l (sp)+` between the call and its `bcs`
proves the need (movem preserves CCR).

## Every flag-result call site + ruling (the adjudication)

| # | Call site | Callee | Consumer (every path) | Ruling |
|---|---|---|---|---|
| 1 | dplc.emp:75 `jbsr {queue}` (Perform_DPLC) | QueueDMA_Important | `bcs .done` after transparent `movem` | CONSUME |
| 2 | dplc.emp:75 `jbsr {queue}` (Perform_DPLC_Deferrable) | QueueDMA_Deferrable | same template, `bcs .done` | CONSUME |
| 3 | entity_window.emp:1005 `jbsr RingBuffer_Add` | RingBuffer_Add | `bcs .gated` (line 1006) | CONSUME |

Zero `@discards` — the best retrofit result: the mechanism lands with no opt-outs.

## Findings — the pass-3 / step-5 breakdown + the neither-bucket

- **Step-3 (language) finding — a real classification bug the checkpoint
  surfaced:** the spec's "ADDX-class consumer" language is correct for an
  `out(extend:)` result but WRONG for a carry result; Fable's rider corrected it
  in the implementation and the spec's intent is now precisely bounded (carry
  results are discharged only by a carry-reading branch). The check's promise is
  now honest, not just today's-corpus-clean.
- **Step-5 (engine) finding — none this tranche.** G2 is pure diagnostics; it
  changes no engine bytes. The dead-save worklist that would drive engine work
  is G3's (`[proc.dead-save]`, D1d), not G2's.
- **Neither-bucket:**
  - **The `.asm` hand-off (cite the concurrent parcel):** the aeon
    `fix/silent-drop-class` worktree fixing the `.asm`-tier silent-drop bugs
    (`buffers.asm` Palette_Dirty/Sprite_Table_Dirty, `load_art`) is this check's
    evidence base made real. When the sound_api/dplc-style `.emp` ports
    eventually absorb `buffers.asm`, those sites arrive PRE-CONTRACTED
    (`out(carry:)`), and `[call.flag-result-unused]` takes over from the s4lint
    W021 approximation automatically. No file overlap this week; cite both.
  - **The corpus proved the mechanism is currently a GUARD, not a fixer:** zero
    firings means the `.emp` side has no live dropped-carry bug today (dplc and
    entity_window both consume). The value is forward — it makes the bug class
    impossible for every future `out(carry:)` caller.

## Ledgered limitations (forward machinery / known gaps)

- **`@discards` inside a comptime-fn template body is not seen** (AST-body walk,
  same limitation the closure carries for indirect sites). No corpus call site
  discards, so inert today; log if a templated discard is ever needed.
- **Conditional register results** (`out(rN if cc)` + `[call.result-invalid-path]`)
  are forward machinery — built + TDD'd against synthetic vectors, ZERO corpus
  sites declare one (the G1 subcontract-check precedent).
- **`movea`-vs-`move` spelling:** `move` is a carry-writer, `movea` is
  transparent; the evaluator spells them distinctly. A `move`-from-sr (a reader)
  is over-conservatively a writer — harmless (would only close a window early,
  and none sits in a corpus window). Noted for a future reader.
- **Tail transfer to an external symbol DEFERS** (the flag flows out of the
  proc; local analysis cannot judge it). A tail-callee that drops the flag is not
  caught locally — accepted, no corpus instance.

## Next

**Pass-3 (object/render contract surgery) is now unblocked** once this packet
merges — G1's transitive-clobber closure + `[call.live-clobbered]`-safe hoisting
substrate (G4) and G2's flag-result net together are what the adjudication named
as the gate. G3 (verified `preserves` §5 + `[proc.dead-save]` D1d) emits pass-3's
dead-save worklist and flips the G1 firing check WARN→ERROR; G4 adds
`[call.input-undefined]`/`[call.live-clobbered]`. Order G3→G4 swappable per the
spec if pass-3 wants dead-saves later.
