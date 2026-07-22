# S2-D6 item #3 (A+B) — design + adversarial refute-the-design (pre-code)

Scope ruled: **A** (write-detector completeness) + **B** (local `check_clobbers` verified-
preserves subtraction). (d) deferred (gap-ledger). Four overseer riders carried. Byte-neutral.

## Design

### A — grow the shared detector `instr_written_regs` (proc.rs:509)

The shared detector feeds `proc_written_registers` → the closure's `local_writes` (the ERROR
gate's input) AND `check_out`. Two write forms are added:

1. **dbcc-family counter.** `dbf`/`dbeq`/`dbcc`/… `dN, <label>` decrements `dN` (the FIRST
   operand — confirmed `lower_m68k_dbcc`, code.rs:564, shape `[Reg(dN), Sym(label)]`). Add: if
   `mnemonic.starts_with("db")` (the convention flag_check.rs:228 / out_verify.rs:587 already
   use) and `ops.first()` is a `Reg`, push it.
2. **Non-stack movem-LOAD reglist.** `movem <ea>, <reglist>` (reglist = LAST operand = the
   destination) writes every listed register. Add: if `ops.last()` is `RegList(mask)` AND
   `ops.first()` is NOT `PostInc(A7)`, push `expand_mask(mask)`. **The `(sp)+` exemption is the
   crux** — a `movem.l (sp)+, …` restore is stack-preserve-discipline (the exact parallel of
   the existing a7 push/pop exemption); counting a restored reglist would false-positive a
   defensive over-save (rider 2). A non-stack load (`movem.l (a0)+, d0-d6/a2`) IS a real write.

Existing dedup (proc.rs:526) covers the postinc-base ∪ reglist overlap. **Comment updates:**
`preserves.rs:139` and `calls.rs:86` currently say "instr_written_regs does not expand movem
reglists" — becomes "expands NON-STACK movem-load reglists; `(sp)+` restores stay exempt as
preserve discipline." (Both consumers re-expand ALL movem themselves + dedupe, so they are
unaffected — verified below.)

### B — `check_clobbers` subtracts VERIFIED preserves (proc.rs:343)

Add `verified_preserves_regs(proc, buf)` (existing helper, proc.rs:1082) to `check_clobbers`'
`allowed` set, beside clobbers ∪ params ∪ out. That helper returns the declared preserves set
**only when §5 verification passes** (it runs `check_preserves` into a throwaway sink and
returns ∅ on any Error) — so a declared-but-UNVERIFIABLE preserves subtracts NOTHING and the
register still fires (rider 1, by construction). This is the SAME subtraction the transitive
closure already trusts (closure.rs:187 `− verified_preserves`). Kills the 25 verified-preserved
FPs; aligns the three surfaces (WARN lint = ERROR gate = check_preserves).

### Consumer-perturbation proof (rider 3 — every consumer of the shared detector)

- **`out_verify::produced_regs`** (135) — has its OWN movem-load branch (expands the mask
  unconditionally for ALL movem-loads, dedupes `instr_written_regs`). My change feeds it the
  same regs it already computes → dedup → identical. dbf: mnem≠movem → width filter, `.w` data
  reg → DROPPED (dbf cannot satisfy `out(dN)` — rider-3 guard).
- **`calls::written_names`** (93, feeds must-def/D1b, D1c) — OWN movem-load expansion into a
  BTreeSet (dedupes). Identical result.
- **`preserves.rs::ever_clobbered`** (127) — OWN `reglist_mask` expansion (idempotent bit-set).
  Unchanged.
- **`preserves.rs::transfer`** (336) — stack movem handled by early return at `is_push`/
  `is_pop` (a7-only, 287/292) BEFORE the `instr_written_regs` path (425). A non-stack movem-
  load is NOT a pop → falls to 425 → now correctly clears entry-bit/delta for freshly-loaded
  regs (1 corpus site, DecompressBlock, clobbers-only → no preserve verdict changes). dbf
  counter → clears d7 entry-bit (already cleared by its `moveq` init → no verdict change).
- **`proc_written_registers`** (537, feeds closure `local_writes` + check_out) — pure union of
  `instr_written_regs`, no own expansion → this is where completeness lands (intended).

**Net firing prediction:** 0 new firings on every surface. dbf counters are always `moveq`-
preinitialized (grep-proven: all 86 sites) → already in `local_writes`. The 1 non-stack movem-
load's regs (d0-d6/a2) are all in DecompressBlock's `clobbers(d0-d7/a0/a2-a4)` AND written
elsewhere in the decompress body → `local_writes` unchanged. Empirical snapshot at build.

---

## Adversarial refute-the-design (second pass, only job = break it on paper)

**R1 — the `(sp)+` exemption is a false NEGATIVE, and §2 calls FNs flip-blocker-class.**
A `movem.l (sp)+, d0-d3` that pops FRESH values (not a restore) genuinely clobbers d0-d3 and
would be missed. → *Resolved:* (a) it is NOT a new FN — `instr_written_regs` misses ALL movem-
loads today, so `(sp)+` stays exactly status-quo while `(a0)+` is fixed; (b) it is PROVEN
ABSENT — every `(sp)+` movem in the corpus is the restore-half of a matching `-(sp)` save (0
fresh-pops); (c) §5 stack-balance/underflow is the backstop for malformed stacks; (d) the
alternative (count `(sp)+` restores, subtract via verified_preserves) does NOT work — a
defensive over-save doesn't DECLARE preserves(d4-d7), so §5 subtracts nothing and it FPs
(rider 2). Trading a proven-absent FN for a real FP, matching the a7-discipline philosophy, is
correct. Documented boundary; a future non-restore `(sp)+` movem would revisit it.

**R2 — dbf on an out-register produces a false out.** → width filter drops dbf (`.w` data);
no `out(dN)` names a dbf counter (grep). Guard test pins it.

**R3 — double diagnostic from B re-running check_preserves.** → `verified_preserves_regs` uses
a THROWAWAY sink; the user-facing check_preserves runs once (proc.rs:136). No dup.

**R4 — B hides a real clobber by over-subtracting.** → only §5-VERIFIED regs subtract (∅ on
any preserves error, incl. preserves/clobbers/out overlap). Identical to the closure's trusted
subtraction. Sound.

**R5 (the empirical watch-point) — A grows DecompressBlock's `local_writes` → a CALLER newly
fires.** The closure propagates `local_writes` (not declared clobbers). IF any reg were written
ONLY by the movem burst and nowhere else in DecompressBlock, adding it would propagate to
callers → a new transitive firing. That firing would be a real CATCH (a genuine clobber the
detector hole hid), not a regression — but it must be adjudicated, not assumed away. → Predict
0 (DecompressBlock writes d0-d6/a2 throughout its body). VERIFY in the before/after snapshot;
if a firing appears, STOP and adjudicate (catch vs. mislabel) per §6.

**R6 — `starts_with("db")` over-matches.** → only dbcc-family lowers to a `db*` mnemonic; the
push is further gated on `ops.first()` being a `Reg`. Matches existing convention. Acceptable.

**Verdict:** design survives. R1 is the load-bearing decision (documented boundary); R5 is the
one thing the snapshot must actually check, not predict.

## Build plan (bisectable, brief §5)
- Commit A: detector growth + comment updates + tests (dbcc-counter fires; non-stack movem-load
  fires; `(sp)+` defensive-over-save does NOT fire d4-d7 [rider 2]; dbf does not satisfy
  out(dN) [rider 3]; runtime-loop preserve guard still red).
- Commit B: check_clobbers verified-preserves subtraction + tests (5 verified procs no longer
  fire; declared-but-unverifiable preserves STILL fires [rider 1]; push-without-restore /
  restore-into-different-reg still fire; G3 five green).
- Snapshot: full before/after across closure / D1b / D1c / §5 / §6 / dead-save / out-verify.
- Gates: byte-neutral vs canonical (plain 3aa43cb6/420749, debug ce0e83a6/428768, seeded
  worktree @ ae1de4d); strict from tips (baseline 2434/0/1); clippy clean on touched files.
