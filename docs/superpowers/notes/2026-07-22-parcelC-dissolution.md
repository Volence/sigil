# Parcel C — dissolved at stage-0 (roadmap item 7)

Stage-0 check of item 7's three riders against current code (all flagged pre-Parcel-A;
lines have since moved):

| Rider | Definition (verified real) | Current firings | Verdict |
|---|---|---|---|
| **W022** loop-invariant memory operand in a `dbf` loop | `s4lint.py:65` | **0** | evaporated |
| **W025** `adda.w #imm,aN` where `lea imm(aN),aN` fits | `s4lint.py` (test_s4lint W025) | **0** | evaporated |
| **ledger-1092 `move.l` pairing** | segment copy loops | survives | → folded into 8b |

`python3 tools/s4lint.py games/sonic4/main.asm` → **"no issues found"**: the entire `.asm`
corpus is s4lint-clean, every code zero. W022/W025 sites were dissolved by the pass-2
restructures + the port campaign. (The 2026-07-22 s4lint-absorption census independently
reclassified W022 as a future sigil perf-lint pass, "Not pass-3 work" — consistent.)

The **`move.l`-pairing** rider survives — `tile_cache.emp` FillRow 1523/1532 (nametable) +
collision 1608-1619 (plane A/B byte runs), `plane_buffer.emp` drain 334/340 are still
`move.w`/`move.b`. It halves `dbf` trips (~40% of the copy-loop portion) but is
**non-lag-critical** (design gate: ≥35% idle every regime; pass-2 already cut FillRow ~60%).
Per the overseer ruling it is **folded into the 8b parcel** as its own bisectable commit(s)
(tile_cache sites + plane_buffer sites), even-word-alignment verified, sharing 8b's ripple +
PROVENANCE ceremony — a logged slot, not a silent skip.

## The pattern: third net-conversion

This is the **third** time a stage-0 check has found anticipated pass-3 byte-surgery already
done or dissolved:

1. **S2-D6 #3 stage-0** — the transitive `[proc.clobber-undeclared]` residue was already 0
   (error-gated since G3); the parcel re-scoped to detector completeness.
2. **Parcel B** — the D1c "hoist fuel" contained no code hoists (D1c-clear sites are tight by
   construction); it became 4 byte-neutral contract tightenings.
3. **Parcel C** — W022/W025 fire 0 times (the corpus is s4lint-clean).

The mechanism: the contract net + the port campaign's step-2 modernization keep *retiring*
the very defects pass-3 was scheduled to hand-fix. **Lesson (banked): stage-0 every parcel** —
confirm the target still exists in current code before paying any byte-changing ceremony. The
survival bar the overseer set for the Parcel D re-scope census is this lesson formalized.
