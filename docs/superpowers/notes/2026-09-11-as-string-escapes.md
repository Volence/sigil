# AS string escapes: the grammar asl implements, and the 2,176 bytes it cost Sonic 2

2026-09-11, branch `parcel/as-string-escapes`, base `42d3fd29` (contains the merge `8107f3b7`),
implementation commit `99445b2c`, then `9b3f22fd` (two comment lines in `eval.rs` reworded, no
compiled change). Evidence (probe sources, every exit-0 listing, every refused
transcript, the scripts, the run logs and row multisets) is in `2026-09-11-as-string-escapes/`
beside this note.

## Headlines

1. **Sonic 2's silent 2,176 bytes are gone.** On the census's stub B tree (driver moved out of the
   image), sigil differed from the reference ROM in 2,176 bytes over 231 runs outside the one
   asserted window `[0xEC0E8, 0xED04C)`, extent `0xF64`. With this parcel: **0 bytes in 0 runs**,
   the same window printed and asserted, and the stub B image is byte-identical to the census's
   stub D image (md5 `29d1b81e332245f1bd06cb3c4a0df773`), which is the tree where the 35 charset
   lines were rewritten by hand. The uncompressed driver still equals asl's Z80 record in all
   4,872 bytes.
2. **The grammar is wider than the four forms Sonic 2 uses.** asl has eight letter escapes in
   either case, a hex form, an OCTAL and a DECIMAL form (a leading `0` picks octal), three
   self-escapes, and `\{expr}` interpolation in the same scan. Everything else is refused. The
   census's `\NNN decimal` is half of the numeric rule: `\012` is `0A`, not `0C`.
3. **The charset interaction, decided by asl on a non-identity page: the escape is processed
   first, and the character it yields goes through the live code page like any other.** `dc.b
   "A\x41\65"` under `charset $41,$11` is `11 11 11`; a raw byte would give `11 41 41`. Same in
   every context probed. The charset STRING target stays raw, escapes processed.
4. **A second silent defect, found by the probe matrix:** an escaped quote closed the literal.
   `dc.b "a\";b"` assembled to `61 5C` at exit 0 (the lexer ended the string at `\"` and read
   `;b"` as a comment); asl writes `61 22 3B 62`. Fixed by the same change.
5. **Corpora.** Sonic 2 loses exactly its two class-7 rows (`charset '\H'`) and gains nothing.
   Sonic 1 and S3K write no escape other than `\{...}`; their diagnostic multisets and stdout
   are identical before and after.
6. **aeon moves no bytes.** At `origin/master` `826159e7`, no AS-routed `.asm` string carries an
   escape: two backslashes sit in comments and two are `\{...}` in `error` text.

## Provenance

| instrument | identity |
|---|---|
| asl (every probe) | `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`**, through `asl_run` (`asl_ref.sh`), `-xx -n -q -A -L -U -i .`, one construct per file. Every run log prints the md5. |
| asl (the Sonic 2 reference ROM only) | the census's reference, built with `s2disasm`'s own asl md5 `0dee1f98e6480a4783d27ffd8b90896f` under the census's controls; reused, not rebuilt: `b3-final.bin` md5 `9feeb724052c39982d432a7851c98d3e`, p-file `s2.run1.p` md5 `ecf36eb473c8c8371b945fe1a98fe366` (both match the census note), listing `s2.lst` md5 `88299a61cea08cc2ac3641e7fbfabcb8`, Z80 record `z80blob.bin` md5 `d16f56408b30330335598b2ad358e7ba`. |
| sigil before | built from `42d3fd29` (clean), md5 `952c3fa8ffd4dcf6bd3cd8d1d554b12e`. Its stub B image is the census's stub B image (`04048be0...`), the control that the baseline reproduces. |
| sigil final | built from `99445b2c`, sources clean, md5 `de9f5a920f61f3e795bd10f7c2317640`. Every result below was re-run with this binary. (An earlier build of the same sources from the dirty base tree, md5 `cadfd95e...`, gave identical results; the md5 differs because the binary embeds its revision, read from `--version`, not assumed.) The tip `9b3f22fd` differs from `99445b2c` in two comment lines only (`git diff --stat 99445b2c 9b3f22fd`: `eval.rs`, 2 insertions, 2 deletions, both inside `//` comments), so no measured byte depends on which of the two built the binary; the scoped suite and clippy were run at the tip. |
| corpora | `s2disasm` `e45ebf33` (the census revision), plus `build.lua`'s 77 generated inputs, whose md5 list equals the census's `logs/generated-inputs.md5`; `s1disasm` `f6ece657`; `skdisasm` `2fcd861c`. All `cp -a` copies in the scratch directory; nothing was built in the shared checkouts. |

The CARGO target was `/home/volence/sonic_hacks/.scratch/as-string-escapes/target`.

## The grammar, as asl implements it

Every value from an exit-0 listing; probe names are `probes/p_<name>.asm`.

| form | asl | probes |
|---|---|---|
| `\\` `\"` `\'` | `5C` `22` `27` | `a_punct_bslash`, `a_punct_dquote`, `a_punct_squote` |
| `\a` `\b` `\e` `\h` `\i` | `07` `08` `1B` `27` `22` | `a_letter_lc*`, `a_letter_uc*` |
| `\n` `\r` `\t` | `0A` `0D` `09` | same |
| upper case of each | same values (`\A` is `07`, `\H` is `27`, `\I` is `22`) | same |
| `\x` / `\X` + hex | at most TWO hex digits, either case; zero digits is `00` | `\x41` `41`, `\X41` `41`, `\x4` `04`, `\x` `00`, `\xG` `00 47`, `\x4G` `04 47`, `\x414` `41 34`, `\x100` `10 30`, `\xAb` `AB` |
| `\0` + digits | OCTAL: a `0` then at most THREE more digits | `\0` `00`, `\07` `07`, `\010` `08`, `\012` `0A`, `\0123` `53`, `\0377` `FF`, `\01234` `53 34`, `\00012` `01 32`, `\0a` `00 61`, `\0x41` `00 78 34 31` |
| `\1` to `\9` + digits | DECIMAL: at most THREE digits in all | `\8` `08`, `\12` `0C`, `\18` `12`, `\123` `7B`, `\255` `FF`, `\1234` `7B 34`, `\65B` `41 42`, `\1a` `01 61` |
| `\{expr}` | interpolation, same left-to-right scan: `\\{n}` is a backslash and the text `{n}` | `b_interp_after_esc` `41 35`, `d_set_bslash_brace` `5C 7B 6E 7D`, `d_message_bslash_brace` prints `\{n}|5` |

Refused (`error #2010: invalid escape sequence`): `\c \d \f \g \j \k \l \m \o \p \q \s \u \v \w
\y \z` in both cases, every punctuation character tried (`\? \. \, \; \$ \# \@ \% \& \( \) \* \+
\- \/ \: \< \= \> \[ \] \^ \_ \` \| \~ \} \!`), a space, and a backslash that ends the literal.
Refused (`error #1320: range overflow`): `\256`, `\999`, `\0400`, `\0777`, and an octal escape
holding an 8 or a 9 (`\08`, `\09`, `\018`, `\08A`). The digit run is read with `0-9` in both
numeric forms, which is why `\08` is a range error and not `\0` followed by `8`. An unterminated
`\{` is refused (`#1020`) and so is an undefined one (`#1010`).

An escaped quote does not end a literal: `"a\"b"` is `61 22 62`, `"a\";b"` is `61 22 3B 62`,
`"a\",b",$11` is `61 22 2C 62 11`, and a macro argument `"\"x,y"` is ONE argument.

Where asl does NOT process escapes: an `include` path (`include "p_inc\x41.inc"` fails to open,
with `p_incA.inc` present), and a literal that is never evaluated (`if 0` branch, an uncalled
macro body: both exit 0 with `"\c"` in them).

## The probe matrix

232 probes (`scripts/gen_probes.py`), each through asl and each sigil build (`scripts/run_probes.sh`),
checked by `scripts/check_probes.py`: an exit-0 probe must match asl's whole byte stream (every
listing byte, continuation lines and the sentinel included), a refused probe must be refused.

| | before (`952c3fa8`) | final (`de9f5a92`) |
|---|---:|---:|
| MATCH (asl exit 0, same bytes) | 6 | **138** |
| BOTH-REFUSE | 7 | **87** |
| MISMATCH | 136 | **4**, every one a LOUD sigil refusal (below) |
| ACCEPTS-WHAT-ASL-REFUSES | 83 | **3**, none of them bytes an escape produces (below) |

`logs/check-before.txt` and `logs/check-final.txt`. The six baseline matches are the three
escape-free controls (`c_char_plain`, `d_squote_in_dq`, `d_dquote_in_sq`) and the three literals
asl never evaluates (`d_if0_bad_escape`, `g_if0_bad_char`, `d_macro_unused_bad_escape`), so
before this parcel no probe that evaluates an escape matched, and 83 forms asl refuses were
accepted with no error.

Contexts covered: `dc.b` and Z80 `db` strings; `'...'` character constants in `dc.w`/`dc.l`/`dc.b`,
`move.w #'...'` and Z80 `ld a,'...'`; packed expression strings (`move.l #"\x41\x42",d0`); the
charset string target; a character constant as a charset SOURCE (`charset '\H',...`); `message`,
`warning`, `error` text; macro arguments (string and quote-bearing); `strlen`, `substr`, `val`,
`lowstring`, string comparison; `equ` and `:=` string symbols; `irpc` over a string; name
composition `name{"\x41"}`; interpolation next to escapes.

The seven rows that do not match, all outside this defect:

| probe | asl | sigil final | why |
|---|---|---|---|
| `b_dcw_str` | `dc.w "\x41\66"` is `0041 0042` | refused (`STRING_IN_WIDE_DATA`) | wide data strings are refused before this parcel; loud |
| `b_macro_arg_instr` | `m \x41` into `dc.b "<a>"` is `3C 41 3E` | lexer refuses `\` | bare macro argument TEXT is lexed: census class 6, not escapes |
| `d_char_interp` | `'\{n}'` is `0035` | refused | the lexer packs character constants and has no evaluator |
| `d_expr_interp` | `#"\{n}"` is `0035` | refused | the expression parser has no evaluator |
| `d_esc_in_interp_expr` | refused (`n+\x31` is not an expression) | `message` prints the text | `message` keeps an unresolved `\{...}` verbatim, as before; text only |
| `e_charset_src_str`, `e_charset_src_plainstr` | `charset "A",$99` is `#1110` | accepted | a string first operand of `charset`; pre-existing, unrelated to escapes |

## The charset interaction

A non-identity page is the only thing that can tell "the escape goes through the page" from "the
escape is a raw byte"; on the identity page both readings give the same bytes. Probes
`c_*`, `e_*`, `d_z80_page`, page `$41->$11 $27->$55 $5C->$66 $07->$77 $0A->$AA $22->$BB`:

| probe | asl | through the page | raw byte would be |
|---|---|---|---|
| `dc.b "A\x41\65"` | `11 11 11` | yes | `11 41 41` |
| `dc.b "'\H"` | `55 55` | yes | `55 27` |
| `dc.b "\\"`, `"\A"`, `"\n"`, `"\""` | `66`, `77`, `AA`, `BB` | yes | `5C`, `07`, `0A`, `22` |
| `dc.w '\x41'`, `dc.w '\H'` | `0011`, `0055` | yes | `0041`, `0027` |
| `move.w #"\x41",d0` | `303C 0011` | yes | `303C 0041` |
| macro argument `"\x41"`, `s := "\x41"` | `11`, `11` | yes | `41` |
| `dc.b "\0101"` (octal `41`) | `11` | yes | `41` |
| Z80 `db "A\x41\65"`, `ld a,'\x41'` | `11 11 11`, `3E 11` | yes | |
| `charset '\H',$99` then `dc.b "'U"` | `55 99` | the SOURCE index `'\H'` is translated too: index `$55` is remapped | `99 55` |
| `charset $50,"\x41A"` then `dc.b "PQ"` | `41 41` | the string TARGET is raw, escapes processed | |

So: escapes first, then the page, everywhere a character becomes a byte; the one raw place is
unchanged.

## What changed

A token keeps the SOURCE form (`Tok::Str` is the text between the quotes); the new
`crates/sigil-frontend-as/src/escape.rs` computes a value where a literal is used. That is why a
string asl never evaluates (an include path, an `if 0` line) is never judged here either. A
value that becomes a literal again goes back through `escape::quote`, so nothing is unescaped
twice (`g_quote_roundtrip`: `mq "a\\b\"c"` then `dc.b pa` and `strlen(pa)` are `61 5C 62 22 63
05` in both).

- `escape.rs` (new): one walker; `unescape` (string value, interpolation by callback),
  `unescape_plain`, `unescape_keep_interp`, `unescape_bytes` (character constants: source text
  contributes its own bytes, so escape-free constants pack exactly as before), `literal_end`
  (escape-aware end of a literal), `quote`.
- `lexer.rs`: string extent is escape-aware; character constants are unescaped, then every byte
  is mapped through the page.
- `expr.rs`: `string_to_int` unescapes before counting and packing.
- `token.rs`: the `Tok::Str` doc names the source-form contract.
- `eval.rs`, every hunk on the string path:
  - `leading_str_rhs`, `eval_str` (literal arm): the escaped value, `\{...}` left in place for the
    binding sites;
  - `interp_string` and the new `literal_value`: escapes and interpolation in ONE scan;
  - the `equ` and `set` string-binding branches and `eval_name_brace`: a bare literal goes
    through `literal_value`;
  - `subst_name_braces`, `brace_group_end`: escape-aware literal skipping;
  - `eval_if_expr`: compares values, not source text;
  - `directive_db`: a bare literal goes through `literal_value` and is refused, not written, on an
    invalid escape or an interpolation with no value;
  - `directive_charset`: the string target goes through `literal_value` and is counted after
    escapes (`charset $FD,"\x10\11\x12"` is three entries and fits);
  - `bind_macro_arg`: re-quotes through `escape::quote`.

No other `eval.rs` region was touched.

## Tests and the red-first proofs

`crates/sigil-frontend-as/tests/as_string_escapes.rs`, 17 tests, every expected byte from an
exit-0 asl listing. Each half-fix the brief named was applied by `scripts/mutate.py` (which
asserts the original text occurs exactly once and quotes the mutated line back from disk), the
test file run, and the file restored with `git show HEAD:<path> > <path>` and
`git status --porcelain --untracked-files=no` empty before the next. Logs: `logs/red-*.log`.

| half-fix | mutated line (from disk) | red |
|---|---|---|
| escapes in `dc.b`, not in charset targets | `eval.rs:7801  match Ok::<String, crate::escape::EscapeError>(raw.clone()) {` (charset target) | 5 of 17, incl. `a_charset_target_takes_its_escapes_and_stores_them_raw`, `sonic_2_s_charset_backslash_h_line_assembles` |
| escapes in charset targets, not in `dc.b` | `eval.rs:7336  match Ok::<String, crate::escape::EscapeError>(raw.clone()) {` (data string) | 9 of 17, incl. `every_escape_form_asl_accepts_emits_asl_s_bytes_in_a_data_string`; the charset-target tests stay green |
| `\NNN` read as octal | `escape.rs:235  let v = radix_value(text, 8);` (decimal arm) | 13 of 17 (`\12` gives `0A`, `\256` accepted as `AE`) |
| `\0NNN` read as decimal | `escape.rs:226  let v = radix_value(text, 10);` (octal arm) | 3 of 17 (`\012` gives `0C`, `\08` accepted) |
| an escaped `dc.b` character bypasses the page | `eval.rs:7339  ...map(\|c\| if raw.contains('\\') { c as u8 } else { cs.map_char(c) })...` | **exactly 1**: `an_escaped_character_goes_through_the_live_code_page`; all 16 identity-page tests stay green |
| an escaped character-constant byte bypasses the page | `lexer.rs:221  let escaped = line[i + 1..close].contains('\\');` (and the pack below it) | **exactly 1**: the same test |
| character constants left unescaped | `lexer.rs:210  let body = Ok::<Vec<u8>, ...>(line[i + 1..close].as_bytes().to_vec())...` | 5 of 17, incl. `sonic_2_s_charset_backslash_h_line_assembles` (`charset operand 23624 out of range`, the census's class-7 face) |

The two bypass rows are the brief's charset point measured: only a non-identity page catches
them. One correction to the harness's own output: for the two base mutations `mutate.py` printed
two "APPLIED" lines, because it echoes every line equal to the new text and the other numeric
arm already contained it. The line quoted in the table is the one mutated; the region was
quoted from disk before the restore (`sed -n 218,241p`).

## The Sonic 2 byte proof

`scripts/run_s2.sh` = the census's committed `stub.py` (stub B: classes 1 to 9 stubbed, driver out
of the image; stub D: stub B plus the 35 charset lines rewritten by hand) and its committed
`compare.py`, unchanged. `logs/s2-proof-final.log`, `logs/compare-*-stub*.txt`:

```text
before stub B  image 04048be02369bad522f5a9b4a9f0eeea   DIFF outside the window: 2176 bytes in 231 runs
final  stub B  image 29d1b81e332245f1bd06cb3c4a0df773   DIFF outside the window: 0 bytes in 0 runs
final  stub D  image 29d1b81e332245f1bd06cb3c4a0df773   DIFF outside the window: 0 bytes in 0 runs
WINDOW driver hole [0xEC0E8, 0xED04C) len 0xF64  (Snd_Driver from the p-file, guess from s2.constants.asm)
WINDOW asserted: exactly one window, extent 0xF64 bytes = 0.376% of the reference
CONTROL planted outside at 0x200,0x7FFF0,0xFFFFE and inside at 0xEC100 -> reported [('0x200', 1), ('0x7fff0', 1), ('0xffffe', 1)]
DRIVER at 0x300000, 4872 bytes vs asl Z80 record: 0 bytes differ
```

The zero has two positive controls: the planted bytes, and the same instrument reporting 2,176
on the baseline binary. Nothing else changed: the stub B image now equals stub D's to the byte.

## Diagnostic multiset diffs (exact lines, never totals)

`scripts/run_diag.sh`, stderr rows sorted and diffed; full rows in `logs/rows-*.txt`.

| corpus | before | final | rows that left | rows that arrived |
|---|---:|---:|---|---|
| Sonic 2 (census `corpus-gen`: clean tree plus pre-step outputs, the 71-row configuration) | 71 | 69 | `s2.asm(14480):2: error: charset operand 23644 out of range 0..=255` and `s2.asm(14606):2: error: charset operand 12593 out of range 0..=255` | none |
| Sonic 1 (`sonic.asm`) | 1 | 1 | none | none |
| S3K (`sonic3k.asm`) | 189 | 189 | none | none |

stdout (the `message` stream) is identical for Sonic 1 and S3K; for Sonic 2 only the failure
count line moves (71 to 69). The Sonic 1 and S3K zeroes are not vacuous: the same two binaries
move two Sonic 2 rows.

## Escape counts in the corpora

`scripts/corpus_escapes.py` over every `.asm`/`.inc` (escape-aware literal scanning, `logs/corpus-escapes.txt`):

| corpus | lines with a backslash in a literal | escapes other than `\{` | where |
|---|---:|---:|---|
| Sonic 1 | 18 | **0** | all `\{...}`: `message` 6, `fatal` 5, `error` 4, `warning` 2, one `function` body |
| Sonic 2 | 69 | **415** (`\x` 211, digit 202, `\H` 2) | all in the 35 `charset` lines (33 with a string target, 2 also with `'\H'` as the source); the rest `\{...}` in `fatal`/`message`/`:=`/`warning`/`error` and one name composition |
| S3K | 22 | **0** | all `\{...}`: `fatal` 13, `message` 5, `error` 3, one name composition |

No Sonic 2 digit escape begins with `0`, which is why the census's decimal-only reading of `\NNN`
produced a byte-neutral stub C; the grammar still has an octal half. Neither Sonic 1 nor S3K has
an escape in emitted text, so this parcel cannot move their bytes; their interpolation sites now
run through `literal_value`, and their identical row multisets and stdout are the check.

## aeon

```text
git -C /home/volence/sonic_hacks/aeon fetch -q origin
git -C /home/volence/sonic_hacks/aeon rev-parse origin/master          -> 826159e7923d6347c6c7f639347785198d79aa4f
git -C ... grep -n '\\\\' origin/master -- '*.asm'                       -> no output, exit 1
git -C ... grep -n -F '\' origin/master -- '*.asm'                       -> 4 hits
```

| hit | in a string literal? |
|---|---|
| `engine/debug/debugger.asm:271` `; otherwise, just specify \opts ...` | no, a comment |
| `engine/debug/debugger.asm:272` `; ... in case \opts argument is empty or skipped` | no, a comment |
| `engine/debug/debugger.asm:694` `error "Unrecognized type in string operand: \{.__type}"` | yes, `\{...}` interpolation in `error` text: diagnostic text, no bytes, and escape-free around the interpolation, so its text is what it was |
| `engine/debug/debugger.asm:747` `!error "Illegal operand format setting: \{.__param}. ..."` | same |

`*.inc` at the same revision carries no backslash at all. No aeon ROM byte can move. The
controller's strict landing gate proves the four engine shapes.

## Scoped suite and clippy

At the tip `9b3f22fd`, stamped from the worktree on `parcel/as-string-escapes` with no tracked
change (`logs/suite-final.summary.txt`); the same command at `99445b2c` gave the identical totals:

```text
SIGIL_ALLOW_PARTIAL=1 cargo test --release -p sigil-frontend-as -p sigil-cli --no-fail-fast
230 test binaries: passed 1549, failed 0, ignored 1, SUITE_EXIT=0
partial-run banner: 130 test binaries are reference-dependent and were left UNMEASURED (no AEON_DIR, on purpose)
cargo clippy --release -p sigil-frontend-as -p sigil-cli --all-targets -- -D warnings: exit 0
```

The implementation alone, before the test file existed, gave 1532 / 0 / 1 on the same command:
no existing test depended on the old raw-string behaviour, and the 17 added are the difference.
`version_reports_the_head_of_the_tree_it_was_built_from` and
`the_published_line_states_this_revision_s_position_against_a_named_remote_ref` both passed.

## Open, and why

- **Interpolation in a character constant or a packed expression string** (`'\{n}'`,
  `move.w #"\{n}",d0`): asl interpolates; sigil refuses out loud (neither the lexer nor the
  expression parser has an evaluator). No corpus writes either.
- **A bare escape as unquoted macro argument text** (`m \x41`): census class 6, arguments are
  lexed where AS treats them as text. The quoted form works.
- **`\\{` inside a NESTED literal** (`s := lowstring("\\{n}")`): the nested-literal value keeps
  `\{...}` in place for the binding site's interpolation, and cannot tell a literal `\{` made by
  `\\{` from a pending one, so it is interpolated. Bare literals (every corpus site) take one exact
  scan. No corpus has the nested form.
- **`message` keeps an unresolved `\{...}` verbatim** where asl refuses (`d_esc_in_interp_expr`),
  as before this parcel: text, not bytes.
- **`charset "A",$99`** is accepted, asl refuses it (`#1110`): pre-existing, not an escape.
- **Non-ASCII source text in a string** keeps its existing one-byte truncation (unchanged).

## Things in the brief that turned out wrong

1. **The aeon command.** `git grep -n '\\\\'` hands git the basic regex `\\\\`, which matches TWO
   consecutive backslashes, so it cannot find a single one; at `826159e7` it printed nothing and
   exited 1. A single-backslash search (`-F '\'`) finds the four hits above. The conclusion (no
   aeon bytes move) survives, but not on that command's evidence.
2. **"`\NNN` decimal"** (from the census list the brief quotes as the escapes it saw): true only
   when the first digit is not `0`. A leading `0` is octal, three more digits at most.
3. Not wrong, but narrower than it reads: "the 35 `charset` lines" are 33 lines whose only escapes
   are in the string target plus 2 whose source is also the character constant `'\H'`.
