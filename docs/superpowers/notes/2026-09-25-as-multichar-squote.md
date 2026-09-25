# AS-MULTICHAR-SQUOTE: a single-quoted literal is a string; Sonic 3 alone builds

2026-09-25, branch `parcel/as-multichar-squote`, base `1e146771` (master at start).
Commits `c4ec3968` (implementation and tests), `3e083f46` (one comment), and this
note's commit. Evidence beside this note in `2026-09-25-as-multichar-squote/`:
`probe-table.md` (all 307 probes), `scripts/` (generators, runners, the mutation
runner), `logs/`. Scratch: `/home/volence/sonic_hacks/.scratch/as-squote/`.

## Headlines

1. **Sonic 3 alone builds byte-identical.** sigil `3e083f46` on skdisasm `2fcd861c`
   (`s3.asm`, `buildS3.lua`'s p2bin line) exits 0 with 0 rows and writes md5
   `d724ea4dd417fe330c9dcfd955c596b2`, CRC32 `9bc192ce`, 2,097,152 bytes: **0
   differing bytes**, planted-byte control reported exactly. The reference was
   re-derived: `buildS3.lua` run unmodified three times in a `git archive` copy (tar
   md5 `0a4e468053abc841d867d130748629be`), the same md5 all three times.
2. **Nothing else moved.** S1 `afe05eee`, S2 `7b905383`, S3K `0658f691`, S3 Complete
   `a651623a` and aeon's four shapes are identical before and after.
3. **The lexer's reading was wrong in general, not only for `dc.b`.** Base sigil
   silently mis-emitted at exit 0 on at least 20 of the probes below: `dc.w 'A'+'B'`
   (`0083`, asl `4142`), `move.w #'A'+'B',d0` (`303C 0083`, asl `303C 4142`), `dc.w
   'AB'+1` (`4143`, asl `0041 0043`), `move.w #'',d0` (accepted, asl `#1141`), a
   function body's `'a'` never substituted (`61`, asl `28 36 36 29`), `switch 'A'`
   against `case "A"`, `charset 'A','B'+0`. All now match asl.

## asl's rule, as measured

Oracle: `asl_ref.sh`'s `asl_run -xx -n -q -A -L -U -i .` (pinned md5
`61e672562465725a8c102288a7da9098`), then its `p2bin -p=0`. Bytes are quoted only from
runs that exited 0 with `ASL_DIAG=complete`; a refused probe quotes asl's error number
and nothing else.

1. **`'...'` is a STRING**, exactly as `"..."` is, under every operator. `'A'+'B'`
   concatenates; `'AB'+1` is the string `"AC"` (string plus integer, the existing R3
   rule); `-`, `*`, `>>` pack it to an integer; `strlen('ABC')` is 3; string
   comparisons, `switch`/`case`, `message`/`warning` and function-body parameter
   substitution all treat it as a string. In an integer slot it packs big-endian, 1 to
   4 characters (`''` and `'ABCDE'` are `#1141`). Escapes and `\{...}` work inside it.
2. **A data directive writes a string one element per character at its width,**
   except a SINGLE-QUOTED operand whose string has at most `width` characters, which
   is one packed element. `dc.b 'AB'` is `41 42`, `dc.w 'AB'` is `4142`, `dc.w 'ABC'`
   is `0041 0042 0043`, `dc.l 'ABC'` is `0041 4243`, `dc.b ''` is `00` (where `dc.b
   ""` writes nothing).
3. **"Single-quoted" is a property of the operand TEXT,** not of the value: after
   stripping parentheses that enclose the whole operand, the text begins and ends
   with `'`. `dc.w 'A'+'B'` and `dc.w ('A'+'B')` are `4142`; `dc.l 'A'+"B"+'C'`,
   `'A'+('B')+'C'`, `'A'+substr("BC",0,1)+'D'` are single packed longs; `dc.w
   ('A')+'B'`, `'A'+"B"`, `'AB'+1`, `(('A')+('B'))` are sequences. A string symbol
   named bare (or in parentheses) carries the quoting of the operand that defined it
   (`X equ 'AB'`: `dc.w X` is `4142`); inside a larger operand it lends nothing (`dc.w
   X+'B'` is `0041 0042`). No value-level flag combined through `+` reproduces this
   table; the probe set t1..t19 was built to separate the two readings.
4. **A single-quoted `charset` target is the integer** through the live page, any
   length (`charset $41,'BC'` is `#1320`); an unquoted string target is the raw-byte
   table (`charset 'A','B'+0` maps to `$42`). `charset $41,''` and `charset
   $41,'BCDEF'` exit 0 and change neither probed character; sigil refuses both by name.
5. **Element extension:** 68000 `dc.w`/`dc.l` zero-extend a character element; the Z80
   `dw` SIGN-extends it (`dw "\x99\x41"` is `99 FF 41 00`, `dw "\x7f\x80"` is `7F 00
   80 FF`).
6. **String plus integer under a non-identity page** (a cell the earlier parcel
   refused as undecidable): the arithmetic runs on the mapped bytes, and each result
   byte becomes the LOWEST character the page maps to it, dropped when none does. Six
   pages separate this from "write raw" (cp4, cs6 drop a byte) and from "map the raw
   byte again" (rv5 keeps `11`), and rv1/rv2 show the value holds the lowest preimage
   (`('AB'+1)="\x11C"` is 1 under `charset 'A',$11`).
7. **The code page applies at the USE,** not where a symbol is bound (cs8, cs9).
8. **`include 'p.inc'` / `binclude 'p.bin'`** are `#10001 error in opening file`;
   sigil refuses a single-quoted path by name.
9. **Refused by asl:** `'A''B'` and an unterminated `'AB` (`#1020`), `'\q'` (`#2010`),
   `ld a,'AB'` (`#1320`), `dc.l +'A'` (`#1110`).

Z80 vs 68000: same rule; `db`/`dw` behave as `dc.b`/`dc.w` except for rule 5 and
endianness (`dw 'AB'` is `42 41`).

## Probe table (condensed; all 307 in `probe-table.md`)

| probe | source | asl | sigil base `1e146771` | sigil tip `3e083f46` |
|---|---|---|---|---|
| b2 | `dc.b 'AB'` | 4142 | refused | 4142 |
| b5 | `dc.b 'ABCDE'` | 4142434445 | refused | 4142434445 |
| bp1 | `dc.b 'AB'+1` | 4143 | refused | 4143 |
| b1s | `dc.b 'A'+'B'` | 4142 | 83 | 4142 |
| w2 | `dc.w 'AB'` | 4142 | 4142 | 4142 |
| w2d | `dc.w "AB"` | 00410042 | refused | 00410042 |
| w3 | `dc.w 'ABC'` | 004100420043 | refused | 004100420043 |
| wp1 | `dc.w 'AB'+1` | 00410043 | 4143 | 00410043 |
| lp1 | `dc.l 'ABCD'+1` | 00000041 00000042 00000043 00000045 | 41424345 | same as asl |
| cc1 | `dc.w 'A'+'B'` | 4142 | 0083 | 4142 |
| cc2 | `move.w #'A'+'B',d0` | 303c4142 | 303c0083 | 303c4142 |
| imme | `move.w #'',d0` | refused #1141 | 303c0000 | refused |
| f6 | `dc.w ('A')+'B'` | 00410042 | 0083 | 00410042 |
| f8 | `dc.l 'A'+'B'+'C'` | 00414243 | 000000c6 | 00414243 |
| t1 | `dc.l 'A'+substr("BC",0,1)+'D'` | 00414244 | refused | 00414244 |
| t2 | `dc.l 'A'+"B"+'C'` | 00414243 | refused | 00414243 |
| t8 | `dc.l 'A'+1+'C'` | 00004243 | 00000085 | 00004243 |
| sym3 | `X set 'A'` / `X set X+'B'` / `dc.w X` | 00410042 | 0083 | 00410042 |
| cs3 | page A,B,C->11,22,99, $23->$77 / `dc.w 'ABC'` | 001100220099 | refused | 001100220099 |
| cs6 | same page / `dc.b 'AB'+1` | 11 | refused | 11 |
| cs8 | page / `X equ 'CA'` / `charset` / `dc.b X` / `dc.w X` | 43414341 | refused | 43414341 |
| cs9 | `X equ 'CA'` / page / `dc.b X` / `dc.w X` | 99119911 | refused | 99119911 |
| cp4 | `charset $43,$77` / `dc.b 'AB'+1` | 41 | refused | 41 |
| rv1 | `charset 'A',$11` / `dc.b ('AB'+1)="AC",('AB'+1)="\x11C"` | 0001 | 0101 | 0001 |
| rv5 | `charset 'A',$11` / `charset $11,$55` / `dc.b 'AB'+1` | 1143 | refused | 1143 |
| zcs4 | z80 page A,B,C->11h,22h,99h / `dw 'CAB'` | 99ff11002200 | refused | 99ff11002200 |
| h3 | z80 `dw "\x99\x41"` | 99ff4100 | refused | 99ff4100 |
| esc4 | `dc.b 'A\{1+1}B'` | 413242 | refused | 413242 |
| cq1 | `charset 'B',$77` / `charset 'A','B'` / `dc.b "A"` | 77 | 77 | 77 |
| cq6 | `charset 'B',$77` / `charset 'A','B'+0` / `dc.b "A"` | 42 | 77 | 42 |
| fn5 | `f function a,'a'` / `dc.b f(66)` | 28363629 | 61 | 28363629 |
| swq1 | `switch 'A'` / `case 65` / `case "A"` / `elsecase` | 02 | 01 | 02 |
| bsq2 | `dc.b 'A''B'` | refused #1020 | refused | refused |
| cq2 | `charset $41,'BCD'` | refused #1320 | refused | refused |

Totals over the 307 probes (tip vs asl): 171 fixed (base differed or refused, tip
matches), 114 already matching and still matching, 22 differing. Of the 22: 18 are
refusals of shapes asl accepts that sigil already refused and still refuses (below,
open items); **4 read as regressions and are not this parcel's class**: sigil accepts
`dc.b` under `cpu z80` and `dw` under `cpu 68000` for every operand (`zdcb2`: base
writes `01 41 42` for `dc.b 1,"AB"` under Z80; `tdw68`: base writes `01 00` for `dw 1`
under 68000; asl `#1200` both), and base happened to refuse the single-quoted operand
on those lines (`zdcb`, `tw6..8`). Booked.

## Byte identity

Every compare is whole-image against the stock lua build from the same `git archive`
tree (`scripts/mk_trees.sh`, `logs/mk_trees.log`), with `compare.py`'s three-byte
planted control reported exactly (`logs/corpus-base.log`, `logs/corpus-tip-final.log`).
Base binary `1e146771` (own target dir, `--version` clean at `1e146771`); tip binary
`3e083f46` (separate target dir, `--version` clean at `3e083f46`).

| shape | reference (lua build) | base | tip | single-quoted literals it reaches |
|---|---|---|---|---|
| S1 `sonic.asm` | afe05eee | afe05eee, 0 B | afe05eee, 0 B | 4 lines (`#'SEGA'`, `#'init'` x2, `#-'0'+...`) and 8 `charset` lines with a `'...'` operand |
| S2 `s2.asm` | 7b905383 | 7b905383, 0 B | 7b905383, 0 B | `#'SEGA'`, `#'init'` x2, `dc.w make_art_tile(... + 'chr'\|0,0,0)` in an `irpc` over `@ 1992 SEGA`, 71 `charset` lines (2 of them `charset 'a','z','A'`) |
| S3K wrapper | 0658f691 | 0658f691, 0 B | 0658f691, 0 B | `#'SEGA'`, `Ref_Checksum_String := 'SM&K'` (a string symbol packed in two immediates), 7 `make_art_tile('x',...)`, 28 `charset` lines |
| S3 Complete | a651623a | a651623a, 0 B | a651623a, 0 B | as S3K |
| **S3 alone** | **9bc192ce** (3 lua runs) | exit 1, 6 errors, 2576 warnings | **9bc192ce, 0 B** | the 6 rows plus `'J'`/`'U'`/`'E'`, `Ref_Checksum_String := 'init'`, 5 `make_art_tile('x',...)`, 21 `charset` lines |

**Aeon**, `AEON_DIR=/home/volence/sonic_hacks/.aeon-squote` provisioned by
`scripts/provision-aeon-ref.sh` at aeon `ec640bcf` (both REBUILD CONTROLs matched the
golden; `repin --check` printed `pins.rs unchanged`). `scripts/four_shapes.sh` built all
four shapes with each binary (`logs/four-base.log`, `logs/four-tip.log`), each pair
compared whole with its own planted control reported exactly:

| shape | base `1e146771` | tip `3e083f46` |
|---|---|---|
| `sonic4` | 91c46c94 / 820,209 | 91c46c94 / 820,209, 0 B |
| `sonic4` DEBUG | 8a378de6 / 846,509 | 8a378de6 / 846,509, 0 B |
| `demo` | 1c7a34d3 / 96,863 | 1c7a34d3 / 96,863, 0 B |
| `demo` DEBUG | 72e405a5 / 103,185 | 72e405a5 / 103,185, 0 B |

Reachability: a grep for a quoted-pair `'...'` outside a comment in aeon's
`engine/` and `games/` `.asm` finds 0 lines, so the aeon identity attests only that
nothing else moved.

## Tests

* New `crates/sigil-frontend-as/tests/as_single_quoted_string.rs`, run by `cargo test
  -p sigil-frontend-as` (the workspace suite): 82 cases, each `head + body + "\tend\n"`
  with asl's own bytes of that exact source (or its error number), generated from the
  runs by `scripts/gen_cases.py` (`logs/probes-cases.log`), plus named-refusal checks
  and the two `charset` refusals.
* Re-pinned to asl's bytes, each measured on the test's exact source: `as_string_
  numeric_typing.rs` (`a_string_symbol_in_wide_data_is_written_per_character`, which
  pinned a refusal; the new `string_plus_integer_under_a_page_takes_the_lowest_
  preimage`, which replaces a refusal assertion), `as_string_literal_integer.rs`
  (`a_wide_data_directive_writes_a_string_per_character`, which pinned a refusal),
  `as_signed_int_literal.rs` (`+'A'`/`+'AB'` moved from the accept-more residual to a
  refusal, as that test's own message asked), and the eval.rs unit test that pinned
  the `switch`/`case` divergence now asserts asl's `EE`. Its `dw` rows were written
  under `cpu 68000`, where asl refuses `dw`; they now use `cpu z80`.

* **Full suite** at `3e083f46` plus this note's uncommitted docs and one comment
  (`logs/suite-full.log`, stamped pwd/HEAD/branch, `AEON_DIR` as above): `cargo test
  --release --workspace --no-fail-fast` 5688 passed, 1 failed, 2 ignored; the one
  failure is the known environmental `m1b_gate::oracle_loadfromaslisting_resolves_
  emit_listing` (no `ORACLE_DIR`). `cargo clippy --release --workspace --all-targets
  -- -D warnings`: clean.

### Red-first

* **R0, the defect itself:** the committed new test file run against a `git archive`
  of base `1e146771` (`scripts/red_base.sh`, `logs/red-base.log`): 0 passed, 3 failed,
  `59 of 82 cases differ from asl`, the first row `s3_region_block: asl 4a005545...,
  sigil refused ["operand 21829 out of range -128..=255", ...]`: the lexer packing
  `'UE'` into one integer.
* **Implementation mutations** (`scripts/mutate.py`, `logs/mutations*.stdout`), each one
  exact replacement shown as the on-disk `git diff`, run, then restored with `git show
  HEAD:path >path` and `git status` clean. All nine red:

| mutation | red diagnostic (first rows) |
|---|---|
| M1 no operand single-quoted | `23 of 82 cases differ`; `w_fits: asl 4142, sigil 00410042` |
| M2 rule reads only the first token | `9 of 82`; `w_plus_int: asl 00410043, sigil 4143`, `w_double_tail: asl 00410042, sigil 4142` |
| M3 Z80 `dw` zero-extends | `3 of 82`; `z_dw_sign: asl 99ff41007f0080ff, sigil 990041007f008000` |
| M4 string+int keeps raw bytes | `cs_plus_int_dropped: asl 11, sigil 1177`; `cs_target_string: asl 42, sigil 77`; preimage test red |
| M5 symbol forgets its quoting | `6 of 82`; `sym_equ: asl 414241424142, sigil 00410042414200410042` |
| M6 quoted charset target as table | `cs_target_char: asl 7777, sigil 4242`; `r_charset_two` accepted |
| M7 include opens a `'...'` path | `r_include: refusal does not say "include needs a double-quoted path": cannot include .../p.inc` |
| M8 fit is `<` not `<=` | `14 of 82`; `w_fits: asl 4142, sigil 00410042` |
| M9 highest preimage | `cs_target_string: asl 42, sigil 77`; preimage test `left: [1, 0]` |

## Open items

* **Shapes asl accepts that sigil refuses (loud, unchanged by this parcel):**
  `upstring` (unimplemented builtin); `defb`/`defw` (unimplemented directives); a
  string of 5+ characters under a non-`+` operator (asl gives its LENGTH: `move.l
  #'ABCDEF'|0,d0` is `203C 0000 0006`; `imm8`'s base answer was silently wrong, now
  refused); a 5+ character string plus an integer (asl emits nothing); `charset
  $41,''`/`'BCDEF'`.
* **CPU-agnostic `dc.b`/`dw`** accept operands under the wrong CPU (above). Booked.
* **A `+` whose integer side does not fold yet** in `dc.w`/`dc.l`/`dw` with a
  single-quoted side takes the numeric path; asl writes the sum per character, which
  differs once it outgrows one element. Documented on `STRING_IN_WIDE_DATA`, booked.
* **Runtime:** whether Sonic 3 alone plays is not a byte question. TAGGED for the
  controller; no emulator was touched.

## Things in the brief that turned out wrong

1. **"Elsewhere (`dc.w`, an immediate) it stays the packed integer"** (the S3K note's
   reading of sq1). Only when it fits: `dc.w 'ABC'` is `0041 0042 0043`, and the fit is
   decided by the operand's text, not the value.
2. **"`'AB'+1` in `dc.w`"** is `0041 0043`, a sequence, not `4143`; base sigil wrote
   `4143` silently.
3. **"Refuse by name anything asl refuses"** is not the whole contract here: asl also
   ACCEPTS shapes with no dependable answer (the length-valued 5-character string,
   the empty or long `charset` target), which sigil refuses by name.
4. The lexer comment claiming "a character constant is a packed INTEGER in every
   width, never a character sequence" was drawn from `dc.l 'INIT'` (which fits); it is
   false for every non-fitting length.
5. The earlier ledger line "asl zero-extends one element per character" for wide data
   is false for the Z80 `dw`, which sign-extends.
