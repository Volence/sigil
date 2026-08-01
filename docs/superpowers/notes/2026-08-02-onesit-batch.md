# The language-round one-sitting batch — L3 · L4 · L12 (+ bookkeeping)

**Parcel:** the Tier-3 one-sitting batch (Opus porter; overseer countersigns/merges
— NOT merged/pushed). **Repos:** sigil `lang-onesit` · aeon `lang-onesit`
(paired worktrees). **Date:** 2026-08-02.
**Agenda:** `specs/2026-08-02-language-round-agenda.md` Tier 3.
**Ledger:** `notes/2026-08-02-language-round-ledger.md` §L3/§L4/§L6/§L11/§L12/§L13.

Three ruled BUILD items + the batch's bookkeeping. **Six-target byte-identical to
chain-19**; strict **2941 → 2959** (+18 new tests, 0 failed); refreeze OK; repin
`pins.rs unchanged`.

---

## L3 — per-DATA-item `(align: N)` alignment (option (b))

**The wall.** conv-i8 folded five golden `embed()`s into `compression_selftest.emp`
AFTER a proc holding `bra.s`/`beq.s` relaxables; a top-level `align` there fired
`[align.provisional]` (SPEC2 D2.29's v1 loud stop) because the pad position can't
resolve until the proc's size pins. The aligns were dropped; five comptime
`ensure(_Vec_X.len % 2 == 0, …)` guards stood watch.

**The design choice — option (b), NOT (a).** The agenda ruled BUILD without
picking. I built **(b) a per-data-item `(align: N)` attribute**, and REJECTED
**(a) making `align` participate in the relaxation fixpoint**:

- Option (a) is the general fix but its regression surface is *every relaxable
  module*. The whole layout/link fixpoint (`sigil-link::relax`, `replay_extent`,
  `final_size`, the overlap / org-overrun / bank checks, the listing filter)
  assumes a fragment's length is POSITION-INDEPENDENT (a function of its relax
  *rung* only). An adaptive align pad is POSITION-dependent — a genuinely new
  fragment shape threaded through the whole joint fixpoint. Too broad for a
  one-sitting item, and it would change how *all* existing aligns lower.
- Option (b) is additive and local: a new `align: Option<Expr>` on `DataDecl`,
  lowered through the EXISTING `emit_align_pad` machinery. **Bare `align` is
  untouched** — its provisional wall still stands (documented). So every existing
  module is byte-identical by construction; the new path only appears when a
  source writes `data X (align: N)`.

**Why the eager pad is CORRECT past relaxable code (the crux).** The pad is
computed from the lowering-baseline offset, which counts every relaxable at its
smallest rung. Every m68k instruction encoding is EVEN-length, so every relaxation
delta is a multiple of 2 → the PARITY of any position is invariant under
relaxation. Word alignment (`N = 2`) — the corpus demand — is therefore
relaxation-INVARIANT: the baseline pad equals the final pad no matter how the
`CompressionSelfTest` proc's branches settle. `emit_align_pad` also records a
`__align$` congruence anchor + a link-time `anchor % N == 0` assert that runs
POST-fixpoint against the FINAL address — the backstop, so an item can never be
*silently* misaligned.

Generalized: `relax_granularity(cpu)` = 2 for m68k, 1 for Z80 (a `jr`→`jp` grows
by 1). A per-item `(align: N)` is exempt from the provisional wall iff `N` divides
that granularity. So `(align: 2)` on 68k lays a real pad past relaxables;
`(align: 4)` past a relaxable still refuses (`[align.provisional]`) — that residual
IS the option-(a) demand, left ledgered.

**Byte-neutrality.** Every golden blob is even-length today, so every pad is ZERO
bytes — the six targets are byte-identical. A future odd blob now gets ONE pad
byte instead of a build failure (the point). The `__align$` anchors are dropped
from the deb2 appendix by the listing demangler (`__align$…` filter), and the
congruence asserts are tagged `[layout.align]` (structural) → excluded from
`guard_assert_count` (no twin-guard count shifts).

**Shipped.**
- sigil `ast.rs` — `DataDecl.align: Option<Expr>`.
- sigil `parser.rs` — the data attribute list accepts `(max_size: E, align: N)`
  (either order, trailing comma tolerated); unknown key is a finite error.
- sigil `lower/mod.rs` — `relax_granularity`, `lower_data_align`, the
  `emit_align_pad` `relax_invariant` param (bare `align` passes `false`), the
  per-item pad emitted BEFORE the item's `here`/label.
- aeon `compression_selftest.emp` — the 5 `ensure(_Vec_X.len % 2 == 0)` guards
  RETIRE; each `pub data CSelf_*` gains `(align: 2)` and inlines its `embed()`
  (the 5 now-purposeless `const _Vec_*` bindings dropped).

**Probes (`tests/align.rs`, `tests/parser_decls.rs`).** pad-before-item;
zero-pad-when-aligned (byte-neutral); **survives a word relaxable with a REAL pad
byte** (odd base past a `bra`); `(align: 4)` past a relaxable STILL refuses;
without-relaxable pads like a bare align; structural `[layout.align]` tag;
placement-drift fails the congruence assert. Parser: `(align: N)` with/without a
type, `max_size`+`align` both orders, trailing comma, unknown-key finite error.
No contact with L10/L14 (no fold-opportunistic items touched).

## L4 — module-scope `@allow("layout.odd-field")`

`check_struct_odd_fields` (layout.rs) emitted `[layout.odd-field]` UNCONDITIONALLY;
the 5 Z80 sound structs (DacSample/FmPatch/SfxHeader/SfxChannel/SeqChannel, all in
`engine.sound_constants`) have intentionally-unaligned words and no way to declare
that.

**Scope decision — MODULE scope, one rule not two.** The DATA-side
`[layout.odd-item]` allow (`lower::allows_lint`) is MODULE-scope (it reads
`file.attrs`). To "mirror that path … one rule, not two", odd-field now honors the
SAME module-scope `@allow`. A struct-DECL-level attribute was rejected: structs
carry no attribute grammar today, and adding one would make odd-field
struct+module while odd-item stays module-only — a scope asymmetry (two rules).
The 5 structs live in one dedicated Z80-records module, so a single module-scope
`@allow` is precisely targeted.

**Shipped.**
- sigil `eval/mod.rs` — `Evaluator.allowed_lints`, populated from `file.attrs`
  in `with_file_and_ambient`; `module_allows_lint(id)`.
- sigil `layout.rs` — `check_struct_odd_fields` early-returns under
  `@allow("layout.odd-field")`.
- aeon `sound_constants.emp` — `@allow("layout.odd-field")` at module scope with a
  present-tense intent comment (the packing is intentional, Z80-native records).

**Probes (`tests/eval_layout.rs`).** with the allow → silent; an allow naming a
DIFFERENT lint → still fires. Zero bytes (warning-tier).

## L12 — braced `use` multi-line / trailing-comma form

K3 hit `parse error: expected imported name, found RBrace` on a braced multi-line
`use` with a trailing comma: after eating the last comma the loop unconditionally
expected another name. Newlines were already skipped; only the trailing comma
broke.

**Shipped.** sigil `parser.rs` `use_decl` — after eating a comma, skip newlines
and break on `}` (trailing comma). Empty braces STAY an error (the loop still
demands a name on the first pass). Adopted at one natural site: aeon
`act_descriptor.emp`'s 9-import `ojz_block_dicts_act1` `use` (the exact K3 context)
folds to the multi-line braced form — byte-neutral (imports are compile-time).

**Probes (`tests/parser_decls.rs`).** trailing comma; multi-line with/without a
trailing comma; empty braces still an error.

## Bookkeeping

- **L6 (`[u8; _]` sugar) DECLINED** and **L13 (`parallax_combine` sugar) DECLINED**
  — gap-ledger rows (conv-h2, conv-g) annotated with the agenda ruling + date +
  citation. No kill-list rows reference either.
- **L11 (bare vs `extern("…")` data-label refs) DOCUMENTED AS IDIOM** — no
  existing `.emp` idioms/authoring doc existed, so `notes/emp-idioms.md` is seeded
  with the rule (bare = cross-module link ref; `extern("…")` = same-module
  forward/back ref in data position) plus the L9 `offsets` cross-module
  bare-target example that already relies on it. SPEC2 left untouched (v1-freeze).
- The overseer's transient `2026-08-02-onesit-orphan-wip.patch` rescue file
  (committed to master when an earlier attempt's edits landed in the main checkout)
  is removed in this branch — the proper work supersedes it. **Flag for the
  overseer:** origin/master advanced cc0e2e3c → 0481d852 with that rescue commit;
  this branch is based on 0481d852 and its source is chain-19-identical.

## Gates

- **Strict** (`SIGIL_STRICT_GATE=1`, AEON_DIR + SIGIL_EMIT): **2959 / 0 / 4**
  (baseline 2941 + 18: align 7, parser_decls 9, eval_layout 2).
- **Six targets byte-identical to chain-19:** s4 5f72b9c3/412134 · s4.debug
  e6171a80/421970 · demo 55b70266/90576 · demo.debug 6487a47c/93073 · config_a
  818bb109/422321 · config_b 947e4c57/303555.
- **refreeze --check:** OK (tip `l1-p2-game-contract-conversion`, chain len 19).
- **repin --check:** `pins.rs unchanged`.
- **clippy:** clean on the changed lines (fixed a stray duplicate `#[allow]` +
  a `%…==0` → `is_multiple_of`).

## step-3 / step-5 / neither

- **step-3 (retrospect):** L3 surfaced the parity-invariance argument as the clean
  boundary between the local per-item fix (option b, shipped) and the general
  fixpoint fix (option a, still ledgered for `N > granularity` past relaxables).
  L4 confirmed the odd-item/odd-field allow paths should share ONE module-scope
  rule. L12 is pure cosmetic grammar.
- **step-5 (hardening):** the per-item `(align: N)` REPLACES an approximate guard
  (`.len % 2 == 0`, which only proves each blob even) with a real self-adjusting
  pad + a link-time congruence assert on the actual address — soundness up, and a
  future odd blob is made correct instead of merely caught.
- **neither:** the L12 corpus adoption (act_descriptor) is readability only.
