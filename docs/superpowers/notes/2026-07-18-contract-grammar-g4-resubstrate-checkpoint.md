# Contract-grammar v2 — G4 RE-RUN CHECKPOINT (post-substrate) (→ Fable, via Volence)

**2026-07-18, Opus.** The substrate parcel is merged to master (sigil c817250 /
aeon 1506822). This checkpoint re-runs D1b/D1c over the now-COMPLETE substrate
and hands you the trustworthy firing lists + per-firing adjudication, **before
any retrofit**, per the ruling. Terrain-mapping, not prediction-confirmation:
there is no census prediction for input coverage, so this maps what the resolved
corpus actually shows.

Branch: sigil `feat/contract-grammar-g4` — merge commit `77bb770` (origin/master
→ g4; 2 conflicts, both additive: emp_contracts.rs print sections + the
ContractReport struct fields; resolved keep-both, compiles, all pins green).

## Gate (artifacts)

- **Merged g4 strict, frontend-emp + cli, AEON_DIR set: 1805 / 0.** Every
  G1/G2/G3 pin green under the substrate (closure residue 0, flag 0, dead-saves,
  §5 preserves, the `corpus_has_zero_dropped_instructions`=0 + residue-error
  pins). D1b/D1c assert nothing (WARN). clippy clean.
- Corpus: 34 files, 131 procs (5 externs), 5 contract-types. **Dropped
  instructions: 0** (the substrate precondition holds on the g4 tree too).

## The trustworthy lists

| Check | Pre-substrate (polluted) | Post-substrate (trustworthy) |
|---|---|---|
| **D1b** `[call.input-undefined]` | 1 | **0** |
| **D1c** `[call.live-clobbered]` | 19 | **12** |
| **D1d** dead-saves (worklist) | 16 | **16** (identical to substrate re-issue) |

**D1b = 0 is DORMANT, not "inputs are clean."** D1b only checks the 7
declared-param procs + 5 externs today; the ~119 empty-param procs are
unchecked until `In:`→params lands. The single pre-substrate D1b firing (`a4`
into `S4LZ_DecompressDict`) was a drop artifact (its `movea.l
Sec.sec_block_dict(a1), a4` definer dropped) and is now correctly gone. **The
real input terrain is invisible until the retrofit** — that is exactly what the
retrofit + ERROR flip exists to expose.

## What the substrate fix changed in D1c (19 → 12)

- **−7** `Render_Sprites → Emit_ObjectPieces` (a3×3, d0×2, d1×2): all were
  `Sst.field(a0)` drop artifacts in sprites.emp (32 field accesses, 1 surviving
  pre-fix). GONE, as predicted.
- **−1** `DespawnObjects → DeleteObject d1`: a drop artifact in the *caller's*
  stream (DeleteObject genuinely `clobbers(d0-d1)`, no d1 save/restore). GONE.
  NOTE: this retires the pre-substrate §5 guess "DeleteObject → preserves(d1)" —
  it was itself artifact-polluted; no fix is owed there. (Do not blind-apply the
  old §5 list.)
- **+1** `TrySpawnObject → Load_Object a1`: NEW — surfaced *because* the fix now
  resolves `EntityScanState.ess_rom_type_tbl_ptr(a1)` (line 1191/1194), so a1's
  pre-call definition is finally visible. Guilty-until-proven checked
  instrument-side first (§ adjudication row 4): the instrument is correct.

## Per-firing adjudication (all 12)

Every one resolves to a **callee contract-tightening fix** (declare the real
`out`/`preserves`). **Zero genuine missing params. Zero surviving instrument
gaps** — the one surprising firing was investigated instrument-first and the
instrument was right.

| # | Firing(s) | Callee fact | Fix (callee) | Clears |
|---|---|---|---|---|
| 1 | 4× `{DespawnObjects,DespawnRings,RingCollision,Killed_MarkObject} → EntryForSection` **d0** | returns section index in d0 (`move.w d1,d0; rts`), declares `clobbers(d0-d1,a0)` | **`out(d0)`** | 4 |
| 2 | 2× `{RescanObjects,ScanObjectsRight} → TrySpawnObject` **d3** + 2× `{RescanY→RescanObjects, Scan→ScanObjectsRight}` **d5** | TrySpawnObject `movem.l d3/d5/…` save+restore (1158/1219), declares them `clobbers`; d5 reaches the Rescan/Scan callers ONLY transitively (neither writes d5) | **`preserves(d3,d5)`** on TrySpawnObject | 4 |
| 3 | 2× `{RescanRings,ScanRingsRight} → TrySpawnRing` **d3** | TrySpawnRing `movem.l d3-d4/…` save+restore (947/1014), declares `clobbers` | **`preserves(d3,d4)`** on TrySpawnRing | 2 |
| 4 | 1× `TrySpawnObject → Load_Object` **a1** | Load_Object header `Out: a1 = new SST pointer`, declares `clobbers(…a1…)`; callers read the produced SST | **`out(a1)`** on Load_Object | 1 |
| 5 | 1× `TileCache_FillRow → FindStagedBlock` **a1** | returns a1 (all callers rely), declares `clobbers(d3-d4/a1)` | **`out(a1)`** on FindStagedBlock | 1 |

Total cleared: 4+4+2+1+1 = **12**.

### The d5 cascade (why row 2 is one fix, not four)

D1c keys off the **computed effective** set, and the closure seeds each node from
its ACTUAL local writes (`closure.rs:155 acc.union_regs(&node.local_writes)`),
subtracting verified-preserves (`:187`) — NOT declared clobbers. RescanObjects /
ScanObjectsRight do not write d5 in their own bodies; d5 enters their effective
set solely through TrySpawnObject. So `preserves(d3,d5)` on TrySpawnObject
removes d5 from its effective → removes it from theirs → the RescanY/Scan d5
firings clear transitively. (Their declared `clobbers(d0-d5)` then over-declares
d5 harmlessly; tightening to `clobbers(d0-d4)` is an honest optional tidy, not
required to clear the firing.)

## Proposed adjudication (ruling requested)

**5 callee contract edits** (out/preserves), all byte-neutral, clear all 12 D1c
firings. This is the D1c net doing precisely its job: every firing is a real
contract mislabel (a produced result declared as scratch, or a movem-preserved
register declared as clobbered), not a caller bug and not an analysis hole.

Recommendation — the agreed sequence, on your ruling:
1. **`In:`→params retrofit sweep** (the 95 `// In:` comments across 18 files →
   real register params) — its own commit. This is what populates D1b's real
   terrain; expect NEW D1b firings here (the first true input-coverage map) and a
   second, smaller adjudication pass on whatever it surfaces.
2. **The 5 out/preserves contract fixes** (fold into the retrofit sweep or land
   as a sibling commit — your call on granularity).
3. **The ERROR flip** (D1b/D1c WARN→ERROR) as its own bisectable commit, G3's
   pattern — gated so any residual firing is a build error.

I did NOT retrofit, apply the 5 fixes, or flip, per the ground rules. The lists
above are the raw re-run; the fixes are proposals.

## Per-pass findings (step-3 vs step-5 vs neither)

- **NEITHER-BUCKET:** the +1 `Load_Object a1` firing is the cleanest possible
  demonstration that the substrate fix *revealed* terrain rather than inventing
  it — a genuine contract mislabel that was hidden behind a dropped field
  operand for the whole campaign, surfaced the instant the operand resolved.
- **Step-3 (modernize/language):** all 5 fixes are honest contract upgrades the
  D1c net makes visible (out = a produced result; preserves = a movem round-trip).
- **Step-5 (optimize):** none (byte-neutral). The trustworthy D1c list, once the
  contracts are tightened, leaves pass-3 a clean seatbelt alongside the 16-row
  dead-save worklist.
