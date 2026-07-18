# Contract-grammar v2 — G3 CHECKPOINT (verified preserves + dead-save; pre-retrofit)

**2026-07-17, Opus.** The mandatory pre-retrofit checkpoint the brief calls for:
§5 verified `preserves` (symbolic stack tracking) + `[proc.dead-save]` (D1d) are
built, TDD-green, and run over the **real aeon corpus** — BEFORE any aeon retrofit
or the WARN→ERROR flip. Reported here for Fable's gate (via Volence).

Branch (isolated, byte-neutral): sigil `feat/contract-grammar-g3` (2 commits:
G3.1 verified preserves, G3.2 dead-save). aeon: no retrofit yet.

## What is built (sigil, all TDD, byte-neutral)

| Piece | Spec | Tests |
|---|---|---|
| `preserves.rs` — §5 symbolic-stack verified `preserves` | §5 | 13 (unit) |
| — the 6-residue-proc corpus measurement | §5 | 1 (gated) |
| `find_dead_saves` — `[proc.dead-save]` (D1d) | §6 | 6 (unit) |
| — the corpus dead-save worklist dumper | §6 | 1 (gated) |
| `analyze_corpus.dead_saves` wiring | §11 Q2 | (corpus) |

frontend-emp **79 suites green**, clippy clean. G1 residue pin + G2 flag pin
UNCHANGED (both re-run green over the retrofit-free corpus).

## §5 design (recorded)

A forward dataflow over G2's **reused** CFG (`flag_check::Cfg`/`Edge` → `pub(crate)`,
spec §11 Q1 — extended, not duplicated). Per program point: a symbolic stack of
slots (each tagged with which register's ENTRY value it holds) + per-register
entry-value bits. Saves push, restores pop-and-match, the `(sp)` peek restores
without popping. A call clobbers all registers (conservative — no local callee
contract; the closure's transitive job) but nets zero on the stack. `preserves(rN)`
holds iff every return path restores rN's entry value or never writes it. Soundness
bailouts → `Unverifiable` (error-tier for a declared preserves): bare `a7` operand
(computed sp / escape), displaced/indexed sp access (aliasing), unbalanced-stack
join. The movem entry/exit pair is the trivial fast path — D2.32 subsumed.

## CHECKPOINT RESULT 1 — which of the 6 residue rows verify

Run over the REAL corpus (`preserves_corpus` test), each proc against its residue
register:

| Proc | Reg | Result | Mechanism |
|---|---|---|---|
| AllocDynamic | a0 | **Verified** | individual-push, branch-straddled |
| Collected_ParkSlot | a0 | **Verified** | push + `(sp)` peek + `dbf` copy loop |
| Collected_UnparkSlot | a0 | **Verified** | push + `(sp)` peek + `dbf` copy loop |
| Collected_CheckRing | d1 | **Verified** | mid-body movem around Collected_FindSlot |
| Killed_CheckObject | d1 | **Verified** | mid-body movem around Collected_FindSlot |
| Load_Object | a0 | NotPreserved *locally* | **clears TRANSITIVELY** via AllocDynamic |

**Prediction held exactly.** 5 verify by local save/restore; Load_Object never
touches a0 itself — its a0 clears once AllocDynamic declares+verifies `preserves(a0)`
and the closure subtracts it (NOT local preservation). Local NotPreserved is
CORRECT — the retrofit declares `preserves` on 5 procs, never Load_Object. **Zero
soundness bailouts.**

### NEITHER-BUCKET (the finding of the phase) — a shared-CFG gap, real code found it

The FIRST real-corpus preserves run reported Collected_Park/UnparkSlot as
NotPreserved. Root cause was NOT the residue procs: the shared CFG's `branch_target`
read `ops.first()`, missing the `dbcc dN, label` two-operand form (label is the
SECOND operand), so the park/unpark `dbf` copy loops resolved as an EXTERNAL
`Defer` edge instead of a local back-edge — and the Defer path is treated as
non-preservable. Fixed: scan for the LAST `Sym` operand (correct for
`bcc`/`bra`/`jbsr`/`dbcc` alike). **G2 flag_check + both corpus pins unaffected**
(no `dbf` sits between the 3 flag call sites and their `bcs` consumers, so G2 never
exercised the shape). This bug was invisible to G2's tests, to my synthetic §5
vectors, and to the entire flag_check corpus run — surfaced only by the first
preserves pass over real code. It is the cleanest demonstration yet of why the
checkpoint runs against the real corpus BEFORE a retrofit touches anything. Pinned
by `dbcc_target_is_second_operand` (a vector that flips to NotPreserved on regression).

## CHECKPOINT RESULT 2 — the dead-save worklist (16 firings)

`[proc.dead-save]` over the real corpus (full TSV:
`2026-07-17-contract-grammar-g3-dead-save-worklist.tsv`). The verdict "callee
preserves rN" rides the closure's VERIFIED `effective` set — never raw declared
text (Fable's boundary; pass-3 cuts code on this list).

**All three review-named customers found:**
- **dplc** (~575 cy/frame-change): `Perform_DPLC` / `Perform_DPLC_Deferrable` save
  a3 across QueueDMA_Important/_Deferrable (which preserve a3).
- **load_object** (~76/spawn): `Load_Object` saves d1, d2 across AllocDynamic
  (AllocDynamic is a leaf writing only d0/a1 — d1/d2 preserved). NOT d0 (clobbered)
  and NOT a1 (out — and its slot is deliberately restored into a2, a move-through).
- **children** (44-116/child): `EntityWindow_BuildEntries` saves d6/d7/a3 across
  Collected_ClaimSlot.

**Beyond the review (10 more firings, 8 procs):** EntityWindow_DespawnRings d5/d6/d7
(RingBuffer_Remove), EntityWindow_Scan d7 (RescanY), Section_UpdateColumns a3
(Draw_TileColumn/Row), TileCache_FillColumn d7 (CopyBlockColumn),
TileCache_WarmupBelowRow d6 (DecompressBlock), and a PRECISE partial:
**Collected_CheckRing / Killed_CheckObject `movem.l d0-d1` saves only NEED d1** —
Collected_FindSlot preserves d0, so the d0 half is dead. Pass-3 narrows the movem
list rather than deleting it. (These are the SAME two procs whose d1 is a genuine
G3 residue save — the lint distinguishes the needed half from the dead half within
one movem.)

Soundness: false-negative-leaning. A save with ANY clobbering path, an
indirect/hole/⊤ callee, or a scratch use is not reported. Findings are stable
w.r.t. the pending §5 integration (no corpus site saves a0 around AllocDynamic, so
subtracting AllocDynamic's a0 adds nothing here).

## Proposed path past the checkpoint (for Fable's ruling via Volence)

1. Wire `verified_preserves_regs` → the §5 analysis (replace the D2.32 fast path;
   it subsumes it — the 6 existing movem/sr adopters keep verifying via the trivial
   fast path).
2. Retrofit the 5 honest `preserves` decls (aeon, byte-neutral contract text):
   `AllocDynamic preserves(a0)`, `Collected_ParkSlot preserves(a0)`,
   `Collected_UnparkSlot preserves(a0)`, `Collected_CheckRing preserves(d1)`,
   `Killed_CheckObject preserves(d1)`. Load_Object gets NONE (clears transitively).
3. Re-run: closure residue → **0** (all 6 rows clear: 5 by verified-preserves
   subtraction, Load_Object transitively). Prediction check — any non-zero = STOP.
4. **The flip:** closure firing check WARN→ERROR (spec §9 tier-timing ruling, zero
   residue precondition now met); convert `corpus_closure_residue_is_the_g3_handoff`
   from expect-EXACTLY-6 to expect-EMPTY — the permanent error gate.
5. Byte gates both shapes (must reproduce 8984e510/453533 · c80465dc/461554);
   paired strict from both tips; packet.

**Recommendation:** both checkpoint results match prediction (6-row table exact;
all 3 dead-save customers found + a precise partial + 8 beyond). The dbcc gap was
a real shared-CFG fix, not a residue-proc problem. Barring a Fable objection,
proceeding with the integration + retrofit + flip is safe. Pausing here for that
ruling per the brief — retrofit + flip are gated on this checkpoint.
