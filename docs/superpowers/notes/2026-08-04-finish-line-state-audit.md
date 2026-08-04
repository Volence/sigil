# 2026-08-04 — finish-line plan STATE AUDIT: four items already done, and a
# K1-class STOP on Track B

**Overseer session (Opus), 2026-08-04.** Brief: execute the finish-line plan
(`specs/2026-08-03-finish-line-plan.md`). **No parcel was dispatched.** This
note is the reason: a pre-dispatch state verification found that the plan's
premises are stale in four places, one of them load-bearing enough to stop
Track B outright. Escalation per the brief: FABLE gets this note (do not wait);
VOLENCE gets the status report.

**Nothing was built, merged, pushed, or reverted this session.** The only
commits are `0e4f520f` (Fable's three specs + gap-ledger, committed so worktrees
can see them) and this note. Sigil master is NOT pushed — see §5.

---

## §1 — Track 0 (the five round merges): ALREADY LANDED

All five checkpointed branch pairs are ancestors of master in BOTH repos, and
of `origin/master` on sigil:

| Branch | sigil | aeon |
|---|---|---|
| `l1-p1-contract` | merged (also in origin/master) | n/a — sigil-side only (the construct) |
| `l1-p2-conversion` | merged | merged |
| `lang-l5l8-types` | merged | merged |
| `lang-l9-offsets` | merged | merged |
| `lang-onesit` | merged | merged |

Verified with `git merge-base --is-ancestor <branch> master` per pair. A prior
overseer session landed them between the plan's authoring (2026-08-03) and now.
Master has since moved a long way on engine work: sigil master is at
`a932c5aa` (refreeze **chain entry 33**, `wave4-z80-sound-reclaim`, −231 B
resident Z80), aeon master at `5810960`. The plan's chain-18-era CRC
expectations are therefore stale by construction — **any future parcel must
re-derive its byte bar against the current chain, never against a packet CRC
quoted in an older note.**

## §2 — D6 (parser hang): ALREADY FIXED at t28

`crates/sigil-frontend-emp/tests/parser_recovery_hang.rs` is no longer
`#[ignore]`d. Both tests pass (`2 passed; 0 failed; 0 ignored`). The file's own
header records the t28 P1 root cause and fix: `recover_to_next_decl` listed
`extern` as a declaration opener, but `extern` is CONTEXTUAL (only `extern proc`
is an item), so recovery stopped on a bare `extern` without consuming and
`item()` bounced it back — the loop was in TOP-LEVEL RECOVERY, not in
operand/expression parse as the gap-ledger row (~:1619) and the plan's D6
brief both state. The `asm_body` zero-progress guard is a separate, retained
defense. **The plan's D6 porter brief ("extend the zero-progress guard down the
operand/expression recursion") would have been work against a non-existent
bug.**

## §3 — T4 (phase-aware repin): ALREADY BUILT

`crates/sigil-harness/src/bin/repin.rs:62` carries an explicit `(T4)` comment
and calls `native::phase_bank_lmas(&aeon, …)` for both shapes, feeding
`Listing::with_phase_lma`. Phase-bank label LMAs resolve to the section LMA
rather than the phase VMA. Nothing to build.

## §4 — TRACK B: STOP. The contract system is already ~the unification.

**This is the K1-class finding.** The contract-unification spec's premise is
that `clobbers`/`preserves`/`out` shipped as *syntactic slices* (D2.32/D2.35)
and that the S2-D6/D7 dataflow pass is *unbuilt*. That was true when those
decisions were recorded. It is **not true now**: the campaign built
**contract-grammar v2**, and it already covers most of P1–P4.

Evidence (module headers + the import surface of
`crates/sigil-frontend-emp/src/corpus_contracts.rs`):

| Spec parcel item | Spec says | Actually shipped |
|---|---|---|
| P1 `[contract.live-clobbered]` — "the S2-D6(a) headline check" | build it | **`[call.live-clobbered]` (D1c)** in `calls.rs`, over a real CFG with joins |
| P1 caller-side input checking | not specced | **`[call.input-undefined]` (D1b)**, forward MUST-def dataflow |
| P1 write sets from instructions | build it | `lower::proc_written_registers`, reused by the corpus walk "with no drift" |
| P1 transitive propagation | build it | `closure::{compute_closure, ProcNode, RegEffect}` — the whole-corpus transitive closure |
| P1 `calls(...)` blind-call surface | new surface | **already exists in another spelling**: `jsr (aN) [as Type]` indirect sites contribute a declared bound or ⊤ |
| P3 contexts / the Z80 bracket — "the memory-safety headline" | build `context` + `with` | **`[bus.*]` in `z80_bus.rs`**: a 3-point lattice (Stopped/Running/Unknown), forward MUST dataflow over the CFG, worklist to fixpoint, path-sensitive, zero-false-positive stance, keyed on RESOLVED operands ($A11100 / VDP ports) so macro spelling cannot evade it |
| P4 CCR | build it | `flag_check::{check_flag_unused, check_result_invalid_path}` + the CFG the other nets reuse |
| P4 conditional `out(reg) if cc` | "deferred to P4" | `out_verify::CondOutMap` / `UncondOutMap` exist |

Also present and unaccounted for by the spec: `type_slice::check_slot_types`,
`branch_const::check_branch_const`, `preserves::find_dead_saves`,
`preserve_oracle_inputs`, `verified_preserves_regs`, extern/proc collision
detection (§11 Q4), unresolved-callee holes, and an `emp_contracts` driver
binary that prints firings + boundary stats.

**Consequence:** dispatching B-P1 as written would have a porter rebuild a
shipped, more sophisticated system. Track B is STOPPED pending Fable's
re-scope. This also ANSWERS the gap-ledger's own open pre-spec check
("verify clobbers/preserves/out enforcement depth", 2026-08-03): the answer is
*much* deeper than declaration-only — v2 is a real dataflow contract system.

### What of Track B still looks genuinely open (NOT verified in depth — Fable to confirm)

1. **Generalized contexts.** `z80_bus.rs` is HARD-CODED to the Z80/VDP hazard.
   The spec's `context` declaration + `with` bracket + `requires` propagation —
   i.e. letting the ENGINE and GAMES declare their own contexts, and proving
   acquire/release pairing structurally rather than inferring bus state — is a
   real generalization of a proven net. Note the shipped net's zero-FP stance
   deliberately does NOT flag an unpaired toggle at proc entry; a declared
   bracket WOULD close exactly that gap. This is the strongest surviving piece
   of the memory-safety arc.
2. **Stack-delta (S2-D7(b))** — not found in the audit; likely open.
3. **Cycles (P5, S2-D7(c))** — not found; likely open.
4. **The t30 G2 STOP** (callee-declared-preserves crediting) and the t24 asks
   (never-written-as-proof; conditional `out` coexisting with `clobbers`) —
   NOT re-verified this session. Given v2's scope, some may already be closed.
   **Verify before speccing.**
5. **Track C (niche Option)** — untouched by v2; genuinely open as written.

## §5 — Repo hygiene flags for Volence

- **Sigil master is 7 commits ahead of `origin/master` and unpushed** (6 are the
  concurrent session's engine-support work: the wave-4 Z80 reclaim re-pins,
  `pad_to_cycles` dense mode, seam1 base derivation; the 7th is `0e4f520f`,
  this session's spec commit). I did NOT push — pushing would carry another
  session's unpushed work, which is not this session's call.
- **Aeon has an overnight hardening run** (review items 25–30) staged at
  `5810960`, committed 00:11 today. The aeon main checkout was NOT touched.
- Aeon master is likewise well ahead of its `origin/master`.

## §6 — Verified-open, safe to dispatch as written

D1 (`[operand.const-as-address]`), D2 (`[table.name-collision]`), D3 (shadow
lints) — greps find no implementation. D5 (patch/bind demotion) — `patch`/`bind`
still appear in the Spec-2 §10 inventory, so the ratified demotion is unstarted.
T1 (RAM-map report) — no `ram_map` surface found. T2 (parametric memory_hash) —
NOT verified (the oracle grep was inconclusive); keep the plan's confirm-first
instruction.
