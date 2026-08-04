# 2026-08-04 — OVERSEER HANDOFF #2 (read after `2026-08-04-session-handoff.md`)

Written at the end of the follow-on overseer run. Everything below was verified
at the time of writing and **all of it re-goes-stale the moment the engine
session commits** — which it did twice during this run. Re-derive before acting.

---

## §1 — What this run did

1. **D-batch MERGED** — sigil master `2287eabc`. aeon got no merge (the branch
   carried zero aeon commits, as its packet said). Packet:
   `notes/2026-08-04-d-batch.md`, its bar tables corrected to chain 40 first.
2. **B′-0b BUILT, gate-green, NOT MERGED** — branch `bprime-0b` in both repos.
   Packet: `notes/2026-08-04-bprime-0b-survives-verifier.md`.

## §2 — Two traps this run paid for (new; the first handoff's rules still hold)

1. **`AEON_DIR` at the wrong COMMIT is as damaging as the wrong TREE.** The aeon
   `d-batch` worktree was parked at `b96051a` (chain-38 era) while the goldens
   had moved to chain 40. Building the byte bar there would have compared
   chain-38 ROMs against chain-40 goldens: four failures that look exactly like a
   real code defect. Fast-forward the aeon worktree to aeon master BEFORE any
   byte bar. Generalises handoff-1 §6 rule 2.
2. **Chase a test-count delta; never wave one through.** D-batch's strict run
   came in at 3053 against a packet claiming 3054. The −1 was master's own —
   `64114c3e` (item 29 part 4) deleted `error_handler_region_matches_reference`
   when `error_handler` went DEBUG-only. Method that settled it in two minutes:
   `git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'` at both commits,
   then `diff` the per-file counts to name the function. Cheaper than a baseline
   suite run and it names the culprit instead of just reconciling a number.

## §3 — B′-0b: what it is, and the one decision that got REVERSED

`[proc.out-cond-survives-unverifiable]`, error tier. A cond-out register absent
from `clobbers` is a claim it survives the ¬cc exits; this verifies it, reusing
`preserves::verify_preserved` via a new `ReturnScope` rather than forking a
second proof, and reusing `out_verify`'s flag lattice (extracted to `flags_after`)
rather than forking a second cc classifier.

**The overseer recommended firing at ⊤ (unanalyzable cc). That was WRONG and the
porter reversed it — correctly.** Recorded because the reasoning generalises:

- The corpus measurement said the two polarities were identical. **True and
  worthless** — every return in both claim sites ends in the `moveq #0`/`#1`
  Z-result convention, so the corpus contains zero instances of the condition the
  policies differ on. A corpus-only measurement of a ⊤ policy on a ⊤-free corpus
  proves nothing. The lens panel built the probe the corpus could not be.
- On *honest* bodies, firing at ⊤ rejects at error tier whenever the success path
  uses `clr.w d0`, `move.w #0,d0`, a store, or **any call** — each sends flags to
  ⊤, dragging the cc-**success** return into the ¬cc set, where the register is
  written *by contract*.
- The `preserves` mirror does not hold. A `preserves` bailout means "your claim
  was checked and the proof gave up" → fire. A ⊤ cc means "I cannot tell whether
  you made a claim at this exit" → firing charges an obligation never incurred.
- The escape hatch is not free there either: on a true claim, `clobbers(rN)`
  publishes a FALSE statement to buy silence — the exact polarity §7.2 rejects
  when it turns down option (a).

Ruled: **⊤ does not obligate.** Cost is a bounded false negative, pinned by a
test documented to flip when `Flags::after` widens.

**The spec's rider in §7.2 is WRONG and the ledger is corrected:** the
`TileCache_FillRow` D1c false positive does NOT dissolve (21 firings before and
after the flip — a declared clobber is not an input to D1c; the FP is
edge-blindness in D1c's close), and B′-0c's `Load_Object @ AllocDynamic`
allowlist must NOT be retired.

## §4 — B′-0b's gates (proven at its base `2287eabc`, chain 40 — ALL NOW STALE)

Byte-identical ×4 · strict **3071 / 0 / 4** across 305 result lines, exit 0 ·
`repin --check` pins unchanged · `refreeze --check` OK chain 40 · clippy net −1.

The test arithmetic was independently confirmed by the overseer: base tree-wide
`#[test]` = 3057, branch = 3075, delta +18 ⇒ 3071 passed + 4 ignored. ✓

**Every one of these must be re-proven after the rebase.** See §5.

## §5 — TO MERGE B′-0b (the window was CLOSED when this was written)

At time of writing BOTH main checkouts are DIRTY with an active engine session
mid-parcel, and both masters moved during the porter's run:
sigil `2287eabc` → `0d924929` (refreeze chain 41 `mddbg-symbols`),
aeon `c424dfd` → `2987c24`.

1. Confirm both main checkouts are clean and no `cargo`/`build.sh` is running.
2. Fast-forward the **aeon** `bprime-0b` worktree to aeon master, and rebase the
   **sigil** `bprime-0b` onto sigil master. Both, in that order.
3. Re-prove every bar in §4 against the THEN-current goldens, re-derived from
   `crates/sigil-harness/golden/` in your own worktree. Update packet §7/§8
   in-place, exactly as D-batch's merge did.
4. Merge both repos (unlike D-batch, B′-0b HAS an aeon commit — the tile_cache
   flip).

**⚠ THE SHAPE COUNT MAY BE CHANGING FROM FOUR.** The engine session's in-flight
WIP includes an untracked `crates/sigil-harness/golden/lean.bin` +
`offcanonical_sizes/lean.txt` and edits to `native_full_rom.rs` / `native_rom.rs`
/ `capture_goldens.sh` / `refreeze.rs`. If a fifth `lean` shape lands, "byte
bar ×4" becomes ×5 and every packet's bar table (including B′-0b's) is
under-specified. Check before re-proving.

## §6 — Open, needs a Volence/Fable ruling

Two raised by the porter, both real:
- Should a cond-out proc with **no `clobbers` clause at all** escape the gate?
  It currently does, by inherited convention rather than by ruling.
- Is "error tier" satisfied by a per-file diagnostic plus a strict-suite corpus
  assertion, or must it be a hard build failure?

Still open from handoff #1, unchanged and still the overseer's pick for highest
near-term value:
- **THE INVISIBLE WARNING TIER.** `native::build_emp` filters for `Level::Error`
  and drops the rest; **156 warnings fire on the corpus unseen**. Every new
  warn-tier lint is born invisible — D1/D2/D3 all were, and they just merged.
- The cc-precision hole (`out_cond` is a bare register set, so `out(a1 if ne)`
  satisfies a bound demanding `out(a1 if eq)`). Needs `(reg, cc)` pairs — spec
  surface. **B′-0b added `ProcDecl::cond_out_pairs(rf)`, which is plausibly the
  primitive this wants; re-check the hole against it before speccing.**
- `cargo clippy -D warnings` fails on master (`sigil-ir/src/symbols.rs:55`).

## §7 — Next lanes after B′-0b merges

1. **B′-1 — generalized contexts** (`context`/`with`/`requires`), spec §7.4.
   The memory-safety headline of the arc, still unbuilt.
2. T1 (RAM-map report) + T2 (parametric memory_hash, CONFIRM-first against the
   oracle tree). Plan §3b, untouched.
3. Track C (niche-sentinel Option) — spec exists, unstarted.
