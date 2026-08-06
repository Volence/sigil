# The enumerated-dispatch form `targets(...)` — parcel packet (2026-08-05)

Spec: `specs/2026-08-05-enumerated-dispatch-design.md` (RULED, Fable), amended on
master `a5a24390` ("targets() names the landing points, and the example stops
teaching the trap" — the §1 example now shows the landing-label spelling; this
adoption follows it). Closes the gap-ledger enumerated-dispatch row
(`[bprime-3b, 2026-08-05]`); links row 1140 (typed jump-table) as the
exhaustive-by-construction successor.

## Numbers first

- **Adoption walk = 670 cycles**, pinned as `@budget(cycles: 670)` on
  `Process_DMA_Critical`. The three numbers now MEET: prose ~670 / B′-3b table 670
  / walk 670.
- **Byte bar: all SEVEN ROM anchors byte-identical** — s4 b5be8fef/383336 · s4.debug
  8448717a/391000 · demo 1a72e3e0/70180 · demo.debug efd16be5/70180 · config_a
  bf1dde89/391000 · config_b db32d41b/273808 · lean d32cee18/379110 (lean has no
  appendix, so its full == anchor and is identical outright). The `targets(...)`
  labels emit ZERO ROM bytes; the assembled ROM does not move.
- **full_crc drifts on the six deb2-carrying shapes (appendix-only)** — the nine
  `.slot_k` landing labels grow the sigil-canonical deb2 symbol table per shape
  (s4 +171, s4.debug +191, demo +171, demo.debug +178, config_a +178, config_b
  +173 B; lean unchanged — no appendix; and the header checksum shifts), so the
  FULL-FILE CRC moves while the ROM does not. The `native_full_rom` gate pins the
  full-file, so
  this is a GOLDEN REFREEZE (chain **47→48**, ruled Option 1 — see below), not a
  free drift. lean (appendix-free) is identical outright.
- Warn tiers UNCHANGED: plain 19 / DEBUG 18, id-identical. `pins.rs` UNCHANGED (the
  ROM anchor did not move); `refreeze --check: OK (tip enum-dispatch, chain len 48)`.

## Golden refreeze — chain 47→48 (anchor-primary doctrine)

The nine landing labels are real code locations (`.jump_table + k·14`); the deb2
symbol appendix naming them is the appendix doing its job. The ASSEMBLED-ROM ANCHOR
is the primary provenance class precisely because the appendix drifts with symbol
changes by design — and all seven anchors held byte-identical (lean identical
outright), which IS the chain-48 entry's evidence (no A/B ref needed; `--ab ""`).
Refrozen via the standard procedure (`refreeze --freeze enum-dispatch` →
`capture_goldens.sh --write` + `derive_offcanonical_sizes.sh` + `repin` + chain
append). Appendix growth per shape: s4 +171, s4.debug +191, demo +171, demo.debug
+178, config_a +178, config_b +173 B (lean unchanged).

REFUSED alternative (Option 2): suppressing the `.slot_k` label class from deb2 to
hold the checksum — that hides true code locations from the debugger to preserve a
number, and is refused (overseer ruling).

Chain-48 CRC table (full = new after append; anchor = unchanged from chain 47):

| target | full_crc / size | anchor_crc / end |
|---|---|---|
| s4 | b5ffb094 / 411267 | b5be8fef / 383336 |
| s4_debug | 57fd08f9 / 423671 | 8448717a / 391000 |
| demo | cbddc142 / 91429 | 1a72e3e0 / 70180 |
| demo_debug | b61f462d / 94133 | efd16be5 / 70180 |
| config_a | 61e4e78e / 424049 | bf1dde89 / 391000 |
| config_b | 07e3f465 / 301305 | db32d41b / 273808 |
| lean | b92cb485 / 379110 | d32cee18 / 379110 |
- Strict gate close (failures-first: 0 FAILED, 0 panics): base `#[test]` at master
  `326809e5` = 3369; +26 (12 unit in `cycle_budget.rs`, 14 integration in
  `tests/cycle_budget.rs`) = 3395. Final run: PASSED + IGNORED = 3395 (4 ignored).
  Full strict green with AEON_DIR = the b7 aeon worktree; `native_full_rom`
  plain+debug pass against the chain-48 goldens. clippy `-D warnings` clean.

## The three numbers reconciled (spec §4) — they MEET

| figure | value | what it counts |
|---|---|---|
| prose (dma_queue comment) | ~670 | full whole-proc worst case WITH the per-slot dispatch prologue |
| B′-3b table (cycle-exact) | 670 (662 post-relaxation) | the same chain, priced to the cycle |
| **the walk** | **670** | the enumeration names the PHYSICAL landing labels, so the prologue is ON the charged path |

The `targets(...)` set names `.slot_0..8` — each `.slot_k` label sits at exactly
`.jump_table + k·sizeof(DMAEntry)`, the byte the computed `jmp` actually reaches,
so the walk charges each slot's own dispatch body before following into the drain.
The dearest is the slot-8 arm: `movea.w Sym` 16 + `suba.w #imm` 12 + `jmp d16(An)`
10 + `lea VDP_CTRL` 12 + `lea DMA_Critical` 12 + 8×72 drain 576 + `move.w
#imm,(abs).w` reset 16 + `rts` 16 = **670**. Every relaxable operand is charged its
abs.l ceiling; the linker's abs.w widths make the true worst 662, and `@budget`
holds either way.

**The drain-label trap (corrected):** the first cut named `.done, .drain_1..8` —
labels DOWNSTREAM of the landing points — and measured 646, exactly 24 (the two
slot-8 leas) under the true ceiling. A `@budget` the hardware can exceed by 24 is
the false-confidence failure the construct exists to prevent. The premise that the
table slots were "unnameable" was false: a label placed in the proc body before
each `jt_slot(...)` sits at the true landing offset and emits no ROM bytes. The
regression pin `targets_charge_the_landed_on_code_not_a_downstream_label` fixes the
class — a downstream enumeration MUST measure smaller than the landing one.

Measurement method: set `@budget(cycles: 1)` in the b7 aeon worktree, `sigil build
--config-b`, read `[cycles.over-budget] … costs 670`, then pinned 670. The prose
and the B′-3b table were NOT adjusted (they already said 670).

## What shipped

- **Grammar** (`parser.rs`, `ast.rs`): a trailing `targets(.a, .b, …)` clause on an
  instruction line, parsed after the operands and the `as ContractType` bound,
  before `@discards`. Empty `targets()` is refused at parse (the
  `dispatch.targets-empty` id). AST `InstrLine.targets: Vec<String>` (source-spelled
  names).
- **Carry** (`value.rs`, `eval/asm.rs`): `CodeItem::Instr.targets: Vec<String>`,
  each name resolved through the proc's label scope (`LabelScope::resolve_ref`) so
  it matches the mangled `CodeItem::Label` symbol the CFG keys on. Emits nothing.
  A `dc` directive lowers to inline DATA before the targets resolution, so a stray
  `dc.w 5 targets(.x)` is refused there loudly (`dispatch.targets-on-data`) rather
  than silently dropped.
- **Consumption** (`cycle_budget.rs`): `enumerated_succs` is the ONE consumer — a
  bare `Defer` (a `jmp`/`jp (hl)` naming no symbol) carrying a non-empty `targets`
  becomes N `Follow` edges, the fixed cost charged once, exactly as a two-edge
  branch fans out. Both `postorder`'s successor closure and `charged_edges` route
  through it, so the topo order and the cost pass agree. No `Cfg` edge builder
  changed — the arm is self-contained (the noreturn lane's later `charged_edges`
  arm composes). The `[cycles.computed-transfer]` / `[cycles.unbounded-transfer]`
  messages gained a pointer to the form. `z80_cycles` gains `jp (hl)` = 4 T so the
  Z80 computed dispatch is measurable (unenumerated it is still a computed-transfer
  refusal — the structural check comes first).
- **Refusals** (`cycle_budget::check_dispatch_targets`, wired at the SINGLE
  `lower_code_buf` chokepoint every code buf funnels through — a named proc, a
  dispatch-table inline body, a script body, or an item-position `asm {}` template,
  so no path can carry a clause unchecked, and a proc never double-reports because
  its validation no longer runs a second copy): `dispatch.targets-on-call`,
  `dispatch.targets-redundant` (an already-exact edge or a non-transfer),
  `dispatch.target-unknown` (undefined `.local`), `dispatch.target-nonlocal`
  (cross-proc/global), `dispatch.target-duplicate`, and `dispatch.target-trailing`
  (a local label that closes the body — a fall-off, not a landing point; the b8
  reading, which otherwise produced contradictory diagnostics). ERROR-tier,
  unsoftened (author-written claim, like a budget).
- **Adoption** (`engine/system/dma_queue.emp`): `Process_DMA_Critical`'s
  `jmp .jump_table(a1)` gains `targets(.slot_0..8)` — nine zero-byte labels at the
  physical landing points — and the proc gains `@budget(cycles: 670)`; the header
  comment shows the three numbers meeting at 670.

### §4 non-adoptions, each keeping its refusal (recorded, not omitted)

- `animate.emp .cc_table-4` — arms carry `jsr (a2)` (opaque-call); refused
  `[cycles.opaque-call]`.
- `player_sensors` ×2 — arms `jbsr` the probes (opaque-call).
- `player_common` — cross-proc targets + a `jsr` twin (opaque-call + nonlocal).
- `s4lz` ×2 — the "targets" are unrolled instruction addresses, not labels: not
  this form's shape, and the loop refusal binds anyway.

## Soundness (spec §2), stated

Exhaustiveness is the AUTHOR's claim. The compiler verifies existence, locality,
and distinctness of every named label — NOT that the runtime index stays inside
the table (that is the site's `andi` clamp + `ensure` geometry, which dma_queue
already writes). Blast radius is confined because ONLY the opt-in cycle-budget walk
reads the clause: the preserves prover, the flag walks, and the clobbers closure
keep treating the instruction as an opaque computed transfer. A pin proves it —
`enumerated_targets_leave_the_base_edge_model_untouched` asserts `flag_check::Cfg`
edges for the annotated jmp are `[Defer]` identically with the clause and without
it. A wrong enumeration can therefore mis-measure only the budget its author asked
to have measured, and can corrupt no soundness-bearing analysis. The day a prover
must see through a dispatch, the member list must come from a structure the
compiler OWNS — the typed jump-table decl of ledger row 1140 — not a `targets(...)`
assertion. The two ledger rows are now cross-linked.

## Tests

Unit (`src/cycle_budget.rs`): the miniature drain shape measures (26/66);
without-clause refuses `[cycles.computed-transfer]`; the STRENGTHENED orthogonality
pin (`enumerated_targets_leave_the_base_analyses_untouched` — compares the
preserves-prover, flag def-use, and stack-balance VERDICTS with-and-without the
clause, not just Cfg edges, so a future direct-item-reader consumer flips it red); a
perturbed drain arm moves the budget (§5 perturbation pin, fixture not corpus); the
**drain-label-trap regression pin** (`targets_charge_the_landed_on_code_not_a_
downstream_label`: a downstream enumeration measures strictly smaller than the
landing one, 54 vs 46 — the walk charges landed-on code); a trailing-label target
refused; a Z80 `jp (hl) targets(...)` measures (18); an enumerated cycle falls to
`[cycles.unbounded-loop]`; the six validity refusals + a passing twin.

Integration (`tests/cycle_budget.rs`, full pipeline incl. label mangling): the
positive `@budget` pin (66 verifies clean, no refusal); over-budget names 66;
without-clause still refuses; the validity refusals end-to-end (on-call, redundant,
unknown, nonlocal, duplicate, trailing, on-data); the empty-set refusal at parse;
`as T` + `targets(...)` compose in the grammar; and the clause is checked on the
non-proc funnels — a DISPATCH inline body and a SCRIPT body each refuse a
`targets(.typo)` through the shared `lower_code_buf` chokepoint.

## Per-pass findings

- **step-3 (retrospect):** the walk already handled arbitrary `Follow` fan-out
  (the DAG min/max), so consumption was a five-line arm, not a rewrite — the spec's
  scout ground truth held. The corrected design point: `targets(...)` must name the
  PHYSICAL landing labels, not labels downstream of them, or the ceiling under-
  counts the dispatch prologue that executes. The "unnameable table slots" premise
  was false — zero-byte labels in the proc body sit at the true landing offsets —
  and the fix keeps the three reconciled numbers MEETING at 670.
- **step-5 (engine):** none — ROM byte-identical by construction (labels emit no
  code bytes); no engine optimization in scope. `@budget(670)` is now a regression
  pin: a slot/drain-geometry change that moves the worst arm fails loud here, and
  the ensure-pinned stride and the `.slot_k` label set now say the same thing.
- **neither-bucket headline:** the full_crc-vs-anchor split. The nine landing
  labels are ROM-neutral (every anchor held) but grow the deb2 symbol table, so the
  `native_full_rom` full-file gate fires and the goldens are REFROZEN (chain 47→48).
  The anchor-primary doctrine is what makes this a routine appendix refreeze rather
  than a byte regression: the ROM did not move, only the debug symbols naming the
  new code locations did. Suppressing those symbols to hold the checksum was
  refused (it would hide real locations from the debugger).

## Land order (measured)

sigil merges FIRST. Evidence: the b7 aeon corpus now carries the `targets(.slot_0..8)`
clause AND `@budget(cycles: 670)` on `Process_DMA_Critical`; an OLD sigil (master,
no `targets` grammar, no `@budget` 68k walk) parsing that source errors at the
`targets` token — it is not grammar it knows. The b7 aeon corpus builds only with
the b7 sigil (all seven ROM anchors byte-identical; the six deb2 fulls refrozen at
chain 48). The ordering is forced by the parser, not assumed.

## Spec conformance (§4)

"Measures what the walk actually computes" now lands on the TRUE ceiling: the
enumeration names the physical landing labels, so the walk's 670 is the real
worst-case path (the same chain the prose and B′-3b table price), not a model that
fans past executing instructions. The construct now does what it exists to do —
turn the hand-derived ceiling into a machine-checked one that the hardware cannot
exceed.

## Adjudicated

- **Wiring — ACCEPTED, then consolidated.** The first cut wired the dispatch-
  validity check in `lower/proc.rs` (accepted by the coordinator). The panel then
  took the wider fix: the check now runs at the SINGLE `lower_code_buf` chokepoint
  every code buf funnels through (named proc, dispatch inline body, script body,
  item-position `asm {}` template), and the `lower/proc.rs` call is removed so a
  proc never double-reports. No preserves/flag/closure behavior touched.

## Commits (for the queue)

sigil (branch `enum-dispatch`, off `326809e5`):
- `32ca21e4` — the feature (grammar, carry, consumption, refusals, the drain-label-
  trap pin) + the pre-existing mul_lower `redundant_closure` fold for `-D warnings`.
- `02ff1f00` — the docs (packet + gap-ledger + B′-3b §6 pointer).
- `19969cc7` — the golden refreeze (chain 47→48).
- this round's commit(s) — the consolidated panel fixes (centralized dispatch
  check on the three funnels + `dc` + trailing-label refusals; strengthened
  orthogonality pin; `as T`/`jp (hl)` tests; `jp (hl)` cost; packet/ledger/docs).

aeon (branch `enum-dispatch`, off `b9b1056`):
- `de078af` — `dma_queue.emp`: the `targets(.slot_0..8)` clause on the physical
  landing labels + `@budget(cycles: 670)`.
- this round's commit — the ensure-message + comment corrections.

Scope touched: `parser.rs`, `ast.rs`, `value.rs`, `eval/asm.rs`, `cycle_budget.rs`,
`z80_cycles.rs`, `lower/code.rs` (the centralized check), `lower/proc.rs` (call
removed), the `targets:` field threaded through every `CodeItem::Instr` /
`InstrLine` construction site, tests, and docs (this packet + gap-ledger + B′-3b §6
pointer) + the golden refreeze (chain 48). One aeon file (`dma_queue.emp`). No
preserves/flag/closure consumer touched; `charged_edges` not restructured beyond
the self-contained targets arm. Byte bar: all seven ROM anchors byte-identical;
full_crc drift on the six deb2 shapes is appendix-only (chain-48 refreeze); warn
tiers 19/18 unchanged.
