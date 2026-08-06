# 2026-08-05 — mul-w parcel packet (`mul_const.w` / `mul_bounded.w`)

Lane: b6 pair (sigil `mul-w` off `ea0ee860` chain 47 · aeon `mul-w` off
`b9b1056`). Porter parcel; no merge-state claims — commits + the measured land
order at the end. Successor to the long-form mul-lowering parcel; this packet
states only what DIFFERS from that one (its design + packet are the base docs).

## 1 · What the sized variant adds

The suffix IS the name — two contracts, one meaning per spelling:

```
mul_const.w   dN, #n              // dN.w = (u16(dN.w) × n) mod 2^16   (n comptime 0..=$FFFF)
mul_const.w   dN, #n, dScratch    // same; scratch may come back clobbered
mul_bounded.w dD, dSrc, #M        // dD.w = (u16(dD.w) × u16(dSrc.w)) mod 2^16, src ≤ M
mul_bounded.w dD, dSrc, #M, dScr  // same; scratch enables the loop candidate
```

- **Word result, UPPER WORD UNDEFINED.** `mulu.w` stays a candidate (it writes
  all 32 bits — licensed by the undefined-upper contract). The corpus's word
  chains happen to leave the upper word untouched; callers may not rely on it,
  and the unit oracle seeds garbage to prove the freedom (§4).
- **Author owns the range proof** — carried where the corpus already carries it
  (the module `ensure`), never checked by the construct.
- **`.l` (or any non-`.w` suffix) REFUSED** — `[mul.size]` names both facts
  ("bare IS the long form; `.w` is the word form; there is no `.l`").
- CC clobbered-undefined; reg-class / scratch-alias / mandatory-bound rules —
  all identical to the long form.

**Where it lives (only the shipped-long-form files touched):**
`crates/sigil-frontend-emp/src/mul_lower.rs` (a `MulWidth` enum threaded through
`expand_item` → `expand_mul_const`/`expand_mul_bounded` → a `_word` candidate
generator + a `shift_run_word` helper + word-op oracle arms), `lower/code.rs`
(untouched — the item-position net calls the same `expand_item`), `tests/
mul_lowering.rs` (2 new integration tests + the `.w`-refusal probe flipped to
`.l`). No parallel generator file — the word set is a sibling function sharing
`choose`/`seq_worst_cycles`/`seq_bytes`/`instr`.

## 2 · The word candidate set (§3 of the spec)

Differs from the long set in exactly one structural way: **NO zero-extend seed
anywhere** — that is the whole economic point. Priced through the SAME
`instr_cost` seam (no new tables, no cycle literals):

1. `mulu.w #n, dst` — always legal, enumerates first (equal-cost ties → mulu).
2. n = 0 → `clr.w dst`.
3. n = 1 → **ZERO instructions** (×1 mod 2^16 is already in place; pinned).
4. n = 2^k → `lsl.w` run(s) on dst directly (chunks ≤ 8).
5. with scratch, ≥ 2 set bits: the **two-power sum** `move.w D,S / lsl.w #a,D /
   lsl.w #b,S / add.w S,D` for n = 2^a+2^b (the corpus stride idiom), plus the
   subtract form and a general LTR for ≥ 3 bits. The two-power arm emits a plain
   `lsl.w` per term (NOT the long form's `add.w`-doubling optimization) — that
   is what makes it byte-identical to the hand strides.

**Spec vs implementation (the §3 contradiction, resolved).** The spec's §3 prose
says the word chain is "left-to-right binary … mirroring the long generators at
word width", but its OWN pinned costs (×66 chain = 34, ×80 = 40, ×160 = 44) are
the two-power hand-chain costs, NOT the LTR costs (a faithful word LTR prices
×66/×80/×160 at 32/32/34 — cheaper). Byte-identity at the four sites and those
pinned costs BOTH require the **two-power** generator, so that is what shipped;
the LTR arm is deliberately gated to ≥ 3 set bits (where no adoption site exists)
so it can never undercut the two-power chain at a stride site. The master spec
was amended to state this two-power/LTR ruling (**sigil `06d3cc40`**); this
lane's branch base (`ea0ee860`) predates the amendment — NOT rebased (docs-only,
the merge handles it), cited here.

## 3 · The four-site byte-identity proof

Each adopted site, its chosen word candidate, and the cost derivation that makes
the chain the WINNER (mulu.w #n prices `38 + 2·ones(n) + 4`; the value-aware
row from the long form). Every `lsl.w #k` = `6 + 2k`; `move.w Dn,Dn` /
`add.w Dn,Dn` = 4:

| site | n | chosen chain | chain cy | mulu.w cy | verdict |
|---|---|---|---|---|---|
| section.emp `Section_GetSecPtrXY` | 66 (2⁶+2¹) | `move.w d0,d1 / lsl.w #6,d0 / lsl.w #1,d1 / add.w d1,d0` | 4+18+8+4 = **34** | 46 | chain |
| tile_cache.emp `TileCache_DecompressBlock` | 66 | `move.w d3,d4 / lsl.w #6,d3 / lsl.w #1,d4 / add.w d4,d3` | **34** | 46 | chain |
| tile_cache.emp `mul_cache_stride` (comptime fn) | 80 (2⁶+2⁴) | `move.w {d},{s} / lsl.w #6,{d} / lsl.w #4,{s} / add.w {s},{d}` | 4+18+14+4 = **40** | 46 | chain |
| plane_buffer.emp `Draw_TileColumn` | 160 (2⁷+2⁵) | `move.w d1,d2 / lsl.w #7,d1 / lsl.w #5,d2 / add.w d2,d1` | 4+20+16+4 = **44** | 46 | chain |

The comptime fn `mul_cache_stride` splices its body at BOTH call sites
(`Tile_Cache_GetTile` d1/d2, `TileCache_FillRow` d2/d3) — the `.w` construct
inside the `asm {}` defers (cpu-less template) and expands at each caller's
CodeBuf completion, so both copies emit the same ×80 chain. Byte-identity is
proven three ways: the isolated integration test
`word_strides_are_byte_identical_to_hand_chains` (lower `mul_const.w` vs the
hand chain, `flatten` equal, 66/80/160), the unit shape pins
`word_corpus_strides_resolve_to_chains`, and the WHOLE-ROM golden bar (§5) —
every one of the seven target CRCs unchanged, which is strictly stronger than
the four sites.

The two loop sites (`section.emp .gxy_mul`, `tile_cache.emp .mul_loop`) are NOT
adopted — `mul_bounded.w` there emits `mulu` (word loop ceiling 28 + 14·M beats
mulu's 70 only through M = 2; any real grid width picks mulu) and is
byte-CHANGING. Ledgered (gap-ledger loop-site row); R-B annotation updated.

## 4 · Proof obligations (§5 of the spec)

- **Word equivalence oracle** (`word_lowering_matches_low_word_and_leaves_upper_free`):
  every chosen `.w` lowering executed over concrete registers with GARBAGE upper
  words on dst + scratch, across 23 multipliers × 9 boundary x × 3 garbage
  patterns × {no-scratch, scratch}; asserts `dst.w == (zx(x)×n) mod 2^16` and
  NOTHING about the upper word. The oracle grew `lsl.w`/`add.w`/`sub.w` arms
  that operate on the low word and PRESERVE the upper — so the check is honest.
- **The dedicated upper-word-freedom pin**
  (`word_upper_is_free_mulu_trashes_chain_preserves`): the ×66 chain leaves the
  seeded `0xDEAD` upper word intact; ×$8001 (whose far-apart bits make the chain
  lose to mulu on cycles) picks `mulu.w` and TRASHES the upper word — both
  accepted, proving the contract's freedom is real, not incidental.
- **`mul_bounded.w`** (`word_bounded_semantics_and_boundary`): sweeps every
  in-bound src for M ∈ {0,1,2,3,16}; the loop accumulates with `add.w`; the
  boundary matches the long form (loop through M = 2, mulu at M ≥ 3).
- **Authorship**: the `.w` path shares `expand_item`'s tail
  (`reauthor_user_items`), so the expansion propagates the construct item's
  author identically — no new variant, no compiler author (the long form's §11
  invariant holds; `word_scratch_clobber_is_seen_by_the_clobber_lint` pins the
  scratch write is charged to the proc).
- The mnemonic-vocabulary panic guard in the oracle carries over unchanged.

## 5 · Bars

- **Byte bar — SEVEN targets, every CRC UNCHANGED** (`capture_goldens.sh`
  procedure, `SIGIL_BUILD`/`SIGIL_EMIT` exported to the b6 release binaries,
  `AEON_DIR` the b6 aeon): s4 `c2d17ee3`/411096 · s4.debug `6c296656`/423480 ·
  demo `4a09314e`/91258 · demo.debug `f3e5ed3e`/93955 · config_a
  `4e34a38a`/423871 · config_b `b8cce891`/301132 · lean `b92cb485`/379110.
  Canonical restore OK. No target moved — the four adoptions are byte-identical.
- **refreeze --check:** OK (tip `sst-fold`, chain len 47). **repin --check
  (SIGIL_EMIT, b6 aeon):** `pins.rs unchanged`.
- **Warn tiers:** plain 19 (module.path-mismatch 9, proc.undeclared-fallthrough
  6, proc.out-unwritten 3, proc.clobber-undeclared 1), s4-DEBUG 18 — id sets +
  counts identical to the expected bar (the four adoptions add no warning: each
  proc already declared its scratch in `clobbers`).
- **Full strict `SIGIL_STRICT_GATE=1` (AEON_DIR = b6 adopted corpus):**
  **3375 passed / 0 failed / 4 ignored = 3379 over 310 suites.** Failures-first
  scan of the capture: ZERO `FAILED` / `panicked` / `error[` lines. Test
  arithmetic: base `#[test]` 3369, this parcel adds 10 — unit (6):
  `word_lowering_matches_low_word_and_leaves_upper_free`,
  `word_upper_is_free_mulu_trashes_chain_preserves`,
  `word_corpus_strides_resolve_to_chains`, `word_degenerates_and_powers`,
  `word_bounded_semantics_and_boundary`, `word_size_suffix_routing`;
  integration (4): `word_strides_are_byte_identical_to_hand_chains`,
  `word_scratch_clobber_is_seen_by_the_clobber_lint`,
  `word_through_a_comptime_asm_template_is_byte_identical` (the template path),
  `word_byte_suffix_refuses` (the `.b` probe) → 3379; the `.w`-refusal probes
  were flipped to `.l` in place (not added). 3375 passed + 4 ignored = 3379 =
  3369 + 10 — closes exactly. (n = 11/19 added to the oracle's NS so the ≥ 3-bit
  LTR arm is a CHOSEN lowering and executes with garbage-upper seeds — iterations,
  not new tests.)
- **Clippy `-D warnings` (`-p sigil-frontend-emp --all-targets`):** clean. One
  finding fixed en route — a pre-existing `redundant_closure` in
  `expand_mul_bounded`'s loop-cost sum (`|i| item_worst_cycles(i)` →
  `item_worst_cycles`), behavior-identical (the byte bar re-ran clean after it).

## 6 · Ledger / docs

- Gap-ledger: the sized-variant row (all four sites as its demand census)
  CLOSED with a pointer here; a new loop-site row OPENED (the two deferred
  byte-changing sites).
- The mul-lowering packet §5 "negative result" paragraph gains an UPDATE
  pointer (the bar the long contract could not meet is now satisfiable).
- The R-B closure annotation (2026-08-02 lens-sweep adjudication) points here:
  the STRIDE sites adopted, the LOOP sites deferred.
- `emp-idioms.md`: the "when NOT to use" bullet replaced with the `mul_const.w`
  word-stride idiom (name the constant, the compiler emits the two-power chain)
  + the deferred loop-site note.

## 7 · Merge order (the intermediate windows measured)

Unlike the long-form parcel, **sigil merges FIRST this time.**

- OLD-sigil + NEW-corpus (adopted `.w`): RED — the master `expand_item` refuses
  ANY size suffix (`[mul.size]`), so `mul_const.w` fails to parse. Proven by the
  pre-existing master test `refusals_surface_through_the_pipeline`
  (`mul_const.w d0, #66` → `[mul.size]`), which this parcel had to FLIP to `.l`.
- NEW-sigil + OLD-corpus (aeon master, no `.w`): GREEN — measured by stashing
  the four adoption edits and building s4 with the b6 release sigil: build
  complete, `s4.bin c2d17ee3 / 411096` — BYTE-IDENTICAL to the adopted build.
  (This single measurement proves both directions: sigil-first is safe, and the
  adoption is byte-identical — old and new corpus produce the same ROM.)

So aeon cannot precede sigil; sigil is a backward-compatible superset. Per the
standing trap, merging this two-repo parcel stales every other in-flight lane's
aeon worktree.

**Masters moved after these bars ran** (the residue lane merged): aeon master
now carries a `Section_RedrawPlanes` contract flip in `section.emp` — a
DIFFERENT proc from this lane's `Section_GetSecPtrXY` hunk, so a clean textual
merge is expected; sigil master moved on docs + a comment only, none of it code
this lane touches. All bars above were run against the pre-move masters
(`ea0ee860` / `b9b1056`); that is fine — the moves are contract/docs-only and
byte-neutral, and the merge gate re-verifies at the merge commit.

## 8 · Step-3 vs step-5 findings

**Step-3 (language asks):** none new — the sized variant WAS the standing
step-3 ask #1 of the long-form parcel, now delivered. The `shl_l` companion
(long-form step-3 #3) and the DEBUG-assert mechanism (#2) remain open, untouched.

**Step-5 (engine findings, not taken):**
1. Other word-chain sites exist that this census did NOT name — plane_buffer's
   `Draw_TileRow_FromCache` ×80 (`lsl.w #6/#4`), section.emp `Section_RedrawPlanes`
   ×160, tile_cache's `CopyBlockColumn`/`FillRow` single-temp ×80
   (`((x<<2)+x)<<4`). All are byte-identically adoptable under `mul_const.w`
   (the two-power sites) or need the ±1/single-temp forms; left for
   opportunistic adoption when their files are next touched (construct-first
   discipline stays thin — the spec named exactly four).
2. The single-temp ×80 form (`((x<<2)+x)<<4`, 3 ops + shift) is a DIFFERENT
   encoding the two-power generator does not emit; adopting those sites would be
   byte-CHANGING (a cheaper-or-equal chain, decided by the model) — a candidate
   only for a deliberate byte-changing parcel, not this one.
3. The LTR shape is 2–10 cy CHEAPER than the two-power chain at the four stride
   sites (a faithful word LTR prices ×66/×80/×160 at 32/32/34 vs 34/40/44);
   it is deliberately SUPPRESSED (gated to ≥ 3 set bits) to hold byte-identity,
   and is recoverable only by a deliberate byte-changing parcel that re-derives
   each stride to the LTR encoding and re-freezes the goldens.
   **TAKEN by the 2026-08-06 byte-changing parcel** (specs/2026-08-06-byte-
   changing-mul-parcel.md R1; notes/2026-08-06-ltr-mul-packet.md): the gate
   relaxed to ≥ 2 set bits, `choose()` now takes LTR at every stride site on
   cycles, and the goldens re-froze. The suppression no longer exists.

**Neither-bucket headline:** the whole corpus stride family sits 2–12 cycles
UNDER mulu's 46 at word width — and exactly ON mulu at LONG width (the long
form's structural tie). The sized variant is not a micro-optimization; it is the
contract that prices what an `adda.w`-bound stride actually computes, and the
2-cycle table granularity (a ±2 correction on `move`/`lsl`/`add` flips
`chain↔mulu` for the ×160 site, which sits at 44 vs 46) is doing real work — any
future table change rides the golden gates, same as the long form's note.

## 9 · Commits

- sigil `mul-w` (off `ea0ee860`): `e5bd21bf` (the `.w` construct + word
  candidates + oracle arms + tests + the redundant-closure clippy fix),
  `7f6de4eb` (docs + this packet), plus the lens-panel fix-up commit at the tip
  (comment corrections — LTR-gate reason, word M=3 tie, add.w 4-cy; the
  `[mul.size]` `.l`-vs-`.b/.s` message split; NS n = 11/19 so the ≥3-bit LTR arm
  executes; the template-path + `.b` integration pins; and the packet/ledger
  dispositions).
- aeon `mul-w` (off `b9b1056`): `a95b44f` (the four byte-identical adoptions),
  plus the lens-panel comment-trim commit at the tip (narration → multiplier
  fact, matching the plane_buffer style).

Branches stay UNMERGED. The aeon commits build only against a sigil binary
carrying the `.w` construct — so the merge queue's ORDER constraint is: sigil
must precede aeon (a land-order fact, not a claim about what has merged).
