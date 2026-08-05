# 2026-08-04 — B′-2: stack delta (close packet)

Status: **checkpoint for the overseer's countersign + merge. NOT merged, NOT
pushed.** Branch `bprime-2` off sigil `21f5aef7`, **zero commits in aeon** — a
sigil-only parcel (see §9, lane discipline). Master has since moved to `ba09c82a`
(two notes-only commits); no rebase was needed to build.

**Read §10 first if you are short of time.** The lens panel found TWO BLOCKERS in
work that was already gate-green — a `bsr` misread by the SHARED CFG, and
`falls_into` procs charged as returns — plus four soundness fixes. Both blockers
are fixed and both are pre-existing substrate bugs that a new ERROR-tier consumer
made fatal. §4's coverage numbers are the POST-panel measurement.

Spec: `specs/2026-08-04-contract-delta-spec.md` §3 (the work order) over
`specs/2026-08-03-contract-unification-spec.md` §4-stack (the surface) and §6
(the tier map).

## §0 — THE HEADLINE

**No fourth dataflow, and no way to build one by accident.** The symbolic stack
`preserves.rs` already walks *is* the sp delta — its slot map's byte depth is
exactly sp's displacement from proc entry — so `[stack.unbalanced]` and
`[stack.merge-mismatch]` are a different CONCLUSION read off the same states,
not a second analysis.

The extraction that makes that structural: both consumers now walk through one
`run_stack_dataflow` driver over a `StackObserver` trait. **One worklist, one
`transfer`, one `join`, therefore one bailout set.** A consumer chooses what to
conclude from a state; it never gets to decide whether the state is trustworthy.
That is the whole drift argument, and it is a property of the code's shape rather
than of a comment asking future porters to keep two lists in sync.

The second-order win was unplanned: **`link`/`unlk` were a live soundness hole in
the shipped `preserves` proof.** A `link a6,#-8` moves sp without naming it, so
`sp_hazard` never saw it and the slot map silently claimed a depth the machine
did not have. Modeling the frame in the shared `transfer` fixes the balance
checker and the entry-value proof in the same eight lines.

## §1 — What was verified OPEN before building

| Claim | Verified how | Result |
|---|---|---|
| No stack-delta analysis exists | `grep -rn 'stack_delta\|\[stack\.' crates/` | absent |
| `link`/`unlk` reach the 68k backend | `grep -rn 'Link\b\|Unlk\b' crates/sigil-backend-m68k/src/` | **no encoder** — the only mention anywhere is `branch_const.rs:115`'s CC-transparency string table |
| `link`/`unlk` in the corpus | `grep -rniE '^\s*(link\|unlk)(\.\|\s)' aeon --include='*.emp'` | **zero sites** |
| `pea` in the corpus | same, `pea` | **zero sites** (one prose mention in a comment) |
| `@as_compat` modules in the corpus | `grep -rn '@as_compat' aeon` | **zero** |
| non-`move`/`movem` pushes | mnemonic histogram over all 248 `-(sp)` sites | `movem.l` 97, `move.w` 65, `move.l` 33, `movem.w` 3, `move.b` 2 — nothing else |

## §2 — The design

### §2.1 The extraction

`verify_preserved_on`'s worklist moved into `run_stack_dataflow` **verbatim** —
same order of operations, same `transfer`-before-edges structure, same
`Edge::Follow` / `Abandon` / `Defer` arms, same path-local bail semantics. What
was a `match edge` with two inline `checkpoint` calls is now two `obs.exit(idx,
&st, is_return)` calls; the `AllReturns`-ignores-`Defer` rule moved into
`PreserveObserver::exit`, where it reads as one `scope.checks(idx, is_return)`.
The empty-body early return stayed in `verify_preserved_on` (it returns
all-`Verified`, which is a `preserves` fact, not a driver fact).

The trait has one required method and one defaulted one:

```rust
trait StackObserver {
    fn exit(&mut self, idx: usize, st: &State, is_return: bool);
    fn merge(&mut self, _succ: usize, _existing: &State, _incoming: &State) {}
}
```

`merge` exists because the balance checker needs to see a disagreement that
`join` deliberately ERASES — `join`'s answer to a depth mismatch is to taint the
merged state `bailed`, which is right for the entry-value proof and destroys
exactly the fact `[stack.merge-mismatch]` reports. So the observer is called with
both incoming states BEFORE the join, and `join` itself is untouched.

**Nothing was extracted into a new module.** The balance checker lives in
`preserves.rs` as a new section, next to the `find_dead_saves` section, because
the private `State` / `transfer` / `join` are the whole point of the exercise. A
separate module would have needed all three made `pub(crate)`, which is the first
step toward the drift this parcel exists to prevent.

### §2.2 What the checker concludes

- `[stack.unbalanced]` — at a CHARGED exit on an UNBAILED path, the tracked depth
  must be zero.
- `[stack.merge-mismatch]` — two UNBAILED paths reaching one instruction must
  carry the same byte depth.

**Which exits are charged.** `Cfg::edges` uses one `Edge::Abandon` for a real
return AND for control running off the end of the body, so the edge alone cannot
tell them apart. A RETURN instruction is always charged. A fall-off-end is charged
only when the proc declares no `falls_into`: a declared fallthrough CONTINUES into
its successor, the pair may legitimately share one frame across the boundary, and
charging it produced an error message about an `rts` the body does not contain
(§10 — both lenses found this independently, over a construct with 45 corpus
users). The undeclared case keeps its obligation, since whatever follows it in the
section never agreed to inherit a dirty stack.

Findings are keyed `(span.source, span.start, kind)`. The SOURCE is load-bearing
and was missing in the first cut: a `with <ctx> { }` bracket splices the context
module's instructions into the consuming body carrying THEIR file's spans, so two
findings in one proc can sit at the same byte offset in different files — Lens C
demonstrated a genuine ERROR-tier finding being dropped by aligning them.

**A finding is a fact about a PATH, not about the merged view.** The walk records
during iteration, which reads like a fixpoint bug and is not: the state at any
visit is either one path's exactly (first arrival) or a merge whose members AGREED
on depth, so every recorded depth is true of a real path. Waiting for convergence
would be strictly worse — a later merge answers a disagreement with a bail, which
would erase an imbalance the model had already proven. Pinned by
`a_real_per_path_imbalance_survives_a_tainted_merge`.

**Only one direction is representable.** `Slot::bytes` is unsigned, so there is no
negative depth: an sp RAISED past entry drains the tracked slots and hits the
underflow bail instead of producing a positive delta. The finding therefore
reports a DEPTH (bytes still held), not a signed delta, and the "popped more than
was pushed" class is a stated blind spot rather than an implied coverage claim.

The policy is `CallPolicy::ClobberAll` — a call's STACK effect is
policy-independent (it nets zero: the return address it pushes is popped by its
own `rts`), and the entry-value bits the oracle varies are the bits this consumer
never reads. So the balance checker needs no closure and runs per-file at
lowering time.

**Scope: returns only.** A tail transfer out (`Edge::Defer`) is deliberately not
charged, for the reason the entry-value proof already gives in place — it may
diverge (a noreturn error rail owes its caller nothing) and nothing in the
language marks that yet, so charging it would reject an honest failure rail.
Pinned by `a_tail_transfer_out_is_not_charged`.

### §2.3 `link` / `unlk`

`link aN, #-d` pushes the saved frame pointer as an ORDINARY tagged slot (so a
frame is visible to the depth model like any other save), pushes one opaque slot
of the true allocation size, and records an `(aN, depth)` mark. `unlk aN`
truncates the stack back to the matching mark and pops the saved fp, restoring
its entry bit iff the slot holds its own value.

`Slot::bytes` widened `u8` → `u32` so a frame carries its true allocation. That
is the only change to an existing data shape; every other path treats it as
before.

**The pairing rule, honestly.** Only one direction is decidable and only one is
implemented as a finding:

| shape | verdict | why |
|---|---|---|
| `link` with no `unlk` on a path to a return | **`[stack.unbalanced]`** | the frame is still on the stack; the depth check reports it with no special case |
| `unlk` with no open `link` | **bail (silent)** | sp is set from a register the model never computed |
| `unlk aM` against an open `link aN` | **bail (silent)** | same — the frame chain disagrees, so sp is unmodeled |
| paths merging with different open frames | **bail** | `join` gained a `frames` equality test, the depth mismatch's sibling |

Reporting a delta in rows 2 and 3 would be a guess, and an ERROR tier does not
get to guess.

## §3 — The lint set, and its POSITIVE and SILENCE probes

28 integration tests (`crates/sigil-frontend-emp/tests/stack_balance.rs`) + 10
unit tests (`preserves.rs::frame_tests`).

**Positives** — `push_without_pop_is_unbalanced` (asserts the tier is
`Level::Error` and the message names the depth), `one_sided_push_is_a_merge_mismatch`
(and that the tainted merge does NOT also charge the return below it),
`a_loop_that_grows_the_stack_is_a_merge_mismatch`,
`an_undeclared_fall_off_end_is_still_charged`,
`a_real_per_path_imbalance_survives_a_tainted_merge`.

**Correct code is silent** — `a_matched_movem_pair_is_silent`,
`an_immediate_sp_cleanup_balances_a_push`, `a_call_nets_zero_on_the_stack`,
`a_balanced_loop_is_silent`, `a_tail_transfer_out_is_not_charged`,
`a_z80_body_is_not_checked`, `a_local_bsr_does_not_charge_its_helper_with_the_callers_stack`,
`a_declared_fallthrough_end_is_not_a_return`,
`a_width_mismatched_pop_silences_the_checker`,
`a_top_of_stack_read_is_not_a_hazard`.

**Silence probes.** Most run through `assert_bailout_silences`, which FIRST proves
the hazard-free twin fires and THEN proves the hazard version is silent — so a
green cannot mean the checker is dead.

| bailout arm | probe |
|---|---|
| bare `a7` operand (sp's value escapes) | `a_bare_sp_operand_silences_the_checker` |
| computed `adda` to sp | `a_computed_sp_advance_silences_the_checker` |
| displaced sp WRITE (`d(sp)`) | `a_displaced_sp_write_silences_the_checker` |
| indexed sp access (`(sp,Xn)`, base) | `an_indexed_sp_access_silences_the_checker` |
| sp as the INDEX register (`(aN,sp)`) | `sp_as_an_index_register_silences_the_checker` |
| plain `(sp)` STORE | `a_top_of_stack_store_silences_the_checker` |
| pop underflow | `a_pop_underflow_silences_the_checker` |
| pop width disagreement | `a_width_mismatched_pop_silences_the_checker` |
| sp cleanup over-drain | `an_over_draining_cleanup_silences_the_checker` |
| sp cleanup landing mid-slot | `a_mid_slot_cleanup_silences_the_checker` |
| `unlk` with no `link` | `an_unlk_with_no_link_bails_rather_than_guessing` (unit) |
| `unlk` of the wrong register | `an_unlk_of_the_wrong_register_bails` (unit) |
| positive `link` displacement | `a_positive_link_displacement_bails` (unit) |
| unrepresentable `link` frame size | `an_unrepresentable_frame_size_bails` (unit) |
| `join` depth mismatch | covered indirectly by `one_sided_push_is_a_merge_mismatch`'s second assert (the merge fires, the return below it does not) |
| `join` slot-BYTES mismatch | **no probe — and none is constructible.** It only decides when two paths agree on total depth and disagree on geometry, which forces at least one of them to be individually ill-formed; its real consumer is the entry-value proof, where a tainted merge makes `preserves` unverifiable rather than wrongly verified |
| `join` FRAMES mismatch | **no probe** — `frames` is non-empty only after a `link`, which no `.emp` source can lower (no backend encoder), so it is a defensive sibling of the depth arm |
| `apply_link` non-register / `a7` fp, non-immediate size | **no probe** — same reason |

Lens C audited this table and the earlier draft's "every bailout class gets a
probe" was an OVERSTATEMENT; it now says what it covers. The three uncovered arms
are named above with the reason each is unreachable, and Lens C verified by hand
that the ones it could exercise all behave correctly.

`link`/`unlk` are proven at UNIT level and cannot be otherwise: the 68k backend
has no encoder for either mnemonic. See §6 for the honest gap that follows.

## §4 — What the checker found over the corpus: NOTHING, and here is what that is worth

All seven shapes lower CLEAN — zero `[stack.*]` firings, at ERROR tier, so a
single firing anywhere would have failed the build.

"Silent" only means something if the analysis was actually looking, so the
coverage was MEASURED with throwaway instrumentation (a per-proc return-site
census printed behind an env var, run over all seven shapes, then removed —
it is not in the commit):

Measured TWICE — before and after the panel's fixes, since the `falls_into`
correction removes ~18 fall-off-ends per shape from the charged set and the
`(sp)`-store hazard adds one. These are the FINAL numbers:

| shape | procs | charged return sites tracked EXACTLY | bailed | procs with a bail |
|---|---|---|---|---|
| sonic4 plain | 275 | **383** | 3 | 2 |
| sonic4 debug | 276 | 388 | 3 | 2 |
| demo plain | 180 | 296 | 3 | 2 |
| demo debug | 181 | 303 | 3 | 2 |
| config_a | 279 | 398 | 3 | 2 |
| config_b | 263 | 375 | 3 | 2 |
| lean | 263 | 383 | 3 | 2 |

**99.2% of the corpus's charged return sites are tracked exactly, and every one
of them is balanced.** The result is a guarantee over almost the whole engine,
not an absence of measurement.

The intermediate measurement is worth recording because it caught a fix that was
too broad: narrowing the `(sp)` hazard to STORES only (a load cannot alter a
slot) took the bailing-proc count back from 3 to 2 — `Render_Sprites` had started
bailing on `adda.w (sp), a2` at `sprites.emp:257`, which is entirely safe. That
is now a pinned wall (`a_top_of_stack_read_is_not_a_hazard`).

The three bailing sites live in two procs, and both bail for the SAME documented
reason — a displaced-sp STORE, which could alias a tracked slot:

- `engine/objects/children.emp:476` — `add.l d0, 4(sp)` in `CreateChild_Linked`
  (2 return sites bail, 2 are clean)
- `engine/level/tile_cache.emp:611` — `move.w d0, 2(sp)` in `TileCache_FillAll`
  (its 1 return site)

Those are the ONLY two displaced-sp stores in the entire 122-file corpus, and
`children.emp:481`'s `add.l d0, (sp)` is the only plain-`(sp)` store — in the same
already-bailing proc, which is why hazarding it (a real soundness fix, §10) costs
zero coverage. Every sp READ form is exempt, so the ~6 `d(sp)` reads and the one
`(sp)` read cost nothing.

**The checker was NOT tuned to produce an interesting number.** No threshold was
adjusted, no bailout was widened or narrowed to move the corpus result; the
bailout set is exactly the one `preserves.rs` already shipped plus the two new
`link`/`unlk` cases.

## §5 — `@as_compat` and `@allow`

Per U-spec §6's tier map (`[stack.*]`: error · softens to warn · `@allow` yes):

- default → `Level::Error`
- `@as_compat` → `Level::Warning`, both arms
  (`as_compat_softens_the_finding_to_a_warning`,
  `as_compat_softens_the_merge_mismatch_too`)
- `@allow("stack.unbalanced")` → suppressed, per id, so allowing one arm leaves
  the other reporting (`allow_suppresses_the_finding`, `allow_is_keyed_per_lint_id`)

The reason the softening is right HERE, where a declared contract never softens:
a declared contract is a claim its author opted into, so a wrong one is worse
than none. This gate reads raw ported assembly nobody annotated. A faithful port
should see the finding without having its build fail on it. That rationale is
recorded at `report_stack_balance` rather than only in this packet.

**Corpus reach of the softening: zero.** No aeon module carries `@as_compat`, so
the warn path is proven by tests only — stated here rather than implied.

## §6 — HONEST GAPS

1. **`link`/`unlk` have ZERO corpus adopters and cannot get any.** The 68k
   backend has no encoder for either mnemonic, so the rule is proven by 10 unit
   tests over hand-built `CodeItem`s and by nothing else. This is the same shape
   as B′-1's `requires(z80_stopped)` having zero corpus adopters outside tests —
   implemented for soundness (an unmodeled `link` corrupted the shipped
   `preserves` model), not for coverage. No corpus adopter was manufactured.
2. **`pea` is not modeled as a push.** Zero corpus sites today. The direction is
   safe (the model UNDER-counts depth, so it stays silent or bails) but it is a
   silent hole the day someone writes one. Ledger row.
3. **`rtd #N` and `rtr` are treated as plain returns.** `rtd` pops N extra bytes
   and could turn a correct proc into a false `[stack.unbalanced]`; `rtr` pops a
   CCR word. Both are zero-site in the corpus and `rtd` is 68010+, so neither is
   reachable on this hardware — but the model does not know that. Ledger row.
4. **Tail transfers are not charged** (§2.2). A `move.l d0,-(sp)` followed by
   `jmp Foo` is unreported. Closing it wants `@noreturn`, which is the same
   language ask B′-0b's ledger already carries.
5. **No Z80 arm.** `z80_preserves` is the Z80 sibling and has no stack-delta
   model; the checker is gated 68k-only and pinned by
   `a_z80_body_is_not_checked`.
6. **`find_dead_saves` still has its own transfer and join** (`ds_transfer` /
   `ds_join`) with a STRICTLY MORE CONSERVATIVE bailout set — it bails on
   `addq #4,sp`, which the shared transfer models, and its slots carry no byte
   width. Pre-existing, and it is the one place in the module where the "one
   bailout set" claim does not hold — §0's claim is scoped accordingly.
   **CORRECTION to this packet's first draft**, which said the parcel "neither
   worsens nor fixes it": Lens B showed it WORSENED it. `ds_transfer` neither
   modeled nor bailed on `link`/`unlk`, so once this parcel made those mnemonics
   meaningful its depth could go wrong and pair a restore to the wrong slot — a
   WRONG code-cutting suggestion, not a missing one. Closed in-parcel with an
   explicit bail.
7. **`[stack.unbalanced]` covers only ONE direction of imbalance.** The slot map
   has no negative depth, so sp raised past entry bails instead of firing. Stated
   in the finding's own doc; ledger row.
8. **`rte`/`rtr`/`rtd` share the `rts` rule.** Correct for the corpus's one `rte`
   (a balanced `movem` pair) and for the normal case generally — the exception
   frame and the caller's parameters both sit BELOW the handler's entry sp. Not
   correct for a handler that hand-builds its own frame, which the art-streaming
   phase-2 plan proposes. Ledger row; `@allow` is the escape.
9. **`Cfg::edges` still conflates a return with a fall-off-end.** B′-2 needed the
   distinction and got it by re-reading the mnemonic at the exit site. That works,
   but a consumer is re-deriving what the edge model knew and discarded. Splitting
   the variant touches every `edges` consumer and was declined in a byte-frozen
   parcel. Ledger row — and since BOTH lenses found the same symptom
   independently, expect it to recur.

## §7 — `warn_tier_corpus.rs`: UNTOUCHED, and why

`crates/sigil-cli/tests/warn_tier_corpus.rs` freezes a per-shape SET of firing
lint ids for WARN-tier diagnostics. It is correctly untouched, for two
independent reasons — either alone would settle it:

1. **Tier.** `[stack.*]` is ERROR-tier by default. `collect_warnings` gathers
   `Level::Warning`/`Note`; an error never reaches that baseline at all — it
   fails the build instead.
2. **Reach.** The one path that WOULD produce a warn-tier `[stack.*]` is
   `@as_compat`, and no aeon module carries it (§1). So even the softened form
   has no corpus instance to change the frozen set.

Verified own-run: `warn_tier_lint_ids_match_the_frozen_baseline` passes unchanged
across all seven shapes with the checker live.

## §8 — BARS

### §8.1 Byte bar — SEVEN targets, `cmp`, in `capture_goldens.sh` order

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

Byte-neutral ×7, as a pure-checker parcel should be. **No refreeze, no chain
bump, no 5-site ripple** — nothing in the diff can emit a byte (the checker runs
after `lower_code_buf` and only appends diagnostics).

Run THREE times: after the first cut, after the Lens A pass, and after the panel
B/C fixes — the last of which touched the SHARED `Cfg` (`bsr`) and the shipped
entry-value model (`(sp)` stores, pop widths), so re-proving was not a formality.
The corpus CONTRACT gates were re-run alongside it and all pass unchanged:
`warn_tier_lint_ids_match_the_frozen_baseline`, `d1c_firings_match_the_frozen_baseline`
(the pinned 21-row D1c set), `cond_out_survives_claims_all_prove`,
`residue_procs_verify_as_predicted`, and the whole `corpus_contracts` walk. So the
model changes are neutral for DIAGNOSTICS as well as for bytes.

### §8.2 Strict suite

```
AEON_DIR=<b2 aeon worktree> SIGIL_EMIT=… SIGIL_BUILD=… SIGIL_STRICT_GATE=1 \
  cargo test --workspace --release
```

**Failures first: NONE.** No `failures:` block, no `FAILED` line, no `error[` /
`error:` anywhere in the 5000+-line log.

| | branch `af08ed0d` |
|---|---|
| result lines | 308 (includes the 10 zero-count doc-test lines) |
| **passed** | **3168** |
| **failed** | **0** |
| **ignored** | **4** |
| filtered out | 0 |

`3168 + 4 = 3172`, which is EXACTLY the branch's own `#[test]` total (§8.3) — so
nothing is silently skipped. Master's total is 3134 by the same count; the diff is
+38, named function by function below. Master's suite was NOT re-run (45–75 min on
a machine shared with the concurrent `sr` lane); the identity above is the check
the standing rule asks for and it holds directly on the branch.

Run TWICE: once at `2b735698` (3156 passed / 0 / 4, identity held at 3160) and
again after the panel fixes. The second run is the one reported.

### §8.3 Test-delta arithmetic — every added function NAMED

`git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'`, diffed per file:

| | master `21f5aef7` | branch `af08ed0d` | delta |
|---|---|---|---|
| `#[test]` total | 3134 | **3172** | **+38** |

The per-file diff shows exactly TWO changed files, both new counts — no existing
file gained or lost a test:

**`crates/sigil-frontend-emp/src/preserves.rs` +10** (`frame_tests`, unit-level
because no `.emp` source can lower `link`/`unlk`):
`a_paired_frame_is_balanced`, `a_link_with_no_unlk_is_unbalanced`,
`an_unlk_with_no_link_bails_rather_than_guessing`,
`an_unlk_of_the_wrong_register_bails`,
`a_paired_frame_preserves_its_frame_pointer`,
`an_unclosed_frame_does_not_preserve_its_frame_pointer`,
`a_closed_zero_size_frame_is_balanced`,
`an_unclosed_zero_size_frame_still_carries_its_saved_pointer`,
`a_positive_link_displacement_bails`, `an_unrepresentable_frame_size_bails`.

**`crates/sigil-frontend-emp/tests/stack_balance.rs` +28**:
`push_without_pop_is_unbalanced`, `one_sided_push_is_a_merge_mismatch`,
`a_loop_that_grows_the_stack_is_a_merge_mismatch`,
`a_real_per_path_imbalance_survives_a_tainted_merge`,
`a_matched_movem_pair_is_silent`, `an_immediate_sp_cleanup_balances_a_push`,
`a_call_nets_zero_on_the_stack`, `a_balanced_loop_is_silent`,
`a_tail_transfer_out_is_not_charged`, `a_z80_body_is_not_checked`,
`a_bare_sp_operand_silences_the_checker`,
`a_computed_sp_advance_silences_the_checker`,
`a_displaced_sp_write_silences_the_checker`,
`an_indexed_sp_access_silences_the_checker`,
`a_pop_underflow_silences_the_checker`,
`a_local_bsr_does_not_charge_its_helper_with_the_callers_stack`,
`a_declared_fallthrough_end_is_not_a_return`,
`an_undeclared_fall_off_end_is_still_charged`,
`a_width_mismatched_pop_silences_the_checker`,
`a_top_of_stack_store_silences_the_checker`,
`a_top_of_stack_read_is_not_a_hazard`,
`sp_as_an_index_register_silences_the_checker`,
`an_over_draining_cleanup_silences_the_checker`,
`a_mid_slot_cleanup_silences_the_checker`,
`as_compat_softens_the_finding_to_a_warning`,
`as_compat_softens_the_merge_mismatch_too`, `allow_suppresses_the_finding`,
`allow_is_keyed_per_lint_id`.

10 + 28 = **38**. Confirmed against the strict log's own per-binary lines
(`running 28 tests` for `stack_balance.rs`; 10 `preserves::frame_tests::` results).
No test was removed, renamed away, or silently skipped.

### §8.4 Clippy

CI runs `clippy --workspace --all-targets -- -D warnings`. **The parcel adds
ZERO clippy findings** — verified own-run: no warning in the crate's list points
at `stack_balance.rs`, `preserves.rs`, `flag_check.rs` or `context.rs`. (Lens A
caught two `useless_format` hits in the first cut; they are gone. The surviving
hits in this crate are the pre-existing toolchain drift handoff §3 item 7
records, including `lower/proc.rs:514`, which this parcel merely shifted 59 lines.)

## §9 — LANE DISCIPLINE: aeon adoption HELD

Zero aeon commits. The concurrent `sr` lane is editing `irq.emp`,
`dma_queue.emp` and B′-1's `with ints_off { }` machinery; this parcel touches no
aeon file and never needed to. Nothing in it WANTS an aeon-side change either —
the corpus is already stack-clean (§4), so there is no adoption to hold over for
later. A sigil-only parcel, complete on its own.

## §10 — LENS PANEL

Three fresh read-only subagents, one lens each, over `git diff master...bprime-2`
plus surroundings. The porter reviewed nothing of its own.

### Lens A — ceremony / style: 11 findings, **all applied** (`1e75b140`)

One was effectively a BLOCKER and the porter had missed it:

| # | finding | disposition |
|---|---|---|
| 4 | **`tests/stack_balance.rs` added 2 clippy `useless_format` hits**, and CI runs `clippy --all-targets -- -D warnings` | **FIXED.** Verified own-run: the crate's clippy warning list no longer mentions `stack_balance.rs` or `preserves.rs` at all. The parcel adds ZERO clippy warnings; the surviving hits in this crate are pre-existing (incl. `lower/proc.rs:514`, which my step 11 merely shifted 59 lines) |
| 1 | `checkpoint`'s doc still said "Called from BOTH exit arms" — the extraction collapsed them to one | FIXED (comment truth) |
| 2 | `checkpoint`'s six `&mut` out-params are scaffolding `PreserveObserver` retires — one caller, owning all four | FIXED — it is a method now |
| 3 | **The anti-drift claim said "Every analysis", which is false**: `find_dead_saves` has its own `DsState`/`ds_transfer`/`ds_join` and a narrower bailout set | FIXED — the claim scopes to the two `State` consumers and NAMES the exception. Ledger row added (§11) |
| 4b | the merge-mismatch body was hand-spelled three times | FIXED — `MERGE_MISMATCH` + `proc_src_with` |
| 5 | `report_stack_balance` breaks the file's `check_*` family; takes `&ProcDecl` where it uses only `.name` | FIXED — `check_stack_balance(file, name, …)`, matching `check_context_brackets` |
| 6 | the `@allow` test ran AFTER building the message it then discarded | FIXED — hoisted, matching `lower/mod.rs:1004` |
| 7 | `record(idx, 0, …)` / `record(idx, 1, …)` — unnamed `u8` class tags | FIXED — `StackFindingKind::class()` |
| 8 | `MergeMismatch { a, b }` in a module that names everything | FIXED — `{ existing, incoming }`, the observer's own words |
| 9 | third differently-spelled copy of the "first `Instr` item" probe | FIXED — `entry_instr_idx`, three call sites |
| 10 | `frame_tests` nits: `frame` vs the ISA's `disp`, one test asserting two shapes, bare `assert_eq!`s | FIXED — split, renamed, every bailout assert now states why its silence is correct |
| 11 | one 103-column line | FIXED |

Lens A found NO change-history narration (it grepped every added line for
`was`/`previously`/`used to`/`dropped`/`now`/`parcel`) and no brace-indent
violation, and called `assert_bailout_silences`' precondition-then-probe shape
"what stops a green from meaning dead checker".

**Three PRE-EXISTING findings, declined as out of scope and recorded here**:
`apply_callee_effect`'s "Shared by the CALL transfer and the scoped TAIL-exit
charge" has had one call site on master too; `CallPolicy::ClobberAll`'s "The
pre-oracle behavior" is change-history narration in a shipped doc comment; and
`lower/proc.rs`'s module header said "three §5.1 proc-contract checks" over a
function that had ten steps before this parcel. The third WAS fixed — the parcel
added the eleventh step, so leaving the count stale would have been this
parcel's debt.

### Lens B — corpus-pattern: the central risk did NOT happen, and one blocker did

**On the parcel's own thesis, Lens B verified it structurally rather than
believing the comment**, by call-site census: `transfer` has exactly ONE call
site, `join` has exactly ONE, `run_stack_dataflow` has exactly TWO callers, and
`State::bailed` is written only in the driver and in `join`. `Slot`, `State`,
`transfer`, `join`, `run_stack_dataflow` and `StackObserver` are all
module-private, **so a fourth dataflow is not constructible without editing this
module** — which is a stronger statement than the one the packet made. It also
checked the thing the packet asserted and did not prove: `apply_callee_effect`
writes only `entry`/`delta`, never `stack`/`frames`/`bailed`, so the two
consumers' different `CallPolicy` cannot move the depth model.

It confirmed the extraction is behaviour-preserving line by line against
`git show master:`, built the corpus with branch and master binaries and got
byte-identical ROMs, and measured build wall-clock at no cost (2.161 s vs
2.221 s, min of 4).

| finding | disposition |
|---|---|
| **BLOCKER: `falls_into` procs are charged as returns** — reproduced by appending one push to `TestSolid_Init` and getting an error about an `rts` that does not exist. 45 corpus users of the construct | **FIXED.** Also found independently by Lens C (C2) |
| `ds_transfer` neither models nor bails on `link`/`unlk`, so this parcel made the dead-save worklist's divergence WORSE — a wrong suggestion, not a missing one — and §6.6 said the opposite | **FIXED** (explicit bail) **+ packet corrected** (§6.6) |
| the packet's ledger rows were not yet in `campaign-gap-ledger.md` | **FIXED** — they were committed in `39fae901`, which Lens B measured before; the panel's own new rows are now added too |
| three copies of "the span of instruction `idx`" (`preserves.rs` ×2, `context.rs`) | **FIXED** — `flag_check::instr_span`, next to `Cfg::instr` |
| a declared stack frame is the language ask the residual bail surface is asking for | **LEDGERED** — see §11, this is the parcel's strongest step-3 row |
| the emitter's local name now shadows `crate::preserves::check_stack_balance` | **DECLINED.** Lens A asked for the `check_*` family name and this is the cost; the call site is fully qualified, and `check_preserves` similarly wraps `verify_preserved`. Two lenses, opposite nits — the house convention wins |
| `@as_compat` softening is the first error→warn softening in `lower_proc` | **NOTED, no action** — spec-mandated (delta spec §3, U-spec §6's tier row), flagged only so the gate sees the local-precedent break is deliberate |

Independently of the packet, Lens B mass-injected 297 unmatched pushes across 84
corpus files and got 107 errors — **the checker is demonstrably alive on real
engine code**, not only on hand-built test items — and its own grep corroborated
§4's bail analysis.

### Lens C — soundness + hazard: TWO BLOCKERS, both reproduced with runnable probes

| # | finding | disposition |
|---|---|---|
| **C1** | **BLOCKER: `bsr` to a LOCAL label is misread as a conditional branch.** `flag_check`'s `is_cond_branch` is `starts_with('b') && len() == 3`, which matches `bsr`, so the helper's body is analyzed carrying the caller's stack and its own `rts` fires. `jbsr` (4 letters) is silent, so the two spellings of one call disagreed | **FIXED** in `flag_check`. Corpus-neutral by measurement (all 9 corpus `bsr` sites are external). A PRE-EXISTING substrate bug every `Cfg` consumer inherited; B′-2 is just the first for which it would be fatal. Ledgered as a class |
| **C2** | **BLOCKER: `falls_into` fall-off-end charged as a return** | **FIXED** — see Lens B |
| C3 | **the pop path counted SLOTS, not BYTES**: `move.w`×2 then `move.l (sp)+` is balanced on the machine and was reported as 2 bytes held | **FIXED** — the pop drains bytes and a width disagreement bails |
| C4 | `move.l #Target,-(sp)` + `rts` (computed jump) fires, while the `pea` spelling is silent | **LEDGERED.** Zero corpus sites, verified by grep; `@allow` is the escape |
| C5 | **the dedup key dropped `span.source`**, so a `with <ctx> {}` splice can collide two findings at one byte offset and DROP an error-tier one. Demonstrated by aligning offsets across two files | **FIXED** — keyed `(source, start, class)` |
| C6 | the checker structurally cannot report sp TOO HIGH, so the `"above"` message branch was dead and the docs overstated coverage | **FIXED** — reports a DEPTH, not a signed delta; the blind spot is stated and ledgered |
| C7 | the unregistered-pop arm had no underflow guard, unlike its sibling — a silent divergence inside the one function whose thesis is "one bailout set" | **FIXED** (folded into C3) |
| C8 | **a plain `(sp)` STORE aliases the top slot and was not a hazard**, so the shipped entry-value proof accepted a `preserves` claim the store had destroyed | **FIXED** — but the first cut hazarded READS too and started bailing `Render_Sprites` on `adda.w (sp), a2`. Narrowed to the STORE direction, mnemonic-agnostic on reads, with a pinned wall |
| — | the silence-probe claim was overstated (5 of ~14 arms) | **FIXED** — §3's table now names every arm and says which have no probe and why |
| — | two doc claims false: "a diverging bailed path never poisons a returning one" (`join` ORs `bailed`), and findings recorded during iteration are not the fixpoint's answer | **FIXED** — both docs corrected; the per-path semantics are now explained and PINNED as a test, since they read like a bug |

Lens C also confirmed things the packet had asserted: `rte`/`rtr`/`rtd` sharing
the `rts` rule is CORRECT for the normal case (the exception frame and the
caller's parameters both sit below the handler's entry sp); the merge-mismatch arm
is sound modulo the CFG's edge model, which is exactly why C1 mattered; `pea`'s
under-count can never manufacture a positive depth because every drain path bails
on underflow; and the analysis terminates on a finite descending lattice. It found
the `link` arithmetic hazard already fixed mid-review.

**Panel score: 3 blockers between them (2 distinct), all fixed; 11 further
findings fixed; 5 ledgered; 1 declined with reason.** Both blockers were
pre-existing bugs in shared substrate that only became fatal when an ERROR-tier
consumer was pointed at them — which is the strongest argument yet for the panel
discipline: the parcel was gate-green, byte-neutral and strict-clean before the
panel ran.

## §11 — Per-pass findings

### Step 3 — retrospect and LANGUAGE ASKS

**A named stack frame is the ask this parcel found, and the corpus wrote its own
demand in a comment.** `tile_cache.emp:528`:

```
// stack layout: 0(sp)=end_row, 2(sp)=cur_row, 4(sp)=end_col, 6(sp)=start_col
```

Four hand-maintained word slots, described in prose, addressed by raw
displacement for ~120 lines, and torn down by two `addq.l #4, sp` at :1641/:1648.
This is the same shape B′-1's headline celebrated converting — *a comment that
should have been a checked declaration* — and it is the ONLY reason two corpus
procs cannot be proven stack-balanced: `TileCache_FillAll`'s single displaced
STORE (`move.w d0, 2(sp)`, :611) and `CreateChild_Linked`'s (`add.l d0, 4(sp)`,
children.emp:476) are what trip the aliasing bailout (§4).

A `with stack_frame { end_row: u16, cur_row: u16, … } { … }` bracket — B′-1's
context machinery generalized from a machine-state lattice to a memory layout —
would give the compiler the slot map the author already has in their head. It
would (a) name the slots so the code stops counting bytes, (b) make the
displaced store CHECKABLE instead of a bailout, taking coverage from 383/386 to
386/386, and (c) make the 17 `addq #N, sp` cleanup sites structural rather than
hand-balanced. Recorded as a demand, not built: it is a real construct and this
parcel is a checker.

**`@noreturn` gains its SECOND independent consumer.** B′-0b's ledger already
asks for it (a divergent tail cannot be told from a real tail call, so the
survives-claim declines to charge either). `[stack.unbalanced]` needs the exact
same distinction for the exact same reason (§6 gap 4). One attribute closes a
hole in two analyses — which is the argument for building it that a single
consumer could not make.

**The extend-don't-replace pattern held for a fourth time, and it paid a
dividend the first three did not.** Adding a second CONSUMER to an existing model
forced the model's COMPLETENESS to be examined rather than just its interface.
Everything the panel found in shared code — the `link`/`unlk` hole, the `bsr`
edge, the `falls_into` conflation, the plain-`(sp)` store, the slot-vs-byte pop —
was PRE-EXISTING and had been shipping under `preserves`. None of it could fire
there, because `preserves` only answers questions about procs that DECLARE a
contract, and none of the affected shapes did. Point an unconditional ERROR-tier
consumer at the same model and every one of them becomes reachable.

Worth stating as a general campaign finding: **a substrate with one consumer is
only tested on the paths that consumer takes, and "it has shipped for months" is
evidence about the consumer, not about the substrate.** The `bsr` case is the
sharpest instance — a three-letter mnemonic test in the SHARED CFG, inherited by
four analyses, over an ISA that spells a call like a branch.

### Step 5 — engine optimize

**Nothing.** This is a diagnostics parcel — byte-neutral ×7, no codegen path
touched, no cycle or DMA-window claim made. The one engine-shaped observation is
the language ask above, and the honest version of it is "two procs use the stack
as a scratch frame", which is a legitimate 68000 technique and not a defect.

The census DOES hand the engine a fact it did not have: **383 of the 386 charged
return sites are now proven to leave sp exactly at its entry value**, and the
three that are not are named. Any future parcel that touches sp discipline starts from a
measured baseline rather than an assumption.

### Neither bucket — the headline

**A shipped ERROR gate had a soundness hole that no corpus test could ever have
found.** `preserves.rs` has been the authority on `[proc.preserves-unverifiable]`
for the whole campaign, and it modeled `link`/`unlk` as no-ops — a `link a6,#-8`
moved sp invisibly, so the slot map claimed a depth the machine did not have and
every save/restore after it was matched against the wrong slot. Zero corpus
sites and no backend encoder mean it could not have fired; it was still wrong,
and it is fixed. The finding is not "we caught a bug" so much as **"a second
consumer is an audit"** — the hole was invisible while `preserves` was the only
question being asked of the model.
