# S2-D6 clobbers/preserves lint (Phase-1 item #3) — Stage-0 census + re-scope

**MANDATORY Stage 0, run before any design (per the overseer brief §0).** Corpus:
aeon `ae1de4d` (34 `.emp`, 131 procs incl. 5 externs, 5 contract-types). Tooling:
`emp_contracts` (transitive closure census) + per-file `sigil emp` sweep (local lints).
Sigil `32a362d`.

## Headline: the roadmap's FP list is EMPTY (brief hypothesis CONFIRMED)

The **transitive** `[proc.clobber-undeclared]` residue — the flip-grade surface
(`closure.rs::check_firings`, which subtracts §5 verified-preserves via
`corpus_contracts.rs:211`) — is **0**, and has been an **ERROR gate under strict since
G3** (`contract_closure_corpus.rs::corpus_closure_residue_is_empty_the_error_gate`, green).
The 3 historical individual-push sites (AllocDynamic / Collected_Park / UnparkSlot a0) all
**verify and subtract** — 0 transitive firings, exactly as the brief predicted. Dropped
instructions: 0. Extern holes: 0. Collisions: 0.

**So item #3 is NOT the roadmap's "≥3 individual-push FPs" item — that debt was retired
incrementally (G3 + substrate).** Per brief §6, I do not inflate #3 to match the stale text.
But Stage 0 surfaced **two real defects the stale roadmap didn't name**, both squarely
inside the brief's stated goal ("no FP class that pressures a dishonest `clobbers()`; no
unmodeled write form that silently under-fires"):

---

## Defect A — write-detector UNDER-APPROXIMATION (soundness / flip-blocker polarity)

`instr_written_regs` (the shared detector behind the LOCAL lint AND the transitive
`local_writes` the ERROR gate trusts) misses two write forms. Per brief §2 a missed write
is **flip-blocker-class** — the error gate can silently under-fire.

- **(a) dbcc-family counter.** `dbf`/`dbeq dN, label` decrements `dN`; `writes_dest_register`
  excludes the whole family and the detector never counts the first-operand counter (the
  code comment at `proc.rs:432` documents this as a deliberate S2-D6 TODO). **86 sites**
  (82 `dbf` + 4 `dbeq`). **Live false negatives: 0** — every counter is `moveq #N, dN`-
  initialized before its loop (that move IS counted), so `dN` is already in the write set.
  Proven-absent as a *live* FN, but a genuine completeness hole in an error gate's input.
- **(a′) non-stack movem-LOAD reglist.** `movem <ea>, <reglist>` writes the reglist;
  `movem` is not in `writes_dest_register` and the detector doesn't read `CodeOperand::RegList`.
  **1 non-stack site**: `movem.l (a0)+, d0-d6/a2` (tile_cache.emp:344, inside
  `TileCache_DecompressBlock clobbers(d0-d7/a0/a2-a4)` — d0-d6/a2 all declared). Only `a0`
  (post-inc) is currently counted; d0-d6/a2 are missed. **Live FN: 0** (fully declared), but
  a real hole. NOTE the 44 other movem sites are all `(sp)+` restores / `-(sp)` saves — a
  `(sp)+` restore's reglist must stay EXEMPT (it's preserve-discipline; counting it would
  regress a defensive over-save `movem d0-d7,-(sp)…(sp)+,d0-d7 clobbers(d0-d3)` into a
  d4-d7 FP). Rule: count movem-load reglist iff source EA ≠ `(sp)+`.

Other write-forms swept against the corpus mnemonic census (brief §1a): `exg`, `abcd`,
`sbcd`, `movep`, `divs`/`divu`, `bchg`, `negx` — **none present**. `st`/`sf` (Scc t/f) —
**covered** (`is_condition_code` includes `t|f`). `movea`/`swap`/`ext`/`neg`/`not`/shifts/
`bset`/`bclr`/`tas` — covered. So dbcc + non-stack-movem-load are the only two holes.

## Defect B — LOCAL check_clobbers FALSE-POSITIVE class (brief §2 dishonest-`clobbers` pressure)

The per-file `check_clobbers` (proc.rs, runs on every build when `clobbers.is_some()`,
`allowed = clobbers ∪ params ∪ out`) does **NOT subtract verified preserves** — unlike both
the transitive closure AND `check_preserves` (which §5-verifies the *same* registers in the
same lowering pass). Result: **25 live `[proc.clobber-undeclared]` firings, every one on a
§5-VERIFIED-preserved register**:

| Proc | Reg(s) | Preserve form | `preserves()` decl |
|---|---|---|---|
| `AllocDynamic` | a0 | individual push/pop (branch-straddled) | `preserves(a0)` ✓verified |
| `Collected_ParkSlot` | a0 | individual push + `(sp)` peek | `preserves(a0)` ✓ |
| `Collected_UnparkSlot` | a0 | individual push + `(sp)` peek | `preserves(a0)` ✓ |
| `EntityWindow_TrySpawnRing` | d3 | `movem.l d3-d4/a0,-(sp)` pair | `preserves(d3,d4)` ✓ |
| `EntityWindow_TrySpawnObject` | d3,d5 | `movem.l d3/d5/a0-a1/a3,-(sp)` pair | `preserves(d3,d5)` ✓ |

Three surfaces disagree: the WARN lint calls "undeclared clobber" what the ERROR gate calls
"preserved" and what `check_preserves` proves preserved. This IS the brief §2 social-failure
FP — it pressures the author to silence it with a false `clobbers(a0)`, which would then
poison must-def/§6/D1c. **Fix (the verifier grows): add `verified_preserves_regs(proc, buf)`
to `check_clobbers`' `allowed`** — the same subtraction the closure already trusts; a self-
contained existing helper. Kills all 25, byte-neutral, aligns the three surfaces.

### Sibling (report-only): local `check_out` `[proc.out-unwritten]` on `Load_Object out(a1)`
The per-file `check_out` fires out-unwritten on `Load_Object out(a1)`, but the flip-grade
`out_verify` (item #2 edge-credit: a1 flows from `AllocDynamic out(a1 if eq)` on the eq
success edge) passes it (NOT in the 15-firing out-unverified residue). Same superseded-
local-surface class as B — but the fix needs cross-proc edge-credit that can't cheaply run
in a per-file lint. **Recommend: document `out_verify` as the authoritative out surface and
leave the local `check_out` as a coarse fast-path** (or gate it to skip edge-credited outs).
Not core #3.

---

## Candidate residual gaps (brief §1) — adjudicated

- **(a) dbcc / write-form completeness** → REAL. Defect A. Build it.
- **(b) movem-pair expressiveness** → ALREADY CLOSED. The substrate §5 movem-frame growth
  verifies every movem-preserved proc; the only residual is Defect B's *local subtraction*
  gap, not a §5 expressiveness gap. No §5 growth needed.
- **(c) `.asm` W021 tier** → MOOT here. `s4lint`/W021 exists only in docs — not a live tool
  in the current sigil codebase. The `.asm` remainder is 3 boundary-declared externs
  (`VSync_Wait`, `S4LZ_DecompressDict`, `Debug_MusicToggle`). Nothing to port. Recorded.
- **(d) per-callee clobber union export (Tier-C unlock)** → feasible + cheap. The union
  (`declared clobbers ∪ NOT-verified-preserved writes ∪ callee effects`) already IS each
  proc's `closure.effective[name]`. A thin accessor over existing facts. Define consumer
  contract; ship if it stays small.
- **(e) D1a transitivity** → OUT (per brief). Local write set stays local.

## Other census facts (not #3 targets)
- D1c live-clobbered: **2** (Load_Object@AllocDynamic a1, TileCache_FillRow@FindStagedBlock
  a1) — the documented, deliberately-not-edge-coupled observe-only FPs from item #2.
- Dead-saves worklist: **16** (pass-3 register-surgery fuel).
- out_verify residue: **15** (observe-only Buckets 2/3 → G5; narrow-width outs + in-out
  accumulators). None is #3's concern.
- `[proc.undeclared-fallthrough]`: 1 (`Debug_AssertObjLoop`, DEBUG-only) — modernization
  warning, separate lint, not a clobber gap.

## Re-scoped item #3 (byte-neutral, this is the build)
1. **Defect A** — grow `instr_written_regs`: count dbcc counter + non-stack movem-load
   reglist. + mutation tests (drop each arm → a guard test goes green, proving it load-
   bearing) + the grep-proof that live FNs are 0.
2. **Defect B** — `check_clobbers` subtracts `verified_preserves_regs`. Kills 25 FPs.
   + FP-kill regression per class; push-without-restore / restore-into-different-reg still
   FIRE; the existing G3 five stay green.
3. **(d)** — thin per-callee clobber-union export; consumer contract documented.
4. Confirm (b)/(c) closed as above. Report the `check_out` sibling.

**Gates:** transitive residue stays 0 (A adds 0 corpus firings — all newly-counted writes
are declared or verified-preserved); canonical CRCs EXACT both shapes (plain `3aa43cb6`/
420749, debug `ce0e83a6`/428768) from a clean seeded aeon worktree; full strict from tips.
