# Contract-grammar v2 — G1 CHECKPOINT (closure skeleton over the real corpus)

**2026-07-17, Opus.** The pre-retrofit checkpoint the brief calls for: the
transitive closure (§1) + boundary grammar (§3/§4/§8) are built and TDD-green,
and the closure runs over the **real aeon corpus** (126 procs). This is the
firing list compared against the census's predicted debt — reported here BEFORE
any aeon retrofit, per the standing rule *"if the closure surfaces firings the
census didn't predict, stop and report, don't retrofit through surprises."*

Branches (isolated, byte-neutral): sigil `feat/contract-grammar-g1`
(6 commits), aeon `feat/contract-grammar-g1` (empty so far — no retrofit yet).

## What is built (sigil, all TDD, byte-neutral)

| Piece | Spec | Tests |
|---|---|---|
| `closure.rs` — RegEffect lattice + fixpoint + firing check | §1 | 14 |
| `extern proc` grammar | §3 | 4 |
| `type X = proc` contract types | §4 | 2 |
| `@scaffolding("reason")` item attr + byte-neutrality | §8 | 4 |
| `as ContractType` dispatch bound on call instrs | §4 | 2 |
| `corpus_contracts.rs` — whole-corpus walk → closure | §1/§11 Q2 | 7 |
| `emp_contracts` bin — the checkpoint driver | — | — |

Full frontend-emp suite **1410/0**, clippy clean. Baseline strict was 2271/0.

## §11 implementation decisions (recorded)

1. **CFG granularity** — G1's closure needs NO CFG (monotone whole-proc set
   union). The lightweight-CFG-with-real-joins answer is pre-registered for
   G2/G3 where §5/§6 path-sensitivity bites (never straight-line-only — the
   stale-1030 trap).
2. **Where the call graph lives** — a whole-corpus **frontend** pass,
   name-resolved, NOT post-link. Reuses the real write detector
   (`proc_written_registers`), so zero drift; contracts + spans native. The
   corpus lowers as independent `.emp` modules under an AS root, so there is no
   single lowered Module anyway — the walk collects procs by name (the
   `emp_census` substrate, Fable-blessed as firing-authoritative). Names are
   globally unique (a duplicate is a link error); `.asm` callees are exactly the
   externs.
3. **@discards attachment** (G2) — pre-register trailing-attribute-on-call form.
4. **extern proc = real symbol decl** — YES (spec preference). Collision
   detected at the walk level (`extern proc X` + `proc X` → flagged).

## Firing reconciliation — census predicted vs closure found

Closure over 126 procs (config-invariant: no-defines == `SOUND_DRIVER_ENABLED=1`,
34 firings either way). Every census-predicted firing IS present.

**Census-predicted DIRECT under-declarations — all present (12 reg + the 3 FPs):**
DeleteObject d1 · EntityWindow_EntryForSection d0 · Section_GetSecPtrXY d2 ·
TouchResponse a4 · DrawRings a4 · Emit_ObjectPieces a4 · InsertSpriteMasks a4 ·
RunObjects_Frozen d7 · EntityWindow_TrySpawnObject a0/a1/d3/d5 ·
EntityWindow_TrySpawnRing a0/d3 · **FPs** AllocDynamic a0 · Collected_ParkSlot a0
· Collected_UnparkSlot a0 (left ALONE — G3, never a false clobbers(a0)).
`Section_RedrawPlanes sr` correctly ABSENT (sr is out of closure scope by design
— stays the local `[proc.sr-undeclared]` check, per §1).

**Pre-retrofit ⊤ (unbounded indirect — resolves when the 6 `as` bounds land):**
RunObjects · AnimateSprite · GameLoop (census indirect sites #1/#2, #3, #5).
RunObjects's own d7 under-decl is currently MASKED by its ⊤; it will re-surface
as `direct d7` once `jsr (a1) as ObjRoutine` bounds the site.

## SURPRISES the census did NOT predict (the reason for this checkpoint)

**S1 — the extern boundary is 4-5, not 3.** Closure holes (plain build):
`VSync_Wait`, `S4LZ_DecompressDict`, **`QueueDMA_Important`**, **`QueueDMA_Deferrable`**.
The two QueueDMA routines are real `.asm` externs (`engine/system/dma_queue.asm`,
via `perform_dplc(QueueDMA_*)` in dplc.emp) that census part (b) missed. Their
contract (shared QueueDMATransfer core): `In d1.l/d2.w/d3.w · Clobbers d0-d4,
a1-a2 · Out carry = dropped`. `Debug_MusicToggle` (census's 3rd) is
`SOUND_DEBUG_HOTKEYS`-gated → elided in the plain/no-hotkey build, so it is NOT a
plain-build hole; it still needs its decl for debug-build analysis.
**⇒ boundary retrofit is 5 extern decls (4 plain-visible + Debug_MusicToggle),
not 3.** (The QueueDMA `out(carry:)` rides G2; G1 declares only their clobbers.)

**S2 — ~13 transitive-leak firings beyond the census's local list.** Procs that
are locally clean-ish but transitively leak a callee's under-declared/FP writes:
Collected_CheckRing d1 · Killed_CheckObject d1 · Sound_PlayRing a0/d1 ·
Load_Object a0 · EntityWindow_RescanObjects a1/a3/d5 ·
EntityWindow_ScanObjectsRight a1/a3/d5 · TrySpawnObject a3 · TrySpawnRing d4.
This is the intended NEW class (the checkpoint's whole point). Expected
behaviour: **most vanish once the direct under-declarations are retrofitted**
(e.g. the Rescan/Scan a1/a3/d5 leaks come from TrySpawn* under-declaring those
regs). Two — **Load_Object a0 and Sound_PlayRing a0 — trace to the AllocDynamic
FP (a0)** and will only clear at **G3** (verified `preserves(a0)`), NOT during
G1's clobbers/out sweep. That is a real cross-phase dependency to note.

**S3 — TestParticle_Main ⊤ is TRANSITIVE, not a 7th indirect site.** It calls a
⊤ callee (a test-harness dispatch); resolves once the source ⊤s are bounded.

**S4 — the "12" stubs are a WORKLIST, not firings.** The 11 Touch stubs +
GameState_Idle are no-contract → invisible to the lint until they declare
`clobbers()` (census A2). So "13+12+3" = 13 firing procs + 3 FP firings + 12
worklist procs; the 12 do not appear in the firing list yet (correct).

## Proposed path past the checkpoint (for Fable's ruling via Volence)

1. **Confirm S1**: boundary retrofit = 5 extern decls incl. the 2 QueueDMA
   (contract above) + Debug_MusicToggle; 4 contract types + 6 `as` bounds; the
   `@scaffolding` on Plane_Buffer_Reset. Then re-run — the 4 ⊤s should collapse
   to their true residual (RunObjects → d7, etc.) and the holes clear.
2. **Proceed with the clobbers/out debt retrofit** (13 under-decls: 3 SAT a4s →
   `out(a4)`, Section_RedrawPlanes → `clobbers(sr)`, the rest → add the reg; 11
   Touch + GameState_Idle → `clobbers()`), expecting the S2 transitive rows to
   SHRINK toward the AllocDynamic-FP residue (Load_Object/Sound_PlayRing a0),
   which is left for G3.
3. **Still-to-build in G1** (post-decision): the §4 subcontract check
   `[dispatch.target-exceeds-bound]` (target ⊑ bound), the extern-collision +
   `[dispatch.unbounded]` diagnostics wired to the build, and the drift-guards +
   kill-list rows for the extern mirrors.

**Recommendation:** the surprises are benign (S2 mostly self-clears; S1 is a
+2 extern undercount with a known contract; S3/S4 are classification, not new
debt). Barring a Fable objection, proceeding with the 5-extern boundary retrofit
+ the clobbers/out sweep is safe. Pausing here for that ruling.

---

## Boundary retrofit landed — prediction check (2026-07-17, post-Fable-ruling)

Boundary decls committed (aeon a661db9, sigil kill-list e40c5d9): 5 externs + 4
types + 6 as-bounds + @scaffolding. **Byte-neutral CONFIRMED**: paired strict
`SIGIL_STRICT_GATE=1` = **202 suites / 2305 / 0**, ROMs byte-identical (was
198/2271; +34 = new contract tests). Closure re-run over the retrofitted corpus:
**131 procs, 5 externs, 4 types, 0 holes, 46 firings**.

Against Fable's four pre-registered predictions:

| # | Prediction | Result |
|---|---|---|
| (b) | all extern holes clear | ✅ 4 → 0 |
| (c) | RunObjects d7 re-surfaces direct | ✅ `RunObjects direct d7` |
| (a) | 4 ⊤s collapse (RunObjects/AnimateSprite/GameLoop direct, TestParticle transitively) | ⚠️ RunObjects→direct ✅, GameLoop→**gone** (declares the full set the GameState ⊤ bound charges — no firing) ✅, TestParticle→transitive ✅, **AnimateSprite→8 TRANSITIVE rows, NOT direct** ❌ |
| (d) | no new firing class | ❌ AnimateSprite (8) + TestParticle downstream (7) = 15 transitive rows the clobbers/out sweep will NOT clear |

**The closure is arithmetically CORRECT — NOT a bug.** ObjRoutine bound =
universe − preserves(a0,d7) = `d0-d6/a1-a6`. AnimateSprite declares
`clobbers(d0-d2/a1-a2)`. Bound − declared = `d3-d6/a3-a6` = the exact 8 firings.
TestParticle_Main declares `clobbers(d0-d3/a1-a2)` — authored as the union of its
callees' DECLARED clobbers (its own comment: "AnimateSprite(d0-d2/a1-a2)"), the
correct discipline — so it inherits AnimateSprite's EFFECTIVE (not declared) via
`jbsr AnimateSprite` and fires `d4-d6/a3-a6`.

**Root cause (single):** `animate.emp:210 jsr (a2) as ObjRoutine`. The AF_CALLBACK
targets are general object routines (census part-c #3: "ObjRoutine offsets baked
into animation scripts"), so per the ObjRoutine bound the callback MAY clobber
everything but a0/d7. AnimateSprite's `clobbers(d0-d2/a1-a2)` therefore
UNDER-DECLARES — a caller relying on d3 surviving an AnimateSprite call would be
wrong (the callback could trash it). The census missed this because AnimateSprite
was ⊤-masked (site #3 unbounded). Bounding it revealed real debt.

**This is why the prediction missed — real debt behind the ⊤, not a closure bug.
Per the ruling, I STOP before the clobbers/out sweep and report.** The decision is
Fable's:

- **Option A — AnimateSprite genuinely under-declares.** Widen it to
  `clobbers(d0-d6/a1-a6)` (whatever the AF_CALLBACK bound allows); TestParticle
  and any other AnimateSprite callers widen in lockstep (they correctly track its
  declared). The clobbers/out sweep scope GROWS; "exactly 5" residue was an
  undercount (the census couldn't see behind the ⊤). This is my lean — it is the
  honest contract for a callback that can run arbitrary object code.
- **Option B — AF_CALLBACK deserves a tighter contract type than ObjRoutine.**
  If the animation callbacks are a KNOWN tight set (only d0-d2/a1-a2), define
  `type AnimCallback = proc (a0: *Sst) clobbers(d0-d2/a1-a2)` and re-annotate
  site #3; AnimateSprite's contract then holds. Requires the §4 subcontract check
  to VERIFY every baked-in callback target conforms (that check is still unbuilt —
  it would be the enforcement). Census part-c #3 gives no evidence the callbacks
  ARE tight, so this needs a corpus audit of the AF_CALLBACK targets first.

Everything else is on track: the 12 census under-decls + 3 FPs are present and
ready for the sweep; holes cleared; ⊤s collapsed. Only the AnimateSprite/AF_CALLBACK
bound blocks the sweep.

**Walk limitation noted:** the Touch dispatch (census site #4, `jsr (a0,d4.w)`)
lives inside a comptime-fn template spliced into two sites, so the AST-body
indirect scan does not see it — the `as TouchHandler` annotation is correct but
INERT for the closure until the walk descends into comptime-fn bodies. Harmless
here (the 11 Touch_* targets are rts-stubs), and it happens to keep TouchResponse
free of G3-residue rows. Logged for G1's remaining work / a future walk pass.

---

## Clobbers/out debt sweep — DONE, residue is 8 not 5 (2026-07-17)

Fable pre-authorized the sweep with the falsifiable check "residue EXACTLY 5,
any other number → stop, report." **Measured residue = 8.** Reporting per the
protocol. The sweep itself is complete and correct; the 8 residue rows are all
genuinely G3 (save/restore-preserved), verified by reading each proc — Fable's 5
undercounted the G3-FP set by 3 d1-preserve rows.

**Sweep applied (aeon 669c287, byte-neutral):**
- 13 census under-decls: DeleteObject +d1 · RunObjects/_Frozen +d7 ·
  EntryForSection +d0 · GetSecPtrXY +d2 · TouchResponse +a4 · TrySpawnObject
  →`d0-d3/d5,a0-a3` · TrySpawnRing →`d0-d4,a0` · the 3 SAT a4s (DrawRings /
  Emit_ObjectPieces / InsertSpriteMasks) as **out(a4)** · Section_RedrawPlanes
  **clobbers(sr)**.
- 12 stubs (11 Touch + GameState_Idle) → `clobbers()`.
- **+2 GENUINE transitive callers the local census could not see:**
  EntityWindow_Rescan/ScanObjectsRight → `d0-d5/a0-a3`. Verified genuine, not FP:
  TrySpawnObject's `movem.l d3/d5/a0-a1/a3,-(sp)` at :1158 is a SPILL frame
  (values reloaded via `8(sp)`/`12(sp)` into *different* regs at :1177/:1201,
  never restored to source), so those regs are real scratch; Rescan/Scan don't
  reload after the loop's `jbsr TrySpawnObject`. This is the transitive closure
  finding debt the LOCAL census structurally cannot — its whole point.

**The 8-row residue — every one save/restore-preserved (→ G3, never a false
clobbers):**

| Row | Preservation mechanism | Fable's 5? |
|---|---|---|
| AllocDynamic a0 | individual-push `move.l a0,-(sp)`, branch-straddled (row 1030) | ✅ |
| Collected_ParkSlot a0 | individual-push | ✅ |
| Collected_UnparkSlot a0 | individual-push | ✅ |
| Load_Object a0 | inherits AllocDynamic's a0 (Load_Object itself preserves a0) | ✅ |
| Sound_PlayRing a0 | inherits Sound_PlaySFX's **declared** `preserves(d1/a0)` | ✅ |
| Sound_PlayRing d1 | inherits Sound_PlaySFX's declared `preserves(d1/a0)` | ❌ MISSED |
| Collected_CheckRing d1 | undeclared `movem.l d0-d1` save/restore around Collected_FindSlot | ❌ MISSED |
| Killed_CheckObject d1 | undeclared `movem.l d0-d1` save/restore around Collected_FindSlot | ❌ MISSED |

**Why the miss (the neither-bucket finding that matters):** Fable's "5" counted
only the `a0` individual-push FPs the census flagged. The closure surfaced 3 more
G3-FPs of a DIFFERENT sub-class — `d1` preserved by (a) a callee's DECLARED
`preserves` (Sound_PlaySFX) propagating through a tail call, and (b) an
UNDECLARED `movem.l` save/restore in the caller (Collected/Killed). Both are
legitimately G3 (§5 verifiedPreserved), and both would be a FALSE clobbers if
widened — so they stay. The census's genuine-vs-FP call was `a0`-shaped; the
closure's transitive view is register-agnostic and caught the `d1` shapes too.

**Open decision for Fable (one, small):** spec §1 phases `verifiedPreserved` to
G3, so the closure as-built subtracts NOTHING for preserves → residue 8. But the
DECLARED-and-movem-verified subset is provable NOW via the existing D2.32 slice
(`check_preserves`). Subtracting DECLARED+verified preserves in G1 would clear
Sound_PlayRing's 2 rows (Sound_PlaySFX declares `preserves(d1/a0)`), dropping the
residue to 6 — and it's arguably a G1 completeness fix, not a scope grab (it uses
existing machinery; only the UNDECLARED individual-push/movem cases wait for G3's
symbolic stack tracking). Left OUT pending your call: (A) keep §1's phasing,
residue 8, all 8 clear at G3; or (B) subtract declared+verified preserves in G1,
residue 6, the 6 individual-push/undeclared-movem cases clear at G3.

Either way: **zero genuine debt remains; the residue is 100% G3-FP.** The G1
clobbers/out sweep has found and fixed every real under-declaration (15 direct/
census + 2 transitive-caller the census missed).
