# 2026-08-04 — THE `sr` LANE: whose SR write is it? (close packet)

Two repos, branch `sr-contracts`, branched from sigil `21f5aef7` / aeon `0e1f32c`.
Checkpoint for the overseer's countersign + merge.

- sigil `b1dbc19a` — the lint fix + its probes + the frozen-baseline split.
- sigil `5a7eb9ab` — the panel round.
- aeon `b995638` — two contract declarations the bodies already keep.
- aeon `095aa26` — the panel round.

> **MERGE ORDER IS LOAD-BEARING: aeon FIRST, then sigil.** Found by two lenses
> independently and measured both directions — §7. Sigil-branch against
> aeon-master fails `warn_tier_lint_ids_match_the_frozen_baseline` on four of
> seven shapes; the reverse order is green in both windows.

**Byte-neutral: seven targets for seven, `cmp`-identical. `pins.rs unchanged`,
`refreeze --check: OK (tip 'b-jumps', chain len 44)`. No chain bump, no 5-site
ripple.**

---

## §1 — RE-MEASUREMENT vs HANDOFF §7

Measured own-run at chain 44 before any edit, `SIGIL_WARNINGS=full`, all seven
shapes. Handoff §7's two classes are CONFIRMED, with one number corrected.

| | handoff §7 | measured | verdict |
|---|---|---|---|
| sonic4 plain firings | 8 | **8** | confirmed |
| class (a) `QueueDMA_Deferrable` | 4, at `dma_queue.emp:106,137,145,181` | **exactly those four** | confirmed |
| class (b) `Parallax_Update` + `GameState_OJZScroll_Init` | 4, all at `irq.emp:37`/`:40` | **exactly those four** | confirmed |
| sonic4 DEBUG firings | 52 | **51** | §7 is one high |

The DEBUG figure is the only correction. 51 is what the shape fires at chain 44;
52 was the warn-tier packet's UNION across all seven shapes, which is a different
number and has since moved on its own.

Also measured, and not in §7: the class fires in **six** of the seven shapes at
different counts (`demo plain` 6, `config_b` 8, `lean` 9, `demo debug` 48,
`config_a` 51). `lean` is the only shape carrying `ReleaseFault` — see §4.

---

## §2 — THE LINT FIX (landed first, so §3 could be measured honestly)

### 2.1 The defect

`engine/irq.emp`'s `ints_off` context is

```
acquire = asm { move.w sr, -(sp)      release = asm { move.w (sp)+, sr }
                move.w #$2700, sr }
```

A `with ints_off { }` bracket splices both halves into the CONSUMER's item
stream. `[proc.sr-undeclared]` walks that stream per proc, so it charged
`Parallax_Update` and `GameState_OJZScroll_Init` for the mask and the restore —
writes they do not make, anchored at a source line in a module they do not own,
and which no contract clause on them could honestly name. **Every future bracket
adopter inherited it**, which is what makes this a lint bug and not four
warnings.

### 2.2 The fix, and why it is gated

`region_round_trips_sr()` asks, of each bracket region recovered from the
`ContextMark` items B′-1 plants, whether its net effect on SR is nil; the lint
then exempts item indices inside such a region's **acquire** or **release** —
never the body.

Two scoping decisions, each of which a probe pins:

1. **Acquire and release only.** The body between them is the consumer's own
   code. An SR write there defeats the mask the bracket just installed, which is
   precisely what the lint is for.
2. **Only regions whose acquire+release ROUND-TRIP SR.** A context that masks and
   never restores leaves the consumer's SR genuinely changed, and that still has
   to be charged to somebody. A blanket region exemption would have made a
   badly-written context silently un-lintable.

The round-trip recognizer is FACTORED OUT of the existing `check_preserves_sr`
(`sr_writes_round_trip`) rather than written twice, so the declared-contract
reading of `move.w sr,-(sp)` … `move.w (sp)+,sr` and the region reading of the
same idiom cannot drift apart.

`Region::in_release` joins the existing `in_acquire` in `context.rs` — the pair
now names the two halves the CONTEXT authored, as against the body.

### 2.3 The probes, and the negative control each survived

Seven gates — four written with the fix, three added by the lens panel (§7 C6).
Each mechanism piece was DELETED or MUTATED and the run repeated; the table
records which gate went red.

| gate | pins | negative control run |
|---|---|---|
| `bracketed_sr_traffic_is_the_contexts_declaration_not_the_consumers` | the exemption exists | delete the exemption term → **FAILED** |
| `a_hand_written_sr_write_inside_the_bracketed_body_still_fires` | it is acquire+release, NOT the whole region | widen to `r.contains(idx)` → **FAILED** (and delete the exemption → also FAILED) |
| `a_context_that_never_restores_sr_still_charges_its_consumer` | the round-trip gate | replace the filter with `\|_\| true` → **FAILED** |
| `a_release_that_re_masks_after_restoring_still_charges_its_consumer` | the trailing-write limb, reached from the RELEASE half | (the limb no other gate touches) |
| **`each_region_is_judged_on_its_own_round_trip`** | the decision is PER REGION | fold the regions into one global bool → **FAILED**, and it is the ONLY gate that fails |
| `nested_brackets_keep_their_own_acquire_and_release_ranges` | the corpus's real shape at all five sites, both directions | — |
| `a_bracket_does_not_disturb_the_declared_sr_clauses` | the exemption ADDS a fourth way to declare, it does not replace `preserves(sr)`/`clobbers(sr)` | (green under every doctoring BY DESIGN — it pins the clauses, not the exemption, and its doc says so) |

The body probe is the sharpest form available: one proc, three SR-writing
instructions, **exactly one** firing asserted by count. A blanket exemption fails
it while the acquire/release halves stay silent.

**The per-region gate is the panel's catch and it is worth stating plainly.** All
four original gates built a proc with exactly ONE bracket, so a mutant that
computes a single "do all regions round-trip" bool and exempts everything or
nothing passed every one of them. Lens C predicted it; the mutant was built and
run; it passes 4/4 originals and dies only on the new gate.

An eighth gate lives in `warn_tier_corpus.rs` — see §7 C2.

### 2.4 Measured effect

| shape | before | after lint fix | final |
|---|---|---|---|
| sonic4 plain | 8 | 4 | **0** |
| sonic4 debug | 51 | 47 | **43** |
| demo plain | 6 | 4 | **0** |
| demo debug | 48 | 46 | **42** |
| config_a | 51 | 47 | **43** |
| config_b | 8 | 4 | **0** |
| lean | 9 | 5 | **0** |

The lint fix alone retires 4 firings per shape (2 in `demo plain`/`demo debug`,
which carry `Parallax_Update` but not the sonic4-only
`GameState_OJZScroll_Init`). Everything remaining is the `assert` desugar, and
§7's new gate holds that property executably rather than in prose.

---

## §3 — CLASS (a): `with ints_off { }` DOES NOT FIT, and here is the control flow

The brief asked me to confirm the bracket before committing to it. **It does not
fit, for two independent reasons, and `irq.emp`'s own module header already names
both.**

### 3.1 Three exits against one save

`QueueDMA_Deferrable.transfer` (`engine/system/dma_queue.emp`) saves SR
ONCE at entry and restores it on **three** separate return paths:

The line numbers below are the pre-parcel ones the firings named; the header
paragraph the parcel added shifts them down by 16.

```
export .transfer:
        move.w  sr, -(sp)              // the one save
        move.w  #$2700, sr             // :106  <- firing
        …
        beq     .full
        …
    .finish_entry:
        …
        move.w  (sp)+, sr              // :137  <- firing
        andi.b  #$FE, ccr              // carry CLEAR = enqueued OK
        rts
    .full:
        …
        move.w  (sp)+, sr              // :145  <- firing
        ori.b   #1, ccr                // carry SET = dropped
        rts
    .split:
        …
        bhs     .finish_entry          // one free slot -> exits via .finish_entry
        …
        move.w  (sp)+, sr              // :181  <- firing
        andi.b  #$FE, ccr
        rts
```

A `with` bracket is ONE lexical region with ONE entry and ONE exit, and the
`[context.escape]` proof (`context.rs::check_regions`) fires on any edge out of
the acquire+body range that does not land back inside the region. All three
`rts`es are exactly that edge. The bracket would produce three errors, not four
retired warnings.

### 3.2 The structural exclusion: a CCR out-contract can never adopt

All three entry points declare `out(carry: dropped)`. The carry is pinned by
`andi.b #$FE, ccr` / `ori.b #1, ccr` **after** the SR restore, because a full-SR
restore overwrites CCR — the header has said so since the port. A bracket splices
its release at `BodyEnd`, so the carry pin would land INSIDE the region and be
overwritten. There is no ordering of `with ints_off { }` that produces
"restore, then set carry, then return", three times.

`irq.emp:18-26` already records both, and names this exact site:

> Sites that stay HAND-SPELLED by design … multi-exit brackets (one save, several
> restore-and-return paths — **the DMA-queue transfer core**) … STRUCTURAL
> exclusion: any bracket with a CCR out-contract can NEVER adopt.

So B′-1 did not skip this site; it ruled on it and wrote the ruling down. The
re-measurement's job was to check that ruling still holds, and it does.

### 3.3 The deliberate fallback: `preserves(sr)`, not `clobbers(sr)`

Path-checked by hand, since `check_preserves_sr` is a static-ORDER slice with no
path analysis and therefore cannot prove this:

| path | save | restore | returns |
|---|---|---|---|
| queue full (`beq .full`) | ×1 at `.transfer` | ×1 at `:145` | `rts` |
| no boundary crossing → `.finish_entry` | ×1 | ×1 at `:137` | `rts` |
| `.split` with one free slot (`bhs .finish_entry`) | ×1 | ×1 at `:137` | `rts` |
| `.split` with two free slots | ×1 | ×1 at `:181` | `rts` |

Every path saves once and restores once. **The interrupt MASK round-trips; CCR
does not, and CCR is the declared result.**

`clobbers(sr)` would be the safe over-claim, and the direction rule licenses it —
but only where the truth is unverifiable. Here it is verifiable and it is the
load-bearing fact: callers of the DMA queue rely on their mask surviving (a
VBlank landing mid-enqueue would see a half-written queue entry). Declaring
`clobbers(sr)` would throw that away to buy nothing.

The exact precedent is `Sound_DrainSfxRing` (`sound_api.emp:342-347`), which
declares `preserves(sr)` over a body that leaves CCR clobbered on one path, with
a header that spells the mask/CCR split out. `QueueDMA_Deferrable`'s new header
paragraph is written to the same standard, and names the three exits so the claim
is checkable without re-reading the body.

**Not taken, deliberately:** `QueueDMA_Critical` and `QueueDMA_Important` reach
the same core through `jbra QueueDMA_Deferrable.transfer` and carry the same SR
behaviour, but their own bodies contain no SR write, so `check_preserves_sr`
would verify a `preserves(sr)` there VACUOUSLY. That is a manual-honor claim,
which the honest-contract rule prohibits. Ledgered as a language gap instead —
see §7 — and it is the same shape as the warn-tier packet's §9 item 1 (`out()`
not discharged by a callee's or fallthrough target's declared `out()`).

---

## §4 — `ReleaseFault`: a FLAGGED SCOPE CALL for the overseer

The brief scoped corpus work to the plain shape's two classes. `ReleaseFault`
(`engine/system/release_fault.emp:57`) is in NEITHER — it is `lean`-only. **I
took it anyway, and the overseer should countersign or revert it (a one-word
revert).**

The reasons:

1. It is one word (`clobbers(d0)` → `clobbers(d0/sr)`), byte-neutral, and already
   adjudicated: the warn-tier packet's §9 item 3 names this exact proc and this
   exact clause.
2. It is the same lint class this lane exists to retire.
3. **It changes the shape of the `warn_tier_corpus.rs` diff.** Leaving it makes
   `lean` the sole shape holding a spelled-out divergent row for a class the lane
   otherwise cleared everywhere it could — a worse artifact than the fix, and one
   the overseer would have to re-adjudicate later anyway.

`clobbers(sr)`, never `preserves(sr)`: the proc masks every interrupt level and
freezes forever. The mask is permanent by design, so nothing restores it, and it
never returns so no caller observes it. The clobber is declared because the
contract is a statement about the BODY — the header already makes that argument
for the identical `d0` case.

---

## §5 — THE `warn_tier_corpus.rs` DISPOSITION, AND WHY

**Measured, all seven shapes, post-fix:** `proc.sr-undeclared` clears COMPLETELY
in four (`sonic4 plain`, `demo plain`, `config_b`, `lean`) and still fires in
three (`sonic4 debug` 43, `demo debug` 42, `config_a` 43).

The file freezes a per-shape id SET, so the baseline moves exactly for the four
that cleared. The diff, after the panel reshaped it (§7 A2):

```rust
const CORPUS_LINTS: &[&str] = &[ … ];                    // the 4 every shape fires
const DEBUG_ONLY_LINTS: &[&str] = &["proc.sr-undeclared"];  // what a DEBUG shape ADDS
```

Rows carry only what a shape ADDS, and the gate unions at comparison. My first
draft spelled the five ids out a second time, which quietly broke the file's own
promise that "retiring a class is the ONE-LINE diff it should be" and let the two
lists drift while both stayed green. Lens A caught it.

**And a set alone was not enough.** Admitting the id for the DEBUG rows means the
gate cannot tell 43 firings from 44, so a NEW hand-written undeclared SR write
would hide among 43 compiler-generated ones — the lane retiring this class would
have ended by making it unwatchable. `every_surviving_sr_firing_is_the_assert_desugar`
closes that without pinning a count the campaign already rejected for this id.
Full argument and its negative control: §7 C2.

**The split falls exactly on `DEBUG`, and the reason is measurable.** Of the 43
firings surviving in `sonic4 debug`, **43 sit on an `assert.b`/`assert.w`/
`assert.l` line** — checked by reading the source line every firing points at, not
inferred. Zero hand-written SR writes remain undeclared anywhere in the corpus.
So `ASSERT_SHAPE_LINTS` is named for its cause, and the day the desugar stops
being linted (warn-tier §9 item 2) the two consts collapse back into one.

Two stale claims in that file were corrected in the same commit: the header's
`lean` example (`lean` no longer "gains `ReleaseFault`'s `[proc.sr-undeclared]`")
and the divergence note, both rewritten to what is now measurably true.

The gate was re-run against the branch: 4/4 green.

---

## §6 — THE BARS

### 6.1 Seven-target byte bar

Target list derived from `crates/sigil-harness/golden/*.bin` in this worktree.
Built in `capture_goldens.sh` order — four canonical via `./build.sh <game>`, one
shape per invocation; then `--config-a` (writes `s4.debug.bin`), `--config-b` and
`--lean` (both write `s4.bin`); canonical rebuilt afterwards and re-compared.
Compared with `cmp`.

| # | target | vs `golden/` |
|---|---|---|
| 1 | `s4.bin` | **IDENTICAL** |
| 2 | `s4.debug.bin` | **IDENTICAL** |
| 3 | `demo.bin` | **IDENTICAL** |
| 4 | `demo.debug.bin` | **IDENTICAL** |
| 5 | `config_a.bin` | **IDENTICAL** |
| 6 | `config_b.bin` | **IDENTICAL** |
| 7 | `lean.bin` | **IDENTICAL** |
| — | `s4.bin` / `s4.debug.bin` after the canonical restore | **IDENTICAL** |

`repin --check` → `pins.rs unchanged`.
`refreeze --check` → `OK (tip 'b-jumps', chain len 44)`.

A contract clause emits no bytes and a lint emits none either, so byte-neutrality
was the expectation — it is measured rather than assumed.

### 6.2 Strict suite

```
AEON_DIR=<this branch's aeon worktree> SIGIL_EMIT=… SIGIL_BUILD=… \
  SIGIL_STRICT_GATE=1 cargo test --workspace --release
```

Full capture to a file, never piped through `tail`/`head`. Failures first:

| | value |
|---|---|
| **failed** | **0** |
| passed | 3138 |
| ignored | 4 |
| result lines | 307 |
| `cargo` exit | 0 |

`AEON_DIR` pointed at `aeon/.worktrees/sr` at `095aa26` — this branch's tree at
the commit matching this sigil tree, per the paired-state gate.

Run TWICE: once on the pre-panel commits (3134/0/4) and again on the final ones.
Both green, and the +4 between them is the panel's four added gates.

### 6.3 Test-delta arithmetic

`git grep -c '^\s*#\[test\]' <commit> -- 'crates/**/*.rs'`, summed and diffed
per-file:

| | master | `sr-contracts` | delta |
|---|---|---|---|
| `#[test]` total | 3134 | **3142** | **+8** |

The per-file diff names exactly TWO files and nothing else moves:

| file | master | branch | added |
|---|---|---|---|
| `crates/sigil-frontend-emp/tests/lower_proc.rs` | 101 | 108 | **+7** — the four in §2.3 plus the panel's three (`a_release_that_re_masks_after_restoring_still_charges_its_consumer`, `each_region_is_judged_on_its_own_round_trip`, `nested_brackets_keep_their_own_acquire_and_release_ranges`) |
| `crates/sigil-cli/tests/warn_tier_corpus.rs` | 4 | 5 | **+1** — `every_surviving_sr_firing_is_the_assert_desugar` |

Eight functions, all named, in two files.

**It closes exactly: 3138 passed + 4 ignored = 3142 = the branch's own `#[test]`
total.** Nothing is being silently skipped.

Master's suite was NOT re-run for a comparison row — the invariant that catches a
silent skip is `passed + ignored == the branch's own total`, and it is proven on
the branch. The implied master figure is 3134/0/4, and every added test is a named
function in a named file, so there is no unattributed delta to chase.

---

## §7 — LENS PANEL

Three fresh read-only lenses over `git diff master...sr-contracts` in BOTH repos.
They returned **two blockers on work that had already passed every gate green**,
and the second one is the finding of the parcel: the baseline split I had just
written would have opened a permanent hiding place.

**Two lenses independently found the same contract gap** (A6 / B4 — the
`out(carry:)` ∩ `preserves(sr)` partition no checker can see), which is the
strongest signal the panel produced.

### THE TWO BLOCKERS

**LENS B1 / LENS C1 (same finding, both lenses, measured both directions) — THE
MERGE ORDER IS LOAD-BEARING. aeon MUST merge before sigil.**

`warn_tier_corpus.rs` defaults `AEON_DIR` to the MAIN aeon checkout. Measured:

| sigil | aeon | `sonic4 plain` sr firings | new baseline |
|---|---|---|---|
| master | branch | 4 (all `irq.emp:37/40`) | master's row includes the id → **passes** |
| **branch** | **master** | **4** (`dma_queue.emp` ×4) | new row excludes the id → **FAILS** |
| branch | branch | 0 | **passes** |

So aeon-first is safe in both windows and sigil-first is red. **This is the
overseer's to sequence and it is the one thing in this packet that cannot be
fixed in code.**

**LENS C2 — the frozen id SET would have turned this class into a hiding place,
permanently.** Admitting `proc.sr-undeclared` on the three `DEBUG == 1` rows is
correct, but an id set cannot tell 43 firings from 44 — so after this parcel a
NEW hand-written undeclared SR write in any debug-gated proc would join 43
compiler-generated firings and never be seen again. The lane that exists to
retire this class would have ended by making it unwatchable.

A COUNT would catch it, and was **rejected for this exact id yesterday on
measured grounds** (warn-tier §3.1 item 3: one added debug assert moves the
number, so the baseline churns on unrelated work and gets rubber-stamped). Rather
than overturn a one-day-old ruling, the fix asserts the PROPERTY the class
actually has:

`every_surviving_sr_firing_is_the_assert_desugar` reads the source line every
firing points at and requires it to be an `assert.` line. A new assert is still
an assert and moves nothing; the first hand-written SR write to go undeclared
fails immediately **with the site named and the fix in the message**. Its kill
condition is the desugar's own — when sigil stops linting its own emitted code
the class goes to zero and the gate asserts emptiness on its own.

**Negative control run:** dropping `ReleaseFault`'s `clobbers(sr)` fails it with
`a HAND-WRITTEN SR write is undeclared at …/release_fault.emp:57:9`.

### Lens A — ceremony / style (1 blocker, 7 should-fix, 11 nits)

| # | finding | disposition |
|---|---|---|
| **A1** | **BLOCKER: change-history narration in a test doc** ("before the region exemption both fired") | **FIXED** — rewritten to the counterfactual in present tense. |
| A2 | `ASSERT_SHAPE_LINTS` re-spelled all four shared ids, breaking the file's own "retiring a class is a ONE-LINE diff" property; the two lists could drift silently while both stayed green | **FIXED, the strong way** — rows now carry only what a shape ADDS (`DEBUG_ONLY_LINTS`), and the gate unions at comparison. Retiring a shared class is one line again. |
| A3 | stale hedge: header said "**most** `sr-undeclared` firings are the assert desugar", inconsistent with the sibling doc 40 lines below | **FIXED** — "every", which is measured (43/43). |
| A4 | the `lean`/`ErrorHandlerBlob` example is a COUNT-level difference and no longer supports its own conclusion in a file that pins SETS | **FIXED** — the id-level example leads; the count-level one is labelled as such. |
| A5 | the `dma_queue` reliance clause states the mechanism BACKWARDS — a mid-enqueue VBlank is stopped by INSTALLING the mask, not by restoring the caller's | **FIXED** — caller-side reliance stated as the reason for the clause; the half-built-entry fact moved onto the mask install. A genuine cold-reader catch. |
| A6 | `out(carry: dropped) preserves(sr)` self-contradicts on its face; first corpus site with both | **LEDGERED** (with B4) + row 1067 gains it as instance #2. |
| A7 | the test doc claimed "the three contract clauses" but exercised only `preserves(sr)` | **FIXED** — `clobbers(sr)` arm added. |
| A8 | change-history narration in the `release_fault.emp` header block the parcel edits | **FIXED** — present-tense rewrites; the durable ruling citation kept. |
| A9 | the worktree had uncommitted work at review time | **FIXED** — all committed. |
| A10-A15 | wrong locator ("below" for a fn 700 lines above); "two callers" naming the wrong second caller; `check_clobbers` doc one exemption short; two indentation idioms; assertion voice; `in_release` doc written in terms of a downstream lint | **ALL FIXED**. |
| A16 | `ASSERT_SHAPE_LINTS` names a cause, not the axis that selects the rows | **SUPERSEDED** by the A2 fix (`DEBUG_ONLY_LINTS`). |
| A17 | `/sr` vs `, sr` separator divergence | **DECLINED** — pre-existing, and the parcel follows the majority. Already ledgered as the comma-group sweep. |
| A18 | `regions_of` per proc | **RESOLVED by B2** + measured free (C8). |
| A19 | three general fixes now proposed for one shape, with no cross-reference | **FIXED** — a ledger row that names all three and states the underlying question (should a `CodeItem` carry its AUTHOR). |
| A20 | duplicated rationale in `release_fault.emp` | **FIXED** — stated once. |

### Lens B — corpus pattern (1 blocker, 3 should-fix, 4 nits)

| # | finding | disposition |
|---|---|---|
| **B1** | **BLOCKER: merge order** (above) | **REPORTED** — overseer's call. |
| B2 | the parcel scanned the mark stream TWICE per proc, against a seam the corpus split for exactly this reason — `check_regions` is `pub` so a second consumer need not re-scan, and `corpus_contracts.rs` is the model | **FIXED** — `lower_proc` recovers regions once and hands them to both consumers. A pattern finding, not a perf one: B measured no separable cost. |
| B3 | the five `with ints_off` sites were left split 3-vs-2 on `preserves(sr)`, with `irq.emp` still prescribing the clause — the one outcome that leaves the corpus reading two ways | **FIXED** — `Parallax_Update` and `GameState_OJZScroll_Init` declare it (true, and checker-verified against the spliced pair), and `irq.emp` now states what the lint credits. **This is the finding I most needed:** my lint fix had quietly REDUCED the corpus's declared contract surface, which is the wrong direction for a correctness parcel. |
| B4 | `[proc.out-preserves-overlap]` is structurally blind to `out(carry:)` ∩ `preserves(sr)`; `QueueDMA_Deferrable` is the only one of seven adopters co-declaring both | **LEDGERED** (with A6). |
| B5 | "which no contract closure credits yet" over-claims — the CLOBBERS closure does credit tail transfers; only `preserves` does not | **FIXED** — one word. |
| B6 | `sr_neutral_regions` returned a filtered `Vec` where `z80_bus::region_acquires_bus` is the per-region-predicate precedent | **FIXED** — `region_round_trips_sr(items, region) -> bool`, same shape as its sibling. |
| B7 | two over-100 lines | **FIXED**. |
| B8 | `section.emp`'s comma separator | **DECLINED** with A17. |

### Lens C — perf + hazard (1 blocker, 5 should-fix, 4 nits)

| # | finding | disposition |
|---|---|---|
| **C1** | **BLOCKER: merge order** (above) | **REPORTED**. |
| **C2** | **the id set makes the class a permanent hiding place** (above) | **FIXED** with the property gate + negative control. |
| C3 | ledger row 2063 asserts a fix that would never be applied | **FIXED** — closed with the reversal spelled out: two of the four were a compiler FP, not a contract omission, and the fix was in the lint. |
| C4 | design asymmetry — silence about SR became ambiguous, and `irq.emp`'s convention went stale | **FIXED** with B3. The SR-only scope of the exemption is now argued in the code where the next reader will ask. |
| C5 | `preserves(sr)` is literally false for the CCR half and nothing cross-checks it; second corpus instance | **LEDGERED** — row 1067 gains `QueueDMA_Deferrable` as instance #2, noting these are the two procs that will lie to S2-D7 when it reads `preserves(sr)` as "SR == entry SR at return". |
| C6 | **three real test gaps.** (i) per-region judgement unpinned — every gate built ONE bracket, so a mutant folding the regions into a single bool passed all four; (ii) NESTING unpinned by any unit test, though it is the corpus's real shape at all five sites, and the corpus gate returns green silently when the tree is absent; (iii) a release whose LAST write is not the restore reaches an unpinned limb | **ALL THREE FIXED.** `each_region_is_judged_on_its_own_round_trip` (two disagreeing brackets, exactly 1 firing), `nested_brackets_keep_their_own_acquire_and_release_ranges` (both directions), `a_release_that_re_masks_after_restoring_still_charges_its_consumer`. **C's predicted mutant was built and run: the global-bool fold passes all four original gates and is killed only by the new per-region test.** |
| C6(iv) | `a_bracket_does_not_disturb_the_declared_sr_clauses`'s first assertion is vacuous as a gate ON THE FIX (`preserves(sr)` short-circuits before the region check) | **ACCEPTED, doc corrected** — it pins the CLAUSES, not the exemption, and now says so and exercises both clauses. |
| C7 | an exempted SR write now falls into the register-clobber arm | **FIXED** — one comment; C verified harmless and measured `clobber-undeclared` unmoved (1 plain / 10 debug on both binaries). |
| C8 | eager `regions_of` | **MEASURED FREE, left alone.** C's interleaved A/B, 12 reps: median −0.0087 s (**−0.39%**), i.e. noise. A naive non-interleaved run showed +3% drift — the interleaved number is recorded here so it is not "optimized" later. |
| C9 | the round-trip warrant assumes a stack-balanced body and nothing checks it | **FIXED** — the assumption is stated at `region_round_trips_sr`, pointing at S2-D7(b). |
| C10 | the sibling entry points could declare `preserves(sr)` today and be accepted VACUOUSLY | **ALREADY LEDGERED**, and C's independent derivation confirms the §3.3 reasoning. |

### What the panel independently CONFIRMED CLEAN

- **`sr_writes_round_trip` is a faithful refactor**, proven line-by-line by two
  lenses independently (B and C), plus C's check that no collected data was lost
  (the diagnostic uses `proc.span`, never an index).
- **No panic from the slice bounds**, with the invariant traced: `regions_of`
  pushes a Region only when both marks are `Some`, both assigned at indices
  strictly between `enter` and `exit`. Marks are planted at exactly ONE site and
  every error/gate path plants none; a half-quadruple cannot be spliced because
  `Value::Code` is only ever constructed and concatenated.
- **Nesting is correct including same-context nesting**, traced through
  `rev().find()` / `rposition`.
- **`@as_compat` and Z80 do not interact** — both return before the region work.
- **`QueueDMA_Deferrable`'s mask claim is TRUE on every path**, re-derived
  independently by C with the stack checked: nothing is pushed or popped between
  save and restore on any path, and `.split`'s `bhs .finish_entry` shares that
  restore rather than doubling it.
- **"Every hand-written SR write in the corpus is declared"** — enumerated
  site-by-site across `section`/`bg`/`boot`/`sound_api`/`vblank`.
- **`bracketed_at` was correctly NOT reused** — it spans acquire+body+release, so
  it would have exempted the body and failed this parcel's own probe.
- Clippy clean on every added line; byte-identical ROMs; no other warn class moved.

---

## §8 — WHAT EACH PASS ADDED

### Pass 1 — step 3 (retrospect / language + tooling asks)

- **A lint that reads one proc's instruction stream is reading the wrong stream
  once a construct SPLICES.** `[proc.sr-undeclared]` was correct for every proc
  in the corpus until B′-1 shipped `with`, and then it was wrong for every
  adopter — not because the lint changed, but because "this proc's instructions"
  stopped meaning "instructions this proc's author wrote". The general rule, and
  it should bind the next construct that splices: **a lint over an item stream
  must ask who AUTHORED each item, and a construct that splices owes its
  consumers a way to tell.** This is the same root cause as the warn-tier
  packet's two self-inflicted classes (the synthetic entry module, the `assert`
  desugar) — the compiler blaming an author for the compiler's code — arriving a
  third time by a different route.
- **ASK: a context should declare its own perturbation set.** The exemption
  built here is a per-lint hand-rolled rule that happens to be right for
  `ints_off` because SR round-trips. A context whose acquire genuinely clobbers a
  scratch register has no honest answer today, and `[proc.clobber-undeclared]`
  will charge its consumers the moment one exists. Ledgered.
- **ASK: `preserves` is not discharged through a declared tail transfer.** It
  cost this parcel two contract declarations it could not honestly make
  (`QueueDMA_Critical` / `QueueDMA_Important`). `closure.rs` already does this
  reasoning for `clobbers`; `out` has the same gap (warn-tier §9 item 1). One fix
  closes all three. Ledgered.
- **Reads-wrong found and fixed:** two stale claims in `warn_tier_corpus.rs`'s
  own doc comments, both about which shape fires what. A file whose entire job is
  to freeze a measurement had two sentences describing a measurement that had
  moved.
- **Kill rows:** none — this parcel creates no twin scaffolding.

### Pass 1 — step 5 (engine optimize)

- **No engine code was changed and none should have been.** The parcel's aeon
  side is two contract clauses and their prose; the byte bar is seven-for-seven
  by construction.
- **C1 (cycle/perf) ruled INACTIVE, with the sites named** (a flagged call, not a
  silent skip): the only two aeon sites touched are `QueueDMA_Deferrable`'s and
  `ReleaseFault`'s contract clauses, which emit nothing. There is no instruction
  to cost.
- **The step-5 finding the parcel did surface, and did NOT take:**
  `QueueDMA_Deferrable`'s `.split` path with one free slot enqueues the first
  half and returns carry CLEAR — a half-transfer reported as success. The header
  already records it as a known edge deferred to the art-streaming plan's
  rollback work. Not this parcel's to take (byte-changing, behavioral, needs an
  oracle A/B), and it is already ledgered upstream. Recorded here because reading
  the four exit paths closely enough to declare `preserves(sr)` is exactly the
  reading that surfaces it.

### Pass 2 — step 3 (the panel round)

- **ASK, and it is the one worth acting on: should a `CodeItem` carry its
  AUTHOR?** Three general fixes are now on the table for one shape — a
  generated-code provenance marker (for the `assert` desugar and the synthetic
  entry module), a context perturbation set (for spliced acquire/release), and
  the region exemption actually built here. They are three answers to the same
  question, and the campaign is one point solution per lint away from never
  answering it. Ledgered as ONE row that names all three, per Lens A19.
- **Reads-wrong the panel found that I could not:** the `dma_queue` header stated
  its own mechanism backwards (A5), and my lint fix had quietly REDUCED the
  corpus's declared contract surface by removing the only pressure toward
  `preserves(sr)` at two of five bracket sites (B3). The second is the one that
  should have been obvious: a correctness parcel that leaves the corpus saying
  LESS about its contracts has moved in the wrong direction, and no gate can see
  it because silence is always green.
- **Kill rows:** still none.

### Pass 2 — step 5 (engine optimize)

- **Two contract declarations added** (`Parallax_Update`,
  `GameState_OJZScroll_Init`), byte-neutral, both checker-verified against the
  spliced pair. No instruction changed.
- **C1 measured rather than ruled inactive this round**, because the panel had a
  number: the extra `regions_of` scan costs a median **−0.39%** on a full
  `sonic4 debug` build over 12 interleaved reps — noise. The interleaved figure
  is recorded because a naive non-interleaved run showed +3% drift, and the next
  reader deserves the number that is not an artifact.

### Neither bucket — the headline

**B′-1's own adoption manufactured the false positives, and only re-measuring
found them.** The handoff's warning-tier packet was four hours old and named nine
true positives in four procs; by the time this lane ran, two of those procs had
adopted the bracket B′-1 built, and their firings had become artifacts of the
adoption rather than findings about the code. Neither the byte bar nor the strict
suite nor the frozen id-set gate could see the difference — all three were green
across the change that introduced the class, because the id set did not move and
the counts are deliberately not pinned.

The cheap defence is the one the warn-tier packet already recommended and this
parcel is the argument for: **once the false-positive classes are retired, pin
the per-`(shape, id)` COUNTS.** With `proc.sr-undeclared` now at 0 in four
shapes, a count baseline for those four would be exact, meaningful, and would
have caught this class the day it was created.

**And the panel supplied the sharper version of that lesson.** For the three
shapes where the class is NOT zero, a count was already ruled out — it churns on
every added assert. Lens C's finding forced the third option: pin the PROPERTY,
not the number. "Every firing in this class sits on an `assert` line" is
shape-invariant, churn-free, and fails the instant the class stops being pure
compiler noise. That is the same instinct the warn-tier packet named as *ratchet
what the project controls* — applied one level finer, to what a class IS rather
than how large it is. It is a better answer than either of the two the ruling
chose between, and it only appeared because a fresh lens read the gate as an
attacker would.
