# `.emp` authoring idioms

A running reference of `.emp` conventions that are coherent-by-design and worth
knowing, but are not (yet) language changes. Ruled during the campaign; each
entry names the demand that surfaced it. This is a NOTES doc, not the spec —
SPEC2 stays v1-frozen; spec consolidation is a separate future pass.

## Data-label references: bare = cross-module, `extern("…")` = same-module

In a DATA initializer (a `data`/`offsets` cell, a struct-literal field, a call
argument in value position), a link-label reference is spelled two ways, and
which one you use depends on WHERE the label is defined:

- **A cross-module label → a BARE identifier.** A name that this module does not
  define resolves as a deferred link symbol: the frontend records the NAME by
  shape and leaves it for the linker. Example — a jump/offset table whose targets
  live in other modules:

  ```
  // engine.player.offsets — targets defined in sibling player modules
  pub offsets Player_States {
      Idle:    Player_Idle,      // bare: cross-module link label
      Walk:    Player_Walk,
      Jump:    Player_Jump,
  }
  ```

  The `offsets` construct emits each row as `dc.w target - Player_States`, exactly
  the `extern("target") - extern("Player_States")` difference form by hand — the
  bare cross-module target is accepted like any local label and folded at link
  (see `2026-08-02-l9-offsets-cross-module.md`).

- **A SAME-module label → `extern("Name")`.** A bare identifier naming a label
  defined in THIS module fails `unknown name` (a data initializer does not
  resolve local labels as bare barewords); spell it `extern("Name")` so it
  resolves as a forward/back link reference:

  ```
  // A page table pointing at blobs defined below it in the same module
  pub data OJZ_Act_Pool_PageTable: [*u8; 3] = [
      extern("OJZ_Act_Pool_Page0"),   // same-module: extern("…")
      extern("OJZ_Act_Pool_Page1"),
      extern("OJZ_Act_Pool_Page2"),
  ]
  ```

**Why the asymmetry is coherent, not a wart.** A bare name is the natural
spelling for "a symbol I don't define — resolve it at link"; `extern("…")` is the
explicit "I mean the label, not a value" marker for a same-module symbol whose
name would otherwise be an unresolved local. The rule is one sentence and reads
the same at every site. (Surfaced by K3 run A — `2026-08-01-k3-run-a.md` — which
hit it on the `OJZ_Act_Pool_PageTable` page pointers and a block-blob dedup alias;
ledgered as L11 and DOCUMENTED-AS-IDIOM by the language round,
`specs/2026-08-02-language-round-agenda.md` Tier 3. Revisit as a grammar change
only if the asymmetry keeps biting.)

## Multiplying: name the constant, let the compiler pick the encoding

Demand: the lens sweep's R-B (mulu-vs-repeated-add) and the corpus's
hand-derived stride chains (×66/×80/×160) — every one a paper derivation the
reader had to re-verify. Ruled 2026-08-03; shipped in the mul-lowering parcel.

- **A constant multiplier → `mul_const dN, #n[, dScratch]`.** The intent
  (`×66`) stays in the source; the compiler picks shift-add chain vs `mulu` by
  the M68000UM cycle table. Contract: `dN.l = u16(dN.w) × n` — exactly
  `mulu.w`'s, all 32 result bits valid; condition codes are
  clobbered-undefined. Grant a scratch when you can spare one: the general
  chains need the original value twice, and without a scratch the candidate
  set shrinks to `mulu` + pure shifts (no hidden register allocation, so the
  grant is yours to make). The scratch may come back clobbered — declare it in
  `clobbers(...)` like any write; the lint sees the chain's real instructions.

- **A data-dependent multiplier → `mul_bounded dDst, dSrc, #M[, dScratch]`**,
  where `M` is the largest value `dSrc` can hold (inclusive). The bound is
  MANDATORY — an unbounded operand's worst case is undecidable and the
  construct refuses rather than guesses. Choose `M` from the same fact a
  module-level `ensure` already states (grid width, table count); the cost
  decision is worst-vs-worst, and for any real bound (`M ≥ 3`) it picks
  `mulu`. `dSrc` is clobbered-undefined after the splice under EITHER
  lowering — never rely on it.

- **A word-result stride → `mul_const.w dN, #n[, dScratch]`** (the suffix IS
  part of the name). Contract: `dN.w = (u16(dN.w) × n) mod 2^16`; the **upper
  word of `dN` is UNDEFINED** after the construct. This is the idiom for a
  stride into an `adda.w` (the section/tile_cache/plane ×66/×80/×160 chains):
  the product fits a word by construction, so the range is YOUR obligation —
  carry it exactly where the corpus already does, the module-level `ensure` —
  and the compiler emits the two-power shift-add chain that beats `mulu` (no
  zero-extend to pay for). Spell bare `mul_const` (the long form) only when you
  need the full 32-bit product; `.l` is refused (a second name for one meaning).

- **When NOT to use them:** the two repeated-add loop sites (section.emp
  `.gxy_mul`, tile_cache.emp `.mul_loop`) are `mul_bounded.w` shapes, but the
  cost model picks `mulu` at their bound — adopting there is byte-CHANGING and
  waits for a deliberate byte-changing parcel (gap-ledger loop-site row).
