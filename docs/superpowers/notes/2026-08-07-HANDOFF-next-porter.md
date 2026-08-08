# HANDOFF — 2026-08-07 EOD, mid-arc, session ended on usage

You are picking up the contract-verification arc. Everything below was verified
at handoff time, not remembered. **Re-derive anything you are about to build on
— that rule produced every catch of the last session, in both directions,
including against the reviewer and against my own earlier claims in the same
session.** Commands to re-derive are given inline.

## FIRST — read the boilerplate

`sigil/docs/superpowers/notes/porter-brief-boilerplate.md`, in full. Two rules in
it were paid for twice each, and I nearly paid for one a third time (see TRAPS).

## STATE (re-derive: `git -C <repo> log --oneline -1` + `git worktree list`)

| | commit | unpushed |
|---|---|---|
| sigil master | `3e0824b1` | **11** |
| aeon master | `d8c93d7` | 0 |

- **origin/sigil is at `fe7a2b73`** — 11 commits behind local master.
  **PUSH IS VOLENCE'S GATE. It was not requested and not done. Do not push.**
- aeon's working tree has 5 dirty entries (`M .gitignore` + 4 untracked
  `docs/research/2026-08-07-*`) belonging to a CONCURRENT SESSION. Leave them.
  Never `git add -u` in either main checkout.
- Golden chain 53, tip `slide-fixture`, seven targets. `refreeze --check` OK.
- Strict on master: **3517 / 0 / 4 = 3521**, and 3521 == master's own `#[test]`
  total. clippy `--workspace --release --all-targets -- -D warnings` CLEAN.

### ⚠ WORKTREE NAMING TRAP

`sigil/.worktrees/collision` **currently holds the `triage` branch**, not
`collision`. The `collision` branch (`9d07ac3c`) is merged into master and done.
`aeon/.worktrees/collision` is still on `collision` == `d8c93d7` == aeon master,
clean. Check `git rev-parse --abbrev-ref HEAD` before assuming which lane you are
standing in.

## WHAT LANDED

**Merged: `collision` → sigil master (`3e0824b1`), sigil-only lane.**

1. The Load_Object contradiction **does not exist**. `out(a1)` is verified on all
   seven shapes via the advertised edge-sensitive conditional-out credit. The
   firing that prompted the question is **D1c's**, a different and already
   adjudicated lint over the same (proc, callee, reg) triple. Now witnessed by
   `corpus_conditional_callee_out_is_credited_edge_sensitively`.
2. **A soundness defect in an earlier commit of that same lane was found and
   fixed**: the `falls_into` exemption DROPPED the out obligation instead of
   transferring it, so a proc whose only exit is a fall-off verified every
   declared out on zero evidence. Now charges the successor's verified out,
   mirroring the `TailOut` arm. Residue correctly returned 29 → **30**; the merge
   commit records that the branch supersedes its own earlier claim.

**Open lane: `triage` @ `bee0eb37`** — docs-only, based on merged master, NOT
merged. `git diff master..HEAD -- crates/` is empty (proven; both measurement
probes were reverted).

## THE QUEUE — order is Fable-ratified, do not reorder

Item 1 is DONE. Full census: `docs/superpowers/notes/2026-08-07-out-residue-fixpoint-census.md`.

**30 residue rows, four causes** (measured by two reverted perturbations, each
read as a SET DIFF, never a count):

| cause | rows | item |
|---|---|---|
| width gap (sub-width production) | 15 | item 2 |
| `probe_core` d1/d2 unproduced | 8 | item 3 |
| `S4LZ_Decompress::a1` chain | 3 | highest leverage |
| `DrawRings` / `InsertSpriteMasks` | 4 | unexamined |

**2. Width survey + `out(dN: u8/u16)` adoption.** Design is RULED — a type, not a
width suffix; bare `out(dN)` keeps meaning 32 bits, no migration; composes as
`out(d0: u8 if ne)`. Spelling asymmetry (`preserves` = facet, `out` = type) is
deliberate and must be written into the design note. RMW nuance decided by
measurement: start with "defining writes produce, RMW does not"; extend only if
real sites still fire, and never let RMW alone count as production from nothing.
**ADOPTION BAR: each site adopts the type its body actually produces, with a
per-site caller read-width sweep — list every caller and the width it reads. A
caller reading wider than the adopted type is a REAL FINDING; escalate, don't
paper.** No caller-side width check exists; the manual sweep is what makes
adoption honest. Scope: **15 rows across 13 procs**, named in the census.

**3. The `.cl_hanging` → `.full_back` oracle trace** (collision step 2). Still
owed, independent of item 2. The census sharpened it before it starts: the four
`Collision_Probe*` procs are one `probe_core` macro body
(`aeon/games/sonic4/player/player_sensors.emp:~170-232`) stamped four times.
`d0` is a width gap in all four (`ext.w d0`); `d1`/`d2` are NOT. **`d2` is
written in exactly one place — `.cl_air`'s `moveq #0, d2` — and neither
`.cl_hanging` (ends `moveq #16, d0` / `rts`) nor the partial-height `rts` writes
it.** Decide real bug vs benign-by-downstream-filter vs contract-only. Worth 8
rows. The `out(rN if rM != #K)` language ask stays demand-gated behind this.
**OPEN SUB-QUESTION, deliberately not rounded off: `d1`'s failure is unexplained.**
It is written `.b` at the attr/angle read, which the width-off probe should have
credited, so some path must reach a return without passing that write. I did not
find it. Do not assume it is the same story as `d2`.

**`S4LZ_Decompress::a1` is the single highest-leverage row.** It holds THREE open
(itself + `Art_Decompress::a1` + `S4LZ_DecompressDict::a1`). **When it closes,
re-run the site-2 `falls_into` plumbing mutant and confirm it goes RED** — see
the dormant-guard row in the ledger. If it stays green, that is a finding, not a
footnote.

**Also still open** (ledgered, latent, verified site-by-site — not urgent, not
forgotten): `flag_check::abandons_flag` returns true on `Return | FallOff` with
no `falls_into`; `z80_preserves`' tail arm consults `falls_into` but not
`noreturn`; the per-file `[proc.out-unwritten]` exemption is proc-wide and is the
ONLY out gate that ever sees Z80. Lane C is still HELD with ratified redesign
requirements (see its packet).

## TRAPS THAT COST REAL TIME THIS SESSION

- **The shell cwd RESETS to the main checkout between tool calls.** A build, a
  test run, and a clippy run executed against master unnoticed. A green gate from
  the wrong tree is worthless and I briefly believed one. **Prefix every command
  with an explicit `cd` and print `pwd`.**
- **A stale test binary in the main checkout's `target/` reported residue 29 where
  the truth is 30**, printing "Finished in 0.02s". **A suspiciously fast gate is a
  PROVENANCE question before it is a result** — ask which tree and which binary
  before reading the number.
- **`git checkout -- <file>` is FORBIDDEN in a lane worktree.** I used
  `git checkout HEAD -- <file>` once to revert a probe. It was safe (verified the
  lane had zero uncommitted work first) but "I checked it's safe" is exactly how
  this fires the second time. Revert probes by line-targeted string-replace and
  prove it with `git diff`.
- Strict runs: streams separated (`> out 2> err`), never `2>&1`. Failures-first.
  Closing arithmetic: passed + ignored == the branch's own `#[test]` total.
- `capture_goldens.sh` needs `SIGIL_EMIT` **and** `SIGIL_BUILD` exported. Note the
  seven byte gates also run INSIDE the strict suite (`native_full_sonic4_*`,
  `*_anchor_matches_golden`) — verify they appear in your run rather than
  assuming.
- A lens panel is dispatched only against a CLEAN worktree with the review SHA
  named. A dirty-tree panel review is VOID.

## STANDING BARS RATIFIED THIS SESSION

- **A cited precedent transfers only with its soundness argument.** The
  `falls_into` defect entered UPSTREAM of the porter, in a ruling that prescribed
  `check_stack_balance`'s signature without the argument that makes dropping the
  charge legal there and illegal here. Applies to both sides of the relay.
- **"Name the mutant this test is supposed to catch, then run it."** The
  operational form of "does this assert the CONTRACT or the current BEHAVIOUR?".
  It caught the vacuous witness, the vacuity-pinning test, and the untested
  plumbing. Third instance of the test-pins-the-hazard family.
- Read measurements as SET DIFFS, not counts — the 15/15 width split and the
  28/2 root split both came from diffs; counts alone would have hidden which rows
  moved.

## AUTHORITATIVE RECORDS

- `docs/superpowers/notes/campaign-gap-ledger.md` — tail is newest, every row
  carries a kill condition. **Resolve rows by SUBSTANCE (grep the claim), never
  by a row number you were handed.**
- `docs/superpowers/notes/2026-08-07-collision-step1c-packet.md` — the merged lane.
- `docs/superpowers/notes/2026-08-07-out-residue-fixpoint-census.md` — item 1.
- `docs/superpowers/specs/2026-08-03-contract-unification-spec.md` §6.1 — the
  two-tier map, ratchet semantics, three-bucket taxonomy.
- `crates/sigil-harness/src/contract_baseline.rs` — the ONE baseline copy, read by
  both the build gate and the CI gates.

**Do not merge or push without Volence's explicit go-ahead. He gates both, every
time.** Fable reviews and rules; his MERGE GO for `collision` was given and
consumed. `triage` has no such ruling yet.
