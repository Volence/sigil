# D1b WARN→ERROR flip (Phase-1 item #4) — build spec brief

**Overseer-authored, 2026-07-21.** The LAST Phase-1 item — after this the net is live and
pass-3 register surgery opens. Off masters: **sigil `ad357ed`** (or later; item-#3 merge
`3f333d2` is in), **aeon `ae1de4d`**. Byte-neutral (analysis + gate wiring only). Canonical
ROMs: plain **`3aa43cb6`/420749**, debug **`ce0e83a6`/428768**.

**Goal:** `[call.input-undefined]` (D1b) becomes a HARD gate: zero firings over the live
corpus, enforced as an ERROR under the standard strict invocation — and the credits D1b
rests on become **verified, not declared**. The FindStagedBlock mislabel (#1's existence
proof) is why "declared" is not good enough for an ERROR gate: today `must_defined_in`
credits `callee_out`/`cond_callees` built from DECLARED contracts
(`corpus_contracts.rs`), and #1's close-out named this the Finding-2 flip-blocker.

---

## 0. MANDATORY Stage 0 — terrain, before design

- **(0a) The residue-consumer map — this drives the Buckets-2/3 decision.** Enumerate the
  current `[proc.out-unverified]` residue (15 at the #3 close). For EACH residue proc:
  which CALLERS' must-def runs actually CONSUME its `out()` as a credit (i.e., which D1b
  firings would APPEAR under verified-only crediting)? Produce the exact predicted new-
  firing set. Plausible outcome: many residue outs (narrow-width, in-out accumulators) are
  never load-bearing for must-def at any live call site — in which case the flip needs NO
  G5 pull-forward.
- **(0b) Strict-gate coverage audit.** `dead_save_corpus.rs:29` has the same
  skip-when-AEON_DIR-unset defect the overseer fixed in the tripwire (`c5505f8`): under
  the STANDARD strict invocation (`SIGIL_STRICT_GATE=1 cargo test --workspace`, no
  AEON_DIR) it silently skips. Audit EVERY AEON_DIR-gated corpus test; apply the
  `c5505f8` pattern (default sibling path; missing tree hard-fails under strict). The
  flip's own gate is worthless if it doesn't run in the gate invocation people use.
- **(0c) Re-confirm conditional-external-tail** is still grep-absent in the corpus and the
  guard from #1 still stands.
- Checkpoint back to the overseer with 0a's predicted-firing table before designing.

## 1. The mechanism (design freedom within these fences; refute-pre-code required)

- **Verified-out fixpoint (retires Finding 2).** A proc's `out(rN)` is VERIFIED iff
  `out_verify` passes when callee-out credit is drawn ONLY from already-VERIFIED callee
  outs. Compute as a monotone fixpoint (start ⊥/leaves, iterate until stable — it
  terminates; the verified set only grows). A mutual/circular out-dependency that never
  grounds stays UNVERIFIED — the correct conservative answer, not an error.
- **must-def switches credit source to verified outs** (both the unconditional map and the
  conditional edge-credit's `cond_callees`). Declared-but-unverified ⇒ NO credit ⇒ new
  D1b firings appear exactly where Stage 0a predicted. Verified-only crediting can only
  ADD firings — the safe polarity; it can never silently bless.
- **Buckets 2/3 — the ruling is data-gated, default framed here:**
  - If 0a shows their outs are NOT consumed by any live must-def credit: **flip without
    any G5 pull-forward**; Buckets 2/3 stay WARN-tier out-verify residue with their
    per-trace adjudications documented. (Preferred if true — smallest honest flip.)
  - If some ARE consumed: pull forward the MINIMAL slice per bucket — **width-typed outs**
    (`out(d0.w)`: the verifier accepts a `.w` production for a declared `.w` out — no type-
    system creep beyond the out-clause grammar) for the Bucket-2 consumers only, and an
    **explicit `inout(rN)` marker** for Bucket-3 consumers only. SOUNDNESS FENCE for
    `inout`: it is NOT #1's reverted param∩out∩read seed — an explicit `inout(rN)` moves
    the obligation to the CALLER (rN is a param: D1b requires it defined AT THE CALL), so
    seeding it as produced in the callee's out-verify is sound precisely because the
    caller-side check exists. Spell this argument out in the design note; the reverted
    seed's unsoundness (blessing a non-producing bail path with no caller obligation) is
    the mutation trap.
- **The flip itself is the LAST commit, alone:** a live-corpus strict gate asserting
  `input_firings` is EMPTY (ERROR tier), following the corpus-sweep pattern WITH the 0b
  hard-fail behavior. Nothing else rides that commit.

## 2. Tests (both directions + mutation traps)

1. **Fixpoint cycle:** procs A↔B each declaring `out(rN)` sourced only from the other →
   both UNVERIFIED → caller crediting nothing → D1b fires at the caller. MUTATION:
   drawing credit from DECLARED outs goes green — proves the fixpoint is load-bearing.
2. **Verified-only credit:** a caller consuming a callee whose `out()` FAILS verification
   must fire D1b even though the contract declares the out. (The FindStagedBlock-mislabel
   shape, as a permanent regression.)
3. **Chain grounding:** A←B←C where C produces locally, B's out is sourced from C, A
   consumes B — the fixpoint verifies C then B, and A does NOT fire. (Guards against an
   over-conservative one-pass implementation that never credits chains.)
4. **`inout` caller obligation** (if the marker ships): caller with rN UNDEFINED calling
   `inout(rN)` proc FIRES D1b at the call; the callee's out-verify passes via the seed.
   MUTATION: the #1-reverted inference (param∩out∩read, no caller check) must break it.
5. **Width-typed out** (if that slice ships): `out(d0.w)` verified by a `.w` write; a
   declared `out(d0)` (full) with only `.w` writes still FIRES out-unverified (Finding-1
   width rule unchanged).
6. **The flip gate itself** red-tests: inject a synthetic undefined-input corpus case in a
   hermetic fixture and assert the gate REJECTS it at ERROR tier.

## 3. Gates

- Byte-neutral: canonical EXACT both shapes (plain `3aa43cb6`/420749, debug
  `ce0e83a6`/428768); aeon expected UNTOUCHED unless a Bucket-2/3 retrofit needs an
  honest contract respelling (each such edit its own byte-neutral commit, like `99bb941`).
- Full strict from tips under the STANDARD invocation (post-0b it must be trustworthy
  without AEON_DIR), failures-first, explicit counts (baseline 2445/0/1); clippy;
  bisectable commits (0b audit fixes / fixpoint / credit-switch / [slice or markers] /
  retrofits / FLIP LAST); packet with per-pass breakdown + the full before/after firing
  snapshot on every surface.

## 4. STOP-don't-bank forks

- **Any new D1b firing under verified-only crediting NOT in 0a's predicted set = a
  potential REAL bug** — the arc's would-be headline. Stop, adjudicate with the overseer
  before touching the code it points at.
- The G5 width-typing slice starts leaking beyond the out-clause grammar (into eval/type
  machinery broadly) → stop; that's G5 proper, re-scope with the overseer.
- The fixpoint needs widening / doesn't stabilize → stop (that's a design flaw, not a
  tuning knob).
- A Bucket-2/3 proc's honest contract turns out to be inexpressible even with the slice →
  the verifier grows or the item re-scopes; never a dishonest contract, never a suppressed
  firing.

## References
- #1 brief + residue: `2026-07-18-out-verification-spec-brief.md`,
  `2026-07-19-out-verification-residue.md` (Bucket definitions + the Finding-2 blocker).
- #2 brief (the cc-lattice + edge credit #4 consumes): `2026-07-19-edge-sensitive-conditional-out-spec-brief.md`.
- #3 packet (final snapshot, 15-row dead-save worklist): `2026-07-21-s2d6-packet.md`.
- Machinery: `calls.rs` (`must_defined_in`, credit maps), `out_verify.rs` (`verify_out`,
  `check_out`), `corpus_contracts.rs` (map construction + `input_firings`),
  `flag_check.rs` (`conditional_out_edge_credits`).
- Roadmap: `pre-t18-roadmap.md` item #4 (updated alongside this brief).
