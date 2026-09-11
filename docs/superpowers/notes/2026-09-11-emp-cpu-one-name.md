# `.emp` processor names: one name each, `m68k` refused by name

Date: 2026-09-11. Branch `parcel/emp-cpu-one-name`, base `5e3d389a`, code
commit `e3e25546`.

## The decision this lands, and whose it is

Decision card `d-27` (`docs/decisions.jsonl`) asked whether `.emp` keeps three
processor spellings (`z80`, `m68000`, `m68k`) or settles on one per processor.
It is answered `one_name_each` **by the coordinating hub session**, under the
owner's delegation "answer any you can", taking this lane's own recommendation.
It is a hub answer, not the owner's own words, and the owner can overturn it
with one word.

The ruling: `z80` and `m68000` are the spellings. `m68k` is refused by name,
with a message giving the line to write instead.

## What the front end does

`crates/sigil-frontend-emp/src/lower/mod.rs`:

- `CPU_SPELLINGS` has one row per processor: `m68000`, `z80`.
- `CPU_REFUSED_ALIASES` pairs `m68k` with `m68000`. `cpu_alias_replacement`
  looks a name up in it, folding case like `cpu_for_spelling`.
- `attr_cpu` (the one resolver both call sites use) tries the accepted table
  first. On a miss it checks the alias table, but only for a single-segment
  path, so `foo.m68k` is a typo, not an alias. An alias earns
  `refused_cpu_alias`; anything else earns `unrecognized_cpu`. Neither path
  defaults.

The refusal, verbatim, for `section s (cpu: m68k, vma: $0)`:

```
processor `m68k` is spelled `m68000` in .emp: write `cpu: m68000`. .emp has one name for each processor, so a reader never meets two spellings and has to wonder whether they select two different targets.
```

It echoes the value as written (`M68K` stays `M68K`) and points its span at the
value. It names ONE fix, not the menu the typo refusal prints. The typo refusal
lists the table, which now reads `cpu: m68000`, `cpu: z80`.

## Consumer enumeration

Enumerated by what each site does, not by grepping `m68k` alone: every place
that parses, accepts, documents, tests or prints a `.emp` processor name.
Paraphrase searches: `68k`, `M68K`, `m68000`, `cpu:`, `processor name`,
`spelling`, plus every reader of the `cpu` attribute key.

### Accept (parse or resolve a processor name)

| Site | What it does | Action |
|---|---|---|
| `lower/mod.rs` `CPU_SPELLINGS`, `cpu_for_spelling` | the accepted table | `m68k` row removed |
| `lower/mod.rs` `CPU_REFUSED_ALIASES`, `cpu_alias_replacement` | the named-refusal table | new |
| `lower/mod.rs` `attr_cpu` | refusal point for both heads | alias branch added |
| `lower/mod.rs` `section_attrs` (`"cpu" => attr_cpu`) | section-head call site | unchanged code, now refuses `m68k` |
| `lower/mod.rs` `module_declared_cpu` (`"cpu" => Some(attr_cpu(..))`) | module-head call site | unchanged code, now refuses `m68k` |
| `lower/mod.rs` `attr_cpu_opt`, read again by `validate_inout_boundaries` | silent re-read of an already-checked section value | unchanged: accepted spellings only, so a refused value inherits the enclosing processor and the file still carries its one refusal |
| `frontend-emp/src/corpus_contracts.rs` `module_is_z80` | classifies `(cpu: z80)` modules through `cpu_for_spelling` | unaffected: only the Z80 answer is read, and `m68k` never gave it |
| `sigil-harness/src/bin/cycle_fraction.rs` `attrs_are_z80` | same, for the cycle measurement | unaffected, same reason |
| `sigil_frontend_as::CPU_SPELLINGS` (`68000`, `68008`, `z80`, `z80undoc`) | AS `cpu` directive | out of scope: a different surface that has never accepted `m68k`, and another lane's crate |

These are the only two positions where a processor name can appear in `.emp`:
`ModuleDecl.attrs` and `SectionDecl.attrs` (`ast.rs`). Item-level `@` attributes
carry no processor, and the CLI has no `--cpu` flag. `LowerOptions.initial_cpu`
is a `Cpu` value, never a spelling.

### Document

| Site | Action |
|---|---|
| `examples/main.emp` lines 29 to 33 (five `cpu: m68k`) | now `cpu: m68000`; the `z80drv` row realigned; the "68k reset/exception vectors" comment now says 68000 |
| `docs/superpowers/plans/2026-07-06-offset-table.md` lines 635, 759, 806, 825 (fixture strings) | now `cpu: m68000` |
| `docs/superpowers/specs/2026-07-07-spec2-plan7-item4-module-resolution-design.md` line 58 | now `cpu: m68000` |
| `lower/mod.rs` doc on `CPU_SPELLINGS` (called the canonical spelling "a question for the owner") | rewritten to state the rule and cite `d-27` as hub-answered |
| `lower/mod.rs` doc on `section_attrs` (said `cpu:` "defaults to `M68000` (`z80` selects Z80)", wrong since the typo refusal landed) | now names the accepted set and the refusal |
| `lower/mod.rs` comment in `validate_inout_boundaries` ("an unrecognized value") | now "a refused value", covering both refusals |
| `ast.rs` docs for `ModuleDecl.attrs` and `SectionDecl.attrs` | cite only `cpu: z80`; no change |

Records left as they stand. These are dated history or append-only logs, not
instructions anyone follows:
- `docs/superpowers/notes/2026-09-09-ux-seat-b-heuristic-audit.md:105`: lists the table as it stood that day (`m68000`, `m68k`, `z80`).
- `docs/OVERSEER-REFERENCE.md:2851-2853`: the overseer's reasoning about the grandfathering; that doc belongs to the overseer.
- `docs/2026-09-09-audit-briefing.md:17`: states the `d-27` answer, consistent with this parcel.
- `docs/decisions.jsonl`, `docs/lane-log.jsonl`, `docs/OVERSEER-LOG.md`.

Not processor names:
- `README.md:65`: `m68k` is the `sigil-isa` module name.
- The crate name `sigil-backend-m68k`, the `m68k_cycles` module and the `RegFile::M68k` type are code identifiers.
- The `.m68k.emp` scopes in `editors/vscode/syntaxes/emp.tmLanguage.json` are TextMate scope names. No highlighter rule matches `cpu:` values.

### Test

| Site | Action |
|---|---|
| `crates/sigil-cli/tests/emp_cpu_attr_spelling.rs` | the gate; assertions rewritten (below), 7 tests added |
| `crates/sigil-frontend-emp/tests/z80_operands.rs:224-238` | mismatch message must name `m68000`; unchanged, consistent |
| `crates/sigil-frontend-emp/tests/corpus.rs:15` | asserts the attribute KEY is `cpu`; unaffected |
| `crates/sigil-cli/tests/cpu_undeclared.rs` | refuses a unit with no processor; unaffected |
| `crates/sigil-cli/tests/cpu_spellings.rs` | the AS surface; out of scope |

### Print

| Site | Action |
|---|---|
| `lower/mod.rs` `cpu_name` (`[module.cpu-mismatch]`) | prints `m68000` / `z80`, already the accepted spellings; gated by `a_diagnostic_that_names_a_processor_names_an_acceptable_spelling` |
| `lower/mod.rs` `unrecognized_cpu` | lists `CPU_SPELLINGS`, so now prints `cpu: m68000`, `cpu: z80` |
| `lower/mod.rs` `refused_cpu_alias` | new |
| `Debug` output of `Cpu` (`M68000`, `Z80`) in maps and listings | Rust enum names, not `.emp` source; they match the accepted spellings once case is folded |
| `sigil-frontend-as/src/eval.rs:5654` (`Z80` / `68000`) | AS surface; out of scope |

## Assertions retired or rewritten, and why

Before the sweep I grepped the pre-sweep revision (`5e3d389a`) for negative
assertions keyed to `m68k`, both as a word and inside `assert` / `contains` /
`matches` / `is_none`. Outside the gate file every hit is an ISA, crate or test
identifier. Inside it, the one a mechanical change would silently have killed
is not literally keyed to `m68k`:

1. **`refusals()` matched only the prefix `unrecognized processor`.** Every "no
   refusal" assertion in the file reads through it (`every_accepted_spelling_selects_the_target_the_table_names`,
   `the_spellings_aeon_writes_today_are_accepted`, the derived aeon gate, the
   first-contact gate). A refusal with its own wording would pass all of them
   unseen, so `examples/main.emp` could have taken `cpu: m68k` back with its
   gate green. Widened to both refusal shapes, with the reason in its doc.
   Proof status: this widening is defence in depth and is NOT separately
   red-proven. In the first-contact gate the text census (point 3) now fires
   before the lowering, so no single mutation isolates the helper.
2. **`the_same_fixture_under_an_accepted_spelling_assembles`** used `m68k` as
   its accepted control and asserted it assembles to `11 11`. The ruling makes
   that false. The control now uses `m68000`, and the new
   `the_shipped_command_refuses_m68k_and_prints_the_line_to_write` asserts the
   opposite for `m68k` at the process level.
3. **`the_first_contact_examples_processor_declarations_all_resolve`** had the
   precondition `text.matches("cpu: m68k").count() >= 5`, which made the gate
   about `m68k`. Replaced with a derived census:
   - every `cpu:` value in the file's text must be an accepted spelling, and an alias found there is reported with its replacement;
   - the count is tied to the six sections the lowering half checks, so the census and the lowering cover the same declarations.

   Its refusal message ("Refusing `m68k` breaks it on the first command anyone types") was rewritten.
4. **The file header** ("Why the accepted set is `m68000` / `m68k` / `z80`",
   and the owner-question framing) was rewritten to state the rule and cite
   `d-27` as hub-answered. The "What was silently wrong" history section, the
   case-arm comment and the dotted-path doc were rewritten in the present tense.
5. Tests that iterate `CPU_SPELLINGS` narrowed with the table and needed no edit:
   - the section-head acceptance loop;
   - the typo refusal's "lists every accepted spelling".

## Outside sigil

- **aeon**, read at committed `origin/master` `c773129d` (fetched 2026-09-11,
  `git grep -n -i m68k origin/master -- '*.emp'`). **Two hits, not zero**, and
  both are comments:
  - `engine/debug/compression_selftest.emp:377`: `// m68k relaxation delta is even)...`
  - `engine/objects/core.emp:553`: `// COSTS (sigil's table, crates/sigil-isa/src/m68k_cycles.rs, ...`

  Neither is a processor-name site, and neither is read by the `cpu:` attribute.
  The `cpu:` census at that revision is `m68000` x9, `z80` x24, and one prose
  match (`cpu: the`, a comment). Zero `m68k` in any `cpu` position. The
  ruling's premise, that there are no game-side sites, holds. The brief's
  "expected zero" did not, because the grep is over text rather than over
  processor declarations.
- **empyrean**, read at `origin/main` `1c7a17ac` (not edited):
  - 448 case-insensitive hits over 84 files. The hits were classified in two passes: a filter for the benign shapes, then a read of the whole residual, not just a sample. They are the 68000 track name (`M68K.md`, `wiki/m68k/*`), crate and type names (`sigil-backend-m68k`, `isa-m68k`, `M68kBackend`), the competitor assembler `ASM68K`, reference-core names, or hardware prose ("the Co-Processor to the M68k").
  - No hit writes `cpu: m68k` or defines `m68k` as a `.emp` spelling (`git grep -n -i -E 'cpu *: *m68k' origin/main` exits 1 with no output).
  - `docs/SIGIL_SPEC2_LANGUAGE.md` writes only `cpu: z80` (870-871) and `cpu: m68000` (878), and `wiki/specs/2026-08-06-emp-language-factsheet.md` only `cpu: z80`. Both already match the ruling.
  - **One hub record goes stale when this lands.** `docs/OVERSEER-PENDING-BARS.md:137` says, in the present tense, "The parcel grandfathers the observed set (`z80`, `m68000`, `m68k`), refuses everything else BY NAME". Line 94 describes the same parcel's landing and its census catching sigil's `m68k`. Both are accurate for `5ce742cf`. After this parcel, line 137's description of the accepted set is out of date. Reported for the hub, not edited.

## Test evidence

Tests added to `emp_cpu_attr_spelling.rs`:
- `each_processor_has_exactly_one_accepted_spelling`: the ruling as a table fact.
- `every_refused_alias_names_a_replacement_the_attribute_accepts`
- `m68k_is_refused_by_name_with_the_line_to_write`: `m68k`, `M68K` and `M68k` at the section head and the module head. Each is refused exactly once, by the alias refusal, echoed, `write \`cpu: m68000\``, and no other `cpu:` offered.
- `the_alias_refusal_points_at_the_offending_value`
- `a_near_miss_of_an_alias_is_refused_as_unrecognized`: `m68kk`, `m68`, `xm68k`, `foo.m68k` stay typos.
- `every_accepted_spelling_works_at_the_module_head`: both spellings, both cases, clean, seeding the named target. The section-head half already existed.
- `the_shipped_command_refuses_m68k_and_prints_the_line_to_write`: exits nonzero, prints the line, writes no binary.

Genuine typos are still refused at both heads by the existing tests (`banana`, `z81`, `m6800`, `68020`, `gbz80`, `68000`, dotted paths).

### Red-first

Each mutation was applied with an edit, and the mutated line was quoted from
disk (`grep -n`) with `git diff --stat` showing one changed line. The gate file
was run, then the file was restored by writing the committed blob back
(`git cat-file -p HEAD:<path> > <path>`). Each restore was verified by
`git hash-object` equalling `git rev-parse HEAD:<path>`, with an empty
`git diff --stat`: `lower/mod.rs` `b48a7e77`, `examples/main.emp` `4c302e29`.
Baseline commit `e3e25546`.

| # | Mutation (quoted from disk) | Result | Red tests |
|---|---|---|---|
| M1 | `lower/mod.rs:2109: ("m68k", Cpu::M68000),` (the refusal back to acceptance, via the table) | 14 passed, **5 failed** | `each_processor_has_exactly_one_accepted_spelling`, `every_refused_alias_names_a_replacement_the_attribute_accepts`, `m68k_is_refused_by_name_with_the_line_to_write`, `the_alias_refusal_points_at_the_offending_value`, `the_shipped_command_refuses_m68k_and_prints_the_line_to_write` (the command exited 0) |
| M2 | `lower/mod.rs:2120: pub const CPU_REFUSED_ALIASES: &[(&str, &str)] = &[];` (`m68k` falls through to the typo refusal) | 15 passed, **4 failed** | `every_refused_alias_...` (precondition), `m68k_is_refused_...` (left `None`, right `Some("m68000")`), `the_alias_refusal_points_...`, `the_shipped_command_refuses_m68k_...` (the command still failed, but printed the menu, not the line) |
| M3 | `lower/mod.rs:2225: Some(spelling) => return cpu_for_spelling(spelling).unwrap_or(Cpu::M68000),` (the refusal back to acceptance, via the alias path) | 16 passed, **3 failed** | `m68k_is_refused_...` (0 refusals), `the_alias_refusal_points_...`, `the_shipped_command_refuses_m68k_...` (the command exited 0) |
| M4 | `examples/main.emp:29: section vectors  (cpu: m68k, vma: $000000)` (an alias back in the first-contact file) | 18 passed, **1 failed** | `the_first_contact_examples_processor_declarations_all_resolve`: "declares `cpu: m68k`, which the attribute refuses: write `cpu: m68000`" |

M2 is why the table-coherence and fix-it assertions are separate from the
refusal-count ones: the typo refusal keeps the command failing, so a test that
checked only the exit status would stay green.

### Scoped suites

All runs were at `e3e25546`, from this worktree, with no reference tree named:
`SIGIL_ALLOW_PARTIAL=1`, a private `CARGO_TARGET_DIR`, `--release
--no-fail-fast -- --nocapture`. Each log carries a `STAMP pwd=` line naming the
worktree. HEAD was `e3e25546` before and after.

| Crate | Result lines | Passed | Failed | Ignored | Exit | Unmeasured (partial-run banner) |
|---|---|---|---|---|---|---|
| `sigil-frontend-emp` | 130 | 2668 | 0 | 0 | 0 | banner printed once: "130 test binaries are reference-dependent and every row in them is left UNMEASURED" |
| `sigil-cli` | 165 | 778 | 0 | 1 (`sigil_diff_reports_byte_identity`, "reads the aeon source tree; run with --ignored") | 0 | banner printed 104 times, each stating the same 130 |

The banner's 130 is one workspace-wide figure, printed identically by every
binary that loads the harness. This run does not break it down per crate, and
this note does not either.

The `sigil-cli` log contains the parcel's own test names as `ok`
(`m68k_is_refused_by_name_with_the_line_to_write`,
`the_shipped_command_refuses_m68k_and_prints_the_line_to_write`,
`each_processor_has_exactly_one_accepted_spelling`,
`the_first_contact_examples_processor_declarations_all_resolve`). The two tests
that depend on repository state also passed:
`version_reports_the_head_of_the_tree_it_was_built_from` and
`the_published_line_states_this_revision_s_position_against_a_named_remote_ref`.

Clippy, `cargo clippy --release -p <crate> --all-targets -- -D warnings`:
`sigil-frontend-emp` exit 0, `sigil-cli` exit 0.

## What the brief and the card got wrong

- The brief expected zero aeon hits for `git grep -i m68k -- '*.emp'`. There
  are two, both comments. Its STOP condition was keyed to the raw count, but the
  ruling's premise is about processor declarations, and there are none. This
  parcel did not stop, for that reason.
- `d-27` priced the change at seven lines: five in `examples/main.emp` and "two
  lines in two internal design documents". The two documents carry five lines
  (four fixture strings in the offset-table plan, one in the module-resolution
  spec), so ten in all.
