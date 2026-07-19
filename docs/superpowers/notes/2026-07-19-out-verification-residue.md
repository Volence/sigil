# out()-verification (G4.5) — corpus residue + adjudication

**2026-07-19.** The callee-side `[proc.out-unverified]` verifier ships **observe-only
(WARN foundation)** in G4.5: it is NOT error-tier and is NOT wired to a residue-empty
error pin. The WARN→ERROR flip is deferred — it needs item #2 (edge-sensitive
conditional-out crediting, clears Bucket 1) and G5 (width-typed outs + in-out markers,
clears Buckets 2/3).

Corpus run: **17 `[proc.out-unverified]` firings** across 12 procs, **1 `[call.live-
clobbered]` D1c firing**. Every firing was verified against the proc body; nothing was
self-cleared, no label was lied. Four classes, all observe-only:

## Bucket 1 — should-be-conditional (real, → item #2)
A declared UNCONDITIONAL `out(a1)` where `a1` is produced only on the success return and
left unproduced on the failure (pool-exhausted) return. Same class as the FindStagedBlock
existence proof. **No live bug** (verified: the only caller guards on Z / no callers):

| Proc | Detail |
|---|---|
| `AllocDynamic out(a1)` | `a1` via `movea.w` on success; `.full`/`.latch_full` return `moveq #1,d0; rts` with `a1` unproduced. Sole caller `Load_Object` guards on Z (`beq`). |
| `AllocEffect out(a1)` | Identical shape; `.full` returns `moveq #1,d0` with `a1` unproduced. No callers. |

**Honest resolution (with #2):** relabel to `out(a1 if eq)` (the code genuinely IS
conditional — provable). This removes `a1` from `callee_uncond_out`, which makes
`Load_Object out(a1)` fire until #2 credits `AllocDynamic`'s conditional out on
`Load_Object`'s eq edge (edge-sensitive callee-out crediting). So the relabel + cascade
clear lands together with #2.

## Bucket 2 — narrow-width data outputs (consumer-honest, → G5)
Legitimate 16-/8-bit outputs written `.w`/`.b` (high word stale by design). The full-width
rule (Finding 1) correctly rejects them. **Consumer-width trace: every consumer reads only
`.w`/`.b` — zero `.l`/high-word reads → no latent false-negative.**

| Out register | Consumer(s) | Read width |
|---|---|---|
| `GetSineCosine d0` | player_ground.asm ×4 | `.w` (`asr.w`, `move.w`) |
| `GetSineCosine d1` | player_ground.asm ×2 | `.w` (`muls.w d2,d1`) |
| `Tile_Cache_GetTile d2` | none (only its `.asm` stub) | — (unconsumed API) |
| `Tile_Cache_GetCollision d0` | →`Collision_GetType`→player_sensors.asm | `.b` (`d0.b = attr`) |
| `EntityWindow_EntryForSection d0` | 4 `.emp` sites | `.w` (`tst.w d0; bmi`) |
| `EntityWindow_DeriveWindow d2,d3` | `EntityWindow_BuildEntries` ×2 | `.b` (`move.b`/`cmp.b`) |
| `EntityWindow_DeriveWindow d4,d5` | `EntityWindow_BuildEntries` | `.w` (`asl.w`) |
| `Section_RedrawPlanes d7` | `Section_UpdateColumns` | `.w` (`move.w d7,…`) |

`Section_RedrawPlanes d5` correctly VERIFIES (`move.l Camera_X,d5` — full-width) — the width
rule is discriminating, not blanket. G5 resolution: width-typed outs (e.g. `out(d0.w)`) or
an accept-with-marker.

## Bucket 3 — in-out accumulators (always-defined, → G5)
The sprite/ring SAT-streaming convention: a register that is BOTH a param and an out, threaded
in and returned updated. Fires because there is no param seed (Finding 2) and the register is
returned unchanged on the empty path / advanced by a byte op (`addq.b`) / auto-inc.

| Proc | Registers |
|---|---|
| `DrawRings` | `d5` (sprite index), `a4` (SAT write cursor) |
| `InsertSpriteMasks` | `d5`, `a4` |
| `Emit_ObjectPieces` | `d5` (`a4` VERIFIES — full-width auto-inc on all paths) |

An in-out seed was prototyped and **reverted** (commit revert of the seed): crediting a
param∩out register as produced-at-entry is unsound in general — it blesses a non-producing
bail path and cannot distinguish an accumulator (input is a valid output) from a TRANSFORM
(input≠output, e.g. `Tile_Cache_GetCollision d0` — world_col in, collision_byte out), which it
wrongly cleared. G5 resolution: an explicit in-out param marker.

## D1c false positive (observe-only)
`TileCache_FillRow @ TileCache_FindStagedBlock :: a1`. FillRow reads `a1` at `.fr_have_block`,
reached either via `beq .fr_have_block` (the eq/hit path where FindStagedBlock's `a1` IS valid)
or via fall-through after `jbsr TileCache_DecompressBlock` (unconditional `out(a1)`, which
redefines `a1`). Every read is a freshly-produced valid value — not a stale held value. The
firing is the simple D1c close being edge-blind across loop iterations (`a1` is may-defined from
the prior iteration). This is the anticipated tiny surface — FindStagedBlock `a1` is the only
register-conditional out. **D1c is observe-only (not gated); this does not break any pin.**
Item #2's cc-edge precision retires it if it grows.

## Known limitations — flip-blockers
Two soundness gaps in the verifier itself (distinct from the corpus residue above).
Neither has a corpus instance today, but both must close before the WARN→ERROR flip —
they join Bucket 1 (→ #2) and Buckets 2/3 (→ G5) as flip preconditions.

- **Mutual/circular callee-out sourcing** (adversarial Finding 2, theoretical, not in
  corpus). The callee-out / tail-out credit reads each callee's DECLARED unconditional
  out, with no verification fixpoint. Two procs that mutually source `out(rN)` from each
  other — A `out(a1)` credited only from `jbsr B`, B `out(a1)` only from `jbsr A`, neither
  actually writing `a1` — would each verify against the OTHER's declared label: a
  self-consistent lie. No corpus instance. Close before the flip via a verified-out
  fixpoint (credit only VERIFIED callee outs, not merely declared ones) or a
  proof-of-absence.
- **Conditional-external-tail `Defer`** (adversarial Finding 3, independently confirmed,
  not live). A conditional branch to a non-local/unresolved target (`beq SomeExternalProc`)
  yields an `Edge::Defer` the verifier IGNORES (the `if !is_uncond_tail(mnem)` guard),
  mirroring `preserves`. For out-honesty that Defer is a REQUIRED return path, so an out
  left unproduced there would escape the check. **0 corpus instances** (grep-confirmed: no
  conditional branch to a non-local symbol in the corpus). Close before the flip.
