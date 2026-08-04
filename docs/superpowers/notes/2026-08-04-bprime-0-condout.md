# 2026-08-04 — B′-0: the conditional-out overlap relax (close packet)

Status: **countersigned and MERGED** — sigil `4a21063a`, aeon `b96051a`. The
merge waited for a quiet window and a rebase, exactly as this line originally
predicted it would need to: the concurrent engine session moved both masters
(chain 33 → 36 during the build session alone, and 36 → 38 while the parcel
waited, with `demo.debug` changing SIZE in between). Branch pair
`bprime-0-condout` (sigil + aeon),
**Rebased 2026-08-04 onto the post-overnight-run masters** — sigil `b36932e0`
(refreeze chain entry 38 `item28-bg-guard` + the §7 rulings), aeon `f67cc4f`
(review items 25-30 executed). Originally cut at chain 36 (sigil `df7eaccf` /
aeon `26dc266`); every byte bar below is the chain-38 re-proof.

Spec: `specs/2026-08-04-contract-delta-spec.md` §1. Ledger: gap-ledger t24 row
(~:1603), closed same-commit.

## §0 — THE HEADLINE

**The spec's hypothesis was WRONG, and the pinning test is why we know.** The
delta spec said the t24 ask was "likely closed by construction" (reasoning:
`out_cond` is a separate AST field from the set the overlap check iterates) and
told the porter to pin it rather than assume it. The pin **failed on first
run** — the honest declaration was still rejected, exactly as the `core.emp`
site comment claimed. The spec's own fallback ("if the positive FAILS, the
relax is a one-sitter scoped by the t24 row") is what shipped.

Had the hypothesis been taken on faith, the ledger row would have been closed
false and `AllocDynamic`'s contract would still be lying.

## §1 — Root cause (measured)

`parser::out_list` (`crates/sigil-frontend-emp/src/parser.rs`) handles
`rN if cc` by pushing the register into **both** collections:

```rust
conds.push(CondResult { reg: lo.clone(), cc, span });
regs.push((lo, None));
```

The comment there explains why the second push is correct — out-verify must see
the register to check it is written. But `check_out` then expands `proc.out`
(which includes that register) and tests every member against `clobbers`, so a
conditional result read as an unconditional one and fired
`[proc.out-clobbers-overlap]`.

## §2 — The fix

`lower/proc.rs::check_out` builds `cond_guarded` — the lowercased register names
carrying an `out(rN if cc)` guard — and exempts them from the **clobbers**
overlap on both CPU paths (68k and the Z80 early-return branch). Scope
discipline:

- **`out ∩ preserves` still fires for a conditional result.** Written on ANY
  path contradicts untouched on ALL paths.
- **Unconditional `out` + `clobbers` still errors.**
- The register stays in the out reglist, so out-verify's written-ness check is
  unchanged.
- **The guard set is canonical, not textual.** It is expanded through the same
  register file as the sets it is tested against, so `sp`/`a7` on 68k and Z80
  pair spellings (`hl` → `h`+`l`) cannot slip past. The first draft of this
  parcel got this wrong on both CPU paths; the lens panel caught it.

All three walls are pinned by tests (§3) — the first draft asserted this
scope discipline in prose while pinning only the second.

## §3 — Tests (sigil)

`crates/sigil-frontend-emp/tests/lower_proc.rs`, +4:

- `cond_out_may_overlap_clobbers` — the AllocDynamic shape
  (`clobbers(d0/a1) out(a1 if eq)`) compiles with no overlap diagnostic.
- `uncond_out_still_may_not_overlap_clobbers` — the wall: unconditional
  `out(a1)` + `clobbers(a1)` still errors at Level::Error naming `a1`.
- `cond_out_may_not_overlap_preserves` — the preserves half is NOT relaxed.
- `cond_out_exemption_is_canonical_not_textual` — `clobbers(a7) out(sp if eq)`
  must not fire. Non-vacuity confirmed by probe: with a textual guard set the
  sets are `{"sp"}` vs `{"a7"}` and the diagnostic fires.

`cargo test -p sigil-frontend-emp --test lower_proc`: **87 passed, 0 failed, 0
ignored.**

## §4 — Corpus (aeon)

`engine/objects/core.emp` — `AllocDynamic` declares the honest contract:

```
pub proc AllocDynamic () clobbers(d0/a1) out(a1 if eq) preserves(a0) {
```

The ten-line `CONTRACT CAVEAT` block that stood in for the missing grammar is
replaced by a present-tense `CONTRACT` statement of the same fact (a1 is a
result on the eq edge, destroyed scratch on every other; callers holding a1
must save on every path). Per the comment rule: no change-history narration, no
"was rejected" framing — the comment states what is true now. The `Clobbers: d0`
header line becomes `Clobbers: d0, a1 (a1 is a result ONLY on the eq edge)`.

Contracts are metadata; no codegen.

## §5 — Byte identity (own-run, against the CURRENT chain)

All four canonical shapes rebuilt fresh from the worktree pair and CRC-checked
against `crates/sigil-harness/golden/` — **not** against any packet-quoted CRC
(the audit's standing warning). **Re-proven at chain 38 after rebasing onto the
post-overnight-run masters** (sigil `b36932e0`, aeon `f67cc4f`); the chain-36
figures this packet first carried are superseded, and note `demo.debug` changed
SIZE between the two chains — a packet CRC reused across a rebase would have
been silently wrong:

| target | built | golden (chain 38) | verdict |
|---|---|---|---|
| s4.bin | `3879b953` / 384048 | `3879b953` / 384048 | identical |
| s4.debug.bin | `2623ee7f` / 423383 | `2623ee7f` / 423383 | identical |
| demo.bin | `f7a93a04` / 70180 | `f7a93a04` / 70180 | identical |
| demo.debug.bin | `e3243cbb` / 93943 | `e3243cbb` / 93943 | identical |

config_a / config_b ride the strict off-canonical gates (§6). The aeon worktree
was seeded with the gitignored `games/sonic4/data/editor/` (196 files, rsync'd
from the main checkout) before any build — the known wrong-ROM trap.

## §6 — Strict suite

`AEON_DIR=<aeon-bp0> cargo test --workspace --release`, full output captured
(no `tail`/`head` in the pipeline — see §7's process note):

**3004 passed · 0 failed · 4 ignored**, across 304 test binaries, `CARGO_EXIT=0`
(3002 at the pre-panel draft, +2 for the preserves wall and the canonicalization
pin). Run three times in full: after the lens-panel fix pass, and again after
the chain-38 rebase — same 3004/0/4 both times, with the four ROM shapes
rebuilt at each point (§5 carries the chain-38 figures).
Failure scan (`FAILED` / `failures:` / `panicked at` / `error[` / `error:`) is
empty. The 4 ignored are the standing set, none of them new: `chained_resume_plain`
+ `chained_resume_debug` + `secondary_pin_classes_match_the_hand_typed_baseline`
(all three RETIRED by Wave-B B-0's packed placement) and
`sigil_diff_reports_byte_identity` (opt-in `--ignored`, reads the aeon tree).

`repin --check` / `refreeze --check`: no re-freeze taken and pins untouched (the
parcel emits no bytes); the golden gates in the suite above are the operative
proof.

## §7 — Why the aeon edit is provably inert (not merely observed)

The panel supplied the structural argument the first draft lacked. Adding `a1`
to `AllocDynamic`'s `clobbers` cannot move any analysis, because:

1. **`effective` is body-derived, not declaration-derived.** `closure.rs:181-186`
   computes a non-extern proc's effect as
   `local_writes ∪ ⋃ effective(callees) − verified_preserves`; declared clobbers
   are read only in the `is_extern` arm. `AllocDynamic` writes a1 at
   `core.emp:123`, so **a1 was already in `effective` before this parcel.**
2. **Every consumer of a declared clobber set unions `out` into it** —
   `closure.rs:330` (`allowed = clobbers ∪ out`), `corpus_contracts.rs:953`
   (`extern_node`), `type_slice.rs:117` (degrade set). Since a conditional-out
   register is already in `out`, `clobbers ∪= {a1}` is a set-theoretic identity
   at all three. This generalizes past this parcel.
3. **D1c already refused to excuse a1.** `destroys_value` reads
   `callee_uncond_out` = `out − cond` = ∅ for this proc.
4. **No backend consumes a clobber set.** `clobbers` reaches only diagnostic
   producers; there is no register allocator (registers are author-written).
   The one byte-affecting path — D1d dead-save elimination — is fed the
   closure's `effective`, never declared clobbers, and is a printed worklist
   rather than a transform.

**Correction to §6's evidential weight (panel finding):** the strict suite is
NOT evidence of D1c neutrality. `sigil-cli/tests/out_verify_corpus.rs:53-56`
prints `[call.live-clobbered]` firings with `eprintln!` and asserts nothing
about them, while the sibling gates assert `is_empty()` for input-firings, §9
firings and flag-firings. A D1c regression on this corpus is structurally
invisible to `cargo test`. The argument above plus a hand check of every caller
(`children.emp:158/269/351/461`, `object_test_state.emp:102/279`,
`test_churn.emp:83`, `load_object.emp:39` — each either stack-saves a1 across
the call or holds nothing in it) is what carries the claim. Ledgered as a gate
weakness.

## §8 — THE COST THIS PARCEL SHIPPED (escalated to Fable)

**The relax opened a silent-lie surface, and B′-0 does not close it.** Read this
before treating the parcel as purely additive.

`[proc.out-clobbers-overlap]` was the only thing forcing `out(rN if cc)` to have
one meaning. The corpus now carries two incompatible readings of the same
syntax, distinguished solely by whether the author also typed the register into
`clobbers`, and **no checker verifies which one is true**:

- `AllocDynamic clobbers(d0/a1) out(a1 if eq)` — "a1 is destroyed on every edge"
- `AllocEffect  clobbers(d0)    out(a1 if eq)` — "a1 survives the ¬eq edge"

Both are true today; both would still compile if either were false. Concretely:
hoist `AllocEffect`'s pop above its exhaustion test and a1 becomes trash on the
failure edge while the contract still claims `clobbers(d0)` — **zero
diagnostics fire**, and `children.emp:578`'s exhaustive-license reasoning is
silently invalidated.

Before this parcel that lie was *unexpressible* — the grammar rejected the
split, so the fact lived in prose. B′-0 made the declaration honest for one proc
and made dishonesty newly expressible for all of them. That is a real trade, not
a footnote.

Not fixed here because the fix is a new CFG-backed check whose polarity, tier
and home are a spec owner's call, and the brief forbids redesigning spec
surface. Two candidates are ledgered; the dual form (a cond-out register ABSENT
from `clobbers` must be provably unwritten on the ¬cc edges) is the one that
catches the AllocEffect regression. `out_verify.rs` already owns the CFG and
return-edge machinery, so the parcel is small — it is the *design* that needs a
ruling.

## §9 — Step-3 (retrospect) vs step-5 (engine) findings

**Step-3 (language/spec):**
1. **The conditional/unconditional out split has been re-learned SIX times**
   (`corpus_contracts`, `out_verify`, `calls`, `seam1`, `lower::check_out`, and
   `resolve/contract.rs` — which got it wrong). The first draft of this packet
   claimed "only one consumer exists today"; that was wrong. A
   `ProcDecl::unconditional_outs()` accessor would have prevented the sixth.
   **Ledgered.**
2. The exemption is exact in the cc dimension but not in the cond/uncond
   dimension — one conditional mention exempts the register wholesale,
   including a simultaneous unconditional mention. The first draft called the
   per-register scope "exact"; corrected. Unreachable in practice. **Ledgered.**
3. The Z80 arm is dead code today: `VALID_CCS` is the 68k set and is applied to
   both CPUs, so a genuine Z80 `out(a if z)` is rejected before the overlap
   check. The pair-expansion bug is fixed but untestable until ccs go
   CPU-parametric. **Ledgered with an instruction to test it then.**

**Step-5 (engine):** none. Contract metadata only; no instruction was read,
moved, or re-timed.

**Neither bucket — the transferable lesson.** The spec's hypothesis-plus-pin
structure ("likely closed, pin it") cost one test run to disprove and left no
false ledger entry. The lens panel then caught, in the parcel that ran on that
lesson: two factual errors in its own comments, three stale contract quotations
it had itself invalidated, a canonicalization bug on both CPU paths, an
unbacked "pinned by tests" claim, a wrong consumer count, and the soundness
cost in §8 — none of which a green suite could surface. Both mechanisms earned
their keep on the same parcel.
