# out()-verification (G4.5) — corpus residue + adjudication

**2026-07-19.** The callee-side `[proc.out-unverified]` verifier ships **observe-only
(WARN foundation)** in G4.5: it is NOT error-tier and is NOT wired to a residue-empty
error pin. The WARN→ERROR flip is deferred — it needs item #2 (edge-sensitive
conditional-out crediting, clears Bucket 1) and G5 (width-typed outs + in-out markers,
clears Buckets 2/3).

Corpus run: **17 `[proc.out-unverified]` firings** across 12 procs, **1 `[call.live-
clobbered]` D1c firing**. Every firing was verified against the proc body; nothing was
self-cleared, no label was lied. Four classes, all observe-only:

---

## 2026-07-21 — THE FLIP (Phase-1 item #4): verified-out fixpoint + the dividing line

D1b is now an ERROR gate. The credits it rests on are **verified, not declared** — a
least-fixpoint (`out_verify::compute_verified_outs`) proves each `out(rN)` produced on
every required return path, crediting a callee/tail out only once THAT out is itself
verified; **extern outs seed verified** (§3 boundary axioms — no body to check; the
verifier deliberately stops at the `.asm` link seam, an extern's out-honesty is its
twin's contract). A mutual/circular out that never grounds in a local production stays
UNVERIFIED. Post-#2 the residue is **16 firings / 10 procs** (the AllocDynamic/AllocEffect
`out(a1)` relabels are now `if eq` and verify).

**The define-vs-redefine dividing line** (the architecture this arc produced — which
surfaces switch to verified credit and which keep declared):

| Surface | Credit question | Source | Δ at the flip |
|---|---|---|---|
| **D1b must-def** (`check_input_undefined`) | is the callee's param *defined* here? — needs a **guaranteed** production | **verified** (fixpoint) | 0 firings |
| **out-verify residue** (`out_firings`) | which outs are honest? — IS the out-verify surface | **verified** (ruling 3) | 15→**16** (adds `Collision_GetType`) |
| §6 invalid-path (`check_result_invalid_path`) | does an uncond out *redefine / kill taint*? — a **narrow** out still redefines | **declared** | 0 either way (tripwire-guarded) |
| D1c live-clobbered (`destroys_value`) | is reg a *produced result* vs a *held value*? — a narrow out IS a result | **declared** | verified would add **11 FPs** (2→13) |
| closure / dead-save / may-def | (do not credit out-as-definition) | unchanged | — |

The line: **verified-out is right for DEFINE semantics (D1b + the out-verify surface),
wrong for REDEFINE-EXCUSE semantics (§6/D1c).** A width-unverified out (Bucket 2)
genuinely *redefines* a register (low word fresh) — so §6/D1c keep declared — but does
not *guarantee a full definition* and might be an existence-lie (the FindStagedBlock
shape) — so D1b conservatively drops it (firing-safe; empirically adds 0). §6 retains the
existence-lie exposure D1b closed; it is 0-firing today and guarded by the
`corpus_flag_results_declared_vs_verified_credit_agree` tripwire. The per-lie-class credit
that closes §6 cleanly is a gap-ledger row (`campaign-gap-ledger.md`, 2026-07-21).

**New residue entry — `Collision_GetType::out(d0)`** (fixpoint chain-grounding, not a new
bug). It verified under DECLARED credit only via `jbra Tile_Cache_GetCollision`
(collision_lookup.emp:36, a tail-out credit) ∩ the local `.cgt_air moveq`. Since
`Tile_Cache_GetCollision::out(d0)` is a Bucket-2 `.b` narrow-width production (unverified),
the fixpoint correctly withdraws the tail credit → `Collision_GetType` flips unverified.
`d0` is in-out (param∩out) → **no D1b/§6 consequence** (must-def never kills a register
already defined as the call's own input). Chain-grounding in an unverified leaf, **not a
cycle** — Finding-2 mutual/circular out-sourcing remains grep-absent in the corpus.

**Flip-blockers — all closed:**
- *Bucket 1* (→ item #2): CLOSED — the relabels verify.
- *Buckets 2/3* (was → G5): **flip needs NO G5** (overseer ruling, Stage-0a). Predicted new
  D1b firings under verified-only crediting = **0** at the aggressive 17-out bound: every
  unverified out is either in-out (defined upstream; must-def never kills) or a pure-out
  read-consumed / `.asm`-only (never threaded as a fresh `.emp` callee param). They stay
  WARN-tier residue, adjudicated below.
- *Mutual/circular callee-out* (Finding 2): CLOSED by the verified-out fixpoint (credit only
  VERIFIED outs); no corpus instance.
- *Conditional-external-tail* (Finding 3): re-confirmed grep-absent (the sole conditional
  branch-to-global, `bne RunObjects_Frozen`, resolves to a known proc in an out-less caller);
  the `is_uncond_tail` Defer-guard stands.

The four observe-only classes below remain the documented WARN residue (now computed against
the verified fixpoint, consistent with the D1b ERROR gate):

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
