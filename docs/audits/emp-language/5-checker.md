---
kind: audit-stage
system: emp-language
stage: "5 Checker"
agent: Opus
status: done
description: "36 checks: 30 hold, 3 wrong, 3 mislabelled; every verdict-bearing probe reproduces. Gaps: ungated contract families, unread Z80 tier and comptime DSLs, 9 of 14 pitfalls unexamined, two stale or mislabelled claims."
---
# EMP language: checker

> [!verdict] Result
> - **The findings the verdicts rest on are real.** I re-ran reviewer A's probes x1, x2, x5, lit, x7, d_mask_leak, c_no_pin and b_pin_before_restore with the shipping `sigil 0.1.0 (7630ba0d)`. Each one builds with exit 0 and emits exactly the bytes A reports. The controls (e_no_restore, f_ctl, x6, x8) fire as A says. Reviewer B's probe p4 (comptime `()` escaping `-> int`) and the `indirect-cost` report (11 sites, 0 trusted, 71 forced; built for sonic4 plain, i.e. release) reproduce as well. All MEASURED.
> - **I checked 36 claims: 30 hold, 3 are wrong, 3 are mislabelled.** None of the wrong or mislabelled ones changes a verdict. Two of them change numbers the lead would quote: A lists pitfall §7 (ADDX) as still open, but it is fixed. B's "functional everywhere" table prices the object core at 1.07× to 1.25× "if compiled", but that is stage 2's estimate for a compiler nobody has written. GCC measured 1.73× to 1.77×.
> - **Coverage holes that matter for the direction question:** neither reviewer read the Z80 driver, which is 3,758 lines and 81% of the cycle-critical code, yet both answer "functional everywhere: no" for it. Neither read the two comptime DSLs (7,525 lines), which are the language's *existing* functional tier.
> - **Known defects:** 9 of the 14 `EMP_PITFALLS.md` sections were not examined by either reviewer. The lens-sweep S12 siblings were not examined either: interface signature checks go vacuous when the bound proc has no `clobbers`, and three analysis families are computed by the build without gating it. A's walk asserts that one of those families "fires" as a check.

Trees: aeon `936439fa`, sigil `7630ba0d` (both confirmed with `git log -1`). Probe outputs are in `<scratch>/audit-builds/checker/`. No repo was written.

## 1. Citation and claim checks

**Holds** = the cited text or behaviour is as stated. **Wrong** = it is not. **Mislabelled** = the fact is right but its label or build is wrong.

| # | Claim (stage) | Re-opened | Result |
|---|---|---|---|
| 1 | Sized immediates truncate silently: `303C 2345`, `103C 0034`, `0C00 002C` (A F1) | re-ran x1, x2, x5 | **Holds** (MEASURED, exit 0, identical bytes) |
| 2 | Only a 32-bit range check on the 68k path; Z80 range-checks (A F1) | `lower/code.rs:1396-1405`, `:2038-2048` | Holds |
| 3 | Bare number operand = immediate (A F2) | re-ran lit, x7: `30 3C 80 00` | **Holds**. Control x6 fires `[operand.const-as-address]` |
| 4 | `preserves(sr.mask)` is textual order, "Static order only" (A F3) | `lower/proc.rs:2528`, `:2916-2941`; re-ran d_mask_leak (exit 0), f_ctl (fires) | **Holds**. The leak path passes because the textually last SR write is the `.full` path's restore |
| 5 | The callee side of a flag result is never checked (A F4) | `flag_check.rs:1-12`; re-ran b, c (both exit 0) | Holds |
| 6 | `requires` is checked only when the callee is a symbol (A F5) | `corpus_contracts.rs:1214` (`call_target_sym … else { continue }`) | Holds |
| 7 | `VInt_Level` is `requires(vblank)` and is stored into `VInt_Ptr: u32` (A F5) | `vblank.emp:110`; `ram.emp:406`; `boot.emp:313`; `ojz_scroll_test.emp:979`; `vblank.emp:30`, `:84` | Holds |
| 8 | `irq.emp:63` says a CCR-out bracket "can NEVER adopt" (A) | the text is at `irq.emp:64` | Holds (one line off) |
| 9 | Hand-spelled mask sites: 8, against 8 `with` brackets (A F3) | grep `move.w #$27xx, sr` and `with ints_off*` | **Holds**: 8 + 8 (`irq.emp:82`, `:91` are the context definitions themselves) |
| 10 | `dma_queue.emp:327` ==8 pin, `:332` sizeof==14, `:358` `@budget(cycles: 670)`, `:581` "caller holds the mask" | all four lines | Holds |
| 11 | `vblank.emp:146` `clr.w DMA_Enq_Bytes_Frame` | the line | Holds |
| 12 | `vblank.emp`: 360 of 553 lines are comments | awk count | Holds, exactly |
| 13 | `vblank.emp:250` "128,010-cycle frame" | the text is at `:250` | Holds |
| 14 | `@budget` refuses loops, calls and tails, and roots at entry zero only (A F7, B F6) | `cycle_budget.rs:27-37`, `:75-77` | Holds |
| 15 | `script`/`dispatch` store table ordinals only; `code_word` has 0 hits in `src/` (A F8, B F2) | `ast.rs:730-736`; grep | Holds |
| 16 | Spec2 calls `code_word` "REQUIRED before the first scripted-object port" (A, B) | `SIGIL_SPEC2_LANGUAGE.md:1297` | Holds |
| 17 | The exhibit compiles only against a mock prelude (B F2) | `pitcher_plant_script.emp:54-61` | Holds |
| 18 | A self-yield adds one store, about 12 cycles, with no extra branch under `shows Draw_Sprite` (B W2) | `lower/script.rs:8-11`, `:537-541` (the epilogue is the `shows` label) | Holds (ESTIMATED; `move.w #,(a0)` = 12) |
| 19 | `as Type` narrowing is TRUSTED, not proven (B F1) | `closure.rs:208-216` | Holds. This is lens finding S12, still partial by design (closeout) |
| 20 | 11 bounded dispatch sites; 0 / 71 firings (B F1) | B's `ic.txt` header "sonic4 plain"; lines 5, 22, 95 | **Holds** (MEASURED, release shape) |
| 21 | `ObjRoutine` is both a `newtype` and a proc type (B F1) | `types.emp:59`; `core.emp:27` | Holds |
| 22 | 25 `move … ObjCodeBase` store sites (B F1) | grep: **26**, one of them in `test/fixtures/sigil_objroutine_probe.emp` | **Wrong by 1**. Stage 2 says 26, which is right |
| 23 | Table-less dispatch (`core.emp:595-599`); `sst_custom` is 32 bytes (`sst.emp:82`) | both | Holds |
| 24 | `PState_Spindash` reads `a4` without declaring it (B F5) | `player_spindash.emp:89`, `:106`, `:108`, `:134`; `player_common.emp:51` (bound lists a4 as a clobber) | Holds |
| 25 | `calls.rs:225-229`: input check at direct calls only | the text | Holds |
| 26 | 13 hand-typed PSTATE byte offsets plus a popcount proof (B F4) | `constants.emp:292-296`, `:325-339`; `player_common.emp:1982/2059/2088` | Holds. The row-swap failure B proposes is reasoned, not run (PLAUSIBLE) |
| 27 | `camera.emp` mirrors PSTATE_JUMP/ROLLJUMP (B F4) | `camera.emp:52-62` | Holds |
| 28 | 24 `const PBLK_*` in 8 files (B) | grep: 24 / 8 | Holds. The file's own comment says 21, so B's challenge is right |
| 29 | Spindash is 222 bytes, `$111DC-$112BA` (B W1, release) | `s4.lst:2207`, `:2216`; `$112BA-$111DC = $DE` | Holds (MEASURED, release listing) |
| 30 | `player_spindash.emp:8` names a gate that exists nowhere else | grep over engine, games, tools, build.sh: 1 hit, the comment itself | Holds |
| 31 | `game_debug.emp:69-77`: widths kept fixed for `align` (B F6) | the text | Holds |
| 32 | `TestEmitter_Main` is "7 instructions" (A §1, §8) | the body has **6** (`subq bne move lea jbsr jbra`) | **Wrong** (minor; strengthens A's point) |
| 33 | Pitfall §7 (`add.w dN,aM` → ADDX) listed among the "remain open" silent classes (A F9) | re-probed: now `unsupported form … write adda` (MEASURED); B's p2 agrees | **Wrong**: fixed. A said it did not re-test |
| 34 | Frontend tests 2,784 / 0 / 0 (A) | summed A's `test.log`: 138 binaries, 2,784 passed, 0 failed, 0 ignored | Holds (stage 0's 2,782 + 2 failed is the run without `AEON_DIR`) |
| 35 | Real-compiler cost "1.3 to 2.8 times (MEASURED, stages 1 and 2)" (A §8 dropped ideas, rationale 2) | stage 1 W-table cycles come from sigil's table; stage 2 cycles are "MEASURED bytes, ESTIMATED cycles" (stage 2 §0) | **Mislabelled**: the stage 2 half is ESTIMATED. A's own challenge 5 notes the two sources. Stage 1 used GCC 15, stage 2 GCC 13.3, which no reviewer says |
| 36 | §8.4 "Object core, sprite emit: cost if compiled 1.07× to 1.25× (stage 2 §5.2, ESTIMATED)" (B) | stage 2 header: GCC MEASURED codegen 1.73× (1.77×) on the sprite loop; 1.07-1.25× is the *hypothetical* functional tier | **Mislabelled**: the cheaper figure is about a compiler that does not exist. A's table uses 1.07-1.77 |
| — | Stage 1 conclusion line 26 "97% of vertical blank" with no build named (B challenge 2) | `1-research.md:26` against `:192` | **Mislabelled** (B caught it): DEBUG; release is 82% |

Counted in the 36: 30 holds, 3 wrong (#22, #32, #33), 3 mislabelled (#35, #36, plus the stage 1 row).

## 2. Headline numbers against the raw outputs

| Number | Label | Raw file | Result |
|---|---|---|---|
| 86,080 hand lines; 48,841 comment-only; 60% comment share | MEASURED | `emp_audit/out/summary.json`; stage 2 reconciles exactly | Consistent across stages 0 and 2 |
| 596 of 605 procs with `clobbers` | MEASURED | stage 0 `procs.py` | Not re-run; used by both reviewers unchanged |
| 1 `@budget`, 28 `cycles()`, 349 hand cycle comments | MEASURED | `dma_queue.emp:358`; stage 2 `measure.json` | `@budget` site confirmed |
| 11 / 0 / 71 indirect cost | MEASURED, release (sonic4 plain) | `reviewer-b/ic.txt` | Holds; B states the shape |
| 222 bytes spindash | MEASURED, release | `aeon/s4.lst` | Holds |
| ≈316 cycles spindash path | ESTIMATED (hand count) | none | Labelled correctly |
| 82% / 97% VBlank | MEASURED in the frame-architecture audit | cited | A labels both builds. Stage 1 line 26 does not |
| +5.6k cycles = 4.4% for the named workload | ESTIMATED (6×W4 + 64×W2 + 24×W7) | stage 1 `:198` | Labelled correctly. The reviewers quote it as a ceiling, correctly |
| "40%" / "47%" of code in an escape hatch | ESTIMATED from stage 0 §5 shares | stage 0 | A says it is an upper bound; B says ESTIMATED. Consistent |

## 3. Coverage: in-scope files nobody read

| File or area | A | B | Why it matters |
|---|---|---|---|
| Z80 driver `.emp` (`engine/sound/*`, 3,758 cycle-critical lines) and the Z80 contract crates (`z80_preserves.rs`, `z80_out_verify.rs`, `z80_bus.rs`, `z80_cycles.rs`) | `sound_fm.emp:195-232` only | none | Both rule "exact" for the Z80 under "functional everywhere" without reading it. Stage 2 cost: DID NOT RUN |
| `engine/effects/raster_dsl.emp` (3,861), `engine/level/scene_dsl.emp` (3,664): 48% of all comptime | scene_dsl, 46 code lines | none | This is the language's existing functional tier. How well it works is direct evidence for or against `.emp2` |
| `aeon/CODING_CONVENTIONS.md` (1,157, "the law") | headings | none | §2.1's `mulu`/`divu` rule is enforced "by the comment" (`aeon/CLAUDE.md`): a hand-kept protocol the language could check, listed by neither |
| `lower/script.rs` (885) | none | none | Both reason about script cost and fit. I read `:1-60` and the epilogue (row 18) |
| `type_slice.rs`, `resumable.rs`, `branch_const.rs`, `resolve/contract.rs`, `eval/*` | none | none | See known defects S12-b and S12-c below |
| `SIGIL_CORE_SPEC`, `M0_*`, `PLAN_AUDIT`, `ORACLE_ISA_SHARING` | headings (1-40 of ISA) | headings | The scope said "all of them". Leaving them to the Sigil assembler audit is reasonable; it should be stated on the lead page |
| `generated/` (14 files, 1,029 lines) | none | none | Counted separately by stage 0, as required. Acceptable |
| `sigil/editors/vscode` grammar | first 60 lines | via stage 0 | Acceptable: no new claim |

## 4. Known-defect list

| Known defect (source) | Found by | Status |
|---|---|---|
| Pitfall §1: nested `if` folds to `()` | A F9 (cited), B F3 (re-probed) | **Found**; B's probe reproduces |
| §2 comptime imports don't travel; §4 register tokens in comptime; §5 `extern()` poisons comptime-ness; §8 struct re-elaborated per importing module; §9 `Game.*` absent in layout contexts; §11 placer mis-measure | none | **Unexamined**. §2, §5, §8, §9 are language-design defects (module system, comptime purity), not author slips |
| §3 guards in unreachable modules are dead (45 to 50 modules) | A F9 (cited, not re-run) | Found, not re-measured |
| §6 `assert.<w>` traps | B (Trap C only, in the edges box) | Partial |
| §7 ADDX alias | B (fixed); A lists it as open | **A wrong**, see row 33 |
| §12 always-red cross-type `==`; label-vs-int stays *defined* false (`EMP_PITFALLS_EQUALITY.md` §12 table) | none | **Unexamined**. Label-vs-int being defined as false is §3's always-pass shape, kept on purpose |
| §13 `[T; N]` length contract | B F3, indirectly (the `[int; 3]` holding `()`) | Partial: B shows the element type is not enforced |
| §14 `Struct.field(aN)` unresolved | A F9 (cited); B (single-module probe: now named) | Found. The multi-module path is not reproduced |
| Lens S12: `as Type` trusted | B F1 (A F5 is the context-side twin) | **Found** |
| Lens S12-b: interface/hook signature check vacuous when the bound proc has no `clobbers` | none | **Unexamined**. Still in code: `resolve/contract.rs:488`, `:493` (`p.clobbers.as_ref()?`) |
| Lens S12-c: `[call.slot-type-mismatch]`, `[branch.condition-constant]`, `[bus.*]` computed but "gated by nothing" | none | **Unexamined, and A's walk contradicts it**. `run_contract_gate` (`sigil-cli/src/main.rs:2230-2360`) gates the other families. These three are only printed (`:2524-2575`); only slot types have a corpus test (`sigil-cli/tests/slot_type_corpus.rs`). A step 11 says `[bus.vdp-write-unstopped]` "fires on a definite running bus" (PLAUSIBLE: a `sigil build` fixture would settle it) |
| Lens S13: `@scaffolding` suppresses a lint that does not exist (closeout fixed only the doc) | none | Unexamined. Stage 0 lists it SHIPPED with 5 uses |
| Lens `sig-guard-consumer-class` / `sig-dcw-dcl-truncate`: check-then-use truncation, fixed for `dc.w`/`dc.l` and six sites at `0a729aaf`, "residual same-class sites booked as CHECK-THEN-USE-RESIDUAL" | A F1 found the instruction-immediate sibling without linking it | Found, but not linked to its class or its residual ledger row |
| Lens S18/S19: parser hang, depth-guard abort | none | Fixed per closeout; out of language-design scope. Not a gap |
| Engine-systems review item 6: 68k↔Z80 sound mailbox race (latent) | none | **Unexamined**. A's `shared(vblank)` proposal (F6) covers only 68k-to-interrupt sharing |
| Engine-systems review: Set_VDP_Reg doc, solids after camera, and similar | n/a | Engine scope, not language |
| `2026-09-09-audit-briefing.md`: `flag_arms` heuristic, memo key, Z80 fixup kind | n/a | Read; all items are assembler or CLI scope and are discharged in its addendum. Nothing for this audit |

## 5. Gaps returned to the reviewers (most important first)

1. **A:** Settle lens S12-c. Find whether `[bus.*]`, `[branch.condition-constant]` and `[call.slot-type-mismatch]` gate `sigil build`. `run_contract_gate` (`sigil-cli/src/main.rs:2230-2360`) does not read them. If they do not gate, correct Walk A step 11 ("fires"), add a finding, and put it in improvement 1.
2. **A (lead B):** The Z80 tier was not read: `engine/sound/*` Z80 `.emp` (3,758 lines) and the `z80_*` contract modules. Both "functional everywhere: no" rows for the Z80 rest on no reading. Read at least one driver path and the Z80 contract checks, or mark the row unexamined.
3. **B:** Read `raster_dsl.emp` and `scene_dsl.emp` (7,525 lines, 48% of comptime). They are the language's existing functional tier. Report what works, what fails (the §2, §5, §8 pitfalls live here), and what that predicts for `.emp2`.
4. **B:** Correct the §8.4 row "Object core, sprite emit: 1.07× to 1.25× if compiled". That is stage 2's hypothetical-tier estimate; GCC measured 1.73× to 1.77×. Also note that the stage 1 (GCC 15, sigil cycle table) and stage 2 (GCC 13.3, hand counts) ratios are not comparable.
5. **B:** Examine lens S12-b, the S12 sibling that F1 did not cover. Interface and hook signature conformance is skipped when the bound proc declares no `clobbers` (`resolve/contract.rs:488`, `:493`), so a proc with no `clobbers` clause satisfies every signature. Add it to F1 and its fix if it still holds.
6. **A:** Correct finding 9. §7 (ADDX) is fixed: it is now a loud `unsupported form … write adda` (re-probed). Add the unexamined pitfalls §2, §5, §8, §9 and EQUALITY §12's defined label-vs-int `false` as examined or explicitly unexamined, and fix "7 instructions" to 6 in §1 and §8.
7. **A:** Link F1 (immediate truncation) to its fixed class, `sig-guard-consumer-class` / `sig-dcw-dcl-truncate`, and to the "CHECK-THEN-USE-RESIDUAL" ledger row. Also fix the "1.3 to 2.8 times (MEASURED, stages 1 and 2)" label: stage 2's cycle ratios are ESTIMATED.
8. **A:** State whether `shared(ctx)` (F6, improvement 4) covers the 68k↔Z80 mailbox race in the engine-systems review (item 6), or list cross-CPU shared data as outside the proposal.
