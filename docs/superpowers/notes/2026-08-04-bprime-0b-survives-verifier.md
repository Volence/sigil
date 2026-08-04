# 2026-08-04 — B′-0b: the survives-claim verifier (close packet)

Status: **Checkpoint for the overseer's countersign + merge. NOT merged.** Branch
pair `bprime-0b`: one commit per repo, sigil on master `2287eabc`, aeon `33aed57` on master
`c424dfd`. Chain 40, tip `item28-bg-transpose`.

Spec: `specs/2026-08-04-contract-delta-spec.md` §7.1 (normative semantics) and
§7.2 (scope). Ledger rows closed: the B′-0 lens-C silent-lie surface (~:2025),
the `tile_cache.emp:130` dishonest shape (~:2032), the register-keyed exemption
nit (~:2034). The Z80 must-test row (~:2031) stands, untouched.

## §0 — THE HEADLINE

**The spec's rider prediction was wrong, and measuring is why we know.** §7.2
states that with the `tile_cache` flip landing, "`calls.rs`'s documented D1c
false positive at `TileCache_FillRow` dissolves". It does not. Measured before
and after the flip: **21 D1c firings both times, the FP row present both times.**

The mechanism rules it out. `destroys_value` reads exactly two inputs —
`closure.effective` (body-derived for a non-extern proc, so a1 was already in
`FindStagedBlock`'s effect before this parcel) and `callee_uncond_out`
(`out − out_cond`, from which a1 is excluded whether or not it is also declared
clobbered). **A declared clobber is not an input to D1c at all.** The FP is
edge-blindness in D1c's close — the a1 `FillRow` reads is the callee's PRODUCED
value on the eq edge, not a destroyed held value — and only an edge-precise D1c
dissolves it. Ledgered as such; the existing `destroys_value` FP row stands
unchanged and needs no edit.

Same answer for the second question §7.2 raised: **B′-0c's item-4 D1c baseline
allowlist for `Load_Object @ AllocDynamic :: a1` must NOT be retired.** It is the
same class, driven by the same edge-blindness, and it still fires. The baseline
is unchanged in this parcel and `d1c_firings_match_the_frozen_baseline` passes.

## §1 — What was verified OPEN before building

Every claim in the overseer's reconnaissance was re-derived against the trees,
not taken on faith:

| Claim | Verified |
|---|---|
| Corpus surface is exactly THREE cond-out procs | Yes — `tile_cache.emp:130`, `core.emp:123` (`AllocDynamic`), `core.emp:182` (`AllocEffect`). Nothing else declares `out(rN if cc)`. |
| `TileCache_FindStagedBlock` MUST fire | Yes — `lea Block_Stage_Keys, a1` runs before the probe on every path; the `.miss` exit returns a1 pointing into the key table. It was the verifier's first and only corpus firing. |
| `AllocEffect` must PASS | Yes — the pool test (`beq .full`) precedes the pop, so a1 is untouched on the `.full` path. |
| `AllocDynamic` must be SKIPPED | Yes — a1 ∈ `clobbers` (B′-0 landed that), so no claim. Note it WOULD fire if checked: `.full` is reached both from `beq .full` (a1 clean) and by falling through `.latch_full` (a1 popped), so the join leaves a1 clobbered. That is precisely the shape §7.1's clobbers-membership rule exists to license. |
| `Flags::after` models `moveq #0` → z=true and `moveq #1` → z=false | Yes, and it is load-bearing: both claim sites terminate in a `moveq`-classified return, which is exactly why the corpus could not see the ⊤ cost (§4). |
| B′-0b is the exact complement of `check_return` | **Only away from ⊤ — and that turned out to be the parcel's central finding (§4).** `check_return` obligates production on `Some(true)` and ⊤; this obligates survival on `Some(false)` alone. |

## §2 — Root cause (what B′-0 left open)

B′-0's own close packet §8 escalated it: `[proc.out-clobbers-overlap]` was the
only rule forcing `out(rN if cc)` to have one meaning. After the relax the corpus
carried two incompatible readings of the same syntax — "destroyed on every edge"
(register also in `clobbers`) and "survives the ¬cc edge" (register absent) —
distinguished solely by what the author typed, and **neither was checked**.

Concretely: hoist `AllocEffect`'s pop above its exhaustion test and a1 is trash
on the failure edge while the contract still says `clobbers(d0)`. Before this
parcel that compiled with zero diagnostics. `children.emp:578`'s
exhaustive-license arithmetic rests on the claim being true.

## §3 — The fix

### §3.1 — The proof is `preserves`', scoped — not a second one

`preserves::verify_preserved` gained a `ReturnScope` parameter and a
`verify_preserved_on` entry point. `AllReturns` is the `preserves(rN)` contract
(every `rts`/fall-off; tail transfers ignored, because the closure carries them
transitively). `Sites(&BTreeSet<usize>)` names its own exits.

That is the whole bridge the brief flagged as the core engineering task.
`verify_preserved` proves over EVERY return; the survives claim needs the ¬cc
returns only, because rN is DELIBERATELY written on the cc edge and an all-paths
proof would reject every honest claim. Scoping the exits — rather than forking
the analysis — keeps one implementation of the save/restore round-trip, the
never-written proof, the linear-delta proof, the sp-hazard bailouts and the
callee-preserves oracle. **No logic was forked.**

Two supporting extractions inside `preserves.rs`, both pure de-duplication:

- `checkpoint(...)` — the per-exit charge, previously inline in the `Abandon`
  arm, now called from both exit arms so a scoped tail checkpoint and a return
  checkpoint are judged by identical rules.
- `apply_callee_effect(...)` — the entry-bit/delta effect of handing control to
  a callee, previously inline in the call transfer, now shared with the tail
  charge. (This also removed one pre-existing clippy `needless_range_loop`
  finding rather than adding a second.)

### §3.2 — ONE cc classification, not two

`out_verify`'s dataflow state was a fused `{ produced, flags }`. The two
components never read each other — the production transfer never consults the
flags and the flag transfer never consults `produced` — so the fused fixpoint is
the product of two independent fixpoints. The flags half is now its own function,
`flags_after(cfg, items) -> BTreeMap<usize, Flags>`, and **both halves of the
contract read that one map**:

- `verify_out` keeps the PRODUCTION obligation wherever `eval(cc) != Some(false)`
  (unchanged);
- `not_cc_exit_sites` takes exactly the exits where `eval(cc) == Some(false)`.

So the two can never disagree about which edge an exit sits on — that much is now
structural rather than two approximations that happen to agree today. They are
NOT complements at ⊤, and §4 is why. As a side effect `verify_out`'s `State`
collapses to a bare `[bool; 16]`, `split_flags` becomes the flags-only
`split_cc`, and the branch-split call disappears from the production loop where
it was doing nothing.

### §3.3 — The gates

| | policy | tier | authority |
|---|---|---|---|
| per-file (`lower/proc.rs` step 8) | `ClobberAll`, then a `PreserveAll` deferral probe | ERROR, not `@as_compat`-silenced | fires on anything a single file can disprove |
| whole-corpus (`corpus_contracts.rs`) | `CallPolicy::Oracle(&closure.effective)` | assert-empty **suite** gate | FINAL — settles every per-file deferral |

**Scope, stated plainly (lens C):** the only diagnostic a BUILD emits is the
per-file one, and it defers everything a call / tail / indirect blocks. Classes
measured as per-file-clean but corpus-firing: a relabeling wrapper over a cond-out
callee, and `jmp (a0)` on the ¬cc edge. Aeon is genuinely backstopped — the strict
suite always runs the corpus gate — but "error tier" describes the per-file half
only. This is exactly `check_preserves`' shape, not a new hole.

This is `check_preserves`' three-way split copied faithfully, including its
rationale: a register that fails under `ClobberAll` but verifies under
`PreserveAll` is blocked SOLELY by a call, a fact only the whole-corpus closure
can settle, so the per-file gate stays silent and defers. Unlike `preserves`,
this check had no pre-existing corpus backstop, so one was built —
`out_verify_corpus::cond_out_survives_claims_all_prove` — rather than letting the
deferral become a silent pass.

**Gated on a DECLARED `clobbers(...)` clause** (per-file `proc.clobbers.is_some()`,
corpus `node.has_clobber_contract`), mirroring `check_clobbers`. §7.1's rule reads
clobbers MEMBERSHIP; a proc with no clobber contract states nothing about its
failure edges, so there is no claim to check. Imposing one would be inventing a
contract the author did not write.

68k only. The Z80 arm stays dead: `VALID_CCS` is the 68k set applied to both
CPUs, so a genuine Z80 `out(a if z)` is rejected as `[proc.out-cond-invalid]`
before this check is reached. The ledger's must-test-it-then instruction stands.

### §3.4 — Tail exits are charged their target

An unconditional tail transfer out of a ¬cc edge is a return of P from the
caller's view (out-verify's Finding 3, applied to the dual), so `Sites` treats it
as an exit — and charges the target's clobbers under the same `policy` a call
gets. `preserves(rN)` ignores a tail transfer because the closure accounts for it
via its tail edge; a scoped claim has no such backstop, and an unresolved target
preserves nothing. Zero corpus sites today; pinned both directions by test.

### §3.5 — The register-keyed nit

`ProcDecl::cond_only_out_regs(rf)` (and the `ProcSig` twin) is the new key for
the `[proc.out-clobbers-overlap]` exemption: a register is exempt only when
EVERY `out()` mention of it carries an `if cc`. It COUNTS reglist segments
covering the register against `if cc` clauses naming it, rather than subtracting
sets — which also catches a RANGE mention (`out(a0-a2, a1 if eq)` covers a1 twice
against one guard, so `clobbers(a1)` still errors there; the set-subtraction form
would have dropped a1 wholesale). So `out(a1, a1 if eq) clobbers(a1)` errors
again, and `out(a1 if eq) clobbers(a1)` still does not.

### §3.6 — The corpus edit (aeon)

`engine/level/tile_cache.emp` — `TileCache_FindStagedBlock` declares
`clobbers(d3-d4/a1) out(a1 if eq)`, matching the `Clobbers: d3-d4, a1` its own
header has always carried, plus a present-tense `CONTRACT` statement of the fact
(a1 is a result on the eq edge and scratch on the others; the probe walks it
before the hit/miss is known). Contract metadata; no codegen.

### §3.4 — What counts as an EXIT

Any edge that leaves the proc: an `Abandon` (`rts` / fall-off-end) or a `Defer`
(a transfer out, conditional or not). The claim is a promise to the CALLER, so it
must hold wherever control leaves, not only where it `rts`; a `bne ErrorPath` out
of the ¬cc edge would otherwise be a hole (lens C found exactly that hole in the
first draft, which only counted unconditional tails). `preserves(rN)`'s
`AllReturns` scope still ignores `Defer` entirely — unchanged.

The transfer's TARGET is deliberately **not** charged. The first draft did charge
it (an unresolved target preserves nothing ⇒ the claim dies), and lens C showed
that rejects an honest failure rail: a `raise_error` / noreturn handler owes the
caller nothing, and with no `@noreturn` in the language the analysis cannot tell
divergence from a real tail call. Charging it would be an error-tier false
positive on the very procs most likely to grow a "pool exhausted" rail. The
transitive half stays where `preserves` leaves it — the closure folds an
unconditional tail's effect into `effective` through its own tail edge.

### §3.5 — The register-keyed nit

`ProcDecl::cond_only_out_regs(rf)` is the new key for the
`[proc.out-clobbers-overlap]` exemption: a register is exempt only when EVERY
`out()` mention of it carries an `if cc`. It COUNTS reglist segments covering the
register against `if cc` clauses naming it, rather than subtracting sets — which
also catches a RANGE mention (`out(a0-a2, a1 if eq)` covers a1 twice against one
guard, so `clobbers(a1)` still errors there; the set-subtraction form would have
dropped a1 wholesale). So `out(a1, a1 if eq) clobbers(a1)` errors again, and
`out(a1 if eq) clobbers(a1)` still does not.

Riding it: `ProcDecl::cond_out_pairs(rf)`, the `(register, cc)` view. Lens B
caught that the split had been re-learned a SEVENTH time in pair form — the
set-returning accessors answer "which registers are guarded", so every consumer
needing the guard rebuilt `out_cond.iter().filter_map(Reg::from_name …)` with its
own canonicalisation. Both new consumers now take the accessor. It also drops a
register with an unconditional mention, which closes lens C's **U-2**: for
`out(a1, a1 if eq)` the survives claim's own remedy (`clobbers(a1)`) is itself an
error, so the contract had no legal spelling. Now it simply makes no claim.

### §3.6 — The corpus edit (aeon)

`engine/level/tile_cache.emp` — `TileCache_FindStagedBlock` declares
`clobbers(d3-d4/a1) out(a1 if eq) preserves(d0-d2)`, plus a present-tense
`CONTRACT` statement of the a1 fact. The `preserves(d0-d2)` is lens B's catch and
the parcel's own thesis applied one line up: the header had said "d0–d2
preserved" in prose for as long as it had said "Clobbers: d3-d4, a1", and the
parcel spent its whole budget converting the second into a checked declaration.
§5 verifies it (d0-d2 are read into d3 and never written). Contract metadata; no
codegen.

## §4 — THE ⊤ RULING — **REVERSED ON MEASUREMENT**

**RULED: ⊤ does NOT obligate.** The survives claim is charged only at an exit
where the cc is PROVABLY false. This reverses the overseer's recommendation, and
the reversal is the most important thing in this packet.

### §4.1 — Why firing at ⊤ looked right

§7.2 says to mirror `[proc.preserves-unverifiable]`, which errors on a proof
*bailout* — an unprovable contract treated as a wrong one. Mirroring means
firing. The first implementation did, and the first measurement supported it:

| polarity | corpus firings |
|---|---|
| fire at ⊤ | 1 — `TileCache_FindStagedBlock :: out(a1 if eq)` |
| skip at ⊤ | 1 — the same firing |

Identical. **That measurement was true and useless**, and the reason is the trap:
every return in both corpus claim sites ends in the `moveq #0` / `moveq #1`
Z-result convention, so no return in the corpus reaches ⊤ at all. A corpus-only
measurement of a ⊤ policy on a corpus with no ⊤ cannot say anything.

### §4.2 — What the panel measured past the corpus

Lens B built the probe the corpus could not be. Reproduced own-run before acting
(one honest `out(a1 if eq)` body per row, a1 genuinely untouched on the `!eq`
path, differing only in how the success path sets its Z result):

| success-path tail | fires at ⊤? |
|---|---|
| `moveq #0, d0` | no |
| `clr.w d0` | **YES** |
| `move.w #0, d0` | **YES** |
| `moveq #0, d0` ; `move.l d0, (a1)` | **YES** |
| `moveq #0, d0` ; `jbsr Init` | **YES** |

All five contracts are TRUE. Rows 2-3 are the same Z convention spelled with an
instruction the lattice does not fold; rows 4-5 are an allocator storing through
or initialising the slot it just handed back — the most ordinary thing such a
proc does. Any of them sends the flags to ⊤, which drags the **cc-SUCCESS**
return into the ¬cc set, where a1 is written *by contract*.

Lens C reached the same wall from the other side (**U-1**): `Flags::refine`
deliberately refines nothing for a composite condition, so a proc guarded by
`hi`/`ls`/`ge`/`lt`/`gt`/`le` leaves BOTH returns at ⊤ and cannot state a
survives claim at all. `out(d0 if ge)` on a signed bounds check is an ordinary
thing to want.

### §4.3 — Why the escape is not free (the argument that decides it)

The fire-at-⊤ case rested on "the honest downgrade is free — add rN to
`clobbers`". It is not free here. On these bodies the claim is TRUE, so
`clobbers(rN)` publishes a FALSE statement to buy compiler silence. That is
exactly the polarity §7.2 rejects when it turns down option (a): *"the house
never forces a weaker claim to be sharpened."* The dual holds — the house must
not force a true claim to be weakened. Lens C confirmed the diagnostic's other
advertised remedy is also unavailable: a correct save/restore round-trip across
the failure edge still fires, because the failing exit is the *success* return.

### §4.4 — Why the `preserves` mirror does not apply

The two ⊤s are different facts:

- `[proc.preserves-unverifiable]` at a bailout means **"your claim was checked
  and the proof machinery gave up."** Firing is right: the obligation is real and
  undischarged.
- A ⊤ cc means **"I cannot tell whether you made a claim at this exit."** Firing
  charges an obligation the author never incurred — and specifically, the exit may
  be the cc-success return, where writing rN *is* the contract.

`verify_out` keeps ITS obligation at ⊤ for the same reason inverted: its escape
genuinely is free and honest — produce the register. One lattice, opposite
correct answers, and the asymmetry is now documented at `not_cc_exit_sites`.

### §4.5 — The cost, named and pinned

A false NEGATIVE: a survives claim whose ¬cc exits are all unclassifiable goes
unchecked rather than misjudged. Bounded — every exit ending in the corpus's
`moveq` Z-result convention is classified, which is why both live claim sites are
fully checked. Pinned by
`an_all_unclassifiable_body_leaves_the_survives_claim_unchecked`, whose doc says
it is EXPECTED to flip when `Flags::after` learns `clr` / `move #imm`. That
widening (lens B) is ledgered rather than taken here, because `Flags::after` also
feeds `verify_out`'s production obligation, where more precision DROPS the
obligation at more returns — a false-negative direction on a shipping gate whose
30-firing residue is a recorded surface. Under the skip-⊤ ruling the widening can
only ADD checking to this gate, which is what makes deferring it safe.

## §5 — Corpus firing list

| proc | contract | verdict |
|---|---|---|
| `TileCache_FindStagedBlock` | `clobbers(d3-d4) out(a1 if eq)` | **FIRED** — a1 destroyed on the `!eq` exit. Fixed same parcel. |
| `AllocEffect` | `clobbers(d0) out(a1 if eq)` | **PASSES** — the required witness. |
| `AllocDynamic` | `clobbers(d0/a1) out(a1 if eq) preserves(a0)` | **SKIPPED** — a1 ∈ clobbers ⇒ no claim. |

After the flip: **0 firings**, gate assert-empty, and `survives_claim_sites`
pinned to `["AllocEffect"]` so the gate cannot go quietly vacuous by a contract
edit that deletes a claim instead of proving it.

## §6 — Tests, and why each is non-vacuous

`tests/lower_proc.rs` +14 (per-file ERROR gate, end-to-end from real `.emp`):

| test | pins | non-vacuity |
|---|---|---|
| `cond_out_survives_claim_proves_when_the_test_precedes_the_write` | the `AllocEffect` shape passes | paired with the row below — same terminals, only the write moves |
| `cond_out_survives_claim_fires_when_the_write_precedes_the_test` | the hoisted-pop regression fires, Error tier, names a1 and the remedy | its pair passes |
| `a_clobbered_cond_out_makes_no_survives_claim` | the `AllocDynamic` shape is skipped | same body as the firing test; only the clobbers clause differs |
| `an_unclassifiable_exit_is_not_charged_the_survives_claim` | the ⊤ ruling — an honest ⊤-on-success body compiles clean | fails under the fire-at-⊤ polarity (§4.2 row 4) |
| `cond_out_survives_claim_still_fires_past_an_unclassifiable_exit` | skipping ⊤ costs nothing when a classified exit exists | same ⊤ success path as the row above, write hoisted |
| `an_all_unclassifiable_body_leaves_the_survives_claim_unchecked` | THE COST, kept visible | expected to flip when the lattice widens |
| `a_save_restore_round_trip_carries_the_survives_claim` | the proof is `preserves`', not "never written" | a1 IS written here; the hoisted-pop test is the same write without the restore |
| `no_clobber_contract_means_no_survives_claim` | the `clobbers.is_some()` gate | same hoisted-pop body; only the clause is gone |
| `as_compat_does_not_silence_the_survives_claim` | tier discipline | carries a CONTROL assert (`[proc.clobber-undeclared]` absent) proving `@as_compat` is in force |
| `the_survives_skip_expands_the_clobbers_reglist` | `clobbers(d0-d1)` skips `out(d1 if eq)` | paired with the row below |
| `a_cond_out_outside_the_clobbers_range_still_claims` | the control — d1 outside the range fires | without it the range test passes with the check deleted |
| `a_register_named_unconditionally_too_makes_no_survives_claim` | lens C's U-2 | non-vacuous against the hoisted-pop firing, which is this body minus the plain `a1` mention |
| `an_unconditional_mention_defeats_the_cond_out_exemption` | `out(a1, a1 if eq) clobbers(a1)` errors | non-vacuous against the pre-existing `cond_out_may_overlap_clobbers` |
| `a_range_mention_defeats_the_cond_out_exemption` | `out(a0-a2, a1 if eq) clobbers(a1)` errors | the set-subtraction key passes this; only the counting key catches it |

The first draft's `the_survives_skip_is_canonical_not_textual` was **DELETED as
vacuous** — lens A proved it: `preserves` exempts `a7` from `ever_clobbered`
outright, so an `sp`/`a7` survives claim can never fire and the test passed with
the whole check removed. The range pair above replaces it with a property the
checker can actually observe.

`tests/out_verify.rs` +3 — the behaviours the per-file gate cannot show:

| test | pins |
|---|---|
| `a_call_on_the_not_cc_path_defers_rather_than_fires_per_file` | BOTH directions of the deferral (`ClobberAll` 1 firing, `PreserveAll` 0) — one side alone would make "defers" unfalsifiable |
| `the_oracle_settles_a_call_blocked_survives_claim_both_ways` | one body, two oracles: a preserving callee carries the claim, a clobbering one kills it |
| `a_tail_exit_on_the_not_cc_path_is_an_exit_but_its_target_is_not_charged` | a `jbra` out of the ¬cc edge with a1 destroyed FIRES (under `AllReturns` the same `Defer` is ignored, so this is the scope difference); the same tail with a1 untouched is CLEAN even for an unknown target — the noreturn-rail decision of §3.4 |

`sigil-cli/tests/out_verify_corpus.rs` +1 —
`cond_out_survives_claims_all_prove`: assert-empty over the real corpus under the
oracle, plus the `survives_claim_sites` pin. The obvious mutation (reverting the
`tile_cache` flip) lives in the OTHER repo, so no sigil-side test can perform it;
it was run by hand and the gate failed as designed, and the test's doc says so
rather than implying a mutation the suite could run.

## §7 — Byte identity (own-run, chain 40)

All four canonical shapes rebuilt one-per-invocation from this worktree pair at
the FINAL state, CRC compared against `crates/sigil-harness/golden/` in my own
worktree — goldens re-derived from the blobs, never quoted:

| target | built | golden (chain 40) | verdict |
|---|---|---|---|
| `s4.bin` | `730a9f99` / 379822 | `730a9f99` / 379822 | identical |
| `s4.debug.bin` | `b3aaa1df` / 423388 | `b3aaa1df` / 423388 | identical |
| `demo.bin` | `ea6213bc` / 65954 | `ea6213bc` / 65954 | identical |
| `demo.debug.bin` | `18e5ec7f` / 93963 | `18e5ec7f` / 93963 | identical |

Structurally expected: both aeon edits are contract declarations, and declared
clobbers/preserves reach only diagnostic producers. `config_a` / `config_b` ride
the strict off-canonical gates.

`repin --check`: **pins.rs unchanged.** `refreeze --check`: **OK (tip
`item28-bg-transpose`, chain len 40).** No re-freeze taken.

## §8 — Strict suite

`AEON_DIR=<aeon-wt> SIGIL_BUILD=… SIGIL_EMIT=… cargo test --workspace --release`,
full output captured to a file (no `tail`/`head` in the pipeline), `CARGO_EXIT=0`.

**3071 passed · 0 failed · 4 ignored**, across **305** result lines.
Failure scan (`FAILED` / `^failures:` / `panicked at` / `^error[` / `^error:`):
**empty**.

| file | added | running |
|---|---|---|
| master baseline | — | 3053 |
| `tests/lower_proc.rs` | +14 | 3067 |
| `tests/out_verify.rs` | +3 | 3070 |
| `sigil-cli/tests/out_verify_corpus.rs` | +1 | 3071 |

+18, exactly the tests in §6 (the deleted vacuous test is netted out: +15 at the
first gate-green, then +4 panel-driven additions −1 deletion). Result-line count
is unchanged at 305 — every test landed in an existing binary. The 4 ignored are
the standing set, none new.

`cargo clippy -D warnings` fails on master already (`sigil-ir/src/symbols.rs:55`;
the workspace run also never reaches past it to `sigil-frontend-as/src/eval.rs`
and `sigil-link/src/relax.rs`) — none mine. Over `sigil-frontend-emp` with only
the master `question_mark` finding allowed, two findings remain, both pre-existing
in code this diff does not touch (`lower/proc.rs:300` `manual_contains` inside
`check_z80_preserves`, `lower/regions.rs:116` `type_complexity`). **Net −1**: the
`apply_callee_effect` extraction removed a master `needless_range_loop`.

## §9 — Lens panel adjudication

The panel earned its keep again, and this time on the parcel's central ruling
rather than on comment hygiene. Every finding adjudicated.

### FIXED — soundness / correctness

| # | finding | disposition |
|---|---|---|
| B-1b | `Flags::after` folds only `moveq`; `clr` / `move #imm` / any store or call on the SUCCESS path sends flags to ⊤, and firing at ⊤ then rejects honest contracts at error tier. Four measured cases. | **Reproduced own-run (found a fifth: any call), then REVERSED THE ⊤ RULING.** §4. The single most valuable finding of the parcel. |
| C-U1 | A composite cc (`hi`/`ls`/`ge`/…) leaves both returns ⊤, so a survives claim is unstateable; the save/restore remedy is also unavailable because the failing exit is the success return. | Same root cause, **fixed by the same reversal**. Independent confirmation from the opposite direction is what made the reversal unarguable. |
| C-FN1 | A conditional branch to an external symbol on the ¬cc edge is invisible at BOTH tiers — total silence in the parcel's own target class. | **FIXED.** An exit is now ANY edge leaving the proc (`Abandon` or `Defer`), not just unconditional tails. The specific example is additionally covered by ⊤-skip, but the hole was real. |
| C-FP1 | A divergent tail (`raise_error` rail) on the ¬cc edge is charged its target and fires — no honest remedy, no `@noreturn` in the language. | **FIXED** by dropping the target charge; the exit stays a checkpoint for LOCAL state. §3.4. Test rewritten to pin both halves. Ledgered `@noreturn` as the thing that would let the charge come back. |
| C-U2 | `out(rN, rN if cc)` has no legal remedy — the survives claim fires and `clobbers(rN)` is itself `[proc.out-clobbers-overlap]`. | **FIXED** — `cond_out_pairs` drops a register with an unconditional mention, at both tiers. Wall test added. |
| B-2b | `ReturnScope::Sites::checks` ignored `is_return`, so an index in the set for its `Abandon` also had its `Defer` charged, contradicting the documented exclusion. | **FIXED** — dissolved by the uniform exit model; the exclusion prose it contradicted is gone. |
| B-2a | Missing-status polarity: `Some(Verified) \| None => continue` passes an absent proof, opposite to `check_preserves`. | **FIXED** — only a positive `Verified` clears the claim; `None` produces a firing with its own reason. |
| A-B1 / B-5b | `survives_message`'s doc claimed it was shared by both gates; it had ONE caller, with the corpus test and `emp_contracts` rolling two more renderings. | **FIXED** — all three now route through `survives_message`; the doc is true. |
| A-E1 | `the_survives_skip_is_canonical_not_textual` is VACUOUS — a7 is exempt from `ever_clobbered`, so the test passes with the check deleted. | **FIXED by deletion**, replaced with the observable range pair. A green test asserting nothing is worse than no test. |
| B-5d / A-E2 | The corpus gate's non-vacuity assert pinned `verified_cond_out`, which is the PRODUCTION half's output and survives every claim being downgraded away. | **FIXED** — `ContractReport::survives_claim_sites` exposes the claim set and the gate pins it. |

### FIXED — ceremony, convention, hygiene

A-A1/A2/A3/A4/A5/A6 (six comment-discipline violations: change-history narration
in `flags_after`'s and `State`'s docs, "the relax created", "the set-subtraction
form dropped", a `B′-0` parcel tag in a test doc, and "(the ruling)" as process
metadata) — **all fixed**, two of them self-caught before the panel reported.
A-B2/B3/B4/B5/B6 (stale docs the diff invalidated: `PreserveStatus`' quantifier,
three `preserves.rs` accumulator comments plus their `saw_return` /
`all_returns_preserve` / `bailed_reached_return` names, `join`'s vanished
`produced` field, the section banner, and `lower/proc.rs` saying "untouched" where
the criterion is "holds its entry value") — **all fixed**, including the rename to
`saw_exit` / `all_exits_preserve` / `bailed_reached_exit`.
A-C3 / B-6 (the 18-line corpus preamble tripled, walking the aeon tree three times
per run) — **fixed**, extracted to `corpus_report()`.
A-C4 / C-5 (`flags_after` rebuilt per `(reg, cc)`) — **fixed**, hoisted.
A-D1 (`check_cond_out_survives` shadowing the `out_verify` function it wraps) —
**fixed**, renamed `check_survives_claims`.
A-D2/D3 (the firing reason said "written and not restored" for call- and
tail-derived failures, and spliced `preserves`' raw sp-hazard text) — **fixed**.
A-D4 (doc claimed a gate the function did not implement) — **fixed**, reworded to
"the caller gates this on…".
A-E3 (the two exemption walls had mismatched rigor) — **fixed**, both now pin
level and register.
B-4b (the cond/uncond split re-learned a SEVENTH time, in `(reg, cc)` pair form) —
**fixed**, `ProcDecl::cond_out_pairs`; both consumers migrated.
B-5a (sort key missing the span/cc tiebreakers every sibling carries) — **fixed**.
B-5c (`SurvivesFiring` names a verb, off the `<Subject>Firing` pattern) —
**fixed**, `CondOutSurvivesFiring`.
B-3 (Z80: the corpus walk had no CPU gate, inert only by accident) — **fixed** by
comment naming both accidents and the ledger row that will disturb them.
A-C2 / B-4a (the ledger close claimed a `ProcSig::cond_only_out_regs` twin that
does not exist) — **fixed** in the ledger text rather than by adding dead code.
B-0 (the reviewed commit lacked four uncommitted improvements) — noted; everything
is squashed into one commit per repo below.

### DECLINED, with reason

| # | finding | why not |
|---|---|---|
| B-1b (the widening half) | Lift `clr` / `move #imm` into `Flags::after`, ideally unifying with `branch_const`'s lattice. | **Right, but not here.** `Flags::after` also feeds `verify_out`'s PRODUCTION obligation, where more precision DROPS the obligation at more returns — a false-negative direction on a shipping ERROR gate whose 30-firing residue is a recorded surface. It needs its own parcel with a residue-delta measurement. Under skip-⊤ it can only ADD checking here, so deferring is safe. Ledgered with the `cc_transparent` drift and the duplicate branch-cond table. |
| B-1c | Replace the flag lattice with edge-identification + reachability (`valid_edge`/`invalid_edge`). | Correct diagnosis of the mechanism's ceiling, and a genuine redesign of a shipping primitive's consumer. Out of scope for a one-sitting parcel; the ⊤ reversal removes its urgency (imprecision now costs checking, not false rejections). Ledgered inside the lattice row. |
| C-FN2 | A cond-out proc with no `clobbers(...)` clause escapes entirely. | Kept, because §7.1's rule reads clobbers MEMBERSHIP and the house convention (`check_clobbers`, `closure.rs`) already reads an absent clause as "no contract". But lens C is right that this is inherited, not ruled — **escalated as a ruling ask**, not silently kept. |
| C-FP2 | The oracle is edge-blind, so a truthful TRANSITIVE survives claim (a wrapper over a cond-out callee) cannot be proven. | Correct-but-imprecise, and the fix is an edge-sensitive `RegEffect` — the same primitive an edge-precise D1c wants. No corpus site. Ledgered. |
| A-B7 / B-7 (the deletion half) | Delete `tile_cache.emp`'s `// Clobbers:` and `// d0–d2 preserved` prose now that both are declared. | The DECLARATION half was taken (`preserves(d0-d2)` added and verified). The deletion is declined: ~40 sibling procs in that file carry the same header convention, so removing one proc's lines trades one inconsistency for another. Ledgered as a corpus-wide sweep. |
| A-C1 | `ProcDecl::cond_out_regs` is now test-only (both production callers moved to `cond_only_out_regs`). | Kept — it is half of the documented accessor pair the six-times ledger row demanded, `ProcSig::cond_out_regs` is live, and deleting one half invites the seventh re-derivation. Noted here so a reader is not surprised. |
| C-6 nits | `ever_clobbered` unscoped under `Sites`; `bail_reason` is the first bail anywhere in the body. | Neither changes a verdict — the first only weakens a precision arm, the second only picks message text. Recorded. |
| C-5 (batching) | Batch `verify_preserved_on` across registers and share one `Cfg`. | `flags_after` was hoisted (the real repeat); the rest is k ≤ 1 per proc today. Not worth the churn against a fresh analysis. |

## §10 — Step-3 (LANGUAGE) vs step-5 (ENGINE)

**Step-3 — what this taught the .emp LANGUAGE:**

1. **A two-part contract needs two rulings, and mirroring is not one of them.**
   `out(rN if cc)`'s halves look like duals, so the natural instinct — and the
   spec's instruction to mirror `[proc.preserves-unverifiable]` — was to give ⊤
   the same answer in both. Measurement says the halves are asymmetric at ⊤,
   because their ESCAPES are asymmetric: production's escape (write the register)
   is honest, survival's escape (declare it clobbered) is a lie whenever the claim
   is true. **The rule this suggests for the language: a diagnostic may only be
   error-tier where its documented remedy is always TRUE, not merely always
   available.** B′-0's error tier was justified on availability alone.
2. **`@noreturn` is now demanded by two analyses.** Without it a transfer out
   cannot be told from a divergent rail, which forced this parcel to accept a
   false negative it could otherwise have closed. Ledgered.
3. **`RegEffect` wants to be edge-sensitive.** A conditional out's clobber is
   real on one edge and absent on the other; a flat per-proc set cannot say that,
   which is why a transitive survives claim is unprovable AND why D1c's two
   documented false positives persist. One primitive would close both.
4. **The cond/uncond `out` split has now been re-learned SEVEN times** — the
   seventh in `(reg, cc)` pair form, which the set-returning accessors added by
   B′-0c could not prevent. Closed with `cond_out_pairs`.
5. **The spec's own rider was wrong** (§0), and the ledger row for the D1c FP
   class is corrected rather than closed.

**Step-5 — what this taught the ENGINE:** one real contract tightening —
`TileCache_FindStagedBlock` gains `preserves(d0-d2)`, converting a prose header
claim into a machine-checked one, alongside the honest `clobbers(d3-d4/a1)`. No
instruction was read, moved, or re-timed; both ROMs of both games are
byte-identical.

**Neither bucket — the transferable lesson.** *A measurement is only as good as
the population it ranges over, and a corpus is not a population.* The first ⊤
measurement was correct, reproducible, and worthless: it compared two policies on
a corpus that contained zero instances of the condition the policies differ on,
and returned "identical" — which reads as "no cost" and is really "no evidence".
The overseer's brief said "measure the real cost before you commit", and the
parcel did measure, and still got it wrong, because it measured the only thing
that was easy to measure. What caught it was a lens agent constructing inputs the
corpus does not contain. **For any ruling about how an analysis behaves at its
imprecision boundary, the corpus is the wrong instrument by construction — the
corpus is exactly the set of programs that already compile.**
