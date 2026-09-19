# 2026-09-18 — `MIXED-MAP-LOCATE`: a span that did not say which map could read it

Branch `parcel/mixed-map-locate`, off master `72348c9a`.
Reference tree: `AEON_DIR=/home/volence/sonic_hacks/.aeon-sigil-ref` at `ec640bcf`.

## The defect, established before the fix

`resolve_chained` (`crates/sigil-harness/src/native.rs`) builds ONE section list
out of BOTH front ends and then located every diagnostic raised over that list
through the `.emp` manifest's `SourceIndex` — the emp front end's index, and only
it. A `Span` is an id and a byte range; nothing in it says which map can read it.
AS file ids count the AS root and its `include` splices, `.emp` ids count the
scanned `.emp` files, and **both count from 0**, so feeding one map's span to the
other's index does not fail. It names a different file, confidently.

The rule was already written in this same file, on `BuildWarning::from_as`:

> the two location authorities are NOT interchangeable ... Feeding one map's span
> to the other's index does not fail — it names a DIFFERENT FILE, confidently.

**Red-first, a wrong PATH and not merely a wrong id** (commit `4d25307a`, whose
message carries this verbatim):

```
declared-chain: resolve_layout: 1 diag(s):
  /tmp/.tmpm0JIuj/second.emp:3:1: [Error] [layout.overlap] section `snd` overlaps `obj`
test result: FAILED. 0 passed; 1 failed; 0 ignored; 228 filtered out
```

## Reachability — measured, not argued

A temporary probe (a test target, run once against the reference tree and then
removed) over the shipped `sonic4` non-debug shape:

```
as_sections=1   distinct AS span ids=2 (both file ids, 0 expansions)
emp index covers 203 ids;  AS map holds 2 files
AS file ids the emp index would answer for: 2 of 2  -- ALL of them
  id 0: AS map says games/sonic4/game_root.asm(1):1
        emp index says engine/compression/s4lz.emp:1:1
  id 1: AS map says engine/debug/debugger.asm(1):1
        emp index says engine/compression/zx0_resume.emp:1:1
```

So it was live on the shipped shape. On this shape the AS side's spans reach
`resolve_layout` through `Section::equ_syms` rather than fragments (the AS
residual contributes equates), and `sigil_link::relax::unresolved_equ_diag`
raises at `eq.span` — the trigger is an undefined symbol or an equ cycle in the
residual `.asm`, which is an ordinary failure and not a corner.

## Three consumers, not two

The brief named `true_bases_by_index`'s width-flip report and the
`resolve_layout` failure. There is a third and a fourth:

- `build_rom_chained_with_listing`'s `link` failure, which reads the same single
  authority out of `ChainedResolve.sources`;
- `collect_warnings(&sources, &[&adiags], None)` over the link-assert warn tier.

`resolve_frozen_sections`'s `&|_| None` sink was left alone as instructed; it is
booked (`FROZEN-RESOLVE-LOCATE-SINK`).

## The fix: the class, not the instances

`ChainSources` holds both authorities over ONE id space.

- `AsSide` now KEEPS the AS front end's `SourceMap` instead of dropping it.
  Dropping it is what made the mislocation unfixable rather than merely wrong:
  with the AS map gone, the `.emp` index was the only authority left in scope.
- `ChainSources::join(emp_index, as_map, as_sections, emp_sections)` returns the
  authority AND the chained section list, with every AS-side span rebased by the
  `.emp` index's length. Because the list a caller resolves comes out of the
  constructor, there is no second list to use instead: the seam cannot be
  half-applied. The concatenation order lives there too, beside the id rule it
  has to agree with.
- `rebase_section`'s `match` over `Fragment` is exhaustive with NO `_` arm, so a
  variant added later with a span of its own does not compile until it is named.
- Expansion ids are NOT rebased: they are in their own high range by
  construction (`SourceId::is_expansion`, added to `sigil-span`), only the AS
  front end makes them, and the AS map reads them at their own numbers. Through
  the `.emp` index they named nothing at all; they now render asl's call-site
  trail. The no-source id is in that range too and still answers `None`.
- `BuildWarning::located` makes the authority an ARGUMENT. `from_as` and the new
  `collect_warnings_located` are that constructor with a particular authority
  named, so a caller has to say which map reads its spans rather than inheriting
  whichever one is in scope.

Not changed: how either front end SPELLS its answer. `.emp` keeps
`path:line:col`, AS keeps asl's `file(line):col` with its trail — the split ruled
in `docs/OVERSEER.md`, and the one `from_as` has always produced.

## Gates — the family, not the delta

`crates/sigil-harness/src/native.rs`, `mod mixed_front_end_location_tests`, ten
tests. Offsets are DERIVED from the fixture text (`third_line(ASM_TEXT)`), not
counted into a literal — the first cut of this gate copied the `.emp` fixture's
line-3 offset onto the `.asm` fixture and asserted a column it never meant to.

| gate | what only it can see |
|---|---|
| `an_as_side_layout_diagnostic_names_its_own_asm_file` | the defect |
| `an_emp_diagnostic_still_names_its_own_emp_file` | the case that already worked |
| `mixed_diagnostics_each_reach_their_own_front_end` | two spans with the SAME raw id, one per side, in one render |
| `an_as_expansion_span_renders_its_call_site_trail` | the ledger's "names nothing" half |
| `an_unattributed_diagnostic_renders_the_same_bare_line` | the no-source id still degrades |
| `with_no_as_side_the_text_is_exactly_what_the_emp_index_renders` | single-front-end text invariance |
| `rebasing_separates_the_two_id_spaces_without_merging_anything` | injectivity (the warn tier's dedup key) |
| `a_joined_space_reaching_the_expansion_range_is_refused` | loud on unmeasurable |
| `the_seam_returns_the_as_half_rebased_and_the_emp_half_untouched` | the SEAM |
| `joining_rebases_every_span_an_as_side_section_carries` | every span-carrying `Fragment` variant |

### Red-first evidence, mutation shown on disk

Both mutations were applied to the COMMITTED tree (clean before, `git diff`
quoted, restored with `git checkout --` from that committed baseline).

1. **Drop the rebase from the seam** (`-for sec in &mut as_sections { chain.rebase_section(sec); }`):
   `8 passed; 2 failed` — `the_seam_returns_...` and `joining_rebases_...`. The
   other eight construct their AS spans already-rebased, so they gate the
   ROUTING and cannot see the wiring; that separation is the point.
2. **Route everything to the `.emp` index** (`if (span.source.0 as usize) < self.as_base as usize` -> `if true`):
   `5 passed; 5 failed`.

Neither mutation left the suite green, so the runner is executing what was
patched.

## Byte-neutrality — established, not asserted

Every shipped and off-canonical shape rebuilt with the release `sigil` linked
from this branch tip (`target/release/sigil`, built 20:57, tip `84afa765`) and
compared to `crates/sigil-harness/golden/`:

| shape | bytes | crc32 | golden crc32 | |
|---|---|---|---|---|
| s4 | 820209 | `91c46c94` | `91c46c94` | identical |
| s4.debug | 846509 | `8a378de6` | `8a378de6` | identical |
| demo | 96863 | `1c7a34d3` | `1c7a34d3` | identical |
| demo.debug | 103185 | `72e405a5` | `72e405a5` | identical |
| config_a | 846881 | `a300dafc` | `a300dafc` | identical |
| config_b | 620727 | `06bad0ef` | `06bad0ef` | identical |
| lean | 773136 | `302ea1ee` | `302ea1ee` | identical |

Positive control on the comparison itself: `s4.bin` against the `s4.debug.bin`
golden reports DIFFERS, so the check can go red. No golden, pin, or
`repin.toml` was touched.
