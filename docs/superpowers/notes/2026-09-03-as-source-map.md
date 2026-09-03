# AS front end: a diagnostic says which file and which line

**Branch:** `parcel/as-source-map`. Closes the headline of
`2026-09-03-as-replacement-first-pass.md` §2 — *"not one diagnostic carries a file or a
line number"* — and, as that note predicted, unblocks §6, which could not decompose its
own bucket table without one.

## What the front end does now

`sigil_span::Diagnostic` always carried `primary: Span` and `SourceMap::location` always
resolved one. Two things were missing, and only one of them was the CLI.

1. **`sigil-frontend-as` built no `SourceMap` and stamped every span `SourceId(0)`.** With
   `include` splicing hundreds of files into one running assembly, a byte offset had no
   text to resolve against — the information was not being discarded, it was never formed.
   Each spliced file now registers under its own `SourceId`, and **`SrcLine` carries the
   file it was read from** rather than the assembler carrying "the current file". Per line
   rather than per assembler is what makes a macro come out right: a macro body captured
   in `macros.inc` and expanded at a call site in `main.asm` reports `macros.inc`, which is
   where the text a reader must fix actually lives.
2. **The CLI printed `error: {message}` and dropped the span.** The failing pass hands its
   map back through `sigil_frontend_as::Failure { diags, sources }`, and the CLI renders
   `file(line): error: message` — AS's own shape (`smps-bug.asm(9): error: …`), so a user
   moving off AS reads the same thing in the same place.

`SourceMap` gained per-source names, `add_named`, `name`, and `label(span) -> Option<String>`.
A span whose source is unnamed or absent from the map has **no** label and prints bare: a
whole-run failure (non-convergence) or a root that never opened must not be attributed to
line 1 of a file that had nothing to do with it.

`assemble_root` / `assemble` / `assemble_root_relocating` keep their signatures and their
`Vec<Diagnostic>` error; `assemble_root_located` is the variant that keeps the map.

## The corpus, before and after

`~/sonic_hacks/s2disasm` at `e45ebf3`, `sigil s2.asm` from the corpus's own directory.
Both tables re-derived this pass, not copied.

| | before | after |
|---|---|---|
| diagnostics | 237 | **237** |
| carrying a `file(line)` | **0** | **237** |

The count did not move, which is what a diagnostics-plumbing parcel owes.

| count | diagnostic | before | after |
|---|---|---|---|
| 121 | `trailing tokens in expression` | ✓ | ✓ |
| 53 | `unresolved rept count` | ✓ | ✓ |
| 30 | ``malformed number (hex needs a trailing `h`)`` | ✓ | ✓ |
| 22 | `expected mnemonic or directive after label` | ✓ | ✓ |
| 9 | `unknown directive or mnemonic` | ✓ | ✓ |
| 1 | `The RAM variable declarations are too large by $N bytes.` | ✓ | ✓ |
| 1 | `phase needs a constant expression` | ✓ | ✓ |

## What the locations immediately bought

**237 diagnostics are 128 distinct sites, and three of them are 112 of the report.**

| diagnostics | site | source line |
|---|---|---|
| 90 | `s2.macrosetup.asm(70)` | `rept ALLARGS` — inside the `ds` macro |
| 11 | `s2.constants.asm(374)` | `__LABEL__ = zoneID` |
| 11 | `s2.constants.asm(375)` | `zone_id_{cur_zone_str} = zoneID` |

The first line alone carries **every one** of the 53 `unresolved rept count` diagnostics
plus 37 `trailing tokens`. §6 of the first-pass note called its buckets undecomposable;
one line of output now decomposes 38% of the report.

One line can carry 90 diagnostics because it is a macro body: each `ds.` call re-expands
it. **It is not one per call site, and the gap is not explained here** — the corpus has 314
`ds.[bwl]` invocations in `s2.constants.asm` against 90 diagnostics. The pass does abort
partway (`fatal` at `s2.constants.asm(1882)` sets the abort flag), which would truncate the
count, but nothing here has tested that it is the cause.

### §6's open question, answered — and its hypothesis refuted

§6 recorded the `malformed number` bucket as *"measured and NOT explained"*, with an
untested hypothesis that macro expansion was synthesizing the offending tokens. **It is
not macro expansion.** The sites name themselves:

```
s2.constants.asm(25): error: malformed number (hex needs a trailing `h`)
   →  mapping_frame =		$1A
```

`$1A` is an ordinary 68000 hex literal. The lexer reads `$` as the **Z80 program-counter
token** whenever the active CPU is Z80 (`lexer.rs`, the `b'$' if cpu == Cpu::Z80` arm),
leaving `1A` as a bare digit-leading run — which is exactly the malformed-number rule. The
active CPU is Z80 because `s2.asm(55)` says `CPU 68000` **in upper case**, and this front
end matches directives and mnemonics by exact spelling. Reduced to two lines:

```
	CPU 68000                 →  up.asm(1): error: expected mnemonic or directive after label
	dc.w $1A                     up.asm(2): error: malformed number (hex needs a trailing `h`)

	cpu 68000                 →  00 1A
	dc.w $1A
```

The same root cause splits into two buckets on one detail: a `$`-hex whose digits contain
a letter (`$1A`, `$2F00`) is a malformed number, and one whose digits are all decimal
(`$6174`) parses as a decimal integer and reports `trailing tokens in expression` instead.

### Case sensitivity is the corpus's largest compatibility gap

AS accepts `CPU`, `EQU`, `STRUCT` and `ENDSTRUCT` in upper case — the corpus assembles
under AS, which is the evidence — while this front end matches directives and mnemonics by
exact spelling. Attributed site by site, that one difference accounts for **115 of the
237**:

| count | shape | example site |
|---|---|---|
| 62 | `$`-hex under the Z80 default, all-decimal digits → `trailing tokens` | `s2.constants.asm(8)`: `Size_of_SEGA_sound = $6174` |
| 30 | `$`-hex under the Z80 default, letter digits → `malformed number` | `s2.constants.asm(25)`: `mapping_frame = $1A` |
| 20 | uppercase `EQU` | `s2.constants.asm(336)`: `button_up: EQU 0` |
| 2 | uppercase `STRUCT` / `ENDSTRUCT` | `s2.constants.asm(1857)`, `(1863)` |
| 1 | the `CPU 68000` line that starts it | `s2.asm(55)` |

(Counted by matching each diagnostic against its own source line, not by assuming a bucket
is homogeneous — the `trailing tokens` bucket is not: 22 of its 84 constants-file rows are
lines 374/375, a different cause.) **This outranks the nameless-label finding by volume** —
that one is 1,875 sites in the source but zero diagnostics here, because the corpus dies
long before reaching them. Whether `@as_compat` should case-fold is a language-surface
decision and is NOT ruled here; it is booked, not fixed, and fixing it would move the 237.

One row is not a defect at all: `s2.constants.asm(1882)` is the corpus's own
`fatal "The RAM variable declarations are too large by $\{*} bytes."` firing — sigil ran the
source's error directive and interpolated `*` correctly.

## How this was verified

Four mutations, each applied to a committed baseline, quoted back from disk before the
run, and restored with `git checkout --` on a clean tree.

| mutation | predicted red | observed |
|---|---|---|
| `directive_include` reuses the includer's `SourceId` (the naive splice) | `a_diagnostic_names_its_own_file_and_line_across_an_include` | FAILED — reported `root.asm(2)` for an error on `part.asm(4)`, i.e. the includer at the `include` line |
| the CLI renderer drops the span (`eprintln!("error: {}", …)`) | both tests in `cli_diagnostic_location` | FAILED — `error: <message>`, no location on any line |
| `SourceMap::label` resolves every span against the FIRST source | `sigil_span … label_names_the_source_the_span_belongs_to` | FAILED — `Some("root.asm(4)")` where `Some("sub/part.asm(4)")` was required |
| `expand_macro` stamps expanded body lines with the CALL SITE's file | `an_error_in_a_macro_body_names_the_file_the_body_was_written_in` | FAILED — reported `mroot.asm(2)`, resolving the body's offset against the caller's text, where `mac.inc(3)` was required |

The first mutation is the one that matters: it reproduces exactly the failure mode the
gate exists to prevent, and its output is the naive implementation's answer.

**Runners.** `crates/sigil-cli/tests/cli_diagnostic_location.rs` (three tests, executed by
`cargo test -p sigil-cli`) and the `sigil-span` lib test
`label_names_the_source_the_span_belongs_to`. Both are in the workspace suite.

**Workspace suite**, `SIGIL_STRICT_GATE=1 AEON_DIR=…/.aeon-ref-201 cargo test --release
--workspace --no-fail-fast`, run from this worktree
(`sigil/.claude/worktrees/agent-a75a308c0b6ca9305`) on `parcel/as-source-map` at
`95bb25c0`: **4237 passed / 0 failed / 2 ignored** across 374 result lines, exit 0. An
earlier run at `ab99f17e` gave 4236/0/2 — the difference is the one macro-body test added
after it, and the two reconcile exactly.

**No aeon byte moved**, and the log says so directly rather than by the absence of a
failure: the run rebuilds and stamps the ROMs, at
`S1.4 plain: assembled=0xa5c82 full=719700 … crc=14ee2440` and
`S1.4 debug: assembled=0xa81fc full=737683 … crc=142294b3` — the same size/CRC pairs
`golden/provenance.toml` carries for the reference tree.

The two ignored are pre-existing and named as such by their own skip reasons —
`sigil_diff_reports_byte_identity` (reads the aeon source tree; `--ignored` only) and
`secondary_pin_classes_match_the_hand_typed_baseline` (retired by Wave-B B-0).

**There was no pre-existing red to reconcile against.** The dispatch expected one —
`repin_pins.rs`'s `DEBUG_ASSEMBLED_LEN` assert wanting `0xA7F38` against an advanced
corpus — and it is already closed on master: `pins.rs:44` reads `0xA81FC` and the assert at
`repin_pins.rs:1166` asserts `0xA81FC`, both landed by `ea95bc18` (*"repin baseline: the
corpus-pin-advance span's two terms"*). The suite is green, not green-minus-one.

## Still open

- **Two location formats in one binary — an owner call.** The `.emp` tier already located
  its diagnostics, and renders `path:line:col: level: message` (`render_program_diags`,
  and the `sigil emp` / `sigil test` paths). The AS tier now renders `file(line): level:
  message`, which is AS's own shape and therefore the one an outsider's editor and habits
  already parse. Each surface matching its own idiom is defensible, and one binary
  speaking two dialects is not; this parcel took the AS shape for the AS surface and
  leaves the reconciliation as a decision, not a silent divergence.
- **The call-site trail.** An error inside a macro body names the body's file and line, not
  the call site — AS reports the same way, and the body is the text a reader edits. Neither
  assembler prints the chain of call sites that led there, and for a deeply nested
  expansion that chain is what a user actually wants.
- **Case-insensitive directives/mnemonics** for `@as_compat` — the corpus's largest single
  gap (115 of 237). Booked, not fixed: it changes the count, and the case rule is a
  language-surface decision.
- **`s2.macrosetup.asm(70)`'s `rept ALLARGS`** — 90 diagnostics from one line, the `rept`
  count naming a macro-argument-count symbol. Now has an address; still needs a ruling.
- **Column numbers.** `SourceMap::location` returns `(line, col)` and the renderer uses the
  line only, matching AS. A macro-expanded line's column would be wrong anyway: argument
  substitution changes the text's length, so only the line survives the rewrite.
- **The harness's `fmt_diag_list`** still renders raw byte spans, with a comment saying the
  map is unavailable. `assemble_root_located` is now the map it wanted; wiring it is a
  separate parcel.
