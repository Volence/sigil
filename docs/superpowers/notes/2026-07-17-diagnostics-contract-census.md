# Contract Census — Sigil diagnostics-tier prework (2026-07-17)

**Purpose.** Machine-checkable demand data for the contract-grammar-v2 spec (Fable
drafting) and the error-tier retrofit sweep that will consume it. Three censuses over
the whole `.emp` corpus (aeon `engine/` + `games/sonic4/`, 34 files, 126 procs):

- **(a)** every proc's declared `clobbers`/`preserves`/`out` vs the lint's computed
  write set; every current `[proc.*]` firing; every proc with NO contract.
- **(b)** every `.emp` → `.asm` call (the extern trust boundary).
- **(c)** every indirect-call site (dispatch, function pointers, jump tables).

This is a CENSUS — no fixes. Evidence base: `aeon docs/reviews/2026-07-16-emp-port-optimization-review.md`
("SIGIL DIAGNOSTICS TIER", D1–D11). Spec rows in play: SIGIL_SPEC2_LANGUAGE.md D2.32
(`preserves`), D2.35 (`out`). Lint source: `sigil crates/sigil-frontend-emp/src/lower/proc.rs`.

## How it was generated (reproducible)

The census reuses the REAL lint so it can never drift from what the compiler enforces
(and the auto-inc/dec follow-up updates both at once):

- **Computed write set + full per-proc table** — `emp_census` bin
  (`crates/sigil-cli/src/bin/emp_census.rs`), which parses each file and, for every proc,
  runs `eval_proc_body` → `lower::proc_written_registers` (the SAME detector
  `check_clobbers`/`check_out` use). Output: the appendix TSV
  `2026-07-17-contract-census-procs.tsv`.
  ```
  emp_census $(find <aeon> -name '*.emp' | sort)   # excludes .worktrees
  ```
- **Firings** — a plain single-file lower per file; its diagnostics ARE the authoritative
  `[proc.*]` list. The lint keys on the destination *register*, which resolves even when
  cross-module symbols don't, so a single-file lower reports the same firings as a fully
  linked build with ZERO false positives (an unresolved symbol is a `Sym` operand, never a
  `Reg`).
  ```
  for f in <files>; do sigil emp "$f" 2>&1 | grep -E '\[proc\.|\[dispatch\.'; done
  ```

**Caveats that shape every table below:**

1. **The computed write set is LOCAL, not transitive.** It is a proc's OWN direct register
   writes — callee effects are excluded (D1a's transitivity is future work). So a broad
   declared `clobbers` on a proc that only `jsr`s out shows `COMPUTED=(none)` — correct and
   expected, not an over-declaration bug. Example: `Section_Init clobbers(d0-d7/a0-a4)`,
   computed `(none)` (all work is in callees).
2. **All 126 procs lower as M68000.** The three `cpu: z80` sections (`mt_bank`,
   `dac_samples`, `sfx_bank`) are zero-proc data banks. The clobber lint is 68k-only.
3. **0 procs eval-bailed** — every body resolved to `Code`, so no write set is unknown (`?`).
4. **The write set is heuristic** (this is assembly; full register dataflow is deferred
   S2-D6). One known blind spot remains below — individual-push preservation
   (false-positive firings). The auto-inc/dec blind spot (false-negatives) was CLOSED by
   deliverable 2; the appendix TSV and the firing counts here are the POST-FIX census.
5. **`a7` is filtered from the appendix `COMPUTED_WRITES` column.** Post-deliverable-2 the
   detector counts `-(sp)`/`(sp)+` push/pop as writing a7 (honest — a7 IS advanced), but
   `check_clobbers` exempts it as stack discipline and it is never a clobber/out retrofit
   target, so the census display drops it to keep the clobber-relevant diff readable. A
   genuine stack REPLACEMENT (`movea.l x, sp`) is NOT exempt and would still surface as a
   live firing. (0 such firings in this corpus — every a7 write is push/pop.)

---

## Part (a) — proc contract census

**Totals:** 126 procs · 112 declare a contract (`clobbers`/`preserves`/`out`) · **14 declare
NONE** · **19** distinct `[proc.*]` firings (16 at the deliverable-1 checkpoint + 3 surfaced
by deliverable 2's auto-inc/dec fix). Full per-proc table:
`2026-07-17-contract-census-procs.tsv` (columns: FILE LINE PROC PUB CLOBBERS PRESERVES OUT
COMPUTED_WRITES CONTRACT).

Contract-attribute usage across the corpus: `preserves(...)` is used by ~~**0** procs~~; `out(...)`
by 12 (`collision_lookup`, `section`, `tile_cache`, `dplc`, and the alloc procs). Nearly every
contract is `clobbers`-only — so the retrofit's surface is overwhelmingly clobbers.

> **ERRATUM (2026-07-17, G1 closure gate, spec §5 amend 8b11a9f):** `preserves(...)`
> has **SIX adopters** (`HBlank_Dispatch` + five `sound_api` procs incl.
> `Sound_PlaySFX preserves(d1/a0)`), all movem/`sr`-verified today — not 0. The
> true row-1030 statement is **zero adopters OF THE INDIVIDUAL-PUSH CLASS** (the
> `AllocDynamic`/`Collected_Park`/`UnparkSlot` a0 saves that the syntactic slice
> can't express). The G1 closure subtracts DECLARED+movem-verified `preserves`
> (the D2.32 fast path is existing proof machinery, not deferred to G3), which is
> how those six procs' preserved registers stop leaking into their callers.

### A1 — Every `[proc.*]` firing (16), with adjudication

Firing counts (occurrences, post-deliverable-2): 77 `clobber-undeclared`, 2 `sr-undeclared`,
1 `undeclared-fallthrough` → 19 distinct (proc, register) pairs (16 at the deliverable-1
checkpoint were 42 clobber occurrences; deliverable 2 added the a4 SAT-pointer catches). The
`[out-unwritten]` count is 0 today (no `out()` is currently a false claim — but that check
is exactly what the auto-inc/dec gap blocks for in-out pointer outputs; see deliverable 2).

| Proc | Reg | First site | Class | Note |
|---|---|---|---|---|
| `RunObjects` | d7 | core.emp:421 | **under-decl** | review **core #11** — clobbers omit d7, body writes it (`moveq #.., d7` loop count) |
| `RunObjects_Frozen` | d7 | core.emp:570 | **under-decl** | review **core #11**, same |
| `EntityWindow_EntryForSection` | d0 | entity_window.emp:605 | **under-decl** | review **rings** note — clobber list omits d0 (its own output) |
| `Section_GetSecPtrXY` | d2 | section.emp:221 | **under-decl** | d2 scratched, not declared |
| `DeleteObject` | d1 | core.emp:242 | **under-decl** | d1 written, not declared |
| `TouchResponse` | a4 | collision.emp:151 | **under-decl** | a4 (dynamic cursor) written, not declared |
| `EntityWindow_TrySpawnObject` | a0,a1,d3,d5 | entity_window.emp:1177+ | **under-decl** | four scratch regs written, not declared |
| `EntityWindow_TrySpawnRing` | a0,d3 | entity_window.emp:966+ | **under-decl** | two scratch regs written, not declared |
| `Section_RedrawPlanes` | sr | section.emp:277 | **under-decl (sr)** | `move.w #$2700, sr` interrupt-mask; declare `clobbers(sr)` |
| `DrawRings` | a4 | rings.emp | **under-decl (deliv. 2)** | SAT pointer advanced via `(a4)+`; declares `clobbers(...) out(d5)` — a4 undeclared. Retrofit: `out(a4)` (the gap-ledger's motivating in-out pointer). |
| `Emit_ObjectPieces` | a4 | sprites.emp | **under-decl (deliv. 2)** | SAT pointer via `(a4)+`, not in `clobbers` |
| `InsertSpriteMasks` | a4 | sprites.emp | **under-decl (deliv. 2)** | SAT pointer via `(a4)+`, `clobbers(d0,d1)` only |
| `AllocDynamic` | a0 | core.emp:125 | **FALSE POSITIVE** | individual-push preservation (see below) |
| `Collected_ParkSlot` | a0 | entity_window.emp:375 | **FALSE POSITIVE** | individual-push preservation |
| `Collected_UnparkSlot` | a0 | entity_window.emp:430 | **FALSE POSITIVE** | individual-push preservation |

The 3 `deliv. 2` rows are the auto-inc/dec fix's newly-surfaced correct firings (all a4 SAT
pointers); the other 130 non-a7 auto-inc/dec sites are on already-declared scratch
(a0/a1/a2/a3) and produced no new firing. Plus **1 `[proc.undeclared-fallthrough]`** — a proc reaching `}` without a terminator (see
the TSV / firing sweep; a modernization warning, not a contract gap).

**The individual-push false-positive class (3 firings).** `AllocDynamic`,
`Collected_ParkSlot`, `Collected_UnparkSlot` each WRITE `a0` (`lea Table, a0` /
`lea OFFSET(a0), a0`) but PRESERVE it by hand — `move.l a0, -(sp)` … `movea.l (sp)+, a0` — so
the true contract is "a0 untouched." The syntactic `preserves` slice (proc.rs:461) only
recognizes a `movem` save/restore PAIR, not individual `move.l rN,-(sp)`/`(sp)+`, so
`preserves(a0)` errors and `clobbers(a0)` would be a lie — the contract is INEXPRESSIBLE
today. This is exactly the gap-ledger row `[tranche 13 load_object, 2026-07-13]`
("clobber-undeclared false-positives on individual-push preservation"). **CORRECTION for the
spec/ledger:** that row states *"core.emp's AllocDynamic saves a0 the same way but WITHIN one
straight-line block ending in rts, so it passes."* The census shows **AllocDynamic a0 FIRES**
— the claim is stale (the save/restore straddles the `.append`/`.latch_full` branch split, so
the heuristic can't pair it). The row should be re-opened with its (a)/(b) asks; the class has
≥3 live instances, not zero.

**Retrofit demand:** the 13 `under-decl` firings are the clobbers/out retrofit targets. The 3
FP firings must NOT be "fixed" by adding a false `clobbers(a0)` — they need the individual-push
`preserves` extension (spec ask), or the movem→individual-push contract widening.

### A2 — The 14 no-contract procs (invisible to the clobber lint)

A proc with NO `clobbers`/`preserves`/`out` gets NO clobber lint at all — so these produce
zero firings, and their COMPUTED write set is the retrofit's only demand data.

| Proc | File | Computed writes (local) | Disposition |
|---|---|---|---|
| `Touch_None`/`Enemy`/`Boss`/`Monitor`/`Ring`/`Bubble`/`Projectile`/`SolidBreak`/`Spring`/`SolidHurt`/`Touch` (11) | collision.emp | `(none)` | Empty stub handlers — all alias one `rts` (the `falls_into` stub chain). Genuinely clobber nothing → `clobbers()` is the honest retrofit. **These 11 are the installable-target set of the Touch dispatch (Part c).** |
| `Touch_HandlerTable` | collision.emp | `(none)` | Not a proc — the jump table itself (`bra.w` entries); Part (c) indirect site, not a contract subject. |
| `Plane_Buffer_Reset` | plane_buffer.emp | `(none)` | Ratified zero-caller scaffolding (forward reset hook, VInt_Lag race fix). The adjudication's "scaffolding annotation" case: mark so D7 dead-symbol analysis does not nag it. |
| `GameState_Idle` | game_loop.emp | `(none)` | Minimal state (`rts`); `clobbers()` retrofit. Installable target of the Game_State dispatch (Part c). |

Every no-contract proc computes `(none)` locally — none is a hidden scribbler. The retrofit is
mechanical: `clobbers()` for the stubs, a scaffolding annotation for `Plane_Buffer_Reset`.

### A3 — Over-broad `clobbers` (informational, not bugs)

5 procs declare a wide `clobbers` but write nothing locally (all work in callees):
`Section_Init` (`d0-d7/a0-a4`), `TestParticle`/`TestParticle_Main`/`TestSolid_Init`/
`TestSolid_Main` (`d0-d3/a1-a2`). These are CORRECT under a non-transitive lint (they cover
callee effects the local write set can't see). They are the standing argument for D1a
transitivity — with a call-graph closure the compiler could verify the width instead of
trusting it. Not retrofit targets.

---

## Part (b) — the `.emp` → `.asm` extern trust boundary

`use` imports only TYPES/CONSTANTS; cross-module PROC calls resolve as link-time externs with
no `use`. So the trust boundary is exactly the calls whose target is defined in NO `.emp` (a
still-`.asm` routine). Corpus-wide there are only **3** such direct calls — the engine is
largely self-contained now:

| Callee | Call site | `.asm` header contract | `.emp`-side annotation? |
|---|---|---|---|
| `VSync_Wait` | game_loop.emp:19 (`jbsr`) | vblank.asm:167 — In: none · Out: none · **Clobbers: d0** | **NONE** (bare `jbsr`; the d0 clobber is undocumented caller-side) |
| `S4LZ_DecompressDict` | tile_cache.emp:317 (`jbsr`) | s4lz_decompress.asm:58 — Extra In: **a4**=dict base, **d4.w**=dict len · Extra clobbers: **a4** (d4 preserved); shared body clobbers a3, advances a1 | **PROSE ONLY** — `// cross-seam: …s4lz_decompress.asm` + inline `// decompress clobbers a3, advances a1`. The In (a4/d4) is set up but not declared as a contract. |
| `Debug_MusicToggle` | game_loop.emp:28 (`jsr`) | game_debug.asm:20 — reads Ctrl_1_Press · **Clobbers: d0-d2/a0/a1** | **PARTIAL** — a long comment explains the `jsr`-not-`jbsr` placement decision, but says nothing of the register contract. |

**Demand for extern-proc contract declarations (the adjudication's D1a amendment):** all three
callees have a real, stable `.asm` header contract that today is either undocumented or a prose
comment on the `.emp` side. An `extern proc VSync_Wait () clobbers(d0)` / `extern proc
S4LZ_DecompressDict (in a4, in d4) clobbers(a3, a4) …` declaration would (1) give the caller a
CHECKABLE contract (D1c caller-side liveness could then verify the `move.l a3,-(sp)` save at
tile_cache.emp:316 is necessary and sufficient) and (2) close the call-graph closure hole D1a
needs for transitivity. Three declarations cover the entire current boundary — cheap. The
`.asm` twins remain the source of truth (drift-guard them like the SST equs).

Note: cross-seam DATA symbols (`Current_Act_Ptr`, `Plane_Buffer_Ptr`, `HBlank_Handler_Ptr`,
`Camera_X/Y`, `Cache_*` in ram.asm) and comptime `extern(...)` value reads (SST_*/Act_*/SFX_*
drift guards) are NOT calls — out of scope for (b), already guarded where they're read.

---

## Part (c) — indirect-call sites

Six indirect `jsr` sites across five mechanisms. For each, the installable-target set and the
contract BOUND every target must satisfy — which the `.emp` already documents in PROSE (exactly
what an indirect-call contract bound would formalize; the adjudication's D1 amendment).

| # | Site | Mechanism | Installable targets | Contract BOUND (today: prose) |
|---|---|---|---|---|
| 1 | core.emp:464 `jsr (a1)` (`.run_always`) | Object code dispatch | Every object's `code_addr` (all objdef `ObjRoutine`s) | **Must preserve a0 (SST ptr) and d7 (loop count).** Enforced in DEBUG by `Debug_AssertObjLoop`. Everything else clobberable. |
| 2 | core.emp:520 `jsr (a1)` (`.run_culled`) | Object code dispatch (culled pool) | Same as #1 | Same a0/d7 bound; a2 (live cursor) is CALLER-saved, so targets MAY clobber a2. |
| 3 | animate.emp:210 `jsr (a2)` | Anim event callback (`AF_CALLBACK`, `.evt_callback`) | `ObjRoutine` offsets baked into animation scripts | Called with **a0 = SST ptr**; a1 (script cursor) is caller-saved. Body resumes using a0 → **target must preserve a0**. |
| 4 | collision.emp:85 `jsr (a0, d4.w)` | Touch dispatch — indexed jump table `Touch_HandlerTable` (type × 4, `bra.w` entries) | The **11 `Touch_*` stubs** (the no-contract procs in A2) | Caller saves **d6-d7/a2-a4** around dispatch → handlers MAY clobber those, must preserve everything else. |
| 5 | game_loop.emp:36 `jsr (a0)` | Game-state dispatch (`Game_State` ptr) | Game states (`GameState_Idle` + others) | **Unconstrained** — arbitrary state code; `GameLoop` declares `clobbers(d0-d7/a0-a6)` to reflect it. |
| 6 | hblank.emp:20 `jsr (a0)` | Raster dispatch (`HBlank_Handler_Ptr`, RAM-patched) | HBlank handlers (`HBlank_Null` + future raster) | **`clobbers ⊆ {d0, d1, a0}`** — explicit prose HANDLER CONTRACT: the dispatch `movem`s only d0-d1/a0; every other reg MUST survive (interrupt context). |

**Demand for indirect-call contract bounds:** #6 (hblank) and #1/#2 (object dispatch) are the
strongest cases — the bound is a real, tight, currently-uncheckable invariant carried only by a
comment and, in the object case, a DEBUG-only runtime assert (`Debug_AssertObjLoop`). A
declared bound (`dispatch through HBlank_Handler_Ptr where target clobbers ⊆ (d0-d1/a0)`) lets
the compiler check every installable target at its definition instead of trusting prose + a
debug rail. #4 ties directly to Part (a): the 11 Touch stubs are the installable set, so their
retrofit `clobbers(...)` and the dispatch bound are the same fact from two directions.

Out-of-`.emp`-scope indirect sites for the s4lint `.asm` tier: `VInt_Ptr` (vblank.asm — still
`.asm`) is the review-named indirect the `.emp` census can't see; log it for the `.asm`
best-effort tier.

---

## Feeds-forward summary (for the spec)

- **contract-grammar-v2 / D1b (declared inputs):** Part (b) shows the extern `In` registers
  (a4/d4 for S4LZ) that want `in(reg)` at the boundary.
- **D1a (transitive write-set) + extern-proc contracts:** Part (b)'s 3 externs are the entire
  call-graph-closure hole; A3's 5 over-broad clobbers are the transitivity payoff.
- **D2 (must-use outputs):** 0 `out-unwritten` today ONLY because the in-out pointer outputs
  that WOULD false-positive are left undeclared — unblocked by deliverable 2 (auto-inc/dec).
- **indirect-call bounds:** Part (c) #6/#1/#2/#4 are the ready-made exhibits (prose bounds +
  a debug rail → a checkable declaration).
- **individual-push `preserves` extension:** A1's 3 FP firings (re-opens ledger row 1030).
- **scaffolding annotation:** `Plane_Buffer_Reset` (A2) is the ratified-zero-caller case.

## Deliverable-2 result (auto-inc/dec write detection — CLOSED)

The write set previously MISSED `(An)+`/`-(An)` register modification (133 non-a7 auto-inc/dec
operand sites across 11 files: `(a0)+`×46, `(a2)+`×29, `(a4)+`×26, `(a1)+`×18, `(a3)+`×10,
`-(a1)`×4). Deliverable 2 (TDD, byte-neutral, gap-ledger `[out-clause, 2026-07-11]` CLOSED)
made `instr_written_regs` count `(An)+`/`-(An)` in ANY operand position and ANY mnemonic (so
`tst.w (a0)+` writes a0), with `a7` push/pop exempt as stack discipline.

**Result of the re-census (folded into A1 above):**
- **3 new correct firings** — `DrawRings` / `Emit_ObjectPieces` / `InsertSpriteMasks`, all
  advancing the SAT pointer via `(a4)+` without declaring a4. Exactly the in-out pointer case
  the gap-ledger predicted.
- **0 spurious firings** — the other 130 non-a7 sites are on already-declared scratch
  (a0/a1/a2/a3); a7 push/pop stayed exempt (0 a7 firings).
- **`out(a4)` unblocked** — an in-out pointer advanced only via `(a4)+` is now written, so the
  DrawRings SAT pointer can be declared `out(a4)` without a false `[proc.out-unwritten]`
  (regression-guarded by `out_pointer_advanced_via_postinc_is_written` in `tests/lower_proc.rs`).

Tests (both-direction negative vectors) in `crates/sigil-frontend-emp/tests/lower_proc.rs`:
`postinc_dest_clobber_undeclared_warns`, `postinc_source_clobber_undeclared_warns`,
`predec_clobber_undeclared_warns`, `autoinc_on_read_only_mnemonic_warns`,
`declared_autoinc_pointer_is_silent`, `out_pointer_advanced_via_postinc_is_written`,
`stack_push_pop_is_not_a_clobber`.
