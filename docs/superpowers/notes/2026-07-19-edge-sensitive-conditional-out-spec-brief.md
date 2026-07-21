# Edge-sensitive conditional-out crediting (Phase-1 item #2) — build spec brief

**Overseer-authored, 2026-07-19.** Phase-1 item #2. Builds on #1 (out()-verification,
merged at sigil `cd10321`). Off masters: **sigil `cd10321`, aeon `e55d7f1`**. Byte-neutral
(analysis + contract-text only; zero codegen change).

Depends on #1: #2 CREDITS a callee's conditional out as if honest — which is sound only
because #1 now VERIFIES conditional-out honesty (`[proc.out-unverified]`). That ordering is
the whole reason #1 came first.

> **CORRECTED 2026-07-19 after a pre-code adversarial soundness review** (6 findings; the
> load-bearing one re-verified in code). The v1 "reuse `invalid_edge` / add a `valid_edge`
> complement" plan was UNSOUND: `invalid_edge`'s bail uses `writes_carry`
> (`flag_check.rs:99`), a **carry-polarity allowlist that DELIBERATELY treats the Z-only
> writers `btst`/`bset`/`bclr`/`bchg` as CC-transparent** (its own doc comment says so). §6
> can afford that (a missed clobber over-fires = safe); **#2 cannot — its only cc is `eq`
> (Z)**, so a Z-clobber between a conditional-out call and its `beq` guard would not bail, and
> #2 would credit on a stale-Z edge = must-def FALSE NEGATIVE on an ERROR gate. The corrected
> mechanism (§2) uses a **sound-complete** CC-clobber check (`out_verify.rs`'s `cc_transparent`
> model — bail unless provably transparent), the SHARED thing is the **edge identification**
> not a "defined" fact (§3), crediting requires the caller's guard to test the callee's
> **exact** cc (§2), and #2 does **not** couple D1c to the edge primitive (§4). Findings folded
> below.

---

## 1. The problem

A conditional `out(rN if cc)` is SUBTRACTED from `callee_uncond_out` (`corpus_contracts.rs:179`),
so nothing credits it. Consequences:

- **D1b false alarm (the roadmap-#2 target):** `FillColumn`/`FillRow` define `a1` via
  `jbsr FindStagedBlock` (`out(a1 if eq)`) on the `beq .have_block` edge, and via
  `DecompressBlock` (unconditional `out(a1)`) on the other. must-def credits only the
  unconditional path, so on the eq path `a1` looks undefined → `[call.input-undefined]` fires
  at the downstream use (documented false positive; `a1` is defined on all real paths).
- **Bucket-1 blocker:** #1 found `AllocDynamic`/`AllocEffect` `out(a1)` are really
  `out(a1 if eq)`. Relabeling them (the honest form) removes `a1` from `callee_uncond_out`,
  so `Load_Object`'s success-path `a1` credit (from AllocDynamic) vanishes → `Load_Object`
  fires. Re-verifying needs crediting AllocDynamic's conditional out on `Load_Object`'s
  eq-success edge. That is THIS item.

## 2. The property + mechanism

At a call to a callee declaring `out(rN if cc)`, credit `rN` as defined/produced **only on
the caller's provably-cc-SUCCESS successor edge** — never on the `!cc` edge, never on all
edges.

- **Edge identification — a NEW #2 walk, NOT `invalid_edge` as-is (Finding 3).** Walk the
  caller's fall-through chain from the call to the guard branch, exactly like `invalid_edge`,
  BUT the "did anything clobber the cc between the call and the guard?" bail must be
  **sound-complete for the flag(s) the callee's `cc` names** — bail unless every intervening
  instruction is **provably CC-transparent**. Use `out_verify.rs`'s `cc_transparent` model
  (a known-transparent allowlist: `movea`/`lea`/`pea`/`adda`/`suba`/`movem`/`exg`/`nop`/
  branches — everything else, INCLUDING `btst`/`bset`/`bclr`/`bchg` and any unmodeled
  mnemonic, bails). Do **NOT** reuse `writes_carry` (`flag_check.rs:99`): it is a
  carry-polarity KNOWN-CLOBBER allowlist that deliberately lets Z-only writers through as
  transparent — sound for §6's over-fire polarity, a FALSE NEGATIVE for #2's credit polarity
  (§2 corrected banner). `invalid_edge` may be parameterized to take the transparency
  predicate, or #2 gets its own walk; either way #2's bail is the `cc_transparent` one.
- **Exact-cc match (Finding 4).** Credit only when the guard branch tests the callee's EXACT
  declared cc, or its EXACT negation (`branch_cond(guard) == cc` → cc holds on the taken edge;
  `== negate_cc(cc)` → cc holds on the fall-through). Any other condition — even a correlated
  one (e.g. `bpl` after a `moveq #0` that also cleared N) → **bail, do not credit.** The
  soundness fence IS the lexical cc-identity; never "improve precision" by treating correlated
  conditions as compatible.
- **On bail (`None`): DO NOT CREDIT.** The load-bearing rule. `valid_edge` is a PARTIAL
  function — it bails whenever the guard is displaced by a possible cc-clobber, is an unrelated
  condition, or the guard is missing. Bail → the register stays "not defined by this call" → a
  real false positive may remain (acceptable), never a silent miss. **Never** credit-on-bail to
  quiet a residual false positive.

## 3. Soundness polarity — the MIRROR of #1 (read this twice)

#1 was callee-side: "the obligation holds at a return UNLESS cc is provably false"
(fire-leaning). #2 is caller-side crediting: "credit rN as produced ONLY where cc is provably
TRUE at the success edge" (credit-leaning-conservative). **Opposite conservative defaults on
the same declared cc.** must-def joins by INTERSECTION, so an over-credit (crediting on an
edge where cc might not hold) is a false negative — forbidden. Credit is earned only by a
proven-cc-success edge. "Symmetry with #1's rule" is NOT an argument — the polarity is
inverted; derive each side from its own join direction.

**What is shared, and what is NOT (Finding 1).** The credit is applied as a per-edge TRANSFER
into each consumer's OWN forward must-analysis, which then re-joins at every control-flow
merge by intersection. The shared, reusable thing is the **cc-success EDGE IDENTIFICATION**
(§2's walk) — NOT a global "rN is defined after this call" fact. The distinction is
load-bearing: at a merge reachable from the cc-success edge AND from another predecessor that
never produced rN, soundness comes from the consumer's intersection dropping rN — it is NOT a
property of the credit in isolation. So do NOT expose "rN defined post-call" as a standalone
fact any consumer can read at the successor node; expose only "which successor edge is
cc-success," and let each must-analysis apply it and re-join. A shared "credit fact" that
skips the re-join is exactly the future silent miss the review flagged.

## 4. Consumers (share ONE edge-credit primitive — no drift)

Route all of these through a single caller-side edge-credit helper so they can't disagree on
which edge is cc-success:

- **must-def (D1b), `calls.rs:135`:** credit a callee's conditional `out(rN if cc)` on the
  valid successor edge only. Clears `FillColumn`/`FillRow` a1.
- **The out-verifier's own call-credit (`out_verify.rs` `transfer`, the `is_call` arm):** when
  proc P's `out(rN)` is sourced from a callee's CONDITIONAL out on P's cc-success path, credit
  it there. This is the `Load_Object`←`AllocDynamic`-relabeled cascade. NOTE the current
  call-credit is edge-blind (credits `callee_uncond_out` on entry to all successors) — the
  conditional credit must be edge-scoped.
- **§6 / D1c — do NOT couple D1c to the edge primitive (Finding 5).** #2 crediting rN as
  *defined* (D1b) does not touch D1c's *clobber* fact — different gates, different maps
  (`destroys_value` reads `callee_uncond_out` only), so no current contradiction. But the
  temptation to "retire #1's documented FillRow D1c false positive with edge precision" by
  making D1c consume this edge fact is FORBIDDEN: on a `valid_edge` bail (§2), that refinement
  degrades silently back to the miss #1's simple close deliberately closed (#1 §4's own
  warning). **D1c stays #1's simple sound close, untouched by #2.** Edge-precise D1c, if ever
  wanted, is separate work that must itself never degrade-to-miss on bail. Confirm #2 changes
  no §6 firing; if it does, that's a fork — report it.

## 5. Bucket-1 retrofit (aeon, byte-neutral, its own commit)

Relabel `AllocDynamic`/`AllocEffect` `out(a1)` → `out(a1 if eq)` (the honest form #1 proved).
Verify: (a) #1's out-verifier now VERIFIES them (a1 produced on the eq/success returns —
`moveq #0` folds Z=true there); (b) the `Load_Object` cascade CLEARS via #2's edge credit
(`jbsr AllocDynamic; bne .alloc_fail` — the fall-through IS the eq-success edge, cleanly
identified). If the cascade does NOT clear cleanly, STOP and report — do not force it.

## 6. Tests (both directions + trap)

1. **clears:** the FillRow/FillColumn shape — conditional `out(rN if eq)` produced on the
   `beq` edge → downstream use no longer fires D1b.
2. **still-fires (bail = conservative):** the same conditional out where the success edge is
   NOT identifiable (an unrelated branch / CC-redefine before the guard) → NOT credited → the
   D1b firing REMAINS. (A false positive we keep, never silence.)
3. **Z-clobber bail (Finding 3, the critical trap):** a `btst`/`bset`/`bclr`/`bchg` (or any
   non-transparent instr) between the conditional-out call and its `beq` guard → the walk
   BAILS → rN NOT credited → the downstream use STILL fires D1b. Then the **mutation**: swap
   the bail predicate to `writes_carry` (the §6 one) — it lets the `btst` through as
   transparent, so rN gets credited on the stale-Z edge and the D1b firing vanishes → this
   test must go from RED-firing to GREEN-not-firing under the wrong impl. If it doesn't, the
   sound-complete bail isn't load-bearing.
4. **trap/mutation (crediting polarity):** a caller that reads `rN` on the `!cc` path where the
   callee did NOT produce it must FIRE; a mutation that credits on the `!cc` edge / all edges /
   on bail must break it.
5. **exact-cc (Finding 4):** a caller guarding on a correlated-but-different condition (e.g.
   `bpl` where the callee declares `if eq`) → BAIL → not credited → D1b still fires. A mutation
   that treats correlated conditions as compatible must break it.
6. **cascade:** `Load_Object` verifies after the AllocDynamic relabel; the un-relabeled
   (unconditional) form + edge-blind credit is the regression control.

## 7. Gates

- Bisectable commits: `valid_edge` helper / must-def edge-credit / out-verifier edge-credit /
  AllocDynamic-AllocEffect relabel (aeon) — each its own commit.
- Byte-neutral: prove at ROM level, canonical CRCs EXACT both shapes (plain `8984e510`/`453533`,
  debug `c80465dc`/`461554`).
- Paired strict from tips (failures-first, explicit counts), both critical pins green, clippy.
- Residue after #2: Bucket 1 should CLEAR (relabel + cascade). Report the new firing set.

## 8. STOP-don't-bank forks

- **§6/D1c interaction** — if #2's crediting changes a §6 or D1c firing, that's a soundness
  fork; report it, don't self-resolve.
- **A conditional out whose success edge legitimately can't be identified** — if a real corpus
  site can't be credited (bail) and leaves a firing, that's a documented limitation (like #1's),
  not something to force. Report it.
- **Does #2 retire any of #1's flip-blockers?** After #2, re-check: Bucket 1 cleared? The
  mutual-callee-out (Finding 2) and conditional-external-tail (Finding 3) limitations still
  open? Report the updated flip-blocker list.

## References
- #1: `docs/superpowers/notes/2026-07-18-out-verification-spec-brief.md` + the residue note.
- Machinery: `flag_check.rs:252` (`invalid_edge`), `calls.rs:135` (`must_defined_in`),
  `out_verify.rs` (`transfer` call-credit), `corpus_contracts.rs:179` (`callee_uncond_out`).
- Roadmap: `docs/superpowers/notes/pre-t18-roadmap.md` Phase-1 item #2.

---

## Addendum (Fable post-build review, 2026-07-21): Finding 7 — label-join bail

The as-built `valid_edge` walked `next_instr`, which chains instruction items and steps
over LABELS invisibly — so a jump-target label between the call and the guard (a JOIN:
a bypass path can enter there without executing the call) did not bail, and the guard's
success edge was credited for the bypass path too = a must-def FALSE NEGATIVE (the
§3-forbidden polarity). Proven empirically pre-fix (probe returned `[]` where `a1` must
fire); dormant in the live corpus (both real sites are call-adjacent-guard) and D1b is
WARN, but flip-blocker-class. FIX: bail on ANY `CodeItem::Label` in the raw item range
between consecutive walk steps — referrer-blind by design (a referrer added later must
not silently open the hole). One fix in the shared primitive covers both consumers.
Regression tests: `conditional_out_label_join_between_call_and_guard_still_fires` (the
proven hole) + `conditional_out_unreferenced_label_before_guard_still_fires` (the
deliberate conservatism). NOTE the asymmetry: §6's `invalid_edge` KEEPS its label-skip —
over-fire polarity makes a join harmless there (same shape as the `writes_carry` split).
