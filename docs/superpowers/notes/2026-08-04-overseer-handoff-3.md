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
| **9 `sr` true positives** | aeon only: `irq.emp`, `dma_queue.emp`, `release_fault.emp`, `ojz_scroll_test.emp` | nothing |
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

1. **The 9 `sr` true positives** — the warning tier's first real customers. Four
   procs hand-write `move.w sr,-(sp)` … `move.w (sp)+,sr` and want
   `preserves(sr)`; `release_fault`'s terminal mask wants `clobbers(sr)`. The
   diagnostic already names the right answer. **Re-measure first:** B′-1 deleted
   `sr_masked` and moved its push/pop into `ints_off`'s acquire/release, so the
   set may have shifted (chain-44 `sonic4` plain shows `proc.sr-undeclared 8`,
   against 9 measured across all seven shapes at chain 43). Small, aeon-only.
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
