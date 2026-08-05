# 2026-08-04 — B′-3a: cycle budgets, the Z80 half (close packet)

Status: Merge state lives in the campaign log, not here. Branch `bprime-3` off sigil `50382ddc`, **zero commits in aeon** — a
sigil-only parcel (§9). Two commits: `ce64b6d1` (the parcel) + `1506d070` (the panel round).

**§11 first if you are short of time.** The lens panel found THREE blockers in
work that was gate-green, byte-neutral and strict-clean — one of them the exact
recurrence B′-2's ledger predicted, one a soundness hole in an unsuppressible
ERROR contract, and one that the feature could not be written on a single Z80
proc in the engine. All fixed and probed. Every number below is the POST-panel
measurement.

Spec: `specs/2026-08-04-contract-delta-spec.md` §4 (the work order) over
`specs/2026-08-03-contract-unification-spec.md` §4-cycles / §8-P5 (the surface)
and §6 (the tier map).

§0 is the split decision and its argument; §5 is the honest answer to "what
would actually adopt this".

## §0 — THE SPLIT: the engine ships, the 68000 table does not

The spec said this parcel "may split 68k/Z80 if the porter finds the tables
dominate". They dominate. **B′-3a is the path engine + the Z80 cost model;
B′-3b is the 68000 timing table**, designed, sourced and sized here (§8), not
built.

Three independent arguments, each sufficient:

1. **Volume.** The path walk is ~230 lines and is CPU-parametric by
   construction. The 68000 model is ~9 Motorola tables (EA calculation, MOVE's
   full source×dest matrix, standard/immediate/single-operand/shift/bit,
   branches, `movem`) over ~45 instruction families, every entry a separate
   claim. The table is an order of magnitude more work AND carries all of the
   correctness burden.
2. **The Z80 model is already campaign-verified; a new 68000 table would not
   be.** `z80_cycles.rs`'s numbers reproduce the sound driver's own hand-derived
   CYCLE-BALANCE PROOF (FILL 195 / DRAIN 195 / DRAINING_TAIL 194), which is
   checked at every build by three `ensure(cycles(...))` comptime facts. So the
   engine lands on a cost model that is independently pinned by a shipping
   consumer. Landing the engine and a fresh table together would mean a defect
   in either could hide in the other, with nothing but my own arithmetic between
   them and the corpus.
3. **The 68000 side has a soundness hazard the Z80 side does not, and it is
   design work rather than data entry.** A bare-`Sym` operand RELAXES at link
   time: `jbra`/`jbsr` emit four ordered candidate rungs
   (`backend-m68k/src/lib.rs:153`), `RelaxAbsSym` a short/long pair
   (`lower/code.rs:1280`). Those rungs differ in LENGTH and therefore in CYCLE
   COST, and the linker picks after this walk has run. A 68000 budget must
   charge the longest rung or refuse — and deciding which is a ruling, not a
   transcription. Discovering that inside a table-transcription parcel would
   have been the worst place to discover it.

**What the split costs, stated plainly:** the shipped half covers the Z80 only,
and the Z80 is 158 of the corpus's 577 procs. A 68k proc carrying `@budget`
gets `[cycles.unmodeled-cpu]` — a refusal by name, never a number off the wrong
table (`a_68k_body_is_refused_by_name`).

## §1 — What was verified OPEN before building

| Claim | Verified how | Result |
|---|---|---|
| `@budget` / `@cycles_exact` absent | `grep -rn 'budget\|cycles_exact' crates/` | absent; `@cycles_exact` PARSED cleanly and was silently ignored, like `@scaffolding` |
| No worst-case-path machinery anywhere | census of every worklist in the crate (`branch_const`, `calls`, `context`, `closure`, `preserves`, `flag_check`) | all five are monotone fixpoints or visited-set reachability. **Nothing in the repo computes a longest path.** `z80_cycles::span_cost` sidesteps it by operating on a flat slice between two labels, never the CFG |
| No 68000 cycle numbers anywhere | `grep -rn cycle\|timing\|clock crates/sigil-isa/` | zero hits; the ISA crate is timing-free and has **zero dependencies**, so a table fits there with no new edge |
| `@budget(cycles: N)`'s keyword-argument shape | read `parse_one_attr` | `Attr.args: Vec<Expr>` — attributes had NO keyword form; `@budget(cycles: 100)` produced two unrelated parse errors, neither naming `budget` |

## §2 — The design

### §2.1 One table, two conclusions

`z80_cycles::instr_cost` is the sole authority on what a Z80 instruction costs,
and it now has two consumers that differ only in what they conclude:

- `span_cost` — the `cycles(L1, L2)` comptime builtin. Straight-line, single
  path, by ruling (recon §4.3 ruling 2).
- `cycle_budget::path_costs` — the whole-proc walk.

**The extraction that made the borrow work is a strictly better cost model, not
an adapter.** `Cost::Ambiguous` becomes `Cost::Split { taken, not_taken }`. The
four outcome-split conditionals were never *unknown*: their two costs are
outcome-keyed, and only the straight-line consumer could not use them. So:

```rust
("jr",   [cc, _]) if is_cc(cc) => Cost::Split { taken: 12, not_taken: 7 },
("djnz", _)                    => Cost::Split { taken: 13, not_taken: 8 },
("ret",  [cc])    if is_cc(cc) => Cost::Split { taken: 11, not_taken: 5 },
("call", [cc, _]) if is_cc(cc) => Cost::Split { taken: 17, not_taken: 10 },
```

`span_cost` maps `Split` to the same `[cycles.ambiguous-branch]` bail it always
raised — behaviour-preserving, byte-for-byte — while the path walk routes each
number to its own edge. The observable win is precision: a `jr z` diamond's two
arms differ by **1 T-state**, not by the 5 a single nominal cost would imply
(`a_split_conditional_charges_each_edge_separately`).

A second table is not constructible without editing `z80_cycles.rs`, which is
the anti-drift claim in its structural form rather than as a comment.

### §2.2 The walk

`path_costs(items, cpu)`:

1. A DFS from the first instruction that (a) proves the reachable subgraph
   ACYCLIC and (b) leaves a post-order. The two jobs share one walk because a
   back edge is exactly what makes the second impossible.
2. One forward pass over that post-order — successors are ordered before
   predecessors — filling per-node `(min, max)` cost to an exit.

Cost is charged per EDGE, not per node, which is what lets an outcome-split
conditional contribute two different numbers.

**Complexity is linear in edges, not in paths.** Ten nested diamonds are 2^10
paths and 41 instructions; the walk visits each once
(`a_reconverging_chain_does_not_blow_up`).

### §2.3 What it refuses, and why refusal is the feature

An exact worst-case cost is only definable over a FINITE, LOCAL, TOTALLY
MODELED path set. Everything outside that is a named refusal — the same stance
the T-state table already takes toward an op it does not enumerate.

| shape | diagnostic | why |
|---|---|---|
| a back edge | `[cycles.unbounded-loop]` | the longest path through a cycle is unbounded and no trip count is declared |
| `call` / `rst` | `[cycles.opaque-call]` | the callee's cost is not a local fact |
| a tail transfer out, or control off the end of the body | `[cycles.unbounded-transfer]` | the path continues into code the walk cannot see |
| an op outside the T-state table | `[cycles.unknown-op]` | no cost is assignable |
| an outcome-split conditional not presenting two edges | `[cycles.ambiguous-branch]` | the two numbers cannot be routed |
| inline data in the code stream | `[cycles.inline-data]` | those bytes DECODE if control reaches them, and the CFG steps over them (§11 Lens C) |
| a 68000 body | `[cycles.unmodeled-cpu]` | B′-3b |

**Only a RETURN ends a charged path, and only on the EDGE that returns.** The
second clause was the panel's sharpest catch (§11) and it is not a detail: a
`ret cc` closing a body yields `[Abandon, Abandon]`, one a real return and one
control running off the end, and a mnemonic-keyed test closes both. One rule, no
special cases, and it can never under-count. It is stricter than B′-2's stack rule
deliberately: B′-2 charges an undeclared fall-off-end *as* a return because
"whatever follows never agreed to inherit a dirty stack"; the cycle analogue is
that whatever follows *costs more*, so treating a fall-off as a zero-cost exit
would produce a bound that is simply wrong. Refusing is the only honest answer.

**A refusal REPLACES the conclusions.** An unmeasurable proc never also reports
a number, because there is no number it could honestly report
(`a_refusal_replaces_the_conclusions`).

### §2.4 The surface

`@budget(cycles: N)` and `@cycles_exact`, both proc-level.

The keyword form is new: `Attr.args` went `Vec<Expr>` → `Vec<ast::Arg>` and
`parse_one_attr` now parses arguments with the same `self.arg()` the call
grammar uses, so `name: value` works in any attribute. `@budget` REQUIRES its
`cycles:` keyword (`budget_requires_its_resource_keyword`) — the unit is spelled,
not implied by position, so a later budget over a different resource reads
unambiguously beside this one. `N` is any comptime integer expression, folded
through `layout::eval_attr_int` with the `-D` defines in scope, because a real
ceiling is usually derived (`the_budget_may_be_a_comptime_expression`).

Two guards make the declaration hard to get silently wrong, both panel-driven:
the attribute NAME is now a closed set (`[attr.unknown]` — a misspelled
`@cycles_exakt` used to read as a proof and buy none), and a repeated
declaration is `[cycles.form]` rather than silently keeping the first. And
attributes now attach to a proc inside a `section { }`, which is where all 158
of the engine's Z80 procs live — without that the feature was unreachable.

### §2.5 The tier: error, unsoftened, unsuppressible

U-spec §6's row reads `| Budget overrun | error | n/a (new-style attr) | no |`.
Both halves of that follow from the same fact and are worth stating as an
argument rather than as a citation:

- **Error is licensed by the one-file rule** (delta spec §7.2: a fact provable
  FROM ONE FILE may be a hard build failure; a fact needing whole-corpus
  knowledge is enforced at the merge gate). A budget finding is a fact about one
  proc's own instructions and its own control flow. Everything needing knowledge
  past the proc boundary is *refused*, never guessed — so the checker can only
  ever fail a build on something it followed exactly.
- **`@as_compat` does not soften it.** `[stack.*]` softens because it reads raw
  ported assembly nobody annotated, and a faithful port should see the finding
  without failing. A budget exists only because its author wrote it, so there is
  no faithful-port case to protect (`as_compat_does_not_soften_a_budget`).
- **`@allow` does not suppress it.** Asking for the proof and then discarding it
  is a contradiction, and it would leave a claim standing that nothing checks.
  The escape is to delete the attribute: free, honest, and visible in the source
  (`allow_does_not_suppress_a_budget`).

## §3 — The lint set, and its POSITIVE and SILENCE probes

44 tests: 20 unit (`cycle_budget.rs`) + 24 integration
(`tests/cycle_budget.rs`, through the real `.emp` surface).

**Positives** — `the_budget_bounds_the_worst_path` (and that the message names
both the measured cost and the declared ceiling), `a_budget_met_exactly_is_silent`,
`cycles_exact_fires_on_unequal_paths`, `both_attributes_report_together`,
`a_split_conditional_charges_each_edge_separately`.

**Correct code is silent** — `a_straight_line_is_trivially_exact`,
`cycles_exact_proves_a_padded_pair`, `a_diamond_is_not_a_loop` (reconvergence is
not a cycle), `an_empty_body_costs_nothing`,
`a_reconverging_chain_does_not_blow_up`.

**Every refusal arm has a probe, and each is paired with a passing twin** so a
green cannot mean the checker is dead:

| refusal | probe | its twin |
|---|---|---|
| back edge | `a_loop_is_refused` | the same body without the `jp .loop` |
| counted loop | `a_djnz_loop_is_refused` | — |
| call | `a_call_is_refused` | — |
| tail transfer out | `a_tail_transfer_out_is_refused` | — |
| fall off the end | `a_fall_off_the_end_is_refused` | — |
| off-table op | `an_off_table_op_is_refused` | `nop`/`ret`, silent |
| unroutable split (`djnz` to a non-local label) | `an_unroutable_split_is_refused` | — |
| **tail `ret cc`'s fall-through** | `a_tail_conditional_return_is_refused` (+ unit `a_tail_conditional_return_refuses_its_fall_through`) | the same shape with the fall-through closed measures 15/19 |
| **inline data** | `inline_data_is_refused` (+ unit `inline_data_is_refused`) | the same body without the `dc.b` |
| 68000 body | `a_68k_body_is_refused_by_name` (asserts the message names the CPU) | — |
| refusal beats the verdict | `a_refusal_replaces_the_conclusions` | — |
| declared `falls_into` | `a_declared_fallthrough_is_refused_by_its_own_name` (asserts the message names the successor) | — |

**The declaration's own form** — `budget_requires_its_resource_keyword`,
`cycles_exact_takes_no_arguments`, `the_budget_may_be_a_comptime_expression`,
`an_unfoldable_budget_reports_the_form_and_stops` (asserts BOTH that the broken
ceiling is named and that the body is not then also judged),
`a_repeated_declaration_is_reported`, `a_misspelled_attribute_is_loud`,
`a_section_scoped_proc_carries_its_budget`.

**The shared model** — `the_walk_agrees_with_the_cycles_builtin` compiles one
proc carrying BOTH an `ensure(cycles(.start, .end) == 18)` and
`@budget(cycles: 28)` over the same instructions plus a `ret`, so the two
consumers are pinned to agree on the same source. A second table would break it.

**One arm has no probe and cannot get one:** `charged_edges`' empty-successor
guard. `Cfg::z80_edges` yields `vec![]` only when the item index is not an
instruction, which the walk never passes it. It is a defensive refusal, and
stating that is better than implying coverage it does not have.

## §4 — What the checker found over the corpus: NOTHING, and here is what that is worth

All seven shapes lower CLEAN — zero `[cycles.*]` firings — for the trivial
reason that **no corpus proc declares a budget**. The parcel is purely
declaration-driven (`an_undeclared_proc_is_never_walked`), so silence here is
not evidence of anything by itself.

What IS evidence is the census. Throwaway instrumentation (a per-proc
`path_costs` verdict behind `SIGIL_CYCLE_CENSUS`, run over the sonic4 shape,
then removed — it is not in the commit) measured what the walk WOULD conclude
for every proc it lowered:

| | procs | measurable | refused |
|---|---|---|---|
| **Z80** | 158 | **35** | 123 |
| 68000 | 275 | 0 | 275 (`unmodeled-cpu`) |

Of the 35 measurable Z80 procs, 26 are data items lowered with no instructions
(cost 0 — after the panel's inline-data fix these report `[cycles.inline-data]`
instead, which is the more honest answer for a blob with no paths). **Nine are
real, and the numbers below are unaffected by the panel round** — none of the
nine ends in a `ret cc` or carries inline data:

| proc | min | max |
|---|---|---|
| `Snd_ParkDac` | 30 | 30 |
| `Psg_HwCh` | 36 | 36 |
| `Snd_AckBump` | 40 | 40 |
| `Sfx_RouteKind` | 36 | 50 |
| `Snd_RouteClassFlags` | 36 | 50 |
| `Snd_TempoCommand` | 42 | 50 |
| `Fm_RoutePart` | 59 | 68 |
| `Snd_FadeCommand` | 76 | 99 |
| `Psg_SilenceAll` | 90 | 90 |

The 123 refusals break down as `unbounded-transfer` ~55, `unbounded-loop` ~35,
`unknown-op` ~35, `opaque-call` ~10 (first-bail-wins, so a proc counted under
one may have others).

## §5 — HONEST ADOPTION: nothing on the Z80, ONE named site on the 68k

**Not one of those nine procs carries a stated timing obligation.** I grepped
every one of their headers for `cycle` / `T-state` / `timing` / `budget` /
`hot` / `critical`. The only hit is `Snd_ParkDac`, and what it says is that the
proc sits **outside** the timed span.

The corpus's real Z80 timing obligations are all SPAN-scoped, and already served:

- the 195-T sample clock — `ensure(cycles(.loop, .exhaust) == 195)` and its two
  siblings, `z80_sound_driver.emp:489-493`;
- the YM address→data spacing — nine
  `ensure(cycles(...) >= YM_ADDR_TO_DATA_MIN_T)` sites across `sound_fm.emp`,
  `sound_sequencer.emp`, `z80_sound_driver.emp`;
- the derived pads — two `pad_to_cycles(...)` sites.

**A whole-proc budget is the wrong shape for every one of them.** The sample
clock bounds one PASS through a loop, and the walk refuses loops (§8 ledgers the
per-pass form with this as its named demand); the YM spacing bounds two interior
labels, which is what `cycles(L1, L2)` already is.

So no `@budget` was written onto a corpus proc, and no adopter was manufactured.

### The panel found the site I had missed, and it is 68k

Lens B went looking for timing prose I had not and found
`dma_queue.emp:250-254`, `Process_DMA_Critical`:

```
// Zero branches per entry. 72 cycles/entry (3x move.l 20 + move.w 12),
// 576 for all 8 drain groups; ~670 whole-proc worst case with dispatch,
// slot-var reset, and rts (68000 table timing — VDP FIFO waits can only
// add to this, never subtract).
```

**That is `@budget(cycles: 670)` written in prose**, in a proc that already
carries two `ensure(...)` structure guards (`:242`, `:248`) written specifically
to protect that derivation from a geometry change — so the author has already
reached for the machine-checked form everywhere the language offered one, and
stopped at the one place it did not. It is the campaign's recurring "a comment
that should have been a checked declaration" shape, and it is a REAL demand site
for B′-3b that my structural census (235 boundable procs) had generalised past.

The ledger row is corrected to name it, and the "growing the table for procs
nobody wants to bound would be manufacturing demand" argument is narrowed
accordingly: it holds for the Z80 table, and it does not describe the 68000 one,
which has a customer waiting.

Second, weaker, Z80: `sound_sfx.emp:606` `SfxDispatch` — *"it must be fast (the
mailbox handler is on the timing-critical Z80 interrupt path)"*. A whole-proc
obligation with the number missing. It is loop-bearing, so it would be refused
even with a complete table — which is the honest reason it is not an adopter,
rather than the reason I would have given before measuring.

### And the feature could not be WRITTEN on a Z80 proc until the panel round

Lens B's second blocker: `@`-attributes were collected only by the TOP-LEVEL item
loop. Every Z80 proc in the engine — all 158, across 11 `cpu: z80` files — lives
inside a `section { }`, where the attribute was a parse error. So the shipped
half was not merely unadopted, it was unreachable, and my first draft of this
section asserted the former. Fixed in-parcel (both item loops share one
`attach_item_attrs`), pinned by `a_section_scoped_proc_carries_its_budget`, and
byte-neutral — no corpus item gains an attribute.

## §6 — HONEST GAPS

1. **No 68000 timing model.** The largest gap and the whole of B′-3b. §0 argues
   the split; the ledger row carries the source, the sizing and the two hazards.
2. **A loop cannot carry a budget**, so the corpus's sharpest demand site (the
   DAC streaming pass) cannot adopt. Ledgered with the per-pass design.
3. **A call is refused rather than costed through the call graph.** `closure.rs`
   already propagates per-proc facts; a `cycles` facet would compose the same
   way and is what would make budgets usable above a leaf. Ledgered.
4. **11 Z80 mnemonic families block an otherwise-boundable proc** (`add`, `ld`
   forms, `pop`, `srl`, `scf`, `set`, `rrca`, `res`, `ei`, `dec` forms, `bit`),
   measured with site counts. NOT added: the table's stance is that a form
   enters when a timed region needs it, and growing it for procs nobody wants to
   bound is manufacturing demand. Ledgered so a future adopter pays a lookup, not
   a rediscovery.
5. **A budget bounds ISSUED cycles, not elapsed time.** Bus contention (the Z80
   losing the bus to a 68000 DMA, VDP wait states) is a whole-machine fact and
   is not modeled. This is the same unit the driver's own balance proof uses, so
   the two agree — but the word "budget" invites the stronger reading and no
   diagnostic can correct it. Stated in the module header, ledgered.
6. **`reti` / `retn` are unreachable from `.emp` source** — the frontend does not
   recognise the mnemonics, so their 14-T entries are correct, needed by the
   model for completeness, and exercised only by hand-built `CodeItem`s. Stated
   rather than left as apparent coverage (Lens C).
7. **Inline data in a code stream is refused, not costed** (`[cycles.inline-data]`).
   The sound direction, but blunt: a proc with an unreachable jump table cannot
   carry a budget either. Distinguishing them needs a reachability proof no `Cfg`
   consumer has — and the same blind spot is inherited unexamined by `preserves`,
   `flag_check`, `context` and `branch_const`. Ledgered as a class.
8. **A budget covers ENTRY-POINT-ZERO paths only.** The walk roots at the body's
   first instruction, so a path reachable only from an `export`ed mid-body label
   is outside the claim. Stated in the module header (Lens C).
9. **`@budget` is not accepted on an `extern proc` / contract-type signature.**
   `ProcSig` has no `attrs` field. Nothing asks for it today (a bound would be a
   claim about code the file cannot see, which the walk refuses anyway), but the
   asymmetry with `clobbers`/`preserves` is real.
10. **CI clippy is RED, on master as well as here.** Lens A measured 10
   workspace-crate warnings against `clippy --workspace --all-targets -D
   warnings`; the parcel adds zero of them and fixes zero. Not this lane's to
   fix, but "clippy clean" cannot be claimed at merge and past `PROVENANCE.md`
   entries have claimed it — so something regressed on master unnoticed. Flagged
   for the overseer as a separate one-sitter.

## §7 — `warn_tier_corpus.rs`: UNTOUCHED, and why

Two independent reasons, either alone sufficient:

1. **Tier.** `[cycles.*]` and `[budget.form]` are ERROR. `collect_warnings`
   gathers `Level::Warning`/`Note`; an error never reaches that baseline.
2. **Reach.** No corpus module declares `@budget` or `@cycles_exact`, so the
   checker does not walk a single corpus proc.

Verified own-run: `warn_tier_lint_ids_match_the_frozen_baseline` passes unchanged
across all seven shapes, and the seven builds' warning tallies are identical to
the pre-edit baseline (22 / 73 / 22 / 72 / 73 / 22 / 21).

## §8 — B′-3b: the 68000 table, designed and sized

Recorded in `campaign-gap-ledger.md` and summarised here so the overseer can
scope it without reading the diff.

- **Interface:** already fixed. `path_costs` needs one function of
  `(mnemonic, size, operands) -> Cost` plus the CPU's edge model, both of which
  the shared `Cfg` already provides for the 68k (`Cfg::edges`).
- **Home:** `crates/sigil-isa/`, next to the operand metadata, as the spec says.
  The crate has **zero dependencies**, and a table keyed on
  `sigil_isa::m68k::{Mnemonic, Size, Operand}` adds none.
- **Source:** `oracle/Devices/M68000/*.h` in this workspace — the Exodus-derived
  core transcribes M68000UM Section 8 per opcode, mode by mode. `MOVE.h:13-24`
  is the full source×dest matrix as `ExecuteTime(cycles, busRead, busWrite)`.
  Every entry is cross-checkable against a shipping emulator instead of against
  memory. `oracle-next`'s microop core, validated against the SingleStepTests
  680x0 suite, is the second opinion.
- **Sizing:** 235 of 419 corpus procs are structurally boundable (no call, no
  back edge — measured); ~45 instruction families cover the corpus histogram.
- **THE DEMAND SITE, named:** `dma_queue.emp:250-254`, `Process_DMA_Critical` —
  *"~670 whole-proc worst case with dispatch, slot-var reset, and rts (68000
  table timing)"*, hand-derived in prose, in a proc already carrying two
  `ensure(...)` guards written to protect that derivation. `@budget(cycles: 670)`
  is what that comment wants to be. B′-3b is not a speculative table; it has a
  customer (§5).
- **Two hazards it must answer** (neither exists on the Z80 side):
  (a) `mulu`/`muls`/`divu`/`divs` and register-count shifts are DATA-dependent,
  so each needs a worst case or a refusal, never a nominal number;
  (b) a bare-`Sym` operand relaxes at LINK time across candidate rungs of
  different length and cost, so a lowering-time budget must charge the longest
  rung or refuse.
- **Also needed:** `sigil_isa::m68k::encode_ea` is private and allocates; the
  table wants a `pub` `(mode, reg, ext_words)` accessor so the timing model and
  the encoder cannot drift. `lower/code.rs::m68k_operand` is the one
  `CodeOperand -> Operand` classifier and is module-private; reuse it, do not
  write a second.

## §9 — LANE DISCIPLINE: aeon adoption HELD, and nothing wanted it

Zero aeon commits, zero aeon edits. The parcel touches no `.emp` file. **Nothing
in it wants one, and after the panel that is a measured statement rather than an
assumption:** the one corpus proc with a stated whole-proc cycle ceiling is
`Process_DMA_Critical`, which is 68k and therefore B′-3b's adopter, not this
parcel's (§5). No Z80 proc is asking.

**Conflict surface with the concurrent B′-4 lane** (which is in
`emp_contracts.rs` / `corpus_contracts.rs` / `closure.rs`), stated fully because
the panel round widened it past the first draft:

| file | what this parcel does to it | risk |
|---|---|---|
| `corpus_contracts.rs` | the `Attr.args` migration, at `:1247-1256` only | the only real overlap; a four-line textual change in one helper |
| `closure.rs` | **untouched** | none |
| `emp_contracts.rs` | **untouched** | none |
| `flag_check.rs`, `preserves.rs`, `context.rs` | `is_return_mnemonic` gains a `cpu`; `entry_instr_idx` moves; `is_call_mnemonic` goes `pub(crate)` | B′-4 is a report parcel and should not be in these |
| `parser.rs`, `ast.rs`, `eval/builtins.rs`, `eval/mod.rs`, `lower/mod.rs`, `lower/proc.rs` | attribute grammar + the pad-unit derivation | not B′-4's files |

Whichever merges second rebases; if that is this one, the `corpus_contracts.rs`
hunk is the only place to look.

## §10 — BARS

### §10.1 Byte bar — SEVEN targets, `cmp`, in `capture_goldens.sh` order

Built in capture order (`config_a` writes `s4.debug.bin`; `config_b` AND `lean`
both write `s4.bin`), then canonical rebuilt and re-checked:

```
OK        s4.bin
OK        s4.debug.bin
OK        demo.bin
OK        demo.debug.bin
OK        config_a.bin
OK        config_b.bin
OK        lean.bin
>> restoring canonical s4.bin + s4.debug.bin
OK        s4.bin
OK        s4.debug.bin
```

Byte-neutral ×7. **No refreeze, no chain bump, no 5-site ripple.**

**The byte argument, in the form Lens B corrected it to.** The checker itself
cannot emit (it runs after `lower_code_buf` and only appends diagnostics), but
the COST MODEL can: `span_cost` → `cycles()` → `pad_to_cycles`'s `measured` →
the emitted pad count. My first draft claimed `Cost` never reaches emission,
which is false. The sound argument is this one: the only value-changing table
addition is `ret`/`reti`/`retn`, and at master a `cycles()` span containing one
produced a build-FAILING `[cycles.unknown-op]`. The corpus builds, therefore no
span contains a return, therefore no span value can move — verified against all
13 real `cycles()` sites, and now enforced going forward by `[cycles.path-end]`
(§11 Lens C 3). The `pad_to_cycles` rewrite that derives its two units from
`instr_cost` is byte-neutral for the same reason the numbers agreed.

Run TWICE: once on the first cut, and again after the panel round — which
touched the SHARED `Cfg` (`is_return_mnemonic` gained a CPU), the SHIPPED
emitting path (`pad_to_cycles`), and the parser's item loops. Re-proving was not
a formality.

`refreeze --check` → `OK (tip 'b-jumps', chain len 44)`.

**The baseline was proven before any edit**, per the standing rule: the same
seven-target `cmp` plus `refreeze --check` plus a full strict run at the branch
point. That mattered — the handed-over warm `target/` had been seeded from the
B′-2 worktree, so the release `sigil` binary carried a baked
`CARGO_MANIFEST_DIR` pointing at `.worktrees/b2` and the three off-canonical
shapes panicked reading a frozen table from the wrong tree. A forced rebuild
fixed it, and it would have looked exactly like a real defect an hour later.
**Worth a standing rule of its own: a copied `target/` is not a warm cache, it is
another worktree's binaries.**

### §10.2 Strict suite

```
AEON_DIR=<b3 aeon worktree> SIGIL_EMIT=… SIGIL_BUILD=… SIGIL_STRICT_GATE=1 \
  cargo test --workspace --release
```

**Failures first: NONE.** No `failures:` block, no `FAILED` line, no `error[` /
`error:` anywhere in the log.

| | baseline `50382ddc` | branch `1506d070` |
|---|---|---|
| result lines | 308 | 309 |
| **passed** | 3176 | **3228** |
| **failed** | 0 | **0** |
| **ignored** | 4 | **4** |
| filtered out | 0 | 0 |

`3228 + 4 = 3232`, which is EXACTLY the branch's own `#[test]` total (§10.3), so
nothing is silently skipped. The baseline row is a REAL RUN at the branch point,
not a derivation — it was taken before any edit, together with the seven-target
byte bar, per the standing rule. `3176 + 52 = 3228` closes on the nose. The one
extra result line is the new `cycle_budget` integration binary.

Run TWICE: `3220 / 0 / 4` on the first cut (identity held at 3224) and again
after the panel round. The second run is the one reported.

### §10.3 Test-delta arithmetic — every added function NAMED

`git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'`, diffed per file:

| | master `50382ddc` | branch `1506d070` | delta |
|---|---|---|---|
| `#[test]` total | 3180 | **3232** | **+52** |

The per-file diff shows exactly TWO changed files, both NEW — no existing file
gained or lost a test:

**`crates/sigil-frontend-emp/src/cycle_budget.rs` +22** (unit, over hand-built
`CodeItem`s): `a_straight_line_has_one_cost`,
`a_split_conditional_charges_each_edge_its_own_cost`,
`a_fixed_conditional_charges_both_edges_the_same`,
`a_conditional_return_ends_one_path_and_continues_the_other`,
`a_loop_is_unbounded`, `a_diamond_is_not_a_loop`, `a_call_is_opaque`,
`a_tail_transfer_out_is_unbounded`, `a_fall_off_the_end_is_unbounded`,
`an_off_table_op_is_unknown`, `a_68k_body_is_unmodeled`,
`an_empty_body_costs_nothing`, `the_budget_is_checked_against_the_worst_path`,
`cycles_exact_proves_equal_paths`, `an_undeclared_proc_is_not_walked`,
`both_attributes_report_together`, `a_bail_replaces_the_conclusions`,
`a_reconverging_chain_does_not_blow_up`, `a_djnz_loop_is_reported_as_a_loop`,
`a_tail_conditional_return_refuses_its_fall_through`, `inline_data_is_refused`,
`the_walk_and_the_span_builtin_agree`.

**`crates/sigil-frontend-emp/tests/cycle_budget.rs` +30** (through the real
`.emp` surface): `the_budget_bounds_the_worst_path`,
`a_budget_met_exactly_is_silent`, `an_undeclared_proc_is_never_walked`,
`cycles_exact_fires_on_unequal_paths`, `cycles_exact_proves_a_padded_pair`,
`a_straight_line_is_trivially_exact`, `both_attributes_report_together`,
`a_loop_is_refused`, `a_djnz_loop_is_refused`, `an_unroutable_split_is_refused`,
`a_call_is_refused`, `a_tail_transfer_out_is_refused`,
`a_fall_off_the_end_is_refused`, `an_off_table_op_is_refused`,
`a_68k_body_is_refused_by_name`, `a_refusal_replaces_the_conclusions`,
`as_compat_does_not_soften_a_budget`, `allow_does_not_suppress_a_budget`,
`budget_requires_its_resource_keyword`, `cycles_exact_takes_no_arguments`,
`the_budget_may_be_a_comptime_expression`,
`an_unfoldable_budget_reports_the_form_and_stops`,
`the_walk_agrees_with_the_cycles_builtin`,
`a_split_conditional_charges_each_edge_separately`,
`a_tail_conditional_return_is_refused`, `inline_data_is_refused`,
`a_section_scoped_proc_carries_its_budget`,
`a_declared_fallthrough_is_refused_by_its_own_name`,
`a_repeated_declaration_is_reported`, `a_misspelled_attribute_is_loud`.

22 + 30 = **52**. The per-file diff shows exactly TWO changed files, both NEW —
no existing file gained or lost a test, so every one of the 52 is named above.
No test was removed, renamed away, or silently skipped.

### §10.4 Clippy

CI runs `clippy --workspace --all-targets -- -D warnings`. **The parcel adds
ZERO clippy findings** — verified by running clippy on the crate at the branch
and then again with the working tree stashed to master: the same warnings appear
in both runs, at shifted line numbers (`eval/mod.rs:849`, and `lower/proc.rs`'s
`invariant_regs.iter().any(...)`, which my step 12 moved from `:526` to `:658`).
Lens A re-verified both by `git blame` — neither traces to this branch.

**But clippy is not GREEN, and it is not green on master either.** Lens A
measured 10 workspace-crate warnings against `-D warnings`, so CI clippy is red
independently of this parcel. It neither causes nor fixes any of them; recorded
as §6 gap 10 because "clippy clean" has been claimed in past `PROVENANCE.md`
entries and cannot be claimed at this merge.

## §11 — LENS PANEL

Three fresh read-only subagents, one lens each, over `git diff master...bprime-3`
plus surroundings. The porter reviewed nothing of its own.

**Panel score: 3 distinct BLOCKERS (two found independently by two lenses), all
fixed and probed; 14 further findings fixed; 5 ledgered; 1 declined with reason.**
The parcel was gate-green, byte-neutral ×7 and strict-clean before the panel ran.

### The three blockers

| # | finding | found by | disposition |
|---|---|---|---|
| **B1** | **A tail `ret cc` produced an UNSOUND bound — too LOW, the one polarity an unsuppressible ERROR tier cannot afford.** `Cfg::z80_edges` returns `[Abandon, Abandon]` for a `ret cc` ending a body: edge 0 is the real return, edge 1 is control running off the end. `charged_edges` keyed `is_return` on the MNEMONIC, so both were closed as path ends — the escaping path was charged 5 T and declared finished. Reproduced on the built CLI: `cp 1 / ret z` reported "worst-case 18 cycles" and `@budget(cycles: 18)` passed silently; with a `falls_into` the true worst path is 26. **It directly contradicted the module's own stated invariant**, and the identical hole through a plain `nop` WAS refused by an existing test. | **Lens B and Lens C independently**, both with runnable repros | **FIXED** — the return test is per-EDGE (`this_edge_returns = returns && (!two_way \|\| i == 0)`). Pinned both ways: `a_tail_conditional_return_is_refused` (integration) and `a_tail_conditional_return_refuses_its_fall_through` (unit, with the measuring twin at 15/19). |
| **B2** | **`CodeItem::Inline` is invisible to the walk and charged ZERO.** A `DataBuf` spliced into a code stream is not an `Instr`, so `Cfg::build` links straight across it — while on hardware those bytes decode and execute if control reaches them (`$FF` is `rst 38h`). Demonstrated: `nop / dc.b $FF,$FF,$FF,$FF / ret` measured 14 T. `@cycles_exact` also "proved" two arms equal when one carried 6 data bytes. | Lens C | **FIXED** — `[cycles.inline-data]`, refused rather than costed. Blunt (an unreachable jump table is refused too) and stated as such in §6; the sound direction. Ledgered as a class, since four other `Cfg` consumers inherit the same blind spot unexamined. |
| **B3** | **The feature could not be written on a single Z80 proc in the engine.** `@`-attributes were collected only by the top-level item loop; the section-body loop had no attribute pass. All 158 corpus Z80 procs are section-scoped, so `@budget` there was `expected a declaration, found At`. The packet's honesty section asserted "declined an adopter" when the truth was "unreachable feature". | Lens B | **FIXED** — both item loops share one `attach_item_attrs`. Byte-neutral (no corpus item gains an attribute). Pinned by `a_section_scoped_proc_carries_its_budget`. §5 rewritten. |

### Lens A — ceremony / style: 13 findings

Two were rated BLOCKER on the house's own recorded re-spelling rule, and both
were coupling hazards rather than cosmetics:

| # | finding | disposition |
|---|---|---|
| 1 | **`const Z80_RETURN_MNEMONICS` restated a table whose named owner's doc-comment literally says "asks here rather than restating the table"** — `flag_check::is_return_mnemonic`, which was 68k-only. It had to agree with `Cfg::z80_edges`' own inline `matches!` or a `ret` would be reported as an unbounded transfer. | **FIXED** — `is_return_mnemonic(mnem, cpu)` is CPU-parametric, `z80_edges` and `charged_edges` both call it, the const is gone. This is also half of B1's structural fix. |
| 2 | **`entry_instr_idx` was a verbatim copy** of `preserves.rs`'s private one. | **FIXED** — promoted to `flag_check` beside `instr_span`; both copies deleted. |
| 3 | `const Z80_CALL_MNEMONICS` was a fourth spelling of the Z80 call set; `context::is_call_mnemonic` was already CPU-aware with that exact arm. | **FIXED** — `pub(crate)`, reused, const deleted. |
| 4 | "the proc's first instruction's span" open-coded twice inside the new file. | **FIXED** — one `at()` closure over `entry_instr_idx` + `instr_span`; and the verdicts now use the DECLARATION span instead (Lens C 6). |
| 5, 6 | **Two test comments stated arithmetic the assertion three lines below refutes** ("now both arms cost 24" over a body asserted at 34/36; a not-taken sum that drops a `jp .join`), plus a binding named `even` asserted uneven and a docstring describing the wrong half of its own test. | **FIXED** — `rejoined`, correct sums, docstring retargeted. Lens C found the same two independently. |
| 7 | `#[allow(clippy::type_complexity)]` suppressed nothing — verified empirically against the real threshold. | **FIXED** — deleted, and the pair is now a named `struct ChargedEdge { cost, succ }`, so "`None` means the path ends here" is in the type rather than only in a doc. |
| 8 | A redundant `base: Option<u64>` forced an `unreachable!` arm. | **FIXED** — matched on `cost` directly; the `unreachable!` is gone. |
| 9 | Two byte-identical `Edge` arms back to back. | **FIXED** — `Edge::Abandon \| Edge::Defer`. |
| 10 | The `Expr`→`Arg` migration open-coded five reach-throughs of one concept and pushed three lines to 108–123 columns. | **FIXED** — `Arg::str_value()`; all three nested `matches!` collapse. |
| 11 | The header said "Nominal 68000/Z80 clocks" in a module that refuses every 68000 body. | **FIXED** |
| 12, 13 | Three over-100-column header rows; one unpaired `assert_silent` against the file's own stated pairing bar. | **FIXED** (the pairing); the header rows are a markdown table where the 100-col bar is not enforced anywhere in this corpus (`parser.rs` alone carries ~25 pre-existing violations up to 191). |

Lens A found **NO change-history narration** (it grepped every added line for 14
history words and inspected all seven hits), **no brace-indent violation**, and
rated the module header and the test suite above the campaign bar. It verified
the pre-existing-clippy claim independently by `git blame` — correct — and then
found the porter's clippy count was an UNDERCOUNT: 10 workspace warnings, so CI
clippy is red on master too (§6 gap 10).

### Lens B — corpus pattern: the thesis verified structurally, two blockers

Lens B checked the claims rather than believing them, and several came back
stronger than stated:

- **The T-state numbers.** Cross-checked against `oracle-next`'s SingleStepTests-
  validated Z80 core: all six new values correct. It also caught that my own
  Exodus cross-check was the WRONG WITNESS for `reti`/`retn` — Exodus adds no
  cycles for the `ED` prefix and systematically undercounts every `ED` opcode by
  4, so its "10" and my "14" agree once the prefix is accounted, but the
  corroboration only holds through a second source.
- **The `Attr.args` migration**: all 8 master reader sites verified migrated,
  the two-arg `@allow("clobbers.unanalyzable", reason)` form preserved in both
  places that read it.
- **Byte neutrality: airtight, but NOT for the reason I gave.** I argued `Cost`
  cannot reach emission. It can — `span_cost` → `cycles()` → `pad_to_cycles`'s
  `measured` → the emitted pad count. The correct argument is stronger: the only
  value-changing addition is `ret`/`reti`/`retn`, and at master any span
  containing one produced a build-FAILING `[cycles.unknown-op]`. The corpus
  builds, therefore no `cycles()` span contains a return, therefore no span value
  can move. Verified against all 13 real `cycles()` sites. **The packet's §10.1
  argument is corrected to this one.**
- **Alive on real code**: it transliterated three of the nine census procs and
  ran the built CLI — `Snd_AckBump` 40 T, `Snd_ParkDac` 30 T, `Snd_TempoCommand`
  42/50 T — each matching its own hand computation from the table.

| finding | disposition |
|---|---|
| **BLOCKER: tail `ret cc`** (B1 above) | **FIXED** |
| **BLOCKER: section-scoped procs cannot carry the attribute** (B3 above) | **FIXED** |
| **"There is not a second cost model" was FALSE, and the second one is the EMITTING one** — `pad_to_cycles` hard-codes `nop`=4 and `jr`=12 and never calls `instr_cost`. A correction to the table would leave the pad emitting the old count. | **FIXED** — both pad units read `instr_cost` through `pad_unit_cost`. Byte-neutral (the numbers agreed). This is the parcel's own thesis, asserted in a header before it was true in the code. |
| 28 corpus `falls_into` procs get a message describing a NAMED, checked successor as an unknown escape | **FIXED** — the refusal stands (the cost does leave the proc) but the message names the successor. Pinned. |
| **The porter missed a demand site, and it is the strongest in the corpus** — `Process_DMA_Critical`'s "~670 whole-proc worst case" | **FIXED** — §5 rewritten, ledger row corrected, the "manufacturing demand" argument narrowed to the Z80 table. |
| `@allow(id: "x")` silently accepts and discards a keyword name, against the doctrine `pad_to_cycles` states verbatim | **FIXED** — `[attr.form]` on a named argument to an attribute that takes none. |
| Two lint-id families (`[budget.form]` + `[cycles.*]`) for one feature | **FIXED** — one `[cycles.form]`. |
| The `topo_order` comment is inverted | **FIXED** — and the function renamed `postorder`, since post-order is what it returns and what the cost pass needs (Lens C found the same). |

Lens B also argued the module PLACEMENT independently and agreed, with a
sharper reason than mine: a CPU-parametric path walk inside a file named
`z80_cycles.rs` guarantees the wrong home one parcel later, when B′-3b lands.

### Lens C — soundness + the numbers: could not break the model, broke the seam

**Lens C could not find a single wrong T-state** — new or pre-existing — after
checking all 22 entries the walk depends on against `oracle-next`'s core. It
also could not break the algorithm: it verified post-order validity on a DAG,
built a genuine cross edge and confirmed no false `[cycles.unbounded-loop]`,
proved `lo` can never stay `u64::MAX`, ruled out overflow, ran a
20 000-instruction chain (exact to the T-state, no stack overflow), and confirmed
every refusal it could reach — `jp (hl)`, `rst`, `halt`, `di`, `jr cc` to a
proc-closing label, `jr cc` to the very next instruction, `call cc`.

The failures it found were all at the **edge/mnemonic seam**, not in the model:

| # | finding | disposition |
|---|---|---|
| **1 / 1b** | tail `ret cc` (B1) | **FIXED** |
| **2** | `CodeItem::Inline` (B2) | **FIXED** |
| 3 | **Adding `ret` to the shared table REGRESSED the other consumer** — a `cycles(L1,L2)` span containing a return used to bail `[cycles.unknown-op]` and now silently sums past a path end. No corpus span is affected (they would have failed to build), but the guard was gone. | **FIXED** — `span_cost` refuses a return with its own `[cycles.path-end]`, which is the honest statement ("a straight-line sum cannot reach past a return") rather than the accidental one it used to get. |
| 4 | Two `@budget` attributes: the second silently dropped by `.find()` | **FIXED** — `[cycles.form]`, pinned |
| 5 | A misspelled attribute is inert — newly load-bearing for a feature whose value is a proof | **FIXED** — closed attribute name set, `[attr.unknown]`. Closes an OPEN ledger row. |
| 6 | The verdict pointed at an arbitrary interior instruction, which for a spliced body lands in another file entirely | **FIXED** — both verdicts anchor on the DECLARATION's span |
| 7 | Empty body + non-Z80 fabricated `Span { SourceId(0), 0, 0 }` | **FIXED** — folded into 6 |
| 8 | The taken-first invariant is **not** held by the `djnz` arm (an external target drops the branch edge, making edge 0 the fall-through); today only the `edges.len() != 2` guard rescues it, and that guard was not identified as load-bearing | **FIXED** in the doc — `charged_edges` now names both positional readings and says the two-edge guard is what makes them safe. The `z80_edges` dropped-escape-edge is a pre-existing issue for its other consumers; ledgered. |
| 9 | `topo_order` returns a post-order, and both the name and the comment said otherwise | **FIXED** (with Lens B) |
| 10 | Interrupts unmentioned, and `di`/`ei` are off-table so the guard cannot be written inside a budgeted proc | **FIXED** — the header now has a "three things a budget does NOT say" section: nominal T-states, interrupts uncounted, entry-point-zero only |
| 11 | Wrong test-comment arithmetic (with Lens A) | **FIXED** |
| 12 | `an_unfoldable_budget_reports_once` asserted only an absence, not its own name | **FIXED** — renamed and now asserts both halves |
| 13 | `reti`/`retn` unreachable from source | **STATED** — §6 gap 6 |
| 14 | Table coverage makes the feature apply to little more than the DAC inner loop | **STATED** — §6 gap 4 and the ledger, with site counts |
| 15 | Multi-entry procs: paths reachable only from an `export`ed mid-body label are outside the claim | **FIXED** in the header (§6 gap 8) |
| 16 | `n as u64` truncates an out-of-range `i128` ceiling rather than refusing | **DECLINED** — unreachable: the lexer rejects the literal and `[cycles.form]` fires on the negative arm. Recorded here rather than fixed, because adding a branch for a state no input can reach is scaffolding. |

Lens C's verdict before the fixes was "should not merge until 1, 1b and 2 are
fixed with probes and 3 is adjudicated". All four are done.



## §12 — Per-pass findings

### Step 3 — retrospect and LANGUAGE ASKS

**The `Cost::Ambiguous` → `Cost::Split` change is the shape to look for in
future ports.** A model had recorded a fact as *unknowable* when it was merely
*unusable by the only consumer that existed*. The four conditionals' taken and
not-taken costs were both written down — in a COMMENT, on the line above the
bail — because the author knew them and had nowhere to put them. That is the
campaign's recurring pattern (a comment that should have been a checked
declaration) in an unusual place: not in the corpus, but inside the compiler's
own model. Worth stating as a general prompt: **when an enum has a "cannot
answer" variant, check whether it means "no answer exists" or "this caller
cannot use the answer", because the two look identical until a second caller
arrives.**

**A per-pass cycle budget is the language ask this parcel found, and the corpus
wrote its own demand for it.** `z80_sound_driver.emp:489-493`:

```
ensure(cycles(.loop, .exhaust) == 195, "FILL streaming pass must be 195 T-states")
ensure(cycles(.loop, .fill_body) + cycles(.drain, .drain_end) == 195, "DRAIN pass must equal FILL")
ensure(cycles(.loop, .dma_check) + cycles(.draining, .stop) == 194, "DRAINING_TAIL pass must be 194")
```

Three paths, enumerated BY HAND, each composed BY HAND from a prefix span and a
body span, with the cut labels existing solely to make the hand composition
expressible. The walk built here enumerates exactly that path set automatically —
it just refuses to, because the back edge makes a *whole-proc* bound unbounded.
A per-pass budget (a back edge ends the path rather than unbounding it) would
replace all three lines with one declaration and would survive an edit that adds
a fourth path, which the hand-composed form silently would not. It needs its own
spelling, not a mode of `@budget`.

**`Edge::Abandon`'s two meanings cost a second parcel a blocker, in the opposite
polarity, one parcel after the ledger predicted it.** B′-2's row said the
variant conflates a return with a fall-off-end, that its consumer had to
re-derive the distinction from the mnemonic, and — verbatim — "expect it to
recur". It recurred immediately: B′-2 hit a false POSITIVE (a `falls_into` end
charged as a return); B′-3a hit a false NEGATIVE (a `ret cc`'s escaping edge
closed as a return), which is the worse direction. Both lenses found it
independently, which is the same tell B′-2 recorded. **The structural fix —
splitting `Edge::Return` / `Edge::FallOff` — is now demanded by two parcels, and
the second one shows that re-deriving from the mnemonic is not merely inelegant
but insufficient**: the mnemonic is IDENTICAL on both edges of a `ret cc`. The
half that could be done byte-frozen was done here (`is_return_mnemonic` gained a
`cpu`); the variant split is the ledger row.

**`@noreturn` gains its THIRD independent consumer.** B′-0b's ledger asks for it
(a divergent tail cannot be told from a real tail call); B′-2 needs the same
distinction for `[stack.unbalanced]`; `[cycles.unbounded-transfer]` needs it for
the same reason again — a `jp Elsewhere` that never returns owes this proc's
budget nothing, and one that does is a cost the budget must include. Three
analyses, one attribute.

**The extend-don't-replace pattern held for a fifth time, and paid the same
dividend.** Adding a second consumer to `z80_cycles` immediately exposed that
**`ret` was not in the table** — a shipped model that had never been asked to
cost a return, because `cycles(L1, L2)` measures interior spans and every
driver span ends at a cut label. Any budgeted proc ends with a `ret`, so the
new consumer hit it on its first test. Same shape as B′-2's `link`/`unlk`
finding, and the same general lesson: **a substrate with one consumer is only
tested on the paths that consumer takes.** This is now the third recorded
instance; it should probably be a standing pre-flight question rather than a
per-parcel discovery.

### Step 5 — engine optimize

**Nothing shipped**, and that is correct for a diagnostics parcel: byte-neutral
×7, no codegen path touched.

The census does hand the engine two facts it did not have:

1. **Nine Z80 procs have measured worst-case costs** (§4), so any future
   argument about sound-driver timing starts from numbers rather than from
   reading. `Snd_FadeCommand`'s 76–99 T spread is the widest, and it runs once
   per command rather than on the sample clock, so it is not a target — but the
   number exists now.
2. **123 of 158 Z80 procs are not boundable, and the reason distribution is
   measured** (~55 tail transfers, ~35 loops, ~35 table gaps, ~10 calls). The
   tail-transfer count is the interesting one: it is much larger than the loop
   count, which says the sound stack's dominant shape is proc-to-proc `jp`
   chaining rather than looping. That is an engine fact worth having before
   anyone proposes a timing budget for it.

### Neither bucket — the headline (POST-panel)

**I measured the corpus carefully, concluded "no adopter", and the panel showed
the measurement was aimed at the wrong half of the machine.** My census was
structural — which procs COULD be bounded — and it correctly found nine Z80
candidates and no obligation among them. Lens B did the search I should have
done instead: it looked for procs whose AUTHOR had already stated a whole-proc
cycle claim, and found `Process_DMA_Critical`'s "~670 whole-proc worst case,
68000 table timing" sitting between two `ensure(...)` guards written to protect
exactly that derivation. One proc, and it reframes the parcel: the shipped half
has no adopter because the demand is on the side I deferred, not because the
construct has no customer.

**The general lesson is about how to measure demand.** "Which procs can the
analysis handle" is a fact about the analysis. "Which procs have already written
the claim by hand" is a fact about the corpus, and it is the one that decides
whether a construct earns its keep. I ran the first census and reported it as
though it answered the second question. It did not, and the difference was one
grep.

The rest of the honest outcome stands: the Z80's real timing obligations are
*span*-scoped and already served by `cycles(L1, L2)`, its one whole-body
obligation is *per-pass* and structurally out of reach, and the 68000 — where
235 procs are structurally boundable and one is asking in prose — has no cost
model. All three are follow-ups whose scope is now written down. None of it was
knowable before the walk existed, which is the argument for having built it.
