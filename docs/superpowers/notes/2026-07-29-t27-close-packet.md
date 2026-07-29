# 2026-07-29 — t27 close packet (Z80 ladder rung 0+1 — the satellite ports)

Status: **CLOSE PACKET** — merge gate open (overseer). Two porters, direct-dispatch (Fable),
context valve invoked once (porter 1 → porter 2 between lane A and lanes B/C). This packet closes
the tranche for the sequential merges.

## 0. Bars (overseer-countersigned at checkpoints a + b)
- Branch `port-tranche27` both repos, off aeon `6e25b6e` / sigil `f43d5fc`.
- Canonical UNCHANGED end to end: plain **c51342d0/421041** (`s4.bin`) · debug **992d9e7d/429102**
  (`s4.debug.bin`). Verified by both porters' own dual rebuilds AND the overseer's own rebuilds.
- Strict-paired **2675/0** (2 ignored) with `AEON_DIR` at the aeon worktree (2654 baseline + 7
  phase-1(a) + 4 phase-1(c) + 3 lane-A + 3 lane-B + 4 lane-C).
- **EXPECTED BYTE MOVEMENT: ZERO at every commit — HELD.** Nothing is wired into any build: the
  `.asm` twins stay canonical, and the `.emp` files are proven by oracles, not placed. The
  scale-(2) whole-ROM placement was STOPped, so no `.emp` ever entered a build.

## 1. What landed

**Phase 1 — three T1-deferred sigil capabilities (porter 1; each failing-test-first, byte-neutral):**
- sigil `1c7336a` — **(a) Z80 expression symbolic imm16 defer** (`ld rr, Label±k` / `jp`/`call`
  route the symbol-bearing expr through the 68k `#(Label+1)` deferral → `Value16Le` carrying the
  residual Expr). 7 TDD tests.
- sigil `c960210` — **(b) end-of-code label**: OPTIONS + CHOICE, no code change. A trailing LOCAL
  label inside a proc body works today (the 68k `.code_end` precedent); a global-label-in-body is a
  parse error (parser surgery, deferred). CHOSE the trailing-local spelling.
- sigil `64c2e12` — **(c) resident-Z80-address 16-bit value cell**: split `fixup_kind(Z80,_,false)`
  by width — a width-2 Z80-local `dc.w <resident-label>` reuses `Value16Le`; the width-4 pointer
  keeps `[cross-cpu.unwindowed-pointer]`; a >$FFFF address is a link-time `[value.out-of-range]`.
  4 TDD tests (fragment + link-LE + 68k-out-of-range negative + width-4 guard-survives).

**Lane A — z80_init.emp (porter 1) — the FIRST Z80 CODE port + first live `module (cpu: z80)`:**
- aeon `ee9b51a` + sigil `5c59803` — off-canonical `z80_init_port` oracle (3 tests; the game_debug
  template for a Z80 phase-0 section), the `(cpu: z80)` corpus-contract SKIP (`module_is_z80`), kill
  row 55, ledger. Canonically empty (both shipped shapes have sound ENABLED).

**clobbers() fix (porter 2, from the overseer's lane-A read):**
- aeon `2baf367` — z80_init.emp OMITS the false empty `clobbers()` on `Z80_IdleProgram`. Empty
  `clobbers()` = the "verified: touches nothing" LICENSE, a false claim for a code proc that
  clobbers nearly every register; `clobbers=None` ("no contract declared", legal) is honest until
  the rung-2 Z80 model. Safe because `(cpu: z80)` modules are skipped by corpus-contract analysis.
  Byte-neutral (canonically-empty, not wired).

**Lanes B + C — scale (1) windowed oracles (porter 2):**
- aeon `5ab1d6f` + sigil `9303372` — **lane B seq_opcode_tab.emp** (32 × `dc.w <resident-label>`).
  Byte-identical 64 B vs the AS twin; P1 (Z80 LE data) + P2 (banked link values); t24 positive
  control (one doctored handler address diverges) + both-sides-equal. Kill row 56. 3 tests.
- aeon `bc7415f` + sigil `3c5b003` — **lane C dac_sample_tab.emp** (10 × 9-byte descriptor).
  Byte-identical 90 B; P1 (Z80 LE data for COMPTIME cells) + P5 (the `if/fatal` size guard →
  `ensure(10*9 == extern("DAC_SAMPLE_COUNT") * extern("DacSample_len"))` defers to a `LinkAssert`,
  FIRES on a doctored count=11, passes undoctored). Kill row 57. 4 tests.

**Scale (2) — STOPped (porter 2):**
- sigil `c3426ac` — the whole-ROM CANONICAL banked-head placement seam analyzed as a discrete step
  (step-0 note §10) and STOPped per STOP-if-no-clean-seam. Four blockers (see §7).

**Loop + panel + step-6 (porter 2):** sigil `685e338` (pass-1 retrospect), `7cb9382` + aeon
`c628cfb` (panel adjudication), sigil `7ed121c` (step-6 sweeps).

## 2. Byte-delta table — ALL ZEROS canonically
| Commit | Repo | Byte delta |
|---|---|---|
| ee9b51a · 2baf367 · 5ab1d6f · bc7415f · c628cfb | aeon | 0 (no `.emp` wired into any build; comment/oracle-only) |
| 9333381 · 1c7336a · c960210 · 64c2e12 · 5c59803 · 8012511 · 9303372 · 3c5b003 · c3426ac · 685e338 · 7cb9382 · 7ed121c | sigil | 0 (frontend capabilities + tests + notes) |

Dual build **c51342d0/421041 · 992d9e7d/429102** unchanged at every step (both porters + overseer verified).

## 3. Per-pass findings (standing format: step-3 vs step-5, + neither-bucket headlines)

### Loop pass 1 (the only pass — dry after, panel confirmed)
**Step 3 (retrospect):**
- 3(a) LANGUAGE ASKS — **NEW, formalized:** a comptime SECTION-LENGTH / table-span primitive (both
  tables' AS `(End-Start) <> …` span guards have no comptime `.emp` equivalent; demand 2 files).
  **NEW (lane C), ledgered:** the Z80 width-1-`db`-can't-carry-a-link-symbol + `extern()`-can't-fold-
  into-`dc` composition → comptime data cells need a comptime source. **CARRIED (not this tranche):**
  the leftmost-local-label parser bug (porter-1's catch), `sizeof`-on-label, global-label-in-body,
  the generated-file row.
- 3(b) READS-WRONG — nothing new at retrospect; C3 later verified the headers 10/10 accurate; C2
  found ONE over-claim in the size-guard comment (fixed byte-neutral).
- 3(c) — kill rows 56/57 born at lane B/C landing; seam-STOP row cross-referenced (bidirectional)
  to the generated-file row.

**Step 5 (optimize):** NOTHING — everything in scope is cold data (two tables) + the no-sound idle
path (a comment-only change); C1 (cycle/perf) inactive-recorded (no hot path). No changes, logged.

**Neither-bucket headlines (pass 1):**
- Step-4 CONSTRUCT (LOG-ONLY per LEAN): both tables deferred — lane B (dense absolute Z80 pointers,
  `offsets`/`table` don't fit, `vectors.emp` precedent = raw `dc.w` proc is right); lane C (the
  `act_descriptor.emp` struct-array form — `DacSample` struct + `[DacSample;10]` + `dac()` — logged
  after B1 surfaced it, deferred as FIRST-OF-KIND Z80 struct-typed data = a capability step).

### Panel (A1 + B1 + C2 + C3, synchronous read-only; overseer-activated on the header claims)
- **A1 (cold reader):** clean; one comment-tier (dac_sample_tab dropped the mailbox rationale
  clause) → TAKEN.
- **C3 (hardware-contract):** **10/10 header claims VERIFIED** against the resident tree (`.coord`
  indexed reader, no-code-through-window, both bank guarantees, replicate-per-bank, the $E2 mid-frame
  B2 stash-only rule, two-bank sample split, both read sites, the DacSample layout, Snd_DacLookup
  math). One comment-parity drop (same mailbox clause) → TAKEN.
- **C2 (correctness hazard):** no hazard; byte gates non-vacuous, positive controls valid, symbol
  drift compile-caught, the `ensure` confirmed enforced in the REAL CLI link path (not test-only).
  One doc nit (the size-guard "Mirror" comment over-claims — it mirrors only the RHS constant-
  coherence, not the emitted span) → TAKEN (comment sharpened).
- **B1 (corpus-pattern):** seq_opcode_tab judgment HOLDS (`vectors.emp`); the dac struct-array
  construct was un-evaluated in the log → adjudicated code-tier / first-of-kind-Z80 / LOG-ONLY per
  the step-4 ruling → NAMED in the log (the takeable fallback); adoption deferred to the scale-2 seam.

All four panel fixes byte-neutral (comments/notes); a re-panel returns nothing new. **Panel went 4/4
useful** (two lenses independently caught the same dropped clause; C2 caught a real over-claim; B1
caught the one missed construct evaluation).

## 4. Step-6 corpus sweeps (overseer-ordered)
1. **`(cpu: z80)` module census** — exactly THREE modules (z80_init, seq_opcode_tab, dac_sample_tab);
   no z80 *section* lives outside a z80-declared module, so the module-keyed corpus-contract skip
   (`module_is_z80`) covers exactly this set and nothing else. ✓
2. **Config-shape/canonically-empty DEMAND-3 family** — the off-canonical whole-ROM mixed-placement
   machinery ledger row now states ALL THREE members: game_debug (hotkeys shape), sound_debug
   (SOUND_DBG_MIRROR, kill row 42), z80_init (no-sound, kill row 55); one machinery serves all three.
   Distinct from the canonical banked-head seam.
3. **House-spelling feed-forward** (campaign-port-loop.md step-2 item 5, one commit `7ed121c`): (a)
   trailing-local `.code_end` end-of-code label; (b) OMIT the contract (never empty `clobbers()`) on
   a CPU with no contract model, pure-data procs keep the honest empty form; (c) the commuted
   `<const> + .local` immediate spelling.
4. **Leftmost-local-label soundness census** — **ZERO live immediate-expression instances**
   corpus-wide (68k included). The one adjacent leading-`.` site (`animate.emp:140` pc-relative disp
   `jmp .cc_table-4(pc,d0.w)`) is a DIFFERENT operand class, byte-proven correct by `animate_port` →
   the bug is specific to the IMMEDIATE / fixup path (scope now sharpened). Bug stays latent (a hit
   is a loud dangling-symbol link failure); z80_init uses the commuted workaround.
5. **Value16Le resident-cell census** — the `dw <resident-label>` cell class has exactly TWO more
   demand sites, BOTH in the GENERATED `sound_tables_z80.asm` (`PsgVolEnv_Ptrs` 11 + `FmVolEnv_Ptrs`
   3); ZERO in the five resident CODE files (SeqOpcodeTable, already extracted, was the only one).
   Owned by the generated-file/DSL row — converges a THIRD time with it. Enumerated, not ported.

## 5. Corrections list (designer/overseer/brief errors caught by porters — tree wins)
- **(a) T1 design-note over-claims (designer/overseer errors, caught by PORTER 1's probes):** T1
  note §1.1/§4 claimed `ld de/hl, Label±k` was a handled, demanded, wired form — the IMPLEMENTATION
  handled only the BARE `ld hl, Label`; two of z80_init's three imm16 lines (`±k` exprs) did not
  lower (fixed by phase-1(a)). T1 note §1.2 (brief §1 lane B) claimed seq_opcode_tab "demands
  NOTHING new … rides `Cell::Expr{le:true}`/`Value16Le`" — the resident-label→`Value16Le` routing
  was UNIMPLEMENTED and self-documented "defer to T6" (fixed by phase-1(c)). Both surfaced by
  porter-1's throwaway probes at step 0, re-scoping the tranche (R1: capabilities first).
- **(b) Brief §1 lane-C "SND_* consts … as equ carriers" (brief error, TREE WON, PORTER 2):** a
  width-1 `db` cannot carry a link symbol in a Z80 section (the `(Z80,1,false)`
  `[cross-cpu.unwindowed-pointer]` guard), and `extern()` returns a `LinkExpr` that `dc` rejects.
  The SND_* are genuinely COMPTIME constants (the AS twin folds them), so they port as barewords
  resolved from `-D` defines, NOT equ carriers. Flagged + ledgered; the size-guard symbols
  (DacSample_len/DAC_SAMPLE_COUNT) DO use equ carriers (the extern-based guard).
- **(c) Dispatch stale note paths (PORTER 2):** the dispatch's step-0-note path pointed at the sigil
  MAIN checkout (`/home/volence/sonic_hacks/sigil/docs/...`); the step-0 note (and the current
  port-loop / kill-list) exist only in the WORKTREE. Read the worktree copies (tree wins). No content
  error in the notes themselves — a path/staleness catch.

## 6. Porter catches credited
- **Porter 1:** the two T1 over-claims (§5a) via step-0 probes; the R1 re-scope (capabilities as a
  demanded-feature sub-tranche); the leftmost-local-label parser bug (a real CPU-agnostic correctness
  catch); the `(cpu: z80)` corpus-contract skip; the end-of-code-label options analysis.
- **Porter 2:** the clobbers() honesty fix; the lane-C SND_* comptime-source finding (§5b) + its
  scale-2 implication; lanes B/C scale-(1) oracles with P1/P2/P5 on the real files; the scale-(2)
  four-blocker STOP analysis; the panel adjudication (4/4 useful); the five step-6 censuses.

## 7. Kill-list + ledger state
- **Kill row 55** (z80_init twin, lane A) — off-canonical `z80_init_port` oracle-gated; kill = the
  DEMAND-3 whole-ROM off-canonical no-sound machinery, or Spec 5.
- **Kill row 56** (seq_opcode_tab twin, lane B) — scale-(1) `seq_opcode_tab_port` oracle-gated, no
  build gate yet; kill = the whole-ROM banked-head seam (scale 2), or Spec 5.
- **Kill row 57** (dac_sample_tab twin, lane C) — scale-(1) `dac_sample_tab_port` oracle-gated; kill
  = the seam + a comptime SND_* source, or Spec 5.
- **Ledger:** ~825-831 (Z80 LE data unprobed) CLOSED at file scale (P1/P2 lane B, P1/P5 lane C). NEW
  rows: the width-1-`db`/comptime-source gap, the section-length primitive ask, the scale-2 seam STOP
  (+ its 4 blockers), the two censuses (leftmost-local-label = 0, Value16Le = 2 generated-file sites),
  the struct-array construct log. Three convergences on the generated-file/DSL row (lane C source,
  seam blocker 1, Value16Le demand).

## 8. Named next Z80 steps (for the roadmap marker — overseer-owned)
1. **The whole-ROM CANONICAL banked-head seam sub-tranche** (lanes B/C scale 2) — its own step-0
   seam design + dual-build byte check; lead blocker = a comptime SND_* source, which is the
   generated-file/DSL toolchain step (solve that first). Would also unblock the DacSample struct-array
   construct adoption (first-of-kind Z80 struct-typed data) at the same seam.
2. **The DEMAND-3 off-canonical whole-ROM mixed-placement machinery** — one build serves game_debug +
   sound_debug + z80_init (all canonically-empty, all region-oracle-proven today).
3. **Rung 2** (the recon ladder): contracts + the jr→jp ladder (psg then fm), the module-`invariant`
   design call, and the banked RUNG-2 TEST OBLIGATION from the T1 note §0 (the `Value::Z80Reg`-in-a-
   68k-section source-level splice test when register params land).
