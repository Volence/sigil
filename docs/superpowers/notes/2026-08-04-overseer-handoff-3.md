# 2026-08-04 — OVERSEER HANDOFF #3: verified state + the PARALLEL-PARCEL playbook

Supersedes handoff #1 (`2026-08-04-session-handoff.md`) and #2
(`2026-08-04-overseer-handoff-2.md`) for anything they disagree on. Everything
here was verified at the time of writing. **The engine session moved both masters
five times during the session that produced this file. Re-derive before acting.**

---

## §1 — State at handoff

| | commit | note |
|---|---|---|
| sigil master | `9d212559` | refreeze chain **44**, tip `b-jumps` |
| aeon master | `0e1f32c` | B′-1 adoption merged |

Both checkouts CLEAN, no builds running, nothing pushed (local masters carry the
engine session's unpushed work — pushing is not a porter/overseer call).

**The byte bar is SEVEN targets:** `s4`, `s4.debug`, `demo`, `demo.debug`,
`config_a`, `config_b`, `lean`. It went 4 → 6 → 7 over the campaign and grew
DURING a parcel. Never assume a count; derive the list from
`crates/sigil-harness/golden/` in your own worktree and compare with `cmp`.

### Merged this session (six parcels)

D-batch `2287eabc` · B′-0b `4993825e`/`11da450` · warning tier `1a62f4b6` ·
path-mismatch narrowing `6d332f5b` · **B′-1 generalized contexts**
`f1de60df`/`0e1f32c` · packet-status hygiene `9d212559`.

---

## §2 — CAN PARCELS RUN IN PARALLEL? Yes — with three hard limits

Measured on this machine: **16 cores**, 60 GB RAM (~31 GB free), root disk
**656 GB free**, `/tmp` **tmpfs with 24 GB** (RAM-BACKED).

### Limit 1 — put worktrees in `.worktrees/`, NOT `/tmp`

**Both repos already have a gitignored `.worktrees/` convention** ("git worktrees
for isolated feature work", `sigil/.gitignore:3`). Use it. A warm sigil `target/`
reaches **40 GB**; `/tmp` is tmpfs, so a target dir there is eaten out of RAM.
One parcel in `/tmp` is survivable (this session did it repeatedly). Two or three
is not. `.worktrees/` sits on the 656 GB root disk.

```
git -C sigil worktree add -b <branch> .worktrees/<name> master
git -C aeon  worktree add -b <branch> .worktrees/<name> master
```

### Limit 2 — the strict suite is the bottleneck, so stagger it

`cargo test --workspace --release` saturates all 16 cores for roughly 45–75
minutes. Two concurrent runs roughly halve each; three thrash. **Two parcels in
their build phase is the sweet spot**; a third is fine only if it is tiny or
aeon-only. Stagger deliberately so two suites do not start together.

### Limit 3 — MERGES ARE STRICTLY SEQUENTIAL, and each one re-proves

This is not negotiable and it is where parallelism's cost actually lands. Every
parcel merges by: rebase onto the THEN-current master → rebuild → re-prove all
seven targets + repin + refreeze + full strict → merge. Masters move constantly,
so **N parallel parcels means N full re-proves**, and each later parcel rebases
onto the earlier ones' merges.

Parallelism therefore buys wall-clock on the BUILD phase and buys nothing on the
MERGE phase. Two or three lanes is worth it. Five is not.

### De-conflicting: pick lanes with disjoint file sets

Recommended pairing, lowest conflict risk first:

| lane | repo footprint | conflicts with |
|---|---|---|
| **`sr` lane** (see §7) | sigil `lower/proc.rs` sr-lint + `warn_tier_corpus.rs` baseline; aeon `dma_queue.emp` | nothing |
| **B′-2 stack-delta** | sigil `preserves.rs` + new checker; aeon adoption LATER | B′-4 lightly (`corpus_contracts.rs`) |
| **B′-4 report consolidation** | sigil `emp_contracts.rs`, `corpus_contracts.rs` | B′-2 lightly |
| clippy drift | SIX crates | **everything — run solo, last** |

**Run the `sr` lane concurrently with B′-2.** If B′-2 wants aeon adoption, tell it
to hold that until the `sr` lane merges, or the two collide in `irq.emp`.

---

## §3 — Lanes, verified open (each was re-checked, not taken from the plan)

**FIVE plan items this session were found already done or with no referent**
(T1, T2, D6, most of Track B, B′-0b's spec rider). Verify before dispatching —
it has paid off every single time.

1. **The `sr` lane — SEE §7, which supersedes this row.** It was re-measured at
   chain 44 and is NOT the warning-tier packet's "nine true positives wanting
   `preserves(sr)`". It is two classes wanting opposite treatment, it needs BOTH
   repos, and the lint half should land before the corpus half.
2. **B′-2 — stack-delta** (delta spec §3). Verified genuinely absent (no
   `stack_delta` / `[stack.*]` anywhere). Reuses `preserves.rs`' delta tracking —
   the extend-don't-replace pattern that has now worked three times. **My pick for
   the next substantive lane.**
3. **B′-3 — cycle budgets** (§4). `@budget(cycles: N)` + `@cycles_exact` are
   absent, BUT the substrate partly exists: `z80_cycles`, `pad_to_cycles`, and a
   comptime `ensure(cycles(L1,L2) == N)` with `[cycles.ambiguous-branch]` all
   ship. The 68k table is new work and belongs in the ISA crate. Spec calls this
   the largest B′ parcel and says it may split 68k/Z80.
4. **B′-4 — report consolidation** (§5). `--report contracts` extending
   `emp_contracts` (211 lines today).
5. **Track C — niche-sentinel Option.** Spec exists, unstarted.
6. **The 12 `module.path-mismatch` survivors.** Three want a FILE rename
   (`engine.s4lz` in `s4lz_decompress.emp`, `engine.zx0`,
   `engine.compression_vectors`). Eight are generated OJZ modules carrying an
   `_act1` the per-act directory already expresses — **wait for a second act to
   exist** before ruling.
7. **clippy `-D warnings`** — 62 workspace findings, pre-existing toolchain drift
   from `sigil-ir/src/symbols.rs:55`. Touches six crates: solo, last.
8. **~15 stale packet status headers** from 08-01/02 (ledgered). Structural fix
   argued in the ledger: packets should stop carrying a merge-state claim at all,
   leaving the campaign log the single authority.

---

## §4 — The standing rules (every one was paid for)

1. **A quiet window is a snapshot, not a lease.** Re-check TREE CLEANLINESS
   immediately before every merge. Process-activity and commit-recency both miss
   uncommitted WIP.
2. **`AEON_DIR` must be YOUR worktree at the RIGHT COMMIT.** A stale commit is as
   damaging as a tree you don't own — `animate_port` reads its reference ROM from
   `AEON_DIR`, so a chain-38 tree against chain-40 goldens produces four failures
   that look exactly like a real code defect.
3. **A fresh aeon worktree needs TWO gitignored seeds:**
   `games/sonic4/data/editor/` (~8 MB) AND `engine/debug/generated/` (the MD
   Debugger island — NOT regenerated by `build.sh`; only
   `engine/sound/generated/` is). Enumerate with `git status --ignored --short |
   grep '^!!'` rather than trusting any list — it has grown twice. **Then prove
   the seed**: build the canonical shapes and `cmp` against golden BEFORE any
   edit. A byte-identical baseline is the only evidence the seed is complete.
4. **Never pipe cargo through `tail`/`head`.** It truncates the log AND returns
   the pipe's exit code, not cargo's. Full capture, failures-first, explicit
   counts.
5. **Chase every test-count delta; never wave one through.** Method:
   `git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'` at both commits,
   then diff per-file counts to NAME the function. Two minutes, and it names the
   culprit instead of merely reconciling a number. `passed + ignored` must equal
   the branch's own `#[test]` total or something is being silently skipped.
   (A result-LINE count can move on its own — that is binary packaging, and the
   `#[test]` arithmetic is what distinguishes it from a skipped binary.)
6. **Verify each plan item's current state before building it.** Five were stale
   this session.
7. **Lens panels are load-bearing, not ceremony.** They caught a blocker in every
   parcel, always on work already gate-green — and TWICE overturned the central
   design ruling, including one of mine.
8. **Byte bar = seven targets, `cmp`, in `capture_goldens.sh` order.** `config_a`
   writes `s4.debug.bin`; `config_b` AND `lean` both write `s4.bin`. Out of order
   they clobber the canonical reference. Rebuild canonical afterwards.
9. Never `git add -u` in a shared checkout; explicit paths, check the branch.

---

## §5 — Open, needs a Volence/Fable ruling

- **The cc-precision hole** (ledgered at B′-0c): `Contract::out_cond` is a bare
  register set, so `out(a1 if ne)` satisfies a bound demanding `out(a1 if eq)`.
  **B′-1 added `ProcDecl::cond_out_pairs(rf)`, which is plausibly the `(reg, cc)`
  primitive this wants — re-check the hole against it before speccing a fix.**
- The 8 act-suffixed path-mismatch survivors (needs a second act).
- Whether packets should carry a merge-state claim at all (§3 item 8).

## §6 — Honest gaps left by B′-1 (all ledgered, none blocking)

`requires(z80_stopped)` has **zero corpus adopters** outside tests. The `requires`
chain measures 2 deep, not 3 — `VInt_Level` is reached only through an indirect
dispatch that cannot carry a context bound. The corpus walk's no-`-D` shape does
not cover the three widest brackets. Four other worklists remain in-tree (the
"one worklist" claim was corrected by the panel).

---

## §7 — THE `sr` LANE WAS RE-MEASURED AT CHAIN 44 AND IT IS NOT WHAT THE PACKET SAID

Measured own-run, `SIGIL_WARNINGS=full`, chain 44, AFTER B′-1 merged. The
warning-tier packet's "nine true positives in four procs, all wanting
`preserves(sr)`" is **superseded**. The `sonic4` plain shape now fires 8, in two
CLASSES that want opposite treatment:

**(a) `QueueDMA_Deferrable` — 4 firings, `dma_queue.emp:106,137,145,181`.**
Genuine hand-written `sr` save/mask/restore that B′-1 did NOT convert. The right
fix is probably NOT `preserves(sr)` — it is to adopt `with ints_off { }`, which
is what B′-1 built. Confirm the bracket fits the control flow first (there are
four sites in one proc, which may be why it was skipped).

**(b) `Parallax_Update` and `GameState_OJZScroll_Init` — 4 firings, all pointing
at `irq.emp:37` and `irq.emp:40`. This is a NEW FALSE-POSITIVE CLASS THAT B′-1
CREATED.** Those two lines are the `ints_off` CONTEXT's own acquire/release asm.
The bracket inlines them into each consumer, and `proc.sr-undeclared` charges the
write to the CONSUMING proc — which has no way to declare it, because the `sr`
traffic belongs to the context, not to the proc. Every future `with ints_off`
adopter will inherit this. **The fix is in the lint, not the corpus:** a write
originating inside a context's acquire/release region is declared BY THE CONTEXT
and must not be charged to the consumer. The `ContextMark` items B′-1 already
plants (`Enter` / `AcquireEnd` / `BodyEnd` / `Exit`) give the exact ranges needed
to identify it.

That is the good kind of finding — B′-1's own adoption produced it, the warning
tier made it visible, and it was caught by re-measuring rather than by trusting a
four-hour-old packet. **It also means this lane is no longer trivial**: it is one
lint fix plus one corpus adoption, and the lint half should land first so the
corpus half can be measured honestly.

The DEBUG shape fires **52** `proc.sr-undeclared` (vs 8 plain) — the debug-only
procs are a much larger surface and have never been looked at. Scope the lane to
the plain shape's two classes first and report the debug number separately.

## §8 — READY TO GO: worktrees built, seeded, and baseline-proven

Both pairs live in the `.worktrees/` convention on the root disk (NOT `/tmp`):

| lane | sigil | aeon | branch |
|---|---|---|---|
| `sr` | `sigil/.worktrees/sr` | `aeon/.worktrees/sr` | `sr-contracts` |
| B′-2 | `sigil/.worktrees/b2` | `aeon/.worktrees/b2` | `bprime-2` |

Each has: release binaries built, both gitignored aeon seeds rsync'd, and a
**measured byte-identical baseline across all seven targets before any edit**.
Both branch from sigil `21f5aef7` / aeon `0e1f32c`.

Note the `sr` lane needs BOTH repos, not just aeon: retiring `sr` firings changes
the firing lint-id set, so `crates/sigil-cli/tests/warn_tier_corpus.rs`'s frozen
baseline must be updated deliberately in the same parcel.

---

## §9 — MERGING A TWO-REPO PARCEL INVALIDATES EVERY OTHER LANE'S AEON WORKTREE

Added 2026-08-05, paid for by the overseer, not a porter.

`define-gates` merged into aeon master. The `edge-split` lane's aeon worktree
stayed at the pre-merge commit, and its re-prove ran against a corpus missing
the five clobber declarations that parcel had just added. Two ERROR-tier gates
failed, naming all five sites and a missing probe anchor — **failures that look
exactly like a real code defect in the lane under test**, which is precisely
what §4 rule 2 warns about. The lane's own code was never implicated.

**THE STEP, and it belongs to whoever runs the merge:** after ANY aeon merge,
refresh the aeon worktree of EVERY lane still in flight before its next gate
run. Nothing enforces this — `git worktree list` shows the path, not whether
the tree is current, and a lane that committed nothing to aeon looks identical
to one that is simply stale.

```
for w in .worktrees/*; do git -C "$w" rev-parse --short HEAD; done   # vs master
git -C .worktrees/<lane> reset --hard master    # safe iff that lane has 0 own aeon commits
```

Rule 2 said "YOUR worktree at the RIGHT COMMIT" and the right commit was
already understood to move when the ENGINE session pushed. **It also moves when
YOUR OWN OTHER LANE MERGES** — which is new with parallel parcels, and is the
form that actually caught me, one merge after I quoted rule 2 into four briefs.

The good half: the gate `define-gates` had just built is what caught it. Its
first real outing was against the overseer, on an unfixed corpus, and it named
every site rather than failing vaguely. Nobody staged that probe.
