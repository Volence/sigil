# Packet — struct pad markers `pad(N)` / `pad_to(N)` (Spec 2 §4.3.1, D2.38)

Branch `feat/struct-pad`, based on sigil master `67575c3c`.
Spec read at empyrean `2000b5ca03592377ea1881671db53e03ad36f264`, §4.3.1, at that
committed revision (never through the working-tree path).

## 1. What changed

| file | change |
|---|---|
| `crates/sigil-frontend-emp/src/ast.rs` | `PadKind` (`Count`/`To`), `StructPad { before, kind, operand, span }`, `StructDecl::pads` |
| `crates/sigil-frontend-emp/src/parser.rs` | `at_struct_pad()`; the marker branch in `struct_decl`; the `vars`-body `pad_to` refusal in `region_field` |
| `crates/sigil-frontend-emp/src/layout.rs` | `place_pads_before`, `pad_width`, `check_hand_counted_pad`; the pad walk woven into `layout_of_struct` |
| `crates/sigil-frontend-emp/src/eval/emit.rs` | `lower_struct` fills the layout's gaps and tail with `$00` |
| `crates/sigil-frontend-emp/tests/eval_layout.rs` | +32 tests (pad semantics, the lint, the align debt, the Scope clause) |
| `crates/sigil-frontend-emp/tests/lower_data.rs` | +6 tests (pad bytes, incl. `@as_compat`) |
| `crates/sigil-frontend-emp/tests/parser_decls.rs` | +5 tests (contextual parse, AST shape, `vars` refusal) |

Commits: `f4285515` (implementation + tests), `93dbee23` (a poison-found gap in one
assertion's matcher — see §4), plus this note.

### The one design call worth naming

Pads are held **beside** the fields (`StructDecl::pads`, each anchored by the index of
the field it precedes) rather than interleaved into one member list. A pad has no name,
so every consumer that looks a field up by name — struct literals, `offsetof`, the
emitted-shape check, `resolve_bare_window`, the harness's struct-offset harvest — is
asking a question a pad can never answer, and reads `fields` unchanged. The alternative
encoding would have required each of those to remember to filter, and forgetting is
silent. Only the two consumers of a pad's WIDTH change: the layout walk, and the
emission that follows its offsets.

Consequence worth stating plainly: `layout.fields` contains no pad, so a pad's bytes are
**the gap between one field's end offset and the next field's offset**, plus the tail
between the last field's end and `layout.size`. Emission derives the `$00` runs from
those offsets, never from what the fields emitted — a field that lowers to nothing (a
diagnosed shape mismatch) must not slide the pad.

### Deliberate deviations from the spec's literal rendering

1. **The lint tag is inline in the message** (`[layout.pad-overflow] struct S: …`).
   `sigil_span::Diagnostic` has `level`/`message`/`primary` and no code field, so a tag
   rendered as a separate header line has nowhere to live. Every other tagged
   diagnostic in this compiler embeds it (`[layout.odd-field]`, `[overlay.window-overflow]`,
   `[bitfield.overflow]`, `[struct.missing-field]`). The spec's sentence is preserved
   verbatim as a substring.
2. **`[layout.pad-count]` is the tag for BOTH spellings**, per the derivation table;
   only the noun varies (`count` for `pad`, `target` for `pad_to`), per the two
   enumerated strings.
3. **The non-int case interpolates the value's type name** — `pad(string) — a pad count
   must be a non-negative comptime int`. The spec spells only the negative-int rendering
   (`pad(-1)`); there is no operand value to quote when the operand is not a number, and
   the sentence is unchanged.
4. **`[layout.pad-hand-counted]` withdraws when the neighbouring `(align:)` is itself
   refused** (non-power-of-two, or an expression that does not evaluate). Not in the
   spec; it follows from two things the spec does say. The fix-it promises "the assertion
   still proves it", and a refused assertion proves nothing — so firing there is false
   advice. And the lint must evaluate the align expression to quote it, which
   `check_struct_field_align` also evaluates for the verdict; without the withdrawal an
   unresolvable name is reported twice. Same precedence as `(align: N)` superseding
   `[layout.odd-field]`. Pinned by `hand_counted_pad_lint_withdraws_from_a_refused_alignment_claim`
   and `hand_counted_pad_lint_does_not_double_report_an_unevaluatable_alignment`.
5. **`pad_to` in a `vars` body is refused by name**, as recommended. Recovered as a
   **zero-width** pad — the same width `[layout.pad-overflow]` continues with — so the
   region cursor lands exactly where it would with the line absent and no second,
   spurious placement diagnostic follows.

## 2. Clause-by-clause verification

Every gate below was proven by making it fail on purpose. The "poison" column names
the perturbation; each was applied, run, seen red with the named mismatch, and reverted.
22 poisons, 22 red.

| §4.3.1 clause | test | poison → the assertion seen failing |
|---|---|---|
| `pad(n)` width = n | `pad_count_occupies_bytes_and_moves_the_next_field` | covered by P5/P13 below |
| `pad_to(n)`, n > cursor → n − cursor | `pad_to_derives_the_width_that_lands_the_next_field_on_the_target` | **P1** width ← `n` → *"pad_to(4) must land b at 4"*, left 4 right 7 |
| `pad_to(n)`, n == cursor → 0, legal, inert, silent | `pad_to_at_the_cursor_is_legal_inert_and_silent` | P1 (same edit makes the inert case non-inert) |
| last-in-body pad's end offset IS the struct size | `pad_to_last_in_the_body_sets_the_struct_total_size` | **P13** drop the trailing `place_pads_before` → the `(size: 8)` diff fires |
| `pad_to(n)`, n < cursor → `[layout.pad-overflow]` ERROR | `pad_to_below_the_cursor_is_pad_overflow` | **P2** rename the tag → *was "SILENCED struct S: pad_to(2) before field b, …"* |
| …final-pad variant names the end of the struct | `final_pad_to_below_the_cursor_names_the_end_of_the_struct` | **P2** (after the §4 fix) → same, on the "at the end of the struct" arm |
| overflow takes width 0, layout continues, `(size:)` still diffs | `pad_overflow_takes_width_zero_so_the_size_diff_still_prints` | **P3** recover at width 1 → *"the failed pad must take width 0"*, left 5 right 4 |
| n < 0 or non-int → `[layout.pad-count]` ERROR | `negative_pad_count_is_refused`, `negative_pad_to_target_is_refused_and_says_target_not_count`, `non_int_pad_operand_is_refused_by_the_pad_count_rule` | **P4** accept negatives → no diagnostic at all; **P5** swap the nouns → *"a pad target must be…"* on the `pad(-1)` arm |
| operands are comptime expressions | `pad_operands_are_comptime_expressions` | P1/P4 reach it |
| `[layout.pad-hand-counted]` WARNING on `pad(N)` + `(align: N)` | `hand_counted_pad_before_an_aligned_field_warns` | P6/P7/P8 below all move it |
| …only on `pad`, never `pad_to` | `pad_to_before_an_aligned_field_does_not_warn` | **P6** drop the `PadKind::Count` test → *"the derived spelling is the fix"*, the lint fires on `pad_to(2)` |
| …only on the IMMEDIATE neighbour | `hand_counted_pad_lint_needs_the_aligned_field_to_be_the_next_one` | **P7** drop the `peek()` test → *"only the neighbouring pad can pair"* |
| …`@allow` covers the honest case | `hand_counted_pad_lint_is_silenced_by_its_allow` | **P8** ignore the allow → the warning survives the allow |
| …names the exact `pad_to(...)` to write | (asserted in the warn test: `Write pad_to(2) instead`) | P13 moves the derived number |
| …withdraws from a refused claim / no double report | `hand_counted_pad_lint_withdraws_from_a_refused_alignment_claim`, `hand_counted_pad_lint_does_not_double_report_an_unevaluatable_alignment` | **P10** drop the power-of-two withdrawal → the warning joins the `(align: 3)` error; **P9** drop the truncate → `unknown name \`NOPE\`` twice |
| contextual: marker only when followed by `(` | `a_field_named_pad_coexists_with_pad_markers`, `a_struct_field_named_pad_or_pad_to_still_parses` | **P14** drop the `(` test → *expected a clean parse, got "expected \`)\`, found Comma"* |
| no prefix collision with `pad_to_cycles` | `pad_to_cycles_is_not_a_pad_marker` + the whole aeon corpus (§3) | — (whole-token compare; the corpus gates are the behavioural witness) |
| `[layout.odd-field]` exempts pads | `pad_bytes_are_exempt_from_the_odd_field_lint`, `a_pad_does_not_shift_the_odd_field_lint_off_a_real_field` | **P15** push the pad into `layout.fields` as a named 2-byte entry → *"a pad must never trip odd-field: [layout.odd-field] struct S: field pad (2-byte) at odd offset 1"* |
| `sizeof` counts a pad's bytes | `sizeof_counts_a_pads_bytes`, `a_pad_inside_a_nested_struct_counts_toward_the_outer_layout` | P13 (tail), P1 (width) |
| `offsetof` cannot name one | `offsetof_cannot_name_a_pad`, `a_field_named_pad_coexists_with_pad_markers` | P15 (a pad in the field list becomes nameable) |
| pads fill with `$00` | `struct_pad_emits_zero_bytes_between_the_fields` (+3 more) | **P11** fill with `$FF` → all five byte-vector assertions fail |
| tail pad emits | `a_trailing_struct_pad_emits_after_the_last_field` | **P12** skip the tail fill → `[0x12,0x34]` vs `[0x12,0x34,0,0]` |
| `@as_compat` reproduces the `dc.b 0,…` run byte for byte | `struct_pad_bytes_are_identical_under_as_compat` | P11 |
| `(size: N)` stays an assertion, never auto-satisfied | `size_assertion_is_never_satisfied_by_auto_padding_the_tail` | (the counter-arm to the tail-pad test: no pad line, no pad) |
| struct literals untouched | the six `lower_data` tests (each names exactly the declared fields and lowers with zero diagnostics) | P15 (a pad in the field list becomes a `[struct.missing-field]`) |
| `vars` unchanged in v1; `pad_to` refused by name there | `pad_to_in_a_vars_region_is_refused_by_name`, `a_vars_region_keeps_its_own_pad_form` | **P16** rename the refusal → *"SILENCED derives its width from…"* |

### The owed `(align: N)` test debt

Coverage re-measured against **this** branch's base (`67575c3c`), not taken from the
brief. All six claims held: #1 well pinned; #2 pinned only on `must be a power of two`;
#3 only on ``asserts its alignment with `(align: N)` ``; #4/#5/#6 by nothing anywhere in
the repo (`git grep -F` over `*.rs`, hits only in `parser.rs` itself).

| debt | done | poison |
|---|---|---|
| #2 widen to the full string | `field_align_non_power_of_two_is_refused`, `field_align_zero_is_refused_not_a_modulo_by_zero` now `assert_eq!` the whole message | **P17** drop the `(1, 2, 4, 8, ...)` parenthetical → *"the whole string is the contract, parenthetical included"* |
| #3 widen to the full string | `vars_form_align_on_a_struct_field_is_refused_by_name` now compares the whole message | **P18** reword the leading clause → *got "`@align(N)` is not it; a struct field asserts…"* |
| #4 ``duplicate `@ offset` `` | `duplicate_at_offset_on_one_field_is_diagnosed` (new) | **P19** `duplicate` → `repeated` |
| #5 ``duplicate `(align:)` `` | `duplicate_field_align_on_one_field_is_diagnosed` (new) | **P20** `duplicate` → `repeated` |
| #6 ``expected `align:` `` | `an_unknown_field_attribute_keyword_names_the_one_that_belongs` (new) | **P21** reword → *got "expected an attribute keyword in field attribute list"* |
| §4.3 Scope clause | `field_align_does_not_propagate_into_a_nested_structs_own_fields` (new, both directions) | **P22** add a nested-propagating walk to `check_struct_field_align` → *"a nested struct's own claim must not be re-judged in the outer struct: struct Outer: field bridge declares (align: 2) but lands at offset 1"* |

The Scope test is built so the nested struct is internally CLEAN (its `bridge` satisfies
`(align: 2)` at inner offset 0) and lands at outer offset 1, so any propagating check
must report — and must not.

## 3. Byte identity

**Zero ROM bytes moved, in all four shapes.** Witnessed, not reasoned: every corpus and
golden gate in the suite is green (`native_full_sonic4_plain`/`_debug`, the seven
`*_regions_match_reference` families, `config_a`/`config_b`/`lean`, both demo shapes,
`pins_rs_is_current`). Nothing under `crates/sigil-harness/golden/`, `src/pins.rs` or
`repin.toml` was touched.

The corroborating source fact: all 26 `pad(` lines in the aeon corpus at `b08b35c0` sit
in `vars` bodies; **none** is in a struct body (classified by walking each `.emp` and
tracking the enclosing declaration). The `vars` path is untouched by this parcel.

The `pad_to_cycles` prefix collision the brief flagged does not exist: contextual keyword
matching is a whole-token compare (`at_kw`), `pad_to_cycles` lexes as one identifier, and
`engine/sound/z80_sound_driver.emp:455,556` compile into byte-identical ROMs.

## 4. The suite

```
AEON_DIR=/home/volence/sonic_hacks/.sigil-portfix-aeon SIGIL_STRICT_GATE=1 \
  cargo test --release --workspace --no-fail-fast
```

Log: `~/sonic_hacks/pad-suite.log`, stamped before cargo wrote to it with
`pwd=/home/volence/sonic_hacks/sigil/.worktrees/pad`, `head=93dbee23`,
`branch=feat/struct-pad`, `AEON_DIR`, `AEON_HEAD=b08b35c0`.

| | passed | failed | ignored | declared |
|---|---|---|---|---|
| master `67575c3c`, **measured in this environment** | 3885 | 0 | 4 | 3889 |
| `feat/struct-pad` `93dbee23` | **3928** | **0** | **4** | **3932** |

Zero `skip:` lines; zero `error: test failed` lines. `cargo clippy --workspace
--all-targets -- -D warnings` exits 0.

**Reconciliation.** `git grep -c '#[test]' HEAD -- '*.rs'` summed = 3932;
`passed + ignored = 3928 + 4 = 3932`. Exact. The delta of **+43** is accounted for
file-by-file, by `git grep -c` at both revisions:

| file | master | HEAD | Δ |
|---|---|---|---|
| `tests/eval_layout.rs` | 36 | 68 | +32 |
| `tests/lower_data.rs` | 42 | 48 | +6 |
| `tests/parser_decls.rs` | 59 | 64 | +5 |
| | | | **+43** |

**The brief's stated bar of 3881/3885 is 4 low.** Measured here, master `67575c3c`'s
sources run **3885 passed / 4 ignored / 3889 declared** against the same AEON_DIR, and
`git grep -c '#[test]'` at `67575c3c` returns 3889, agreeing with the run rather than
with the brief. So the delta against the *measured* master bar is +43, matching the
declared-count delta exactly; the delta against the *stated* 3881 would have been +47,
which is 4 tests that do not exist. Anyone re-deriving the bar should measure it rather
than quote it.

### The AEON_DIR the parcel was dispatched with is the wrong revision

The dispatch named `AEON_DIR=/home/volence/sonic_hacks/.aeon-landing` (aeon `94b384a2`)
and verified its four artifacts firsthand. Those artifacts are real; they are **not** the
ones sigil master's goldens are frozen against. `golden/provenance.toml`'s chain tip
(entry `replay-restamp`, landed by sigil `029868e5` "paired with aeon `b08b35c0`") wants
s4 `654bcd74`/699672 and s4.debug `f8d06cae`/715582; `.aeon-landing` carries
`b96319e3`/699408 and `7be32302`/715308. The demo shapes match; the s4 shapes do not.

Run against it, the suite reported **71 failures** that read exactly like a byte-moving
regression — `` `Ground_Move_Cap` resolved to 0x10902, expected 0x10912``, "assembled
prefix diverges from asl at 240 offset(s)". **The identical 71 failures occur with
master's own sources** in the same worktree, which is what settled it.

`/home/volence/sonic_hacks/.sigil-portfix-aeon` is a clean detached worktree at
`b08b35c0` whose four ROMs match the chain tip on CRC32 **and** size, and is the tree
every number above was measured against. Read only; nothing was built or cleaned there.

### One green leak, found by the poison pass

`final_pad_to_below_the_cursor_names_the_end_of_the_struct` asserted the message's
sentence but not its `[layout.pad-overflow]` tag, so P2 (rename the tag) left it green
while its sibling went red. Fixed in `93dbee23`; both arms now name wording unique to
the rule, and P2 was re-run to red on the fixed test. This is the bar working: a
uniqueness grep over the source would not have found it, because the sentence *is*
unique — it was the assertion that was too narrow, in the other direction.

## 5. Nightly source-gates self-audit

No new `crates/*/tests/*.rs` file was added — all three test files are modifications —
so the classifier's input set is unchanged in shape. Replayed against this branch
anyway: **`SOURCE_GATES=39 scanned=121 unclassified=0`** (artifact-lane files: 82;
39 + 82 = 121). The load-bearing figure, `unclassified`, is 0.

## 6. Left open

- **`pad_to` in `vars` regions** — deliberately v1-deferred by the spec (a region's
  coordinate is a VMA, and `@align(N)` already moves that cursor). Refused by name with
  a teaching diagnostic, as recommended and as tested.
- **A pad whose width is derived from an alignment** (`pad_to_align(2)` or similar) —
  §4.3.1 says explicitly that this is a separate construct needing its own ruling. Not
  built, not designed. Booked in the gap ledger as a language ask.
- **`docs/DEFERRED_WORK.md` does not exist** in sigil (or in empyrean). Nothing was
  closed or discovered there because there is no there; the deliverable is noted as
  unfulfillable rather than silently skipped.
- **Nothing needs runtime confirmation.** No ROM byte moved, so there is no emulator
  follow-up to tag.
