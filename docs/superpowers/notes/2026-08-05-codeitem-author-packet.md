# CodeItem-AUTHOR — parcel packet (2026-08-05)

Porter packet for the RULED design `specs/2026-08-05-codeitem-author-design.md`
(sr-contracts pass-2 panel ask §8; the three-answers ledger row). Sigil-only;
branch `codeitem-author`, two commits on master `2c6507b8` (chain 47):
`bdca8335` (the parcel) + `53edc9ea` (the lens-panel dispositions, including
a Lens-C-demonstrated must-fix). No merge-state claims — the overseer owns
the queue.

## 1 — The field and the setting sites

`CodeItem::Instr` gains `author: ItemAuthor` (`value.rs`):

```
User | AssertDesugar | Context { name, phase: Acquire|Release } | EntrySynth
     | Splice { template }
```

Spans untouched — authorship is a semantic fact, not a location fact; every
diagnostic keeps pointing where it pointed. The field emits nothing
(`lower/code.rs` streams instructions with `..`-tolerant patterns).

Construction sites: 5 in the lib (`eval/asm.rs` ×3 — `lower_instr_to_item` ×2 +
the trap arm; `eval/builtins.rs` ×2 — the `pad_to_cycles` jr/nop pads), all
stamping the evaluator's scoped `item_author` (default `User`; the pads are
`User` at construction and picked up as `Splice { "pad_to_cycles" }` at the
statement-call boundary). 6 more in tests (`cycle_budget`/`flag_check`/
`preserves`/`z80_cycles` in-crate helpers, `eval_code.rs`), all `User`. Fewer
sites than the spec's ~20/54 estimate because most `CodeItem::Instr {` matches
in the tree are destructuring patterns, which `..` absorbs.

Setting sites, exactly the ruled four:

1. **Assert desugar** (`eval/asm.rs::lower_assert`): the evaluator's
   `item_author` is swapped to `AssertDesugar` around the recursive lowering of
   the 11-step expansion and restored after. The expansion contains no
   statement-call/splice forms (verified — `build_assert_expansion` emits only
   labels, instrs and `dc.b`), so the stamp cannot leak onto user code.
   `raise_error`/`raise_exception` expansions are deliberately NOT stamped:
   the spec's site list does not name them, and they contain zero
   SR-DESTINATION writes (the frame's `move.w sr,-(sp)` is a read), so no
   exemption pressure exists (§7 of this packet, step-3 findings).
2. **Context splicer** (`eval/asm.rs::splice_context_code`): spliced
   acquire/release items are authored `Context { name, phase }` at emission —
   no new analysis; the splicer already plants the marks.
3. **Entry synthesis**: `ItemAuthor::EntrySynth` has ZERO live constructions,
   and that is the pinned truth — the synthetic entry is bare `use` lines
   (`sigil-harness/native.rs::synthetic_entry_emits_no_code` fails the day the
   synthesis emits anything else, naming the stamping duty). The `SourceId`
   exclusion in `collect_warnings` stays: `use`-statement diagnostics are not
   `Instr` items, and the field is Instr-only by ruling (§1).
4. **Template splices** (statement call, bare statement call, `{expr}` — all
   three in `eval/asm.rs`): `reauthor_user_items` re-authors the template's own
   lines as `Splice { template: <callee path> }`. Only `User` items are
   re-authored — the splice-boundary rule — so an assert inside a template
   stays `AssertDesugar` and a whole `with` bracket inside a template keeps its
   `Context` authorship through the outer splice. Carried, not consumed (§5;
   ledger row opened).

## 2 — Each redirected obligation and where it now lives

The §2 invariant: authorship never exempts, it REDIRECTS. The exemption ledger,
obligation by obligation:

| Exempted author | Exempt from | The receiving proof | Pinned by |
|---|---|---|---|
| `AssertDesugar` | consumer's `[proc.sr-undeclared]` (`check_clobbers`) | the desugar's push/pop balance at the EMISSION site | `diag_desugar.rs::the_assert_expansion_is_desugar_authored_and_sr_balanced` (first instr = `move.w sr,-(sp)` save, last = `move.w (sp)+,sr` restore, restore = the expansion's ONLY SR-destination write, every instr desugar-authored) |
| `Context {..}` | consumer's `[proc.sr-undeclared]` | the SR round-trip proof against the DEFINITION's spliced code — `lower_with` runs `sr_writes_round_trip` over the acquire ∪ release actually spliced at EVERY bracket (the halves evaluate per site, so each site's stream earns its own exemption — the Lens C fix, §6); only the REPORT is deduped (`sr_reported_contexts`, one firing per context per evaluator, anchored at the DECLARATION span; cross-proc duplicates collapse on the `(level, message, span)` dedup key). A half that fails to splice as Code skips the check for that bracket (no spurious stacking on the reported error) | `lower_proc.rs::a_context_that_never_restores_sr_fires_at_its_own_definition`, `..._a_release_that_re_masks_after_restoring_fires_at_the_definition`, `each_context_is_judged_once_on_its_own_round_trip`, `a_context_whose_halves_diverge_per_site_is_checked_at_every_site` |
| `EntrySynth` | (nothing — never constructed) | the synthesis emits no instructions | `native.rs::synthetic_entry_emits_no_code`; the census gate PANICS if an EntrySynth SR write ever appears |
| `User`, `Splice {..}` | NOT exempt | the lint itself; the id-set baseline pins every row to zero surviving firings | `warn_tier_corpus.rs::warn_tier_lint_ids_match_the_frozen_baseline`; `lower_proc.rs::a_spliced_templates_sr_write_is_charged_to_the_consumer` |

Retired with their obligations relocated: `check_clobbers`' `ContextMark`-range
walk and `region_round_trips_sr` (the recognizer `sr_writes_round_trip`
survives, now `pub(crate)` and shared by `check_preserves_sr` and the
definition check — the two readings still cannot drift). `check_clobbers` lost
its `regions` parameter (nothing else in it read them).

Consumer-side negative controls kept and passing: a hand-written SR write
inside a bracketed BODY is `User`-authored and still fires
(`a_hand_written_sr_write_inside_the_bracketed_body_still_fires`); the user's
lines around an assert stay `User`
(`diag_desugar.rs::lines_around_an_assert_stay_user_authored`).

Behavior deltas vs the range walk, both DESIGNED (§2): a non-round-tripping
context now fires ONCE at its declaration instead of once per spliced SR
write at the consumer (the old `..._still_charges_its_consumer` tests are
rewritten to pin the new address — 3 firings became 1, at the decl span,
naming the context), and the REPORT is per-context while the CHECK stays
per-bracket (a second bracket of the same failing context is still checked
but adds no second firing — one declaration, one report; the per-bracket
check is the Lens C fix, §6).

## 3 — The DEBUG surface: numbers per shape

`[proc.sr-undeclared]` firings on the corpus (dedup'd build-report counts):

| shape | before (chain 47, base `2c6507b8`) | after |
|---|---|---|
| sonic4 debug | 42 | **0** |
| demo debug | 41 | **0** |
| config_a | 42 | **0** |
| the four `DEBUG == 0` shapes | 0 | 0 |

The spec quotes 43/42/43 — those were the chain-44 numbers in the ledger row;
the counts had moved by one per shape with ordinary corpus work since (exactly
the count-churn the id-set gate was designed not to pin). Both measured this
parcel: the before-numbers from the base-commit baseline builds, the
after-numbers from the post-change builds. Every OTHER id count is unchanged
in every shape (debug shapes: 60→18 / 59→18 / 60→18 total, delta exactly the
sr class; plain shapes 19/19/19/18 unchanged).

**The baseline edit (the ONE deliberate one):** `DEBUG_ONLY_LINTS` is deleted
and all seven `WARN_ID_BASELINE` rows are empty. Explained in the baseline's
own doc: the retirement has teeth in the id-set gate's favorite direction —
with the DEBUG rows empty, the FIRST hand-written undeclared SR write in a
debug-gated proc makes `proc.sr-undeclared` APPEAR in that row and fails the
gate loudly. The hiding place (a new firing joining a 42-strong crowd) cannot
re-form; the "never-examined DEBUG sr surface" open item closes with numbers.

**The property-test replacement:** the source-line-reading
`every_surviving_sr_firing_is_the_assert_desugar` (which re-read aeon source
text to prove every firing was the desugar) is REPLACED by the typed
`debug_shape_sr_writes_are_author_checked`: `ContractReport` gains an
`sr_writes: Vec<(proc, ItemAuthor, Span)>` census (every write-form
SR-destination instruction in the walked 68k procs), and the gate walks the
three `DEBUG == 1` shapes asserting every SR write's author has a receiving
obligation — `AssertDesugar`/`Context` counted (exempt, proofs above),
`User`/`Splice` charged by the lint (id-set gate pins zero survivors),
`EntrySynth` a panic (an authored effect with no obligation home). Non-vacuity
guards: `seen > 0` on desugar-authored items, per-shape non-empty census,
`walked_debug_shapes == 3`. Measured census: **132 desugar-authored + 30
context-authored SR writes** across the three DEBUG shapes (132 > the 125
old firings because the census also sees asserts in procs whose contracts
cover `sr` or that declare no `clobbers` — the lint's own preconditions).
A future `ItemAuthor` variant fails the gate's match at compile time, so a
new author cannot ship without declaring where its obligation lands.

## 4 — Bars

All run in the b3 worktrees (sigil `bdca8335`, aeon ref `2ccb40f`), binaries
rebuilt at HEAD before every corpus-facing step.

1. **Byte bar ×7**: baseline PROVEN at `2c6507b8` first (all seven golden
   targets rebuilt in `capture_goldens.sh` order, `cmp`-identical), then
   re-proven at `bdca8335` AND at convergence `53edc9ea` (binaries rebuilt at
   each head): **all seven `cmp`-identical** every time (s4, s4.debug, demo,
   demo.debug, config_a, config_b, lean). No target moved.
2. **`refreeze --check`**: OK, tip `sst-fold`, chain len **47** (base, parcel
   and convergence). **repin**: `pins.rs unchanged` (all three).
3. **Warn tier**: §3 above — the DEBUG-row edit is the one deliberate baseline
   change; id sets otherwise unchanged, per-shape counts stated.
4. **Full strict** (`SIGIL_STRICT_GATE=1`, release, workspace):
   - base `2c6507b8`: **3317 passed / 0 failed / 4 ignored**; `#[test]` count
     3321 = 3317 + 4. ✔
   - parcel `bdca8335`: **3321 passed / 0 failed / 4 ignored**; `#[test]`
     3325 = 3321 + 4. ✔
   - convergence `53edc9ea` (+ this packet): **3323 passed / 0 failed /
     4 ignored** (exit 0, 311 test binaries, failures-first review clean);
     `#[test]` 3327 = 3323 + 4. ✔
   - Delta arithmetic: +6 tests base→convergence, every one named. Added:
     `the_assert_expansion_is_desugar_authored_and_sr_balanced`,
     `lines_around_an_assert_stay_user_authored`,
     `an_assert_does_not_fire_sr_undeclared_on_its_proc` (diag_desugar.rs),
     `synthetic_entry_emits_no_code` (native.rs),
     `a_context_whose_halves_diverge_per_site_is_checked_at_every_site`,
     `a_spliced_templates_sr_write_is_charged_to_the_consumer`
     (lower_proc.rs, lens dispositions). Removed:
     `every_surviving_sr_firing_is_the_assert_desugar`; added its replacement
     `debug_shape_sr_writes_are_author_checked` (net 0). Renamed (net 0):
     `a_context_that_never_restores_sr_still_charges_its_consumer` →
     `..._fires_at_its_own_definition`,
     `a_release_that_re_masks_after_restoring_still_charges_its_consumer` →
     `..._fires_at_the_definition`,
     `each_region_is_judged_on_its_own_round_trip` →
     `each_context_is_judged_once_on_its_own_round_trip`.
5. **Lens panel**: §6 below.

## 5 — Ledger

Closed: the three-answers row (sr-lane 2026-08-04 / sr packet §8 / Lens A19),
the t16 `[proc.sr-undeclared]`-fires-on-asserts row (its kill condition —
"lint exemption ships → 0" — fired), the warn-tier "43 of 52 are sigil
warning about its own desugar" row, and the property test's source-reading
brittleness (replaced, §3). The 2026-08-04 warning-tier triage note's §9
item 2 is marked DONE with the mechanism named.

Opened/updated: the perturbation-set row is DEFERRED WITH A NAMED TRIGGER
(§4 of the design — demand-parked like Z80 `VALID_CCS`; trigger = the first
context whose acquire writes a register it does not restore, with the author
field as the ready substrate); a new `Splice { template }` carried-not-consumed
row pointing at the t21 A1 `-> Code` fn-contract ask (the row-1551 parcel
starts whole). From the lens panel: the zero-instance
Splice-inside-a-context-half proven-but-charged row (OPEN), the
divergent-halves row (CLOSED in-parcel by the every-bracket fix), and the
pre-existing recursive-context stack-overflow row (OPEN, fix shape named).

## 6 — Lens panel (3 fresh read-only lenses; all dispositions in `53edc9ea`)

**Lens C (soundness adversary, pointed at §2 as the spec directs) — THE CATCH
OF THE PARCEL: an authored-but-unchecked effect path, CONSTRUCTED AND
DEMONSTRATED.** A context's acquire/release evaluate per `with` site in the
consumer's env, and a comptime fn's param is in that env — so
`context masked { acquire = asm { if n == 1 { move.w sr,-(sp) } move.w #$2700, sr } … }`
called through `gate(1)` then `gate(0)` in ONE proc round-trips at site 1
(check passed, name inserted) and splices a mask-with-no-save-no-restore at
site 2 — which the once-per-name dedup NEVER CHECKED while stamping it
`Context`-authored and exempt. Probe output: zero diagnostics on a
`clobbers()` proc that permanently masks interrupts; the control (bad call
first) fired as designed. Exactly the exemption-without-a-receiving-contract
§2 forbids. FIXED per the lens's disposition: the round-trip check runs at
EVERY bracket over that site's actually-spliced ranges; `sr_reported_contexts`
dedups only the firing. Regression-pinned
(`a_context_whose_halves_diverge_per_site_is_checked_at_every_site`, attack
order). Corpus behavior identical — site-independent contexts round-trip at
every site. Lens C also verified sound, with evidence: the `insert() &&`
short-circuit order, same-name context shadowing, author composition under
nesting (only `User` re-authored; the outer check reads the combined range,
which errs toward firing — safe polarity), the AssertDesugar stamp window
(the expansion emits only Instr/Label/`dc.b` — no statement form can pull
user Code inside the stamp; no raise path contains an SR-DESTINATION write,
so their non-stamping is sound), const-memo Code cannot acquire Context
authorship (splices mutate an owned clone), and the census gate's match is
wildcard-free (a new `ItemAuthor` variant fails compile). Secondary find,
ledgered not fixed (a new diagnostic id is outside this parcel's §5 ruling):
a SELF-REFERENTIAL context (`acquire = asm { with c { … } }`) crashes the
compiler with a native stack overflow — pre-existing (B′-1 splicer shape),
loud not unsound; row filed with the fix shape (contexts-in-progress guard).
Its suggested Splice-charged pin was added
(`a_spliced_templates_sr_write_is_charged_to_the_consumer`).

**Lens B (corpus-pattern/behavior)** — no must-fix. Verified: exemption
coverage is EXACT vs the old range walk for plain-asm brackets on today's
corpus (both live contexts are splice-free literals; a bracket inside a
template keeps `Context` authorship through the outer splice boundary);
census predicate byte-identical and single-sourced with the lint's
(`writes_dest_register` + last-op `Sr`); all ~35 `CodeItem::Instr` pattern
sites `..`-tolerant, exactly the two new sites bind `author`, and no non-test
code compares CodeItems by equality; `pad_to_cycles` pads cannot reach a
buffer un-re-authored (all four Code→CodeBuf boundaries re-author). Filed:
the zero-instance proven-but-charged shape (SR traffic entering a context
half via a NESTED splice stays `Splice`-authored — the definition proof
passes but adopters are still charged; false-positive direction, ledgered
OPEN). Its abstract version of the divergent-halves gap was subsumed by the
Lens C fix (row closed). Disposed in-parcel: the "once per context" comment
overclaim (the check is per-evaluator; the collector dedup delivers
one-per-context — comments corrected, then superseded by the every-bracket
fix), and the Poison-half noise (a bracket whose half failed to splice as
Code skips the check instead of stacking a spurious non-round-trip).

**Lens A (ceremony/clarity)** — 1 must-fix: a dead doc pointer to the deleted
`region_round_trips_sr` (`check_preserves_sr_ccr`) — repointed. Should-fixes,
all taken: change-history narration rewritten present-tense
(`warn_tier_corpus.rs` baseline/`corpus_warnings`/census-gate docs,
`lower/proc.rs` step-4 comment); test renamed
`an_assert_does_not_fire_sr_undeclared_on_its_proc`; `reauthor_user_items`;
orphaned `Region::in_release` deleted (`in_acquire`'s doc records why the
range form survives — the reacquire EDGE rule); the census doc's non-vacuity
claim singularized (only the desugar class is pinned; the Context count is
reported). Lens A judged the rewritten test docs and the `lower_with`
obligation comment at the house bar; no other findings.

## 7 — Step-3 vs step-5 findings

**Step-3 (language/design asks surfaced by the port):**

- `raise_error`/`raise_exception` expansions remain `User`-authored: the ruled
  site list names the assert desugar only, and the raise tails contain no
  SR-destination write, so nothing is mis-charged today — but they ARE
  compiler-emitted lines charged to the containing proc for every OTHER
  effect class (their `subq`/`move`-to-`-(sp)` traffic is sp-discipline,
  currently exempt anyway). If a future effect lint starts charging arg-push
  shapes, a `RaiseDesugar` author (additive) is the move. Not ledgered as its
  own row — noted here; the enum extension is one variant.
- `pad_to_cycles` pads are authored `Splice { "pad_to_cycles" }` via the
  boundary rather than a dedicated variant — honest (it IS a Code-returning
  builtin spliced in statement position) and free, but a census reader should
  know pads are not distinguishable from user templates by author alone.
- The `Splice` template name for a non-call `{expr}` splice is the flat
  `"<expr>"` — enough for the carried-not-consumed stage; the row-1551 parcel
  will want the resolved fn identity (module-qualified) when it grows a
  checking consumer.

**Step-5 (engine-side / follow-on candidates):**

- The nine TRUE-POSITIVE hand-written SR sites from the warning-tier triage
  (irq.emp, dma_queue.emp, ojz_scroll_test.emp, release_fault.emp) are now the
  ONLY thing that could ever re-populate the DEBUG rows — they are declared
  today and fire nowhere, but the aeon-side `preserves(sr)`/`clobbers(sr)`
  spellings recommended by triage §9 item 3 remain the honest hardening.
- The definition-site round-trip check gives every context a checked SR
  contract for free; when the perturbation-set trigger fires (§5), the same
  check site is where `perturbs(...)` validation slots in — one clause plus
  one lookup, as designed.

## 8 — Files touched

Lib: `value.rs` (enum + field + `reauthor_user_items`), `eval/mod.rs`
(evaluator state), `eval/asm.rs` (all four setting sites + definition check +
`splice_template_name`), `eval/builtins.rs` (pad constructions),
`lower/proc.rs` (typed exemption; `region_round_trips_sr` deleted;
`sr_writes_round_trip`/`writes_dest_register` exposed), `lower/mod.rs`
(re-exports), `lower/code.rs` (pattern), `corpus_contracts.rs` (census),
`sigil-harness/native.rs` (EntrySynth pin). Tests: `diag_desugar.rs`,
`lower_proc.rs`, `eval_code.rs`, in-crate helpers, `warn_tier_corpus.rs`.
Docs: `campaign-gap-ledger.md`, `2026-08-04-warning-tier.md`, this packet.
