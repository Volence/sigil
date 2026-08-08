# `S4LZ_Decompress :: out(a1)` — the root verdict, and a second CFG blind spot

The last unexamined cause in the `[proc.out-unverified]` residue: the three-row
`S4LZ_Decompress :: a1` chain. Investigation only — nothing was fixed, and no
`crates/` file carries a deliverable edit.

Measured on sigil `3e0824b1` (branch `s4lz`) against aeon `d8c93d7` (branch
`s4lz`), in the lane worktrees. Every perturbation was reverted by
string-replace and the reversion proven with `git status --porcelain` (0 lines,
both repos) plus a `S4LZPROBE` / `S4LZMUTANT` residue grep (0 hits, both repos);
the unperturbed gates were re-run green afterwards (`contract_closure_corpus`
26/0, `warn_tier_corpus` 5/0). Provenance of the numbers: 54 crates compiled
in-worktree, test binary `target/release/deps/contract_closure_corpus-eee0a8773132a509`
built this session under this worktree — not a warm binary from elsewhere.

**Baseline re-derived this session**, by counting `OUT_UNVERIFIED_BASELINE`'s
rows in `crates/sigil-harness/src/contract_baseline.rs`: **30**. Every set diff
below is against that array, read as a SET DIFF from the gate's own
NEW/GONE report, never as a count.

---

## HEADLINE

**Verdict (c) — CONTRACT-ONLY. `out(a1)` is false as written, on a path the
verifier is right about: a stream that decompresses to ZERO bytes never writes
`a1` at all, and the "output" is then the caller's own input pointer.** The
honest claim is a threaded in-out — *advanced past the bytes written, or
unchanged when none were* — the same shape probe2 found in
`DrawRings`/`InsertSpriteMasks`. No caller anywhere in the tree reads `a1` after
any of the three procs, so verdict (a) has no victim.

**Two further findings, neither inherited:**

1. **A second CFG blind spot, sibling to probe2's local-`bsr`: a COMPUTED
   INTRA-PROC DISPATCH (`jmp .table(pc,Xn)`) is modeled as a transfer OUT of the
   proc.** `S4LZ_Decompress` therefore has **three** failing obligation sites,
   not one, and **two of them are not return paths at all**. Each was attributed
   individually by probe. Same fail-safe polarity as the `bsr` gap (over-fires,
   cannot bless), and the `targets(...)` clause that would fix it already exists
   — but is read by ONE consumer (the cycle-budget walk), not by the CFG.
2. **The dormant site-2 `falls_into` plumbing guard is REAL. Armed and run: it
   goes RED**, caught structurally by exactly the test the ledger row named.

**One live gate will break the day the root closes**, and must be re-pointed in
the same commit: `corpus_out_residue_is_the_verified_complement` hard-codes
`Art_Decompress::out(a1)` as its witness (§9).

---

## 1. THE STRUCTURAL FINDING — a computed dispatch leaves the proc

`flag_check::branch_target` matches **only** `CodeOperand::Sym`:

```rust
pub(crate) fn branch_target(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        _ => None,
    })
}
```

`jmp .lit_end(pc,d1.w)` lowers its operand to `CodeOperand::PcRelIdx { target, addend, xn, xlong }`
(`value.rs`, the `(d8,PC,Xn)` brief-extension form) — not a bare `Sym`. So
`branch_target` returns `None`, `Cfg::branch_edge` falls to its `None` arm, and
because `jmp ∈ UNCOND_MNEMONICS` the flavor is `OutFlavor::TailOut`. The
consequences are both halves of the `bsr` shape, inverted in direction:

- **An obligation with no return.** `verify_out`'s `Edge::TailOut` arm calls
  `check_return` there. `direct_target` also rejects the operand, so the site
  gets no credit and any unproduced `out` FAILS at a program point where control
  never leaves the proc.
- **A landing block with no reachability.** The `jmp` contributes no `Follow`
  edge, so the unrolled copy block it dispatches into is unreachable to every
  CFG-based analysis and its productions are invisible.

`preserves` sees the same edge: `ExitKind::Defer` reads the callee via
`transfer_target_sym`, which also rejects `PcRelIdx`, so it charges the
"unknown/indirect target → conservative clobber" arm. **Fail-safe in both
consumers — precision gap, not a soundness defect.**

**The clobber closure is NOT affected, proven rather than argued.** Inserting
`moveq #0, d5` inside the unreachable unrolled block fired
`[proc.clobber-undeclared]` on `S4LZ_Decompress` and transitively on
`Art_Decompress` / `Level_LoadArt` / `CompressionSelfTest`. `proc_written_registers`
is a LINEAR item scan, so an unreachable block cannot hide an undeclared write.

**Corpus census of the idiom — 6 sites, 5 procs** (`grep -n '\bjmp\b'` over
`engine/` + `games/`, all forms):

| site | operand form | enclosing proc | declares `out`? |
|---|---|---|---|
| `engine/compression/s4lz.emp:111` | `PcRelIdx` | `S4LZ_Decompress` | **yes — `out(a0, a1)`** |
| `engine/compression/s4lz.emp:165` | `PcRelIdx` | `S4LZ_Decompress` | **yes** |
| `engine/objects/animate.emp:139` | `PcRelIdx` (with addend) | `AnimateSprite` | no |
| `engine/system/dma_queue.emp:274` | `DispSymInd` + `targets(...)` | `Process_DMA_Critical` | no |
| `games/sonic4/player/player_sensors.emp:330` | `PcRelIdx` | `Player_SensorSurface` | no |
| `games/sonic4/player/player_sensors.emp:435` | `PcRelIdx` | `Player_SensorWallDir` | no |

(`player_common.emp:562` `jmp (a1,d1.w) as PlayerHook` is a genuine external
indirect dispatch and is correctly a transfer out.)

**So the blast radius TODAY is exactly `S4LZ_Decompress`** — the other four
procs declare no `out`, so their false exit sites are latent. That is a
coverage fact, not a licence: the next `out` added to any of them fires
inexplicably.

`targets(...)` already exists and already means the right thing — but
`cycle_budget::dispatch_successors` is, in its own words, "the ONE consumer of
`targets`", and `CodeItem::Instr::targets`' doc says so too: *"ONLY the
cycle-budget walk reads it … every other CFG consumer keeps treating the
instruction as an opaque computed transfer."* Making `Cfg::edges` honor the
clause is the structural fix (§8, F3).

---

## 2. THE RETURN-PATH × REGISTER TABLE

Body: `aeon/engine/compression/s4lz.emp`, `S4LZ_Decompress`, lines **85-216**
(re-resolved this session; all line numbers current).

### Obligation sites the verifier charges: THREE

| # | site | line | edge | is it really a return? |
|---|---|---|---|---|
| **S1** | `rts` at `.return` | 215 | `Edge::Return` | **yes** — the proc's only real exit |
| **S2** | `jmp .lit_end(pc,d1.w)` | 110 | `Edge::TailOut` | **NO** — intra-proc dispatch (§1) |
| **S3** | `jmp .match_end(pc,d0.w)` | 163 | `Edge::TailOut` | **NO** — intra-proc dispatch (§1) |

All three fail for `a1`, independently. `a0` passes at all three (`move.w (a0)+, d3`
at line 87 produces it before any branch), which is why only `a1` is in the
residue.

### The paths that reach S1 (the real return)

`.stream_done` (line 203) is entered ONLY from `beq .stream_done` at line 97, so
every real return runs `.token_loop` at least once.

| # | path to `.return` | `a1` write on the path | `a0` |
|---|---|---|---|
| **R1** | first token word is `$0000` — zero output bytes | **NONE** | produced @87 |
| **R2** | every token has 0 literals AND 0 matches (`beq .token_loop` @141), then EOS | **NONE** | produced @87 |
| **R3** | ≥1 literal or match word emitted, tile-delta flag CLEAR | `(a1)+` @187 / @198 — full width | produced |
| **R4** | as R3, tile-delta flag SET → `jbsr TileDelta_Undo` @209 | `(a1)+`, then callee clobber | produced |

- **R1/R2 are the local production gap.** `a1` reaches `rts` holding its ENTRY
  value, and that value is CORRECT — zero bytes written means "past end of
  decompressed data" *is* the destination start. The `out()` property is
  "produced on this pass", which is a strictly stronger claim, and it is false
  here.
- **R3/R4 produce it.** The unrolled copy blocks (112-125, 165-178) are
  unreachable to the CFG (§1), but `.lit_dbf_loop` @187 and `.match_dbf_loop`
  @198 are reachable and both `(a1)+`.

**An inherited engine claim, checked rather than trusted.** Lines 209-211 say
`TileDelta_Undo` "clobbers a1 — but its XOR walk ends at buffer_end = the
decode's own a1 endpoint (size is a multiple of 32), so out(a1) holds". Verified
against the body (lines 227-258): for ≥2 tiles `a1 = a0 + 32 + 32*(tiles-1) =
buffer_end`; for 0 or 1 tiles the `ble .done` @230 is taken **before**
`movea.l a0, a1` @233, so `a1` is never written and keeps the decoder's
endpoint. **The comment is true.** It is also irrelevant to the residue — the
verifier's `transfer` is gen-only and a call never un-produces, so R4 is not a
failing path for `a1` under the current model.

---

## 3. EVIDENCE — six probes, each read as a set diff

Gate: `contract_baselines_hold_for_every_shipped_shape` (all seven shipped
shapes), `SIGIL_STRICT_GATE=1`, `AEON_DIR` = the lane's aeon worktree. Baseline
run GREEN first.

Probe instruction: `lea 0(a1), a1  // S4LZPROBE` — a full-width `a1` write. Its
non-vacuity is established by probe **P-TOP**, not assumed.

| probe | perturbation | predicted | **measured set diff** |
|---|---|---|---|
| **P-R** | cover S1 only (`rts`) | rows go if `rts` is the only site | **EMPTY** — rows persist |
| **P-TOP** | one write at the proc's FIRST instruction (dominates everything) | all 3 go | **exactly 3 GONE, 0 NEW** |
| **P-RJ** | cover S1 + S2 + S3 | all 3 go if the site set is exactly these | **exactly 3 GONE, 0 NEW** |
| **A** | cover S1 + S2, leave **S3** uncovered | rows persist ⇒ S3 fails | **EMPTY** (rows persist) |
| **B** | cover S1 + S3, leave **S2** uncovered | rows persist ⇒ S2 fails | **EMPTY** (rows persist) |
| **C** | cover S2 + S3, leave **S1** uncovered | rows persist ⇒ S1 fails | **EMPTY** (rows persist) |

P-TOP is the non-vacuity control AND the cascade proof. P-RJ proves the site set
`{S1,S2,S3}` is exhaustive — with those three covered no obligation remains, so
there is no fourth site hiding. A/B/C attribute one failing site each; **the
`rts` alone is not the story, and any fix derived from reading only the `rts`
would be wrong.**

Two further probes, reported where they belong: the clobber-hole probe (§1) and
the F2 adoption probe (§8).

---

## 4. THE VERDICT — (c) CONTRACT-ONLY, and why not (a) or (b)

**Not (a), a real bug.** For a caller to be harmed it must READ `a1` after the
call and be wrong when `a1` equals its own input. The caller sweep (§5) finds
**no reader of `a1` anywhere in the tree** — not after `S4LZ_Decompress`, not
after `Art_Decompress`, not after `S4LZ_DecompressDict`. Two of the three call
sites explicitly discard it. And on the failing paths the value is not even
stale: zero bytes written makes `dest_start` the correct endpoint.

**Not (b), benign-by-mechanism.** There is a mechanism and it must be NAMED, not
waved at, so here it is: `Art_Decompress`'s header requires callers to skip
size-0 blobs before calling, and `Level_LoadArt` enforces it (`move.w (a0), d4` /
`beq .next`, `load_art.emp:116-117`). A stream with a nonzero declared size must
emit at least one literal or match word, so R1/R2 are unreachable from shipped
call sites. **But that is a caller-side precondition the callee's signature does
not state and the verifier cannot see** — and it does not make the declaration
true, it makes the false case unreached. Filing this as (b) would leave a reader
believing the body guarantees something it does not.

**(c) CONTRACT-ONLY.** `out(a1)` asserts production on every required return
path. The body's real guarantee is *"`a1` is advanced past the bytes written, or
unchanged when none were"* — a threaded in-out, not an output. That is the same
contract shape probe2 §7 identified for `DrawRings`/`InsertSpriteMasks`, arrived
at from an unrelated proc.

Sites S2/S3 are a separate, independent defect: the declaration is not at fault
there at all, the CFG model is (§1). **Neither cause alone explains the row**,
and the correct fix depends on which one you pick (§8).

---

## 5. CALLER SWEEP

Every call site of the three procs in the tree, and what it does with `a1`:

| caller | site | consumes `a1`? |
|---|---|---|
| `Art_Decompress` | `load_art.emp:52` `jbra S4LZ_Decompress` | tail — forwards its own claim, reads nothing |
| `Level_LoadArt` | `load_art.emp:120` `jbsr Art_Decompress` | **no** — `a1` is re-`lea`'d each iteration (@119) and `QueueDMA_Critical` (@128) clobbers `a1` before any use |
| `CompressionSelfTest` | `compression_selftest.emp:48, 68` `jbsr Art_Decompress` | **no** — `.checksum` re-`lea`s `Art_Staging_Buffer, a1` (@86) |
| `CompressionSelfTest` | `compression_selftest.emp:59` `jbsr S4LZ_DecompressDict` | **no** — same |
| `TileCache_*` | `tile_cache.emp:254` `jbsr S4LZ_DecompressDict` | **no, deliberately** — `move.l a3,-(sp)` @253 / `movea.l (sp)+, a1` @255, comment "a1 = slot base"; the decompressor's `a1` is discarded on the next instruction |

**Result: `out(a1)` on all three procs has ZERO consumers.** That is the fact
that decides between the two candidate fixes in §8, and it is the fact a
width-style narrowing would have missed entirely.

---

## 6. THE CASCADE — verified structurally, not inherited

The brief said to verify the structure rather than inherit it, because an
inherited structural claim about this exact kind of dataflow was probe2's
category error. Both dependant edges were re-derived from the source:

- **`Art_Decompress :: a1` — a TAIL dependant, and a PURE one.** `load_art.emp:46-53`
  has exactly two exits, both `Edge::TailOut`: `jbra ZX0_Decompress` (@50) and
  `jbra S4LZ_Decompress` (@52). It writes `a1` nowhere. `a0` verifies because
  `addq.l #ART_HDR_SIZE, a0` @49 produces it on the ZX0 arm and
  `S4LZ_Decompress`'s verified `out(a0)` credits the other. `a1` fails on the
  S4LZ arm only — the MUST-intersection of the two tail credits. So the census's
  "tail MUST-intersection" description is **correct**.
- **`S4LZ_DecompressDict :: a1` — a FALL-OFF dependant.** `s4lz.emp:74-83` writes
  only `a4` (`adda.w d4, a4` / `suba.l a1, a4`) and ends with a declared
  `falls_into S4LZ_Decompress`. `verify_out`'s `Edge::FallOff` arm credits the
  successor's VERIFIED unconditional out. So the census's "fall-off charged to
  its successor" description is **correct**.

**Measured confirmation:** P-TOP produced a set diff of **exactly**
`{Art_Decompress::a1, S4LZ_Decompress::a1, S4LZ_DecompressDict::a1}` GONE, 0 NEW.
One production at the root closes all three, and closes nothing else. The
cascade is real, narrow, and exactly as advertised — the one inherited claim in
this area that survived checking intact.

---

## 7. THE DORMANT SITE-2 GUARD — ARMED AND **RED**

The ledger row (located by substance — grep `"mutant GREEN across all 26
closure-corpus"`) records: *"Site 2 (`corpus_contracts.rs`, the per-proc firing
tier) — mutant GREEN … the guard is real but dormant"*, with kill condition
*"`S4LZ_Decompress` produces `a1` … re-run the site-2 mutant then and confirm it
goes red"*.

**Site 2 re-resolved by substance**: `crates/sigil-frontend-emp/src/corpus_contracts.rs`,
the `check_out(...)` call inside the `out_firings` loop, argument
`pb.falls_into.as_deref()`. **The mutant**: replace that argument with `None`.
(Site 1, for contrast, is the `&falls_into_succ` argument to `compute_verified_outs`
in the same file, and is already guarded by
`out_verify.rs`'s `falls_into_successor_credit_reaches_the_fixpoint`.)

**It was armed, not handed off.** The root was simulated with the P-TOP corpus
perturbation (a `lea 0(a1), a1` at `S4LZ_Decompress`'s entry), which makes
`S4LZ_DecompressDict`'s successor carry a VERIFIED out — the precise condition
the ledger names.

| run | corpus | site-2 mutant | result |
|---|---|---|---|
| control | P-TOP | **off** | `[proc.out-unverified]` GONE set = 3 rows; `corpus_out_residue_is_the_verified_complement`'s complement loop **passes** |
| **mutant** | P-TOP | **on** | GONE set = **2** rows (`S4LZ_DecompressDict` still fires); complement loop **FAILS** |

The mutant's diagnostic, verbatim:

```
S4LZ_DecompressDict::out(a1) is in the out-verify residue yet marked VERIFIED —
the residue surface and must-def credit have drifted apart
```

**That is the guard the ledger predicted, firing for exactly the predicted
reason** — site 1 marks the out verified while site 2 fires it, and
`corpus_out_residue_is_the_verified_complement` asserts the residue is the
verified complement. The mutant is ALSO caught independently by the frozen
baseline's set diff (3 GONE vs 2 GONE), so two gates discriminate it, not one.

The mutant was reverted by string-replace; `git status --porcelain` in the sigil
worktree is 0 lines and `grep -rn S4LZMUTANT crates/` is 0 hits.

**Conclusion: no finding to escalate here — the guard works.** The residual
coverage fact (worth one ledger line, §11) is that the guard is *conditional on
the root closing*: revert the root fix and the plumbing goes untested again. A
unit-level discriminating pair at site 2, mirroring site 1's
`falls_into_successor_credit_reaches_the_fixpoint`, would make it unconditional.

---

## 8. RECOMMENDED FIX, AND ITS MEASURED RESIDUE SET DELTA

Three candidates. **The recommendation is F2, with F3 as an independent
correctness follow-up.** F1 is the more elegant answer and is recorded because
it is where the shape belongs — but it does not close the row without design
work that this lane can only scope, not settle.

### F2 (RECOMMENDED) — retire `out(a1)`; declare `clobbers(a1)`

`out(a1)` on all three procs has zero consumers (§5) and is false on R1/R2 (§2).
The honest declaration says what a caller may rely on: `a1` is destroyed.

```
S4LZ_DecompressDict … clobbers(d0-d3/a0/a2-a4) out(a1) falls_into S4LZ_Decompress
                    → clobbers(d0-d3/a0-a4)            falls_into S4LZ_Decompress
S4LZ_Decompress     … clobbers(d0-d3/a2-a3) out(a0, a1)
                    → clobbers(d0-d3/a1-a3) out(a0)
Art_Decompress      … clobbers(d0-d3/a2-a3) out(a0, a1)
                    → clobbers(d0-d3/a1-a3) out(a0)
```

**MEASURED** (probe F2, the exact edit above, whole `contract_closure_corpus`
suite + `warn_tier_corpus`):

- `[proc.out-unverified]` set diff: **GONE = `{Art_Decompress::a1,
  S4LZ_Decompress::a1, S4LZ_DecompressDict::a1}`, NEW = `{}`.** Residue 30 → **27**.
- `corpus_closure_residue_is_empty_the_error_gate`: **PASSES.** No caller needs a
  new declaration — `Level_LoadArt` (`clobbers(d0-d4/d7/a0-a3)`) and
  `CompressionSelfTest` (`clobbers(d0-d4/a0-a4)`) already license `a1`.
- D1c baseline: **unmoved** (both families).
- `warn_tier_corpus`: **green**, id set unchanged. (`proc.out-unwritten` stays in
  the set — `Art_Decompress` is one firer of an already-open class, not its only
  one.)
- Byte-neutral by construction: `clobbers`/`out` emit nothing. The next wave
  still owns the ×7 byte bar.

**Same-commit obligations, both mandatory:**

1. Delete the three rows from `OUT_UNVERIFIED_BASELINE`
   (`crates/sigil-harness/src/contract_baseline.rs`), **and rewrite that array's
   doc paragraph** — it currently teaches the cascade using these three procs as
   its worked example ("`S4LZ_Decompress :: a1` is the root, and …"). Leaving it
   describing rows that no longer exist is a citation-decay seed. **Coordinate:
   the `outw` lane is rewriting this file.**
2. **Re-point `corpus_out_residue_is_the_verified_complement`'s witness** (§9).
3. Rewrite the `Out:` lines in `s4lz.emp:38-41` and `load_art.emp:40-41`. The
   "past end of decompressed data" fact is still TRUE for a nonzero stream and
   worth keeping as prose, but it must stop reading as a contract callers may
   rely on. Present-tense contract fact only — no history.

**The honest cost of F2**, stated so nobody has to rediscover it: a future
consumer that wants the endpoint must re-derive it (`dest_start + header size`,
which the header already says is the authoritative quantity anyway). Weigh that
against a declared output no caller has ever used.

### F1 (the shape's real home) — an `inout(a1)` facet

Probe2 §8(3) proposes `inout(rN)`: *"on every required return path, rN is either
PRODUCED on this pass or holds its ENTRY value"*. `S4LZ_Decompress::a1` is the
same shape and would be a second adopter. **Two things must be settled before it
is built, and this lane could not settle either:**

- **Vacuity.** Composing the proposal as "`out_verify`'s production dataflow OR
  `verify_preserved_on` scoped to the exits" is wrong at the disjunction: neither
  disjunct holds on ALL paths here (R1/R2 preserve, R3/R4 produce), so an
  all-paths OR of two all-paths analyses rejects an honest contract. The model
  that works is a THREE-valued per-path lattice — `Entry | Produced | Broken`,
  seeded `Entry`, full-width write ⇒ `Produced`, partial write or
  clobbering-callee ⇒ `Broken`, fail only at `Broken`. A param-seeded gen-only
  lattice (the naive reading) verifies `inout` for **anything**, including
  `InsertSpriteMasks`' `addq.b #1, d5` — which is the exact row probe2 escalated
  as needing care. **`out_verify`'s Finding 2 rejects param seeding for a
  reason; `inout` re-introduces it and must earn it back with a kill rule.**
- **Composition across calls, with a corpus exhibit that forces it.** Under a
  `Broken`-on-clobbering-callee rule, `jbsr TileDelta_Undo` (@209, declares
  `clobbers(d0-d1/a0-a1)`, no out) would BREAK `a1` on R4 and `inout(a1)` would
  still fire. `TileDelta_Undo`'s own honest declaration is *also* `inout(a1)` (it
  advances `a1` to `buffer_end`, or leaves it untouched for ≤1 tile — §2), so the
  facet must COMPOSE: an `inout(rN)` callee neither produces nor breaks its
  caller's `inout(rN)`. `TileDelta_Undo` is the corpus's forcing case for that
  rule and should be in the spec.

If F1 is built to that design, its predicted yield is `S4LZ_Decompress::a1` +
`S4LZ_DecompressDict::a1` + `Art_Decompress::a1` + `DrawRings::a4` +
`InsertSpriteMasks::a4` = **5 rows**, with the two `d5` rows still open behind
probe2 §7's escalation. **That is a PREDICTION, not a measurement** — no such
facet exists to probe, and the two `a4` rows are probe2's to confirm.

Note F1 would also make S2/S3 harmless as a side effect (`a1` is `Entry` at both
`jmp`s), which is a reason to like it and NOT a reason to skip F3.

### F3 (independent, do it regardless) — teach the CFG about `targets(...)`

Turn a computed transfer carrying a `targets(...)` clause into N `Follow` edges
in `Cfg::edges`, exactly as `cycle_budget::dispatch_successors` already does for
the budget walk, and adopt the clause on the two `s4lz.emp` `jmp`s.

**Closes 0 residue rows on its own** — S1 is untouched by it, and P-R measured
that covering S1 alone changes nothing either. Its value is correctness of the
model, not burn-down: it removes two false return paths from a shipping analysis
and makes a dispatch-landing block visible. Design notes for whoever builds it:

- The `.lit_end` dispatch lands on any of 15 points (entries 14…1 and `.lit_end`
  itself), 14 of which carry no label today. Enumerating them means labelling
  them. Weigh that against the cheaper alternative of *not charging an out
  obligation at a computed transfer whose target symbol is a local label* —
  which needs `branch_target` widened to `PcRelIdx`/`DispSymInd`, and which
  restores `Cfg::branch_edge`'s existing three-way (local label → `Follow`) for
  free.
- Adding reachability does NOT change this proc's residue: entering at `.lit_end`
  produces no `a1`, and `.no_literals` is already reachable via `beq` @103, so
  the MUST-intersection is unchanged. Do not sell F3 as a burn-down.
- The four other computed-dispatch procs declare no `out` today, so F3 is not
  urgent — but it is the kind of latent false-positive that surfaces as an
  inexplicable firing at the worst moment.

### Projected burn-down after F2

| item | rows | running total |
|---|---|---|
| baseline today (re-derived, counted) | — | **30** |
| `outw` lane, width types | −15 | 15 |
| probe2 lane, `probe_core` `(1)+(2)` together | −8 | 7 |
| probe2 lane, `inout` for `DrawRings`/`InsertSpriteMasks` | −4 | 3 |
| **this lane: F2** | **−3** | **0** |

---

## 9. A LIVE GATE THAT BREAKS WHEN THE ROOT CLOSES

`crates/sigil-cli/tests/contract_closure_corpus.rs:935`,
`corpus_out_residue_is_the_verified_complement`, asserts:

```rust
r.out_firings.iter().any(|f| f.proc == "Art_Decompress" && f.reg == "a1")
```

This is the witness that the residue surface reads the VERIFIED map and not the
DECLARED one. **It fails under BOTH candidate fixes** — measured, twice (P-TOP
and F2 each produced its "expected Art_Decompress::out(a1) in the fixpoint
residue … its absence means the residue surface is reading the declared map"
panic).

The test's own doc already records the precedent: `Collision_GetType::out(d0)`
was the original witness and went silently inert when aeon fused a callee. **This
is the second instance of the same failure mode in one test, and the first one
was silent while this one is loud** — so the replacement must be chosen for
durability, not convenience.

**Requirement for the replacement witness** (from the test's own doc): a firing
that exists ONLY under verified credit — i.e. a proc that produces the register
nowhere itself and sources it from a callee/tail that DECLARES it while failing
verification. After F2 no such row remains in the `a1` family. Candidates to
check against the post-F2 residue, in the same commit:
`Collision_GetType::d0` (declared-but-narrow source — re-check whether it is
still a leaf), and the `probe_core` family once probe2's fix lands. **If no
corpus row has the property, say so and replace the witness with a synthetic
two-proc unit case rather than a weaker corpus assertion** — an inert witness is
worse than an honest unit test.

---

## 10. THE THREE LATENT ITEMS

### (i) `flag_check::abandons_flag` returns `true` on `Return | FallOff` with no `falls_into` consultation

**Confirmed as written**, `flag_check.rs:948-951`. For a proc declaring
`falls_into Successor`, running off the end is NOT abandonment — the carry flows
into the successor's frame inside the same call, and the successor may consume
it. The arm's own comment states the false half out loud: *"running off the end
drops it. This check does not care which."*

- **Polarity: over-fires.** Returning `true` raises `[call.flag-result-unused]`.
  Loud, not silent, and there is a documented escape (`@discard`).
- **Corpus-dead today, measured**: `corpus_closure_residue_is_empty_the_error_gate`
  asserts `flag_firings` is empty and is green, so no `falls_into` proc reaches
  its fall-off with an unconsumed carry result. The corpus has 38 `falls_into`
  procs and 4 68k carry-result callees (`RingBuffer_Add`, `QueueDMA_{Critical,
  Important,Deferrable}`); no `falls_into` proc calls one at its tail.
- **Verdict: OPEN, precision gap, corpus-dead.** Kill condition below.
- **A SECOND gap in the same function, not previously recorded, and in the
  OPPOSITE direction:** the `Edge::TailOut | Edge::BranchOut => continue` arm
  prunes the path as "the flag flows out of the proc". At a computed intra-proc
  dispatch (§1) that is FALSE — control stays inside the proc — so an abandoned
  carry past such a dispatch would go **unreported**. That is the false-NEGATIVE
  direction on a shipping gate. Checked all six computed-dispatch sites: none is
  preceded by a carry-result call inside its own proc, so it is corpus-dead too —
  but this one is dead by coincidence, not by design, and F3 closes it.

### (ii) `z80_preserves`' tail arm consults `falls_into` but not `noreturn`

**Confirmed, and it is structural rather than a missing call**:
`verify_z80_preserved`'s signature (`z80_preserves.rs:301-307`) does not TAKE a
`noreturn` set at all. Its `Edge::TailOut | Edge::BranchOut` arm charges the
preserve obligation unconditionally, while the 68k twin (`preserves.rs`,
`ExitKind::Defer`) calls `exit_diverges(&items[idx], self.noreturn)` and returns
early on a diverging target — and `out_verify`'s `TailOut` arm does the same.
**The two CPUs' exit models have drifted on exactly this axis.**

- **Polarity: over-obligates** (a Z80 proc tail-transferring to a `@noreturn`
  target would be charged a `preserves` claim at a point that never returns).
- **Corpus-dead today, measured.** Four Z80 `@noreturn` procs exist
  (`Z80_Sound_Entry`, `SndDrv_Init`, `SndDrv_Idle`, `SndDrv_Sample`) and five
  `jp` sites target them — but every enclosing proc (`Z80_Sound_Entry`,
  `SndDrv_Idle`, `SndDrv_Sample`, `SndDrv_TimerATick`) declares
  `clobbers(af, bc, de, hl, ix, iy)` and NO `preserves`, so there is nothing to
  charge. Note `SndDrv_Init … falls_into SndDrv_Idle` is a `falls_into` to a
  `@noreturn` target — the same gap through the `FallOff`/`ever_clobbered` arms —
  and it too declares no `preserves`.
- **Verdict: OPEN, precision gap, corpus-dead.** Kill condition below.

### (iii) The per-file `[proc.out-unwritten]` exemption is proc-wide, and is the only out gate that sees Z80

**Both halves confirmed, and the sentence's implied conclusion is wrong — which
is the finding.**

- **Proc-wide: yes.** `lower/proc.rs`, `charge_unwritten = proc.falls_into.is_none()`
  drops the claim for EVERY declared out of a `falls_into` proc, not only for the
  ones the successor produces. **Its own comment already says so and says why**
  ("THIS TIER'S EXEMPTION IS WEAKER THAN THE CLOSURE'S, deliberately and
  visibly … dropping a value obligation is only safe where something else
  re-charges it").
- **The re-charge is REAL and I proved it this session.** `out_verify`'s
  `Edge::FallOff` arm charges the successor's verified out; the site-2 mutant
  going RED (§7) is a direct, positive demonstration that the re-charge is wired
  and consulted. The chain closes: `falls_into` proc → closure tier → frozen
  `[proc.out-unverified]` baseline → ratchet. Not theoretical — `build.sh` runs
  the closure by default (aeon `d8c93d7`). **Verdict for this half: NOT a gap.
  Discharged, with the discharge now witnessed.**
- **"Only out gate that sees Z80": true but it does not compose into a Z80
  hazard the way the sentence suggests.** `check_out` runs for both CPUs, but its
  UNWRITTEN half is 68k-only by explicit design (`proc_written_registers` is the
  68k heuristic and would false-fire on every Z80 out). So on Z80 the exemption
  is INERT — there is nothing for it to exempt.
- **The real Z80 finding is bigger and is not about the exemption at all: NO
  production check of any kind runs on a Z80 `out`.** The per-file unwritten half
  is 68k-only; the closure tier is 68k-only twice over (`proc_bufs` excludes
  `(cpu: z80)` modules, and `Reg::from_name` rejects Z80 spellings —
  `corpus_contracts.rs` says so in its own comment). **Measured: 27 Z80 procs
  across 6 modules declare `out(...)`** — `sound_fm` (7), `sound_psg` (7),
  `sound_sfx` (9), `sound_sequencer` (2), `z80_sound_driver` (2) — including
  register outs (`out(hl)`, `out(iy)`, `out(b, c)`, `out(d, e)`) and conditional
  flag outs (`out(carry: found)`, `out(carry: dropped)`). **Every one is
  unverified, and none is on any baseline** — the residue does not record them
  because nothing looks. The whole sigil-native sound stack ships declared
  outputs with zero out-honesty coverage.
- **Tractability, since it changes the priority:** the pieces exist.
  `z80_preserves::z80_writes` is a Z80 write detector, and `Cfg::z80_edges` is a
  Z80 edge model. A Z80 `verify_out` is an assembly job, not a research one.
- **Verdict: OPEN, and this is the largest single coverage gap the three latent
  items contain.**

---

## 11. WHAT I DID NOT DO, AND WHY

- **No `crates/` deliverable edit.** The only `crates/` touch was the site-2
  mutant, reverted by string-replace and proven (`git status --porcelain` = 0,
  `S4LZMUTANT` grep = 0), with both gates re-run green afterwards.
  `contract_baseline.rs` was never opened for writing; its count is unchanged
  at 30.
- **No ROM build and no byte bar.** Every measurement rides the corpus walk,
  which parses `.emp` source and emits nothing. Whoever lands F2 owns the ×7 byte
  bar; the change is byte-neutral by construction (contract text emits no bytes)
  but that is a claim to VERIFY, not to inherit from here.
- **No oracle trace.** The question is static: `a1`'s failing paths are the ones
  that write zero bytes, and no caller reads `a1` at all. An A/B would compare
  two ROMs that differ in no bytes.
- **`d0`/width rows untouched** (the `outw` lane's), and `DrawRings` /
  `InsertSpriteMasks` untouched (probe2's, closed). §8's F1 yield for their
  `a4` rows is flagged as a prediction for them to confirm, not a result.
- **F1's design is scoped, not settled.** The three-valued lattice and the
  composition rule are what a spec must decide; this lane names the corpus
  exhibit (`TileDelta_Undo`) that forces the second one.
