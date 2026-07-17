# Contract-grammar v2 — G1 packet

**2026-07-17, Opus.** G1 of the diagnostics-tier build (spec
`2026-07-17-contract-grammar-v2-design.md`): the transitive register-effect
closure (§1) + its boundary grammar (§3/§4/§8) + the full corpus retrofit. G1+G2
are the pass-3 gate. This packet is the merge checkpoint for Volence's gate.

Branches (isolated worktrees, byte-neutral throughout):
`sigil feat/contract-grammar-g1` · `aeon feat/contract-grammar-g1`.

## Gates (artifacts, not adjectives)

- **Paired strict** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon-branch> cargo test
  --workspace` = **202 suites / 2305 / 0** (baseline 198/2271; +34 = the new
  contract tests). Failures-first: 0.
- **Byte gates both shapes**: ROMs byte-IDENTICAL to master — every G1 change is
  lint/metadata (contract text emits nothing). No re-pin, no provenance change.
  (One test-coupling fix: a negative probe doctored collision.emp by exact
  string; the `clobbers()` stub retrofit changed the spelling — ROMs unaffected.)
- **frontend-emp unit** = 75 suites / 1413 / 0, clippy clean.
- **Closure residue pin** (strict-gated): `corpus_closure_residue_is_the_g3_
  handoff` — 0 holes, 0 §11-Q4 collisions, 0 unbounded indirects, exact 6-row
  residue.
- **TDD**: closure 25 tests, grammar 12, corpus-walk 10 — every production
  function watched fail first (the closure fixpoint, firing check, extern-out
  fix, preserves subtraction, subcontract relation).

## What shipped

**sigil (the machinery):**
- `closure.rs` — RegEffect lattice + monotone-union fixpoint (SCC-terminating) +
  firing check + verifiedPreserved subtraction (D2.32 fast path) + the §4
  subcontract relation.
- `corpus_contracts.rs` + `emp_contracts` bin — the whole-corpus frontend walk
  (§11 Q2) building the ProcNode map by name, reusing `proc_written_registers`
  and `check_preserves` verbatim (zero drift).
- Grammar: `extern proc` (§3), `type X = proc` contract types (§4), `as` dispatch
  bounds (§4), `@scaffolding` item attribute (§8).

**aeon (the retrofit, all byte-neutral):**
- Boundary: 5 extern decls (drift-guarded, kill-list 29-33), 5 contract types
  (ObjRoutine, AnimCallback, TouchHandler, GameState, HBlankHandler), 6 `as`
  bounds, `@scaffolding` on Plane_Buffer_Reset.
- Debt sweep: 13 census under-decls + 12 stubs + 2 transitive callers the census
  couldn't see (Rescan/Scan).

## The §11 decisions (all ratified by Fable)

1. **CFG granularity** — G1 needs none (whole-proc union); lightweight-CFG-with-
   joins pre-registered for G2/G3.
2. **Call graph lives in a whole-corpus frontend pass**, name-resolved, not
   post-link — reuses the real detectors, spans native.
3. **@discards** (G2) — trailing-attribute-on-call, pre-registered.
4. **extern proc = real symbol decl** — collision flagged (§11 Q4).

## What each pass added

**Boundary-first ⊤-collapse (the closure's correctness proof):** Fable's four
pre-registered predictions over the real corpus. (b) holes clear ✅, (c)
RunObjects d7 re-surfaces ✅, (a) ⊤s collapse — 3 of 4 as predicted, **AnimateSprite
diverged** (8 transitive rows, not direct). Closure arithmetic verified correct →
a real finding, not a bug.

**Debt sweep + residue convergence:** census-13 sweep left residue 14 (not the
predicted 5). The delta required per-proc save/restore analysis (genuine-widen vs
G3-preserve) — resolved to: widen 2 genuine transitive callers (Rescan/Scan,
verified via TrySpawnObject's spill frame), leave the rest as G3-FP. Fable ruled
Option 2 (subtract declared+verified preserves in G1) → **residue EXACTLY 6**,
Fable's precise prediction. The 6: 3 individual-push a0 + Load_Object a0 +
Collected_CheckRing/Killed_CheckObject d1 — all row-1030/G3.

### Findings — the neither-bucket (both directions)

The instrument audited its own evidence base — findings in BOTH directions:

- **Debt the census MISSED** (closure found real under-declarations local analysis
  structurally cannot):
  - 2 genuine transitive callers (EntityWindow_Rescan/ScanObjectsRight) — they
    call TrySpawnObject whose `movem` at :1158 is a spill frame, so the regs are
    real scratch that propagates.
  - +2 extern-boundary calls (QueueDMA_Important/Deferrable) — census part-(b)
    text-scan missed the `perform_dplc(QueueDMA_*)` comptime-fn-parameter calls
    (spec §3 erratum, 6054119: 5 externs not 3).
- **Census FACTS that were WRONG** (closure disproved the evidence base):
  - `preserves` has **6 adopters** (HBlank_Dispatch + 5 sound_api), not 0 — the
    "0" was the individual-push CLASS only (spec §5 erratum 8b11a9f).
  - The spec-author's AF_CALLBACK bound was over-wide (ObjRoutine); the installable
    set is EMPTY, so `AnimCallback = preserves(a0) clobbers(d0-d2,a1-a2)` is a
    design commitment, not a description (spec §4 erratum e12a4ca).
  - S4LZ_DecompressDict's spec-draft `clobbers(a3,a4)` under-read the .asm
    (d0-d3/a2-a4); the extern decl follows the .asm to the letter.

### The G3 handoff (6 rows, all inexpressible today)

| Proc/reg | Preservation the closure can't yet prove |
|---|---|
| AllocDynamic / Collected_Park / Collected_Unpark a0 | individual-push, branch-straddled (row 1030) |
| Load_Object a0 | inherits AllocDynamic's a0 |
| Collected_CheckRing / Killed_CheckObject d1 | UNDECLARED `movem.l d0-d1` around Collected_FindSlot |

G3's symbolic stack tracking extends the SAME subtraction G1 built (declared+
movem-verified) to the undeclared individual-push/movem classes → these 6 clear.
The G1 firing check ships WARN through G1/G2, flips ERROR at G3's close (spec §9
tier-timing) — the residue pin becomes the error gate then. No suppression
mechanism (per ruling).

## Still open in G1 (small, ledgered)

- **§4 subcontract-check target DETECTION** — the relation is built + TDD'd, but
  the corpus has no installable targets yet (empty AF_CALLBACK, `Touch_HandlerTable`
  is a `bra.w` proc, RAM cells can't carry a type). The typed-table / typed-
  pointer-cell grammar lands when the first target appears (gap-ledger, this date;
  the objdef-table-as-`[ObjRoutine; N]` is the strongest candidate). Per Fable,
  this is forward machinery for G1.

## Walk limitations (noted, low-impact)

- Indirect sites inside comptime-fn/template bodies aren't scanned (AST-body only)
  — the Touch dispatch (#4) is the one instance; its `as TouchHandler` is correct
  but inert until the walk descends into comptime-fns. Harmless (the 11 Touch_*
  targets are rts-stubs), and it keeps TouchResponse free of G3-residue rows.
- Debug-only calls (Debug_MusicToggle under SOUND_DEBUG_HOTKEYS) are elided in the
  plain-build closure — its extern decl exists for debug-build analysis.

## Next

G2 (`out(carry:)` must-use + `@discards` + `[call.flag-result-unused]`) — with
G1 it unblocks pass-3. The residue pin + WARN-tier firing list are the standing
regression guard; G3 flips them to ERROR and clears the 6-row handoff.

## Corrections (Fable gate)

- **Commit count:** 16 sigil + **3** aeon (a661db9 boundary, 576c283 AnimCallback,
  669c287 debt sweep) — an earlier hand-off said "4 aeon", which was wrong.
- **S4LZ §3 erratum (spec master 00dd4e6):** the shipped `extern proc
  S4LZ_DecompressDict … clobbers(d0-d3/a0/a2-a4) out(a1)` was correct; the spec
  §3 SKETCH quoted only the dict entry's "Extra:" header lines and missed the
  shared body's d0-d3/a0/a2-a3. §3 now records the shipped contracts verbatim. No
  code change — the extern decl already follows s4lz_decompress.asm to the letter.
