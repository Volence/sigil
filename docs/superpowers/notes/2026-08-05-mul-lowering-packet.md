# 2026-08-05 — mul-lowering parcel packet (`mul_const` / `mul_bounded`)

Lane: b4 pair (sigil `mul-lowering` off `2c6507b8` chain 47 · aeon `mul-lowering`
off `2ccb40f`). Porter parcel; no merge-state claims — commits listed at the end
for the merge queue.

## 1 · The construct surface shipped

Two 68k-only mnemonic-position words (the `jbra`/`dc` reservation class — a
comptime fn can never shadow them; tenet 3):

```
mul_const   dN, #n              // dN.l = u16(dN.w) × n     (n comptime, 0..=$FFFF)
mul_const   dN, #n, dScratch    // same; scratch may come back clobbered
mul_bounded dD, dSrc, #M        // dD.l = u16(dD.w) × u16(dSrc.w), src ≤ M (inclusive)
mul_bounded dD, dSrc, #M, dScr  // same; scratch enables the loop candidate
```

Result contract pinned to `mulu.w`'s: all 32 result bits valid, condition codes
clobbered-undefined, `mul_bounded`'s src clobbered-undefined under EITHER
lowering. No size suffix exists (`[mul.size]` refuses; one contract, one
meaning — design §1/§5).

**Where it lives.** `crates/sigil-frontend-emp/src/mul_lower.rs` (recognition,
validation, candidate generators, cost decision, expansion — plus 13 unit
tests); `lower/code.rs` (recognizer membership + the item-position safety net +
`encoded_len_m68k`, the encoder-backed byte-length seam); `eval/asm.rs` (ONE
choke-point call at CodeBuf completion — see §3); `sigil-isa/m68k_cycles.rs`
(the value-aware MULU row, §4); `tests/mul_lowering.rs` (9 integration tests).
No `value.rs` variant was added and `lower/proc.rs` is untouched (the lane
boundary): the construct expands into EXISTING CodeItem shapes.

**The one boundary note for the overseer:** the design's "declared through the
ordinary clobber machinery" is only achievable by expanding BEFORE the analyses
walk the buffer, and the single point where every proc body's CodeBuf completes
is `eval_asm_owned`'s tail (eval/asm.rs). That is a 6-line call into the
lane-owned module — but eval/ is the b3 porter's declared territory, so this
touch is flagged prominently: it is additive, at the function's last statement,
and all logic lives in `mul_lower.rs`. If b3 touched `eval_asm_owned`, merge
attention lands on those 6 lines.

## 2 · Owned decisions, stated and defended

**Operand order = the design signature's order** (`mul_const(reg, n, scratch)`
→ `mul_const d0, #66, d1`), not 68k src-first. Defence: the construct's
published contract is the design doc's signature; a reader cross-referencing
the spec must not find the arguments permuted, and the words are emp-only —
no native-mnemonic muscle memory binds them. The cost: `mul_const d0, #66`
reads dst-first beside `mulu.w #66, d0`. Judged acceptable because the
mnemonic names the abstraction, not the instruction.

**Tie-breaking (determinism):** fewest worst-case cycles → fewest encoded bytes
→ earliest candidate in a FIXED enumeration order, with `mulu` enumerated
first. So an exact tie resolves to `mulu` (the design's "then mulu (simpler)"),
and among chains, scratch-less precedes scratch-using. This is total, stable
across shapes, and load-bearing: the ×66 chain TIES `mulu` at 46 cycles and
loses on bytes (12 vs 4) — pinned by test. Byte counts come from the encoder
itself (`encoded_len_m68k`), never a shadow length table (the B′-3a drift
class).

**`muls` / signed contexts: refused, not guessed.** No signed variant exists in
v1 — the design defers it (§5, zero corpus demand: every corpus `muls` has a
runtime operand) and signedness is not inferable at a bare register operand.
The unsigned contract is stated in the construct's one meaning; a signed site
simply cannot spell the construct and keeps `muls`.

**`mul_bounded`'s bound is mandatory and INCLUSIVE (src ≤ M).** The design
writes both "0..M" and "src ≤ M"; an instruction operand is a scalar, not a
range, and the inclusive reading is the conservative resolution of the
ambiguity (cost charged at M covers M−1). An unbounded spelling does not parse
(`[mul.operands]` names the refusal stance). Cost is worst-vs-worst: `mulu`'s
UM ceiling 70 vs the loop's 28 + 18·M (M ≥ 1; 26 at M = 0) — the loop wins
only through M = 2, so any real bound picks `mulu`, and that verdict is now a
computed fact (§6, the R-B closure). Degenerate M = 0 is legal and lowers to
the loop shape (guard exits immediately; correct for the only in-bound src).

**Register/flag effects — how the silent-scratch-clobber failure mode is
prevented STRUCTURALLY.** The expansion runs at CodeBuf completion, before
`check_clobbers` / `preserves` / `check_out` / flag_check / cycle_budget walk
the items. Every analysis therefore sees the chosen lowering's ordinary
instructions: the chain's `move.l dN, dS` is an ordinary
`instr_written_regs`-detected write, so a proc that declares `clobbers(d0)`
but hands `d1` as scratch gets `[proc.clobber-undeclared]` on `d1`
(integration-pinned both polarities), a declared `preserves(dS)` fails its
entry-value proof, and `out(dst)` verifies against the real dst writes. No
analysis table learned a `mul_*` row — there is nothing to drift. A raw item
cannot outlive expansion on any CPU-resolved path; the two escape paths are
handled: a cpu-less template expands when the CPU-resolved buffer finally
carries it, and item-position streams expand in `lower_code_buf` via the SAME
pure function (bytes cannot differ by path).

**Aliasing and register class:** all operands must be data registers
(`[mul.reg-class]` — mulu's destination and `lsl` demand Dn); scratch must
differ from dst (`mul_const`) and from both dst and src (`mul_bounded`) —
`[mul.scratch-aliases]`. `mul_bounded dst == src` (a square) is legal: `mulu`
handles it and the loop candidate is structurally excluded (src IS the loop
counter), deterministically.

**Missing scratch is a silent (correct) narrowing, not a diagnostic:** without
scratch the candidate set is {mulu, degenerates, pure shifts} — mulu is always
legal, so falling back silently is the design's own ruling; the diagnostic
surface is for genuinely illegal inputs only.

**Loop label hygiene:** loop labels are `$mul$<module>$<k>$loop/done` — the
module id embedded per the hygiene model's own convention (a counter alone
would collide across modules; caught in self-review, unit-pinned).

## 3 · Candidate generators (fixed, bounded)

For `mul_const(n)`: (1) `mulu.w #n, dst` — always; (2) n = 0 → `moveq #0`;
(3) n = 1 → in-place zero-extend `swap/clr.w/swap` (12 cycles, beats
`andi.l #$FFFF`'s 16); (4) n = 2^k → zero-extend + shift-run (scratch-free);
(5) with scratch and ≥ 2 set bits: (a) left-to-right binary over the seed
`moveq #0,S / move.w D,S / move.l S,D` (both registers hold zx(x) for 12
cycles), doubling runs run-coded (single double = `add.l D,D` at 8 vs
`lsl.l #1`'s 10; runs ≥ 2 = `lsl.l` chunks ≤ 8); (b) the subtract form
n = (2^a − 1)·2^b (×63 = x·64 − x). Every straight-line candidate must price
EXACT through `instr_cost` (unit-asserted across the whole n sample — an
inexact candidate would make the decision depend on a ceiling).

Computed boundaries, pinned by tests: powers of two flip chain→mulu between
2^8 (36 cycles) and 2^9 (46 vs mulu's 44); ×3 chain = 28 vs 46; ×63 subtract
form = 40 vs LTR 92 vs mulu 54; every corpus stride resolves to mulu
(66: 46/46 tie → bytes; 80: 48 vs 46; 160: 50 vs 46; 36 and 40: 46/46 ties).

## 4 · Cost-model integration: consumed, and one extension

**Consumed:** all pricing flows through `crate::m68k_cycles::instr_cost` — the
one classifier the cycle-budget walk already uses, over the B′-3b ISA table.
The loop candidate's worst case is assembled from the same seam
(`moveq`/`move.w`/`subq` costs, `bcs`'s Branch pair, `dbf`'s taken/expiry pair)
— zero literal cycle numbers in `mul_lower.rs`.

**Extended (with citation + pins):** `sigil_isa::m68k_cycles` gains the
value-aware MULU row: `(Mulu, [ImmVal(v), Dn])` prices EXACT at
`38 + 2·ones(v) + 4` — M68000UM Table 8-4's footnote (`38+2n, n = the number
of ones in the <ea>`) applied to a compile-time-known source, plus the
Table 8-2 immediate fetch. The all-ones pin (74) meets the pre-existing
ceiling row, so the exact form and the maximum cannot drift apart. MULS
deliberately keeps its ceiling (its n counts 01/10 transitions of the 17-bit
sign-extended source; value-aware pricing has no consumer, and an unconsumed
exact row is an untested claim). Corpus blast radius of the flip
inexact-74 → exact-42..74 for `mulu #imm`: ZERO — the corpus declares no
`@budget`/`@cycles_exact` anywhere (grepped), and the frontend's only mulu
cost pin uses an `(a0)` source.

## 5 · Adoption census (construct-first discipline held)

**Adopted, byte-identical (2 sites, aeon):**
`games/sonic4/test/object_test_state.emp` — `mulu.w #36, d0` and
`mulu.w #40, d0` (the churn-grid pitches) → `mul_const d0, #36` / `#40`. The
sites pass no scratch, so mulu is the SOLE candidate there (Lens C's
correction of this porter's grander tie-break narrative — the 46-cycle
chain-vs-mulu ties are the with-scratch fact, unit-pinned); either way the
emitted instruction IS the hand instruction, same mnemonic/size/operand
order — all seven golden targets `cmp`-identical (§7). These two sites put
the construct in the living corpus on day one.

**Rejected, with numbers (4 chain sites + 2 loop sites — NOT taken; each is a
word-width contract mismatch, not merely a cost delta):** every hand chain in
the corpus is WORD-width, licensed by a module `ensure` bounding the product
to a word; the ratified long contract must price its zero-extend, and then
mulu ties-or-wins every stride — so no chain site can be byte-identical:

| site | hand sequence | hand cost | `mul_const` emits | delta |
|---|---|---|---|---|
| section.emp `Section_GetSecPtrXY` ×66 | `move.w/lsl.w #6/lsl.w #1/add.w` | 34 cy / 8 B | `mulu.w #66` | +12 cy, −4 B, word→long |
| tile_cache.emp `TileCache_DecompressBlock` ×66 | same shape | 34 cy / 8 B | `mulu.w #66` | +12 cy, −4 B |
| tile_cache.emp `mul_cache_stride` ×80 (comptime fn + caller copy) | `move.w/lsl.w #6/lsl.w #4/add.w` | 40 cy / 8 B | `mulu.w #80` | +6 cy, −4 B |
| plane_buffer.emp ×160 | `move.w/lsl.w #7/lsl.w #5/add.w` | 44 cy / 8 B | `mulu.w #160` | +2 cy, −4 B |
| section.emp `.gxy_mul` repeated-add (mul_bounded shape) | word loop | ~14·sec_y + setup | `mulu.w` (70 ceiling) | word→long; §4a defers adoption anyway |
| tile_cache.emp `.mul_loop` repeated-add | word loop | same class | `mulu.w` | same |

Step-5 finding for the overseer: the design's own byte-identity acceptance bar
("retrofit ONE ×66 site") is UNSATISFIABLE under the ratified v1 contract —
the hand chains win only because they are word-width (the ×66 word chain is 34
cycles against the long contract's 46). Recorded as the design's anticipated
negative result; the sized-variant demand (`mul_const.w`, upper word
unchanged-garbage, author owns the range proof) is a gap-ledger row with all
four sites as its demand census — a language-round ask, not this lane's call.

**Refused:** nothing — no site presented an unknowable bound or signedness
(the corpus `muls` sites never entered scope; signed is refused by v1's
surface, §2).

## 6 · R-B, closed structurally

The 2026-08-02 adjudication's open ruling R-B (mulu-vs-repeated-add,
section.emp) is closed in place (annotation added to the adjudication doc):
the construct makes the choice the cost model's, and the verdict at any real
bound is `mulu` — ceiling 70 vs loop 28 + 18·M, loop wins only through M = 2.
The section.emp site itself stays as-written until a byte-changing parcel
adopts `mul_bounded` (design §4a), now blocked only on the sized-variant
ruling.

## 7 · Bars

- **Byte bar:** all SEVEN golden targets `cmp`-identical in
  `capture_goldens.sh` order (the four canonical/demo shapes via `./build.sh`,
  config-a/config-b/lean via `sigil build --config-*`, canonical restore
  after — driven by an uncommitted lane script; the procedure, not the
  script, is the bar), WITH the two aeon adoption edits in the tree — baseline
  BYTEBAR_RC=0 proven pre-change at chain 47 (binaries rebuilt at HEAD first),
  and again at convergence with binaries rebuilt at the new HEAD:
  BYTEBAR_RC=0, s4/s4.debug/demo/demo.debug/config_a/config_b/lean all OK,
  canonical restore OK. No target moved.
- **refreeze --check:** OK (tip `sst-fold`, chain len 47). **repin --check
  (against the b4 aeon):** `pins.rs unchanged`.
- **Warn-tier id sets:** plain 19 (module.path-mismatch 9,
  proc.undeclared-fallthrough 6, proc.out-unwritten 3, proc.clobber-undeclared
  1) and s4-DEBUG 60 (proc.sr-undeclared 42, module.path-mismatch 9,
  proc.undeclared-fallthrough 5, proc.out-unwritten 3, proc.clobber-undeclared
  1) — id sets AND counts identical base vs convergence (captured both runs).
  demo-DEBUG's warn line (59) was not captured at base (tail truncation in the
  base capture) but is structurally unreachable by this parcel: the demo
  corpus contains no construct item, the choke-point pass is a no-op scan
  there, and demo.debug.bin is byte-identical — stated, not assumed.
- **Full strict `SIGIL_STRICT_GATE=1`:** branch point — proven with a caveat
  owned below; convergence — <FILL-CONV>. Failures first: <FILL>.
- **An owned process fault (reported, not hidden):** the branch-point strict
  started clean (no cargo running, per the brief), but this porter then ran
  TARGETED cargo tests in the same target dir mid-capture AND landed the aeon
  adoption edit mid-run — so the base capture is degraded (suites recorded
  incompletely) and its 2 recorded failures are both
  `native_offcanonical_placement` tests that read the SHARED aeon corpus after
  the adoption edit landed: base binaries cannot parse `mul_const`, so they
  failed on this lane's own mid-run contamination, not on base health (every
  other recorded suite green; byte bar at base was RC=0 pre-edit; both tests
  green at convergence). Lesson for the lane notes: the stagger rule applies
  to YOUR OWN targeted runs against YOUR OWN strict, and the corpus is an
  input to a running gate — do not edit it mid-capture.
- **Test arithmetic:** base `#[test]` count 3321. Added 23, every one named:
  sigil-isa `mulu_immediate_prices_on_its_ones_count`; mul_lower unit
  `every_chosen_lowering_matches_mulu_semantics`,
  `mul_bounded_matches_mulu_semantics_within_bound`,
  `corpus_strides_resolve_to_mulu`, `small_factors_win_as_chains`,
  `degenerates_and_powers_of_two`,
  `power_of_two_cost_boundary_flips_at_2_to_the_9`,
  `missing_scratch_falls_back_to_mulu`,
  `mul_bounded_cost_boundary_and_candidate_gating`,
  `loop_labels_embed_the_module_id`, `expansion_is_deterministic`,
  `every_straight_line_candidate_prices_exactly`, `refusals_are_tagged`,
  `z80_refuses_and_cpuless_defers`; integration
  `mulu_winning_mul_const_is_byte_identical_to_hand_mulu`,
  `scratch_clobber_is_seen_by_the_clobber_lint`,
  `out_contract_sees_the_expanded_writes`, `chain_bytes_are_pinned`,
  `bounded_loop_labels_link_and_are_unique`,
  `bounded_above_boundary_is_byte_identical_to_mulu`,
  `lowering_is_deterministic_end_to_end`, `z80_bodies_refuse_the_construct`,
  `refusals_surface_through_the_pipeline`. Expected 3344: <FILL-VERIFY>.

## 8 · Equivalence proof (design §3's bar)

The unit oracle executes every CHOSEN lowering (and the loop shape) over
concrete registers — upper-word garbage seeded — across 23 multipliers
(corpus strides, ±1 forms, powers of two, degenerates, $8001/$AAAA/$FFFF) ×
9 boundary x values × 3 garbage patterns, with and without scratch, asserting
exact u32 equality with `zx(x) × n`; `mul_bounded` sweeps every in-bound src
for M ∈ {0,1,2,3,16}. The oracle panics on any mnemonic outside the emitted
vocabulary, so it cannot silently under-check as generators grow. The ×3
chain's linked bytes are additionally pinned literally
(`chain_bytes_are_pinned`).

## 9 · Lens findings and dispositions

Three fresh read-only lenses (A ceremony · B soundness · C cost decision), all
run against the committed diff `2c6507b8..d0067ce6` + aeon `f52d247`; the
fix-ups below landed as the follow-up commit.

**Lens A (ceremony): 2 should-fix, 4 nits, no must-fix.**
1. Duplicated `[mul.non-68k]` string across the two expansion sites →
   **FIXED**: one `mul_lower::non_68k_err` spelling, both sites call it.
2. `bytebar.sh` untracked-but-cited → **FIXED in the packet**: the bar is the
   `capture_goldens.sh` PROCEDURE; the lane script stays uncommitted scratch.
3. Aeon comments narrating the compiler's pick → **FIXED**: trimmed to
   `// ×36 grid pitch` / `// ×40 grid pitch`.
4. `expand_item` determinism doc overstated (labels carry module+counter) →
   **FIXED** (clause added).
5. `"item"` sentinel shares module namespace → **FIXED** beyond the ask: the
   net now namespaces by `item<source-id>` + span start (also closes Lens B's
   info (a)); the comment owns the loud-collision fallback.
6. Idioms-entry title mismatch in the packet → **FIXED** (§10 wording).
   Clean categories: zero change-history narration, all six diagnostics name
   rule + why, precedent-consistent guards, ledger rows well-formed.

**Lens B (soundness): NO must-fix, NO should-fix; the central claim verified
by tracing every `CodeItem::Instr` producer and every CodeBuf consumer** —
all proc-analysis paths run post-expansion (`eval_proc_body_env` always sets a
CPU, so no proc body completes cpu-less); the safety net is genuinely
unreachable today (defense in depth); label hygiene proven structurally
(4-segment `$mul$…` pattern unreachable by hygiene/user names; counter
threading verified across proc/script/dispatch callers); loop arithmetic
verified over the full u16 domain with garbage upper words; Z80 refusal
airtight; adoption confirmed stronger than claimed (scratch-free → mulu is
structurally the sole candidate, stable under ANY future table change). Info
notes and dispositions: (a) net labels lacked SourceId → **FIXED** (above);
(b) message duplication → **FIXED** (Lens A #1); (c) a refused construct
dropped without `dropped_instrs` accounting → **FIXED**: the choke point now
counts one drop per refusal diagnostic, keeping the corpus walk's
`dropped == 0` pin honest; (d) `cycle_scope` snapshots the pre-expansion
buffer — holds today by exclusion (only consumer is the Z80 T-state summer,
which refuses loudly) → **LEDGERED** with a kill condition on the first 68k
`cycles()` reader; (e) `as_type` on a construct item silently discarded →
**FIXED**: `[mul.operands] … takes no \`as\` dispatch bound` refusal.

**Lens C (cost decision): everything verifies — zero should-fix/must-fix.**
Independently re-derived, from the table arms directly: the value-aware MULU
row (incl. ceiling-consistency at all-ones and the UM source-operand
reading), all twelve pinned encoding classes, both boundaries (2^8→2^9,
M=2→3), the tie-impossibility of loop-vs-mulu, no second table and no cycle
literals, and the byte-identical adoption claim — including the hunted
negative: there is NO n in 0..=$FFFF where a winning mulu differs from the
hand encoding. Info notes: (i) the aeon commit message's tie-break narrative
is grander than the scratch-free sites (mulu is sole candidate there) →
**owned in §5**; (ii) `shift_run(9)` coded its 1-bit remainder as `lsl #1`
(34) where `add.l d,d` (32) is cheaper — never reachable in a CHOSEN
sequence, but **FIXED anyway** (the remainder-1 chunk now emits `add.l`;
hand-checked that no pick moves: the improved ×512 chain at 44 still loses
the byte tie to mulu).

## 10 · Ledger / docs

Four gap-ledger rows (sized-variant demand w/ 4-site census · deferred
mul_bounded DEBUG assert · Z80 variant · generator extensions), the R-B
closure annotation, and the `emp-idioms.md` entry ("Multiplying: name the
constant, let the compiler pick the encoding" — satisfying the design's
acceptance-list item on when to annotate bounds). SPEC2 docs row deferred to
the next unfreeze, as the design specifies.

## 11 · Commits (merge-order constraint: sigil BEFORE aeon)

- sigil `mul-lowering`: `d3079680` (construct + cost row + tests + docs),
  `d0067ce6` (label namespace fix), <FILL packet commit>.
- aeon `mul-lowering`: <FILL> (the two byte-identical adoptions in
  object_test_state.emp). **The aeon commit builds only against a sigil binary
  containing the construct — the sigil merge must land first, and per the
  standing trap, merging this two-repo parcel stales every other in-flight
  lane's aeon worktree.**

## 12 · Step-3 vs step-5 findings

**Step-3 (language asks):**
1. THE finding: the corpus's multiply-chain idiom is word-width with
   ensure-guarded ranges; v1's long contract covers none of it. Ask: ratify
   `mul_const.w` (distinct name-by-suffix, distinct contract: word result,
   upper word unchanged-garbage, author-owned range proof) — 4 adoption sites
   waiting; adjudication belongs to the language round.
2. A compiler-emittable DEBUG assert mechanism (the design assumed one; none
   exists at the construct layer) — blocks `mul_bounded`'s bound check.
3. `shl_l` (R2-panel row) remains a natural companion — same table, same
   generator core; the ×2^k arm of `mul_const` already subsumes its multiply
   half.

**Step-5 (engine findings, not taken by this lane):**
1. The four word-chain sites each beat the long-contract construct by 2-12
   cycles at +4 bytes — evidence FOR the sized variant, not for hand-editing.
2. plane_buffer's ×160 chain at 44 cycles sits 2 cycles from mulu's 46: if
   that proc is ever budget-pressed, `mulu.w #160` costs +2 cycles and −4
   bytes with intent visible — a candidate only under the sized-variant or a
   deliberate byte-changing parcel.
3. object_test_state's spawn-churn loop recomputes `i & 7` / `i >> 3` per
   iteration; irrelevant to frame budget (test scene, spawn-time only) —
   noted, declined.

**Neither-bucket headline:** the mulu tie at 46 cycles is structural, not
coincidence — a two-set-bit multiplier's mulu cost (42 + 4) equals the
minimal seeded two-add chain (12 + 34), so the WHOLE corpus stride family
sits exactly ON the boundary, and the byte tie-break (4 vs 12) is what
actually decides. The cost table's 2-cycle granularity is doing real work;
any future table correction of ±2 on `move`/`lsl`/`add` flips corpus-visible
decisions and MUST ride the golden gates (stated in the module header).
