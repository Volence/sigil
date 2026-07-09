# Tranche 0 — language-completion sprint: COMPLETE, awaiting Volence checkpoint

Branch `tranche0-language` (off master `1fe2406`), 17 commits. **All nine brief
items built; the acceptance gate is green by demonstration** (the kickoff's
criterion): `tranche0_acceptance.rs` patches exactly the one
`(encoding: code_word, base: ObjCodeBase)` line of the D2.30 preview to
`word_offsets` and builds it through the real CLI with **zero diagnostics**.
`code_word` itself was NOT built (excluded by design — rides the first
scripted-object port). Final state: workspace **1764/0**, clippy clean,
**strict byte gates green** (`SIGIL_STRICT_GATE=1 AEON_DIR=…` — the mt/sfx/dac
reference-dependent suites ran hard against `aeon/s4.bin`, no skips).

Discipline as briefed: worktree, TDD per item (RED verified before every
implementation), two-stage code review per item (six review agents; every
CRITICAL/MAJOR fixed and pinned, choices recorded below).

## Per-item status (all DONE)

| # | Item | Commits | Notes |
|---|------|---------|-------|
| 1 | `todo!`/`unreachable!` (S2-D11e) | 21a631a, f41e5ae | `illegal` added to sigil-isa (0x4AFC — bonus: closes an @as_compat gap); `[todo.present]` per site; `--deny-todo` on `sigil emp` (build gains it on first demand); trap = fallthrough terminator |
| 2 | `///` doc comments (S2-D11d) | 957dba6, 0447e72 | lex + attach-to-item, `[doc.dangling]`; CRLF/attr-adjacency/recovery bugs review-caught + fixed; `//!` stays ledgered |
| 3 | `align N` (D2.29 §4.8) | 0dd4ccc, 21d6a7a | AS-parity vectors (3, in sigil-cli per the crate-graph invariant); `[align.provisional]`; review-caught parser hang on `align = 5` fixed |
| 4 | `[layout.odd-item]` (D2.29 amendment) | fe52259, 550b180 | link-time parity asserts (LinkAssert gained `level`); proc/script=error, wordy data/offsets/dispatch=warning (dispatch WITH bodies promoted to error); exemptions: Z80, @as_compat, LE cells, `@allow` (warning tier only) |
| 5 | struct `..` rest-fill (S2-D13h struct half) | a59758a, 550b180 | field defaults were ALREADY shipped; `..` added + **semantic tightening** (below); ObjDef vel/frame defaults in the prelude, byte-neutral |
| 6 | inline `offsets` bodies (§4.7) | 624a6fe, 9042e64, 35e41ae | `OffsetsTarget::Ref\|Inline`; required in-block-ordinal test green; per-body odd-item check + member-precise spans review-added |
| 7 | `yield shows` (D2.30a) | 82918cf | bare-label yield retired with a teaching error; byte-equivalence proven |
| 8 | `yield .label` (D2.30b) | d02dc63, 1b177b6 | shared first-need members, zero-cost park; duplicate labels now a spanned frontend error |
| 9 | `wait_frames` (D2.30c) | ff4017c, 1b177b6 | byte-identical to the hand tick idiom (u8 AND u16); width via the SAME field space operands use (overlay timers work — the acceptance shape); literal range check (`#0/#256/#-1` refused) |
| 10 | `comptime test` + `sigil test` (S2-D11a) | c076be3, 1b177b6 | expect_error = ERROR-level only; stripped always (byte-proven); duplicate names refused in BOTH paths; section-nested tests rejected loudly; --root sweep keeps going past broken modules |
| — | Acceptance | ddbd0f6 | the gate — plus the real bug it caught (below) |

## Decisions — 1 and 2 RESOLVED AT THE CHECKPOINT (Volence, 2026-07-09); 3-6 ratified as shipped

1. **Struct elision = NAMED, per field: `vel: default` (RATIFIED, reworked
   from the sprint's `..`).** Volence's review of the sprint's `..` marker:
   it says THAT something is elided, not WHICH — reading cold you can't tell
   `..` = vel. Reworked to `field: default` (contextual bareword in
   field-value position): every declared field appears in every literal, the
   VALUE collapses to "the declared default". Omission is always
   `[struct.missing-field]` (message teaches `name: default` when one
   exists); `default` on a defaultless field = `[struct.no-default]`; a const
   named `default` stays usable in arithmetic (contextual rule). Refactor
   property: a struct gaining a defaulted field makes every literal ERROR —
   a per-object checklist, never silent absorption. The bulk `..` is RETIRED
   (its spelling now errors teaching the replacement) and RE-LEDGERED with
   the gate "a struct with enough defaulted fields that per-field `default`
   reads as noise". **Bitfields keep silent zero-fill DELIBERATELY** (the
   flag-word idiom); `field: default` on a bitfield is an error. The preview
   exhibit demonstrates the spelling.
2. **`access: byte` section attribute (RATIFIED — the declarative shape;
   `@allow` kept as the generic hatch).** Volence's push: the dac shape isn't
   a lint violation to ignore, it's a fact the check was missing. A section
   may declare `(access: byte)` — its DATA cells are read byte-wise (the
   packed-record discipline), and the D2.29 word-parity check is exempted as
   a CONSEQUENCE of the declared fact. Positive information, reusable: a
   future lint can flag a 68k word READ against a byte-access section —
   suppression never could. Code items are NOT covered (instruction fetch is
   never byte-wise); a dispatch WITH inline bodies keeps its error-tier
   check. Unknown `access:` values refused. `dac_samples.emp` switched from
   `@allow` to `access: byte` — the tree now carries ZERO `@allow` uses; the
   mechanism stays wired (string-form ids, `[attr.allow-form]` on the
   unquoted typo, warning-tier only) as the last-resort hatch for future
   lints without a semantic home.
3. **`align` congruence link-assert** (Item 3, beyond the spec text): padding
   computes at the lowering baseline, but D2.25 places chained/map sections at
   link — every `align` therefore records `anchor % N == 0`, so placement
   drift fails the build naming the final address, never a silent misalign.
4. **The D2.30(b) note-tier yield+jbra collapse lint is DEFERRED**: the
   Diagnostic enum has no Note tier, and a Warning-tier version would flag the
   pinned v1 exhibit (which deliberately demonstrates the old idiom). Needs
   the tier first — proposed as its own small item.
5. **`sigil test` is module-local in v1** (colocated tests beside the fns they
   exercise — the campaign's actual shape). Cross-module imports in test
   bodies are the recorded next increment. Also: a test body that defers a
   link-time condition FAILS (never vacuously passes).
6. **`--deny-todo` on `sigil emp` only** — `build` routes through the harness
   and the aeon tree has no `todo!`; gains the flag on first demand.

## Real bugs the tranche caught outside its own items

- **`report_unresolved` rejected mid-name-`$` hidden labels** (`__dispatch$…`,
  `__offsets$…`) under the program path — `$` is unlexable in both frontends,
  so ANY `$`-bearing symbol is compiler-internal; fixed to `contains('$')`.
  Latent for dispatch inline bodies since 9a; first bitten by inline offsets
  in a multi-module build. The acceptance gate caught it.
- **The preview "not wired into any test" assumption was wrong** — `--map`
  module discovery parses every `.emp` under `examples/game`, so the
  non-compiling preview broke three suites. It now lives at
  `examples/previews/pitcher_plant_script_next.emp` (breadcrumb comment in the
  file); it cannot return to the game root until `code_word` lands.
- **`sigil-frontend-as` does not implement `even` at all** (probed while
  building the align parity vectors) — the D2.29 "AS `even` ports as
  `align 2`" translation is MANDATORY at port time on both frontends. The
  kickoff note says both anims files carry one `even` each; worth verifying
  how those files assemble today before the first anims port.

## Deferred with rationale (all recorded, none blocking)

- **`continue`/"loop end" for script loops — considered and NOT added**
  (Volence raised it at the checkpoint, reading the exhibit's empty `.rearm:`
  end-label). The case dissolves into an existing idiom: put the USER label
  at the loop TOP and branch there directly (`bhi .park`) — one transfer
  instead of label-then-back-edge, and the label names the STATE, which a
  builtin `continue` target never could. A `continue` STATEMENT can't serve
  the real case anyway (asm branches are conditional — you need a target,
  not a statement). Counts as evidence FOR the S2-D15/9c gate on `for`/
  `break`; if ports keep growing dummy end-labels regardless, that's the
  demand signal to revisit.

- Even-rounding of chained 68k section bases in `place_sequential`/
  `place_sections` (Item-4 review M1's structural half): an odd chained base
  makes the odd-item ERROR unactionable at module level (the fix-it now also
  names the placement remedy). A placer-level even-round is a byte-layout
  change on the emp-native no-map path — needs a ratification nod.
- Structural lint discriminant on `LinkAssert` (test filters key on message
  substrings today); `warning:` prefix in CLI diag rendering (pre-existing).
- `wait_frames` with a const-hidden out-of-range value (literals are checked
  width-in-hand; consts arrive unevaluated at the desugar layer).
- Doc comments above struct FIELDS warn dangling (field docs = future
  extension); `//!` module docs (non-breaking later).
- Shared `[todo.present]` const between frontend and CLI (next touch).

## Suggested checkpoint flow

1. ~~Ratify/veto the decisions~~ **DONE — 1 and 2 resolved (named elision +
   `access: byte`), 3–6 ratified as shipped.**
2. Merge `tranche0-language` → master (`--no-ff`), delete the worktree.
3. Spec passes for empyrean (docs cadence): §4.5 (named elision `field: default`; `..` retired+re-ledgered), §4.8 status →
   shipped (+ congruence-assert refinement + `access: byte` section attr + `[attr.allow-form]`), §5.6
   D2.30 status → shipped (+ the "named resume uses the header epilogue"
   sentence + the deferred note-tier lint), §6.5/§10 (`comptime test`,
   `todo!`/`unreachable!`, `///`), D2.29 row (even-not-in-AS discovery).
4. Port #1 (hblank) starts — it needs none of tranche 0, per the kickoff.
