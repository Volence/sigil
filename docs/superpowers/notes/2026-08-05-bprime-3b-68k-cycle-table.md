# 2026-08-05 — B′-3b: the 68000 cycle table (close packet)

Status: Merge state lives in the campaign log, not here. Branch `bprime-3b`, built off sigil `de9d4ca2` and REBASED onto
master `22e7274f` (chain 46) at the overseer's direction — every bar in §8 was
then re-proven at the new base (§8.4). Worktree `.worktrees/b4` (the directory
name deliberately does not match the branch). **ZERO aeon commits, zero aeon
edits** — a sigil-only parcel; the aeon worktree (at `e541028` during the
build, reset to master `77f80c6` for the re-proof) was used read-only and for
gate builds.

Predecessor: `2026-08-04-bprime-3a-cycle-budgets.md` — the path engine and the
Z80 half. This parcel is the 68000 arm that packet designed, sourced and split
out (its §0/§8); the design was executed, not re-litigated.

## §0 — What shipped

- **`crates/sigil-isa/src/m68k_cycles.rs`** — the M68000UM Section 8 timing
  tables as data: the Table 8-2 EA-calculation times, the Table 8-1 MOVE
  matrix as its generating rule (with the `-(An)`-destination quirk), and the
  per-family rows (8-4 standard, 8-5 immediate/quick, 8-6 single-operand,
  8-7 shifts, 8-8 bit ops, 8-9 multiprecision/movem/control). Zero new crate
  dependencies. Every data-dependent form carries its documented MAXIMUM,
  marked `exact: false`; every form outside the table is `Unmodeled`.
- **`crates/sigil-frontend-emp/src/m68k_cycles.rs`** — the `CodeOperand`
  classifier and THE RULING (§2): linker-relaxed forms are charged their
  dearest rung and marked inexact.
- **`cycle_budget.rs`** — the walk goes CPU-parametric in earnest: per-CPU cost
  table + per-CPU edge builder (`Cfg::edges` / `Cfg::z80_edges`), one
  algorithm. `[cycles.unmodeled-cpu]` is GONE (a 68k body measures). Three new
  refusals: `[cycles.computed-transfer]` (a transfer naming no symbol — the
  destination set is data), `[cycles.inexact-cost]` (`@cycles_exact` over a
  ceiling charge), and `[cycles.empty-body]` (panel round — a body with no
  instructions never returns, so a budget on it was a vacuous pass).
  `PathCosts` records the first ceiling-charged mnemonic as the inexact
  witness.
- **Attribute surface untouched**: no change to the parser, `ast.rs`,
  `lower/proc.rs` wiring, or the `@budget`/`@cycles_exact` grammar. The only
  `lower/` edits are two visibility widenings (`reg_kind`,
  `m68k_default_size`) plus their re-export, so the classifier ASKS the
  existing owners instead of restating their tables.
- A 9-line test-infra fix in `crates/sigil-cli/tests/sigil_test_runner.rs`
  (§8.0 — a pid+clock temp-dir collision that aborted a full workspace run at
  master; found by this parcel's own baseline gate).

## §1 — STEP ZERO: the baseline was proven before any edit

At `de9d4ca2` / aeon `e541028`: seven-target byte bar all OK (capture order),
`refreeze --check` OK (tip `b-jumps`, chain 44), full strict
**3248 passed / 0 failed / 4 ignored** — matching the recorded master state.

The first baseline attempt FAILED, and the failure was worth having: cargo's
default fail-fast aborted the workspace run on
`sigil_test_runner::all_passing_exits_zero`, which had run the WRONG fixture —
`unique_temp_dir()` keys on pid+nanos, two parallel tests landed in one clock
tick, shared a directory, and both wrote `m.emp`. A master-state flake in the
gate infrastructure itself: nondeterministic, and it looks exactly like a
parcel defect to whichever lane it bites. Fixed in its own commit (a
process-wide counter is the discriminating term); the overseer can drop that
commit trivially if the fix belongs elsewhere.

## §2 — THE RULING: a budget charges the LONGEST rung, and says so

**The hazard B′-3a split this parcel out over:** on the 68000 a bare-`Sym`
operand does not have one encoding at lowering time. The LINKER picks it:

| form | rungs (cycles) | charged | source |
|---|---|---|---|
| bare `Sym`/`SymOff` absolute | abs.w (+8 / +12 by size) vs abs.l (+12 / +16) | **abs.l** | `RelaxAbsSym`, `lower/code.rs` |
| `jmp Sym` | abs.w 10 / abs.l 12 | **12** | `lower_jmp_jsr_sym` |
| `jbra Sym` | `bra.s` 10 / `bra.w` 10 / `jmp abs.w` 10 / `jmp abs.l` 12 | **12** | `backend-m68k lower_jbra_jbsr_candidates` |
| unsized `Bcc` | taken 10 on both rungs; not-taken 8 (`.s`) / 12 (`.w`) | **10 / 12** | `lower_unsized_branch_candidates` |
| `bra` (any width) | 10 / 10 | 10, **exact** | rungs agree |

**The ruling: charge the dearest rung, mark the charge a CEILING.** Defence:

1. **Soundness has one direction.** The walk runs before the linker chooses.
   Charged cheapest, `@budget` passes at compile time and is wrong on the real
   ROM — the unsuppressible-ERROR tier's one unaffordable polarity. Charged
   dearest, the walk's worst path is ≥ the machine's under EVERY linker
   outcome, because each instruction's charge is ≥ its cost on every rung.
2. **Refusal was the alternative, and it kills the feature.** The corpus
   carries ~2,700 bare symbolic operands (measured, §7); refusing them puts
   essentially every 68k proc out of reach to avoid at most 4 cycles of
   ceiling slack per operand. The author who needs tightness has the escape
   in the language already: pin the width (`(Sym).w`, `beq.s`), and the charge
   is exact.
3. **The ceiling cannot masquerade as a measurement.** `PathCosts` carries the
   inexact witness; `@budget` concludes over it, `@cycles_exact` REFUSES over
   it (`[cycles.inexact-cost]`, naming the first offender in item order) —
   because an equality cannot be proven from maxima, and B′-3a's twin lesson
   was that the walk must never report a number it cannot vouch for. Pinned
   both ways: `a_ceiling_holds_a_budget_but_not_an_exactness_proof` (unit),
   `a_ceiling_refuses_an_exactness_proof_but_holds_a_budget` (integration).

The same ceiling discipline prices the data-dependent forms — none gets a
nominal number:

| form | charged | why it is a machine fact |
|---|---|---|
| `mulu`/`muls` | 70 + ea | 38+2n, n ≤ 16 bit positions |
| `divu` | 140 + ea | UM maximum; oracle-next's exact driver ranges 88..130 + fixed overhead |
| `divs` | 158 + ea | UM maximum; §4 — the one disputed entry |
| shift by register | 6/8 + 2·63 | the hardware takes the count MODULO 64 |
| `dbcc` (live cond) fall-through | 14 | expiry 14 vs cond-true 12 — two reasons, one edge; `dbf`/`dbra` expire only, exact |
| `Scc` on Dn | 6 | 4 false / 6 true; `st` 6 / `sf` 4 exact |
| `bset`/`bclr` on Dn | 8 / 10 (dyn), 12 / 14 (imm) | UM marks the Dn cells as maxima (bit-number dependent) |

## §3 — The table: sources and pins

**Primary source: M68000UM Section 8**, cross-checked per family against the
Exodus core (`oracle/Devices/M68000/*.h`, which transcribes the same tables as
`ExecuteTime(cycles, reads, writes)` per opcode) and, for the data-dependent
forms, against oracle-next's SingleStepTests-validated core
(`oracle-core/src/m68000/microop.rs`).

Pinned by 12 ISA-side tests, one per UM table, entry-by-entry where the errors
hide: all 24 `ea_time` cells; 20 MOVE matrix cells pinned DIRECTLY against
Table 8-1 (not through the generating rule), corners and the
`-(An)`-destination column included (the one column where `4 + src + dst`
naively over-charges by 2); the long-size `+2` footnote on ADD-class
register/immediate sources and CMP's exemption from it; CMPI's cheaper
read-only memory column; ADDQ-to-An's flat 8; TAS's 4/10+ea; the shift
base/2n forms; the bit-op Dn maxima; MOVEM both directions × both sizes ×
per-mode bases × the register-count term; Bcc 10/8/12; DBcc 10/14 with the
`dbf`-exactness rule; JMP/JSR/LEA/PEA per-mode rows; MOVE to/from SR;
ANDI-to-CCR 20; the mul/div maxima. The frontend classifier adds 6 tests
pinning the ruling (§2) form by form, and the walk adds 10 unit + 6
integration 68k tests, including the DMA drain group's own 72-cycles-per-entry
arithmetic reproduced through the walk.

## §4 — The source disagreement: DIVS

**Exodus charges DIVS a flat 168** (`DIVS.h:45`). The M68000UM says 158
maximum. oracle-next's `divs_cycles` — a faithful port of the restoring-
division microcode, 0-mismatch against its vendored SingleStepTests streams —
returns 126..152 on the normal path, plus a 4-cycle trailing refill booked by
its recipe: worst total ≤ 156, under the UM's 158.

**The table charges 158**: two independent sources (the vendor's manual and an
SST-validated implementation) agree it bounds the hardware; Exodus's 168 is an
overcount that tracks neither source. Recorded here rather than adopted
silently, per the lane brief. (Exodus's DIVU flat-140 and MUL 38+2n agree with
the UM exactly; the same header family was the one whose Z80 `ED`-prefix
undercount B′-3a caught, so Exodus alone is never treated as sufficient
corroboration.)

## §5 — What is unmodeled, and therefore refused

- **Mnemonics off-table** (all absent from the corpus census, §7): `link`,
  `unlk`, `exg`, `bchg`, `stop`, `reset`, `chk`, `rtr`, `movec`-class. `trap`
  (5 corpus sites) and `illegal` refuse DELIBERATELY: they transfer into an
  exception handler whose cost is not the instruction's to state.
- **`rtd`** is 68010+; `m68k_mnemonic` does not recognize it, so it cannot
  reach the walk.
- **EA/mnemonic combinations the machine does not have** (e.g. `clr An`,
  `jmp Dn`) refuse rather than price.
- **A computed transfer** (`jmp .table(a1)`, `jp (hl)`) is
  `[cycles.computed-transfer]` — structural, not a table gap; a table entry
  would not make it boundable. The structural refusals now come BEFORE the
  cost-table refusal, so `jp (hl)` says "computed target" rather than "add it
  to the table" (which would have been a misleading invitation). Stated
  plainly: this NARROWS `[cycles.unbounded-transfer]` — a Z80 `jp (hl)` that
  previously reported unbounded-transfer (via unknown-op ordering) now
  reports computed-transfer. Invisible on real builds (no corpus proc
  declares a budget), pinned by the Z80 twin in
  `a_computed_transfer_is_refused_by_its_own_name`.
- **Exception/interrupt entry-exit costs, bus arbitration, VDP FIFO waits**:
  out of model by the same "issued cycles, not elapsed time" contract the Z80
  half states; the module header says so.

## §6 — The named customer: an honest refusal, and the number corroborated

`@budget(cycles: 670)` was placed on `Process_DMA_Critical`
(`engine/system/dma_queue.emp`) in the aeon worktree and built. The build
fails exactly as designed:

```
[cycles.computed-transfer] in `Process_DMA_Critical`: `jmp` transfers to a
COMPUTED target, so where this path goes is data, not structure — the walk
cannot enumerate destinations the program text does not name
```

The edit was reverted; the prose stays; zero aeon commits. **The shape is
named:** the proc dispatches via `jmp .jump_table(a1)` — a computed jump into
its own jump table — and independently, everything past that `jmp` is
reachable only THROUGH it, so the entry-point-zero walk could see nothing
else anyway (B′-3a gap 8). The author's two `ensure(...)` geometry guards
remain necessary — they pin the slot stride and count the prose derivation
depends on, and nothing in a refused walk subsumes them.

**But the table can vouch for the author's number even though the walk cannot
carry it.** The worst chain (slot-8 entry, every relaxable operand at its
ceiling) prices: `movea.w Sym` 16 + `suba.w #imm` 12 + `jmp d16(An)` 10 +
`lea VDP_CTRL` 12 + `lea DMA_Critical` 12 + 8 drain groups × 72 (3× `move.l
(a1)+,(a5)` at 20 + `move.w` at 12 — the comment's own "72 cycles/entry") +
`move.w #imm,(abs).w` 16 + `rts` 16 = **670 exactly**. The prose "~670" is a
correct ceiling under this table; with the abs.w widths the linker will
actually pick for the RAM slot vars, the true worst case is 662.

`jt_slot`'s `bra.w` slots (10) and the `lea`/`lea`/`bra.w` hops price cheaper
than the slot-8 chain, so slot-8 is the worst path — consistent with the
author's own framing.

## §7 — The corpus census (sonic4 shape, throwaway instrumentation, removed)

**Mnemonic/EA coverage:** the 68k corpus is 8,006 instructions over ~60
mnemonics; every one is either in the table or deliberately refused (§5). EA
histogram: Dn 5258 · bare Sym 2708 · Imm 2004 · d16(An) 1142 · An 752 ·
(An)+ 584 · (An) 204 · -(An) 115 · d8(An,Xn) 106 · ImmLink 83 · RegList 68 ·
AbsW 46 · SymOff 39 · Sr 34 · AbsL 24 · d16(PC) 21 · Ccr 19 · d8(PC,Xn) 9 ·
DispSymInd 1. The 2,708 bare-Sym operands are why the §2 ruling is
charge-not-refuse.

**Per-proc verdicts (275 68k procs, post-panel — the empty-body refusal, §9
Lens C F1, moved 12 label-only procs out of "measurable"):**

| verdict | procs |
|---|---|
| **measurable** | **52** (11 fully exact, 41 ceiling-bearing) |
| refused: back edge | 90 |
| refused: tail transfer / fall-off | 60 |
| refused: call | 38 |
| refused: inline data | 21 |
| refused: empty body | 12 (`Touch_None`-class label-only handlers — a vacuous 0-cost "pass" would have certified paths that escape) |
| refused: computed transfer | 2 (`Process_DMA_Critical`, `Player_SetState` — both jump-table dispatch) |
| refused: off-table op | **0** |

The zero in the last row is the coverage claim in measured form: no 68k proc
is refused for a missing table entry — every refusal is structural.

Headline measurables: `Init_DMA_Queue` 1244 exact, `Camera_Update` 292–774,
`Camera_Init` 628–652, `QueueDMA_Deferrable` 126–432, `AllocDynamic` 46–264,
`GetSineCosine` 72 exact, `ObjectMove` 144 exact, the substantive `Touch_*`
handlers, `Player_LevelBound` 136–346, `Tile_Cache_GetTile` 190–210.
Ceiling witnesses across the 41: bare-Sym data references (move/lea/tst/sub/
clr, 24 procs) and unsized conditionals (17 procs).

**Honest adoption count: ZERO today.** The corpus's stated PROC-SHAPED 68k
cycle claims are three: `dma_queue.emp` ~670 (refused: computed dispatch,
§6), `boot.emp:153` "~264 cycles" (a `dbf` delay loop) and `boot.emp:122`
"~360k cycles" (the RAM-clear loop) — the latter two loop-shaped, wanting the
LEDGERED per-pass/trip-count form, not a whole-proc budget. Lens B's wider
sweep found three more cycle-prose sites, none walkable and none proc-shaped:
`constants.emp:361-370` (the frame-window DMA-budget derivation — the
corpus's largest cycle claim, but a whole-frame fact, not a proc's),
`s4lz_decompress.emp:69` ("+16 cycles per match" — per-iteration, and it
cross-checks against this table: cmpa.l 6 + taken bcc 10), and
`tile_cache.emp:1683` (about replaced code). Nothing was adopted and no
adopter was manufactured; what changed is capacity: 64 procs (nine of them
the sound-facing `Sound_PlaySFX`/`QueueDMA_*` family) can now carry a budget
the moment their author states one, where before the count was zero.

**Z80 regression check:** the same census reproduces B′-3a's nine measurable
Z80 procs with IDENTICAL numbers (`Snd_ParkDac` 30 … `Psg_SilenceAll` 90),
all exact. The one Z80 delta is deliberate: the empty `Seq_Trace` moves from
a vacuous (0,0) to `[cycles.empty-body]` (§9 F1) — the shared walk's cost
model is otherwise behaviorally untouched.

## §8 — BARS

### §8.0 — the baseline flake

§1: `sigil_test_runner` temp-dir collision, fixed in its own commit,
re-verified green ×4 (three loop runs + the branch strict).

### §8.1 — Byte bar: SEVEN targets, `cmp`, capture order, run THREE times (baseline, parcel commit, post-panel)

```
OK s4.bin · OK s4.debug.bin · OK demo.bin · OK demo.debug.bin ·
OK config_a.bin · OK config_b.bin · OK lean.bin
>> canonical restored: OK s4.bin · OK s4.debug.bin
```

Byte-neutral ×7. `refreeze --check`: OK (tip `b-jumps`, chain len 44). No
chain bump, no 5-site ripple. The byte argument is structural this time and
Lens B was asked to verify rather than believe it: the new modules' only
consumer is the budget walk, which runs after `lower_code_buf` and appends
diagnostics; the EMITTING cost path (`z80_cycles.rs` → `pad_to_cycles`) has a
zero-line diff; no corpus proc declares a budget, so no new diagnostic can
fire on a real build (warning tallies identical at baseline and branch:
22/64/22/63/64/22/21).

### §8.2 — Full strict, `SIGIL_STRICT_GATE=1`, `--no-fail-fast`

Run THREE times end to end: at the baseline (3248/0/4), at the parcel commit
(3280/0/4), and again after the panel round — the post-panel run is the one
reported. Failures first: **NONE** — no `failures:` block, no `FAILED` line,
no `error[`/`error:` in the final log.

| | baseline `de9d4ca2` | branch tip (post-panel) |
|---|---|---|
| passed | 3248 | **3281** |
| failed | 0 | **0** |
| ignored | 4 | **4** |

`3281 + 4 = 3285`, exactly the branch's `#[test]` total (§8.3): nothing
silently skipped. `3248 + 33 = 3281` closes on the nose.

### §8.3 — Test delta: +33, every function named

`git grep -c '^\s*#\[test\]'` per file, diffed: 3252 → 3285, in EXACTLY four
files; no other file gained or lost a test. (+32 in the parcel commit, +1 in
the panel round — F2's ISA pin.)

- **`crates/sigil-isa/src/m68k_cycles.rs` +13** (new):
  `ea_calculation_times_match_table_8_2`, `move_matrix_cells_match_table_8_1`,
  `standard_alu_rows_match_table_8_4`,
  `immediate_and_quick_rows_match_table_8_5`,
  `single_operand_rows_match_table_8_6`, `shift_rows_match_table_8_7`,
  `bit_op_rows_match_table_8_8`, `control_rows_match_tables_8_6_and_8_9`,
  `move_families_match_tables_8_1_and_8_6`, `movem_rows_match_table_8_9`,
  `data_dependent_maxima_are_ceilings_not_exact`,
  `base_spellings_price_as_their_refined_forms` (panel round, F2),
  `off_table_forms_are_unmodeled`.
- **`crates/sigil-frontend-emp/src/m68k_cycles.rs` +6** (new):
  `a_bare_symbol_charges_the_long_rung_and_is_inexact`,
  `transfer_ladders_charge_their_dearest_rung`,
  `an_unsized_conditional_charges_the_word_fall_through`,
  `default_sizes_resolve_before_pricing`,
  `corpus_shapes_price_through_the_table`, `off_table_forms_refuse`.
- **`cycle_budget.rs` 22 → 31** (+10 −1): added
  `a_68k_straight_line_measures`, `a_68k_sized_conditional_charges_each_edge`,
  `an_unsized_68k_conditional_is_a_ceiling`, `a_68k_call_is_opaque`,
  `a_68k_loop_is_unbounded`, `a_computed_transfer_is_refused_by_its_own_name`,
  `a_ceiling_holds_a_budget_but_not_an_exactness_proof`,
  `a_68k_fall_off_the_end_is_unbounded`, `an_off_table_68k_op_is_unknown`,
  `the_dma_drain_group_measures_72_per_entry`; REMOVED
  `a_68k_body_is_unmodeled` (its refusal no longer exists — the body
  measures, and the replacement asserts the measurement). In the panel round
  `an_empty_body_costs_nothing` was REWRITTEN IN PLACE as
  `an_empty_body_cannot_hold_a_budget` (F1 — same count, opposite verdict,
  both CPUs).
- **`tests/cycle_budget.rs` 30 → 35** (+6 −1): added
  `a_68k_budget_bounds_the_worst_path`,
  `an_unsized_68k_conditional_is_charged_its_word_fall_through`,
  `a_ceiling_refuses_an_exactness_proof_but_holds_a_budget`,
  `a_68k_computed_dispatch_is_refused_by_its_own_name`,
  `a_68k_call_is_refused`, `a_68k_loop_is_refused`; REMOVED
  `a_68k_body_is_refused_by_name` (same reason, same replacement shape).

Both runs report 309 result lines — no new test binary; the whole delta lands
in existing binaries and closes through the `#[test]` arithmetic.

### §8.4 — The post-rebase re-proof (chain 46)

Both masters moved by real code while the parcel ran: sigil to `22e7274f`
(chain entries 45 `defect-batch-8` + 46 `objtest-gate`, then the
`[comptime.unresolved]` merge) and aeon to `77f80c6` (the object-test scene
leaves release ROMs — golden CRCs moved). The branch REBASED onto `22e7274f`
(one conflict, the ledger file — both sections retained, code applied clean);
the aeon worktree, carrying zero own commits, reset to `77f80c6` with both
gitignored seeds verified present. Then the whole bar again at the new base:

- golden target list re-derived from `golden/` — still the same SEVEN;
- byte bar ×7 `cmp` in capture order: **all OK**, canonical restored;
- `refreeze --check`: **OK (tip `objtest-gate`, chain len 46)**;
- test delta vs `22e7274f`: 3265 → 3298 = **+33**, in exactly the same four
  files as §8.3 (master's own +13 is the other lane's);
- full strict at the new base: see the table below — the POST-REBASE run is
  the one reported as final.

| | master `22e7274f` (derived: 3252 + 13) | branch post-rebase |
|---|---|---|
| passed | 3261 (+4 ignored = 3265) | **3294** |
| failed | 0 | **0** |
| ignored | 4 | **4** |

`3294 + 4 = 3298`, the branch's own `#[test]` total: nothing silently
skipped; `3261 + 33 = 3294` closes on the nose.

## §9 — LENS PANEL

Three fresh read-only subagents (A ceremony/style, B corpus-pattern/claims,
C soundness + the table's NUMBERS), over `git diff de9d4ca2..HEAD` plus the
sources. Per the standing rule they were instructed read-only (no
stash/checkout/cargo); none mutated anything.

### Lens A — ceremony/style: approve with two fixes, both applied

| # | finding | disposition |
|---|---|---|
| 1 | **Stale doc comment** above the renamed integration test still asserted "there is no 68000 timing model" — the exact claim the parcel retires, contradicting the assertions below it. (Lens B found the same independently.) | **FIXED** — rewritten to state what the test now proves. |
| 2 | The frontend's `Bra`/`Bsr`/sized-`Bcc` arms RESTATED the ISA table's numbers (10/18/10-8/10-12) where the `Dbcc` arm shows the delegate pattern — two owners of the `Bcc` pair could drift. | **FIXED** — all sized branch arms now ask `instr_cycles`; the unsized ceiling is DERIVED from the table's own two rungs (`nt_s.max(nt_w)`), so no branch number has a second owner. |
| 3 | `(B′-3b)` tags in Rust module headers vs the comments rule. | **KEPT** — Lens A's own precedent check: the Rust corpus carries module-header provenance (`z80_cycles.rs` "rung 4", isa lib.rs "M1.A"); the ban as recorded governs .asm/.emp change-history narration. Extending it to Rust headers is a corpus-wide sweep, not a parcel edit. |
| 4 | `ea_time`/`move_cycles` pub with no external consumer. | **KEPT** — declared API of a published table module (and the natural seam for a future consumer to pin against); Lens A rated leaving them acceptable. |
| 5 | `PathCosts.inexact` field doc said "instruction" where the field holds the mnemonic. | **FIXED**. |

Lens A's clean checks: zero history-word hits on added lines; every test
annotation's arithmetic verified correct (including the MOVE quirk cells and
the modulo-64 shift ceilings); ownership verified (the walk asks
`is_call_mnemonic`/`branch_target`/the edge builders; the classifier asks
`reg_kind`/`m68k_default_size`/`m68k_mnemonic` via widened visibility, not
restatement); brace-indent conforming; no dead ceremony.

### Lens B — corpus-pattern / claims: PASS, five claims exact, two softened

All seven porter claims verified against the repos: attribute surface
untouched (diff-empty on parser/ast/proc.rs); byte-neutrality structural
(consumer trace: isa table → backend re-export → frontend classifier →
`walk_cost` → diagnostics only; `z80_cycles.rs` diff empty); corpus declares
no budget attribute (grep over all .emp); the customer refusal's mechanism
confirmed (`DispSymInd` is not `branch_target`'s `Sym`, `jmp` gets a single
`Defer`, everything past it unreachable from entry); **the 670 recomputed
independently and confirmed, with slot-8 verified the dearest chain (slot 7
totals 570)**; ~25 ISA entries spot-checked with no errors; the aeon worktree
clean at `e541028`.

Two claims held in substance but were overstated, and the packet now says so:

| finding | disposition |
|---|---|
| The "only other 68k cycle claims" sentence missed three non-proc-shaped prose sites (`constants.emp` frame-window DMA budget, `s4lz_decompress.emp` +16/match — which cross-checks against this table, `tile_cache.emp` historical). | **FIXED in §7** — the census sentence now carries all six sites and their shapes. |
| "No existing lint id changed meaning" was not exact: `[cycles.unbounded-transfer]` NARROWED (Z80 `jp (hl)` moved to computed-transfer). | **FIXED in §5** — the narrowing is stated plainly, with its pin named. |
| The same stale test doc comment Lens A found. | **FIXED** (once). |
| Precedence note: structural refusals now outrank `unknown-op` at one instruction. | Already stated in §5; Lens B rated the ordering correct and the rationale in-code. |

### Lens C — soundness + the numbers: PASS, zero wrong entries

**Lens C could not find a single wrong cycle number.** Its method was the one
this parcel exists for: every entry re-derived from the sources rather than
from the diff — all 216 MOVE matrix cells scripted out of Exodus `MOVE.h` and
compared against the generating rule (0 mismatches, `-(An)` column included),
both `ea_time` columns, every family row per-header, the +2 long-size
footnote's EXACT trigger set (`{Dn, An, Imm}`) read out of `ADD.h`'s own
branch, the shift modulo-64 (`op1 %= 64` in `LSR.h`), Scc's +2-when-true,
MOVEM's per-mode bases, the branch pairs, and the mul/div maxima against
oracle-next. **The DIVS ruling confirmed**: oracle-next's SST-validated range
(126..152) already INCLUDES its trailing refill, so the hardware worst is
≤ 152 (+ ea) and the UM's 158 bounds it with margin; Exodus's 168 is an
over-transcription. **No missed relaxable form**: all 32 `CodeOperand`
variants enumerated against the lowering routes; every linker-chosen encoding
is charged its dearest rung, every one-encoding form is rightly exact.
**No under-bound construction found**: mnemonic-shape attacks (`bt`/`bf`,
`rtr`/`rtd`, `bchg`/`exg`/`link`/`trap`), edge-order attacks, and
postorder/witness attacks all terminate in refusals, never a low number.

| # | finding | disposition |
|---|---|---|
| F1 | **An EMPTY body measured (0,0) and a budget on it passed vacuously** — control entering a label-only proc falls into whatever follows, the exact unaccounted-continuation the module refuses everywhere else; the (0,0) was B′-3a's shipped behavior and it contradicted the module's own "only a return ends a charged path" doctrine. | **FIXED** — `[cycles.empty-body]`, a refusal; the `falls_into` message override covers it (an empty aliasing proc with a declared successor gets the true reason). Census re-run: 12 corpus 68k procs + Z80 `Seq_Trace` move from vacuously-measurable to refused (§7). Pinned by the rewritten `an_empty_body_cannot_hold_a_budget`, both CPUs. |
| F2 | **The base spellings of the refined forms (`cmp #1, (a0)` → `cmpi`, `cmp d0, a1` → `cmpa`) fell to `[cycles.unknown-op]`** — a refusal (sound polarity) but claiming a legal corpus spelling is "not in the table", which is false in spirit: the encoder refines and prices it. | **FIXED** — the ISA table prices the base spelling BY RECURSING into the refined form's own arm, so the numbers keep one owner; pinned by `base_spellings_price_as_their_refined_forms`, which asserts spelling-equivalence and the refined values. |
| F3 | `jmp (Sym).w` / `jmp Item.field` (AbsSym/SymOff targets) refused as **ComputedTransfer**, a wrong name — those targets ARE named by the program text. | **FIXED** — the computed test now asks whether ANY operand names a symbol (`Sym`/`SymOff`/`AbsSym`); the named-but-external forms refuse as `unbounded-transfer`, the truly computed (`jp (hl)`, `DispSymInd` dispatch) keep their own name. |
| F4 | The DIVS corroboration comment double-counted the trailing refill ("152 + 4 = 156" — the 4 is inside the 152). Conservative, comment-only. | **FIXED** — comment and packet §4 corrected to "≤ 152, refill included". |
| F5 | `bchg` absent from the table while its three siblings are present (UM: same rows as BSET). | **KEPT** — zero corpus sites (census); the table's stance is demand-driven; noted in §5. |
| F6 | The test-infra flake fix (`746482ff`) verified correct: pid+nanos genuinely collides across parallel threads. | recorded |

**Panel score: 0 blockers; 6 fixes applied (A×3, B-text×2 counted once with A's duplicate, C×4); 3 kept with reasons; every gate re-run after the round** (byte bar ×7, refreeze, full strict, census — the post-panel numbers are the ones in this packet).

## §10 — Lane discipline

- **Zero aeon commits.** The `@budget(cycles: 670)` adoption attempt was made,
  observed, and reverted (§6); `git status` clean at `e541028`.
- **No overlap with the B′-4 lane's files**: `corpus_contracts.rs`,
  `emp_contracts.rs`, `closure.rs` untouched (verified by the diff's file
  list). The parcel's only shared-substrate edits are additive
  (`flag_check::branch_target` gained a consumer, not a change).
- Strict suite staggered: targeted runs during implementation; the full runs
  (baseline, convergence, post-panel) launched when load showed the other
  lane idle.
- **One operations incident, disclosed:** midway through the panel round the
  session's working directory was silently reset to the MAIN sigil checkout
  (where a live session's byte-changing WIP sits). Three targeted `cargo
  test`/`git` READ commands ran there before the wrong test counts gave it
  away; their results were discarded and re-run in the worktree, nothing in
  the main checkout was mutated (its WIP diff was inspected read-only and
  left alone), and every command thereafter names its directory explicitly.
  The tell: a suite total that matches MASTER's arithmetic instead of the
  branch's. Worth a standing-rule line — a lane session must prefix every
  command with its worktree path, because the harness cwd is a main-checkout
  path and can reassert itself between turns.

## §11 — HONEST GAPS

1. **The ceiling is not a measurement, and 41 of the 64 measurable procs carry
   one.** An author wanting exactness pins widths; the language already has
   the spelling. Stated in the module headers, enforced by
   `[cycles.inexact-cost]`.
2. **The two computed-dispatch procs cannot be budgeted at all**, and one of
   them is the corpus's only stated whole-proc demand. A future
   "enumerated-dispatch" form (the jump table's targets are structurally
   pinned by the surrounding `ensure`s) could serve both sites; ledgered.
3. **Loops still refuse**, and both boot.emp cycle claims are loops — the
   per-pass/trip-count budget B′-3a ledgered now has 68k demand too; row
   updated.
4. **`trap` refuses** rather than pricing its exception entry (34 + handler);
   the handler's cost is not a local fact. Deliberate, stated.
5. **The `+2` long-size footnote is modeled for ADD/SUB/AND/OR/ADDA/SUBA and
   exempted for CMP/CMPA/EOR**, per UM and Exodus; if a future form disagrees
   with hardware the ISA tests pin where to look.
6. **`movea` byte size, `cmpm` non-(An)+ forms, and other illegal encodings**
   are not defended against here — they cannot lower, so the cost fn never
   sees them; the table prices only what the encoder accepts.
7. **CI clippy remains red on master** (B′-3a §6 gap 10); this parcel adds
   zero findings against its changed files (checked with clippy over the
   workspace after the strict run) and fixes none of the pre-existing ones.

## §12 — Per-pass findings

### Step 3 — retrospect and language asks

- **"Charge the ceiling, refuse the equality" is a reusable polarity rule.**
  The walk now has three cost classes (exact, ceiling, absent) and two
  consumers with different soundness needs (`@budget` needs ≥, `@cycles_exact`
  needs =). Splitting the enum's "cannot answer" into "cannot answer exactly"
  vs "cannot answer at all" is the same move B′-3a made with
  `Cost::Ambiguous`→`Cost::Split`, one level up — the second instance of the
  campaign's "an Unknown variant conflating two different unknowns" pattern.
- **The structural-before-table refusal ordering matters for message honesty.**
  `jp (hl)` used to refuse as "not in the table — add it", an invitation that
  would not have helped. Refusal ORDER is part of the diagnostic contract, not
  an implementation detail.
- **A width pin is now a performance-contract tool.** `(Sym).w` and `beq.s`
  buy exactness under `@cycles_exact`; that is a new, checkable reason to pin
  a width, and the first time the width-pinning spelling has a semantic
  consumer beyond bytes. Worth a line in any future style doc.
- **Language ask (ledgered): the enumerated-dispatch budget.** Two procs
  dispatch via `jmp .table(a1)` over a structurally-pinned table; the path
  set IS finite and the author has already written the pins (`ensure`
  geometry guards). A form that declares the dispatch's target set would turn
  both refusals into measurements — and `Process_DMA_Critical`'s 670 into a
  checked fact.

### Step 5 — engine optimize

**Nothing shipped** — byte-neutral ×7, correct for a diagnostics parcel. Facts
handed to the engine:

1. **The ~670 prose ceiling is corroborated to the cycle** (§6), and the
   true post-relaxation worst case is 662 — the margin the comment's `~`
   was holding is now a number.
2. **64 procs have measured worst cases** where yesterday there were none,
   `Init_DMA_Queue`'s 1244 and `Camera_Update`'s 292–774 spread being the
   headline pair. Any future VBlank-budget argument starts from these
   numbers.
3. **The 68k refusal distribution (90 loops / 60 transfers / 38 calls)**
   says the engine's dominant unboundable shape is the loop — the OPPOSITE
   of the Z80 side's transfer-chaining (B′-3a §4) — so the per-pass form
   pays on 68k procs first.

### Neither bucket — the headline

**The customer was refused and the refusal is the deliverable working as
designed.** The brief said a refused adoption with the reason named is a good
outcome; what makes it a GOOD one here is that the refusal is specific
(`computed-transfer`, naming the `jmp`), the number the author wrote is
independently corroborated by the same table that refuses to certify it, and
the gap between "the table can price the chain" and "the walk can prove the
chain is the worst one" is exactly the enumerated-dispatch language ask —
written down with both demand sites named. The parcel's real product is that
the 68000 timing model now EXISTS with its correctness burden discharged
(pinned to the UM, cross-checked against two emulator cores, one source
disagreement adjudicated in the open); adoption is a decision for authors who
now have the instrument.
