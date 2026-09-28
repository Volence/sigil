# INOUT-PROOF-INVOKE-HOOK: Stage 0 (reproduce and judge)

Queue row: `docs/QUEUE.md`, `### INOUT-PROOF-INVOKE-HOOK`. Sigil base: `a0cc4693`.
Aeon tree: detached probe copy `/home/volence/sonic_hacks/.aeon-inout-probe` at aeon `3d369bc7`.

## The answer, plain

1. **It reproduces.** With `Rings_AnimTick`'s body (and its `invoke Game.ring_frame`) inlined
   back into `DrawRings`, sigil at `a0cc4693` fires `[proc.inout-unverified]` for
   `DrawRings :: inout(d5)` and `inout(a4)`, reason "crosses an indirect or unknown callee".
   Both `./build.sh` and `DEBUG=1 ./build.sh` go red on it.
2. **It is the wrong answer: a checker gap, not a correct refusal.** The hook is declared
   `hook ring_frame (d1: u8) clobbers(d0-d4/a1-a2) = empty`, and sonic4 binds it to
   `RingArt_StreamFrame (d1: u8) clobbers(d0-d4/a1-a2)`. Neither names d5 or a4, and the bind
   pass already checks the bound proc against the hook's bound. The verifier never looks: it
   cannot see WHICH proc the call names, so it charges an unknown callee. The contract
   closure, in the same build, does resolve the same call site to `RingArt_StreamFrame`.
3. **Where:** `crates/sigil-frontend-emp/src/out_verify.rs:461`, `fn direct_target`. It accepts
   only a bare `CodeOperand::Sym`. A bound `invoke` lowers to `jsr (Sym).l`, an
   `CodeOperand::AbsSym` (`crates/sigil-frontend-emp/src/eval/asm.rs:1076`, `lower_invoke`), so
   `inout_transfer` (`out_verify.rs:1112`) passes `target = None` to
   `inout_call_disposition` (`out_verify.rs:1078`), which returns
   `InDisp::Broken(InBroke::UnknownCallee)` before it ever consults `effective_clobbers`, where
   `RingArt_StreamFrame` is a known key. The closure side reads the call through
   `flag_check::transfer_target_sym` (`flag_check.rs:678`), which does accept `AbsSym`; the two
   readers of "what does this call name" disagree.

**Fix shape (not implemented):** let `out_verify::direct_target` resolve an `AbsSym` target
too, with the same `$` local-label exclusion, so a bound `invoke` resolves exactly as the
closure resolves it (simplest: delegate to `transfer_target_sym` filtered on `$`, as
`corpus_contracts::call_target_sym` does). The disposition then comes from the bound proc's
effective clobbers, which the bind pass holds under the hook's declared bound. Nothing else
needs to change: a target absent from `effective_clobbers` (for instance an `assert` rail's
`jsr (MDDBG__...).l` equ boundary) still lands on `UnknownCallee`, so no refusal is weakened
for a callee the verifier cannot name. The same helper also feeds `out` crediting
(`out_verify.rs:490`, `:807`, `:1234`), which would then credit a bound proc's unconditional
outs at an `invoke`; that is the same fact the closure already uses. A design alternative is
to prove against the INTERFACE bound rather than the bound proc (a proof valid for every
game's binding); the verifier has no interface env today, and the bound-proc reading is
sound for each build because the bind pass enforces bound-proc within hook-bound.

## Evidence

### Probe tree

`git -C /home/volence/sonic_hacks/aeon worktree add --detach /home/volence/sonic_hacks/.aeon-inout-probe 3d369bc7`,
then `jbsr Rings_AnimTick` in `DrawRings` replaced by `Rings_AnimTick`'s body verbatim
(12 lines, `invoke Game.ring_frame` included). `Rings_AnimTick` itself left defined.

Sigil binary: `target-inout/release/sigil` built from this worktree at `a0cc4693`, clean
sources (md5 `a1c89748aab9b98acbe41b614bf5b23b` for the rebuild after the mutation below
was reverted; the first build, used for the four `build.sh` runs, was the same sources).

| run (in the probe tree) | result |
|---|---|
| `./build.sh` | rc=1. Pytest lane 3648 passed; expect-fail lane: sentinel failed on `./engine/objects/rings.emp:204:5: DrawRings :: inout(a4) (got 1, want 0) \| ... inout(d5) (got 1, want 0) \| error: contract closure gate FAILED.` |
| `NO_LINT=1 ./build.sh` (to reach the main sigil build) | rc=1. `error: [proc.inout-unverified] moved against the frozen baseline. NEW firings (...): ["DrawRings :: inout(a4) (got 1, want 0)", "DrawRings :: inout(d5) (got 1, want 0)"]` |
| `DEBUG=1 ./build.sh` | rc=1, same sentinel text as the plain run |
| `NO_LINT=1 DEBUG=1 ./build.sh` | rc=1, same two NEW firings at `rings.emp:204:5` |
| `sigil build --aeon . --native --report contracts` | `DrawRings inout(a4), \`a4\` crosses an indirect or unknown callee on a required exit path, nothing proves it survives` and the same for `d5` |

### Two-direction probe

"Fixed" = the sketch below applied on disk to `out_verify.rs:461` (one added match arm),
built to `target-inout/sigil-mutated` (md5 `b73e204a5a98dc4ac1dc2a777d0d19c0`), then reverted:

```
+        [CodeOperand::AbsSym { target, .. }] if !target.contains('$') => Some(target.as_str()),
```

| probe (probe tree, `--report contracts`) | sigil `a0cc4693` | sigil + fix sketch |
|---|---|---|
| EXCLUDE: stock hook bound `d0-d4/a1-a2`, bound proc unchanged | fires d5 and a4, unknown callee | **0 inout firings** (whole corpus) |
| INCLUDE: hook bound and `RingArt_StreamFrame` widened to `d0-d5/a1-a2`, plus a real `moveq #0, d5` as its first instruction | fires d5 and a4, unknown callee | fires **d5 only**: "`d5` is destroyed by a callee on a required exit path"; a4 verifies |

The include edits were reverted after the run; the probe tree keeps only the inlined
`rings.emp`. Under the fix sketch the whole-corpus `[proc.out-unverified]` count stayed 0
in both directions. The fix sketch was run only through `--report contracts` and the
minimal tests; no full `build.sh` and no sigil test suite were run with it.

### Minimal repro (committed)

`crates/sigil-frontend-emp/tests/inout_invoke_hook.rs`, target
`cargo test -p sigil-frontend-emp --test inout_invoke_hook`. It evaluates
`proc P (d5: u16) clobbers(d0-d4/a1-a2) inout(d5: u16) { invoke Game.tick; rts }` under an
`InterfaceEnv` binding `Game.tick` to `Bound`, and runs `verify_inout` with `Bound` a known
callee:

- `bound_invoke_emits_a_call_item`: the invoke emits exactly one item naming `Bound`
  (so the probe is not an empty hook trivially verifying).
- `bare_jsr_of_preserving_callee_verifies` (control): `jsr Bound`, same maps, verifies.
- `invoke_of_preserving_hook_is_charged_unknown` (THE GAP, pinned as current behaviour):
  `Bound` clobbers `d0-d4/a1-a2`; the invoke still fires, reason contains
  "indirect or unknown callee".
- `invoke_of_clobbering_hook_fires`: `Bound` clobbers `d0, d5`; must fire before and after a fix.

Red-first: with the one-line fix sketch on disk, 3 passed and
`invoke_of_preserving_hook_is_charged_unknown` FAILED
(`panicked at ...inout_invoke_hook.rs:103:21: GAP PINNED: the invoke of a d5-preserving hook
currently fires`); reverted, 4 passed. The fix lands by flipping that one test's assertion to
"verifies"; the include test stays as is.

## Out of scope, noticed

- The same Sym-only call-target reading exists in `calls.rs:52` (`direct_target`, feeding the
  D1b must-def / section-6 caller gates and the D1c live-clobbered walk, which treats an
  unresolved call as "kills" at `calls.rs:437`), `type_slice.rs:104`, and
  `z80_out_verify.rs:104`. At a bound `invoke` these see no target either. Their effect at
  such sites was not assessed.
- `tests/corpus_contracts.rs:254` says the corpus walk uses the EMPTY interface env so an
  `invoke` emits nothing there. In the aeon build the edge is live (the closure charges
  `RingArt_StreamFrame` into the invoker, and aeon's commit calls it a known clobber set
  through `Rings_AnimTick`), so that comment may be stale; not verified which entry point
  threads the env.
- aeon's `Rings_AnimTick` header (`engine/objects/rings.emp:162`) records this gap as its
  reason to exist; a fix makes that proc optional, which is aeon's call.

# The fix

Branch `worktree-agent-af87b9b8fbe5b5aa2`, on top of the stage-0 commit `dc9af1d4`.

## For the aeon lane, plain

sigil now reads a bound `invoke Game.hook` as a call to the proc the game binds, the same
way it reads a bare `jsr` of that proc. With `Rings_AnimTick`'s body inlined back into
`DrawRings` (the probe copy at aeon `3d369bc7`), `--report contracts` shows 0
`[proc.inout-unverified]` firings, and both `./build.sh` and `DEBUG=1 ./build.sh` finish with
rc=0. So the `Rings_AnimTick` split is no longer needed to get past the contract gates. Keeping
it is your call; it is correct either way. If you inline it, the proof that `d5`/`a4` survive
rests on the bound proc's declared clobbers (sonic4's `RingArt_StreamFrame`, which the bind pass
holds inside the hook's `clobbers(d0-d4/a1-a2)`), so a game that binds a proc touching `d5` or
`a4` will now fire as "destroyed by a callee" instead of "unknown callee".

## What changed

- `flag_check::direct_proc_target` is the one reading of "which proc does this direct call or
  tail name": a single bare `Sym`, or a single `AbsSym` (`jsr (Foo).l` / `.w`, the form a bound
  `invoke` lowers to). `$` local labels stay excluded.
- **SymOff stays excluded.** `jsr Item.field` targets `Item + offset`, an address inside
  `Item`, not its entry; `Item`'s contract (clobbers, outs, params, typed slots) describes a
  call to its entry and says nothing about entering it mid-body. Accepting it would let a
  mid-body entry borrow the whole proc's proof. It stays an unknown callee to these readers.
  (The closure's `transfer_target_sym` does include `SymOff`, charging `Item`'s clobbers to a
  `jsr Item.field`. Whether that is always an over-approximation was not assessed; booked below.)
- Every consumer uses the returned name only as a key into its contract maps; a map miss
  behaves exactly as before (unknown callee, nothing credited, conservative kill, all
  registers untyped). No refusal weakens for a callee the verifier cannot name, pinned by
  `invoke_of_uncontracted_bound_proc_is_unknown`.

## The sibling readers

| site | same gap at a bound invoke | action |
|---|---|---|
| `out_verify.rs` `direct_target` (inout disposition + out crediting at call, tail, `falls_into`) | yes, the reported defect | fixed, `7fb7d501` |
| `calls.rs:52` `direct_target` (D1b input-undefined, must-def and section-6 out credit, D1c live-clobbered + its kill walk) | yes: D1b skipped the site, no out credit, D1c skipped the site and treated it as a kill | fixed, `04c89bb0`. Sound both ways: params of the bound proc now checked at the invoke (stricter), its uncond outs credited (as for `jsr`), D1c now sees it (observe-only gate) |
| `type_slice.rs:104` `direct_target` (slot-type check) | yes: the invoke degraded every register and its typed slots went unchecked | fixed, `1cfe30a4`. A type survives only through a register the bound proc's DECLARED (S2-D6 gated) clobbers leave alone |
| `z80_out_verify.rs:104` | no: `invoke` has no Z80 form. Measured with a throwaway test: under `Cpu::Z80` it is refused `[lower.z80-unsupported] a size-suffixed indirect is not a Z80 operand form` and emits no call item | not changed |

Red-first, each on disk against its committed fix, restored with `git restore --source=HEAD`:
- `out_verify.rs` arm removed (`// REVERTED: AbsSym arm removed`): `invoke_of_preserving_hook_verifies`
  and `invoke_of_clobbering_hook_fires` (its reason pin) FAILED, 3 passed.
- `calls.rs` back to `[CodeOperand::Sym(name)] if !name.contains('$') => Some(name.as_str()),`
  only: the three D1b/D1c tests FAILED, 5 passed.
- `type_slice.rs` the same: `slot_check_reaches_the_bound_proc_at_an_invoke` and
  `a_type_survives_an_invoke_that_preserves_it` FAILED, 9 passed.

## Baseline

The frozen inout baseline is `INOUT_UNVERIFIED_BASELINE` in
`crates/sigil-harness/src/contract_baseline.rs` (empty; the build gate diffs against it).
Not under `golden/`, `pins.rs` or `repin.toml`. `sigil build --report contracts` on
`.aeon-sigil-ref` (`ec640bcf`) is byte-identical between base `a0cc4693` and the fixed binary
for sonic4 and demo, plain and `--debug`, and for `--config-a`, `--config-b`, `--lean`,
`--stress-evict`, `--stress-art`. No baseline (inout, out, D1c, slot) moves; nothing updated.
On the probe copy the only report change is DrawRings `inout(a4)`/`inout(d5)`, 2 firings to 0.

## Booked, not fixed

- `preserves.rs:1735` `call_target` is another Sym-only call reader (last `Sym` operand, no
  `$` filter), feeding verified-preserves and the dead-save walk. It is not in the stage-0
  list; the dead-save walk cuts code, so a change there can move bytes. Needs its own parcel.
- `corpus_contracts::call_target_sym` (via `flag_check::transfer_target_sym`) reads `SymOff` as
  a call to the item, so the closure and the caller-side readers disagree on `jsr Item.field`.
  Not a gap in the caller readers (they refuse it as unknown); whether the closure's charge is
  always an over-approximation of a mid-item entry is unassessed.
