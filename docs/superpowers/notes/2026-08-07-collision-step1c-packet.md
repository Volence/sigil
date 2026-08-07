# Collision lane, step 1c — the Load_Object contradiction, read and closed
# (+ the panel fixup: an unsound widening the lane had already landed)

Branch `collision` @ 2ae03153 (sigil only; aeon has no collision commits).
Base: sigil master fe7a2b73, aeon master d8c93d7.

**Read the fixup section before the rest.** The investigation below closed the
question it was given; the panel then found that an EARLIER commit on this same
lane was unsound, and that finding is the more important half of the packet.

## The headline

**The contradiction does not exist, and never did.** The ruled probe order was
(a) find the test covering the documented worked example, (b) minimal synthetic
repro to split verifier-bug from stale-doc, (c) git-log the corpus site against
the doc's vintage. Probe (a) answered it and (b) was never needed.

`out_verify.rs`'s header (~:21-26) names `Load_Object`←`AllocDynamic` as the
worked example of edge-sensitive conditional-out credit. The brief and gap-ledger
row 2255 both held that `Load_Object out(a1, zero: success)` "still fires
`[proc.out-unverified]`", making a documented capability contradicted by a live
firing.

Measured, over all seven shipped shapes:

```
Load_Object:     uncond={"a1"}   cond=None
AllocDynamic:    uncond={}       cond=[("a1","eq")]
```

`Load_Object::out(a1)` is **VERIFIED**, through exactly the advertised mechanism.
It is not in `OUT_UNVERIFIED_BASELINE` and the git history shows it never was.

## What produced the misreading

A near-miss across two lint families over ONE (proc, callee, register) triple.
`Load_Object @ AllocDynamic :: a1` *does* fire — as `[call.live-clobbered]`
(D1c), a different check. That firing is already adjudicated: `calls.rs`'s
`destroys_value` header (documented 2026-07-19, :289-304) names it as one of two
KNOWN false positives, because D1c's close is **edge-blind by ruling** (§4
Finding 5 — coupling D1c to the edge primitive risks a degrade-to-miss on a
`valid_edge` bail, judged worse than the documented FP), where `out_verify` is
edge-SENSITIVE. Same triple, two checks, opposite verdicts, both intended.

The two families sit in one baseline file, one array apart.

## The second finding: the residue has cascade structure

`Art_Decompress` is a different cause and is NOT an independent loose contract.
It is a pure dispatcher — `bne .s4lz` / `jbra ZX0_Decompress` /
`.s4lz: jbra S4LZ_Decompress` — with no local production of `a1`. The residue
reads the **verified** map (`corpus_out_residue_is_the_verified_complement` pins
this), so tail-out credit is a MUST-intersection:

```
ZX0_Decompress   verified {a0, a1}
S4LZ_Decompress  verified {a0}
Art_Decompress   verified {a0}      == the intersection
```

Its declared `out(a0, a1)` is HALF verified — which is why only `a1` appears in
the baseline, though the ledger row's wording ("`Art_Decompress out(a0, a1)`")
reads as if both fired.

**Falsifiable prediction, recorded before the work that tests it:** whatever
makes `S4LZ_Decompress` produce `a1` full-width should clear TWO baseline rows in
one run, with no edit to `Art_Decompress`. If it clears only one, the tail-out
credit model is wrong somewhere and that is the finding.

**Consequence for the out-type adoption work: 29 rows are not 29 sites.** The
adoption census must be computed over the fixpoint, not read off the baseline
array, or it will pay for a fix twice or paper an upstream row.

## What landed

`corpus_conditional_callee_out_is_credited_edge_sensitively` — a corpus witness
over all seven shipped shapes. Before it, the capability's only evidence was an
ABSENCE (Load_Object not in the residue), and an absence is not a claim: a
regression would have surfaced as an unexplained new baseline violation rather
than as the named capability breaking. It was in fact misread as its own opposite
for weeks, which is the argument for the test.

**The witness is two-part, and the second part is load-bearing — probe-measured,
not assumed.** If `AllocDynamic` is relabeled to an unconditional `out(a1)`,
`Load_Object`'s credit STILL LANDS via the trivial path; a one-part witness would
have gone on passing while testing nothing about edge-sensitivity. The guard
asserts the source is still genuinely conditional before the credit assertion
means anything.

This is the lane-A panel's negative-space bar applied forward: "does this assert
the CONTRACT or the current BEHAVIOUR?" is what produced the two-part shape.

## THE FIXUP — a 3-lens panel found an unsound widening this lane had landed

All three lenses, independently and by three different routes, returned the same
MUST-FIX. It was then confirmed against the code before anything was changed.

**The defect.** `out_verify`'s `Edge::FallOff` arm ran `check_return` only when
`charge_fall_off_end`. So a proc declaring `falls_into Successor` had its
fall-off charged to NOBODY — the successor's identity was never threaded in and
nothing credited from it. `ok` is seeded all-true and only `check_return` clears
it, so **a proc whose only exit is a fall-off finished the walk with every
declared out `Produced` on zero evidence.** That is the vacuity hole
`preserves.rs:656-660` documents having closed ("a tail-only proc used to have
ZERO obligation sites"), re-opened one module over.

**Why it matters beyond the module:** a verified out is consumed as a must-def
DEFINITION by D1b — an ERROR-tier gate with no baseline. A claim wrongly blessed
here silences a live check at every call site. This is the exact polarity the
module header names as the dangerous direction.

**The corpus instance was live, in the lane's own diff.**
`S4LZ_DecompressDict out(a1) falls_into S4LZ_Decompress` (`s4lz.emp:74-83`) only
READS `a1` (`suba.l a1, a4`), has no `rts` and no tail. The lane had removed its
baseline row — certifying it VERIFIED — while `("S4LZ_Decompress","a1")`, the
claim it rests on, stayed in the same array marked UNVERIFIED, twelve lines
apart.

**The precedent was misread.** `check_stack_balance`'s bool IS sound for stack
depth, because the successor's own unconditional balance check discharges the
obligation. No such discharge exists for a value claim — nothing requires a
successor to declare the register at all. The right neighbour was `preserves`,
which does not exempt but DEFERS (`ExitKind::FallOff =>
checkpoint_after_tail(st, Some(succ))`), and `out_verify`'s own `Edge::TailOut`
arm was already the correct template.

**Re-implemented as a transfer.** `verify_out` takes `falls_into: Option<&str>`
— the successor NAME, because a bit cannot credit anything — and the `FallOff`
arm credits the successor's VERIFIED uncond out before charging `check_return`.
Monotone, so the fixpoint argument is untouched. `compute_verified_outs` takes a
`BTreeMap<name, successor>`, which also collapses the two-derivations-of-one-fact
split the panel flagged.

**Measured: residue 29 → 30**, `S4LZ_DecompressDict :: a1` restored as a THIRD
dependant of `S4LZ_Decompress :: a1`. The lane's "30 → 29" headline is withdrawn:
the 29 was bought by not checking.

### Three further panel findings, all fixed

- **A negative test pinned the vacuity as the contract** — the named defect
  family, in the parcel that had just been warned about it. The `falls_into`
  test asserted `is_produced` while passing an EMPTY callee map and no successor;
  its two "polarities" varied the FLAG, not the SUCCESSOR, so it would pass under
  an implementation that ignores the successor entirely — which is the one it
  shipped with. Replaced with four cases that vary the successor's contract,
  including the one a blanket exemption cannot express: **successor does not
  produce → FIRES**.
- **The residue-complement witness had gone vacuous, and it is PROVEN by probe,
  not argued.** It named `Collision_GetType::out(d0)` as grounding in
  `Tile_Cache_GetCollision`; aeon `49b7f3d` (2026-07-31) fused that proc INTO
  `Collision_GetType`, now a LEAF (zero call mnemonics), so callee credit cannot
  apply and it fires identically under both maps. Switching the residue surface
  to declared credit: residue 30 → 28, `Art_Decompress::a1` and
  `S4LZ_DecompressDict::a1` VANISH while `Collision_GetType::d0` REMAINS. The old
  witness detects nothing; the replacement demonstrably does.
- **The census was asserted, not run.** "The class had ONE hole, not four" is
  wrong: `flag_check` IS a policy consumer (`abandons_flag` returns true on
  `Return | FallOff` with no `falls_into`), and `z80_preserves`' tail arm consults
  `falls_into` but not `noreturn`. Both LATENT — verified site by site — but the
  row's wording would have stopped the next author from checking. Ledgered as
  open members rather than silently fixed.

Also folded the third open-coded copy of `exit_diverges` inside `preserves`
itself, corrected its doc (which told the next author that a `falls_into`
fall-off "is not an exit" — the reading that causes this bug), and restored
`clippy -D warnings`, which the branch had broken by pushing `check_out` to nine
arguments with no allow.

## Gates (all own-run, none waived)

- **Strict** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon collision worktree> cargo test
  --workspace --release`: **3517 passed / 0 failed / 4 ignored = 3521**.
  Closing arithmetic: branch's own `#[test]` total = **3521**. Closes exactly.
  (+3 over the branch's inherited 3518: the credit witness, the plumbing test,
  the edge-locality test. The first fixup replaced one test with another, 1 for
  1, so it moved no count.)
- **clippy** `--workspace --release --all-targets -- -D warnings`: clean. It was
  NOT clean on the inherited branch — `check_out` had been pushed to nine
  arguments with no allow, and master's is exactly seven.
- **Byte bar, seven targets, fresh-build**: verified INSIDE the strict run rather
  than assumed — `native_full_sonic4_plain`, `native_full_sonic4_debug`,
  `config_a_anchor_matches_golden`, `config_b_anchor_matches_golden`,
  `lean_anchor_matches_golden`, `demo_plain_anchor_matches_golden`,
  `demo_debug_anchor_matches_golden`, all ok. Each builds the ROM from the aeon
  tree and compares against the chain goldens.
- **refreeze --check**: OK (tip `slide-fixture`, chain len 53) — unmoved.
- **Negative probes, both polarities, both reverted by string-replace:**
  - clearing `conditional_out_edge_credits`' result → credit half fails with its
    own message (`verified=Some({})`).
  - relabeling the aeon-side `AllocDynamic` contract to unconditional → the
    non-vacuity guard fails.

## Per-pass

**Step 3 (retrospect):** for the investigation, the defect was in the RECORD, not
the code — a ledger row asserting a contradiction that measurement refutes,
inherited verbatim into a brief and into a ruled work order. Both lint families
name the same triple in the same file; nothing in either baseline
cross-references the other. A pointer between them is the cheap structural fix.

For the fixup, the retrospective finding is sharper and worth stating plainly:
**a parcel adopted a neighbouring checker's POLICY by copying its SIGNATURE.**
`check_stack_balance(items, charge_fall_off_end: bool)` was read as "the shape to
copy" when the thing that made it sound — the successor's own check discharging
the obligation — does not transfer to a value claim. The bool was the tell: a
flag can only say *whether* to charge, never *whom* to charge. When the correct
model is "the obligation moves", a boolean parameter cannot express it, and
reaching for one should read as a design smell.

**Step 5 (engine/analysis):** the investigation warranted no analysis change —
that capability works. The fixup warranted a real one, and it is a soundness
restoration rather than an improvement: the analysis now refuses where it
previously blessed.

**Neither bucket:** the residue's cascade structure was not previously written
down anywhere, and it materially resizes the out-type adoption work — now THREE
dependants of one root rather than two.

## The re-panel on the fixup — verdict SOUND, two gaps closed

A fourth read-only lens verified the fix against a clean tree at a named SHA.
Verdict: **correct and sound, no MUST-FIX.** It checked the fixpoint algebra
(monotone; every `true` bit traces to a local full-width production, so
self-loops and mutual `falls_into` stabilise at ⊥ rather than at a false fixed
point), the edge model (`Edge::FallOff` also covers a branch to a trailing local
label — and the credit is correct there too, since such a label sits at the same
address as the fall-through into the successor), and confirmed the credit is
edge-LOCAL (a `credit` copy, so it cannot leak to an `Edge::Return` on another
path). It also confirmed the `check_cond_out_survives` `None` argument is a DEAD
argument on the `Sites` path, not a policy inconsistency.

Two SHOULD-FIX, both acted on, and the first is the more interesting:

**The plumbing was untestable from the corpus, and half of it still is.** The
lens argued a mutant replacing the successor lookup with `None` at either end
would pass every gate. **Both mutants were run rather than argued.** Site 1 (the
fixpoint lookup) is now CAUGHT by a test that builds its own discriminating pair.
Site 2 (the per-proc firing tier) — **mutant GREEN, measured across 28 corpus
tests.** The reason is structural: the corpus's only 68k `falls_into`+`out` site
is `S4LZ_DecompressDict`, whose successor's `a1` is itself unverified, so the row
fires identically whether the credit is wired or dropped. It is not unguarded by
accident — `corpus_out_residue_is_the_verified_complement` catches the divergence
the moment any corpus site gets credited — but that guard is dormant today, and
that is a coverage fact worth writing down rather than a green light.

A second test pins edge-locality (a body with both an `rts` and a fall-off still
fires), closing the lens's NOTE that the original could pass under a globally-
crediting implementation.

**And a comment that taught the bug.** `lower/proc.rs`'s exemption justified
itself by citing `out_verify`'s `charge_fall_off_end` as "the same line" — written
by the commit that introduced the unsound drop. That flag no longer exists and
the closure now draws a different line, so the comment endorsed exactly the
reasoning that produced the defect. A stale comment is not cosmetic when it
encodes a rejected design as precedent.

## Open, deliberately not fixed here

- **`flag_check::abandons_flag`** returns `true` on `Edge::Return | Edge::FallOff`
  with no `falls_into` in hand — a false positive on a shipping lint for a
  `falls_into` proc whose successor consumes the carry. LATENT (no such site in
  aeon today, verified by enumerating the flag callees and their callers).
- **`z80_preserves`'** `TailOut|BranchOut` arm consults `falls_into` but not
  `noreturn` — the mirror-image half-knowledge. LATENT (the affected procs
  declare no `preserves`). Polarity is safe: it over-obligates.
- **`lower/proc.rs`'s per-file `[proc.out-unwritten]` exemption is proc-wide**,
  broader than the corpus gate's per-exit one, and it is the ONLY out gate that
  ever sees Z80 (`out_verify` is 68k-only). The honest version exempts only when
  the successor declares the same register; the successor is already in hand
  there via `check_fallthrough_adjacent`.

Each is ledgered with its evidence. They are latent, not live, and bundling them
into a soundness fixup would have made the fixup unreviewable.

## Process notes carried forward

- **The shell cwd resets to the MAIN checkout between tool calls in this
  harness.** It bit twice: early greps and a `cargo build`/unit-test run executed
  against master rather than the lane worktree, and a clippy run did the same.
  Neither produced a wrong conclusion — the edits themselves always used absolute
  paths — but a green gate from the wrong tree is worthless, and one of them was
  briefly believed. Every command now carries an explicit `cd` and prints `pwd`.
- **A stale test binary in the main checkout's `target/` reported a residue of 29
  where the truth was 30**, with cargo printing "Finished in 0.02s" — it never
  rebuilt. Treat a sub-second "Finished" on a supposedly-fresh measurement as a
  staleness signal, not as speed.
- Re-deriving the relayed premise is what produced this entire result. The brief's
  item-1 premise was wrong in both of its two clauses (Load_Object does not fire
  that lint; Art_Decompress fires on `a1` only, not `a0, a1`). The panel then
  applied the same bar to THIS lane's own prior claims and refuted two of them.
- Negative probes were reverted by string-replace and by restoring the original
  text; `git checkout --` was never used in the worktree. Probe residue was
  grepped for by marker (`PROBE-A`, `PROBE-DECLARED`) and confirmed absent before
  each commit.
