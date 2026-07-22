# S2-D6 checked-clobbers / preserves lint (Phase-1 item #3) — build spec brief

**Overseer-authored, 2026-07-21.** Phase-1 item #3 (roadmap `pre-t18-roadmap.md`), the last
verifier item before the D1b WARN→ERROR flip (#4). Off masters: **sigil `661b79b`, aeon
`ae1de4d`**. Byte-neutral (analysis + contract-text only; zero codegen change). Canonical
ROMs (2026-07-21 Deep-Forest-BG re-baseline — NOTE these are NEW, do not use the old pins):
plain **`3aa43cb6`/420749**, debug **`ce0e83a6`/428768**.

**Goal:** `[proc.clobber-undeclared]` becomes flip-grade honest — every register a proc
writes is either declared (`clobbers`/`out`), a param, or **mechanically proven preserved**
— with no false-positive class left that pressures a *dishonest* `clobbers()` declaration,
and no unmodeled write form that silently under-fires. This also delivers what Phase-2.5's
Tier-C movem deletions need: a per-callee clobber union proven mechanically, not by prose.

---

## 0. MANDATORY Stage 0 — re-census the terrain BEFORE designing (the roadmap text is stale)

The roadmap's item-#3 rationale ("`preserves()` can't express individual push/pop pairs →
FPs on ≥3 live instances: AllocDynamic a0, Collected_Park/UnparkSlot a0") **predates G3**.
G3 (2026-07-17) declared honest `preserves()` on those exact procs and §5 **verified all of
them** (individual-push branch-straddled; push + `(sp)` peek + `dbf` copy loop — see the G3
checkpoint table), the substrate parcel (07-18) grew §5 further (linear-delta, movem-frame
recovery), and `corpus_contracts.rs:211` proves verified preserves already SUBTRACT from
the clobber lint. So the headline FP class is probably ALREADY CLOSED.

Stage 0 = run the clobber census on the CURRENT corpus (post-G3/substrate/G4/G4.5) and
report, before any design:

- every live `[proc.clobber-undeclared]` firing (proc, reg, adjudication: real / FP-class /
  already-cleared);
- every proc whose declared `clobbers` exceeds computed-local + declared-callee union (the
  over-declaration census — pass-3 fuel, not a firing);
- explicit re-test of the 3 historical individual-push sites — expected: 0 firings.

**If Stage 0 shows the roadmap's FP list is empty, RE-SCOPE item #3 to the residual gaps
below and say so in the packet — do not inflate the item to match its stale description.**

## 1. Candidate residual gaps (adjudicate each at Stage 0; build only what's real)

- **(a) `dbcc` counter-write blind spot** (tranche-4 ledger, "dbcc clobber-lint blind
  spot"): `dbf`/`dbcc dN, label` decrements dN — confirm `instr_written_regs` counts it.
  If missed, that's a FALSE NEGATIVE (under-fire) — the worst polarity for a flip-grade
  lint. Same completeness sweep for any other write-form stragglers (`abcd`? `exg` both
  regs? `movep`? — enumerate against the corpus mnemonic census, don't guess).
- **(b) movem-pair expressiveness** (tranche-13: "movem pairs inexpressible; 1 residual
  warning"): a proc that saves/restores via `movem` with a register list §5 can't currently
  pair. Check whether the substrate movem-frame growth already closed this; if a residual
  warning remains on the live corpus, grow §5 (the-verifier-grows precedent), never relabel.
- **(c) the `.asm` tier**: the roadmap says the FP pattern existed "both `.emp` and `.asm`
  tiers". The `.emp` side is §5's; the `.asm` side is s4lint **W021** (which already
  requires a RESTORE, not just a push, to exempt — R2 rider). Census W021's current state
  on the live `.asm` remainder; port the same individual-push/movem discipline if it still
  FPs. If s4lint is already honest here, record that and move on.
- **(d) per-callee clobber union at call sites** (the Tier-C unlock): expose the
  proven union (declared clobbers ∪ NOT-verified-preserved writes) per callee so Phase-2.5
  can mechanically justify deleting caller-side movem frames. Likely a small export over
  existing facts, not new analysis. Define its exact consumer contract in the design note.
- **(e) OUT of scope — D1a transitivity.** The computed write set stays LOCAL (census fact
  #1: a proc whose work is all in callees legitimately shows `COMPUTED=(none)`).
  Transitive closure is D1a, its own future item. Do not fold it in.

## 2. Soundness polarity (read before designing)

`[proc.clobber-undeclared]` is an over-fire-safe observe-tier lint — but its FAILURE MODE
is social, not mechanical: a false positive pressures the author to "fix" it with a false
`clobbers(rN)`, and THAT lie then propagates into every caller-side analysis that trusts
declared clobbers (must-def, §6, D1c, pass-3 hoists). So:

- An **FP is a design defect** here even though the gate polarity tolerates it — the fix
  is always THE VERIFIER GROWS (§5 extension, s4lint growth), never a dishonest
  declaration and never a silent allowlist.
- A **false negative (missed write) is flip-blocker-class** — dbcc-style completeness
  holes must close or be proven absent from the corpus (grep-proof + a guard test, the
  conditional-external-tail precedent).
- The `a7` push/pop exemption stays (stack discipline); a genuine `movea.l x, sp`
  replacement must still fire (census fact #5 — keep its guard).

## 3. Process (the G4.5 loop, verbatim — it caught real holes all three times)

spec brief → **adversarial refute-the-design pre-code** (a second pass whose only job is
to break the proposed mechanism on paper) → build in bisectable commits → checkpoint-gate
each Stage-0 adjudication VERIFIED IN CODE (not the summary) → **adversarial attack on the
finished diff** (item #2's post-build review found the label-join hole this way — assume
yours has one too and hunt it) → independent gate from a clean checkout → merge.

## 4. Tests (both directions + mutation traps)

1. **FP-kill regression** per gap actually built (e.g. movem-pair proc verifies → no
   firing) — plus the existing G3 five stay green.
2. **Push-without-restore still FIRES**; restore into a DIFFERENT register still FIRES
   (mutation: a balance-counter heuristic that only counts pushes/pops must break these).
3. **dbcc**: undeclared `dbf d7, .loop` fires on d7 (if gap (a) is real); mutation: drop
   the dbcc arm from `instr_written_regs` → test goes green → proves it load-bearing.
4. **Runtime-trip-count round-trip stays unverifiable** (the existing
   `runtime_loop_advance_is_not_falsely_verified` guard must survive any §5 growth).
5. **a7-replacement guard** stays red on `movea.l x, sp`.
6. If (d) ships: a call-site union test — callee with verified `preserves(a0)` +
   `clobbers(d0)` yields union {d0} at the caller, and a mutation that reads DECLARED
   preserves (unverified) instead of VERIFIED must break it.

## 5. Gates

- Byte-neutral: canonical CRCs EXACT both shapes (plain `3aa43cb6`/420749, debug
  `ce0e83a6`/428768) — build from a clean seeded aeon worktree at `ae1de4d`; the MAIN
  aeon checkout is canonical right now, keep it that way.
- Full strict from tips (failures-first, explicit counts; currently 2434/0/1), clippy
  clean on touched files, bisectable commits, packet with per-pass step-3/step-5 findings.

## 6. STOP-don't-bank forks

- Stage 0 contradicts the roadmap (expected) → re-scope + report, don't inflate.
- A true contract §5 still can't verify after reasonable growth → STOP, report the
  shape; do not declare a false `clobbers`.
- Anything here changes a D1b/D1c/§6 firing → that's a cross-gate fork; report it.
- If the item reduces to near-nothing (all gaps already closed), that is a GOOD outcome:
  the packet says "item #3 was already delivered incrementally; here is the proof", and
  #4 (the flip) moves up. Do not manufacture work.

## References
- Roadmap: `docs/superpowers/notes/pre-t18-roadmap.md` item #3 (+ the Tier-C row it unlocks).
- G3 checkpoint table (the 5 verified residue procs): `2026-07-17-contract-grammar-g3-checkpoint.md`.
- Census: `2026-07-17-diagnostics-contract-census.md` (facts #1/#4/#5; A1 FP list — stale, re-run).
- Machinery: `preserves.rs` (§5), `closure.rs` (`check_clobbers`), `corpus_contracts.rs:211`
  (verified-preserves subtraction), s4lint W021 (R2 restore-required rider).
- Ledger rows: 1023/1030/1062 lineage (1030 CLOSED by G3 — the packet should cite this).
