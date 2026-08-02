# `mul_const` / `mul_bounded` — cost-model multiply lowering (design)

**Provenance:** Volence's proposal at the lens-sweep rulings (2026-08-02): "a new
multiply command, akin to our own jbsr, that will do add X amount of times or mul
depending on which is faster — so we don't have to decide." This is the jbra/jbsr
precedent generalized: encode the encoding decision in the compiler, retire the
human taste fight. Resolves ruling R-B structurally (the section.emp
repeated-add-vs-mulu question stops being a code-review argument and becomes a
cost-model fact).

**Corpus demand (sweep-proven):**
- Hand-derived shift-add chains for constant strides: ×66 (section/tile_cache),
  ×160, ×80 (plane rows) — someone derived each chain on paper; readers must
  re-derive to verify. `mul_const` keeps the "×66" intent visible and emits the
  chain mechanically.
- The section.emp runtime `sec_y × grid_w` repeated-add loop (C1-2 / C4-1 O3,
  "deliberate, no multiply" comment) — `mul_bounded` makes the choice the
  compiler's, with the bound in the signature instead of in the reviewer's head.
- The camera `lsl.l #16`-split fossil (fixed this round via pixels_to_coord) is
  the same family: transliterated arithmetic surviving because no construct owned
  the decision.

## 1 · Surface (m68k only, v1)

Two builtin comptime fns (compiler-owned — the cost model and chain search need
compiler internals; not expressible as a prelude splice):

```
mul_const(reg: Reg, n: const [, scratch: Reg])   // reg.l = u16(reg.w) * n
mul_bounded(dst: Reg, src: Reg in 0..M)          // dst.l = u16(dst.w) * u16(src.w), src ≤ M
```

- `n` (and `M`) are comptime ints, 0..$FFFF. Wider constants refuse loud
  ([mul.const-range]) — 16×16→32 is the m68k multiply domain; a 32-bit multiplier
  is a different (rarer) problem, deferred until a consumer exists.
- **Result contract (pinned):** identical to `mulu.w` for the full input domain —
  dst.l = zero-extended(dst.w) × n, all 32 result bits valid. Every candidate
  lowering must be exactly equivalent on all 2^16 inputs (analytic: k·x < 2^32
  always for k,x ≤ $FFFF, so a chain in .l ops after zero-extension is exact).
  No "close enough in the low word" variants in v1 — one contract, one meaning.
- **CC contract:** condition codes are CLOBBERED-UNDEFINED after the splice
  (lowerings differ). Declared through the ordinary clobber machinery so the
  write-analysis/checked-clobbers layer sees it; callers relying on CC after a
  mul get the standard diagnostics, not silence.
- **Scratch policy:** chains that need the original value twice (×66 = x·64 + x·2)
  need a second register. No hidden register allocation — if `scratch:` is
  absent, the candidate set is only {mulu, pure-shift sequences, shift+add-self
  forms that never need a copy}; with `scratch:` the full chain space opens.
  The chosen lowering may leave scratch clobbered (declared).

## 2 · Lowering candidates + cost model

Candidates for `mul_const(reg, n)`:
1. `mulu.w #n, reg` — baseline, always legal.
2. Shift-add/sub chain: zero-extend once, then a bounded search over
   left-to-right binary + factored forms (n = 2^a·m decompositions, ±1 forms for
   n near a power of two: ×63 = x·64 − x). Search is bounded and DETERMINISTIC
   (fixed candidate generators, fixed tie-breaks — same input, same chain,
   forever; goldens depend on this).
3. Degenerates: n=0 → `moveq #0`; n=1 → zero-extend only; n=2^k → shift.

Cost model: one documented static table of worst-case 68000 cycle counts for the
involved ops (mulu.w #imm worst-case, lsl/asl.l #m, add.l, sub.l, move.l, swap,
the zero-extend). Conservative worst-case only — no average-case guessing.
Tie-break: fewer bytes, then `mulu` (simpler). The table lives beside the
implementation and in the docs; it is part of the construct's contract because
byte output depends on it (a table change is a byte-changing event and rides the
golden gates like any other).

For `mul_bounded(dst, src in 0..M)`:
1. `mulu.w src, dst` — cost = worst-case constant.
2. Repeated-add loop — cost = worst case at src = M (loop overhead × M).
Chosen on worst-case comparison (the engine budgets worst frames, not average
ones). Expected outcome for the section.emp site (M=16): mulu wins — and that
verdict is then a printed fact, not an opinion. DEBUG builds assert src ≤ M
(the existing assert.w idiom); release trusts the declared bound.

## 3 · Acceptance bars (implementation parcel)

- Equivalence tests: every emitted chain proven against mulu semantics (unit
  test sweeps representative n including all corpus strides, ±1 forms, powers
  of two, degenerates; property = exact u32 equality over sampled + boundary x).
- **The byte-identity acceptance:** retrofit ONE existing hand-chain site
  (a ×66 stride) and prove the ×6 goldens byte-identical — the model must
  rediscover the hand-derived optimum. If it picks a different-but-cheaper
  chain, that's a byte-changing adoption and waits for a byte-changing parcel;
  the construct itself still ships (negative result recorded, adoption deferred).
- Negative probes: [mul.const-range], missing-scratch-when-chain-needs-one
  (falls back to mulu silently? NO — falls back silently is fine ONLY if mulu
  is legal, which it always is; the diagnostic surface is for out-of-range n
  and a scratch that aliases reg).
- Docs row in SPEC2 at the next unfreeze; emp-idioms.md entry (when to annotate
  bounds).

## 4 · Adoption plan (deliberately thin)

No mass retrofit. (a) The section.emp repeated-add site adopts `mul_bounded` in
a future byte-changing parcel (it will flip to mulu per the model — that IS the
R-B resolution, compiler-adjudicated). (b) Hand-chain strides adopt
`mul_const` opportunistically when their files are next touched, byte-identical
where the model agrees with the hand chain. (c) New content-era code uses it
freely from day one.

## 5 · Deferred / out of scope

- Z80 variant (no hardware multiply; chains/loops ARE the idiom there — a
  z80-side `mul_const` is plausible later; ledger row, waits for a consumer).
- 32-bit multipliers; signed `muls` variants (no corpus demand yet).
- Range-relaxed short results ("upper word unspecified") — rejected v1: two
  contracts for one name is a trap; revisit only with a measured need.
- Division analogue (`div_const` by reciprocal tricks) — noted, unruled.
