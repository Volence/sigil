# 2026-08-04 — SESSION HANDOFF (read this first on resume)

Written at the end of a long overseer run so the next session can pick up cold.
Everything below is verified state, not recollection.

---

## §1 — Repo state RIGHT NOW

| | commit | note |
|---|---|---|
| sigil master | `438147c4` | B′-0 + B′-0c merged, ledger commits on top |
| aeon master | `b96051a` | B′-0 merge; no aeon work since |
| refreeze chain | entry 38 (`item28-bg-guard`) | moved 32→38 during the session |

**Goldens at chain 38** (the CURRENT byte bar — re-derive anyway, never trust a
quoted CRC; these moved twice in one night and `demo.debug` changed SIZE):
s4 `3879b953`/384048 · s4.debug `2623ee7f`/423383 · demo `f7a93a04`/70180 ·
demo.debug `e3243cbb`/93943.

**⚠ BOTH MAIN CHECKOUTS ARE DIRTY** — the concurrent engine session holds
uncommitted WIP:
- sigil: `native.rs`, `repin.rs`, `repin.toml`, `m1c_root.asm`,
  `error_handler_port.rs`, `native_full_rom.rs`, `vectors_port.rs`,
  `m1c_vector_table.rs`
- aeon: `vectors.emp`, `null_interrupt.emp` (deleted), `docs/DEFERRED_WORK.md`,
  `games/demo/{game_root.asm,map.toml}`

`repin.toml` + `native.rs` together mark it a BYTE-CHANGING parcel in flight.
Nothing may merge until both trees are clean. Local masters are AHEAD of
`origin/master` and deliberately UNPUSHED (they carry the engine session's work;
pushing it is not a porter/overseer call).

Neither repo has any leftover worktree from this session except the D-batch pair
(§3). A stray detached worktree `scratchpad/master-baseline` may still exist —
safe to remove.

## §2 — Merged this session

- **B′-0** (sigil `4a21063a`, aeon `b96051a`) — `out(rN if cc)` may coexist with
  `clobbers(rN)`. Packet: `notes/2026-08-04-bprime-0-condout.md`.
- **B′-0c** (sigil `6632a8d3`) — the closure-soundness batch, six fixes. Packet:
  `notes/2026-08-04-bprime-0c-closure-soundness.md`. Headline: `contract_type_bound`
  no longer drops `sig.out`, closing a hole whose failure mode was the dead-save
  worklist advising DELETION of a load-bearing save.

## §3 — D-batch: FINISHED, GATE-GREEN, DELIBERATELY NOT MERGED

Branch `d-batch` (sigil only; **aeon side has zero commits**), tip `72e1974c`,
5 commits, worktrees at `scratchpad/sigil-d` + `scratchpad/aeon-d`.
Packet: `notes/2026-08-04-d-batch.md`.

Countersigned by overseer own-run: **strict 3054/0/4 across 307 binaries, exit
0**; **byte-identical ×4** at chain 38; range fix verified present at
`eval/mod.rs:1347`.

Held ONLY because the engine session resumed and dirtied both checkouts.

**To resume (do exactly this):**
1. Confirm BOTH main checkouts are clean (`git status --short` — empty) AND no
   `cargo`/`rustc`/`build.sh` processes. Tree cleanliness is the check that
   matters; process/commit-recency signals both miss uncommitted WIP.
2. `git rebase master` the `d-batch` branch (expect a clean ledger merge).
3. Re-prove ×4 + strict own-run against the THEN-current goldens.
4. Merge sigil only. Do not create an aeon merge — there is nothing to merge.

## §4 — Next lanes, in order

1. **Merge D-batch** (above).
2. **B′-0b — the survives-claim verifier.** Spec: `specs/2026-08-04-contract-delta-spec.md`
   §7.2. Now UNBLOCKED (0c landed). This closes the hole B′-0 itself opened:
   `out(rN if cc)` with rN ∉ `clobbers` is a normative "rN survives the ¬cc
   edges" claim that NOTHING currently verifies. Error tier
   (`[proc.out-cond-survives-unverifiable]`), affordable precisely because B′-0
   created the free honest downgrade (add rN to `clobbers`). Bundles the
   `tile_cache.emp:130` honest flip and requires `AllocEffect` as a PASSING
   witness.
3. **T1 (RAM-map report) + T2 (parametric memory_hash — CONFIRM-first against
   the oracle tree).** Plan §3b. Untouched.
4. **B′-1 — generalized contexts** (`context`/`with`/`requires`). Spec §7.4 says
   after 0b+0c. **This is the memory-safety headline of the whole arc** and is
   still unbuilt.
5. Track C (niche-sentinel Option) — spec exists, unstarted.

## §5 — OPEN, needs a decision

- **THE INVISIBLE WARNING TIER — highest-value open item.** `native::build_emp`
  filters diagnostics for `Level::Error` and DROPS the rest; `run_build_native`
  never renders them. **156 warnings are firing on the corpus unseen** (51
  `[proc.sr-undeclared]`, 10 `[proc.clobber-undeclared]`, 6
  `[proc.undeclared-fallthrough]`, 3 `[proc.out-unwritten]`). Compounding: every
  new warn-tier lint is born invisible — D1 and D3 were, this session. My
  recommendation on record: this probably outranks B′-1 for near-term value,
  because it costs little and may surface defects we already detect and never
  show anyone. Needs a Volence/Fable call on where it slots.
- **The cc-precision hole** (ledgered, B′-0c): `Contract::out_cond` is a bare
  register set, so `out(a1 if ne)` satisfies a bound demanding `out(a1 if eq)` —
  opposite edges. Fix needs `(reg, cc)` pairs, which is spec surface (Fable).
- **`cargo clippy -D warnings` fails on master**, pre-existing toolchain drift
  (`sigil-ir/src/symbols.rs:55`), masking an unknown backlog. Own parcel.

## §6 — Rules this session paid for (do not re-learn)

1. **A quiet window is a snapshot, not a lease.** Re-check TREE CLEANLINESS
   immediately before every merge.
2. **Never point `AEON_DIR` at a checkout you do not own.** A baseline run
   against the main aeon tree read the engine session's in-progress
   `s4.debug.bin` (`b45a553a`/423354 vs golden `2623ee7f`/423383) and failed on
   a 4-byte symbol shift that looked like a real code defect. `animate_port`
   reads its reference ROM out of `AEON_DIR`. Use a worktree whose four shapes
   YOU built this session.
3. **Never pipe cargo through `tail`/`head`.** It truncates the log AND returns
   the pipe's exit code, not cargo's. Full capture, failures-first, explicit
   counts.
4. **Targeted `-p` test runs are not a substitute for the workspace build** when
   a test file changes across crate boundaries.
5. **Verify each plan item's current state before building it.** Five items this
   session were already done or had no referent (Track 0, D6, T4, most of Track
   B, D2's "existing test").
6. **Lens panels are load-bearing, not ceremony.** They caught a blocker in each
   parcel — both times on work that had already passed every gate green.
7. **When a spec and a ledger row it supersedes disagree, that disagreement is
   the signal.** B′-0c's blocker came from building the row, not the spec.
