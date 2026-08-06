# mul_const.w / mul_bounded.w — the sized-variant ruling (2026-08-05)

Status: RULED (Fable). Successor to the 2026-08-03 mul-lowering design's §5
deferral; answers the mul-lowering packet's step-3 ask #1 and closes the
gap-ledger sized-variant row (4-site demand census). The long forms' design and
packet are the base documents; this spec states only what differs.

## 1 · Why a second contract exists

The corpus's multiply-chain idiom is WORD-width, licensed by module `ensure`s
bounding the product to a word. The ratified long contract must price its
zero-extend, and with it every hand chain loses to `mulu` — the round-1
adoption bar was unsatisfiable by construction (packet §5's negative result).
The word contract prices what the sites actually compute; under it the chains
win back exactly the 2–12 cycles the census measured, and the four sites can
adopt byte-identically.

## 2 · Surface and contract

```
mul_const.w   dN, #n              // dN.w = (u16(dN.w) × n) mod 2^16
mul_const.w   dN, #n, dScratch    // same; scratch may come back clobbered
mul_bounded.w dD, dSrc, #M        // dD.w = (u16(dD.w) × u16(dSrc.w)) mod 2^16, src ≤ M
mul_bounded.w dD, dSrc, #M, dScr  // same; scratch enables the loop candidate
```

One contract, one meaning, per name — the suffix IS part of the name:

- **Result**: the low word of the product, written to `dst.w`. The multiply is
  total (mod 2^16); whether the product FITS a word is the author's range
  obligation, carried exactly where the corpus already carries it (the module
  `ensure`), never checked by the construct.
- **Upper word of dst: UNDEFINED after the construct.** This deliberately
  loosens the step-3 ask's "unchanged-garbage" phrasing, for one reason: an
  unchanged-upper promise excludes `mulu` from the candidate set (it writes all
  32 bits), and without `mulu` a many-set-bits multiplier is condemned to an
  arbitrarily long chain — a cost trap the compiler could never escape. A
  caller who needs the upper word spells the bare (long) form; that IS the
  distinction between the two names. The word chains the corpus adopts today
  happen to leave the upper word unchanged; callers may not rely on it, and
  the oracle seeds garbage to enforce that (§5).
- **Condition codes: clobbered-undefined** (parity with the long form).
- **Scratch/aliasing/reg-class rules**: identical to the long form
  (`[mul.reg-class]`, `[mul.scratch-aliases]`); `mul_bounded.w`'s bound is
  mandatory and inclusive, `dst == src` legal with the loop structurally
  excluded — all unchanged.
- **`.l` is REFUSED**: bare `mul_const` IS the long form; a second spelling of
  one meaning is a reader tax. `[mul.size]`'s message updates to name both
  facts ("bare is long; `.w` is the word form; there is no `.l`").
- Signed: still refused entirely, both widths (unchanged v1 stance).

## 3 · Candidates and cost

All candidates price exactly through `instr_cost` (no new tables, no cycle
literals — the long form's discipline). The word set differs from the long set
in exactly one structural way: **no zero-extend seed exists anywhere** — that
is the entire economic point.

For `mul_const.w(n)`:
1. `mulu.w #n, dst` — always legal (upper-word write is licensed by the
   undefined-upper contract). Priced by the value-aware row (38 + 2·ones + 4).
2. n = 0 → `clr.w dst`.
3. n = 1 → ZERO instructions (the word result is already in place; the honest
   lowering of ×1 mod 2^16 is nothing). Deterministic, pinned.
4. n = 2^k → `lsl.w` run(s) on dst directly (k ≥ 1; runs ≤ 8 per lsl).
5. With scratch, ≥ 2 set bits: the word add/shift chain over
   `move.w dst, scr` seeds (`add.w`/`lsl.w` bodies — word ops throughout),
   left-to-right binary + the subtract form, mirroring the long generators
   at word width.

For `mul_bounded.w`: `mulu.w` (70 ceiling) vs the word repeated-add loop
(priced from the same seam). Same tie-breaking as the long form: fewest
worst-case cycles → fewest encoded bytes → fixed enumeration order with `mulu`
first.

Pinned boundary expectations (the porter verifies, from the table, and pins):
×66 word chain 34 vs mulu 46 → chain; ×80 chain 40 vs 46 → chain; ×160 chain
44 vs 46 → chain; the corpus stride family flips from the long form's
mulu-always to chain-always — state the flip in the module header as the
2-cycle-granularity load-bearing note already requires.

## 4 · Adoption (this parcel) and non-adoption (deferred)

**Adopt byte-identically, 4 sites** (the census, packet §5 table):
`section.emp Section_GetSecPtrXY` ×66, `tile_cache.emp
TileCache_DecompressBlock` ×66, `tile_cache.emp mul_cache_stride` ×80 (the
comptime fn and its caller copy), `plane_buffer.emp` ×160. The chosen word
chain must equal the hand bytes at every site — that is the acceptance bar the
long form could not meet; if ANY site is not byte-identical, STOP and report
(the cost re-derivation is wrong, not the site).

**Defer, ledgered**: the two repeated-add loop sites (`section.emp .gxy_mul`,
`tile_cache.emp .mul_loop`) — adopting `mul_bounded.w` there emits `mulu`
(the cost winner at any real bound) and is byte-CHANGING; it rides a
deliberate byte-changing parcel with the full ripple (twin lockstep is gone
post-K, but pins/goldens re-freeze applies). Update the R-B closure annotation
to point here.

## 5 · Proof obligations

The unit oracle extends to the word contract: execute every chosen lowering
over concrete registers with GARBAGE UPPER WORDS on dst and scratch, assert
`dst.w == (zx(x) × n) mod 2^16` AND assert nothing about dst's upper word
(and, one dedicated pin: a lowering that picks `mulu` genuinely trashes the
upper word while a chain does not — both accepted, proving the contract's
freedom is real). `mul_bounded.w` sweeps every in-bound src for
M ∈ {0, 1, 2, 3, 16}. The mnemonic-vocabulary panic guard carries over.

Authorship: the expansion propagates the construct item's author through
`reauthor_user_items`, identical to the long form — no new variant, no
compiler author (packet §11's invariant).

## 6 · Bars

Byte bar seven targets: the four adoption sites byte-identical, every target
CRC unchanged. Full strict with closing arithmetic. The `[mul.size]` negative
probes update. Gap-ledger: sized-variant row CLOSED (this spec + the parcel),
loop-site row opened/updated per §4. The mul-lowering packet's §5 "negative
result" paragraph gains a pointer here (the bar is now satisfiable — say so
where the failure was recorded).
