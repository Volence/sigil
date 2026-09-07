# AS-MESSAGE-AND-INTERPOLATION: `message` reaches stdout, and `\{expr}` pastes floats and strings

Parcel note. Branch `parcel/as-message-interp`, base master `0a07b4ac`, code
commit `67fdea97`, this note on top. Closes the two ledger sections dated
2026-09-06: "`message` prints nothing, on every pass, and asl prints it to
stdout" and "`\{expr}` interpolation drops a float-division form".

## Provenance

| | |
|---|---|
| sigil before | master `0a07b4ac`, `sigil-cli` release binary md5 `7a07c7804e9e26690189c5590f73e117`, built into `/home/volence/sonic_hacks/.target-msg` |
| sigil after | `67fdea97`, md5 `17d1b6c92ac5872bacfb504751fb5c20`, same target dir |
| reference asl | md5 `61e672562465725a8c102288a7da9098` through `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run`; exit status and `ASL_DIAG` read on every run; no value in this note comes from a run that exited non-zero |
| asl source | `asl-current-142-bld212.tar.bz2` from the upstream site, `version.c` `"1.42 Beta [Bld 212]"`, read for `asmsub.c::FloatString` and `strutil.c::FloatConvert`; the port is checked against the binary, not against the source alone |
| corpora | `s1disasm` `f6ece65`, `s2disasm` `e45ebf3`, `skdisasm` `2fcd861`, each a detached worktree under `/home/volence/sonic_hacks/.scratch-msg/corpus/`, prepared with `scripts/corpus-prepare.sh` (8 / 74 / 100 generated files; `corpus-baseline.sh` reports READY 4/4, 39/39, 50/50); the shared checkouts were never written |
| probes | `2026-09-07-as-message-interp-probes/`, `./run.sh`, three runs each, stdout byte-identical across runs for every probe |
| emulator | none |

## What asl does with `message`

Stdout, unprefixed, once per pass that reaches it. Probe `p4` has one
`message "fwd \{Later}"` above `Later:` and prints

```
fwd 0
fwdf 0
pass 1
fwd 3
fwdf 3
pass 2
```

on a two-pass run. Both corpus sites that print are inside `if MOMPASS=2`
(s1disasm `sound/z80.asm(231)`, s2disasm `s2.asm(91272)`), so each prints
once on asl.

Sigil records every firing per pass and returns the CONVERGED pass's list
(`Assembled::messages` on success, `Failure::messages` on a failing run,
because asl prints the line when it is reached and the failure comes later;
s1disasm is that shape). The CLI prints the list with `println!`, before any
diagnostic, on both arms. The rule is one line per firing from the final pass
only; sigil runs 3 to 5 pass evaluations where asl runs 2, so printing per
pass would print sigil's pass count. For `p4` sigil prints `fwd 3` / `fwdf
3` / `pass 2` once each. A `message` reached on the first pass only (`if
MOMPASS=1`) is not printed; asl prints it, and the difference is pinned as a
decision in `as_message_stdout.rs`.

Deleted on purpose: `a_message_is_dropped_on_every_pass_including_the_final_one`
in `as_fatal_survives_its_pass.rs`, which pinned the old behaviour.

## The interpolation probe table

Shape, asl's rendering, sigil's rendering after. Every asl cell is from a
probe that exited 0 with `ASL_DIAG=complete`. "same" means byte-identical to
asl's line.

### Integers (probe `p1b`, `cpu 68000`)

| expression | asl | sigil after |
|---|---|---|
| `42` | `2A` | same |
| `$FF` | `FF` | same |
| `n` (`n equ 42`) | `2A` | same |
| `(n+3)*2` | `5A` | same |
| `-1` | `FFFFFFFFFFFFFFFF` | same |
| `0` | `0` | same |
| `7/2` | `3` | same |
| `*` | `0` | same |
| `(*)&$FFFFFFFF` | `0` | same |
| `3<4` | `1` | same |
| `n} and \{h` (two in one string) | `2A and FF` | same |
| `'A'` | `A` | **`41`** (open, below) |
| `$` under `cpu 68000` (probe `p1`) | asl refuses: `error #1020: invalid symbol name`, exit 2 | not a cell |

### Z80 program counter (probe `p6b`, `cpu z80`)

| expression | asl | sigil after |
|---|---|---|
| `Uncompressed driver size: \{$}h bytes.` after three `nop`s | `Uncompressed driver size: 3h bytes.` | same |

### Strings (probe `p3`)

| expression | asl | sigil after |
|---|---|---|
| `s` (`s equ "abc"`) | `abc` | same |
| `t` (`t := "xyz"`) | `xyz` | same |
| `soundBank \{s} has $\{$8000+n-*} bytes free at end.` | `soundBank abc has $802A bytes free at end.` | same |
| `Table \{s} has \{n/1.0} entries, but it should have \{(n)/1.0} entries` | `Table abc has 42 entries, but it should have 42 entries` | same |

### Floats (probes `p2s`, `p7s`, `p8`)

| expression | asl | sigil after |
|---|---|---|
| `(2048-1024)/1024.0` | `1` | same |
| `1536/1024.0` | `1.5` | same |
| `n/1.0` | `42` | same |
| `$400/1024.0` | `1` | same |
| `1024.0` | `1024` | same |
| `3.5` | `3.5` | same |
| `2.5*4` | `10` | same |
| `0.0` | `0` | same |
| `-2.5` | `-2.5` | same |
| `-1024.0` | `-1024` | same |
| `0.1` | `0.1` | same |
| `0.1+0.2` | `0.3` | same |
| `123456789.5` | `123456789.5` | same |
| `1234.5678` | `1234.5678` | same |
| `fs` (`fs equ 2.5`) | `2.5` | same |
| `fi` (`fi equ 1024.0`) | `1024` | same |
| `fs*2` | `5` | same |
| `fi/2` | `512` | same |
| `3.5<4` | `1` | same |
| `3.5>4` | `0` | same |
| `1/3.0` | `0.33333333333333` | same |
| `2/3.0` | `0.66666666666666` | same |
| `1/7.0` | `0.14285714285718` | same |
| `2/7.0` | `0.28571428571427` | same |
| `1/9.0` | `0.11111111111111` | same |
| `22/7.0` | `3.142857142857143` | same |
| `100.0/3` | `33.3333333333334` | same |
| `1000000.0/3` | `333333.333333333` | same |
| `3.14159265358979` | `3.14159265358979` | same |
| `2.718281828459045` | `2.718281828459045` | same |
| `9.99999999999999` | `9.999999999999989` | same |
| `0.99999999999999999` | `1` | same |
| `-1/3.0` | `-0.3333333333333` | same |
| `0.1+0.7` | `0.79999999999999` | same |
| `4.0/3` | `1.333333333333333` | same |
| `10.0/3` | `3.333333333333333` | same |
| `5.0/3` | `1.666666666666667` | same |
| `1.23456789012345` | `1.23456789012345` | same |
| `123456789.123456789` | `123456789.123458` | same |
| `0.1234567890123456789` | `0.12345678901237` | same |
| `123456789012345.678` | `123456789012370` | same |
| `1234567890123456.0` | `1234567890123600` | same |
| `12345678901234567.0` | `12345678901237000` | same |
| `123456789012345678.0` | `123456789012370000` | same |
| `12345678901234567890.0` | `1.2345678901237E19` | same |
| `1.0e14` .. `1.0e17` | `100000000000000` .. `100000000000000000` | same |
| `1.0e18`, `1.0e19`, `1.0e20`, `1.0e21`, `1.0e22`, `1.0e100` | `1E18` .. `1E100` | same |
| `1.5e20`, `1.25e21`, `-1.0e21` | `1.5E20`, `1.25E21`, `-1E21` | same |
| `1.0e-4` .. `1.0e-10`, `1.5e-7`, `1.5e-8`, `-1.0e-10` | `0.0001` .. `0.0000000001`, `0.00000015`, `0.000000015`, `-0.0000000001` | same |
| `1.0e-15` | `0.000000000000001` | same |
| `1.0e-20` | `9.999999999999E-21` | same |
| `0.01`, `0.001`, `0.0001`, `0.00001` | `0.01`, `0.001`, `0.0001`, `0.00001` | same |
| `123456.0`, `1234567.0`, `12345678.0`, `1000000.0`, `10000000.0` | `123456`, `1234567`, `12345678`, `1000000`, `10000000` | same |

### The corpus lines (probe `p5b`)

| expression | asl (pass 2) | sigil after |
|---|---|---|
| `ROM size is $\{EndOfRom-StartOfRom} bytes (\{(EndOfRom-StartOfRom)/1024.0} KiB). About $\{paddingSoFar} bytes are padding. ` | `ROM size is $5 bytes (0.0048828125 KiB). About $3 bytes are padding. ` | same |
| `#\{n/1.0}: line=\{MOMLINE/1.0} PC=$\{(*)&$FFFFFFFF}` | `#7: line=9 PC=$3` | **`#7: line=\{MOMLINE/1.0} PC=$3`** (open, below) |

102 interpolation cells in all; 100 byte-identical; 2 open.

## asl's float rule, which is not a `%g`

`FloatString` (`asmsub.c`, Bld 212): `sprintf("%.15e")`, then the exponent
loses its `+` and leading zeros and is dropped when zero, then the mantissa
loses trailing zeros, then **if the whole string exceeds 18 characters the
excess is removed from the decimals immediately BEFORE the last one** (`strmov(d
+ (n - k), d + n)`, `d + n` being the last decimal), then the exponent is
written out as zeros or a leading `0.` whenever the result fits in 18
characters, then a trailing point goes. The cut is why `1/7.0` is
`0.14285714285718` rather than `...714`: the 16-digit `%.15e` mantissa
`1.428571428571428` loses its 13th and 14th decimals and keeps the rounded
`8`. `render_interp_float` in `eval.rs` ports the steps one for one and its
doc comment names them; the rows above are its acceptance.

The first draft of the port's doc comment said asl's own LITERAL parse was
inexact for `1.0e-20`. That was wrong and was corrected before commit: the
nearest double to 1e-20 lies below it, and `%.15e` of that double begins
`9.999999999999999` on glibc and in Rust alike, so both print
`9.999999999999E-21`.

## Red-first

On the committed sources (`git stash` of the four `src`/test edits, the new
test files left in place; both new suites compile against the pre-fix tree):

```
crates/sigil-cli/tests/as_message_stdout.rs
test a_forward_referenced_message_prints_once_with_its_final_value ... FAILED   left: ""  right: "fwd 3\n"
test a_float_division_in_a_message_interpolates ... FAILED                       left: ""  right: "ROM size is $5 bytes (0.0048828125 KiB). About $3 bytes are padding. \n"
test a_message_is_a_bare_stdout_line_and_not_a_diagnostic ... FAILED             left: ""  right: "int 2A\n"
test a_failing_run_prints_its_message_before_its_diagnostics ... FAILED          left: ""  right: "size 3h bytes\n"
test result: FAILED. 0 passed; 4 failed

crates/sigil-frontend-as/tests/as_interp_shapes.rs
test result: FAILED. 3 passed; 8 failed
  the_s2disasm_rom_size_line_renders_as_asl_renders_it:
    left:  ["ROM size is $5 bytes (\{(EndOfRom-StartOfRom)/1024.0} KiB). About $3 bytes are padding. "]
    right: ["ROM size is $5 bytes (0.0048828125 KiB). About $3 bytes are padding. "]
  a_float_division_pastes_decimal_and_an_integral_float_has_no_point:
    left:  ["r0 \{(2048-1024)/1024.0}", "r1 \{1536/1024.0}", "r2 \{n/1.0}", ...]
  a_string_symbol_pastes_its_characters:
    left:  ["r0 \{s}", "r1 \{t}"]
  a_string_symbol_composes_a_name_through_a_nested_interpolation: panicked (assembly refused)
  the three passing ones are the integer-only cells, the Z80 `$` cell and the
  two-in-one-string cell, which the pre-fix code already rendered.
```

The converged-pass test (`a_message_prints_once_with_the_converged_value_of_a_forward_reference`)
reads `Assembled::messages`, a field the pre-fix tree does not have, so its
red is the CLI form above: pre-fix stdout is `""` for the same program; after,
`fwd 3\n` exactly once (the frontend test asserts the list is exactly
`["fwd 3"]`).

`as_message_stdout.rs`'s first draft of the failing-run test used a bare
undefined symbol in `dc.b`, which the front end accepts as a deferred link
symbol; it was changed to an `error` directive so the front end itself fails.
The CLI test keeps the undefined symbol, where the LINK fails, exit 1, and the
message still precedes the diagnostics.

## Corpus: stdout, asl versus sigil after, line by line

| root | asl stdout | sigil before | sigil after |
|---|---|---|---|
| s1disasm | `Uncompressed driver size: 1BC6h bytes.` | nothing | `Uncompressed driver size: 1BBDh bytes.` |
| s2disasm | `ROM size is $100000 bytes (1024 KiB). About $8F1 bytes are padding. ` (trailing space is in the source) | nothing | `ROM size is $FFFED bytes (1023.9814453125 KiB). About $2375 bytes are padding. ` |
| skdisasm | (none) | nothing | (none) |

Same LINES as asl on every root, same count (1 / 1 / 0). The VALUES differ
on the two that print, and the difference is upstream of `message`: s1's
Z80 driver assembles 9 bytes shorter on sigil (the four `ld r,userfunc(arg)`
lines the org-backwards note records), and s2's ROM size and padding are
sigil's own build figures. The interpolation reproduces asl's rendering of
sigil's numbers; asl's stdout on s2disasm is itself from a run that exited 2
with `ASL_DIAG=INCOMPLETE` and is recorded, not relied on.

## Corpus: stderr

| root | before | after | delta |
|---|---|---|---|
| s1disasm | 50 | 50 | none |
| s2disasm | 5,162 | 5,151 | 11 lines removed, 0 added: all `s2.macros.asm(246): error: unexpected character` |
| skdisasm | 2,448 | 2,424 | 24 lines removed, 0 added: all `sonic3k.macros.asm(160): error: unexpected character` |

Not byte-identical, and the brief asked for byte-identical. The 35 removed
lines are one construct: `zoneanimcount_{"\{zoneanimcur}"} = zoneanimcount-1`,
a string symbol pasted through a `\{}` nested in `{}` name composition. Before,
the `\{zoneanimcur}` stayed verbatim (no string branch), the composed name
carried a backslash, and every expansion was an `unexpected character` error.
The string-value shape is on the brief's own probe list, and asl assembles
both lines clean (skdisasm under asl: exit 0, zero diagnostics; asl's s2
stderr names line 246 nowhere). Pinned by
`a_string_symbol_composes_a_name_through_a_nested_interpolation`. No stderr
line was added on any root.

## Bytes

No corpus root emits bytes on either side: all three exit 1 before and after
(the corpus runner passes no `-o`, and the front end fails on each). The
integer interpolation path is untouched, so the `equ`/`set` string-binding
branches that reach bytes render as before for every program that already
assembled; the float and string branches only change text that was
previously left as verbatim `\{...}`.

## Aeon

`git -C aeon grep -n 'message\|warning' origin/master -- '*.asm'`: two hits,
both comments (`debugger.asm:138` `; bypass warnings on privileged
instructions`, `:232` `; Raises an error with the given message`). Neither
directive appears in any aeon `.asm`. `\{` appears twice, both in `error`
directive text (`debugger.asm:694` `\{.__type}`, `:747` `\{.__param}`),
which renders only when the `error` fires. No construct this parcel changed
is on aeon's build path; the controller's four-shape gate is the byte proof.

## Tests

| suite | result |
|---|---|
| `cargo test --release --no-fail-fast -p sigil-frontend-as` | 643 passed / 0 failed / 0 ignored, 54 binaries |
| `SIGIL_ALLOW_PARTIAL=1 cargo test --release --no-fail-fast -p sigil-cli` | 706 passed / 0 failed / 1 ignored (`sigil_diff_reports_byte_identity`, reads the aeon tree; run with `--ignored`), 157 binaries |
| `cargo clippy --release --all-targets -- -D warnings` | clean |

All with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-msg`.

## Open

1. **Character literal**: `\{'A'}` pastes `A` on asl and `41` on sigil. The
   lexer folds `'A'` to `Tok::Int(65)` before interpolation sees it, so the
   two are indistinguishable from `\{65}` at the point of rendering. Corpus
   population zero; lexing is another agent's lane this week.
2. **`MOMLINE`**: sigil has no such builtin, so `\{MOMLINE/1.0}` stays
   verbatim where asl pastes the line number. Corpus population: the four
   trace-macro `message` lines (`s2.macrosetup.asm:84-92`,
   `MacroSetup.asm:106/108`, `sonic3k.macrosetup.asm:82/84`), none of which
   fires on any root.
3. **Exponent-form float literals** (`1e17` without a point) are not lexed as
   floats; the probes were respelled `1.0e17` and render identically on asl.
   Corpus population zero. Lexer lane.
4. The two corpus values that differ from asl (`1BBDh` / `$FFFED`) are the
   ledger's Z80-sizing and ROM-size rows, not this parcel's.

## What in the brief was wrong or drifted

* The brief's s2 census line (`$FFFED` ... `$2375`) was quoted from the
  ledger's pre-fix census and matches what sigil prints now; the asl values it
  gave (`$100000`, `$8F1`) are what asl prints. Both were derived here rather
  than taken.
* "stderr diagnostics byte-identical" could not hold once the string-value
  shape was implemented: the 35-line reduction above is that shape's corpus
  footprint. Reported rather than suppressed.
* The first commit message on this branch said "96 cells"; the count is 102
  measured, 100 matched, and the message was amended before anything was
  built on it.
