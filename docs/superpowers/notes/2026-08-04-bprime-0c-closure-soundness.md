# 2026-08-04 — B′-0c: the closure-soundness batch (close packet)

Status: Merge state lives in the campaign log, not here. Built as branch pair
`bprime-0c`, cut from the post-B′-0-merge masters — sigil `4a21063a` /
aeon `b96051a`. (Commit ids are not quoted here: this packet lives INSIDE the
commit it would name, so any id written down is stale by construction — read
`git log` on the branch pair.) The aeon commit is an EMPTY pair marker recording
the byte-bar provenance point: this parcel is entirely sigil-side, and the queue
may drop it.

Spec: `specs/2026-08-04-contract-delta-spec.md` §7.3 (six items), normative
semantics in §7.1. Ledger: the six `bprime-0 lens` rows closed same-commit, one
new row opened.

## §0 — THE HEADLINE

**All six items were real, all six shipped, and NONE of them changes a single
firing over the aeon corpus.** The whole-corpus §9 / D1b / D1c / §6 / out-verify
firing sets are byte-for-byte identical before and after — as each ledger row
predicted ("latent", "not exhibiting today", "unreached today"). That is the
point: this batch closes holes that are one contract edit away from mattering,
before the contexts parcel (B′-1) lands on top of them.

**The aeon repo carries NO code change.** Every fix is sigil-side; the aeon
branch exists only to pin the byte bar and to carry nothing. All four canonical
ROM shapes are byte-identical to the goldens **in this worktree at this chain**.

**The batch found one NEW hole while fixing item 2/3** — the §4 subcontract
relation now compares conditional-out registers but still not their condition
CODES, so `out(a1 if ne)` satisfies a bound demanding `out(a1 if eq)`. Ledgered
open rather than fixed: it needs `(reg, cc)` pairs in `Contract`, which is spec
surface.

**Item 3 shipped WRONG in the first draft and the era lens panel caught it.**
The draft implemented lens-C's original ledger row (`∪ bound.out_cond`) instead
of the delta spec's `∪ bound.out`, which §7.1 of the same document invalidates —
the extra term erased the survives-claim that a cond-out register's ABSENCE from
`clobbers` encodes. It is corrected here, both walls are pinned in both
directions, and §3 records the reasoning; the two tests that had pinned the
unsound case are retargeted. Two smaller panel findings (an expander mismatch in
`contract_of_sig`, an overstated reach claim for item 5) and a set-vs-multiset
weakness in the new D1c gate are likewise fixed, each with its own pin.

## §1 — Item 1: `contract_type_bound` drops the bound's `out`

**HIGHEST PRIORITY, and it earned the label.** `corpus_contracts.rs`'s
`contract_type_bound` returned only the contract type's `clobbers` (or the
preserves-complement). `extern_node`, twenty lines above, deliberately does
`effective = clobbers ∪ out` with the reasoning recorded in place: an `out`
register is WRITTEN by the callee, so a caller relying on it across the call is
wrong and must be charged it. The contract-type path did not.

**Why the failure mode is destructive, not permissive.** The same `effective`
set feeds `preserves::find_dead_saves`. A bound narrower than the truth makes a
save that is load-bearing *precisely because the dispatch target writes an out
register* appear dead — the D1d worklist would tell a porter to DELETE it.

**Fix:** `regs.extend(expand_reglist_regs(t.sig.out.as_deref().unwrap_or(&[])))`.
Conditional outs are INCLUDED: the register is written on the cc edge, so from
the caller's side it is destroyed on every edge — the FULL reglist is the
conservative read here, not `unconditional_outs`.

**Non-vacuity (measured):** with the `extend` line deleted,
`bounded_indirect_charges_the_bounds_out` and
`bounded_indirect_charges_a_conditional_bound_out` both FAIL (27 passed / 2
failed). A third pin, `bounded_indirect_with_the_out_declared_is_clean`, is the
wall — the fix widens the bound, it does not make bounded dispatch unusable.

**Corpus impact: none.** The live shape (`player_sensors.emp`'s `SensorProbe …
out(d0, d1, d2)` reached via `jsr (a2)`) stays covered because
`Player_SensorPair` writes d0–d2 locally, exactly as the ledger row said.

## §2 — Item 2: `contract_of_sig` drops `out_cond` on the floor

The parser folds an `out(rN if cc)` register into the plain `out` reglist as
well as `out_cond`; `contract_of_sig` expanded only the reglist, so a
`Contract` could not tell an unconditional promise from a conditional one. A
target producing `rN` only on its `eq` edge silently satisfied a bound
promising `out(rN)` on every return — and since B′-0 that target may
simultaneously and legally declare `clobbers(rN)`, i.e. state outright that it
destroys the register the bound promises callers.

**Fix:** `Contract` gained an `out_cond: BTreeSet<String>` field.
`contract_of_sig` subtracts the conditional registers out of `out` into it (via
the new accessor, §6). `subcontract_violations` reads the two apart:

- `bound.out ⊆ target.out` — an unconditional promise needs an unconditional
  producer. The bound's callers may read the register with no cc test at all.
- `bound.out_cond ⊆ target.out ∪ target.out_cond` — a conditional promise is
  satisfied by a conditional OR an unconditional producer (producing on every
  path is strictly stronger).

**Both halves come from ONE expander.** `Contract::out` is
`sig.unconditional_outs(M68k)` and `Contract::out_cond` is
`sig.cond_out_regs(M68k)`; the `regfile::expand_reglist` call on the out reglist
survives for its DIAGNOSTICS only. The first draft subtracted an accessor-produced
subtrahend from a `regfile`-produced minuend — the two disagree on `sp` (the seam
drops it as `[contract.unknown-register]`, the production expander canonicalizes
it to `a7`), which is the raw-vs-canonical bug class of item 5 reproduced two
lines from the accessor built to prevent it. Sound only because both sides
happened to be empty; corrected after the panel flagged it.

**Non-vacuity (measured):** with the conditional subtraction removed,
`probe_hook_conditional_out_does_not_satisfy_unconditional_promise` and
`probe_hook_survives_claim_rejects_a_target_that_clobbers_the_register` FAIL
while the conformance pin still passes.

## §3 — Item 3: `subcontract_violations`' asymmetric clobber test

`target.clobbers ⊆ bound.clobbers` was tested with no `∪ out`, while
`bound.out ⊆ target.out` was tested separately — asymmetric. A register the bound
itself declares an UNCONDITIONAL result is one every caller of the bound already
knows is written, so charging the target for writing it rejects a conforming
target.

**Fix:** `target.clobbers ⊆ bound.clobbers ∪ bound.out`, exactly as the delta
spec §7.3 item 3 specifies.

**`out_cond` is deliberately NOT a term** — and the first draft of this parcel got
that wrong. It implemented the lens-C ledger row's `∪ bound.out_cond`, which §7.1
of the same document invalidates: for `out(rN if cc)`, rN ABSENT from `clobbers`
normatively means "rN is PRESERVED on every ¬cc return path", and `Contract`
encodes that claim purely by absence from `clobbers`. A third term reading
`out_cond` erases exactly the set that carries it. The concrete break, caught by
the era lens panel: bound `hook alloc() clobbers(d0) out(a1 if eq)` (the
AllocEffect shape — its callers are entitled to hold a1 across the call and
re-read it on the ne edge) accepting target `AllocDynamic clobbers(d0/a1)
out(a1 if eq)` (a1 indeterminate there), with zero diagnostics. Every hook caller
holding a1 would be silently broken. Latent — no hook bindings exist in aeon yet.

**Pins (5), and what each one proves:**

- `a_bounds_out_licenses_the_target_to_clobber_it` and
  `the_out_license_does_not_widen_to_arbitrary_registers` are the REGRESSION pins
  for the `∪ out` term — with the term removed, both FAIL (measured: 25 passed /
  2 failed). The second is the scope wall: a target clobbering `d5` still
  violates.
- `a_conditional_out_does_not_license_the_target_to_clobber_it` (unit) and
  `probe_hook_survives_claim_rejects_a_target_that_clobbers_the_register`
  (end-to-end) are the SOUNDNESS pins for the absent `out_cond` term — with the
  term restored, both FAIL (measured).
- `the_honest_alloc_dynamic_shape_conforms_to_a_matching_bound` and its
  end-to-end twin pin the motivating shape: bound `clobbers(d0/a1) out(a1 if
  eq)`, target identical, conforms via the bound's own `clobbers`. **Stated
  honestly: this pair is a WALL, not a regression pin** — that shape conforms
  with or without the `∪ out` term, because a1 is in `bound.clobbers`. It exists
  so a future tightening cannot break the shape B′-0 made expressible.

## §4 — Item 4: D1c gate teeth

`out_verify_corpus.rs` printed the `[call.live-clobbered]` firings with
`eprintln!` and asserted nothing about them, while every sibling gate asserts
`is_empty()`. So "3004 passed" was not evidence that D1c firings were unchanged
by a contract edit — B′-0's own neutrality had to be argued structurally and
checked by hand.

**Fix:** `d1c_firings_match_the_frozen_baseline` pins the exact 21-row
`(caller, callee, register)` set. The brief called for allowlisting the
documented `Load_Object @ AllocDynamic :: a1` FP; the honest form is a FROZEN
FULL BASELINE, because the corpus's D1c surface is 21 rows, not one. The table
marks the **two** documented edge-blind FPs (`Load_Object @ AllocDynamic :: a1`
and `TileCache_FillRow @ TileCache_FindStagedBlock :: a1`, both reasoned out in
`calls.rs::destroys_value`'s header). The other 19 rows are **recorded, not
adjudicated** — the doc comment says so rather than implying a clean bill of
health. D1c is observe-only; the baseline's job is to make the set immovable,
not to claim it is empty.

The failure message separates NEW firings from GONE ones and names why GONE is
the dangerous direction (a narrowed effective set also feeds `find_dead_saves`).
The dump test is kept alongside — the baseline gives teeth, the dump gives the
adjudication surface.

The added/removed diff is a **multiset count**, not a set membership test: the
same `(caller, callee, register)` triple can fire at two call sites in one proc,
and with `Vec::contains` such a firing gaining or losing a duplicate would land
in neither list and surface only at the trailing assert whose message blames
ordering. Counting names it.

**Non-vacuity (measured twice):** deleting one baseline row
(`Ground_Move_Cap @ Player_SensorWallDir :: d0`) fails with that row reported as
NEW; DUPLICATING one row fails with it reported as GONE with its count.

## §5 — Item 5: raw-vs-canonical cond names

`corpus_contracts.rs:292` filtered CANONICAL `outs` against RAW `cond` register
names, so `out(sp if eq)` — canonical `a7` in the out set, raw `sp` in the cond
set — was credited as an UNCONDITIONAL out. That is the false-negative polarity
on a shipping ERROR gate (D1b must-def credit, §6 taint-kill), and the exact bug
B′-0 had just fixed in `check_out`.

**Fix taken one level up from the ledger's:** rather than canonicalizing at the
:292 subtraction, `conds_of` canonicalizes at COLLECTION time.

**Honest scope (corrected after the panel):** ONE consumer needed it — the
`callee_uncond_out` subtraction, which compares raw against canonical. The other
two (`conditional_out_edge_credits`' operand match and `check_out`'s cond list)
already read these names through `Reg::from_name`, which is alias tolerant, and
were never broken. Canonicalizing at collection makes the invariant a property of
the map rather than a duty each consumer must remember; it does not fix three
bugs. An unrecognizable name keeps its raw spelling (it matches nothing
downstream, which is the same outcome as dropping it, minus the silent loss).

**Non-vacuity (measured):** with raw names,
`conditional_out_spelled_sp_is_not_credited_unconditionally` FAILS while its
wall `unconditional_out_spelled_sp_is_still_credited` passes — the canonicalization
must not swallow a genuine unconditional result.

## §6 — Item 6: the `unconditional_outs()` accessor

`ProcDecl::unconditional_outs(rf)` / `ProcDecl::cond_out_regs(rf)` (and the same
pair on `ProcSig`) now live in `ast.rs`, CPU-parametric: 68k routes through the
frozen production expander (`sp`→`a7`, `sr` dropped, movem ranges), Z80 through
the register-file seam (pair sugar splits to halves). `ast.rs`'s "pure data — no
semantics" header is amended to say what it now also carries.

The accessor doc states the **dividing line**, which is the fact that had been
re-learned six times: a gate that treats an out as a DEFINITION on every return
edge takes `unconditional_outs`; a gate asking "does the callee WRITE this"
takes the full reglist.

**Migrated (3 sites):** `lower::check_out` (both CPU arms — the Z80 arm's
`regfile::expand_reglist` and the 68k arm's `reglist_set_quiet` become one
call), `resolve::contract_of_sig` (**both** the minuend and the subtrahend — the
first draft migrated only the subtrahend and this packet claimed the site done;
see §2), `corpus_contracts::conds_of`.

**NOT migrated, deliberately, and now commented in place (2 sites):**
`corpus_contracts::extern_node` and `sigil-harness/seam1.rs`'s stub derivation.
Both need the FULL out. seam1's is the sharp one: it SUBTRACTS `produced` from
the register universe to derive a preserves set, so dropping conditional outs
there would claim a written register preserved — the accessor would have made
it WRONG. This is the honest count correction to the ledger row's "six sites":
**three sites subtract and now share one implementation; two consume the
already-subtracted map (`calls::destroys_value`, `out_verify::check_out`); two
deliberately do not subtract at all.** "Six independent subtractions" was an
over-count in the original row.

**Non-vacuity (measured):** with the guard set made textual,
`unconditional_outs_is_canonical_not_textual` and
`unconditional_outs_expands_z80_pairs` FAIL. Five pins total, including the
range case (`out(d0-d2, d1 if eq)` leaves d0+d2) and the no-`out` case.

## §7 — Byte identity (own-run, against the CURRENT chain)

All four canonical shapes rebuilt fresh from this worktree pair with
`SIGIL_BUILD`/`SIGIL_EMIT` pointed at the branch binaries, one shape per
invocation, and CRC-compared against `crates/sigil-harness/golden/` **in this
worktree** — never against a packet-quoted CRC:

| target | built | golden (this chain) | verdict |
|---|---|---|---|
| s4.bin | `3879b953` / 384048 | `3879b953` / 384048 | identical |
| s4.debug.bin | `2623ee7f` / 423383 | `2623ee7f` / 423383 | identical |
| demo.bin | `f7a93a04` / 70180 | `f7a93a04` / 70180 | identical |
| demo.debug.bin | `e3243cbb` / 93943 | `e3243cbb` / 93943 | identical |

config_a / config_b ride the strict off-canonical gates (§8). The aeon worktree
was seeded with the gitignored `games/sonic4/data/editor/` (196 files, rsync'd
from the main checkout) before any build — the known wrong-ROM trap.

Byte neutrality here is structural, not lucky: **the aeon tree is unmodified by
this parcel**, and every sigil change is in a diagnostic producer. No backend
consumes a clobber/out set (registers are author-written; there is no allocator),
and the one byte-affecting path — D1d dead-save elimination — is a printed
worklist, not a transform.

## §8 — Strict suite

`AEON_DIR=<aeon-0c> cargo test --workspace --release`, FULL output captured to a
file (no `tail`/`head` in the pipeline — that truncates the log AND returns
`tail`'s exit code).

**3025 passed · 0 failed · 4 ignored**, `CARGO_EXIT=0`, over 302 `test result:`
lines (289 test binaries + 13 doc-test targets). Baseline was 3004/0/4 — the
delta is **exactly +21**, the count of tests this parcel adds (5 corpus-contract,
8 subcontract, 5 accessor, 3 game-contract, 1 D1c gate).

Failure scan (`FAILED` / `failures:` / `panicked at` / `error[` / `error:`) over
the full log is EMPTY. The 4 ignored are the standing set, none of them new:
`chained_resume_plain` + `chained_resume_debug` +
`secondary_pin_classes_match_the_hand_typed_baseline` (all three RETIRED by
Wave-B B-0's packed placement) and `sigil_diff_reports_byte_identity` (opt-in
`--ignored`, reads the aeon tree).

`cargo clippy --workspace --release --all-targets` diffed against a `git stash`ed
baseline: **no new warnings.**

`repin --check` / `refreeze --check`: no re-freeze taken and pins untouched (the
parcel emits no bytes and does not touch the aeon tree); the golden gates in the
suite above are the operative proof.

**The D1c gate now carries its own weight.** Unlike B′-0, whose neutrality had to
be argued structurally because the strict suite could not see a D1c regression,
this run's 3025 INCLUDES `d1c_firings_match_the_frozen_baseline`. The corpus's
D1c surface is pinned by the suite, not by hand.

## §9 — Step-3 (retrospect) vs step-5 (engine) findings

**Step-3 (language/spec):**

1. **NEW HOLE, opened by the item-2/3 fix: the §4 relation compares conditional-out
   REGISTERS but not their condition CODES.** A target declaring `out(a1 if ne)`
   now satisfies a bound declaring `out(a1 if eq)` — the register is produced on
   *some* edge, but not the one the bound's callers test. Register-level was the
   right first cut (the split had to exist before the cc could be compared), but
   this is a real hole in the relation, not a nit: the two contracts describe
   opposite edges. Fix: carry `(reg, cc)` pairs in `Contract::out_cond`.
   **Ledgered.**
2. **The "six subtract sites" count in the ledger was an over-count** — three
   sites subtract, two consume the already-subtracted map, and two must NOT
   subtract. The accessor's value turned out to be less "stop repeating the
   subtraction" and more "write down which of the two views a consumer wants",
   which is now its doc comment. The seam1 site is the proof that a blanket
   migration would have introduced a bug.
3. **The brief's item-4 scope ("allowlist the documented FP") was written against
   a one-row picture of a 21-row surface.** The gate shipped as a frozen full
   baseline with the two documented FPs marked and the other 19 explicitly
   recorded as un-adjudicated. Claiming they were all FPs would have been the
   comfortable lie; leaving the gate toothless to avoid the question would have
   been the other one.
4. **`Contract::params` is NOT canonicalized** (`resolve/contract.rs` keeps the
   raw param spelling while every other field is expanded through the register
   file). Same class as item 5, different field; unreached today because no
   corpus signature spells a param `sp`. Not taken — outside the six items.
   **Worth a row if the panel wants it.**

**Step-5 (engine):** none. Contract metadata and diagnostics only; no
instruction was read, moved, or re-timed, and the aeon tree is untouched.

**Neither bucket — the transferable lesson.** Every one of the six items was
verified against the code before being built, per the brief's instruction, and
all six were still live. What the verification changed was the SHAPE of two of
them: item 4's baseline is 21 rows rather than the one the spec named, and item
6's migration is three sites rather than six with two that must be left alone.

**What verification did NOT catch — and the panel did.** Item 3 shipped a
soundness regression because the parcel built the LEDGER ROW rather than the
SPEC. The row (`∪ bound.out_cond`) predates §7.1 of the same document; §7.1 makes
a cond-out register's absence from `clobbers` a normative survives-claim, which
the extra term erases. Every bar was green on the wrong semantics — byte-neutral,
3023/0/4, corpus firings unchanged — because the failure is latent (no hook
bindings in aeon) and the parcel's own tests pinned the unsound case as correct.
**A test written from the same misreading as the code cannot detect the
misreading.** The rule this earns: when a spec and a ledger row that spec
supersedes disagree, the disagreement is itself the signal — reconcile it in the
packet before writing the code, not after the panel asks.

The same round also produced the expander mismatch in §2 (the accessor built to
prevent raw-vs-canonical subtraction, then bypassed on one side of a subtraction
two lines away) and the set-vs-multiset weakness in §4 — both green under every
bar, both real.