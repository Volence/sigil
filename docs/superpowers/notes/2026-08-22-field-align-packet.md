# `(align: N)` — a per-field, error-tier alignment assertion for `.emp` structs

Branch `feat/field-align`. Queue item: a field alignment attribute / even-offset
assertion for struct declarations.

---

## 1. The subject, re-verified

`aeon@origin/master:engine/level/scene_dsl.emp` declares `struct Scene` with a
hand-computed `sc_pad_5D: u16 = 0` holding two `i16` bridges even, followed by two
module-scope `ensure(offsetof(...) % 2 == 0, ...)` guards. Its own comment block
(read at `origin/master`, ~lines 995–1030) records the incident verbatim:

- the pad's width is a function of every field above it, so an insertion up-tree
  silently re-parities the bridges;
- that happened between 2026-08-18 and 2026-08-22 — the pad was computed for
  offsets 94/96, four commits added fields above it, the bridges drifted to an odd
  119/121;
- **`[layout.odd-field]` fired the whole time and was swallowed by a warning
  baseline nobody re-read**;
- the author asserted the PROPERTY (even) rather than a snapshot (`@offset 94`),
  because a pinned offset must be hand-updated on every legitimate insertion —
  which re-arms the same staling-constant trap.

The load-bearing datum is the third bullet. The gap is not "no alignment check
exists" — one exists and it fired. The gap is that it is drownable. What is
missing is an **error-tier, per-field, opt-in** claim.

## 2. What shipped

A struct field may carry `(align: N)` immediately around its type:

```emp
struct Scene {
    // ...
    sc_pad_5D:             u16 = 0,
    sc_mask_raw:           i16 (align: 2),
    sc_v_deform_shift_raw: i16 (align: 2),
}
```

- **Error tier.** Not a lint. No `@allow` reaches it (§6).
- **A property, not a snapshot.** Parity survives every legitimate insertion above
  the field; a hand-written offset does not. This is the same reasoning the aeon
  author used for preferring `%2 == 0` to `@offset 94`, applied one layer down so
  the `ensure` pair is no longer needed to say it.
- **`N` is any comptime expression** evaluating to a power of two — consistent with
  `@offset` and `(size:)`, both of which already take exprs. A named constant works.
- **`(align: 1)`** is the identity claim: every offset satisfies it. It doubles as
  the per-FIELD opt-out of `[layout.odd-field]`, where the existing `@allow` can
  only speak for a whole module. This was a fall-out of the design, not a goal, and
  it is a strictly better escape hatch than the module-wide one.
- **`(align: 0)`, negatives, non-powers-of-two** are refused as a malformed
  alignment and never reach the modulo.
- **Scope:** this field's offset within THIS struct. A field of struct type asserts
  where that struct starts; nothing propagates into the nested struct's own fields,
  which carry their own claims.

Implementation: `ast::StructField::align`, `Parser::struct_field_attrs`,
`Evaluator::check_struct_field_align`, plus a two-line suppression in
`check_struct_odd_fields`.

## 3. The spelling, and the alternative rejected with the strongest reason

**`@align(N)` was already taken.** `crates/sigil-frontend-emp/src/parser.rs`'s
`opt_align` parses `@align(N)` on `vars` typed fields, and
`lower/regions.rs::align_to` implements it as **advancing the allocation cursor** —
it MOVES things. Spec D2.29 says so explicitly: "`vars` regions keep `@align(N)` on
fields (reserved space, no bytes) — different mechanism, deliberately different
spelling."

So reusing `@align(N)` on a struct field would give one spelling two opposite
meanings in the same language: reserve vs assert. That is the same class of trap
this parcel exists to close, and it would be worse than the trap it closes, because
a reader who learned `@align(256)` in a `vars` block would reasonably expect a
struct's to pad.

Candidates weighed:

| Spelling | Verdict |
|---|---|
| `@align(N)` | **Rejected.** Collides head-on with the `vars` prescriptive form. |
| `@aligned(N)` | Rejected. The imperative/participle distinction is genuinely apt, but one letter and a suffix are too little signal between two attributes with opposite semantics. |
| `@even` | Rejected. Covers only 2; no path to 4. |
| `(align: N)` | **Shipped.** |

`(align: N)` joins the paren-attribute family, **every existing member of which
already verifies a placement rather than choosing one**: `struct Name (size: expr)`
asserts a total and never resizes; a map item's `(align: N)` is documented in
`ast.rs` as "a congruence assert on the final address". A reader who has seen
`struct Name (size: 0x50)` — which is right there in the same declaration —
already knows that `(key: expr)` on a struct means "check this".

This is D2.29's own principle in its third application: a different mechanism gets
a deliberately different spelling.

**The confusion hazard is closed by a diagnostic, not by hoping.** Writing
`@align(N)` on a struct field is refused by name:

```
`@align(N)` reserves space and belongs to a `vars` field; a struct field asserts
its alignment with `(align: N)`, which moves nothing
```

rather than being parsed as an offset expression that happens to begin with the
identifier `align` (which is what the old grammar would have done, producing a
baffling downstream error). Test: `vars_form_align_on_a_struct_field_is_refused_by_name`.

Grammar position: the two trailing assertions parse in a loop, so
`i16 @ 1 (align: 2)` and `i16 (align: 2) @ 1` are the same field. Duplicates of
either are diagnosed. A `(` after a complete field type was a parse error before
this change, so the grammar is strictly widened, never reinterpreted.

## 4. T2 — auto-padding: priced, NOT shipped, with a finding that changes the question

**The finding: T2 is not a feature decision, it is an amendment to a ratified spec
rule that already states this exact rationale.** `SIGIL_SPEC2_LANGUAGE.md` §4.3
(line 264):

> Fields lay out in declaration order at the next byte. **The compiler never
> inserts alignment or padding** — Aeon runs `padding off` globally and hand-pads;
> an auto-aligning struct would silently break byte-exact ports.

and D2.29 (line 181): "alignment is never *inserted* automatically".

The overseer's concern — "in a language whose entire discipline is exact byte
layout, this would be the first construct that moves bytes the author did not
write" — is not a new judgement call. It is the ratified rule, with the same
reason, already written down. That materially raises the bar on T2: it is a spec
amendment, not a feature request.

### What T2 would take

1. **A §4.3 + D2.29 spec amendment** (empyrean). This is the load-bearing item and
   the only one that is not mechanical.
2. `layout_of_struct` gains an insertion pass; `Layout` must record synthesized pad
   bytes distinctly so `(size: N)`'s field-by-field diff, `offsetof`, and `sizeof`
   stay truthful about what the author wrote vs what the engine added.
3. **A defined fill byte.** The `align N` item uses `$00`; a struct pad would need
   to say so and match.
4. **Overlay blast radius.** Overlay fields lay out "by §4.3 struct rules" (§4.3,
   overlay bullet) and overflow their window as a hard error. Auto-padding changes
   whether an overlay fits, with the cause invisible in the source.
5. **The byte-identity campaign.** Every struct that emits changes size the moment
   someone adds an alignment attribute, which fires the 5-site ripple (pins.rs,
   engine.inc, mixed_dac_rom.rs, repin_pins.rs, repin.toml) on edits that read like
   pure annotations.
6. **AS parity has a hole exactly where the backstop is absent.** Aeon runs
   `padding off` globally, so a ported struct that auto-pads diverges from its AS
   twin — which the port loop's step-1 byte gate catches. New `.emp`-only structs
   have no twin, so the drift is silent precisely where nothing would catch it.

### The strongest argument FOR T2, stated honestly

**The assertion tier does not remove the hand-computed constant. It only guards
it.** After this parcel, `sc_pad_5D: u16 = 0` still has a width that is a function
of every field above it, and a human still widens or narrows it by hand each time
the guard fires. T1 converts a silent wrong answer into a loud one — a real and
sufficient improvement for the incident that motivated it — but it does not convert
a hand-maintained number into a derived one. Under a strict "kill the staling
constant" principle, T2 is the actual fix and T1 is a smoke alarm. And for `Scene`
specifically the objection is at its weakest: `Scene` is comptime-only and emits
nothing, so auto-padding it would cost literally zero ROM bytes.

The counter is that the attribute cannot be scoped to comptime-only structs without
becoming a confusing partial feature, and for a struct that emits, T2 does the one
thing §4.3 exists to forbid.

### My recommendation for the T2 slot: neither ship nor drop it — reshape it

There is a third position that gets T2's benefit without T2's cost, and I recommend
it over both. **Derive the pad's width; do not insert the pad.** Give structs the
field form that `vars` region bodies already have — `pad(N)` — plus a `pad_to(N)`
sibling:

```emp
struct Scene {
    // ...
    pad_to(2),                          // width chosen to reach the next multiple of 2
    sc_mask_raw:           i16 (align: 2),
    sc_v_deform_shift_raw: i16 (align: 2),
}
```

The distinction that matters: **the bytes are at a site the author wrote.** The
author places the pad on its own line; the engine computes only its width. That is
not "bytes the author did not write" — it is an author-placed pad whose width is
derived instead of hand-computed. §4.3's rule survives intact in the sense that
makes it valuable: nothing appears in a struct that has no such line in it, so no
existing struct can change size, and the byte-identity campaign is untouched.

It kills the staling constant outright, which is the actual goal T2 was reaching
for, and `(align: N)` stays as the independent proof that the derivation did what
was intended.

Precedent: `pad(N)` is already a region-form field (`parser.rs::region_field`), so
this is the sibling of an existing construct rather than a new idea, and the
`vars`/struct grammars converge instead of diverging further.

**Not shipped — this needs its own ruling and its own spec text.** Recorded as a
gap-ledger row.

## 5. Red-first evidence, both arms

### Arm 1 — the poison arm (does the guard fire at all?)

`check_struct_field_align` neutered with `if true { return; }`:

```
---- field_align_violation_is_an_error stdout ----
thread 'field_align_violation_is_an_error' panicked at
crates/sigil-frontend-emp/tests/eval_layout.rs:405:5:
assertion `left == right` failed: expected exactly one diagnostic, got []
  left: 0
 right: 1
```

`test result: FAILED. 26 passed; 10 failed` — the ten:
`field_align_violation_is_an_error`, `field_align_accepts_either_attribute_order`,
`field_align_and_at_offset_are_independent_assertions`,
`field_align_and_a_wrong_at_offset_both_report`,
`field_align_error_is_not_silenced_by_the_odd_field_allow`,
`field_align_non_power_of_two_is_refused`,
`field_align_even_offset_missing_a_wider_alignment_omits_the_cpu_claim`,
`field_align_catches_an_insertion_that_re_parities_a_bridge`,
`field_align_takes_a_comptime_expression`,
`field_align_zero_is_refused_not_a_modulo_by_zero`.

Probe removed; restored to green.

### Arm 2 — the control arm (would a reject-everything check pass?)

The satisfied-case early return disabled (`if slack == 0 && false`), making the
check reject every field that carries the attribute:

```
---- field_align_satisfied_is_silent stdout ----
thread 'field_align_satisfied_is_silent' panicked at
crates/sigil-frontend-emp/tests/eval_layout.rs:391:5:
a satisfied (align:) must be silent: [Diagnostic { level: Error, message:
"struct Rec: field bridge declares (align: 2) but lands at offset 2 — add 2 or
remove 0 byte(s) of padding above it. Do not pin an @offset to satisfy this; a
hand-written offset is the number that goes stale.", primary: Span { source:
SourceId(0), start: 53, end: 54 } }]
```

`test result: FAILED. 33 passed; 3 failed` — `field_align_satisfied_is_silent`,
`field_align_one_is_the_per_field_odd_field_opt_out`,
`field_align_catches_an_insertion_that_re_parities_a_bridge`.

A reject-everything check passes the poison arm and is refuted only here. Probe
removed; restored to green.

### The live diagnostic, verbatim

```
struct Rec: field bridge declares (align: 2) but lands at offset 1 — add 1 or
remove 1 byte(s) of padding above it. A word or long access to an odd address is
a 68000 address error. Do not pin an @offset to satisfy this; a hand-written
offset is the number that goes stale.
```

It follows the house idiom (`engine/system/replay.emp:96-97`, and the aeon
`ensure` messages): it names the INSTRUCTION that would fault, quotes the live
offset as data, and explicitly tells the reader NOT to pin an offset — a message
that told them to pin one would re-arm the exact trap.

The 68000 clause is conditional on the offset actually being odd. A field that
misses a wider alignment by an even margin gets no CPU claim, because none would
fault — asserted negatively by
`field_align_even_offset_missing_a_wider_alignment_omits_the_cpu_claim`.

The padding delta is derived, not copied: add `N - (offset % N)`, remove
`offset % N`. At offset 1 / align 2 → add 1, remove 1. At offset 2 / align 4 →
add 2, remove 2. Both asserted.

## 6. Matcher uniqueness

The violation matcher is `"but lands at offset"`. Grep over every `crates/**/*.rs`:

- `crates/sigil-frontend-emp/src/layout.rs:1057` — this rule. **The only diagnostic
  string anywhere that contains it.**
- `tests/offsets_inline.rs:152`, `tests/dc_link_expr.rs:39`,
  `sigil-link/src/lib.rs:1075` — prose comments, not diagnostic text.

The refusal matcher would have been `"must be a power of two"`, which is NOT
unique-by-phrase: `lower/mod.rs:1736` refuses a section's `bank:` attribute. Those
two texts differ (`power of two` spaced vs `power-of-two` hyphenated) so
`contains()` does not in fact cross-match — **but that is a coincidence of
hyphenation and is exactly the kind of thing a future rephrase breaks silently.**
Both refusal tests therefore also assert the rule-unique
`"declares (align: N)"` prefix, which grep confirms appears only in this rule's two
messages. Matching the call site would not have caught this; only reading every
diagnostic that shares the concept does.

The parser refusal matches `"asserts its alignment with \`(align: N)\`"` —
`parser.rs:1110`, sole occurrence.

## 7. Interaction verdicts

### With `[layout.odd-field]`

| Situation | Result | Test |
|---|---|---|
| `(align: 2)` on a field at an odd offset | The **error** only. The warning is suppressed for that field. | `field_align_supersedes_the_odd_field_warning` |
| Same, under module-scope `@allow("layout.odd-field")` | The **error still fires**. | `field_align_error_is_not_silenced_by_the_odd_field_allow` |
| `(align: 1)` on an odd word field | Silent. Neither error nor warning, for that field only. | `field_align_one_is_the_per_field_odd_field_opt_out` |
| A bare odd word field beside one carrying `(align: 1)` | Still warns. | same test |

**The `@allow` does not reach the assertion, by construction:** the allow is
consulted in `check_struct_odd_fields` only; `check_struct_field_align` never calls
`module_allows_lint`. That is the point — the whole motivation is that the
silenceable tier was silenced.

The suppression direction was a design call. A field carrying an explicit
error-tier claim has already been judged at the tier its author asked for; the
heuristic lint has nothing to add, and double-reporting one fault at two severities
is noise. `(align: 1)` inherits the suppression, which is what makes it a coherent
per-field opt-out rather than an accident.

### With `@offset`

They are independent assertions on the same computed offset, and "sanely" means
neither suppresses the other: they are different claims (one says the snapshot is
stale, the other says the parity broke) and an author who broke both should see
both.

| Situation | Result | Test |
|---|---|---|
| `@ 1 (align: 2)`, field at 1 | `@offset` holds, `(align:)` fails — one diagnostic. | `field_align_and_at_offset_are_independent_assertions` |
| `@ 99 (align: 2)`, field at 1 | **Both** report — two diagnostics. | `field_align_and_a_wrong_at_offset_both_report` |
| Either order written | Identical parse. | `field_align_accepts_either_attribute_order` |

`(align:)` also inherits `@offset`'s re-entrancy discipline: a self-referential
alignment expression that closes a layout cycle bails out of the check rather than
piling an alignment mismatch on top of the cycle report.

## 8. Source-gate self-audit

`scripts/nightly_source_gates.sh` classifies every `crates/*/tests/*.rs` that greps
positive for the aeon-tree identifiers as either listed in `SOURCE_GATES` or
derivably artifact-dependent, and exits 2 if any file is unclassifiable — which
takes the whole nightly backstop dark. The grep cannot tell a use from a mention,
so a doc comment merely *describing* those inputs is enough to arm it.

**This parcel adds no new test file** — the 14 tests join the existing
`crates/sigil-frontend-emp/tests/eval_layout.rs`, which does not grep positive and
is not in the audit set. The classification surface is unchanged. Audit replayed
faithfully (the script's own `SOURCE_GATES` array parsed out of the script, its own
two greps) against both trees:

```
feat/field-align : gates=35 unclassified=0
master (baseline): gates=35 unclassified=0
```

**M = 0.**

## 9. Full suite

`AEON_DIR=/home/volence/sonic_hacks/.aeon-landing SIGIL_STRICT_GATE=1
cargo test --release --workspace --no-fail-fast`, log stamped BEFORE cargo wrote to
it (cargo prints no cwd, no branch and no HEAD, so a run launched from the wrong
tree yields a log that is green, plausible, and about somebody else's branch):

```
### pwd=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a462de4218ab64ae8
### head=851268d8908ba13f07cf504a84775908185f98d7
### branch=feat/field-align
### date=2026-08-22T20:03:45-04:00
### AEON_DIR=/home/volence/sonic_hacks/.aeon-landing
### cargo_exit=0
```

`AEON_DIR` points at the clean built aeon checkout at `1ee8f8e6` (working tree
clean; `s4.bin`, `s4.debug.bin`, `demo.bin`, `demo.debug.bin` all present).
Deliberately NOT `.aeon-sigil-gates`, which is source-only and whose artifacts a
nightly lane deletes — an artifact-dependent run there reports ~127
`reference missing: …/s4.bin` failures that read exactly like a golden divergence.
Both aeon trees were read-only throughout; no test wrote into either.

| | |
|---|---|
| test binaries | 336 |
| **passed** | **3835** |
| **failed** | **0** |
| **ignored** | **4** |
| `skip:` lines | **0** |
| cargo exit | 0 |

Failures-first sweep of the whole log for `^failures:`, `FAILED`, `panicked at`,
`^error` — **zero matches**. Not a tail excerpt and not `grep | head`.

**Reconciliation against the tree, not against a remembered number:**

```
git grep -c '#[test]' HEAD -- '*.rs'  summed  =  3839 declared
passed 3835 + ignored 4                       =  3839   ✔ equal
```

Against the previous bar of 3821/0/4 (3825 declared): 3839 − 3825 = **+14**, which
is exactly the 14 tests this parcel adds. No test was displaced or silently lost.

**The log contains this parcel's own tests.** All 14 matched `^test <name> ... ok$`
with count ≥ 1 — a landing whose own new tests do not appear in its own green log
did not happen:

`field_align_satisfied_is_silent`, `field_align_violation_is_an_error`,
`field_align_error_is_not_silenced_by_the_odd_field_allow`,
`field_align_supersedes_the_odd_field_warning`,
`field_align_one_is_the_per_field_odd_field_opt_out`,
`field_align_even_offset_missing_a_wider_alignment_omits_the_cpu_claim`,
`field_align_non_power_of_two_is_refused`,
`field_align_zero_is_refused_not_a_modulo_by_zero`,
`field_align_takes_a_comptime_expression`,
`field_align_and_at_offset_are_independent_assertions`,
`field_align_and_a_wrong_at_offset_both_report`,
`field_align_accepts_either_attribute_order`,
`vars_form_align_on_a_struct_field_is_refused_by_name`,
`field_align_catches_an_insertion_that_re_parities_a_bridge`.

## 10. Diagnostic strings introduced — a cross-repo interface

Aeon fixtures assert on exact diagnostic text. A future rephrase of any of these
is a breaking change:

1. `layout.rs` — `struct {name}: field {f} declares (align: {n}) but lands at offset {off} — add {a} or remove {r} byte(s) of padding above it.` then optionally ` A word or long access to an odd address is a 68000 address error.` then ` Do not pin an @offset to satisfy this; a hand-written offset is the number that goes stale.`
2. `layout.rs` — `struct {name}: field {f} declares (align: {n}) but an alignment must be a power of two (1, 2, 4, 8, ...)`
3. `parser.rs` — ``\`@align(N)\` reserves space and belongs to a \`vars\` field; a struct field asserts its alignment with \`(align: N)\`, which moves nothing``
4. `parser.rs` — `duplicate \`@ offset\` on one field`
5. `parser.rs` — `duplicate \`(align:)\` on one field`
6. `parser.rs` — ``expected \`align:\` in field attribute list``

## 11. Open / not done

- **The spec is not updated.** `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §4.3 needs
  the `(align: N)` tier alongside `[layout.odd-field]`, and the §12 attribute list
  (line 775) needs it added. That is a different repo; flagged for the overseer
  rather than edited from this worktree.
- **T2 / `pad_to(N)`** — priced above, awaiting a ruling. Ledgered.
- **Whole-struct alignment** — scoped out, ledgered. A struct's base address is not
  known at its declaration, so the claim would have to be checked at every
  placement site: a link-time surface (where map items' `(align: N)` congruence
  asserts already live), not a layout-engine one. More than a small increment.
- **Overlay fields** cannot carry `(align: N)` — ledgered.
- **No runtime confirmation was attempted** (no emulator, per the standing
  invariant). Nothing here needs one: the feature is a compile-time diagnostic and
  emits no bytes.
