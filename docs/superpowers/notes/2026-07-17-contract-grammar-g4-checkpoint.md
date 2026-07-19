# Contract-grammar v2 — G4 CHECKPOINT (→ Fable, via Volence)

**2026-07-17, Opus.** G4 builds `[call.input-undefined]` (D1b) and
`[call.live-clobbered]` (D1c) — the last correctness net before pass-3's
register surgery. Per the proven G-pattern: shipped both at WARN, ran the real
corpus PRE-retrofit, and now checkpoint before touching anything. **This
checkpoint has a load-bearing surprise — a substrate finding that blocks the
ERROR flip and touches G1–G3. STOP-don't-chase; ruling requested.**

Branch: sigil `feat/contract-grammar-g4` (1 commit, `f55d9cb`, byte-neutral).
Precondition verified GREEN (masters sigil 1af06cd / aeon c75924e; G3 flip pin
green; canonical plain 8984e510/453533 · debug c80465dc/461554).

---

## 1. What G4 built (the checks are sound)

- **[call.input-undefined] (D1b):** forward MUST-def (all-paths intersection)
  over the shared G2/G3 CFG from the caller's own params; a callee register
  param outside the def set on some path fires. Direct calls/tails only
  (indirect-site input coverage logged, not checked — §4 machinery; object
  pointer always live).
- **[call.live-clobbered] (D1c):** forward MAY-def ("holds a value") + an
  effective-keyed liveness walk. Fires when a value defined before a returning
  call is read after it and the register is in the callee's VERIFIED effective
  clobber set **minus its declared outputs**. Keying off the closure's output
  gives the "callee that verifiably preserves d6 never fires against a caller
  holding d6" property for free (per the G3 packet).
- **TDD: 17 unit tests, all green.** Two of them pin analysis gaps the
  real-corpus run surfaced and I fixed BEFORE this checkpoint (the G3-style
  "real code finds the gap" pattern):
  1. **movem save/restore.** `instr_written_regs` does not expand movem
     reglists, so a caller's `movem.l (sp)+, d5/d7` restore looked like a
     read-after without a redefine. Fix: credit a movem LOAD (reglist as
     destination) as writing its registers. Dropped the raw D1c firing count
     34 → 20 (all the correctly-saved tile_cache registers cleared).
  2. **intervening out-call.** An intervening call declaring `out(R)` produces a
     fresh R, so a later read reads the produced value, not the held one. Fix:
     an intervening call redefines R iff R ∈ its effective set (clobber OR out),
     not just the destroyed-held-value subset. Cleared the Tile_Cache_Fill→
     VSlide(d0) firing (20 → 19).

Full workspace: `sigil-frontend-emp` + `sigil-cli` **1795 passed / 0 failed**
with `AEON_DIR` set — every G1/G2/G3 pin (closure residue 0, flag 0, dead-saves,
preserves) still green. WARN adds firing lists, asserts nothing. clippy clean.

## 2. The firing lists (raw, current corpus — DO NOT adjudicate as-is)

`analyze_corpus` (no defines, engine+games, 34 files, 131 procs incl. 5 externs):
closure residue **0**, flag firings **0**, and:

- **[call.input-undefined] (D1b): 1**
  - `TileCache_DecompressBlock` → `S4LZ_DecompressDict`, input `a4`.
- **[call.live-clobbered] (D1c): 19**
  - EntityWindow group (10): DespawnObjects/DeleteObject d1;
    DespawnObjects/DespawnRings/RingCollision/Killed_MarkObject →
    EntryForSection d0; Rescan/Scan → TrySpawnObject/TrySpawnRing d3;
    RescanY/Scan → Rescan/ScanObjectsRight d5.
  - Render_Sprites → Emit_ObjectPieces (7): a3 ×3, d0 ×2, d1 ×2.
  - TileCache_FillRow → TileCache_FindStagedBlock a1.

## 3. THE SURPRISE — the analysis substrate silently drops instructions

**`analyze_corpus` evaluates each `.emp` in ISOLATION (raw `parse_str` per file,
no cross-file resolution). Struct-field-displacement operands that reference an
IMPORTED struct fail `resolve_field_disp` (asm.rs:1015/1041 `?`) and the WHOLE
instruction is dropped from the CodeBuf** (its error goes to the discarded
`_diags`). The caller-side analyses then reason over an instruction stream with
holes.

Evidence (artifacts, not adjectives):
- `sprites.emp` uses `use engine.objects.sst.{Sst}`; `Sst` is defined in
  `sst.emp`. Source has **32** `Sst.field(aN)` accesses; the analyzed CodeBuf has
  **1** surviving `DispInd` operand. ~31 instructions dropped from ONE proc.
- The dropped writes directly cause specific firings:
  - **D1b a4:** `a4` is defined by `movea.l Sec.sec_block_dict(a1), a4`
    (tile_cache.emp:322, one instruction before the call). `Sec` is imported →
    instruction dropped → `a4` looks undefined. FALSE POSITIVE.
  - **Sprites a3/d0/d1 (all 7):** the registers are redefined by
    `movea.l Sst.mappings(a0), a3`, `move.b Sst.sprite_piece_count(a0), d0`,
    etc. — all dropped → the redefine is invisible → the value looks live to a
    later read. FALSE POSITIVES. (Traced via instrumentation: the "read" the
    check found is `move.l a3, d0` at line 272, immediately after the dropped
    `movea.l Sst.mappings(a0), a3` at line 271.)
- Corpus-wide magnitude: **~176 `Struct.field(aN)` operand instructions across
  24 of 34 files** — most referencing imported structs, i.e. on the order of
  ~150 instructions dropped from the caller-side substrate.

**Consequence:** the D1b/D1c firing lists are dominated by drop artifacts and are
NOT adjudicable as-is. Some genuine findings are mixed in (§4) but cannot be
cleanly separated from artifacts without a resolved CodeBuf.

## 4. Tier-wide implication (the reason this is a ruling, not a hand-fix)

The SAME single-file substrate underlies **the closure (local_writes), the D1d
dead-save worklist, and the flag check** — all of G1/G2/G3. Direction of the
error:
- **Closure:** dropped writes UNDER-approximate `effective`. A register a proc
  writes only via a cross-file field instruction is missing from `local_writes`
  → the residue could be FALSELY 0. The "no undeclared register effect can ship"
  guarantee has a hole wherever a field-only write exists. (The residue gate is
  green — but possibly for the wrong reason in some procs.)
- **Dead-save (D1d):** a dropped instruction between a save and its call can make
  a genuinely-clobbered register look preserved → a FALSE dead-save. Pass-3 cuts
  code on this worklist. **This is the sharp edge: acting on a false dead-save
  deletes a needed save.**
- **Flag check:** a dropped CC-writing field instruction → a false "carry
  survives" → a missed firing.

These are RISKS with a mechanism, not proven breaks — but they must be
re-validated on a resolved substrate before pass-3 acts, and before D1b/D1c flip
to ERROR.

## 5. Genuine findings that likely survive a resolved run (movem-visible)

Independent of the drop (movem/contract text always resolves), the sub-agent
audit found real contract slack worth tightening regardless:
- `EntityWindow_TrySpawnObject` / `TrySpawnRing`: save+restore `d3` (and d5/d4)
  via internal movem but declare `clobbers` → should declare `preserves(d3…)`.
  Clears the Rescan/Scan d3/d5 firings AND is honest.
- `EntityWindow_EntryForSection`: returns its result in `d0` but declares
  `clobbers(d0)`, not `out(d0)` → the d0 firings are "caller reads the return."
  Declaring `out(d0)` both models reality and silences them.
- `TileCache_FindStagedBlock`: returns `a1` (all callers rely on it) under
  `clobbers(a1)`, not `out(a1)` → declare `out(a1)`.
- `DeleteObject` (core.emp): saves/restores `d1` internally (comment says so) but
  declares `clobbers(d1)` → `preserves(d1)`.

These are exactly the kind of contract tightening the D1c net is FOR — but they
should be confirmed on a resolved buf, not committed off the artifact-polluted
list.

## 6. The retrofit-scope census (checkpoint part b)

- 126 procs: **119 empty param lists** + **7 declared params**.
- **95 `// In:` comments across 18 files** — the retrofit target.
- Today D1b fires only against the 7 typed-param procs + 5 externs (hence the
  single a4 firing, an extern). **Post-retrofit, D1b checks ~119 procs' inputs
  at every call site — and every input set up via a cross-file field access will
  false-fire under the current substrate.** The retrofit + ERROR flip is
  therefore HARD-blocked on the substrate fix, not merely gated by it.

## 7. Proposed adjudication (Opus's recommendation — ruling requested)

1. **Fix the substrate first (own parcel).** Give `analyze_corpus` a merged
   cross-corpus struct/type environment so field operands resolve (bring
   imported struct layouts into each file's evaluator; no need for the full
   place→resolve→link pipeline — just the type scope). Then re-run D1b/D1c for a
   trustworthy list.
2. **Re-validate G1–G3 on the resolved substrate** — does closure residue stay
   0? Do the 16 dead-save rows change? Any new flag firings? This protects
   pass-3 before it cuts code.
3. **Then** the In:→params retrofit + the ERROR flip, off a clean list.
4. **Fallback if scope demands:** land D1b/D1c at WARN-only now (this commit),
   ship the four contract-tightening fixes from §5 (verified on a resolved buf),
   and split the substrate fix into its own parcel that pass-3 depends on.

My recommendation: **option 1+2 before any flip** — a false dead-save is a
code-deletion hazard for pass-3, so the substrate must be sound before pass-3
consumes the worklist, and D1b/D1c can't be trusted at ERROR until then. I did
NOT retrofit or flip through this surprise, per the ground rules.

## Per-pass findings (step-3 vs step-5 vs neither)

- **NEITHER-BUCKET (the finding of the phase):** the isolated-parse
  field-instruction drop. Invisible to every prior phase because the closure
  tolerates missing writes (procs over-declare clobbers) and the G3 preserves/
  dead-save targets happened not to straddle dropped instructions in a
  verdict-changing way. Surfaced only by D1b/D1c, which reason over general
  register liveness where a missing write flips the answer. The cleanest
  demonstration yet (after G3's dbcc/Defer) of why the checkpoint runs against
  real code before anything is retrofitted.
- **Step-3 (modernize/language):** D1b/D1c give pass-3 a machine-checkable
  seatbelt; the §5 contract-tightening opportunities (out/preserves) are honest
  upgrades the net makes visible.
- **Step-5 (optimize):** none (byte-neutral phase). The surviving D1c firings,
  once trustworthy, feed pass-3 alongside the dead-save worklist.
